use crate::error::{DebugWidth, ParserError};
use crate::{Code, ParserResult, Span, Tracer};
use std::fmt;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;

/// Tracing and error collection.
pub struct NoTracer<'s, C: Code> {
    _phantom: PhantomData<(&'s str, C)>,
}

impl<'s, C: Code> Tracer<'s, C> for NoTracer<'s, C> {
    /// New one.
    fn new() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }

    /// Enter a parser function. Absolutely necessary for the rest.
    fn enter(&self, _func: C, _span: Span<'s>) {}

    /// Keep track of steps in a complicated parser.
    fn step(&self, _step: &'static str, _span: Span<'s>) {}

    /// Some detailed debug information.
    fn debug<T: Into<String>>(&self, _step: T) {}

    /// Adds a suggestion for the current stack frame.
    fn suggest(&self, _suggest: C, _span: Span<'s>) {}

    /// Keep track of this error.
    fn stash(&self, _err: ParserError<'s, C>) {}

    /// Write a track for an ok result.
    fn ok<'t, T>(
        &'t self,
        rest: Span<'s>,
        _span: Span<'s>,
        val: T,
    ) -> ParserResult<'s, C, (Span<'s>, T)> {
        Ok((rest, val))
    }

    /// Write a track for an error.
    fn err<'t, T>(&'t self, err: ParserError<'s, C>) -> ParserResult<'s, C, T> {
        // Freshly created error.
        // if !err.tracing {
        //     err.tracing = true;
        // }

        // when backtracking we always replace the current error code.
        //err.code = self.func();

        Err(err)
    }
}

// output
impl<'s, C: Code> NoTracer<'s, C> {
    /// Write a debug output of the Tracer state.
    pub fn write(&self, _out: &mut impl fmt::Write, _w: DebugWidth) -> fmt::Result {
        Ok(())
    }
}

// expect
impl<'s, C: Code> NoTracer<'s, C> {}

// suggest
impl<'s, C: Code> NoTracer<'s, C> {}

// call frame tracking
impl<'s, C: Code> NoTracer<'s, C> {}

// basic tracking
impl<'s, C: Code> NoTracer<'s, C> {}

// Track -----------------------------------------------------------------

/// Hint at how the ExpectTrack and SuggestTrack were used.
#[derive(Debug)]
pub enum Usage {
    /// Newly created, currently in use.
    Track,
    /// Forgotten.
    Drop,
    /// Move to a ParseOFError.
    Use,
}

impl Display for Usage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Usage::Track => write!(f, "track"),
            Usage::Drop => write!(f, "drop"),
            Usage::Use => write!(f, "use"),
        }
    }
}
