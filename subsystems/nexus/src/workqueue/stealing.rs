//! Work Stealing Optimizer
//!
//! This module provides NUMA-aware work stealing optimization for load balancing.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use super::CpuId;

/// Work stealing statistics per CPU
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StealingStats {
    /// CPU ID
    pub cpu_id: CpuId,
    /// Total steal attempts
    pub steal_attempts: u64,
    /// Successful steals
    pub successful_steals: u64,
    /// Failed steals (empty queue)
    pub failed_steals: u64,
    /// Items stolen from this CPU
    pub items_stolen: u64,
    /// Average steal latency (nanoseconds)
    pub avg_steal_latency_ns: u64,
    /// Current local queue depth
    pub local_queue_depth: u64,
    /// CPU idle time (nanoseconds)
    pub idle_time_ns: u64,
    /// CPU busy time (nanoseconds)
    pub busy_time_ns: u64,
}

impl StealingStats {
    /// Create new stealing stats
    pub fn new(cpu_id: CpuId) -> Self {
        Self {
            cpu_id,
            steal_attempts: 0,
            successful_steals: 0,
            failed_steals: 0,
            items_stolen: 0,
            avg_steal_latency_ns: 0,
            local_queue_depth: 0,
            idle_time_ns: 0,
            busy_time_ns: 0,
        }
    }

    /// Calculate steal success rate
    #[inline]
    pub fn steal_success_rate(&self) -> f32 {
        if self.steal_attempts == 0 {
            return 0.0;
        }
        self.successful_steals as f32 / self.steal_attempts as f32
    }

    /// Calculate CPU utilization
    #[inline]
    pub fn cpu_utilization(&self) -> f32 {
        let total = self.idle_time_ns + self.busy_time_ns;
        if total == 0 {
            return 0.0;
        }
        self.busy_time_ns as f32 / total as f32
    }
}

/// Victim selection for work stealing
#[derive(Debug, Clone, Copy)]
pub struct StealTarget {
    /// Target CPU ID
    pub cpu_id: CpuId,
    /// Estimated items to steal
    pub items_to_steal: u32,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Expected benefit (work units)
    pub expected_benefit: f32,
}

/// Work stealing optimizer
#[repr(align(64))]
pub struct WorkStealingOptimizer {
    /// Per-CPU statistics
    stats: BTreeMap<CpuId, StealingStats>,
    /// Stealing affinity matrix (from -> to -> success_rate)
    affinity_matrix: BTreeMap<(CpuId, CpuId), f32>,
    /// NUMA-aware stealing preference
    numa_preference: BTreeMap<CpuId, Vec<CpuId>>,
    /// Minimum work to consider stealing
    steal_threshold: u64,
    /// Maximum items to steal at once
    max_steal_batch: u32,
    /// Global steal attempts counter
    global_steal_attempts: AtomicU64,
    /// Global successful steals counter
    global_successful_steals: AtomicU64,
}

