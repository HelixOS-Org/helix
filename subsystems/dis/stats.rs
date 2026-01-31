//! # Statistics Collector - Runtime Learning System
//!
//! The Statistics Collector gathers runtime data about task execution patterns,
//! system behavior, and resource usage. This data feeds the adaptive optimizer.
//!
//! ## Collected Metrics
//!
//! ### Per-Task Metrics
//! - CPU time (user/kernel)
//! - Wait time
//! - Context switches
//! - Cache misses
//! - Memory usage
//! - I/O operations
//! - Execution patterns (burst/steady)
//!
//! ### System Metrics
//! - Overall CPU load
//! - Memory pressure
//! - I/O wait
//! - Interrupt frequency
//! - Context switch rate
//!
//! ## Data Flow
//!
//! ```text
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │   TASK      │    │   SYSTEM    │    │   EVENTS    │
//! │   STATS     │    │   STATS     │    │   STREAM    │
//! └──────┬──────┘    └──────┬──────┘    └──────┬──────┘
//!        │                  │                  │
//!        ▼                  ▼                  ▼
//! ┌──────────────────────────────────────────────────────┐
//! │              STATISTICS COLLECTOR                     │
//! │                                                       │
//! │  • Aggregate                                          │
//! │  • Analyze                                            │
//! │  • Detect patterns                                    │
//! │  • Compute derivatives                                │
//! └───────────────────────┬──────────────────────────────┘
//!                         │
//!                         ▼
//! ┌──────────────────────────────────────────────────────┐
//! │              STATISTICS DATABASE                      │
//! │                                                       │
//! │  • Time-series data                                   │
//! │  • Moving averages                                    │
//! │  • Pattern signatures                                 │
//! │  • Anomaly detection                                  │
//! └──────────────────────────────────────────────────────┘
//! ```

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use spin::RwLock;

use super::{TaskId, CpuId, Nanoseconds};

// =============================================================================
// Task Statistics
// =============================================================================

/// Comprehensive statistics for a single task
#[derive(Debug, Clone, Default)]
pub struct TaskStats {
    // Time metrics
    /// Total CPU time consumed
    pub cpu_time: Nanoseconds,
    /// Time spent in user mode
    pub user_time: Nanoseconds,
    /// Time spent in kernel mode
    pub kernel_time: Nanoseconds,
    /// Total time spent waiting for CPU
    pub wait_time: Nanoseconds,
    /// Total time spent blocked on I/O
    pub io_wait_time: Nanoseconds,
    /// Total time spent sleeping
    pub sleep_time: Nanoseconds,
    
    // Scheduling metrics
    /// Number of times scheduled
    pub schedule_count: u64,
    /// Number of context switches
    pub context_switches: u64,
    /// Voluntary context switches (yields)
    pub voluntary_switches: u64,
    /// Involuntary context switches (preemptions)
    pub involuntary_switches: u64,
    /// Number of times preempted
    pub preemptions: u64,
    /// Number of migrations between CPUs
    pub migrations: u64,
    
    // Resource metrics
    /// Peak memory usage (bytes)
    pub peak_memory: u64,
    /// Current memory usage (bytes)
    pub current_memory: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// I/O operations count
    pub io_operations: u64,
    
    // Performance metrics
    /// CPU percentage (0-100)
    pub cpu_percent: u8,
    /// Average time slice utilization (0-100)
    pub slice_utilization: u8,
    /// Cache miss rate (0-100)
    pub cache_miss_rate: u8,
    
    // Pattern detection
    /// Detected behavior pattern
    pub behavior_pattern: BehaviorPattern,
    /// Burst score (0-100, higher = more bursty)
    pub burst_score: u8,
    /// Interactive score (0-100)
    pub interactive_score: u8,
    
    // Deadline metrics (for real-time)
    /// Number of deadlines met
    pub deadlines_met: u64,
    /// Number of deadlines missed
    pub deadlines_missed: u64,
    /// Worst case execution time observed
    pub worst_case_exec_time: Nanoseconds,
    /// Average execution time
    pub avg_exec_time: Nanoseconds,
    
