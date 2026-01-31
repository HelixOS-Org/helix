//! Network Intelligence
//!
//! AI-powered network analysis and recommendations.

use alloc::string::String;
use alloc::vec::Vec;

use super::{Duplex, IfIndex, InterfaceStats, InterfaceType, NetworkInterface, NetworkManager};

// ============================================================================
// ANALYSIS TYPES
// ============================================================================

/// Network analysis
#[derive(Debug, Clone)]
pub struct NetworkAnalysis {
    /// Health score (0-100)
    pub health_score: f32,
    /// Performance score (0-100)
    pub performance_score: f32,
    /// Issues
    pub issues: Vec<NetworkIssue>,
    /// Recommendations
    pub recommendations: Vec<NetworkRecommendation>,
}

/// Network issue
#[derive(Debug, Clone)]
pub struct NetworkIssue {
    /// Issue type
    pub issue_type: NetworkIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Interface
    pub interface: Option<IfIndex>,
}

/// Network issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkIssueType {
    /// No link
    NoLink,
    /// High error rate
    HighErrorRate,
    /// High drop rate
    HighDropRate,
    /// Slow speed
    SlowSpeed,
    /// Half duplex
    HalfDuplex,
    /// No offloads
    NoOffloads,
    /// Small ring buffer
    SmallRingBuffer,
    /// No driver
    NoDriver,
}

/// Network recommendation
#[derive(Debug, Clone)]
pub struct NetworkRecommendation {
    /// Action
    pub action: NetworkAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Network action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkAction {
    /// Check cable
    CheckCable,
    /// Enable offloads
    EnableOffloads,
    /// Increase ring buffer
    IncreaseRingBuffer,
    /// Upgrade speed
    UpgradeSpeed,
    /// Enable flow control
    EnableFlowControl,
    /// Install driver
    InstallDriver,
}

// ============================================================================
// NETWORK INTELLIGENCE
// ============================================================================

/// Network Intelligence
pub struct NetworkIntelligence {
    /// Manager
    manager: NetworkManager,
}

impl NetworkIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: NetworkManager::new(),
        }
    }

    /// Register interface
    pub fn register_interface(&mut self, interface: NetworkInterface) {
        self.manager.register_interface(interface);
    }

    /// Update stats
    pub fn update_stats(&mut self, index: IfIndex, stats: InterfaceStats) {
        self.manager.update_stats(index, stats);
    }

    /// Analyze network subsystem
    pub fn analyze(&self) -> NetworkAnalysis {
        let mut health_score = 100.0f32;
        let mut performance_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        for iface in self.manager.interfaces.values() {
            // Skip loopback
            if matches!(iface.if_type, InterfaceType::Loopback) {
                continue;
            }

            // Check link state
            if iface.is_up() && !iface.has_link() {
                health_score -= 20.0;
                issues.push(NetworkIssue {
                    issue_type: NetworkIssueType::NoLink,
                    severity: 8,
                    description: alloc::format!("Interface {} is up but has no link", iface.name),
                    interface: Some(iface.index),
                });
                recommendations.push(NetworkRecommendation {
                    action: NetworkAction::CheckCable,
                    expected_improvement: 15.0,
                    reason: String::from("Check cable connection and switch port"),
                });
            }

            // Check error rate
            if iface.stats.error_rate() > 0.001 {
                health_score -= 15.0;
                issues.push(NetworkIssue {
                    issue_type: NetworkIssueType::HighErrorRate,
                    severity: 7,
                    description: alloc::format!(
                        "Interface {} has high error rate: {:.2}%",
                        iface.name,
                        iface.stats.error_rate() * 100.0
                    ),
                    interface: Some(iface.index),
                });
            }

            // Check drop rate
            if iface.stats.drop_rate() > 0.001 {
                health_score -= 10.0;
                issues.push(NetworkIssue {
                    issue_type: NetworkIssueType::HighDropRate,
                    severity: 6,
                    description: alloc::format!(
                        "Interface {} has high drop rate: {:.2}%",
                        iface.name,
                        iface.stats.drop_rate() * 100.0
                    ),
                    interface: Some(iface.index),
                });
                recommendations.push(NetworkRecommendation {
                    action: NetworkAction::IncreaseRingBuffer,
                    expected_improvement: 8.0,
                    reason: String::from("Increase ring buffer size to reduce drops"),
                });
            }

            // Check duplex
            if matches!(iface.duplex, Duplex::Half) {
                performance_score -= 15.0;
                issues.push(NetworkIssue {
                    issue_type: NetworkIssueType::HalfDuplex,
                    severity: 6,
                    description: alloc::format!(
                        "Interface {} is running at half duplex",
                        iface.name
                    ),
                    interface: Some(iface.index),
                });
            }

            // Check speed
            if let Some(speed) = iface.speed {
                if speed.0 < 1000 && iface.if_type.is_physical() {
                    performance_score -= 10.0;
                    issues.push(NetworkIssue {
                        issue_type: NetworkIssueType::SlowSpeed,
                        severity: 5,
                        description: alloc::format!(
                            "Interface {} running at {}",
                            iface.name,
                            speed.to_string()
                        ),
                        interface: Some(iface.index),
                    });
                    recommendations.push(NetworkRecommendation {
                        action: NetworkAction::UpgradeSpeed,
                        expected_improvement: 10.0,
                        reason: String::from("Check cable and switch for Gigabit support"),
                    });
                }
            }

            // Check ring buffer
            if iface.ring_stats.rx_max > 0
                && iface.ring_stats.rx_pending < iface.ring_stats.rx_max / 2
            {
                performance_score -= 5.0;
                issues.push(NetworkIssue {
                    issue_type: NetworkIssueType::SmallRingBuffer,
                    severity: 4,
                    description: alloc::format!("Interface {} has small ring buffer", iface.name),
                    interface: Some(iface.index),
                });
                recommendations.push(NetworkRecommendation {
                    action: NetworkAction::IncreaseRingBuffer,
                    expected_improvement: 5.0,
                    reason: String::from("Increase ring buffer to reduce packet drops under load"),
                });
            }
        }

        health_score = health_score.max(0.0);
        performance_score = performance_score.max(0.0);

        NetworkAnalysis {
            health_score,
            performance_score,
            issues,
            recommendations,
        }
    }

    /// Get manager
    pub fn manager(&self) -> &NetworkManager {
        &self.manager
    }

    /// Get manager mutably
    pub fn manager_mut(&mut self) -> &mut NetworkManager {
        &mut self.manager
    }
}

impl Default for NetworkIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
