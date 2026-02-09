//! Seccomp Intelligence
//!
//! AI-powered seccomp analysis and optimization.

use alloc::string::String;
use alloc::vec::Vec;

use super::{
    Architecture, AttackSurfaceAnalysis, AttackSurfaceAnalyzer, FilterAction, FilterId, Pid,
    ProfileId, SeccompFilter, SeccompManager, SyscallNum, SyscallProfile, SyscallProfiler,
};

/// Seccomp analysis
#[derive(Debug, Clone)]
pub struct SeccompAnalysis {
    /// Security score (0-100)
    pub security_score: f32,
    /// Attack surface analysis
    pub attack_surface: Option<AttackSurfaceAnalysis>,
    /// Issues detected
    pub issues: Vec<SeccompIssue>,
    /// Recommendations
    pub recommendations: Vec<SeccompRecommendation>,
}

/// Seccomp issue
#[derive(Debug, Clone)]
pub struct SeccompIssue {
    /// Issue type
    pub issue_type: SeccompIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Related filter
    pub filter_id: Option<FilterId>,
}

/// Seccomp issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompIssueType {
    /// No filter applied
    NoFilter,
    /// Default allow
    DefaultAllow,
    /// Critical syscalls allowed
    CriticalAllowed,
    /// High risk syscalls allowed
    HighRiskAllowed,
    /// Filter too permissive
    TooPermissive,
    /// Filter too restrictive
    TooRestrictive,
}

/// Seccomp recommendation
#[derive(Debug, Clone)]
pub struct SeccompRecommendation {
    /// Action
    pub action: SeccompAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
    /// Related syscalls
    pub syscalls: Vec<SyscallNum>,
}

/// Seccomp action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAction {
    /// Apply filter
    ApplyFilter,
    /// Block syscall
    BlockSyscall,
    /// Allow syscall
    AllowSyscall,
    /// Change default action
    ChangeDefault,
    /// Use strict mode
    UseStrict,
}

/// Seccomp Intelligence
pub struct SeccompIntelligence {
    /// Manager
    manager: SeccompManager,
    /// Profiler
    profiler: SyscallProfiler,
    /// Analyzer
    analyzer: AttackSurfaceAnalyzer,
}

impl SeccompIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: SeccompManager::new(),
            profiler: SyscallProfiler::new(100),
            analyzer: AttackSurfaceAnalyzer::new(),
        }
    }

    /// Create filter
    #[inline(always)]
    pub fn create_filter(
        &mut self,
        arch: Architecture,
        default_action: FilterAction,
        timestamp: u64,
    ) -> FilterId {
        self.manager.create_filter(arch, default_action, timestamp)
    }

    /// Start profiling
    #[inline(always)]
    pub fn start_profiling(&mut self, pid: Pid, timestamp: u64) -> ProfileId {
        self.profiler.start_profile(pid, timestamp)
    }

    /// Stop profiling
    #[inline(always)]
    pub fn stop_profiling(&mut self, pid: Pid, timestamp: u64) -> Option<SyscallProfile> {
        self.profiler.stop_profile(pid, timestamp)
    }

    /// Generate filter from profile
    pub fn generate_filter_from_profile(
        &mut self,
        profile: &SyscallProfile,
        timestamp: u64,
    ) -> Option<FilterId> {
        if profile.syscalls.is_empty() {
            return None;
        }

        // Create filter with deny-by-default
        let id = self
            .manager
            .create_filter(Architecture::X86_64, FilterAction::Kill, timestamp);

        if let Some(filter) = self.manager.get_filter_mut(id) {
            // Allow all syscalls that were observed
            for &syscall in profile.syscalls.keys() {
                filter.add_rule(syscall, FilterAction::Allow);
            }
            filter.compile();
        }

        Some(id)
    }

    /// Attach filter to process
    #[inline(always)]
    pub fn attach_filter(&mut self, filter_id: FilterId, pid: Pid) -> bool {
        self.manager.attach(filter_id, pid)
    }

    /// Analyze security
    pub fn analyze(&self, filter_id: Option<FilterId>) -> SeccompAnalysis {
        let mut security_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();
        let mut attack_surface = None;

        if let Some(id) = filter_id {
            if let Some(filter) = self.manager.get_filter(id) {
                // Analyze attack surface
                let surface = self.analyzer.analyze(filter);

                // Check for default allow
                if matches!(filter.default_action, FilterAction::Allow) {
                    security_score -= 40.0;
                    issues.push(SeccompIssue {
                        issue_type: SeccompIssueType::DefaultAllow,
                        severity: 8,
                        description: String::from("Filter uses allow-by-default policy"),
                        filter_id: Some(id),
                    });
                    recommendations.push(SeccompRecommendation {
                        action: SeccompAction::ChangeDefault,
                        expected_improvement: 30.0,
                        reason: String::from("Switch to deny-by-default for better security"),
                        syscalls: Vec::new(),
                    });
                }

                // Check critical syscalls
                if !surface.critical_allowed.is_empty() {
                    security_score -= 30.0;
                    issues.push(SeccompIssue {
                        issue_type: SeccompIssueType::CriticalAllowed,
                        severity: 10,
                        description: alloc::format!(
                            "{} critical syscalls allowed",
                            surface.critical_allowed.len()
                        ),
                        filter_id: Some(id),
                    });
                    recommendations.push(SeccompRecommendation {
                        action: SeccompAction::BlockSyscall,
                        expected_improvement: 25.0,
                        reason: String::from("Block critical syscalls unless required"),
                        syscalls: surface.critical_allowed.clone(),
                    });
                }

                // Check high risk
                if !surface.high_risk_allowed.is_empty() {
                    security_score -= 15.0;
                    issues.push(SeccompIssue {
                        issue_type: SeccompIssueType::HighRiskAllowed,
                        severity: 7,
                        description: alloc::format!(
                            "{} high-risk syscalls allowed",
                            surface.high_risk_allowed.len()
                        ),
                        filter_id: Some(id),
                    });
                }

                attack_surface = Some(surface);
            }
        } else {
            security_score = 0.0;
            issues.push(SeccompIssue {
                issue_type: SeccompIssueType::NoFilter,
                severity: 9,
                description: String::from("No seccomp filter applied"),
                filter_id: None,
            });
            recommendations.push(SeccompRecommendation {
                action: SeccompAction::ApplyFilter,
                expected_improvement: 50.0,
                reason: String::from("Apply a seccomp filter to reduce attack surface"),
                syscalls: Vec::new(),
            });
        }

        security_score = security_score.max(0.0);

        SeccompAnalysis {
            security_score,
            attack_surface,
            issues,
            recommendations,
        }
    }

    /// Get manager
    #[inline(always)]
    pub fn manager(&self) -> &SeccompManager {
        &self.manager
    }

    /// Get manager mutably
    #[inline(always)]
    pub fn manager_mut(&mut self) -> &mut SeccompManager {
        &mut self.manager
    }

    /// Get profiler
    #[inline(always)]
    pub fn profiler(&self) -> &SyscallProfiler {
        &self.profiler
    }

    /// Get profiler mutably
    #[inline(always)]
    pub fn profiler_mut(&mut self) -> &mut SyscallProfiler {
        &mut self.profiler
    }

    /// Get analyzer
    #[inline(always)]
    pub fn analyzer(&self) -> &AttackSurfaceAnalyzer {
        &self.analyzer
    }

    /// Get filter
    #[inline(always)]
    pub fn get_filter(&self, id: FilterId) -> Option<&SeccompFilter> {
        self.manager.get_filter(id)
    }
}

impl Default for SeccompIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
