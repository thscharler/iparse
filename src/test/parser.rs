use crate::error::{DebugWidth, ParserError};
use crate::test::{Report, Test, TestFail, TestSpan, TestSpanPair};
use crate::tracer::{CTracer, Track};
use crate::{Code, ParseResult, Span, Tracer};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::fmt::Debug;
use std::time::Instant;

/// Parser function.
pub type ParserFn<'s, O, C> = fn(&'_ CTracer<'s, C>, Span<'s>) -> ParseResult<'s, O, C>;

/// Extra data for the parser fn.
pub struct TestTracer<'s, C: Code> {
    pub trace: CTracer<'s, C>,
    pub trace_filter: RefCell<&'s dyn Fn(&Track<'_, C>) -> bool>,
}

// matches a ParserFn
impl<'s, O, C> Test<TestTracer<'s, C>, Span<'s>, (Span<'s>, O), ParserError<'s, C>>
where
    O: Debug,
    C: Code,
{
    /// Runs the parser and records the results.
    /// Use ok(), err(), ... to check specifics.
    ///
    /// Finish the test with q().
    #[must_use]
    pub fn parse(span: &'s str, fn_test: ParserFn<'s, O, C>) -> Self {
        let span = Span::new(span);
        let trace = CTracer::new();

        let now = Instant::now();
        let result = fn_test(&trace, span);
        let elapsed = now.elapsed();

        Self {
            x: TestTracer {
                trace,
                trace_filter: RefCell::new(&|_| true),
            },
            span,
            result,
            duration: elapsed,
            fail: Cell::new(false),
        }
    }

    /// Sets a filter on the trace.
    #[must_use]
    pub fn filter(&self, filter: &'s dyn Fn(&Track<'_, C>) -> bool) -> &Self {
        self.x.trace_filter.replace(filter);
        self
    }
}

/// Dumps the full parser trace if any test failed.
pub struct CheckTrace;

impl<'s, O, C, E> Report<TestTracer<'s, C>, Span<'s>, (Span<'s>, O), E> for CheckTrace
where
    E: Debug,
    O: Debug,
    C: Code,
{
    fn report(testn: &Test<TestTracer<'s, C>, Span<'s>, (Span<'s>, O), E>) {
        if testn.fail.get() {
            trace(testn);
            panic!()
        }
    }
}

/// Dumps the full parser trace.
pub struct Trace;

impl<'s, O, C, E> Report<TestTracer<'s, C>, Span<'s>, (Span<'s>, O), E> for Trace
where
    E: Debug,
    O: Debug,
    C: Code,
{
    fn report(testn: &Test<TestTracer<'s, C>, Span<'s>, (Span<'s>, O), E>) {
        trace(testn);
    }
}

fn trace<'s, O, C, E>(testn: &Test<TestTracer<'s, C>, Span<'s>, (Span<'s>, O), E>)
where
    O: Debug,
    E: Debug,
    C: Code,
{
    struct TracerDebug<'a, 'b, 's, C: Code> {
        trace: &'a CTracer<'s, C>,
        track_filter: &'b dyn Fn(&Track<'s, C>) -> bool,
    }

    impl<'a, 'b, 's, C: Code> Debug for TracerDebug<'a, 'b, 's, C> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.trace.write(f, DebugWidth::Medium, self.track_filter)
        }
    }

    println!();
    println!(
        "when parsing '{}' in {}ns =>",
        testn.span,
        testn.duration.as_nanos()
    );

    let trace = &testn.x.trace;
    let track_filter_r = testn.x.trace_filter.borrow();
    let track_filter = &*track_filter_r;

    println!(
        "{:?}",
        TracerDebug {
            trace,
            track_filter
        }
    );

    match &testn.result {
        Ok((rest, token)) => {
            println!("rest {}:\"{}\"", rest.location_offset(), rest);
            println!("{:0?}", token);
        }
        Err(e) => {
            println!("error");
            println!("{:1?}", e);
        }
    }
}

