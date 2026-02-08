// SPDX-License-Identifier: GPL-2.0
//! Apps coredump_mgr — core dump generation and management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Core dump format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreFormat {
    Elf,
    Compressed,
    MiniDump,
    Custom,
}

/// Signal that triggered the dump
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreSignal {
    Segfault,
    BusError,
    IllegalInstruction,
    FloatingPoint,
    Abort,
    Trap,
    SysError,
}

/// Memory region info for core dump
#[derive(Debug, Clone)]
pub struct CoreMemRegion {
    pub start: u64,
    pub end: u64,
    pub offset: u64,
    pub permissions: u32,
    pub mapped_file: Option<String>,
    pub is_stack: bool,
    pub is_heap: bool,
    pub included: bool,
}

impl CoreMemRegion {
    pub fn size(&self) -> u64 { self.end.saturating_sub(self.start) }
    pub fn is_readable(&self) -> bool { self.permissions & 0x4 != 0 }
    pub fn is_writable(&self) -> bool { self.permissions & 0x2 != 0 }
    pub fn is_executable(&self) -> bool { self.permissions & 0x1 != 0 }
}

/// Register state snapshot
#[derive(Debug, Clone)]
pub struct RegisterSnapshot {
    pub gpr: [u64; 16],
    pub rip: u64,
    pub rflags: u64,
    pub cs: u64,
    pub ss: u64,
    pub fs_base: u64,
    pub gs_base: u64,
}

/// Core dump filter (which regions to include)
#[derive(Debug, Clone, Copy)]
pub struct CoreFilter(pub u32);

impl CoreFilter {
    pub const ANON_PRIVATE: Self = Self(0x01);
    pub const ANON_SHARED: Self = Self(0x02);
    pub const MAPPED_PRIVATE: Self = Self(0x04);
    pub const MAPPED_SHARED: Self = Self(0x08);
    pub const ELF_HEADERS: Self = Self(0x10);
    pub const HUGETLB_PRIVATE: Self = Self(0x20);
    pub const HUGETLB_SHARED: Self = Self(0x40);
    pub const DAX_PRIVATE: Self = Self(0x80);

    pub fn default_filter() -> Self { Self(0x33) }
    pub fn contains(&self, other: Self) -> bool { self.0 & other.0 != 0 }
}

/// Core dump state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreDumpState {
    Pending,
    Collecting,
    Writing,
    Compressing,
    Complete,
    Failed,
    Truncated,
}

/// Core dump entry
#[derive(Debug, Clone)]
pub struct CoreDumpEntry {
    pub id: u64,
    pub pid: u32,
    pub tid: u32,
    pub signal: CoreSignal,
    pub format: CoreFormat,
    pub state: CoreDumpState,
    pub filter: CoreFilter,
    pub regions: Vec<CoreMemRegion>,
    pub registers: RegisterSnapshot,
    pub dump_size: u64,
    pub max_size: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub path: String,
    pub thread_count: u32,
}

impl CoreDumpEntry {
    pub fn new(id: u64, pid: u32, tid: u32, signal: CoreSignal, now: u64) -> Self {
        Self {
            id, pid, tid, signal, format: CoreFormat::Elf,
            state: CoreDumpState::Pending, filter: CoreFilter::default_filter(),
            regions: Vec::new(),
            registers: RegisterSnapshot {
                gpr: [0u64; 16], rip: 0, rflags: 0,
                cs: 0, ss: 0, fs_base: 0, gs_base: 0,
            },
            dump_size: 0, max_size: 2 * 1024 * 1024 * 1024,
            start_time: now, end_time: 0,
            path: String::new(), thread_count: 1,
        }
    }

    pub fn total_region_size(&self) -> u64 {
        self.regions.iter().filter(|r| r.included).map(|r| r.size()).sum()
    }

    pub fn is_oversized(&self) -> bool {
        self.total_region_size() > self.max_size
    }

    pub fn duration(&self) -> u64 {
        if self.end_time > 0 { self.end_time - self.start_time } else { 0 }
    }
}

