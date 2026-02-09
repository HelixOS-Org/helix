//! Behavioral profiling and anomaly detection.

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

// ============================================================================
// BEHAVIORAL PROFILE
// ============================================================================

/// Process behavioral profile
#[derive(Debug, Clone, Default)]
pub struct BehavioralProfile {
    /// Process ID
    pub process_id: u64,
    /// Normal syscall frequencies
    pub syscall_baseline: ArrayMap<f64, 32>,
    /// Normal memory usage
    pub memory_baseline: MemoryBaseline,
    /// Normal file access patterns
    pub file_baseline: FileBaseline,
    /// Normal network patterns
    pub network_baseline: NetworkBaseline,
    /// Profile creation time
    pub created_at: u64,
    /// Last update time
    pub updated_at: u64,
    /// Training samples
    pub samples: u64,
}

/// Memory usage baseline
#[derive(Debug, Clone, Default)]
pub struct MemoryBaseline {
    /// Average heap size
    pub avg_heap_size: u64,
    /// Average stack usage
    pub avg_stack_usage: u64,
    /// Normal allocation rate
    pub alloc_rate: f64,
    /// Normal deallocation rate
    pub dealloc_rate: f64,
    /// Standard deviation of heap size
    pub heap_std_dev: f64,
}

/// File access baseline
#[derive(Debug, Clone, Default)]
pub struct FileBaseline {
    /// Common file paths accessed
    pub common_paths: Vec<u64>, // Path hashes
    /// Average read rate
    pub read_rate: f64,
    /// Average write rate
    pub write_rate: f64,
    /// Normal open file count
    pub avg_open_files: u32,
}

/// Network activity baseline
#[derive(Debug, Clone, Default)]
pub struct NetworkBaseline {
    /// Common destination IPs (hashes)
    pub common_destinations: Vec<u64>,
    /// Average packet rate (in)
    pub avg_packet_rate_in: f64,
    /// Average packet rate (out)
    pub avg_packet_rate_out: f64,
    /// Average bandwidth in
    pub avg_bandwidth_in: u64,
    /// Average bandwidth out
    pub avg_bandwidth_out: u64,
}

impl BehavioralProfile {
    /// Create new profile
    pub fn new(process_id: u64) -> Self {
        let now = NexusTimestamp::now().raw();
        Self {
            process_id,
            created_at: now,
            updated_at: now,
            ..Default::default()
        }
    }

    /// Update syscall baseline
    #[inline]
    pub fn update_syscall(&mut self, syscall_num: u32, count: u64, total_calls: u64) {
        let freq = count as f64 / total_calls as f64;
        let entry = self.syscall_baseline.entry(syscall_num).or_insert(freq);
        // Exponential moving average
        *entry = 0.9 * *entry + 0.1 * freq;
        self.samples += 1;
        self.updated_at = NexusTimestamp::now().raw();
    }

    /// Check if syscall pattern is anomalous
    #[inline]
    pub fn is_syscall_anomalous(&self, syscall_num: u32, current_freq: f64) -> bool {
        if let Some(&baseline_freq) = self.syscall_baseline.try_get(syscall_num as usize) {
            // Anomalous if more than 3x baseline
            current_freq > baseline_freq * 3.0
        } else {
            // Never seen this syscall before
            self.samples > 100 && current_freq > 0.1
        }
    }

    /// Check if memory usage is anomalous
    #[inline]
    pub fn is_memory_anomalous(&self, current_heap: u64) -> bool {
        if self.samples < 100 {
            return false;
        }

        let deviation = (current_heap as f64 - self.memory_baseline.avg_heap_size as f64).abs();
        deviation > self.memory_baseline.heap_std_dev * 3.0
    }

    /// Get anomaly score (0.0 = normal, 1.0 = very anomalous)
    pub fn anomaly_score(&self, current: &CurrentBehavior) -> f64 {
        if self.samples < 50 {
            return 0.0; // Not enough data
        }

        let mut score = 0.0;
        let mut factors = 0;

        // Check syscall pattern
        for (syscall, &current_freq) in &current.syscall_freq {
            if let Some(&baseline) = self.syscall_baseline.get(syscall) {
                if baseline > 0.0 {
                    let ratio = current_freq / baseline;
                    if ratio > 3.0 {
                        score += ((ratio - 3.0) / 10.0).min(1.0) * 0.2;
                        factors += 1;
                    }
                }
            }
        }

        // Check memory
        if self.memory_baseline.avg_heap_size > 0 {
            let ratio = current.heap_size as f64 / self.memory_baseline.avg_heap_size as f64;
            if ratio > 2.0 {
                score += ((ratio - 2.0) / 5.0).min(1.0) * 0.3;
                factors += 1;
            }
        }

        // Check network
        if self.network_baseline.avg_bandwidth_out > 0 {
            let ratio =
                current.bandwidth_out as f64 / self.network_baseline.avg_bandwidth_out as f64;
            if ratio > 5.0 {
                score += ((ratio - 5.0) / 10.0).min(1.0) * 0.3;
                factors += 1;
            }
        }

        if factors > 0 {
            (score / factors as f64).min(1.0)
        } else {
            0.0
        }
    }
}

/// Current behavior snapshot
#[derive(Debug, Clone, Default)]
pub struct CurrentBehavior {
    /// Syscall frequencies
    pub syscall_freq: ArrayMap<f64, 32>,
    /// Current heap size
    pub heap_size: u64,
    /// Current bandwidth out
    pub bandwidth_out: u64,
    /// Open files
    pub open_files: u32,
}
