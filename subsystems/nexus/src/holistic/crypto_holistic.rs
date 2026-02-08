// SPDX-License-Identifier: GPL-2.0
//! Holistic crypto — cross-layer cryptographic usage analysis

extern crate alloc;
use alloc::vec::Vec;

/// Crypto holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoHolisticMetric {
    WeakAlgorithm,
    KeyReuse,
    EntropyQuality,
    TfmContention,
    AlgorithmDiversity,
}

/// Crypto finding
#[derive(Debug, Clone)]
pub struct CryptoHolisticFinding {
    pub metric: CryptoHolisticMetric,
    pub score: u64,
    pub alg_hash: u64,
    pub key_size: u32,
    pub usage_count: u64,
    pub entropy_bits: u32,
}

impl CryptoHolisticFinding {
    pub fn new(metric: CryptoHolisticMetric) -> Self {
        Self { metric, score: 0, alg_hash: 0, key_size: 0, usage_count: 0, entropy_bits: 0 }
    }
}

/// Crypto holistic stats
#[derive(Debug, Clone)]
pub struct CryptoHolisticStats {
    pub total_analyses: u64,
    pub weak_algorithms: u64,
    pub key_reuses: u64,
    pub low_entropy: u64,
}

/// Main holistic crypto
#[derive(Debug)]
pub struct HolisticCrypto {
    pub stats: CryptoHolisticStats,
}

impl HolisticCrypto {
    pub fn new() -> Self {
        Self { stats: CryptoHolisticStats { total_analyses: 0, weak_algorithms: 0, key_reuses: 0, low_entropy: 0 } }
    }

    pub fn analyze(&mut self, finding: &CryptoHolisticFinding) {
        self.stats.total_analyses += 1;
        match finding.metric {
            CryptoHolisticMetric::WeakAlgorithm => self.stats.weak_algorithms += 1,
            CryptoHolisticMetric::KeyReuse => self.stats.key_reuses += 1,
            CryptoHolisticMetric::EntropyQuality => {
                if finding.entropy_bits < 128 { self.stats.low_entropy += 1; }
            }
            _ => {}
        }
    }
}
