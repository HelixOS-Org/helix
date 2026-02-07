//! # App VMA Tracker
//!
//! Virtual Memory Area (VMA) tracking per application:
//! - VMA layout analysis (heap, stack, mmap, file-backed)
//! - Fragmentation detection
//! - Page fault attribution to VMAs
//! - Growth pattern analysis
//! - VMA merging opportunity detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// VMA TYPES
// ============================================================================

/// VMA type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmaType {
    /// Code/text segment
    Code,
    /// Read-only data
    Rodata,
    /// BSS segment
    Bss,
    /// Heap
    Heap,
    /// Stack
    Stack,
    /// mmap anonymous
    MmapAnon,
    /// mmap file-backed
    MmapFile,
    /// Shared memory
    Shared,
    /// vDSO
    Vdso,
    /// Other
    Other,
}

/// VMA permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmaPerms {
    /// Readable
    pub read: bool,
    /// Writable
    pub write: bool,
    /// Executable
    pub exec: bool,
    /// Shared
    pub shared: bool,
}

impl VmaPerms {
    pub fn rwx() -> Self {
        Self {
            read: true,
            write: true,
            exec: true,
            shared: false,
        }
    }

    pub fn rw() -> Self {
        Self {
            read: true,
            write: true,
            exec: false,
            shared: false,
        }
    }

    pub fn rx() -> Self {
        Self {
            read: true,
            write: false,
            exec: true,
            shared: false,
        }
    }

    pub fn ro() -> Self {
        Self {
            read: true,
            write: false,
            exec: false,
            shared: false,
        }
    }

    /// Security: writable + executable is dangerous
    pub fn is_wx(&self) -> bool {
        self.write && self.exec
    }
}

/// VMA growth pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrowthPattern {
    /// Stable size
    Stable,
    /// Linear growth
    Linear,
    /// Exponential growth
    Exponential,
    /// Oscillating
    Oscillating,
    /// Shrinking
    Shrinking,
}

// ============================================================================
// VMA ENTRY
// ============================================================================

/// Single VMA entry
#[derive(Debug, Clone)]
pub struct VmaEntry {
    /// Start address
    pub start: u64,
    /// End address
    pub end: u64,
    /// VMA type
    pub vma_type: VmaType,
    /// Permissions
    pub perms: VmaPerms,
    /// Resident pages
    pub resident_pages: u64,
    /// Total pages
    pub total_pages: u64,
    /// Page faults
    pub page_faults: u64,
    /// Swap pages
    pub swapped_pages: u64,
    /// Dirty pages
    pub dirty_pages: u64,
    /// Size samples for growth tracking
    size_samples: Vec<u64>,
}

impl VmaEntry {
    pub fn new(start: u64, end: u64, vma_type: VmaType, perms: VmaPerms) -> Self {
        let total = (end.saturating_sub(start)) / 4096;
        Self {
            start,
            end,
            vma_type,
            perms,
            resident_pages: 0,
            total_pages: total,
            page_faults: 0,
            swapped_pages: 0,
            dirty_pages: 0,
            size_samples: Vec::new(),
        }
    }

    /// Size in bytes
    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    /// Residency ratio
    pub fn residency(&self) -> f64 {
        if self.total_pages == 0 {
            return 0.0;
        }
        self.resident_pages as f64 / self.total_pages as f64
    }

    /// Record page fault
    pub fn record_fault(&mut self) {
        self.page_faults += 1;
    }

    /// Sample size for growth tracking
    pub fn sample_size(&mut self) {
        let sz = self.size();
        if self.size_samples.len() >= 64 {
            self.size_samples.remove(0);
        }
        self.size_samples.push(sz);
    }

    /// Detect growth pattern
    pub fn growth_pattern(&self) -> GrowthPattern {
        if self.size_samples.len() < 4 {
            return GrowthPattern::Stable;
        }
        let n = self.size_samples.len();
        let first = self.size_samples[0] as f64;
        let last = self.size_samples[n - 1] as f64;

        if libm::fabs(last - first) < first * 0.05 {
            return GrowthPattern::Stable;
        }

        if last < first * 0.9 {
            return GrowthPattern::Shrinking;
        }

        // Check for oscillation: count direction changes
        let mut changes = 0u32;
        for i in 2..n {
            let d1 = self.size_samples[i - 1] as i64 - self.size_samples[i - 2] as i64;
            let d2 = self.size_samples[i] as i64 - self.size_samples[i - 1] as i64;
            if (d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0) {
                changes += 1;
            }
        }
        if changes as usize > n / 3 {
            return GrowthPattern::Oscillating;
        }

        // Check linear vs exponential
        let mid = self.size_samples[n / 2] as f64;
        let linear_mid = (first + last) / 2.0;
        if mid > linear_mid * 1.2 {
            GrowthPattern::Exponential
        } else {
            GrowthPattern::Linear
        }
    }

    /// Can merge with adjacent VMA
    pub fn can_merge_with(&self, other: &VmaEntry) -> bool {
        self.end == other.start
            && self.vma_type == other.vma_type
            && self.perms.read == other.perms.read
            && self.perms.write == other.perms.write
            && self.perms.exec == other.perms.exec
            && self.perms.shared == other.perms.shared
    }
}

// ============================================================================
// FRAGMENTATION ANALYSIS
// ============================================================================