    // History
    /// Last N execution times for averaging
    exec_time_history: VecDeque<Nanoseconds>,
    /// Last N wait times
    wait_time_history: VecDeque<Nanoseconds>,
}

/// Detected behavior pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BehaviorPattern {
    /// Unknown pattern
    #[default]
    Unknown,
    /// CPU-intensive, continuous execution
    CpuBound,
    /// I/O-intensive, frequent blocking
    IoBound,
    /// Mix of CPU and I/O
    Mixed,
    /// Burst execution followed by idle
    Bursty,
    /// Regular periodic execution
    Periodic,
    /// Interactive with varying workload
    Interactive,
    /// Long-running background
    Background,
}

impl TaskStats {
    /// Create new task statistics
    pub fn new() -> Self {
        Self {
            exec_time_history: VecDeque::with_capacity(32),
            wait_time_history: VecDeque::with_capacity(32),
            ..Default::default()
        }
    }
    
    /// Update runtime statistics
    pub fn update_runtime(&mut self, delta: Nanoseconds) {
        self.cpu_time += delta;
        self.schedule_count += 1;
        
        // Update execution time history
        if self.exec_time_history.len() >= 32 {
            self.exec_time_history.pop_front();
        }
        self.exec_time_history.push_back(delta);
        
        // Update worst case
        if delta > self.worst_case_exec_time {
            self.worst_case_exec_time = delta;
        }
        
        // Update average
        let sum: u64 = self.exec_time_history.iter().map(|t| t.raw()).sum();
        self.avg_exec_time = Nanoseconds::new(sum / self.exec_time_history.len().max(1) as u64);
    }
    
    /// Update wait time statistics
    pub fn update_wait_time(&mut self, delta: Nanoseconds) {
        self.wait_time += delta;
        
        if self.wait_time_history.len() >= 32 {
            self.wait_time_history.pop_front();
        }
        self.wait_time_history.push_back(delta);
    }
    
    /// Record a context switch
    pub fn record_context_switch(&mut self, voluntary: bool) {
        self.context_switches += 1;
        if voluntary {
            self.voluntary_switches += 1;
        } else {
            self.involuntary_switches += 1;
        }
    }
    
    /// Record a preemption
    pub fn record_preemption(&mut self) {
        self.preemptions += 1;
        self.involuntary_switches += 1;
        self.context_switches += 1;
    }
    
    /// Record a migration
    pub fn record_migration(&mut self) {
        self.migrations += 1;
    }
    
    /// Record deadline result
    pub fn record_deadline(&mut self, met: bool) {
        if met {
            self.deadlines_met += 1;
        } else {
            self.deadlines_missed += 1;
        }
    }
    
    /// Update memory usage
    pub fn update_memory(&mut self, current: u64) {
        self.current_memory = current;
        if current > self.peak_memory {
            self.peak_memory = current;
        }
    }
    
    /// Record I/O operation
    pub fn record_io(&mut self, bytes_read: u64, bytes_written: u64) {
        self.bytes_read += bytes_read;
        self.bytes_written += bytes_written;
        self.io_operations += 1;
    }
    
    /// Detect behavior pattern
    pub fn detect_pattern(&mut self) {
        // Calculate I/O ratio
        let total_time = (self.cpu_time + self.io_wait_time).raw().max(1);
        let io_ratio = (self.io_wait_time.raw() * 100 / total_time) as u8;
        
        // Calculate burst score from variance
        let burst = self.calculate_burst_score();
        self.burst_score = burst;
        
        // Calculate interactive score
        let interactive = self.calculate_interactive_score();
        self.interactive_score = interactive;
        
        // Determine pattern
        self.behavior_pattern = if io_ratio > 70 {
            BehaviorPattern::IoBound
        } else if io_ratio < 20 && self.slice_utilization > 80 {
            BehaviorPattern::CpuBound
        } else if burst > 70 {
            BehaviorPattern::Bursty
        } else if interactive > 70 {
            BehaviorPattern::Interactive
        } else if self.is_periodic() {
            BehaviorPattern::Periodic
        } else if io_ratio > 30 && io_ratio < 70 {
            BehaviorPattern::Mixed
        } else {
            BehaviorPattern::Background
        };
    }
    
