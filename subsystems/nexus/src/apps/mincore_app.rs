// SPDX-License-Identifier: GPL-2.0
//! Apps mincore_app — resident page query.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page residency state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageResidency {
    Resident,
    SwappedOut,
    NotMapped,
    FileBacked,
}

/// Mincore query
#[derive(Debug, Clone)]
pub struct MincoreQuery {
    pub pid: u64,
    pub start: u64,
    pub length: u64,
    pub results: Vec<PageResidency>,
    pub timestamp: u64,
}

impl MincoreQuery {
    #[inline(always)]
    pub fn resident_count(&self) -> usize { self.results.iter().filter(|&&r| r == PageResidency::Resident).count() }
    #[inline(always)]
    pub fn resident_ratio(&self) -> f64 { if self.results.is_empty() { 0.0 } else { self.resident_count() as f64 / self.results.len() as f64 } }
}

/// Process residency info
#[derive(Debug)]
pub struct ProcessResidencyInfo {
    pub pid: u64,
    pub total_queries: u64,
    pub total_pages_queried: u64,
    pub total_resident: u64,
    pub last_query_ns: u64,
}

impl ProcessResidencyInfo {
    pub fn new(pid: u64) -> Self { Self { pid, total_queries: 0, total_pages_queried: 0, total_resident: 0, last_query_ns: 0 } }

    #[inline]
    pub fn record_query(&mut self, query: &MincoreQuery) {
        self.total_queries += 1;
        self.total_pages_queried += query.results.len() as u64;
        self.total_resident += query.resident_count() as u64;
        self.last_query_ns = query.timestamp;
    }

    #[inline(always)]
    pub fn overall_residency(&self) -> f64 {
        if self.total_pages_queried == 0 { 0.0 } else { self.total_resident as f64 / self.total_pages_queried as f64 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MincoreAppStats {
    pub tracked_processes: u32,
    pub total_queries: u64,
    pub total_pages_queried: u64,
    pub avg_residency: f64,
}

/// Main mincore app
pub struct AppMincore {
    processes: BTreeMap<u64, ProcessResidencyInfo>,
}

impl AppMincore {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }
    #[inline(always)]
    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessResidencyInfo::new(pid)); }
    #[inline(always)]
    pub fn unregister(&mut self, pid: u64) { self.processes.remove(&pid); }

    #[inline(always)]
    pub fn query(&mut self, query: &MincoreQuery) {
        if let Some(p) = self.processes.get_mut(&query.pid) { p.record_query(query); }
    }

    #[inline]
    pub fn stats(&self) -> MincoreAppStats {
        let queries: u64 = self.processes.values().map(|p| p.total_queries).sum();
        let pages: u64 = self.processes.values().map(|p| p.total_pages_queried).sum();
        let residencies: Vec<f64> = self.processes.values().map(|p| p.overall_residency()).collect();
        let avg = if residencies.is_empty() { 0.0 } else { residencies.iter().sum::<f64>() / residencies.len() as f64 };
        MincoreAppStats { tracked_processes: self.processes.len() as u32, total_queries: queries, total_pages_queried: pages, avg_residency: avg }
    }
}

// ============================================================================
// Merged from mincore_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageResidencyV2 {
    Resident,
    SwappedOut,
    NotMapped,
    FileBackedClean,
    FileBackedDirty,
}

/// Mincore v2 query
#[derive(Debug)]
pub struct MincoreV2Query {
    pub addr: u64,
    pub length: u64,
    pub page_count: u32,
    pub resident_count: u32,
    pub queried_at: u64,
}

/// Residency snapshot v2
#[derive(Debug)]
pub struct ResidencySnapshotV2 {
    pub pid: u64,
    pub total_pages: u64,
    pub resident_pages: u64,
    pub swapped_pages: u64,
    pub file_pages: u64,
    pub residency_ratio: f64,
    pub snapshot_at: u64,
}

/// Per-process tracker v2
#[derive(Debug)]
pub struct ProcessResidencyV2 {
    pub pid: u64,
    pub queries: Vec<MincoreV2Query>,
    pub total_checked: u64,
    pub total_resident: u64,
    pub snapshots: Vec<ResidencySnapshotV2>,
}

impl ProcessResidencyV2 {
    pub fn new(pid: u64) -> Self {
        Self { pid, queries: Vec::new(), total_checked: 0, total_resident: 0, snapshots: Vec::new() }
    }

    #[inline]
    pub fn query(&mut self, addr: u64, len: u64, pages: u32, resident: u32, now: u64) {
        self.queries.push(MincoreV2Query { addr, length: len, page_count: pages, resident_count: resident, queried_at: now });
        self.total_checked += pages as u64;
        self.total_resident += resident as u64;
    }

    #[inline(always)]
    pub fn snapshot(&mut self, total: u64, resident: u64, swapped: u64, file: u64, now: u64) {
        let ratio = if total == 0 { 0.0 } else { resident as f64 / total as f64 };
        self.snapshots.push(ResidencySnapshotV2 { pid: self.pid, total_pages: total, resident_pages: resident, swapped_pages: swapped, file_pages: file, residency_ratio: ratio, snapshot_at: now });
    }

    #[inline(always)]
    pub fn avg_residency(&self) -> f64 {
        if self.total_checked == 0 { return 0.0; }
        self.total_resident as f64 / self.total_checked as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MincoreV2AppStats {
    pub tracked_procs: u32,
    pub total_queries: u64,
    pub total_pages_checked: u64,
    pub avg_residency: f64,
}

/// Main app mincore v2
pub struct AppMincoreV2 {
    procs: BTreeMap<u64, ProcessResidencyV2>,
}

impl AppMincoreV2 {
    pub fn new() -> Self { Self { procs: BTreeMap::new() } }

    #[inline(always)]
    pub fn track(&mut self, pid: u64) { self.procs.insert(pid, ProcessResidencyV2::new(pid)); }

    #[inline(always)]
    pub fn query(&mut self, pid: u64, addr: u64, len: u64, pages: u32, resident: u32, now: u64) {
        if let Some(p) = self.procs.get_mut(&pid) { p.query(addr, len, pages, resident, now); }
    }

    #[inline(always)]
    pub fn untrack(&mut self, pid: u64) { self.procs.remove(&pid); }

    #[inline]
    pub fn stats(&self) -> MincoreV2AppStats {
        let queries: u64 = self.procs.values().map(|p| p.queries.len() as u64).sum();
        let pages: u64 = self.procs.values().map(|p| p.total_checked).sum();
        let avg = if self.procs.is_empty() { 0.0 }
            else { self.procs.values().map(|p| p.avg_residency()).sum::<f64>() / self.procs.len() as f64 };
        MincoreV2AppStats { tracked_procs: self.procs.len() as u32, total_queries: queries, total_pages_checked: pages, avg_residency: avg }
    }
}
