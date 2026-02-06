//! RAII span guard

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

use super::span::{Span, SpanValue};
use super::tracer::Tracer;

// ============================================================================
// SPAN GUARD
// ============================================================================

/// RAII guard for automatic span ending
pub struct SpanGuard<'a> {
    tracer: &'a mut Tracer,
    span: Span,
}

impl<'a> SpanGuard<'a> {
    /// Create a new span guard
    pub fn new(tracer: &'a mut Tracer, span: Span) -> Self {
        tracer.start_span(&span);
        Self { tracer, span }
    }

    /// Add an event to the span
    pub fn add_event(&mut self, name: &'static str) {
        self.span.add_event(name);
    }

    /// Add an attribute
    pub fn add_attribute(&mut self, key: impl Into<String>, value: impl Into<SpanValue>) {
        self.span.attributes.push((key.into(), value.into()));
    }
}

impl Drop for SpanGuard<'_> {
    fn drop(&mut self) {
        self.span.end();
        self.tracer.end_span(&self.span);
    }
}
