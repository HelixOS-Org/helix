// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Taskstats (per-task and per-tgid accounting)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskstatsVersion {
    V8,
    V9,
    V10,
    V11,
    V12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskstatsCmd {
    Get,
    GetTgid,
    RegisterCpuMask,
    DeregisterCpuMask,
}

#[derive(Debug, Clone)]
pub struct TaskstatsCpuAccounting {
    pub utime_ns: u64,
    pub stime_ns: u64,
    pub guest_time_ns: u64,
    pub blkio_delay_ns: u64,
    pub swapin_delay_ns: u64,
    pub freepages_delay_ns: u64,
    pub thrashing_delay_ns: u64,
    pub compact_delay_ns: u64,
    pub wpcopy_delay_ns: u64,
    pub irq_delay_ns: u64,
}

impl TaskstatsCpuAccounting {
    pub fn new() -> Self {
        Self {
            utime_ns: 0, stime_ns: 0, guest_time_ns: 0,
            blkio_delay_ns: 0, swapin_delay_ns: 0, freepages_delay_ns: 0,
            thrashing_delay_ns: 0, compact_delay_ns: 0,
            wpcopy_delay_ns: 0, irq_delay_ns: 0,
        }
    }

    pub fn total_cpu_ns(&self) -> u64 {
        self.utime_ns + self.stime_ns + self.guest_time_ns
    }

    pub fn total_delay_ns(&self) -> u64 {
        self.blkio_delay_ns + self.swapin_delay_ns + self.freepages_delay_ns
            + self.thrashing_delay_ns + self.compact_delay_ns
            + self.wpcopy_delay_ns + self.irq_delay_ns
    }

    pub fn cpu_efficiency_pct(&self) -> u64 {
        let total = self.total_cpu_ns() + self.total_delay_ns();
        if total == 0 { 100 }
        else { (self.total_cpu_ns() * 100) / total }
    }
}

#[derive(Debug, Clone)]
pub struct TaskstatsIoAccounting {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub cancelled_write_bytes: u64,
    pub read_syscalls: u64,
    pub write_syscalls: u64,
    pub read_char: u64,
    pub write_char: u64,
}

impl TaskstatsIoAccounting {
    pub fn new() -> Self {
        Self {
            read_bytes: 0, write_bytes: 0, cancelled_write_bytes: 0,
            read_syscalls: 0, write_syscalls: 0,
            read_char: 0, write_char: 0,
        }
    }

    pub fn total_io_bytes(&self) -> u64 {
        self.read_bytes + self.write_bytes
    }

    pub fn avg_read_size(&self) -> u64 {
        if self.read_syscalls == 0 { 0 } else { self.read_bytes / self.read_syscalls }
    }

    pub fn avg_write_size(&self) -> u64 {
        if self.write_syscalls == 0 { 0 } else { self.write_bytes / self.write_syscalls }
    }
}

#[derive(Debug, Clone)]
pub struct TaskstatsMemAccounting {
    pub rss_pages: u64,
    pub vsize_bytes: u64,
    pub hiwater_rss: u64,
    pub hiwater_vm: u64,
    pub nr_page_faults: u64,
    pub nr_minor_faults: u64,
    pub nr_major_faults: u64,
    pub nr_voluntary_switches: u64,
    pub nr_involuntary_switches: u64,
}

impl TaskstatsMemAccounting {
    pub fn new() -> Self {
        Self {
            rss_pages: 0, vsize_bytes: 0, hiwater_rss: 0, hiwater_vm: 0,
            nr_page_faults: 0, nr_minor_faults: 0, nr_major_faults: 0,
            nr_voluntary_switches: 0, nr_involuntary_switches: 0,
        }
    }

    pub fn major_fault_rate(&self) -> u64 {
        if self.nr_page_faults == 0 { 0 }
        else { (self.nr_major_faults * 100) / self.nr_page_faults }
    }
}

#[derive(Debug, Clone)]
pub struct TaskstatsEntry {
    pub pid: u64,
    pub tgid: u64,
    pub cpu: TaskstatsCpuAccounting,
    pub io: TaskstatsIoAccounting,
    pub mem: TaskstatsMemAccounting,
    pub exit_code: Option<i32>,
}

impl TaskstatsEntry {
    pub fn new(pid: u64, tgid: u64) -> Self {
        Self {
            pid,
            tgid,
            cpu: TaskstatsCpuAccounting::new(),
            io: TaskstatsIoAccounting::new(),
            mem: TaskstatsMemAccounting::new(),
            exit_code: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskstatsBridgeStats {
    pub total_tasks_tracked: u64,
    pub total_queries: u64,
    pub total_exits: u64,
    pub total_cpu_ns: u64,
    pub total_io_bytes: u64,
}

pub struct BridgeTaskstats {
    entries: BTreeMap<u64, TaskstatsEntry>,
    tgid_members: BTreeMap<u64, Vec<u64>>,
    stats: TaskstatsBridgeStats,
}

impl BridgeTaskstats {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            tgid_members: BTreeMap::new(),
            stats: TaskstatsBridgeStats {
                total_tasks_tracked: 0,
                total_queries: 0,
                total_exits: 0,
                total_cpu_ns: 0,
                total_io_bytes: 0,
            },
        }
    }

    pub fn register_task(&mut self, pid: u64, tgid: u64) {
        self.entries.insert(pid, TaskstatsEntry::new(pid, tgid));
        self.tgid_members.entry(tgid).or_insert_with(Vec::new).push(pid);
        self.stats.total_tasks_tracked += 1;
    }

    pub fn get_stats(&mut self, pid: u64) -> Option<&TaskstatsEntry> {
        self.stats.total_queries += 1;
        self.entries.get(&pid)
    }

    pub fn record_exit(&mut self, pid: u64, exit_code: i32) {
        if let Some(entry) = self.entries.get_mut(&pid) {
            entry.exit_code = Some(exit_code);
            self.stats.total_exits += 1;
        }
    }

    pub fn stats(&self) -> &TaskstatsBridgeStats {
        &self.stats
    }
}
