//! # Syscall Prefetch Engine
//!
//! Predictive prefetching of resources based on detected patterns:
//! - File content prefetch (read-ahead)
//! - Directory entry prefetch
//! - Socket buffer pre-allocation
//! - Memory region pre-mapping
//! - IPC channel pre-warm
//! - DNS pre-resolution

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// PREFETCH TYPES
// ============================================================================

/// Type of resource to prefetch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefetchType {
    /// File content read-ahead
    FileReadAhead,
    /// Directory entries
    DirectoryEntries,
    /// Socket receive buffer
    SocketBuffer,
    /// Memory page pre-map
    MemoryPreMap,
    /// IPC channel warm-up
    IpcChannelWarm,
    /// DNS pre-resolution
    DnsPreResolve,
    /// Metadata prefetch
    MetadataPrefetch,
    /// Library pre-load
    LibraryPreload,
    /// Block device pre-read
    BlockDeviceRead,
    /// Page cache warm
    PageCacheWarm,
}

/// Priority of prefetch operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrefetchPriority {
    /// Background (idle time only)
    Background = 0,
    /// Low (when resources available)
    Low = 1,
    /// Normal
    Normal = 2,
    /// High (likely needed soon)
    High = 3,
    /// Critical (almost certainly needed)
    Critical = 4,
}

/// A prefetch request
#[derive(Debug, Clone)]
pub struct PrefetchRequest {
    /// Type of prefetch
    pub prefetch_type: PrefetchType,
    /// Priority
    pub priority: PrefetchPriority,
    /// Resource identifier (file descriptor, address, etc.)
    pub resource_id: u64,
    /// Offset to start prefetch
    pub offset: u64,
    /// Size to prefetch
    pub size: u64,
    /// Confidence that this will be needed (0.0 - 1.0)
    pub confidence: f64,
    /// Process ID that will benefit
    pub pid: u64,
    /// Deadline (timestamp by which prefetch must complete)
    pub deadline: u64,
    /// Whether this is speculative
    pub speculative: bool,
}

/// Result of a prefetch operation
#[derive(Debug, Clone, Copy)]
pub struct PrefetchResult {
    /// Whether the prefetch was accepted
    pub accepted: bool,
    /// Whether it was a hit (data already present)
    pub already_present: bool,
    /// Estimated completion time (ns)
    pub est_completion_ns: u64,
    /// Bytes prefetched
    pub bytes: u64,
}

// ============================================================================
// PREFETCH POLICY
// ============================================================================

/// Configuration for the prefetch engine
#[derive(Debug, Clone)]
pub struct PrefetchConfig {
    /// Whether prefetching is enabled
    pub enabled: bool,
    /// Maximum outstanding prefetch requests
    pub max_outstanding: usize,
    /// Maximum memory for prefetch buffers
    pub max_memory_bytes: u64,
    /// Minimum confidence to trigger prefetch
    pub min_confidence: f64,
    /// Whether to allow speculative prefetching
    pub allow_speculative: bool,
    /// Maximum prefetch size (bytes)
    pub max_prefetch_size: u64,
    /// Read-ahead multiplier (e.g., 2x = prefetch twice what was read)
    pub read_ahead_multiplier: f64,
    /// Cool-down period between prefetches for same resource (ms)
    pub cooldown_ms: u64,
}

impl Default for PrefetchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_outstanding: 64,
            max_memory_bytes: 16 * 1024 * 1024, // 16 MB
            min_confidence: 0.5,
            allow_speculative: true,
            max_prefetch_size: 1024 * 1024, // 1 MB
            read_ahead_multiplier: 2.0,
            cooldown_ms: 100,
        }
    }
}

// ============================================================================
// FILE READ-AHEAD ENGINE
// ============================================================================

/// Tracks sequential file access and decides read-ahead
#[derive(Debug)]
pub struct FileReadAhead {
    /// Per-(pid, fd) access tracking
    access_patterns: BTreeMap<(u64, u64), FileAccessTracker>,
    /// Config
    config: PrefetchConfig,
    /// Total prefetch suggestions
    pub suggestions: u64,
    /// Successful prefetches (data was used)
    pub hits: u64,
    /// Wasted prefetches (data was not used)
    pub wastes: u64,
}

