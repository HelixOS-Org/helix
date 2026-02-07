//! # Application Threading Analysis
//!
//! Thread-level behavior analysis:
//! - Thread creation/destruction tracking
//! - Thread pool detection
//! - Thread contention analysis
//! - Thread affinity recommendation
//! - Thread communication patterns

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// THREAD TYPES
// ============================================================================

/// Thread type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadType {
    /// Main thread
    Main,
    /// Worker thread (pool member)
    Worker,
    /// I/O thread
    Io,
    /// Timer/scheduler thread
    Timer,
    /// Signal handler
    SignalHandler,
    /// Background/idle
    Background,
    /// Unknown
    Unknown,
}

/// Thread state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppThreadState {
    /// Running
    Running,
    /// Sleeping
    Sleeping,
    /// Blocked on I/O
    BlockedIo,
    /// Blocked on lock
    BlockedLock,
    /// Blocked on futex
    BlockedFutex,
    /// Zombie
    Zombie,
    /// Stopped
    Stopped,
}

// ============================================================================
// THREAD DESCRIPTOR
// ============================================================================

/// Thread descriptor
#[derive(Debug, Clone)]
pub struct ThreadDescriptor {
    /// Thread id
    pub tid: u64,
    /// Parent process id
    pub pid: u64,
    /// Type
    pub thread_type: ThreadType,
    /// State
    pub state: AppThreadState,
    /// CPU core affinity
    pub core: Option<u32>,
    /// CPU usage (0.0-1.0)
    pub cpu_usage: f64,
    /// Wall time active (ns)
    pub active_ns: u64,
    /// Wall time blocked (ns)
    pub blocked_ns: u64,
    /// Syscalls made
    pub syscall_count: u64,
    /// Context switches (voluntary)
    pub voluntary_csw: u64,
    /// Context switches (involuntary)
    pub involuntary_csw: u64,
    /// Created at
    pub created_at: u64,
}

impl ThreadDescriptor {
    pub fn new(tid: u64, pid: u64, now: u64) -> Self {
        Self {
            tid,
            pid,
            thread_type: ThreadType::Unknown,
            state: AppThreadState::Running,
            core: None,
            cpu_usage: 0.0,
            active_ns: 0,
            blocked_ns: 0,
            syscall_count: 0,
            voluntary_csw: 0,
            involuntary_csw: 0,
            created_at: now,
        }
    }

    /// Active fraction
    pub fn active_fraction(&self) -> f64 {
        let total = self.active_ns + self.blocked_ns;
        if total == 0 {
            return 0.0;
        }
        self.active_ns as f64 / total as f64
    }

    /// Context switch rate
    pub fn csw_rate(&self, elapsed_ns: u64) -> f64 {
        if elapsed_ns == 0 {
            return 0.0;
        }
        let total = self.voluntary_csw + self.involuntary_csw;
        total as f64 / (elapsed_ns as f64 / 1_000_000_000.0)
    }
}

// ============================================================================
// THREAD POOL
// ============================================================================

/// Thread pool descriptor
#[derive(Debug, Clone)]
pub struct ThreadPool {
    /// Pool id
    pub id: u64,
    /// Process id
    pub pid: u64,
    /// Worker tids
    pub workers: Vec<u64>,
    /// Active workers
    pub active_count: usize,
    /// Idle workers
    pub idle_count: usize,
    /// Tasks completed
    pub tasks_completed: u64,
    /// Average task latency (ns)
    pub avg_task_latency_ns: f64,
}

impl ThreadPool {
    pub fn new(id: u64, pid: u64) -> Self {
        Self {
            id,
            pid,
            workers: Vec::new(),
            active_count: 0,
            idle_count: 0,
            tasks_completed: 0,
            avg_task_latency_ns: 0.0,
        }
    }

    /// Pool size
    pub fn size(&self) -> usize {
        self.workers.len()
    }

    /// Utilization
    pub fn utilization(&self) -> f64 {
        if self.workers.is_empty() {
            return 0.0;
        }
        self.active_count as f64 / self.workers.len() as f64
    }

    /// Is oversized?
    pub fn is_oversized(&self) -> bool {
        self.utilization() < 0.3 && self.workers.len() > 2
    }

    /// Is undersized?
    pub fn is_undersized(&self) -> bool {
        self.utilization() > 0.9 && self.workers.len() < 128
    }
}

// ============================================================================
// COMMUNICATION PATTERN
// ============================================================================

/// Thread communication event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommType {
    /// Mutex
    Mutex,
    /// Condition variable
    CondVar,
    /// Channel/pipe
    Channel,
    /// Shared memory
    SharedMem,
    /// Signal
    Signal,
}

