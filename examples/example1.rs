use crate::ICode::ICNonTerminal1;
use iparse::error::ParserNomResult;
use iparse::span::{span_union, span_union_opt};
use iparse::tracer::CTracer;
use iparse::tracer::TrackParseResult;
use iparse::{Code, LookAhead, Parser, ParserResult, Span, Tracer};
use nom::bytes::complete::tag;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ICode {
    ICNomError,
    ICNomFailure,
    ICParseIncomplete,

    ICTerminalA,
    ICTerminalB,
    ICNonTerminal1,
    ICNonTerminal2,
    ICInteger,
}

impl Code for ICode {
    const NOM_ERROR: Self = Self::ICNomError;
    const NOM_FAILURE: Self = Self::ICNomError;
    const PARSE_INCOMPLETE: Self = Self::ICNomError;
}

impl Display for ICode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ICode::ICNomError => "NomError",
            ICode::ICNomFailure => "NomFailure",
            ICode::ICParseIncomplete => "ParseIncomplete",
            ICode::ICTerminalA => "TerminalA",
            ICode::ICInteger => "Int",
            ICode::ICTerminalB => "TerminalB",
            ICode::ICNonTerminal1 => "NonTerminal1",
            ICode::ICNonTerminal2 => "NonTerminal2",
        };
        write!(f, "{}", name)
    }
}

pub type IParserResult<'s, O> = ParserResult<'s, O, ICode>;
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
pub struct NonTerminal1<'s> {
    pub a: TerminalA<'s>,
    pub b: TerminalB<'s>,
    pub span: Span<'s>,
}

#[derive(Debug)]
pub struct NonTerminal2<'s> {
    pub a: Option<TerminalA<'s>>,
    pub b: TerminalB<'s>,
    pub span: Span<'s>,
}

pub fn nom_parse_a(i: Span<'_>) -> INomResult<'_> {
    tag("A")(i)
}

pub fn nom_parse_b(i: Span<'_>) -> INomResult<'_> {
    tag("B")(i)
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
            Err(e.as_err(ICode::ICTerminalA))
        }
        Err(e) => Err(e.into()),
    }
}

pub struct ParseTerminalA;

impl<'s> Parser<'s, TerminalA<'s>, ICode> for ParseTerminalA {
    fn id() -> ICode {
        ICode::ICTerminalA
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

        trace.ok(token.span, rest, token)
    }
}

pub struct ParseTerminalB;

impl<'s> Parser<'s, TerminalB<'s>, ICode> for ParseTerminalB {
    fn id() -> ICode {
        ICode::ICTerminalB
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

pub struct ParseNonTerminal1;

impl<'s> Parser<'s, NonTerminal1<'s>, ICode> for NonTerminal1<'s> {
    fn id() -> ICode {
        ICNonTerminal1
    }

    fn parse<'t>(
        trace: &'t impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> ParserResult<'s, NonTerminal1<'s>, ICode> {
        let (rest, a) = ParseTerminalA::parse(trace, rest).track(trace)?;
        let (rest, b) = ParseTerminalB::parse(trace, rest).track(trace)?;

        let span = unsafe { span_union(a.span, b.span) };

        trace.ok(span, rest, NonTerminal1 { a, b, span })
    }
}

pub struct ParseNonTerminal2;

impl<'s> Parser<'s, NonTerminal2<'s>, ICode> for NonTerminal2<'s> {
    fn id() -> ICode {
        ICNonTerminal1
    }

    fn parse<'t>(
        trace: &'t impl Tracer<'s, ICode>,
        rest: Span<'s>,
    ) -> ParserResult<'s, NonTerminal2<'s>, ICode> {
        let (rest, a) = match ParseTerminalA::parse(trace, rest) {
            Ok((rest, a)) => (rest, Some(a)),
            Err(e) => {
                trace.stash(e);
                (rest, None)
            }
        };

        let (rest, b) = ParseTerminalB::parse(trace, rest).track(trace)?;

        let span = unsafe {
            if let Some(a) = a {
                span_union(a.span, b.span)
            } else {
                b.span
            }
        };

        trace.ok(span, rest, NonTerminal2 { a, b, span })
    }
}

fn main() {
    let trace = CTracer::new();
    let res = ParseTerminalA::parse(&trace, Span::new("A"));
    dbg!(&res);

    // don't know if tests in examples are a thing. simulate.
    tests::test_terminal_a();
}

mod tests {
    use crate::ParseTerminalA;
    use iparse::test::{CheckTrace, Test};
    use iparse::Parser;

    // #[test]
    pub fn test_terminal_a() {
        Test::parse("A", ParseTerminalA::parse)
            .okok()
            .q::<CheckTrace>();
        Test::parse("AA", ParseTerminalA::parse)
            .errerr()
            .q::<CheckTrace>();
    }
}
