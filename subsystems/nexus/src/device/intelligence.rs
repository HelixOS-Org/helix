//! Device Intelligence
//!
//! AI-powered device management and analysis.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    BusType, DeviceAction, DeviceAnalysis, DeviceId, DeviceInfo, DeviceIssue, DeviceIssueType,
    DevicePowerManager, DeviceRecommendation, DeviceState, DeviceTreeParser, DriverId, DriverInfo,
    DriverMatcher, HotplugEvent, HotplugHandler, HotplugNotification, MatchScore, PowerState,
};

/// Device Intelligence - comprehensive device management
pub struct DeviceIntelligence {
    /// Registered devices
    devices: BTreeMap<DeviceId, DeviceInfo>,
    /// Device tree parser
    device_tree: DeviceTreeParser,
    /// Driver matcher
    driver_matcher: DriverMatcher,
    /// Power manager
    power_manager: DevicePowerManager,
    /// Hotplug handler
    hotplug_handler: HotplugHandler,
    /// Device hierarchy (parent -> children)
    hierarchy: BTreeMap<DeviceId, Vec<DeviceId>>,
    /// Next device ID
    next_device_id: AtomicU64,
    /// Next driver ID
    next_driver_id: AtomicU64,
    /// Total probe attempts
    probe_attempts: AtomicU64,
    /// Successful probes
    probe_successes: AtomicU64,
}

