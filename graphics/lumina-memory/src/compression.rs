//! Memory Compression
//!
//! GPU memory compression, deduplication, and intelligent streaming.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                  Memory Compression Pipeline                        │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  ┌───────────────┐    ┌───────────────┐    ┌───────────────────┐   │
//! │  │ Uncompressed  │───▶│  Compression  │───▶│   Compressed      │   │
//! │  │ Data          │    │  Engine       │    │   Storage         │   │
//! │  └───────────────┘    └───────────────┘    └───────────────────┘   │
//! │                                                                     │
//! │  ┌───────────────────────────────────────────────────────────────┐ │
//! │  │                   Delta Compression                           │ │
//! │  │  Frame N-1 ────┬──── XOR ────┬──── Frame N Delta             │ │
//! │  │  Frame N   ────┘             └──── (Much Smaller)            │ │
//! │  └───────────────────────────────────────────────────────────────┘ │
//! │                                                                     │
//! │  ┌───────────────────────────────────────────────────────────────┐ │
//! │  │                    Deduplication                              │ │
//! │  │  ┌──────┐ ┌──────┐ ┌──────┐    ┌──────┐                      │ │
//! │  │  │ A    │ │ B    │ │ A    │ ─▶ │ A    │ + refs               │ │
//! │  │  └──────┘ └──────┘ └──────┘    └──────┘                      │ │
//! │  └───────────────────────────────────────────────────────────────┘ │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::hash::{Hash, Hasher};
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// ============================================================================
// Compression Types
// ============================================================================

/// Compression algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionAlgorithm {
    /// No compression.
    None,
    /// LZ4 (fast).
    Lz4,
    /// LZ4 high compression.
    Lz4Hc,
    /// ZSTD (balanced).
    Zstd,
    /// ZSTD dictionary.
    ZstdDict,
    /// Delta compression.
    Delta,
    /// Delta + LZ4.
    DeltaLz4,
    /// GPU hardware compression.
    GpuHardware,
    /// BC/ASTC texture compression.
    TextureBlock,
}

impl Default for CompressionAlgorithm {
    fn default() -> Self {
        CompressionAlgorithm::None
    }
}

impl CompressionAlgorithm {
    /// Get typical compression ratio.
    pub fn typical_ratio(&self) -> f32 {
        match self {
            CompressionAlgorithm::None => 1.0,
            CompressionAlgorithm::Lz4 => 2.0,
            CompressionAlgorithm::Lz4Hc => 2.5,
            CompressionAlgorithm::Zstd => 3.0,
            CompressionAlgorithm::ZstdDict => 3.5,
            CompressionAlgorithm::Delta => 4.0,
            CompressionAlgorithm::DeltaLz4 => 6.0,
            CompressionAlgorithm::GpuHardware => 2.0,
            CompressionAlgorithm::TextureBlock => 4.0,
        }
    }

    /// Is lossless.
    pub fn is_lossless(&self) -> bool {
        match self {
            CompressionAlgorithm::TextureBlock => false,
            _ => true,
        }
    }
}

/// Compression level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompressionLevel {
    /// Fastest (lowest compression).
    Fastest,
    /// Fast.
    Fast,
    /// Default (balanced).
    Default,
    /// Better.
    Better,
    /// Best (slowest).
    Best,
}

impl Default for CompressionLevel {
    fn default() -> Self {
        CompressionLevel::Default
    }
}

/// Compression flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompressionFlags(u32);

impl CompressionFlags {
    /// None.
    pub const NONE: Self = Self(0);
    /// Use dictionary.
    pub const USE_DICTIONARY: Self = Self(1 << 0);
    /// Allow lossy for textures.
    pub const ALLOW_LOSSY: Self = Self(1 << 1);
    /// Streaming mode.
    pub const STREAMING: Self = Self(1 << 2);
    /// Enable deduplication.
    pub const DEDUPLICATE: Self = Self(1 << 3);
    /// Delta compress.
    pub const DELTA: Self = Self(1 << 4);

    /// Combine.
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Check.
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl Default for CompressionFlags {
    fn default() -> Self {
        Self::NONE
    }
}

// ============================================================================
// Compression Settings
// ============================================================================

/// Compression settings.
#[derive(Debug, Clone)]
pub struct CompressionSettings {
    /// Algorithm.
    pub algorithm: CompressionAlgorithm,
    /// Level.
    pub level: CompressionLevel,
    /// Flags.
    pub flags: CompressionFlags,
    /// Dictionary (optional).
    pub dictionary: Option<Vec<u8>>,
    /// Block size for streaming.
    pub block_size: u32,
}

impl Default for CompressionSettings {
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Lz4,
            level: CompressionLevel::Default,
            flags: CompressionFlags::NONE,
            dictionary: None,
            block_size: 65536, // 64KB
        }
    }
}

