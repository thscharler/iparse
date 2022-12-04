use iparse::error::{Expect, Hints, Nom, ParserError, Suggest};
use iparse::{Code, Span};
use std::fmt::{Display, Formatter};
use std::mem::{align_of, size_of};

#[test]
pub fn sizes() {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum XCode {
        Nom,
    }

    impl Code for XCode {
        const NOM_ERROR: Self = Self::Nom;
        const NOM_FAILURE: Self = Self::Nom;
        const PARSE_INCOMPLETE: Self = Self::Nom;
    }

    impl Display for XCode {
        fn fmt(&self, _f: &mut Formatter<'_>) -> std::fmt::Result {
            todo!()
        }
    }

    dbg!(size_of::<Nom<'_>>());
    dbg!(size_of::<Suggest<'_, XCode>>());
    dbg!(size_of::<Expect<'_, XCode>>());
    dbg!(size_of::<ParserError<'_, XCode>>());
    dbg!(size_of::<nom::Err<nom::error::Error<Span<'_>>>>());
    dbg!(size_of::<nom::Err<nom::error::VerboseError<Span<'_>>>>());

    dbg!(align_of::<XCode>());
    dbg!(align_of::<Span<'_>>());
    dbg!(align_of::<bool>());
    dbg!(align_of::<Vec<Hints<'_, XCode>>>());
}
