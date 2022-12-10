use crate::debug::restrict;
use crate::tracer::CTracer;
use crate::{Code, IntoParserError, IntoParserResultAddCode, ParserResult, Span};
use nom::error::ErrorKind;
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

/// Combined error including the CTracer.
/// Make your own if you need a different Tracer.
pub struct TracerError<'s, C: Code, const TRACK: bool> {
    pub parse: ParserError<'s, C>,
    pub trace: CTracer<'s, C, TRACK>,
}

impl<'s, C: Code, const TRACK: bool> Debug for TracerError<'s, C, TRACK> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.parse)?;
        Ok(())
    }
}

impl<'s, C: Code, const TRACK: bool> Display for TracerError<'s, C, TRACK> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?}", self.parse)?;
        Ok(())
    }
}

impl<'s, C: Code, const TRACK: bool> Error for TracerError<'s, C, TRACK> {}

/// Error for the Parser.
pub struct ParserError<'s, C: Code> {
    /// Error code.
    pub code: C,
    /// Error span.
    pub span: Span<'s>,
    /// Flag for Tracer.
    pub tracing: bool,
    /// Collected nom errors if any.
    pub hints: Vec<Hints<'s, C>>,
}

impl<'s, C: Code> ParserError<'s, C> {
    /// New error.
    pub fn new(code: C, span: Span<'s>) -> Self {
        Self {
            code,
            span,
            tracing: false,
            hints: Vec::new(),
        }
    }

    /// New error adds the code as Suggestion too.
    pub fn new_suggest(code: C, span: Span<'s>) -> Self {
        Self {
            code,
            span,
            tracing: false,
            hints: vec![Hints::Suggest(Suggest {
                code,
                span,
                parents: vec![],
            })],
        }
    }

    /// New error. Adds information about a nom error.
    pub fn new_with_nom(code: C, nom_code: ErrorKind, span: Span<'s>) -> Self {
        Self {
            code,
            span,
            tracing: false,
            hints: vec![Hints::Nom(Nom {
                kind: nom_code,
                span,
            })],
        }
    }

    /// Convert to a new error code.
    pub fn into_code(mut self, code: C) -> Self {
        self.code = code;
        self
    }

    /// Special error code. Encodes errors occurring at the margins.
    pub fn is_special(&self) -> bool {
        self.code.is_special()
    }

    /// Error code of the parser.
    pub fn is_parser(&self) -> bool {
        !self.code.is_special()
    }

    /// Is this one of the nom errorkind codes?
    pub fn is_kind(&self, kind: ErrorKind) -> bool {
        for n in &self.hints {
            if let Hints::Nom(n) = n {
                if n.kind == kind {
                    return true;
                }
            }
        }
        false
    }

    /// Was this one of the expected errors.
    pub fn is_expected(&self, code: C) -> bool {
        for exp in &self.hints {
            if let Hints::Expect(exp) = exp {
                if exp.code == code {
                    return true;
                }
            }
        }
        false
    }