impl CompressionSettings {
    /// Create fast settings.
    pub fn fast() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Lz4,
            level: CompressionLevel::Fastest,
            ..Default::default()
        }
    }

    /// Create balanced settings.
    pub fn balanced() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Zstd,
            level: CompressionLevel::Default,
            ..Default::default()
        }
    }

    /// Create best settings.
    pub fn best() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Zstd,
            level: CompressionLevel::Best,
            flags: CompressionFlags::DEDUPLICATE,
            ..Default::default()
        }
    }

    /// Create delta settings.
    pub fn delta() -> Self {
        Self {
            algorithm: CompressionAlgorithm::DeltaLz4,
            level: CompressionLevel::Fast,
            flags: CompressionFlags::DELTA,
            ..Default::default()
        }
    }
}

// ============================================================================
// Compressed Block
// ============================================================================

/// Compressed block handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompressedBlockHandle(pub u64);

impl CompressedBlockHandle {
    /// Invalid.
    pub const INVALID: Self = Self(u64::MAX);

    /// Is valid.
    pub fn is_valid(&self) -> bool {
        self.0 != u64::MAX
    }
}

impl Default for CompressedBlockHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Compressed block info.
#[derive(Debug, Clone)]
pub struct CompressedBlockInfo {
    /// Handle.
    pub handle: CompressedBlockHandle,
    /// Original size.
    pub original_size: u64,
    /// Compressed size.
    pub compressed_size: u64,
    /// Algorithm used.
    pub algorithm: CompressionAlgorithm,
    /// Checksum.
    pub checksum: u32,
    /// Is resident in GPU memory.
    pub resident: bool,
    /// Reference count (for deduplication).
    pub ref_count: u32,
}

impl CompressedBlockInfo {
    /// Get compression ratio.
    pub fn compression_ratio(&self) -> f32 {
        if self.compressed_size == 0 {
            1.0
        } else {
            self.original_size as f32 / self.compressed_size as f32
        }
    }

    /// Get savings.
    pub fn savings(&self) -> u64 {
        self.original_size.saturating_sub(self.compressed_size)
    }
}

// ============================================================================
// Compression Engine
// ============================================================================

/// Compression result.
#[derive(Debug, Clone)]
pub struct CompressionResult {
    /// Compressed data.
    pub data: Vec<u8>,
    /// Original size.
    pub original_size: u64,
    /// Compressed size.
    pub compressed_size: u64,
    /// Algorithm.
    pub algorithm: CompressionAlgorithm,
    /// Checksum.
    pub checksum: u32,
}

impl CompressionResult {
    /// Get ratio.
    pub fn ratio(&self) -> f32 {
        if self.compressed_size == 0 {
            1.0
        } else {
            self.original_size as f32 / self.compressed_size as f32
        }
    }
}

/// Compression engine.
pub struct CompressionEngine {
    /// Default settings.
    default_settings: CompressionSettings,
    /// Compression dictionary.
    dictionary: Vec<u8>,
    /// Statistics.
    total_input: AtomicU64,
    total_output: AtomicU64,
    compressions: AtomicU64,
    decompressions: AtomicU64,
}

impl CompressionEngine {
    /// Create new engine.
    pub fn new(settings: CompressionSettings) -> Self {
        Self {
            default_settings: settings,
            dictionary: Vec::new(),
            total_input: AtomicU64::new(0),
            total_output: AtomicU64::new(0),
            compressions: AtomicU64::new(0),
            decompressions: AtomicU64::new(0),
        }
    }

    /// Set dictionary.
    pub fn set_dictionary(&mut self, dict: Vec<u8>) {
        self.dictionary = dict;
    }

    /// Compress data (placeholder - real impl would use actual algorithms).
    pub fn compress(&self, data: &[u8], settings: Option<&CompressionSettings>) -> CompressionResult {
        let settings = settings.unwrap_or(&self.default_settings);
        let original_size = data.len() as u64;

        self.total_input.fetch_add(original_size, Ordering::Relaxed);
        self.compressions.fetch_add(1, Ordering::Relaxed);

        // Simulated compression (in real impl, use actual algorithm)
        let compressed = match settings.algorithm {
            CompressionAlgorithm::None => data.to_vec(),
            _ => {
                // Placeholder: just copy data
                // Real implementation would use LZ4/ZSTD/etc
                data.to_vec()
            }
        };

        let compressed_size = compressed.len() as u64;
        self.total_output.fetch_add(compressed_size, Ordering::Relaxed);

        let checksum = self.calculate_checksum(data);

        CompressionResult {
            data: compressed,
            original_size,
            compressed_size,
            algorithm: settings.algorithm,
            checksum,
        }
    }

