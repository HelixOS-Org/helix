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
use alloc::collections::VecDeque;
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

    #[inline(always)]
    pub fn default_filter() -> Self { Self { bits: Self::ANON_PRIVATE | Self::ANON_SHARED | Self::ELF_HEADERS } }
    pub fn new(bits: u32) -> Self { Self { bits } }
    #[inline(always)]
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
    #[inline(always)]
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
    pub recent_crashes: VecDeque<u64>,
    pub dominant_signal: CrashSignal,
    pub unique_fault_addrs: VecDeque<u64>,
}

impl ExeCrashHistory {
    pub fn new(name: String) -> Self {
        Self {
            exe_name: name, crash_count: 0, last_crash_ts: 0,
            recent_crashes: VecDeque::new(), dominant_signal: CrashSignal::Segfault,
            unique_fault_addrs: VecDeque::new(),
        }
    }

    #[inline]
    pub fn record_crash(&mut self, ts: u64, signal: CrashSignal, fault_addr: u64) {
        self.crash_count += 1;
        self.last_crash_ts = ts;
        self.recent_crashes.push_back(ts);
        if self.recent_crashes.len() > 32 { self.recent_crashes.remove(0); }
        self.dominant_signal = signal;
        if fault_addr != 0 && !self.unique_fault_addrs.contains(&fault_addr) {
            self.unique_fault_addrs.push_back(fault_addr);
            if self.unique_fault_addrs.len() > 64 { self.unique_fault_addrs.remove(0); }
        }
    }

    #[inline]
    pub fn crash_rate(&self, window_ns: u64) -> f64 {
        if self.recent_crashes.len() < 2 { return 0.0; }
        let first = self.recent_crashes[0];
        let last = *self.recent_crashes.back().unwrap();
        let span = last.saturating_sub(first);
        if span == 0 { return 0.0; }
        (self.recent_crashes.len() as f64 / span as f64) * window_ns as f64
    }

    #[inline(always)]
    pub fn is_repeated_crash(&self) -> bool { self.crash_count > 3 }
    #[inline(always)]
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
    #[inline]
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
#[repr(align(64))]
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
    records: VecDeque<CoredumpRecord>,
    exe_history: BTreeMap<String, ExeCrashHistory>,
    max_records: usize,
    stats: CoredumpStats,
}

impl AppsCoredump {
    pub fn new() -> Self {
        Self {
            config: CoredumpConfig::default_config(),
            records: VecDeque::new(), exe_history: BTreeMap::new(),
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
        self.records.push_back(record);
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

    #[inline(always)]
    pub fn set_config(&mut self, config: CoredumpConfig) { self.config = config; }

    #[inline(always)]
    pub fn exe_crash_history(&self, exe: &str) -> Option<&ExeCrashHistory> {
        self.exe_history.get(exe)
    }

    #[inline(always)]
    pub fn recompute(&mut self) {
        self.stats.unique_crashers = self.exe_history.len();
        self.stats.repeated_crashers = self.exe_history.values().filter(|h| h.is_repeated_crash()).count();
    }

    #[inline(always)]
    pub fn config(&self) -> &CoredumpConfig { &self.config }
    #[inline(always)]
    pub fn stats(&self) -> &CoredumpStats { &self.stats }
}

// ============================================================================
// Merged from coredump_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreFormat {
    Elf,
    Minidump,
    Compressed,
    Filtered,
    Custom,
}

/// Core dump state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreState {
    Pending,
    Generating,
    Writing,
    Complete,
    Failed,
    Truncated,
}

/// Filter rule for core content
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreFilter {
    IncludeAnon,
    IncludeFile,
    IncludeElfHeaders,
    IncludeShared,
    IncludeHugetlb,
    IncludeDax,
    ExcludeAnon,
    ExcludeFile,
}

/// ELF note type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteType {
    Prstatus,
    Prpsinfo,
    Fpregset,
    Siginfo,
    Auxv,
    File,
    Custom(u32),
}

/// Memory segment descriptor for core dump
#[derive(Debug, Clone)]
pub struct CoreSegment {
    pub vaddr: u64,
    pub size: u64,
    pub file_offset: u64,
    pub flags: u32,
    pub is_anon: bool,
    pub is_file_backed: bool,
    pub written: bool,
}

