//! State event types for reconstruction.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::{ComponentId, NexusTimestamp};

/// An event that modifies state
#[derive(Debug, Clone)]
pub struct StateEvent {
    /// Unique ID
    pub id: u64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Component
    pub component: ComponentId,
    /// Event type
    pub event_type: StateEventType,
    /// Key affected
    pub key: String,
    /// Old value (if any)
    pub old_value: Option<Vec<u8>>,
    /// New value (if any)
    pub new_value: Option<Vec<u8>>,
    /// Checksum
    pub checksum: u64,
}

impl StateEvent {
    /// Create a new event
    pub fn new(component: ComponentId, event_type: StateEventType, key: impl Into<String>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: NexusTimestamp::now(),
            component,
            event_type,
            key: key.into(),
            old_value: None,
            new_value: None,
            checksum: 0,
        }
    }

    /// Set old value
    pub fn with_old_value(mut self, value: Vec<u8>) -> Self {
        self.old_value = Some(value);
        self
    }

    /// Set new value
    pub fn with_new_value(mut self, value: Vec<u8>) -> Self {
        self.new_value = Some(value);
        self
    }

    /// Calculate checksum
    pub fn calculate_checksum(&mut self) {
        let mut hash = 0xcbf29ce484222325u64; // FNV-1a offset basis

        hash ^= self.component.raw();
        hash = hash.wrapping_mul(0x100000001b3);

        for byte in self.key.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        if let Some(ref old) = self.old_value {
            for byte in old {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }

        if let Some(ref new) = self.new_value {
            for byte in new {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }

        self.checksum = hash;
    }

    /// Verify checksum
    pub fn verify_checksum(&self) -> bool {
        let mut event = self.clone();
        event.calculate_checksum();
        event.checksum == self.checksum
    }
}

/// Type of state event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateEventType {
    /// Create new entry
    Create,
    /// Update existing entry
    Update,
    /// Delete entry
    Delete,
    /// Snapshot
    Snapshot,
    /// Checkpoint
    Checkpoint,
}
