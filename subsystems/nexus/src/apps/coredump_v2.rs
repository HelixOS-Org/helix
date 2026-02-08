// SPDX-License-Identifier: GPL-2.0
//! Apps coredump_v2 — advanced core dump generation and management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Core dump format
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

    pub fn add_segment(&mut self, seg: CoreSegment) {
        self.total_size += seg.size;
        self.segments.push(seg);
    }

    pub fn begin_write(&mut self) { self.state = CoreState::Generating; }

    pub fn write_progress(&mut self, bytes: u64) {
        self.written_bytes += bytes;
        self.state = CoreState::Writing;
    }

    pub fn complete(&mut self, now: u64) {
        self.state = CoreState::Complete;
        self.completed_at = now;
    }

    pub fn fail(&mut self) { self.state = CoreState::Failed; }

    pub fn progress(&self) -> f64 {
        if self.total_size == 0 { return 0.0; }
        self.written_bytes as f64 / self.total_size as f64
    }

    pub fn duration_ns(&self) -> u64 {
        self.completed_at.saturating_sub(self.started_at)
    }
}

/// Stats
#[derive(Debug, Clone)]
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

    pub fn begin_dump(&mut self, pid: u64, signal: u32, format: CoreFormat, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.dumps.insert(id, CoreDump::new(id, pid, signal, format, now));
        id
    }

    pub fn add_segment(&mut self, id: u64, seg: CoreSegment) {
        if let Some(d) = self.dumps.get_mut(&id) { d.add_segment(seg); }
    }

    pub fn complete_dump(&mut self, id: u64, now: u64) {
        if let Some(d) = self.dumps.get_mut(&id) { d.complete(now); }
    }

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
