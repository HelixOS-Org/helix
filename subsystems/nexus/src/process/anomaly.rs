//! Process Anomaly Detector
//!
//! Detects abnormal process behavior.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;
use super::{ProcessId, ProcessProfile, ProcessType};

/// Baseline for process type
#[derive(Debug, Clone)]
struct TypeBaseline {
    /// Average CPU
    avg_cpu: f64,
    /// Average memory
    avg_memory: f64,
    /// Sample count
    samples: u64,
}

/// Process anomaly
#[derive(Debug, Clone)]
pub struct ProcessAnomaly {
    /// Anomaly type
    pub anomaly_type: ProcessAnomalyType,
    /// Affected process
    pub pid: ProcessId,
    /// Severity (0.0 - 1.0)
    pub severity: f64,
    /// Detection time
    pub detected_at: NexusTimestamp,
    /// Description
    pub description: String,
}

/// Types of process anomalies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessAnomalyType {
    /// Excessive CPU usage
    ExcessiveCpu,
    /// Memory leak suspected
    MemoryLeak,
    /// Runaway process
    Runaway,
    /// Zombie accumulation
    ZombieAccumulation,
    /// Fork bomb detection
    ForkBomb,
    /// Unusual syscall pattern
    UnusualSyscalls,
    /// Resource exhaustion
    ResourceExhaustion,
}

/// Detects process anomalies
pub struct ProcessAnomalyDetector {
    /// Baseline CPU per process type
    type_baselines: BTreeMap<ProcessType, TypeBaseline>,
    /// Detected anomalies
    anomalies: Vec<ProcessAnomaly>,
    /// Max anomalies
    max_anomalies: usize,
}

impl ProcessAnomalyDetector {
    /// Create new detector
    pub fn new() -> Self {
        let mut baselines = BTreeMap::new();

        baselines.insert(ProcessType::Interactive, TypeBaseline {
            avg_cpu: 0.05,
            avg_memory: 100_000_000.0,
            samples: 0,
        });
        baselines.insert(ProcessType::Batch, TypeBaseline {
            avg_cpu: 0.8,
            avg_memory: 500_000_000.0,
            samples: 0,
        });
        baselines.insert(ProcessType::Daemon, TypeBaseline {
            avg_cpu: 0.02,
            avg_memory: 50_000_000.0,
            samples: 0,
        });
        baselines.insert(ProcessType::System, TypeBaseline {
            avg_cpu: 0.1,
            avg_memory: 200_000_000.0,
            samples: 0,
        });

        Self {
            type_baselines: baselines,
            anomalies: Vec::new(),
            max_anomalies: 1000,
        }
    }

    /// Check process for anomalies
    pub fn check(&mut self, profile: &ProcessProfile) -> Option<ProcessAnomaly> {
        let baseline = self
            .type_baselines
            .get(&profile.process_type)
            .or_else(|| self.type_baselines.get(&ProcessType::Daemon))?;

        // Check for excessive CPU
        if profile.avg_cpu_usage > baseline.avg_cpu * 5.0 && profile.avg_cpu_usage > 0.9 {
            return Some(self.record_anomaly(
                ProcessAnomalyType::ExcessiveCpu,
                profile.pid,
                (profile.avg_cpu_usage / baseline.avg_cpu - 1.0).min(1.0),
                format!(
                    "Excessive CPU: {:.1}% (expected: {:.1}%)",
                    profile.avg_cpu_usage * 100.0,
                    baseline.avg_cpu * 100.0
                ),
            ));
        }

        // Check for memory leak
        if profile.memory_growth_rate > 1_000_000.0 && profile.sample_count > 1000 {
            return Some(self.record_anomaly(
                ProcessAnomalyType::MemoryLeak,
                profile.pid,
                (profile.memory_growth_rate / 10_000_000.0).min(1.0),
                format!(
                    "Memory leak suspected: growing at {:.1} MB/sample",
                    profile.memory_growth_rate / 1_000_000.0
                ),
            ));
        }

        None
    }

    /// Check for fork bomb
    pub fn check_fork_bomb(
        &mut self,
        pid: ProcessId,
        child_count: u32,
        spawn_rate: f64,
    ) -> Option<ProcessAnomaly> {
        if child_count > 100 && spawn_rate > 10.0 {
            return Some(self.record_anomaly(
                ProcessAnomalyType::ForkBomb,
                pid,
                1.0,
                format!(
                    "Fork bomb detected: {} children, {:.1} spawns/sec",
                    child_count,
                    spawn_rate
                ),
            ));
        }

        None
    }

    /// Record anomaly
    fn record_anomaly(
        &mut self,
        anomaly_type: ProcessAnomalyType,
        pid: ProcessId,
        severity: f64,
        description: String,
    ) -> ProcessAnomaly {
        let anomaly = ProcessAnomaly {
            anomaly_type,
            pid,
            severity,
            detected_at: NexusTimestamp::now(),
            description,
        };

        self.anomalies.push(anomaly.clone());
        if self.anomalies.len() > self.max_anomalies {
            self.anomalies.remove(0);
        }

        anomaly
    }

    /// Get recent anomalies
    pub fn recent_anomalies(&self, n: usize) -> &[ProcessAnomaly] {
        let start = self.anomalies.len().saturating_sub(n);
        &self.anomalies[start..]
    }
}

impl Default for ProcessAnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}
