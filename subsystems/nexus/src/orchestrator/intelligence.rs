//! Orchestrator Intelligence
//!
//! System-wide analysis and recommendations.

use alloc::string::String;
use alloc::vec::Vec;

use super::{
    DecisionId, DecisionStatus, DecisionType, HealthLevel, OrchestratorEventType,
    OrchestratorManager, SubsystemId, SubsystemState, subsystems,
};

// ============================================================================
// ANALYSIS TYPES
// ============================================================================

/// Orchestrator analysis
#[derive(Debug, Clone)]
pub struct OrchestratorAnalysis {
    /// System health score (0-100)
    pub health_score: f32,
    /// Decision efficiency (0-100)
    pub decision_efficiency: f32,
    /// Subsystem statuses
    pub subsystem_statuses: Vec<SubsystemStatus>,
    /// Issues
    pub issues: Vec<OrchestratorIssue>,
    /// Recommendations
    pub recommendations: Vec<OrchestratorRecommendation>,
}

/// Subsystem status summary
#[derive(Debug, Clone)]
pub struct SubsystemStatus {
    /// Subsystem ID
    pub id: SubsystemId,
    /// Name
    pub name: String,
    /// Health level
    pub health: HealthLevel,
    /// Pending issues
    pub pending_issues: u32,
}

/// Orchestrator issue
#[derive(Debug, Clone)]
pub struct OrchestratorIssue {
    /// Issue type
    pub issue_type: OrchestratorIssueType,
    /// Severity (1-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Affected subsystem
    pub subsystem: Option<SubsystemId>,
}

/// Orchestrator issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorIssueType {
    /// Subsystem degraded
    SubsystemDegraded,
    /// Subsystem critical
    SubsystemCritical,
    /// Decision backlog
    DecisionBacklog,
    /// Resource contention
    ResourceContention,
    /// Policy conflict
    PolicyConflict,
    /// Communication failure
    CommunicationFailure,
}

