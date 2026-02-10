//! # Application Lifecycle Management
//!
//! Tracks and manages the full lifecycle of applications:
//! - Process birth, evolution, and death
//! - Phase detection (startup, warmup, steady-state, burst, cooldown, shutdown)
//! - Lifecycle event tracking and reaction
//! - Resource pre-allocation based on lifecycle phase
//! - Graceful degradation management

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// LIFECYCLE PHASES
// ============================================================================

/// Application lifecycle phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    /// Just created, loading libraries, parsing config
    Startup,
    /// Warming caches, establishing connections
    Warmup,
    /// Normal operation
    SteadyState,
    /// High-load burst
    Burst,
    /// Load decreasing
    Cooldown,
    /// Preparing to exit
    Shutdown,
    /// Sleeping / suspended
    Dormant,
    /// Recovering from error/crash
    Recovery,
    /// Checkpointing state
    Checkpoint,
    /// Migrating (e.g., between NUMA nodes)
    Migration,
}

/// Phase transition
#[derive(Debug, Clone, Copy)]
pub struct PhaseTransition {
    /// Previous phase
    pub from: LifecyclePhase,
    /// New phase
    pub to: LifecyclePhase,
    /// When the transition occurred
    pub timestamp: u64,
    /// Confidence in the transition detection (0.0 - 1.0)
    pub confidence: f64,
}

/// Lifecycle event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// Process created
    Created,
    /// Process exec'd a new binary
    Execed,
    /// Process forked a child
    Forked,
    /// Thread created
    ThreadCreated,
    /// Thread exited
    ThreadExited,
    /// Signal received
    SignalReceived(u32),
    /// Out of memory
    OomTriggered,
    /// Resource limit hit
    ResourceLimitHit,
    /// Checkpoint completed
    CheckpointComplete,
    /// Migration completed
    MigrationComplete,
    /// Crash detected
    CrashDetected,
    /// Graceful shutdown initiated
    ShutdownInitiated,
    /// Process exited
    Exited(i32),
}

// ============================================================================
// LIFECYCLE STATE MACHINE
// ============================================================================

/// Per-process lifecycle state
#[derive(Debug, Clone)]
pub struct ProcessLifecycle {
    /// Process ID
    pub pid: u64,
    /// Current phase
    pub phase: LifecyclePhase,
    /// Phase entry time
    pub phase_start: u64,
    /// Creation time
    pub created_at: u64,
    /// Phase history
    pub transitions: VecDeque<PhaseTransition>,
    /// Event history
    pub events: VecDeque<(u64, LifecycleEvent)>,
    /// Time spent in each phase (ns)
    pub phase_durations: BTreeMap<u8, u64>,
    /// Syscall rate (recent, for phase detection)
    recent_syscall_rates: VecDeque<f64>,
    /// CPU usage (recent, for phase detection)
    recent_cpu_usage: VecDeque<f64>,
    /// Memory usage (recent, for phase detection)
    recent_memory_usage: VecDeque<u64>,
    /// Is the first N seconds after creation
    is_young: bool,
    /// Number of phase changes
    pub phase_changes: u64,
}

impl ProcessLifecycle {
    pub fn new(pid: u64, timestamp: u64) -> Self {
        Self {
            pid,
            phase: LifecyclePhase::Startup,
            phase_start: timestamp,
            created_at: timestamp,
            transitions: VecDeque::new(),
            events: VecDeque::new(),
            phase_durations: BTreeMap::new(),
            recent_syscall_rates: VecDeque::new(),
            recent_cpu_usage: VecDeque::new(),
            recent_memory_usage: VecDeque::new(),
            is_young: true,
            phase_changes: 0,
        }
    }

    /// Record a lifecycle event
    #[inline]
    pub fn record_event(&mut self, event: LifecycleEvent, timestamp: u64) {
        if self.events.len() >= 256 {
            self.events.remove(0);
        }
        self.events.push_back((timestamp, event));
    }

