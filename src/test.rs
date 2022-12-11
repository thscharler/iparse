use crate::debug::restrict;
use crate::error::{DebugWidth, ParserError};
use crate::notracer::NoTracer;
use crate::rtracer::RTracer;
use crate::tracer::CTracer;
use crate::{Code, FilterFn, ParserResult, Span, Tracer};
use ::nom::IResult;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;
use std::time::Instant;

/// Most general test fn.
pub type TestedFn<I, O, E> = fn(I) -> Result<O, E>;

/// Value comparison.
pub type CompareFn<O, V> = for<'a> fn(&'a O, V) -> bool;

/// Signature of a classic nom function for Test.
pub type NomFn<'s, O> = fn(Span<'s>) -> IResult<Span<'s>, O>;

/// Signature of a classic nom function for Test.
pub type NomFn2<'s, C, O> = fn(Span<'s>) -> Result<(Span<'s>, O), nom::Err<ParserError<'s, C>>>;

/// Tokenizer function.
pub type TokenFn<'s, O, C> = fn(Span<'s>) -> ParserResult<'s, C, (Span<'s>, O)>;

/// Signature of a parser function for Test.
pub type ParserFn<'s, O, C, const TRACK: bool> =
    fn(&'_ CTracer<'s, C, TRACK>, Span<'s>) -> ParserResult<'s, C, (Span<'s>, O)>;

/// Signature of a parser function for Test.
pub type RParserFn<'s, O, C> =
    fn(&'_ RTracer<'s, C>, Span<'s>) -> ParserResult<'s, C, (Span<'s>, O)>;

/// Signature of a parser function for Test.
pub type NoParserFn<'s, O, C> =
    fn(&'_ NoTracer<'s, C>, Span<'s>) -> ParserResult<'s, C, (Span<'s>, O)>;

/// Test runner.
pub struct Test<P, I, O, E>
where
    I: Debug,
    O: Debug,
    E: Debug,
{
    /// Extension data.
    pub x: P,

    /// Input span.
    pub span: I,

    /// Result
    pub result: Result<O, E>,
    /// Timer
    pub duration: Duration,

    /// Any test failed?
    pub fail: Cell<bool>,
}

/// Result reporting.
pub trait Report<T> {
    fn report(&self, test: &T);
}

/// Transform a test-fn so that it can take Option values.
///
/// '''
/// fn sheetname<'s>(result: &'s OFSheetName<'s>, test: &'s str) -> bool {
///     result.name == test
/// }
///
/// optional!(opt_sheetname(OFSheetName<'s>, &'s str), sheetname);
/// '''
///
#[allow(unused_macros)]
#[macro_export]
macro_rules! optional {
    ($name:ident( $O:ty, $V:ty ), $testfn:ident) => {
        fn $name<'s>(result: &'s Option<$O>, test: Option<$V>) -> bool {
            match result {
                None => match test {
                    None => true,
                    Some(_v) => false,
                },
                Some(o) => match test {
                    None => false,
                    Some(v) => {
                        if !$testfn(o, v) {
                            false
                        } else {
                            true
                        }
                    }
                },
            }
        }
    };
}

// General stuff ---------------------------------------------------------

/// Run a test for a nom parser.
/// Uses the default nom::error::Error
#[must_use]
pub fn test_nom<'s, T: Debug>(
    span: &'s str,
    fn_test: NomFn<'s, T>,
) -> Test<(), Span<'s>, (Span<'s>, T), nom::Err<nom::error::Error<Span<'s>>>> {
    let span: Span<'s> = span.into();

    let now = Instant::now();
    let result = fn_test(span.clone());
    let elapsed = now.elapsed();

    Test {
        x: (),
        span,
        result,
        duration: elapsed,
        fail: Cell::new(false),
    }
}

/// Run a test for a nom parser.
/// Uses ParserError as nom error.
#[must_use]
pub fn test_nom2<'s, C: Code, T: Debug>(
    span: &'s str,
    fn_test: NomFn2<'s, C, T>,
) -> Test<(), Span<'s>, (Span<'s>, T), nom::Err<ParserError<'s, C>>> {
    let span: Span<'s> = span.into();

    let now = Instant::now();
    let result = fn_test(span.clone());
    let elapsed = now.elapsed();

    Test {
        x: (),
        span,
        result,
        duration: elapsed,
        fail: Cell::new(false),
    }
}

