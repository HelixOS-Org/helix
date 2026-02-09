//! # Cognitive Mode Management
//!
//! Manages cognitive operation modes.
//! Supports mode switching and mode-specific behaviors.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// MODE TYPES
// ============================================================================

/// Cognitive operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CognitiveMode {
    /// Normal operation
    Normal,
    /// Learning mode (acquiring new knowledge)
    Learning,
    /// Focused mode (deep processing)
    Focused,
    /// Reactive mode (quick responses)
    Reactive,
    /// Creative mode (exploration)
    Creative,
    /// Analytical mode (detailed analysis)
    Analytical,
    /// Monitoring mode (low activity)
    Monitoring,
    /// Emergency mode (crisis handling)
    Emergency,
    /// Maintenance mode (self-repair)
    Maintenance,
    /// Sleep mode (minimal activity)
    Sleep,
}

impl CognitiveMode {
    /// Get mode characteristics
    pub fn characteristics(&self) -> ModeCharacteristics {
        match self {
            Self::Normal => ModeCharacteristics {
                processing_depth: 0.5,
                response_latency: 0.5,
                resource_usage: 0.5,
                creativity: 0.5,
                focus: 0.5,
            },
            Self::Learning => ModeCharacteristics {
                processing_depth: 0.8,
                response_latency: 0.3,
                resource_usage: 0.7,
                creativity: 0.6,
                focus: 0.7,
            },
            Self::Focused => ModeCharacteristics {
                processing_depth: 0.9,
                response_latency: 0.2,
                resource_usage: 0.8,
                creativity: 0.3,
                focus: 1.0,
            },
            Self::Reactive => ModeCharacteristics {
                processing_depth: 0.2,
                response_latency: 1.0,
                resource_usage: 0.4,
                creativity: 0.2,
                focus: 0.3,
            },
            Self::Creative => ModeCharacteristics {
                processing_depth: 0.6,
                response_latency: 0.4,
                resource_usage: 0.6,
                creativity: 1.0,
                focus: 0.4,
            },
            Self::Analytical => ModeCharacteristics {
                processing_depth: 1.0,
                response_latency: 0.1,
                resource_usage: 0.9,
                creativity: 0.2,
                focus: 0.9,
            },
            Self::Monitoring => ModeCharacteristics {
                processing_depth: 0.1,
                response_latency: 0.7,
                resource_usage: 0.2,
                creativity: 0.1,
                focus: 0.2,
            },
            Self::Emergency => ModeCharacteristics {
                processing_depth: 0.4,
                response_latency: 1.0,
                resource_usage: 1.0,
                creativity: 0.1,
                focus: 0.8,
            },
            Self::Maintenance => ModeCharacteristics {
                processing_depth: 0.3,
                response_latency: 0.2,
                resource_usage: 0.4,
                creativity: 0.1,
                focus: 0.6,
            },
            Self::Sleep => ModeCharacteristics {
                processing_depth: 0.0,
                response_latency: 0.0,
                resource_usage: 0.1,
                creativity: 0.0,
                focus: 0.0,
            },
        }
    }

    /// Get all modes
    pub fn all() -> &'static [CognitiveMode] {
        &[
            Self::Normal,
            Self::Learning,
            Self::Focused,
            Self::Reactive,
            Self::Creative,
            Self::Analytical,
            Self::Monitoring,
            Self::Emergency,
            Self::Maintenance,
            Self::Sleep,
        ]
    }
}

/// Mode characteristics
#[derive(Debug, Clone, Copy)]
pub struct ModeCharacteristics {
    /// Processing depth (0-1)
    pub processing_depth: f64,
    /// Response latency priority (0-1, 1 = fastest)
    pub response_latency: f64,
    /// Resource usage (0-1)
    pub resource_usage: f64,
    /// Creativity level (0-1)
    pub creativity: f64,
    /// Focus level (0-1)
    pub focus: f64,
}

