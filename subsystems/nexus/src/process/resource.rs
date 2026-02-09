//! Resource Prediction and Priority Optimization
//!
//! Predicts process resource needs and optimizes priorities.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;
use super::{CpuProfile, ProcessId, ProcessProfile, ProcessType};

/// Resource prediction
#[derive(Debug, Clone)]
pub struct ResourcePrediction {
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Predicted CPU need
    pub cpu_need: f64,
    /// Predicted memory need
    pub memory_need: u64,
    /// Predicted I/O need
    pub io_need: f64,
    /// Confidence
    pub confidence: f64,
}

/// Predicts process resource needs
pub struct ResourcePredictor {
    /// Historical predictions
    history: BTreeMap<ProcessId, Vec<ResourcePrediction>>,
    /// Max history per process
    max_history: usize,
}

impl ResourcePredictor {
    /// Create new predictor
    pub fn new() -> Self {
        Self {
            history: BTreeMap::new(),
            max_history: 100,
        }
    }

    /// Predict resources for process
    pub fn predict(&mut self, profile: &ProcessProfile) -> ResourcePrediction {
        let prediction = ResourcePrediction {
            timestamp: NexusTimestamp::now(),
            cpu_need: profile.avg_cpu_usage,
            memory_need: (profile.avg_memory + profile.memory_growth_rate * 10.0) as u64,
            io_need: profile.avg_io_rate,
            confidence: self.calculate_confidence(profile),
        };

        let history = self.history.entry(profile.pid).or_default();
        history.push(prediction.clone());
        if history.len() > self.max_history {
            history.pop_front();
        }

        prediction
    }

    /// Calculate prediction confidence
    fn calculate_confidence(&self, profile: &ProcessProfile) -> f64 {
        let sample_factor = (profile.sample_count as f64 / 1000.0).min(1.0);
        let variance_factor = 1.0 - profile.cpu_variance.min(1.0);
        (sample_factor * 0.5 + variance_factor * 0.5).max(0.1)
    }

    /// Get prediction accuracy
    pub fn accuracy(&self, pid: ProcessId) -> Option<f64> {
        let history = self.history.get(&pid)?;
        if history.len() < 10 {
            return None;
        }

        let cpu_predictions: Vec<_> = history.iter().map(|p| p.cpu_need).collect();
        let avg: f64 = cpu_predictions.iter().sum::<f64>() / cpu_predictions.len() as f64;
        let variance: f64 = cpu_predictions
            .iter()
            .map(|&x| (x - avg).powi(2))
            .sum::<f64>()
            / cpu_predictions.len() as f64;

        Some(1.0 - variance.min(1.0))
    }

    /// Get last prediction
    #[inline(always)]
    pub fn last_prediction(&self, pid: ProcessId) -> Option<&ResourcePrediction> {
        self.history.get(&pid)?.last()
    }
}

impl Default for ResourcePredictor {
    fn default() -> Self {
        Self::new()
    }
}

/// Priority adjustment
#[derive(Debug, Clone)]
struct PriorityAdjustment {
    /// Current priority
    current: i8,
    /// Suggested priority
    suggested: i8,
    /// Confidence
    confidence: f64,
    /// Last adjustment
    last_adjustment: NexusTimestamp,
}

/// Optimizes process priorities
pub struct PriorityOptimizer {
    /// Priority adjustments
    adjustments: BTreeMap<ProcessId, PriorityAdjustment>,
    /// Priority history
    history: VecDeque<(ProcessId, i8, i8)>,
    /// Max history
    max_history: usize,
    /// Learning rate
    learning_rate: f64,
}

impl PriorityOptimizer {
    /// Create new optimizer
    pub fn new() -> Self {
        Self {
            adjustments: BTreeMap::new(),
            history: VecDeque::new(),
            max_history: 10000,
            learning_rate: 0.1,
        }
    }

    /// Suggest priority for process
    pub fn suggest_priority(&mut self, profile: &ProcessProfile, current_priority: i8) -> i8 {
        let mut suggested = current_priority;

        match profile.process_type {
            ProcessType::Interactive => {
                if current_priority < -10 {
                    suggested = -10;
                }
            },
            ProcessType::Batch => {
                if current_priority <= 5 {
                    suggested = 5;
                }
            },
            ProcessType::RealTime => {
                suggested = -20;
            },
            ProcessType::Daemon => {
                suggested = 0;
            },
            _ => {},
        }

        match profile.cpu_profile {
            CpuProfile::CpuBound if profile.avg_cpu_usage > 0.9 => {
                suggested = suggested.saturating_add(2);
            },
            CpuProfile::IoBound => {
                suggested = suggested.saturating_sub(1);
            },
            _ => {},
        }

        suggested = suggested.clamp(-20, 19);

        self.adjustments.insert(profile.pid, PriorityAdjustment {
            current: current_priority,
            suggested,
            confidence: self.calculate_confidence(profile),
            last_adjustment: NexusTimestamp::now(),
        });

        if suggested != current_priority {
            self.history.push_back((profile.pid, current_priority, suggested));
            if self.history.len() > self.max_history {
                self.history.pop_front();
            }
        }

        suggested
    }

    /// Calculate confidence
    fn calculate_confidence(&self, profile: &ProcessProfile) -> f64 {
        (profile.sample_count as f64 / 100.0).min(1.0)
    }

    /// Get adjustment for process
    #[inline(always)]
    pub fn get_adjustment(&self, pid: ProcessId) -> Option<(i8, f64)> {
        self.adjustments.get(&pid).map(|a| (a.suggested, a.confidence))
    }

    /// Record feedback
    #[inline]
    pub fn record_feedback(&mut self, _pid: ProcessId, was_beneficial: bool) {
        if was_beneficial {
            self.learning_rate = (self.learning_rate * 1.1).min(0.5);
        } else {
            self.learning_rate = (self.learning_rate * 0.9).max(0.01);
        }
    }
}

impl Default for PriorityOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
