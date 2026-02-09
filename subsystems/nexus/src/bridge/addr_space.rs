// SPDX-License-Identifier: GPL-2.0
//! Bridge address space manager — kernel↔userspace virtual address space translation and management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Memory region type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    /// Code/text segment
    Code,
    /// Read-only data
    RoData,
    /// Read-write data
    Data,
    /// Stack region
    Stack,
    /// Heap region
    Heap,
    /// Memory-mapped file
    MappedFile,
    /// Anonymous mapping
    Anonymous,
    /// Shared memory
    SharedMem,
    /// Device I/O mapping
    DeviceIo,
    /// Guard page
    Guard,
}

/// Protection flags for a region
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddrProt {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
    pub user: bool,
}

impl AddrProt {
    #[inline(always)]
    pub const fn none() -> Self {
        Self { read: false, write: false, execute: false, user: false }
    }

    #[inline(always)]
    pub const fn ro_user() -> Self {
        Self { read: true, write: false, execute: false, user: true }
    }

    #[inline(always)]
    pub const fn rw_user() -> Self {
        Self { read: true, write: true, execute: false, user: true }
    }

    #[inline(always)]
    pub const fn rx_user() -> Self {
        Self { read: true, write: false, execute: true, user: true }
    }

    #[inline]
    pub fn as_bits(&self) -> u32 {
        let mut bits = 0u32;
        if self.read { bits |= 1; }
        if self.write { bits |= 2; }
        if self.execute { bits |= 4; }
        if self.user { bits |= 8; }
        bits
    }

    #[inline(always)]
    pub fn violates_wx(&self) -> bool {
        self.write && self.execute
    }
}

/// A virtual memory region
#[derive(Debug, Clone)]
pub struct VmaRegion {
    pub start: u64,
    pub end: u64,
    pub prot: AddrProt,
    pub region_type: RegionType,
    pub name: String,
    pub file_offset: u64,
    pub shared: bool,
    pub locked: bool,
    fault_count: u64,
    cow_count: u64,
}

impl VmaRegion {
    pub fn new(start: u64, end: u64, prot: AddrProt, region_type: RegionType, name: String) -> Self {
        Self {
            start,
            end,
            prot,
            region_type,
            name,
            file_offset: 0,
            shared: false,
            locked: false,
            fault_count: 0,
            cow_count: 0,
        }
    }

    #[inline(always)]
    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    #[inline(always)]
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }

    #[inline(always)]
    pub fn overlaps(&self, start: u64, end: u64) -> bool {
        self.start < end && start < self.end
    }

    #[inline(always)]
    pub fn record_fault(&mut self) {
        self.fault_count = self.fault_count.saturating_add(1);
    }

    #[inline(always)]
    pub fn record_cow(&mut self) {
        self.cow_count = self.cow_count.saturating_add(1);
    }

    #[inline(always)]
    pub fn page_count(&self) -> u64 {
        (self.size() + 4095) / 4096
    }
}

/// Address space layout randomization state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AslrPolicy {
    /// No randomization
    Disabled,
    /// Conservative randomization (8 bits)
    Conservative,
    /// Standard randomization (28 bits)
    Standard,
    /// Full randomization (32+ bits)
    Full,
}

impl AslrPolicy {
    #[inline]
    pub fn entropy_bits(&self) -> u32 {
        match self {
            Self::Disabled => 0,
            Self::Conservative => 8,
            Self::Standard => 28,
            Self::Full => 33,
        }
    }
}

/// Per-process address space descriptor
#[derive(Debug)]
pub struct ProcessAddrSpace {
    pub pid: u64,
    pub regions: Vec<VmaRegion>,
    pub aslr: AslrPolicy,
    pub stack_base: u64,
    pub heap_start: u64,
    pub heap_end: u64,
    pub mmap_base: u64,
    total_mapped: u64,
    total_resident: u64,
    split_count: u64,
    merge_count: u64,
}

