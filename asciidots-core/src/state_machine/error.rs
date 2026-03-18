use crate::parser::AsciiCell;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("The Program died Unexpectedly")]
pub enum RuntimeDiagnostic {
    None,
    #[error(
        "The printing modifier encountered is invalid {}",
        char::from(_invalid)
    )]
    #[diagnostic(help("Try: one of @, #, \"...\" or '...' "), severity(Warning))]
    InvalidPrintingModifier {
        _invalid: AsciiCell,
        #[label("this here")]
        _span: SourceSpan,
    },
    #[error(
        "The assertion failed around this position expected: {} found: {}",
        _expected,
        _found
    )]
    #[diagnostic(severity(Warning))]
    AssertionFailed {
        _expected: f64,
        _found: f64,
        #[label("here")]
        span: SourceSpan,
    },
    #[error(
        "The assertion could not be parsed because {} is not a valid float",
        _accumulator
    )]
    #[diagnostic(severity(Warning))]
    FailedToParseAssert {
        _accumulator: String,
        #[label("This is not a valid float")]
        span: SourceSpan,
    },
}

impl RuntimeDiagnostic {
    pub fn invalid_printing_modifier(invalid: AsciiCell, span: SourceSpan) -> Self {
        RuntimeDiagnostic::InvalidPrintingModifier {
            _invalid: invalid,
            _span: span,
        }
    }

    pub fn assertion_failed(expected: f64, found: f64, span: SourceSpan) -> Self {
        RuntimeDiagnostic::AssertionFailed {
            _expected: expected,
            _found: found,
            span,
        }
    }

    pub(crate) fn failed_to_parse_assert(accumulator: String, span: SourceSpan) -> Self {
        RuntimeDiagnostic::FailedToParseAssert {
            _accumulator: accumulator,
            span,
        }
    }
}