impl WorkStealingOptimizer {
    /// Create new work stealing optimizer
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
            affinity_matrix: BTreeMap::new(),
            numa_preference: BTreeMap::new(),
            steal_threshold: 2,
            max_steal_batch: 4,
            global_steal_attempts: AtomicU64::new(0),
            global_successful_steals: AtomicU64::new(0),
        }
    }

    /// Register CPU for work stealing
    #[inline]
    pub fn register_cpu(&mut self, cpu_id: CpuId) {
        self.stats
            .entry(cpu_id)
            .or_insert_with(|| StealingStats::new(cpu_id));
    }

    /// Set NUMA preference for CPU
    #[inline(always)]
    pub fn set_numa_preference(&mut self, cpu_id: CpuId, preferred_cpus: Vec<CpuId>) {
        self.numa_preference.insert(cpu_id, preferred_cpus);
    }

    /// Update local queue depth for CPU
    #[inline]
    pub fn update_queue_depth(&mut self, cpu_id: CpuId, depth: u64) {
        if let Some(stats) = self.stats.get_mut(&cpu_id) {
            stats.local_queue_depth = depth;
        }
    }

    /// Record steal attempt
    pub fn record_steal_attempt(
        &mut self,
        from_cpu: CpuId,
        to_cpu: CpuId,
        success: bool,
        latency_ns: u64,
    ) {
        self.global_steal_attempts.fetch_add(1, Ordering::Relaxed);

        // Update from_cpu stats (victim)
        if let Some(stats) = self.stats.get_mut(&from_cpu) {
            if success {
                stats.items_stolen += 1;
            }
        }

        // Update to_cpu stats (thief)
        if let Some(stats) = self.stats.get_mut(&to_cpu) {
            stats.steal_attempts += 1;
            if success {
                stats.successful_steals += 1;
                self.global_successful_steals
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                stats.failed_steals += 1;
            }
            // Update average latency with exponential moving average
            let alpha = 0.1;
            stats.avg_steal_latency_ns = (alpha * latency_ns as f64
                + (1.0 - alpha) * stats.avg_steal_latency_ns as f64)
                as u64;
        }

        // Update affinity matrix
        let key = (from_cpu, to_cpu);
        let current = self.affinity_matrix.get(&key).copied().unwrap_or(0.5);
        let new_value = if success {
            current * 0.9 + 0.1 * 1.0
        } else {
            current * 0.9 + 0.1 * 0.0
        };
        self.affinity_matrix.insert(key, new_value);
    }

    /// Find best victim for work stealing
    pub fn find_steal_target(&self, thief_cpu: CpuId) -> Option<StealTarget> {
        let mut best_target: Option<StealTarget> = None;

        // Get NUMA-local CPUs first
        let numa_cpus = self.numa_preference.get(&thief_cpu);

        for (cpu_id, stats) in &self.stats {
            if *cpu_id == thief_cpu {
                continue;
            }

            // Skip if queue is below threshold
            if stats.local_queue_depth < self.steal_threshold {
                continue;
            }

            // Calculate score based on queue depth and affinity
            let affinity = self
                .affinity_matrix
                .get(&(*cpu_id, thief_cpu))
                .copied()
                .unwrap_or(0.5);

            // NUMA bonus
            let numa_bonus = if let Some(preferred) = numa_cpus {
                if preferred.contains(cpu_id) { 1.2 } else { 1.0 }
            } else {
                1.0
            };

            let score = (stats.local_queue_depth as f32) * affinity * numa_bonus;
            let items_to_steal = ((stats.local_queue_depth / 2) as u32).min(self.max_steal_batch);

            let target = StealTarget {
                cpu_id: *cpu_id,
                items_to_steal,
                confidence: affinity,
                expected_benefit: score,
            };

            if best_target
                .as_ref()
                .map_or(true, |t| target.expected_benefit > t.expected_benefit)
            {
                best_target = Some(target);
            }
        }

        best_target
    }

    /// Calculate work imbalance across CPUs
    pub fn calculate_imbalance(&self) -> f32 {
        if self.stats.is_empty() {
            return 0.0;
        }

        let depths: Vec<u64> = self.stats.values().map(|s| s.local_queue_depth).collect();
        let mean = depths.iter().sum::<u64>() as f32 / depths.len() as f32;

        if mean == 0.0 {
            return 0.0;
        }

        let variance = depths
            .iter()
            .map(|d| {
                let diff = *d as f32 - mean;
                diff * diff
            })
            .sum::<f32>()
            / depths.len() as f32;

        libm::sqrtf(variance) / mean // Coefficient of variation
    }

    /// Get global steal success rate
    #[inline]
    pub fn global_steal_success_rate(&self) -> f32 {
        let attempts = self.global_steal_attempts.load(Ordering::Relaxed);
        if attempts == 0 {
            return 0.0;
        }
        let successes = self.global_successful_steals.load(Ordering::Relaxed);
        successes as f32 / attempts as f32
    }

    /// Get stats for CPU
    #[inline(always)]
    pub fn get_stats(&self, cpu_id: CpuId) -> Option<&StealingStats> {
        self.stats.get(&cpu_id)
    }

    /// Get CPU count
    #[inline(always)]
    pub fn cpu_count(&self) -> usize {
        self.stats.len()
    }
}

impl Default for WorkStealingOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