/// Mode transition rule
#[derive(Debug, Clone)]
pub struct ModeTransitionRule {
    /// Rule ID
    pub id: u64,
    /// Rule name
    pub name: String,
    /// From mode
    pub from: CognitiveMode,
    /// To mode
    pub to: CognitiveMode,
    /// Condition
    pub condition: TransitionCondition,
    /// Priority
    pub priority: u32,
    /// Cooldown (ns)
    pub cooldown_ns: u64,
    /// Enabled
    pub enabled: bool,
}

/// Transition condition
#[derive(Debug, Clone)]
pub enum TransitionCondition {
    /// Always transition
    Always,
    /// Time in current mode
    TimeBased(u64),
    /// Load threshold
    LoadThreshold(f64, ThresholdOp),
    /// External trigger
    Trigger(String),
    /// Multiple conditions (AND)
    And(Vec<TransitionCondition>),
    /// Multiple conditions (OR)
    Or(Vec<TransitionCondition>),
}

/// Threshold operator
#[derive(Debug, Clone, Copy)]
pub enum ThresholdOp {
    GreaterThan,
    LessThan,
    Equal,
}

/// Mode transition event
#[derive(Debug, Clone)]
pub struct ModeTransition {
    /// Transition ID
    pub id: u64,
    /// From mode
    pub from: CognitiveMode,
    /// To mode
    pub to: CognitiveMode,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Reason
    pub reason: String,
    /// Rule ID (if rule-based)
    pub rule_id: Option<u64>,
    /// Duration in previous mode (ns)
    pub previous_duration_ns: u64,
}

// ============================================================================
// MODE MANAGER
// ============================================================================

