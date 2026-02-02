//! HelixFS Compression Layer
//!
//! Provides transparent data compression with multiple algorithms,
//! adaptive selection, and streaming support.
//!
//! # Algorithms
//! - LZ4: Ultra-fast compression for hot data
//! - ZSTD: Balanced compression for general use
//! - LZO: Legacy fast compression
//!
//! # Features
//! - Per-extent compression
//! - Adaptive algorithm selection
//! - Transparent compression/decompression
//! - Inline small data compression

#![allow(dead_code)]

pub mod adaptive;
pub mod lz4;
pub mod zstd;

use crate::core::error::HfsResult;

// ============================================================================
// Constants
// ============================================================================

/// Maximum uncompressed block size
pub const MAX_BLOCK_SIZE: usize = 128 * 1024; // 128 KB

/// Minimum data size for compression
pub const MIN_COMPRESS_SIZE: usize = 64;

/// Compression ratio threshold (don't store compressed if >= this ratio)
pub const COMPRESSION_THRESHOLD: u32 = 90; // 90%

/// Maximum compressed size (must be smaller than input or use uncompressed)
pub const MAX_COMPRESS_RATIO: u32 = 100; // 100%

/// LZ4 magic number
pub const LZ4_MAGIC: u32 = 0x184D2204;

/// ZSTD magic number
pub const ZSTD_MAGIC: u32 = 0xFD2FB528;

// ============================================================================
// Compression Algorithm
// ============================================================================

/// Compression algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionType {
    /// No compression
    None  = 0,
    /// LZ4 fast compression
    Lz4   = 1,
    /// LZ4 high compression
    Lz4Hc = 2,
    /// ZSTD compression
    Zstd  = 3,
    /// LZO compression
    Lzo   = 4,
    /// ZLIB compression
    Zlib  = 5,
}

impl CompressionType {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::None,
            1 => Self::Lz4,
            2 => Self::Lz4Hc,
            3 => Self::Zstd,
            4 => Self::Lzo,
            5 => Self::Zlib,
            _ => Self::None,
        }
    }

    /// Is any compression enabled
    #[inline]
    pub fn is_compressed(&self) -> bool {
        *self != Self::None
    }

    /// Is fast compression
    #[inline]
    pub fn is_fast(&self) -> bool {
        matches!(*self, Self::Lz4 | Self::Lzo)
    }

    /// Default compression level
    pub fn default_level(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Lz4 => 1,
            Self::Lz4Hc => 9,
            Self::Zstd => 3,
            Self::Lzo => 1,
            Self::Zlib => 6,
        }
    }

    /// Maximum compression level
    pub fn max_level(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Lz4 => 1,
            Self::Lz4Hc => 12,
            Self::Zstd => 22,
            Self::Lzo => 1,
            Self::Zlib => 9,
        }
    }
}

impl Default for CompressionType {
    fn default() -> Self {
        Self::None
    }
}

// ============================================================================
// Compression Configuration
// ============================================================================

/// Compression configuration.
#[derive(Clone, Copy, Debug)]
pub struct CompressionConfig {
    /// Default algorithm
    pub algorithm: CompressionType,
    /// Compression level
    pub level: i32,
    /// Minimum size to compress
    pub min_size: usize,
    /// Maximum uncompressed block size
    pub block_size: usize,
    /// Force compression even if larger
    pub force: bool,
    /// Enable adaptive selection
    pub adaptive: bool,
}

impl CompressionConfig {
    /// Create default config
    pub const fn new() -> Self {
        Self {
            algorithm: CompressionType::None,
            level: 3,
            min_size: MIN_COMPRESS_SIZE,
            block_size: MAX_BLOCK_SIZE,
            force: false,
            adaptive: false,
        }
    }

    /// Fast compression config
    pub const fn fast() -> Self {
        Self {
            algorithm: CompressionType::Lz4,
            level: 1,
            min_size: MIN_COMPRESS_SIZE,
            block_size: MAX_BLOCK_SIZE,
            force: false,
            adaptive: false,
        }
    }

    /// Balanced compression config
    pub const fn balanced() -> Self {
        Self {
            algorithm: CompressionType::Zstd,
            level: 3,
            min_size: MIN_COMPRESS_SIZE,
            block_size: MAX_BLOCK_SIZE,
            force: false,
            adaptive: true,
        }
    }

    /// High compression config
    pub const fn high() -> Self {
        Self {
            algorithm: CompressionType::Zstd,
            level: 9,
            min_size: MIN_COMPRESS_SIZE,
            block_size: MAX_BLOCK_SIZE,
            force: false,
            adaptive: true,
        }
    }

