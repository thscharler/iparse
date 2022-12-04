//!
//! Functions that can reconstruct a span from two spans.
//! Unsafe of course.
//!

extern crate memchr;

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

///
pub fn get_lines_after(span0: Span<'_>, n: u32) -> Vec<Span<'_>> {
    let line0 = span0.location_line();
    let offset0 = span0.location_offset();
    let slice = get_unoffsetted_slice(span0);

    let mut v = Vec::new();

    // find beginning of current line
    let loop_slice = &slice[..offset0 + 1];
    let offset_b = match memchr::memrchr(b'\n', loop_slice) {
        None => 0,
        Some(offset) => offset + 1,
    };

    // current line
    let mut loop_offset = offset_b;
    let mut loop_slice = &slice[loop_offset..];

    (loop_slice, loop_offset) = match memchr::memchr(b'\n', loop_slice) {
        None => {
            // no more \n
            let new_offset = loop_offset + loop_slice.len();
            let (current, new_slice) = loop_slice.split_at(loop_slice.len());

            v.push((line0, loop_offset, current));

            (new_slice, new_offset)
        }
        Some(offset) => {
            // slice started at offset_b
            let new_offset = loop_offset + offset + 1;
            let (current, new_slice) = loop_slice.split_at(offset + 1);

            v.push((line0, loop_offset, &current[..current.len() - 1]));

            (new_slice, new_offset)
        }
    };

    if loop_slice.len() > 0 {
        for i in 1..=n {
            (loop_slice, loop_offset) = match memchr::memchr(b'\n', loop_slice) {
                None => {
                    // at end
                    v.push((line0 + i, loop_offset, &loop_slice[..]));
                    break;
                }
                Some(offset) => {
                    let new_offset = loop_offset + offset + 1;
                    let (current, new_slice) = loop_slice.split_at(offset + 1);

                    // previous \n is at offset
                    v.push((line0 + i, loop_offset, &current[..current.len() - 1]));

                    (new_slice, new_offset)
                }
            }
        }
    }

    let r: Vec<_> = unsafe {
        v.into_iter()
            .map(|(n, offset, b)| (n, offset, from_utf8_unchecked(b)))
            .map(|(n, offset, s)| Span::new_from_raw_offset(offset, n, s, span0.extra))
            .collect()
    };

    r
}

/// Return n lines before the span, if possible. Maybe less.
/// The current line is completed and output too and not included in the count.
pub fn get_lines_before(span0: Span<'_>, n: u32) -> Vec<Span<'_>> {
    let line0 = span0.location_line();
    let offset0 = span0.location_offset();
    let slice = get_unoffsetted_slice(span0);

    let mut v = Vec::new();

    // find beginning of current line
    let loop_slice = &slice[..offset0 + 1];
    let offset_b = match memchr::memrchr(b'\n', loop_slice) {
        None => 0,
        Some(offset) => offset + 1,
    };

    // current line
    let loop_slice = &slice[offset_b..];
    match memchr::memchr(b'\n', loop_slice) {
        None => {
            // no more \n
            // slice started at offset_b
            v.push((line0, offset_b, &loop_slice[..]));
        }
        Some(offset) => {
            // slice started at offset_b
            v.push((line0, offset_b, &loop_slice[..offset]));
        }
    }

    if offset_b == 0 {
        // no more lines before
    } else {
        // offset_b -1 was \n
        let mut loop_slice = &slice[..offset_b - 1];
        for i in 1..=n {
            match memchr::memrchr(b'\n', loop_slice) {
                None => {
                    // at beginning
                    v.push((line0 - i, 0, &loop_slice[..]));
                    break;
                }
                Some(offset) => {
                    // previous \n is at offset
                    v.push((line0 - i, offset + 1, &loop_slice[offset + 1..]));
                    // cut back to before \n
                    loop_slice = &loop_slice[..offset]
                }
            }
        }
    }

    let mut r: Vec<_> = unsafe {
        v.into_iter()
            .map(|(n, offset, b)| (n, offset, from_utf8_unchecked(b)))
            .map(|(n, offset, s)| Span::new_from_raw_offset(offset, n, s, span0.extra))
            .collect()
    };

    r.reverse();

    r
}

/// Returns the slice before any offset.
///
/// Safety
///
/// * `data` must be [valid] for reads for `len * mem::size_of::<T>()` many bytes,
///   and it must be properly aligned. This means in particular:
///
///     * The entire memory range of this slice must be contained within a single allocated object!
///       Slices can never span across multiple allocated objects. See [below](#incorrect-usage)
///       for an example incorrectly not taking this into account.
///     * `data` must be non-null and aligned even for zero-length slices. One
///       reason for this is that enum layout optimizations may rely on references
///       (including slices of any length) being aligned and non-null to distinguish
///       them from other data. You can obtain a pointer that is usable as `data`
///       for zero-length slices using [`NonNull::dangling()`].
///
///     => We use only a single Span, should be ok.
///
/// * `data` must point to `len` consecutive properly initialized values of type `T`.
///
///     => Same.
///
/// * The memory referenced by the returned slice must not be mutated for the duration
///   of lifetime `'a`, except inside an `UnsafeCell`.
///
///     => No mutation occurs.
///
/// * The total size `len * mem::size_of::<T>()` of the slice must be no larger than `isize::MAX`.
///
///     => Is checked by an assert.
///
pub fn get_unoffsetted_span(span0: Span<'_>) -> Span<'_> {
    unsafe {
        let slice = get_unoffsetted_slice(span0);
        // span0 was a valid str before so this should be ok.
        let str = from_utf8_unchecked(slice);
        // offset is 0, line is 1
        Span::new_from_raw_offset(0, 1, str, span0.extra)
    }
}

fn get_unoffsetted_slice(span0: Span<'_>) -> &[u8] {
    let self_ptr = get_unoffsetted_ptr(span0);
    let new_len = span0.location_offset() + span0.len();
    assert!(new_len <= isize::MAX as usize, "new_len is too big");

    unsafe { slice::from_raw_parts(self_ptr, new_len) }
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
///     => Use get_unoffsetted_slice from nom_locate-4.0.0 to compare the original
///     pointers of both spans.
///
/// * The distance between the pointers, in bytes, cannot overflow an isize.
///     
///     => Assert that span0 has a lower offset than span1.
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
    use crate::span::{get_lines_after, get_lines_before, span_union};
    use crate::{Code, ParserNomResult, Span};
    use nom::bytes::complete::{take_while, take_while1};
    use nom::character::complete::digit1;
    use nom::combinator::recognize;
    use nom::sequence::preceded;
    use nom::InputTakeAtPosition;
    use std::fmt::{Debug, Display, Formatter};

    #[test]
    pub fn test_lines_after() {
        let span0 = Span::new("1234\n5678\nabcd\nefgh\n");
        let (s0, s1) = span0
            .split_at_position::<_, nom::error::Error<Span<'_>>>(|c| c == '7')
            .unwrap();
        dbg!(&s0, &s1);

        dbg!(get_lines_after(s0, 0));
        dbg!(get_lines_after(s0, 1));
        dbg!(get_lines_after(s0, 2));
        dbg!(get_lines_after(s0, 3));
        dbg!(get_lines_after(s0, 4));

        let span0 = Span::new("1234\n5678\nabcd\nefgh");
        let (s0, s1) = span0
            .split_at_position::<_, nom::error::Error<Span<'_>>>(|c| c == 'g')
            .unwrap();
        dbg!(&s0, &s1);
        dbg!(get_lines_after(s0, 0));
        dbg!(get_lines_after(s0, 1));
        dbg!(get_lines_after(s0, 2));
    }

    #[test]
    pub fn test_lines_before() {
        let span0 = Span::new("1234\n5678\nabcd\nefgh\n");
        let (s0, s1) = span0
            .split_at_position::<_, nom::error::Error<Span<'_>>>(|c| c == 'c')
            .unwrap();
        dbg!(&s0, &s1);

        dbg!(get_lines_before(s0, 0));
        dbg!(get_lines_before(s0, 1));
        dbg!(get_lines_before(s0, 2));
        dbg!(get_lines_before(s0, 3));
        dbg!(get_lines_before(s0, 4));

        dbg!(get_lines_before(s1, 4));
    }

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