    /// Was this one of the expected errors, and is in the call stack of parent?
    pub fn is_expected2(&self, code: C, parent: C) -> bool {
        for exp in &self.hints {
            if let Hints::Expect(exp) = exp {
                if exp.code == code {
                    for par in &exp.parents {
                        if *par == parent {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// ParseIncomplete variant.
    pub fn parse_incomplete(span: Span<'s>) -> ParserError<'s, C> {
        ParserError::new(C::PARSE_INCOMPLETE, span)
    }

    pub fn nom(&self) -> Vec<&Nom<'s>> {
        self.hints
            .iter()
            .filter_map(|v| match v {
                Hints::Nom(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    pub fn append_expect(&mut self, exp: Vec<Expect<'s, C>>) {
        for exp in exp.into_iter() {
            self.hints.push(Hints::Expect(exp));
        }
    }

    pub fn add_suggest(&mut self, code: C, span: Span<'s>) {
        self.hints.push(Hints::Suggest(Suggest {
            code,
            span,
            parents: Vec::new(),
        }))
    }

    pub fn append_suggest(&mut self, sug: Vec<Suggest<'s, C>>) {
        for sug in sug.into_iter() {
            self.hints.push(Hints::Suggest(sug));
        }
    }

    pub fn expect(&self) -> Vec<&Expect<'s, C>> {
        self.hints
            .iter()
            .filter_map(|v| match v {
                Hints::Expect(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    /// Get Expect grouped by offset into the string, starting with max first.
    pub fn expect_grouped_by_offset(&self) -> Vec<(usize, Vec<&Expect<'s, C>>)> {
        Expect::group_by_offset(self.expect())
    }

    /// Get Expect grouped by offset into the string, starting with max first.
    pub fn expect_grouped_by_line(&self) -> Vec<(u32, Vec<&Expect<'s, C>>)> {
        Expect::group_by_line(self.expect())
    }

    pub fn suggest(&self) -> Vec<&Suggest<'s, C>> {
        self.hints
            .iter()
            .filter_map(|v| match v {
                Hints::Suggest(n) => Some(n),
                _ => None,
            })
            .collect()
    }

    /// Get Suggest grouped by offset into the string, starting with max first.
    pub fn suggest_grouped_by_offset(&self) -> Vec<(usize, Vec<&Suggest<'s, C>>)> {
        Suggest::group_by_offset(self.suggest())
    }

    /// Get Suggest grouped by offset into the string, starting with max first.
    pub fn suggest_grouped_by_line(&self) -> Vec<(u32, Vec<&Suggest<'s, C>>)> {
        Suggest::group_by_line(self.suggest())
    }
}

impl<'s, C: Code> Display for ParserError<'s, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} expects ", self.code)?;

        let expect = self.expect();
        for (i, exp) in expect.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(
                f,
                "{}:\"{}\"",
                exp.code,
                restrict(DebugWidth::Short, exp.span)
            )?;
        }
        // no suggest
        write!(
            f,
            " for span {} \"{}\"",
            self.span.location_offset(),
            restrict(DebugWidth::Short, self.span)
        )?;
        Ok(())
    }
}

impl<'s, C: Code> Error for ParserError<'s, C> {}

/// Coop with nom.
impl<'s, C: Code> nom::error::ParseError<Span<'s>> for ParserError<'s, C> {
    fn from_error_kind(span: Span<'s>, kind: ErrorKind) -> Self {
        ParserError {
            code: C::NOM_ERROR,
            span,
            tracing: false,
            hints: vec![Hints::Nom(Nom { kind, span })],
        }
    }

    fn append(input: Span<'s>, kind: ErrorKind, mut other: Self) -> Self {
        other.hints.push(Hints::Nom(Nom { kind, span: input }));
        other
    }
}

impl<'s, C> From<nom::Err<ParserError<'s, C>>> for ParserError<'s, C>
where
    C: Code,
{
    fn from(e: nom::Err<ParserError<'s, C>>) -> Self {
        match e {
            nom::Err::Error(e) => e,
            nom::Err::Failure(e) => e,
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}

impl<'s, C, O> IntoParserResultAddCode<'s, C, O> for ParserResult<'s, C, O>
where
    C: Code,
{
    fn into_with_code(self, code: C) -> ParserResult<'s, C, O> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_code(code)),
        }
    }
}

impl<'s, C> IntoParserError<'s, C> for nom::Err<ParserError<'s, C>>
where
    C: Code,
{
    fn into_with_code(self, code: C) -> ParserError<'s, C> {
        match self {
            nom::Err::Error(e) => e.into_code(code),
            nom::Err::Failure(e) => e.into_code(code),
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}

impl<'s, C, O> IntoParserResultAddCode<'s, C, O> for Result<O, nom::Err<ParserError<'s, C>>>
where
    C: Code,
{
    fn into_with_code(self, code: C) -> ParserResult<'s, C, O> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_with_code(code)),
        }
    }
}

impl<'s, C> From<nom::Err<nom::error::Error<Span<'s>>>> for ParserError<'s, C>
where
    C: Code,
{
    fn from(e: nom::Err<nom::error::Error<Span<'s>>>) -> Self {
        match e {
            nom::Err::Error(e) => ParserError::new_with_nom(C::NOM_ERROR, e.code, e.input),
            nom::Err::Failure(e) => ParserError::new_with_nom(C::NOM_FAILURE, e.code, e.input),
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}

impl<'s, C> IntoParserError<'s, C> for nom::Err<nom::error::Error<Span<'s>>>
where
    C: Code,
{
    fn into_with_code(self, code: C) -> ParserError<'s, C> {
        match self {
            nom::Err::Error(e) => ParserError::new_with_nom(code, e.code, e.input),
            nom::Err::Failure(e) => ParserError::new_with_nom(code, e.code, e.input),
            nom::Err::Incomplete(_) => unreachable!(),
        }
    }
}

impl<'s, C, O> IntoParserResultAddCode<'s, C, O>
    for Result<O, nom::Err<nom::error::Error<Span<'s>>>>
where
    C: Code,
{
    fn into_with_code(self, code: C) -> ParserResult<'s, C, O> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => Err(e.into_with_code(code)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DebugWidth {
    /// Debug flag, can be set with width=0.
    Short,
    /// Debug flag, can be set with width=1.
    Medium,
    /// Debug flag, can be set with width=2.
    Long,
}

pub enum Hints<'s, C: Code> {
    Nom(Nom<'s>),
    Suggest(Suggest<'s, C>),
    Expect(Expect<'s, C>),
}

/// Data gathered from nom.
#[derive(Clone)]
pub struct Nom<'s> {
    /// Errorkind ala nom
    pub kind: ErrorKind,
    /// Span
    pub span: Span<'s>,
}

/// Suggestions, optional tokens.
#[derive(Clone)]
pub struct Suggest<'s, C> {
    /// Code for the token.
    pub code: C,
    /// Span
    pub span: Span<'s>,
    /// Parser call stack.
    pub parents: Vec<C>,
}

impl<'s, C> Suggest<'s, C> {
    pub fn group_by_offset_owned<'a>(
        vec: &'a Vec<Suggest<'s, C>>,
    ) -> Vec<(usize, Vec<&'a Suggest<'s, C>>)> {
        Self::group_by_offset(vec.iter().collect())
    }

    /// Get Suggest grouped by offset into the string, starting with max first.
    pub fn group_by_offset<'a>(
        vec: Vec<&'a Suggest<'s, C>>,
    ) -> Vec<(usize, Vec<&'a Suggest<'s, C>>)> {
        let mut sorted = vec;
        sorted.reverse();
        sorted.sort_by(|a, b| b.span.location_offset().cmp(&a.span.location_offset()));

        // per offset
        let mut grp_offset = 0;
        let mut grp = Vec::new();
        let mut subgrp = Vec::new();
        for exp in &sorted {
            if exp.span.location_offset() != grp_offset {
                if !subgrp.is_empty() {
                    grp.push((grp_offset, subgrp));
                    subgrp = Vec::new();
                }
                grp_offset = exp.span.location_offset();
            }

            subgrp.push(*exp);
        }
        if !subgrp.is_empty() {
            grp.push((grp_offset, subgrp));
        }

        grp
    }

    pub fn group_by_line_owned<'a>(
        vec: &'a Vec<Suggest<'s, C>>,
    ) -> Vec<(u32, Vec<&'a Suggest<'s, C>>)> {
        Self::group_by_line(vec.iter().collect())
    }

    /// Get Suggest grouped by offset into the string, starting with max first.
    pub fn group_by_line<'a>(vec: Vec<&'a Suggest<'s, C>>) -> Vec<(u32, Vec<&'a Suggest<'s, C>>)> {
        let mut sorted = vec;
        sorted.reverse();
        sorted.sort_by(|a, b| b.span.location_offset().cmp(&a.span.location_offset()));

        // per offset
        let mut grp_line = 0;
        let mut grp = Vec::new();
        let mut subgrp = Vec::new();
        for exp in &sorted {
            if exp.span.location_line() != grp_line {
                if !subgrp.is_empty() {
                    grp.push((grp_line, subgrp));
                    subgrp = Vec::new();
                }
                grp_line = exp.span.location_line();
            }

            subgrp.push(*exp);
        }
        if !subgrp.is_empty() {
            grp.push((grp_line, subgrp));
        }

        grp
    }
}

/// Expected tokens.
#[derive(Clone)]
pub struct Expect<'s, C> {
    /// Code for the token.
    pub code: C,
    /// Span.
    pub span: Span<'s>,
    /// Parser call stack.
    pub parents: Vec<C>,
}

impl<'s, C> Expect<'s, C> {
    pub fn group_by_offset_owned<'a>(
        vec: &'a Vec<Expect<'s, C>>,
    ) -> Vec<(usize, Vec<&'a Expect<'s, C>>)> {
        Self::group_by_offset(vec.iter().collect())
    }

    /// Get Expect grouped by offset into the string, starting with max first.
    pub fn group_by_offset<'a>(
        vec: Vec<&'a Expect<'s, C>>,
    ) -> Vec<(usize, Vec<&'a Expect<'s, C>>)> {
        let mut sorted = vec;
        sorted.reverse();
        sorted.sort_by(|a, b| b.span.location_offset().cmp(&a.span.location_offset()));

        // per offset
        let mut grp_offset = 0;
        let mut grp = Vec::new();
        let mut subgrp = Vec::new();
        for exp in &sorted {
            if exp.span.location_offset() != grp_offset {
                if !subgrp.is_empty() {
                    grp.push((grp_offset, subgrp));
                    subgrp = Vec::new();
                }
                grp_offset = exp.span.location_offset();
            }

            subgrp.push(*exp);
        }
        if !subgrp.is_empty() {
            grp.push((grp_offset, subgrp));
        }

        grp
    }

