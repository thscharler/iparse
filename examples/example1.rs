use crate::ICode::*;
use iparse::error::ParserError;
use iparse::span::span_union;
use iparse::test::{test_parse, Trace};
use iparse::tracer::CTracer;
use iparse::TrackParseResult;
use iparse::{
    Code, IntoParserResult, LookAhead, Parser, ParserNomResult, ParserResult, Span, Tracer,
};
use nom::bytes::complete::tag;
use nom::character::complete::digit1;
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
    ICNonTerminal1,
    ICNonTerminal2,
    ICInteger,
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
            ICTerminalC => "TerminalC",
        };
        write!(f, "{}", name)
    }
}

pub type IParserResult<'s, O> = ParserResult<'s, ICode, (Span<'s>, O)>;
pub type INomResult<'s> = ParserNomResult<'s, ICode>;

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

impl<'s, T> IntoParserResult<'s, ICode, T> for Result<T, ParseIntError> {
    fn into_parser_err(self, span: Span<'s>) -> ParserResult<'s, ICode, T> {
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

pub struct ParseTerminalA;

impl<'s> Parser<'s, TerminalA<'s>, ICode> for ParseTerminalA {
    fn id() -> ICode {
        ICTerminalA
    }

    fn lah(_: Span<'s>) -> LookAhead {
        LookAhead::Parse
    }

    fn parse<'t>(
        trace: &'t impl Tracer<'s, ICode>,
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
        trace: &'t impl Tracer<'s, ICode>,
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
        trace: &'t impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> IParserResult<'s, TerminalC<'s>> {
        trace.enter(Self::id(), rest);

        let (rest, tok) = match nom_parse_c(rest) {
            Ok((rest, tok)) => (
                rest,
                TerminalC {
                    term: (*tok).parse::<u32>().into_parser_err(tok).track(trace)?,
                    span: tok,
                },
            ),
            Err(e) => return trace.err(e.into()),
        };

        trace.ok(rest, tok.span, tok)
    }
}

pub struct ParseNonTerminal1;

impl<'s> Parser<'s, NonTerminal1<'s>, ICode> for ParseNonTerminal1 {
    fn id() -> ICode {
        ICNonTerminal1
    }

    fn parse<'t>(
        trace: &'t impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> IParserResult<'s, NonTerminal1<'s>> {
        let (rest, a) = ParseTerminalA::parse(trace, rest).track(trace)?;
        let (rest, b) = ParseTerminalB::parse(trace, rest).track(trace)?;

        let span = unsafe { span_union(a.span, b.span) };

        trace.ok(rest, span, NonTerminal1 { a, b, span })
    }
}

pub struct ParseNonTerminal2;

impl<'s> Parser<'s, NonTerminal2<'s>, ICode> for ParseNonTerminal2 {
    fn id() -> ICode {
        ICNonTerminal1
    }

    fn parse<'t>(
        trace: &'t impl Tracer<'s, ICode>,
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

        let span = unsafe {
            if let Some(a) = &a {
                span_union(a.span, c.span)
            } else {
                c.span
            }
        };

        trace.ok(rest, span, NonTerminal2 { a, b, c, span })
    }
}

fn run_parser() -> IParserResult<'static, TerminalA<'static>> {
    let trace = CTracer::new();
    ParseTerminalA::parse(&trace, Span::new("A"))
}

fn main() {
    let _ = run_parser();

    // don't know if tests in examples are a thing. simulate.
    test_terminal_a();
    test_nonterminal2();
}

type R = Trace;

// #[test]
pub fn test_terminal_a() {
    test_parse("A", ParseTerminalA::parse).okok().q::<R>();
    test_parse("AA", ParseTerminalA::parse).errerr().q::<R>();
}

pub fn test_nonterminal2() {
    test_parse("AAA", ParseNonTerminal2::parse)
        .errerr()
        .q::<R>();
}
