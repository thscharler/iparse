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
    unsafe {
        match span0 {
            None => span1,
            Some(span0) => span_union(span0, span1),
        }
    }
}

/// Returns a new Span that reaches from the beginning of span0 to the end of span1.
///
/// # Safety
///
/// If any of the following conditions are violated, the result is Undefined Behavior:
/// * Both the starting and other pointer must be either in bounds or one byte past the end of the same allocated object.
///      Should be guaranteed if both were obtained from on ast run.
/// * Both pointers must be derived from a pointer to the same object.
///      Should be guaranteed if both were obtained from on ast run.
/// * The distance between the pointers, in bytes, cannot overflow an isize.
/// * The distance being in bounds cannot rely on “wrapping around” the address space.
pub unsafe fn span_union<'a>(span0: Span<'a>, span1: Span<'a>) -> Span<'a> {
    let ptr = span0.as_ptr();
    // offset to the start of span1 and add the length of span1.
    let size = span0.offset(&span1) + span1.len();

    unsafe {
        // The size should be within the original allocation, if both spans are from
        // the same ast run. We must ensure that the ast run doesn't generate
        // Spans out of nothing that end in the ast.
        let slice = slice::from_raw_parts(ptr, size);
        // This is all from a str originally and we never got down to bytes.
        let str = from_utf8_unchecked(slice);

        // As span0 was ok the offset used here is ok too.
        Span::new_from_raw_offset(span0.location_offset(), span0.location_line(), str, ())
    }
}
