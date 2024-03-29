#![doc=include_str!("../readme.md")]

mod debug;
pub mod error;
pub mod notracer;
pub mod rtracer;
pub mod span;
pub mod test;
pub mod test2;
pub mod tracer;

pub use crate::debug::restrict_n;

use crate::error::ParserError;
use crate::tracer::Track;
use nom_locate::LocatedSpan;
use std::fmt::{Debug, Display};

/// Standard input type.
pub type Span<'s> = LocatedSpan<&'s str>;

/// Result type.
pub type ParserResult<'s, C, O> = Result<O, ParserError<'s, C>>;

/// Type alias for a nom parser. Use this to create a ParserError directly in nom.
pub type ParserNomResult<'s, C> = Result<(Span<'s>, Span<'s>), nom::Err<ParserError<'s, C>>>;

/// Filter type for Tracer::write_debug
pub type FilterFn<'a, C> = &'a dyn Fn(&Track<'_, C>) -> bool;

/// Code for parser errors and parser functions.
pub trait Code: Copy + Display + Debug + PartialEq {
    const NOM_ERROR: Self;
    const NOM_FAILURE: Self;
    const PARSE_INCOMPLETE: Self;

    fn is_special(&self) -> bool {
        *self == Self::NOM_ERROR || *self == Self::NOM_FAILURE || *self == Self::PARSE_INCOMPLETE
    }
}

/// Adds a span as location and converts the foreign error to a ParserError.
pub trait IntoParserResultAddSpan<'s, C, O>
where
    C: Code,
{
    /// Maps some error and adds the information of the span where the error occurred.
    fn into_with_span(self, span: Span<'s>) -> ParserResult<'s, C, O>;
}

/// Adds a code and converts the foreign error to a ParserError.
/// And wraps it up in a ParserResult.
pub trait IntoParserResultAddCode<'s, C, O>
where
    C: Code,
{
    /// Maps some error with a special error code.
    fn into_with_code(self, code: C) -> ParserResult<'s, C, O>;
}

/// Adds a code and converts the foreign error to a ParserError.
pub trait IntoParserError<'s, C>
where
    C: Code,
{
    /// Maps some error with a special error code.
    fn into_with_code(self, code: C) -> ParserError<'s, C>;
}

/// Trait for one static parser.
pub trait Parser<'s, O, C: Code> {
    /// Function and error code.
    fn id() -> C;

    /// Possible look-ahead.
    fn lah(span: Span<'s>) -> bool {
        !span.is_empty()
    }

    /// Parses the expression.
    fn parse<'t>(
        trace: &'t mut impl Tracer<'s, C>,
        rest: Span<'s>,
    ) -> ParserResult<'s, C, (Span<'s>, O)>;
}

/// Trait for one parser with configuration.
pub trait ConfParser<'s, O, C: Code> {
    /// Function and error code.
    fn id(&self) -> C;

    /// Possible look-ahead.
    fn lah(&self, _span: Span<'s>) -> bool {
        true
    }

    /// Parses the expression.
    fn parse<'t>(
        &self,
        trace: &'t mut impl Tracer<'s, C>,
        rest: Span<'s>,
    ) -> ParserResult<'s, C, (Span<'s>, O)>;
}

/// Treats the result of a parser as optional.
///
/// The exact return value is defined in the impl, but should include some Option<..>.
pub trait ParseAsOptional<'s, C: Code, O> {
    /// Returns a ParserResult.
    fn optional(self) -> ParserResult<'s, C, O>;
    /// Returns a ParserResult.
    /// The original ParserError can be processed with the closure.
    fn optional_with(self, err_op: &dyn Fn(ParserError<'s, C>)) -> ParserResult<'s, C, O>;
}

impl<'s, C: Code, O> ParseAsOptional<'s, C, (Span<'s>, Option<O>)>
    for ParserResult<'s, C, (Span<'s>, O)>
{
    /// Returns None for any Err
    fn optional(self) -> ParserResult<'s, C, (Span<'s>, Option<O>)> {
        match self {
            Ok((rest, tok)) => Ok((rest, Some(tok))),
            Err(e) => Ok((e.span, None)),
        }
    }

    /// Returns None for any Err, calls err_op.
    fn optional_with(
        self,
        err_op: &dyn Fn(ParserError<'s, C>),
    ) -> ParserResult<'s, C, (Span<'s>, Option<O>)> {
        match self {
            Ok((rest, tok)) => Ok((rest, Some(tok))),
            Err(e) => {
                let span = e.span;
                err_op(e);
                Ok((span, None))
            }
        }
    }
}

