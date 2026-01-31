//! Main optimizer implementation

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::core::NexusTimestamp;

use super::arch::{Architecture, CpuFeatures};
use super::level::{OptimizationLevel, OptimizationTarget};
use super::parameter::OptimizationParameter;

// ============================================================================
// OPTIMIZATION METRIC
// ============================================================================

/// An optimization metric sample
#[derive(Debug, Clone)]
pub struct OptimizationMetric {
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Metric name
    pub name: String,
    /// Value
    pub value: f64,
}

// ============================================================================
// OPTIMIZATION CHANGE
// ============================================================================

/// An optimization change
#[derive(Debug, Clone)]
pub struct OptimizationChange {
    /// Parameter name
    pub parameter: String,
    /// Old value
    pub old_value: f64,
    /// New value
    pub new_value: f64,
    /// Reason for change
    pub reason: String,
}

// ============================================================================
// OPTIMIZER STATS
// ============================================================================

/// Optimizer statistics
#[derive(Debug, Clone)]
pub struct OptimizerStats {
    /// Current architecture
    pub arch: Architecture,
    /// Current level
    pub level: OptimizationLevel,
    /// Current target
    pub target: OptimizationTarget,
    /// Number of parameters
    pub parameter_count: usize,
    /// Total optimizations applied
    pub total_optimizations: u64,
    /// Metrics recorded
    pub metrics_recorded: usize,
}

// ============================================================================
// OPTIMIZER
// ============================================================================

/// The optimization engine
pub struct Optimizer {
    /// Current architecture
    arch: Architecture,
    /// CPU features
    features: CpuFeatures,
    /// Optimization level
    level: OptimizationLevel,
    /// Optimization target
    target: OptimizationTarget,
    /// Parameters
    parameters: BTreeMap<String, OptimizationParameter>,
    /// Is optimizer enabled?
    enabled: AtomicBool,
    /// Total optimizations applied
    total_optimizations: AtomicU64,
    /// Metrics history for adaptive optimization
    metrics: Vec<OptimizationMetric>,
    /// Maximum metrics to keep
    max_metrics: usize,
}

impl Optimizer {
    /// Create a new optimizer
    pub fn new() -> Self {
        let arch = Architecture::detect();
        let features = CpuFeatures::detect();

        let mut optimizer = Self {
            arch,
            features,
            level: OptimizationLevel::Moderate,
            target: OptimizationTarget::Balanced,
            parameters: BTreeMap::new(),
            enabled: AtomicBool::new(true),
            total_optimizations: AtomicU64::new(0),
            metrics: Vec::new(),
            max_metrics: 10000,
        };

        // Initialize default parameters
        optimizer.init_default_parameters();

        optimizer
    }

    /// Initialize default parameters
    fn init_default_parameters(&mut self) {
        // Memory parameters
        self.add_parameter(
            OptimizationParameter::new("memory.prefetch_distance", 8.0, 0.0, 64.0)
                .with_description("Prefetch distance in cache lines"),
        );

        self.add_parameter(
            OptimizationParameter::new(
                "memory.alignment",
                self.arch.cache_line_size() as f64,
                8.0,
                4096.0,
            )
            .with_description("Memory alignment for allocations"),
        );

        // Scheduler parameters
        self.add_parameter(
            OptimizationParameter::new("scheduler.quantum_us", 10000.0, 100.0, 100000.0)
                .with_description("Scheduler time quantum in microseconds"),
        );

        self.add_parameter(
            OptimizationParameter::new(
                "scheduler.load_balance_interval_ms",
                100.0,
                10.0,
                1000.0,
            )
            .with_description("Load balancing interval in milliseconds"),
        );

        // I/O parameters
        self.add_parameter(
            OptimizationParameter::new("io.batch_size", 32.0, 1.0, 256.0)
                .with_description("I/O batch size"),
        );

        self.add_parameter(
            OptimizationParameter::new("io.queue_depth", 64.0, 1.0, 1024.0)
                .with_description("I/O queue depth"),
        );

        // Network parameters
        self.add_parameter(
            OptimizationParameter::new("network.buffer_size", 65536.0, 4096.0, 1048576.0)
                .with_description("Network buffer size"),
        );

        // Vector parameters
        self.add_parameter(
            OptimizationParameter::new(
                "vector.width",
                self.features.best_vector_width() as f64,
                8.0,
                64.0,
            )
            .with_description("Vector operation width"),
        );
    }

    /// Add a parameter
    pub fn add_parameter(&mut self, param: OptimizationParameter) {
        self.parameters.insert(param.name.clone(), param);
    }

    /// Get a parameter
    pub fn get_parameter(&self, name: &str) -> Option<&OptimizationParameter> {
        self.parameters.get(name)
    }

    /// Set a parameter value
    pub fn set_parameter(&mut self, name: &str, value: f64) -> bool {
        if let Some(param) = self.parameters.get_mut(name) {
            param.set(value);
            true
        } else {
            false
        }
    }

    /// Get architecture
    pub fn arch(&self) -> Architecture {
        self.arch
    }

