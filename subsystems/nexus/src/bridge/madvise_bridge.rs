// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Madvise (memory advice bridge)

extern crate alloc;
use alloc::vec::Vec;

/// Madvise advice type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMadviseAdvice { Normal, Random, Sequential, WillNeed, DontNeed, Free, Mergeable, Hugepage, DontFork, Cold }

/// Madvise record
#[derive(Debug, Clone)]
pub struct BridgeMadviseRecord { pub addr: u64, pub length: u64, pub advice: BridgeMadviseAdvice }

/// Madvise stats
#[derive(Debug, Clone)]
pub struct BridgeMadviseStats {
    pub total_ops: u64, pub dontneed: u64, pub willneed: u64, pub hugepage_hints: u64,
    pub mergeable_hints: u64, pub total_affected_bytes: u64,
}

/// Manager for madvise bridge
pub struct BridgeMadviseManager {
    history: Vec<BridgeMadviseRecord>,
    stats: BridgeMadviseStats,
}

impl BridgeMadviseManager {
    pub fn new() -> Self {
        Self { history: Vec::new(), stats: BridgeMadviseStats { total_ops: 0, dontneed: 0, willneed: 0, hugepage_hints: 0, mergeable_hints: 0, total_affected_bytes: 0 } }
    }

    pub fn advise(&mut self, addr: u64, length: u64, advice: BridgeMadviseAdvice) {
        self.stats.total_ops += 1;
        self.stats.total_affected_bytes += length;
        match advice {
            BridgeMadviseAdvice::DontNeed | BridgeMadviseAdvice::Free => self.stats.dontneed += 1,
            BridgeMadviseAdvice::WillNeed => self.stats.willneed += 1,
            BridgeMadviseAdvice::Hugepage => self.stats.hugepage_hints += 1,
            BridgeMadviseAdvice::Mergeable => self.stats.mergeable_hints += 1,
            _ => {}
        }
        self.history.push(BridgeMadviseRecord { addr, length, advice });
    }

    pub fn stats(&self) -> &BridgeMadviseStats { &self.stats }
}
