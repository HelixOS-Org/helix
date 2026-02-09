//! Adaptive RCU Configuration
//!
//! This module provides adaptive configuration management for RCU tuning.

use alloc::string::String;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use super::{GracePeriodStats, MemoryPressureLevel};

/// RCU configuration parameter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcuConfigParam {
    /// Grace period delay
    GpDelay,
    /// Jiffies till first fqs
    JiffiesTillFirstFqs,
    /// Jiffies till next fqs
    JiffiesTillNextFqs,
    /// Callback batch limit
    CallbackBatchLimit,
    /// Expedited threshold
    ExpeditedThreshold,
    /// Stall timeout
    StallTimeout,
    /// Nocb threshold
    NocbThreshold,
}

/// Configuration recommendation
#[derive(Debug, Clone)]
pub struct ConfigRecommendation {
    /// Parameter to adjust
    pub param: RcuConfigParam,
    /// Current value
    pub current_value: u64,
    /// Recommended value
    pub recommended_value: u64,
    /// Reason for recommendation
    pub reason: String,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Confidence (0-1)
    pub confidence: f32,
}

/// Adaptive RCU configuration
#[derive(Debug, Clone)]
pub struct RcuConfig {
    /// Grace period delay (nanoseconds)
    pub gp_delay_ns: u64,
    /// Jiffies till first force quiescent state
    pub jiffies_till_first_fqs: u32,
    /// Jiffies till next force quiescent state
    pub jiffies_till_next_fqs: u32,
    /// Callback batch limit
    pub callback_batch_limit: u32,
    /// Expedited callback threshold
    pub expedited_threshold: u64,
    /// Stall timeout (nanoseconds)
    pub stall_timeout_ns: u64,
    /// Nocb (offload) threshold
    pub nocb_threshold: u64,
    /// Power-saving mode
    pub power_saving: bool,
    /// Lazy callbacks enabled
    pub lazy_callbacks: bool,
}

impl Default for RcuConfig {
    fn default() -> Self {
        Self {
            gp_delay_ns: 0,
            jiffies_till_first_fqs: 3,
            jiffies_till_next_fqs: 3,
            callback_batch_limit: 64,
            expedited_threshold: 10000,
            stall_timeout_ns: 21_000_000_000, // 21 seconds
            nocb_threshold: 1000,
            power_saving: false,
            lazy_callbacks: false,
        }
    }
}

/// Adaptive RCU configurator
pub struct AdaptiveConfigurator {
    /// Current configuration
    config: RcuConfig,
    /// Configuration history
    history: VecDeque<(u64, RcuConfig)>,
    /// Maximum history entries
    max_history: usize,
    /// Auto-tune enabled
    auto_tune: bool,
    /// Last tuning timestamp
    last_tune_ns: u64,
    /// Tuning interval (nanoseconds)
    tune_interval_ns: u64,
    /// Pending recommendations
    recommendations: Vec<ConfigRecommendation>,
    /// Applied configuration changes
    changes_applied: u64,
}

impl AdaptiveConfigurator {
    /// Create new adaptive configurator
    pub fn new() -> Self {
        Self {
            config: RcuConfig::default(),
            history: VecDeque::new(),
            max_history: 100,
            auto_tune: true,
            last_tune_ns: 0,
            tune_interval_ns: 60_000_000_000, // 1 minute
            recommendations: Vec::new(),
            changes_applied: 0,
        }
    }

    /// Get current configuration
    #[inline(always)]
    pub fn config(&self) -> &RcuConfig {
        &self.config
    }

    /// Get mutable configuration
    #[inline(always)]
    pub fn config_mut(&mut self) -> &mut RcuConfig {
        &mut self.config
    }