impl<'a, C: Code> TestSpan for ParseResult<'a, Span<'a>, C> {
    /// Test for fn that return a ParseResult.
    #[track_caller]
    fn ok(&self, offset: usize, fragment: &str) -> &Self {
        match self {
            Ok((_rest, token)) => {
                token.ok(offset, fragment);
            }
            Err(e) => {
                println!("{:?}", e);
                assert!(false);
            }
        }
        self
    }
}

impl<'a, C: Code> TestSpanPair for ParseResult<'a, (Option<Span<'a>>, Span<'a>), C> {
    /// Test for fn that return a ParseResult containing a (Option<Span>, Span).
    #[track_caller]
    fn ok_0(&self, offset: usize, fragment: &str) -> &Self {
        match self {
            Ok((_, (test, _))) => {
                if let Some(test) = test {
                    test.ok(offset, fragment);
                } else {
                    println!("Was None, should be {} '{}'", offset, fragment);
                    assert!(false);
                }
            }
            Err(e) => {
                println!("{:?}", e);
                assert!(false);
            }
        }
        self
    }

    /// Test for fn that return a ParseResult containing a (Option<Span>, Span).
    #[track_caller]
    fn ok_0_isnone(&self) -> &Self {
        match self {
            Ok((_, (test, _))) => {
                if let Some(test) = test {
                    println!(
                        "Was something {} '{}', should be None",
                        test.location_offset(),
                        test.fragment()
                    );
                }
            }
            Err(e) => {
                println!("{:?}", e);
                assert!(false);
            }
        }
        self
    }

    /// Test for fn that return a ParseResult containing a (Option<Span>, Span).
    #[track_caller]
    fn ok_1(&self, offset: usize, fragment: &str) -> &Self {
        match self {
            Ok((_, (_, test))) => {
                test.ok(offset, fragment);
            }
            Err(e) => {
                println!("{:?}", e);
                assert!(false);
            }
        }
        self
    }
}

impl<'a, C: Code> TestFail<C> for ParseResult<'a, Span<'a>, C> {
    #[track_caller]
    fn err(&self, kind: C) -> &Self {
        match self {
            Ok((rest, token)) => {
                println!("Ok, but should have failed:");
                println!("    rest='{}' token='{}'", rest, token);
                assert!(false);
            }
            Err(e) if e.code == C::NOM_ERROR => {
                println!("Failed with ErrNomError. To unspecified.");
                println!("{:?}", e);
                assert!(false);
            }
            Err(e) if e.code == C::NOM_FAILURE => {
                println!("Failed with ErrNomFailure.");
                println!("{:?}", e);
                assert!(false);
            }
            Err(e) => {
                if e.code != kind {
                    println!("Failed with the wrong ErrorKind:");
                    println!("    '{}' => result={} <> kind={:?}", e.span, e, kind);
                    assert!(false);
                }
            }
        }
        self
    }

    #[track_caller]
    fn dump(&self) -> &Self {
        match self {
            Ok(v) => {
                println!("Always fail: {:?}", v);
            }
            Err(e) => {
                println!("Always fail: {:?}", e);
            }
        }
        self
    }
}

impl<'a, C: Code> TestFail<C> for ParseResult<'a, (Option<Span<'a>>, Span<'a>), C> {
    #[track_caller]
    fn err(&self, kind: C) -> &Self {
        match self {
            Ok((rest, token)) => {
                println!("Ok, but should have failed:");
                println!("    rest='{}' token='{:?}'", rest, token);
                assert!(false);
            }
            Err(e) if e.code == C::NOM_ERROR => {
                println!("Failed with ErrNomError. To unspecified.");
                println!("{:?}", e);
                assert!(false);
            }
            Err(e) if e.code == C::NOM_FAILURE => {
                println!("Failed with ErrNomFailure.");
                println!("{:?}", e);
                assert!(false);
            }
            Err(e) => {
                if e.code != kind {
                    println!("Failed with the wrong ErrorKind:");
                    println!("    '{}' => result={} <> kind={:?}", e.span, e, kind);
                    assert!(false);
                }
            }
        }
        self
    }

    #[track_caller]
    fn dump(&self) -> &Self {
        match self {
            Ok(v) => {
                println!("Always fail: {:?}", v);
            }
            Err(e) => {
                println!("Always fail: {:?}", e);
            }
        }
        self
    }
}
