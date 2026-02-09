// SPDX-License-Identifier: GPL-2.0
//! Bridge select — fd_set based I/O multiplexing with tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Select fd set type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectFdSet {
    Read,
    Write,
    Except,
}

/// Select fd mask (1024-bit)
#[derive(Debug, Clone)]
pub struct SelectMask {
    pub bits: [u64; 16],
}

impl SelectMask {
    pub fn new() -> Self {
        Self { bits: [0; 16] }
    }

    #[inline]
    pub fn set(&mut self, fd: i32) {
        if fd >= 0 && (fd as usize) < 1024 {
            let idx = fd as usize / 64;
            let bit = fd as usize % 64;
            self.bits[idx] |= 1u64 << bit;
        }
    }

    #[inline]
    pub fn clear(&mut self, fd: i32) {
        if fd >= 0 && (fd as usize) < 1024 {
            let idx = fd as usize / 64;
            let bit = fd as usize % 64;
            self.bits[idx] &= !(1u64 << bit);
        }
    }

    #[inline]
    pub fn is_set(&self, fd: i32) -> bool {
        if fd >= 0 && (fd as usize) < 1024 {
            let idx = fd as usize / 64;
            let bit = fd as usize % 64;
            self.bits[idx] & (1u64 << bit) != 0
        } else {
            false
        }
    }

    #[inline]
    pub fn count(&self) -> u32 {
        let mut c = 0u32;
        for word in &self.bits {
            let mut w = *word;
            while w != 0 {
                c += 1;
                w &= w - 1;
            }
        }
        c
    }

    #[inline]
    pub fn intersect(&self, other: &SelectMask) -> SelectMask {
        let mut result = SelectMask::new();
        for i in 0..16 {
            result.bits[i] = self.bits[i] & other.bits[i];
        }
        result
    }

    #[inline]
    pub fn highest_fd(&self) -> i32 {
        for i in (0..16).rev() {
            if self.bits[i] != 0 {
                let bit = 63 - self.bits[i].leading_zeros() as i32;
                return (i as i32) * 64 + bit;
            }
        }
        -1
    }
}

/// Select call record
#[derive(Debug, Clone)]
pub struct SelectCall {
    pub call_id: u64,
    pub nfds: i32,
    pub read_set: SelectMask,
    pub write_set: SelectMask,
    pub except_set: SelectMask,
    pub timeout_us: i64,
    pub result: i32,
    pub duration_ns: u64,
}

impl SelectCall {
    pub fn new(call_id: u64, nfds: i32, timeout_us: i64) -> Self {
        Self {
            call_id,
            nfds,
            read_set: SelectMask::new(),
            write_set: SelectMask::new(),
            except_set: SelectMask::new(),
            timeout_us,
            result: 0,
            duration_ns: 0,
        }
    }

    #[inline(always)]
    pub fn total_fds_monitored(&self) -> u32 {
        self.read_set.count() + self.write_set.count() + self.except_set.count()
    }
}

/// Select bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SelectBridgeStats {
    pub total_calls: u64,
    pub total_fds_monitored: u64,
    pub total_ready: u64,
    pub timeouts: u64,
    pub max_nfds_seen: i32,
}

/// Main bridge select
#[derive(Debug)]
pub struct BridgeSelect {
    pub stats: SelectBridgeStats,
    pub next_call_id: u64,
}

impl BridgeSelect {
    pub fn new() -> Self {
        Self {
            stats: SelectBridgeStats {
                total_calls: 0,
                total_fds_monitored: 0,
                total_ready: 0,
                timeouts: 0,
                max_nfds_seen: 0,
            },
            next_call_id: 1,
        }
    }

    pub fn record_call(&mut self, call: &SelectCall) {
        self.stats.total_calls += 1;
        self.stats.total_fds_monitored += call.total_fds_monitored() as u64;
        if call.result > 0 {
            self.stats.total_ready += call.result as u64;
        } else if call.result == 0 {
            self.stats.timeouts += 1;
        }
        if call.nfds > self.stats.max_nfds_seen {
            self.stats.max_nfds_seen = call.nfds;
        }
    }

    #[inline(always)]
    pub fn timeout_rate(&self) -> f64 {
        if self.stats.total_calls == 0 { 0.0 } else { self.stats.timeouts as f64 / self.stats.total_calls as f64 }
    }

    #[inline(always)]
    pub fn avg_fds_per_call(&self) -> f64 {
        if self.stats.total_calls == 0 { 0.0 } else { self.stats.total_fds_monitored as f64 / self.stats.total_calls as f64 }
    }
}

// ============================================================================
// Merged from select_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectV2Set { ReadFds, WriteFds, ExceptFds }

/// Select v2 record
#[derive(Debug, Clone)]
pub struct SelectV2Record {
    pub set: SelectV2Set,
    pub nfds: i32,
    pub ready: u32,
    pub timeout_us: i64,
}

impl SelectV2Record {
    pub fn new(set: SelectV2Set, nfds: i32) -> Self { Self { set, nfds, ready: 0, timeout_us: -1 } }
}

/// Select v2 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SelectV2BridgeStats { pub total_selects: u64, pub read_ready: u64, pub write_ready: u64, pub timeouts: u64 }

/// Main bridge select v2
#[derive(Debug)]
pub struct BridgeSelectV2 { pub stats: SelectV2BridgeStats }

impl BridgeSelectV2 {
    pub fn new() -> Self { Self { stats: SelectV2BridgeStats { total_selects: 0, read_ready: 0, write_ready: 0, timeouts: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &SelectV2Record) {
        self.stats.total_selects += 1;
        match rec.set {
            SelectV2Set::ReadFds => self.stats.read_ready += rec.ready as u64,
            SelectV2Set::WriteFds => self.stats.write_ready += rec.ready as u64,
            _ => {}
        }
        if rec.ready == 0 && rec.timeout_us >= 0 { self.stats.timeouts += 1; }
    }
}
