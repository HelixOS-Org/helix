//! Causal node definitions

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::{ComponentId, NexusTimestamp};
use crate::trace::{SpanId, TraceId};

// ============================================================================
// CAUSAL NODE TYPE
// ============================================================================

/// Type of causal node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CausalNodeType {
    /// System event
    Event,
    /// Function/operation
    Operation,
    /// Error/exception
    Error,
    /// State change
    StateChange,
    /// Message send
    Send,
    /// Message receive
    Receive,
    /// Lock acquire
    LockAcquire,
    /// Lock release
    LockRelease,
    /// Interrupt
    Interrupt,
    /// Checkpoint
    Checkpoint,
}

// ============================================================================
// CAUSAL NODE
// ============================================================================

/// A node in the causal graph
#[derive(Debug, Clone)]
pub struct CausalNode {
    /// Unique node ID
    pub id: u64,
    /// Node type
    pub node_type: CausalNodeType,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Component
    pub component: Option<ComponentId>,
    /// Related span
    pub span_id: Option<SpanId>,
    /// Related trace
    pub trace_id: Option<TraceId>,
    /// Node name/description
    pub name: String,
    /// Duration (if applicable)
    pub duration: Option<u64>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

impl CausalNode {
    /// Create a new node
    pub fn new(node_type: CausalNodeType, name: impl Into<String>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            node_type,
            timestamp: NexusTimestamp::now(),
            component: None,
            span_id: None,
            trace_id: None,
            name: name.into(),
            duration: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Set component
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.component = Some(component);
        self
    }

    /// Set span
    pub fn with_span(mut self, span_id: SpanId) -> Self {
        self.span_id = Some(span_id);
        self
    }

    /// Set trace
    pub fn with_trace(mut self, trace_id: TraceId) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
