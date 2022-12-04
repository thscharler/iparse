use crate::error::DebugWidth;
use crate::Span;
use nom::bytes::complete::take_while_m_n;
use nom::InputIter;

pub mod error;
pub mod rtracer;
pub mod tracer;

pub fn restrict(w: DebugWidth, span: Span<'_>) -> String {
    match w {
        DebugWidth::Short => restrict_n(20, span),
        DebugWidth::Medium => restrict_n(40, span),
        DebugWidth::Long => restrict_n(60, span),
    }
}

pub fn restrict_n(max_len: usize, span: Span<'_>) -> String {
    let shortened =
        match take_while_m_n::<_, _, nom::error::Error<Span<'_>>>(0, max_len, |_c| true)(span) {
            Ok((_rest, short)) => *short,
            Err(_) => "?error?",
        };

    shortened
        .escape_default()
        .chain("...".iter_elements())
        .collect()
}
