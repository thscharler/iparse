use crate::debug::rtracer::debug_rtracer;
use crate::error::{DebugWidth, Expect, ParserError, Suggest};
use crate::{Code, ParserResult, Span, Tracer};
use std::cell::RefCell;
use std::fmt;
use std::fmt::{Debug, Display};

/// Tracing and error collection.
pub struct RTracer<'s, C: Code> {
    pub(crate) func: RefCell<Vec<C>>,
    pub(crate) suggest: RefCell<Vec<SuggestTrack<'s, C>>>,
    pub(crate) expect: RefCell<Vec<ExpectTrack<'s, C>>>,
}

impl<'s, C: Code> Tracer<'s, C> for RTracer<'s, C> {
    /// New one.
    fn new() -> Self {
        Self {
            func: RefCell::new(Vec::new()),
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
        // self.debug(format!(
        //     "suggest {}:\"{}\" ...",
        //     suggest,
        //     restrict(DebugWidth::Medium, span)
        // ));
        self.add_suggest(suggest, span);
    }

    /// Keep track of this error.
    fn stash(&self, err: ParserError<'s, C>) {
        // self.debug(format!(
        //     "expect {}:\"{}\" ...",
        //     err.code,
        //     restrict(DebugWidth::Medium, err.span)
        // ));
        self.add_expect(err.code, err.span);
        self.append_expect(err.expect);
        self.append_suggest(err.suggest);
    }

    /// Write a track for an ok result.
    fn ok<'t, T>(
        &'t self,
        rest: Span<'s>,
        span: Span<'s>,
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
            // special codes are not very usefull in this position.
            if !err.code.is_special() {
                self.add_expect(err.code, err.span);
            } else {
                self.add_expect(self.func(), err.span);
            }
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
}

// output
impl<'s, C: Code> RTracer<'s, C> {
    /// Write a debug output of the Tracer state.
    pub fn write(&self, out: &mut impl fmt::Write, w: DebugWidth) -> fmt::Result {
        debug_rtracer(out, w, self)
    }
}

// expect
impl<'s, C: Code> RTracer<'s, C> {
    fn push_expect(&self, func: C) {
        self.expect.borrow_mut().push(ExpectTrack {
            func,
            usage: Usage::Track,
            list: Vec::new(),
            parents: self.parent_vec(), //TODO?
        })
    }

    fn pop_expect(&self) -> ExpectTrack<'s, C> {
        self.expect
            .borrow_mut()
            .pop()
            .expect("Vec<Expect> is empty")
    }

    fn add_expect(&self, code: C, span: Span<'s>) {
        self.expect
            .borrow_mut()
            .last_mut()
            .expect("Vec<Expect> is empty")
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
            .expect("Vec<Expect> is empty")
            .list
            .append(&mut expect);
    }
}

// suggest
impl<'s, C: Code> RTracer<'s, C> {
    fn push_suggest(&self, func: C) {
        self.suggest.borrow_mut().push(SuggestTrack {
            func,
            usage: Usage::Track,
            list: Vec::new(),
            parents: self.parent_vec(), //TODO:?
        })
    }

    fn pop_suggest(&self) -> SuggestTrack<'s, C> {
        self.suggest
            .borrow_mut()
            .pop()
            .expect("Vec<Suggest> is empty")
    }

    fn add_suggest(&self, code: C, span: Span<'s>) {
        self.suggest
            .borrow_mut()
            .last_mut()
            .expect("Vec<Suggest> is empty")
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
            .expect("Vec<Suggest> is empty")
            .list
            .append(&mut suggest);
    }
}

// call frame tracking
impl<'s, C: Code> RTracer<'s, C> {
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
        *self
            .func
            .borrow()
            .last()
            .expect("Vec<FnCode> is empty. forgot to trace.enter()")
    }

    fn parent_vec(&self) -> Vec<C> {
        self.func.borrow().clone()
    }
}

// basic tracking
impl<'s, C: Code> RTracer<'s, C> {
    fn track_enter(&self, _span: Span<'s>) {}

    fn track_step(&self, _step: &'static str, _span: Span<'s>) {}

    fn track_debug(&self, _dbg: String) {}

    fn track_suggest(&self, _usage: Usage, _suggest: Vec<Suggest<'s, C>>) {}

    fn track_expect(&self, _usage: Usage, _expect: Vec<Expect<'s, C>>) {}

    fn track_ok(&self, _rest: Span<'s>, _span: Span<'s>) {}

    fn track_error(&self, _err: &ParserError<'s, C>) {}

    fn track_exit(&self) {}
}

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

/// One track of the parsing trace.
#[allow(missing_docs)]
pub enum Track<'s, C: Code> {
    Expect(ExpectTrack<'s, C>),
    Suggest(SuggestTrack<'s, C>),
}

impl<'s, C: Code> Track<'s, C> {
    /// Returns the func value for each branch.
    pub fn func(&self) -> C {
        match self {
            Track::Expect(v) => v.func,
            Track::Suggest(v) => v.func,
        }
    }

    /// Returns the parser call stack for each branch.
    pub fn parents(&self) -> &Vec<C> {
        match self {
            Track::Expect(v) => &v.parents,
            Track::Suggest(v) => &v.parents,
        }
    }
}
