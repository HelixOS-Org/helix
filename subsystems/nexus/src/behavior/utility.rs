//! NEXUS Year 2: Utility-Based AI
//!
//! Utility-based decision making for kernel AI. Uses utility curves,
//! considerations, and reasoner architecture for flexible decisions.

#![allow(dead_code)]

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    vec::Vec,
};

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for considerations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConsiderationId(pub u64);

impl ConsiderationId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionId(pub u64);

impl ActionId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Input context for utility calculations
pub struct UtilityContext {
    pub inputs: BTreeMap<String, f64>,
    pub time: u64,
}

impl UtilityContext {
    pub fn new() -> Self {
        Self {
            inputs: BTreeMap::new(),
            time: 0,
        }
    }

    pub fn with_time(mut self, time: u64) -> Self {
        self.time = time;
        self
    }

    pub fn set(&mut self, key: impl Into<String>, value: f64) {
        self.inputs.insert(key.into(), value);
    }

    pub fn get(&self, key: &str) -> Option<f64> {
        self.inputs.get(key).copied()
    }

    pub fn get_or(&self, key: &str, default: f64) -> f64 {
        self.inputs.get(key).copied().unwrap_or(default)
    }
}

impl Default for UtilityContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Utility Curves
// ============================================================================

/// Response curve for mapping input values to utility
#[derive(Debug, Clone)]
pub enum UtilityCurve {
    /// Linear: y = mx + b, clamped to [0,1]
    Linear { slope: f64, intercept: f64 },
    /// Quadratic: y = a*x^2 + b*x + c
    Quadratic { a: f64, b: f64, c: f64 },
    /// Exponential: y = a * e^(k*x)
    Exponential { a: f64, k: f64 },
    /// Logistic (S-curve): y = 1 / (1 + e^(-k*(x-x0)))
    Logistic { k: f64, x0: f64 },
    /// Inverse logistic: 1 - logistic
    InverseLogistic { k: f64, x0: f64 },
    /// Step function
    Step { threshold: f64 },
    /// Smooth step (Hermite)
    SmoothStep { edge0: f64, edge1: f64 },
    /// Constant value
    Constant(f64),
    /// Custom lookup table with linear interpolation
    LookupTable(Vec<(f64, f64)>),
}

impl UtilityCurve {
    pub fn evaluate(&self, x: f64) -> f64 {
        let result = match self {
            Self::Linear { slope, intercept } => {
                slope * x + intercept
            }
            Self::Quadratic { a, b, c } => {
                a * x * x + b * x + c
            }
            Self::Exponential { a, k } => {
                a * libm::exp(*k * x)
            }
            Self::Logistic { k, x0 } => {
                1.0 / (1.0 + libm::exp(-k * (x - x0)))
            }
            Self::InverseLogistic { k, x0 } => {
                1.0 - 1.0 / (1.0 + libm::exp(-k * (x - x0)))
            }
            Self::Step { threshold } => {
                if x >= *threshold { 1.0 } else { 0.0 }
            }
            Self::SmoothStep { edge0, edge1 } => {
                if x <= *edge0 {
                    0.0
                } else if x >= *edge1 {
                    1.0
                } else {
                    let t = (x - edge0) / (edge1 - edge0);
                    t * t * (3.0 - 2.0 * t)
                }
            }
            Self::Constant(v) => *v,
            Self::LookupTable(table) => {
                if table.is_empty() {
                    return 0.0;
                }
                if x <= table[0].0 {
                    return table[0].1;
                }
                if x >= table[table.len() - 1].0 {
                    return table[table.len() - 1].1;
                }

                // Find interval and interpolate
                for i in 1..table.len() {
                    if x < table[i].0 {
                        let t = (x - table[i-1].0) / (table[i].0 - table[i-1].0);
                        return table[i-1].1 + t * (table[i].1 - table[i-1].1);
                    }
                }
                table[table.len() - 1].1
            }
        };

        // Clamp to [0, 1]
        result.clamp(0.0, 1.0)
    }

    /// Create a linear curve from 0 to 1 over [min, max]
    pub fn linear_ascending(min: f64, max: f64) -> Self {
        let slope = 1.0 / (max - min);
        let intercept = -min / (max - min);
        Self::Linear { slope, intercept }
    }

