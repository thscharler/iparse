use crate::test::{Report, Test, TestedFn};
use std::cell::Cell;
use std::fmt::Debug;
use std::time::Instant;

impl<P, I, O, E> Test<P, I, O, E>
where
    I: Clone + Debug,
    O: Debug,
    E: Debug,
{
    /// Run a test function and record the results.
    pub fn run<'a, T>(span: T, fn_test: TestedFn<I, O, E>, x: &dyn Fn() -> P) -> Self
    where
        T: Into<I>,
    {
        let span: I = span.into();

        let now = Instant::now();
        let result = fn_test(span.clone());
        let elapsed = now.elapsed();

        Self {
            x: x(),
            span,
            result,
            duration: elapsed,
            fail: Cell::new(false),
        }
    }

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
