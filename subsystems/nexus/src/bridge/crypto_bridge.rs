// SPDX-License-Identifier: GPL-2.0
//! Bridge crypto_bridge — cryptographic subsystem interface bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Crypto algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CryptoAlgType {
    Cipher,
    Hash,
    Aead,
    Skcipher,
    Akcipher,
    Rng,
    Kdf,
    Compress,
}

/// Crypto priority
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoPriority {
    Software,
    Accelerated,
    Hardware,
}

impl CryptoPriority {
    pub fn weight(&self) -> u32 {
        match self {
            Self::Software => 100,
            Self::Accelerated => 200,
            Self::Hardware => 300,
        }
    }
}

/// A registered crypto algorithm
#[derive(Debug, Clone)]
pub struct CryptoAlg {
    pub name: String,
    pub driver_name: String,
    pub alg_type: CryptoAlgType,
    pub priority: CryptoPriority,
    pub block_size: u32,
    pub min_key_size: u32,
    pub max_key_size: u32,
    pub iv_size: u32,
    pub digest_size: u32,
    pub ref_count: u32,
    pub selftest_passed: bool,
}

impl CryptoAlg {
    pub fn new(name: String, alg_type: CryptoAlgType) -> Self {
        Self {
            name, driver_name: String::new(),
            alg_type, priority: CryptoPriority::Software,
            block_size: 0, min_key_size: 0, max_key_size: 0,
            iv_size: 0, digest_size: 0,
            ref_count: 0, selftest_passed: false,
        }
    }

    pub fn key_size_range(&self) -> (u32, u32) {
        (self.min_key_size, self.max_key_size)
    }

    pub fn is_aead(&self) -> bool {
        self.alg_type == CryptoAlgType::Aead
    }
}

/// Crypto operation request
#[derive(Debug, Clone)]
pub struct CryptoRequest {
    pub alg_name: String,
    pub op: CryptoOp,
    pub data_len: u64,
    pub result: CryptoResult,
    pub latency_ns: u64,
    pub timestamp: u64,
}

/// Crypto operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoOp {
    Encrypt,
    Decrypt,
    Hash,
    Sign,
    Verify,
    Generate,
    Compress,
    Decompress,
}

/// Crypto result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoResult {
    Success,
    InvalidKey,
    InvalidInput,
    BufferTooSmall,
    HardwareError,
    NotSupported,
    Timeout,
}

/// Per-algorithm stats
#[derive(Debug)]
pub struct AlgStats {
    pub name: String,
    pub total_ops: u64,
    pub total_bytes: u64,
    pub errors: u64,
    pub avg_latency_ns: u64,
    pub peak_latency_ns: u64,
}

impl AlgStats {
    pub fn new(name: String) -> Self {
        Self {
            name, total_ops: 0, total_bytes: 0,
            errors: 0, avg_latency_ns: 0, peak_latency_ns: 0,
        }
    }

    pub fn throughput_mbps(&self, elapsed_s: f64) -> f64 {
        if elapsed_s <= 0.0 { return 0.0; }
        (self.total_bytes as f64 / (1024.0 * 1024.0)) / elapsed_s
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_ops == 0 { return 0.0; }
        self.errors as f64 / self.total_ops as f64
    }
}

/// Crypto bridge stats
#[derive(Debug, Clone)]
pub struct CryptoBridgeStats {
    pub registered_algs: u32,
    pub total_requests: u64,
    pub total_bytes_processed: u64,
    pub total_errors: u64,
    pub hw_offload_count: u64,
    pub avg_latency_ns: u64,
}

/// Main crypto bridge
pub struct BridgeCrypto {
    algorithms: BTreeMap<String, CryptoAlg>,
    alg_stats: BTreeMap<String, AlgStats>,
    requests: Vec<CryptoRequest>,
    max_requests: usize,
    stats: CryptoBridgeStats,
}

impl BridgeCrypto {
    pub fn new() -> Self {
        Self {
            algorithms: BTreeMap::new(),
            alg_stats: BTreeMap::new(),
            requests: Vec::new(),
            max_requests: 4096,
            stats: CryptoBridgeStats {
                registered_algs: 0, total_requests: 0,
                total_bytes_processed: 0, total_errors: 0,
                hw_offload_count: 0, avg_latency_ns: 0,
            },
        }
    }

    pub fn register_alg(&mut self, alg: CryptoAlg) {
        self.stats.registered_algs += 1;
        self.algorithms.insert(alg.name.clone(), alg);
    }

