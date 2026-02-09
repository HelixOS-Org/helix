//! State reconstruction engine.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::event::{StateEvent, StateEventType};
use super::log::StateLog;
use super::snapshot::StateSnapshot;
use crate::core::{ComponentId, NexusTimestamp};
use crate::error::{HealingError, NexusResult};

/// State reconstruction engine
#[repr(align(64))]
pub struct StateReconstructor {
    /// State logs by component
    logs: BTreeMap<u64, StateLog>,
    /// Reconstructed states
    states: BTreeMap<u64, BTreeMap<alloc::string::String, Vec<u8>>>,
    /// Maximum logs to keep
    max_logs: usize,
    /// Verification enabled
    verify: bool,
}

impl StateReconstructor {
    /// Create a new reconstructor
    pub fn new() -> Self {
        Self {
            logs: BTreeMap::new(),
            states: BTreeMap::new(),
            max_logs: 100,
            verify: true,
        }
    }

    /// Enable/disable verification
    #[inline(always)]
    pub fn with_verification(mut self, enabled: bool) -> Self {
        self.verify = enabled;
        self
    }

    /// Get or create log for component
    #[inline(always)]
    pub fn log_for(&mut self, component: ComponentId) -> &mut StateLog {
        self.logs.entry(component.raw()).or_default()
    }

    /// Record an event
    #[inline(always)]
    pub fn record(&mut self, event: StateEvent) {
        let component = event.component;
        self.log_for(component).append(event);
    }

    /// Record a state change
    pub fn record_change(
        &mut self,
        component: ComponentId,
        key: impl Into<alloc::string::String>,
        old_value: Option<Vec<u8>>,
        new_value: Option<Vec<u8>>,
    ) {
        let event_type = match (&old_value, &new_value) {
            (None, Some(_)) => StateEventType::Create,
            (Some(_), Some(_)) => StateEventType::Update,
            (Some(_), None) => StateEventType::Delete,
            (None, None) => return,
        };

        let mut event = StateEvent::new(component, event_type, key);
        if let Some(old) = old_value {
            event = event.with_old_value(old);
        }
        if let Some(new) = new_value {
            event = event.with_new_value(new);
        }

        self.record(event);
    }

    /// Take a snapshot
    #[inline]
    pub fn snapshot(
        &mut self,
        component: ComponentId,
        state: BTreeMap<alloc::string::String, Vec<u8>>,
    ) {
        let mut snapshot = StateSnapshot::new(component);
        snapshot.state = state;
        snapshot.calculate_checksum();

        self.log_for(component).add_snapshot(snapshot);
    }

    /// Reconstruct state at a given timestamp
    pub fn reconstruct(
        &self,
        component: ComponentId,
        timestamp: NexusTimestamp,
    ) -> NexusResult<BTreeMap<alloc::string::String, Vec<u8>>> {
        let log = self
            .logs
            .get(&component.raw())
            .ok_or_else(|| HealingError::ReconstructionFailed("No log found".into()))?;

        // Start from latest snapshot before timestamp
        let mut state = if let Some(snapshot) = log.snapshot_before(timestamp) {
            if self.verify && !snapshot.verify_checksum() {
                return Err(
                    HealingError::ReconstructionFailed("Snapshot checksum failed".into()).into(),
                );
            }
            snapshot.state.clone()
        } else {
            BTreeMap::new()
        };

        // Get start timestamp
        let start = log
            .snapshot_before(timestamp)
            .map(|s| s.timestamp)
            .unwrap_or_else(|| NexusTimestamp::from_ticks(0));

        // Apply events in order
        for event in log.events() {
            if event.timestamp.ticks() > start.ticks()
                && event.timestamp.ticks() <= timestamp.ticks()
            {
                if self.verify && !event.verify_checksum() {
                    return Err(
                        HealingError::ReconstructionFailed("Event checksum failed".into()).into(),
                    );
                }

                match event.event_type {
                    StateEventType::Create | StateEventType::Update => {
                        if let Some(ref value) = event.new_value {
                            state.insert(event.key.clone(), value.clone());
                        }
                    },
                    StateEventType::Delete => {
                        state.remove(&event.key);
                    },
                    StateEventType::Snapshot | StateEventType::Checkpoint => {
                        // Handled via snapshots
                    },
                }
            }
        }

        Ok(state)
    }

    /// Reconstruct current state
    #[inline(always)]
    pub fn reconstruct_current(
        &self,
        component: ComponentId,
    ) -> NexusResult<BTreeMap<alloc::string::String, Vec<u8>>> {
        self.reconstruct(component, NexusTimestamp::now())
    }

    /// Verify reconstruction matches expected state
    #[inline(always)]
    pub fn verify_state(
        &self,
        component: ComponentId,
        expected: &BTreeMap<alloc::string::String, Vec<u8>>,
    ) -> NexusResult<bool> {
        let reconstructed = self.reconstruct_current(component)?;
        Ok(reconstructed == *expected)
    }

    /// Get log for component
    #[inline(always)]
    pub fn get_log(&self, component: ComponentId) -> Option<&StateLog> {
        self.logs.get(&component.raw())
    }

    /// Clear log for component
    #[inline(always)]
    pub fn clear(&mut self, component: ComponentId) {
        self.logs.remove(&component.raw());
        self.states.remove(&component.raw());
    }

    /// Clear all logs
    #[inline(always)]
    pub fn clear_all(&mut self) {
        self.logs.clear();
        self.states.clear();
    }
}

impl Default for StateReconstructor {
    fn default() -> Self {
        Self::new()
    }
}
