//! # Global Syscall Optimizer
//!
//! System-wide optimization of syscall processing based on:
//! - Global workload analysis
//! - Cross-process optimization opportunities
//! - Resource contention detection
//! - Adaptive tuning of all subsystems
//! - Cost-benefit analysis for optimizations

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// OPTIMIZATION TYPES
// ============================================================================

/// Type of optimization opportunity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationType {
    /// Merge syscalls across processes
    CrossProcessMerge,
    /// Batch I/O across processes
    GlobalIoBatch,
    /// Shared page dedup via CoW
    SharedPageDedup,
    /// Lock contention reduction
    LockContention,
    /// Cache warming for predicted access
    PredictiveCacheWarm,
    /// Reduce context switches via batching
    ContextSwitchReduction,
    /// IPC shortcut (direct copy)
    IpcShortcut,
    /// File descriptor sharing optimization
    FdSharing,
    /// Network socket multiplexing
    SocketMultiplex,
    /// Memory allocation coalescing
    AllocCoalesce,
    /// Syscall elimination (redundant calls)
    RedundantElimination,
    /// Priority inversion fix
    PriorityInversionFix,
}

/// Estimated benefit of an optimization
#[derive(Debug, Clone, Copy)]
pub struct OptimizationBenefit {
    /// Estimated latency reduction (ns)
    pub latency_reduction_ns: u64,
    /// Estimated throughput improvement (%)
    pub throughput_improvement_pct: f64,
    /// Estimated memory savings (bytes)
    pub memory_savings: u64,
    /// Estimated CPU savings (%)
    pub cpu_savings_pct: f64,
    /// Risk level (0.0 = safe, 1.0 = risky)
    pub risk: f64,
    /// Implementation cost (0.0 = free, 1.0 = expensive)
    pub cost: f64,
}

impl OptimizationBenefit {
    /// Net benefit score (higher = better)
    pub fn score(&self) -> f64 {
        let benefit = self.latency_reduction_ns as f64 / 1000.0 // µs saved
            + self.throughput_improvement_pct * 10.0
            + self.memory_savings as f64 / (1024.0 * 1024.0) // MB saved
            + self.cpu_savings_pct * 5.0;
        let penalty = self.risk * 100.0 + self.cost * 50.0;
        benefit - penalty
    }
}

/// An optimization opportunity
#[derive(Debug, Clone)]
pub struct OptimizationOpportunity {
    /// Type
    pub opt_type: OptimizationType,
    /// Affected processes
    pub affected_pids: Vec<u64>,
    /// Affected syscall types
    pub affected_syscalls: Vec<SyscallType>,
    /// Estimated benefit
    pub benefit: OptimizationBenefit,
    /// Confidence in the opportunity (0.0 - 1.0)
    pub confidence: f64,
    /// Discovery timestamp
    pub discovered_at: u64,
    /// Whether currently active
    pub active: bool,
}

// ============================================================================
// CONTENTION DETECTOR
// ============================================================================

/// Detects resource contention between processes
pub struct ContentionDetector {
    /// Lock contention tracking: resource_id → (pid, acquire_count, wait_time_ns)
    lock_contention: BTreeMap<u64, Vec<(u64, u64, u64)>>,
    /// File contention: inode → pids accessing
    file_contention: BTreeMap<u64, Vec<u64>>,
    /// Network port contention
    port_contention: BTreeMap<u16, Vec<u64>>,
    /// Memory region contention (page → pids)
    memory_contention: BTreeMap<u64, Vec<u64>>,
    /// Contention threshold (number of processes)
    threshold: usize,
}

impl ContentionDetector {
    pub fn new(threshold: usize) -> Self {
        Self {
            lock_contention: BTreeMap::new(),
            file_contention: BTreeMap::new(),
            port_contention: BTreeMap::new(),
            memory_contention: BTreeMap::new(),
            threshold,
        }
    }