    /// Decompress data (placeholder).
    pub fn decompress(&self, data: &[u8], original_size: u64, algorithm: CompressionAlgorithm) -> Vec<u8> {
        self.decompressions.fetch_add(1, Ordering::Relaxed);

        match algorithm {
            CompressionAlgorithm::None => data.to_vec(),
            _ => {
                // Placeholder: just copy
                // Real implementation would decompress
                let mut result = data.to_vec();
                result.resize(original_size as usize, 0);
                result
            }
        }
    }

    /// Calculate checksum.
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        // Simple FNV-1a hash
        let mut hash: u32 = 2166136261;
        for byte in data {
            hash ^= *byte as u32;
            hash = hash.wrapping_mul(16777619);
        }
        hash
    }

    /// Get overall compression ratio.
    pub fn overall_ratio(&self) -> f32 {
        let input = self.total_input.load(Ordering::Relaxed);
        let output = self.total_output.load(Ordering::Relaxed);
        if output == 0 {
            1.0
        } else {
            input as f32 / output as f32
        }
    }

    /// Get total savings.
    pub fn total_savings(&self) -> u64 {
        let input = self.total_input.load(Ordering::Relaxed);
        let output = self.total_output.load(Ordering::Relaxed);
        input.saturating_sub(output)
    }

    /// Get compression count.
    pub fn compression_count(&self) -> u64 {
        self.compressions.load(Ordering::Relaxed)
    }

    /// Get decompression count.
    pub fn decompression_count(&self) -> u64 {
        self.decompressions.load(Ordering::Relaxed)
    }
}

impl Default for CompressionEngine {
    fn default() -> Self {
        Self::new(CompressionSettings::default())
    }
}

// ============================================================================
// Deduplication
// ============================================================================

/// Content hash for deduplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash {
    /// Hash value.
    pub hash: [u64; 2],
}

impl ContentHash {
    /// Create from data.
    pub fn from_data(data: &[u8]) -> Self {
        // Simple hash (real impl would use something like xxHash or Blake3)
        let mut h1: u64 = 14695981039346656037;
        let mut h2: u64 = 1099511628211;

        for (i, chunk) in data.chunks(8).enumerate() {
            let mut val: u64 = 0;
            for (j, &byte) in chunk.iter().enumerate() {
                val |= (byte as u64) << (j * 8);
            }

            if i % 2 == 0 {
                h1 ^= val;
                h1 = h1.wrapping_mul(1099511628211);
            } else {
                h2 ^= val;
                h2 = h2.wrapping_mul(14695981039346656037);
            }
        }

        Self { hash: [h1, h2] }
    }
}

/// Deduplicated entry.
#[derive(Debug, Clone)]
pub struct DeduplicatedEntry {
    /// Content hash.
    pub hash: ContentHash,
    /// Compressed block.
    pub block: CompressedBlockHandle,
    /// Reference count.
    pub ref_count: AtomicU32,
    /// Size.
    pub size: u64,
}

/// Deduplication manager.
pub struct DeduplicationManager {
    /// Hash to entry map.
    entries: BTreeMap<[u64; 2], DeduplicatedEntry>,
    /// Next block handle.
    next_handle: AtomicU64,
    /// Statistics.
    deduplicated_count: AtomicU64,
    deduplicated_bytes: AtomicU64,
}

