//! # Bridge Memory Mapping Manager
//!
//! Memory mapping syscall bridge management:
//! - mmap/munmap/mremap/mprotect tracking
//! - VMA (Virtual Memory Area) state management
//! - Shared mapping reference counting
//! - CoW (Copy-on-Write) fault tracking
//! - Memory-mapped file association
//! - Address space layout tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// VMA permission flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmaPerms {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
    pub shared: bool,
}

impl VmaPerms {
    #[inline(always)]
    pub fn rwx() -> Self { Self { read: true, write: true, exec: true, shared: false } }
    #[inline(always)]
    pub fn rw() -> Self { Self { read: true, write: true, exec: false, shared: false } }
    #[inline(always)]
    pub fn ro() -> Self { Self { read: true, write: false, exec: false, shared: false } }
    #[inline(always)]
    pub fn rx() -> Self { Self { read: true, write: false, exec: true, shared: false } }
}

/// VMA type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmaType {
    Anonymous,
    FileBacked,
    SharedAnon,
    Stack,
    Heap,
    Vdso,
    Vvar,
    DeviceMapped,
}

/// Virtual Memory Area
#[derive(Debug, Clone)]
pub struct Vma {
    pub start: u64,
    pub end: u64,
    pub perms: VmaPerms,
    pub vma_type: VmaType,
    pub file_inode: Option<u64>,
    pub file_offset: u64,
    pub page_count: u64,
    pub resident_pages: u64,
    pub cow_pages: u64,
    pub swap_pages: u64,
    pub ref_count: u32,
    pub locked: bool,
}

impl Vma {
    pub fn new(start: u64, end: u64, perms: VmaPerms, vtype: VmaType) -> Self {
        Self {
            start,
            end,
            perms,
            vma_type: vtype,
            file_inode: None,
            file_offset: 0,
            page_count: (end - start) / 4096,
            resident_pages: 0,
            cow_pages: 0,
            swap_pages: 0,
            ref_count: 1,
            locked: false,
        }
    }

    #[inline(always)]
    pub fn size(&self) -> u64 { self.end - self.start }
    #[inline(always)]
    pub fn contains(&self, addr: u64) -> bool { addr >= self.start && addr < self.end }

    #[inline(always)]
    pub fn residency_ratio(&self) -> f64 {
        if self.page_count == 0 { return 0.0; }
        self.resident_pages as f64 / self.page_count as f64
    }

    #[inline(always)]
    pub fn is_writable(&self) -> bool { self.perms.write }
    #[inline(always)]
    pub fn is_executable(&self) -> bool { self.perms.exec }
    #[inline(always)]
    pub fn is_shared(&self) -> bool { self.perms.shared }
}

/// Per-process address space
#[derive(Debug, Clone)]
pub struct ProcessAddrSpace {
    pub process_id: u64,
    pub vmas: Vec<Vma>,
    pub total_mapped: u64,
    pub total_resident: u64,
    pub brk: u64,
    pub stack_top: u64,
    pub mmap_base: u64,
    pub mmap_count: u64,
    pub munmap_count: u64,
    pub mprotect_count: u64,
    pub cow_faults: u64,
}

impl ProcessAddrSpace {
    pub fn new(pid: u64) -> Self {
        Self {
            process_id: pid,
            vmas: Vec::new(),
            total_mapped: 0,
            total_resident: 0,
            brk: 0,
            stack_top: 0,
            mmap_base: 0x7f0000000000,
            mmap_count: 0,
            munmap_count: 0,
            mprotect_count: 0,
            cow_faults: 0,
        }
    }

    #[inline]
    pub fn add_vma(&mut self, vma: Vma) {
        self.total_mapped += vma.size();
        self.total_resident += vma.resident_pages * 4096;
        self.vmas.push(vma);
    }

    #[inline(always)]
    pub fn find_vma(&self, addr: u64) -> Option<&Vma> {
        self.vmas.iter().find(|v| v.contains(addr))
    }

    #[inline(always)]
    pub fn find_vma_mut(&mut self, addr: u64) -> Option<&mut Vma> {
        self.vmas.iter_mut().find(|v| v.contains(addr))
    }

    #[inline]
    pub fn remove_vma(&mut self, start: u64) -> Option<Vma> {
        if let Some(idx) = self.vmas.iter().position(|v| v.start == start) {
            let vma = self.vmas.remove(idx);
            self.total_mapped = self.total_mapped.saturating_sub(vma.size());
            Some(vma)
        } else { None }
    }

    #[inline(always)]
    pub fn vma_count(&self) -> usize { self.vmas.len() }

