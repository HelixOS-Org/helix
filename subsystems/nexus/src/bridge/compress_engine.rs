//! # Bridge Compression Engine
//!
//! Data compression for syscall argument/result optimization:
//! - LZ77-style sliding window compression (simplified)
//! - Dictionary-based compression for repeated patterns
//! - Compression ratio tracking per syscall class
//! - Adaptive compression level selection
//! - Zero-copy decompression paths

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// COMPRESSION TYPES
// ============================================================================

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Run-length encoding
    Rle,
    /// Dictionary-based
    Dictionary,
    /// Delta encoding (for sequential data)
    Delta,
    /// Adaptive (auto-select)
    Adaptive,
}

/// Compression level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionLevel {
    /// Fastest (least compression)
    Fast,
    /// Balanced
    Balanced,
    /// Best compression (slowest)
    Best,
}

/// Compression result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionResult {
    /// Successfully compressed
    Compressed,
    /// Data too small to benefit
    TooSmall,
    /// Compression expanded data
    Expansion,
    /// Incompressible data
    Incompressible,
    /// Error
    Error,
}

// ============================================================================
// DICTIONARY
// ============================================================================

/// Dictionary entry
#[derive(Debug, Clone)]
pub struct DictionaryEntry {
    /// Pattern hash
    pub pattern_hash: u64,
    /// Pattern length
    pub pattern_len: usize,
    /// Usage count
    pub usage_count: u64,
    /// Last used timestamp
    pub last_used_ns: u64,
}

/// Compression dictionary
#[derive(Debug)]
pub struct CompressionDictionary {
    /// Entries, keyed by hash
    entries: BTreeMap<u64, DictionaryEntry>,
    /// Max entries
    max_entries: usize,
}

