// SPDX-License-Identifier: GPL-2.0
//! Bridge BIO — block I/O submission bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// BIO bridge operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioBridgeOp {
    SubmitBio,
    CompleteBio,
    MergeBio,
    SplitBio,
    CloneBio,
    EndIo,
    Remap,
    Throttle,
}

/// BIO bridge result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioBridgeResult {
    Success,
    Io,
    Again,
    NoMem,
    Busy,
    Error,
}

/// BIO bridge record
#[derive(Debug, Clone)]
pub struct BioBridgeRecord {
    pub op: BioBridgeOp,
    pub result: BioBridgeResult,
    pub sector: u64,
    pub nr_sectors: u32,
    pub device_id: u64,
    pub is_write: bool,
    pub latency_ns: u64,
}

impl BioBridgeRecord {
    pub fn new(op: BioBridgeOp, sector: u64, nr_sectors: u32, is_write: bool) -> Self {
        Self { op, result: BioBridgeResult::Success, sector, nr_sectors, device_id: 0, is_write, latency_ns: 0 }
    }

    pub fn bytes(&self) -> u64 { self.nr_sectors as u64 * 512 }
}

/// BIO bridge stats
#[derive(Debug, Clone)]
pub struct BioBridgeStats {
    pub total_ops: u64,
    pub submits: u64,
    pub completions: u64,
    pub merges: u64,
    pub splits: u64,
    pub errors: u64,
    pub total_bytes: u64,
    pub total_latency_ns: u64,
}

/// Main bridge BIO
#[derive(Debug)]
pub struct BridgeBio {
    pub stats: BioBridgeStats,
}

impl BridgeBio {
    pub fn new() -> Self {
        Self { stats: BioBridgeStats { total_ops: 0, submits: 0, completions: 0, merges: 0, splits: 0, errors: 0, total_bytes: 0, total_latency_ns: 0 } }
    }

    pub fn record(&mut self, rec: &BioBridgeRecord) {
        self.stats.total_ops += 1;
        self.stats.total_bytes += rec.bytes();
        self.stats.total_latency_ns += rec.latency_ns;
        match rec.op {
            BioBridgeOp::SubmitBio => self.stats.submits += 1,
            BioBridgeOp::CompleteBio | BioBridgeOp::EndIo => self.stats.completions += 1,
            BioBridgeOp::MergeBio => self.stats.merges += 1,
            BioBridgeOp::SplitBio => self.stats.splits += 1,
            _ => {}
        }
        if rec.result != BioBridgeResult::Success { self.stats.errors += 1; }
    }

    pub fn merge_rate(&self) -> f64 {
        if self.stats.submits == 0 { 0.0 } else { self.stats.merges as f64 / self.stats.submits as f64 }
    }
}
