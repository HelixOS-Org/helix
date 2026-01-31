//! Debug context and stack frames

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::{ComponentId, NexusTimestamp};

// ============================================================================
// STACK FRAME
// ============================================================================

/// A stack frame
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// Instruction pointer
    pub ip: u64,
    /// Stack pointer
    pub sp: u64,
    /// Base pointer
    pub bp: u64,
    /// Function name (if known)
    pub function: Option<String>,
    /// File name (if known)
    pub file: Option<String>,
    /// Line number (if known)
    pub line: Option<u32>,
}

impl StackFrame {
    /// Create a new frame
    pub fn new(ip: u64, sp: u64, bp: u64) -> Self {
        Self {
            ip,
            sp,
            bp,
            function: None,
            file: None,
            line: None,
        }
    }

    /// Set function name
    pub fn with_function(mut self, name: impl Into<String>) -> Self {
        self.function = Some(name.into());
        self
    }

    /// Set file info
    pub fn with_location(mut self, file: impl Into<String>, line: u32) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self
    }
}

// ============================================================================
// DEBUG CONTEXT
// ============================================================================

/// Context for debugging
#[derive(Debug, Clone)]
pub struct DebugContext {
    /// Error message
    pub error: String,
    /// Component involved
    pub component: Option<ComponentId>,
    /// Stack trace (if available)
    pub stack_trace: Vec<StackFrame>,
    /// Register state (if available)
    pub registers: BTreeMap<String, u64>,
    /// Memory state around crash
    pub memory_context: Vec<(u64, Vec<u8>)>,
    /// Recent events
    pub recent_events: Vec<String>,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

impl DebugContext {
    /// Create a new debug context
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            component: None,
            stack_trace: Vec::new(),
            registers: BTreeMap::new(),
            memory_context: Vec::new(),
            recent_events: Vec::new(),
            timestamp: NexusTimestamp::now(),
        }
    }

    /// Set component
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.component = Some(component);
        self
    }

    /// Add stack frame
    pub fn with_frame(mut self, frame: StackFrame) -> Self {
        self.stack_trace.push(frame);
        self
    }

    /// Add register
    pub fn with_register(mut self, name: impl Into<String>, value: u64) -> Self {
        self.registers.insert(name.into(), value);
        self
    }

    /// Add recent event
    pub fn with_event(mut self, event: impl Into<String>) -> Self {
        self.recent_events.push(event.into());
        self
    }
}
