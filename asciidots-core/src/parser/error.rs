use crate::{parser::Token, primitives::Orientation};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic, Clone)]
pub enum LexError {
    #[error("missing opening brace for operator found '{ch_found}' instead")]
    #[diagnostic(help("operators need braces: try '{expected_full}'"))]
    MissingOpeningBrace {
        ch_expected: char,
        ch_found: char,
        expected_full: String,
        #[label("Expected '{ch_expected}' here")]
        at_span: SourceSpan,
        #[label("operator")]
        op_span: SourceSpan,
    },
    #[error("missing closing brace for operator found '{ch_found}' instead")]
    #[diagnostic(help("operators need braces: try '{expected_full}'"))]
    MissingClosingBrace {
        ch_expected: char,
        ch_found: char,
        expected_full: String,
        #[label("Expected '{ch_expected}' here")]
        at_span: SourceSpan,
        #[label("operator")]
        op_span: SourceSpan,
    },
    #[error("missing condition path (guaranteed deadlock)")]
    #[diagnostic(help(""))]
    MissingConditionalPath {
        #[label("There needs to be a path here")]
        at_span: SourceSpan,
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
            ch_expected: Token::OperatorBegin(orientation).into(),
            ch_found: found,
            expected_full: format!(
                "{}?{}",
                char::from(Token::OperatorBegin(orientation)),
                char::from(Token::OperatorEnd(orientation))
            ),
            at_span,
            op_span,
        }
    }

    pub fn missing_closing_brace(
        found: char,
        at_span: SourceSpan,
        op_span: SourceSpan,
        orientation: Orientation,
    ) -> Self {
        LexError::MissingClosingBrace {
            ch_expected: Token::OperatorEnd(orientation).into(),
            ch_found: found,
            expected_full: format!(
                "{}?{}",
                char::from(Token::OperatorBegin(orientation)),
                char::from(Token::OperatorEnd(orientation))
            ),
            at_span,
            op_span,
        }
    }

    pub fn missing_conditional_path(at_span: SourceSpan) -> Self {
        LexError::MissingConditionalPath { at_span }
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("{} problem(s) found", errors.len())]
pub struct LexErrors {
    #[source_code]
    pub src: miette::NamedSource<String>,
    #[related]
    pub errors: Vec<LexError>,
}
