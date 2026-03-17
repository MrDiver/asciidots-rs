use crate::{parser::Token, primitives::Orientation};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic, Clone)]
pub enum LexError {
    #[error("missing opening brace for operator found '{}' instead", _ch_found)]
    #[diagnostic(help("operators need braces: try '{}'", _expected_full))]
    MissingOpeningBrace {
        _ch_expected: char,
        _ch_found: char,
        _expected_full: String,
        #[label("Expected '{_ch_expected}' here")]
        _at_span: SourceSpan,
        #[label("operator")]
        _op_span: SourceSpan,
    },
    #[error("missing closing brace for operator found '{}' instead", _ch_found)]
    #[diagnostic(help("operators need braces: try '{}'", _expected_full))]
    MissingClosingBrace {
        _ch_expected: char,
        _ch_found: char,
        _expected_full: String,
        #[label("Expected '{_ch_expected}' here")]
        _at_span: SourceSpan,
        #[label("operator")]
        _op_span: SourceSpan,
    },
    #[error("missing condition path (guaranteed deadlock)")]
    #[diagnostic(help(""))]
    MissingConditionalPath {
        #[label("There needs to be a path here")]
        _at_span: SourceSpan,
    },
}

impl LexError {
    pub fn missing_opening_brace(
        found: char,
        at_span: SourceSpan,
        op_span: SourceSpan,
        orientation: Orientation,
    ) -> Self {
        LexError::MissingOpeningBrace {
            _ch_expected: Token::OperatorBegin(orientation).into(),
            _ch_found: found,
            _expected_full: format!(
                "{}?{}",
                char::from(Token::OperatorBegin(orientation)),
                char::from(Token::OperatorEnd(orientation))
            ),
            _at_span: at_span,
            _op_span: op_span,
        }
    }

    pub fn missing_closing_brace(
        found: char,
        at_span: SourceSpan,
        op_span: SourceSpan,
        orientation: Orientation,
    ) -> Self {
        LexError::MissingClosingBrace {
            _ch_expected: Token::OperatorEnd(orientation).into(),
            _ch_found: found,
            _expected_full: format!(
                "{}?{}",
                char::from(Token::OperatorBegin(orientation)),
                char::from(Token::OperatorEnd(orientation))
            ),
            _at_span: at_span,
            _op_span: op_span,
        }
    }

    pub fn missing_conditional_path(at_span: SourceSpan) -> Self {
        LexError::MissingConditionalPath { _at_span: at_span }
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("{} problem(s) found", _errors.len())]
pub struct LexErrors {
    #[source_code]
    pub _src: miette::NamedSource<String>,
    #[related]
    pub _errors: Vec<LexError>,
}
