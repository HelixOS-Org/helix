//! # Application Resource Tracking
//!
//! Comprehensive resource tracking per application:
//! - CPU time accounting (user, system, idle-wait)
//! - Memory usage tracking (RSS, VSZ, shared, private, swap)
//! - I/O accounting (read/write bytes, IOPS)
//! - Network accounting (tx/rx bytes, connections)
//! - File descriptor tracking
//! - IPC resource tracking

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CPU ACCOUNTING
// ============================================================================

/// CPU time accounting for a process
#[derive(Debug, Clone)]
pub struct CpuAccounting {
    /// User-mode CPU time (ns)
    pub user_time_ns: u64,
    /// System-mode CPU time (ns)
    pub system_time_ns: u64,
    /// Voluntary context switches
    pub voluntary_switches: u64,
    /// Involuntary context switches
    pub involuntary_switches: u64,
    /// Time waiting for I/O (ns)
    pub io_wait_ns: u64,
    /// Time waiting for locks (ns)
    pub lock_wait_ns: u64,
    /// Time in syscalls (ns)
    pub syscall_time_ns: u64,
    /// Last schedule-in timestamp
    pub last_schedule_in: u64,
    /// CPU migrations between cores
    pub cpu_migrations: u64,
    /// Cache misses (L1, L2, L3 estimated)
    pub cache_miss_estimate: u64,
    /// Instructions retired (estimated)
    pub instructions_estimate: u64,
    /// IPC (instructions per cycle) estimate
    pub ipc_estimate: f64,
}

impl CpuAccounting {
    pub fn new() -> Self {
        Self {
            user_time_ns: 0,
            system_time_ns: 0,
            voluntary_switches: 0,
            involuntary_switches: 0,
            io_wait_ns: 0,
            lock_wait_ns: 0,
            syscall_time_ns: 0,
            last_schedule_in: 0,
            cpu_migrations: 0,
            cache_miss_estimate: 0,
            instructions_estimate: 0,
            ipc_estimate: 0.0,
        }
    }

    /// Total CPU time
    pub fn total_cpu_ns(&self) -> u64 {
        self.user_time_ns + self.system_time_ns
    }

    /// System/User ratio
    pub fn system_ratio(&self) -> f64 {
        let total = self.total_cpu_ns();
        if total == 0 {
            0.0
        } else {
            self.system_time_ns as f64 / total as f64
        }
    }

    /// Context switch rate (switches per second given a time window)
    pub fn switch_rate(&self, window_ns: u64) -> f64 {
        if window_ns == 0 {
            return 0.0;
        }
        let total = self.voluntary_switches + self.involuntary_switches;
        total as f64 / (window_ns as f64 / 1_000_000_000.0)
    }
}

// ============================================================================
// MEMORY ACCOUNTING
// ============================================================================

/// Memory usage tracking for a process
#[derive(Debug, Clone)]
pub struct MemoryAccounting {
    /// Virtual memory size (bytes)
    pub virtual_size: u64,
    /// Resident set size (bytes)
    pub resident_size: u64,
    /// Shared memory (bytes)
    pub shared_size: u64,
    /// Private memory (bytes)
    pub private_size: u64,
    /// Swap usage (bytes)
    pub swap_size: u64,
    /// Stack size (bytes)
    pub stack_size: u64,
    /// Heap size (bytes)
    pub heap_size: u64,
    /// Memory-mapped file size (bytes)
    pub mmap_size: u64,
    /// Page faults (minor)
    pub minor_faults: u64,
    /// Page faults (major)
    pub major_faults: u64,
    /// Peak RSS
    pub peak_rss: u64,
    /// Allocations since last check
    pub recent_allocs: u64,
    /// Frees since last check
    pub recent_frees: u64,
    /// Memory pressure score (0.0 - 1.0)
    pub pressure: f64,
}

impl MemoryAccounting {
    pub fn new() -> Self {
        Self {
            virtual_size: 0,
            resident_size: 0,
            shared_size: 0,
            private_size: 0,
            swap_size: 0,
            stack_size: 0,
            heap_size: 0,
            mmap_size: 0,
            minor_faults: 0,
            major_faults: 0,
            peak_rss: 0,
            recent_allocs: 0,
            recent_frees: 0,
            pressure: 0.0,
        }
    }

    /// Update RSS and track peak
    pub fn update_rss(&mut self, rss: u64) {
        self.resident_size = rss;
        if rss > self.peak_rss {
            self.peak_rss = rss;
        }
    }

    /// RSS as percentage of virtual
    pub fn rss_ratio(&self) -> f64 {
        if self.virtual_size == 0 {
            0.0
        } else {
            self.resident_size as f64 / self.virtual_size as f64
        }
    }

    /// Swap pressure
    pub fn swap_ratio(&self) -> f64 {
        let total = self.resident_size + self.swap_size;
        if total == 0 {
            0.0
        } else {
            self.swap_size as f64 / total as f64
        }
    }