impl ProcessAddrSpace {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            regions: Vec::new(),
            aslr: AslrPolicy::Standard,
            stack_base: 0x7FFF_FFFF_F000,
            heap_start: 0x0060_0000,
            heap_end: 0x0060_0000,
            mmap_base: 0x7F00_0000_0000,
            total_mapped: 0,
            total_resident: 0,
            split_count: 0,
            merge_count: 0,
        }
    }

    #[inline(always)]
    pub fn find_region(&self, addr: u64) -> Option<usize> {
        self.regions.iter().position(|r| r.contains(addr))
    }

    pub fn find_free_space(&self, size: u64, hint: u64) -> Option<u64> {
        let aligned_size = (size + 4095) & !4095;
        // Try hint first
        if hint > 0 {
            let hint_end = hint.checked_add(aligned_size)?;
            let overlaps = self.regions.iter().any(|r| r.overlaps(hint, hint_end));
            if !overlaps {
                return Some(hint);
            }
        }
        // Search from mmap_base downward
        let mut candidate = self.mmap_base;
        for region in self.regions.iter().rev() {
            if region.end <= candidate.saturating_sub(aligned_size) {
                return Some(candidate.saturating_sub(aligned_size));
            }
            candidate = region.start;
        }
        None
    }

    #[inline(always)]
    pub fn total_virtual_size(&self) -> u64 {
        self.total_mapped
    }

    #[inline(always)]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    pub fn fragmentation_ratio(&self) -> f64 {
        if self.regions.len() < 2 {
            return 0.0;
        }
        let mut gaps = 0u64;
        for i in 1..self.regions.len() {
            gaps = gaps.saturating_add(
                self.regions[i].start.saturating_sub(self.regions[i - 1].end),
            );
        }
        if self.total_mapped == 0 {
            return 0.0;
        }
        gaps as f64 / self.total_mapped as f64
    }
}

/// Kernel↔user address translation record
#[derive(Debug, Clone)]
pub struct AddrTranslation {
    pub user_addr: u64,
    pub kernel_addr: u64,
    pub size: u64,
    pub writable: bool,
    pub pinned: bool,
    pub timestamp_ns: u64,
}

/// Bridge address space stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AddrSpaceStats {
    pub total_processes: u64,
    pub total_regions: u64,
    pub total_mapped_bytes: u64,
    pub translations_performed: u64,
    pub faults_handled: u64,
    pub splits_performed: u64,
    pub merges_performed: u64,
}

/// Main bridge address space manager
#[repr(align(64))]
pub struct BridgeAddrSpace {
    spaces: BTreeMap<u64, ProcessAddrSpace>,
    translations: Vec<AddrTranslation>,
    enforce_wx: bool,
    max_regions_per_process: usize,
    stats: AddrSpaceStats,
}

impl BridgeAddrSpace {
    pub fn new() -> Self {
        Self {
            spaces: BTreeMap::new(),
            translations: Vec::new(),
            enforce_wx: true,
            max_regions_per_process: 65536,
            stats: AddrSpaceStats {
                total_processes: 0,
                total_regions: 0,
                total_mapped_bytes: 0,
                translations_performed: 0,
                faults_handled: 0,
                splits_performed: 0,
                merges_performed: 0,
            },
        }
    }

    #[inline]
    pub fn create_space(&mut self, pid: u64) -> bool {
        if self.spaces.contains_key(&pid) {
            return false;
        }
        self.spaces.insert(pid, ProcessAddrSpace::new(pid));
        self.stats.total_processes += 1;
        true
    }

    pub fn destroy_space(&mut self, pid: u64) -> bool {
        if let Some(space) = self.spaces.remove(&pid) {
            self.stats.total_regions = self.stats.total_regions.saturating_sub(space.regions.len() as u64);
            self.stats.total_mapped_bytes = self.stats.total_mapped_bytes.saturating_sub(space.total_mapped);
            self.stats.total_processes = self.stats.total_processes.saturating_sub(1);
            self.translations.retain(|t| {
                !space.regions.iter().any(|r| r.contains(t.user_addr))
            });
            true
        } else {
            false
        }
    }

