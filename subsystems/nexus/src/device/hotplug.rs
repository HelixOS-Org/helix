//! Hotplug Handler
//!
//! Device hotplug event management.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::DeviceId;

/// Hotplug event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HotplugEvent {
    /// Device added
    Add,
    /// Device removed
    Remove,
    /// Device changed
    Change,
    /// Device moved
    Move,
    /// Device online
    Online,
    /// Device offline
    Offline,
}

/// Hotplug notification
#[derive(Debug, Clone)]
pub struct HotplugNotification {
    /// Event type
    pub event: HotplugEvent,
    /// Device ID
    pub device_id: DeviceId,
    /// Device path
    pub device_path: String,
    /// Subsystem
    pub subsystem: String,
    /// Timestamp
    pub timestamp: u64,
    /// Properties
    pub properties: BTreeMap<String, String>,
}

/// Hotplug handler
pub struct HotplugHandler {
    /// Pending events
    pending_events: Vec<HotplugNotification>,
    /// Event history
    event_history: Vec<HotplugNotification>,
    /// Maximum history
    max_history: usize,
    /// Event count by type
    event_counts: BTreeMap<HotplugEvent, u64>,
    /// Subsystem handlers
    subsystem_handlers: BTreeMap<String, fn(&HotplugNotification)>,
    /// Event rate (events per second)
    event_rate: f32,
    /// Last rate calculation
    last_rate_calc: u64,
    /// Events since last calc
    events_since_calc: u64,
}

impl HotplugHandler {
    /// Create new hotplug handler
    pub fn new() -> Self {
        Self {
            pending_events: Vec::new(),
            event_history: Vec::with_capacity(1000),
            max_history: 1000,
            event_counts: BTreeMap::new(),
            subsystem_handlers: BTreeMap::new(),
            event_rate: 0.0,
            last_rate_calc: 0,
            events_since_calc: 0,
        }
    }

    /// Queue hotplug event
    pub fn queue_event(&mut self, notification: HotplugNotification) {
        *self.event_counts.entry(notification.event).or_default() += 1;
        self.events_since_calc += 1;
        self.pending_events.push(notification);
    }

    /// Process pending events
    pub fn process_events(&mut self) -> Vec<HotplugNotification> {
        let events: Vec<_> = self.pending_events.drain(..).collect();

        // Store in history
        for event in &events {
            if self.event_history.len() >= self.max_history {
                self.event_history.remove(0);
            }
            self.event_history.push(event.clone());
        }

        events
    }

    /// Get pending event count
    pub fn pending_count(&self) -> usize {
        self.pending_events.len()
    }

    /// Get event count by type
    pub fn event_count(&self, event: HotplugEvent) -> u64 {
        self.event_counts.get(&event).copied().unwrap_or(0)
    }

    /// Calculate event rate
    pub fn update_rate(&mut self, current_time: u64) {
        let elapsed = current_time.saturating_sub(self.last_rate_calc);
        if elapsed > 1_000_000_000 {
            // 1 second
            let elapsed_sec = elapsed as f32 / 1_000_000_000.0;
            self.event_rate = self.events_since_calc as f32 / elapsed_sec;
            self.events_since_calc = 0;
            self.last_rate_calc = current_time;
        }
    }

    /// Get current event rate
    pub fn event_rate(&self) -> f32 {
        self.event_rate
    }

    /// Find recent events for device
    pub fn device_events(&self, device_id: DeviceId, limit: usize) -> Vec<&HotplugNotification> {
        self.event_history
            .iter()
            .rev()
            .filter(|e| e.device_id == device_id)
            .take(limit)
            .collect()
    }

    /// Register subsystem handler
    pub fn register_handler(&mut self, subsystem: String, handler: fn(&HotplugNotification)) {
        self.subsystem_handlers.insert(subsystem, handler);
    }

    /// Get event history
    pub fn history(&self) -> &[HotplugNotification] {
        &self.event_history
    }
}

impl Default for HotplugHandler {
    fn default() -> Self {
        Self::new()
    }
}
