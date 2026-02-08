// SPDX-License-Identifier: GPL-2.0
//! Apps pagefault_mgr — page fault handler and tracking system.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFaultType {
    Minor,
    Major,
    CopyOnWrite,
    DemandZero,
    SwapIn,
    FileBacked,
    HugePageSplit,
    WriteProtect,
    Prefetch,
}

impl PageFaultType {
    pub fn is_major(&self) -> bool {
        matches!(self, Self::Major | Self::SwapIn | Self::FileBacked)
    }

    pub fn requires_io(&self) -> bool {
        matches!(self, Self::Major | Self::SwapIn | Self::FileBacked)
    }
}

/// Fault resolution action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultAction {
    AllocPage,
    CopyPage,
    MapPage,
    SwapIn,
    ReadFile,
    SplitHuge,
    MergePage,
    SendSigsegv,
    SendSigbus,
    Retry,
}

/// Page fault record
#[derive(Debug, Clone, Copy)]
pub struct PageFaultRecord {
    pub pid: u32,
    pub address: u64,
    pub fault_type: PageFaultType,
    pub action: FaultAction,
    pub timestamp: u64,
    pub duration_ns: u64,
    pub page_frame: u64,
    pub was_present: bool,
    pub was_writable: bool,
}

/// Per-process fault statistics
#[derive(Debug, Clone)]
pub struct ProcessFaultStats {
    pub pid: u32,
    pub minor_faults: u64,
    pub major_faults: u64,
    pub cow_faults: u64,
    pub total_fault_time_ns: u64,
    pub recent_fault_rate: f64,
    pub last_fault_time: u64,
    pub fault_type_counts: BTreeMap<u32, u64>,
}

impl ProcessFaultStats {
    pub fn new(pid: u32) -> Self {
        Self {
            pid, minor_faults: 0, major_faults: 0, cow_faults: 0,
            total_fault_time_ns: 0, recent_fault_rate: 0.0,
            last_fault_time: 0, fault_type_counts: BTreeMap::new(),
        }
    }

    pub fn total_faults(&self) -> u64 {
        self.minor_faults + self.major_faults + self.cow_faults
    }

    pub fn avg_fault_time_ns(&self) -> u64 {
        let total = self.total_faults();
        if total == 0 { 0 } else { self.total_fault_time_ns / total }
    }

    pub fn major_ratio(&self) -> f64 {
        let total = self.total_faults();
        if total == 0 { return 0.0; }
        self.major_faults as f64 / total as f64
    }

    pub fn record(&mut self, fault_type: PageFaultType, duration_ns: u64, now: u64) {
        match fault_type {
            PageFaultType::Minor | PageFaultType::DemandZero | PageFaultType::Prefetch => {
                self.minor_faults += 1;
            }
            PageFaultType::Major | PageFaultType::SwapIn | PageFaultType::FileBacked => {
                self.major_faults += 1;
            }
            PageFaultType::CopyOnWrite => { self.cow_faults += 1; }
            _ => { self.minor_faults += 1; }
        }
        self.total_fault_time_ns += duration_ns;
        self.last_fault_time = now;
        *self.fault_type_counts.entry(fault_type as u32).or_insert(0) += 1;

        // Update recent rate (exponential moving average)
        if self.last_fault_time > 0 {
            let dt = now.saturating_sub(self.last_fault_time);
            if dt > 0 {
                let instant_rate = 1_000_000_000.0 / dt as f64;
                self.recent_fault_rate = self.recent_fault_rate * 0.9 + instant_rate * 0.1;
            }
        }
    }
}

/// Fault hotspot — frequently faulting addresses
#[derive(Debug, Clone, Copy)]
pub struct FaultHotspot {
    pub page_addr: u64,
    pub count: u64,
    pub last_fault_type: PageFaultType,
    pub last_time: u64,
}

/// Pagefault manager stats
#[derive(Debug, Clone)]
pub struct PagefaultMgrStats {
    pub tracked_processes: u32,
    pub total_minor: u64,
    pub total_major: u64,
    pub total_cow: u64,
    pub total_fault_time_ns: u64,
    pub hotspot_count: u32,
    pub sigsegv_sent: u64,
    pub sigbus_sent: u64,
}

