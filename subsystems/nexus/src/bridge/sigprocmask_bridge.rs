// SPDX-License-Identifier: GPL-2.0
//! Bridge sigprocmask — signal mask manipulation bridge

extern crate alloc;

/// Sigprocmask how
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigprocmaskHow {
    SigBlock,
    SigUnblock,
    SigSetmask,
}

/// Sigprocmask record
#[derive(Debug, Clone)]
pub struct SigprocmaskRecord {
    pub how: SigprocmaskHow,
    pub mask_bits: u64,
    pub old_mask_bits: u64,
    pub pid: u32,
    pub tid: u32,
}

impl SigprocmaskRecord {
    pub fn new(how: SigprocmaskHow, mask: u64) -> Self {
        Self { how, mask_bits: mask, old_mask_bits: 0, pid: 0, tid: 0 }
    }

    #[inline(always)]
    pub fn blocked_count(&self) -> u32 {
        self.mask_bits.count_ones()
    }
}

/// Sigprocmask bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SigprocmaskBridgeStats {
    pub total_ops: u64,
    pub blocks: u64,
    pub unblocks: u64,
    pub full_sets: u64,
}

/// Main bridge sigprocmask
#[derive(Debug)]
pub struct BridgeSigprocmask {
    pub stats: SigprocmaskBridgeStats,
}

impl BridgeSigprocmask {
    pub fn new() -> Self {
        Self { stats: SigprocmaskBridgeStats { total_ops: 0, blocks: 0, unblocks: 0, full_sets: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &SigprocmaskRecord) {
        self.stats.total_ops += 1;
        match rec.how {
            SigprocmaskHow::SigBlock => self.stats.blocks += 1,
            SigprocmaskHow::SigUnblock => self.stats.unblocks += 1,
            SigprocmaskHow::SigSetmask => self.stats.full_sets += 1,
        }
    }
}
