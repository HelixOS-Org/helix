//! # Bridge Compression
//!
//! Syscall data compression and optimization:
//! - Argument compression
//! - Response compression
//! - Repeated pattern encoding
//! - Zero-page dedup
//! - Compression statistics
//! - Adaptive compression levels

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// COMPRESSION METHOD
// ============================================================================

/// Compression algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompressionMethod {
    /// No compression
    None,
    /// Run-length encoding
    Rle,
    /// Delta encoding
    Delta,
    /// Dictionary-based
    Dictionary,
    /// Zero-page dedup
    ZeroDedup,
    /// Repeated pattern
    PatternMatch,
}

/// Compression level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompressionLevel {
    /// Fast (minimal CPU)
    Fast,
    /// Default (balanced)
    Default,
    /// Best (maximum compression)
    Best,
    /// Adaptive (based on content)
    Adaptive,
}

// ============================================================================
// COMPRESSION RESULT
// ============================================================================

/// Compressed data block
#[derive(Debug, Clone)]
pub struct CompressedBlock {
    /// Method used
    pub method: CompressionMethod,
    /// Original size
    pub original_size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Compressed data
    pub data: Vec<u8>,
    /// Checksum
    pub checksum: u32,
}

impl CompressedBlock {
    pub fn new(method: CompressionMethod, original_size: usize) -> Self {
        Self {
            method,
            original_size,
            compressed_size: 0,
            data: Vec::new(),
            checksum: 0,
        }
    }

    /// Compression ratio
    #[inline]
    pub fn ratio(&self) -> f64 {
        if self.original_size == 0 {
            return 1.0;
        }
        self.compressed_size as f64 / self.original_size as f64
    }

    /// Space saved (bytes)
    #[inline(always)]
    pub fn saved(&self) -> usize {
        self.original_size.saturating_sub(self.compressed_size)
    }
}

// ============================================================================
// RLE COMPRESSOR
// ============================================================================

/// Simple RLE compressor for syscall buffers
pub struct RleCompressor;

impl RleCompressor {
    /// Compress data using RLE
    pub fn compress(data: &[u8]) -> CompressedBlock {
        let mut block = CompressedBlock::new(CompressionMethod::Rle, data.len());
        let mut output = Vec::new();

        let mut i = 0;
        while i < data.len() {
            let byte = data[i];
            let mut count: u8 = 1;
            while i + (count as usize) < data.len()
                && data[i + count as usize] == byte
                && count < 255
            {
                count += 1;
            }

            if count >= 3 {
                // Encode run: marker(0xFF) + count + byte
                output.push(0xFF);
                output.push(count);
                output.push(byte);
            } else {
                for _ in 0..count {
                    if byte == 0xFF {
                        output.push(0xFF);
                        output.push(1);
                        output.push(0xFF);
                    } else {
                        output.push(byte);
                    }
                }
            }
            i += count as usize;
        }

        block.compressed_size = output.len();
        block.checksum = Self::checksum(&output);
        block.data = output;
        block
    }

    /// Decompress RLE
    pub fn decompress(block: &CompressedBlock) -> Vec<u8> {
        let mut output = Vec::new();
        let data = &block.data;
        let mut i = 0;

        while i < data.len() {
            if data[i] == 0xFF && i + 2 < data.len() {
                let count = data[i + 1] as usize;
                let byte = data[i + 2];
                for _ in 0..count {
                    output.push(byte);
                }
                i += 3;
            } else {
                output.push(data[i]);
                i += 1;
            }
        }
        output
    }

    fn checksum(data: &[u8]) -> u32 {
        let mut sum: u32 = 0;
        for &b in data {
            sum = sum.wrapping_add(b as u32);
            sum = sum.wrapping_mul(31);
        }
        sum
    }
}

// ============================================================================
// DELTA COMPRESSOR
// ============================================================================

/// Delta encoder for numeric sequences
pub struct DeltaCompressor;

impl DeltaCompressor {
    /// Compress a sequence of u64 values using delta encoding
    pub fn compress_u64(values: &[u64]) -> CompressedBlock {
        let original_size = values.len() * 8;
        let mut block = CompressedBlock::new(CompressionMethod::Delta, original_size);
        let mut output = Vec::new();

        if values.is_empty() {
            block.compressed_size = 0;
            return block;
        }

        // Store first value as-is (8 bytes)
        output.extend_from_slice(&values[0].to_le_bytes());

        // Store deltas
        for i in 1..values.len() {
            let delta = values[i].wrapping_sub(values[i - 1]);
            // Variable-length encode delta
            Self::encode_varint(delta, &mut output);
        }

        block.compressed_size = output.len();
        block.data = output;
        block
    }

    /// Decompress delta-encoded u64 values
    pub fn decompress_u64(block: &CompressedBlock, count: usize) -> Vec<u64> {
        let mut values = Vec::with_capacity(count);
        let data = &block.data;

        if data.len() < 8 {
            return values;
        }

        // Read first value
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&data[0..8]);
        let first = u64::from_le_bytes(bytes);
        values.push(first);

