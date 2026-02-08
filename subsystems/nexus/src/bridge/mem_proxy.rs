//! # Bridge Memory Proxy
//!
//! Memory-related syscall optimization:
//! - mmap/munmap tracking and coalescing
//! - Page fault prediction from syscall patterns
//! - THP (Transparent Huge Pages) hint generation
//! - VMA merge opportunity detection
//! - Memory pressure-aware syscall throttling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Memory operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemOp {
    Mmap,
    Munmap,
    Mprotect,
    Madvise,
    Mremap,
    Brk,
    Mlock,
    Munlock,
}

/// Madvise hint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadvHint {
    Normal,
    Sequential,
    Random,
    WillNeed,
    DontNeed,
    Free,
    HugePage,
    NoHugePage,
    Collapse,
}

/// VMA tracking entry
#[derive(Debug, Clone)]
pub struct VmaProxyEntry {
    /// Start address
    pub start: u64,
    /// End address
    pub end: u64,
    /// Protection (rwx bits)
    pub prot: u8,
    /// Is anonymous
    pub anonymous: bool,
    /// File-backed inode hash
    pub inode_hash: u64,
    /// Creation time (ns)
    pub created_ns: u64,
    /// Access count
    pub accesses: u64,
    /// Fault count
    pub faults: u64,
    /// Last fault (ns)
    pub last_fault_ns: u64,
    /// Eligible for THP
    pub thp_eligible: bool,
}

impl VmaProxyEntry {
    pub fn new(start: u64, end: u64, prot: u8, now_ns: u64) -> Self {
        let size = end.saturating_sub(start);
        // THP eligible if >=2MB aligned anonymous
        let thp_eligible = size >= 2 * 1024 * 1024 && start % (2 * 1024 * 1024) == 0;
        Self {
            start,
            end,
            prot,
            anonymous: true,
            inode_hash: 0,
            created_ns: now_ns,
            accesses: 0,
            faults: 0,
            last_fault_ns: 0,
            thp_eligible,
        }
    }

    /// Size in bytes
    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    /// Can merge with adjacent VMA?
    pub fn can_merge_with(&self, other: &VmaProxyEntry) -> bool {
        self.prot == other.prot
            && self.anonymous == other.anonymous
            && self.inode_hash == other.inode_hash
            && (self.end == other.start || other.end == self.start)
    }

    /// Fault rate (faults per access)
    pub fn fault_rate(&self) -> f64 {
        if self.accesses == 0 {
            return 0.0;
        }
        self.faults as f64 / self.accesses as f64
    }
}

/// Memory operation record
#[derive(Debug, Clone)]
pub struct MemOpRecord {
    /// Operation
    pub op: MemOp,
    /// Address
    pub address: u64,
    /// Size
    pub size: u64,
    /// PID
    pub pid: u64,
    /// Timestamp
    pub timestamp_ns: u64,
    /// Latency
    pub latency_ns: u64,
}

/// THP recommendation
#[derive(Debug, Clone)]
pub struct ThpRecommendation {
    /// VMA start
    pub vma_start: u64,
    /// VMA size
    pub vma_size: u64,
    /// Confidence (0..1)
    pub confidence: f64,
    /// Expected benefit (fault reduction ratio)
    pub expected_benefit: f64,
}

/// Per-process memory proxy
#[derive(Debug)]
pub struct ProcessMemProxy {
    /// PID
    pub pid: u64,
    /// VMAs
    vmas: BTreeMap<u64, VmaProxyEntry>,
    /// Recent operations (ring)
    recent_ops: Vec<MemOpRecord>,
    recent_pos: usize,
    /// Total memory mapped
    pub total_mapped: u64,
    /// mmap count
    pub mmap_count: u64,
    /// munmap count
    pub munmap_count: u64,
    /// Brk current
    pub brk_current: u64,
}

