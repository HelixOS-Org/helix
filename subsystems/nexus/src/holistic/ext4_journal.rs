// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic ext4 journal — JBD2 journal transaction tracking
//!
//! Models the ext4/JBD2 journaling system with transaction lifecycle,
//! checkpoint management, journal space accounting, and fast commit.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Journal transaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalTxState {
    Running,
    Locked,
    Flush,
    Commit,
    CommitDone,
    Checkpoint,
    Finished,
}

/// Journal mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalMode {
    Ordered,
    Writeback,
    Journal,
}

/// A journal transaction.
#[derive(Debug, Clone)]
pub struct JournalTransaction {
    pub tx_id: u64,
    pub sequence: u64,
    pub state: JournalTxState,
    pub blocks_logged: u32,
    pub blocks_reserved: u32,
    pub handles: u32,
    pub start_time: u64,
    pub commit_time: u64,
    pub is_fast_commit: bool,
}

impl JournalTransaction {
    pub fn new(tx_id: u64, sequence: u64) -> Self {
        Self {
            tx_id,
            sequence,
            state: JournalTxState::Running,
            blocks_logged: 0,
            blocks_reserved: 0,
            handles: 0,
            start_time: 0,
            commit_time: 0,
            is_fast_commit: false,
        }
    }

    pub fn add_block(&mut self) {
        self.blocks_logged += 1;
    }

    pub fn commit(&mut self, time: u64) {
        self.state = JournalTxState::Commit;
        self.commit_time = time;
    }

    pub fn commit_latency(&self) -> u64 {
        self.commit_time.saturating_sub(self.start_time)
    }
}

/// Journal space accounting.
#[derive(Debug, Clone)]
pub struct JournalSpace {
    pub total_blocks: u64,
    pub used_blocks: u64,
    pub checkpoint_blocks: u64,
    pub reserved_blocks: u64,
}

impl JournalSpace {
    pub fn new(total: u64) -> Self {
        Self {
            total_blocks: total,
            used_blocks: 0,
            checkpoint_blocks: 0,
            reserved_blocks: 0,
        }
    }

    pub fn free_blocks(&self) -> u64 {
        self.total_blocks
            .saturating_sub(self.used_blocks)
            .saturating_sub(self.checkpoint_blocks)
            .saturating_sub(self.reserved_blocks)
    }

    pub fn utilization(&self) -> f64 {
        if self.total_blocks == 0 {
            return 0.0;
        }
        self.used_blocks as f64 / self.total_blocks as f64
    }
}

/// Statistics for ext4 journal.
#[derive(Debug, Clone)]
pub struct Ext4JournalStats {
    pub total_transactions: u64,
    pub total_commits: u64,
    pub total_checkpoints: u64,
    pub fast_commits: u64,
    pub total_blocks_logged: u64,
    pub avg_commit_latency: u64,
    pub journal_full_count: u64,
}

/// Main holistic ext4 journal manager.
pub struct HolisticExt4Journal {
    pub transactions: BTreeMap<u64, JournalTransaction>,
    pub space: JournalSpace,
    pub mode: JournalMode,
    pub next_tx_id: u64,
    pub next_sequence: u64,
    pub stats: Ext4JournalStats,
}

impl HolisticExt4Journal {
    pub fn new(journal_blocks: u64, mode: JournalMode) -> Self {
        Self {
            transactions: BTreeMap::new(),
            space: JournalSpace::new(journal_blocks),
            mode,
            next_tx_id: 1,
            next_sequence: 1,
            stats: Ext4JournalStats {
                total_transactions: 0,
                total_commits: 0,
                total_checkpoints: 0,
                fast_commits: 0,
                total_blocks_logged: 0,
                avg_commit_latency: 0,
                journal_full_count: 0,
            },
        }
    }

    pub fn begin_transaction(&mut self, time: u64) -> u64 {
        let id = self.next_tx_id;
        self.next_tx_id += 1;
        let seq = self.next_sequence;
        self.next_sequence += 1;
        let mut tx = JournalTransaction::new(id, seq);
        tx.start_time = time;
        self.transactions.insert(id, tx);
        self.stats.total_transactions += 1;
        id
    }

    pub fn commit_transaction(&mut self, tx_id: u64, time: u64) {
        if let Some(tx) = self.transactions.get_mut(&tx_id) {
            tx.commit(time);
            self.stats.total_commits += 1;
            self.stats.total_blocks_logged += tx.blocks_logged as u64;
        }
    }

    pub fn tx_count(&self) -> usize {
        self.transactions.len()
    }
}
