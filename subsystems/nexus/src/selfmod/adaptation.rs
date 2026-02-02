//! # Runtime Adaptation
//!
//! Year 3 EVOLUTION - Dynamic adaptation of self-modifying code

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============================================================================
// ADAPTATION TYPES
// ============================================================================

/// Adaptation ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdaptationId(pub u64);

static ADAPTATION_COUNTER: AtomicU64 = AtomicU64::new(1);

impl AdaptationId {
    pub fn generate() -> Self {
        Self(ADAPTATION_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Adaptation rule
#[derive(Debug, Clone)]
pub struct AdaptationRule {
    /// ID
    pub id: AdaptationId,
    /// Name
    pub name: String,
    /// Trigger condition
    pub trigger: TriggerCondition,
    /// Action to take
    pub action: AdaptationAction,
    /// Priority
    pub priority: i32,
    /// Enabled
    pub enabled: bool,
    /// Cooldown (ticks)
    pub cooldown: u64,
    /// Last triggered
    pub last_triggered: u64,
}

/// Trigger condition
#[derive(Debug, Clone)]
pub enum TriggerCondition {
    /// Metric exceeds threshold
    MetricAbove {
        metric: String,
        threshold: f64,
    },
    /// Metric below threshold
    MetricBelow {
        metric: String,
        threshold: f64,
    },
    /// Metric in range
    MetricInRange {
        metric: String,
        min: f64,
        max: f64,
    },
    /// Event occurs
    Event {
        event_type: String,
    },
    /// Time-based
    Periodic {
        interval: u64,
    },
    /// Trend detection
    Trend {
        metric: String,
        direction: TrendDirection,
        window: usize,
    },
    /// Anomaly detection
    Anomaly {
        metric: String,
        sensitivity: f64,
    },
    /// Combination
    And(Vec<TriggerCondition>),
    Or(Vec<TriggerCondition>),
    Not(Box<TriggerCondition>),
}

/// Trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// Adaptation action
#[derive(Debug, Clone)]
pub enum AdaptationAction {
    /// Adjust parameter
    AdjustParameter { name: String, delta: f64 },
    /// Set parameter
    SetParameter { name: String, value: f64 },
    /// Scale resource
    Scale { resource: String, factor: f64 },
    /// Enable feature
    EnableFeature { feature: String },
    /// Disable feature
    DisableFeature { feature: String },
    /// Switch strategy
    SwitchStrategy { from: String, to: String },
    /// Execute code modification
    ModifyCode { modification_id: u64 },
    /// Trigger recompilation
    Recompile { module: String },
    /// Multiple actions
    Sequence(Vec<AdaptationAction>),
    /// Conditional action
    Conditional {
        condition: Box<TriggerCondition>,
        then: Box<AdaptationAction>,
        else_action: Option<Box<AdaptationAction>>,
    },
}

// ============================================================================
// METRICS
// ============================================================================

/// Metric sample
#[derive(Debug, Clone, Copy)]
pub struct MetricSample {
    /// Value
    pub value: f64,
    /// Timestamp
    pub timestamp: u64,
}

/// Metric buffer with history
pub struct MetricBuffer {
    /// Samples
    samples: Vec<MetricSample>,
    /// Capacity
    capacity: usize,
    /// Position
    position: usize,
    /// Full flag
    full: bool,
}

impl MetricBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
            capacity,
            position: 0,
            full: false,
        }
    }

    /// Add sample
    pub fn push(&mut self, sample: MetricSample) {
        if self.samples.len() < self.capacity {
            self.samples.push(sample);
        } else {
            self.samples[self.position] = sample;
        }
        self.position = (self.position + 1) % self.capacity;
        if self.position == 0 {
            self.full = true;
        }
    }

    /// Get latest value
    pub fn latest(&self) -> Option<f64> {
        if self.samples.is_empty() {
            None
        } else if self.position == 0 && !self.full {
            None
        } else {
            let idx = if self.position == 0 {
                self.capacity - 1
            } else {
                self.position - 1
            };
            Some(self.samples[idx].value)
        }
    }

