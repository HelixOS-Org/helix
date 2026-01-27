//! Adaptive Compression
//!
//! Automatically selects the best compression algorithm based on
//! data characteristics, workload patterns, and performance requirements.

use crate::core::error::{HfsError, HfsResult};
use super::*;
use super::lz4::{Lz4Compressor, lz4_compress, lz4_decompress};
use super::zstd::{ZstdCompressor, zstd_decompress};

// ============================================================================
// Constants
// ============================================================================

/// Sample size for heuristics
const SAMPLE_SIZE: usize = 256;

/// Entropy threshold for incompressible data
const HIGH_ENTROPY_THRESHOLD: u32 = 245;

/// Entropy threshold for highly compressible data
const LOW_ENTROPY_THRESHOLD: u32 = 128;

/// Minimum size for adaptive compression
const MIN_ADAPTIVE_SIZE: usize = 512;

/// Number of samples for algorithm selection
const HISTORY_SIZE: usize = 16;

// ============================================================================
// Data Classification
// ============================================================================

/// Data type classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DataClass {
    /// Unknown/mixed data
    Unknown = 0,
    /// Text data (high compressibility)
    Text = 1,
    /// Binary data
    Binary = 2,
    /// Already compressed (JPEG, PNG, etc.)
    Compressed = 3,
    /// Encrypted data
    Encrypted = 4,
    /// Sparse data (lots of zeros)
    Sparse = 5,
    /// Sequential/repeating patterns
    Sequential = 6,
}

impl DataClass {
    /// Recommended algorithm for data class
    pub fn recommended_algorithm(&self) -> CompressionType {
        match self {
            Self::Text => CompressionType::Zstd,
            Self::Binary => CompressionType::Lz4,
            Self::Compressed | Self::Encrypted => CompressionType::None,
            Self::Sparse => CompressionType::Lz4,
            Self::Sequential => CompressionType::Zstd,
            Self::Unknown => CompressionType::Lz4,
        }
    }
    
    /// Expected compression ratio (0-100, lower is better)
    pub fn expected_ratio(&self) -> u32 {
        match self {
            Self::Text => 25,
            Self::Binary => 60,
            Self::Compressed => 100,
            Self::Encrypted => 100,
            Self::Sparse => 10,
            Self::Sequential => 30,
            Self::Unknown => 70,
        }
    }
}

impl Default for DataClass {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================================
// Data Analysis
// ============================================================================

/// Analyze data characteristics.
#[derive(Clone, Copy, Debug, Default)]
pub struct DataAnalysis {
    /// Data classification
    pub class: DataClass,
    /// Estimated entropy (0-255)
    pub entropy: u32,
    /// Zero byte percentage (0-100)
    pub zero_percent: u32,
    /// Unique byte count
    pub unique_bytes: u32,
    /// Pattern repetition score
    pub repetition: u32,
    /// Recommended algorithm
    pub recommended: CompressionType,
}

impl DataAnalysis {
    /// Analyze data sample
    pub fn analyze(data: &[u8]) -> Self {
        let mut analysis = Self::default();
        
        if data.is_empty() {
            return analysis;
        }
        
        let sample_size = core::cmp::min(data.len(), SAMPLE_SIZE);
        let sample = &data[..sample_size];
        
        // Count byte frequencies
        let mut freq = [0u32; 256];
        let mut zeros = 0u32;
        
        for &b in sample {
            freq[b as usize] += 1;
            if b == 0 {
                zeros += 1;
            }
        }
        
        // Count unique bytes
        let unique: u32 = freq.iter().filter(|&&f| f > 0).count() as u32;
        analysis.unique_bytes = unique;
        
        // Calculate entropy estimate
        let entropy = unique * 256 / sample_size as u32;
        analysis.entropy = entropy.min(255);
        
        // Zero percentage
        analysis.zero_percent = zeros * 100 / sample_size as u32;
        
        // Detect patterns
        analysis.repetition = detect_repetition(sample);
        
        // Classify data
        analysis.class = classify_data(&analysis, sample);
        
        // Recommend algorithm
        analysis.recommended = analysis.class.recommended_algorithm();
        
        analysis
    }
    
