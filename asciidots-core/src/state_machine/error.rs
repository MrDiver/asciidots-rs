use crate::{parser::Token, primitives::Position};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("The Program died Unexpectedly")]
pub enum RuntimeDiagnostic {
    None,
    #[error("The printing modifier encountered is invalid {}", char::from(*invalid))]
    #[diagnostic(help("Try: one of @, #, \"...\" or '...' "), severity(Warning))]
    InvalidPrintingModifier {
        invalid: Token,
        #[label("this here")]
        span: SourceSpan,
    },
    #[error(
        "The assertion failed around this position expected: {} found: {}",
        expected,
        found
    )]
    #[diagnostic(severity(Warning))]
    AssertionFailed {
        expected: f64,
        found: f64,
        #[label("here")]
        span: SourceSpan,
    },
    #[error(
        "The assertion could not be parsed because {} is not a valid float",
        accumulator
    )]
    #[diagnostic(severity(Warning))]
    FailedToParseAssert {
        accumulator: String,
        #[label("This is not a valid float")]
        span: SourceSpan,
    },
}

impl RuntimeDiagnostic {
    pub fn invalid_printing_modifier(invalid: Token, span: SourceSpan) -> Self {
        RuntimeDiagnostic::InvalidPrintingModifier { invalid, span }
    }

    pub fn assertion_failed(expected: f64, found: f64, span: SourceSpan) -> Self {
        RuntimeDiagnostic::AssertionFailed {
            expected,
            found,
            span,
        }
    }

    pub(crate) fn failed_to_parse_assert(accumulator: String, span: SourceSpan) -> Self {
        RuntimeDiagnostic::FailedToParseAssert { accumulator, span }
    }
}