    /// Create a linear curve from 1 to 0 over [min, max]
    pub fn linear_descending(min: f64, max: f64) -> Self {
        let slope = -1.0 / (max - min);
        let intercept = max / (max - min);
        Self::Linear { slope, intercept }
    }
}

// ============================================================================
// Consideration
// ============================================================================

/// A single consideration that contributes to utility score
pub struct Consideration {
    pub id: ConsiderationId,
    pub name: String,
    pub input_key: String,
    pub curve: UtilityCurve,
    pub weight: f64,
    pub is_bonus: bool, // If true, adds to score; if false, multiplies
}

impl Consideration {
    pub fn new(
        id: ConsiderationId,
        name: impl Into<String>,
        input_key: impl Into<String>,
        curve: UtilityCurve,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            input_key: input_key.into(),
            curve,
            weight: 1.0,
            is_bonus: false,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    pub fn as_bonus(mut self) -> Self {
        self.is_bonus = true;
        self
    }

    pub fn evaluate(&self, ctx: &UtilityContext) -> f64 {
        let input = ctx.get_or(&self.input_key, 0.0);
        let raw_score = self.curve.evaluate(input);
        raw_score * self.weight
    }
}

// ============================================================================
// Utility Action
// ============================================================================

/// An action with associated utility calculations
pub struct UtilityAction {
    pub id: ActionId,
    pub name: String,
    pub considerations: Vec<Consideration>,
    pub base_score: f64,
    pub cooldown: u64,
    pub last_execution: u64,
    pub execution_count: u64,
    pub is_enabled: bool,

    // Action execution
    action: Box<dyn Fn(&UtilityContext) + Send + Sync>,
}

impl UtilityAction {
    pub fn new<F>(
        id: ActionId,
        name: impl Into<String>,
        action: F,
    ) -> Self
    where
        F: Fn(&UtilityContext) + Send + Sync + 'static,
    {
        Self {
            id,
            name: name.into(),
            considerations: Vec::new(),
            base_score: 0.0,
            cooldown: 0,
            last_execution: 0,
            execution_count: 0,
            is_enabled: true,
            action: Box::new(action),
        }
    }

    pub fn with_consideration(mut self, consideration: Consideration) -> Self {
        self.considerations.push(consideration);
        self
    }

    pub fn with_base_score(mut self, score: f64) -> Self {
        self.base_score = score;
        self
    }

    pub fn with_cooldown(mut self, cooldown: u64) -> Self {
        self.cooldown = cooldown;
        self
    }

    pub fn add_consideration(&mut self, consideration: Consideration) {
        self.considerations.push(consideration);
    }

    /// Calculate total utility score using compensation factor
    pub fn calculate_score(&self, ctx: &UtilityContext) -> f64 {
        // Check cooldown
        if self.cooldown > 0 && ctx.time < self.last_execution + self.cooldown {
            return 0.0;
        }

        if !self.is_enabled || self.considerations.is_empty() {
            return self.base_score;
        }

        // Separate multiplicative and bonus considerations
        let mut mult_scores = Vec::new();
        let mut bonus_sum = 0.0;

        for consideration in &self.considerations {
            let score = consideration.evaluate(ctx);
            if consideration.is_bonus {
                bonus_sum += score;
            } else {
                mult_scores.push(score);
            }
        }

        if mult_scores.is_empty() {
            return self.base_score + bonus_sum;
        }

        // Calculate multiplicative score with compensation
        // Using the "normalized geometric mean" approach
        let n = mult_scores.len() as f64;
        let product: f64 = mult_scores.iter().product();

        // Compensation factor to avoid over-penalization
        let modification_factor = 1.0 - (1.0 / n);
        let compensated = product + (1.0 - product) * modification_factor * (1.0 - product);

        self.base_score + compensated + bonus_sum
    }

    pub fn execute(&mut self, ctx: &UtilityContext) {
        (self.action)(ctx);
        self.last_execution = ctx.time;
        self.execution_count += 1;
    }

    pub fn enable(&mut self) {
        self.is_enabled = true;
    }

    pub fn disable(&mut self) {
        self.is_enabled = false;
    }
}

// ============================================================================
// Utility Selector
// ============================================================================

/// Selection strategy for utility actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionStrategy {
    /// Always pick highest scoring action
    Highest,
    /// Weighted random based on scores
    WeightedRandom,
    /// Pick from top N with equal probability
    TopN(usize),
    /// Threshold-based: only consider actions above threshold
    Threshold(u64), // Using u64 to represent fixed-point (x1000)
}

