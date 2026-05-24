use miette::{Diagnostic, Report, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic, Clone)]
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

#[derive(Error, Debug, Diagnostic, Clone)]
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
    #[error("parse error: unknown character '{ch}'")]
    #[diagnostic(
        code(asciidots::parse::unknown_character),
        help("Remove or replace this character")
    )]
    UnknownCharacter {
        ch: char,
        #[label("unknown character")]
        _span: miette::SourceSpan,
    },
}

#[derive(Error, Debug, Diagnostic, Clone)]
#[error("Parsing Failures")]
pub struct ParsingErrorCollection {
    #[related]
    errors: Vec<ParsingError>,
}

impl ParsingErrorCollection {
    pub fn new(errors: Vec<ParsingError>) -> Self {
        Self { errors }
    }

    pub fn errors(&self) -> &[ParsingError] {
        &self.errors
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("Linking Failures")]
pub struct LinkingErrorCollection {
    #[related]
    errors: Vec<Report>,
}

impl LinkingErrorCollection {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn from_report(report: impl Into<Report>) -> Self {
        Self {
            errors: vec![report.into()],
        }
    }

    pub fn push_error(&mut self, report: Report) {
        self.errors.push(report);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for LinkingErrorCollection {
    fn default() -> Self {
        Self::new()
    }
}
