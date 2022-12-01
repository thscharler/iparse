#![allow(dead_code)]

mod general;
mod nom;
mod parser;
mod span;
mod token;

pub use self::general::*;
pub use self::nom::*;
pub use self::parser::*;
pub use self::span::*;
pub use self::token::*;
pub use crate::optional;

use crate::tracer::CTracer;
use crate::{ParserResult, Span};
use ::nom::IResult;
use std::cell::Cell;
use std::fmt::Debug;
use std::time::Duration;

/// Most general test fn.
pub type TestedFn<I, O, E> = fn(I) -> Result<O, E>;

/// Value comparison.
pub type CompareFn<O, V> = for<'a> fn(&'a O, V) -> bool;

/// Signature of a classic nom function for Test.
pub type NomFn<'s, O> = fn(Span<'s>) -> IResult<Span<'s>, O>;

/// Signature of a parser function for Test.
pub type ParserFn<'s, O, C> =
    fn(&'_ CTracer<'s, C>, Span<'s>) -> ParserResult<'s, C, (Span<'s>, O)>;

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

/// Extra trait for tests independent of Test.
///
/// Implemented for Result's the contain a Span.
pub trait TestSpan {
    fn ok(&self, offset: usize, fragment: &str) -> &Self;
}

/// Extra trait for tests independent of Test.
///
/// Implemented for Result's the contain a (Option<Span>, Span).
pub trait TestSpanPair {
    fn ok_0(&self, offset: usize, fragment: &str) -> &Self;
    fn ok_0_isnone(&self) -> &Self;
    fn ok_1(&self, offset: usize, fragment: &str) -> &Self;
}

/// Extra trait for tests independent of Test.
///
/// Tests for Result::Err variant.
pub trait TestFail<C> {
    fn err(&self, code: C) -> &Self;
    fn dump(&self) -> &Self;
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
