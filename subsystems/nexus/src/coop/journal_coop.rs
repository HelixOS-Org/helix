// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Journal (cooperative filesystem journaling)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Journal transaction type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopJournalTxType {
    MetadataUpdate,
    DataWrite,
    InodeChange,
    DirEntry,
    ExtentAlloc,
    ExtentFree,
    Checkpoint,
    Barrier,
}

/// Journal transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopJournalTxState {
    Running,
    Committing,
    Committed,
    Checkpointed,
    Aborted,
}

/// Journal transaction entry
#[derive(Debug, Clone)]
pub struct CoopJournalTx {
    pub tx_id: u64,
    pub tx_type: CoopJournalTxType,
    pub state: CoopJournalTxState,
    pub block_count: u32,
    pub timestamp: u64,
    pub sequence: u64,
}

/// Stats for journal cooperation
#[derive(Debug, Clone)]
pub struct CoopJournalStats {
    pub total_transactions: u64,
    pub committed: u64,
    pub aborted: u64,
    pub checkpoints: u64,
    pub journal_full_events: u64,
    pub avg_tx_blocks: u64,
}

/// Manager for journal cooperative operations
pub struct CoopJournalManager {
    transactions: BTreeMap<u64, CoopJournalTx>,
    commit_queue: Vec<u64>,
    next_tx: u64,
    next_seq: u64,
    stats: CoopJournalStats,
    max_journal_blocks: u64,
    used_blocks: u64,
}

impl CoopJournalManager {
    pub fn new() -> Self {
        Self {
            transactions: BTreeMap::new(),
            commit_queue: Vec::new(),
            next_tx: 1,
            next_seq: 1,
            stats: CoopJournalStats {
                total_transactions: 0,
                committed: 0,
                aborted: 0,
                checkpoints: 0,
                journal_full_events: 0,
                avg_tx_blocks: 0,
            },
            max_journal_blocks: 32768,
            used_blocks: 0,
        }
    }

    pub fn begin_tx(&mut self, tx_type: CoopJournalTxType, block_count: u32) -> Option<u64> {
        if self.used_blocks + block_count as u64 > self.max_journal_blocks {
            self.stats.journal_full_events += 1;
            return None;
        }
        let id = self.next_tx;
        self.next_tx += 1;
        let seq = self.next_seq;
        self.next_seq += 1;
        let tx = CoopJournalTx {
            tx_id: id,
            tx_type,
            state: CoopJournalTxState::Running,
            block_count,
            timestamp: id.wrapping_mul(43),
            sequence: seq,
        };
        self.transactions.insert(id, tx);
        self.used_blocks += block_count as u64;
        self.stats.total_transactions += 1;
        Some(id)
    }

    pub fn commit_tx(&mut self, tx_id: u64) -> bool {
        if let Some(tx) = self.transactions.get_mut(&tx_id) {
            if tx.state == CoopJournalTxState::Running {
                tx.state = CoopJournalTxState::Committing;
                self.commit_queue.push(tx_id);
                return true;
            }
        }
        false
    }

    pub fn flush_commits(&mut self) -> usize {
        let queue: Vec<u64> = self.commit_queue.drain(..).collect();
        let count = queue.len();
        for tx_id in queue {
            if let Some(tx) = self.transactions.get_mut(&tx_id) {
                tx.state = CoopJournalTxState::Committed;
                self.stats.committed += 1;
            }
        }
        count
    }

    pub fn checkpoint(&mut self) -> usize {
        let committed: Vec<u64> = self.transactions.iter()
            .filter(|(_, tx)| tx.state == CoopJournalTxState::Committed)
            .map(|(&id, _)| id)
            .collect();
        let count = committed.len();
        for tx_id in committed {
            if let Some(tx) = self.transactions.get_mut(&tx_id) {
                self.used_blocks = self.used_blocks.saturating_sub(tx.block_count as u64);
                tx.state = CoopJournalTxState::Checkpointed;
            }
        }
        self.stats.checkpoints += 1;
        count
    }

    pub fn stats(&self) -> &CoopJournalStats {
        &self.stats
    }
}