    /// Calculate burst score (variance in execution times)
    fn calculate_burst_score(&self) -> u8 {
        if self.exec_time_history.len() < 2 {
            return 50;
        }
        
        let avg = self.avg_exec_time.raw();
        let variance: u64 = self.exec_time_history.iter()
            .map(|t| {
                let diff = if t.raw() > avg { t.raw() - avg } else { avg - t.raw() };
                diff * diff / 1_000_000 // Scale down to avoid overflow
            })
            .sum::<u64>() / self.exec_time_history.len() as u64;
        
        // Normalize to 0-100
        let score = (variance.min(100_000_000) / 1_000_000) as u8;
        score.min(100)
    }
    
    /// Calculate interactive score
    fn calculate_interactive_score(&self) -> u8 {
        let mut score = 50u8;
        
        // Frequent short bursts = more interactive
        if self.avg_exec_time.as_millis() < 10 && self.schedule_count > 100 {
            score = score.saturating_add(30);
        }
        
        // High voluntary switch ratio = interactive
        let vol_ratio = if self.context_switches > 0 {
            (self.voluntary_switches * 100 / self.context_switches) as u8
        } else {
            50
        };
        score = score.saturating_add(vol_ratio / 4);
        
        // Low CPU utilization = interactive
        if self.cpu_percent < 30 {
            score = score.saturating_add(20);
        }
        
        score.min(100)
    }
    
    /// Check if task shows periodic behavior
    fn is_periodic(&self) -> bool {
        if self.exec_time_history.len() < 4 {
            return false;
        }
        
        // Check if intervals are relatively constant
        let avg = self.avg_exec_time.raw();
        let threshold = avg / 4; // 25% tolerance
        
        let consistent = self.exec_time_history.iter()
            .all(|t| {
                let diff = if t.raw() > avg { t.raw() - avg } else { avg - t.raw() };
                diff < threshold
            });
        
        consistent && self.schedule_count > 10
    }
    
    /// Get deadline success rate (0-100)
    pub fn deadline_success_rate(&self) -> u8 {
        let total = self.deadlines_met + self.deadlines_missed;
        if total == 0 {
            return 100;
        }
        ((self.deadlines_met * 100) / total) as u8
    }
    
    /// Get a summary snapshot
    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            cpu_time: self.cpu_time,
            wait_time: self.wait_time,
            context_switches: self.context_switches,
            cpu_percent: self.cpu_percent,
            behavior: self.behavior_pattern,
            deadline_rate: self.deadline_success_rate(),
        }
    }
}

/// Lightweight statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct StatsSnapshot {
    pub cpu_time: Nanoseconds,
    pub wait_time: Nanoseconds,
    pub context_switches: u64,
    pub cpu_percent: u8,
    pub behavior: BehaviorPattern,
    pub deadline_rate: u8,
}

// =============================================================================
// System Statistics
// =============================================================================

/// System-wide statistics
#[derive(Debug, Default, Clone)]
pub struct SystemStats {
    // CPU metrics
    /// Overall CPU load (0-100)
    pub cpu_load: u8,
    /// Per-CPU load
    pub per_cpu_load: Vec<CpuStats>,
    /// Total CPU time (all CPUs)
    pub total_cpu_time: Nanoseconds,
    /// Idle CPU time
    pub idle_time: Nanoseconds,
    
