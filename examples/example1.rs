use iparse::error::ParserNomResult;
use iparse::tracer::CTracer;
use iparse::{Code, LookAhead, Parser, ParserResult, Span, Tracer};
use nom::bytes::complete::tag;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ICode {
    ICNomError,
    ICNomFailure,
    ICParseIncomplete,

    ICTerminalA,
    ICInt,
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
            ICode::ICInt => "Int",
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

pub fn nom_parse_a(i: Span<'_>) -> INomResult<'_> {
    tag("A")(i)
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

fn main() {
    let trace = CTracer::new();
    let res = ParseTerminalA::parse(&trace, Span::new("A"));
    dbg!(&res);

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
