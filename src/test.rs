use crate::debug::restrict;
use crate::error::{DebugWidth, ParserError};
use crate::rtracer::RTracer;
use crate::tracer::CTracer;
use crate::{Code, FilterFn, ParserResult, Span, Tracer};
use ::nom::IResult;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::fmt::Debug;
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
pub type ParserFn<'s, O, C> =
    fn(&'_ CTracer<'s, C>, Span<'s>) -> ParserResult<'s, C, (Span<'s>, O)>;

/// Signature of a parser function for Test.
pub type RParserFn<'s, O, C> =
    fn(&'_ RTracer<'s, C>, Span<'s>) -> ParserResult<'s, C, (Span<'s>, O)>;

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
pub trait Report<P, I, O, E>
where
    I: Debug,
    O: Debug,
    E: Debug,
{
    /// Do something.
    fn report(testn: &Test<P, I, O, E>);
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
    fn_test: ParserFn<'s, V, C>,
) -> Test<TestTracer<'a, 's, C>, Span<'s>, (Span<'s>, V), ParserError<'s, C>> {
    let span = Span::new(span);
    let trace = CTracer::new();

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
    let result = fn_test(&trace, span);
    let elapsed = now.elapsed();

    Test {
        x: TestRTracer { trace },
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
    pub fn q<R: Report<P, I, O, E>>(&self) {
        R::report(self);
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
                    println!("FAIL: Rest mismatch {} <> {}", **rest, test);
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
                    println!("FAIL: {:?} is not an expected token. {:?}", code, e.expect);
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
                    println!("FAIL: {:?} is not an expected token. {:?}", code, e.expect);
                    self.flag_fail();
                }
            }
        }

        self
    }
}

// Parser ----------------------------------------------------------------

/// Extra data for the parser fn.
pub struct TestTracer<'a, 's, C: Code> {
    pub trace: CTracer<'s, C>,
    pub trace_filter: RefCell<FilterFn<'a, C>>,
}

// matches a ParserFn
impl<'a, 's, O, C> Test<TestTracer<'a, 's, C>, Span<'s>, (Span<'s>, O), ParserError<'s, C>>
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

// Reporting -------------------------------------------------------------

/// Dumps the Result data if any test failed.
pub struct CheckDump;

impl<'s, P, O, E> Report<P, Span<'s>, (Span<'s>, O), E> for CheckDump
where
    E: Debug,
    O: Debug,
{
    #[track_caller]
    fn report(testn: &Test<P, Span<'s>, (Span<'s>, O), E>) {
        if testn.fail.get() {
            dump(testn);
            panic!("test failed")
        }
    }
}

/// Dumps the Result data.
pub struct Timing;

impl<'s, P, O, E> Report<P, Span<'s>, (Span<'s>, O), E> for Timing
where
    E: Debug,
    O: Debug,
{
    fn report(testn: &Test<P, Span<'s>, (Span<'s>, O), E>) {
        timing(testn)
    }
}

fn timing<'s, P, O, E>(testn: &Test<P, Span<'s>, (Span<'s>, O), E>)
where
    E: Debug,
    O: Debug,
{
    println!();
    println!(
        "when parsing '{}' in {} =>",
        restrict(DebugWidth::Medium, testn.span),
        humantime::format_duration(testn.duration)
    );
    match &testn.result {
        Ok((_rest, _token)) => {
            println!("OK");
        }
        Err(_e) => {
            println!("ERROR");
        }
    }
}

/// Dumps the Result data.
pub struct Dump;

impl<'s, P, O, E> Report<P, Span<'s>, (Span<'s>, O), E> for Dump
where
    E: Debug,
    O: Debug,
{
    fn report(testn: &Test<P, Span<'s>, (Span<'s>, O), E>) {
        dump(testn)
    }
}

fn dump<'s, P, O, E>(testn: &Test<P, Span<'s>, (Span<'s>, O), E>)
where
    E: Debug,
    O: Debug,
{
    println!();
    println!(
        "when parsing '{}' in {} =>",
        restrict(DebugWidth::Medium, testn.span),
        humantime::format_duration(testn.duration)
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

/// Dumps the full parser trace if any test failed.
pub struct CheckTrace;

impl<'s, O, C, E> Report<TestTracer<'_, 's, C>, Span<'s>, (Span<'s>, O), E> for CheckTrace
where
    E: Debug,
    O: Debug,
    C: Code,
{
    #[track_caller]
    fn report(testn: &Test<TestTracer<'_, 's, C>, Span<'s>, (Span<'s>, O), E>) {
        if testn.fail.get() {
            trace(testn);
            panic!("test failed")
        }
    }
}

/// Dumps the full parser trace.
pub struct Trace;

impl<'s, O, C, E> Report<TestTracer<'_, 's, C>, Span<'s>, (Span<'s>, O), E> for Trace
where
    E: Debug,
    O: Debug,
    C: Code,
{
    fn report(testn: &Test<TestTracer<'_, 's, C>, Span<'s>, (Span<'s>, O), E>) {
        trace(testn);
    }
}

fn trace<'s, O, C, E>(testn: &Test<TestTracer<'_, 's, C>, Span<'s>, (Span<'s>, O), E>)
where
    O: Debug,
    E: Debug,
    C: Code,
{
    struct TracerDebug<'a, 's, C: Code> {
        trace: &'a CTracer<'s, C>,
        track_filter: FilterFn<'a, C>,
    }

    impl<'a, 's, C: Code> Debug for TracerDebug<'a, 's, C> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.trace.write(f, DebugWidth::Medium, self.track_filter)
        }
    }

    println!();
    println!(
        "when parsing '{}' in {} =>",
        restrict(DebugWidth::Medium, testn.span),
        humantime::format_duration(testn.duration)
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

impl<'s, O, C, E> Report<TestRTracer<'s, C>, Span<'s>, (Span<'s>, O), E> for RTrace
where
    E: Debug,
    O: Debug,
    C: Code,
{
    fn report(testn: &Test<TestRTracer<'s, C>, Span<'s>, (Span<'s>, O), E>) {
        rtrace(testn);
    }
}

fn rtrace<'s, O, C, E>(testn: &Test<TestRTracer<'s, C>, Span<'s>, (Span<'s>, O), E>)
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
        restrict(DebugWidth::Medium, testn.span),
        humantime::format_duration(testn.duration)
    );

    let trace = &testn.x.trace;

    println!("{:?}", TracerDebug { trace });

    match &testn.result {
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