    // Scheduling metrics
    /// Total context switches
    pub context_switches: u64,
    /// Context switches per second
    pub context_switches_per_sec: u64,
    /// Total preemptions
    pub preemptions: u64,
    /// Preemptions per second
    pub preemptions_per_sec: u64,
    /// Average scheduling latency
    pub avg_latency: Nanoseconds,
    /// Maximum scheduling latency
    pub max_latency: Nanoseconds,
    
    // Queue metrics
    /// Number of runnable tasks
    pub runnable_tasks: u32,
    /// Number of blocked tasks
    pub blocked_tasks: u32,
    /// Number of sleeping tasks
    pub sleeping_tasks: u32,
    /// Queue depths by class
    pub queue_depths: [u32; 8],
    
    // Memory metrics
    /// Memory usage (0-100)
    pub memory_usage: u8,
    /// Memory pressure level
    pub memory_pressure: MemoryPressure,
    /// Available memory (bytes)
    pub available_memory: u64,
    
    // I/O metrics
    /// I/O wait percentage (0-100)
    pub io_wait: u8,
    /// Disk I/O rate (bytes/sec)
    pub disk_io_rate: u64,
    /// Network I/O rate (bytes/sec)
    pub network_io_rate: u64,
    
    // Interrupt metrics
    /// Interrupts per second
    pub interrupts_per_sec: u64,
    /// Timer interrupts per second
    pub timer_interrupts_per_sec: u64,
    
    // Time tracking
    /// Current timestamp
    pub timestamp: Nanoseconds,
    /// Uptime
    pub uptime: Nanoseconds,
}

/// Per-CPU statistics
#[derive(Debug, Clone, Default)]
pub struct CpuStats {
    /// CPU ID
    pub cpu_id: CpuId,
    /// CPU load (0-100)
    pub load: u8,
    /// Time in user mode
    pub user_time: Nanoseconds,
    /// Time in kernel mode
    pub kernel_time: Nanoseconds,
    /// Time idle
    pub idle_time: Nanoseconds,
    /// Time in interrupt handlers
    pub irq_time: Nanoseconds,
    /// Number of tasks on this CPU
    pub task_count: u32,
    /// Current running task
    pub current_task: Option<TaskId>,
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryPressure {
    /// No pressure
    #[default]
    None,
    /// Low pressure
    Low,
    /// Medium pressure
    Medium,
    /// High pressure
    High,
    /// Critical pressure (OOM imminent)
    Critical,
}

impl SystemStats {
    /// Create new system statistics
    pub fn new(cpu_count: usize) -> Self {
        let mut stats = Self::default();
        for i in 0..cpu_count {
            stats.per_cpu_load.push(CpuStats {
                cpu_id: CpuId::new(i as u32),
                ..Default::default()
            });
        }
        stats
    }
    
    /// Update CPU load
    pub fn update_cpu_load(&mut self, total_time: Nanoseconds, idle_time: Nanoseconds) {
        self.total_cpu_time = total_time;
        self.idle_time = idle_time;
        
        if total_time.raw() > 0 {
            let busy_time = total_time.raw().saturating_sub(idle_time.raw());
            self.cpu_load = ((busy_time * 100) / total_time.raw()) as u8;
        }
    }
    
    /// Record a context switch
    pub fn record_context_switch(&mut self) {
        self.context_switches += 1;
    }
    
    /// Record a preemption
    pub fn record_preemption(&mut self) {
        self.preemptions += 1;
    }
    
    /// Update task counts
    pub fn update_task_counts(&mut self, runnable: u32, blocked: u32, sleeping: u32) {
        self.runnable_tasks = runnable;
        self.blocked_tasks = blocked;
        self.sleeping_tasks = sleeping;
    }
    
    /// Update memory pressure
    pub fn update_memory(&mut self, usage: u8, available: u64) {
        self.memory_usage = usage;
        self.available_memory = available;
        
        self.memory_pressure = match usage {
            0..=50 => MemoryPressure::None,
            51..=70 => MemoryPressure::Low,
            71..=85 => MemoryPressure::Medium,
            86..=95 => MemoryPressure::High,
            _ => MemoryPressure::Critical,
        };
    }
    
