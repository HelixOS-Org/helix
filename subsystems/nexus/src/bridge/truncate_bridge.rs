// SPDX-License-Identifier: GPL-2.0
//! Bridge truncate — file truncate operation bridging

extern crate alloc;

/// Truncate bridge event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncateBridgeEvent { Truncate, Ftruncate, Extend, Punch }

/// Truncate bridge record
#[derive(Debug, Clone)]
pub struct TruncateBridgeRecord {
    pub event: TruncateBridgeEvent,
    pub fd: i32,
    pub old_size: u64,
    pub new_size: u64,
}

impl TruncateBridgeRecord {
    pub fn new(event: TruncateBridgeEvent) -> Self { Self { event, fd: -1, old_size: 0, new_size: 0 } }
}

/// Truncate bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TruncateBridgeStats { pub total_ops: u64, pub shrinks: u64, pub extends: u64, pub punches: u64 }

/// Main bridge truncate
#[derive(Debug)]
pub struct BridgeTruncate { pub stats: TruncateBridgeStats }

impl BridgeTruncate {
    pub fn new() -> Self { Self { stats: TruncateBridgeStats { total_ops: 0, shrinks: 0, extends: 0, punches: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &TruncateBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.event {
            TruncateBridgeEvent::Truncate | TruncateBridgeEvent::Ftruncate => {
                if rec.new_size < rec.old_size { self.stats.shrinks += 1; } else { self.stats.extends += 1; }
            }
            TruncateBridgeEvent::Extend => self.stats.extends += 1,
            TruncateBridgeEvent::Punch => self.stats.punches += 1,
        }
    }
}
