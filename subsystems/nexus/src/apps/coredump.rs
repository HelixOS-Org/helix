//! # Apps Coredump
//!
//! Core dump configuration and tracking:
//! - Core dump pattern and path configuration
//! - Core dump size limits and filtering
//! - Signal-to-coredump mapping
//! - Coredump file generation tracking
//! - Process crash history and analysis
//! - Repeated crash detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Core dump format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoredumpFormat {
    Elf,
    Compressed,
    Minidump,
    Custom,
}

/// Filter flags for what to include
#[derive(Debug, Clone, Copy)]
pub struct CoredumpFilter {
    pub bits: u32,
}

impl CoredumpFilter {
    pub const ANON_PRIVATE: u32 = 1;
    pub const ANON_SHARED: u32 = 2;
    pub const FILE_PRIVATE: u32 = 4;
    pub const FILE_SHARED: u32 = 8;
    pub const ELF_HEADERS: u32 = 16;
    pub const HUGETLB_PRIVATE: u32 = 32;
    pub const HUGETLB_SHARED: u32 = 64;
    pub const DAX_PRIVATE: u32 = 128;

    pub fn default_filter() -> Self { Self { bits: Self::ANON_PRIVATE | Self::ANON_SHARED | Self::ELF_HEADERS } }
    pub fn new(bits: u32) -> Self { Self { bits } }
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
}

/// Crash signal that triggers coredump
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashSignal {
    Segfault,
    BusError,
    IllegalInsn,
    FpeError,
    Abort,
    Trap,
    SysError,
    Other(u8),
}

impl CrashSignal {
    pub fn generates_core(&self) -> bool {
        matches!(self, Self::Segfault | Self::BusError | Self::IllegalInsn | Self::FpeError | Self::Abort | Self::Trap)
    }
}

/// Coredump record
#[derive(Debug, Clone)]
pub struct CoredumpRecord {
    pub pid: u64,
    pub exe_name: String,
    pub signal: CrashSignal,
    pub timestamp: u64,
    pub core_size: u64,
    pub format: CoredumpFormat,
    pub truncated: bool,
    pub fault_addr: u64,
    pub instruction_ptr: u64,
    pub stack_ptr: u64,
}

impl CoredumpRecord {
    pub fn new(pid: u64, exe: String, signal: CrashSignal, ts: u64) -> Self {
        Self {
            pid, exe_name: exe, signal, timestamp: ts, core_size: 0,
            format: CoredumpFormat::Elf, truncated: false,
            fault_addr: 0, instruction_ptr: 0, stack_ptr: 0,
        }
    }
}

/// Per-executable crash history
#[derive(Debug, Clone)]
pub struct ExeCrashHistory {
    pub exe_name: String,
    pub crash_count: u64,
    pub last_crash_ts: u64,
    pub recent_crashes: Vec<u64>,
    pub dominant_signal: CrashSignal,
    pub unique_fault_addrs: Vec<u64>,
}

impl ExeCrashHistory {
    pub fn new(name: String) -> Self {
        Self {
            exe_name: name, crash_count: 0, last_crash_ts: 0,
            recent_crashes: Vec::new(), dominant_signal: CrashSignal::Segfault,
            unique_fault_addrs: Vec::new(),
        }
    }

    pub fn record_crash(&mut self, ts: u64, signal: CrashSignal, fault_addr: u64) {
        self.crash_count += 1;
        self.last_crash_ts = ts;
        self.recent_crashes.push(ts);
        if self.recent_crashes.len() > 32 { self.recent_crashes.remove(0); }
        self.dominant_signal = signal;
        if fault_addr != 0 && !self.unique_fault_addrs.contains(&fault_addr) {
            self.unique_fault_addrs.push(fault_addr);
            if self.unique_fault_addrs.len() > 64 { self.unique_fault_addrs.remove(0); }
        }
    }

    pub fn crash_rate(&self, window_ns: u64) -> f64 {
        if self.recent_crashes.len() < 2 { return 0.0; }
        let first = self.recent_crashes[0];
        let last = *self.recent_crashes.last().unwrap();
        let span = last.saturating_sub(first);
        if span == 0 { return 0.0; }
        (self.recent_crashes.len() as f64 / span as f64) * window_ns as f64
    }

    pub fn is_repeated_crash(&self) -> bool { self.crash_count > 3 }
    pub fn unique_faults(&self) -> usize { self.unique_fault_addrs.len() }
}

/// Coredump configuration
#[derive(Debug, Clone)]
pub struct CoredumpConfig {
    pub enabled: bool,
    pub max_size: u64,
    pub format: CoredumpFormat,
    pub filter: CoredumpFilter,
    pub pipe_program: Option<String>,
    pub compress: bool,
}

impl CoredumpConfig {
    pub fn default_config() -> Self {
        Self {
            enabled: true, max_size: 512 * 1024 * 1024,
            format: CoredumpFormat::Elf, filter: CoredumpFilter::default_filter(),
            pipe_program: None, compress: false,
        }
    }
}

/// Coredump stats
#[derive(Debug, Clone, Default)]
pub struct CoredumpStats {
    pub total_coredumps: u64,
    pub total_core_bytes: u64,
    pub truncated_dumps: u64,
    pub suppressed_dumps: u64,
    pub unique_crashers: usize,
    pub repeated_crashers: usize,
    pub segfaults: u64,
    pub aborts: u64,
}

/// Apps coredump manager
pub struct AppsCoredump {
    config: CoredumpConfig,
    records: Vec<CoredumpRecord>,
    exe_history: BTreeMap<String, ExeCrashHistory>,
    max_records: usize,
    stats: CoredumpStats,
}

impl AppsCoredump {
    pub fn new() -> Self {
        Self {
            config: CoredumpConfig::default_config(),
            records: Vec::new(), exe_history: BTreeMap::new(),
            max_records: 256, stats: CoredumpStats::default(),
        }
    }

    pub fn record_crash(&mut self, pid: u64, exe: String, signal: CrashSignal, fault_addr: u64, ts: u64) -> bool {
        if !self.config.enabled || !signal.generates_core() {
            self.stats.suppressed_dumps += 1;
            return false;
        }

        let mut record = CoredumpRecord::new(pid, exe.clone(), signal, ts);
        record.fault_addr = fault_addr;
        self.records.push(record);
        if self.records.len() > self.max_records { self.records.remove(0); }

        let history = self.exe_history.entry(exe).or_insert_with_key(|k| ExeCrashHistory::new(k.clone()));
        history.record_crash(ts, signal, fault_addr);

        self.stats.total_coredumps += 1;
        match signal {
            CrashSignal::Segfault => self.stats.segfaults += 1,
            CrashSignal::Abort => self.stats.aborts += 1,
            _ => {}
        }
        true
    }

    pub fn set_config(&mut self, config: CoredumpConfig) { self.config = config; }

    pub fn exe_crash_history(&self, exe: &str) -> Option<&ExeCrashHistory> {
        self.exe_history.get(exe)
    }

    pub fn recompute(&mut self) {
        self.stats.unique_crashers = self.exe_history.len();
        self.stats.repeated_crashers = self.exe_history.values().filter(|h| h.is_repeated_crash()).count();
    }

    pub fn config(&self) -> &CoredumpConfig { &self.config }
    pub fn stats(&self) -> &CoredumpStats { &self.stats }
}
