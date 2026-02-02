//! I/O scheduling.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::latency::LatencyPredictor;
use super::pattern::IoPatternAnalyzer;
use super::request::IoRequest;
use super::types::IoPriority;
use crate::core::NexusTimestamp;

// ============================================================================
// SCHEDULING ALGORITHM
// ============================================================================

/// Scheduling algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingAlgorithm {
    /// First-come, first-served
    Fifo,
    /// Deadline-based (like Linux deadline)
    Deadline,
    /// Completely fair queuing
    Cfq,
    /// Budget fair queuing
    Bfq,
    /// AI-optimized scheduling
    AiOptimized,
}

// ============================================================================
// SCHEDULER STATISTICS
// ============================================================================

/// Scheduler statistics
#[derive(Debug, Clone, Default)]
pub struct IoSchedulerStats {
    /// Total requests scheduled
    pub total_scheduled: u64,
    /// Total bytes processed
    pub total_bytes: u64,
    /// Average latency
    pub avg_latency_ns: f64,
    /// Requests merged
    pub merges: u64,
    /// Requests reordered
    pub reorders: u64,
}

// ============================================================================
// I/O SCHEDULER
// ============================================================================

/// Intelligent I/O scheduler
pub struct IoScheduler {
    /// Pending requests per device
    queues: BTreeMap<u32, Vec<IoRequest>>,
    /// Latency predictor
    latency_predictor: LatencyPredictor,
    /// Pattern analyzers per process
    pattern_analyzers: BTreeMap<u64, IoPatternAnalyzer>,
    /// Scheduling algorithm
    algorithm: SchedulingAlgorithm,
    /// Statistics
    stats: IoSchedulerStats,
}

impl IoScheduler {
    /// Create new scheduler
    pub fn new(algorithm: SchedulingAlgorithm) -> Self {
        Self {
            queues: BTreeMap::new(),
            latency_predictor: LatencyPredictor::new(),
            pattern_analyzers: BTreeMap::new(),
            algorithm,
            stats: IoSchedulerStats::default(),
        }
    }

    /// Submit request
    pub fn submit(&mut self, request: IoRequest) {
        let device_id = request.device_id;
        let process_id = request.process_id;

        // Record pattern
        let analyzer = self
            .pattern_analyzers
            .entry(process_id)
            .or_insert_with(IoPatternAnalyzer::default);
        analyzer.record(request.offset, request.size, request.is_read());

        // Add to queue
        let queue = self.queues.entry(device_id).or_default();

        // Try to merge with existing request
        if Self::try_merge_static(queue, &request).is_some() {
            self.stats.merges += 1;
        } else {
            queue.push(request);
        }
    }

    /// Try to merge request with existing (static version)
    fn try_merge_static(queue: &mut Vec<IoRequest>, new: &IoRequest) -> Option<usize> {
        for (i, existing) in queue.iter_mut().enumerate() {
            // Same device and operation type
            if existing.device_id != new.device_id || existing.op_type != new.op_type {
                continue;
            }

            // Adjacent or overlapping
            if existing.end_offset() == new.offset {
                // Extend existing request
                existing.size += new.size;
                return Some(i);
            }

            if new.end_offset() == existing.offset {
                // Prepend to existing
                existing.offset = new.offset;
                existing.size += new.size;
                return Some(i);
            }
        }

        None
    }

    /// Get next request to dispatch
    pub fn dispatch(&mut self, device_id: u32) -> Option<IoRequest> {
        let queue = self.queues.get(&device_id)?;
        if queue.is_empty() {
            return None;
        }

        let idx = match self.algorithm {
            SchedulingAlgorithm::Fifo => 0,
            SchedulingAlgorithm::Deadline => Self::select_deadline_static(queue)?,
            SchedulingAlgorithm::Cfq => Self::select_cfq_static(queue)?,
            SchedulingAlgorithm::Bfq => Self::select_bfq_static(queue)?,
            SchedulingAlgorithm::AiOptimized => Self::select_ai_optimized_static(queue)?,
        };

        let queue = self.queues.get_mut(&device_id)?;
        let request = queue.remove(idx);
        self.stats.total_scheduled += 1;
        self.stats.total_bytes += request.size as u64;

        Some(request)
    }