    /// Update metrics for phase detection
    pub fn update_metrics(
        &mut self,
        syscall_rate: f64,
        cpu_usage: f64,
        memory_bytes: u64,
        timestamp: u64,
    ) {
        // Track recent metrics (keep last 30 samples)
        if self.recent_syscall_rates.len() >= 30 {
            self.recent_syscall_rates.remove(0);
        }
        self.recent_syscall_rates.push_back(syscall_rate);

        if self.recent_cpu_usage.len() >= 30 {
            self.recent_cpu_usage.remove(0);
        }
        self.recent_cpu_usage.push_back(cpu_usage);

        if self.recent_memory_usage.len() >= 30 {
            self.recent_memory_usage.remove(0);
        }
        self.recent_memory_usage.push_back(memory_bytes);

        // After 5 seconds, no longer "young"
        if timestamp.saturating_sub(self.created_at) > 5_000 {
            self.is_young = false;
        }

        // Detect phase
        let new_phase = self.detect_phase(timestamp);
        if new_phase != self.phase {
            self.transition_to(new_phase, timestamp);
        }
    }

    fn detect_phase(&self, _timestamp: u64) -> LifecyclePhase {
        if self.is_young {
            return LifecyclePhase::Startup;
        }

        if self.recent_syscall_rates.len() < 5 {
            return self.phase;
        }

        let avg_rate: f64 =
            self.recent_syscall_rates.iter().sum::<f64>() / self.recent_syscall_rates.len() as f64;
        let recent_rate: f64 = self.recent_syscall_rates.iter().rev().take(3).sum::<f64>() / 3.0;

        let avg_cpu: f64 =
            self.recent_cpu_usage.iter().sum::<f64>() / self.recent_cpu_usage.len() as f64;

        // Very low activity → Dormant
        if avg_rate < 1.0 && avg_cpu < 0.01 {
            return LifecyclePhase::Dormant;
        }

        // Burst detection: recent rate >> average rate
        if recent_rate > avg_rate * 2.0 && avg_rate > 10.0 {
            return LifecyclePhase::Burst;
        }

        // Cooldown: recent rate << average rate
        if recent_rate < avg_rate * 0.5 && avg_rate > 10.0 {
            return LifecyclePhase::Cooldown;
        }

        // Check for warmup (increasing trend in first phase)
        if self.phase == LifecyclePhase::Startup && self.is_increasing_trend() {
            return LifecyclePhase::Warmup;
        }

        LifecyclePhase::SteadyState
    }

    fn is_increasing_trend(&self) -> bool {
        if self.recent_syscall_rates.len() < 5 {
            return false;
        }
        let n = self.recent_syscall_rates.len();
        let first_half: f64 =
            self.recent_syscall_rates.iter().take(n / 2).sum::<f64>() / (n / 2) as f64;
        let second_half: f64 =
            self.recent_syscall_rates.iter().skip(n / 2).sum::<f64>() / (n - n / 2) as f64;
        second_half > first_half * 1.2
    }

    fn transition_to(&mut self, new_phase: LifecyclePhase, timestamp: u64) {
        let duration = timestamp.saturating_sub(self.phase_start);
        *self.phase_durations.entry(self.phase as u8).or_insert(0) += duration;

        let transition = PhaseTransition {
            from: self.phase,
            to: new_phase,
            timestamp,
            confidence: 0.8,
        };

        if self.transitions.len() >= 128 {
            self.transitions.remove(0);
        }
        self.transitions.push_back(transition);

        self.phase = new_phase;
        self.phase_start = timestamp;
        self.phase_changes += 1;
    }

    /// Get time in current phase
    #[inline(always)]
    pub fn time_in_phase(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.phase_start)
    }

    /// Total process uptime
    #[inline(always)]
    pub fn uptime(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.created_at)
    }
}

// ============================================================================
// LIFECYCLE MANAGER
// ============================================================================