/// Selector that chooses between utility actions
pub struct UtilitySelector {
    actions: Vec<UtilityAction>,
    strategy: SelectionStrategy,
    min_score_threshold: f64,
    last_selected: Option<ActionId>,
    rng_state: u64,
}

impl UtilitySelector {
    pub fn new(strategy: SelectionStrategy) -> Self {
        Self {
            actions: Vec::new(),
            strategy,
            min_score_threshold: 0.0,
            last_selected: None,
            rng_state: 12345,
        }
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.min_score_threshold = threshold;
        self
    }

    pub fn add_action(&mut self, action: UtilityAction) {
        self.actions.push(action);
    }

    pub fn action_count(&self) -> usize {
        self.actions.len()
    }

    pub fn get_action(&self, id: ActionId) -> Option<&UtilityAction> {
        self.actions.iter().find(|a| a.id == id)
    }

    pub fn get_action_mut(&mut self, id: ActionId) -> Option<&mut UtilityAction> {
        self.actions.iter_mut().find(|a| a.id == id)
    }

    /// Calculate all scores and return sorted list
    pub fn evaluate_all(&self, ctx: &UtilityContext) -> Vec<(ActionId, f64)> {
        let mut scores: Vec<(ActionId, f64)> = self.actions.iter()
            .filter(|a| a.is_enabled)
            .map(|a| (a.id, a.calculate_score(ctx)))
            .filter(|(_, score)| *score >= self.min_score_threshold)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        scores
    }

    /// Select the best action based on strategy
    pub fn select(&mut self, ctx: &UtilityContext) -> Option<ActionId> {
        let scores = self.evaluate_all(ctx);

        if scores.is_empty() {
            return None;
        }

        let selected = match self.strategy {
            SelectionStrategy::Highest => {
                scores.first().map(|(id, _)| *id)
            }
            SelectionStrategy::WeightedRandom => {
                let total: f64 = scores.iter().map(|(_, s)| s).sum();
                if total <= 0.0 {
                    return scores.first().map(|(id, _)| *id);
                }

                let r = self.random_f64() * total;
                let mut cumulative = 0.0;

                for (id, score) in &scores {
                    cumulative += score;
                    if cumulative >= r {
                        return Some(*id);
                    }
                }

                scores.last().map(|(id, _)| *id)
            }
            SelectionStrategy::TopN(n) => {
                let top_n: Vec<_> = scores.iter().take(n).collect();
                if top_n.is_empty() {
                    return None;
                }

                let idx = (self.random_u64() as usize) % top_n.len();
                Some(top_n[idx].0)
            }
            SelectionStrategy::Threshold(threshold_fixed) => {
                let threshold = threshold_fixed as f64 / 1000.0;
                let above_threshold: Vec<_> = scores.iter()
                    .filter(|(_, s)| *s >= threshold)
                    .collect();

                if above_threshold.is_empty() {
                    return None;
                }

                // Select highest among those above threshold
                above_threshold.first().map(|(id, _)| *id)
            }
        };

        self.last_selected = selected;
        selected
    }

    /// Select and execute the best action
    pub fn select_and_execute(&mut self, ctx: &UtilityContext) -> Option<ActionId> {
        let selected = self.select(ctx)?;

        if let Some(action) = self.actions.iter_mut().find(|a| a.id == selected) {
            action.execute(ctx);
        }

        Some(selected)
    }

