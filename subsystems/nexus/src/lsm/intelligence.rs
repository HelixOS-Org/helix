//! LSM Intelligence
//!
//! Central coordinator for LSM analysis.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{Denial, HookCategory, HookId, LsmManager, LsmState, LsmType, PolicyComplexity};

/// LSM analysis
#[derive(Debug, Clone)]
pub struct LsmAnalysis {
    /// Security score (0-100)
    pub security_score: f32,
    /// Policy complexity
    pub complexity: PolicyComplexity,
    /// Issues detected
    pub issues: Vec<LsmIssue>,
    /// Recommendations
    pub recommendations: Vec<LsmRecommendation>,
}

/// LSM issue
#[derive(Debug, Clone)]
pub struct LsmIssue {
    /// Issue type
    pub issue_type: LsmIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// LSM
    pub lsm: Option<LsmType>,
}

/// LSM issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmIssueType {
    /// No LSM active
    NoLsm,
    /// Permissive mode
    PermissiveMode,
    /// High denial rate
    HighDenialRate,
    /// Unconfined process
    UnconfinedProcess,
    /// AVC cache miss rate
    HighCacheMissRate,
    /// Sensitive denial
    SensitiveDenial,
}

/// LSM recommendation
#[derive(Debug, Clone)]
pub struct LsmRecommendation {
    /// Action
    pub action: LsmAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// LSM action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmAction {
    /// Enable LSM
    EnableLsm,
    /// Switch to enforcing
    SwitchToEnforcing,
    /// Add allow rule
    AddAllowRule,
    /// Confine process
    ConfineProcess,
    /// Increase cache size
    IncreaseCacheSize,
}

/// LSM Intelligence
pub struct LsmIntelligence {
    /// Manager
    manager: LsmManager,
}

impl LsmIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: LsmManager::new(),
        }
    }

    /// Register LSM
    #[inline(always)]
    pub fn register_lsm(&mut self, lsm: LsmType, state: LsmState) {
        self.manager.register_lsm(lsm, state);
    }

    /// Register hook
    #[inline(always)]
    pub fn register_hook(
        &mut self,
        name: String,
        category: HookCategory,
        lsm: LsmType,
    ) -> HookId {
        self.manager.register_hook(name, category, lsm)
    }

    /// Record denial
    #[inline(always)]
    pub fn record_denial(&mut self, denial: Denial) {
        self.manager.record_denial(denial);
    }

    /// Analyze security posture
    pub fn analyze(&self) -> LsmAnalysis {
        let mut security_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check if any LSM is active
        if self.manager.active_lsms.is_empty() {
            security_score = 20.0;
            issues.push(LsmIssue {
                issue_type: LsmIssueType::NoLsm,
                severity: 10,
                description: String::from("No LSM is active"),
                lsm: None,
            });
            recommendations.push(LsmRecommendation {
                action: LsmAction::EnableLsm,
                expected_improvement: 50.0,
                reason: String::from("Enable SELinux or AppArmor for mandatory access control"),
            });
        } else {
            // Check for permissive mode
            for (lsm, state) in &self.manager.active_lsms {
                if *state == LsmState::Permissive {
                    security_score -= 30.0;
                    issues.push(LsmIssue {
                        issue_type: LsmIssueType::PermissiveMode,
                        severity: 7,
                        description: format!("{} is in permissive mode", lsm.name()),
                        lsm: Some(*lsm),
                    });
                    recommendations.push(LsmRecommendation {
                        action: LsmAction::SwitchToEnforcing,
                        expected_improvement: 25.0,
                        reason: format!("Switch {} to enforcing mode", lsm.name()),
                    });
                }
            }
        }

        // Check denial rate
        let total_denials = self.manager.denial_tracker().total();
        let total_calls = self.manager.total_hook_calls();
        if total_calls > 0 && total_denials > 0 {
            let denial_rate = (total_denials as f32 / total_calls as f32) * 100.0;
            if denial_rate > 5.0 {
                security_score -= 10.0;
                issues.push(LsmIssue {
                    issue_type: LsmIssueType::HighDenialRate,
                    severity: 5,
                    description: format!("High denial rate: {:.1}%", denial_rate),
                    lsm: None,
                });
            }
        }

        // Check AVC hit rate
        let avc_hit_rate = self.manager.avc().hit_rate();
        if avc_hit_rate < 80.0 && self.manager.avc().entry_count() > 0 {
            issues.push(LsmIssue {
                issue_type: LsmIssueType::HighCacheMissRate,
                severity: 3,
                description: format!("Low AVC hit rate: {:.1}%", avc_hit_rate),
                lsm: None,
            });
            recommendations.push(LsmRecommendation {
                action: LsmAction::IncreaseCacheSize,
                expected_improvement: 5.0,
                reason: String::from("Increase AVC cache size for better performance"),
            });
        }

        security_score = security_score.max(0.0);

        let complexity = PolicyComplexity::from_rule_count(self.manager.hook_count());

        LsmAnalysis {
            security_score,
            complexity,
            issues,
            recommendations,
        }
    }

    /// Get manager
    #[inline(always)]
    pub fn manager(&self) -> &LsmManager {
        &self.manager
    }

    /// Get manager mutably
    #[inline(always)]
    pub fn manager_mut(&mut self) -> &mut LsmManager {
        &mut self.manager
    }
}

impl Default for LsmIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