    /// Calculate rates (should be called periodically)
    pub fn calculate_rates(&mut self, delta: Nanoseconds) {
        let delta_secs = delta.as_secs().max(1);
        
        self.context_switches_per_sec = self.context_switches / delta_secs;
        self.preemptions_per_sec = self.preemptions / delta_secs;
    }
}

// =============================================================================
// Statistics Collector
// =============================================================================

/// The main statistics collector
pub struct StatsCollector {
    /// Per-task statistics
    task_stats: RwLock<BTreeMap<TaskId, TaskStats>>,
    /// System statistics
    system_stats: RwLock<SystemStats>,
    /// Historical snapshots
    history: RwLock<VecDeque<HistoryEntry>>,
    /// Maximum history entries
    max_history: usize,
    /// Collection interval
    interval: Nanoseconds,
    /// Last collection time
    last_collection: AtomicU64,
    /// Statistics
    collector_stats: CollectorStats,
}

/// Collector internal statistics
#[derive(Debug, Default)]
struct CollectorStats {
    collections: AtomicU64,
    tasks_tracked: AtomicU32,
}

/// Historical entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Timestamp
    pub timestamp: Nanoseconds,
    /// System CPU load
    pub cpu_load: u8,
    /// Memory usage
    pub memory_usage: u8,
    /// Runnable tasks
    pub runnable_tasks: u32,
    /// Context switches in period
    pub context_switches: u64,
}

impl StatsCollector {
    /// Create a new statistics collector
    pub fn new(cpu_count: usize) -> Self {
        Self {
            task_stats: RwLock::new(BTreeMap::new()),
            system_stats: RwLock::new(SystemStats::new(cpu_count)),
            history: RwLock::new(VecDeque::with_capacity(1000)),
            max_history: 1000,
            interval: Nanoseconds::from_millis(100),
            last_collection: AtomicU64::new(0),
            collector_stats: CollectorStats::default(),
        }
    }
    
