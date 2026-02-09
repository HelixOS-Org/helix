//! Work Queue Intelligence
//!
//! This module provides comprehensive analysis and optimization for work queues.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    WorkQueueId, WorkId, WorkQueueType,
    WorkInfo, WorkQueueInfo,
    QueueDepthPredictor,
    WorkStealingOptimizer, StealTarget,
    WorkLatencyAnalyzer, LatencyStats, LatencyTrend,
    PowerAwareWorkScheduler, PowerSchedulingDecision,
    WorkDependencyTracker, DependencyType,
    CpuId,
};

/// Work queue analysis result
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WorkQueueAnalysis {
    /// Queue ID
    pub queue_id: WorkQueueId,
    /// Current health score (0-100)
    pub health_score: f32,
    /// Bottleneck detected
    pub bottleneck: Option<WorkQueueBottleneck>,
    /// Recommended actions
    pub recommendations: Vec<WorkQueueRecommendation>,
    /// Predicted issues
    pub predictions: Vec<WorkQueuePrediction>,
}

/// Work queue bottleneck type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkQueueBottleneck {
    /// Queue depth too high
    QueueDepth,
    /// Worker starvation
    WorkerStarvation,
    /// Work stealing inefficiency
    StealingInefficiency,
    /// Lock contention
    LockContention,
    /// Memory pressure
    MemoryPressure,
    /// CPU saturation
    CpuSaturation,
    /// Latency violation
    LatencyViolation,
}

/// Recommendation for work queue
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WorkQueueRecommendation {
    /// Action to take
    pub action: WorkQueueAction,
    /// Priority (1-10)
    pub priority: u8,
    /// Expected improvement
    pub expected_improvement: f32,
    /// Reason
    pub reason: String,
}

/// Work queue action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkQueueAction {
    /// Increase worker count
    IncreaseWorkers,
    /// Decrease worker count
    DecreaseWorkers,
    /// Enable work stealing
    EnableStealing,
    /// Disable work stealing
    DisableStealing,
    /// Increase batch size
    IncreaseBatchSize,
    /// Decrease batch size
    DecreaseBatchSize,
    /// Enable power saving
    EnablePowerSaving,
    /// Disable power saving
    DisablePowerSaving,
    /// Rebalance work
    RebalanceWork,
}

/// Predicted issue
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WorkQueuePrediction {
    /// Issue type
    pub issue: WorkQueueBottleneck,
    /// Probability (0-1)
    pub probability: f32,
    /// Time until issue (nanoseconds)
    pub time_until_ns: u64,
    /// Recommended preventive action
    pub preventive_action: Option<WorkQueueAction>,
}

/// Work Queue Intelligence - comprehensive analysis and optimization
#[repr(align(64))]
pub struct WorkQueueIntelligence {
    /// Queue information
    queues: BTreeMap<WorkQueueId, WorkQueueInfo>,
    /// Depth predictors per queue
    depth_predictors: BTreeMap<WorkQueueId, QueueDepthPredictor>,
    /// Latency analyzers per queue
    latency_analyzers: BTreeMap<WorkQueueId, WorkLatencyAnalyzer>,
    /// Work stealing optimizer
    stealing_optimizer: WorkStealingOptimizer,
    /// Power-aware scheduler
    power_scheduler: PowerAwareWorkScheduler,
    /// Dependency tracker
    dependency_tracker: WorkDependencyTracker,
    /// Total work items processed
    total_processed: AtomicU64,
    /// Total work items failed
    total_failed: AtomicU64,
    /// Analysis interval (nanoseconds)
    analysis_interval_ns: u64,
    /// Last analysis timestamp
    last_analysis: u64,
}

impl WorkQueueIntelligence {
    /// Create new work queue intelligence
    pub fn new() -> Self {
        Self {
            queues: BTreeMap::new(),
            depth_predictors: BTreeMap::new(),
            latency_analyzers: BTreeMap::new(),
            stealing_optimizer: WorkStealingOptimizer::new(),
            power_scheduler: PowerAwareWorkScheduler::new(),
            dependency_tracker: WorkDependencyTracker::new(),
            total_processed: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
            analysis_interval_ns: 1_000_000_000, // 1 second
            last_analysis: 0,
        }
    }

