use crate::debug::restrict;
use crate::error::DebugWidth;
use crate::{Code, ParserResult, Span};
use nom::IResult;

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

impl<'a> TestSpan for Span<'a> {
    /// Test for fn that return a naked Span.
    #[track_caller]
    fn ok(&self, offset: usize, fragment: &str) -> &Self {
        if *self.fragment() != fragment {
            println!("Fragment fails:");
            println!("    result='{}'", self.fragment());
            println!("    test  ='{}'", fragment);
            panic!();
        }
        if self.location_offset() != offset {
            println!("Offset fails for '{}'", self.fragment());
            println!("    offset={}", self.location_offset());
            println!("    test  ={}", offset);
            panic!();
        }
        self
    }
}

impl<'a> TestSpan for (Span<'a>, Span<'a>) {
    /// Test for fn that return a pair of Span. Tests on r.1.
    #[track_caller]
    fn ok(&self, offset: usize, fragment: &str) -> &Self {
        self.1.ok(offset, fragment);
        self
    }
}

impl<'a> TestSpan for Result<(Span<'_>, Span<'_>), nom::Err<nom::error::Error<Span<'_>>>> {
    /// Test for fn that return an nom IResult
    #[track_caller]
    fn ok(&self, offset: usize, fragment: &str) -> &Self {
        match self {
            Ok((_rest, token)) => {
                token.ok(offset, fragment);
            }
            Err(e) => {
                println!("{:?}", e);
                panic!();
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
                    panic!();
                }
            }
            Err(e) => {
                println!("{:?}", e);
                panic!();
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
                panic!();
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
                panic!();
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
                panic!();
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
                    panic!();
                }
            }
            Err(e @ nom::Err::Failure(_)) => {
                println!("Failed with Err:Failure");
                println!("{:?}", e);
                panic!();
            }
            Err(e @ nom::Err::Incomplete(_)) => {
                println!("Failed with Err:Incomplete");
                println!("{:?}", e);
                panic!();
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
                panic!();
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
                    panic!();
                }
            }
            Err(e @ nom::Err::Failure(_)) => {
                println!("Failed with Err:Failure");
                println!("{:?}", e);
                panic!();
            }
            Err(e @ nom::Err::Incomplete(_)) => {
                println!("Failed with Err:Incomplete");
                println!("{:?}", e);
                panic!();
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

impl<'a, C: Code> TestSpan for ParserResult<'a, C, (Span<'a>, Span<'a>)> {
    /// Test for fn that return a ParseResult.
    #[track_caller]
    fn ok(&self, offset: usize, fragment: &str) -> &Self {
        match self {
            Ok((_rest, token)) => {
                token.ok(offset, fragment);
            }
            Err(e) => {
                println!("{:?}", e);
                panic!();
            }
        }
        self
    }
}

impl<'a, C: Code> TestSpanPair for ParserResult<'a, C, (Span<'a>, (Option<Span<'a>>, Span<'a>))> {
    /// Test for fn that return a ParseResult containing a (Option<Span>, Span).
    #[track_caller]
    fn ok_0(&self, offset: usize, fragment: &str) -> &Self {
        match self {
            Ok((_, (test, _))) => {
                if let Some(test) = test {
                    test.ok(offset, fragment);
                } else {
                    println!("Was None, should be {} '{}'", offset, fragment);
                    panic!();
                }
            }
            Err(e) => {
                println!("{:?}", e);
                panic!();
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
                panic!();
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
                panic!();
            }
        }
        self
    }
}

impl<'a, C: Code> TestFail<C> for ParserResult<'a, C, (Span<'a>, Span<'a>)> {
    #[track_caller]
    fn err(&self, kind: C) -> &Self {
        match self {
            Ok((rest, token)) => {
                println!("Ok, but should have failed:");
                println!("    rest='{}' token='{}'", rest, token);
                panic!();
            }
            Err(e) if e.code == C::NOM_ERROR => {
                println!("Failed with ErrNomError. To unspecified.");
                println!("{:?}", e);
                panic!();
            }
            Err(e) if e.code == C::NOM_FAILURE => {
                println!("Failed with ErrNomFailure.");
                println!("{:?}", e);
                panic!();
            }
            Err(e) => {
                if e.code != kind {
                    println!("Failed with the wrong ErrorKind:");
                    println!(
                        "    '{}' => result={} <> kind={:?}",
                        restrict(DebugWidth::Medium, e.span),
                        e,
                        kind
                    );
                    panic!();
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

impl<'a, C: Code> TestFail<C> for ParserResult<'a, C, (Span<'a>, (Option<Span<'a>>, Span<'a>))> {
    #[track_caller]
    fn err(&self, kind: C) -> &Self {
        match self {
            Ok((rest, token)) => {
                println!("Ok, but should have failed:");
                println!("    rest='{}' token='{:?}'", rest, token);
                panic!();
            }
            Err(e) if e.code == C::NOM_ERROR => {
                println!("Failed with ErrNomError. To unspecified.");
                println!("{:?}", e);
                panic!();
            }
            Err(e) if e.code == C::NOM_FAILURE => {
                println!("Failed with ErrNomFailure.");
                println!("{:?}", e);
                panic!();
            }
            Err(e) => {
                if e.code != kind {
                    println!("Failed with the wrong ErrorKind:");
                    println!(
                        "    '{}' => result={} <> kind={:?}",
                        restrict(DebugWidth::Medium, e.span),
                        e,
                        kind
                    );
                    panic!();
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