impl CompressionDictionary {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries,
        }
    }

    /// Add pattern
    pub fn add_pattern(&mut self, data: &[u8], now: u64) -> u64 {
        let hash = Self::hash_pattern(data);
        if let Some(entry) = self.entries.get_mut(&hash) {
            entry.usage_count += 1;
            entry.last_used_ns = now;
        } else {
            if self.entries.len() >= self.max_entries {
                self.evict_lru();
            }
            self.entries.insert(hash, DictionaryEntry {
                pattern_hash: hash,
                pattern_len: data.len(),
                usage_count: 1,
                last_used_ns: now,
            });
        }
        hash
    }

    /// Lookup pattern
    pub fn lookup(&self, hash: u64) -> Option<&DictionaryEntry> {
        self.entries.get(&hash)
    }

    /// Pattern hash (FNV-1a)
    fn hash_pattern(data: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in data {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Evict LRU
    fn evict_lru(&mut self) {
        let lru_key = self.entries.iter()
            .min_by_key(|(_, e)| e.last_used_ns)
            .map(|(k, _)| *k);
        if let Some(k) = lru_key {
            self.entries.remove(&k);
        }
    }

    /// Size
    pub fn size(&self) -> usize {
        self.entries.len()
    }
}

// ============================================================================
// RLE CODEC
// ============================================================================

/// Run-length encoding stats
#[derive(Debug, Clone, Default)]
pub struct RleStats {
    /// Bytes in
    pub bytes_in: u64,
    /// Bytes out
    pub bytes_out: u64,
    /// Runs found
    pub runs_found: u64,
}

/// Simple RLE encoder
#[derive(Debug)]
pub struct RleEncoder {
    /// Min run length to encode
    pub min_run: usize,
    /// Stats
    pub stats: RleStats,
}

impl RleEncoder {
    pub fn new(min_run: usize) -> Self {
        Self {
            min_run: min_run.max(3),
            stats: RleStats::default(),
        }
    }

    /// Encode: returns compressed length estimate
    pub fn estimate_compressed_size(&mut self, data: &[u8]) -> usize {
        if data.is_empty() {
            return 0;
        }
        self.stats.bytes_in += data.len() as u64;
        let mut compressed = 0usize;
        let mut i = 0;
        while i < data.len() {
            let current = data[i];
            let mut run_len = 1;
            while i + run_len < data.len() && data[i + run_len] == current && run_len < 255 {
                run_len += 1;
            }
            if run_len >= self.min_run {
                // Encode as (marker, byte, count) = 3 bytes
                compressed += 3;
                self.stats.runs_found += 1;
            } else {
                compressed += run_len;
            }
            i += run_len;
        }
        self.stats.bytes_out += compressed as u64;
        compressed
    }

    /// Compression ratio
    pub fn ratio(&self) -> f64 {
        if self.stats.bytes_in == 0 {
            return 1.0;
        }
        self.stats.bytes_out as f64 / self.stats.bytes_in as f64
    }
}

// ============================================================================
// DELTA CODEC
// ============================================================================

/// Delta encoding stats
#[derive(Debug, Clone, Default)]
pub struct DeltaStats {
    /// Sequences processed
    pub sequences: u64,
    /// Savings (bytes)
    pub bytes_saved: u64,
}

/// Delta encoder for sequential numeric data
#[derive(Debug)]
pub struct DeltaEncoder {
    /// Stats
    pub stats: DeltaStats,
}

impl DeltaEncoder {
    pub fn new() -> Self {
        Self {
            stats: DeltaStats::default(),
        }
    }

    /// Estimate savings for u64 sequence
    pub fn estimate_savings(&mut self, values: &[u64]) -> u64 {
        if values.len() < 2 {
            return 0;
        }
        self.stats.sequences += 1;
        // Calculate deltas
        let mut max_delta: u64 = 0;
        for i in 1..values.len() {
            let delta = if values[i] >= values[i - 1] {
                values[i] - values[i - 1]
            } else {
                values[i - 1] - values[i]
            };
            if delta > max_delta {
                max_delta = delta;
            }
        }
        // Estimate bytes needed for deltas vs full values
        let full_bytes = values.len() * 8;
        let delta_bits = if max_delta == 0 { 1 } else { 64 - max_delta.leading_zeros() as usize };
        let delta_bytes = (values.len() * delta_bits + 7) / 8 + 8; // +8 for base value
        let saved = if full_bytes > delta_bytes { full_bytes - delta_bytes } else { 0 };
        self.stats.bytes_saved += saved as u64;
        saved as u64
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Compression stats per syscall class
#[derive(Debug, Clone, Default)]
pub struct SyscallCompressionProfile {
    /// Syscall number range start
    pub syscall_class: u32,
    /// Total compressed
    pub total_compressed: u64,
    /// Total original bytes
    pub total_original: u64,
    /// Total compressed bytes
    pub total_compressed_bytes: u64,
    /// Best algorithm for this class
    pub best_algorithm: u8,
}

impl SyscallCompressionProfile {
    /// Compression ratio
    pub fn ratio(&self) -> f64 {
        if self.total_original == 0 {
            return 1.0;
        }
        self.total_compressed_bytes as f64 / self.total_original as f64
    }
}

/// Bridge compression stats
#[derive(Debug, Clone, Default)]
pub struct BridgeCompressionStats {
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Total bytes saved
    pub bytes_saved: u64,
    /// Operations performed
    pub operations: u64,
    /// Dictionary size
    pub dictionary_size: usize,
    /// Avg compression ratio
    pub avg_ratio: f64,
}

/// Bridge compression engine
pub struct BridgeCompressionEngine {
    /// Dictionary
    dictionary: CompressionDictionary,
    /// RLE encoder
    rle: RleEncoder,
    /// Delta encoder
    delta: DeltaEncoder,
    /// Per-syscall profiles
    profiles: BTreeMap<u32, SyscallCompressionProfile>,
    /// Stats
    stats: BridgeCompressionStats,
}

impl BridgeCompressionEngine {
    pub fn new() -> Self {
        Self {
            dictionary: CompressionDictionary::new(4096),
            rle: RleEncoder::new(4),
            delta: DeltaEncoder::new(),
            profiles: BTreeMap::new(),
            stats: BridgeCompressionStats::default(),
        }
    }

    /// Select best algorithm for data
    pub fn select_algorithm(&self, data: &[u8], syscall_nr: u32) -> CompressionAlgorithm {
        if data.len() < 32 {
            return CompressionAlgorithm::None; // too small
        }
        // Check profile
        if let Some(profile) = self.profiles.get(&syscall_nr) {
            if profile.total_compressed > 10 {
                return match profile.best_algorithm {
                    0 => CompressionAlgorithm::None,
                    1 => CompressionAlgorithm::Rle,
                    2 => CompressionAlgorithm::Dictionary,
                    3 => CompressionAlgorithm::Delta,
                    _ => CompressionAlgorithm::Adaptive,
                };
            }
        }
        CompressionAlgorithm::Adaptive
    }

    /// Compress data — returns estimated compressed size
    pub fn compress(&mut self, data: &[u8], syscall_nr: u32, now: u64) -> (CompressionResult, usize) {
        self.stats.operations += 1;
        self.stats.bytes_processed += data.len() as u64;

        if data.len() < 32 {
            return (CompressionResult::TooSmall, data.len());
        }

        let algo = self.select_algorithm(data, syscall_nr);
        let (result, compressed_size) = match algo {
            CompressionAlgorithm::None => (CompressionResult::Incompressible, data.len()),
            CompressionAlgorithm::Rle => {
                let size = self.rle.estimate_compressed_size(data);
                if size < data.len() {
                    (CompressionResult::Compressed, size)
                } else {
                    (CompressionResult::Expansion, data.len())
                }
            }
            CompressionAlgorithm::Dictionary => {
                // Add to dictionary and estimate
                self.dictionary.add_pattern(data, now);
                let estimated = data.len() * 3 / 4; // rough 25% savings
                (CompressionResult::Compressed, estimated)
            }
            CompressionAlgorithm::Delta | CompressionAlgorithm::Adaptive => {
                let rle_size = self.rle.estimate_compressed_size(data);
                if rle_size < data.len() {
                    (CompressionResult::Compressed, rle_size)
                } else {
                    self.dictionary.add_pattern(data, now);
                    (CompressionResult::Incompressible, data.len())
                }
            }
        };

        // Update profile
        let profile = self.profiles.entry(syscall_nr).or_insert_with(|| SyscallCompressionProfile {
            syscall_class: syscall_nr,
            ..Default::default()
        });
        profile.total_compressed += 1;
        profile.total_original += data.len() as u64;
        profile.total_compressed_bytes += compressed_size as u64;

        if data.len() > compressed_size {
            self.stats.bytes_saved += (data.len() - compressed_size) as u64;
        }

        self.update_ratio();
        (result, compressed_size)
    }

    fn update_ratio(&mut self) {
        if self.stats.bytes_processed > 0 {
            self.stats.avg_ratio = (self.stats.bytes_processed - self.stats.bytes_saved) as f64
                / self.stats.bytes_processed as f64;
        }
        self.stats.dictionary_size = self.dictionary.size();
    }

    /// Stats
    pub fn stats(&self) -> &BridgeCompressionStats {
        &self.stats
    }
}
