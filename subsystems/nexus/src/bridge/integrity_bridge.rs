// SPDX-License-Identifier: GPL-2.0
//! Bridge integrity — IMA/EVM integrity measurement bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Integrity operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityOp {
    Measure,
    Appraise,
    Audit,
    Hash,
    Collect,
    Validate,
}

/// Integrity result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityResult {
    Ok,
    Mismatch,
    Missing,
    Invalid,
    Corrupted,
    Error,
}

/// Integrity record
#[derive(Debug, Clone)]
pub struct IntegrityRecord {
    pub op: IntegrityOp,
    pub result: IntegrityResult,
    pub path_hash: u64,
    pub digest_alg_hash: u64,
    pub inode: u64,
    pub file_size: u64,
}

impl IntegrityRecord {
    pub fn new(op: IntegrityOp, path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            op,
            result: IntegrityResult::Ok,
            path_hash: h,
            digest_alg_hash: 0,
            inode: 0,
            file_size: 0,
        }
    }
}

/// Integrity bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IntegrityBridgeStats {
    pub total_ops: u64,
    pub measurements: u64,
    pub appraisals: u64,
    pub mismatches: u64,
    pub errors: u64,
}

/// Main bridge integrity
#[derive(Debug)]
pub struct BridgeIntegrity {
    pub stats: IntegrityBridgeStats,
}

impl BridgeIntegrity {
    pub fn new() -> Self {
        Self {
            stats: IntegrityBridgeStats {
                total_ops: 0,
                measurements: 0,
                appraisals: 0,
                mismatches: 0,
                errors: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &IntegrityRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            IntegrityOp::Measure | IntegrityOp::Collect => self.stats.measurements += 1,
            IntegrityOp::Appraise | IntegrityOp::Validate => self.stats.appraisals += 1,
            _ => {},
        }
        if rec.result == IntegrityResult::Mismatch {
            self.stats.mismatches += 1;
        }
        if matches!(
            rec.result,
            IntegrityResult::Invalid | IntegrityResult::Corrupted | IntegrityResult::Error
        ) {
            self.stats.errors += 1;
        }
    }
}