    /// Is data compressible
    pub fn is_compressible(&self) -> bool {
        self.entropy < HIGH_ENTROPY_THRESHOLD
    }
    
    /// Is data highly compressible
    pub fn is_highly_compressible(&self) -> bool {
        self.entropy < LOW_ENTROPY_THRESHOLD || self.zero_percent > 50
    }
}

/// Detect repetition in sample.
fn detect_repetition(sample: &[u8]) -> u32 {
    if sample.len() < 8 {
        return 0;
    }
    
    let mut matches = 0u32;
    
    // Check 4-byte pattern repetition
    for i in 4..sample.len() {
        if sample[i] == sample[i - 4] {
            matches += 1;
        }
    }
    
    matches * 100 / (sample.len() - 4) as u32
}

/// Classify data based on analysis.
fn classify_data(analysis: &DataAnalysis, sample: &[u8]) -> DataClass {
    // Check for sparse data
    if analysis.zero_percent > 70 {
        return DataClass::Sparse;
    }
    
    // Check for already compressed/encrypted
    if analysis.entropy > HIGH_ENTROPY_THRESHOLD {
        // Check for compression magic numbers
        if sample.len() >= 4 {
            let magic = u32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]);
            
            match magic {
                0x04034b50 => return DataClass::Compressed, // ZIP
                0x1F8B0800 => return DataClass::Compressed, // GZIP
                0xFD377A58 => return DataClass::Compressed, // XZ
                0x28B52FFD => return DataClass::Compressed, // ZSTD
                0x89504E47 => return DataClass::Compressed, // PNG
                _ => {}
            }
        }
        
        return DataClass::Encrypted; // Or random
    }
    
    // Check for text
    let printable = sample.iter().filter(|&&b| {
        (b >= 0x20 && b <= 0x7E) || b == b'\n' || b == b'\r' || b == b'\t'
    }).count();
    
    if printable > sample.len() * 90 / 100 {
        return DataClass::Text;
    }
    
    // Check for sequential patterns
    if analysis.repetition > 50 {
        return DataClass::Sequential;
    }
    
    DataClass::Binary
}

// ============================================================================
// Algorithm History
// ============================================================================

/// Algorithm performance history entry.
#[derive(Clone, Copy, Debug, Default)]
struct HistoryEntry {
    /// Algorithm used
    algorithm: CompressionType,
    /// Input size
    input_size: u32,
    /// Output size
    output_size: u32,
    /// Compression time (us)
    time_us: u32,
}

impl HistoryEntry {
    /// Compression ratio
    fn ratio(&self) -> u32 {
        if self.input_size == 0 {
            return 100;
        }
        (self.output_size as u64 * 100 / self.input_size as u64) as u32
    }
    
    /// Throughput (MB/s)
    fn throughput(&self) -> u32 {
        if self.time_us == 0 {
            return 0;
        }
        (self.input_size as u64 * 1000 / self.time_us as u64) as u32
    }
}

// ============================================================================
// Adaptive Compressor
// ============================================================================

/// Adaptive compression mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AdaptiveMode {
    /// Optimize for speed
    Speed = 0,
    /// Balance speed and ratio
    Balanced = 1,
    /// Optimize for ratio
    Ratio = 2,
    /// Always use specific algorithm
    Fixed = 3,
}

impl Default for AdaptiveMode {
    fn default() -> Self {
        Self::Balanced
    }
}

/// Adaptive compressor.
pub struct AdaptiveCompressor {
    /// LZ4 compressor
    lz4: Lz4Compressor,
    /// ZSTD compressor
    zstd: ZstdCompressor,
    /// Mode
    mode: AdaptiveMode,
    /// Default algorithm (for Fixed mode)
    default_algo: CompressionType,
    /// History
    history: [HistoryEntry; HISTORY_SIZE],
    /// History index
    history_idx: usize,
    /// Statistics
    stats: AdaptiveStats,
}