/// Main pagefault manager
pub struct AppPagefaultMgr {
    process_stats: BTreeMap<u32, ProcessFaultStats>,
    hotspots: BTreeMap<u64, FaultHotspot>,
    recent_faults: Vec<PageFaultRecord>,
    max_recent: usize,
    max_hotspots: usize,
    total_minor: u64,
    total_major: u64,
    total_cow: u64,
    total_fault_time_ns: u64,
    sigsegv_sent: u64,
    sigbus_sent: u64,
    page_size: u64,
}

impl AppPagefaultMgr {
    pub fn new(page_size: u64) -> Self {
        Self {
            process_stats: BTreeMap::new(),
            hotspots: BTreeMap::new(),
            recent_faults: Vec::new(),
            max_recent: 4096, max_hotspots: 1024,
            total_minor: 0, total_major: 0, total_cow: 0,
            total_fault_time_ns: 0, sigsegv_sent: 0, sigbus_sent: 0,
            page_size,
        }
    }

    pub fn record_fault(&mut self, record: PageFaultRecord) {
        let pid = record.pid;
        if !self.process_stats.contains_key(&pid) {
            self.process_stats.insert(pid, ProcessFaultStats::new(pid));
        }
        if let Some(stats) = self.process_stats.get_mut(&pid) {
            stats.record(record.fault_type, record.duration_ns, record.timestamp);
        }

        match record.fault_type {
            PageFaultType::Minor | PageFaultType::DemandZero | PageFaultType::Prefetch => {
                self.total_minor += 1;
            }
            PageFaultType::Major | PageFaultType::SwapIn | PageFaultType::FileBacked => {
                self.total_major += 1;
            }
            PageFaultType::CopyOnWrite => { self.total_cow += 1; }
            _ => { self.total_minor += 1; }
        }
        self.total_fault_time_ns += record.duration_ns;

        match record.action {
            FaultAction::SendSigsegv => { self.sigsegv_sent += 1; }
            FaultAction::SendSigbus => { self.sigbus_sent += 1; }
            _ => {}
        }

        // Track hotspots
        let page_addr = record.address & !(self.page_size - 1);
        let hotspot = self.hotspots.entry(page_addr).or_insert(FaultHotspot {
            page_addr, count: 0, last_fault_type: record.fault_type,
            last_time: record.timestamp,
        });
        hotspot.count += 1;
        hotspot.last_fault_type = record.fault_type;
        hotspot.last_time = record.timestamp;

        // Trim hotspots
        if self.hotspots.len() > self.max_hotspots {
            if let Some(&min_addr) = self.hotspots.iter()
                .min_by_key(|(_, h)| h.count).map(|(k, _)| k) {
                self.hotspots.remove(&min_addr);
            }
        }

        if self.recent_faults.len() >= self.max_recent { self.recent_faults.remove(0); }
        self.recent_faults.push(record);
    }

    pub fn top_hotspots(&self, n: usize) -> Vec<&FaultHotspot> {
        let mut v: Vec<_> = self.hotspots.values().collect();
        v.sort_by(|a, b| b.count.cmp(&a.count));
        v.truncate(n);
        v
    }

    pub fn process_stats(&self, pid: u32) -> Option<&ProcessFaultStats> {
        self.process_stats.get(&pid)
    }

    pub fn remove_process(&mut self, pid: u32) -> bool {
        self.process_stats.remove(&pid).is_some()
    }

    pub fn high_fault_rate_processes(&self, threshold: f64) -> Vec<(u32, f64)> {
        self.process_stats.iter()
            .filter(|(_, s)| s.recent_fault_rate > threshold)
            .map(|(&pid, s)| (pid, s.recent_fault_rate))
            .collect()
    }

    pub fn stats(&self) -> PagefaultMgrStats {
        PagefaultMgrStats {
            tracked_processes: self.process_stats.len() as u32,
            total_minor: self.total_minor,
            total_major: self.total_major,
            total_cow: self.total_cow,
            total_fault_time_ns: self.total_fault_time_ns,
            hotspot_count: self.hotspots.len() as u32,
            sigsegv_sent: self.sigsegv_sent,
            sigbus_sent: self.sigbus_sent,
        }
    }
}
