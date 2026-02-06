//! USB Intelligence
//!
//! Central coordinator for USB analysis.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{
    BusId, HubPortState, UsbBus, UsbDevice, UsbDeviceId, UsbDeviceState, UsbManager, UsbSpeed,
};

/// USB analysis
#[derive(Debug, Clone)]
pub struct UsbAnalysis {
    /// Health score (0-100)
    pub health_score: f32,
    /// Performance score (0-100)
    pub performance_score: f32,
    /// Issues
    pub issues: Vec<UsbIssue>,
    /// Recommendations
    pub recommendations: Vec<UsbRecommendation>,
}

/// USB issue
#[derive(Debug, Clone)]
pub struct UsbIssue {
    /// Issue type
    pub issue_type: UsbIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Device
    pub device: Option<UsbDeviceId>,
}

/// USB issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbIssueType {
    /// No driver
    NoDriver,
    /// Speed degraded
    SpeedDegraded,
    /// Over current
    OverCurrent,
    /// High error rate
    HighErrorRate,
    /// Hub overloaded
    HubOverloaded,
    /// Power insufficient
    PowerInsufficient,
    /// Suspended
    Suspended,
}

impl UsbIssueType {
    /// Get issue type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::NoDriver => "no_driver",
            Self::SpeedDegraded => "speed_degraded",
            Self::OverCurrent => "over_current",
            Self::HighErrorRate => "high_error_rate",
            Self::HubOverloaded => "hub_overloaded",
            Self::PowerInsufficient => "power_insufficient",
            Self::Suspended => "suspended",
        }
    }
}

/// USB recommendation
#[derive(Debug, Clone)]
pub struct UsbRecommendation {
    /// Action
    pub action: UsbAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// USB action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsbAction {
    /// Install driver
    InstallDriver,
    /// Use faster port
    UseFasterPort,
    /// Check cable
    CheckCable,
    /// Reduce hub load
    ReduceHubLoad,
    /// Add powered hub
    AddPoweredHub,
}

impl UsbAction {
    /// Get action name
    pub fn name(&self) -> &'static str {
        match self {
            Self::InstallDriver => "install_driver",
            Self::UseFasterPort => "use_faster_port",
            Self::CheckCable => "check_cable",
            Self::ReduceHubLoad => "reduce_hub_load",
            Self::AddPoweredHub => "add_powered_hub",
        }
    }
}

/// USB Intelligence
pub struct UsbIntelligence {
    /// Manager
    manager: UsbManager,
}

impl UsbIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: UsbManager::new(),
        }
    }

    /// Register bus
    pub fn register_bus(&mut self, bus: UsbBus) {
        self.manager.register_bus(bus);
    }

    /// Register device
    pub fn register_device(&mut self, bus_id: BusId, device: UsbDevice) {
        self.manager.register_device(bus_id, device);
    }

    /// Analyze USB subsystem
    pub fn analyze(&self) -> UsbAnalysis {
        let mut health_score = 100.0f32;
        let mut performance_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        for bus in self.manager.buses().values() {
            for device in bus.devices.values() {
                // Check for missing driver
                if !device.has_driver && !device.is_hub {
                    health_score -= 5.0;
                    issues.push(UsbIssue {
                        issue_type: UsbIssueType::NoDriver,
                        severity: 4,
                        description: format!("Device {} has no driver", device.id),
                        device: Some(device.id),
                    });
                    recommendations.push(UsbRecommendation {
                        action: UsbAction::InstallDriver,
                        expected_improvement: 5.0,
                        reason: format!(
                            "Install driver for {:04x}:{:04x}",
                            device.vendor.0, device.product.0
                        ),
                    });
                }

                // Check for speed degradation on storage devices
                if device.class.is_storage()
                    && matches!(device.speed, UsbSpeed::Full | UsbSpeed::Low)
                {
                    performance_score -= 20.0;
                    issues.push(UsbIssue {
                        issue_type: UsbIssueType::SpeedDegraded,
                        severity: 7,
                        description: format!(
                            "Storage device {} running at {}",
                            device.id,
                            device.speed.name()
                        ),
                        device: Some(device.id),
                    });
                    recommendations.push(UsbRecommendation {
                        action: UsbAction::UseFasterPort,
                        expected_improvement: 15.0,
                        reason: String::from("Connect to USB 3.x port for better performance"),
                    });
                }

                // Check suspended state
                if matches!(device.state, UsbDeviceState::Suspended) {
                    issues.push(UsbIssue {
                        issue_type: UsbIssueType::Suspended,
                        severity: 2,
                        description: format!("Device {} is suspended", device.id),
                        device: Some(device.id),
                    });
                }
            }

            // Check hub utilization
            for hub in bus.hubs.values() {
                let utilization = hub.connected_ports() as f32 / hub.port_count as f32;
                if utilization > 0.8 {
                    health_score -= 5.0;
                    issues.push(UsbIssue {
                        issue_type: UsbIssueType::HubOverloaded,
                        severity: 5,
                        description: format!(
                            "Hub {} is {}% utilized",
                            hub.device_id,
                            (utilization * 100.0) as u32
                        ),
                        device: Some(hub.device_id),
                    });
                }

                // Check for over-current ports
                for port in &hub.ports {
                    if matches!(port.state, HubPortState::OverCurrent) {
                        health_score -= 15.0;
                        issues.push(UsbIssue {
                            issue_type: UsbIssueType::OverCurrent,
                            severity: 9,
                            description: format!(
                                "Hub {} port {} over-current",
                                hub.device_id, port.number
                            ),
                            device: Some(hub.device_id),
                        });
                    }
                }
            }
        }

        health_score = health_score.max(0.0);
        performance_score = performance_score.max(0.0);

        UsbAnalysis {
            health_score,
            performance_score,
            issues,
            recommendations,
        }
    }

    /// Get manager
    pub fn manager(&self) -> &UsbManager {
        &self.manager
    }

    /// Get manager mutably
    pub fn manager_mut(&mut self) -> &mut UsbManager {
        &mut self.manager
    }
}

impl Default for UsbIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
