//! # Introspection Engine
//!
//! Self-examination of cognitive processes and state.
//! Monitors internal operations and provides insights.
//!
//! Part of Year 2 COGNITION - Reflect/Introspect

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// INTROSPECTION TYPES
// ============================================================================

/// Cognitive state
#[derive(Debug, Clone)]
pub struct CognitiveState {
    /// State ID
    pub id: u64,
    /// Active processes
    pub processes: Vec<ProcessState>,
    /// Resource usage
    pub resources: ResourceUsage,
    /// Attention
    pub attention: AttentionState,
    /// Goals
    pub goals: Vec<GoalState>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Process state
#[derive(Debug, Clone)]
pub struct ProcessState {
    /// Process ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Domain
    pub domain: CognitiveDomain,
    /// Status
    pub status: ProcessStatus,
    /// Load (0-1)
    pub load: f64,
    /// Duration (ms)
    pub duration: u64,
}

/// Cognitive domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CognitiveDomain {
    Sense,
    Understand,
    Reason,
    Decide,
    Act,
    Reflect,
    Learn,
    Memory,
}

/// Process status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    Idle,
    Active,
    Blocked,
    Completing,
}

/// Resource usage
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// Memory usage
    pub memory: f64,
    /// Compute usage
    pub compute: f64,
    /// Attention allocation
    pub attention: f64,
}

/// Attention state
#[derive(Debug, Clone)]
pub struct AttentionState {
    /// Focus targets
    pub focus: Vec<u64>,
    /// Focus intensity
    pub intensity: f64,
    /// Duration
    pub duration: u64,
    /// Divided attention
    pub divided: bool,
}

/// Goal state
#[derive(Debug, Clone)]
pub struct GoalState {
    /// Goal ID
    pub id: u64,
    /// Description
    pub description: String,
    /// Priority
    pub priority: u32,
    /// Progress
    pub progress: f64,
    /// Active
    pub active: bool,
}

/// Introspection insight
#[derive(Debug, Clone)]
pub struct Insight {
    /// Insight ID
    pub id: u64,
    /// Type
    pub insight_type: InsightType,
    /// Description
    pub description: String,
    /// Severity
    pub severity: Severity,
    /// Recommendation
    pub recommendation: Option<String>,
    /// Created
    pub created: Timestamp,
}

/// Insight type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsightType {
    Performance,
    Bottleneck,
    Anomaly,
    Opportunity,
    Pattern,
    Warning,
}

/// Severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================================
// INTROSPECTION ENGINE
// ============================================================================