impl ProcessMemProxy {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            vmas: BTreeMap::new(),
            recent_ops: Vec::new(),
            recent_pos: 0,
            total_mapped: 0,
            mmap_count: 0,
            munmap_count: 0,
            brk_current: 0,
        }
    }

    /// Record mmap
    pub fn record_mmap(&mut self, start: u64, size: u64, prot: u8, now_ns: u64) {
        let end = start + size;
        let vma = VmaProxyEntry::new(start, end, prot, now_ns);
        self.vmas.insert(start, vma);
        self.total_mapped += size;
        self.mmap_count += 1;
        self.record_op(MemOp::Mmap, start, size, now_ns, 0);
    }

    /// Record munmap
    pub fn record_munmap(&mut self, start: u64, size: u64, now_ns: u64) {
        self.vmas.remove(&start);
        self.total_mapped = self.total_mapped.saturating_sub(size);
        self.munmap_count += 1;
        self.record_op(MemOp::Munmap, start, size, now_ns, 0);
    }

    /// Record page fault on VMA
    pub fn record_fault(&mut self, address: u64, now_ns: u64) {
        // Find containing VMA
        for vma in self.vmas.values_mut() {
            if address >= vma.start && address < vma.end {
                vma.faults += 1;
                vma.last_fault_ns = now_ns;
                break;
            }
        }
    }

    fn record_op(&mut self, op: MemOp, addr: u64, size: u64, now_ns: u64, lat: u64) {
        let record = MemOpRecord {
            op, address: addr, size, pid: self.pid,
            timestamp_ns: now_ns, latency_ns: lat,
        };
        if self.recent_ops.len() < 64 {
            self.recent_ops.push(record);
        } else {
            self.recent_ops[self.recent_pos % 64] = record;
        }
        self.recent_pos += 1;
    }

    /// Find merge opportunities
    pub fn merge_opportunities(&self) -> Vec<(u64, u64)> {
        let mut opportunities = Vec::new();
        let vma_list: Vec<&VmaProxyEntry> = self.vmas.values().collect();
        for i in 0..vma_list.len().saturating_sub(1) {
            if vma_list[i].can_merge_with(vma_list[i + 1]) {
                opportunities.push((vma_list[i].start, vma_list[i + 1].start));
            }
        }
        opportunities
    }

    /// THP recommendations
    pub fn thp_recommendations(&self) -> Vec<ThpRecommendation> {
        self.vmas.values()
            .filter(|v| v.thp_eligible && v.faults > 10)
            .map(|v| {
                let benefit = (v.fault_rate() * 0.95).min(0.99);
                ThpRecommendation {
                    vma_start: v.start,
                    vma_size: v.size(),
                    confidence: 0.7,
                    expected_benefit: benefit,
                }
            })
            .collect()
    }

    /// VMA count
    pub fn vma_count(&self) -> usize {
        self.vmas.len()
    }
}

/// Memory proxy stats
#[derive(Debug, Clone, Default)]
pub struct BridgeMemoryProxyStats {
    pub tracked_processes: usize,
    pub total_vmas: usize,
    pub total_mapped_bytes: u64,
    pub merge_opportunities: usize,
    pub thp_candidates: usize,
}

/// Bridge memory proxy
pub struct BridgeMemoryProxy {
    /// Per-process proxies
    processes: BTreeMap<u64, ProcessMemProxy>,
    /// Stats
    stats: BridgeMemoryProxyStats,
}

impl BridgeMemoryProxy {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: BridgeMemoryProxyStats::default(),
        }
    }

    /// Get or create process proxy
    pub fn get_process(&mut self, pid: u64) -> &mut ProcessMemProxy {
        self.processes.entry(pid).or_insert_with(|| ProcessMemProxy::new(pid))
    }

    /// Record mmap
    pub fn record_mmap(&mut self, pid: u64, start: u64, size: u64, prot: u8, now_ns: u64) {
        self.get_process(pid).record_mmap(start, size, prot, now_ns);
        self.update_stats();
    }

    /// Record munmap
    pub fn record_munmap(&mut self, pid: u64, start: u64, size: u64, now_ns: u64) {
        if let Some(proc_proxy) = self.processes.get_mut(&pid) {
            proc_proxy.record_munmap(start, size, now_ns);
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_vmas = self.processes.values().map(|p| p.vma_count()).sum();
        self.stats.total_mapped_bytes = self.processes.values().map(|p| p.total_mapped).sum();
        self.stats.merge_opportunities = self.processes.values()
            .map(|p| p.merge_opportunities().len())
            .sum();
        self.stats.thp_candidates = self.processes.values()
            .map(|p| p.thp_recommendations().len())
            .sum();
    }

    pub fn stats(&self) -> &BridgeMemoryProxyStats {
        &self.stats
    }
}
