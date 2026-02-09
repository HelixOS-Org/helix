//! # Self-Critique Engine
//!
//! Evaluates system performance and identifies areas for improvement.
//! Provides constructive feedback and suggestions.
//!
//! Part of Year 2 COGNITION - Reflection Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// CRITIQUE TYPES
// ============================================================================

/// Critique
#[derive(Debug, Clone)]
pub struct Critique {
    /// Critique ID
    pub id: u64,
    /// Target
    pub target: CritiqueTarget,
    /// Category
    pub category: CritiqueCategory,
    /// Severity
    pub severity: Severity,
    /// Summary
    pub summary: String,
    /// Details
    pub details: String,
    /// Evidence
    pub evidence: Vec<Evidence>,
    /// Suggestions
    pub suggestions: Vec<Suggestion>,
    /// Created
    pub created: Timestamp,
    /// Status
    pub status: CritiqueStatus,
}

/// Critique target
#[derive(Debug, Clone)]
pub enum CritiqueTarget {
    /// Decision
    Decision { id: u64, name: String },
    /// Action
    Action { id: u64, name: String },
    /// Plan
    Plan { id: u64, goal: String },
    /// Model
    Model { name: String, version: u32 },
    /// Process
    Process { name: String },
    /// Output
    Output { id: u64, content_hash: u64 },
}

/// Critique category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CritiqueCategory {
    Accuracy,
    Efficiency,
    Completeness,
    Consistency,
    Relevance,
    Safety,
    Ethics,
    Clarity,
    Robustness,
}

/// Severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info,
    Minor,
    Moderate,
    Major,
    Critical,
}

/// Evidence
#[derive(Debug, Clone)]
pub struct Evidence {
    /// Evidence type
    pub evidence_type: EvidenceType,
    /// Description
    pub description: String,
    /// Source
    pub source: String,
    /// Value
    pub value: EvidenceValue,
}

/// Evidence type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    Metric,
    Observation,
    Comparison,
    Regression,
    Anomaly,
}

/// Evidence value
#[derive(Debug, Clone)]
pub enum EvidenceValue {
    Numeric(f64),
    Boolean(bool),
    Text(String),
    Range { min: f64, max: f64, actual: f64 },
}

/// Suggestion
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Suggestion ID
    pub id: u64,
    /// Action
    pub action: SuggestionAction,
    /// Description
    pub description: String,
    /// Expected impact
    pub expected_impact: Impact,
    /// Effort
    pub effort: Effort,
    /// Status
    pub status: SuggestionStatus,
}

/// Suggestion action
#[derive(Debug, Clone)]
pub enum SuggestionAction {
    Tune { parameter: String, value: String },
    Refactor { component: String },
    AddCheck { description: String },
    RemoveFeature { name: String },
    AddFeature { name: String },
    IncreaseResource { resource: String },
    ModifyProcess { step: String },
}

/// Impact
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Impact {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Effort
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Effort {
    Trivial,
    Small,
    Medium,
    Large,
}

/// Critique status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CritiqueStatus {
    Draft,
    Published,
    Acknowledged,
    Addressed,
    Dismissed,
}

/// Suggestion status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionStatus {
    Proposed,
    Accepted,
    Implemented,
    Rejected,
    Deferred,
}

// ============================================================================
// CRITIQUE ENGINE
// ============================================================================

/// Critique engine
pub struct CritiqueEngine {
    /// Critiques
    critiques: BTreeMap<u64, Critique>,
    /// Rules
    rules: Vec<CritiqueRule>,
    /// Thresholds
    thresholds: BTreeMap<String, f64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: CritiqueConfig,
    /// Statistics
    stats: CritiqueStats,
}

/// Critique rule
#[derive(Debug, Clone)]
pub struct CritiqueRule {
    /// Rule ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Category
    pub category: CritiqueCategory,
    /// Condition
    pub condition: RuleCondition,
    /// Severity
    pub severity: Severity,
    /// Template
    pub template: String,
    /// Enabled
    pub enabled: bool,
}

/// Rule condition
#[derive(Debug, Clone)]
pub enum RuleCondition {
    /// Metric below threshold
    MetricBelow { name: String, threshold: f64 },
    /// Metric above threshold
    MetricAbove { name: String, threshold: f64 },
    /// Pattern detected
    Pattern { pattern: String },
    /// Always true (for custom evaluation)
    Custom { name: String },
}

/// Configuration
#[derive(Debug, Clone)]
pub struct CritiqueConfig {
    /// Enable auto-critique
    pub auto_critique: bool,
    /// Minimum severity to record
    pub min_severity: Severity,
    /// Maximum critiques stored
    pub max_critiques: usize,
}

