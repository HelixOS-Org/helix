//! Semantic Memory
//!
//! This module provides pattern storage and retrieval for knowledge representation.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{EpisodeId, PatternId, Timestamp};

/// Pattern category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PatternCategory {
    /// Resource usage pattern
    ResourceUsage,
    /// Error pattern
    Error,
    /// Performance pattern
    Performance,
    /// Security pattern
    Security,
    /// User behavior pattern
    UserBehavior,
    /// System behavior pattern
    SystemBehavior,
    /// Recovery pattern
    Recovery,
    /// Optimization pattern
    Optimization,
    /// Failure pattern
    Failure,
    /// Success pattern
    Success,
}

impl PatternCategory {
    /// Get category name
    pub fn name(&self) -> &'static str {
        match self {
            Self::ResourceUsage => "resource_usage",
            Self::Error => "error",
            Self::Performance => "performance",
            Self::Security => "security",
            Self::UserBehavior => "user_behavior",
            Self::SystemBehavior => "system_behavior",
            Self::Recovery => "recovery",
            Self::Optimization => "optimization",
            Self::Failure => "failure",
            Self::Success => "success",
        }
    }
}

/// Pattern confidence level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PatternConfidence {
    /// Speculative (< 50%)
    Speculative = 0,
    /// Low (50-70%)
    Low         = 1,
    /// Medium (70-85%)
    Medium      = 2,
    /// High (85-95%)
    High        = 3,
    /// Very High (> 95%)
    VeryHigh    = 4,
}

impl PatternConfidence {
    /// Get confidence name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Speculative => "speculative",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::VeryHigh => "very_high",
        }
    }

    /// From percentage
    pub fn from_percentage(pct: f32) -> Self {
        match pct {
            x if x >= 0.95 => Self::VeryHigh,
            x if x >= 0.85 => Self::High,
            x if x >= 0.70 => Self::Medium,
            x if x >= 0.50 => Self::Low,
            _ => Self::Speculative,
        }
    }

    /// To percentage (midpoint)
    pub fn to_percentage(&self) -> f32 {
        match self {
            Self::Speculative => 0.30,
            Self::Low => 0.60,
            Self::Medium => 0.77,
            Self::High => 0.90,
            Self::VeryHigh => 0.97,
        }
    }
}

/// Condition operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionOperator {
    /// Equal
    Eq,
    /// Not equal
    Ne,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,
    /// Contains
    Contains,
    /// Starts with
    StartsWith,
    /// Ends with
    EndsWith,
    /// Matches regex
    Matches,
}

impl ConditionOperator {
    /// Get operator name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Ne => "ne",
            Self::Lt => "lt",
            Self::Le => "le",
            Self::Gt => "gt",
            Self::Ge => "ge",
            Self::Contains => "contains",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::Matches => "matches",
        }
    }
}

/// Condition for pattern
#[derive(Debug, Clone)]
pub struct PatternCondition {
    /// Variable name
    pub variable: String,
    /// Operator
    pub operator: ConditionOperator,
    /// Value
    pub value: String,
}

impl PatternCondition {
    /// Create new condition
    pub fn new(variable: String, operator: ConditionOperator, value: String) -> Self {
        Self {
            variable,
            operator,
            value,
        }
    }
}

/// Semantic pattern
#[derive(Debug, Clone)]
pub struct SemanticPattern {
    /// Pattern ID
    pub id: PatternId,
    /// Name
    pub name: String,
    /// Category
    pub category: PatternCategory,
    /// Description
    pub description: String,
    /// Conditions
    pub conditions: Vec<PatternCondition>,
    /// Recommended action
    pub action: Option<String>,
    /// Confidence
    pub confidence: PatternConfidence,
    /// Observation count
    pub observations: u64,
    /// Success rate when action applied
    pub success_rate: f32,
    /// First seen
    pub first_seen: Timestamp,
    /// Last seen
    pub last_seen: Timestamp,
    /// Source episodes
    pub source_episodes: Vec<EpisodeId>,
}

impl SemanticPattern {
    /// Create new pattern
    pub fn new(id: PatternId, name: String, category: PatternCategory) -> Self {
        Self {
            id,
            name,
            category,
            description: String::new(),
            conditions: Vec::new(),
            action: None,
            confidence: PatternConfidence::Speculative,
            observations: 0,
            success_rate: 0.0,
            first_seen: Timestamp::new(0),
            last_seen: Timestamp::new(0),
            source_episodes: Vec::new(),
        }
    }

