//! Tracepoint Definition
//!
//! Tracepoint and event field definitions.

use alloc::string::String;
use alloc::vec::Vec;

use super::{FieldType, TracepointId, TracepointState, TracepointSubsystem};

/// Event field definition
#[derive(Debug, Clone)]
pub struct EventField {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Offset in event data
    pub offset: usize,
    /// Size in bytes
    pub size: usize,
    /// Is signed
    pub is_signed: bool,
    /// Array element count (if array)
    pub array_count: Option<usize>,
}

impl EventField {
    /// Create new event field
    pub fn new(name: String, field_type: FieldType, offset: usize, size: usize) -> Self {
        Self {
            name,
            field_type,
            offset,
            size,
            is_signed: matches!(
                field_type,
                FieldType::S8 | FieldType::S16 | FieldType::S32 | FieldType::S64
            ),
            array_count: None,
        }
    }

    /// Create array field
    #[inline]
    pub fn array(
        name: String,
        element_type: FieldType,
        offset: usize,
        element_size: usize,
        count: usize,
    ) -> Self {
        let _ = element_type; // Used for documentation
        Self {
            name,
            field_type: FieldType::Array,
            offset,
            size: element_size * count,
            is_signed: false,
            array_count: Some(count),
        }
    }
}

/// Tracepoint definition
#[derive(Debug, Clone)]
pub struct TracepointDef {
    /// Tracepoint ID
    pub id: TracepointId,
    /// Tracepoint name
    pub name: String,
    /// Subsystem
    pub subsystem: TracepointSubsystem,
    /// Current state
    pub state: TracepointState,
    /// Event format fields
    pub fields: Vec<EventField>,
    /// Event size
    pub event_size: usize,
    /// Registration timestamp
    pub registered_at: u64,
    /// Probe count
    pub probe_count: u32,
}

impl TracepointDef {
    /// Create new tracepoint definition
    pub fn new(
        id: TracepointId,
        name: String,
        subsystem: TracepointSubsystem,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            name,
            subsystem,
            state: TracepointState::Disabled,
            fields: Vec::new(),
            event_size: 0,
            registered_at: timestamp,
            probe_count: 0,
        }
    }

    /// Add field
    #[inline]
    pub fn add_field(&mut self, field: EventField) {
        let end = field.offset + field.size;
        if end > self.event_size {
            self.event_size = end;
        }
        self.fields.push(field);
    }

    /// Is enabled
    #[inline(always)]
    pub fn is_enabled(&self) -> bool {
        matches!(self.state, TracepointState::Enabled)
    }

    /// Has probes
    #[inline(always)]
    pub fn has_probes(&self) -> bool {
        self.probe_count > 0
    }

    /// Get field by name
    #[inline(always)]
    pub fn get_field(&self, name: &str) -> Option<&EventField> {
        self.fields.iter().find(|f| f.name == name)
    }
}