        let mut pos = 8;
        let mut prev = first;

        while values.len() < count && pos < data.len() {
            let (delta, consumed) = Self::decode_varint(&data[pos..]);
            pos += consumed;
            prev = prev.wrapping_add(delta);
            values.push(prev);
        }

        values
    }

    fn encode_varint(mut value: u64, output: &mut Vec<u8>) {
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if value == 0 {
                output.push(byte);
                break;
            } else {
                output.push(byte | 0x80);
            }
        }
    }

    fn decode_varint(data: &[u8]) -> (u64, usize) {
        let mut value: u64 = 0;
        let mut shift = 0;
        let mut consumed = 0;

        for &byte in data {
            consumed += 1;
            value |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift >= 64 {
                break;
            }
        }

        (value, consumed)
    }
}

// ============================================================================
// ZERO PAGE DEDUP
// ============================================================================

/// Zero page deduplication
#[derive(Debug, Clone)]
pub struct ZeroPageDedup {
    /// Pages seen
    pub pages_seen: u64,
    /// Zero pages found
    pub zero_pages: u64,
    /// Partially zero pages
    pub partial_zero: u64,
    /// Page size
    pub page_size: usize,
}

impl ZeroPageDedup {
    pub fn new(page_size: usize) -> Self {
        Self {
            pages_seen: 0,
            zero_pages: 0,
            partial_zero: 0,
            page_size,
        }
    }

    /// Check if page is all zeros
    pub fn is_zero_page(&mut self, data: &[u8]) -> bool {
        self.pages_seen += 1;
        let is_zero = data.iter().all(|&b| b == 0);
        if is_zero {
            self.zero_pages += 1;
        } else {
            // Check if mostly zeros (>75%)
            let zeros = data.iter().filter(|&&b| b == 0).count();
            if zeros > data.len() * 3 / 4 {
                self.partial_zero += 1;
            }
        }
        is_zero
    }

    /// Zero page ratio
    #[inline]
    pub fn zero_ratio(&self) -> f64 {
        if self.pages_seen == 0 {
            return 0.0;
        }
        self.zero_pages as f64 / self.pages_seen as f64
    }

    /// Saved bytes
    #[inline(always)]
    pub fn saved_bytes(&self) -> u64 {
        self.zero_pages * self.page_size as u64
    }
}

// ============================================================================
// COMPRESSION MANAGER
// ============================================================================

/// Compression stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CompressionStats {
    /// Total bytes in
    pub total_input: u64,
    /// Total bytes out
    pub total_output: u64,
    /// Overall ratio
    pub overall_ratio: f64,
    /// Operations count
    pub operations: u64,
    /// Method distribution
    pub method_usage: BTreeMap<u8, u64>,
}

/// Bridge compression manager
#[repr(align(64))]
pub struct BridgeCompressionManager {
    /// Default level
    pub level: CompressionLevel,
    /// Per-syscall compression method
    syscall_methods: BTreeMap<u32, CompressionMethod>,
    /// Zero page dedup
    zero_dedup: ZeroPageDedup,
    /// Stats
    stats: CompressionStats,
}

impl BridgeCompressionManager {
    pub fn new() -> Self {
        Self {
            level: CompressionLevel::Default,
            syscall_methods: BTreeMap::new(),
            zero_dedup: ZeroPageDedup::new(4096),
            stats: CompressionStats::default(),
        }
    }

    /// Set method for syscall
    #[inline(always)]
    pub fn set_method(&mut self, syscall_nr: u32, method: CompressionMethod) {
        self.syscall_methods.insert(syscall_nr, method);
    }

    /// Compress buffer
    pub fn compress(&mut self, data: &[u8], method: CompressionMethod) -> CompressedBlock {
        self.stats.total_input += data.len() as u64;
        self.stats.operations += 1;
        *self.stats.method_usage.entry(method as u8).or_insert(0) += 1;

        let block = match method {
            CompressionMethod::Rle => RleCompressor::compress(data),
            CompressionMethod::None => {
                let mut b = CompressedBlock::new(CompressionMethod::None, data.len());
                b.compressed_size = data.len();
                b.data = data.to_vec();
                b
            },
            _ => {
                // Fallback to RLE for unsupported methods
                RleCompressor::compress(data)
            },
        };

        self.stats.total_output += block.compressed_size as u64;
        if self.stats.total_input > 0 {
            self.stats.overall_ratio =
                self.stats.total_output as f64 / self.stats.total_input as f64;
        }

        block
    }

    /// Get method for syscall
    #[inline]
    pub fn method_for_syscall(&self, syscall_nr: u32) -> CompressionMethod {
        self.syscall_methods
            .get(&syscall_nr)
            .copied()
            .unwrap_or(CompressionMethod::None)
    }

    /// Zero page dedup
    #[inline(always)]
    pub fn zero_dedup(&mut self) -> &mut ZeroPageDedup {
        &mut self.zero_dedup
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CompressionStats {
        &self.stats
    }
}