/// Orchestrator recommendation
#[derive(Debug, Clone)]
pub struct OrchestratorRecommendation {
    /// Action
    pub action: OrchestratorAction,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Orchestrator action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorAction {
    /// Increase subsystem priority
    IncreasePriority,
    /// Reduce subsystem priority
    ReducePriority,
    /// Restart subsystem
    RestartSubsystem,
    /// Change policy
    ChangePolicy,
    /// Clear decision backlog
    ClearBacklog,
    /// Redistribute resources
    RedistributeResources,
}

// ============================================================================
// ORCHESTRATOR INTELLIGENCE
// ============================================================================

/// Orchestrator Intelligence
pub struct OrchestratorIntelligence {
    /// Manager
    manager: OrchestratorManager,
}

impl OrchestratorIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            manager: OrchestratorManager::new(),
        }
    }

    /// Initialize with standard subsystems
    pub fn initialize(&mut self) {
        let standard = [
            (subsystems::MEMORY, "memory"),
            (subsystems::SCHEDULER, "scheduler"),
            (subsystems::FILESYSTEM, "filesystem"),
            (subsystems::NETWORK, "network"),
            (subsystems::BLOCK, "block"),
            (subsystems::POWER, "power"),
            (subsystems::THERMAL, "thermal"),
            (subsystems::SECURITY, "security"),
            (subsystems::VIRTUALIZATION, "virtualization"),
            (subsystems::DRIVERS, "drivers"),
            (subsystems::IPC, "ipc"),
            (subsystems::INTERRUPTS, "interrupts"),
        ];

        for (id, name) in standard {
            self.manager
                .register_subsystem(SubsystemState::new(id, String::from(name)));
        }
    }

    /// Report subsystem health
    #[inline]
    pub fn report_health(&mut self, subsystem: SubsystemId, score: u32, timestamp: u64) {
        if let Some(state) = self.manager.get_subsystem_mut(subsystem) {
            state.set_health_score(score);
            state.health = HealthLevel::from_score(score as u8);
            state.touch(timestamp);
        }
    }

    /// Request decision
    #[inline(always)]
    pub fn request_decision(
        &mut self,
        decision_type: DecisionType,
        source: SubsystemId,
        reason: String,
        timestamp: u64,
    ) -> DecisionId {
        self.manager
            .create_decision(decision_type, source, reason, timestamp)
    }

    /// Analyze system
    pub fn analyze(&self) -> OrchestratorAnalysis {
        let mut health_score = 100.0f32;
        let mut decision_efficiency = 100.0f32;
        let mut issues = Vec::new();
        let mut recommendations = Vec::new();

        // Collect subsystem statuses
        let subsystem_statuses: Vec<SubsystemStatus> = self
            .manager
            .subsystems
            .values()
            .map(|s| SubsystemStatus {
                id: s.id,
                name: s.name.clone(),
                health: s.health,
                pending_issues: s.pending_issues(),
            })
            .collect();

        // Check for critical subsystems
        for state in self.manager.subsystems.values() {
            match state.health {
                HealthLevel::Critical => {
                    health_score -= 30.0;
                    issues.push(OrchestratorIssue {
                        issue_type: OrchestratorIssueType::SubsystemCritical,
                        severity: 10,
                        description: alloc::format!("Subsystem {} is critical", state.name),
                        subsystem: Some(state.id),
                    });
                    recommendations.push(OrchestratorRecommendation {
                        action: OrchestratorAction::RestartSubsystem,
                        expected_improvement: 30.0,
                        reason: alloc::format!("Restart {} to recover", state.name),
                    });
                },
                HealthLevel::Degraded => {
                    health_score -= 15.0;
                    issues.push(OrchestratorIssue {
                        issue_type: OrchestratorIssueType::SubsystemDegraded,
                        severity: 6,
                        description: alloc::format!("Subsystem {} is degraded", state.name),
                        subsystem: Some(state.id),
                    });
                    recommendations.push(OrchestratorRecommendation {
                        action: OrchestratorAction::IncreasePriority,
                        expected_improvement: 15.0,
                        reason: alloc::format!("Increase priority for {}", state.name),
                    });
                },
                HealthLevel::Warning => {
                    health_score -= 5.0;
                },
                _ => {},
            }
        }

        // Check decision backlog
        let pending_count = self.manager.pending_decisions.len();
        if pending_count > 50 {
            decision_efficiency -= 30.0;
            issues.push(OrchestratorIssue {
                issue_type: OrchestratorIssueType::DecisionBacklog,
                severity: 7,
                description: alloc::format!("{} decisions pending", pending_count),
                subsystem: None,
            });
            recommendations.push(OrchestratorRecommendation {
                action: OrchestratorAction::ClearBacklog,
                expected_improvement: 25.0,
                reason: String::from("Process or cancel stale decisions"),
            });
        } else if pending_count > 20 {
            decision_efficiency -= 10.0;
        }

        // Check high priority decisions not being handled
        let high_priority = self.manager.high_priority_decisions();
        if high_priority.len() > 5 {
            decision_efficiency -= 20.0;
            issues.push(OrchestratorIssue {
                issue_type: OrchestratorIssueType::DecisionBacklog,
                severity: 8,
                description: alloc::format!(
                    "{} high priority decisions pending",
                    high_priority.len()
                ),
                subsystem: None,
            });
        }

        health_score = health_score.max(0.0);
        decision_efficiency = decision_efficiency.max(0.0);

        OrchestratorAnalysis {
            health_score,
            decision_efficiency,
            subsystem_statuses,
            issues,
            recommendations,
        }
    }

    /// Get manager
    #[inline(always)]
    pub fn manager(&self) -> &OrchestratorManager {
        &self.manager
    }

    /// Get manager mutably
    #[inline(always)]
    pub fn manager_mut(&mut self) -> &mut OrchestratorManager {
        &mut self.manager
    }

    /// Tick - process pending work
    pub fn tick(&mut self, timestamp: u64) {
        // Auto-approve high confidence decisions
        let high_conf_ids: Vec<DecisionId> = self
            .manager
            .pending_decisions
            .iter()
            .filter(|d| d.status == DecisionStatus::Pending && d.confidence >= 80)
            .map(|d| d.id)
            .collect();

        for id in high_conf_ids {
            self.manager.approve_decision(id);
        }

        // Record tick event periodically
        if timestamp % 1000 == 0 {
            self.manager
                .record_event(OrchestratorEventType::PolicyChanged, timestamp);
        }
    }
}

impl Default for OrchestratorIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
