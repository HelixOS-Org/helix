// SPDX-License-Identifier: GPL-2.0
//! Holistic journal — filesystem journal/log analysis

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Journal operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalOp {
    StartTransaction,
    CommitTransaction,
    AbortTransaction,
    Checkpoint,
    Recovery,
    LogWrite,
    LogFlush,
}

/// Journal state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalState {
    Clean,
    Active,
    Committing,
    Recovering,
    Error,
}

/// Transaction record
#[derive(Debug, Clone)]
pub struct JournalTransaction {
    pub tid: u64,
    pub state: JournalState,
    pub blocks_logged: u64,
    pub start_ns: u64,
    pub commit_ns: u64,
    pub handles: u32,
}

impl JournalTransaction {
    pub fn new(tid: u64) -> Self {
        Self { tid, state: JournalState::Active, blocks_logged: 0, start_ns: 0, commit_ns: 0, handles: 0 }
    }

    pub fn commit(&mut self, now_ns: u64) { self.state = JournalState::Committing; self.commit_ns = now_ns; }
    pub fn latency_ns(&self) -> u64 { self.commit_ns.saturating_sub(self.start_ns) }
    pub fn add_block(&mut self) { self.blocks_logged += 1; }
    pub fn add_handle(&mut self) { self.handles += 1; }
}

/// Holistic journal stats
#[derive(Debug, Clone)]
pub struct HolisticJournalStats {
    pub total_transactions: u64,
    pub committed: u64,
    pub aborted: u64,
    pub checkpoints: u64,
    pub total_blocks_logged: u64,
    pub total_latency_ns: u64,
}

/// Main holistic journal
#[derive(Debug)]
pub struct HolisticJournal {
    pub stats: HolisticJournalStats,
}

impl HolisticJournal {
    pub fn new() -> Self {
        Self { stats: HolisticJournalStats { total_transactions: 0, committed: 0, aborted: 0, checkpoints: 0, total_blocks_logged: 0, total_latency_ns: 0 } }
    }

    pub fn record_op(&mut self, op: JournalOp, txn: &JournalTransaction) {
        match op {
            JournalOp::StartTransaction => self.stats.total_transactions += 1,
            JournalOp::CommitTransaction => { self.stats.committed += 1; self.stats.total_latency_ns += txn.latency_ns(); self.stats.total_blocks_logged += txn.blocks_logged; }
            JournalOp::AbortTransaction => self.stats.aborted += 1,
            JournalOp::Checkpoint => self.stats.checkpoints += 1,
            _ => {}
        }
    }

    pub fn avg_commit_latency_ns(&self) -> u64 {
        if self.stats.committed == 0 { 0 } else { self.stats.total_latency_ns / self.stats.committed }
    }
}

// ============================================================================
// Merged from journal_v2_holistic
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticJournalV2Metric {
    TransactionRate,
    CommitLatency,
    CheckpointInterval,
    JournalUtilization,
    LogWraps,
    AbortRate,
    RecoveryTime,
}

/// Journal analysis sample
#[derive(Debug, Clone)]
pub struct HolisticJournalV2Sample {
    pub metric: HolisticJournalV2Metric,
    pub value: u64,
    pub timestamp: u64,
}

/// Journal health assessment
#[derive(Debug, Clone)]
pub struct HolisticJournalV2Health {
    pub throughput_score: u64,
    pub latency_score: u64,
    pub utilization_score: u64,
    pub reliability_score: u64,
    pub overall: u64,
}

/// Stats for journal analysis
#[derive(Debug, Clone)]
pub struct HolisticJournalV2Stats {
    pub samples: u64,
    pub analyses: u64,
    pub performance_alerts: u64,
    pub reliability_alerts: u64,
}

/// Manager for journal holistic analysis
pub struct HolisticJournalV2Manager {
    samples: Vec<HolisticJournalV2Sample>,
    health: HolisticJournalV2Health,
    stats: HolisticJournalV2Stats,
    window: usize,
}

impl HolisticJournalV2Manager {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            health: HolisticJournalV2Health {
                throughput_score: 100,
                latency_score: 100,
                utilization_score: 50,
                reliability_score: 100,
                overall: 100,
            },
            stats: HolisticJournalV2Stats {
                samples: 0,
                analyses: 0,
                performance_alerts: 0,
                reliability_alerts: 0,
            },
            window: 500,
        }
    }

    pub fn record(&mut self, metric: HolisticJournalV2Metric, value: u64) {
        let sample = HolisticJournalV2Sample {
            metric,
            value,
            timestamp: self.samples.len() as u64,
        };
        self.samples.push(sample);
        self.stats.samples += 1;
        if self.samples.len() > self.window {
            self.samples.remove(0);
        }
    }

    pub fn analyze(&mut self) -> &HolisticJournalV2Health {
        self.stats.analyses += 1;
        let commit_samples: Vec<&HolisticJournalV2Sample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticJournalV2Metric::CommitLatency))
            .collect();
        if !commit_samples.is_empty() {
            let avg: u64 = commit_samples.iter().map(|s| s.value).sum::<u64>() / commit_samples.len() as u64;
            self.health.latency_score = if avg < 1000 { 100 } else if avg < 5000 { 70 } else { 30 };
        }
        let abort_samples: Vec<&HolisticJournalV2Sample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticJournalV2Metric::AbortRate))
            .collect();
        if !abort_samples.is_empty() {
            let avg: u64 = abort_samples.iter().map(|s| s.value).sum::<u64>() / abort_samples.len() as u64;
            if avg > 5 {
                self.stats.reliability_alerts += 1;
                self.health.reliability_score = 100u64.saturating_sub(avg * 10);
            }
        }
        self.health.overall = (self.health.throughput_score + self.health.latency_score + self.health.reliability_score) / 3;
        &self.health
    }

    pub fn stats(&self) -> &HolisticJournalV2Stats {
        &self.stats
    }
}