/// Runs the tokenizer function and records the results.
/// Use ok(), err(), ... to check specifics.
///
/// Finish the test with q().
#[must_use]
pub fn test_token<'s, V: Debug, C: Code>(
    span: &'s str,
    fn_test: TokenFn<'s, V, C>,
) -> Test<(), Span<'s>, (Span<'s>, V), ParserError<'s, C>> {
    let span: Span<'s> = span.into();

    let now = Instant::now();
    let result = fn_test(span.clone());
    let elapsed = now.elapsed();

    Test {
        x: Default::default(),
        span,
        result,
        duration: elapsed,
        fail: Cell::new(false),
    }
}

/// Runs the parser and records the results.
/// Use ok(), err(), ... to check specifics.
///
/// Finish the test with q().
#[must_use]
pub fn test_parse<'a, 's, V: Debug, C: Code>(
    span: &'s str,
    fn_test: ParserFn<'s, V, C, true>,
) -> Test<TestTracer<'a, 's, C, true>, Span<'s>, (Span<'s>, V), ParserError<'s, C>> {
    let span = Span::new(span);
    let trace: CTracer<C, true> = CTracer::new();

    let now = Instant::now();
    let result = fn_test(&trace, span);
    let elapsed = now.elapsed();

    Test {
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

#[must_use]
pub fn test_parse_false<'a, 's, V: Debug, C: Code>(
    span: &'s str,
    fn_test: ParserFn<'s, V, C, false>,
) -> Test<TestTracer<'a, 's, C, false>, Span<'s>, (Span<'s>, V), ParserError<'s, C>> {
    let span = Span::new(span);

    let trace: CTracer<C, false> = CTracer::new();

    let now = Instant::now();
    let _ = fn_test(&trace, span);
    let elapsed = now.elapsed();
    let result = fn_test(&trace, span);

    Test {
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

/// Runs the parser and records the results.
/// Use ok(), err(), ... to check specifics.
///
/// Finish the test with q().
#[must_use]
pub fn test_rparse<'a, 's, V: Debug, C: Code>(
    span: &'s str,
    fn_test: RParserFn<'s, V, C>,
) -> Test<TestRTracer<'s, C>, Span<'s>, (Span<'s>, V), ParserError<'s, C>> {
    let span = Span::new(span);
    let trace = RTracer::new();

    let now = Instant::now();
    let _ = fn_test(&trace, span);
    let elapsed = now.elapsed();
    let result = fn_test(&trace, span);

    Test {
        x: TestRTracer { trace },
        span,
        result,
        duration: elapsed,
        fail: Cell::new(false),
    }
}

/// Runs the parser and records the results.
/// Use ok(), err(), ... to check specifics.
///
/// Finish the test with q().
#[must_use]
pub fn test_noparse<'a, 's, V: Debug, C: Code>(
    span: &'s str,
    fn_test: NoParserFn<'s, V, C>,
) -> Test<TestNoTracer<'s, C>, Span<'s>, (Span<'s>, V), ParserError<'s, C>> {
    let span = Span::new(span);
    let trace = NoTracer::new();

    let now = Instant::now();
    let _ = fn_test(&trace, span);
    let elapsed = now.elapsed();
    let result = fn_test(&trace, span);

    Test {
        x: TestNoTracer {
            _phantom: Default::default(),
        },
        span,
        result,
        duration: elapsed,
        fail: Cell::new(false),
    }
}

impl<P, I, O, E> Test<P, I, O, E>
where
    I: Clone + Debug,
    O: Debug,
    E: Debug,
{
    /// Sets the failed flag.
    pub fn flag_fail(&self) {
        self.fail.set(true);
    }

    /// Always fails.
    ///
    /// Finish the test with q().
    pub fn fail(&self) -> &Self {
        println!("FAIL: Unconditionally");
        self.flag_fail();
        self
    }

    /// Checks for ok.
    /// Any result that is not Err is ok.
    #[must_use]
    pub fn okok(&self) -> &Self {
        match &self.result {
            Ok(_) => {}
            Err(_) => {
                println!("FAIL: Expected ok, but was an error.");
                self.flag_fail();
            }
        }
        self
    }

    /// Checks for any error.
    ///
    /// Finish the test with q()
    #[must_use]
    pub fn errerr(&self) -> &Self {
        match &self.result {
            Ok(_) => {
                println!("FAIL: Expected error, but was ok!");
                self.flag_fail();
            }
            Err(_) => {}
        }
        self
    }

    /// Runs the associated Report. Depending on the type of the Report this
    /// can panic if any of the tests signaled a failure condition.
    ///
    /// Panic
    ///
    /// Panics if any test failed.
    #[track_caller]
    pub fn q(&self, r: &dyn Report<Self>) {
        r.report(self);
    }
}

// works for any fn that uses a Span as input and returns a (Span, X) pair.
impl<'s, P, O, E> Test<P, Span<'s>, (Span<'s>, O), E>
where
    O: Debug,
    E: Debug,
{
    /// Checks for ok.
    /// Uses an extraction function to get the relevant result.
    ///
    /// Finish the test with q()
    #[must_use]
    pub fn ok<V>(&'s self, eq: CompareFn<O, V>, test: V) -> &Self
    where
        V: Debug + Copy,
        O: Debug,
    {
        match &self.result {
            Ok((_, token)) => {
                if !eq(token, test) {
                    println!("FAIL: Value mismatch: {:?} <> {:?}", token, test);
                    self.flag_fail();
                }
            }
            Err(_) => {
                println!("FAIL: Expect ok, but was an error!");
                self.flag_fail();
            }
        }
        self
    }

    /// Tests the remaining string after parsing.
    ///
    /// Finish the test with q()
    #[must_use]
    pub fn rest(&self, test: &str) -> &Self {
        match &self.result {
            Ok((rest, _)) => {
                if **rest != test {
                    println!(
                        "FAIL: Rest mismatch {} <> {}",
                        restrict(DebugWidth::Medium, *rest),
                        test
                    );
                    self.flag_fail();
                }
            }
            Err(_) => {
                println!("FAIL: Expect ok, but was an error!");
                self.flag_fail();
            }
        }
        self
    }
}

// Span based ------------------------------------------------------------

/// Compare with an Ok(Span<'s>)
#[allow(clippy::needless_lifetimes)]
#[allow(dead_code)]
pub fn span<'a, 'b, 's>(span: &'a Span<'s>, value: (usize, &'b str)) -> bool {
    **span == value.1 && span.location_offset() == value.0
}

/// Compare with an Ok(Option<Span<'s>>, Span<'s>). Use the first span, fail on None.
#[allow(clippy::needless_lifetimes)]
#[allow(dead_code)]
pub fn span_0<'a, 'b, 's>(span: &'a (Option<Span<'s>>, Span<'s>), value: (usize, &'b str)) -> bool {
    if let Some(span) = &span.0 {
        **span == value.1 && span.location_offset() == value.0
    } else {
        false
    }
}

/// Compare with an Ok(Option<Span<'s>>, Span<'s>). Use the first span, fail on Some.
#[allow(clippy::needless_lifetimes)]
#[allow(dead_code)]
pub fn span_0_isnone<'a, 's>(span: &'a (Option<Span<'s>>, Span<'s>), _value: ()) -> bool {
    span.0.is_none()
}

/// Compare with an Ok(Option<Span<'s>>, Span<'s>). Use the second span.
#[allow(clippy::needless_lifetimes)]
#[allow(dead_code)]
pub fn span_1<'a, 'b, 's>(span: &'a (Option<Span<'s>>, Span<'s>), value: (usize, &'b str)) -> bool {
    *span.1 == value.1 && span.1.location_offset() == value.0
}

// Nom  ------------------------------------------------------------------

// works for any NomFn.
impl<'s, P, O> Test<P, Span<'s>, (Span<'s>, O), nom::Err<nom::error::Error<Span<'s>>>>
where
    O: Debug,
{
    /// Test for a nom error that occurred.
    #[must_use]
    pub fn err(&self, kind: nom::error::ErrorKind) -> &Self {
        match &self.result {
            Ok(_) => {
                println!("FAIL: Expected error, but was ok!");
                self.flag_fail();
            }
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                if e.code != kind {
                    println!("FAIL: {:?} <> {:?}", e.code, kind);
                    self.flag_fail();
                }
            }
            Err(nom::Err::Incomplete(_)) => {
                println!("FAIL: nom::Err::Incomplete");
                self.flag_fail();
            }
        }
        self
    }
}

// Tokenizer -------------------------------------------------------------

impl<'s, P, O, C> Test<P, Span<'s>, O, ParserError<'s, C>>
where
    O: Debug,
    C: Code,
{
    /// Checks for an error.
    ///
    /// Finish the test with q()
    #[must_use]
    pub fn err(&self, code: C) -> &Self {
        match &self.result {
            Ok(_) => {
                println!("FAIL: Expected error, but was ok!");
                self.flag_fail();
            }
            Err(e) => {
                if e.code != code {
                    println!("FAIL: {:?} <> {:?}", e.code, code);
                    self.flag_fail();
                }
            }
        }
        self
    }

    /// Checks for an expect value.
    ///
    /// Finish the test with q()
    #[must_use]
    pub fn expect(&self, code: C) -> &Self {
        match &self.result {
            Ok(_) => {
                println!("FAIL: {:?} was ok not an error.", code,);
                self.flag_fail();
            }
            Err(e) => {
                if !e.is_expected(code) {
                    println!(
                        "FAIL: {:?} is not an expected token. {:?}",
                        code,
                        e.expect_as_ref()
                    );
                    self.flag_fail();
                }
            }
        }

        self
    }

    /// Checks for an expect value.
    ///
    /// Finish the test with q()
    #[must_use]
    pub fn expect2(&self, code: C, parent: C) -> &Self {
        match &self.result {
            Ok(_) => {
                println!("FAIL: {:?} was ok not an error.", code,);
                self.flag_fail();
            }
            Err(e) => {
                if !e.is_expected2(code, parent) {
                    println!(
                        "FAIL: {:?} is not an expected token. {:?}",
                        code,
                        e.expect_as_ref()
                    );
                    self.flag_fail();
                }
            }
        }

        self
    }
}

// Parser ----------------------------------------------------------------

/// Extra data for the parser fn.
pub struct TestTracer<'a, 's, C: Code, const TRACK: bool> {
    pub trace: CTracer<'s, C, TRACK>,
    pub trace_filter: RefCell<FilterFn<'a, C>>,
}

// matches a ParserFn
impl<'a, 's, O, C, const TRACK: bool>
    Test<TestTracer<'a, 's, C, TRACK>, Span<'s>, (Span<'s>, O), ParserError<'s, C>>
where
    O: Debug,
    C: Code,
{
    /// Sets a filter on the trace.
    #[must_use]
    pub fn filter(&'a self, filter: FilterFn<'a, C>) -> &Self {
        self.x.trace_filter.replace(filter);
        self
    }
}

/// Extra data for the parser fn.
pub struct TestRTracer<'s, C: Code> {
    pub trace: RTracer<'s, C>,
}

/// Extra data for the parser fn.
pub struct TestNoTracer<'s, C: Code> {
    pub _phantom: PhantomData<(&'s str, C)>,
}

// Reporting -------------------------------------------------------------

/// Dumps the Result data if any test failed.
pub struct CheckDump;

impl<'s, P, O, E> Report<Test<P, Span<'s>, (Span<'s>, O), E>> for CheckDump
where
    E: Debug,
    O: Debug,
{
    #[track_caller]
    fn report(&self, test: &Test<P, Span<'s>, (Span<'s>, O), E>) {
        if test.fail.get() {
            dump(test);
            panic!("test failed")
        }
    }
}

/// Dumps the Result data.
pub struct Timing(pub u32);

impl<'s, P, I, O, E> Report<Test<P, I, O, E>> for Timing
where
    E: Debug,
    I: Debug,
    O: Debug,
{
    fn report(&self, test: &Test<P, I, O, E>) {
        println!(
            "when parsing '{}' in {} =>",
            restrict(
                DebugWidth::Medium,
                format!("{:?}", test.span).as_str().into()
            ),
            humantime::format_duration(test.duration / self.0)
        );
        match &test.result {
            Ok(_) => {
                println!("OK");
            }
            Err(_) => {
                println!("ERROR");
            }
        }
    }
}

/// Dumps the Result data.
pub struct Dump;

impl<'s, P, O, E> Report<Test<P, Span<'s>, (Span<'s>, O), E>> for Dump
where
    E: Debug,
    O: Debug,
{
    fn report(&self, test: &Test<P, Span<'s>, (Span<'s>, O), E>) {
        dump(test)
    }
}

fn dump<'s, P, O, E>(test: &Test<P, Span<'s>, (Span<'s>, O), E>)
where
    E: Debug,
    O: Debug,
{
    println!();
    println!(
        "when parsing '{}' in {} =>",
        restrict(DebugWidth::Medium, test.span),
        humantime::format_duration(test.duration)
    );
    match &test.result {
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

/// Dumps the full parser trace if any test failed.
pub struct CheckTrace;

impl<'s, O, C, E, const TRACK: bool>
    Report<Test<TestTracer<'_, 's, C, TRACK>, Span<'s>, (Span<'s>, O), E>> for CheckTrace
where
    E: Debug,
    O: Debug,
    C: Code,
{
    #[track_caller]
    fn report(&self, test: &Test<TestTracer<'_, 's, C, TRACK>, Span<'s>, (Span<'s>, O), E>) {
        if test.fail.get() {
            trace(test);
            panic!("test failed")
        }
    }
}

/// Dumps the full parser trace.
pub struct Trace;

impl<'s, O, C, E, const TRACK: bool>
    Report<Test<TestTracer<'_, 's, C, TRACK>, Span<'s>, (Span<'s>, O), E>> for Trace
where
    E: Debug,
    O: Debug,
    C: Code,
{
    fn report(&self, test: &Test<TestTracer<'_, 's, C, TRACK>, Span<'s>, (Span<'s>, O), E>) {
        trace(test);
    }
}