    pub fn fragmentation(&self) -> f64 {
        if self.vmas.len() < 2 { return 0.0; }
        let mut gaps = 0u64;
        let sorted = {
            let mut v: Vec<(u64, u64)> = self.vmas.iter().map(|v| (v.start, v.end)).collect();
            v.sort_by_key(|&(s, _)| s);
            v
        };
        for i in 1..sorted.len() {
            let gap = sorted[i].0.saturating_sub(sorted[i - 1].1);
            gaps += gap;
        }
        let total_span = sorted.last().map(|v| v.1).unwrap_or(0)
            .saturating_sub(sorted.first().map(|v| v.0).unwrap_or(0));
        if total_span == 0 { return 0.0; }
        gaps as f64 / total_span as f64
    }
}

/// Mmap event record
#[derive(Debug, Clone)]
pub struct MmapEvent {
    pub process_id: u64,
    pub event_type: MmapEventType,
    pub addr: u64,
    pub size: u64,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmapEventType {
    Mmap,
    Munmap,
    Mremap,
    Mprotect,
    Mlock,
    Madvise,
}

/// Bridge Memory Mapping stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeMmapMgrStats {
    pub total_processes: usize,
    pub total_vmas: usize,
    pub total_mapped_bytes: u64,
    pub total_resident_bytes: u64,
    pub total_cow_faults: u64,
}

/// Bridge Memory Mapping Manager
#[repr(align(64))]
pub struct BridgeMmapMgr {
    spaces: BTreeMap<u64, ProcessAddrSpace>,
    events: VecDeque<MmapEvent>,
    max_events: usize,
    stats: BridgeMmapMgrStats,
}