    /// Set algorithm
    pub fn algorithm(mut self, algo: CompressionType) -> Self {
        self.algorithm = algo;
        self
    }

    /// Set level
    pub fn level(mut self, level: i32) -> Self {
        self.level = level;
        self
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Compression Header
// ============================================================================

/// Compressed block header.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct CompressedHeader {
    /// Magic/algorithm identifier
    pub magic: u32,
    /// Compressed size
    pub compressed_size: u32,
    /// Uncompressed size
    pub uncompressed_size: u32,
    /// Checksum of compressed data
    pub checksum: u32,
}

impl CompressedHeader {
    /// Header size in bytes
    pub const SIZE: usize = 16;

    /// Create new header
    pub fn new(algo: CompressionType, compressed: u32, uncompressed: u32) -> Self {
        let magic = match algo {
            CompressionType::Lz4 | CompressionType::Lz4Hc => LZ4_MAGIC,
            CompressionType::Zstd => ZSTD_MAGIC,
            _ => 0,
        };

        Self {
            magic,
            compressed_size: compressed,
            uncompressed_size: uncompressed,
            checksum: 0,
        }
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            magic: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            compressed_size: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            uncompressed_size: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            checksum: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
        })
    }

    /// Write to bytes
    pub fn to_bytes(&self, buf: &mut [u8]) {
        buf[0..4].copy_from_slice(&self.magic.to_le_bytes());
        buf[4..8].copy_from_slice(&self.compressed_size.to_le_bytes());
        buf[8..12].copy_from_slice(&self.uncompressed_size.to_le_bytes());
        buf[12..16].copy_from_slice(&self.checksum.to_le_bytes());
    }

    /// Detect algorithm from magic
    pub fn algorithm(&self) -> CompressionType {
        match self.magic {
            LZ4_MAGIC => CompressionType::Lz4,
            ZSTD_MAGIC => CompressionType::Zstd,
            _ => CompressionType::None,
        }
    }

    /// Compression ratio (0-100)
    pub fn ratio(&self) -> u32 {
        if self.uncompressed_size == 0 {
            return 100;
        }
        (self.compressed_size as u64 * 100 / self.uncompressed_size as u64) as u32
    }
}

// ============================================================================
// Compression Result
// ============================================================================

/// Compression operation result.
#[derive(Clone, Copy, Debug)]
pub struct CompressionResult {
    /// Algorithm used
    pub algorithm: CompressionType,
    /// Input size
    pub input_size: usize,
    /// Output size
    pub output_size: usize,
    /// Data was actually compressed (vs stored)
    pub compressed: bool,
}

impl CompressionResult {
    /// Create new result
    pub fn new(algo: CompressionType, input: usize, output: usize) -> Self {
        Self {
            algorithm: algo,
            input_size: input,
            output_size: output,
            compressed: output < input,
        }
    }

    /// Compression ratio (0-100)
    pub fn ratio(&self) -> u32 {
        if self.input_size == 0 {
            return 100;
        }
        (self.output_size as u64 * 100 / self.input_size as u64) as u32
    }

    /// Space saved
    pub fn saved(&self) -> usize {
        if self.output_size >= self.input_size {
            0
        } else {
            self.input_size - self.output_size
        }
    }
}

// ============================================================================
// Compressor Trait
// ============================================================================

/// Compressor trait for different algorithms.
pub trait Compressor {
    /// Compress data
    fn compress(&self, input: &[u8], output: &mut [u8]) -> HfsResult<usize>;

    /// Decompress data
    fn decompress(&self, input: &[u8], output: &mut [u8]) -> HfsResult<usize>;

    /// Get maximum compressed size for input
    fn max_compressed_size(&self, input_size: usize) -> usize;

    /// Get algorithm type
    fn algorithm(&self) -> CompressionType;
}

// ============================================================================
// Compression Statistics
// ============================================================================

/// Compression statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct CompressionStats {
    /// Total bytes input
    pub bytes_in: u64,
    /// Total bytes output
    pub bytes_out: u64,
    /// Compression operations
    pub compress_ops: u64,
    /// Decompression operations
    pub decompress_ops: u64,
    /// Compression failures
    pub compress_failures: u64,
    /// Decompression failures
    pub decompress_failures: u64,
    /// Blocks stored uncompressed
    pub uncompressed_blocks: u64,
    /// Time spent compressing (us)
    pub compress_time_us: u64,
    /// Time spent decompressing (us)
    pub decompress_time_us: u64,
}

