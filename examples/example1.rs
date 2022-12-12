use crate::ICode::*;
use iparse::error::ParserError;
use iparse::span::span_union;
use iparse::test::{test_parse, Trace};
use iparse::tracer::CTracer;
use iparse::{
    Code, IntoParserResultAddSpan, ParseAsOptional, Parser, ParserNomResult, ParserResult, Span,
    Tracer,
};
use iparse::{IntoParserError, TrackParseResult};
use nom::bytes::complete::{tag, tag_no_case};
use nom::character::complete::{char as nchar, digit1};
use nom::combinator::recognize;
use nom::sequence::{terminated, tuple};
use nom::{AsChar, InputTake, InputTakeAtPosition};
use std::fmt::{Display, Formatter};
use std::num::ParseIntError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ICode {
    ICNomError,
    ICNomFailure,
    ICParseIncomplete,

    ICTerminalA,
    ICTerminalB,
    ICTerminalC,
    ICTerminalD,
    ICNonTerminal1,
    ICNonTerminal2,
    ICNonTerminal3,
    ICInteger,
    ICNummer,
}

impl Code for ICode {
    const NOM_ERROR: Self = ICNomError;
    const NOM_FAILURE: Self = ICNomError;
    const PARSE_INCOMPLETE: Self = ICNomError;
}

impl Display for ICode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ICNomError => "NomError",
            ICNomFailure => "NomFailure",
            ICParseIncomplete => "ParseIncomplete",
            ICTerminalA => "TerminalA",
            ICInteger => "Int",
            ICTerminalB => "TerminalB",
            ICNonTerminal1 => "NonTerminal1",
            ICNonTerminal2 => "NonTerminal2",
            ICNonTerminal3 => "NonTerminal3",
            ICTerminalC => "TerminalC",
            ICTerminalD => "TerminalD",
            ICNummer => "Nummer",
        };
        write!(f, "{}", name)
    }
}

pub type IParserResult<'s, O> = ParserResult<'s, ICode, (Span<'s>, O)>;
pub type INomResult<'s> = ParserNomResult<'s, ICode>;
pub type IParserError<'s> = ParserError<'s, ICode>;

#[derive(Debug)]
pub struct TerminalA<'s> {
    pub term: String,
    pub span: Span<'s>,
}

#[derive(Debug)]
pub struct TerminalB<'s> {
    pub term: String,
    pub span: Span<'s>,
}

#[derive(Debug)]
pub struct TerminalC<'s> {
    pub term: u32,
    pub span: Span<'s>,
}

#[derive(Debug)]
pub struct TerminalD<'s> {
    pub term: INummer<'s>,
    pub span: Span<'s>,
}

#[derive(Debug)]
pub struct NonTerminal1<'s> {
    pub a: TerminalA<'s>,
    pub b: TerminalB<'s>,
    pub span: Span<'s>,
}

#[derive(Debug)]
pub struct NonTerminal2<'s> {
    pub a: Option<TerminalA<'s>>,
    pub b: TerminalB<'s>,
    pub c: TerminalC<'s>,
    pub span: Span<'s>,
}

#[derive(Debug)]
pub struct INummer<'s> {
    pub nummer: u32,
    pub span: Span<'s>,
}

