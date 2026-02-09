//! # Holistic Mmap Advisor
//!
//! Memory mapping advisory and optimization:
//! - VMA (Virtual Memory Area) tracking
//! - Madvise hint processing and enforcement
//! - Transparent huge page eligibility scoring
//! - Address space fragmentation analysis
//! - Mapping merge/split optimization
//! - Memory usage pattern classification

extern crate alloc;

use alloc::collections::BTreeMap;
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
    #[inline]
    pub fn to_bits(&self) -> u8 {
        let mut v = 0u8;
        if self.read { v |= 1; }
        if self.write { v |= 2; }
        if self.exec { v |= 4; }
        if self.shared { v |= 8; }
        v
    }
}

/// Madvise hint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadviseHint {
    Normal,
    Random,
    Sequential,
    WillNeed,
    DontNeed,
    Free,
    Remove,
    DontFork,
    DoFork,
    Hugepage,
    NoHugepage,
    DontDump,
    DoDump,
    Mergeable,
    Unmergeable,
    Cold,
    PageOut,
}

/// VMA type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmaType {
    Anonymous,
    FileBacked,
    Stack,
    Heap,
    SharedMemory,
    DeviceMapping,
    Vdso,
    Vsyscall,
}

/// Virtual Memory Area
#[derive(Debug, Clone)]
pub struct Vma {
    pub start: u64,
    pub end: u64,
    pub perms: VmaPerms,
    pub vma_type: VmaType,
    pub hint: MadviseHint,
    pub file_offset: u64,
    pub resident_pages: u64,
    pub swapped_pages: u64,
    pub fault_count: u64,
    pub thp_eligible: bool,
    pub ksm_mergeable: bool,
    pub access_count: u64,
    pub last_access_ts: u64,
}

impl Vma {
    pub fn new(start: u64, end: u64, perms: VmaPerms, vma_type: VmaType) -> Self {
        Self {
            start, end, perms, vma_type, hint: MadviseHint::Normal,
            file_offset: 0, resident_pages: 0, swapped_pages: 0,
            fault_count: 0, thp_eligible: false, ksm_mergeable: false,
            access_count: 0, last_access_ts: 0,
        }
    }

    #[inline(always)]
    pub fn size(&self) -> u64 { self.end.saturating_sub(self.start) }
    #[inline(always)]
    pub fn pages(&self) -> u64 { self.size() / 4096 }

    #[inline(always)]
    pub fn rss_ratio(&self) -> f64 {
        let total = self.pages();
        if total == 0 { 0.0 } else { self.resident_pages as f64 / total as f64 }
    }

    #[inline(always)]
    pub fn contains(&self, addr: u64) -> bool { addr >= self.start && addr < self.end }

    #[inline(always)]
    pub fn overlaps(&self, other: &Vma) -> bool {
        self.start < other.end && other.start < self.end
    }

    #[inline(always)]
    pub fn can_merge_with(&self, other: &Vma) -> bool {
        self.end == other.start && self.perms.to_bits() == other.perms.to_bits()
            && self.vma_type == other.vma_type && self.hint == other.hint
    }

    pub fn thp_score(&self) -> f64 {
        let size = self.size();
        let mut score = 0.0;
        // Size: 2MB aligned and large enough
        if size >= 2 * 1024 * 1024 { score += 0.3; }
        if self.start % (2 * 1024 * 1024) == 0 { score += 0.2; }
        // Access pattern
        if self.access_count > 100 { score += 0.2; }
        // Anonymous memory prefers THP
        if self.vma_type == VmaType::Anonymous { score += 0.2; }
        // High RSS ratio suggests active usage
        if self.rss_ratio() > 0.8 { score += 0.1; }
        score
    }
}

/// Address space gap
#[derive(Debug, Clone)]
pub struct AddressGap {
    pub start: u64,
    pub end: u64,
}

