//! # Deep Application Profiling
//!
//! Multi-dimensional profiling of application behavior across CPU, memory,
//! I/O, and network dimensions. Builds a rich ProcessProfile that captures
//! the full character of an application.

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// LIFECYCLE PHASES
// ============================================================================

/// Phase of an application's lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLifecyclePhase {
    /// Starting up — loading libraries, initializing
    Startup,
    /// Warming up — filling caches, JIT compiling
    Warmup,
    /// Steady state — normal operation
    Steady,
    /// Burst — temporary spike in activity
    Burst,
    /// Idle — minimal activity
    Idle,
    /// Shutting down — cleanup, flushing
    Shutdown,
}

// ============================================================================
// BEHAVIOR DIMENSIONS
// ============================================================================

/// CPU behavior profile
#[derive(Debug, Clone)]
pub struct CpuBehavior {
    /// Average CPU usage (0.0 - 1.0)
    pub avg_usage: f64,
    /// Peak CPU usage
    pub peak_usage: f64,
    /// Whether CPU usage is bursty
    pub is_bursty: bool,
    /// Average burst duration (ms)
    pub avg_burst_duration_ms: u64,
    /// Whether the app is compute-bound
    pub is_compute_bound: bool,
    /// Average IPC (instructions per cycle)
    pub avg_ipc: f64,
    /// Cache miss rate (0.0 - 1.0)
    pub cache_miss_rate: f64,
    /// Branch misprediction rate (0.0 - 1.0)
    pub branch_mispredict_rate: f64,
    /// Number of threads typically used
    pub typical_thread_count: usize,
    /// Whether the app uses SIMD
    pub uses_simd: bool,
}

impl CpuBehavior {
    pub fn new() -> Self {
        Self {
            avg_usage: 0.0,
            peak_usage: 0.0,
            is_bursty: false,
            avg_burst_duration_ms: 0,
            is_compute_bound: false,
            avg_ipc: 0.0,
            cache_miss_rate: 0.0,
            branch_mispredict_rate: 0.0,
            typical_thread_count: 1,
            uses_simd: false,
        }
    }

    /// CPU efficiency score (0.0 - 1.0)
    pub fn efficiency(&self) -> f64 {
        let ipc_score = (self.avg_ipc / 4.0).min(1.0);
        let cache_score = 1.0 - self.cache_miss_rate;
        let branch_score = 1.0 - self.branch_mispredict_rate;
        (ipc_score + cache_score + branch_score) / 3.0
    }
}

/// Memory behavior profile
#[derive(Debug, Clone)]
pub struct MemoryBehavior {
    /// Average RSS (bytes)
    pub avg_rss: usize,
    /// Peak RSS
    pub peak_rss: usize,
    /// Allocation rate (allocs/sec)
    pub alloc_rate: f64,
    /// Free rate (frees/sec)
    pub free_rate: f64,
    /// Whether memory usage grows over time
    pub growing: bool,
    /// Growth rate (bytes/sec) if growing
    pub growth_rate: f64,
    /// Page fault rate (faults/sec)
    pub page_fault_rate: f64,
    /// Fraction of major faults
    pub major_fault_ratio: f64,
    /// Working set size (bytes)
    pub working_set: usize,
    /// Whether the app uses huge pages
    pub uses_huge_pages: bool,
    /// Whether the app uses mmap extensively
    pub uses_mmap: bool,
}

impl MemoryBehavior {
    pub fn new() -> Self {
        Self {
            avg_rss: 0,
            peak_rss: 0,
            alloc_rate: 0.0,
            free_rate: 0.0,
            growing: false,
            growth_rate: 0.0,
            page_fault_rate: 0.0,
            major_fault_ratio: 0.0,
            working_set: 0,
            uses_huge_pages: false,
            uses_mmap: false,
        }
    }

    /// Whether a memory leak is likely
    pub fn likely_leak(&self) -> bool {
        self.growing && self.growth_rate > 1024.0 && self.alloc_rate > self.free_rate * 1.1
    }

    /// Whether the app would benefit from huge pages
    pub fn should_use_huge_pages(&self) -> bool {
        !self.uses_huge_pages && self.working_set > 2 * 1024 * 1024 && self.major_fault_ratio > 0.1
    }
}

/// I/O behavior profile
#[derive(Debug, Clone)]
pub struct IoBehavior {
    /// Read throughput (bytes/sec)
    pub read_throughput: u64,
    /// Write throughput (bytes/sec)
    pub write_throughput: u64,
    /// Read IOPS
    pub read_iops: u64,
    /// Write IOPS
    pub write_iops: u64,
    /// Average read size
    pub avg_read_size: usize,
    /// Average write size
    pub avg_write_size: usize,
    /// Whether reads are sequential
    pub sequential_reads: bool,
    /// Whether writes are sequential
    pub sequential_writes: bool,
    /// Read-to-write ratio
    pub read_write_ratio: f64,
    /// Average I/O latency (µs)
    pub avg_latency_us: u64,
    /// Number of open file descriptors
    pub open_fds: usize,
    /// Whether the app fsync's frequently
    pub frequent_fsync: bool,
}