    pub fn unregister_alg(&mut self, name: &str) -> bool {
        if self.algorithms.remove(name).is_some() {
            if self.stats.registered_algs > 0 { self.stats.registered_algs -= 1; }
            true
        } else { false }
    }

    pub fn record_request(&mut self, req: CryptoRequest) {
        self.stats.total_requests += 1;
        self.stats.total_bytes_processed += req.data_len;
        if req.result != CryptoResult::Success { self.stats.total_errors += 1; }

        if let Some(alg) = self.algorithms.get(&req.alg_name) {
            if alg.priority == CryptoPriority::Hardware {
                self.stats.hw_offload_count += 1;
            }
        }

        let n = self.stats.total_requests;
        self.stats.avg_latency_ns =
            ((self.stats.avg_latency_ns * (n - 1)) + req.latency_ns) / n;

        let entry = self.alg_stats.entry(req.alg_name.clone())
            .or_insert_with(|| AlgStats::new(req.alg_name.clone()));
        entry.total_ops += 1;
        entry.total_bytes += req.data_len;
        if req.result != CryptoResult::Success { entry.errors += 1; }
        if req.latency_ns > entry.peak_latency_ns { entry.peak_latency_ns = req.latency_ns; }
        entry.avg_latency_ns = ((entry.avg_latency_ns * (entry.total_ops - 1)) + req.latency_ns) / entry.total_ops;

        if self.requests.len() >= self.max_requests { self.requests.remove(0); }
        self.requests.push(req);
    }

    pub fn find_alg(&self, name: &str) -> Option<&CryptoAlg> {
        self.algorithms.get(name)
    }

    pub fn algs_by_type(&self, alg_type: CryptoAlgType) -> Vec<&CryptoAlg> {
        self.algorithms.values().filter(|a| a.alg_type == alg_type).collect()
    }

    pub fn best_alg(&self, name: &str) -> Option<&CryptoAlg> {
        self.algorithms.values()
            .filter(|a| a.name == name || a.driver_name == name)
            .max_by_key(|a| a.priority.weight())
    }

    pub fn busiest_algs(&self, n: usize) -> Vec<(&str, u64)> {
        let mut v: Vec<_> = self.alg_stats.iter()
            .map(|(name, s)| (name.as_str(), s.total_ops))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    pub fn stats(&self) -> &CryptoBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from crypto_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoV2AlgType {
    Cipher,
    Hash,
    Aead,
    Skcipher,
    Akcipher,
    Kpp,
    Rng,
    Compress,
}

/// Crypto operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoV2Op {
    Encrypt,
    Decrypt,
    Hash,
    Sign,
    Verify,
    KeyGen,
    AllocTfm,
    FreeTfm,
}

/// Crypto v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoV2Result {
    Success,
    InvalidKey,
    InvalidInput,
    BufferTooSmall,
    NotSupported,
    Busy,
    Error,
}

/// Crypto v2 record
#[derive(Debug, Clone)]
pub struct CryptoV2Record {
    pub op: CryptoV2Op,
    pub alg_type: CryptoV2AlgType,
    pub result: CryptoV2Result,
    pub alg_hash: u64,
    pub input_size: u32,
    pub output_size: u32,
    pub latency_ns: u64,
}

impl CryptoV2Record {
    pub fn new(op: CryptoV2Op, alg_type: CryptoV2AlgType, alg_name: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in alg_name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { op, alg_type, result: CryptoV2Result::Success, alg_hash: h, input_size: 0, output_size: 0, latency_ns: 0 }
    }
}

/// Crypto v2 bridge stats
#[derive(Debug, Clone)]
pub struct CryptoV2BridgeStats {
    pub total_ops: u64,
    pub encryptions: u64,
    pub decryptions: u64,
    pub hashes: u64,
    pub total_bytes: u64,
    pub errors: u64,
}

/// Main bridge crypto v2
#[derive(Debug)]
pub struct BridgeCryptoV2 {
    pub stats: CryptoV2BridgeStats,
}

impl BridgeCryptoV2 {
    pub fn new() -> Self {
        Self { stats: CryptoV2BridgeStats { total_ops: 0, encryptions: 0, decryptions: 0, hashes: 0, total_bytes: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &CryptoV2Record) {
        self.stats.total_ops += 1;
        self.stats.total_bytes += rec.input_size as u64;
        match rec.op {
            CryptoV2Op::Encrypt => self.stats.encryptions += 1,
            CryptoV2Op::Decrypt => self.stats.decryptions += 1,
            CryptoV2Op::Hash => self.stats.hashes += 1,
            _ => {}
        }
        if rec.result != CryptoV2Result::Success { self.stats.errors += 1; }
    }
}
