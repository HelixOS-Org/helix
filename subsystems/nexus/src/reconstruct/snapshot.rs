//! State snapshots for reconstruction.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::{ComponentId, NexusTimestamp};

/// A snapshot of state at a point in time
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StateSnapshot {
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Component
    pub component: ComponentId,
    /// State data
    pub state: BTreeMap<String, Vec<u8>>,
    /// Checksum of entire state
    pub checksum: u64,
}

impl StateSnapshot {
    /// Create a new snapshot
    pub fn new(component: ComponentId) -> Self {
        Self {
            timestamp: NexusTimestamp::now(),
            component,
            state: BTreeMap::new(),
            checksum: 0,
        }
    }

    /// Set a value
    #[inline(always)]
    pub fn set(&mut self, key: impl Into<String>, value: Vec<u8>) {
        self.state.insert(key.into(), value);
    }

    /// Get a value
    #[inline(always)]
    pub fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.state.get(key)
    }

    /// Calculate checksum
    pub fn calculate_checksum(&mut self) {
        let mut hash = 0xcbf29ce484222325u64;

        for (key, value) in &self.state {
            for byte in key.bytes() {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            for byte in value {
                hash ^= *byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }

        self.checksum = hash;
    }

    /// Verify checksum
    #[inline]
    pub fn verify_checksum(&self) -> bool {
        let mut snapshot = self.clone();
        snapshot.calculate_checksum();
        snapshot.checksum == self.checksum
    }
}