impl AddressGap {
    #[inline(always)]
    pub fn size(&self) -> u64 { self.end.saturating_sub(self.start) }
}

/// Per-process address space
#[derive(Debug, Clone)]
pub struct ProcessAddressSpace {
    pub pid: u64,
    pub vmas: Vec<Vma>,
    pub total_mapped: u64,
    pub total_resident: u64,
    pub total_swapped: u64,
    pub vma_count: usize,
    pub largest_gap: u64,
    pub fragmentation_score: f64,
}

impl ProcessAddressSpace {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, vmas: Vec::new(), total_mapped: 0, total_resident: 0,
            total_swapped: 0, vma_count: 0, largest_gap: 0,
            fragmentation_score: 0.0,
        }
    }

    #[inline(always)]
    pub fn add_vma(&mut self, vma: Vma) {
        self.vmas.push(vma);
        self.vmas.sort_by_key(|v| v.start);
    }

    #[inline(always)]
    pub fn remove_vma(&mut self, start: u64, end: u64) {
        self.vmas.retain(|v| !(v.start == start && v.end == end));
    }

    #[inline(always)]
    pub fn find_vma(&self, addr: u64) -> Option<&Vma> {
        self.vmas.iter().find(|v| v.contains(addr))
    }

    pub fn apply_madvise(&mut self, start: u64, len: u64, hint: MadviseHint) {
        let end = start + len;
        for vma in &mut self.vmas {
            if vma.start < end && start < vma.end {
                vma.hint = hint;
                match hint {
                    MadviseHint::Hugepage => vma.thp_eligible = true,
                    MadviseHint::NoHugepage => vma.thp_eligible = false,
                    MadviseHint::Mergeable => vma.ksm_mergeable = true,
                    MadviseHint::Unmergeable => vma.ksm_mergeable = false,
                    _ => {}
                }
            }
        }
    }

    pub fn merge_adjacent(&mut self) {
        if self.vmas.len() < 2 { return; }
        let mut merged = Vec::new();
        let mut current = self.vmas[0].clone();
        for i in 1..self.vmas.len() {
            if current.can_merge_with(&self.vmas[i]) {
                current.end = self.vmas[i].end;
                current.resident_pages += self.vmas[i].resident_pages;
                current.swapped_pages += self.vmas[i].swapped_pages;
                current.access_count += self.vmas[i].access_count;
            } else {
                merged.push(current);
                current = self.vmas[i].clone();
            }
        }
        merged.push(current);
        self.vmas = merged;
    }

    #[inline]
    pub fn find_gaps(&self) -> Vec<AddressGap> {
        let mut gaps = Vec::new();
        for i in 1..self.vmas.len() {
            if self.vmas[i].start > self.vmas[i - 1].end {
                gaps.push(AddressGap { start: self.vmas[i - 1].end, end: self.vmas[i].start });
            }
        }
        gaps
    }

    pub fn recompute(&mut self) {
        self.vma_count = self.vmas.len();
        self.total_mapped = self.vmas.iter().map(|v| v.size()).sum();
        self.total_resident = self.vmas.iter().map(|v| v.resident_pages * 4096).sum();
        self.total_swapped = self.vmas.iter().map(|v| v.swapped_pages * 4096).sum();
        let gaps = self.find_gaps();
        self.largest_gap = gaps.iter().map(|g| g.size()).max().unwrap_or(0);
        // Fragmentation: ratio of gaps to total span
        if self.vmas.len() >= 2 {
            let span = self.vmas.last().unwrap().end - self.vmas.first().unwrap().start;
            let gap_total: u64 = gaps.iter().map(|g| g.size()).sum();
            self.fragmentation_score = if span == 0 { 0.0 } else { gap_total as f64 / span as f64 };
        }
    }
}