fn trace<'s, O, C, E, const TRACK: bool>(
    test: &Test<TestTracer<'_, 's, C, TRACK>, Span<'s>, (Span<'s>, O), E>,
) where
    O: Debug,
    E: Debug,
    C: Code,
{
    struct TracerDebug<'a, 's, C: Code, const TRACK: bool> {
        trace: &'a CTracer<'s, C, TRACK>,
        track_filter: FilterFn<'a, C>,
    }

    impl<'a, 's, C: Code, const TRACK: bool> Debug for TracerDebug<'a, 's, C, TRACK> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.trace.write(f, DebugWidth::Medium, self.track_filter)
        }
    }

    println!();
    println!(
        "when parsing '{}' in {} =>",
        restrict(DebugWidth::Medium, test.span),
        humantime::format_duration(test.duration)
    );

    let trace = &test.x.trace;
    let track_filter_r = test.x.trace_filter.borrow();
    let track_filter = &*track_filter_r;

    println!(
        "{:?}",
        TracerDebug {
            trace,
            track_filter
        }
    );

    match &test.result {
        Ok((rest, token)) => {
            println!(
                "rest {}:\"{}\"",
                rest.location_offset(),
                restrict(DebugWidth::Medium, *rest)
            );
            println!("{:0?}", token);
        }
        Err(e) => {
            println!("error");
            println!("{:1?}", e);
        }
    }
}