    /// Get mean
    pub fn mean(&self) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.samples.iter().map(|s| s.value).sum();
        sum / self.samples.len() as f64
    }

    /// Get standard deviation
    pub fn std_dev(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }
        let mean = self.mean();
        let variance: f64 = self
            .samples
            .iter()
            .map(|s| (s.value - mean).powi(2))
            .sum::<f64>()
            / (self.samples.len() - 1) as f64;
        variance.sqrt()
    }

    /// Get min
    pub fn min(&self) -> Option<f64> {
        self.samples
            .iter()
            .map(|s| s.value)
            .fold(None, |min, v| Some(min.map_or(v, |m: f64| m.min(v))))
    }

    /// Get max
    pub fn max(&self) -> Option<f64> {
        self.samples
            .iter()
            .map(|s| s.value)
            .fold(None, |max, v| Some(max.map_or(v, |m: f64| m.max(v))))
    }

    /// Detect trend
    pub fn detect_trend(&self, window: usize) -> TrendDirection {
        let n = self.samples.len().min(window);
        if n < 3 {
            return TrendDirection::Stable;
        }

        // Get last n samples in order
        let mut recent: Vec<f64> = Vec::with_capacity(n);
        for i in 0..n {
            let idx = (self.position + self.capacity - n + i) % self.capacity;
            if idx < self.samples.len() {
                recent.push(self.samples[idx].value);
            }
        }

        if recent.len() < 3 {
            return TrendDirection::Stable;
        }

        // Linear regression
        let n_f = recent.len() as f64;
        let sum_x: f64 = (0..recent.len()).map(|i| i as f64).sum();
        let sum_y: f64 = recent.iter().sum();
        let sum_xy: f64 = recent.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let sum_x2: f64 = (0..recent.len()).map(|i| (i * i) as f64).sum();

        let slope = (n_f * sum_xy - sum_x * sum_y) / (n_f * sum_x2 - sum_x * sum_x);

        // Threshold based on standard deviation
        let threshold = self.std_dev() * 0.1;

        if slope > threshold {
            TrendDirection::Increasing
        } else if slope < -threshold {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        }
    }

    /// Detect anomaly (z-score based)
    pub fn is_anomaly(&self, value: f64, sensitivity: f64) -> bool {
        let mean = self.mean();
        let std = self.std_dev();

        if std < 1e-10 {
            return false;
        }

        let z_score = (value - mean).abs() / std;
        z_score > sensitivity
    }
}

// ============================================================================
// ADAPTATION ENGINE
// ============================================================================

/// Adaptation engine
pub struct AdaptationEngine {
    /// Rules
    rules: Vec<AdaptationRule>,
    /// Metrics
    metrics: BTreeMap<String, MetricBuffer>,
    /// Parameters
    parameters: BTreeMap<String, f64>,
    /// Features
    features: BTreeMap<String, bool>,
    /// Current tick
    tick: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
    /// Event queue
    events: Vec<(String, u64)>,
    /// History of adaptations
    history: Vec<AdaptationRecord>,
}

/// Adaptation record
#[derive(Debug, Clone)]
pub struct AdaptationRecord {
    /// Rule ID
    pub rule_id: AdaptationId,
    /// Timestamp
    pub timestamp: u64,
    /// Action taken
    pub action: String,
    /// Result
    pub success: bool,
}

