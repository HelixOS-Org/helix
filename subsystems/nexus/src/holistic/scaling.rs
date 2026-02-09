//! # Holistic Auto-Scaling
//!
//! System-wide auto-scaling decisions:
//! - Vertical scaling (add resources)
//! - Horizontal scaling (add instances)
//! - Predictive scaling (anticipate demand)
//! - Cool-down management
//! - Scale-to-zero support
//! - Multi-dimensional scaling

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// SCALING DIMENSIONS
// ============================================================================

/// Scaling dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalingDimension {
    /// CPU cores
    CpuCores,
    /// Memory
    Memory,
    /// I/O bandwidth
    IoBandwidth,
    /// Network bandwidth
    NetworkBandwidth,
    /// Worker threads
    WorkerThreads,
    /// Queue depth
    QueueDepth,
    /// Cache size
    CacheSize,
    /// Buffer pool
    BufferPool,
}

/// Scaling direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingDirection {
    /// Scale up (increase)
    Up,
    /// Scale down (decrease)
    Down,
    /// No change
    None,
}

/// Scaling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingMode {
    /// Manual (operator-triggered)
    Manual,
    /// Reactive (threshold-based)
    Reactive,
    /// Predictive (ML-based)
    Predictive,
    /// Scheduled (time-based)
    Scheduled,
}

// ============================================================================
// SCALING POLICY
// ============================================================================

/// Threshold-based scaling trigger
#[derive(Debug, Clone)]
pub struct ScalingThreshold {
    /// Dimension
    pub dimension: ScalingDimension,
    /// Scale up threshold (0.0-1.0)
    pub scale_up_threshold: f64,
    /// Scale down threshold (0.0-1.0)
    pub scale_down_threshold: f64,
    /// Scale up amount
    pub scale_up_amount: u64,
    /// Scale down amount
    pub scale_down_amount: u64,
    /// Consecutive samples needed
    pub required_samples: u32,
    /// Current consecutive up samples
    pub consecutive_up: u32,
    /// Current consecutive down samples
    pub consecutive_down: u32,
}

impl ScalingThreshold {
    pub fn new(
        dimension: ScalingDimension,
        up_threshold: f64,
        down_threshold: f64,
        up_amount: u64,
        down_amount: u64,
    ) -> Self {
        Self {
            dimension,
            scale_up_threshold: up_threshold,
            scale_down_threshold: down_threshold,
            scale_up_amount: up_amount,
            scale_down_amount: down_amount,
            required_samples: 3,
            consecutive_up: 0,
            consecutive_down: 0,
        }
    }

    /// Evaluate utilization
    pub fn evaluate(&mut self, utilization: f64) -> ScalingDirection {
        if utilization >= self.scale_up_threshold {
            self.consecutive_up += 1;
            self.consecutive_down = 0;
            if self.consecutive_up >= self.required_samples {
                return ScalingDirection::Up;
            }
        } else if utilization <= self.scale_down_threshold {
            self.consecutive_down += 1;
            self.consecutive_up = 0;
            if self.consecutive_down >= self.required_samples {
                return ScalingDirection::Down;
            }
        } else {
            self.consecutive_up = 0;
            self.consecutive_down = 0;
        }
        ScalingDirection::None
    }

    /// Reset counters
    #[inline(always)]
    pub fn reset(&mut self) {
        self.consecutive_up = 0;
        self.consecutive_down = 0;
    }
}

/// Scaling policy
#[derive(Debug, Clone)]
pub struct ScalingPolicy {
    /// Policy ID
    pub id: u64,
    /// Target entity
    pub target: u64,
    /// Mode
    pub mode: ScalingMode,
    /// Thresholds
    pub thresholds: Vec<ScalingThreshold>,
    /// Minimum scale
    pub min_scale: BTreeMap<u8, u64>,
    /// Maximum scale
    pub max_scale: BTreeMap<u8, u64>,
    /// Cool-down period (ns)
    pub cooldown_ns: u64,
    /// Last scale action time
    pub last_scale_time: u64,
    /// Enabled
    pub enabled: bool,
}

impl ScalingPolicy {
    pub fn new(id: u64, target: u64, mode: ScalingMode) -> Self {
        Self {
            id,
            target,
            mode,
            thresholds: Vec::new(),
            min_scale: BTreeMap::new(),
            max_scale: BTreeMap::new(),
            cooldown_ns: 60_000_000_000, // 60s default
            last_scale_time: 0,
            enabled: true,
        }
    }

    #[inline(always)]
    pub fn add_threshold(&mut self, threshold: ScalingThreshold) {
        self.thresholds.push(threshold);
    }

    #[inline(always)]
    pub fn set_min(&mut self, dimension: ScalingDimension, min: u64) {
        self.min_scale.insert(dimension as u8, min);
    }