    /// Record a lock acquisition
    pub fn record_lock(&mut self, resource_id: u64, pid: u64, wait_ns: u64) {
        let entries = self.lock_contention.entry(resource_id).or_insert_with(Vec::new);
        if let Some(entry) = entries.iter_mut().find(|(p, _, _)| *p == pid) {
            entry.1 += 1;
            entry.2 += wait_ns;
        } else {
            entries.push((pid, 1, wait_ns));
        }
    }

    /// Record a file access
    pub fn record_file_access(&mut self, inode: u64, pid: u64) {
        let pids = self.file_contention.entry(inode).or_insert_with(Vec::new);
        if !pids.contains(&pid) {
            pids.push(pid);
        }
    }

    /// Find contended resources
    pub fn find_contention(&self) -> Vec<ContentionReport> {
        let mut reports = Vec::new();

        // Lock contention
        for (&resource, entries) in &self.lock_contention {
            if entries.len() >= self.threshold {
                let total_wait: u64 = entries.iter().map(|(_, _, w)| w).sum();
                reports.push(ContentionReport {
                    resource_type: ResourceType::Lock,
                    resource_id: resource,
                    participant_pids: entries.iter().map(|(p, _, _)| *p).collect(),
                    severity: self.lock_severity(entries),
                    total_wait_ns: total_wait,
                });
            }
        }

        // File contention
        for (&inode, pids) in &self.file_contention {
            if pids.len() >= self.threshold {
                reports.push(ContentionReport {
                    resource_type: ResourceType::File,
                    resource_id: inode,
                    participant_pids: pids.clone(),
                    severity: if pids.len() > 4 { ContentionSeverity::High } else { ContentionSeverity::Medium },
                    total_wait_ns: 0,
                });
            }
        }

        reports
    }

    fn lock_severity(&self, entries: &[(u64, u64, u64)]) -> ContentionSeverity {
        let avg_wait: u64 = entries.iter().map(|(_, _, w)| w).sum::<u64>() / entries.len() as u64;
        if avg_wait > 10_000_000 {
            ContentionSeverity::Critical
        } else if avg_wait > 1_000_000 {
            ContentionSeverity::High
        } else if avg_wait > 100_000 {
            ContentionSeverity::Medium
        } else {
            ContentionSeverity::Low
        }
    }

    /// Remove process from all tracking
    pub fn remove_process(&mut self, pid: u64) {
        for entries in self.lock_contention.values_mut() {
            entries.retain(|(p, _, _)| *p != pid);
        }
        for pids in self.file_contention.values_mut() {
            pids.retain(|p| *p != pid);
        }
        for pids in self.port_contention.values_mut() {
            pids.retain(|p| *p != pid);
        }
    }
}

