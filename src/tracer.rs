use crate::debug::tracer::debug_tracer;
use crate::error::{DebugWidth, Expect, ParserError, Suggest};
use crate::{Code, FilterFn, ParserResult, Span, Tracer};
use std::cell::RefCell;
use std::fmt;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;

/// Tracing and error collection.
pub struct CTracer<'s, C: Code> {
    /// Function call stack.
    pub(crate) func: RefCell<Vec<C>>,

    /// Collected tracks.
    pub(crate) track: RefCell<Vec<Track<'s, C>>>,
    pub(crate) suggest: RefCell<Vec<SuggestTrack<'s, C>>>,
    pub(crate) expect: RefCell<Vec<ExpectTrack<'s, C>>>,
}

impl<'s, C: Code> Tracer<'s, C> for CTracer<'s, C> {
    /// New one.
    fn new() -> Self {
        Self {
            func: RefCell::new(Vec::new()),
            track: RefCell::new(Vec::new()),
            suggest: RefCell::new(Vec::new()),
            expect: RefCell::new(Vec::new()),
        }
    }

    /// Enter a parser function. Absolutely necessary for the rest.
    fn enter(&self, func: C, span: Span<'s>) {
        self.push_func(func);
        self.push_suggest(func);
        self.push_expect(func);

        self.track_enter(span);
    }

    /// Keep track of steps in a complicated parser.
    fn step(&self, step: &'static str, span: Span<'s>) {
        self.track_step(step, span);
    }

    /// Some detailed debug information.
    fn debug<T: Into<String>>(&self, step: T) {
        self.track_debug(step.into());
    }

    /// Adds a suggestion for the current stack frame.
    fn suggest(&self, suggest: C, span: Span<'s>) {
        self.debug(format!("suggest {}:\"{}\" ...", suggest, span));
        self.add_suggest(suggest, span);
    }

    /// Keep track of this error.
    fn stash(&self, err: ParserError<'s, C>) {
        self.debug(format!("expect {}:\"{}\" ...", err.code, err.span));
        self.add_expect(err.code, err.span);
        self.append_expect(err.expect);
        self.append_suggest(err.suggest);
    }

    /// Write a track for an ok result.
    fn ok<'t, T>(
        &'t self,
        span: Span<'s>,
        rest: Span<'s>,
        val: T,
    ) -> ParserResult<'s, C, (Span<'s>, T)> {
        self.track_ok(rest, span);

        let expect = self.pop_expect();
        self.track_expect(Usage::Drop, expect.list);
        let suggest = self.pop_suggest();
        // Keep suggests, sort them out later.
        // Drop at the toplevel if no error occurs?
        if !self.suggest.borrow().is_empty() {
            self.append_suggest(suggest.list);
            //self.track_suggest(Usage::Drop, suggest.list);
        }

        self.track_exit();
        self.pop_func();

        Ok((rest, val))
    }

    /// Write a track for an error.
    fn err<'t, T>(&'t self, mut err: ParserError<'s, C>) -> ParserResult<'s, C, T> {
        // Freshly created error.
        if !err.tracing {
            err.tracing = true;
            self.add_expect(err.code, err.span);
        }

        // when backtracking we always replace the current error code.
        err.code = self.func();

        let mut exp = self.pop_expect();
        self.track_expect(Usage::Use, exp.list.clone());
        err.expect.append(&mut exp.list);

        let mut sug = self.pop_suggest();
        self.track_suggest(Usage::Use, sug.list.clone());
        err.suggest.append(&mut sug.list);

        self.track_error(&err);

        self.track_exit();
        self.pop_func();

        Err(err)
    }

    /// Write a debug output of the Tracer state.
    fn write(
        &self,
        out: &mut impl fmt::Write,
        w: DebugWidth,
        filter: FilterFn<'_, C>,
    ) -> fmt::Result {
        debug_tracer(out, w, self, filter)
    }
}

// expect
impl<'s, C: Code> CTracer<'s, C> {
    fn push_expect(&self, func: C) {
        self.expect.borrow_mut().push(ExpectTrack {
            func,
            usage: Usage::Track,
            list: Vec::new(),
            parents: self.parent_vec(),
        })
    }

    fn pop_expect(&self) -> ExpectTrack<'s, C> {
        self.expect.borrow_mut().pop().unwrap()
    }

    fn add_expect(&self, code: C, span: Span<'s>) {
        self.expect
            .borrow_mut()
            .last_mut()
            .unwrap()
            .list
            .push(Expect {
                code,
                span,
                parents: self.parent_vec(),
            })
    }

    fn append_expect(&self, mut expect: Vec<Expect<'s, C>>) {
        self.expect
            .borrow_mut()
            .last_mut()
            .unwrap()
            .list
            .append(&mut expect);
    }
}