    #[inline(always)]
    pub fn set_max(&mut self, dimension: ScalingDimension, max: u64) {
        self.max_scale.insert(dimension as u8, max);
    }

    /// Is in cooldown
    #[inline(always)]
    pub fn in_cooldown(&self, now: u64) -> bool {
        now.saturating_sub(self.last_scale_time) < self.cooldown_ns
    }

    /// Clamp value to min/max for dimension
    #[inline]
    pub fn clamp(&self, dimension: ScalingDimension, value: u64) -> u64 {
        let key = dimension as u8;
        let min = self.min_scale.get(&key).copied().unwrap_or(0);
        let max = self.max_scale.get(&key).copied().unwrap_or(u64::MAX);
        value.max(min).min(max)
    }
}

// ============================================================================
// SCALING DECISION
// ============================================================================

/// A scaling decision
#[derive(Debug, Clone)]
pub struct ScalingDecision {
    /// Policy that triggered
    pub policy_id: u64,
    /// Target entity
    pub target: u64,
    /// Direction
    pub direction: ScalingDirection,
    /// Dimension
    pub dimension: ScalingDimension,
    /// Current value
    pub current: u64,
    /// Proposed value
    pub proposed: u64,
    /// Reason
    pub reason: ScalingReason,
    /// Timestamp
    pub timestamp: u64,
    /// Executed
    pub executed: bool,
}

/// Reason for scaling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingReason {
    /// Threshold breach
    ThresholdBreach,
    /// Predictive forecast
    PredictiveForecast,
    /// Schedule trigger
    ScheduleTrigger,
    /// Manual request
    ManualRequest,
    /// Emergency response
    Emergency,
    /// Cool-down expiry rebalance
    Rebalance,
}

// ============================================================================
// PREDICTIVE SCALING
// ============================================================================

/// Demand forecast
#[derive(Debug, Clone)]
pub struct DemandForecast {
    /// Dimension
    pub dimension: ScalingDimension,
    /// Predicted utilization
    pub predicted_utilization: f64,
    /// Confidence (0.0-1.0)
    pub confidence: f64,
    /// Forecast horizon (ns ahead)
    pub horizon_ns: u64,
    /// Timestamp of prediction
    pub timestamp: u64,
}

/// Simple demand predictor using exponential smoothing
#[derive(Debug, Clone)]
pub struct DemandPredictor {
    /// Dimension
    pub dimension: ScalingDimension,
    /// Smoothed value
    smoothed: f64,
    /// Trend
    trend: f64,
    /// Alpha (level smoothing)
    alpha: f64,
    /// Beta (trend smoothing)
    beta: f64,
    /// Sample count
    samples: u64,
}

impl DemandPredictor {
    pub fn new(dimension: ScalingDimension) -> Self {
        Self {
            dimension,
            smoothed: 0.0,
            trend: 0.0,
            alpha: 0.3,
            beta: 0.1,
            samples: 0,
        }
    }

    /// Update with observation
    #[inline]
    pub fn observe(&mut self, utilization: f64) {
        if self.samples == 0 {
            self.smoothed = utilization;
            self.trend = 0.0;
        } else {
            let prev = self.smoothed;
            self.smoothed = self.alpha * utilization + (1.0 - self.alpha) * (prev + self.trend);
            self.trend = self.beta * (self.smoothed - prev) + (1.0 - self.beta) * self.trend;
        }
        self.samples += 1;
    }

    /// Forecast ahead
    #[inline]
    pub fn forecast(&self, steps_ahead: u32) -> f64 {
        let predicted = self.smoothed + self.trend * steps_ahead as f64;
        // Clamp to 0.0..1.0
        if predicted < 0.0 {
            0.0
        } else if predicted > 1.0 {
            1.0
        } else {
            predicted
        }
    }

    /// Confidence based on sample count
    #[inline]
    pub fn confidence(&self) -> f64 {
        let max_conf = 0.95;
        let ramp = self.samples as f64 / 100.0;
        if ramp > max_conf { max_conf } else { ramp }
    }
}

// ============================================================================
// SCALING MANAGER
// ============================================================================

/// Scaling manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticScalingStats {
    /// Active policies
    pub active_policies: usize,
    /// Scale up events
    pub scale_up_events: u64,
    /// Scale down events
    pub scale_down_events: u64,
    /// Decisions pending
    pub pending_decisions: usize,
    /// Predictions made
    pub predictions: u64,
}

