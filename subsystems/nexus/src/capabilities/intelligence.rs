//! Capabilities Intelligence
//!
//! Central coordinator for capability analysis.

use alloc::string::String;
use alloc::vec::Vec;

use super::{
    CapAction, CapEventType, CapIssueType, Capability, CapabilityAnalysis, CapabilityEvent,
    CapabilityIssue, CapabilityRecommendation, CapabilityTracker, LeastPrivilegeAnalyzer,
    LeastPrivilegeRec, Pid, ProcessCaps,
};

/// Capabilities Intelligence
pub struct CapabilitiesIntelligence {
    /// Tracker
    tracker: CapabilityTracker,
    /// Analyzer
    analyzer: LeastPrivilegeAnalyzer,
}

impl CapabilitiesIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            tracker: CapabilityTracker::new(10000),
            analyzer: LeastPrivilegeAnalyzer::new(),
        }
    }

    /// Register process
    #[inline]
    pub fn register_process(&mut self, caps: ProcessCaps) {
        let pid = caps.pid;
        self.tracker.register_process(caps);
        self.analyzer.start_profile(pid, 0);
    }

    /// Record capability use
    pub fn record_use(&mut self, pid: Pid, cap: Capability, timestamp: u64, success: bool) {
        let event = CapabilityEvent {
            timestamp,
            pid,
            event_type: if success {
                CapEventType::Check
            } else {
                CapEventType::Denied
            },
            capability: Some(cap),
            old_set: None,
            new_set: None,
            success,
        };
        self.tracker.record(event);

        if success {
            self.analyzer.record_usage(pid, cap, timestamp);
        }
    }

    /// Analyze system
    pub fn analyze(&self) -> CapabilityAnalysis {
        let mut security_score = 100.0f32;
        let mut risk_score = 0.0f32;
        let mut privileged_count = 0;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        for (_pid, caps) in &self.tracker.process_caps {
            let proc_risk = caps.risk_score();
            risk_score = risk_score.max(proc_risk);

            if caps.is_privileged() {
                privileged_count += 1;
            }

            // Check for CAP_SYS_ADMIN
            if caps.has_capability(Capability::SysAdmin) {
                security_score -= 20.0;
                issues.push(CapabilityIssue {
                    issue_type: CapIssueType::CriticalCapability,
                    severity: 10,
                    description: alloc::format!("Process {} has CAP_SYS_ADMIN", caps.pid.raw()),
                    pid: Some(caps.pid),
                    capability: Some(Capability::SysAdmin),
                });
                recommendations.push(CapabilityRecommendation {
                    action: CapAction::DropCap,
                    expected_improvement: 15.0,
                    reason: String::from("Drop CAP_SYS_ADMIN if not required"),
                    pid: Some(caps.pid),
                    capability: Some(Capability::SysAdmin),
                });
            }

            // Check for missing no_new_privs
            if caps.is_privileged() && !caps.no_new_privs {
                security_score -= 5.0;
                issues.push(CapabilityIssue {
                    issue_type: CapIssueType::MissingNoNewPrivs,
                    severity: 5,
                    description: alloc::format!("Process {} lacks no_new_privs", caps.pid.raw()),
                    pid: Some(caps.pid),
                    capability: None,
                });
                recommendations.push(CapabilityRecommendation {
                    action: CapAction::SetNoNewPrivs,
                    expected_improvement: 5.0,
                    reason: String::from("Set no_new_privs to prevent privilege escalation"),
                    pid: Some(caps.pid),
                    capability: None,
                });
            }

            // Check ambient caps
            if !caps.ambient.is_empty() {
                security_score -= 10.0;
                issues.push(CapabilityIssue {
                    issue_type: CapIssueType::AmbientCapabilities,
                    severity: 6,
                    description: alloc::format!(
                        "Process {} has ambient capabilities",
                        caps.pid.raw()
                    ),
                    pid: Some(caps.pid),
                    capability: None,
                });
            }
        }

        security_score = security_score.max(0.0);

        CapabilityAnalysis {
            security_score,
            risk_score,
            privileged_count,
            issues,
            recommendations,
        }
    }

    /// Get least privilege recommendation
    #[inline]
    pub fn get_least_privilege(&self, pid: Pid) -> Option<LeastPrivilegeRec> {
        self.tracker
            .get_process(pid)
            .map(|caps| self.analyzer.analyze(pid, caps))
    }

    /// Get tracker
    #[inline(always)]
    pub fn tracker(&self) -> &CapabilityTracker {
        &self.tracker
    }

    /// Get tracker mutably
    #[inline(always)]
    pub fn tracker_mut(&mut self) -> &mut CapabilityTracker {
        &mut self.tracker
    }

    /// Get analyzer
    #[inline(always)]
    pub fn analyzer(&self) -> &LeastPrivilegeAnalyzer {
        &self.analyzer
    }

    /// Get analyzer mutably
    #[inline(always)]
    pub fn analyzer_mut(&mut self) -> &mut LeastPrivilegeAnalyzer {
        &mut self.analyzer
    }
}

impl Default for CapabilitiesIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