/// Manages cognitive modes
pub struct ModeManager {
    /// Current mode
    current_mode: CognitiveMode,
    /// Mode entry time
    mode_since: Timestamp,
    /// Domain-specific modes
    domain_modes: BTreeMap<DomainId, CognitiveMode>,
    /// Transition rules
    rules: BTreeMap<u64, ModeTransitionRule>,
    /// Last transition per rule (for cooldown)
    last_transitions: BTreeMap<u64, Timestamp>,
    /// Transition history
    history: VecDeque<ModeTransition>,
    /// Next rule ID
    next_rule_id: AtomicU64,
    /// Next transition ID
    next_transition_id: AtomicU64,
    /// Configuration
    config: ModeConfig,
    /// Current load
    current_load: f64,
    /// Statistics
    stats: ModeStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ModeConfig {
    /// Maximum history
    pub max_history: usize,
    /// Default mode
    pub default_mode: CognitiveMode,
    /// Enable automatic transitions
    pub auto_transition: bool,
    /// Mode transition delay (ns)
    pub transition_delay_ns: u64,
}

impl Default for ModeConfig {
    fn default() -> Self {
        Self {
            max_history: 1000,
            default_mode: CognitiveMode::Normal,
            auto_transition: true,
            transition_delay_ns: 100_000_000, // 100ms
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ModeStats {
    /// Total transitions
    pub total_transitions: u64,
    /// Time per mode (ns)
    pub time_per_mode: BTreeMap<String, u64>,
    /// Transition counts per mode pair
    pub transition_counts: BTreeMap<String, u64>,
}

impl ModeManager {
    /// Create a new mode manager
    pub fn new(config: ModeConfig) -> Self {
        let now = Timestamp::now();
        Self {
            current_mode: config.default_mode,
            mode_since: now,
            domain_modes: BTreeMap::new(),
            rules: BTreeMap::new(),
            last_transitions: BTreeMap::new(),
            history: VecDeque::new(),
            next_rule_id: AtomicU64::new(1),
            next_transition_id: AtomicU64::new(1),
            config,
            current_load: 0.5,
            stats: ModeStats::default(),
        }
    }

    /// Get current mode
    #[inline(always)]
    pub fn current_mode(&self) -> CognitiveMode {
        self.current_mode
    }

    /// Get time in current mode
    #[inline(always)]
    pub fn mode_duration(&self) -> u64 {
        Timestamp::now().elapsed_since(self.mode_since)
    }

    /// Get mode characteristics
    #[inline(always)]
    pub fn current_characteristics(&self) -> ModeCharacteristics {
        self.current_mode.characteristics()
    }

    /// Set mode
    pub fn set_mode(&mut self, mode: CognitiveMode, reason: &str) -> ModeTransition {
        let now = Timestamp::now();
        let previous_duration = now.elapsed_since(self.mode_since);
        let from = self.current_mode;

        // Update time tracking
        let mode_key = format!("{:?}", from);
        *self.stats.time_per_mode.entry(mode_key).or_default() += previous_duration;

        // Create transition record
        let transition = ModeTransition {
            id: self.next_transition_id.fetch_add(1, Ordering::Relaxed),
            from,
            to: mode,
            timestamp: now,
            reason: reason.into(),
            rule_id: None,
            previous_duration_ns: previous_duration,
        };

        // Update history
        if self.history.len() >= self.config.max_history {
            self.history.pop_front();
        }
        self.history.push_back(transition.clone());

        // Update state
        self.current_mode = mode;
        self.mode_since = now;

        // Update stats
        self.stats.total_transitions += 1;
        let pair_key = format!("{:?}->{:?}", from, mode);
        *self.stats.transition_counts.entry(pair_key).or_default() += 1;

        transition
    }

    /// Add transition rule
    pub fn add_rule(&mut self, rule: ModeTransitionRule) -> u64 {
        let id = if rule.id == 0 {
            self.next_rule_id.fetch_add(1, Ordering::Relaxed)
        } else {
            rule.id
        };

        let mut rule = rule;
        rule.id = id;
        self.rules.insert(id, rule);

        id
    }

    /// Remove transition rule
    #[inline(always)]
    pub fn remove_rule(&mut self, rule_id: u64) -> bool {
        self.rules.remove(&rule_id).is_some()
    }

    /// Set rule enabled
    #[inline]
    pub fn set_rule_enabled(&mut self, rule_id: u64, enabled: bool) {
        if let Some(rule) = self.rules.get_mut(&rule_id) {
            rule.enabled = enabled;
        }
    }

    /// Update load
    #[inline(always)]
    pub fn update_load(&mut self, load: f64) {
        self.current_load = load.clamp(0.0, 1.0);
    }

    /// Evaluate rules and potentially transition
    pub fn evaluate(&mut self) -> Option<ModeTransition> {
        if !self.config.auto_transition {
            return None;
        }

        let now = Timestamp::now();
        let mode_duration = self.mode_duration();

        // Find applicable rules
        let mut applicable: Vec<_> = self.rules.values()
            .filter(|r| r.enabled && r.from == self.current_mode)
            .filter(|r| self.check_cooldown(r.id, now))
            .filter(|r| self.evaluate_condition(&r.condition, mode_duration))
            .collect();

        // Sort by priority
        applicable.sort_by_key(|r| core::cmp::Reverse(r.priority));

        // Apply first matching rule
        if let Some(rule) = applicable.first() {
            let mut transition = self.set_mode(rule.to, &rule.name);
            transition.rule_id = Some(rule.id);
            self.last_transitions.insert(rule.id, now);
            return Some(transition);
        }

        None
    }

    /// Check rule cooldown
    fn check_cooldown(&self, rule_id: u64, now: Timestamp) -> bool {
        if let Some(rule) = self.rules.get(&rule_id) {
            if let Some(last) = self.last_transitions.get(&rule_id) {
                return now.elapsed_since(*last) >= rule.cooldown_ns;
            }
        }
        true
    }

    /// Evaluate transition condition
    fn evaluate_condition(&self, condition: &TransitionCondition, mode_duration: u64) -> bool {
        match condition {
            TransitionCondition::Always => true,
            TransitionCondition::TimeBased(duration) => mode_duration >= *duration,
            TransitionCondition::LoadThreshold(threshold, op) => {
                match op {
                    ThresholdOp::GreaterThan => self.current_load > *threshold,
                    ThresholdOp::LessThan => self.current_load < *threshold,
                    ThresholdOp::Equal => (self.current_load - threshold).abs() < f64::EPSILON,
                }
            }
            TransitionCondition::Trigger(_) => false, // External triggers handled separately
            TransitionCondition::And(conditions) => {
                conditions.iter().all(|c| self.evaluate_condition(c, mode_duration))
            }
            TransitionCondition::Or(conditions) => {
                conditions.iter().any(|c| self.evaluate_condition(c, mode_duration))
            }
        }
    }

    /// Trigger external transition
    pub fn trigger(&mut self, trigger_name: &str) -> Option<ModeTransition> {
        let now = Timestamp::now();

        let matching_rule = self.rules.values()
            .filter(|r| r.enabled && r.from == self.current_mode)
            .find(|r| {
                if let TransitionCondition::Trigger(name) = &r.condition {
                    name == trigger_name && self.check_cooldown(r.id, now)
                } else {
                    false
                }
            });

        if let Some(rule) = matching_rule {
            let to = rule.to;
            let id = rule.id;
            let name = rule.name.clone();
            let mut transition = self.set_mode(to, &name);
            transition.rule_id = Some(id);
            self.last_transitions.insert(id, now);
            return Some(transition);
        }

        None
    }

    /// Get domain mode
    #[inline(always)]
    pub fn get_domain_mode(&self, domain: DomainId) -> CognitiveMode {
        self.domain_modes.get(&domain).copied().unwrap_or(self.current_mode)
    }

    /// Set domain mode
    #[inline(always)]
    pub fn set_domain_mode(&mut self, domain: DomainId, mode: CognitiveMode) {
        self.domain_modes.insert(domain, mode);
    }

    /// Clear domain mode (use global)
    #[inline(always)]
    pub fn clear_domain_mode(&mut self, domain: DomainId) {
        self.domain_modes.remove(&domain);
    }

    /// Get transition history
    #[inline(always)]
    pub fn history(&self) -> &[ModeTransition] {
        &self.history
    }

    /// Get recent transitions
    #[inline(always)]
    pub fn recent_transitions(&self, count: usize) -> Vec<&ModeTransition> {
        self.history.iter().rev().take(count).collect()
    }

    /// Get rules
    #[inline(always)]
    pub fn rules(&self) -> Vec<&ModeTransitionRule> {
        self.rules.values().collect()
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &ModeStats {
        &self.stats
    }
}

impl Default for ModeManager {
    fn default() -> Self {
        Self::new(ModeConfig::default())
    }
}

// ============================================================================
// MODE BUILDER
// ============================================================================

/// Builder for transition rules
pub struct RuleBuilder {
    rule: ModeTransitionRule,
}

impl RuleBuilder {
    /// Create a new builder
    pub fn new(name: &str, from: CognitiveMode, to: CognitiveMode) -> Self {
        Self {
            rule: ModeTransitionRule {
                id: 0,
                name: name.into(),
                from,
                to,
                condition: TransitionCondition::Always,
                priority: 100,
                cooldown_ns: 1_000_000_000, // 1 second
                enabled: true,
            },
        }
    }

    /// Set condition
    #[inline(always)]
    pub fn when(mut self, condition: TransitionCondition) -> Self {
        self.rule.condition = condition;
        self
    }

    /// Set time-based condition
    #[inline(always)]
    pub fn after_ns(mut self, duration: u64) -> Self {
        self.rule.condition = TransitionCondition::TimeBased(duration);
        self
    }

    /// Set load condition
    #[inline(always)]
    pub fn when_load_above(mut self, threshold: f64) -> Self {
        self.rule.condition = TransitionCondition::LoadThreshold(threshold, ThresholdOp::GreaterThan);
        self
    }

    /// Set load condition
    #[inline(always)]
    pub fn when_load_below(mut self, threshold: f64) -> Self {
        self.rule.condition = TransitionCondition::LoadThreshold(threshold, ThresholdOp::LessThan);
        self
    }

    /// Set trigger condition
    #[inline(always)]
    pub fn on_trigger(mut self, trigger: &str) -> Self {
        self.rule.condition = TransitionCondition::Trigger(trigger.into());
        self
    }

    /// Set priority
    #[inline(always)]
    pub fn priority(mut self, priority: u32) -> Self {
        self.rule.priority = priority;
        self
    }

    /// Set cooldown
    #[inline(always)]
    pub fn cooldown_ns(mut self, cooldown: u64) -> Self {
        self.rule.cooldown_ns = cooldown;
        self
    }

    /// Build the rule
    #[inline(always)]
    pub fn build(self) -> ModeTransitionRule {
        self.rule
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_transition() {
        let mut manager = ModeManager::default();

        assert_eq!(manager.current_mode(), CognitiveMode::Normal);

        let transition = manager.set_mode(CognitiveMode::Focused, "Test");
        assert_eq!(transition.from, CognitiveMode::Normal);
        assert_eq!(transition.to, CognitiveMode::Focused);

        assert_eq!(manager.current_mode(), CognitiveMode::Focused);
    }

    #[test]
    fn test_rule_based_transition() {
        let config = ModeConfig {
            auto_transition: true,
            ..Default::default()
        };
        let mut manager = ModeManager::new(config);

        // Add rule: switch to Emergency when load > 0.9
        let rule = RuleBuilder::new("high_load", CognitiveMode::Normal, CognitiveMode::Emergency)
            .when_load_above(0.9)
            .build();

        manager.add_rule(rule);

        // Update load below threshold
        manager.update_load(0.5);
        assert!(manager.evaluate().is_none());

        // Update load above threshold
        manager.update_load(0.95);
        let transition = manager.evaluate();
        assert!(transition.is_some());
        assert_eq!(manager.current_mode(), CognitiveMode::Emergency);
    }

    #[test]
    fn test_trigger_transition() {
        let mut manager = ModeManager::default();

        let rule = RuleBuilder::new("maintenance_trigger", CognitiveMode::Normal, CognitiveMode::Maintenance)
            .on_trigger("start_maintenance")
            .build();

        manager.add_rule(rule);

        // Trigger transition
        let transition = manager.trigger("start_maintenance");
        assert!(transition.is_some());
        assert_eq!(manager.current_mode(), CognitiveMode::Maintenance);

        // Wrong trigger does nothing
        let transition = manager.trigger("wrong_trigger");
        assert!(transition.is_none());
    }

    #[test]
    fn test_mode_characteristics() {
        let focused = CognitiveMode::Focused.characteristics();
        let reactive = CognitiveMode::Reactive.characteristics();

        // Focused should have higher processing depth
        assert!(focused.processing_depth > reactive.processing_depth);

        // Reactive should have higher response latency priority
        assert!(reactive.response_latency > focused.response_latency);
    }

    #[test]
    fn test_domain_modes() {
        let mut manager = ModeManager::default();
        let domain1 = DomainId::new(1);
        let domain2 = DomainId::new(2);

        // By default, domains use global mode
        assert_eq!(manager.get_domain_mode(domain1), CognitiveMode::Normal);

        // Set domain-specific mode
        manager.set_domain_mode(domain1, CognitiveMode::Learning);
        assert_eq!(manager.get_domain_mode(domain1), CognitiveMode::Learning);
        assert_eq!(manager.get_domain_mode(domain2), CognitiveMode::Normal);

        // Change global mode
        manager.set_mode(CognitiveMode::Focused, "Test");
        assert_eq!(manager.get_domain_mode(domain1), CognitiveMode::Learning);
        assert_eq!(manager.get_domain_mode(domain2), CognitiveMode::Focused);
    }
}
