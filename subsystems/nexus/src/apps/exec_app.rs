// SPDX-License-Identifier: GPL-2.0
//! Apps exec_app — execve/execveat process execution.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Exec type
use alloc::string::String;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecType {
    Execve,
    Execveat,
    Script,
    Elf,
    MiscBinary,
}

/// Exec request
#[derive(Debug)]
pub struct ExecRequest {
    pub id: u64,
    pub pid: u64,
    pub exec_type: ExecType,
    pub path_hash: u64,
    pub argv_count: u32,
    pub envp_count: u32,
    pub flags: u32,
    pub timestamp: u64,
    pub duration_ns: u64,
    pub success: bool,
}

impl ExecRequest {
    pub fn new(id: u64, pid: u64, et: ExecType, path_hash: u64, now: u64) -> Self {
        Self { id, pid, exec_type: et, path_hash, argv_count: 0, envp_count: 0, flags: 0, timestamp: now, duration_ns: 0, success: false }
    }
}

/// Binary format handler
#[derive(Debug)]
pub struct BinfmtHandler {
    pub magic_hash: u64,
    pub handler_path_hash: u64,
    pub enabled: bool,
    pub invocations: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ExecAppStats {
    pub total_execs: u32,
    pub successful: u32,
    pub failed: u32,
    pub elf_execs: u32,
    pub script_execs: u32,
    pub avg_duration_ns: u64,
}

/// Main exec app
pub struct AppExec {
    requests: BTreeMap<u64, ExecRequest>,
    handlers: Vec<BinfmtHandler>,
    next_id: u64,
}

impl AppExec {
    pub fn new() -> Self { Self { requests: BTreeMap::new(), handlers: Vec::new(), next_id: 1 } }

    #[inline]
    pub fn exec(&mut self, pid: u64, et: ExecType, path_hash: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.requests.insert(id, ExecRequest::new(id, pid, et, path_hash, now));
        id
    }

    #[inline(always)]
    pub fn complete(&mut self, id: u64, dur: u64) {
        if let Some(r) = self.requests.get_mut(&id) { r.duration_ns = dur; r.success = true; }
    }

    #[inline]
    pub fn stats(&self) -> ExecAppStats {
        let ok = self.requests.values().filter(|r| r.success).count() as u32;
        let fail = self.requests.len() as u32 - ok;
        let elf = self.requests.values().filter(|r| r.exec_type == ExecType::Elf).count() as u32;
        let script = self.requests.values().filter(|r| r.exec_type == ExecType::Script).count() as u32;
        let durs: Vec<u64> = self.requests.values().filter(|r| r.success).map(|r| r.duration_ns).collect();
        let avg = if durs.is_empty() { 0 } else { durs.iter().sum::<u64>() / durs.len() as u64 };
        ExecAppStats { total_execs: self.requests.len() as u32, successful: ok, failed: fail, elf_execs: elf, script_execs: script, avg_duration_ns: avg }
    }
}

// ============================================================================
// Merged from exec_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppExecV2Format {
    Elf64,
    Elf32,
    Script,
    Wasm,
    FlatBinary,
}

/// Exec result status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppExecV2Result {
    Success,
    NotFound,
    PermDenied,
    BadFormat,
    OutOfMemory,
    TooManyArgs,
}

/// Exec request
#[derive(Debug, Clone)]
pub struct AppExecV2Request {
    pub pid: u64,
    pub path: String,
    pub format: AppExecV2Format,
    pub argc: u32,
    pub envc: u32,
    pub timestamp: u64,
}

/// Stats for exec operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppExecV2Stats {
    pub total_execs: u64,
    pub successful: u64,
    pub failed: u64,
    pub elf_execs: u64,
    pub script_execs: u64,
    pub avg_load_us: u64,
}

/// Manager for exec application operations
pub struct AppExecV2Manager {
    history: Vec<(AppExecV2Request, AppExecV2Result)>,
    path_cache: BTreeMap<u64, String>,
    stats: AppExecV2Stats,
}

impl AppExecV2Manager {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            path_cache: BTreeMap::new(),
            stats: AppExecV2Stats {
                total_execs: 0,
                successful: 0,
                failed: 0,
                elf_execs: 0,
                script_execs: 0,
                avg_load_us: 0,
            },
        }
    }

    fn hash_path(path: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn exec(&mut self, pid: u64, path: &str, format: AppExecV2Format, argc: u32, envc: u32) -> AppExecV2Result {
        self.stats.total_execs += 1;
        let req = AppExecV2Request {
            pid,
            path: String::from(path),
            format,
            argc,
            envc,
            timestamp: self.stats.total_execs.wrapping_mul(37),
        };
        match format {
            AppExecV2Format::Elf64 | AppExecV2Format::Elf32 => self.stats.elf_execs += 1,
            AppExecV2Format::Script => self.stats.script_execs += 1,
            _ => {}
        }
        let hash = Self::hash_path(path);
        self.path_cache.insert(hash, String::from(path));
        self.history.push((req, AppExecV2Result::Success));
        self.stats.successful += 1;
        AppExecV2Result::Success
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppExecV2Stats {
        &self.stats
    }
}