impl IoBehavior {
    pub fn new() -> Self {
        Self {
            read_throughput: 0,
            write_throughput: 0,
            read_iops: 0,
            write_iops: 0,
            avg_read_size: 0,
            avg_write_size: 0,
            sequential_reads: false,
            sequential_writes: false,
            read_write_ratio: 1.0,
            avg_latency_us: 0,
            open_fds: 0,
            frequent_fsync: false,
        }
    }

    /// Whether the app is I/O intensive
    pub fn is_io_intensive(&self) -> bool {
        self.read_iops + self.write_iops > 1000
            || self.read_throughput + self.write_throughput > 100 * 1024 * 1024
    }

    /// Optimal read-ahead size based on behavior
    pub fn optimal_readahead(&self) -> usize {
        if self.sequential_reads {
            // Sequential: aggressive readahead
            (self.avg_read_size * 8).min(256 * 1024)
        } else {
            // Random: minimal readahead
            self.avg_read_size.min(4096)
        }
    }
}

/// Network behavior profile
#[derive(Debug, Clone)]
pub struct NetworkBehavior {
    /// Inbound throughput (bytes/sec)
    pub rx_throughput: u64,
    /// Outbound throughput (bytes/sec)
    pub tx_throughput: u64,
    /// Active connections
    pub active_connections: usize,
    /// New connections per second
    pub connection_rate: f64,
    /// Average message size
    pub avg_message_size: usize,
    /// Whether the app is a server
    pub is_server: bool,
    /// Whether the app uses UDP
    pub uses_udp: bool,
    /// Whether the app uses multicast
    pub uses_multicast: bool,
    /// Dominant protocol (e.g., "TCP", "UDP")
    pub dominant_protocol: String,
}

impl NetworkBehavior {
    pub fn new() -> Self {
        Self {
            rx_throughput: 0,
            tx_throughput: 0,
            active_connections: 0,
            connection_rate: 0.0,
            avg_message_size: 0,
            is_server: false,
            uses_udp: false,
            uses_multicast: false,
            dominant_protocol: String::new(),
        }
    }

    /// Whether the app is network-intensive
    pub fn is_network_intensive(&self) -> bool {
        self.rx_throughput + self.tx_throughput > 10 * 1024 * 1024
            || self.active_connections > 100
            || self.connection_rate > 100.0
    }
}

// ============================================================================
// PROCESS PROFILE
// ============================================================================

/// Complete multi-dimensional profile of a process
#[derive(Debug, Clone)]
pub struct ProcessProfile {
    /// Process ID
    pub pid: u64,
    /// Application name
    pub name: Option<String>,
    /// Current lifecycle phase
    pub phase: AppLifecyclePhase,
    /// CPU behavior
    pub cpu: CpuBehavior,
    /// Memory behavior
    pub memory: MemoryBehavior,
    /// I/O behavior
    pub io: IoBehavior,
    /// Network behavior
    pub network: NetworkBehavior,
    /// How long this process has been profiled (ms)
    pub profile_duration_ms: u64,
    /// Number of observation windows completed
    pub observation_windows: u64,
    /// Overall health score (0.0 - 1.0)
    pub health_score: f64,
}

impl ProcessProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            name: None,
            phase: AppLifecyclePhase::Startup,
            cpu: CpuBehavior::new(),
            memory: MemoryBehavior::new(),
            io: IoBehavior::new(),
            network: NetworkBehavior::new(),
            profile_duration_ms: 0,
            observation_windows: 0,
            health_score: 1.0,
        }
    }

    /// Compute the overall health score
    pub fn compute_health(&mut self) {
        let mut score = 1.0;

        // Memory leak penalty
        if self.memory.likely_leak() {
            score -= 0.3;
        }

        // High page fault rate penalty
        if self.memory.page_fault_rate > 1000.0 {
            score -= 0.1;
        }

        // Poor CPU efficiency penalty
        if self.cpu.efficiency() < 0.3 {
            score -= 0.2;
        }

        // High I/O latency penalty
        if self.io.avg_latency_us > 10_000 {
            score -= 0.1;
        }

        self.health_score = score.max(0.0);
    }

    /// Whether this profile is mature enough for reliable decisions
    pub fn is_mature(&self) -> bool {
        self.observation_windows >= 10 && self.profile_duration_ms >= 5000
    }
}