// suggest
impl<'s, C: Code> CTracer<'s, C> {
    fn push_suggest(&self, func: C) {
        self.suggest.borrow_mut().push(SuggestTrack {
            func,
            usage: Usage::Track,
            list: Vec::new(),
            parents: self.parent_vec(),
        })
    }

    fn pop_suggest(&self) -> SuggestTrack<'s, C> {
        self.suggest.borrow_mut().pop().unwrap()
    }

    fn add_suggest(&self, code: C, span: Span<'s>) {
        self.suggest
            .borrow_mut()
            .last_mut()
            .unwrap()
            .list
            .push(Suggest {
                code,
                span,
                parents: self.parent_vec(),
            })
    }

    fn append_suggest(&self, mut suggest: Vec<Suggest<'s, C>>) {
        self.suggest
            .borrow_mut()
            .last_mut()
            .unwrap()
            .list
            .append(&mut suggest);
    }
}

// call frame tracking
impl<'s, C: Code> CTracer<'s, C> {
    // enter function
    fn push_func(&self, func: C) {
        self.func.borrow_mut().push(func);
    }

    // leave current function
    fn pop_func(&self) {
        self.func.borrow_mut().pop();
    }

    // current function
    fn func(&self) -> C {
        *self.func.borrow().last().unwrap()
    }

    fn parent_vec(&self) -> Vec<C> {
        self.func.borrow().clone()
    }
}

// basic tracking
impl<'s, C: Code> CTracer<'s, C> {
    fn track_enter(&self, span: Span<'s>) {
        self.track.borrow_mut().push(Track::Enter(EnterTrack {
            func: self.func(),
            span,
            parents: self.parent_vec(),
        }));
    }

    fn track_step(&self, step: &'static str, span: Span<'s>) {
        self.track.borrow_mut().push(Track::Step(StepTrack {
            func: self.func(),
            step,
            span,
            parents: self.parent_vec(),
        }));
    }

    fn track_debug(&self, dbg: String) {
        self.track.borrow_mut().push(Track::Debug(DebugTrack {
            func: self.func(),
            dbg,
            parents: self.parent_vec(),
            _phantom: Default::default(),
        }));
    }

    fn track_suggest(&self, usage: Usage, suggest: Vec<Suggest<'s, C>>) {
        if !suggest.is_empty() {
            self.track.borrow_mut().push(Track::Suggest(SuggestTrack {
                func: self.func(),
                usage,
                list: suggest,
                parents: self.parent_vec(),
            }));
        }
    }

    fn track_expect(&self, usage: Usage, expect: Vec<Expect<'s, C>>) {
        if !expect.is_empty() {
            self.track.borrow_mut().push(Track::Expect(ExpectTrack {
                func: self.func(),
                usage,
                list: expect,
                parents: self.parent_vec(),
            }));
        }
    }

    fn track_ok(&self, rest: Span<'s>, span: Span<'s>) {
        self.track.borrow_mut().push(Track::Ok(OkTrack {
            func: self.func(),
            span,
            rest,
            parents: self.parent_vec(),
        }));
    }

    fn track_error(&self, err: &ParserError<'s, C>) {
        self.track.borrow_mut().push(Track::Err(ErrTrack {
            func: self.func(),
            span: err.span,
            err: err.to_string(),
            parents: self.parent_vec(),
        }));
    }

    fn track_exit(&self) {
        self.track.borrow_mut().push(Track::Exit(ExitTrack {
            func: self.func(),
            parents: self.parent_vec(),
            _phantom: Default::default(),
        }));
    }
}

impl<'s, C: Code> Default for CTracer<'s, C> {
    fn default() -> Self {
        Self::new()
    }
}

// TrackParseResult ------------------------------------------------------

/// Helps with keeping tracks in the parsers.
///
/// This can be squeezed between the call to another parser and the ?-operator.
///
/// Makes sure the tracer can keep track of the complete parse call tree.
pub trait TrackParseResult<'s, 't, C: Code, O> {
    type Result;

    /// Translates the error code and adds the standard expect value.
    /// Then tracks the error and marks the current function as finished.
    fn track(self, trace: &'t impl Tracer<'s, C>) -> Self::Result;
}

impl<'s, 't, O, C: Code> TrackParseResult<'s, 't, C, O> for ParserResult<'s, C, O> {
    type Result = Self;

    fn track(self, trace: &'t impl Tracer<'s, C>) -> Self::Result {
        match self {
            Ok(_) => self,
            Err(e) => trace.err(e),
        }
    }
}

impl<'s, 't, C: Code> TrackParseResult<'s, 't, C, Span<'s>>
    for Result<(Span<'s>, Span<'s>), nom::Err<ParserError<'s, C>>>
{
    type Result = Result<(Span<'s>, Span<'s>), ParserError<'s, C>>;

    fn track(self, trace: &'t impl Tracer<'s, C>) -> Self::Result {
        match self {
            Ok(v) => Ok(v),
            Err(nom::Err::Error(e)) => trace.err(e),
            Err(nom::Err::Failure(e)) => trace.err(e),
            Err(nom::Err::Incomplete(_)) => unreachable!(),
        }
    }
}