/// System-wide scaling manager
pub struct HolisticScalingManager {
    /// Policies
    policies: BTreeMap<u64, ScalingPolicy>,
    /// Pending decisions
    pending: Vec<ScalingDecision>,
    /// Decision history
    history: VecDeque<ScalingDecision>,
    /// Demand predictors per (target, dimension)
    predictors: BTreeMap<(u64, u8), DemandPredictor>,
    /// Current resource levels (target, dimension) → value
    current_levels: BTreeMap<(u64, u8), u64>,
    /// Next policy ID
    next_id: u64,
    /// Max history
    max_history: usize,
    /// Stats
    stats: HolisticScalingStats,
}

impl HolisticScalingManager {
    pub fn new() -> Self {
        Self {
            policies: BTreeMap::new(),
            pending: Vec::new(),
            history: VecDeque::new(),
            predictors: BTreeMap::new(),
            current_levels: BTreeMap::new(),
            next_id: 1,
            max_history: 500,
            stats: HolisticScalingStats::default(),
        }
    }

    /// Add policy
    #[inline]
    pub fn add_policy(&mut self, policy: ScalingPolicy) -> u64 {
        let id = policy.id;
        self.policies.insert(id, policy);
        self.stats.active_policies = self.policies.len();
        id
    }

    /// Remove policy
    #[inline(always)]
    pub fn remove_policy(&mut self, id: u64) {
        self.policies.remove(&id);
        self.stats.active_policies = self.policies.len();
    }

    /// Set current resource level
    #[inline(always)]
    pub fn set_level(&mut self, target: u64, dimension: ScalingDimension, value: u64) {
        self.current_levels.insert((target, dimension as u8), value);
    }

    /// Report utilization
    pub fn report_utilization(
        &mut self,
        target: u64,
        dimension: ScalingDimension,
        utilization: f64,
        now: u64,
    ) {
        // Update predictor
        let key = (target, dimension as u8);
        let predictor = self
            .predictors
            .entry(key)
            .or_insert_with(|| DemandPredictor::new(dimension));
        predictor.observe(utilization);

        // Evaluate policies
        let mut decisions = Vec::new();
        for policy in self.policies.values_mut() {
            if !policy.enabled || policy.target != target || policy.in_cooldown(now) {
                continue;
            }

            for threshold in &mut policy.thresholds {
                if threshold.dimension != dimension {
                    continue;
                }

                let direction = threshold.evaluate(utilization);
                if direction == ScalingDirection::None {
                    continue;
                }

                let current = self.current_levels.get(&key).copied().unwrap_or(0);
                let proposed = match direction {
                    ScalingDirection::Up => {
                        policy.clamp(dimension, current + threshold.scale_up_amount)
                    },
                    ScalingDirection::Down => policy.clamp(
                        dimension,
                        current.saturating_sub(threshold.scale_down_amount),
                    ),
                    ScalingDirection::None => continue,
                };

                if proposed != current {
                    decisions.push(ScalingDecision {
                        policy_id: policy.id,
                        target,
                        direction,
                        dimension,
                        current,
                        proposed,
                        reason: ScalingReason::ThresholdBreach,
                        timestamp: now,
                        executed: false,
                    });
                    threshold.reset();
                }
            }
        }

        for d in &decisions {
            match d.direction {
                ScalingDirection::Up => self.stats.scale_up_events += 1,
                ScalingDirection::Down => self.stats.scale_down_events += 1,
                _ => {},
            }
        }
        self.pending.extend(decisions);
        self.stats.pending_decisions = self.pending.len();
    }

    /// Get predictive forecast
    pub fn forecast(
        &mut self,
        target: u64,
        dimension: ScalingDimension,
        steps_ahead: u32,
        now: u64,
    ) -> Option<DemandForecast> {
        let key = (target, dimension as u8);
        let predictor = self.predictors.get(&key)?;
        self.stats.predictions += 1;
        Some(DemandForecast {
            dimension,
            predicted_utilization: predictor.forecast(steps_ahead),
            confidence: predictor.confidence(),
            horizon_ns: steps_ahead as u64 * 1_000_000_000,
            timestamp: now,
        })
    }

    /// Drain pending decisions
    #[inline]
    pub fn drain_decisions(&mut self) -> Vec<ScalingDecision> {
        let decisions = core::mem::take(&mut self.pending);
        for d in &decisions {
            self.history.push_back(d.clone());
        }
        while self.history.len() > self.max_history {
            self.history.pop_front();
        }
        self.stats.pending_decisions = 0;
        decisions
    }

    /// Acknowledge execution
    #[inline]
    pub fn acknowledge(
        &mut self,
        target: u64,
        dimension: ScalingDimension,
        new_level: u64,
        now: u64,
    ) {
        self.current_levels
            .insert((target, dimension as u8), new_level);
        // Update cooldown on affected policies
        for policy in self.policies.values_mut() {
            if policy.target == target {
                policy.last_scale_time = now;
            }
        }
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticScalingStats {
        &self.stats
    }
}