/// Mmap advisor stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MmapAdvisorStats {
    pub processes_tracked: usize,
    pub total_vmas: usize,
    pub total_mapped_bytes: u64,
    pub total_resident_bytes: u64,
    pub avg_fragmentation: f64,
    pub thp_eligible_vmas: usize,
    pub ksm_mergeable_vmas: usize,
    pub total_merge_candidates: usize,
}

/// Holistic mmap advisor
pub struct HolisticMmapAdvisor {
    processes: BTreeMap<u64, ProcessAddressSpace>,
    stats: MmapAdvisorStats,
}

impl HolisticMmapAdvisor {
    pub fn new() -> Self {
        Self { processes: BTreeMap::new(), stats: MmapAdvisorStats::default() }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.insert(pid, ProcessAddressSpace::new(pid));
    }

    #[inline(always)]
    pub fn add_mapping(&mut self, pid: u64, vma: Vma) {
        if let Some(p) = self.processes.get_mut(&pid) { p.add_vma(vma); }
    }

    #[inline(always)]
    pub fn remove_mapping(&mut self, pid: u64, start: u64, end: u64) {
        if let Some(p) = self.processes.get_mut(&pid) { p.remove_vma(start, end); }
    }

    #[inline(always)]
    pub fn madvise(&mut self, pid: u64, start: u64, len: u64, hint: MadviseHint) {
        if let Some(p) = self.processes.get_mut(&pid) { p.apply_madvise(start, len, hint); }
    }

    #[inline]
    pub fn record_fault(&mut self, pid: u64, addr: u64) {
        if let Some(p) = self.processes.get_mut(&pid) {
            for vma in &mut p.vmas {
                if vma.contains(addr) { vma.fault_count += 1; break; }
            }
        }
    }

    #[inline]
    pub fn record_access(&mut self, pid: u64, addr: u64, ts: u64) {
        if let Some(p) = self.processes.get_mut(&pid) {
            for vma in &mut p.vmas {
                if vma.contains(addr) {
                    vma.access_count += 1;
                    vma.last_access_ts = ts;
                    break;
                }
            }
        }
    }

    #[inline(always)]
    pub fn optimize_merges(&mut self, pid: u64) {
        if let Some(p) = self.processes.get_mut(&pid) { p.merge_adjacent(); }
    }

    pub fn thp_recommendations(&self, pid: u64) -> Vec<(u64, u64, f64)> {
        let mut recs = Vec::new();
        if let Some(p) = self.processes.get(&pid) {
            for vma in &p.vmas {
                let score = vma.thp_score();
                if score > 0.5 && !vma.thp_eligible {
                    recs.push((vma.start, vma.size(), score));
                }
            }
        }
        recs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(core::cmp::Ordering::Equal));
        recs
    }

    pub fn recompute(&mut self) {
        let pids: Vec<u64> = self.processes.keys().copied().collect();
        for pid in &pids {
            if let Some(p) = self.processes.get_mut(pid) { p.recompute(); }
        }
        self.stats.processes_tracked = self.processes.len();
        self.stats.total_vmas = self.processes.values().map(|p| p.vma_count).sum();
        self.stats.total_mapped_bytes = self.processes.values().map(|p| p.total_mapped).sum();
        self.stats.total_resident_bytes = self.processes.values().map(|p| p.total_resident).sum();
        let frags: Vec<f64> = self.processes.values().map(|p| p.fragmentation_score).collect();
        self.stats.avg_fragmentation = if frags.is_empty() { 0.0 } else { frags.iter().sum::<f64>() / frags.len() as f64 };
        self.stats.thp_eligible_vmas = self.processes.values().flat_map(|p| p.vmas.iter()).filter(|v| v.thp_eligible).count();
        self.stats.ksm_mergeable_vmas = self.processes.values().flat_map(|p| p.vmas.iter()).filter(|v| v.ksm_mergeable).count();
    }

    #[inline(always)]
    pub fn process(&self, pid: u64) -> Option<&ProcessAddressSpace> { self.processes.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &MmapAdvisorStats { &self.stats }
}