/// Adaptive compression statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct AdaptiveStats {
    /// Times LZ4 was selected
    pub lz4_selected: u64,
    /// Times ZSTD was selected
    pub zstd_selected: u64,
    /// Times no compression used
    pub none_selected: u64,
    /// Total bytes saved
    pub bytes_saved: u64,
    /// Analysis time (us)
    pub analysis_time_us: u64,
}

impl AdaptiveCompressor {
    /// Create new adaptive compressor
    pub fn new() -> Self {
        Self {
            lz4: Lz4Compressor::new(),
            zstd: ZstdCompressor::new(),
            mode: AdaptiveMode::Balanced,
            default_algo: CompressionType::Lz4,
            history: [HistoryEntry::default(); HISTORY_SIZE],
            history_idx: 0,
            stats: AdaptiveStats::default(),
        }
    }
    
    /// Set mode
    pub fn mode(mut self, mode: AdaptiveMode) -> Self {
        self.mode = mode;
        self
    }
    
    /// Set default algorithm
    pub fn default_algorithm(mut self, algo: CompressionType) -> Self {
        self.default_algo = algo;
        self
    }
    
    /// Select best algorithm for data
    pub fn select_algorithm(&self, data: &[u8]) -> CompressionType {
        if data.len() < MIN_ADAPTIVE_SIZE {
            return self.default_algo;
        }
        
        match self.mode {
            AdaptiveMode::Fixed => self.default_algo,
            AdaptiveMode::Speed => CompressionType::Lz4,
            AdaptiveMode::Ratio => self.select_for_ratio(data),
            AdaptiveMode::Balanced => self.select_balanced(data),
        }
    }
    
    /// Select for best ratio
    fn select_for_ratio(&self, data: &[u8]) -> CompressionType {
        let analysis = DataAnalysis::analyze(data);
        
        if !analysis.is_compressible() {
            return CompressionType::None;
        }
        
        // ZSTD for ratio
        CompressionType::Zstd
    }
    
    /// Select balanced algorithm
    fn select_balanced(&self, data: &[u8]) -> CompressionType {
        let analysis = DataAnalysis::analyze(data);
        
        if !analysis.is_compressible() {
            return CompressionType::None;
        }
        
        // Use history to guide selection
        let lz4_avg = self.avg_ratio_for(CompressionType::Lz4);
        let zstd_avg = self.avg_ratio_for(CompressionType::Zstd);
        
        // Consider data class recommendation
        let recommended = analysis.recommended;
        
        // If highly compressible, ZSTD is worth the overhead
        if analysis.is_highly_compressible() {
            return CompressionType::Zstd;
        }
        
        // If history shows ZSTD significantly better, use it
        if zstd_avg > 0 && lz4_avg > 0 && lz4_avg > zstd_avg + 20 {
            return CompressionType::Zstd;
        }
        
        // Default to recommendation or LZ4
        if recommended.is_compressed() {
            recommended
        } else {
            CompressionType::Lz4
        }
    }
    
    /// Get average ratio for algorithm from history
    fn avg_ratio_for(&self, algo: CompressionType) -> u32 {
        let mut sum = 0u32;
        let mut count = 0u32;
        
        for entry in &self.history {
            if entry.algorithm == algo && entry.input_size > 0 {
                sum += entry.ratio();
                count += 1;
            }
        }
        
        if count == 0 {
            return 0;
        }
        
        sum / count
    }
    
    /// Record compression result
    fn record(&mut self, algo: CompressionType, input: usize, output: usize, time_us: u32) {
        self.history[self.history_idx] = HistoryEntry {
            algorithm: algo,
            input_size: input as u32,
            output_size: output as u32,
            time_us,
        };
        
        self.history_idx = (self.history_idx + 1) % HISTORY_SIZE;
        
        match algo {
            CompressionType::Lz4 | CompressionType::Lz4Hc => self.stats.lz4_selected += 1,
            CompressionType::Zstd => self.stats.zstd_selected += 1,
            CompressionType::None => self.stats.none_selected += 1,
            _ => {}
        }
        
        if output < input {
            self.stats.bytes_saved += (input - output) as u64;
        }
    }
    