    /// Register a new task
    pub fn register_task(&self, task_id: TaskId) {
        self.task_stats.write().insert(task_id, TaskStats::new());
        self.collector_stats.tasks_tracked.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Unregister a task
    pub fn unregister_task(&self, task_id: TaskId) -> Option<TaskStats> {
        let stats = self.task_stats.write().remove(&task_id);
        if stats.is_some() {
            self.collector_stats.tasks_tracked.fetch_sub(1, Ordering::Relaxed);
        }
        stats
    }
    
    /// Get task statistics
    pub fn get_task_stats(&self, task_id: TaskId) -> Option<TaskStats> {
        self.task_stats.read().get(&task_id).cloned()
    }
    
    /// Get system statistics
    pub fn get_system_stats(&self) -> SystemStats {
        self.system_stats.read().clone()
    }
    
    /// Update task runtime
    pub fn update_task_runtime(&self, task_id: TaskId, delta: Nanoseconds) {
        if let Some(stats) = self.task_stats.write().get_mut(&task_id) {
            stats.update_runtime(delta);
        }
    }
    
    /// Update task wait time
    pub fn update_task_wait_time(&self, task_id: TaskId, delta: Nanoseconds) {
        if let Some(stats) = self.task_stats.write().get_mut(&task_id) {
            stats.update_wait_time(delta);
        }
    }
    
    /// Record task context switch
    pub fn record_task_switch(&self, task_id: TaskId, voluntary: bool) {
        if let Some(stats) = self.task_stats.write().get_mut(&task_id) {
            stats.record_context_switch(voluntary);
        }
        self.system_stats.write().record_context_switch();
    }
    
    /// Record task preemption
    pub fn record_task_preemption(&self, task_id: TaskId) {
        if let Some(stats) = self.task_stats.write().get_mut(&task_id) {
            stats.record_preemption();
        }
        self.system_stats.write().record_preemption();
    }
    
    /// Record task migration
    pub fn record_task_migration(&self, task_id: TaskId) {
        if let Some(stats) = self.task_stats.write().get_mut(&task_id) {
            stats.record_migration();
        }
    }
    
    /// Record deadline result
    pub fn record_deadline(&self, task_id: TaskId, met: bool) {
        if let Some(stats) = self.task_stats.write().get_mut(&task_id) {
            stats.record_deadline(met);
        }
    }
    
    /// Update task memory usage
    pub fn update_task_memory(&self, task_id: TaskId, bytes: u64) {
        if let Some(stats) = self.task_stats.write().get_mut(&task_id) {
            stats.update_memory(bytes);
        }
    }
    
    /// Record task I/O
    pub fn record_task_io(&self, task_id: TaskId, read: u64, written: u64) {
        if let Some(stats) = self.task_stats.write().get_mut(&task_id) {
            stats.record_io(read, written);
        }
    }
    
    /// Detect pattern for task
    pub fn detect_task_pattern(&self, task_id: TaskId) -> Option<BehaviorPattern> {
        if let Some(stats) = self.task_stats.write().get_mut(&task_id) {
            stats.detect_pattern();
            return Some(stats.behavior_pattern);
        }
        None
    }
    
    /// Update system CPU load
    pub fn update_system_cpu(&self, total: Nanoseconds, idle: Nanoseconds) {
        self.system_stats.write().update_cpu_load(total, idle);
    }
    
    /// Update system memory
    pub fn update_system_memory(&self, usage: u8, available: u64) {
        self.system_stats.write().update_memory(usage, available);
    }
    
    /// Update system task counts
    pub fn update_system_tasks(&self, runnable: u32, blocked: u32, sleeping: u32) {
        self.system_stats.write().update_task_counts(runnable, blocked, sleeping);
    }
    
    /// Periodic collection tick
    pub fn tick(&self, current_time: Nanoseconds) {
        let last = self.last_collection.load(Ordering::Relaxed);
        
        if current_time.raw() - last < self.interval.raw() {
            return;
        }
        
        self.last_collection.store(current_time.raw(), Ordering::Relaxed);
        self.collector_stats.collections.fetch_add(1, Ordering::Relaxed);
        
        // Create history entry
        let system = self.system_stats.read();
        let entry = HistoryEntry {
            timestamp: current_time,
            cpu_load: system.cpu_load,
            memory_usage: system.memory_usage,
            runnable_tasks: system.runnable_tasks,
            context_switches: system.context_switches,
        };
        
        // Add to history
        let mut history = self.history.write();
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(entry);
        
        // Calculate rates
        drop(system);
        self.system_stats.write().calculate_rates(self.interval);
        
        // Detect patterns for all tasks
        for stats in self.task_stats.write().values_mut() {
            stats.detect_pattern();
        }
    }
    
    /// Get history
    pub fn history(&self) -> Vec<HistoryEntry> {
        self.history.read().iter().cloned().collect()
    }
    
    /// Get recent average CPU load
    pub fn recent_avg_cpu_load(&self, samples: usize) -> u8 {
        let history = self.history.read();
        let count = samples.min(history.len()).max(1);
        
        let sum: u32 = history.iter()
            .rev()
            .take(count)
            .map(|e| e.cpu_load as u32)
            .sum();
        
        (sum / count as u32) as u8
    }
    
    /// Get tasks sorted by CPU usage
    pub fn top_cpu_tasks(&self, limit: usize) -> Vec<(TaskId, u8)> {
        let mut tasks: Vec<_> = self.task_stats.read()
            .iter()
            .map(|(id, stats)| (*id, stats.cpu_percent))
            .collect();
        
        tasks.sort_by(|a, b| b.1.cmp(&a.1));
        tasks.truncate(limit);
        tasks
    }
    
    /// Get tasks by behavior pattern
    pub fn tasks_by_pattern(&self, pattern: BehaviorPattern) -> Vec<TaskId> {
        self.task_stats.read()
            .iter()
            .filter(|(_, stats)| stats.behavior_pattern == pattern)
            .map(|(id, _)| *id)
            .collect()
    }
    
    /// Get statistics summary
    pub fn summary(&self) -> StatsSummary {
        let system = self.system_stats.read();
        let task_count = self.collector_stats.tasks_tracked.load(Ordering::Relaxed);
        
        StatsSummary {
            total_tasks: task_count,
            cpu_load: system.cpu_load,
            memory_usage: system.memory_usage,
            runnable_tasks: system.runnable_tasks,
            context_switches_per_sec: system.context_switches_per_sec,
            collections: self.collector_stats.collections.load(Ordering::Relaxed),
        }
    }
}

/// Statistics summary
#[derive(Debug, Clone)]
pub struct StatsSummary {
    pub total_tasks: u32,
    pub cpu_load: u8,
    pub memory_usage: u8,
    pub runnable_tasks: u32,
    pub context_switches_per_sec: u64,
    pub collections: u64,
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new(1)
    }
}

