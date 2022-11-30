
# IParse

Outline for a handwritten parser.

1. Define your function/error codes. They are used interchangeably.
   Add variants for nom::err::Error and nom::err::Failure to work with nom.
   Add a variant for incomplete parsing.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ICode {
    ICNomError,
    ICNomFailure,
    ICParseIncomplete,

    ICTerminalA,
    ICInt
}
```

2. Mark it as trait Code. This needs Copy + Display + Debug + Eq 

```rust
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
      };
      write!(f, "{}", name)
   }
}
```

3. Add a type alias for the Result type of the parser fn and the nom parser fn.

```rust
pub type IParserResult<'s, O> = ParseResult<'s, O, ICode>;
pub type INomResult<'s> = ParserNomResult<'s, ICode>;
```

4. Define the AST structs. There are no constraints from IParse.

```rust
pub struct TerminalA<'s> {
   pub term: String,
   pub span: Span<'s>,
}
```

5. Create the nom parsers for your terminals. 

```rust
pub fn nom_parse_a(i: Span<'_>) -> INomResult<'_> {
   tag("A")(i)
}
```

6. Create a transform fn for each nom fn. This translates the nom errors to our parsers errors.
This is also a good point for conversions from string.

```rust
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
```

4. Implement the parser in terms of the Parser trait.

```rust
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
```

5. To call the parser use any impl of Tracer. The standard today is CTracer.

```rust
fn main() {
   let trace = CTracer::new();
   let res = ParseTerminalA::parse(&trace, Span::new("A"));
   dbg!(&res);
}
```

6. Testing

Use iparse::test::Test. It has functions for nom::Error and ParseError to
test a single parser and check the results.

```
#[test]
pub fn test_terminal_a() {
    Test::parse("A", ParseTerminalA::parse)
        .okok()
        .q::<CheckTrace>();
    Test::parse("AA", ParseTerminalA::parse)
        .errerr()
        .q::<CheckTrace>();
 }
 ```

# Notes

## Note 1

There is IntoParserError that can be implemented to import external errors.

```rust
   impl<'s, T> IntoParserError<'s, ICode, T> for Result<T, ParseIntError> {
       fn into_parser_err(self, span: Span<'s>) -> Result<T, IParserError<'s>> {
           match self {
               Ok(v) => Ok(v),
               Err(_) => Err(IParserError::new(ICInt, span)),
           }
       }
   }
```

## Note 2