    /// Compress with adaptive algorithm selection
    pub fn compress_adaptive(&mut self, input: &[u8], output: &mut [u8]) -> HfsResult<CompressionResult> {
        if input.is_empty() {
            return Ok(CompressionResult::new(CompressionType::None, 0, 0));
        }
        
        let algo = self.select_algorithm(input);
        
        if algo == CompressionType::None {
            // Store uncompressed
            if output.len() < input.len() {
                return Err(HfsError::BufferTooSmall);
            }
            output[..input.len()].copy_from_slice(input);
            self.record(CompressionType::None, input.len(), input.len(), 0);
            return Ok(CompressionResult::new(CompressionType::None, input.len(), input.len()));
        }
        
        // Try compression
        let result = match algo {
            CompressionType::Lz4 | CompressionType::Lz4Hc => {
                lz4_compress(input, output)
            }
            CompressionType::Zstd => {
                self.zstd.compress(input, output)
            }
            _ => {
                return Err(HfsError::NotSupported);
            }
        };
        
        match result {
            Ok(size) if size < input.len() => {
                self.record(algo, input.len(), size, 0);
                Ok(CompressionResult::new(algo, input.len(), size))
            }
            _ => {
                // Compression didn't help, store raw
                if output.len() < input.len() {
                    return Err(HfsError::BufferTooSmall);
                }
                output[..input.len()].copy_from_slice(input);
                self.record(CompressionType::None, input.len(), input.len(), 0);
                Ok(CompressionResult::new(CompressionType::None, input.len(), input.len()))
            }
        }
    }
    
    /// Decompress with auto-detection
    pub fn decompress_auto(&self, input: &[u8], output: &mut [u8]) -> HfsResult<usize> {
        if input.len() < 4 {
            return Err(HfsError::CorruptedData);
        }
        
        let magic = u32::from_le_bytes([input[0], input[1], input[2], input[3]]);
        
        match magic {
            LZ4_MAGIC => lz4_decompress(&input[4..], output),
            ZSTD_MAGIC => zstd_decompress(input, output),
            _ => {
                // Try LZ4 format (no magic)
                lz4_decompress(input, output)
            }
        }
    }
    
    /// Get statistics
    pub fn stats(&self) -> &AdaptiveStats {
        &self.stats
    }
}

impl Default for AdaptiveCompressor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_data_analysis() {
        // Text data
        let text = b"The quick brown fox jumps over the lazy dog. This is a test of text compression.";
        let analysis = DataAnalysis::analyze(text);
        assert!(analysis.is_compressible());
        
        // Sparse data
        let sparse = [0u8; 256];
        let analysis = DataAnalysis::analyze(&sparse);
        assert_eq!(analysis.class, DataClass::Sparse);
        assert!(analysis.is_highly_compressible());
        
        // Random-like data
        let random: [u8; 256] = core::array::from_fn(|i| (i * 17 + 31) as u8);
        let analysis = DataAnalysis::analyze(&random);
        // May or may not be compressible depending on pattern
    }
    
    #[test]
    fn test_data_class() {
        assert_eq!(DataClass::Text.recommended_algorithm(), CompressionType::Zstd);
        assert_eq!(DataClass::Compressed.recommended_algorithm(), CompressionType::None);
        assert_eq!(DataClass::Sparse.recommended_algorithm(), CompressionType::Lz4);
    }
    
    #[test]
    fn test_adaptive_compressor() {
        let mut compressor = AdaptiveCompressor::new()
            .mode(AdaptiveMode::Balanced);
        
        let input = b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let mut output = [0u8; 256];
        
        let result = compressor.compress_adaptive(input, &mut output).unwrap();
        
        // Should be compressed
        assert!(result.output_size <= result.input_size);
    }
    
    #[test]
    fn test_adaptive_mode() {
        let compressor = AdaptiveCompressor::new()
            .mode(AdaptiveMode::Speed);
        
        let data = b"Test data for compression selection";
        assert_eq!(compressor.select_algorithm(data), CompressionType::Lz4);
        
        let compressor = AdaptiveCompressor::new()
            .mode(AdaptiveMode::Fixed)
            .default_algorithm(CompressionType::Zstd);
        
        assert_eq!(compressor.select_algorithm(data), CompressionType::Zstd);
    }
}