impl CoreSegment {
    pub fn new(vaddr: u64, size: u64) -> Self {
        Self { vaddr, size, file_offset: 0, flags: 0, is_anon: true, is_file_backed: false, written: false }
    }
}

/// Core dump descriptor
#[derive(Debug)]
pub struct CoreDump {
    pub id: u64,
    pub pid: u64,
    pub tid: u64,
    pub signal: u32,
    pub format: CoreFormat,
    pub state: CoreState,
    pub segments: Vec<CoreSegment>,
    pub notes: Vec<NoteType>,
    pub filter_mask: u32,
    pub total_size: u64,
    pub written_bytes: u64,
    pub started_at: u64,
    pub completed_at: u64,
    pub path: String,
    pub compressed: bool,
    pub compression_ratio: f64,
}

impl CoreDump {
    pub fn new(id: u64, pid: u64, signal: u32, format: CoreFormat, now: u64) -> Self {
        Self {
            id, pid, tid: pid, signal, format, state: CoreState::Pending,
            segments: Vec::new(), notes: Vec::new(), filter_mask: 0x33,
            total_size: 0, written_bytes: 0, started_at: now, completed_at: 0,
            path: String::new(), compressed: false, compression_ratio: 1.0,
        }
    }

    #[inline(always)]
    pub fn add_segment(&mut self, seg: CoreSegment) {
        self.total_size += seg.size;
        self.segments.push(seg);
    }

    #[inline(always)]
    pub fn begin_write(&mut self) { self.state = CoreState::Generating; }

    #[inline(always)]
    pub fn write_progress(&mut self, bytes: u64) {
        self.written_bytes += bytes;
        self.state = CoreState::Writing;
    }

    #[inline(always)]
    pub fn complete(&mut self, now: u64) {
        self.state = CoreState::Complete;
        self.completed_at = now;
    }

    #[inline(always)]
    pub fn fail(&mut self) { self.state = CoreState::Failed; }

    #[inline(always)]
    pub fn progress(&self) -> f64 {
        if self.total_size == 0 { return 0.0; }
        self.written_bytes as f64 / self.total_size as f64
    }

    #[inline(always)]
    pub fn duration_ns(&self) -> u64 {
        self.completed_at.saturating_sub(self.started_at)
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoreDumpV2Stats {
    pub total_dumps: u32,
    pub completed_dumps: u32,
    pub failed_dumps: u32,
    pub total_bytes_written: u64,
    pub avg_dump_time_ns: u64,
}

/// Main coredump v2 manager
pub struct AppCoreDumpV2 {
    dumps: BTreeMap<u64, CoreDump>,
    next_id: u64,
}

impl AppCoreDumpV2 {
    pub fn new() -> Self { Self { dumps: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn begin_dump(&mut self, pid: u64, signal: u32, format: CoreFormat, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.dumps.insert(id, CoreDump::new(id, pid, signal, format, now));
        id
    }

    #[inline(always)]
    pub fn add_segment(&mut self, id: u64, seg: CoreSegment) {
        if let Some(d) = self.dumps.get_mut(&id) { d.add_segment(seg); }
    }

    #[inline(always)]
    pub fn complete_dump(&mut self, id: u64, now: u64) {
        if let Some(d) = self.dumps.get_mut(&id) { d.complete(now); }
    }

    #[inline]
    pub fn stats(&self) -> CoreDumpV2Stats {
        let completed = self.dumps.values().filter(|d| d.state == CoreState::Complete).count() as u32;
        let failed = self.dumps.values().filter(|d| d.state == CoreState::Failed).count() as u32;
        let bytes: u64 = self.dumps.values().map(|d| d.written_bytes).sum();
        let durations: Vec<u64> = self.dumps.values().filter(|d| d.completed_at > 0).map(|d| d.duration_ns()).collect();
        let avg = if durations.is_empty() { 0 } else { durations.iter().sum::<u64>() / durations.len() as u64 };
        CoreDumpV2Stats {
            total_dumps: self.dumps.len() as u32, completed_dumps: completed,
            failed_dumps: failed, total_bytes_written: bytes, avg_dump_time_ns: avg,
        }
    }
}
