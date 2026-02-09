//! Device Power Management
//!
//! Power state management and optimization.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{DeviceId, PowerState};

/// Power transition
#[derive(Debug, Clone, Copy)]
pub struct PowerTransition {
    /// Device ID
    pub device_id: DeviceId,
    /// From state
    pub from: PowerState,
    /// To state
    pub to: PowerState,
    /// Transition time (microseconds)
    pub duration_us: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Power policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPolicy {
    /// Performance (always D0)
    Performance,
    /// Balanced
    Balanced,
    /// Power saving
    PowerSave,
    /// Custom
    Custom,
}

/// Device power manager
pub struct DevicePowerManager {
    /// Device power states
    device_states: BTreeMap<DeviceId, PowerState>,
    /// Device policies
    device_policies: BTreeMap<DeviceId, PowerPolicy>,
    /// Transition history
    transitions: VecDeque<PowerTransition>,
    /// Maximum transitions to track
    max_transitions: usize,
    /// Default policy
    default_policy: PowerPolicy,
    /// Idle timeout (microseconds)
    idle_timeout_us: u64,
    /// Device idle times
    idle_times: BTreeMap<DeviceId, u64>,
    /// Total power saved (abstract units)
    power_saved: AtomicU64,
    /// Wake latency predictions
    wake_predictions: BTreeMap<DeviceId, u64>,
}

impl DevicePowerManager {
    /// Create new power manager
    pub fn new() -> Self {
        Self {
            device_states: BTreeMap::new(),
            device_policies: BTreeMap::new(),
            transitions: Vec::with_capacity(1000),
            max_transitions: 1000,
            default_policy: PowerPolicy::Balanced,
            idle_timeout_us: 100_000, // 100ms
            idle_times: BTreeMap::new(),
            power_saved: AtomicU64::new(0),
            wake_predictions: BTreeMap::new(),
        }
    }

    /// Register device
    #[inline(always)]
    pub fn register_device(&mut self, id: DeviceId) {
        self.device_states.insert(id, PowerState::D0);
        self.idle_times.insert(id, 0);
    }

    /// Unregister device
    #[inline]
    pub fn unregister_device(&mut self, id: DeviceId) {
        self.device_states.remove(&id);
        self.device_policies.remove(&id);
        self.idle_times.remove(&id);
        self.wake_predictions.remove(&id);
    }

    /// Set device policy
    #[inline(always)]
    pub fn set_policy(&mut self, id: DeviceId, policy: PowerPolicy) {
        self.device_policies.insert(id, policy);
    }

    /// Get device power state
    #[inline(always)]
    pub fn get_state(&self, id: DeviceId) -> Option<PowerState> {
        self.device_states.get(&id).copied()
    }

    /// Record device activity
    #[inline(always)]
    pub fn record_activity(&mut self, id: DeviceId, timestamp: u64) {
        self.idle_times.insert(id, timestamp);
    }

    /// Transition device to new power state
    pub fn transition(
        &mut self,
        id: DeviceId,
        to: PowerState,
        duration_us: u64,
        timestamp: u64,
    ) -> bool {
        let from = match self.device_states.get(&id) {
            Some(&state) => state,
            None => return false,
        };

        // Record transition
        let transition = PowerTransition {
            device_id: id,
            from,
            to,
            duration_us,
            timestamp,
        };

        if self.transitions.len() >= self.max_transitions {
            self.transitions.pop_front();
        }
        self.transitions.push_back(transition);

        // Update state
        self.device_states.insert(id, to);

        // Track power savings
        if to < from {
            let saved = ((from.power_factor() - to.power_factor()) * 1000.0) as u64;
            self.power_saved.fetch_add(saved, Ordering::Relaxed);
        }

        // Update wake prediction
        self.wake_predictions.insert(id, to.wake_latency_us());

        true
    }

    /// Get recommended power state for device
    pub fn recommend_state(&self, id: DeviceId, current_time: u64) -> Option<PowerState> {
        let policy = self
            .device_policies
            .get(&id)
            .copied()
            .unwrap_or(self.default_policy);
        let last_activity = self.idle_times.get(&id).copied().unwrap_or(current_time);
        let idle_duration = current_time.saturating_sub(last_activity);

        let recommended = match policy {
            PowerPolicy::Performance => PowerState::D0,
            PowerPolicy::PowerSave => {
                if idle_duration > self.idle_timeout_us * 10 {
                    PowerState::D3Hot
                } else if idle_duration > self.idle_timeout_us * 3 {
                    PowerState::D2
                } else if idle_duration > self.idle_timeout_us {
                    PowerState::D1
                } else {
                    PowerState::D0
                }
            }
            PowerPolicy::Balanced => {
                if idle_duration > self.idle_timeout_us * 5 {
                    PowerState::D2
                } else if idle_duration > self.idle_timeout_us {
                    PowerState::D1
                } else {
                    PowerState::D0
                }
            }
            PowerPolicy::Custom => PowerState::D0,
        };

        Some(recommended)
    }

    /// Get predicted wake latency
    #[inline(always)]
    pub fn predicted_wake_latency(&self, id: DeviceId) -> Option<u64> {
        self.wake_predictions.get(&id).copied()
    }

    /// Get total power saved
    #[inline(always)]
    pub fn power_saved(&self) -> u64 {
        self.power_saved.load(Ordering::Relaxed)
    }

    /// Set default policy
    #[inline(always)]
    pub fn set_default_policy(&mut self, policy: PowerPolicy) {
        self.default_policy = policy;
    }

    /// Set idle timeout
    #[inline(always)]
    pub fn set_idle_timeout(&mut self, timeout_us: u64) {
        self.idle_timeout_us = timeout_us;
    }

    /// Get transition history
    #[inline(always)]
    pub fn transitions(&self) -> &[PowerTransition] {
        &self.transitions
    }
}

impl Default for DevicePowerManager {
    fn default() -> Self {
        Self::new()
    }
}