impl Default for CritiqueConfig {
    fn default() -> Self {
        Self {
            auto_critique: true,
            min_severity: Severity::Minor,
            max_critiques: 1000,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CritiqueStats {
    /// Total critiques
    pub total_critiques: u64,
    /// By category
    pub by_category: BTreeMap<String, u64>,
    /// By severity
    pub by_severity: BTreeMap<String, u64>,
    /// Addressed
    pub addressed: u64,
}

impl CritiqueEngine {
    /// Create new engine
    pub fn new(config: CritiqueConfig) -> Self {
        Self {
            critiques: BTreeMap::new(),
            rules: Vec::new(),
            thresholds: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: CritiqueStats::default(),
        }
    }

    /// Add rule
    #[inline(always)]
    pub fn add_rule(&mut self, rule: CritiqueRule) {
        self.rules.push(rule);
    }

    /// Set threshold
    #[inline(always)]
    pub fn set_threshold(&mut self, name: &str, value: f64) {
        self.thresholds.insert(name.into(), value);
    }

    /// Create critique
    pub fn create_critique(
        &mut self,
        target: CritiqueTarget,
        category: CritiqueCategory,
        severity: Severity,
        summary: &str,
        details: &str,
    ) -> u64 {
        if severity < self.config.min_severity {
            return 0;
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let critique = Critique {
            id,
            target,
            category,
            severity,
            summary: summary.into(),
            details: details.into(),
            evidence: Vec::new(),
            suggestions: Vec::new(),
            created: Timestamp::now(),
            status: CritiqueStatus::Draft,
        };

        self.critiques.insert(id, critique);
        self.update_stats(category, severity);

        // Clean old critiques if needed
        self.cleanup_old();

        id
    }

    fn update_stats(&mut self, category: CritiqueCategory, severity: Severity) {
        self.stats.total_critiques += 1;

        let cat_key = format!("{:?}", category);
        *self.stats.by_category.entry(cat_key).or_insert(0) += 1;

        let sev_key = format!("{:?}", severity);
        *self.stats.by_severity.entry(sev_key).or_insert(0) += 1;
    }

    fn cleanup_old(&mut self) {
        if self.critiques.len() > self.config.max_critiques {
            // Remove oldest addressed critiques
            let to_remove: Vec<u64> = self
                .critiques
                .iter()
                .filter(|(_, c)| c.status == CritiqueStatus::Addressed)
                .map(|(id, _)| *id)
                .take(self.critiques.len() - self.config.max_critiques)
                .collect();

            for id in to_remove {
                self.critiques.remove(&id);
            }
        }
    }

    /// Add evidence
    #[inline]
    pub fn add_evidence(&mut self, critique_id: u64, evidence: Evidence) {
        if let Some(critique) = self.critiques.get_mut(&critique_id) {
            critique.evidence.push(evidence);
        }
    }

    /// Add suggestion
    #[inline]
    pub fn add_suggestion(&mut self, critique_id: u64, suggestion: Suggestion) {
        if let Some(critique) = self.critiques.get_mut(&critique_id) {
            critique.suggestions.push(suggestion);
        }
    }

    /// Evaluate metrics
    pub fn evaluate_metrics(&mut self, metrics: &BTreeMap<String, f64>) -> Vec<u64> {
        let mut new_critiques = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            let triggered = match &rule.condition {
                RuleCondition::MetricBelow { name, threshold } => {
                    metrics.get(name).map(|&v| v < *threshold).unwrap_or(false)
                },
                RuleCondition::MetricAbove { name, threshold } => {
                    metrics.get(name).map(|&v| v > *threshold).unwrap_or(false)
                },
                _ => false,
            };

            if triggered {
                let id = self.create_critique(
                    CritiqueTarget::Process {
                        name: "metrics".into(),
                    },
                    rule.category,
                    rule.severity,
                    &rule.name,
                    &rule.template,
                );

                if id > 0 {
                    new_critiques.push(id);
                }
            }
        }

        new_critiques
    }

    /// Get critique
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&Critique> {
        self.critiques.get(&id)
    }

    /// Update status
    #[inline]
    pub fn update_status(&mut self, id: u64, status: CritiqueStatus) {
        if let Some(critique) = self.critiques.get_mut(&id) {
            let was_addressed = critique.status == CritiqueStatus::Addressed;
            critique.status = status;

            if !was_addressed && status == CritiqueStatus::Addressed {
                self.stats.addressed += 1;
            }
        }
    }

    /// Get by category
    #[inline]
    pub fn by_category(&self, category: CritiqueCategory) -> Vec<&Critique> {
        self.critiques
            .values()
            .filter(|c| c.category == category)
            .collect()
    }

    /// Get by severity
    #[inline]
    pub fn by_severity(&self, severity: Severity) -> Vec<&Critique> {
        self.critiques
            .values()
            .filter(|c| c.severity >= severity)
            .collect()
    }

    /// Get pending
    #[inline]
    pub fn pending(&self) -> Vec<&Critique> {
        self.critiques
            .values()
            .filter(|c| {
                c.status != CritiqueStatus::Addressed && c.status != CritiqueStatus::Dismissed
            })
            .collect()
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &CritiqueStats {
        &self.stats
    }
}

impl Default for CritiqueEngine {
    fn default() -> Self {
        Self::new(CritiqueConfig::default())
    }
}

// ============================================================================
// CRITIQUE BUILDER
// ============================================================================

/// Critique builder
pub struct CritiqueBuilder {
    target: Option<CritiqueTarget>,
    category: CritiqueCategory,
    severity: Severity,
    summary: String,
    details: String,
    evidence: Vec<Evidence>,
    suggestions: Vec<Suggestion>,
}

impl CritiqueBuilder {
    /// Create new builder
    pub fn new(category: CritiqueCategory) -> Self {
        Self {
            target: None,
            category,
            severity: Severity::Minor,
            summary: String::new(),
            details: String::new(),
            evidence: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Set target
    #[inline(always)]
    pub fn target(mut self, target: CritiqueTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// Set severity
    #[inline(always)]
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set summary
    #[inline(always)]
    pub fn summary(mut self, summary: &str) -> Self {
        self.summary = summary.into();
        self
    }

    /// Set details
    #[inline(always)]
    pub fn details(mut self, details: &str) -> Self {
        self.details = details.into();
        self
    }

    /// Add evidence
    #[inline(always)]
    pub fn evidence(mut self, evidence: Evidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Add suggestion
    #[inline]
    pub fn suggest(mut self, action: SuggestionAction, description: &str) -> Self {
        self.suggestions.push(Suggestion {
            id: 0, // Will be assigned
            action,
            description: description.into(),
            expected_impact: Impact::Medium,
            effort: Effort::Medium,
            status: SuggestionStatus::Proposed,
        });
        self
    }

    /// Build
    pub fn build(self, engine: &mut CritiqueEngine) -> u64 {
        let target = self.target.unwrap_or(CritiqueTarget::Process {
            name: "unknown".into(),
        });

        let id = engine.create_critique(
            target,
            self.category,
            self.severity,
            &self.summary,
            &self.details,
        );

        for ev in self.evidence {
            engine.add_evidence(id, ev);
        }

        for mut sugg in self.suggestions {
            sugg.id = engine.next_id.fetch_add(1, Ordering::Relaxed);
            engine.add_suggestion(id, sugg);
        }

        id
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_critique() {
        let mut engine = CritiqueEngine::default();

        let id = engine.create_critique(
            CritiqueTarget::Process {
                name: "test".into(),
            },
            CritiqueCategory::Accuracy,
            Severity::Moderate,
            "Test critique",
            "Details here",
        );

        assert!(engine.get(id).is_some());
    }

    #[test]
    fn test_add_evidence() {
        let mut engine = CritiqueEngine::default();

        let id = engine.create_critique(
            CritiqueTarget::Process {
                name: "test".into(),
            },
            CritiqueCategory::Accuracy,
            Severity::Moderate,
            "Test",
            "Details",
        );

        engine.add_evidence(id, Evidence {
            evidence_type: EvidenceType::Metric,
            description: "Accuracy dropped".into(),
            source: "metrics".into(),
            value: EvidenceValue::Numeric(0.85),
        });

        let critique = engine.get(id).unwrap();
        assert_eq!(critique.evidence.len(), 1);
    }

    #[test]
    fn test_evaluate_metrics() {
        let mut engine = CritiqueEngine::default();

        engine.add_rule(CritiqueRule {
            id: 1,
            name: "Low accuracy".into(),
            category: CritiqueCategory::Accuracy,
            condition: RuleCondition::MetricBelow {
                name: "accuracy".into(),
                threshold: 0.9,
            },
            severity: Severity::Major,
            template: "Accuracy below threshold".into(),
            enabled: true,
        });

        let mut metrics = BTreeMap::new();
        metrics.insert("accuracy".into(), 0.85);

        let critiques = engine.evaluate_metrics(&metrics);
        assert_eq!(critiques.len(), 1);
    }

    #[test]
    fn test_builder() {
        let mut engine = CritiqueEngine::default();

        let id = CritiqueBuilder::new(CritiqueCategory::Efficiency)
            .target(CritiqueTarget::Process {
                name: "test".into(),
            })
            .severity(Severity::Moderate)
            .summary("Performance issue")
            .details("Detailed description")
            .suggest(
                SuggestionAction::Tune {
                    parameter: "buffer_size".into(),
                    value: "1024".into(),
                },
                "Increase buffer size",
            )
            .build(&mut engine);

        let critique = engine.get(id).unwrap();
        assert_eq!(critique.suggestions.len(), 1);
    }

    #[test]
    fn test_filter_by_severity() {
        let mut engine = CritiqueEngine::default();

        engine.create_critique(
            CritiqueTarget::Process { name: "a".into() },
            CritiqueCategory::Accuracy,
            Severity::Minor,
            "Minor issue",
            "",
        );

        engine.create_critique(
            CritiqueTarget::Process { name: "b".into() },
            CritiqueCategory::Accuracy,
            Severity::Major,
            "Major issue",
            "",
        );

        let major = engine.by_severity(Severity::Major);
        assert_eq!(major.len(), 1);
    }
}