/// Dumps the full parser trace.
pub struct RTrace;

impl<'s, O, C, E> Report<Test<TestRTracer<'s, C>, Span<'s>, (Span<'s>, O), E>> for RTrace
where
    E: Debug,
    O: Debug,
    C: Code,
{
    fn report(&self, test: &Test<TestRTracer<'s, C>, Span<'s>, (Span<'s>, O), E>) {
        rtrace(test);
    }
}

fn rtrace<'s, O, C, E>(test: &Test<TestRTracer<'s, C>, Span<'s>, (Span<'s>, O), E>)
where
    O: Debug,
    E: Debug,
    C: Code,
{
    struct TracerDebug<'a, 's, C: Code> {
        trace: &'a RTracer<'s, C>,
    }

    impl<'a, 's, C: Code> Debug for TracerDebug<'a, 's, C> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.trace.write(f, DebugWidth::Medium)
        }
    }

    println!();
    println!(
        "when parsing '{}' in {} =>",
        restrict(DebugWidth::Medium, test.span),
        humantime::format_duration(test.duration)
    );

    let trace = &test.x.trace;

    println!("{:?}", TracerDebug { trace });

    match &test.result {
        Ok((rest, token)) => {
            println!(
                "rest {}:\"{}\"",
                rest.location_offset(),
                restrict(DebugWidth::Medium, *rest)
            );
            println!("{:0?}", token);
        }
        Err(e) => {
            println!("error");
            println!("{:1?}", e);
        }
    }
}