    /// Register work queue
    #[inline]
    pub fn register_queue(
        &mut self,
        queue_id: WorkQueueId,
        name: String,
        queue_type: WorkQueueType,
    ) {
        let info = WorkQueueInfo::new(queue_id, name, queue_type);
        self.queues.insert(queue_id, info);
        self.depth_predictors
            .insert(queue_id, QueueDepthPredictor::new(queue_id));
        self.latency_analyzers
            .insert(queue_id, WorkLatencyAnalyzer::new(queue_id));
    }

    /// Record work enqueued
    #[inline]
    pub fn record_enqueue(&mut self, queue_id: WorkQueueId, _work: &WorkInfo) {
        if let Some(info) = self.queues.get_mut(&queue_id) {
            info.pending_count += 1;
            if info.pending_count > info.peak_queue_depth {
                info.peak_queue_depth = info.pending_count;
            }
        }
    }

    /// Record work started
    #[inline]
    pub fn record_work_started(&mut self, queue_id: WorkQueueId, _work_id: WorkId) {
        if let Some(info) = self.queues.get_mut(&queue_id) {
            info.pending_count = info.pending_count.saturating_sub(1);
            info.active_workers += 1;
        }
    }

    /// Record work completed
    pub fn record_work_completed(
        &mut self,
        queue_id: WorkQueueId,
        work_id: WorkId,
        latency_ns: u64,
        success: bool,
    ) {
        if let Some(info) = self.queues.get_mut(&queue_id) {
            info.active_workers = info.active_workers.saturating_sub(1);
            if success {
                info.processed_count += 1;
                self.total_processed.fetch_add(1, Ordering::Relaxed);
            } else {
                info.failed_count += 1;
                self.total_failed.fetch_add(1, Ordering::Relaxed);
            }

            // Update average processing time
            let total = info.processed_count + info.failed_count;
            if total > 0 {
                let alpha = 0.1;
                info.avg_processing_time_ns = (alpha * latency_ns as f64
                    + (1.0 - alpha) * info.avg_processing_time_ns as f64)
                    as u64;
            }
            if latency_ns > info.peak_processing_time_ns {
                info.peak_processing_time_ns = latency_ns;
            }
        }

        // Record latency
        if let Some(analyzer) = self.latency_analyzers.get_mut(&queue_id) {
            analyzer.record_latency(latency_ns);
        }

        // Mark dependencies satisfied
        self.dependency_tracker.mark_completed(work_id);
    }

    /// Record queue depth sample
    #[inline]
    pub fn record_depth_sample(
        &mut self,
        queue_id: WorkQueueId,
        timestamp: u64,
        depth: u64,
        arrival_rate: f32,
        processing_rate: f32,
    ) {
        if let Some(predictor) = self.depth_predictors.get_mut(&queue_id) {
            predictor.record_sample(timestamp, depth, arrival_rate, processing_rate);
        }
    }

    /// Analyze work queue
    pub fn analyze_queue(&self, queue_id: WorkQueueId) -> Option<WorkQueueAnalysis> {
        let info = self.queues.get(&queue_id)?;

        let mut health_score = 100.0f32;
        let mut bottleneck = None;
        let mut recommendations = Vec::new();
        let mut predictions = Vec::new();

        // Check queue depth
        if info.pending_count > 100 {
            health_score -= 20.0;
            bottleneck = Some(WorkQueueBottleneck::QueueDepth);
            recommendations.push(WorkQueueRecommendation {
                action: WorkQueueAction::IncreaseWorkers,
                priority: 8,
                expected_improvement: 30.0,
                reason: String::from("High queue depth detected"),
            });
        }

        // Check worker utilization
        let utilization = info.utilization();
        if utilization > 0.9 {
            health_score -= 15.0;
            if bottleneck.is_none() {
                bottleneck = Some(WorkQueueBottleneck::CpuSaturation);
            }
        } else if utilization < 0.1 && info.pending_count > 0 {
            health_score -= 10.0;
            if bottleneck.is_none() {
                bottleneck = Some(WorkQueueBottleneck::WorkerStarvation);
            }
        }

        // Check failure rate
        let failure_rate = info.failure_rate();
        if failure_rate > 0.1 {
            health_score -= failure_rate * 50.0;
        }

        // Check latency
        if let Some(analyzer) = self.latency_analyzers.get(&queue_id) {
            let stats = analyzer.calculate_stats();
            let trend = analyzer.detect_trend();

            if stats.p99_ns > 100_000_000 {
                // 100ms
                health_score -= 25.0;
                if bottleneck.is_none() {
                    bottleneck = Some(WorkQueueBottleneck::LatencyViolation);
                }
            }

            if trend == LatencyTrend::Increasing {
                predictions.push(WorkQueuePrediction {
                    issue: WorkQueueBottleneck::LatencyViolation,
                    probability: 0.7,
                    time_until_ns: 60_000_000_000, // 1 minute
                    preventive_action: Some(WorkQueueAction::IncreaseWorkers),
                });
            }
        }

        // Predict overflow
        if let Some(predictor) = self.depth_predictors.get(&queue_id) {
            if let Some(overflow_ns) =
                predictor.predict_overflow(info.pending_count, 1000, 60_000_000_000)
            {
                predictions.push(WorkQueuePrediction {
                    issue: WorkQueueBottleneck::QueueDepth,
                    probability: 0.8,
                    time_until_ns: overflow_ns,
                    preventive_action: Some(WorkQueueAction::IncreaseWorkers),
                });
            }
        }

        health_score = health_score.max(0.0);

        Some(WorkQueueAnalysis {
            queue_id,
            health_score,
            bottleneck,
            recommendations,
            predictions,
        })
    }