impl DeduplicationManager {
    /// Create new manager.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            next_handle: AtomicU64::new(1),
            deduplicated_count: AtomicU64::new(0),
            deduplicated_bytes: AtomicU64::new(0),
        }
    }

    /// Try to deduplicate data.
    pub fn deduplicate(&mut self, data: &[u8]) -> Option<CompressedBlockHandle> {
        let hash = ContentHash::from_data(data);

        if let Some(entry) = self.entries.get(&hash.hash) {
            entry.ref_count.fetch_add(1, Ordering::Relaxed);
            self.deduplicated_count.fetch_add(1, Ordering::Relaxed);
            self.deduplicated_bytes.fetch_add(data.len() as u64, Ordering::Relaxed);
            return Some(entry.block);
        }

        None
    }

    /// Store new entry.
    pub fn store(&mut self, data: &[u8], block: CompressedBlockHandle) -> ContentHash {
        let hash = ContentHash::from_data(data);

        let entry = DeduplicatedEntry {
            hash,
            block,
            ref_count: AtomicU32::new(1),
            size: data.len() as u64,
        };

        self.entries.insert(hash.hash, entry);
        hash
    }

    /// Release reference.
    pub fn release(&mut self, hash: ContentHash) -> bool {
        if let Some(entry) = self.entries.get(&hash.hash) {
            let prev = entry.ref_count.fetch_sub(1, Ordering::Relaxed);
            if prev == 1 {
                self.entries.remove(&hash.hash);
                return true; // Block can be freed
            }
        }
        false
    }

    /// Get deduplicated count.
    pub fn deduplicated_count(&self) -> u64 {
        self.deduplicated_count.load(Ordering::Relaxed)
    }

    /// Get deduplicated bytes.
    pub fn deduplicated_bytes(&self) -> u64 {
        self.deduplicated_bytes.load(Ordering::Relaxed)
    }

    /// Get unique block count.
    pub fn unique_block_count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for DeduplicationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Delta Compression
// ============================================================================

/// Delta compressor for frame data.
pub struct DeltaCompressor {
    /// Previous frame data.
    previous_frame: Vec<u8>,
    /// Frame size.
    frame_size: usize,
}

impl DeltaCompressor {
    /// Create new delta compressor.
    pub fn new(frame_size: usize) -> Self {
        Self {
            previous_frame: vec![0; frame_size],
            frame_size,
        }
    }

    /// Compute delta from previous frame.
    pub fn compute_delta(&mut self, current: &[u8]) -> Vec<u8> {
        assert_eq!(current.len(), self.frame_size);

        let mut delta = Vec::with_capacity(self.frame_size);

        for (curr, prev) in current.iter().zip(self.previous_frame.iter()) {
            delta.push(curr ^ prev);
        }

        // Update previous frame
        self.previous_frame.copy_from_slice(current);

        delta
    }

    /// Apply delta to reconstruct frame.
    pub fn apply_delta(&self, delta: &[u8]) -> Vec<u8> {
        assert_eq!(delta.len(), self.frame_size);

        let mut result = Vec::with_capacity(self.frame_size);

        for (d, prev) in delta.iter().zip(self.previous_frame.iter()) {
            result.push(d ^ prev);
        }

        result
    }

    /// Reset with new frame.
    pub fn reset(&mut self, frame: &[u8]) {
        assert_eq!(frame.len(), self.frame_size);
        self.previous_frame.copy_from_slice(frame);
    }

    /// Resize.
    pub fn resize(&mut self, new_size: usize) {
        self.frame_size = new_size;
        self.previous_frame.resize(new_size, 0);
    }
}

// ============================================================================
// Streaming Compression
// ============================================================================

/// Streaming compression state.
pub struct StreamingCompressor {
    /// Settings.
    settings: CompressionSettings,
    /// Buffer.
    buffer: Vec<u8>,
    /// Block index.
    block_index: u32,
    /// Compression engine.
    engine: CompressionEngine,
}

impl StreamingCompressor {
    /// Create new streaming compressor.
    pub fn new(settings: CompressionSettings) -> Self {
        let block_size = settings.block_size as usize;
        Self {
            settings: settings.clone(),
            buffer: Vec::with_capacity(block_size),
            block_index: 0,
            engine: CompressionEngine::new(settings),
        }
    }

    /// Write data and get compressed blocks.
    pub fn write(&mut self, data: &[u8]) -> Vec<CompressionResult> {
        let mut results = Vec::new();
        let block_size = self.settings.block_size as usize;

        for byte in data {
            self.buffer.push(*byte);

            if self.buffer.len() >= block_size {
                let result = self.engine.compress(&self.buffer, Some(&self.settings));
                results.push(result);
                self.buffer.clear();
                self.block_index += 1;
            }
        }

        results
    }

    /// Flush remaining data.
    pub fn flush(&mut self) -> Option<CompressionResult> {
        if self.buffer.is_empty() {
            return None;
        }

        let result = self.engine.compress(&self.buffer, Some(&self.settings));
        self.buffer.clear();
        self.block_index += 1;

        Some(result)
    }

    /// Get block count.
    pub fn block_count(&self) -> u32 {
        self.block_index
    }

    /// Reset.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.block_index = 0;
    }
}

// ============================================================================
// Compression Manager
// ============================================================================