impl CompressionStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            bytes_in: 0,
            bytes_out: 0,
            compress_ops: 0,
            decompress_ops: 0,
            compress_failures: 0,
            decompress_failures: 0,
            uncompressed_blocks: 0,
            compress_time_us: 0,
            decompress_time_us: 0,
        }
    }

    /// Overall compression ratio
    pub fn ratio(&self) -> f32 {
        if self.bytes_in == 0 {
            return 100.0;
        }
        (self.bytes_out as f32 / self.bytes_in as f32) * 100.0
    }

    /// Space saved
    pub fn saved(&self) -> u64 {
        if self.bytes_out >= self.bytes_in {
            0
        } else {
            self.bytes_in - self.bytes_out
        }
    }

    /// Record compression
    pub fn record_compress(&mut self, input: usize, output: usize, time_us: u64) {
        self.bytes_in += input as u64;
        self.bytes_out += output as u64;
        self.compress_ops += 1;
        self.compress_time_us += time_us;

        if output >= input {
            self.uncompressed_blocks += 1;
        }
    }

    /// Record decompression
    pub fn record_decompress(&mut self, output: usize, time_us: u64) {
        self.decompress_ops += 1;
        self.decompress_time_us += time_us;
        let _ = output; // Used for stats extension
    }
}

// ============================================================================
// Extent Compression Info
// ============================================================================

/// Per-extent compression information.
#[derive(Clone, Copy, Debug, Default)]
pub struct ExtentCompressionInfo {
    /// Compression algorithm
    pub algorithm: CompressionType,
    /// Compressed size on disk
    pub compressed_size: u32,
    /// Uncompressed size
    pub uncompressed_size: u32,
    /// Number of compressed blocks
    pub num_blocks: u16,
    /// Flags
    pub flags: u16,
}

impl ExtentCompressionInfo {
    /// Compressed inline flag
    pub const FLAG_INLINE: u16 = 1 << 0;
    /// All blocks same algo flag
    pub const FLAG_UNIFORM: u16 = 1 << 1;

    /// Create new info
    pub fn new(algo: CompressionType, compressed: u32, uncompressed: u32) -> Self {
        Self {
            algorithm: algo,
            compressed_size: compressed,
            uncompressed_size: uncompressed,
            num_blocks: 1,
            flags: 0,
        }
    }

    /// Is compressed
    #[inline]
    pub fn is_compressed(&self) -> bool {
        self.algorithm.is_compressed()
    }

    /// Compression ratio
    pub fn ratio(&self) -> u32 {
        if self.uncompressed_size == 0 {
            return 100;
        }
        (self.compressed_size as u64 * 100 / self.uncompressed_size as u64) as u32
    }
}

// ============================================================================
// Block Compression Table
// ============================================================================

/// Entry in block compression table.
#[derive(Clone, Copy, Debug, Default)]
pub struct BlockCompressEntry {
    /// Offset in compressed stream
    pub offset: u32,
    /// Compressed size
    pub size: u16,
    /// Algorithm for this block
    pub algorithm: u8,
    /// Reserved
    pub reserved: u8,
}

impl BlockCompressEntry {
    /// Entry size
    pub const SIZE: usize = 8;

    /// Create new entry
    pub fn new(offset: u32, size: u16, algo: CompressionType) -> Self {
        Self {
            offset,
            size,
            algorithm: algo as u8,
            reserved: 0,
        }
    }
}

/// Block compression table for random access.
pub struct BlockCompressTable {
    /// Entries
    entries: [BlockCompressEntry; 256],
    /// Number of entries
    count: usize,
    /// Uncompressed block size
    block_size: u32,
}

impl BlockCompressTable {
    /// Create new table
    pub fn new(block_size: u32) -> Self {
        Self {
            entries: [BlockCompressEntry::default(); 256],
            count: 0,
            block_size,
        }
    }

    /// Add entry
    pub fn add(&mut self, offset: u32, size: u16, algo: CompressionType) -> bool {
        if self.count >= 256 {
            return false;
        }

        self.entries[self.count] = BlockCompressEntry::new(offset, size, algo);
        self.count += 1;
        true
    }

    /// Get entry for block index
    pub fn get(&self, block_idx: usize) -> Option<&BlockCompressEntry> {
        if block_idx < self.count {
            Some(&self.entries[block_idx])
        } else {
            None
        }
    }

    /// Find block for offset
    pub fn find_block(&self, offset: u64) -> Option<(usize, &BlockCompressEntry)> {
        let block_idx = (offset / self.block_size as u64) as usize;
        self.get(block_idx).map(|e| (block_idx, e))
    }