    pub fn map_region(
        &mut self,
        pid: u64,
        start: u64,
        size: u64,
        prot: AddrProt,
        region_type: RegionType,
        name: String,
    ) -> Option<u64> {
        if self.enforce_wx && prot.violates_wx() {
            return None;
        }
        let space = self.spaces.get_mut(&pid)?;
        if space.regions.len() >= self.max_regions_per_process {
            return None;
        }
        let aligned_size = (size + 4095) & !4095;
        let actual_start = if start == 0 {
            space.find_free_space(aligned_size, 0)?
        } else {
            let aligned = start & !4095;
            let end = aligned.checked_add(aligned_size)?;
            if space.regions.iter().any(|r| r.overlaps(aligned, end)) {
                return None;
            }
            aligned
        };
        let region = VmaRegion::new(actual_start, actual_start + aligned_size, prot, region_type, name);
        space.total_mapped = space.total_mapped.saturating_add(aligned_size);
        space.regions.push(region);
        space.regions.sort_by_key(|r| r.start);
        self.stats.total_regions += 1;
        self.stats.total_mapped_bytes = self.stats.total_mapped_bytes.saturating_add(aligned_size);
        Some(actual_start)
    }

    pub fn unmap_region(&mut self, pid: u64, addr: u64) -> bool {
        let space = self.spaces.get_mut(&pid);
        if space.is_none() {
            return false;
        }
        let space = space.unwrap();
        if let Some(idx) = space.find_region(addr) {
            let size = space.regions[idx].size();
            space.regions.remove(idx);
            space.total_mapped = space.total_mapped.saturating_sub(size);
            self.stats.total_regions = self.stats.total_regions.saturating_sub(1);
            self.stats.total_mapped_bytes = self.stats.total_mapped_bytes.saturating_sub(size);
            true
        } else {
            false
        }
    }

    pub fn translate_user_to_kernel(
        &mut self,
        pid: u64,
        user_addr: u64,
        size: u64,
        write: bool,
    ) -> Option<u64> {
        let space = self.spaces.get(&pid)?;
        let idx = space.find_region(user_addr)?;
        let region = &space.regions[idx];
        if write && !region.prot.write {
            return None;
        }
        if !region.prot.read {
            return None;
        }
        if user_addr.checked_add(size)? > region.end {
            return None;
        }
        // Simulate kernel mapping: offset from region start + a kernel base
        let offset = user_addr - region.start;
        let kernel_base = 0xFFFF_8800_0000_0000u64;
        let kernel_addr = kernel_base.wrapping_add(region.start).wrapping_add(offset);
        self.translations.push(AddrTranslation {
            user_addr,
            kernel_addr,
            size,
            writable: write,
            pinned: false,
            timestamp_ns: 0,
        });
        self.stats.translations_performed += 1;
        Some(kernel_addr)
    }

    pub fn handle_fault(&mut self, pid: u64, addr: u64, is_write: bool) -> bool {
        let space = self.spaces.get_mut(&pid);
        if space.is_none() {
            return false;
        }
        let space = space.unwrap();
        if let Some(idx) = space.find_region(addr) {
            let region = &mut space.regions[idx];
            if is_write && !region.prot.write && region.shared {
                // COW fault
                region.record_cow();
            }
            region.record_fault();
            self.stats.faults_handled += 1;
            true
        } else {
            false // SIGSEGV
        }
    }

    pub fn split_region(&mut self, pid: u64, addr: u64, split_at: u64) -> bool {
        let space = self.spaces.get_mut(&pid);
        if space.is_none() {
            return false;
        }
        let space = space.unwrap();
        if let Some(idx) = space.find_region(split_at) {
            if space.regions[idx].start == split_at || space.regions[idx].end == split_at {
                return false;
            }
            let original = &space.regions[idx];
            if !original.contains(split_at) || addr != original.start {
                return false;
            }
            let second_half = VmaRegion::new(
                split_at,
                original.end,
                original.prot,
                original.region_type,
                original.name.clone(),
            );
            space.regions[idx].end = split_at;
            space.regions.insert(idx + 1, second_half);
            space.split_count += 1;
            self.stats.splits_performed += 1;
            self.stats.total_regions += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn set_aslr_policy(&mut self, pid: u64, policy: AslrPolicy) -> bool {
        if let Some(space) = self.spaces.get_mut(&pid) {
            space.aslr = policy;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn process_stats(&self, pid: u64) -> Option<(u64, usize, f64)> {
        let space = self.spaces.get(&pid)?;
        Some((space.total_mapped, space.regions.len(), space.fragmentation_ratio()))
    }

    #[inline(always)]
    pub fn stats(&self) -> &AddrSpaceStats {
        &self.stats
    }
}