impl DeviceIntelligence {
    /// Create new device intelligence
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            device_tree: DeviceTreeParser::new(),
            driver_matcher: DriverMatcher::new(),
            power_manager: DevicePowerManager::new(),
            hotplug_handler: HotplugHandler::new(),
            hierarchy: BTreeMap::new(),
            next_device_id: AtomicU64::new(1),
            next_driver_id: AtomicU64::new(1),
            probe_attempts: AtomicU64::new(0),
            probe_successes: AtomicU64::new(0),
        }
    }

    /// Allocate new device ID
    #[inline(always)]
    pub fn allocate_device_id(&self) -> DeviceId {
        DeviceId::new(self.next_device_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Allocate new driver ID
    #[inline(always)]
    pub fn allocate_driver_id(&self) -> DriverId {
        DriverId::new(self.next_driver_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Register device
    pub fn register_device(&mut self, device: DeviceInfo) {
        let id = device.id;

        // Setup hierarchy
        if let Some(parent) = device.parent {
            self.hierarchy.entry(parent).or_default().push(id);
        }

        // Register with power manager
        self.power_manager.register_device(id);

        self.devices.insert(id, device);
    }

    /// Unregister device
    pub fn unregister_device(&mut self, id: DeviceId, timestamp: u64) {
        if let Some(mut device) = self.devices.remove(&id) {
            device.state = DeviceState::Removed;
            device.state_changed_at = timestamp;

            // Queue hotplug event
            let notification = HotplugNotification {
                event: HotplugEvent::Remove,
                device_id: id,
                device_path: device.name.clone(),
                subsystem: String::from(""),
                timestamp,
                properties: BTreeMap::new(),
            };
            self.hotplug_handler.queue_event(notification);

            // Remove from hierarchy
            if let Some(parent) = device.parent {
                if let Some(children) = self.hierarchy.get_mut(&parent) {
                    children.retain(|c| *c != id);
                }
            }

            // Unregister from power manager
            self.power_manager.unregister_device(id);
        }
    }

    /// Register driver
    #[inline(always)]
    pub fn register_driver(&mut self, driver: DriverInfo) {
        self.driver_matcher.register_driver(driver);
    }

    /// Find driver for device
    #[inline(always)]
    pub fn find_driver(&self, device_id: DeviceId) -> Option<MatchScore> {
        let device = self.devices.get(&device_id)?;
        self.driver_matcher.best_match(device)
    }

    /// Bind driver to device
    pub fn bind_driver(
        &mut self,
        device_id: DeviceId,
        driver_id: DriverId,
        timestamp: u64,
    ) -> bool {
        self.probe_attempts.fetch_add(1, Ordering::Relaxed);

        let device = match self.devices.get_mut(&device_id) {
            Some(d) => d,
            None => return false,
        };

        device.driver_id = Some(driver_id);
        device.state = DeviceState::Bound;
        device.state_changed_at = timestamp;

        self.probe_successes.fetch_add(1, Ordering::Relaxed);
        self.driver_matcher
            .record_result(device, driver_id, true, timestamp);

        true
    }

    /// Handle probe failure
    #[inline]
    pub fn probe_failed(&mut self, device_id: DeviceId, driver_id: DriverId, timestamp: u64) {
        if let Some(device) = self.devices.get_mut(&device_id) {
            device.state = DeviceState::DeferredProbe;
            device.state_changed_at = timestamp;
            self.driver_matcher
                .record_result(device, driver_id, false, timestamp);
        }
    }

    /// Get device
    #[inline(always)]
    pub fn get_device(&self, id: DeviceId) -> Option<&DeviceInfo> {
        self.devices.get(&id)
    }

    /// Get device mutably
    #[inline(always)]
    pub fn get_device_mut(&mut self, id: DeviceId) -> Option<&mut DeviceInfo> {
        self.devices.get_mut(&id)
    }

    /// Get device children
    #[inline(always)]
    pub fn get_children(&self, id: DeviceId) -> &[DeviceId] {
        self.hierarchy.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Analyze device
    pub fn analyze_device(&self, device_id: DeviceId) -> Option<DeviceAnalysis> {
        let device = self.devices.get(&device_id)?;
        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check for no driver
        if device.needs_driver() {
            health_score -= 30.0;
            issues.push(DeviceIssue {
                issue_type: DeviceIssueType::NoDriver,
                severity: 7,
                description: String::from("Device has no driver bound"),
            });
            recommendations.push(DeviceRecommendation {
                action: DeviceAction::LoadFallback,
                expected_improvement: 25.0,
                reason: String::from("Load fallback or generic driver"),
            });
        }

        // Check for error state
        if matches!(device.state, DeviceState::Error) {
            health_score -= 40.0;
            issues.push(DeviceIssue {
                issue_type: DeviceIssueType::ErrorState,
                severity: 9,
                description: String::from("Device is in error state"),
            });
            recommendations.push(DeviceRecommendation {
                action: DeviceAction::Reset,
                expected_improvement: 35.0,
                reason: String::from("Reset device to clear error"),
            });
        }

        // Check deferred probe
        if matches!(device.state, DeviceState::DeferredProbe) {
            health_score -= 15.0;
            issues.push(DeviceIssue {
                issue_type: DeviceIssueType::DeferredTimeout,
                severity: 4,
                description: String::from("Device probe deferred"),
            });
            recommendations.push(DeviceRecommendation {
                action: DeviceAction::Reprobe,
                expected_improvement: 15.0,
                reason: String::from("Retry device probe"),
            });
        }

        // Calculate power efficiency
        let power_state = self
            .power_manager
            .get_state(device_id)
            .unwrap_or(PowerState::D0);
        let power_efficiency = (1.0 - power_state.power_factor()) * 100.0;

        health_score = health_score.max(0.0);

        Some(DeviceAnalysis {
            device_id,
            health_score,
            issues,
            recommendations,
            power_efficiency,
        })
    }

    /// Get devices by bus type
    #[inline]
    pub fn devices_by_bus(&self, bus_type: BusType) -> Vec<&DeviceInfo> {
        self.devices
            .values()
            .filter(|d| d.bus_type == bus_type)
            .collect()
    }

    /// Get devices needing drivers
    #[inline(always)]
    pub fn devices_needing_drivers(&self) -> Vec<&DeviceInfo> {
        self.devices.values().filter(|d| d.needs_driver()).collect()
    }

    /// Get device tree parser
    #[inline(always)]
    pub fn device_tree(&self) -> &DeviceTreeParser {
        &self.device_tree
    }

    /// Get device tree parser mutably
    #[inline(always)]
    pub fn device_tree_mut(&mut self) -> &mut DeviceTreeParser {
        &mut self.device_tree
    }

    /// Get driver matcher
    #[inline(always)]
    pub fn driver_matcher(&self) -> &DriverMatcher {
        &self.driver_matcher
    }

    /// Get driver matcher mutably
    #[inline(always)]
    pub fn driver_matcher_mut(&mut self) -> &mut DriverMatcher {
        &mut self.driver_matcher
    }

    /// Get power manager
    #[inline(always)]
    pub fn power_manager(&self) -> &DevicePowerManager {
        &self.power_manager
    }

    /// Get power manager mutably
    #[inline(always)]
    pub fn power_manager_mut(&mut self) -> &mut DevicePowerManager {
        &mut self.power_manager
    }

    /// Get hotplug handler
    #[inline(always)]
    pub fn hotplug_handler(&self) -> &HotplugHandler {
        &self.hotplug_handler
    }

    /// Get hotplug handler mutably
    #[inline(always)]
    pub fn hotplug_handler_mut(&mut self) -> &mut HotplugHandler {
        &mut self.hotplug_handler
    }

    /// Get probe success rate
    #[inline]
    pub fn probe_success_rate(&self) -> f32 {
        let attempts = self.probe_attempts.load(Ordering::Relaxed);
        let successes = self.probe_successes.load(Ordering::Relaxed);
        if attempts == 0 {
            return 1.0;
        }
        successes as f32 / attempts as f32
    }

    /// Get total device count
    #[inline(always)]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Perform periodic maintenance
    #[inline]
    pub fn periodic_maintenance(&mut self, current_time: u64) {
        // Update hotplug rate
        self.hotplug_handler.update_rate(current_time);

        // Process pending hotplug events
        let _ = self.hotplug_handler.process_events();
    }
}

impl Default for DeviceIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
