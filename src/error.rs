use crate::{Code, Span};
use nom::error::ErrorKind;
use nom::IResult;
use std::error::Error;
use std::fmt;
use std::fmt::Display;

/// Type alias for a nom parser. Use this to create a ParserError directly in nom.
pub type ParserNomResult<'s, C> = IResult<Span<'s>, Span<'s>, ParserError<'s, C>>;

/// Error for the Parser.
pub struct ParserError<'s, C: Code> {
    /// Error code.
    pub code: C,
    /// Error span.
    pub span: Span<'s>,
    /// Flag for Tracer.
    pub tracing: bool,
    /// Collected nom errors if any.
    pub nom: Vec<Nom<'s>>,
    /// Suggest values.
    pub suggest: Vec<Suggest<'s, C>>,
    /// Expect values.
    pub expect: Vec<Expect<'s, C>>,
}

impl<'s, C: Code> ParserError<'s, C> {
    /// New error.
    pub fn new(code: C, span: Span<'s>) -> Self {
        Self {
            code,
            span,
            tracing: false,
            nom: Vec::new(),
            suggest: Vec::new(),
            expect: Vec::new(),
        }
    }

    /// Convert to a new error code.
    pub fn as_err(mut self, code: C) -> Self {
        self.code = code;
        self
    }

    /// Special error code. Encodes errors occurring at the margins.
    pub fn is_special(&self) -> bool {
        self.code.is_special()
    }

    /// Error code of the parser.
    pub fn is_parser(&self) -> bool {
        !self.code.is_special()
    }

    /// Is this one of the nom errorkind codes?
    pub fn is_kind(&self, kind: ErrorKind) -> bool {
        for n in &self.nom {
            if n.kind == kind {
                return true;
            }
        }
        false
    }

    /// Was this one of the expected errors.
    pub fn is_expected(&self, code: C) -> bool {
        for exp in &self.expect {
            if exp.code == code {
                return true;
            }
        }
        false
    }

    /// Was this one of the expected errors, and is in the call stack of parent?
    pub fn is_expected2(&self, code: C, parent: C) -> bool {
        for exp in &self.expect {
            if exp.code == code {
                for par in &exp.parents {
                    if *par == parent {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Create a ParseOFError from a nom::Err
    pub fn nom(e: nom::Err<nom::error::Error<Span<'s>>>) -> ParserError<'s, C> {
        match e {
            nom::Err::Error(e) => ParserError::new(C::NOM_ERROR, e.input),
            nom::Err::Failure(e) => ParserError::new(C::NOM_FAILURE, e.input),
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }

    /// ParseIncomplete variant.
    pub fn parse_incomplete(span: Span<'s>) -> ParserError<'s, C> {
        ParserError::new(C::PARSE_INCOMPLETE, span)
    }
}

impl<'s, C: Code> Display for ParserError<'s, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} expects ", self.code)?;
        for (i, exp) in self.expect.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}:\"{}\"", exp.code, exp.span)?;
        }
        // no suggest
        write!(
            f,
            " for span {} \"{}\"",
            self.span.location_offset(),
            self.span
        )?;
        Ok(())
    }
}

impl<'s, C: Code> Error for ParserError<'s, C> {}

/// Coop with nom.
impl<'s, C: Code> nom::error::ParseError<Span<'s>> for ParserError<'s, C> {
    fn from_error_kind(span: Span<'s>, kind: ErrorKind) -> Self {
        ParserError {
            code: C::NOM_ERROR,
            span,
            tracing: false,
            nom: vec![Nom { kind, span }],
            suggest: Vec::new(),
            expect: Vec::new(),
        }
    }

    fn append(input: Span<'s>, kind: ErrorKind, mut other: Self) -> Self {
        other.nom.push(Nom { kind, span: input });
        other
    }
}

impl<'s, C> From<nom::Err<ParserError<'s, C>>> for ParserError<'s, C>
where
    C: Code,
{
    fn from(e: nom::Err<ParserError<'s, C>>) -> Self {
        match e {
            nom::Err::Error(e) => e,
            nom::Err::Failure(e) => e,
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}

impl<'s, C> From<nom::Err<nom::error::Error<Span<'s>>>> for ParserError<'s, C>
where
    C: Code,
{
    fn from(e: nom::Err<nom::error::Error<Span<'s>>>) -> Self {
        match e {
            nom::Err::Error(e) => ParserError::new(C::NOM_ERROR, e.input),
            nom::Err::Failure(e) => ParserError::new(C::NOM_FAILURE, e.input),
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DebugWidth {
    /// Debug flag, can be set with width=0.
    Short,
    /// Debug flag, can be set with width=1.
    Medium,
    /// Debug flag, can be set with width=2.
    Long,
}

/// Data gathered from nom.
#[derive(Clone)]
pub struct Nom<'s> {
    /// Errorkind ala nom
    pub kind: ErrorKind,
    /// Span
    pub span: Span<'s>,
}

/// Suggestions, optional tokens.
#[derive(Clone)]
pub struct Suggest<'s, C> {
    /// Code for the token.
    pub code: C,
    /// Span
    pub span: Span<'s>,
    /// Parser call stack.
    pub parents: Vec<C>,
}

/// Expected tokens.
#[derive(Clone)]
pub struct Expect<'s, C> {
    /// Code for the token.
    pub code: C,
    /// Span.
    pub span: Span<'s>,
    /// Parser call stack.
    pub parents: Vec<C>,
}

impl From<Option<usize>> for DebugWidth {
    fn from(value: Option<usize>) -> Self {
        match value {
            None | Some(0) => DebugWidth::Short,
            Some(1) => DebugWidth::Medium,
            Some(2) => DebugWidth::Long,
            _ => DebugWidth::Short,
        }
    }
}
