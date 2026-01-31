//! # Meta-Cognition
//!
//! Implements meta-cognitive processes for self-awareness.
//! Supports metacognitive monitoring and control.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// META-COGNITIVE TYPES
// ============================================================================

/// Cognitive state
#[derive(Debug, Clone)]
pub struct CognitiveState {
    /// State ID
    pub id: u64,
    /// Knowledge level
    pub knowledge: BTreeMap<String, f64>,
    /// Confidence level
    pub confidence: f64,
    /// Attention focus
    pub attention: Vec<String>,
    /// Cognitive load
    pub load: f64,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Meta-judgment
#[derive(Debug, Clone)]
pub struct MetaJudgment {
    /// Judgment ID
    pub id: u64,
    /// Type
    pub judgment_type: JudgmentType,
    /// Target
    pub target: String,
    /// Confidence
    pub confidence: f64,
    /// Basis
    pub basis: JudgmentBasis,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Judgment type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JudgmentType {
    FeelingOfKnowing,
    JudgmentOfLearning,
    TipOfTongue,
    ConfidenceJudgment,
    EaseOfLearning,
    SourceMonitoring,
}

/// Judgment basis
#[derive(Debug, Clone)]
pub enum JudgmentBasis {
    Fluency(f64),
    Familiarity(f64),
    Cue { cues: Vec<String>, strength: f64 },
    Retrieval { attempts: u32, success: bool },
    Combined(Vec<JudgmentBasis>),
}

/// Metacognitive control action
#[derive(Debug, Clone)]
pub struct ControlAction {
    /// Action ID
    pub id: u64,
    /// Type
    pub action_type: ControlType,
    /// Target
    pub target: String,
    /// Parameters
    pub parameters: BTreeMap<String, f64>,
    /// Triggered by
    pub triggered_by: u64,
}

/// Control type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlType {
    AllocateAttention,
    AdjustEffort,
    SelectStrategy,
    TerminateSearch,
    RequestHelp,
    Delegate,
    Retry,
}

/// Self-model
#[derive(Debug, Clone)]
pub struct SelfModel {
    /// Model ID
    pub id: u64,
    /// Strengths
    pub strengths: BTreeMap<String, f64>,
    /// Weaknesses
    pub weaknesses: BTreeMap<String, f64>,
    /// Preferences
    pub preferences: BTreeMap<String, f64>,
    /// History accuracy
    pub accuracy_history: Vec<f64>,
}

/// Monitoring result
#[derive(Debug, Clone)]
pub struct MonitoringResult {
    /// Current state
    pub state: CognitiveState,
    /// Judgments
    pub judgments: Vec<MetaJudgment>,
    /// Recommended actions
    pub actions: Vec<ControlAction>,
    /// Alerts
    pub alerts: Vec<Alert>,
}

/// Alert
#[derive(Debug, Clone)]
pub struct Alert {
    /// Alert ID
    pub id: u64,
    /// Severity
    pub severity: AlertSeverity,
    /// Message
    pub message: String,
    /// Domain
    pub domain: String,
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

// ============================================================================
// META-COGNITIVE ENGINE
// ============================================================================

/// Meta-cognitive engine
pub struct MetaCognitiveEngine {
    /// Current state
    current_state: CognitiveState,
    /// State history
    state_history: Vec<CognitiveState>,
    /// Judgments
    judgments: BTreeMap<u64, MetaJudgment>,
    /// Actions taken
    actions: Vec<ControlAction>,
    /// Self-model
    self_model: SelfModel,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: MetaCognitiveConfig,
    /// Statistics
    stats: MetaCognitiveStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct MetaCognitiveConfig {
    /// Load threshold for alert
    pub load_threshold: f64,
    /// Confidence threshold
    pub confidence_threshold: f64,
    /// History size
    pub history_size: usize,
}

impl Default for MetaCognitiveConfig {
    fn default() -> Self {
        Self {
            load_threshold: 0.8,
            confidence_threshold: 0.5,
            history_size: 100,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct MetaCognitiveStats {
    /// Judgments made
    pub judgments_made: u64,
    /// Actions triggered
    pub actions_triggered: u64,
    /// Alerts generated
    pub alerts_generated: u64,
}

impl MetaCognitiveEngine {
    /// Create new engine
    pub fn new(config: MetaCognitiveConfig) -> Self {
        let now = Timestamp::now();

        Self {
            current_state: CognitiveState {
                id: 0,
                knowledge: BTreeMap::new(),
                confidence: 0.5,
                attention: Vec::new(),
                load: 0.0,
                timestamp: now,
            },
            state_history: Vec::new(),
            judgments: BTreeMap::new(),
            actions: Vec::new(),
            self_model: SelfModel {
                id: 1,
                strengths: BTreeMap::new(),
                weaknesses: BTreeMap::new(),
                preferences: BTreeMap::new(),
                accuracy_history: Vec::new(),
            },
            next_id: AtomicU64::new(2),
            config,
            stats: MetaCognitiveStats::default(),
        }
    }

    /// Update knowledge
    pub fn update_knowledge(&mut self, domain: &str, level: f64) {
        self.current_state
            .knowledge
            .insert(domain.into(), level.clamp(0.0, 1.0));
        self.current_state.timestamp = Timestamp::now();
    }

    /// Update attention
    pub fn focus_attention(&mut self, targets: Vec<String>) {
        self.current_state.attention = targets;
        self.current_state.timestamp = Timestamp::now();
    }

    /// Update cognitive load
    pub fn update_load(&mut self, load: f64) {
        self.current_state.load = load.clamp(0.0, 1.0);
        self.current_state.timestamp = Timestamp::now();
    }

    /// Make judgment
    pub fn make_judgment(
        &mut self,
        judgment_type: JudgmentType,
        target: &str,
        basis: JudgmentBasis,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let confidence = self.compute_confidence(&basis);

        let judgment = MetaJudgment {
            id,
            judgment_type,
            target: target.into(),
            confidence,
            basis,
            timestamp: Timestamp::now(),
        };

        self.judgments.insert(id, judgment);
        self.stats.judgments_made += 1;

        id
    }

    fn compute_confidence(&self, basis: &JudgmentBasis) -> f64 {
        match basis {
            JudgmentBasis::Fluency(f) => *f,
            JudgmentBasis::Familiarity(f) => *f,
            JudgmentBasis::Cue { strength, .. } => *strength,
            JudgmentBasis::Retrieval { attempts, success } => {
                if *success {
                    1.0 / (*attempts as f64).max(1.0)
                } else {
                    0.1 / (*attempts as f64).max(1.0)
                }
            },
            JudgmentBasis::Combined(bases) => {
                if bases.is_empty() {
                    0.5
                } else {
                    let sum: f64 = bases.iter().map(|b| self.compute_confidence(b)).sum();
                    sum / bases.len() as f64
                }
            },
        }
    }

    /// Feeling of knowing
    pub fn feeling_of_knowing(&mut self, domain: &str) -> f64 {
        let base_knowledge = self
            .current_state
            .knowledge
            .get(domain)
            .copied()
            .unwrap_or(0.0);
        let familiarity = self
            .self_model
            .strengths
            .get(domain)
            .copied()
            .unwrap_or(0.5);

        let fok = (base_knowledge + familiarity) / 2.0;

        self.make_judgment(
            JudgmentType::FeelingOfKnowing,
            domain,
            JudgmentBasis::Combined(vec![
                JudgmentBasis::Familiarity(familiarity),
                JudgmentBasis::Fluency(base_knowledge),
            ]),
        );

        fok
    }

    /// Judgment of learning
    pub fn judgment_of_learning(&mut self, item: &str, ease: f64) -> f64 {
        let jol = ease * 0.7 + self.current_state.confidence * 0.3;

        self.make_judgment(
            JudgmentType::JudgmentOfLearning,
            item,
            JudgmentBasis::Fluency(ease),
        );

        jol
    }

    /// Monitor cognitive state
    pub fn monitor(&mut self) -> MonitoringResult {
        let mut alerts = Vec::new();
        let mut recommended_actions = Vec::new();

        // Check load
        if self.current_state.load > self.config.load_threshold {
            let alert_id = self.next_id.fetch_add(1, Ordering::Relaxed);
            alerts.push(Alert {
                id: alert_id,
                severity: AlertSeverity::Warning,
                message: "Cognitive load is high".into(),
                domain: "load".into(),
            });

            // Recommend action
            let action_id = self.next_id.fetch_add(1, Ordering::Relaxed);
            recommended_actions.push(ControlAction {
                id: action_id,
                action_type: ControlType::Delegate,
                target: "current_task".into(),
                parameters: BTreeMap::new(),
                triggered_by: alert_id,
            });
        }

        // Check confidence
        if self.current_state.confidence < self.config.confidence_threshold {
            let alert_id = self.next_id.fetch_add(1, Ordering::Relaxed);
            alerts.push(Alert {
                id: alert_id,
                severity: AlertSeverity::Info,
                message: "Low confidence detected".into(),
                domain: "confidence".into(),
            });

            let action_id = self.next_id.fetch_add(1, Ordering::Relaxed);
            recommended_actions.push(ControlAction {
                id: action_id,
                action_type: ControlType::RequestHelp,
                target: "current_problem".into(),
                parameters: BTreeMap::new(),
                triggered_by: alert_id,
            });
        }

        self.stats.alerts_generated += alerts.len() as u64;

        // Collect recent judgments
        let recent_judgments: Vec<_> = self.judgments.values().cloned().collect();

        MonitoringResult {
            state: self.current_state.clone(),
            judgments: recent_judgments,
            actions: recommended_actions,
            alerts,
        }
    }

    /// Execute control action
    pub fn execute_control(&mut self, action: ControlAction) {
        match action.action_type {
            ControlType::AllocateAttention => {
                if let Some(target) = action.parameters.get("target") {
                    self.current_state.attention.clear();
                    self.current_state
                        .attention
                        .push(format!("focus_{}", *target as i32));
                }
            },
            ControlType::AdjustEffort => {
                if let Some(&effort) = action.parameters.get("effort") {
                    self.current_state.load = (self.current_state.load + effort).clamp(0.0, 1.0);
                }
            },
            ControlType::SelectStrategy => {
                // Strategy selection would update approach
            },
            ControlType::TerminateSearch => {
                self.current_state.load *= 0.5;
            },
            ControlType::RequestHelp => {
                // Would trigger help request
            },
            ControlType::Delegate => {
                self.current_state.load *= 0.3;
            },
            ControlType::Retry => {
                // Reset for retry
            },
        }

        self.actions.push(action);
        self.stats.actions_triggered += 1;
    }

    /// Update self-model
    pub fn update_self_model(&mut self, domain: &str, performance: f64, expected: f64) {
        let error = (performance - expected).abs();

        if performance > expected + 0.1 {
            *self
                .self_model
                .strengths
                .entry(domain.into())
                .or_insert(0.5) += 0.05;
            self.self_model
                .strengths
                .get_mut(domain)
                .map(|v| *v = (*v).min(1.0));
        } else if performance < expected - 0.1 {
            *self
                .self_model
                .weaknesses
                .entry(domain.into())
                .or_insert(0.5) += 0.05;
            self.self_model
                .weaknesses
                .get_mut(domain)
                .map(|v| *v = (*v).min(1.0));
        }

        self.self_model.accuracy_history.push(1.0 - error);

        // Limit history
        if self.self_model.accuracy_history.len() > self.config.history_size {
            self.self_model.accuracy_history.remove(0);
        }
    }

    /// Get calibration (how accurate are judgments)
    pub fn calibration(&self) -> f64 {
        if self.self_model.accuracy_history.is_empty() {
            return 0.5;
        }

        let sum: f64 = self.self_model.accuracy_history.iter().sum();
        sum / self.self_model.accuracy_history.len() as f64
    }

    /// Snapshot state
    pub fn snapshot(&mut self) {
        self.current_state.id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.state_history.push(self.current_state.clone());

        if self.state_history.len() > self.config.history_size {
            self.state_history.remove(0);
        }
    }

    /// Get current state
    pub fn state(&self) -> &CognitiveState {
        &self.current_state
    }

    /// Get self model
    pub fn self_model(&self) -> &SelfModel {
        &self.self_model
    }

    /// Get statistics
    pub fn stats(&self) -> &MetaCognitiveStats {
        &self.stats
    }
}

impl Default for MetaCognitiveEngine {
    fn default() -> Self {
        Self::new(MetaCognitiveConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_knowledge() {
        let mut engine = MetaCognitiveEngine::default();

        engine.update_knowledge("math", 0.8);

        assert!((engine.state().knowledge.get("math").unwrap() - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_make_judgment() {
        let mut engine = MetaCognitiveEngine::default();

        let id = engine.make_judgment(
            JudgmentType::ConfidenceJudgment,
            "test",
            JudgmentBasis::Fluency(0.7),
        );

        assert!(engine.judgments.get(&id).is_some());
    }

    #[test]
    fn test_feeling_of_knowing() {
        let mut engine = MetaCognitiveEngine::default();

        engine.update_knowledge("rust", 0.9);
        engine.self_model.strengths.insert("rust".into(), 0.8);

        let fok = engine.feeling_of_knowing("rust");
        assert!(fok > 0.7);
    }

    #[test]
    fn test_monitor_high_load() {
        let mut engine = MetaCognitiveEngine::default();

        engine.update_load(0.9);

        let result = engine.monitor();

        assert!(!result.alerts.is_empty());
        assert!(!result.actions.is_empty());
    }

    #[test]
    fn test_update_self_model() {
        let mut engine = MetaCognitiveEngine::default();

        // Good performance
        engine.update_self_model("coding", 0.9, 0.7);

        assert!(engine.self_model.strengths.get("coding").unwrap_or(&0.0) > &0.5);
    }

    #[test]
    fn test_calibration() {
        let mut engine = MetaCognitiveEngine::default();

        for i in 0..10 {
            engine.update_self_model("test", (i % 2) as f64, 0.5);
        }

        let cal = engine.calibration();
        assert!(cal > 0.0 && cal < 1.0);
    }
}