// =============================================================================
// Analysis Functions
// =============================================================================

/// Analyze task for optimization hints
pub fn analyze_task(stats: &TaskStats) -> Vec<AnalysisHint> {
    let mut hints = Vec::new();
    
    // High wait time
    if stats.wait_time > stats.cpu_time {
        hints.push(AnalysisHint::HighWaitTime);
    }
    
    // Low slice utilization
    if stats.slice_utilization < 30 {
        hints.push(AnalysisHint::LowSliceUtilization);
    }
    
    // High cache miss rate
    if stats.cache_miss_rate > 30 {
        hints.push(AnalysisHint::HighCacheMissRate);
    }
    
    // Too many preemptions
    if stats.preemptions > stats.voluntary_switches * 2 {
        hints.push(AnalysisHint::FrequentPreemptions);
    }
    
    // Deadline issues
    if stats.deadlines_missed > 0 && stats.deadline_success_rate() < 95 {
        hints.push(AnalysisHint::DeadlineIssues);
    }
    
    // Migration overhead
    if stats.migrations > stats.context_switches / 10 {
        hints.push(AnalysisHint::FrequentMigrations);
    }
    
    hints
}

/// Analysis hints for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisHint {
    /// Task is waiting too much
    HighWaitTime,
    /// Task not using full time slice
    LowSliceUtilization,
    /// High cache miss rate
    HighCacheMissRate,
    /// Too many involuntary preemptions
    FrequentPreemptions,
    /// Deadline issues detected
    DeadlineIssues,
    /// Too many CPU migrations
    FrequentMigrations,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_task_stats_runtime() {
        let mut stats = TaskStats::new();
        
        stats.update_runtime(Nanoseconds::from_millis(10));
        stats.update_runtime(Nanoseconds::from_millis(15));
        stats.update_runtime(Nanoseconds::from_millis(12));
        
        assert_eq!(stats.schedule_count, 3);
        assert_eq!(stats.cpu_time.as_millis(), 37);
        assert_eq!(stats.worst_case_exec_time.as_millis(), 15);
    }
    
    #[test]
    fn test_pattern_detection() {
        let mut stats = TaskStats::new();
        
        // Simulate CPU-bound task
        for _ in 0..20 {
            stats.update_runtime(Nanoseconds::from_millis(100));
        }
        stats.slice_utilization = 90;
        
        stats.detect_pattern();
        assert_eq!(stats.behavior_pattern, BehaviorPattern::CpuBound);
    }
    
    #[test]
    fn test_stats_collector() {
        let collector = StatsCollector::new(4);
        
        let task_id = TaskId::new(1);
        collector.register_task(task_id);
        
        collector.update_task_runtime(task_id, Nanoseconds::from_millis(50));
        collector.record_task_switch(task_id, true);
        
        let stats = collector.get_task_stats(task_id).unwrap();
        assert_eq!(stats.cpu_time.as_millis(), 50);
        assert_eq!(stats.voluntary_switches, 1);
    }
}
