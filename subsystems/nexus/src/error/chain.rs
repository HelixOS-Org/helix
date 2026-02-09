//! Error chain for context propagation

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::NexusError;

/// Chain of errors for context
#[derive(Debug, Clone)]
pub struct ErrorChain {
    /// Errors in the chain (most recent first)
    pub errors: Vec<NexusError>,
    /// Context messages
    pub context: Vec<String>,
}

impl ErrorChain {
    /// Create a new error chain
    pub fn new(error: NexusError) -> Self {
        Self {
            errors: alloc::vec![error],
            context: Vec::new(),
        }
    }

    /// Add context to the chain
    #[inline(always)]
    pub fn context(mut self, msg: impl Into<String>) -> Self {
        self.context.push(msg.into());
        self
    }

    /// Chain another error
    #[inline(always)]
    pub fn chain(mut self, error: NexusError) -> Self {
        self.errors.push(error);
        self
    }

    /// Get the root cause
    #[inline(always)]
    pub fn root_cause(&self) -> Option<&NexusError> {
        self.errors.last()
    }

    /// Get the most recent error
    #[inline(always)]
    pub fn current(&self) -> Option<&NexusError> {
        self.errors.first()
    }
}

impl fmt::Display for ErrorChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(current) = self.current() {
            write!(f, "{}", current)?;
        }
        for ctx in &self.context {
            write!(f, "\n  Context: {}", ctx)?;
        }
        if self.errors.len() > 1 {
            write!(f, "\n  Caused by:")?;
            for (i, err) in self.errors.iter().skip(1).enumerate() {
                write!(f, "\n    {}: {}", i + 1, err)?;
            }
        }
        Ok(())
    }
}
