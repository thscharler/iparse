use crate::debug::tracer::debug_tracer;
use crate::error::{DebugWidth, Expect, Hints, ParserError, Suggest};
use crate::{Code, FilterFn, ParserResult, Span, Tracer};
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::{fmt, mem};

/// Tracing and error collection.
pub struct CTracer<'s, C: Code, const TRACK: bool = true> {
    /// Function call stack.
    pub(crate) func: Vec<C>,

    /// Collected tracks.
    pub(crate) track: Vec<Track<'s, C>>,
    /// Result data.
    pub(crate) suggest: Vec<SuggestTrack<'s, C>>,
    /// Result data.
    pub(crate) expect: Vec<ExpectTrack<'s, C>>,
}

impl<'s, C: Code, const TRACK: bool> Tracer<'s, C> for CTracer<'s, C, TRACK> {
    /// New one.
    fn new() -> Self {
        Self {
            func: Vec::new(),
            track: Vec::new(),
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
            // if !err.code.is_special() {
            //     self.add_expect(err.code, err.span);
            // } else {
            //     self.add_expect(self.func(), err.span);
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
impl<'s, C: Code, const TRACK: bool> CTracer<'s, C, TRACK> {
    /// Write a debug output of the Tracer state.
    pub fn write(
        &self,
        out: &mut impl fmt::Write,
        w: DebugWidth,
        filter: FilterFn<'_, C>,
    ) -> fmt::Result {
        debug_tracer(out, w, self, filter)
    }

    pub fn to_results(&mut self) -> (Vec<Expect<'s, C>>, Vec<Suggest<'s, C>>) {
        (self.to_expect(), self.to_suggest())
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
impl<'s, C: Code, const TRACK: bool> CTracer<'s, C, TRACK> {
    fn push_expect(&mut self, func: C) {
        let parent = self.parent_vec().clone();
        self.expect.push(ExpectTrack {
            func,
            usage: Usage::Track,
            list: Vec::new(),
            parents: parent,
        })
    }

    fn pop_expect(&mut self) -> ExpectTrack<'s, C> {
        self.expect.pop().expect("Vec<Expect> is empty")
    }

    fn add_expect(&mut self, code: C, span: Span<'s>) {
        let parent = self.parent_vec().clone();
        self.track_expect_single(Usage::Track, code, span);
        self.expect
            .last_mut()
            .expect("Vec<Expect> is empty")
            .list
            .push(Expect {
                code,
                span,
                parents: parent,
            })
    }
}

// suggest
impl<'s, C: Code, const TRACK: bool> CTracer<'s, C, TRACK> {
    fn push_suggest(&mut self, func: C) {
        let parent = self.parent_vec().clone();
        self.suggest.push(SuggestTrack {
            func,
            usage: Usage::Track,
            list: Vec::new(),
            parents: parent,
        })
    }

    fn pop_suggest(&mut self) -> SuggestTrack<'s, C> {
        self.suggest.pop().expect("Vec<Suggest> is empty")
    }

    fn add_suggest(&mut self, code: C, span: Span<'s>) {
        let parent = self.parent_vec().clone();
        self.suggest
            .last_mut()
            .expect("Vec<Suggest> is empty")
            .list
            .push(Suggest {
                code,
                span,
                parents: parent,
            })
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
impl<'s, C: Code, const TRACK: bool> CTracer<'s, C, TRACK> {
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

    fn parent_vec(&self) -> &Vec<C> {
        &self.func
    }
}

// basic tracking
impl<'s, C: Code, const TRACK: bool> CTracer<'s, C, TRACK> {
    fn track_enter(&mut self, span: Span<'s>) {
        if TRACK {
            let parent = self.parent_vec().clone();
            self.track.push(Track::Enter(EnterTrack {
                func: self.func(),
                span,
                parents: parent,
            }));
        }
    }

    fn track_step(&mut self, step: &'static str, span: Span<'s>) {
        if TRACK {
            let parent = self.parent_vec().clone();
            self.track.push(Track::Step(StepTrack {
                func: self.func(),
                step,
                span,
                parents: parent,
            }));
        }
    }

    fn track_debug(&mut self, dbg: String) {
        if TRACK {
            let parent = self.parent_vec().clone();
            self.track.push(Track::Debug(DebugTrack {
                func: self.func(),
                dbg,
                parents: parent,
                _phantom: Default::default(),
            }));
        }
    }

    fn track_suggest(&mut self, usage: Usage, suggest: Cow<Vec<Suggest<'s, C>>>) {
        if TRACK {
            if !suggest.is_empty() {
                let parent = self.parent_vec().clone();
                self.track.push(Track::Suggest(SuggestTrack {
                    func: self.func(),
                    usage,
                    list: suggest.into_owned(),
                    parents: parent,
                }));
            }
        }
    }

    fn track_expect_single(&mut self, usage: Usage, code: C, span: Span<'s>) {
        if TRACK {
            let parent = self.parent_vec().clone();
            self.track.push(Track::Expect(ExpectTrack {
                func: self.func(),
                usage,
                list: vec![Expect {
                    code,
                    span,
                    parents: vec![],
                }],
                parents: parent,
            }));
        }
    }

    fn track_expect(&mut self, usage: Usage, expect: Cow<Vec<Expect<'s, C>>>) {
        if TRACK {
            if !expect.is_empty() {
                let parent = self.parent_vec().clone();
                self.track.push(Track::Expect(ExpectTrack {
                    func: self.func(),
                    usage,
                    list: expect.into_owned(),
                    parents: parent,
                }));
            }
        }
    }

    fn track_ok(&mut self, rest: Span<'s>, span: Span<'s>) {
        if TRACK {
            let parent = self.parent_vec().clone();
            self.track.push(Track::Ok(OkTrack {
                func: self.func(),
                span,
                rest,
                parents: parent,
            }));
        }
    }

    fn track_error(&mut self, err: &ParserError<'s, C>) {
        if TRACK {
            let parent = self.parent_vec().clone();
            self.track.push(Track::Err(ErrTrack {
                func: self.func(),
                span: err.span,
                err: err.to_string(),
                parents: parent,
            }));
        }
    }

    fn track_exit(&mut self) {
        if TRACK {
            let parent = self.parent_vec().clone();
            self.track.push(Track::Exit(ExitTrack {
                func: self.func(),
                parents: parent,
                _phantom: Default::default(),
            }));
        }
    }
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
