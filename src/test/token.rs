use crate::error::ParserError;
use crate::test::Test;
use crate::{Code, ParserResult, Span};
use std::fmt::Debug;

/// Tokenizer function.
pub type TokenFn<'s, O, C> = fn(Span<'s>) -> ParserResult<'s, C, (Span<'s>, O)>;

// matches a TokenFn
impl<'s, O, C> Test<(), Span<'s>, (Span<'s>, O), ParserError<'s, C>>
where
    O: Debug,
    C: Code,
{
    /// Runs the tokenizer function and records the results.
    /// Use ok(), err(), ... to check specifics.
    ///
    /// Finish the test with q().
    pub fn token(span: &'s str, fn_test: TokenFn<'s, O, C>) -> Self {
        Self::run(span, fn_test)
    }
}

impl<'s, P, O, C> Test<P, Span<'s>, O, ParserError<'s, C>>
where
    P: Default,
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
