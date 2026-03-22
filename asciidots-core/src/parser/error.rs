use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum LexError {
    #[error("parse error: {}", _help)]
    #[diagnostic(code(asciidots::lex::parse_error))]
    LexingFailure {
        _help: String,
        #[label("here")]
        span: SourceSpan,
    },

    #[error("unexpected input")]
    #[diagnostic(code(asciidots::lex::unexpected_input), help("{}", _help))]
    UnexpectedInput {
        _help: String,
        #[label("here")]
        _span: SourceSpan,
    },

    #[error("unexpected end of file")]
    #[diagnostic(code(asciidots::lex::unexpected_eof))]
    UnexpectedEof {
        #[label("file ends here")]
        span: SourceSpan,
    },
    #[error("invalid whitespace found")]
    #[diagnostic(code(asciidots::lex::invalid_whitespace))]
    InvalidWhitespaceCharacter {
        c: char,
        #[label("Remove this {:#?}", c)]
        span: SourceSpan,
    },
}

#[derive(Error, Debug, Diagnostic)]
pub enum ParsingError {
    #[error("parse error: missing brace")]
    #[diagnostic(
        code(asciidots::parse::missing_brace),
        help("Try adding a '{}'", _expected)
    )]
    MissingOpeningBrace {
        _expected: char,
        #[label("here")]
        _span: SourceSpan,
    },
    #[error(
        "parse error: invalid whitespace character {}, only use spaces not tabs",
        _c
    )]
    #[diagnostic(
        code(asciidots::parse::invalid_whitespace),
        help("Try removing this whitespace and replacing with space")
    )]
    InvalidWhitespace {
        _c: char,
        #[label("here")]
        _span: SourceSpan,
    },
    #[error("parse error: a function call operator shall always be called with a name")]
    #[diagnostic(
        code(asciidots::parse::missing_name),
        help("Try adding a name next to the call operator")
    )]
    MissingNameForCallOperator {
        #[label("missing name")]
        _span: miette::SourceSpan,
    },
    #[error("parse error: You can not mix braces for operators, even if that would be cool :D")]
    #[diagnostic(
        code(asciidots::parse::mismatching_brace),
        help("Try changing the brace to be '{expected}'")
    )]
    MismatchingBrace {
        expected: char,

        #[label("mismatching brace expected '{expected}'")]
        _span: miette::SourceSpan,
    },
    #[error(
        "parse error: Missing destination for Warp, always have two warps in order for them to work"
    )]
    #[diagnostic(
        code(asciidots::parse::missing_destination),
        help("Try adding another warp, by adding '{}' somewhere on the map", _c)
    )]
    MissingDestinationForWarp {
        _c: char,
        #[label("Sink here")]
        _span: miette::SourceSpan,
    },
}

#[derive(Error, Debug, Diagnostic)]
#[error("Parsing Failures")]
pub struct ParsingErrorCollection {
    #[related]
    errors: Vec<ParsingError>,
}

impl ParsingErrorCollection {
    pub fn new(errors: Vec<ParsingError>) -> Self {
        Self { errors }
    }
}