    /// Major fault rate
    pub fn major_fault_rate(&self) -> f64 {
        let total = self.minor_faults + self.major_faults;
        if total == 0 {
            0.0
        } else {
            self.major_faults as f64 / total as f64
        }
    }
}

// ============================================================================
// I/O ACCOUNTING
// ============================================================================

/// I/O accounting for a process
#[derive(Debug, Clone)]
pub struct IoAccounting {
    /// Bytes read from storage
    pub read_bytes: u64,
    /// Bytes written to storage
    pub write_bytes: u64,
    /// Read operations count
    pub read_ops: u64,
    /// Write operations count
    pub write_ops: u64,
    /// Read syscalls (including cached)
    pub read_syscalls: u64,
    /// Write syscalls (including buffered)
    pub write_syscalls: u64,
    /// Cancelled write bytes
    pub cancelled_write_bytes: u64,
    /// Direct I/O bytes (bypassing cache)
    pub direct_io_bytes: u64,
    /// Average read latency (ns)
    pub avg_read_latency_ns: u64,
    /// Average write latency (ns)
    pub avg_write_latency_ns: u64,
    /// Peak IOPS
    pub peak_iops: u64,
    /// I/O wait percentage
    pub io_wait_pct: f64,
}

impl IoAccounting {
    pub fn new() -> Self {
        Self {
            read_bytes: 0,
            write_bytes: 0,
            read_ops: 0,
            write_ops: 0,
            read_syscalls: 0,
            write_syscalls: 0,
            cancelled_write_bytes: 0,
            direct_io_bytes: 0,
            avg_read_latency_ns: 0,
            avg_write_latency_ns: 0,
            peak_iops: 0,
            io_wait_pct: 0.0,
        }
    }

    /// Total bytes transferred
    pub fn total_bytes(&self) -> u64 {
        self.read_bytes + self.write_bytes
    }

    /// Read/write ratio
    pub fn read_ratio(&self) -> f64 {
        let total = self.total_bytes();
        if total == 0 {
            0.0
        } else {
            self.read_bytes as f64 / total as f64
        }
    }

    /// Average operation size (bytes)
    pub fn avg_op_size(&self) -> u64 {
        let total_ops = self.read_ops + self.write_ops;
        if total_ops == 0 {
            0
        } else {
            self.total_bytes() / total_ops
        }
    }

    /// IOPS
    pub fn iops(&self) -> u64 {
        self.read_ops + self.write_ops
    }
}

// ============================================================================
// NETWORK ACCOUNTING
// ============================================================================

/// Network accounting for a process
#[derive(Debug, Clone)]
pub struct NetworkAccounting {
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Bytes received
    pub rx_bytes: u64,
    /// Packets transmitted
    pub tx_packets: u64,
    /// Packets received
    pub rx_packets: u64,
    /// Active connections
    pub active_connections: u32,
    /// Total connections made
    pub total_connections: u64,
    /// DNS lookups
    pub dns_lookups: u64,
    /// Retransmissions
    pub retransmissions: u64,
    /// Connection errors
    pub connection_errors: u64,
    /// Average RTT (µs)
    pub avg_rtt_us: u64,
}

impl NetworkAccounting {
    pub fn new() -> Self {
        Self {
            tx_bytes: 0,
            rx_bytes: 0,
            tx_packets: 0,
            rx_packets: 0,
            active_connections: 0,
            total_connections: 0,
            dns_lookups: 0,
            retransmissions: 0,
            connection_errors: 0,
            avg_rtt_us: 0,
        }
    }

    /// Total bytes
    pub fn total_bytes(&self) -> u64 {
        self.tx_bytes + self.rx_bytes
    }

    /// Retransmission rate
    pub fn retransmit_rate(&self) -> f64 {
        if self.tx_packets == 0 {
            0.0
        } else {
            self.retransmissions as f64 / self.tx_packets as f64
        }
    }

    /// Connection error rate
    pub fn error_rate(&self) -> f64 {
        if self.total_connections == 0 {
            0.0
        } else {
            self.connection_errors as f64 / self.total_connections as f64
        }
    }
}

// ============================================================================
// FILE DESCRIPTOR TRACKING
// ============================================================================

/// File descriptor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdType {
    RegularFile,
    Directory,
    Socket,
    Pipe,
    EventFd,
    TimerFd,
    SignalFd,
    Epoll,
    Inotify,
    Device,
    Unknown,
}

/// File descriptor info
#[derive(Debug, Clone)]
pub struct FdInfo {
    /// FD number
    pub fd: u32,
    /// Type
    pub fd_type: FdType,
    /// Open flags
    pub flags: u32,
    /// Current offset
    pub offset: u64,
    /// Bytes read through this fd
    pub bytes_read: u64,
    /// Bytes written through this fd
    pub bytes_written: u64,
    /// Operations count
    pub ops: u64,
    /// Open timestamp
    pub opened_at: u64,
}

/// File descriptor tracker
#[derive(Debug, Clone)]
pub struct FdTracker {
    /// Open FDs
    fds: BTreeMap<u32, FdInfo>,
    /// Peak FD count
    pub peak_count: u32,
    /// Total FDs ever opened
    pub total_opened: u64,
    /// Total FDs closed
    pub total_closed: u64,
}

