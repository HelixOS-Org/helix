//! # AI-Powered Debugger
//!
//! Intelligent debugging assistance using pattern analysis.
//!
//! ## Key Features
//!
//! - **Automatic Root Cause Analysis**: Find the source of issues
//! - **Pattern Matching**: Recognize common bug patterns
//! - **Fix Suggestions**: Suggest potential fixes
//! - **Contextual Analysis**: Understand execution context
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `pattern`: Bug patterns and categories
//! - `context`: Debug context and stack frames
//! - `diagnosis`: Diagnosis and fix suggestions
//! - `debugger`: Main debugger implementation

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod pattern;
pub mod context;
pub mod diagnosis;
pub mod debugger;

// Re-export pattern types
pub use pattern::{BugCategory, BugPattern, BugSeverity};

// Re-export context types
pub use context::{DebugContext, StackFrame};

// Re-export diagnosis types
pub use diagnosis::{Diagnosis, Fix, FixType};

// Re-export debugger
pub use debugger::{Debugger, DebuggerStats};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComponentId;

    #[test]
    fn test_bug_pattern() {
        let pattern = BugPattern::new("Test Pattern", BugCategory::Memory)
            .with_symptom("test error")
            .with_fix("test fix");

        assert!(pattern.matches("This is a test error message"));
        assert!(!pattern.matches("Something else"));
    }

    #[test]
    fn test_debug_context() {
        let context = DebugContext::new("Test error")
            .with_component(ComponentId::MEMORY)
            .with_register("rax", 0x1234)
            .with_event("Something happened");

        assert_eq!(context.error, "Test error");
        assert_eq!(context.component, Some(ComponentId::MEMORY));
    }

    #[test]
    fn test_debugger() {
        let mut debugger = Debugger::new();

        let context = DebugContext::new("null pointer dereference at 0x0");
        let diagnosis = debugger.diagnose(&context);

        assert!(diagnosis.confidence > 0.3);
        assert!(diagnosis.pattern.is_some());
    }

    #[test]
    fn test_stack_analysis() {
        let debugger = Debugger::new();

        let frames = vec![
            StackFrame::new(0x1000, 0x7000, 0x7008).with_function("malloc"),
            StackFrame::new(0x2000, 0x7010, 0x7018).with_function("my_function"),
        ];

        let insights = debugger.analyze_stack(&frames);
        assert!(insights.iter().any(|s| s.contains("Memory allocation")));
    }
}
