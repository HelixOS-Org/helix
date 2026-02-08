// SPDX-License-Identifier: GPL-2.0
//! Bridge random — getrandom/urandom bridge with entropy tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Random source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomSource {
    Urandom,
    Random,
    Getrandom,
    GetrandomInsecure,
    HwRng,
}

/// Random operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomOp {
    Read,
    Write,
    Ioctl,
    Getrandom,
    AddEntropy,
}

/// Random result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomResult {
    Success,
    WouldBlock,
    Interrupted,
    Fault,
    Error,
}

/// Random record
#[derive(Debug, Clone)]
pub struct RandomRecord {
    pub op: RandomOp,
    pub source: RandomSource,
    pub result: RandomResult,
    pub bytes_requested: u32,
    pub bytes_returned: u32,
    pub flags: u32,
}

impl RandomRecord {
    pub fn new(op: RandomOp, source: RandomSource, bytes: u32) -> Self {
        Self {
            op,
            source,
            result: RandomResult::Success,
            bytes_requested: bytes,
            bytes_returned: bytes,
            flags: 0,
        }
    }

    pub fn was_partial(&self) -> bool {
        self.bytes_returned < self.bytes_requested
    }
}

/// Random bridge stats
#[derive(Debug, Clone)]
pub struct RandomBridgeStats {
    pub total_ops: u64,
    pub bytes_generated: u64,
    pub getrandom_calls: u64,
    pub would_blocks: u64,
    pub entropy_adds: u64,
}

/// Main bridge random
#[derive(Debug)]
pub struct BridgeRandom {
    pub stats: RandomBridgeStats,
}

impl BridgeRandom {
    pub fn new() -> Self {
        Self {
            stats: RandomBridgeStats {
                total_ops: 0,
                bytes_generated: 0,
                getrandom_calls: 0,
                would_blocks: 0,
                entropy_adds: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &RandomRecord) {
        self.stats.total_ops += 1;
        self.stats.bytes_generated += rec.bytes_returned as u64;
        match rec.op {
            RandomOp::Getrandom => self.stats.getrandom_calls += 1,
            RandomOp::AddEntropy => self.stats.entropy_adds += 1,
            _ => {},
        }
        if rec.result == RandomResult::WouldBlock {
            self.stats.would_blocks += 1;
        }
    }
}