    /// Total compressed size
    pub fn compressed_size(&self) -> u64 {
        self.entries[..self.count]
            .iter()
            .map(|e| e.size as u64)
            .sum()
    }

    /// Count
    pub fn count(&self) -> usize {
        self.count
    }
}

impl Default for BlockCompressTable {
    fn default() -> Self {
        Self::new(MAX_BLOCK_SIZE as u32)
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Check if data is compressible (quick heuristic).
pub fn is_compressible(data: &[u8]) -> bool {
    if data.len() < MIN_COMPRESS_SIZE {
        return false;
    }

    // Quick entropy check: count unique bytes in sample
    let sample_size = core::cmp::min(256, data.len());
    let mut seen = [false; 256];
    let mut unique = 0;

    for &b in &data[..sample_size] {
        if !seen[b as usize] {
            seen[b as usize] = true;
            unique += 1;
        }
    }

    // If very high entropy, likely not compressible
    unique < sample_size * 7 / 8
}

/// Simple run-length encoding for zeros.
pub fn compress_zeros(data: &[u8], output: &mut [u8]) -> Option<usize> {
    if data.is_empty() {
        return Some(0);
    }

    // Check if all zeros
    let all_zeros = data.iter().all(|&b| b == 0);

    if all_zeros && output.len() >= 8 {
        // Store as: magic (4) + length (4)
        output[0..4].copy_from_slice(&0x5a45524f_u32.to_le_bytes()); // "ZERO"
        output[4..8].copy_from_slice(&(data.len() as u32).to_le_bytes());
        return Some(8);
    }

    None
}

/// Decompress zero run.
pub fn decompress_zeros(input: &[u8], output: &mut [u8]) -> Option<usize> {
    if input.len() < 8 {
        return None;
    }

    let magic = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
    if magic != 0x5a45524f {
        return None;
    }

    let len = u32::from_le_bytes([input[4], input[5], input[6], input[7]]) as usize;

    if output.len() < len {
        return None;
    }

    output[..len].fill(0);
    Some(len)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_type() {
        assert!(!CompressionType::None.is_compressed());
        assert!(CompressionType::Lz4.is_compressed());
        assert!(CompressionType::Lz4.is_fast());
        assert!(!CompressionType::Zstd.is_fast());
    }

    #[test]
    fn test_compression_config() {
        let config = CompressionConfig::balanced();

        assert_eq!(config.algorithm, CompressionType::Zstd);
        assert_eq!(config.level, 3);
        assert!(config.adaptive);
    }

    #[test]
    fn test_compressed_header() {
        let header = CompressedHeader::new(CompressionType::Lz4, 100, 200);

        assert_eq!(header.magic, LZ4_MAGIC);
        assert_eq!(header.ratio(), 50);
        assert_eq!(header.algorithm(), CompressionType::Lz4);

        let mut buf = [0u8; 16];
        header.to_bytes(&mut buf);

        let parsed = CompressedHeader::from_bytes(&buf).unwrap();
        assert_eq!(parsed.compressed_size, 100);
        assert_eq!(parsed.uncompressed_size, 200);
    }

    #[test]
    fn test_compression_result() {
        let result = CompressionResult::new(CompressionType::Zstd, 1000, 500);

        assert!(result.compressed);
        assert_eq!(result.ratio(), 50);
        assert_eq!(result.saved(), 500);
    }

    #[test]
    fn test_is_compressible() {
        let zeros = [0u8; 256];
        assert!(is_compressible(&zeros));

        let random: [u8; 256] = core::array::from_fn(|i| i as u8);
        assert!(!is_compressible(&random));

        let small = [0u8; 32];
        assert!(!is_compressible(&small)); // Too small
    }

    #[test]
    fn test_compress_zeros() {
        let zeros = [0u8; 1024];
        let mut output = [0u8; 64];

        let size = compress_zeros(&zeros, &mut output).unwrap();
        assert_eq!(size, 8);

        let mut decompressed = [1u8; 1024];
        let dec_size = decompress_zeros(&output, &mut decompressed).unwrap();
        assert_eq!(dec_size, 1024);
        assert!(decompressed.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_block_compress_table() {
        let mut table = BlockCompressTable::new(4096);

        table.add(0, 2000, CompressionType::Lz4);
        table.add(2000, 1500, CompressionType::Lz4);

        assert_eq!(table.count(), 2);
        assert_eq!(table.compressed_size(), 3500);

        let (idx, entry) = table.find_block(5000).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(entry.size, 1500);
    }
}
