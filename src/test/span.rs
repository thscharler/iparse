use crate::test::{CompareFn, Report, Test, TestSpan};
use crate::Span;
use std::fmt::Debug;

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
            panic!()
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
        "when parsing '{}' in {}ns =>",
        testn.span,
        testn.duration.as_nanos()
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

/// Compare with an Ok(Span<'s>)
#[allow(dead_code)]
pub fn span<'a, 'b, 's>(span: &'a Span<'s>, value: (usize, &'b str)) -> bool {
    **span == value.1 && span.location_offset() == value.0
}

/// Compare with an Ok(Option<Span<'s>>, Span<'s>). Use the first span, fail on None.
#[allow(dead_code)]
pub fn span_0<'a, 'b, 's>(span: &'a (Option<Span<'s>>, Span<'s>), value: (usize, &'b str)) -> bool {
    if let Some(span) = &span.0 {
        **span == value.1 && span.location_offset() == value.0
    } else {
        false
    }
}

/// Compare with an Ok(Option<Span<'s>>, Span<'s>). Use the first span, fail on Some.
#[allow(dead_code)]
pub fn span_0_isnone<'a, 's>(span: &'a (Option<Span<'s>>, Span<'s>), _value: ()) -> bool {
    span.0.is_none()
}

/// Compare with an Ok(Option<Span<'s>>, Span<'s>). Use the second span.
#[allow(dead_code)]
pub fn span_1<'a, 'b, 's>(span: &'a (Option<Span<'s>>, Span<'s>), value: (usize, &'b str)) -> bool {
    *span.1 == value.1 && span.1.location_offset() == value.0
}

impl<'a> TestSpan for Span<'a> {
    /// Test for fn that return a naked Span.
    #[track_caller]
    fn ok(&self, offset: usize, fragment: &str) -> &Self {
        if *self.fragment() != fragment {
            println!("Fragment fails:");
            println!("    result='{}'", self.fragment());
            println!("    test  ='{}'", fragment);
            assert!(false);
        }
        if self.location_offset() != offset {
            println!("Offset fails for '{}'", self.fragment());
            println!("    offset={}", self.location_offset());
            println!("    test  ={}", offset);
            assert!(false);
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
