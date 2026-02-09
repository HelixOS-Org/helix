//! # Syscall Coalescing Engine
//!
//! Merges multiple syscalls into optimized batches:
//! - I/O coalescing (sequential reads/writes)
//! - Metadata coalescing (stat, lstat)
//! - Network coalescing (small sends)
//! - Memory coalescing (adjacent mmaps)
//! - Timer coalescing (wake-up alignment)
//! - Coalescing window management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// COALESCING TYPES
// ============================================================================

/// Coalescing category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoalesceCategory {
    /// Sequential reads to same fd
    SequentialRead,
    /// Sequential writes to same fd
    SequentialWrite,
    /// Multiple small writes (send coalescing)
    SmallWrite,
    /// File metadata ops (stat, fstat, lstat)
    Metadata,
    /// Adjacent mmap regions
    MemoryMap,
    /// Timer-related syscalls
    Timer,
    /// Signal-related syscalls
    Signal,
    /// Inotify/epoll operations
    EventMonitor,
    /// Socket connect/accept
    NetworkSetup,
    /// Close operations
    Close,
}

/// Coalescing state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalesceState {
    /// Window open, collecting calls
    Collecting,
    /// Window closed, ready to flush
    Ready,
    /// Being executed
    Executing,
    /// Complete
    Complete,
    /// Cancelled
    Cancelled,
}

// ============================================================================
// COALESCE ENTRY
// ============================================================================

/// A single pending syscall in the coalesce window
#[derive(Debug, Clone)]
pub struct PendingSyscall {
    /// Syscall number
    pub syscall_nr: u32,
    /// Process ID
    pub pid: u64,
    /// File descriptor (if applicable)
    pub fd: Option<i32>,
    /// Buffer pointer (for I/O)
    pub buffer: u64,
    /// Size
    pub size: u64,
    /// Offset (for positioned I/O)
    pub offset: Option<u64>,
    /// Timestamp when queued
    pub queued_at: u64,
    /// Priority
    pub priority: u32,
}

/// Coalesced batch result
#[derive(Debug, Clone)]
pub struct CoalescedBatch {
    /// Category
    pub category: CoalesceCategory,
    /// Combined syscalls
    pub entries: Vec<PendingSyscall>,
    /// Total combined size
    pub total_size: u64,
    /// First offset
    pub start_offset: u64,
    /// Expected savings (percent)
    pub savings_estimate: u32,
    /// Batch ID
    pub batch_id: u64,
}

// ============================================================================
// COALESCE WINDOW
// ============================================================================

/// Window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Maximum window duration (microseconds)
    pub max_duration_us: u64,
    /// Maximum entries in window
    pub max_entries: usize,
    /// Maximum total size (bytes)
    pub max_total_size: u64,
    /// Minimum entries to trigger coalescing
    pub min_entries: usize,
    /// Category
    pub category: CoalesceCategory,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            max_duration_us: 1000,
            max_entries: 32,
            max_total_size: 1024 * 1024,
            min_entries: 2,
            category: CoalesceCategory::SequentialRead,
        }
    }
}

/// Coalesce window
pub struct CoalesceWindow {
    /// Configuration
    config: WindowConfig,
    /// Pending syscalls
    pending: Vec<PendingSyscall>,
    /// State
    state: CoalesceState,
    /// Window open time
    opened_at: u64,
    /// Total accumulated size
    total_size: u64,
}

impl CoalesceWindow {
    pub fn new(config: WindowConfig) -> Self {
        Self {
            config,
            pending: Vec::new(),
            state: CoalesceState::Collecting,
            opened_at: 0,
            total_size: 0,
        }
    }

    /// Add a syscall to the window
    pub fn add(&mut self, entry: PendingSyscall, timestamp: u64) -> bool {
        if self.state != CoalesceState::Collecting {
            return false;
        }

        if self.pending.is_empty() {
            self.opened_at = timestamp;
        }

        // Check limits
        if self.pending.len() >= self.config.max_entries {
            return false;
        }
        if self.total_size + entry.size > self.config.max_total_size {
            return false;
        }

        self.total_size += entry.size;
        self.pending.push(entry);
        true
    }

