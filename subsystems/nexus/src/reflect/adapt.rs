//! # Adaptation Engine
//!
//! Implements adaptive behavior and self-modification.
//! Supports strategy adaptation based on feedback.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// ADAPTATION TYPES
// ============================================================================

/// Strategy
#[derive(Debug, Clone)]
pub struct Strategy {
    /// Strategy ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Domain
    pub domain: String,
    /// Parameters
    pub parameters: BTreeMap<String, f64>,
    /// Effectiveness
    pub effectiveness: f64,
    /// Uses
    pub uses: u64,
    /// Created
    pub created: Timestamp,
    /// Last used
    pub last_used: Timestamp,
}

/// Feedback
#[derive(Debug, Clone)]
pub struct Feedback {
    /// Feedback ID
    pub id: u64,
    /// Strategy
    pub strategy_id: u64,
    /// Outcome
    pub outcome: Outcome,
    /// Score
    pub score: f64,
    /// Context
    pub context: BTreeMap<String, String>,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    PartialSuccess,
    Failure,
    Timeout,
    Unknown,
}

/// Adaptation rule
#[derive(Debug, Clone)]
pub struct AdaptationRule {
    /// Rule ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Condition
    pub condition: AdaptCondition,
    /// Action
    pub action: AdaptAction,
    /// Priority
    pub priority: u32,
}

/// Adaptation condition
#[derive(Debug, Clone)]
pub enum AdaptCondition {
    EffectivenessBelowThreshold { threshold: f64 },
    FailureStreak { count: u32 },
    ContextMatch { key: String, value: String },
    TimeElapsed { since_last_use_ms: u64 },
    ParameterOutOfRange { param: String, min: f64, max: f64 },
    And(Vec<AdaptCondition>),
    Or(Vec<AdaptCondition>),
}

/// Adaptation action
#[derive(Debug, Clone)]
pub enum AdaptAction {
    AdjustParameter { param: String, delta: f64 },
    ScaleParameter { param: String, factor: f64 },
    ResetParameter { param: String, value: f64 },
    SwitchStrategy { to: u64 },
    DisableStrategy,
    EnableStrategy,
    Explore { exploration_rate: f64 },
}

/// Adaptation result
#[derive(Debug, Clone)]
pub struct AdaptationResult {
    /// Strategy affected
    pub strategy_id: u64,
    /// Rules applied
    pub rules_applied: Vec<u64>,
    /// Parameters changed
    pub changes: Vec<ParameterChange>,
    /// New effectiveness
    pub new_effectiveness: f64,
}

/// Parameter change
#[derive(Debug, Clone)]
pub struct ParameterChange {
    /// Parameter name
    pub parameter: String,
    /// Old value
    pub old_value: f64,
    /// New value
    pub new_value: f64,
}

// ============================================================================
// ADAPTATION ENGINE
// ============================================================================

/// Adaptation engine
pub struct AdaptationEngine {
    /// Strategies
    strategies: BTreeMap<u64, Strategy>,
    /// Feedback history
    feedback: VecDeque<Feedback>,
    /// Rules
    rules: BTreeMap<u64, AdaptationRule>,
    /// Failure counters
    failure_counters: BTreeMap<u64, u32>,
    /// Active strategy per domain
    active_strategy: BTreeMap<String, u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: AdaptationConfig,
    /// Statistics
    stats: AdaptationStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct AdaptationConfig {
    /// Learning rate
    pub learning_rate: f64,
    /// Exploration rate
    pub exploration_rate: f64,
    /// Effectiveness decay
    pub effectiveness_decay: f64,
    /// Maximum feedback history
    pub max_history: usize,
}

impl Default for AdaptationConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            exploration_rate: 0.1,
            effectiveness_decay: 0.99,
            max_history: 1000,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AdaptationStats {
    /// Strategies created
    pub strategies_created: u64,
    /// Feedback received
    pub feedback_received: u64,
    /// Adaptations made
    pub adaptations_made: u64,
}

impl AdaptationEngine {
    /// Create new engine
    pub fn new(config: AdaptationConfig) -> Self {
        Self {
            strategies: BTreeMap::new(),
            feedback: VecDeque::new(),
            rules: BTreeMap::new(),
            failure_counters: BTreeMap::new(),
            active_strategy: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: AdaptationStats::default(),
        }
    }

    /// Create strategy
    pub fn create_strategy(&mut self, name: &str, domain: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let strategy = Strategy {
            id,
            name: name.into(),
            domain: domain.into(),
            parameters: BTreeMap::new(),
            effectiveness: 0.5,
            uses: 0,
            created: now,
            last_used: now,
        };

        self.strategies.insert(id, strategy);
        self.stats.strategies_created += 1;

        // Set as active if first in domain
        if !self.active_strategy.contains_key(domain) {
            self.active_strategy.insert(domain.into(), id);
        }

        id
    }

