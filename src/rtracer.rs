use crate::debug::rtracer::debug_rtracer;
use crate::error::{DebugWidth, Expect, Hints, ParserError, Suggest};
use crate::{Code, ParserResult, Span, Tracer};
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::{fmt, mem};

/// Tracing and error collection.
pub struct RTracer<'s, C: Code> {
    pub(crate) func: Vec<C>,

    pub(crate) suggest: Vec<SuggestTrack<'s, C>>,
    pub(crate) expect: Vec<ExpectTrack<'s, C>>,
}

impl<'s, C: Code> Tracer<'s, C> for RTracer<'s, C> {
    /// New one.
    fn new() -> Self {
        Self {
            func: Vec::new(),
            suggest: Vec::new(),
            expect: Vec::new(),
        }
    }

    /// Enter a parser function. Absolutely necessary for the rest.
    fn enter(&mut self, func: C, span: Span<'s>) {
        self.push_func(func);
        self.push_suggest(func);
        self.push_expect(func);

        self.track_enter(span);
    }

    /// Keep track of steps in a complicated parser.
    fn step(&mut self, step: &'static str, span: Span<'s>) {
        self.track_step(step, span);
    }

    /// Some detailed debug information.
    fn debug<T: Into<String>>(&mut self, step: T) {
        self.track_debug(step.into());
    }

    /// Adds a suggestion for the current stack frame.
    fn suggest(&mut self, suggest: C, span: Span<'s>) {
        self.add_suggest(suggest, span);
    }

    /// Keep track of this error.
    fn stash(&mut self, err: ParserError<'s, C>) {
        self.add_expect(err.code, err.span);

        let expect_vec = &mut self.expect.last_mut().expect("Vec<Expect> is empty").list;
        let suggest_vec = &mut self.suggest.last_mut().expect("Vec<Suggest> is empty").list;

        for hint in err.hints.into_iter() {
            match hint {
                Hints::Nom(_) => {}
                Hints::Suggest(v) => {
                    suggest_vec.push(v);
                }
                Hints::Expect(v) => {
                    expect_vec.push(v);
                }
            }
        }
    }

    /// Write a track for an ok result.
    fn ok<'t, T>(
        &'t mut self,
        rest: Span<'s>,
        span: Span<'s>,
        val: T,
    ) -> ParserResult<'s, C, (Span<'s>, T)> {
        self.track_ok(rest, span);

        let expect = self.pop_expect();
        self.track_expect(Usage::Drop, Cow::Owned(expect.list));
        let suggest = self.pop_suggest();
        // Keep suggests, sort them out later.
        // Drop at the toplevel if no error occurs?
        if !self.suggest.is_empty() {
            self.append_suggest(suggest.list);
        } else {
            self.suggest.push(suggest);
        }

        self.track_exit();
        self.pop_func();

        Ok((rest, val))
    }

    /// Write a track for an error.
    fn err<'t, T>(&'t mut self, mut err: ParserError<'s, C>) -> ParserResult<'s, C, T> {
        // Freshly created error needs to be recorded before we overwrite the code.
        if !err.tracing {
            err.tracing = true;
            // ??? do we really need this anymore. now the code is no longer overwritten,
            // so it ought not be necessary to build up expects.
            // should be at the users digression by using stash.
            // and for mapping external errors it may be better to
            // let the user handle that too. ???

            // special codes are not very usefull in this position.
            // if err.code.is_special() {
            //     self.add_expect(self.func(), err.span);
            // } else {
            //     self.add_expect(err.code, err.span);
            // }
        }

        // when backtracking we always replace the current error code.
        // conclusion: this is useless.
        // err.code = self.func();

        let exp = self.pop_expect();
        self.track_expect(Usage::Use, Cow::Borrowed(&exp.list));
        err.append_expect(exp.list);

        let sug = self.pop_suggest();
        self.track_suggest(Usage::Use, Cow::Borrowed(&sug.list));
        err.append_suggest(sug.list);

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

    pub fn to_expect(&mut self) -> Vec<Expect<'s, C>> {
        mem::replace(&mut self.expect, Vec::new())
            .into_iter()
            .flat_map(|v| v.list.into_iter())
            .collect()
    }

    pub fn to_suggest(&mut self) -> Vec<Suggest<'s, C>> {
        mem::replace(&mut self.suggest, Vec::new())
            .into_iter()
            .flat_map(|v| v.list.into_iter())
            .collect()
    }
}

// expect
impl<'s, C: Code> RTracer<'s, C> {
    fn push_expect(&mut self, func: C) {
        self.expect.push(ExpectTrack {
            func,
            usage: Usage::Track,
            list: Vec::new(),
        })
    }

    fn pop_expect(&mut self) -> ExpectTrack<'s, C> {
        self.expect.pop().expect("Vec<Expect> is empty")
    }

    fn add_expect(&mut self, code: C, span: Span<'s>) {
        self.track_expect_single(Usage::Track, code, span);
        self.expect
            .last_mut()
            .expect("Vec<Expect> is empty")
            .list
            .push(Expect { code, span })
    }
}

// suggest
impl<'s, C: Code> RTracer<'s, C> {
    fn push_suggest(&mut self, func: C) {
        self.suggest.push(SuggestTrack {
            func,
            usage: Usage::Track,
            list: Vec::new(),
        })
    }

    fn pop_suggest(&mut self) -> SuggestTrack<'s, C> {
        self.suggest.pop().expect("Vec<Suggest> is empty")
    }

    fn add_suggest(&mut self, code: C, span: Span<'s>) {
        self.suggest
            .last_mut()
            .expect("Vec<Suggest> is empty")
            .list
            .push(Suggest { code, span })
    }

    fn append_suggest(&mut self, mut suggest: Vec<Suggest<'s, C>>) {
        self.suggest
            .last_mut()
            .expect("Vec<Suggest> is empty")
            .list
            .append(&mut suggest);
    }
}

// call frame tracking
impl<'s, C: Code> RTracer<'s, C> {
    // enter function
    fn push_func(&mut self, func: C) {
        self.func.push(func);
    }

    // leave current function
    fn pop_func(&mut self) {
        self.func.pop();
    }

    // current function
    fn func(&self) -> C {
        *self
            .func
            .last()
            .expect("Vec<FnCode> is empty. forgot to trace.enter()")
    }
}

// basic tracking
impl<'s, C: Code> RTracer<'s, C> {
    fn track_enter(&self, _span: Span<'s>) {}

    fn track_step(&self, _step: &'static str, _span: Span<'s>) {}

    fn track_debug(&self, _dbg: String) {}

    fn track_suggest(&self, _usage: Usage, _suggest: Cow<Vec<Suggest<'s, C>>>) {}

    fn track_expect(&self, _usage: Usage, _expect: Cow<Vec<Expect<'s, C>>>) {}

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
}

/// One per stack frame.
pub struct SuggestTrack<'s, C: Code> {
    /// Function
    pub func: C,
    /// Usage flag.
    pub usage: Usage,
    /// Collected Suggest values.
    pub list: Vec<Suggest<'s, C>>,
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
}
