
# IParse

Outline for a handwritten parser.

_The code can be found as example1.rs._

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
pub type IParserResult<'s, O> = ParserResult<'s, ICode, (Span<'s>, O)>;
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
         Err(e.into_code(ICTerminalA))
      }
      Err(e) => Err(e.into()),
   }
}
```

4. Implement the parser in terms of the Parser trait.

The id() identifies the function in the call stack of the tracer. It acts as 
error code for the same function. For this to work call trace.enter() at the
start of the function and trace.ok() or trace.err() at each exit point.

There is more later.

```rust
pub struct ParseTerminalA;

impl<'s> Parser<'s, TerminalA<'s>, ICode> for ParseTerminalA {
   fn id() -> ICode {
      ICTerminalA
   }

   fn lah(_: Span<'s>) -> bool {
      true
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
```

5. To call the parser use any impl of Tracer. The standard today is CTracer.
The const type argument states wether the actual tracking will be done or not.

```rust
fn run_parser() -> IParserResult<'static, TerminalA<'static>> {
   let trace: CTracer<_, true> = CTracer::new();
   ParseTerminalA::parse(&trace, Span::new("A"))
}
```

6. Testing

Use iparse::test::Test. It has functions for nom::Error and ParseError to
test a single parser and check the results.

When calling q() a report type is needed. These are
* Dump - Output the error/success. Doesn't panic.
* CheckDump - Output the error/success. Panics if any of the test-fn failed.
* Trace - Output the complete trace. Doesn't panic.
* CheckTrace - Output the complete trace. Panics if any of the test-fn failed.
* Timing - Output only the timings. 

```rust
type R = Trace;

#[test]
pub fn test_terminal_a() {
    test_parse("A", ParseTerminalA::parse).okok().q::<R>();
    test_parse("AA", ParseTerminalA::parse).errerr().q::<R>();
}
 ```


# Appendix A

## Note 1

There is IntoParserResultAddSpan that can be implemented to import external errors.

```rust
impl<'s, T> IntoParserResultAddSpan<'s, ICode, T> for Result<T, ParseIntError> {
   fn into_with_span(self, span: Span<'s>) -> ParserResult<'s, ICode, T> {
      match self {
         Ok(v) => Ok(v),
         Err(_) => Err(ParserError::new(ICInteger, span)),
      }
   }
}
```

And to use it ...

```rust
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
               term: (*tok).parse::<u32>().into_with_span(tok).track(trace)?,
               span: tok,
            },
         ),
         Err(e) => return trace.err(e.into()),
      };

      trace.ok(rest, tok.span, tok)
   }
}
```

## Note 2

The trait iparse::tracer::TrackParseResult make the composition of parser 
easier. It provides a track() function for the parser result, that notes a 
potential error and returns the result. This in turn can be used for the ? 
operator. 

It has a second method track_as() that allows to change the error code.

```rust
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
```

## Note 3

It is good to have the full span for non-terminals in the parser. There is no
way to glue the spans together via nom, so there is span_union().

```rust
# fn dummy() {
   let span = span_union(a.span, b.span);
# }
```

## Note 4

Handling optional terms is almost as easy as non-optional ones.

With the function stash() the error can be stored somewhere, and will added 
later in case something else fails. This can add additional context to a later 
error. In the ok case everything is forgotten.


```rust
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
```

## Note 5

The trait ParseAsOptional allows to convert a Err(ParserError) to an 
Ok(Option<T>). This is the default way to mark a subparser as optional.




## Note 5

Repetition: TODO



# Appendix B

# Noteworthy 1

There are more conversion traits:
* IntoParserResultAddCode
* IntoParserError

The std::convert::Into is implemented for nom types to do a 
default conversion into ParserError.

# Noteworthy 2

There is a second tracer RTracer. It's only used to run experiments.
The same with NoTracer that simply does nothing. 

# Noteworthy 3

Besides span_union() there are also get_lines_before(), getlines_after()
and get_lines_around().