impl FdTracker {
    pub fn new() -> Self {
        Self {
            fds: BTreeMap::new(),
            peak_count: 0,
            total_opened: 0,
            total_closed: 0,
        }
    }

    /// Open a new FD
    pub fn open(&mut self, fd: u32, fd_type: FdType, flags: u32, timestamp: u64) {
        self.fds.insert(fd, FdInfo {
            fd,
            fd_type,
            flags,
            offset: 0,
            bytes_read: 0,
            bytes_written: 0,
            ops: 0,
            opened_at: timestamp,
        });
        self.total_opened += 1;
        let count = self.fds.len() as u32;
        if count > self.peak_count {
            self.peak_count = count;
        }
    }

    /// Close an FD
    pub fn close(&mut self, fd: u32) -> Option<FdInfo> {
        self.total_closed += 1;
        self.fds.remove(&fd)
    }

    /// Record I/O on an FD
    pub fn record_io(&mut self, fd: u32, bytes: u64, is_read: bool) {
        if let Some(info) = self.fds.get_mut(&fd) {
            info.ops += 1;
            if is_read {
                info.bytes_read += bytes;
            } else {
                info.bytes_written += bytes;
            }
        }
    }

    /// Current open FD count
    pub fn count(&self) -> usize {
        self.fds.len()
    }

    /// FDs by type
    pub fn count_by_type(&self, fd_type: FdType) -> usize {
        self.fds.values().filter(|f| f.fd_type == fd_type).count()
    }

    /// Get FD info
    pub fn get(&self, fd: u32) -> Option<&FdInfo> {
        self.fds.get(&fd)
    }
}

// ============================================================================
// COMPOSITE RESOURCE TRACKER
// ============================================================================

/// Full resource tracker for a process
#[derive(Debug, Clone)]
pub struct ResourceTracker {
    /// Process ID
    pub pid: u64,
    /// CPU accounting
    pub cpu: CpuAccounting,
    /// Memory accounting
    pub memory: MemoryAccounting,
    /// I/O accounting
    pub io: IoAccounting,
    /// Network accounting
    pub network: NetworkAccounting,
    /// File descriptor tracking
    pub fds: FdTracker,
    /// Last update timestamp
    pub last_update: u64,
    /// Total resource score (0.0 - 1.0, higher = more resource usage)
    pub resource_score: f64,
}

impl ResourceTracker {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            cpu: CpuAccounting::new(),
            memory: MemoryAccounting::new(),
            io: IoAccounting::new(),
            network: NetworkAccounting::new(),
            fds: FdTracker::new(),
            last_update: 0,
            resource_score: 0.0,
        }
    }

    /// Compute overall resource score
    pub fn compute_score(&mut self, total_cpu_ns: u64, total_memory: u64) {
        let cpu_score = if total_cpu_ns > 0 {
            self.cpu.total_cpu_ns() as f64 / total_cpu_ns as f64
        } else {
            0.0
        };

        let mem_score = if total_memory > 0 {
            self.memory.resident_size as f64 / total_memory as f64
        } else {
            0.0
        };

        // Weighted combination
        self.resource_score = cpu_score * 0.4 + mem_score * 0.4 + self.memory.pressure * 0.2;
    }
}

/// Global resource manager
pub struct ResourceManager {
    /// Per-process trackers
    trackers: BTreeMap<u64, ResourceTracker>,
    /// Max tracked processes
    max_processes: usize,
    /// System totals
    pub system_cpu_ns: u64,
    pub system_memory_bytes: u64,
    pub system_io_bytes: u64,
}

impl ResourceManager {
    pub fn new(max_processes: usize) -> Self {
        Self {
            trackers: BTreeMap::new(),
            max_processes,
            system_cpu_ns: 0,
            system_memory_bytes: 0,
            system_io_bytes: 0,
        }
    }

    /// Get or create a tracker
    pub fn get_or_create(&mut self, pid: u64) -> &mut ResourceTracker {
        if !self.trackers.contains_key(&pid) && self.trackers.len() < self.max_processes {
            self.trackers.insert(pid, ResourceTracker::new(pid));
        }
        self.trackers
            .entry(pid)
            .or_insert_with(|| ResourceTracker::new(pid))
    }

    /// Get tracker
    pub fn get(&self, pid: u64) -> Option<&ResourceTracker> {
        self.trackers.get(&pid)
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.trackers.remove(&pid);
    }

    /// Top N resource consumers
    pub fn top_consumers(&self, n: usize) -> Vec<(u64, f64)> {
        let mut consumers: Vec<(u64, f64)> = self
            .trackers
            .iter()
            .map(|(&pid, t)| (pid, t.resource_score))
            .collect();
        consumers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        consumers.truncate(n);
        consumers
    }

    /// Number of tracked processes
    pub fn tracked_count(&self) -> usize {
        self.trackers.len()
    }
}