/// Fragmentation report
#[derive(Debug, Clone)]
pub struct FragReport {
    /// Total VMAs
    pub total_vmas: usize,
    /// Total gaps
    pub total_gaps: usize,
    /// Largest gap (bytes)
    pub largest_gap: u64,
    /// Total free space
    pub total_free: u64,
    /// Total used space
    pub total_used: u64,
    /// Fragmentation score (0..1, higher = more fragmented)
    pub fragmentation: f64,
    /// Mergeable pairs
    pub mergeable_pairs: usize,
}

// ============================================================================
// PER-PROCESS VMA
// ============================================================================

/// Per-process VMA tracker
#[derive(Debug)]
pub struct ProcessVmaTracker {
    /// PID
    pub pid: u64,
    /// VMAs (sorted by start address)
    vmas: BTreeMap<u64, VmaEntry>,
    /// Total page faults
    pub total_faults: u64,
    /// W^X violations
    pub wx_regions: usize,
}

impl ProcessVmaTracker {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            vmas: BTreeMap::new(),
            total_faults: 0,
            wx_regions: 0,
        }
    }

    /// Add VMA
    pub fn add_vma(&mut self, start: u64, end: u64, vma_type: VmaType, perms: VmaPerms) {
        let entry = VmaEntry::new(start, end, vma_type, perms);
        if perms.is_wx() {
            self.wx_regions += 1;
        }
        self.vmas.insert(start, entry);
    }

    /// Remove VMA
    pub fn remove_vma(&mut self, start: u64) {
        if let Some(v) = self.vmas.remove(&start) {
            if v.perms.is_wx() && self.wx_regions > 0 {
                self.wx_regions -= 1;
            }
        }
    }

    /// Record fault to VMA containing address
    pub fn record_fault(&mut self, addr: u64) {
        self.total_faults += 1;
        // Find VMA containing addr
        for vma in self.vmas.values_mut() {
            if addr >= vma.start && addr < vma.end {
                vma.record_fault();
                return;
            }
        }
    }

    /// Analyze fragmentation
    pub fn analyze_fragmentation(&self) -> FragReport {
        let entries: Vec<&VmaEntry> = self.vmas.values().collect();
        let mut gaps = 0usize;
        let mut largest_gap = 0u64;
        let mut total_free = 0u64;
        let mut mergeable = 0usize;

        for i in 1..entries.len() {
            let gap = entries[i].start.saturating_sub(entries[i - 1].end);
            if gap > 0 {
                gaps += 1;
                total_free += gap;
                if gap > largest_gap {
                    largest_gap = gap;
                }
            }
            if entries[i - 1].can_merge_with(entries[i]) {
                mergeable += 1;
            }
        }

        let total_used: u64 = entries.iter().map(|v| v.size()).sum();
        let fragmentation = if total_free + total_used > 0 {
            if gaps == 0 {
                0.0
            } else {
                1.0 - (largest_gap as f64 / total_free.max(1) as f64)
            }
        } else {
            0.0
        };

        FragReport {
            total_vmas: entries.len(),
            total_gaps: gaps,
            largest_gap,
            total_free,
            total_used,
            fragmentation,
            mergeable_pairs: mergeable,
        }
    }

    /// Heap VMAs
    pub fn heap_size(&self) -> u64 {
        self.vmas
            .values()
            .filter(|v| v.vma_type == VmaType::Heap)
            .map(|v| v.size())
            .sum()
    }

    /// Stack VMAs
    pub fn stack_size(&self) -> u64 {
        self.vmas
            .values()
            .filter(|v| v.vma_type == VmaType::Stack)
            .map(|v| v.size())
            .sum()
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// VMA tracker stats
#[derive(Debug, Clone, Default)]
pub struct AppVmaTrackerStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Total VMAs
    pub total_vmas: usize,
    /// W^X violations
    pub wx_violations: usize,
    /// Total page faults
    pub total_faults: u64,
}

/// App VMA tracker engine
pub struct AppVmaTracker {
    /// Per-process trackers
    processes: BTreeMap<u64, ProcessVmaTracker>,
    /// Stats
    stats: AppVmaTrackerStats,
}

impl AppVmaTracker {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppVmaTrackerStats::default(),
        }
    }

    /// Get/create process
    pub fn process(&mut self, pid: u64) -> &mut ProcessVmaTracker {
        self.processes
            .entry(pid)
            .or_insert_with(|| ProcessVmaTracker::new(pid))
    }

    /// Add VMA
    pub fn add_vma(&mut self, pid: u64, start: u64, end: u64, vma_type: VmaType, perms: VmaPerms) {
        let proc = self
            .processes
            .entry(pid)
            .or_insert_with(|| ProcessVmaTracker::new(pid));
        proc.add_vma(start, end, vma_type, perms);
        self.update_stats();
    }

    /// Record fault
    pub fn record_fault(&mut self, pid: u64, addr: u64) {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.record_fault(addr);
            self.stats.total_faults += 1;
        }
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_vmas = self.processes.values().map(|p| p.vmas.len()).sum();
        self.stats.wx_violations = self.processes.values().map(|p| p.wx_regions).sum();
    }

    /// Stats
    pub fn stats(&self) -> &AppVmaTrackerStats {
        &self.stats
    }
}
