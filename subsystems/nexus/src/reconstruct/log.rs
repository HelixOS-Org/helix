//! State log for event sourcing.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::event::StateEvent;
use super::snapshot::StateSnapshot;
use crate::core::{ComponentId, NexusTimestamp};

/// Log of state events
pub struct StateLog {
    /// Events
    events: Vec<StateEvent>,
    /// Maximum events to keep
    max_events: usize,
    /// Snapshots by timestamp
    snapshots: BTreeMap<u64, StateSnapshot>,
    /// Snapshot interval
    snapshot_interval: usize,
    /// Events since last snapshot
    events_since_snapshot: usize,
}

impl StateLog {
    /// Create a new state log
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::new(),
            max_events,
            snapshots: BTreeMap::new(),
            snapshot_interval: 100,
            events_since_snapshot: 0,
        }
    }

    /// Set snapshot interval
    pub fn with_snapshot_interval(mut self, interval: usize) -> Self {
        self.snapshot_interval = interval;
        self
    }

    /// Append an event
    pub fn append(&mut self, mut event: StateEvent) {
        event.calculate_checksum();
        self.events.push(event);
        self.events_since_snapshot += 1;

        // Enforce max events
        while self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }

    /// Add a snapshot
    pub fn add_snapshot(&mut self, snapshot: StateSnapshot) {
        self.snapshots.insert(snapshot.timestamp.ticks(), snapshot);
        self.events_since_snapshot = 0;
    }

    /// Should create snapshot?
    pub fn should_snapshot(&self) -> bool {
        self.events_since_snapshot >= self.snapshot_interval
    }

    /// Get events since timestamp
    pub fn events_since(&self, timestamp: NexusTimestamp) -> Vec<&StateEvent> {
        self.events
            .iter()
            .filter(|e| e.timestamp.ticks() >= timestamp.ticks())
            .collect()
    }

    /// Get events for component
    pub fn events_for(&self, component: ComponentId) -> Vec<&StateEvent> {
        self.events
            .iter()
            .filter(|e| e.component == component)
            .collect()
    }

    /// Get latest snapshot before timestamp
    pub fn snapshot_before(&self, timestamp: NexusTimestamp) -> Option<&StateSnapshot> {
        self.snapshots
            .range(..timestamp.ticks())
            .next_back()
            .map(|(_, s)| s)
    }

    /// Get all events
    pub fn events(&self) -> &[StateEvent] {
        &self.events
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Clear
    pub fn clear(&mut self) {
        self.events.clear();
        self.snapshots.clear();
        self.events_since_snapshot = 0;
    }
}

impl Default for StateLog {
    fn default() -> Self {
        Self::new(10000)
    }
}
