//! AI-powered PCI analysis and intelligence.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::capabilities::CapabilityId;
use super::device::PciDevice;
use super::manager::PciManager;
use super::types::PciDeviceId;

// ============================================================================
// PCI INTELLIGENCE
// ============================================================================

/// PCI analysis
#[derive(Debug, Clone)]
pub struct PciAnalysis {
    /// Overall health score (0-100)
    pub health_score: f32,
    /// Performance score (0-100)
    pub performance_score: f32,
    /// Issues
    pub issues: Vec<PciIssue>,
    /// Recommendations
    pub recommendations: Vec<PciRecommendation>,
}

/// PCI issue
#[derive(Debug, Clone)]
pub struct PciIssue {
    /// Issue type
    pub issue_type: PciIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Device
    pub device: Option<PciDeviceId>,
}

/// PCI issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciIssueType {
    /// No driver
    NoDriver,
    /// Link degraded
    LinkDegraded,
    /// Unassigned BAR
    UnassignedBar,
    /// Legacy interrupt
    LegacyInterrupt,
    /// Power state
    InLowPowerState,
    /// No IOMMU
    NoIommu,
}

/// PCI recommendation
#[derive(Debug, Clone)]
pub struct PciRecommendation {
    /// Action
    pub action: PciAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// PCI action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciAction {
    /// Install driver
    InstallDriver,
    /// Enable MSI
    EnableMsi,
    /// Check link
    CheckLink,
    /// Enable IOMMU
    EnableIommu,
    /// Wake device
    WakeDevice,
}

/// PCI Intelligence
pub struct PciIntelligence {
    /// Manager
    manager: PciManager,
}

impl PciIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: PciManager::new(),
        }
    }

    /// Register device
    pub fn register_device(&mut self, device: PciDevice) {
        self.manager.register_device(device);
    }

    /// Analyze PCI subsystem
    pub fn analyze(&self) -> PciAnalysis {
        let mut health_score = 100.0f32;
        let mut performance_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        for device in self.manager.all_devices.values() {
            // Check for missing driver
            if !device.has_driver {
                health_score -= 5.0;
                issues.push(PciIssue {
                    issue_type: PciIssueType::NoDriver,
                    severity: 4,
                    description: format!("Device {} has no driver", device.id),
                    device: Some(device.id),
                });
                recommendations.push(PciRecommendation {
                    action: PciAction::InstallDriver,
                    expected_improvement: 5.0,
                    reason: format!(
                        "Install driver for {:04x}:{:04x}",
                        device.vendor.0, device.product.0
                    ),
                });
            }

            // Check PCIe link
            if let Some(link) = &device.pcie_link {
                if link.efficiency() < 0.5 {
                    performance_score -= 10.0;
                    issues.push(PciIssue {
                        issue_type: PciIssueType::LinkDegraded,
                        severity: 6,
                        description: format!(
                            "Device {} running at {} {} (max {} {})",
                            device.id,
                            link.speed.name(),
                            link.width.name(),
                            link.max_speed.name(),
                            link.max_width.name()
                        ),
                        device: Some(device.id),
                    });
                    recommendations.push(PciRecommendation {
                        action: PciAction::CheckLink,
                        expected_improvement: 10.0,
                        reason: String::from("Check PCIe slot and connection"),
                    });
                }
            }

            // Check for legacy interrupts
            if device.irq.is_some()
                && !device.msi_enabled
                && device.has_capability(CapabilityId::MSI)
            {
                performance_score -= 2.0;
                issues.push(PciIssue {
                    issue_type: PciIssueType::LegacyInterrupt,
                    severity: 3,
                    description: format!("Device {} using legacy IRQ", device.id),
                    device: Some(device.id),
                });
                recommendations.push(PciRecommendation {
                    action: PciAction::EnableMsi,
                    expected_improvement: 2.0,
                    reason: String::from("Enable MSI/MSI-X for better performance"),
                });
            }

            // Check IOMMU protection
            if device.iommu_domain.is_none() && !device.device_type.is_bridge() {
                health_score -= 3.0;
                issues.push(PciIssue {
                    issue_type: PciIssueType::NoIommu,
                    severity: 5,
                    description: format!("Device {} not in IOMMU domain", device.id),
                    device: Some(device.id),
                });
            }

            // Check unassigned BARs
            for base_addr_reg in &device.bars {
                if base_addr_reg.is_valid() && !base_addr_reg.assigned {
                    health_score -= 2.0;
                    issues.push(PciIssue {
                        issue_type: PciIssueType::UnassignedBar,
                        severity: 4,
                        description: format!(
                            "Device {} BAR{} not assigned",
                            device.id,
                            base_addr_reg.index
                        ),
                        device: Some(device.id),
                    });
                }
            }
        }

        health_score = health_score.max(0.0);
        performance_score = performance_score.max(0.0);

        PciAnalysis {
            health_score,
            performance_score,
            issues,
            recommendations,
        }
    }

    /// Get manager
    pub fn manager(&self) -> &PciManager {
        &self.manager
    }

    /// Get manager mutably
    pub fn manager_mut(&mut self) -> &mut PciManager {
        &mut self.manager
    }
}

impl Default for PciIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
