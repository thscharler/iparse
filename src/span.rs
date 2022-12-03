//!
//! Functions that can reconstruct a span from two spans.
//! Unsafe of course.
//!

use crate::Span;
use nom::Offset;
use std::slice;
use std::str::from_utf8_unchecked;

/// # Safety
///  See span_union for details.
pub unsafe fn span_union_opt<'a>(span0: Option<Span<'a>>, span1: Span<'a>) -> Span<'a> {
    match span0 {
        None => span1,
        Some(span0) => span_union(span0, span1),
    }
}

// from nom_locate-4.0.0
fn get_unoffsetted_ptr(span0: Span<'_>) -> *const u8 {
    let self_bytes = span0.fragment().as_bytes();
    let self_ptr = self_bytes.as_ptr();
    unsafe {
        assert!(
            span0.location_offset() <= isize::MAX as usize,
            "offset is too big"
        );
        self_ptr.offset(-(span0.location_offset() as isize))
    }
}

/// Returns a new Span that reaches from the beginning of span0 to the end of span1.
///
/// # Safety
///
/// If any of the following conditions are violated, the result is Undefined Behavior:
/// * Both the starting and other pointer must be either in bounds or one byte past the end
///   of the same allocated object.
/// * Both pointers must be derived from a pointer to the same object.
///       
///     Use get_unoffsetted_slice from nom_locate-4.0.0 to compare the original
///     pointers of both spans.
///
/// * The distance between the pointers, in bytes, cannot overflow an isize.
///     Assert that span0 has a lower offset than span1.
///
/// * The distance being in bounds cannot rely on “wrapping around” the address space.
pub fn span_union<'a>(span0: Span<'a>, span1: Span<'a>) -> Span<'a> {
    // should be a good start.
    assert_eq!(get_unoffsetted_ptr(span0), get_unoffsetted_ptr(span1));

    unsafe {
        let self_ptr = span0.fragment().as_ptr();

        // Calculate the relative offset of span1 and add its length.
        assert!(span0.location_offset() <= span1.location_offset());
        let new_len = span0.offset(&span1) + span1.len();
        let slice = slice::from_raw_parts(self_ptr, new_len);

        // span0 was a valid str before so this should be ok.
        let str = from_utf8_unchecked(slice);

        // Copy everything else from span0
        Span::new_from_raw_offset(
            span0.location_offset(),
            span0.location_line(),
            str,
            span0.extra,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::error::ParserError;
    use crate::span::span_union;
    use crate::{Code, ParserNomResult, Span};
    use nom::bytes::complete::{take_while, take_while1};
    use nom::character::complete::digit1;
    use nom::combinator::recognize;
    use nom::sequence::preceded;
    use std::fmt::{Debug, Display, Formatter};

    #[test]
    pub fn test_union_ok() {
        let span = Span::new("1234 test");
        let _other = Span::new("5678 xxxx");

        let (rest, number) = nom_number(span).unwrap();
        let (_rest, name) = nom_name(rest).unwrap();

        span_union(number, name);
    }

    #[test]
    #[should_panic]
    pub fn test_union_order() {
        let span = Span::new("1234 test");
        let _other = Span::new("5678 xxxx");

        let (rest, number) = nom_number(span).unwrap();
        let (_rest, name) = nom_name(rest).unwrap();

        span_union(name, number);
    }

    #[test]
    #[should_panic]
    pub fn test_union_other_1() {
        let span = Span::new("1234 test");
        let other = Span::new("5678 xxxx");

        let (rest, _number) = nom_number(span).unwrap();
        let (_rest, name) = nom_name(rest).unwrap();

        span_union(other, name);
    }

    #[test]
    #[should_panic]
    pub fn test_union_other_2() {
        let span = Span::new("1234 test");
        let other = Span::new("5678 xxxx");

        let (rest, _number) = nom_number(span).unwrap();
        let (_rest, name) = nom_name(rest).unwrap();

        span_union(name, other);
    }

    #[allow(dead_code)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum TCode {
        Number,
        Name,
    }

    impl Display for TCode {
        fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
            unreachable!()
        }
    }

    impl Code for TCode {
        const NOM_ERROR: Self = Self::Number;
        const NOM_FAILURE: Self = Self::Number;
        const PARSE_INCOMPLETE: Self = Self::Number;
    }

    fn nom_number(i: Span<'_>) -> ParserNomResult<'_, TCode> {
        preceded(nom_ws, digit1)(i)
    }

    fn nom_name(i: Span<'_>) -> ParserNomResult<'_, TCode> {
        preceded(
            nom_ws,
            recognize(take_while1(|c: char| c.is_alphanumeric())),
        )(i)
    }

    fn nom_ws<'s>(i: Span<'s>) -> ParserNomResult<'s, TCode> {
        recognize(take_while::<_, _, ParserError<'s, TCode>>(|c: char| {
            c == ' ' || c == '\t'
        }))(i)
    }
}
