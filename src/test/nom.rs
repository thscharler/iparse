use crate::test::{Test, TestFail, TestSpan, TestSpanPair};
use crate::Span;
use nom::IResult;
use std::fmt::Debug;

/// Nom parser fn
pub type NomFn<'s, O> = fn(Span<'s>) -> IResult<Span<'s>, O>;

// works for any NomFn.
// the extra restriction on the x-data leaves no imagination for the compiler.
impl<'s, O> Test<(), Span<'s>, (Span<'s>, O), nom::Err<nom::error::Error<Span<'s>>>>
where
    O: Debug,
{
    /// Run a test for a nom parser.
    pub fn nom(span: &'s str, fn_test: NomFn<'s, O>) -> Self {
        Self::run(span, fn_test, &|| ())
    }
}

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

impl<'a> TestSpan for IResult<Span<'a>, Span<'a>> {
    /// Test for fn that return an nom IResult
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

impl<'a> TestSpanPair for IResult<Span<'a>, (Option<Span<'a>>, Span<'a>)> {
    /// Test for fn that return an nom IResult containing a (Option<Span>, Span).
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

    #[track_caller]
    /// Test for fn that return an nom IResult containing a (Option<Span>, Span).
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

    #[track_caller]
    /// Test for fn that return an nom IResult containing a (Option<Span>, Span).
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

impl<'a> TestFail<nom::error::ErrorKind> for IResult<Span<'a>, Span<'a>> {
    /// Tests for fn that return a nom IResult
    #[track_caller]
    fn err(&self, kind: nom::error::ErrorKind) -> &Self {
        match self {
            Ok((rest, token)) => {
                println!("Ok, but should have failed:");
                println!("    rest='{}' token='{}'", rest, token);
                assert!(false);
            }
            Err(nom::Err::Error(e)) => {
                if e.code != kind {
                    println!("Failed with the wrong ErrorKind:");
                    println!(
                        "    '{}' => result={:?} <> kind={:?}",
                        e.input.fragment(),
                        e.code,
                        kind
                    );
                    assert!(false);
                }
            }
            Err(e @ nom::Err::Failure(_)) => {
                println!("Failed with Err:Failure");
                println!("{:?}", e);
                assert!(false);
            }
            Err(e @ nom::Err::Incomplete(_)) => {
                println!("Failed with Err:Incomplete");
                println!("{:?}", e);
                assert!(false);
            }
        }
        self
    }

    /// Tests for fn that return a nom IResult
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

impl<'a> TestFail<nom::error::ErrorKind> for IResult<Span<'a>, (Option<Span<'a>>, Span<'a>)> {
    #[track_caller]
    fn err(&self, kind: nom::error::ErrorKind) -> &Self {
        match self {
            Ok((rest, token)) => {
                println!("Ok, but should have failed:");
                println!("    rest='{}' token='{:?}'", rest, token);
                assert!(false);
            }
            Err(nom::Err::Error(e)) => {
                if e.code != kind {
                    println!("Failed with the wrong ErrorKind:");
                    println!(
                        "    '{}' => result={:?} <> kind={:?}",
                        e.input.fragment(),
                        e.code,
                        kind
                    );
                    assert!(false);
                }
            }
            Err(e @ nom::Err::Failure(_)) => {
                println!("Failed with Err:Failure");
                println!("{:?}", e);
                assert!(false);
            }
            Err(e @ nom::Err::Incomplete(_)) => {
                println!("Failed with Err:Incomplete");
                println!("{:?}", e);
                assert!(false);
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