    /// Check if window should be flushed
    pub fn should_flush(&self, current_time: u64) -> bool {
        if self.pending.is_empty() {
            return false;
        }

        // Time expired
        if current_time.saturating_sub(self.opened_at) >= self.config.max_duration_us {
            return true;
        }

        // Full
        if self.pending.len() >= self.config.max_entries {
            return true;
        }

        // Size limit
        if self.total_size >= self.config.max_total_size {
            return true;
        }

        false
    }

    /// Flush window and produce batch
    pub fn flush(&mut self, batch_id: u64) -> Option<CoalescedBatch> {
        if self.pending.len() < self.config.min_entries {
            // Not enough to coalesce, return entries individually
            self.state = CoalesceState::Cancelled;
            return None;
        }

        self.state = CoalesceState::Ready;

        let start_offset = self.pending.first().and_then(|e| e.offset).unwrap_or(0);

        let savings = self.estimate_savings();

        let batch = CoalescedBatch {
            category: self.config.category,
            entries: core::mem::take(&mut self.pending),
            total_size: self.total_size,
            start_offset,
            savings_estimate: savings,
            batch_id,
        };

        self.total_size = 0;
        self.state = CoalesceState::Complete;

        Some(batch)
    }

    /// Estimate savings from coalescing
    fn estimate_savings(&self) -> u32 {
        let n = self.pending.len() as u32;
        if n <= 1 {
            return 0;
        }

        // Savings depend on category
        match self.config.category {
            CoalesceCategory::SequentialRead | CoalesceCategory::SequentialWrite => {
                // Sequential I/O: ~50-80% reduction in syscall overhead
                50 + (n.min(10) * 3)
            },
            CoalesceCategory::SmallWrite => {
                // Small writes: high savings from Nagle-like coalescing
                60 + (n.min(20) * 2)
            },
            CoalesceCategory::Metadata => {
                // Metadata: moderate savings
                30 + (n.min(8) * 5)
            },
            CoalesceCategory::MemoryMap => {
                // mmap: moderate savings, reduced TLB pressure
                40 + (n.min(5) * 8)
            },
            CoalesceCategory::Timer => {
                // Timer: good savings from alignment
                40 + (n.min(10) * 4)
            },
            _ => 20 + (n.min(5) * 5),
        }
    }

    /// Reset window for reuse
    #[inline]
    pub fn reset(&mut self) {
        self.pending.clear();
        self.total_size = 0;
        self.state = CoalesceState::Collecting;
        self.opened_at = 0;
    }

    /// Pending count
    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// State
    #[inline(always)]
    pub fn state(&self) -> CoalesceState {
        self.state
    }
}

// ============================================================================
// I/O COALESCING LOGIC
// ============================================================================

/// Determine if two I/O operations can be merged
pub fn can_merge_io(a: &PendingSyscall, b: &PendingSyscall) -> bool {
    // Same fd
    if a.fd != b.fd || a.fd.is_none() {
        return false;
    }

    // Same process
    if a.pid != b.pid {
        return false;
    }

    // Check sequential
    if let (Some(off_a), Some(off_b)) = (a.offset, b.offset) {
        let end_a = off_a + a.size;
        // B starts where A ends (or overlaps slightly)
        if off_b >= off_a && off_b <= end_a + 4096 {
            return true;
        }
        // A starts where B ends
        let end_b = off_b + b.size;
        if off_a >= off_b && off_a <= end_b + 4096 {
            return true;
        }
    }

    false
}

/// Merge two I/O operations into one
pub fn merge_io(a: &PendingSyscall, b: &PendingSyscall) -> Option<PendingSyscall> {
    if !can_merge_io(a, b) {
        return None;
    }

    let (off_a, off_b) = match (a.offset, b.offset) {
        (Some(oa), Some(ob)) => (oa, ob),
        _ => return None,
    };

    let start = off_a.min(off_b);
    let end = (off_a + a.size).max(off_b + b.size);

    Some(PendingSyscall {
        syscall_nr: a.syscall_nr,
        pid: a.pid,
        fd: a.fd,
        buffer: a.buffer, // Use first buffer
        size: end - start,
        offset: Some(start),
        queued_at: a.queued_at.min(b.queued_at),
        priority: a.priority.max(b.priority),
    })
}

