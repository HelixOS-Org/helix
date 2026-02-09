//! IOMMU intelligence and security analysis.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::dma::DmaMapping;
use super::domain::DomainType;
use super::fault::IommuFault;
use super::manager::IommuManager;
use super::types::{DeviceId, IommuId, IommuType};

// ============================================================================
// IOMMU ANALYSIS
// ============================================================================

/// IOMMU analysis
#[derive(Debug, Clone)]
pub struct IommuAnalysis {
    /// Security score (0-100)
    pub security_score: f32,
    /// Isolation score (0-100)
    pub isolation_score: f32,
    /// Issues detected
    pub issues: Vec<IommuIssue>,
    /// Recommendations
    pub recommendations: Vec<IommuRecommendation>,
}

// ============================================================================
// IOMMU ISSUE
// ============================================================================

/// IOMMU issue
#[derive(Debug, Clone)]
pub struct IommuIssue {
    /// Issue type
    pub issue_type: IommuIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Device
    pub device: Option<DeviceId>,
}

/// IOMMU issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuIssueType {
    /// No IOMMU present
    NoIommu,
    /// Translation disabled
    TranslationDisabled,
    /// Device in passthrough
    Passthrough,
    /// DMA fault
    DmaFault,
    /// Missing interrupt remapping
    NoInterruptRemap,
    /// Shared domain
    SharedDomain,
    /// Large mapping
    LargeMapping,
}

// ============================================================================
// IOMMU RECOMMENDATION
// ============================================================================

/// IOMMU recommendation
#[derive(Debug, Clone)]
pub struct IommuRecommendation {
    /// Action
    pub action: IommuAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// IOMMU action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuAction {
    /// Enable IOMMU
    EnableIommu,
    /// Enable translation
    EnableTranslation,
    /// Enable interrupt remapping
    EnableInterruptRemap,
    /// Isolate device
    IsolateDevice,
    /// Investigate fault
    InvestigateFault,
}

// ============================================================================
// IOMMU INTELLIGENCE
// ============================================================================

/// IOMMU Intelligence
pub struct IommuIntelligence {
    /// Manager
    manager: IommuManager,
}

impl IommuIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: IommuManager::new(),
        }
    }

    /// Register IOMMU
    #[inline(always)]
    pub fn register_iommu(&mut self, iommu_type: IommuType) -> IommuId {
        self.manager.register_unit(iommu_type)
    }

    /// Record mapping
    #[inline(always)]
    pub fn record_mapping(&mut self, mapping: DmaMapping) {
        self.manager.record_mapping(mapping);
    }

    /// Record fault
    #[inline(always)]
    pub fn record_fault(&mut self, fault: IommuFault) {
        self.manager.record_fault(fault);
    }

    /// Analyze security
    pub fn analyze(&self) -> IommuAnalysis {
        let mut security_score = 100.0f32;
        let mut isolation_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check if IOMMU is present
        if !self.manager.has_iommu() {
            security_score = 20.0;
            isolation_score = 0.0;
            issues.push(IommuIssue {
                issue_type: IommuIssueType::NoIommu,
                severity: 10,
                description: String::from("No IOMMU present or enabled"),
                device: None,
            });
            recommendations.push(IommuRecommendation {
                action: IommuAction::EnableIommu,
                expected_improvement: 50.0,
                reason: String::from("Enable IOMMU for DMA protection"),
            });
            return IommuAnalysis {
                security_score,
                isolation_score,
                issues,
                recommendations,
            };
        }

        // Check each unit
        for unit in self.manager.units.values() {
            // Check translation
            if !unit.is_translation_enabled() {
                security_score -= 30.0;
                issues.push(IommuIssue {
                    issue_type: IommuIssueType::TranslationDisabled,
                    severity: 8,
                    description: alloc::format!("IOMMU {} has translation disabled", unit.id.raw()),
                    device: None,
                });
                recommendations.push(IommuRecommendation {
                    action: IommuAction::EnableTranslation,
                    expected_improvement: 25.0,
                    reason: String::from("Enable IOMMU translation for DMA isolation"),
                });
            }

            // Check interrupt remapping
            if unit.capabilities.interrupt_remap && !unit.is_interrupt_remap_enabled() {
                security_score -= 10.0;
                issues.push(IommuIssue {
                    issue_type: IommuIssueType::NoInterruptRemap,
                    severity: 5,
                    description: String::from("Interrupt remapping not enabled"),
                    device: None,
                });
                recommendations.push(IommuRecommendation {
                    action: IommuAction::EnableInterruptRemap,
                    expected_improvement: 10.0,
                    reason: String::from("Enable interrupt remapping for security"),
                });
            }

            // Check for passthrough domains
            for domain in unit.domains.values() {
                if matches!(domain.domain_type, DomainType::Identity) && domain.device_count() > 0 {
                    isolation_score -= 20.0;
                    issues.push(IommuIssue {
                        issue_type: IommuIssueType::Passthrough,
                        severity: 7,
                        description: alloc::format!(
                            "Domain {} is in passthrough mode with {} devices",
                            domain.id.raw(),
                            domain.device_count()
                        ),
                        device: domain.devices.first().copied(),
                    });
                    recommendations.push(IommuRecommendation {
                        action: IommuAction::IsolateDevice,
                        expected_improvement: 15.0,
                        reason: String::from("Move devices out of passthrough mode"),
                    });
                }
            }
        }

        // Check for faults
        let fault_count = self.manager.fault_tracker.total();
        if fault_count > 0 {
            security_score -= (fault_count as f32 * 0.5).min(20.0);
            issues.push(IommuIssue {
                issue_type: IommuIssueType::DmaFault,
                severity: 6,
                description: alloc::format!("{} IOMMU faults recorded", fault_count),
                device: None,
            });
            recommendations.push(IommuRecommendation {
                action: IommuAction::InvestigateFault,
                expected_improvement: 10.0,
                reason: String::from("Investigate IOMMU faults for potential issues"),
            });
        }

        security_score = security_score.max(0.0);
        isolation_score = isolation_score.max(0.0);

        IommuAnalysis {
            security_score,
            isolation_score,
            issues,
            recommendations,
        }
    }

    /// Get manager
    #[inline(always)]
    pub fn manager(&self) -> &IommuManager {
        &self.manager
    }

    /// Get manager mutably
    #[inline(always)]
    pub fn manager_mut(&mut self) -> &mut IommuManager {
        &mut self.manager
    }
}

impl Default for IommuIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