    /// Set parameter
    #[inline]
    pub fn set_parameter(&mut self, strategy_id: u64, param: &str, value: f64) {
        if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
            strategy.parameters.insert(param.into(), value);
        }
    }

    /// Get active strategy
    #[inline(always)]
    pub fn active(&self, domain: &str) -> Option<&Strategy> {
        let id = self.active_strategy.get(domain)?;
        self.strategies.get(id)
    }

    /// Use strategy
    #[inline]
    pub fn use_strategy(&mut self, strategy_id: u64) {
        if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
            strategy.uses += 1;
            strategy.last_used = Timestamp::now();
        }
    }

    /// Add rule
    pub fn add_rule(
        &mut self,
        name: &str,
        condition: AdaptCondition,
        action: AdaptAction,
        priority: u32,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let rule = AdaptationRule {
            id,
            name: name.into(),
            condition,
            action,
            priority,
        };

        self.rules.insert(id, rule);

        id
    }

    /// Receive feedback
    pub fn receive_feedback(
        &mut self,
        strategy_id: u64,
        outcome: Outcome,
        score: f64,
        context: BTreeMap<String, String>,
    ) -> Option<AdaptationResult> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let feedback = Feedback {
            id,
            strategy_id,
            outcome,
            score: score.clamp(0.0, 1.0),
            context,
            timestamp: Timestamp::now(),
        };

        self.feedback.push_back(feedback);
        self.stats.feedback_received += 1;

        // Limit history
        while self.feedback.len() > self.config.max_history {
            self.feedback.pop_front();
        }

        // Update failure counter
        match outcome {
            Outcome::Failure | Outcome::Timeout => {
                *self.failure_counters.entry(strategy_id).or_insert(0) += 1;
            }
            Outcome::Success | Outcome::PartialSuccess => {
                self.failure_counters.insert(strategy_id, 0);
            }
            _ => {}
        }

        // Update effectiveness
        self.update_effectiveness(strategy_id, score);

        // Apply adaptation rules
        self.adapt(strategy_id)
    }

    fn update_effectiveness(&mut self, strategy_id: u64, score: f64) {
        if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
            // Exponential moving average
            let lr = self.config.learning_rate;
            strategy.effectiveness = strategy.effectiveness * (1.0 - lr) + score * lr;
        }
    }

    /// Adapt strategy
    pub fn adapt(&mut self, strategy_id: u64) -> Option<AdaptationResult> {
        let strategy = self.strategies.get(&strategy_id)?.clone();

        let mut rules_applied = Vec::new();
        let mut changes = Vec::new();

        // Get sorted rules by priority
        let mut rules: Vec<_> = self.rules.values().cloned().collect();
        rules.sort_by_key(|r| r.priority);

        for rule in &rules {
            if self.check_condition(&rule.condition, &strategy) {
                if let Some(change) = self.apply_action(&rule.action, strategy_id) {
                    rules_applied.push(rule.id);
                    changes.push(change);
                }
            }
        }

        if rules_applied.is_empty() {
            return None;
        }

        let new_effectiveness = self.strategies.get(&strategy_id)
            .map(|s| s.effectiveness)
            .unwrap_or(0.0);

        self.stats.adaptations_made += 1;

        Some(AdaptationResult {
            strategy_id,
            rules_applied,
            changes,
            new_effectiveness,
        })
    }

    fn check_condition(&self, condition: &AdaptCondition, strategy: &Strategy) -> bool {
        match condition {
            AdaptCondition::EffectivenessBelowThreshold { threshold } => {
                strategy.effectiveness < *threshold
            }
            AdaptCondition::FailureStreak { count } => {
                self.failure_counters.get(&strategy.id)
                    .map_or(false, |&c| c >= *count)
            }
            AdaptCondition::ContextMatch { key, value } => {
                // Check recent feedback context
                self.feedback.iter().rev()
                    .take(5)
                    .filter(|f| f.strategy_id == strategy.id)
                    .any(|f| f.context.get(key).map_or(false, |v| v == value))
            }
            AdaptCondition::TimeElapsed { since_last_use_ms } => {
                let elapsed = Timestamp::now().0 - strategy.last_used.0;
                elapsed >= *since_last_use_ms
            }
            AdaptCondition::ParameterOutOfRange { param, min, max } => {
                strategy.parameters.get(param)
                    .map_or(false, |&v| v < *min || v > *max)
            }
            AdaptCondition::And(conditions) => {
                conditions.iter().all(|c| self.check_condition(c, strategy))
            }
            AdaptCondition::Or(conditions) => {
                conditions.iter().any(|c| self.check_condition(c, strategy))
            }
        }
    }

    fn apply_action(&mut self, action: &AdaptAction, strategy_id: u64) -> Option<ParameterChange> {
        match action {
            AdaptAction::AdjustParameter { param, delta } => {
                if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
                    let old_value = strategy.parameters.get(param).copied().unwrap_or(0.0);
                    let new_value = old_value + delta;
                    strategy.parameters.insert(param.clone(), new_value);

                    return Some(ParameterChange {
                        parameter: param.clone(),
                        old_value,
                        new_value,
                    });
                }
            }
            AdaptAction::ScaleParameter { param, factor } => {
                if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
                    let old_value = strategy.parameters.get(param).copied().unwrap_or(1.0);
                    let new_value = old_value * factor;
                    strategy.parameters.insert(param.clone(), new_value);

                    return Some(ParameterChange {
                        parameter: param.clone(),
                        old_value,
                        new_value,
                    });
                }
            }
            AdaptAction::ResetParameter { param, value } => {
                if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
                    let old_value = strategy.parameters.get(param).copied().unwrap_or(0.0);
                    strategy.parameters.insert(param.clone(), *value);

                    return Some(ParameterChange {
                        parameter: param.clone(),
                        old_value,
                        new_value: *value,
                    });
                }
            }
            AdaptAction::SwitchStrategy { to } => {
                if let Some(strategy) = self.strategies.get(&strategy_id) {
                    let domain = strategy.domain.clone();
                    self.active_strategy.insert(domain, *to);
                }
            }
            AdaptAction::DisableStrategy => {
                if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
                    strategy.effectiveness = 0.0;
                }
            }
            AdaptAction::EnableStrategy => {
                if let Some(strategy) = self.strategies.get_mut(&strategy_id) {
                    if strategy.effectiveness == 0.0 {
                        strategy.effectiveness = 0.5;
                    }
                }
            }
            AdaptAction::Explore { exploration_rate: _ } => {
                // Would add exploration to selection
            }
        }

        None
    }

    /// Select best strategy for domain
    pub fn select(&self, domain: &str) -> Option<&Strategy> {
        let strategies: Vec<_> = self.strategies.values()
            .filter(|s| s.domain == domain)
            .collect();

        // Simple epsilon-greedy
        let explore = false; // Would use random

        if explore {
            strategies.first().copied()
        } else {
            strategies.iter()
                .max_by(|a, b| a.effectiveness.partial_cmp(&b.effectiveness)
                    .unwrap_or(core::cmp::Ordering::Equal))
                .copied()
        }
    }

    /// Get strategy
    #[inline(always)]
    pub fn get_strategy(&self, id: u64) -> Option<&Strategy> {
        self.strategies.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &AdaptationStats {
        &self.stats
    }
}

