// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Exec (process execution bridge)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Exec format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeExecFormat {
    Elf64,
    Elf32,
    Script,
    FlatBinary,
}

/// Exec request
#[derive(Debug, Clone)]
pub struct BridgeExecRequest {
    pub pid: u64,
    pub path: String,
    pub format: BridgeExecFormat,
    pub argv_count: u32,
    pub envp_count: u32,
    pub timestamp: u64,
}

/// Exec result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeExecResult {
    Success,
    NotFound,
    PermissionDenied,
    InvalidFormat,
    TooManyArgs,
    OutOfMemory,
}

/// Stats for exec operations
#[derive(Debug, Clone)]
pub struct BridgeExecStats {
    pub total_execs: u64,
    pub successful: u64,
    pub failed: u64,
    pub elf_execs: u64,
    pub script_execs: u64,
    pub avg_exec_us: u64,
}

/// Manager for exec bridge operations
pub struct BridgeExecManager {
    history: Vec<(BridgeExecRequest, BridgeExecResult)>,
    active_execs: BTreeMap<u64, BridgeExecRequest>,
    stats: BridgeExecStats,
}

impl BridgeExecManager {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            active_execs: BTreeMap::new(),
            stats: BridgeExecStats {
                total_execs: 0,
                successful: 0,
                failed: 0,
                elf_execs: 0,
                script_execs: 0,
                avg_exec_us: 0,
            },
        }
    }

    pub fn exec(&mut self, pid: u64, path: &str, format: BridgeExecFormat, argc: u32, envc: u32) -> BridgeExecResult {
        self.stats.total_execs += 1;
        let req = BridgeExecRequest {
            pid,
            path: String::from(path),
            format,
            argv_count: argc,
            envp_count: envc,
            timestamp: self.stats.total_execs.wrapping_mul(37),
        };
        match format {
            BridgeExecFormat::Elf64 | BridgeExecFormat::Elf32 => self.stats.elf_execs += 1,
            BridgeExecFormat::Script => self.stats.script_execs += 1,
            _ => {}
        }
        self.active_execs.insert(pid, req.clone());
        self.history.push((req, BridgeExecResult::Success));
        self.stats.successful += 1;
        BridgeExecResult::Success
    }

    pub fn complete_exec(&mut self, pid: u64) -> bool {
        self.active_execs.remove(&pid).is_some()
    }

    pub fn stats(&self) -> &BridgeExecStats {
        &self.stats
    }
}