/// File access tracker for one (pid, fd) pair
#[derive(Debug, Clone)]
struct FileAccessTracker {
    /// Recent read offsets
    offsets: Vec<u64>,
    /// Recent read sizes
    sizes: Vec<u64>,
    /// Last read-ahead offset
    last_readahead: u64,
    /// Sequential read count
    sequential_count: u64,
    /// Random read count
    random_count: u64,
    /// Detected stride (bytes between reads)
    stride: Option<u64>,
    /// Average read size
    avg_read_size: u64,
    /// Last access timestamp
    last_access: u64,
}

impl FileAccessTracker {
    fn new() -> Self {
        Self {
            offsets: Vec::new(),
            sizes: Vec::new(),
            last_readahead: 0,
            sequential_count: 0,
            random_count: 0,
            stride: None,
            avg_read_size: 4096,
            last_access: 0,
        }
    }

    fn record(&mut self, offset: u64, size: u64, timestamp: u64) {
        if let Some(&last_off) = self.offsets.last() {
            if let Some(&last_size) = self.sizes.last() {
                if offset == last_off + last_size {
                    self.sequential_count += 1;
                } else {
                    self.random_count += 1;
                }

                // Detect stride
                let expected = last_off + last_size;
                if offset >= expected {
                    let gap = offset - expected;
                    if gap > 0 {
                        self.stride = Some(gap + last_size);
                    } else {
                        self.stride = Some(last_size);
                    }
                }
            }
        }

        if self.offsets.len() >= 32 {
            self.offsets.remove(0);
            self.sizes.remove(0);
        }
        self.offsets.push(offset);
        self.sizes.push(size);

        // Update average read size
        let total: u64 = self.sizes.iter().sum();
        self.avg_read_size = total / self.sizes.len() as u64;
        self.last_access = timestamp;
    }

    /// Is the access pattern sequential?
    fn is_sequential(&self) -> bool {
        let total = self.sequential_count + self.random_count;
        if total < 3 {
            return false;
        }
        self.sequential_count as f64 / total as f64 > 0.7
    }

    /// Confidence that next read will follow the pattern
    fn confidence(&self) -> f64 {
        let total = self.sequential_count + self.random_count;
        if total < 2 {
            return 0.0;
        }
        let seq_ratio = self.sequential_count as f64 / total as f64;
        // Scale confidence by number of observations
        let observation_factor = 1.0 - libm::exp(-(total as f64) / 10.0);
        seq_ratio * observation_factor
    }
}

impl FileReadAhead {
    pub fn new(config: PrefetchConfig) -> Self {
        Self {
            access_patterns: BTreeMap::new(),
            config,
            suggestions: 0,
            hits: 0,
            wastes: 0,
        }
    }

    /// Record a file read and possibly generate a prefetch request
    pub fn on_read(
        &mut self,
        pid: u64,
        fd: u64,
        offset: u64,
        size: u64,
        timestamp: u64,
    ) -> Option<PrefetchRequest> {
        if !self.config.enabled {
            return None;
        }

        // Check if this read hit a previous prefetch
        let tracker = self.access_patterns.entry((pid, fd)).or_insert_with(FileAccessTracker::new);

        if tracker.last_readahead > 0 && offset <= tracker.last_readahead {
            self.hits += 1;
        }

        tracker.record(offset, size, timestamp);

        // Decide whether to prefetch
        let confidence = tracker.confidence();
        if confidence < self.config.min_confidence {
            return None;
        }

        if !tracker.is_sequential() {
            return None;
        }

        // Calculate prefetch parameters
        let prefetch_offset = offset + size;
        let prefetch_size = (tracker.avg_read_size as f64 * self.config.read_ahead_multiplier) as u64;
        let prefetch_size = prefetch_size.min(self.config.max_prefetch_size);

        // Don't repeat recent prefetches
        if prefetch_offset <= tracker.last_readahead {
            return None;
        }

        tracker.last_readahead = prefetch_offset + prefetch_size;
        self.suggestions += 1;

        let priority = if confidence > 0.9 {
            PrefetchPriority::High
        } else if confidence > 0.7 {
            PrefetchPriority::Normal
        } else {
            PrefetchPriority::Low
        };

        Some(PrefetchRequest {
            prefetch_type: PrefetchType::FileReadAhead,
            priority,
            resource_id: fd,
            offset: prefetch_offset,
            size: prefetch_size,
            confidence,
            pid,
            deadline: timestamp + 10_000_000, // 10ms deadline
            speculative: confidence < 0.7,
        })
    }