impl Default for AdaptationEngine {
    fn default() -> Self {
        Self::new(AdaptationConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_strategy() {
        let mut engine = AdaptationEngine::default();

        let id = engine.create_strategy("greedy", "search");
        assert!(engine.get_strategy(id).is_some());
    }

    #[test]
    fn test_set_parameter() {
        let mut engine = AdaptationEngine::default();

        let id = engine.create_strategy("test", "domain");
        engine.set_parameter(id, "threshold", 0.5);

        let strategy = engine.get_strategy(id).unwrap();
        assert_eq!(strategy.parameters.get("threshold"), Some(&0.5));
    }

    #[test]
    fn test_receive_feedback() {
        let mut engine = AdaptationEngine::default();

        let id = engine.create_strategy("test", "domain");

        engine.receive_feedback(id, Outcome::Success, 0.8, BTreeMap::new());

        let strategy = engine.get_strategy(id).unwrap();
        assert!(strategy.effectiveness > 0.5);
    }

    #[test]
    fn test_effectiveness_below_threshold() {
        let mut engine = AdaptationEngine::default();

        let id = engine.create_strategy("test", "domain");
        engine.set_parameter(id, "rate", 0.1);

        // Lower effectiveness
        for _ in 0..5 {
            engine.receive_feedback(id, Outcome::Failure, 0.2, BTreeMap::new());
        }

        engine.add_rule(
            "increase_rate",
            AdaptCondition::EffectivenessBelowThreshold { threshold: 0.4 },
            AdaptAction::ScaleParameter { param: "rate".into(), factor: 2.0 },
            1,
        );

        let result = engine.adapt(id);
        assert!(result.is_some());
    }

    #[test]
    fn test_active_strategy() {
        let mut engine = AdaptationEngine::default();

        engine.create_strategy("first", "domain");

        let active = engine.active("domain");
        assert!(active.is_some());
    }

    #[test]
    fn test_select_best() {
        let mut engine = AdaptationEngine::default();

        let s1 = engine.create_strategy("low", "domain");
        let s2 = engine.create_strategy("high", "domain");

        // Make s2 better
        for _ in 0..10 {
            engine.receive_feedback(s2, Outcome::Success, 0.9, BTreeMap::new());
        }

        let best = engine.select("domain").unwrap();
        assert_eq!(best.id, s2);
    }
}