// pub trait TrackParseResult<'s, 't, O, C: Code> {
//     type Result;
//
//     /// Translates the error code and adds the standard expect value.
//     /// Then tracks the error and marks the current function as finished.
//     fn track(self, trace: &'t impl Tracer<'s, C>) -> Self::Result;
// }
//
// impl<'s, 't, C: Code> TrackParseResult<'s, 't, Span<'s>, C>
//     for Result<(Span<'s>, Span<'s>), nom::Err<ParserError<'s, C>>>
// {
//     type Result = Result<(Span<'s>, Span<'s>), ParserError<'s, C>>;
//
//     fn track(self, trace: &'t impl Tracer<'s, C>) -> Self::Result {
//         match self {
//             Ok(v) => Ok(v),
//             Err(nom::Err::Error(e)) => trace.err(e),
//             Err(nom::Err::Failure(e)) => trace.err(e),
//             Err(nom::Err::Incomplete(_)) => unreachable!(),
//         }
//     }
// }

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

/// One per stack frame.
pub struct ExpectTrack<'s, C: Code> {
    /// Function.
    pub func: C,
    /// Usage flag.
    pub usage: Usage,
    /// Collected Expect values.
    pub list: Vec<Expect<'s, C>>,
    /// Parser call stack.
    pub parents: Vec<C>,
}

/// One per stack frame.
pub struct SuggestTrack<'s, C: Code> {
    /// Function
    pub func: C,
    /// Usage flag.
    pub usage: Usage,
    /// Collected Suggest values.
    pub list: Vec<Suggest<'s, C>>,
    /// Parser call stack.
    pub parents: Vec<C>,
}

/// Track for entering a parser function.
pub struct EnterTrack<'s, C> {
    /// Function
    pub func: C,
    /// Span
    pub span: Span<'s>,
    /// Parser call stack.
    pub parents: Vec<C>,
}

/// Track for step information.
pub struct StepTrack<'s, C> {
    /// Function
    pub func: C,
    /// Step info.
    pub step: &'static str,
    /// Span
    pub span: Span<'s>,
    /// Parser call stack.
    pub parents: Vec<C>,
}

/// Track for debug information.
pub struct DebugTrack<'s, C> {
    /// Function.
    pub func: C,
    /// Debug info. TODO: Is the string necessary?
    pub dbg: String,
    /// Parser call stack.
    pub parents: Vec<C>,
    /// For the lifetime ...
    pub _phantom: PhantomData<Span<'s>>,
}

/// Track for ok results.
pub struct OkTrack<'s, C> {
    /// Function.
    pub func: C,
    /// Span.
    pub span: Span<'s>,
    /// Remaining span.
    pub rest: Span<'s>,
    /// Parser call stack.
    pub parents: Vec<C>,
}

/// Track for err results.
pub struct ErrTrack<'s, C> {
    /// Function.
    pub func: C,
    /// Span.
    pub span: Span<'s>,
    /// Error message.
    pub err: String, // TODO: check
    /// Parser call stack.
    pub parents: Vec<C>,
}

/// Track for exiting a parser function.
pub struct ExitTrack<'s, C> {
    /// Function
    pub func: C,
    /// Parser call stack.
    pub parents: Vec<C>,
    /// For the lifetime ...
    pub _phantom: PhantomData<Span<'s>>,
}

/// One track of the parsing trace.
#[allow(missing_docs)]
pub enum Track<'s, C: Code> {
    Enter(EnterTrack<'s, C>),
    Step(StepTrack<'s, C>),
    Debug(DebugTrack<'s, C>),
    Expect(ExpectTrack<'s, C>),
    Suggest(SuggestTrack<'s, C>),
    Ok(OkTrack<'s, C>),
    Err(ErrTrack<'s, C>),
    Exit(ExitTrack<'s, C>),
}

impl<'s, C: Code> Track<'s, C> {
    /// Returns the func value for each branch.
    pub fn func(&self) -> C {
        match self {
            Track::Enter(v) => v.func,
            Track::Step(v) => v.func,
            Track::Debug(v) => v.func,
            Track::Expect(v) => v.func,
            Track::Suggest(v) => v.func,
            Track::Ok(v) => v.func,
            Track::Err(v) => v.func,
            Track::Exit(v) => v.func,
        }
    }

    /// Returns the parser call stack for each branch.
    pub fn parents(&self) -> &Vec<C> {
        match self {
            Track::Enter(v) => &v.parents,
            Track::Step(v) => &v.parents,
            Track::Debug(v) => &v.parents,
            Track::Expect(v) => &v.parents,
            Track::Suggest(v) => &v.parents,
            Track::Ok(v) => &v.parents,
            Track::Err(v) => &v.parents,
            Track::Exit(v) => &v.parents,
        }
    }
}