    fn random_u64(&mut self) -> u64 {
        // Simple xorshift64
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    fn random_f64(&mut self) -> f64 {
        (self.random_u64() as f64) / (u64::MAX as f64)
    }
}

// ============================================================================
// Reasoner AI
// ============================================================================

/// High-level reasoner combining multiple utility selectors
pub struct ReasonerAI {
    name: String,
    buckets: BTreeMap<String, UtilitySelector>,
    active_bucket: Option<String>,
    decision_history: Vec<DecisionRecord>,
    max_history: usize,
}

/// Record of a decision made
#[derive(Debug, Clone)]
pub struct DecisionRecord {
    pub timestamp: u64,
    pub bucket: String,
    pub action_id: ActionId,
    pub score: f64,
}

impl ReasonerAI {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            buckets: BTreeMap::new(),
            active_bucket: None,
            decision_history: Vec::new(),
            max_history: 100,
        }
    }

    pub fn add_bucket(&mut self, name: impl Into<String>, selector: UtilitySelector) {
        let bucket_name = name.into();
        if self.active_bucket.is_none() {
            self.active_bucket = Some(bucket_name.clone());
        }
        self.buckets.insert(bucket_name, selector);
    }

    pub fn set_active_bucket(&mut self, name: &str) -> bool {
        if self.buckets.contains_key(name) {
            self.active_bucket = Some(name.to_string());
            true
        } else {
            false
        }
    }

    pub fn get_bucket(&self, name: &str) -> Option<&UtilitySelector> {
        self.buckets.get(name)
    }

    pub fn get_bucket_mut(&mut self, name: &str) -> Option<&mut UtilitySelector> {
        self.buckets.get_mut(name)
    }

    /// Decide on an action from the active bucket
    pub fn decide(&mut self, ctx: &UtilityContext) -> Option<ActionId> {
        let bucket_name = self.active_bucket.as_ref()?;
        let selector = self.buckets.get_mut(bucket_name)?;

        let scores = selector.evaluate_all(ctx);
        let action_id = selector.select(ctx)?;

        // Record decision
        let score = scores.iter()
            .find(|(id, _)| *id == action_id)
            .map(|(_, s)| *s)
            .unwrap_or(0.0);

        let record = DecisionRecord {
            timestamp: ctx.time,
            bucket: bucket_name.clone(),
            action_id,
            score,
        };

        self.decision_history.push(record);
        if self.decision_history.len() > self.max_history {
            self.decision_history.remove(0);
        }

        Some(action_id)
    }

    /// Decide and execute from active bucket
    pub fn decide_and_execute(&mut self, ctx: &UtilityContext) -> Option<ActionId> {
        let action_id = self.decide(ctx)?;

        if let Some(bucket_name) = &self.active_bucket {
            if let Some(selector) = self.buckets.get_mut(bucket_name) {
                if let Some(action) = selector.get_action_mut(action_id) {
                    action.execute(ctx);
                }
            }
        }

        Some(action_id)
    }

    /// Evaluate all buckets and select from the one with highest top score
    pub fn decide_across_buckets(&mut self, ctx: &UtilityContext) -> Option<(String, ActionId)> {
        let mut best_bucket = None;
        let mut best_action = None;
        let mut best_score = f64::NEG_INFINITY;

        for (name, selector) in &self.buckets {
            let scores = selector.evaluate_all(ctx);
            if let Some((action_id, score)) = scores.first() {
                if *score > best_score {
                    best_score = *score;
                    best_bucket = Some(name.clone());
                    best_action = Some(*action_id);
                }
            }
        }

        if let (Some(bucket), Some(action)) = (best_bucket, best_action) {
            self.active_bucket = Some(bucket.clone());
            Some((bucket, action))
        } else {
            None
        }
    }

    pub fn decision_history(&self) -> &[DecisionRecord] {
        &self.decision_history
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

// ============================================================================
// Kernel Utility Actions
// ============================================================================

/// Create a kernel memory management utility selector
pub fn create_kernel_memory_selector() -> UtilitySelector {
    let mut selector = UtilitySelector::new(SelectionStrategy::Highest);

    // Reclaim memory action
    let reclaim_action = UtilityAction::new(
        ActionId::new(1),
        "ReclaimMemory",
        |_ctx| {
            // In real kernel: trigger memory reclamation
        },
    )
    .with_consideration(Consideration::new(
        ConsiderationId::new(1),
        "MemoryPressure",
        "memory_pressure",
        UtilityCurve::Logistic { k: 10.0, x0: 0.7 },
    ))
    .with_consideration(Consideration::new(
        ConsiderationId::new(2),
        "FreePagesLow",
        "free_pages_ratio",
        UtilityCurve::InverseLogistic { k: 10.0, x0: 0.2 },
    ))
    .with_cooldown(1000);

    // Compact memory action
    let compact_action = UtilityAction::new(
        ActionId::new(2),
        "CompactMemory",
        |_ctx| {
            // In real kernel: trigger memory compaction
        },
    )
    .with_consideration(Consideration::new(
        ConsiderationId::new(3),
        "Fragmentation",
        "fragmentation_ratio",
        UtilityCurve::linear_ascending(0.3, 0.8),
    ))
    .with_consideration(Consideration::new(
        ConsiderationId::new(4),
        "LowCpuLoad",
        "cpu_load",
        UtilityCurve::linear_descending(0.1, 0.5),
    ))
    .with_cooldown(5000);

    // Swap out action
    let swap_action = UtilityAction::new(
        ActionId::new(3),
        "SwapOut",
        |_ctx| {
            // In real kernel: swap out cold pages
        },
    )
    .with_consideration(Consideration::new(
        ConsiderationId::new(5),
        "HighPressure",
        "memory_pressure",
        UtilityCurve::Logistic { k: 15.0, x0: 0.85 },
    ))
    .with_consideration(Consideration::new(
        ConsiderationId::new(6),
        "SwapAvailable",
        "swap_free_ratio",
        UtilityCurve::linear_ascending(0.1, 0.5),
    ))
    .with_cooldown(2000);

    selector.add_action(reclaim_action);
    selector.add_action(compact_action);
    selector.add_action(swap_action);

    selector
}

/// Create a kernel CPU scheduling utility selector
pub fn create_kernel_cpu_selector() -> UtilitySelector {
    let mut selector = UtilitySelector::new(SelectionStrategy::Highest);

    // Migrate tasks action
    let migrate_action = UtilityAction::new(
        ActionId::new(10),
        "MigrateTasks",
        |_ctx| {
            // In real kernel: balance tasks across CPUs
        },
    )
    .with_consideration(Consideration::new(
        ConsiderationId::new(10),
        "LoadImbalance",
        "cpu_load_imbalance",
        UtilityCurve::linear_ascending(0.2, 0.6),
    ))
    .with_cooldown(500);

    // Throttle background action
    let throttle_action = UtilityAction::new(
        ActionId::new(11),
        "ThrottleBackground",
        |_ctx| {
            // In real kernel: reduce background task priority
        },
    )
    .with_consideration(Consideration::new(
        ConsiderationId::new(11),
        "HighCpuLoad",
        "cpu_load",
        UtilityCurve::Logistic { k: 10.0, x0: 0.8 },
    ))
    .with_consideration(Consideration::new(
        ConsiderationId::new(12),
        "InteractiveActive",
        "interactive_processes",
        UtilityCurve::Step { threshold: 1.0 },
    ))
    .with_cooldown(1000);

    // Boost interactive action
    let boost_action = UtilityAction::new(
        ActionId::new(12),
        "BoostInteractive",
        |_ctx| {
            // In real kernel: increase interactive task priority
        },
    )
    .with_consideration(Consideration::new(
        ConsiderationId::new(13),
        "InteractiveWaiting",
        "interactive_wait_time",
        UtilityCurve::linear_ascending(10.0, 100.0),
    ))
    .with_cooldown(100);

    selector.add_action(migrate_action);
    selector.add_action(throttle_action);
    selector.add_action(boost_action);

    selector
}

/// Create a complete kernel reasoner AI
pub fn create_kernel_reasoner() -> ReasonerAI {
    let mut reasoner = ReasonerAI::new("KernelReasonerAI");

    reasoner.add_bucket("memory", create_kernel_memory_selector());
    reasoner.add_bucket("cpu", create_kernel_cpu_selector());

    reasoner
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utility_curve_linear() {
        let curve = UtilityCurve::Linear { slope: 1.0, intercept: 0.0 };
        assert!((curve.evaluate(0.5) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_utility_curve_step() {
        let curve = UtilityCurve::Step { threshold: 0.5 };
        assert_eq!(curve.evaluate(0.4), 0.0);
        assert_eq!(curve.evaluate(0.6), 1.0);
    }

    #[test]
    fn test_utility_context() {
        let mut ctx = UtilityContext::new();
        ctx.set("test", 0.5);
        assert_eq!(ctx.get("test"), Some(0.5));
        assert_eq!(ctx.get_or("missing", 1.0), 1.0);
    }

    #[test]
    fn test_consideration() {
        let ctx = {
            let mut c = UtilityContext::new();
            c.set("input", 0.5);
            c
        };

        let consideration = Consideration::new(
            ConsiderationId::new(1),
            "test",
            "input",
            UtilityCurve::Constant(1.0),
        );

        assert_eq!(consideration.evaluate(&ctx), 1.0);
    }

    #[test]
    fn test_utility_selector() {
        let selector = UtilitySelector::new(SelectionStrategy::Highest);
        assert_eq!(selector.action_count(), 0);
    }
}
