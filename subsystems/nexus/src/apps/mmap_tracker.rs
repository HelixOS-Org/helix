//! # Application Memory Mapping Tracker
//!
//! Track memory-mapped regions per application:
//! - mmap/munmap tracking
//! - Region classification
//! - Overlap detection
//! - Virtual address space fragmentation
//! - Shared mapping analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// MAPPING TYPES
// ============================================================================

/// Mapping protection flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MmapProtection {
    /// Readable
    pub read: bool,
    /// Writable
    pub write: bool,
    /// Executable
    pub exec: bool,
}

impl MmapProtection {
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            exec: false,
        }
    }

    pub fn read_write() -> Self {
        Self {
            read: true,
            write: true,
            exec: false,
        }
    }

    pub fn read_exec() -> Self {
        Self {
            read: true,
            write: false,
            exec: true,
        }
    }

    pub fn rwx() -> Self {
        Self {
            read: true,
            write: true,
            exec: true,
        }
    }

    /// Is writable and executable (security concern)
    pub fn is_wx(&self) -> bool {
        self.write && self.exec
    }
}

/// Mapping type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MmapType {
    /// Anonymous private (heap/stack extension)
    AnonPrivate,
    /// Anonymous shared (IPC)
    AnonShared,
    /// File-backed private (code/data)
    FilePrivate,
    /// File-backed shared (shared lib/data)
    FileShared,
    /// Device mapping
    Device,
    /// Stack
    Stack,
    /// vDSO
    Vdso,
}

impl MmapType {
    /// Is shared?
    pub fn is_shared(&self) -> bool {
        matches!(self, Self::AnonShared | Self::FileShared)
    }

    /// Is file-backed?
    pub fn is_file_backed(&self) -> bool {
        matches!(self, Self::FilePrivate | Self::FileShared)
    }
}

/// Mapping flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MmapFlags {
    /// Fixed address
    pub fixed: bool,
    /// Populate/prefault
    pub populate: bool,
    /// Don't reserve swap
    pub noreserve: bool,
    /// Huge pages
    pub hugetlb: bool,
    /// Lock in memory
    pub locked: bool,
}

impl MmapFlags {
    pub fn default_flags() -> Self {
        Self {
            fixed: false,
            populate: false,
            noreserve: false,
            hugetlb: false,
            locked: false,
        }
    }
}

// ============================================================================
// MEMORY REGION
// ============================================================================

/// A mapped memory region
#[derive(Debug, Clone)]
pub struct MmapRegion {
    /// Start address
    pub start: u64,
    /// End address (exclusive)
    pub end: u64,
    /// Protection
    pub prot: MmapProtection,
    /// Type
    pub map_type: MmapType,
    /// Flags
    pub flags: MmapFlags,
    /// Resident pages
    pub resident_pages: u64,
    /// Dirty pages
    pub dirty_pages: u64,
    /// Created timestamp
    pub created_at: u64,
    /// Last accessed
    pub last_access: u64,
    /// Access count
    pub access_count: u64,
    /// Backing file identifier (hash)
    pub file_id: Option<u64>,
}

impl MmapRegion {
    pub fn new(
        start: u64,
        size: u64,
        prot: MmapProtection,
        map_type: MmapType,
        now: u64,
    ) -> Self {
        Self {
            start,
            end: start + size,
            prot,
            map_type,
            flags: MmapFlags::default_flags(),
            resident_pages: 0,
            dirty_pages: 0,
            created_at: now,
            last_access: now,
            access_count: 0,
            file_id: None,
        }
    }

    /// Size in bytes
    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    /// Size in pages (4K)
    pub fn pages(&self) -> u64 {
        (self.size() + 4095) / 4096
    }

    /// Residency ratio
    pub fn residency(&self) -> f64 {
        let pages = self.pages();
        if pages == 0 {
            return 0.0;
        }
        self.resident_pages as f64 / pages as f64
    }

    /// Dirty ratio
    pub fn dirty_ratio(&self) -> f64 {
        if self.resident_pages == 0 {
            return 0.0;
        }
        self.dirty_pages as f64 / self.resident_pages as f64
    }

    /// Overlaps with another region?
    pub fn overlaps(&self, other: &MmapRegion) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Contains address?
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }

    /// Record access
    pub fn record_access(&mut self, now: u64) {
        self.access_count += 1;
        self.last_access = now;
    }

    /// Security concern: W+X
    pub fn is_security_concern(&self) -> bool {
        self.prot.is_wx()
    }
}

// ============================================================================
// ADDRESS SPACE MAP
// ============================================================================

/// Virtual address space statistics
#[derive(Debug, Clone)]
pub struct VasStats {
    /// Total mapped bytes
    pub mapped_bytes: u64,
    /// Region count
    pub region_count: usize,
    /// Largest gap between regions
    pub largest_gap: u64,
    /// Total gap bytes
    pub total_gaps: u64,
    /// Fragmentation score (0-1)
    pub fragmentation: f64,
}