    /// Analyze and generate recommendations
    pub fn analyze(
        &mut self,
        gp_stats: &GracePeriodStats,
        pressure_level: MemoryPressureLevel,
        callback_count: u64,
        current_time_ns: u64,
    ) {
        self.recommendations.clear();

        // Check if we should tune
        if current_time_ns - self.last_tune_ns < self.tune_interval_ns {
            return;
        }
        self.last_tune_ns = current_time_ns;

        // Analyze grace period duration
        if gp_stats.avg_duration_ns > 50_000_000 {
            // > 50ms
            self.recommendations.push(ConfigRecommendation {
                param: RcuConfigParam::JiffiesTillFirstFqs,
                current_value: self.config.jiffies_till_first_fqs as u64,
                recommended_value: (self.config.jiffies_till_first_fqs / 2).max(1) as u64,
                reason: String::from("Long grace periods detected"),
                expected_improvement: 20.0,
                confidence: 0.7,
            });
        }

        // Analyze memory pressure
        if pressure_level >= MemoryPressureLevel::High {
            self.recommendations.push(ConfigRecommendation {
                param: RcuConfigParam::ExpeditedThreshold,
                current_value: self.config.expedited_threshold,
                recommended_value: self.config.expedited_threshold / 2,
                reason: String::from("High memory pressure"),
                expected_improvement: 30.0,
                confidence: 0.8,
            });

            // Recommend lazy callbacks off under pressure
            if self.config.lazy_callbacks {
                self.recommendations.push(ConfigRecommendation {
                    param: RcuConfigParam::CallbackBatchLimit,
                    current_value: 1,     // lazy on
                    recommended_value: 0, // lazy off
                    reason: String::from("Disable lazy callbacks under pressure"),
                    expected_improvement: 25.0,
                    confidence: 0.9,
                });
            }
        }

        // Analyze stall rate
        if gp_stats.stall_count > 0 {
            self.recommendations.push(ConfigRecommendation {
                param: RcuConfigParam::StallTimeout,
                current_value: self.config.stall_timeout_ns,
                recommended_value: self.config.stall_timeout_ns * 2,
                reason: String::from("RCU stalls detected"),
                expected_improvement: 15.0,
                confidence: 0.6,
            });
        }

        // Analyze callback throughput
        if callback_count > self.config.nocb_threshold * 10 {
            self.recommendations.push(ConfigRecommendation {
                param: RcuConfigParam::NocbThreshold,
                current_value: self.config.nocb_threshold,
                recommended_value: self.config.nocb_threshold * 2,
                reason: String::from("High callback load, consider nocb"),
                expected_improvement: 20.0,
                confidence: 0.7,
            });
        }
    }

    /// Apply recommendation
    pub fn apply_recommendation(&mut self, param: RcuConfigParam, value: u64, timestamp_ns: u64) {
        // Save current config to history
        if self.history.len() >= self.max_history {
            self.history.pop_front();
        }
        self.history.push_back((timestamp_ns, self.config.clone()));

        // Apply change
        match param {
            RcuConfigParam::GpDelay => {
                self.config.gp_delay_ns = value;
            },
            RcuConfigParam::JiffiesTillFirstFqs => {
                self.config.jiffies_till_first_fqs = value as u32;
            },
            RcuConfigParam::JiffiesTillNextFqs => {
                self.config.jiffies_till_next_fqs = value as u32;
            },
            RcuConfigParam::CallbackBatchLimit => {
                self.config.callback_batch_limit = value as u32;
            },
            RcuConfigParam::ExpeditedThreshold => {
                self.config.expedited_threshold = value;
            },
            RcuConfigParam::StallTimeout => {
                self.config.stall_timeout_ns = value;
            },
            RcuConfigParam::NocbThreshold => {
                self.config.nocb_threshold = value;
            },
        }

        self.changes_applied += 1;
    }

    /// Get pending recommendations
    #[inline(always)]
    pub fn recommendations(&self) -> &[ConfigRecommendation] {
        &self.recommendations
    }

    /// Enable auto-tuning
    #[inline(always)]
    pub fn set_auto_tune(&mut self, enabled: bool) {
        self.auto_tune = enabled;
    }

    /// Check if auto-tune is enabled
    #[inline(always)]
    pub fn is_auto_tune(&self) -> bool {
        self.auto_tune
    }

    /// Get changes applied count
    #[inline(always)]
    pub fn changes_applied(&self) -> u64 {
        self.changes_applied
    }

    /// Rollback to previous configuration
    #[inline]
    pub fn rollback(&mut self) -> bool {
        if let Some((_, config)) = self.history.pop() {
            self.config = config;
            true
        } else {
            false
        }
    }
}

impl Default for AdaptiveConfigurator {
    fn default() -> Self {
        Self::new()
    }
}
