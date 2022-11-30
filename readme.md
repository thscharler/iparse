
# IParse

Outline for a handwritten parser.

1. Define your function/error codes. They are used interchangeably.
   Add variants for nom::err::Error and nom::err::Failure to work with nom.
   Add a variant for incomplete parsing.

2. The error codes need Copy + Display + Debug + Eq 


```rust
pub enum ICode {
    INomError,
    INomFailure,
    IParseIncomplete,

    ITerminalA,
}
```

3. Add a type alias for the parser error.

```rust
pub type IParserError<'s> = ParserError<'s, ICode>;
```

4. Implement the parser in terms of the Parser trait.

```rust

pub struct TerminalA;

impl<'s> Parser<'s, AstA<'s>, ICode> for TerminalA {
   fn id() -> ICode {
      ICode::ITerminalA
   }

   fn lah() -> LookAhead {
      LookAhead::Parse
   }

   fn parse<'t>(
      trace: &'t impl Tracer<'s, ICode>,
      rest: Span<'s>
   ) -> ParseResult<'s, AstA<'s>, ICode> {
      trace.enter(Self::id(), rest);

      let (rest, token) = match tokens::parse_a(rest) {
         Ok((rest, token)) => (rest, token),
         Err(e) if e.code == ICode::ITerminalA => {
            trace.suggest(ICode::ITerminalA, e.span);
            return trace.err(e);
         }
         Err(e) => {
            return trace.err();
         }
      };

      trace.ok(token.span(), rest, AstA {
         span: token
      })
   }
}

struct AstA<'s> {
   span: Span<'s>
}

mod tokens {
   pub fn parse_a(rest: Span<'_>) -> ParseResult<'_, Span<'_>, ICode> {
      match nom_tokens::nom_parse_a(rest) {
         Ok((rest, token)) => Ok((rest, token)),
         Err(nom::Err::Error(e)) if e.code == ErrorKind::Tag => Err(
                 ParserError::new(ICode::ITerminalA, rest), 
                 ),
         Err(e) => Err(ParserError::nom(e)),
      }
   }
   
   mod nom_tokens {
      pub fn nom_parse_a(i: Span<'_>) -> IResult<Span<'_>, Span<'_>> {
         let (i, token) = tag("A")(i)?;
         Ok((i, token))
      }
   }
}
```

5. To call the parser use any impl of Tracer. The standard today is CTracer.


# Notes

## Note 1

There is IntoParserError that can be implemented to import external errors.

## Note 2

The Tracer trait keeps track of the call stack of parser functions.