/// Manages lifecycle state for all processes
pub struct LifecycleManager {
    /// Per-process lifecycle state
    processes: BTreeMap<u64, ProcessLifecycle>,
    /// Max processes to track
    max_processes: usize,
    /// Global event log
    global_events: VecDeque<(u64, u64, LifecycleEvent)>, // (timestamp, pid, event)
    /// Max global events
    max_global_events: usize,
    /// Phase-change callbacks (stored as counters per phase transition type)
    transition_counts: BTreeMap<(u8, u8), u64>,
}

impl LifecycleManager {
    pub fn new(max_processes: usize) -> Self {
        Self {
            processes: BTreeMap::new(),
            max_processes,
            global_events: VecDeque::new(),
            max_global_events: 10000,
            transition_counts: BTreeMap::new(),
        }
    }

    /// Create a new process lifecycle
    pub fn create_process(&mut self, pid: u64, timestamp: u64) {
        if self.processes.len() >= self.max_processes {
            // Evict oldest dormant process
            let dormant = self
                .processes
                .iter()
                .find(|(_, lc)| lc.phase == LifecyclePhase::Dormant)
                .map(|(&pid, _)| pid);
            if let Some(evict_pid) = dormant {
                self.processes.remove(&evict_pid);
            } else {
                return;
            }
        }

        let lifecycle = ProcessLifecycle::new(pid, timestamp);
        self.processes.insert(pid, lifecycle);
        self.log_global_event(timestamp, pid, LifecycleEvent::Created);
    }

    /// Get process lifecycle
    #[inline(always)]
    pub fn get(&self, pid: u64) -> Option<&ProcessLifecycle> {
        self.processes.get(&pid)
    }

    /// Get mutable process lifecycle
    #[inline(always)]
    pub fn get_mut(&mut self, pid: u64) -> Option<&mut ProcessLifecycle> {
        self.processes.get_mut(&pid)
    }

    /// Record a lifecycle event
    #[inline]
    pub fn record_event(&mut self, pid: u64, event: LifecycleEvent, timestamp: u64) {
        self.log_global_event(timestamp, pid, event);
        if let Some(lifecycle) = self.processes.get_mut(&pid) {
            lifecycle.record_event(event, timestamp);
        }
    }

    /// Update process metrics
    pub fn update_metrics(
        &mut self,
        pid: u64,
        syscall_rate: f64,
        cpu_usage: f64,
        memory_bytes: u64,
        timestamp: u64,
    ) {
        if let Some(lifecycle) = self.processes.get_mut(&pid) {
            let old_phase = lifecycle.phase;
            lifecycle.update_metrics(syscall_rate, cpu_usage, memory_bytes, timestamp);
            if lifecycle.phase != old_phase {
                *self
                    .transition_counts
                    .entry((old_phase as u8, lifecycle.phase as u8))
                    .or_insert(0) += 1;
            }
        }
    }

    /// Remove process
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64, exit_code: i32, timestamp: u64) {
        self.log_global_event(timestamp, pid, LifecycleEvent::Exited(exit_code));
        self.processes.remove(&pid);
    }

    fn log_global_event(&mut self, timestamp: u64, pid: u64, event: LifecycleEvent) {
        if self.global_events.len() >= self.max_global_events {
            self.global_events.remove(0);
        }
        self.global_events.push_back((timestamp, pid, event));
    }

    /// Count of processes in each phase
    #[inline]
    pub fn phase_distribution(&self) -> BTreeMap<u8, usize> {
        let mut dist = BTreeMap::new();
        for lifecycle in self.processes.values() {
            *dist.entry(lifecycle.phase as u8).or_insert(0) += 1;
        }
        dist
    }

    /// Number of tracked processes
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    /// Processes in burst phase
    #[inline]
    pub fn burst_processes(&self) -> Vec<u64> {
        self.processes
            .iter()
            .filter(|(_, lc)| lc.phase == LifecyclePhase::Burst)
            .map(|(&pid, _)| pid)
            .collect()
    }

    /// Processes in dormant phase
    #[inline]
    pub fn dormant_processes(&self) -> Vec<u64> {
        self.processes
            .iter()
            .filter(|(_, lc)| lc.phase == LifecyclePhase::Dormant)
            .map(|(&pid, _)| pid)
            .collect()
    }
}