/// Contention report
#[derive(Debug, Clone)]
pub struct ContentionReport {
    /// Resource type
    pub resource_type: ResourceType,
    /// Resource identifier
    pub resource_id: u64,
    /// Participating processes
    pub participant_pids: Vec<u64>,
    /// Severity
    pub severity: ContentionSeverity,
    /// Total wait time
    pub total_wait_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Lock,
    File,
    Port,
    Memory,
    Cpu,
    Io,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentionSeverity {
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================================
// ADAPTIVE TUNER
// ============================================================================

/// Tunable parameter
#[derive(Debug, Clone)]
pub struct TunableParam {
    /// Parameter name
    pub name: &'static str,
    /// Current value
    pub current: f64,
    /// Minimum allowed value
    pub min: f64,
    /// Maximum allowed value
    pub max: f64,
    /// Step size for adjustments
    pub step: f64,
    /// Whether currently being tuned
    pub tuning: bool,
    /// Last adjustment timestamp
    pub last_adjusted: u64,
    /// Performance before last adjustment
    pub pre_adjustment_perf: f64,
    /// Performance after last adjustment
    pub post_adjustment_perf: f64,
}

impl TunableParam {
    pub fn new(name: &'static str, initial: f64, min: f64, max: f64, step: f64) -> Self {
        Self {
            name,
            current: initial,
            min,
            max,
            step,
            tuning: false,
            last_adjusted: 0,
            pre_adjustment_perf: 0.0,
            post_adjustment_perf: 0.0,
        }
    }

    /// Increase the parameter
    pub fn increase(&mut self) -> bool {
        let new = self.current + self.step;
        if new <= self.max {
            self.current = new;
            true
        } else {
            false
        }
    }

    /// Decrease the parameter
    pub fn decrease(&mut self) -> bool {
        let new = self.current - self.step;
        if new >= self.min {
            self.current = new;
            true
        } else {
            false
        }
    }

    /// Check if last adjustment improved performance
    pub fn adjustment_improved(&self) -> bool {
        self.post_adjustment_perf > self.pre_adjustment_perf
    }
}

/// Adaptive tuning engine
pub struct AdaptiveTuner {
    /// Tunable parameters
    params: Vec<TunableParam>,
    /// Current tuning index
    current_param: usize,
    /// Tuning phase
    phase: TuningPhase,
    /// Iterations since last improvement
    iterations_since_improvement: u64,
    /// Max iterations without improvement before stopping
    max_stale_iterations: u64,
    /// Total adjustments made
    pub total_adjustments: u64,
    /// Successful adjustments
    pub successful_adjustments: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuningPhase {
    /// Measuring baseline
    Baseline,
    /// Trying increase
    TryIncrease,
    /// Trying decrease
    TryDecrease,
    /// Evaluating result
    Evaluate,
    /// Stable (no tuning needed)
    Stable,
}

impl AdaptiveTuner {
    pub fn new() -> Self {
        Self {
            params: Vec::new(),
            current_param: 0,
            phase: TuningPhase::Baseline,
            iterations_since_improvement: 0,
            max_stale_iterations: 10,
            total_adjustments: 0,
            successful_adjustments: 0,
        }
    }

    /// Add a tunable parameter
    pub fn add_param(&mut self, param: TunableParam) {
        self.params.push(param);
    }

    /// Run one tuning iteration
    pub fn tune_step(&mut self, current_perf: f64, timestamp: u64) -> Option<TuningAction> {
        if self.params.is_empty() {
            return None;
        }

        match self.phase {
            TuningPhase::Baseline => {
                if self.current_param < self.params.len() {
                    self.params[self.current_param].pre_adjustment_perf = current_perf;
                    self.phase = TuningPhase::TryIncrease;
                    let param = &mut self.params[self.current_param];
                    if param.increase() {
                        self.total_adjustments += 1;
                        param.last_adjusted = timestamp;
                        return Some(TuningAction::Adjusted(param.name, param.current));
                    }
                }
                self.phase = TuningPhase::TryDecrease;
                None
            }
            TuningPhase::TryIncrease => {
                self.params[self.current_param].post_adjustment_perf = current_perf;
                if self.params[self.current_param].adjustment_improved() {
                    self.successful_adjustments += 1;
                    self.iterations_since_improvement = 0;
                    self.phase = TuningPhase::Baseline;
                    self.advance_param();
                    Some(TuningAction::Improved(self.params[self.current_param.saturating_sub(1)].name))
                } else {
                    // Revert and try decrease
                    self.params[self.current_param].decrease();
                    self.phase = TuningPhase::TryDecrease;
                    let param = &mut self.params[self.current_param];
                    param.pre_adjustment_perf = current_perf;
                    if param.decrease() {
                        self.total_adjustments += 1;
                        param.last_adjusted = timestamp;
                        Some(TuningAction::Adjusted(param.name, param.current))
                    } else {
                        self.phase = TuningPhase::Baseline;
                        self.advance_param();
                        None
                    }
                }
            }
            TuningPhase::TryDecrease => {
                self.params[self.current_param].post_adjustment_perf = current_perf;
                if self.params[self.current_param].adjustment_improved() {
                    self.successful_adjustments += 1;
                    self.iterations_since_improvement = 0;
                } else {
                    // Revert
                    self.params[self.current_param].increase();
                    self.iterations_since_improvement += 1;
                }
                self.phase = TuningPhase::Baseline;
                self.advance_param();
                None
            }
            TuningPhase::Evaluate | TuningPhase::Stable => {
                if self.iterations_since_improvement > self.max_stale_iterations {
                    self.phase = TuningPhase::Stable;
                    None
                } else {
                    self.phase = TuningPhase::Baseline;
                    None
                }
            }
        }
    }

    fn advance_param(&mut self) {
        self.current_param = (self.current_param + 1) % self.params.len();
    }

    /// Get current parameter values
    pub fn param_values(&self) -> Vec<(&str, f64)> {
        self.params.iter().map(|p| (p.name, p.current)).collect()
    }

    /// Success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_adjustments == 0 {
            0.0
        } else {
            self.successful_adjustments as f64 / self.total_adjustments as f64
        }
    }
}

/// Tuning action
#[derive(Debug)]
pub enum TuningAction {
    /// Parameter adjusted
    Adjusted(&'static str, f64),
    /// Adjustment improved performance
    Improved(&'static str),
    /// No action needed
    NoAction,
}

// ============================================================================
// GLOBAL OPTIMIZER
// ============================================================================

/// The global syscall optimizer
pub struct GlobalOptimizer {
    /// Discovered opportunities
    opportunities: Vec<OptimizationOpportunity>,
    /// Contention detector
    pub contention: ContentionDetector,
    /// Adaptive tuner
    pub tuner: AdaptiveTuner,
    /// Max opportunities to track
    max_opportunities: usize,
    /// Total optimizations applied
    pub applied: u64,
    /// Total benefit (estimated ns saved)
    pub total_saved_ns: u64,
}

impl GlobalOptimizer {
    pub fn new() -> Self {
        let mut tuner = AdaptiveTuner::new();

        // Add default tunable parameters
        tuner.add_param(TunableParam::new("batch_size", 16.0, 1.0, 256.0, 4.0));
        tuner.add_param(TunableParam::new("cache_size_mb", 8.0, 1.0, 64.0, 2.0));
        tuner.add_param(TunableParam::new("prefetch_depth", 4.0, 0.0, 32.0, 2.0));
        tuner.add_param(TunableParam::new("async_queue_depth", 32.0, 4.0, 512.0, 8.0));
        tuner.add_param(TunableParam::new("rate_limit_rps", 10000.0, 100.0, 1000000.0, 1000.0));

        Self {
            opportunities: Vec::new(),
            contention: ContentionDetector::new(2),
            tuner,
            max_opportunities: 128,
            applied: 0,
            total_saved_ns: 0,
        }
    }

    /// Add an optimization opportunity
    pub fn add_opportunity(&mut self, opp: OptimizationOpportunity) {
        if self.opportunities.len() >= self.max_opportunities {
            // Remove lowest-score opportunity
            if let Some(min_idx) = self
                .opportunities
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.benefit.score().partial_cmp(&b.benefit.score()).unwrap_or(core::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
            {
                if opp.benefit.score() > self.opportunities[min_idx].benefit.score() {
                    self.opportunities[min_idx] = opp;
                }
            }
        } else {
            self.opportunities.push(opp);
        }
    }

    /// Get top N opportunities by score
    pub fn top_opportunities(&self, n: usize) -> Vec<&OptimizationOpportunity> {
        let mut sorted: Vec<&OptimizationOpportunity> = self.opportunities.iter().collect();
        sorted.sort_by(|a, b| {
            b.benefit
                .score()
                .partial_cmp(&a.benefit.score())
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        sorted.truncate(n);
        sorted
    }

    /// Mark an optimization as applied
    pub fn mark_applied(&mut self, idx: usize, saved_ns: u64) {
        if idx < self.opportunities.len() {
            self.opportunities[idx].active = true;
            self.applied += 1;
            self.total_saved_ns += saved_ns;
        }
    }

    /// Run tuning step
    pub fn tune(&mut self, current_perf: f64, timestamp: u64) -> Option<TuningAction> {
        self.tuner.tune_step(current_perf, timestamp)
    }

    /// Number of opportunities
    pub fn opportunity_count(&self) -> usize {
        self.opportunities.len()
    }

    /// Remove process from tracking
    pub fn remove_process(&mut self, pid: u64) {
        self.contention.remove_process(pid);
        self.opportunities.retain(|opp| !opp.affected_pids.contains(&pid));
    }
}