impl BridgeMmapMgr {
    pub fn new(max_events: usize) -> Self {
        Self {
            spaces: BTreeMap::new(),
            events: VecDeque::new(),
            max_events,
            stats: BridgeMmapMgrStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.spaces.entry(pid)
            .or_insert_with(|| ProcessAddrSpace::new(pid));
    }

    pub fn mmap(&mut self, pid: u64, addr: u64, size: u64, perms: VmaPerms, vtype: VmaType, now: u64) -> u64 {
        let space = self.spaces.entry(pid)
            .or_insert_with(|| ProcessAddrSpace::new(pid));

        let actual_addr = if addr == 0 {
            let a = space.mmap_base;
            space.mmap_base += size + 4096; // simple bump allocator
            a
        } else { addr };

        let vma = Vma::new(actual_addr, actual_addr + size, perms, vtype);
        space.add_vma(vma);
        space.mmap_count += 1;

        self.emit_event(pid, MmapEventType::Mmap, actual_addr, size, now);
        actual_addr
    }

    #[inline]
    pub fn munmap(&mut self, pid: u64, addr: u64, now: u64) -> bool {
        if let Some(space) = self.spaces.get_mut(&pid) {
            if let Some(vma) = space.remove_vma(addr) {
                space.munmap_count += 1;
                self.emit_event(pid, MmapEventType::Munmap, addr, vma.size(), now);
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn mprotect(&mut self, pid: u64, addr: u64, new_perms: VmaPerms, now: u64) -> bool {
        if let Some(space) = self.spaces.get_mut(&pid) {
            if let Some(vma) = space.find_vma_mut(addr) {
                vma.perms = new_perms;
                space.mprotect_count += 1;
                self.emit_event(pid, MmapEventType::Mprotect, addr, vma.size(), now);
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn record_cow_fault(&mut self, pid: u64, addr: u64) {
        if let Some(space) = self.spaces.get_mut(&pid) {
            space.cow_faults += 1;
            if let Some(vma) = space.find_vma_mut(addr) {
                vma.cow_pages += 1;
            }
        }
    }

    fn emit_event(&mut self, pid: u64, etype: MmapEventType, addr: u64, size: u64, ts: u64) {
        self.events.push_back(MmapEvent {
            process_id: pid,
            event_type: etype,
            addr,
            size,
            timestamp_ns: ts,
        });
        while self.events.len() > self.max_events {
            self.events.pop_front();
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.spaces.len();
        self.stats.total_vmas = self.spaces.values().map(|s| s.vma_count()).sum();
        self.stats.total_mapped_bytes = self.spaces.values().map(|s| s.total_mapped).sum();
        self.stats.total_resident_bytes = self.spaces.values().map(|s| s.total_resident).sum();
        self.stats.total_cow_faults = self.spaces.values().map(|s| s.cow_faults).sum();
    }

    #[inline(always)]
    pub fn addr_space(&self, pid: u64) -> Option<&ProcessAddrSpace> { self.spaces.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &BridgeMmapMgrStats { &self.stats }
}

// ============================================================================
// Merged from mmap_v2_mgr
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmapV2Prot {
    None,
    Read,
    Write,
    Exec,
    ReadWrite,
    ReadExec,
    ReadWriteExec,
}

/// Mapping types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmapV2Type {
    Private,
    Shared,
    Anonymous,
    PrivateAnonymous,
    SharedAnonymous,
    FileBacked,
    HugePages,
    Stack,
}

/// Mapping flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmapV2Flag {
    Fixed,
    FixedNoReplace,
    GrowsDown,
    Locked,
    NoReserve,
    Populate,
    NonBlock,
    Sync,
    Uninitialized,
}

/// A memory-mapped region
#[derive(Debug, Clone)]
pub struct MmapV2Region {
    pub start_addr: u64,
    pub length: u64,
    pub prot: MmapV2Prot,
    pub map_type: MmapV2Type,
    pub flags: u32,
    pub file_offset: u64,
    pub file_fd: Option<u64>,
    pub fault_count: u64,
    pub cow_count: u64,
    pub resident_pages: u64,
    pub created_tick: u64,
}

/// A process's virtual memory map
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MmapV2Space {
    pub pid: u64,
    pub regions: BTreeMap<u64, MmapV2Region>,
    pub total_mapped: u64,
    pub total_resident: u64,
    pub brk_addr: u64,
    pub mmap_base: u64,
    pub stack_top: u64,
}

impl MmapV2Space {
    pub fn new(pid: u64, mmap_base: u64, stack_top: u64) -> Self {
        Self {
            pid,
            regions: BTreeMap::new(),
            total_mapped: 0,
            total_resident: 0,
            brk_addr: mmap_base,
            mmap_base,
            stack_top,
        }
    }

    pub fn map_region(&mut self, addr: u64, length: u64, prot: MmapV2Prot, map_type: MmapV2Type, flags: u32, tick: u64) -> u64 {
        let region = MmapV2Region {
            start_addr: addr,
            length,
            prot,
            map_type,
            flags,
            file_offset: 0,
            file_fd: None,
            fault_count: 0,
            cow_count: 0,
            resident_pages: 0,
            created_tick: tick,
        };
        self.regions.insert(addr, region);
        self.total_mapped += length;
        addr
    }

    #[inline]
    pub fn unmap_region(&mut self, addr: u64) -> Option<MmapV2Region> {
        if let Some(region) = self.regions.remove(&addr) {
            self.total_mapped -= region.length;
            self.total_resident -= region.resident_pages * 4096;
            Some(region)
        } else {
            None
        }
    }

    #[inline]
    pub fn protect(&mut self, addr: u64, prot: MmapV2Prot) -> bool {
        if let Some(region) = self.regions.get_mut(&addr) {
            region.prot = prot;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn fault(&mut self, addr: u64) -> bool {
        for (start, region) in self.regions.iter_mut() {
            if addr >= *start && addr < *start + region.length {
                region.fault_count += 1;
                region.resident_pages += 1;
                self.total_resident += 4096;
                return true;
            }
        }
        false
    }
}

/// Statistics for mmap V2 manager
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MmapV2Stats {
    pub total_maps: u64,
    pub total_unmaps: u64,
    pub total_protects: u64,
    pub total_faults: u64,
    pub cow_faults: u64,
    pub spaces_created: u64,
    pub total_mapped_bytes: u64,
}

/// Main mmap V2 bridge manager
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeMmapV2 {
    spaces: BTreeMap<u64, MmapV2Space>,
    stats: MmapV2Stats,
}

impl BridgeMmapV2 {
    pub fn new() -> Self {
        Self {
            spaces: BTreeMap::new(),
            stats: MmapV2Stats {
                total_maps: 0,
                total_unmaps: 0,
                total_protects: 0,
                total_faults: 0,
                cow_faults: 0,
                spaces_created: 0,
                total_mapped_bytes: 0,
            },
        }
    }

    #[inline(always)]
    pub fn create_space(&mut self, pid: u64, mmap_base: u64, stack_top: u64) {
        self.spaces.insert(pid, MmapV2Space::new(pid, mmap_base, stack_top));
        self.stats.spaces_created += 1;
    }

    #[inline]
    pub fn mmap(&mut self, pid: u64, addr: u64, length: u64, prot: MmapV2Prot, map_type: MmapV2Type, flags: u32, tick: u64) -> Option<u64> {
        if let Some(space) = self.spaces.get_mut(&pid) {
            let result = space.map_region(addr, length, prot, map_type, flags, tick);
            self.stats.total_maps += 1;
            self.stats.total_mapped_bytes += length;
            Some(result)
        } else {
            None
        }
    }

    #[inline]
    pub fn munmap(&mut self, pid: u64, addr: u64) -> bool {
        if let Some(space) = self.spaces.get_mut(&pid) {
            if space.unmap_region(addr).is_some() {
                self.stats.total_unmaps += 1;
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn page_fault(&mut self, pid: u64, addr: u64) -> bool {
        if let Some(space) = self.spaces.get_mut(&pid) {
            if space.fault(addr) {
                self.stats.total_faults += 1;
                return true;
            }
        }
        false
    }

    #[inline(always)]
    pub fn stats(&self) -> &MmapV2Stats {
        &self.stats
    }
}