impl AdaptationEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            metrics: BTreeMap::new(),
            parameters: BTreeMap::new(),
            features: BTreeMap::new(),
            tick: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
            events: Vec::new(),
            history: Vec::new(),
        }
    }

    /// Add rule
    pub fn add_rule(&mut self, rule: AdaptationRule) {
        self.rules.push(rule);
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Remove rule
    pub fn remove_rule(&mut self, id: AdaptationId) {
        self.rules.retain(|r| r.id != id);
    }

    /// Register metric
    pub fn register_metric(&mut self, name: &str, buffer_size: usize) {
        self.metrics
            .insert(String::from(name), MetricBuffer::new(buffer_size));
    }

    /// Update metric
    pub fn update_metric(&mut self, name: &str, value: f64) {
        let timestamp = self.tick.load(Ordering::Relaxed);
        if let Some(buffer) = self.metrics.get_mut(name) {
            buffer.push(MetricSample { value, timestamp });
        }
    }

    /// Set parameter
    pub fn set_parameter(&mut self, name: &str, value: f64) {
        self.parameters.insert(String::from(name), value);
    }

    /// Get parameter
    pub fn get_parameter(&self, name: &str) -> Option<f64> {
        self.parameters.get(name).copied()
    }

    /// Set feature
    pub fn set_feature(&mut self, name: &str, enabled: bool) {
        self.features.insert(String::from(name), enabled);
    }

    /// Push event
    pub fn push_event(&mut self, event_type: &str) {
        let timestamp = self.tick.load(Ordering::Relaxed);
        self.events.push((String::from(event_type), timestamp));

        // Keep limited history
        if self.events.len() > 1000 {
            self.events.drain(0..500);
        }
    }

    /// Process one tick
    pub fn tick(&mut self) -> Vec<AdaptationRecord> {
        let current_tick = self.tick.fetch_add(1, Ordering::SeqCst);

        if !self.enabled.load(Ordering::Relaxed) {
            return Vec::new();
        }

        let mut triggered = Vec::new();

        for rule in &mut self.rules {
            if !rule.enabled {
                continue;
            }

            // Check cooldown
            if current_tick < rule.last_triggered + rule.cooldown {
                continue;
            }

            if self.check_condition(&rule.trigger, current_tick) {
                rule.last_triggered = current_tick;
                triggered.push((rule.id, rule.action.clone()));
            }
        }

        // Execute actions
        let mut records = Vec::new();
        for (rule_id, action) in triggered {
            let success = self.execute_action(&action);
            let record = AdaptationRecord {
                rule_id,
                timestamp: current_tick,
                action: alloc::format!("{:?}", action),
                success,
            };
            records.push(record.clone());
            self.history.push(record);
        }

        // Trim history
        if self.history.len() > 10000 {
            self.history.drain(0..5000);
        }

        records
    }

    fn check_condition(&self, condition: &TriggerCondition, current_tick: u64) -> bool {
        match condition {
            TriggerCondition::MetricAbove { metric, threshold } => self
                .metrics
                .get(metric)
                .and_then(|b| b.latest())
                .map(|v| v > *threshold)
                .unwrap_or(false),
            TriggerCondition::MetricBelow { metric, threshold } => self
                .metrics
                .get(metric)
                .and_then(|b| b.latest())
                .map(|v| v < *threshold)
                .unwrap_or(false),
            TriggerCondition::MetricInRange { metric, min, max } => self
                .metrics
                .get(metric)
                .and_then(|b| b.latest())
                .map(|v| v >= *min && v <= *max)
                .unwrap_or(false),
            TriggerCondition::Event { event_type } => {
                self.events.iter().any(|(e, _)| e == event_type)
            },
            TriggerCondition::Periodic { interval } => current_tick % *interval == 0,
            TriggerCondition::Trend {
                metric,
                direction,
                window,
            } => self
                .metrics
                .get(metric)
                .map(|b| b.detect_trend(*window) == *direction)
                .unwrap_or(false),
            TriggerCondition::Anomaly {
                metric,
                sensitivity,
            } => self
                .metrics
                .get(metric)
                .and_then(|b| b.latest().map(|v| b.is_anomaly(v, *sensitivity)))
                .unwrap_or(false),
            TriggerCondition::And(conditions) => conditions
                .iter()
                .all(|c| self.check_condition(c, current_tick)),
            TriggerCondition::Or(conditions) => conditions
                .iter()
                .any(|c| self.check_condition(c, current_tick)),
            TriggerCondition::Not(condition) => !self.check_condition(condition, current_tick),
        }
    }

    fn execute_action(&mut self, action: &AdaptationAction) -> bool {
        match action {
            AdaptationAction::AdjustParameter { name, delta } => {
                if let Some(current) = self.parameters.get_mut(name) {
                    *current += delta;
                    true
                } else {
                    false
                }
            },
            AdaptationAction::SetParameter { name, value } => {
                self.parameters.insert(name.clone(), *value);
                true
            },
            AdaptationAction::Scale { resource, factor } => {
                if let Some(current) = self.parameters.get_mut(resource) {
                    *current *= factor;
                    true
                } else {
                    false
                }
            },
            AdaptationAction::EnableFeature { feature } => {
                self.features.insert(feature.clone(), true);
                true
            },
            AdaptationAction::DisableFeature { feature } => {
                self.features.insert(feature.clone(), false);
                true
            },
            AdaptationAction::SwitchStrategy { from: _, to } => {
                // Would trigger actual strategy switch
                self.push_event(&alloc::format!("strategy_switched:{}", to));
                true
            },
            AdaptationAction::ModifyCode { modification_id: _ } => {
                // Would trigger code modification
                self.push_event("code_modified");
                true
            },
            AdaptationAction::Recompile { module } => {
                // Would trigger recompilation
                self.push_event(&alloc::format!("recompile:{}", module));
                true
            },
            AdaptationAction::Sequence(actions) => actions.iter().all(|a| self.execute_action(a)),
            AdaptationAction::Conditional {
                condition,
                then,
                else_action,
            } => {
                let tick = self.tick.load(Ordering::Relaxed);
                if self.check_condition(condition, tick) {
                    self.execute_action(then)
                } else if let Some(else_act) = else_action {
                    self.execute_action(else_act)
                } else {
                    true
                }
            },
        }
    }

    /// Get adaptation history
    pub fn history(&self) -> &[AdaptationRecord] {
        &self.history
    }

    /// Enable/disable
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

impl Default for AdaptationEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ADAPTIVE PARAMETERS
// ============================================================================

/// Self-tuning parameter
pub struct AdaptiveParameter {
    /// Current value
    value: f64,
    /// Minimum value
    min: f64,
    /// Maximum value
    max: f64,
    /// Step size
    step: f64,
    /// Performance history
    performance_history: Vec<(f64, f64)>, // (value, performance)
    /// Best value found
    best_value: f64,
    /// Best performance
    best_performance: f64,
}

impl AdaptiveParameter {
    pub fn new(initial: f64, min: f64, max: f64, step: f64) -> Self {
        Self {
            value: initial,
            min,
            max,
            step,
            performance_history: Vec::new(),
            best_value: initial,
            best_performance: f64::NEG_INFINITY,
        }
    }

    /// Get current value
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Record performance
    pub fn record(&mut self, performance: f64) {
        self.performance_history.push((self.value, performance));

        if performance > self.best_performance {
            self.best_performance = performance;
            self.best_value = self.value;
        }

        // Trim history
        if self.performance_history.len() > 100 {
            self.performance_history.drain(0..50);
        }
    }

    /// Adjust using gradient estimation
    pub fn adjust(&mut self) {
        if self.performance_history.len() < 3 {
            return;
        }

        // Estimate gradient
        let recent: Vec<_> = self.performance_history.iter().rev().take(10).collect();

        if recent.len() < 2 {
            return;
        }

        let mut gradient = 0.0;
        for i in 1..recent.len() {
            let dv = recent[i - 1].0 - recent[i].0;
            let dp = recent[i - 1].1 - recent[i].1;
            if dv.abs() > 1e-10 {
                gradient += dp / dv;
            }
        }
        gradient /= (recent.len() - 1) as f64;

        // Move in gradient direction
        self.value += self.step * gradient.signum();
        self.value = self.value.clamp(self.min, self.max);
    }

    /// Reset to best known value
    pub fn reset_to_best(&mut self) {
        self.value = self.best_value;
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_buffer() {
        let mut buffer = MetricBuffer::new(10);

        for i in 0..5 {
            buffer.push(MetricSample {
                value: i as f64,
                timestamp: i,
            });
        }

        assert!((buffer.mean() - 2.0).abs() < 0.01);
        assert_eq!(buffer.latest(), Some(4.0));
    }

    #[test]
    fn test_trend_detection() {
        let mut buffer = MetricBuffer::new(20);

        // Increasing trend
        for i in 0..10 {
            buffer.push(MetricSample {
                value: i as f64,
                timestamp: i,
            });
        }

        assert_eq!(buffer.detect_trend(10), TrendDirection::Increasing);
    }

    #[test]
    fn test_adaptation_engine() {
        let mut engine = AdaptationEngine::new();

        engine.register_metric("cpu", 100);
        engine.set_parameter("threads", 4.0);

        let rule = AdaptationRule {
            id: AdaptationId::generate(),
            name: String::from("scale_up"),
            trigger: TriggerCondition::MetricAbove {
                metric: String::from("cpu"),
                threshold: 80.0,
            },
            action: AdaptationAction::AdjustParameter {
                name: String::from("threads"),
                delta: 2.0,
            },
            priority: 1,
            enabled: true,
            cooldown: 10,
            last_triggered: 0,
        };

        engine.add_rule(rule);
        engine.update_metric("cpu", 90.0);

        let records = engine.tick();

        assert_eq!(records.len(), 1);
        assert_eq!(engine.get_parameter("threads"), Some(6.0));
    }

    #[test]
    fn test_adaptive_parameter() {
        let mut param = AdaptiveParameter::new(0.5, 0.0, 1.0, 0.1);

        // Record increasing performance as value increases
        param.record(0.5);
        param.value = 0.6;
        param.record(0.7);
        param.value = 0.7;
        param.record(0.9);

        param.adjust();

        // Should move towards higher values
        assert!(param.value() >= 0.7);
    }
}