    pub fn group_by_line_owned<'a>(
        vec: &'a Vec<Expect<'s, C>>,
    ) -> Vec<(u32, Vec<&'a Expect<'s, C>>)> {
        Self::group_by_line(vec.iter().collect())
    }

    /// Get Expect grouped by offset into the string, starting with max first.
    pub fn group_by_line<'a>(vec: Vec<&'a Expect<'s, C>>) -> Vec<(u32, Vec<&'a Expect<'s, C>>)> {
        let mut sorted = vec;
        sorted.reverse();
        sorted.sort_by(|a, b| b.span.location_offset().cmp(&a.span.location_offset()));

        // per offset
        let mut grp_line = 0;
        let mut grp = Vec::new();
        let mut subgrp = Vec::new();
        for exp in &sorted {
            if exp.span.location_line() != grp_line {
                if !subgrp.is_empty() {
                    grp.push((grp_line, subgrp));
                    subgrp = Vec::new();
                }
                grp_line = exp.span.location_line();
            }

            subgrp.push(*exp);
        }
        if !subgrp.is_empty() {
            grp.push((grp_line, subgrp));
        }

        grp
    }
}

impl From<Option<usize>> for DebugWidth {
    fn from(value: Option<usize>) -> Self {
        match value {
            None | Some(0) => DebugWidth::Short,
            Some(1) => DebugWidth::Medium,
            Some(2) => DebugWidth::Long,
            _ => DebugWidth::Short,
        }
    }
}
