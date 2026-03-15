use crate::parser::Token;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("The Program died Unexpectedly")]
pub enum RuntimeError {
    None,
    #[error("The printing modifier encountered is invalid {}", char::from(*invalid))]
    #[diagnostic(help("Try: one of @, #, \"...\" or '...' "))]
    InvalidPrintingModifier {
        invalid: Token,
        #[label("this here")]
        span: SourceSpan,
    },
}

impl RuntimeError {
    pub fn invalid_printing_modifier(invalid: Token, span: SourceSpan) -> Self {
        RuntimeError::InvalidPrintingModifier { invalid, span }
    }
}