/// Compression statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct CompressionStatistics {
    /// Total uncompressed size.
    pub total_uncompressed: u64,
    /// Total compressed size.
    pub total_compressed: u64,
    /// Deduplicated bytes.
    pub deduplicated_bytes: u64,
    /// Compression operations.
    pub compression_ops: u64,
    /// Decompression operations.
    pub decompression_ops: u64,
    /// Average compression ratio.
    pub avg_ratio: f32,
}

/// Compression manager.
pub struct CompressionManager {
    /// Compression engine.
    engine: CompressionEngine,
    /// Deduplication manager.
    deduplication: DeduplicationManager,
    /// Delta compressors by resource.
    delta_compressors: BTreeMap<u64, DeltaCompressor>,
    /// Compressed blocks.
    blocks: Vec<CompressedBlockInfo>,
    /// Statistics.
    statistics: CompressionStatistics,
    /// Next handle.
    next_handle: AtomicU64,
}

impl CompressionManager {
    /// Create new manager.
    pub fn new(settings: CompressionSettings) -> Self {
        Self {
            engine: CompressionEngine::new(settings),
            deduplication: DeduplicationManager::new(),
            delta_compressors: BTreeMap::new(),
            blocks: Vec::new(),
            statistics: CompressionStatistics::default(),
            next_handle: AtomicU64::new(1),
        }
    }

    /// Compress data.
    pub fn compress(
        &mut self,
        data: &[u8],
        settings: Option<&CompressionSettings>,
    ) -> CompressedBlockHandle {
        // Try deduplication first
        if let Some(handle) = self.deduplication.deduplicate(data) {
            self.statistics.deduplicated_bytes += data.len() as u64;
            return handle;
        }

        // Compress
        let result = self.engine.compress(data, settings);
        let handle = CompressedBlockHandle(self.next_handle.fetch_add(1, Ordering::Relaxed));

        // Store dedup entry
        self.deduplication.store(data, handle);

        // Create block info
        let info = CompressedBlockInfo {
            handle,
            original_size: result.original_size,
            compressed_size: result.compressed_size,
            algorithm: result.algorithm,
            checksum: result.checksum,
            resident: true,
            ref_count: 1,
        };

        self.blocks.push(info);

        // Update statistics
        self.statistics.total_uncompressed += result.original_size;
        self.statistics.total_compressed += result.compressed_size;
        self.statistics.compression_ops += 1;
        self.update_avg_ratio();

        handle
    }

    /// Decompress data.
    pub fn decompress(&mut self, handle: CompressedBlockHandle) -> Option<Vec<u8>> {
        let info = self.blocks.iter().find(|b| b.handle == handle)?;

        self.statistics.decompression_ops += 1;

        // In real impl, we'd have the actual compressed data stored
        // This is a placeholder
        Some(vec![0; info.original_size as usize])
    }

    /// Create delta compressor for resource.
    pub fn create_delta_compressor(&mut self, resource_id: u64, frame_size: usize) {
        self.delta_compressors.insert(resource_id, DeltaCompressor::new(frame_size));
    }

    /// Compress frame with delta.
    pub fn compress_delta(&mut self, resource_id: u64, frame: &[u8]) -> Option<CompressionResult> {
        let compressor = self.delta_compressors.get_mut(&resource_id)?;
        let delta = compressor.compute_delta(frame);

        // Compress the delta
        let settings = CompressionSettings::delta();
        let result = self.engine.compress(&delta, Some(&settings));

        Some(result)
    }

    /// Get block info.
    pub fn get_block_info(&self, handle: CompressedBlockHandle) -> Option<&CompressedBlockInfo> {
        self.blocks.iter().find(|b| b.handle == handle)
    }

    /// Get statistics.
    pub fn statistics(&self) -> &CompressionStatistics {
        &self.statistics
    }

    /// Get overall ratio.
    pub fn overall_ratio(&self) -> f32 {
        if self.statistics.total_compressed == 0 {
            1.0
        } else {
            self.statistics.total_uncompressed as f32 / self.statistics.total_compressed as f32
        }
    }

    /// Get total savings.
    pub fn total_savings(&self) -> u64 {
        self.statistics.total_uncompressed.saturating_sub(self.statistics.total_compressed)
            + self.statistics.deduplicated_bytes
    }

    fn update_avg_ratio(&mut self) {
        self.statistics.avg_ratio = self.overall_ratio();
    }
}

impl Default for CompressionManager {
    fn default() -> Self {
        Self::new(CompressionSettings::default())
    }
}