    /// Schedule work with intelligence
    #[inline(always)]
    pub fn schedule_work(&mut self, work: &WorkInfo, current_time: u64) -> PowerSchedulingDecision {
        self.power_scheduler.schedule_work(work, current_time, None)
    }

    /// Find steal target
    #[inline(always)]
    pub fn find_steal_target(&self, cpu_id: CpuId) -> Option<StealTarget> {
        self.stealing_optimizer.find_steal_target(cpu_id)
    }

    /// Add work dependency
    #[inline(always)]
    pub fn add_dependency(
        &mut self,
        source: WorkId,
        target: WorkId,
        dep_type: DependencyType,
    ) -> bool {
        self.dependency_tracker
            .add_dependency(source, target, dep_type)
    }

    /// Check if work is ready
    #[inline(always)]
    pub fn is_work_ready(&self, work_id: WorkId) -> bool {
        self.dependency_tracker.is_ready(work_id)
    }

    /// Get queue count
    #[inline(always)]
    pub fn queue_count(&self) -> usize {
        self.queues.len()
    }

    /// Get total processed count
    #[inline(always)]
    pub fn total_processed(&self) -> u64 {
        self.total_processed.load(Ordering::Relaxed)
    }

    /// Get total failed count
    #[inline(always)]
    pub fn total_failed(&self) -> u64 {
        self.total_failed.load(Ordering::Relaxed)
    }

    /// Get work stealing optimizer
    #[inline(always)]
    pub fn stealing_optimizer(&self) -> &WorkStealingOptimizer {
        &self.stealing_optimizer
    }

    /// Get work stealing optimizer mutably
    #[inline(always)]
    pub fn stealing_optimizer_mut(&mut self) -> &mut WorkStealingOptimizer {
        &mut self.stealing_optimizer
    }

    /// Get power scheduler
    #[inline(always)]
    pub fn power_scheduler(&self) -> &PowerAwareWorkScheduler {
        &self.power_scheduler
    }

    /// Get power scheduler mutably
    #[inline(always)]
    pub fn power_scheduler_mut(&mut self) -> &mut PowerAwareWorkScheduler {
        &mut self.power_scheduler
    }

    /// Get dependency tracker
    #[inline(always)]
    pub fn dependency_tracker(&self) -> &WorkDependencyTracker {
        &self.dependency_tracker
    }

    /// Get dependency tracker mutably
    #[inline(always)]
    pub fn dependency_tracker_mut(&mut self) -> &mut WorkDependencyTracker {
        &mut self.dependency_tracker
    }

    /// Get queue info
    #[inline(always)]
    pub fn get_queue(&self, queue_id: WorkQueueId) -> Option<&WorkQueueInfo> {
        self.queues.get(&queue_id)
    }

    /// Get latency stats for queue
    #[inline]
    pub fn get_latency_stats(&self, queue_id: WorkQueueId) -> Option<LatencyStats> {
        self.latency_analyzers
            .get(&queue_id)
            .map(|a| a.calculate_stats())
    }

    /// Perform periodic maintenance
    pub fn periodic_maintenance(&mut self, current_time: u64) {
        if current_time - self.last_analysis < self.analysis_interval_ns {
            return;
        }
        self.last_analysis = current_time;

        // Cleanup completed dependencies
        self.dependency_tracker.cleanup_completed();

        // Reset energy counter periodically
        self.power_scheduler.reset_energy(current_time);
    }
}

impl Default for WorkQueueIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