    /// Report that a prefetch was wasted
    pub fn report_waste(&mut self, _pid: u64, _fd: u64) {
        self.wastes += 1;
    }

    /// Remove file tracking
    pub fn close_file(&mut self, pid: u64, fd: u64) {
        self.access_patterns.remove(&(pid, fd));
    }

    /// Remove all tracking for a process
    pub fn remove_process(&mut self, pid: u64) {
        self.access_patterns.retain(|&(p, _), _| p != pid);
    }

    /// Hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.wastes;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

// ============================================================================
// GLOBAL PREFETCH MANAGER
// ============================================================================

/// Manages all prefetch engines
pub struct PrefetchManager {
    /// File read-ahead engine
    pub file_readahead: FileReadAhead,
    /// Pending prefetch queue
    pending: Vec<PrefetchRequest>,
    /// Max queue size
    max_pending: usize,
    /// Config
    config: PrefetchConfig,
    /// Total requests
    pub total_requests: u64,
    /// Accepted requests
    pub accepted_requests: u64,
    /// Rejected requests
    pub rejected_requests: u64,
    /// Current memory usage for prefetch buffers
    pub memory_usage: u64,
}

impl PrefetchManager {
    pub fn new(config: PrefetchConfig) -> Self {
        let file_readahead = FileReadAhead::new(config.clone());
        Self {
            file_readahead,
            pending: Vec::new(),
            max_pending: config.max_outstanding,
            config,
            total_requests: 0,
            accepted_requests: 0,
            rejected_requests: 0,
            memory_usage: 0,
        }
    }

    /// Submit a prefetch request
    pub fn submit(&mut self, request: PrefetchRequest) -> PrefetchResult {
        self.total_requests += 1;

        // Check limits
        if self.pending.len() >= self.max_pending {
            self.rejected_requests += 1;
            return PrefetchResult {
                accepted: false,
                already_present: false,
                est_completion_ns: 0,
                bytes: 0,
            };
        }

        if self.memory_usage + request.size > self.config.max_memory_bytes {
            self.rejected_requests += 1;
            return PrefetchResult {
                accepted: false,
                already_present: false,
                est_completion_ns: 0,
                bytes: 0,
            };
        }

        if !self.config.allow_speculative && request.speculative {
            self.rejected_requests += 1;
            return PrefetchResult {
                accepted: false,
                already_present: false,
                est_completion_ns: 0,
                bytes: 0,
            };
        }

        self.memory_usage += request.size;
        self.accepted_requests += 1;

        let est_ns = self.estimate_completion(&request);
        self.pending.push(request);

        PrefetchResult {
            accepted: true,
            already_present: false,
            est_completion_ns: est_ns,
            bytes: 0,
        }
    }

    fn estimate_completion(&self, request: &PrefetchRequest) -> u64 {
        // Rough estimate based on type
        match request.prefetch_type {
            PrefetchType::FileReadAhead => request.size * 10, // ~10ns/byte from cache
            PrefetchType::DirectoryEntries => 100_000,
            PrefetchType::SocketBuffer => 50_000,
            PrefetchType::MemoryPreMap => request.size / 4096 * 5000, // ~5µs per page
            PrefetchType::PageCacheWarm => request.size * 5,
            _ => 100_000,
        }
    }

    /// Process a syscall and generate prefetch requests
    pub fn on_syscall(
        &mut self,
        pid: u64,
        syscall_type: SyscallType,
        fd: u64,
        offset: u64,
        size: u64,
        timestamp: u64,
    ) {
        match syscall_type {
            SyscallType::Read => {
                if let Some(request) = self.file_readahead.on_read(pid, fd, offset, size, timestamp) {
                    self.submit(request);
                }
            }
            SyscallType::Close => {
                self.file_readahead.close_file(pid, fd);
            }
            _ => {}
        }
    }

    /// Drain expired prefetch requests
    pub fn drain_expired(&mut self, current_time: u64) -> usize {
        let before = self.pending.len();
        self.pending.retain(|r| r.deadline > current_time);
        let removed = before - self.pending.len();
        // Release memory for expired requests
        for _ in 0..removed {
            // Approximate; in practice we'd track per-request
            if self.memory_usage > 0 {
                self.memory_usage = self.memory_usage.saturating_sub(4096);
            }
        }
        removed
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.file_readahead.remove_process(pid);
        self.pending.retain(|r| r.pid != pid);
    }
}