impl<'s, T> IntoParserResultAddSpan<'s, ICode, T> for Result<T, ParseIntError> {
    fn into_with_span(self, span: Span<'s>) -> ParserResult<'s, ICode, T> {
        match self {
            Ok(v) => Ok(v),
            Err(_) => Err(ParserError::new(ICInteger, span)),
        }
    }
}

pub fn nom_parse_a(i: Span<'_>) -> INomResult<'_> {
    tag("A")(i)
}

pub fn nom_parse_b(i: Span<'_>) -> INomResult<'_> {
    tag("B")(i)
}

pub fn nom_parse_c(i: Span<'_>) -> INomResult<'_> {
    digit1(i)
}

pub fn parse_a(rest: Span<'_>) -> IParserResult<'_, TerminalA> {
    match nom_parse_a(rest) {
        Ok((rest, token)) => Ok((
            rest,
            TerminalA {
                term: token.to_string(),
                span: token,
            },
        )),
        Err(nom::Err::Error(e)) if e.is_kind(nom::error::ErrorKind::Tag) => {
            Err(e.into_code(ICTerminalA))
        }
        Err(e) => Err(e.into()),
    }
}

pub fn nom_star_star(i: Span<'_>) -> INomResult<'_> {
    terminated(recognize(tuple((nchar('*'), nchar('*')))), nom_ws)(i)
}

pub fn nom_tag_kdnr(i: Span<'_>) -> INomResult<'_> {
    terminated(recognize(tag_no_case("kdnr")), nom_ws)(i)
}

pub fn nom_ws(i: Span<'_>) -> INomResult<'_> {
    i.split_at_position_complete(|item| {
        let c = item.as_char();
        !(c == ' ' || c == '\t')
    })
}

pub fn nom_number(i: Span<'_>) -> INomResult<'_> {
    terminated(digit1, nom_ws)(i)
}

pub fn token_nummer(rest: Span<'_>) -> IParserResult<'_, INummer<'_>> {
    match nom_number(rest) {
        Ok((rest, tok)) => Ok((
            rest,
            INummer {
                nummer: tok.parse::<u32>().into_with_span(rest)?,
                span: tok,
            },
        )),
        Err(e) => Err(e.into_with_code(ICNummer)),
    }
}

pub struct ParseTerminalA;

impl<'s> Parser<'s, TerminalA<'s>, ICode> for ParseTerminalA {
    fn id() -> ICode {
        ICTerminalA
    }

    fn lah(_: Span<'s>) -> bool {
        true
    }

    fn parse<'t>(
        trace: &'t mut impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> IParserResult<'s, TerminalA<'s>> {
        trace.enter(Self::id(), rest);

        let (rest, token) = match parse_a(rest) {
            Ok((rest, token)) => (rest, token),
            Err(e) => return trace.err(e),
        };

        trace.ok(rest, token.span, token)
    }
}

pub struct ParseTerminalB;

impl<'s> Parser<'s, TerminalB<'s>, ICode> for ParseTerminalB {
    fn id() -> ICode {
        ICTerminalB
    }

    fn parse<'t>(
        trace: &'t mut impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> IParserResult<'s, TerminalB<'s>> {
        trace.enter(Self::id(), rest);

        let (rest, token) = match nom_parse_b(rest) {
            Ok((rest, token)) => (rest, token),
            Err(e) => return trace.err(e.into()),
        };

        trace.ok(
            token,
            rest,
            TerminalB {
                term: token.to_string(),
                span: token,
            },
        )
    }
}

pub struct ParseTerminalC;

impl<'s> Parser<'s, TerminalC<'s>, ICode> for ParseTerminalC {
    fn id() -> ICode {
        ICTerminalC
    }

    fn parse<'t>(
        trace: &'t mut impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> IParserResult<'s, TerminalC<'s>> {
        trace.enter(Self::id(), rest);

        let (rest, tok) = match nom_parse_c(rest) {
            Ok((rest, tok)) => (
                rest,
                TerminalC {
                    term: (*tok).parse::<u32>().into_with_span(tok).track(trace)?,
                    span: tok,
                },
            ),
            Err(e) => return trace.err(e.into()),
        };

        trace.ok(rest, tok.span, tok)
    }
}

pub struct ParseTerminalD;

impl<'s> Parser<'s, TerminalD<'s>, ICode> for ParseTerminalD {
    fn id() -> ICode {
        ICTerminalD
    }

    fn parse<'t>(
        trace: &'t mut impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> IParserResult<'s, TerminalD<'s>> {
        trace.enter(Self::id(), rest);

        let (rest, _) = nom_star_star(rest).optional().track(trace)?;
        let (rest, tag) = nom_tag_kdnr(rest).track(trace)?;
        let (rest, term) = token_nummer(rest).track(trace)?;
        let (rest, _) = nom_star_star(rest).optional().track(trace)?;

        let span = span_union(tag, term.span);
        trace.ok(rest, span, TerminalD { term, span })
    }
}

pub struct ParseNonTerminal1;

impl<'s> Parser<'s, NonTerminal1<'s>, ICode> for ParseNonTerminal1 {
    fn id() -> ICode {
        ICNonTerminal1
    }

    fn parse<'t>(
        trace: &'t mut impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> IParserResult<'s, NonTerminal1<'s>> {
        let (rest, a) = ParseTerminalA::parse(trace, rest).track(trace)?;
        let (rest, b) = ParseTerminalB::parse(trace, rest).track(trace)?;

        let span = span_union(a.span, b.span);

        trace.ok(rest, span, NonTerminal1 { a, b, span })
    }
}

pub struct ParseNonTerminal2;

impl<'s> Parser<'s, NonTerminal2<'s>, ICode> for ParseNonTerminal2 {
    fn id() -> ICode {
        ICNonTerminal1
    }

    fn parse<'t>(
        trace: &'t mut impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> IParserResult<'s, NonTerminal2<'s>> {
        trace.enter(Self::id(), rest);

        let (rest, a) = match ParseTerminalA::parse(trace, rest) {
            Ok((rest, a)) => (rest, Some(a)),
            Err(e) => {
                trace.stash(e);
                (rest, None)
            }
        };

        let (rest, b) = ParseTerminalB::parse(trace, rest).track(trace)?;
        let (rest, c) = ParseTerminalC::parse(trace, rest).track(trace)?;

        let span = if let Some(a) = &a {
            span_union(a.span, c.span)
        } else {
            c.span
        };

        trace.ok(rest, span, NonTerminal2 { a, b, c, span })
    }
}

pub struct ParseNonTerminal3;

impl<'s> Parser<'s, (), ICode> for ParseNonTerminal3 {
    fn id() -> ICode {
        ICNonTerminal3
    }

    fn parse<'t>(trace: &'t mut impl Tracer<'s, ICode>, rest: Span<'s>) -> IParserResult<'s, ()> {
        let mut loop_rest = rest;
        loop {
            let rest2 = loop_rest;

            let (rest2, _a) = ParseTerminalA::parse(trace, rest2).track(trace)?;

            let (rest2, _b) = match ParseTerminalB::parse(trace, rest2) {
                Ok((rest3, b)) => (rest3, Some(b)),
                Err(e) => {
                    trace.suggest(e.code, e.span);
                    (rest2, None)
                }
            };

            if rest2.is_empty() {
                break;
            }

            // endless loop
            if loop_rest == rest2 {
                return trace.err(ParserError::new(ICNonTerminal3, rest2));
            }

            loop_rest = rest2;
        }

        trace.ok(rest, rest.take(0), ())
    }
}

fn run_parser() -> IParserResult<'static, TerminalA<'static>> {
    let mut trace: CTracer<_, true> = CTracer::new();
    ParseTerminalA::parse(&mut trace, Span::new("A"))
}

fn main() {
    let _ = run_parser();

    // don't know if tests in examples are a thing. simulate.
    test_terminal_a();
    test_nonterminal2();
}

const R: Trace = Trace;

// #[test]
pub fn test_terminal_a() {
    test_parse("A", ParseTerminalA::parse).okok().q(&R);
    test_parse("AA", ParseTerminalA::parse).errerr().q(&R);
}

pub fn test_nonterminal2() {
    test_parse("AAA", ParseNonTerminal2::parse).errerr().q(&R);
}
