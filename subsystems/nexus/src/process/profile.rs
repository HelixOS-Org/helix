//! Process Profile
//!
//! Behavioral profiling for processes.

use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;
use crate::math;
use super::{CpuProfile, ProcessId, ProcessMetrics, ProcessType};

/// Process behavioral profile
#[derive(Debug, Clone)]
pub struct ProcessProfile {
    /// Process ID
    pub pid: ProcessId,
    /// Process name
    pub name: String,
    /// Process type
    pub process_type: ProcessType,
    /// CPU profile
    pub cpu_profile: CpuProfile,
    /// Average CPU usage
    pub avg_cpu_usage: f64,
    /// CPU usage variance
    pub cpu_variance: f64,
    /// Average memory usage
    pub avg_memory: f64,
    /// Memory growth rate
    pub memory_growth_rate: f64,
    /// Average I/O rate
    pub avg_io_rate: f64,
    /// I/O pattern (read/write ratio)
    pub io_read_ratio: f64,
    /// Average runtime
    pub avg_runtime: u64,
    /// Wake-up frequency
    pub wakeup_frequency: f64,
    /// Preferred CPUs
    pub preferred_cpus: Vec<u32>,
    /// Total samples
    pub sample_count: u64,
    /// Last update
    pub last_update: NexusTimestamp,
}

impl ProcessProfile {
    /// Create new profile
    pub fn new(pid: ProcessId) -> Self {
        Self {
            pid,
            name: String::new(),
            process_type: ProcessType::Unknown,
            cpu_profile: CpuProfile::Balanced,
            avg_cpu_usage: 0.0,
            cpu_variance: 0.0,
            avg_memory: 0.0,
            memory_growth_rate: 0.0,
            avg_io_rate: 0.0,
            io_read_ratio: 0.5,
            avg_runtime: 0,
            wakeup_frequency: 0.0,
            preferred_cpus: Vec::new(),
            sample_count: 0,
            last_update: NexusTimestamp::now(),
        }
    }

    /// Update profile with new metrics
    pub fn update(&mut self, metrics: &ProcessMetrics, previous: Option<&ProcessMetrics>) {
        self.sample_count += 1;
        self.last_update = NexusTimestamp::now();

        if let Some(prev) = previous {
            let duration = metrics.timestamp.duration_since(prev.timestamp);
            if duration == 0 {
                return;
            }

            // Update CPU usage
            let cpu = metrics.cpu_usage(prev, duration);
            let alpha = 0.1;
            self.avg_cpu_usage = alpha * cpu + (1.0 - alpha) * self.avg_cpu_usage;

            // Update variance
            let diff = cpu - self.avg_cpu_usage;
            self.cpu_variance = alpha * diff * diff + (1.0 - alpha) * self.cpu_variance;

            // Update memory
            self.avg_memory = alpha * metrics.memory_bytes as f64 + (1.0 - alpha) * self.avg_memory;
            let mem_delta = metrics.memory_delta(prev);
            self.memory_growth_rate =
                alpha * mem_delta as f64 + (1.0 - alpha) * self.memory_growth_rate;

            // Update I/O
            let (read_rate, write_rate) = metrics.io_rate(prev, duration);
            let io_rate = read_rate + write_rate;
            self.avg_io_rate = alpha * io_rate + (1.0 - alpha) * self.avg_io_rate;

            if io_rate > 0.0 {
                self.io_read_ratio =
                    alpha * (read_rate / io_rate) + (1.0 - alpha) * self.io_read_ratio;
            }

            // Update CPU profile
            self.update_cpu_profile();

            // Update process type (requires more context)
            if self.sample_count > 100 {
                self.infer_process_type();
            }
        }
    }

    /// Update CPU profile classification
    fn update_cpu_profile(&mut self) {
        let cpu = self.avg_cpu_usage;
        let io = self.avg_io_rate;
        let mem = self.memory_growth_rate.abs();

        if cpu < 0.01 && io < 1000.0 {
            self.cpu_profile = CpuProfile::Idle;
        } else if cpu > 0.7 && io < 10_000.0 {
            self.cpu_profile = CpuProfile::CpuBound;
        } else if io > 100_000.0 && cpu < 0.3 {
            self.cpu_profile = CpuProfile::IoBound;
        } else if mem > 1_000_000.0 {
            self.cpu_profile = CpuProfile::MemoryBound;
        } else {
            self.cpu_profile = CpuProfile::Balanced;
        }
    }

    /// Infer process type from behavior
    fn infer_process_type(&mut self) {
        if self.cpu_variance > 0.1 && self.avg_cpu_usage < 0.5 {
            self.process_type = ProcessType::Interactive;
        } else if self.avg_cpu_usage > 0.8 && self.cpu_variance < 0.05 {
            self.process_type = ProcessType::Batch;
        } else if self.avg_cpu_usage < 0.1 && self.cpu_variance < 0.01 {
            self.process_type = ProcessType::Daemon;
        }
    }

    /// Get expected CPU usage
    pub fn expected_cpu(&self) -> f64 {
        self.avg_cpu_usage
    }

    /// Get expected memory
    pub fn expected_memory(&self) -> u64 {
        self.avg_memory as u64
    }

    /// Is process misbehaving?
    pub fn is_misbehaving(&self, current_cpu: f64, current_memory: u64) -> bool {
        let cpu_std = math::sqrt(self.cpu_variance);
        if current_cpu > self.avg_cpu_usage + 3.0 * cpu_std {
            return true;
        }

        if current_memory as f64 > self.avg_memory * 2.0 {
            return true;
        }

        false
    }
}