/// Process address space
#[derive(Debug)]
pub struct ProcessAddressSpace {
    /// Process id
    pub pid: u64,
    /// Regions sorted by start address
    regions: Vec<MmapRegion>,
    /// Total mapped
    total_mapped: u64,
}

impl ProcessAddressSpace {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            regions: Vec::new(),
            total_mapped: 0,
        }
    }

    /// Map a region
    pub fn map_region(&mut self, region: MmapRegion) {
        self.total_mapped += region.size();
        // Insert sorted by start address
        let pos = self
            .regions
            .iter()
            .position(|r| r.start > region.start)
            .unwrap_or(self.regions.len());
        self.regions.insert(pos, region);
    }

    /// Unmap by address range
    pub fn unmap(&mut self, start: u64, size: u64) {
        let end = start + size;
        self.regions.retain(|r| {
            let keep = r.end <= start || r.start >= end;
            if !keep {
                self.total_mapped = self.total_mapped.saturating_sub(r.size());
            }
            keep
        });
    }

    /// Find region containing address
    pub fn find_region(&self, addr: u64) -> Option<&MmapRegion> {
        self.regions.iter().find(|r| r.contains(addr))
    }

    /// Find overlapping regions
    pub fn find_overlaps(&self) -> Vec<(usize, usize)> {
        let mut overlaps = Vec::new();
        for i in 0..self.regions.len() {
            for j in (i + 1)..self.regions.len() {
                if self.regions[i].overlaps(&self.regions[j]) {
                    overlaps.push((i, j));
                }
            }
        }
        overlaps
    }

    /// Shared regions
    pub fn shared_regions(&self) -> Vec<&MmapRegion> {
        self.regions.iter().filter(|r| r.map_type.is_shared()).collect()
    }

    /// W+X regions (security concern)
    pub fn wx_regions(&self) -> Vec<&MmapRegion> {
        self.regions.iter().filter(|r| r.is_security_concern()).collect()
    }

    /// VAS statistics
    pub fn vas_stats(&self) -> VasStats {
        let mapped_bytes = self.regions.iter().map(|r| r.size()).sum::<u64>();
        let mut largest_gap = 0u64;
        let mut total_gaps = 0u64;
        for i in 1..self.regions.len() {
            let gap = self.regions[i].start.saturating_sub(self.regions[i - 1].end);
            total_gaps += gap;
            if gap > largest_gap {
                largest_gap = gap;
            }
        }
        let fragmentation = if mapped_bytes + total_gaps > 0 {
            total_gaps as f64 / (mapped_bytes + total_gaps) as f64
        } else {
            0.0
        };
        VasStats {
            mapped_bytes,
            region_count: self.regions.len(),
            largest_gap,
            total_gaps,
            fragmentation,
        }
    }

    /// Region count
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Total mapped bytes
    pub fn total_mapped(&self) -> u64 {
        self.total_mapped
    }
}

// ============================================================================
// MMAP TRACKER
// ============================================================================

/// Mmap tracker stats
#[derive(Debug, Clone, Default)]
pub struct AppMmapStats {
    /// Processes tracked
    pub processes: usize,
    /// Total regions
    pub total_regions: usize,
    /// Security concerns
    pub wx_regions: usize,
}

/// Application memory mapping tracker
pub struct AppMmapTracker {
    /// Address spaces
    spaces: BTreeMap<u64, ProcessAddressSpace>,
    /// Stats
    stats: AppMmapStats,
}

impl AppMmapTracker {
    pub fn new() -> Self {
        Self {
            spaces: BTreeMap::new(),
            stats: AppMmapStats::default(),
        }
    }

    /// Map region for process
    pub fn map(
        &mut self,
        pid: u64,
        start: u64,
        size: u64,
        prot: MmapProtection,
        map_type: MmapType,
        now: u64,
    ) {
        let space = self
            .spaces
            .entry(pid)
            .or_insert_with(|| ProcessAddressSpace::new(pid));
        let region = MmapRegion::new(start, size, prot, map_type, now);
        space.map_region(region);
        self.update_stats();
    }

    /// Unmap region
    pub fn unmap(&mut self, pid: u64, start: u64, size: u64) {
        if let Some(space) = self.spaces.get_mut(&pid) {
            space.unmap(start, size);
            self.update_stats();
        }
    }

    /// Get address space
    pub fn address_space(&self, pid: u64) -> Option<&ProcessAddressSpace> {
        self.spaces.get(&pid)
    }

    /// Security audit: find all W+X regions
    pub fn security_audit(&self) -> Vec<(u64, usize)> {
        self.spaces
            .values()
            .filter_map(|s| {
                let wx = s.wx_regions().len();
                if wx > 0 {
                    Some((s.pid, wx))
                } else {
                    None
                }
            })
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.processes = self.spaces.len();
        self.stats.total_regions = self.spaces.values().map(|s| s.region_count()).sum();
        self.stats.wx_regions = self
            .spaces
            .values()
            .map(|s| s.wx_regions().len())
            .sum();
    }

    /// Stats
    pub fn stats(&self) -> &AppMmapStats {
        &self.stats
    }
}
