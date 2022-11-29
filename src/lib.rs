#![doc=include_str!("../readme.md")]

mod debug;
pub mod error;
pub mod span;
pub mod tracer;

use crate::error::{DebugWidth, ParserError};
use crate::tracer::Track;
use nom_locate::LocatedSpan;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::BitOr;

/// Code for parser errors and parser functions.
pub trait Code: Copy + Display + Debug + PartialEq {
    const NOM_ERROR: Self;
    const NOM_FAILURE: Self;
    const PARSE_INCOMPLETE: Self;

    fn is_special(&self) -> bool {
        *self == Self::NOM_ERROR || *self == Self::NOM_FAILURE || *self == Self::PARSE_INCOMPLETE
    }
}

/// Standard input type.
pub type Span<'s> = LocatedSpan<&'s str>;

/// Result type.
pub type ParseResult<'s, O, C> = Result<(Span<'s>, O), ParserError<'s, C>>;

/// Adds a span as location and converts the error to a ParserError.
pub trait IntoParserError<'s, T, C>
where
    C: Code,
{
    /// Maps some error and adds the information of the span where the error occured.
    fn parser_error(self, span: Span<'s>) -> Result<T, ParserError<'s, C>>;
}

/// Result of a look-ahead. Can be chained with | (bit-or).
/// Can be converted from Result for use with nom.
#[derive(PartialEq, Eq)]
pub enum LookAhead {
    /// Do parse this branch.
    Parse,
    /// Don't parse this branch.
    Break,
}

/// Trait for one parser function.
pub trait Parser<'s, O, C: Code> {
    /// Function and error code.
    fn id() -> C;

    /// Possible look-ahead.
    fn lah(_: Span<'s>) -> LookAhead {
        LookAhead::Parse
    }

    /// Parses the expression.
    fn parse<'t>(trace: &'t impl Tracer<'s, C>, rest: Span<'s>) -> ParseResult<'s, O, C>;
}

/// Compose look ahead values. BitOr seems plausible.
impl BitOr for LookAhead {
    type Output = LookAhead;

    fn bitor(self, rhs: Self) -> Self::Output {
        if self == LookAhead::Parse || rhs == LookAhead::Parse {
            LookAhead::Parse
        } else {
            LookAhead::Break
        }
    }
}

/// Any Ok() result means parse, break otherwise.
impl<T, E> From<Result<T, E>> for LookAhead {
    fn from(e: Result<T, E>) -> Self {
        if e.is_ok() {
            LookAhead::Parse
        } else {
            LookAhead::Break
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

    fn enter(&self, func: C, span: Span<'s>);

    fn step(&self, step: &'static str, span: Span<'s>);

    fn debug<T: Into<String>>(&self, step: T);

    fn suggest(&self, suggest: C, span: Span<'s>);

    fn stash(&self, err: ParserError<'s, C>);

    fn ok<T>(&'_ self, span: Span<'s>, rest: Span<'s>, val: T) -> ParseResult<'s, T, C>;

    fn err<T>(&'_ self, err: ParserError<'s, C>) -> ParseResult<'s, T, C>;

    fn write_debug<'a, 'b>(
        &'a self,
        f: &mut Formatter<'_>,
        w: DebugWidth,
        filter: FilterFn<'b, 's, C>,
    ) -> fmt::Result;
}

/// Filter type for Tracer::write_debug
pub type FilterFn<'a, 's, C> = &'a dyn for<'t> Fn(&'t Track<'s, C>) -> bool;