/// Communication edge between threads
#[derive(Debug, Clone)]
pub struct CommEdge {
    /// Source tid
    pub from: u64,
    /// Dest tid
    pub to: u64,
    /// Type
    pub comm_type: CommType,
    /// Event count
    pub count: u64,
    /// Total latency (ns)
    pub total_latency_ns: u64,
}

impl CommEdge {
    /// Average latency
    pub fn avg_latency_ns(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.total_latency_ns as f64 / self.count as f64
    }
}

// ============================================================================
// THREAD ANALYZER
// ============================================================================

/// Thread analysis stats
#[derive(Debug, Clone, Default)]
pub struct AppThreadStats {
    /// Threads tracked
    pub threads_tracked: usize,
    /// Pools detected
    pub pools_detected: usize,
    /// Comm edges
    pub comm_edges: usize,
    /// Average utilization
    pub avg_utilization: f64,
}

/// Application thread analyzer
pub struct AppThreadAnalyzer {
    /// Threads per process
    threads: BTreeMap<u64, Vec<ThreadDescriptor>>,
    /// Thread pools
    pools: BTreeMap<u64, ThreadPool>,
    /// Communication edges
    comm: Vec<CommEdge>,
    /// Next pool id
    next_pool_id: u64,
    /// Stats
    stats: AppThreadStats,
}

impl AppThreadAnalyzer {
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new(),
            pools: BTreeMap::new(),
            comm: Vec::new(),
            next_pool_id: 1,
            stats: AppThreadStats::default(),
        }
    }

    /// Register thread
    pub fn register(&mut self, desc: ThreadDescriptor) {
        let pid = desc.pid;
        self.threads.entry(pid).or_insert_with(Vec::new).push(desc);
        self.update_stats();
    }

    /// Update thread state
    pub fn update_state(&mut self, tid: u64, state: AppThreadState) {
        for threads in self.threads.values_mut() {
            for t in threads.iter_mut() {
                if t.tid == tid {
                    t.state = state;
                    return;
                }
            }
        }
    }

    /// Record communication
    pub fn record_comm(&mut self, from: u64, to: u64, comm_type: CommType, latency_ns: u64) {
        if let Some(edge) = self
            .comm
            .iter_mut()
            .find(|e| e.from == from && e.to == to && e.comm_type == comm_type)
        {
            edge.count += 1;
            edge.total_latency_ns += latency_ns;
        } else {
            self.comm.push(CommEdge {
                from,
                to,
                comm_type,
                count: 1,
                total_latency_ns: latency_ns,
            });
        }
        self.stats.comm_edges = self.comm.len();
    }

    /// Detect thread pools
    pub fn detect_pools(&mut self) -> Vec<u64> {
        let mut new_pools = Vec::new();

        for (&pid, threads) in &self.threads {
            // Workers: similar CPU usage, similar syscall patterns
            let workers: Vec<u64> = threads
                .iter()
                .filter(|t| {
                    matches!(t.thread_type, ThreadType::Worker | ThreadType::Unknown)
                        && t.active_fraction() > 0.1
                })
                .map(|t| t.tid)
                .collect();

            if workers.len() >= 2 {
                let pool_id = self.next_pool_id;
                self.next_pool_id += 1;
                let mut pool = ThreadPool::new(pool_id, pid);
                pool.workers = workers;
                pool.active_count = pool.workers.len();
                self.pools.insert(pool_id, pool);
                new_pools.push(pool_id);
            }
        }

        self.stats.pools_detected = self.pools.len();
        new_pools
    }

    /// Process thread count
    pub fn thread_count(&self, pid: u64) -> usize {
        self.threads.get(&pid).map(|t| t.len()).unwrap_or(0)
    }

    /// Hottest threads (highest CPU)
    pub fn hottest_threads(&self, pid: u64, count: usize) -> Vec<(u64, f64)> {
        let mut threads: Vec<(u64, f64)> = self
            .threads
            .get(&pid)
            .map(|ts| ts.iter().map(|t| (t.tid, t.cpu_usage)).collect())
            .unwrap_or_default();
        threads.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        threads.truncate(count);
        threads
    }

    fn update_stats(&mut self) {
        let total: usize = self.threads.values().map(|t| t.len()).sum();
        self.stats.threads_tracked = total;

        let all_usage: Vec<f64> = self
            .threads
            .values()
            .flat_map(|ts| ts.iter().map(|t| t.cpu_usage))
            .collect();
        if !all_usage.is_empty() {
            self.stats.avg_utilization = all_usage.iter().sum::<f64>() / all_usage.len() as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &AppThreadStats {
        &self.stats
    }
}
