//! Lifecycle Tracker
//!
//! Tracking kobject lifecycle events.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use super::KobjectId;

/// Lifecycle event
#[derive(Debug, Clone)]
pub struct LifecycleEvent {
    /// Kobject ID
    pub kobject: KobjectId,
    /// Event type
    pub event_type: LifecycleEventType,
    /// Timestamp
    pub timestamp: u64,
    /// Additional info
    pub info: String,
}

/// Lifecycle event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEventType {
    /// Object created
    Created,
    /// Object added to parent
    Added,
    /// Object registered in sysfs
    Registered,
    /// Object deleted from parent
    Deleted,
    /// Object unregistered from sysfs
    Unregistered,
    /// Object released
    Released,
}

/// Lifecycle tracker
pub struct LifecycleTracker {
    /// Lifecycle events
    events: VecDeque<LifecycleEvent>,
    /// Maximum events
    max_events: usize,
    /// Per-kobject events
    per_object: BTreeMap<KobjectId, Vec<LifecycleEvent>>,
    /// Object creation times
    creation_times: BTreeMap<KobjectId, u64>,
    /// Object lifetimes (for released objects)
    lifetimes: Vec<(KobjectId, u64)>,
}

impl LifecycleTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            events: Vec::with_capacity(10000),
            max_events: 10000,
            per_object: BTreeMap::new(),
            creation_times: BTreeMap::new(),
            lifetimes: Vec::new(),
        }
    }

    /// Record lifecycle event
    pub fn record_event(
        &mut self,
        kobject: KobjectId,
        event_type: LifecycleEventType,
        timestamp: u64,
        info: String,
    ) {
        let event = LifecycleEvent {
            kobject,
            event_type,
            timestamp,
            info,
        };

        // Track creation time
        if event_type == LifecycleEventType::Created {
            self.creation_times.insert(kobject, timestamp);
        }

        // Calculate lifetime on release
        if event_type == LifecycleEventType::Released {
            if let Some(created) = self.creation_times.remove(&kobject) {
                let lifetime = timestamp.saturating_sub(created);
                self.lifetimes.push((kobject, lifetime));
            }
        }

        // Store event
        if self.events.len() >= self.max_events {
            self.events.pop_front();
        }
        self.events.push_back(event.clone());

        self.per_object.entry(kobject).or_default().push(event);
    }

    /// Get events for kobject
    #[inline(always)]
    pub fn get_events(&self, kobject: KobjectId) -> Option<&[LifecycleEvent]> {
        self.per_object.get(&kobject).map(|v| v.as_slice())
    }

    /// Get average lifetime
    #[inline]
    pub fn average_lifetime(&self) -> u64 {
        if self.lifetimes.is_empty() {
            return 0;
        }
        let sum: u64 = self.lifetimes.iter().map(|(_, l)| *l).sum();
        sum / self.lifetimes.len() as u64
    }

    /// Get recent events
    #[inline(always)]
    pub fn recent_events(&self, limit: usize) -> &[LifecycleEvent] {
        let start = self.events.len().saturating_sub(limit);
        &self.events[start..]
    }

    /// Get long-lived objects
    pub fn long_lived_objects(
        &self,
        threshold_ns: u64,
        current_time: u64,
    ) -> Vec<(KobjectId, u64)> {
        self.creation_times
            .iter()
            .filter_map(|(id, created)| {
                let age = current_time.saturating_sub(*created);
                if age > threshold_ns {
                    Some((*id, age))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for LifecycleTracker {
    fn default() -> Self {
        Self::new()
    }
}
