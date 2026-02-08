// SPDX-License-Identifier: GPL-2.0
//! Coop crypto — cooperative crypto resource sharing

extern crate alloc;
use alloc::vec::Vec;

/// Crypto coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoCoopEvent {
    TfmShare,
    KeyScheduleCache,
    BatchEncrypt,
    BatchDecrypt,
    AlgNegotiate,
}

/// Crypto coop mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoCoopMode {
    Exclusive,
    SharedReadOnly,
    PooledTfm,
    Batched,
}

/// Crypto coop record
#[derive(Debug, Clone)]
pub struct CryptoCoopRecord {
    pub event: CryptoCoopEvent,
    pub mode: CryptoCoopMode,
    pub alg_hash: u64,
    pub participants: u32,
    pub bytes_processed: u64,
}

impl CryptoCoopRecord {
    pub fn new(event: CryptoCoopEvent, alg: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in alg { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { event, mode: CryptoCoopMode::Exclusive, alg_hash: h, participants: 0, bytes_processed: 0 }
    }
}

/// Crypto coop stats
#[derive(Debug, Clone)]
pub struct CryptoCoopStats {
    pub total_events: u64,
    pub tfm_shares: u64,
    pub batch_ops: u64,
    pub bytes_saved: u64,
}

/// Main coop crypto
#[derive(Debug)]
pub struct CoopCrypto {
    pub stats: CryptoCoopStats,
}

impl CoopCrypto {
    pub fn new() -> Self {
        Self { stats: CryptoCoopStats { total_events: 0, tfm_shares: 0, batch_ops: 0, bytes_saved: 0 } }
    }

    pub fn record(&mut self, rec: &CryptoCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            CryptoCoopEvent::TfmShare | CryptoCoopEvent::KeyScheduleCache => self.stats.tfm_shares += 1,
            CryptoCoopEvent::BatchEncrypt | CryptoCoopEvent::BatchDecrypt => self.stats.batch_ops += 1,
            _ => {}
        }
    }
}