/// Introspection engine
pub struct IntrospectionEngine {
    /// State history
    history: Vec<CognitiveState>,
    /// Current processes
    processes: BTreeMap<u64, ProcessState>,
    /// Current goals
    goals: BTreeMap<u64, GoalState>,
    /// Generated insights
    insights: Vec<Insight>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: IntrospectionConfig,
    /// Statistics
    stats: IntrospectionStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct IntrospectionConfig {
    /// Maximum history
    pub max_history: usize,
    /// Anomaly threshold
    pub anomaly_threshold: f64,
    /// Insight interval (ms)
    pub insight_interval: u64,
}

impl Default for IntrospectionConfig {
    fn default() -> Self {
        Self {
            max_history: 100,
            anomaly_threshold: 2.0,
            insight_interval: 1000,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct IntrospectionStats {
    /// States captured
    pub states_captured: u64,
    /// Insights generated
    pub insights_generated: u64,
    /// Anomalies detected
    pub anomalies_detected: u64,
}

impl IntrospectionEngine {
    /// Create new engine
    pub fn new(config: IntrospectionConfig) -> Self {
        Self {
            history: Vec::new(),
            processes: BTreeMap::new(),
            goals: BTreeMap::new(),
            insights: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: IntrospectionStats::default(),
        }
    }

    /// Register process
    pub fn register_process(&mut self, name: &str, domain: CognitiveDomain) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let process = ProcessState {
            id,
            name: name.into(),
            domain,
            status: ProcessStatus::Idle,
            load: 0.0,
            duration: 0,
        };

        self.processes.insert(id, process);

        id
    }

    /// Update process
    pub fn update_process(&mut self, id: u64, status: ProcessStatus, load: f64) {
        if let Some(process) = self.processes.get_mut(&id) {
            process.status = status;
            process.load = load.clamp(0.0, 1.0);
        }
    }

    /// Register goal
    pub fn register_goal(&mut self, description: &str, priority: u32) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let goal = GoalState {
            id,
            description: description.into(),
            priority,
            progress: 0.0,
            active: true,
        };

        self.goals.insert(id, goal);

        id
    }

    /// Update goal progress
    pub fn update_goal(&mut self, id: u64, progress: f64) {
        if let Some(goal) = self.goals.get_mut(&id) {
            goal.progress = progress.clamp(0.0, 1.0);
            if goal.progress >= 1.0 {
                goal.active = false;
            }
        }
    }

    /// Capture current state
    pub fn capture(&mut self) -> CognitiveState {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let state = CognitiveState {
            id,
            processes: self.processes.values().cloned().collect(),
            resources: self.calculate_resources(),
            attention: self.get_attention(),
            goals: self.goals.values().cloned().collect(),
            timestamp: Timestamp::now(),
        };

        self.history.push(state.clone());
        self.stats.states_captured += 1;

        // Limit history
        while self.history.len() > self.config.max_history {
            self.history.remove(0);
        }

        state
    }

    fn calculate_resources(&self) -> ResourceUsage {
        let total_load: f64 = self.processes.values().map(|p| p.load).sum();

        let active_count = self
            .processes
            .values()
            .filter(|p| p.status == ProcessStatus::Active)
            .count();

        ResourceUsage {
            memory: total_load / self.processes.len().max(1) as f64,
            compute: active_count as f64 / self.processes.len().max(1) as f64,
            attention: 0.5, // Default
        }
    }

    fn get_attention(&self) -> AttentionState {
        let active_goals: Vec<u64> = self
            .goals
            .values()
            .filter(|g| g.active)
            .map(|g| g.id)
            .collect();

        AttentionState {
            focus: active_goals.clone(),
            intensity: if active_goals.is_empty() { 0.0 } else { 0.8 },
            duration: 0,
            divided: active_goals.len() > 1,
        }
    }

    /// Analyze and generate insights
    pub fn analyze(&mut self) -> Vec<Insight> {
        let mut new_insights = Vec::new();

        // Check for bottlenecks
        if let Some(insight) = self.check_bottlenecks() {
            new_insights.push(insight);
        }

        // Check for anomalies
        if let Some(insight) = self.check_anomalies() {
            new_insights.push(insight);
            self.stats.anomalies_detected += 1;
        }

        // Check for patterns
        if let Some(insight) = self.check_patterns() {
            new_insights.push(insight);
        }

        // Check resource usage
        if let Some(insight) = self.check_resources() {
            new_insights.push(insight);
        }

        self.stats.insights_generated += new_insights.len() as u64;
        self.insights.extend(new_insights.clone());

        new_insights
    }

    fn check_bottlenecks(&self) -> Option<Insight> {
        // Find blocked processes
        let blocked: Vec<_> = self
            .processes
            .values()
            .filter(|p| p.status == ProcessStatus::Blocked)
            .collect();

        if !blocked.is_empty() {
            return Some(Insight {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                insight_type: InsightType::Bottleneck,
                description: format!("{} processes blocked", blocked.len()),
                severity: if blocked.len() > 2 {
                    Severity::High
                } else {
                    Severity::Medium
                },
                recommendation: Some("Check dependencies and resource allocation".into()),
                created: Timestamp::now(),
            });
        }

        None
    }

    fn check_anomalies(&self) -> Option<Insight> {
        if self.history.len() < 3 {
            return None;
        }

        // Check for sudden load changes
        let recent: Vec<f64> = self
            .history
            .iter()
            .rev()
            .take(5)
            .map(|s| s.resources.compute)
            .collect();

        if recent.len() >= 2 {
            let diff = (recent[0] - recent[1]).abs();
            if diff > self.config.anomaly_threshold * 0.2 {
                return Some(Insight {
                    id: self.next_id.fetch_add(1, Ordering::Relaxed),
                    insight_type: InsightType::Anomaly,
                    description: format!("Sudden load change: {:.2}", diff),
                    severity: Severity::Medium,
                    recommendation: Some("Investigate cause of load spike".into()),
                    created: Timestamp::now(),
                });
            }
        }

        None
    }

    fn check_patterns(&self) -> Option<Insight> {
        // Check for recurring high load
        let high_load_count = self
            .history
            .iter()
            .rev()
            .take(10)
            .filter(|s| s.resources.compute > 0.8)
            .count();

        if high_load_count > 5 {
            return Some(Insight {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                insight_type: InsightType::Pattern,
                description: "Sustained high load pattern detected".into(),
                severity: Severity::Medium,
                recommendation: Some("Consider load balancing or optimization".into()),
                created: Timestamp::now(),
            });
        }

        None
    }

    fn check_resources(&self) -> Option<Insight> {
        let current = self.calculate_resources();

        if current.memory > 0.9 {
            return Some(Insight {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                insight_type: InsightType::Warning,
                description: format!("High memory usage: {:.1}%", current.memory * 100.0),
                severity: Severity::High,
                recommendation: Some("Free unused resources".into()),
                created: Timestamp::now(),
            });
        }

        if current.compute > 0.95 {
            return Some(Insight {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                insight_type: InsightType::Warning,
                description: "Compute saturation".into(),
                severity: Severity::Critical,
                recommendation: Some("Reduce active processes".into()),
                created: Timestamp::now(),
            });
        }

        None
    }

    /// Query state
    pub fn query(&self, domain: Option<CognitiveDomain>) -> Vec<&ProcessState> {
        self.processes
            .values()
            .filter(|p| domain.map(|d| p.domain == d).unwrap_or(true))
            .collect()
    }

    /// Get process
    pub fn get_process(&self, id: u64) -> Option<&ProcessState> {
        self.processes.get(&id)
    }

    /// Get goal
    pub fn get_goal(&self, id: u64) -> Option<&GoalState> {
        self.goals.get(&id)
    }

    /// Get recent states
    pub fn recent_states(&self, count: usize) -> Vec<&CognitiveState> {
        self.history.iter().rev().take(count).collect()
    }

    /// Get insights by type
    pub fn insights_by_type(&self, insight_type: InsightType) -> Vec<&Insight> {
        self.insights
            .iter()
            .filter(|i| i.insight_type == insight_type)
            .collect()
    }

    /// Get high severity insights
    pub fn high_severity_insights(&self) -> Vec<&Insight> {
        self.insights
            .iter()
            .filter(|i| i.severity >= Severity::High)
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &IntrospectionStats {
        &self.stats
    }
}

impl Default for IntrospectionEngine {
    fn default() -> Self {
        Self::new(IntrospectionConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_process() {
        let mut engine = IntrospectionEngine::default();

        let id = engine.register_process("reasoner", CognitiveDomain::Reason);
        assert!(engine.get_process(id).is_some());
    }

    #[test]
    fn test_update_process() {
        let mut engine = IntrospectionEngine::default();

        let id = engine.register_process("learner", CognitiveDomain::Learn);
        engine.update_process(id, ProcessStatus::Active, 0.7);

        let p = engine.get_process(id).unwrap();
        assert_eq!(p.status, ProcessStatus::Active);
        assert_eq!(p.load, 0.7);
    }

    #[test]
    fn test_register_goal() {
        let mut engine = IntrospectionEngine::default();

        let id = engine.register_goal("Complete analysis", 5);
        assert!(engine.get_goal(id).is_some());
    }

    #[test]
    fn test_capture() {
        let mut engine = IntrospectionEngine::default();

        engine.register_process("p1", CognitiveDomain::Sense);
        engine.register_goal("g1", 3);

        let state = engine.capture();
        assert_eq!(state.processes.len(), 1);
        assert_eq!(state.goals.len(), 1);
    }

    #[test]
    fn test_analyze_bottleneck() {
        let mut engine = IntrospectionEngine::default();

        let id = engine.register_process("blocked", CognitiveDomain::Act);
        engine.update_process(id, ProcessStatus::Blocked, 0.5);

        let insights = engine.analyze();
        assert!(
            insights
                .iter()
                .any(|i| i.insight_type == InsightType::Bottleneck)
        );
    }

    #[test]
    fn test_query_domain() {
        let mut engine = IntrospectionEngine::default();

        engine.register_process("sense1", CognitiveDomain::Sense);
        engine.register_process("reason1", CognitiveDomain::Reason);

        let sense_only = engine.query(Some(CognitiveDomain::Sense));
        assert_eq!(sense_only.len(), 1);
    }
}