    /// Add condition
    pub fn add_condition(&mut self, condition: PatternCondition) {
        self.conditions.push(condition);
    }

    /// Set action
    pub fn set_action(&mut self, action: String) {
        self.action = Some(action);
    }

    /// Record observation
    pub fn record_observation(&mut self, timestamp: Timestamp, success: bool) {
        if self.observations == 0 {
            self.first_seen = timestamp;
        }
        self.last_seen = timestamp;
        self.observations += 1;

        // Update success rate (exponential moving average)
        let alpha = 0.1;
        let success_val = if success { 1.0 } else { 0.0 };
        self.success_rate = alpha * success_val + (1.0 - alpha) * self.success_rate;

        // Update confidence
        self.update_confidence();
    }

    /// Update confidence based on observations
    fn update_confidence(&mut self) {
        let base_conf = match self.observations {
            0..=5 => 0.30,
            6..=20 => 0.55,
            21..=50 => 0.75,
            51..=100 => 0.88,
            _ => 0.95,
        };

        // Adjust by success rate
        let adjusted = base_conf * (0.5 + 0.5 * self.success_rate);
        self.confidence = PatternConfidence::from_percentage(adjusted);
    }

    /// Is reliable
    pub fn is_reliable(&self) -> bool {
        self.confidence >= PatternConfidence::Medium && self.observations >= 10
    }
}

/// Semantic memory store
#[derive(Debug)]
pub struct SemanticMemory {
    /// Patterns
    patterns: BTreeMap<PatternId, SemanticPattern>,
    /// Patterns by category
    by_category: BTreeMap<PatternCategory, Vec<PatternId>>,
    /// Patterns by name
    by_name: BTreeMap<String, PatternId>,
    /// Pattern counter
    counter: AtomicU64,
    /// Max patterns
    max_patterns: usize,
}

impl SemanticMemory {
    /// Create new semantic memory
    pub fn new() -> Self {
        Self {
            patterns: BTreeMap::new(),
            by_category: BTreeMap::new(),
            by_name: BTreeMap::new(),
            counter: AtomicU64::new(0),
            max_patterns: 100000,
        }
    }

    /// Create pattern
    pub fn create_pattern(&mut self, name: String, category: PatternCategory) -> PatternId {
        // Check if pattern with this name exists
        if let Some(&id) = self.by_name.get(&name) {
            return id;
        }

        let id = PatternId(self.counter.fetch_add(1, Ordering::Relaxed));
        let pattern = SemanticPattern::new(id, name.clone(), category);

        self.by_name.insert(name, id);
        self.by_category
            .entry(category)
            .or_default()
            .push(id);
        self.patterns.insert(id, pattern);

        id
    }

    /// Get pattern
    pub fn get(&self, id: PatternId) -> Option<&SemanticPattern> {
        self.patterns.get(&id)
    }

    /// Get pattern mutably
    pub fn get_mut(&mut self, id: PatternId) -> Option<&mut SemanticPattern> {
        self.patterns.get_mut(&id)
    }

    /// Find by name
    pub fn find_by_name(&self, name: &str) -> Option<&SemanticPattern> {
        self.by_name.get(name).and_then(|id| self.patterns.get(id))
    }

    /// Find by category
    pub fn find_by_category(&self, category: PatternCategory) -> Vec<&SemanticPattern> {
        self.by_category
            .get(&category)
            .map(|ids| ids.iter().filter_map(|id| self.patterns.get(id)).collect())
            .unwrap_or_default()
    }

    /// Find reliable patterns
    pub fn find_reliable(&self) -> Vec<&SemanticPattern> {
        self.patterns.values().filter(|p| p.is_reliable()).collect()
    }

    /// Find matching patterns (simplified)
    pub fn find_matching(&self, _conditions: &BTreeMap<String, String>) -> Vec<&SemanticPattern> {
        // Simplified: would implement proper condition matching
        self.patterns.values().collect()
    }

    /// Record observation for pattern
    pub fn record_observation(&mut self, id: PatternId, timestamp: Timestamp, success: bool) {
        if let Some(pattern) = self.patterns.get_mut(&id) {
            pattern.record_observation(timestamp, success);
        }
    }

    /// Pattern count
    pub fn count(&self) -> usize {
        self.patterns.len()
    }

    /// Reliable pattern count
    pub fn reliable_count(&self) -> usize {
        self.patterns.values().filter(|p| p.is_reliable()).count()
    }
}

impl Default for SemanticMemory {
    fn default() -> Self {
        Self::new()
    }
}
