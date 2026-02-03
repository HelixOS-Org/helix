//! Error handling for LUMINA derive macros.

use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use std::fmt;

/// Result type for macro operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Error type for shader macro processing.
#[derive(Debug)]
pub struct Error {
    /// Error kind.
    pub kind: ErrorKind,
    /// Span where the error occurred.
    pub span: Span,
    /// Additional notes.
    pub notes: Vec<String>,
    /// Help message.
    pub help: Option<String>,
}

/// Kinds of errors that can occur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// Syntax error in shader code.
    Syntax(String),
    /// Type error.
    Type(String),
    /// Unsupported feature.
    Unsupported(String),
    /// Invalid attribute.
    InvalidAttribute(String),
    /// Missing required attribute.
    MissingAttribute(String),
    /// Duplicate definition.
    Duplicate(String),
    /// Invalid shader stage.
    InvalidStage(String),
    /// Invalid binding.
    InvalidBinding(String),
    /// Layout error.
    Layout(String),
    /// Validation error.
    Validation(String),
    /// Internal error.
    Internal(String),
}

impl Error {
    /// Create a new error.
    pub fn new(kind: ErrorKind, span: Span) -> Self {
        Self {
            kind,
            span,
            notes: Vec::new(),
            help: None,
        }
    }

    /// Create a syntax error.
    pub fn syntax(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Syntax(message.into()), span)
    }

    /// Create a type error.
    pub fn type_error(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Type(message.into()), span)
    }

    /// Create an unsupported feature error.
    pub fn unsupported(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Unsupported(message.into()), span)
    }

    /// Create an invalid attribute error.
    pub fn invalid_attribute(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::InvalidAttribute(message.into()), span)
    }

    /// Create a missing attribute error.
    pub fn missing_attribute(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::MissingAttribute(message.into()), span)
    }

    /// Create a duplicate definition error.
    pub fn duplicate(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Duplicate(message.into()), span)
    }

    /// Create an invalid stage error.
    pub fn invalid_stage(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::InvalidStage(message.into()), span)
    }

    /// Create an invalid binding error.
    pub fn invalid_binding(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::InvalidBinding(message.into()), span)
    }

    /// Create a layout error.
    pub fn layout(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Layout(message.into()), span)
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Validation(message.into()), span)
    }

    /// Create an internal error.
    pub fn internal(message: impl Into<String>, span: Span) -> Self {
        Self::new(ErrorKind::Internal(message.into()), span)
    }

    /// Add a note to the error.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add a help message.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Convert to a compile error token stream.
    pub fn to_compile_error(&self) -> TokenStream {
        let message = self.to_string();
        let span = self.span;
        quote_spanned!(span => compile_error!(#message);)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::Syntax(msg) => write!(f, "syntax error: {}", msg),
            ErrorKind::Type(msg) => write!(f, "type error: {}", msg),
            ErrorKind::Unsupported(msg) => write!(f, "unsupported: {}", msg),
            ErrorKind::InvalidAttribute(msg) => write!(f, "invalid attribute: {}", msg),
            ErrorKind::MissingAttribute(msg) => write!(f, "missing attribute: {}", msg),
            ErrorKind::Duplicate(msg) => write!(f, "duplicate: {}", msg),
            ErrorKind::InvalidStage(msg) => write!(f, "invalid shader stage: {}", msg),
            ErrorKind::InvalidBinding(msg) => write!(f, "invalid binding: {}", msg),
            ErrorKind::Layout(msg) => write!(f, "layout error: {}", msg),
            ErrorKind::Validation(msg) => write!(f, "validation error: {}", msg),
            ErrorKind::Internal(msg) => write!(f, "internal error: {}", msg),
        }?;

        for note in &self.notes {
            write!(f, "\n  note: {}", note)?;
        }

        if let Some(help) = &self.help {
            write!(f, "\n  help: {}", help)?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {}

/// Extension trait for Option to convert to Error.
pub trait OptionExt<T> {
    /// Convert None to an error.
    fn ok_or_error(self, kind: ErrorKind, span: Span) -> Result<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_error(self, kind: ErrorKind, span: Span) -> Result<T> {
        self.ok_or_else(|| Error::new(kind, span))
    }
}

/// Diagnostic accumulator for multiple errors.
pub struct Diagnostics {
    errors: Vec<Error>,
    warnings: Vec<Warning>,
}

/// A warning message.
pub struct Warning {
    pub message: String,
    pub span: Span,
    pub notes: Vec<String>,
}

impl Diagnostics {
    /// Create a new diagnostics accumulator.
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add an error.
    pub fn error(&mut self, error: Error) {
        self.errors.push(error);
    }

    /// Add a warning.
    pub fn warning(&mut self, message: impl Into<String>, span: Span) {
        self.warnings.push(Warning {
            message: message.into(),
            span,
            notes: Vec::new(),
        });
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the number of errors.
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get the number of warnings.
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Convert to a compile error if there are any errors.
    pub fn to_result(&self) -> Result<()> {
        if let Some(error) = self.errors.first() {
            // Clone the first error for now
            Err(Error {
                kind: error.kind.clone(),
                span: error.span,
                notes: error.notes.clone(),
                help: error.help.clone(),
            })
        } else {
            Ok(())
        }
    }

    /// Generate compile errors for all errors.
    pub fn to_compile_errors(&self) -> TokenStream {
        let errors: Vec<_> = self.errors.iter().map(|e| e.to_compile_error()).collect();
        quote::quote! { #(#errors)* }
    }

    /// Take all errors.
    pub fn take_errors(&mut self) -> Vec<Error> {
        std::mem::take(&mut self.errors)
    }
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to create an error with the call site span.
#[macro_export]
macro_rules! error {
    ($kind:expr) => {
        $crate::error::Error::new($kind, proc_macro2::Span::call_site())
    };
    ($kind:expr, $span:expr) => {
        $crate::error::Error::new($kind, $span)
    };
}

/// Macro to bail with an error.
#[macro_export]
macro_rules! bail {
    ($kind:expr) => {
        return Err($crate::error::Error::new($kind, proc_macro2::Span::call_site()))
    };
    ($kind:expr, $span:expr) => {
        return Err($crate::error::Error::new($kind, $span))
    };
}

/// Macro to ensure a condition or bail.
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $kind:expr) => {
        if !$cond {
            $crate::bail!($kind);
        }
    };
    ($cond:expr, $kind:expr, $span:expr) => {
        if !$cond {
            $crate::bail!($kind, $span);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = Error::syntax("unexpected token", Span::call_site());
        assert!(error.to_string().contains("syntax error"));
    }

    #[test]
    fn test_error_with_notes() {
        let error = Error::type_error("mismatched types", Span::call_site())
            .with_note("expected Vec3")
            .with_note("found Vec4")
            .with_help("try converting with .xyz()");

        let msg = error.to_string();
        assert!(msg.contains("type error"));
        assert!(msg.contains("expected Vec3"));
        assert!(msg.contains("found Vec4"));
        assert!(msg.contains(".xyz()"));
    }

    #[test]
    fn test_diagnostics() {
        let mut diag = Diagnostics::new();
        assert!(!diag.has_errors());

        diag.error(Error::syntax("test", Span::call_site()));
        assert!(diag.has_errors());
        assert_eq!(diag.error_count(), 1);

        diag.warning("test warning", Span::call_site());
        assert_eq!(diag.warning_count(), 1);
    }
}
