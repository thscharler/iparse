use crate::error::DebugWidth;
use crate::Span;
use nom::{InputIter, Slice};

pub mod error;
pub mod tracer;

pub fn restrict(w: DebugWidth, span: Span<'_>) -> String {
    let l = match w {
        DebugWidth::Short => 20,
        DebugWidth::Medium => 40,
        DebugWidth::Long => 80,
    };

    let shortened = if span.len() > l {
        span.slice(..l)
    } else {
        span
    };

    shortened
        .escape_default()
        .chain("...".iter_elements())
        .collect()
}
