//! Namespace Intelligence
//!
//! Central coordinator for namespace analysis.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{
    IsolationAnalyzer, NamespaceAction, NamespaceAnalysis, NamespaceId, NamespaceInfo,
    NamespaceIssue, NamespaceIssueType, NamespaceManager, NamespaceOptions, NamespaceRecommendation,
    NamespaceType, ProcessId, SecurityEnforcer, UserId,
};

/// Namespace Intelligence - comprehensive namespace analysis and management
pub struct NamespaceIntelligence {
    /// Namespace manager
    manager: NamespaceManager,
    /// Isolation analyzer
    isolation_analyzer: IsolationAnalyzer,
    /// Security enforcer
    security_enforcer: SecurityEnforcer,
}

impl NamespaceIntelligence {
    /// Create new namespace intelligence
    pub fn new() -> Self {
        Self {
            manager: NamespaceManager::new(),
            isolation_analyzer: IsolationAnalyzer::new(),
            security_enforcer: SecurityEnforcer::new(),
        }
    }

    /// Initialize with init namespaces
    pub fn initialize(&mut self, creator: ProcessId, timestamp: u64) {
        // Create initial namespaces for all types
        for ns_type in NamespaceType::all() {
            let id = self.manager.create_namespace(
                *ns_type,
                creator,
                UserId::ROOT,
                timestamp,
                &NamespaceOptions::default(),
            );
            self.isolation_analyzer.set_init_namespace(*ns_type, id);
        }
    }

    /// Create namespace
    pub fn create_namespace(
        &mut self,
        ns_type: NamespaceType,
        creator: ProcessId,
        creator_uid: UserId,
        timestamp: u64,
        options: &NamespaceOptions,
    ) -> NamespaceId {
        self.manager
            .create_namespace(ns_type, creator, creator_uid, timestamp, options)
    }

    /// Delete namespace
    pub fn delete_namespace(&mut self, id: NamespaceId) -> bool {
        self.manager.delete_namespace(id)
    }

    /// Enter namespace
    pub fn enter_namespace(&mut self, pid: ProcessId, ns_id: NamespaceId) -> bool {
        let info = match self.manager.get_namespace(ns_id) {
            Some(i) => i,
            None => return false,
        };

        let ns_type = info.ns_type;
        self.manager.add_process(ns_id, pid, ns_type);

        // Update isolation analyzer
        if let Some(proc_ns) = self.manager.get_process_namespaces(pid) {
            self.isolation_analyzer
                .register_process(pid, proc_ns.clone());
        }

        true
    }

    /// Leave namespace
    pub fn leave_namespace(&mut self, pid: ProcessId, ns_id: NamespaceId) -> bool {
        self.manager.remove_process(ns_id, pid)
    }

    /// Analyze namespace
    pub fn analyze_namespace(&mut self, ns_id: NamespaceId) -> Option<NamespaceAnalysis> {
        let info = self.manager.get_namespace(ns_id)?;
        let mut health_score = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Check for empty namespace
        if info.is_empty() {
            issues.push(NamespaceIssue {
                issue_type: NamespaceIssueType::EmptyNamespace,
                severity: 2,
                description: String::from("Namespace has no processes"),
            });
            recommendations.push(NamespaceRecommendation {
                action: NamespaceAction::CleanupEmpty,
                expected_improvement: 5.0,
                reason: String::from("Consider cleaning up empty namespace"),
            });
        }

        // Check user namespace mappings
        if info.ns_type == NamespaceType::User {
            if let Some(user_info) = self.manager.get_user_ns(ns_id) {
                if user_info.uid_map.is_empty() {
                    health_score -= 20.0;
                    issues.push(NamespaceIssue {
                        issue_type: NamespaceIssueType::MissingUserMapping,
                        severity: 7,
                        description: String::from("User namespace has no UID mappings"),
                    });
                    recommendations.push(NamespaceRecommendation {
                        action: NamespaceAction::AddUserMapping,
                        expected_improvement: 20.0,
                        reason: String::from("Add UID/GID mappings for proper isolation"),
                    });
                }
            }
        }

        // Check for too many processes
        if info.process_count() > 1000 {
            health_score -= 10.0;
            issues.push(NamespaceIssue {
                issue_type: NamespaceIssueType::TooManyProcesses,
                severity: 4,
                description: format!("Namespace has {} processes", info.process_count()),
            });
        }

        // Check security violations
        let violations = self.security_enforcer.total_violations();
        if violations > 0 {
            health_score -= (violations as f32).min(30.0);
            issues.push(NamespaceIssue {
                issue_type: NamespaceIssueType::SecurityViolation,
                severity: 8,
                description: format!("{} security violations detected", violations),
            });
            recommendations.push(NamespaceRecommendation {
                action: NamespaceAction::IncreaseIsolation,
                expected_improvement: 25.0,
                reason: String::from("Increase isolation to prevent violations"),
            });
        }

        health_score = health_score.max(0.0);

        // Get isolation analysis for first process
        let isolation = info
            .processes
            .first()
            .and_then(|pid| self.isolation_analyzer.analyze(*pid));

        Some(NamespaceAnalysis {
            ns_id,
            health_score,
            issues,
            recommendations,
            isolation,
        })
    }

    /// Get namespace manager
    pub fn manager(&self) -> &NamespaceManager {
        &self.manager
    }

    /// Get namespace manager mutably
    pub fn manager_mut(&mut self) -> &mut NamespaceManager {
        &mut self.manager
    }

    /// Get isolation analyzer
    pub fn isolation_analyzer(&self) -> &IsolationAnalyzer {
        &self.isolation_analyzer
    }

    /// Get isolation analyzer mutably
    pub fn isolation_analyzer_mut(&mut self) -> &mut IsolationAnalyzer {
        &mut self.isolation_analyzer
    }

    /// Get security enforcer
    pub fn security_enforcer(&self) -> &SecurityEnforcer {
        &self.security_enforcer
    }

    /// Get security enforcer mutably
    pub fn security_enforcer_mut(&mut self) -> &mut SecurityEnforcer {
        &mut self.security_enforcer
    }

    /// Get namespace by ID
    pub fn get_namespace(&self, id: NamespaceId) -> Option<&NamespaceInfo> {
        self.manager.get_namespace(id)
    }

    /// Get total namespace count
    pub fn namespace_count(&self) -> usize {
        self.manager.total_namespaces()
    }
}

impl Default for NamespaceIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