    /// Get CPU features
    pub fn features(&self) -> &CpuFeatures {
        &self.features
    }

    /// Set optimization level
    pub fn set_level(&mut self, level: OptimizationLevel) {
        self.level = level;
        self.apply_level_defaults();
    }

    /// Set optimization target
    pub fn set_target(&mut self, target: OptimizationTarget) {
        self.target = target;
        self.apply_target_defaults();
    }

    /// Apply level defaults
    fn apply_level_defaults(&mut self) {
        match self.level {
            OptimizationLevel::None => {
                // Reset all to defaults
                for param in self.parameters.values_mut() {
                    param.reset();
                }
            }
            OptimizationLevel::Light => {
                // Conservative settings
                self.set_parameter("memory.prefetch_distance", 4.0);
                self.set_parameter("io.batch_size", 16.0);
            }
            OptimizationLevel::Moderate => {
                // Balanced settings
                self.set_parameter("memory.prefetch_distance", 8.0);
                self.set_parameter("io.batch_size", 32.0);
            }
            OptimizationLevel::Aggressive => {
                // Performance-focused
                self.set_parameter("memory.prefetch_distance", 16.0);
                self.set_parameter("io.batch_size", 64.0);
                self.set_parameter("io.queue_depth", 128.0);
            }
            OptimizationLevel::Maximum => {
                // Maximum performance
                self.set_parameter("memory.prefetch_distance", 32.0);
                self.set_parameter("io.batch_size", 128.0);
                self.set_parameter("io.queue_depth", 256.0);
            }
        }
        self.total_optimizations.fetch_add(1, Ordering::Relaxed);
    }

    /// Apply target defaults
    fn apply_target_defaults(&mut self) {
        match self.target {
            OptimizationTarget::Latency => {
                self.set_parameter("scheduler.quantum_us", 1000.0);
                self.set_parameter("io.batch_size", 8.0);
            }
            OptimizationTarget::Throughput => {
                self.set_parameter("scheduler.quantum_us", 20000.0);
                self.set_parameter("io.batch_size", 64.0);
            }
            OptimizationTarget::Memory => {
                self.set_parameter("network.buffer_size", 16384.0);
                self.set_parameter("io.queue_depth", 32.0);
            }
            OptimizationTarget::Power => {
                self.set_parameter("scheduler.quantum_us", 50000.0);
                self.set_parameter("scheduler.load_balance_interval_ms", 500.0);
            }
            OptimizationTarget::Balanced => {
                // Use level defaults
            }
        }
        self.total_optimizations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a metric for adaptive optimization
    pub fn record_metric(&mut self, name: impl Into<String>, value: f64) {
        let metric = OptimizationMetric {
            timestamp: NexusTimestamp::now(),
            name: name.into(),
            value,
        };

        if self.metrics.len() >= self.max_metrics {
            self.metrics.remove(0);
        }
        self.metrics.push(metric);
    }

    /// Run adaptive optimization
    pub fn optimize(&mut self) -> Vec<OptimizationChange> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Vec::new();
        }

        let mut changes = Vec::new();

        // Simple adaptive optimization based on recent metrics
        // In a real system, this would use ML or control theory

        // Example: Adjust prefetch distance based on cache miss rate
        if let Some(miss_rate) = self.get_recent_metric("cache_miss_rate") {
            let current = self
                .get_parameter("memory.prefetch_distance")
                .map(|p| p.value)
                .unwrap_or(8.0);

            let new_value = if miss_rate > 0.1 {
                (current * 1.5).min(64.0)
            } else if miss_rate < 0.01 {
                (current * 0.75).max(1.0)
            } else {
                current
            };

            if (new_value - current).abs() > 0.5 {
                self.set_parameter("memory.prefetch_distance", new_value);
                changes.push(OptimizationChange {
                    parameter: "memory.prefetch_distance".into(),
                    old_value: current,
                    new_value,
                    reason: "Adaptive cache optimization".into(),
                });
            }
        }

        self.total_optimizations.fetch_add(1, Ordering::Relaxed);
        changes
    }

    /// Get recent metric average
    fn get_recent_metric(&self, name: &str) -> Option<f64> {
        let recent: Vec<_> = self
            .metrics
            .iter()
            .filter(|m| m.name == name)
            .rev()
            .take(10)
            .collect();

        if recent.is_empty() {
            None
        } else {
            Some(recent.iter().map(|m| m.value).sum::<f64>() / recent.len() as f64)
        }
    }

    /// Enable optimizer
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable optimizer
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Get all parameter values
    pub fn all_parameters(&self) -> Vec<(&str, f64)> {
        self.parameters
            .iter()
            .map(|(k, v)| (k.as_str(), v.value))
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> OptimizerStats {
        OptimizerStats {
            arch: self.arch,
            level: self.level,
            target: self.target,
            parameter_count: self.parameters.len(),
            total_optimizations: self.total_optimizations.load(Ordering::Relaxed),
            metrics_recorded: self.metrics.len(),
        }
    }
}

impl Default for Optimizer {
    fn default() -> Self {
        Self::new()
    }
}