/// Pipe-to-program handler
#[derive(Debug, Clone)]
pub struct CorePipeHandler {
    pub pattern: String,
    pub max_size: u64,
    pub compress: bool,
    pub uses: u64,
}

/// Coredump manager stats
#[derive(Debug, Clone)]
pub struct CoredumpMgrStats {
    pub total_dumps: u64,
    pub completed: u64,
    pub failed: u64,
    pub truncated: u64,
    pub total_bytes_written: u64,
    pub signal_counts: BTreeMap<u32, u64>,
}

/// Main coredump manager
pub struct AppCoredumpMgr {
    active_dumps: BTreeMap<u64, CoreDumpEntry>,
    completed: Vec<CoreDumpEntry>,
    max_completed_history: usize,
    next_id: u64,
    default_filter: CoreFilter,
    default_format: CoreFormat,
    core_size_limit: u64,
    pipe_handler: Option<CorePipeHandler>,
    stats: CoredumpMgrStats,
}

impl AppCoredumpMgr {
    pub fn new(max_history: usize) -> Self {
        Self {
            active_dumps: BTreeMap::new(),
            completed: Vec::new(),
            max_completed_history: max_history,
            next_id: 1, default_filter: CoreFilter::default_filter(),
            default_format: CoreFormat::Elf,
            core_size_limit: 2 * 1024 * 1024 * 1024,
            pipe_handler: None,
            stats: CoredumpMgrStats {
                total_dumps: 0, completed: 0, failed: 0, truncated: 0,
                total_bytes_written: 0, signal_counts: BTreeMap::new(),
            },
        }
    }

    pub fn initiate_dump(&mut self, pid: u32, tid: u32, signal: CoreSignal, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.stats.total_dumps += 1;
        *self.stats.signal_counts.entry(signal as u32).or_insert(0) += 1;

        let mut entry = CoreDumpEntry::new(id, pid, tid, signal, now);
        entry.format = self.default_format;
        entry.filter = self.default_filter;
        entry.max_size = self.core_size_limit;
        self.active_dumps.insert(id, entry);
        id
    }

    pub fn add_region(&mut self, dump_id: u64, region: CoreMemRegion) {
        if let Some(entry) = self.active_dumps.get_mut(&dump_id) {
            entry.regions.push(region);
        }
    }

    pub fn set_registers(&mut self, dump_id: u64, regs: RegisterSnapshot) {
        if let Some(entry) = self.active_dumps.get_mut(&dump_id) {
            entry.registers = regs;
        }
    }

    pub fn advance_state(&mut self, dump_id: u64, state: CoreDumpState) {
        if let Some(entry) = self.active_dumps.get_mut(&dump_id) {
            entry.state = state;
        }
    }

    pub fn complete_dump(&mut self, dump_id: u64, bytes_written: u64, now: u64) {
        if let Some(mut entry) = self.active_dumps.remove(&dump_id) {
            entry.state = CoreDumpState::Complete;
            entry.dump_size = bytes_written;
            entry.end_time = now;
            self.stats.completed += 1;
            self.stats.total_bytes_written += bytes_written;
            if self.completed.len() >= self.max_completed_history {
                self.completed.remove(0);
            }
            self.completed.push(entry);
        }
    }

    pub fn fail_dump(&mut self, dump_id: u64, now: u64) {
        if let Some(mut entry) = self.active_dumps.remove(&dump_id) {
            entry.state = CoreDumpState::Failed;
            entry.end_time = now;
            self.stats.failed += 1;
            if self.completed.len() >= self.max_completed_history {
                self.completed.remove(0);
            }
            self.completed.push(entry);
        }
    }

    pub fn set_pipe_handler(&mut self, handler: CorePipeHandler) {
        self.pipe_handler = Some(handler);
    }

    pub fn set_size_limit(&mut self, limit: u64) { self.core_size_limit = limit; }

    pub fn active_count(&self) -> usize { self.active_dumps.len() }

    pub fn stats(&self) -> &CoredumpMgrStats { &self.stats }
}
