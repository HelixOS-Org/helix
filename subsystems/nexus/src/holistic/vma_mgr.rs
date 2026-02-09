// SPDX-License-Identifier: GPL-2.0
//! Holistic vma_mgr — Virtual Memory Area management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// VMA flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VmaFlags(pub u64);

impl VmaFlags {
    pub const READ: u64 = 1 << 0;
    pub const WRITE: u64 = 1 << 1;
    pub const EXEC: u64 = 1 << 2;
    pub const SHARED: u64 = 1 << 3;
    pub const GROWSDOWN: u64 = 1 << 4;
    pub const HUGETLB: u64 = 1 << 5;
    pub const LOCKED: u64 = 1 << 6;
    pub const IO: u64 = 1 << 7;
    pub const DONTCOPY: u64 = 1 << 8;
    pub const DONTEXPAND: u64 = 1 << 9;
    pub const MERGEABLE: u64 = 1 << 10;

    pub fn new() -> Self { Self(0) }
    #[inline(always)]
    pub fn set(&mut self, f: u64) { self.0 |= f; }
    #[inline(always)]
    pub fn has(&self, f: u64) -> bool { self.0 & f != 0 }
}

/// VMA type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmaType {
    Anonymous,
    FileBacked,
    Shared,
    Stack,
    Heap,
    Vdso,
    Vsyscall,
}

/// Virtual Memory Area
#[derive(Debug)]
pub struct Vma {
    pub start: u64,
    pub end: u64,
    pub flags: VmaFlags,
    pub vma_type: VmaType,
    pub file_offset: u64,
    pub inode: u64,
    pub pgoff: u64,
    pub page_count: u64,
    pub rss_pages: u64,
    pub swap_pages: u64,
}

impl Vma {
    pub fn new(start: u64, end: u64, flags: VmaFlags, vtype: VmaType) -> Self {
        Self { start, end, flags, vma_type: vtype, file_offset: 0, inode: 0, pgoff: 0, page_count: 0, rss_pages: 0, swap_pages: 0 }
    }

    #[inline(always)]
    pub fn size(&self) -> u64 { self.end - self.start }
    #[inline(always)]
    pub fn pages(&self) -> u64 { self.size() / 4096 }
    #[inline(always)]
    pub fn overlaps(&self, start: u64, end: u64) -> bool { self.start < end && start < self.end }
    #[inline(always)]
    pub fn can_merge(&self, other: &Vma) -> bool { self.end == other.start && self.flags.0 == other.flags.0 && self.vma_type == other.vma_type }
}

/// Process MM
#[derive(Debug)]
pub struct ProcessMm {
    pub pid: u64,
    pub vmas: Vec<Vma>,
    pub total_vm: u64,
    pub rss: u64,
    pub brk_start: u64,
    pub brk_end: u64,
    pub stack_start: u64,
    pub mmap_base: u64,
}

impl ProcessMm {
    pub fn new(pid: u64) -> Self {
        Self { pid, vmas: Vec::new(), total_vm: 0, rss: 0, brk_start: 0, brk_end: 0, stack_start: 0, mmap_base: 0x7f0000000000 }
    }

    #[inline(always)]
    pub fn add_vma(&mut self, vma: Vma) { self.total_vm += vma.size(); self.vmas.push(vma); }
    #[inline(always)]
    pub fn vma_count(&self) -> u32 { self.vmas.len() as u32 }

    #[inline(always)]
    pub fn find_vma(&self, addr: u64) -> Option<&Vma> {
        self.vmas.iter().find(|v| v.start <= addr && addr < v.end)
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct VmaMgrStats {
    pub total_processes: u32,
    pub total_vmas: u32,
    pub total_vm_bytes: u64,
    pub total_rss_bytes: u64,
    pub avg_vmas_per_process: f64,
}

/// Main VMA manager
pub struct HolisticVmaMgr {
    processes: BTreeMap<u64, ProcessMm>,
}

impl HolisticVmaMgr {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }
    #[inline(always)]
    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessMm::new(pid)); }

    #[inline(always)]
    pub fn mmap(&mut self, pid: u64, start: u64, end: u64, flags: VmaFlags, vtype: VmaType) {
        if let Some(mm) = self.processes.get_mut(&pid) { mm.add_vma(Vma::new(start, end, flags, vtype)); }
    }

    #[inline]
    pub fn stats(&self) -> VmaMgrStats {
        let vmas: u32 = self.processes.values().map(|mm| mm.vma_count()).sum();
        let vm: u64 = self.processes.values().map(|mm| mm.total_vm).sum();
        let rss: u64 = self.processes.values().map(|mm| mm.rss).sum();
        let avg = if self.processes.is_empty() { 0.0 } else { vmas as f64 / self.processes.len() as f64 };
        VmaMgrStats { total_processes: self.processes.len() as u32, total_vmas: vmas, total_vm_bytes: vm, total_rss_bytes: rss, avg_vmas_per_process: avg }
    }
}