// ============================================================================
// COALESCING ENGINE
// ============================================================================

/// Coalescing statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoalesceStats {
    /// Total syscalls seen
    pub total_seen: u64,
    /// Total coalesced
    pub total_coalesced: u64,
    /// Total batches produced
    pub total_batches: u64,
    /// Total bytes coalesced
    pub total_bytes_coalesced: u64,
    /// Average batch size
    pub avg_batch_size: f64,
    /// Syscalls saved
    pub syscalls_saved: u64,
}

/// Main coalescing engine
#[repr(align(64))]
pub struct CoalesceEngine {
    /// Per-process, per-category windows
    windows: BTreeMap<(u64, u8), CoalesceWindow>,
    /// Default configurations per category
    configs: BTreeMap<u8, WindowConfig>,
    /// Next batch ID
    next_batch_id: u64,
    /// Statistics
    pub stats: CoalesceStats,
    /// Enabled
    pub enabled: bool,
}

impl CoalesceEngine {
    pub fn new() -> Self {
        Self {
            windows: BTreeMap::new(),
            configs: BTreeMap::new(),
            next_batch_id: 1,
            stats: CoalesceStats::default(),
            enabled: true,
        }
    }

    /// Set configuration for category
    #[inline(always)]
    pub fn configure(&mut self, category: CoalesceCategory, config: WindowConfig) {
        self.configs.insert(category as u8, config);
    }

    /// Submit syscall for potential coalescing
    pub fn submit(
        &mut self,
        pid: u64,
        category: CoalesceCategory,
        entry: PendingSyscall,
        timestamp: u64,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        self.stats.total_seen += 1;

        let key = (pid, category as u8);
        let config = self
            .configs
            .get(&(category as u8))
            .cloned()
            .unwrap_or_default();

        let window = self
            .windows
            .entry(key)
            .or_insert_with(|| CoalesceWindow::new(config));

        if window.state() != CoalesceState::Collecting {
            window.reset();
        }

        window.add(entry, timestamp)
    }

    /// Check and flush ready windows
    pub fn flush_ready(&mut self, current_time: u64) -> Vec<CoalescedBatch> {
        let mut batches = Vec::new();

        let keys: Vec<(u64, u8)> = self.windows.keys().copied().collect();

        for key in keys {
            if let Some(window) = self.windows.get_mut(&key) {
                if window.should_flush(current_time) {
                    let batch_id = self.next_batch_id;
                    self.next_batch_id += 1;

                    if let Some(batch) = window.flush(batch_id) {
                        let saved = batch.entries.len().saturating_sub(1) as u64;
                        self.stats.total_coalesced += batch.entries.len() as u64;
                        self.stats.total_batches += 1;
                        self.stats.total_bytes_coalesced += batch.total_size;
                        self.stats.syscalls_saved += saved;

                        if self.stats.total_batches > 0 {
                            self.stats.avg_batch_size =
                                self.stats.total_coalesced as f64 / self.stats.total_batches as f64;
                        }

                        batches.push(batch);
                    }
                    window.reset();
                }
            }
        }

        batches
    }

    /// Force flush all windows
    pub fn flush_all(&mut self) -> Vec<CoalescedBatch> {
        let mut batches = Vec::new();

        for (_, window) in self.windows.iter_mut() {
            if window.pending_count() > 0 {
                let batch_id = self.next_batch_id;
                self.next_batch_id += 1;

                if let Some(batch) = window.flush(batch_id) {
                    batches.push(batch);
                }
                window.reset();
            }
        }

        batches
    }

    /// Window count
    #[inline]
    pub fn active_windows(&self) -> usize {
        self.windows
            .values()
            .filter(|w| w.pending_count() > 0)
            .count()
    }
}