impl<'s, C: Code> ParseAsOptional<'s, C, (Span<'s>, Option<Span<'s>>)> for ParserNomResult<'s, C> {
    /// Returns nom::Err::Error as None.
    /// Returns nom::Err::Failure as Err.
    /// Panics for nom::Err::Incomplete.
    fn optional(self) -> ParserResult<'s, C, (Span<'s>, Option<Span<'s>>)> {
        match self {
            Ok((rest, tok)) => Ok((rest, Some(tok))),
            Err(nom::Err::Error(e)) => Ok((e.span, None)),
            Err(nom::Err::Failure(e)) => Err(e.into()),
            Err(nom::Err::Incomplete(_)) => unreachable!(),
        }
    }

    /// Returns nom::Err::Error as None and calls err_op.
    /// Returns nom::Err::Failure as Err.
    /// Panics for nom::Err::Incomplete.
    fn optional_with(
        self,
        err_op: &dyn Fn(ParserError<'s, C>),
    ) -> ParserResult<'s, C, (Span<'s>, Option<Span<'s>>)> {
        match self {
            Ok((rest, tok)) => Ok((rest, Some(tok))),
            Err(nom::Err::Error(e)) => {
                let span = e.span;
                err_op(e);
                Ok((span, None))
            }
            Err(nom::Err::Failure(e)) => Err(e),
            Err(nom::Err::Incomplete(_)) => unreachable!(),
        }
    }
}

/// Traces the parser and helps generating errors and suggestions.
///
/// The necessary framing are the call to trace.enter() to establish the environment, and
/// either a call to ok or err at each exit point of the function.
///
/// TrackParseResult can be useful when calling further parse functions. It's method trace()
/// helps keep track of an early exit via the ? operator.
///
/// Use suggest() for optional parts that should be hinted somewhere.
///
/// Use stash() to store parser errors that might be used later. Eg if none of several
/// alternatives fit. All stashed parser errors will be collected and attach as Expect value
/// to a new summary error.
///
pub trait Tracer<'s, C: Code> {
    /// Create a new tracer.
    fn new() -> Self;

    /// Enter a parser function. Absolutely necessary for the rest.
    fn enter(&mut self, func: C, span: Span<'s>);

    /// Keep track of steps in a complicated parser.
    fn step(&mut self, step: &'static str, span: Span<'s>);

    /// Some detailed debug information.
    fn debug<T: Into<String>>(&mut self, step: T);

    /// Adds a suggestion for the current stack frame.
    fn suggest(&mut self, suggest: C, span: Span<'s>);

    /// Adds a expectation for the current stack frame.
    fn expect(&mut self, expect: C, span: Span<'s>);

    /// Keep track of this error.
    fn stash(&mut self, err: ParserError<'s, C>);

    /// Write a track for an ok result.
    fn ok<T>(
        &'_ mut self,
        rest: Span<'s>,
        span: Span<'s>,
        val: T,
    ) -> ParserResult<'s, C, (Span<'s>, T)>;

    /// Write a track for an error.
    fn err<T>(&'_ mut self, err: ParserError<'s, C>) -> ParserResult<'s, C, T>;
}

// TrackParseResult ------------------------------------------------------

/// Can be used to track the results of calls to another Parser or nom-parser.
///
pub trait TrackParseResult<'s, 't, C: Code> {
    type Result;

    /// Translates the error code and adds the standard expect value.
    /// Then tracks the error and marks the current function as finished.
    fn track(self, trace: &'t mut impl Tracer<'s, C>) -> Self::Result;

    /// Translates the error code and adds the standard expect value.
    /// Then tracks the error and marks the current function as finished.
    fn track_as(self, trace: &'t mut impl Tracer<'s, C>, code: C) -> Self::Result;
}

impl<'s, 't, O, C: Code> TrackParseResult<'s, 't, C> for ParserResult<'s, C, O> {
    type Result = Self;

    fn track(self, trace: &'t mut impl Tracer<'s, C>) -> Self::Result {
        match self {
            Ok(_) => self,
            Err(e) => trace.err(e),
        }
    }

    fn track_as(self, trace: &'t mut impl Tracer<'s, C>, code: C) -> Self::Result {
        match self {
            Ok(_) => self,
            Err(e) => trace.err(e.into_code(code)),
        }
    }
}

impl<'s, 't, C: Code> TrackParseResult<'s, 't, C>
    for Result<(Span<'s>, Span<'s>), nom::Err<ParserError<'s, C>>>
{
    type Result = Result<(Span<'s>, Span<'s>), ParserError<'s, C>>;

    fn track(self, trace: &'t mut impl Tracer<'s, C>) -> Self::Result {
        match self {
            Ok(v) => Ok(v),
            Err(e) => trace.err(e.into()),
        }
    }

    fn track_as(self, trace: &'t mut impl Tracer<'s, C>, code: C) -> Self::Result {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                let pe: ParserError<'s, C> = e.into();
                trace.err(pe.into_code(code))
            }
        }
    }
}

impl<'s, 't, C: Code> TrackParseResult<'s, 't, C>
    for Result<(Span<'s>, Span<'s>), nom::Err<nom::error::Error<Span<'s>>>>
{
    type Result = Result<(Span<'s>, Span<'s>), ParserError<'s, C>>;

    fn track(self, trace: &'t mut impl Tracer<'s, C>) -> Self::Result {
        match self {
            Ok(v) => Ok(v),
            Err(e) => trace.err(e.into()),
        }
    }

    fn track_as(self, trace: &'t mut impl Tracer<'s, C>, code: C) -> Self::Result {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                let pe: ParserError<'s, C> = e.into();
                trace.err(pe.into_code(code))
            }
        }
    }
}