    /// Deadline scheduling (static)
    fn select_deadline_static(queue: &[IoRequest]) -> Option<usize> {
        // Find request with earliest deadline (oldest submission + priority)
        queue
            .iter()
            .enumerate()
            .min_by_key(|(_, r)| {
                let priority_factor = match r.priority {
                    IoPriority::RealTime => 0,
                    IoPriority::High => 1,
                    IoPriority::Normal => 2,
                    IoPriority::Low => 3,
                    IoPriority::Idle => 4,
                };
                (priority_factor, r.submitted_at.raw())
            })
            .map(|(i, _)| i)
    }

    /// CFQ-like scheduling (static)
    fn select_cfq_static(queue: &[IoRequest]) -> Option<usize> {
        // Round-robin by process with priority boost
        // Simplified: just use priority + age
        Self::select_deadline_static(queue)
    }

    /// BFQ-like scheduling (static)
    fn select_bfq_static(queue: &[IoRequest]) -> Option<usize> {
        // Budget-based fair queuing
        // Simplified: prefer smaller requests for interactivity
        queue
            .iter()
            .enumerate()
            .min_by_key(|(_, r)| {
                let size_factor = r.size / 4096;
                let priority_factor = match r.priority {
                    IoPriority::RealTime => 0,
                    IoPriority::High => size_factor,
                    IoPriority::Normal => size_factor * 2,
                    IoPriority::Low => size_factor * 4,
                    IoPriority::Idle => size_factor * 8,
                };
                priority_factor
            })
            .map(|(i, _)| i)
    }

    /// AI-optimized scheduling (static)
    fn select_ai_optimized_static(queue: &[IoRequest]) -> Option<usize> {
        // Simplified version without needing self
        // Consider priority and age only
        let mut best_idx = 0;
        let mut best_score = f64::NEG_INFINITY;

        for (i, request) in queue.iter().enumerate() {
            let mut score = 0.0;

            // Priority boost
            score += (request.priority as u32 as f64) * 100.0;

            // Age factor (older requests get priority)
            let now = NexusTimestamp::now();
            let age = now.duration_since(request.submitted_at) as f64;
            score += age / 1_000_000.0; // Normalize

            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        Some(best_idx)
    }

    /// Record completion
    pub fn complete(&mut self, request: &IoRequest) {
        if let Some(latency) = request.latency() {
            // Update latency predictor
            self.latency_predictor
                .record(request.device_id, request.size, latency);

            // Update average
            let count = self.stats.total_scheduled as f64;
            if count > 0.0 {
                self.stats.avg_latency_ns =
                    (self.stats.avg_latency_ns * (count - 1.0) + latency as f64) / count;
            }
        }
    }

    /// Get queue depth
    pub fn queue_depth(&self, device_id: u32) -> usize {
        self.queues.get(&device_id).map(|q| q.len()).unwrap_or(0)
    }

    /// Set algorithm
    pub fn set_algorithm(&mut self, algorithm: SchedulingAlgorithm) {
        self.algorithm = algorithm;
    }

    /// Get statistics
    pub fn stats(&self) -> &IoSchedulerStats {
        &self.stats
    }

    /// Get latency predictor
    pub fn latency_predictor(&self) -> &LatencyPredictor {
        &self.latency_predictor
    }

    /// Get mutable latency predictor
    pub fn latency_predictor_mut(&mut self) -> &mut LatencyPredictor {
        &mut self.latency_predictor
    }
}

impl Default for IoScheduler {
    fn default() -> Self {
        Self::new(SchedulingAlgorithm::AiOptimized)
    }
}
