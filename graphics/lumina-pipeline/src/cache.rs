//! Pipeline Cache
//!
//! This module provides pipeline caching functionality including:
//! - Memory-based pipeline cache
//! - Disk persistence
//! - Cache statistics
//! - Automatic cache management

use alloc::{boxed::Box, string::String, vec::Vec};
use core::hash::{Hash, Hasher};

// ============================================================================
// Cache Configuration
// ============================================================================

/// Pipeline cache configuration.
#[derive(Clone)]
pub struct CacheConfig {
    /// Maximum cache size in bytes.
    pub max_size_bytes: usize,
    /// Maximum number of pipelines.
    pub max_pipelines: usize,
    /// Enable disk persistence.
    pub enable_persistence: bool,
    /// Cache file path.
    pub cache_path: Option<String>,
    /// Enable cache validation.
    pub validate_cache: bool,
    /// Cache version for invalidation.
    pub cache_version: u32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 256 * 1024 * 1024, // 256 MB
            max_pipelines: 4096,
            enable_persistence: true,
            cache_path: None,
            validate_cache: true,
            cache_version: 1,
        }
    }
}

impl CacheConfig {
    /// Create a new cache config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum size in bytes.
    pub fn max_size_bytes(mut self, size: usize) -> Self {
        self.max_size_bytes = size;
        self
    }

    /// Set maximum pipeline count.
    pub fn max_pipelines(mut self, count: usize) -> Self {
        self.max_pipelines = count;
        self
    }

    /// Enable or disable persistence.
    pub fn persistence(mut self, enabled: bool) -> Self {
        self.enable_persistence = enabled;
        self
    }

    /// Set cache path.
    pub fn cache_path(mut self, path: &str) -> Self {
        self.cache_path = Some(String::from(path));
        self
    }

    /// Set cache version.
    pub fn cache_version(mut self, version: u32) -> Self {
        self.cache_version = version;
        self
    }
}

// ============================================================================
// Cache Statistics
// ============================================================================

/// Pipeline cache statistics.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of pipelines in cache.
    pub pipeline_count: u32,
    /// Current cache size in bytes.
    pub size_bytes: usize,
    /// Number of evictions.
    pub evictions: u64,
    /// Number of insertions.
    pub insertions: u64,
    /// Number of validation failures.
    pub validation_failures: u64,
    /// Time spent compiling (microseconds).
    pub compile_time_us: u64,
    /// Time saved by cache (microseconds).
    pub saved_time_us: u64,
}

impl CacheStats {
    /// Get hit rate.
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f32 / total as f32
        }
    }

    /// Get average compile time.
    pub fn average_compile_time_us(&self) -> u64 {
        if self.insertions == 0 {
            0
        } else {
            self.compile_time_us / self.insertions
        }
    }

    /// Get cache efficiency.
    pub fn efficiency(&self) -> f32 {
        if self.compile_time_us == 0 {
            0.0
        } else {
            self.saved_time_us as f32 / (self.compile_time_us + self.saved_time_us) as f32
        }
    }

    /// Reset statistics.
    pub fn reset(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
        self.insertions = 0;
        self.validation_failures = 0;
        self.compile_time_us = 0;
        self.saved_time_us = 0;
    }
}

// ============================================================================
// Cache Entry
// ============================================================================

/// Pipeline cache entry.
#[derive(Clone)]
struct CacheEntry {
    /// Pipeline hash.
    hash: u64,
    /// Cached binary data.
    data: Vec<u8>,
    /// Entry size in bytes.
    size: usize,
    /// Last access time (frame number).
    last_access: u64,
    /// Access count.
    access_count: u32,
    /// Creation time (frame number).
    creation_frame: u64,
}

impl CacheEntry {
    fn new(hash: u64, data: Vec<u8>, frame: u64) -> Self {
        let size = data.len();
        Self {
            hash,
            data,
            size,
            last_access: frame,
            access_count: 1,
            creation_frame: frame,
        }
    }

    fn touch(&mut self, frame: u64) {
        self.last_access = frame;
        self.access_count += 1;
    }

    /// Calculate LRU-K score.
    fn lru_score(&self, current_frame: u64) -> u64 {
        let age = current_frame.saturating_sub(self.last_access);
        let frequency = self.access_count as u64;
        // Higher score = more likely to be evicted
        age / frequency.max(1)
    }
}

// ============================================================================
// Pipeline Cache
// ============================================================================

/// Pipeline cache for storing compiled pipeline state.
pub struct PipelineCache {
    /// Configuration.
    config: CacheConfig,
    /// Cache entries.
    entries: Vec<CacheEntry>,
    /// Statistics.
    stats: CacheStats,
    /// Current frame.
    current_frame: u64,
    /// Device UUID for validation.
    device_uuid: [u8; 16],
    /// Driver version for validation.
    driver_version: u32,
}

impl PipelineCache {
    /// Create a new pipeline cache.
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            entries: Vec::new(),
            stats: CacheStats::default(),
            current_frame: 0,
            device_uuid: [0; 16],
            driver_version: 0,
        }
    }

    /// Create with device info.
    pub fn with_device_info(config: CacheConfig, device_uuid: [u8; 16], driver_version: u32) -> Self {
        Self {
            config,
            entries: Vec::new(),
            stats: CacheStats::default(),
            current_frame: 0,
            device_uuid,
            driver_version,
        }
    }

    /// Set device info.
    pub fn set_device_info(&mut self, uuid: [u8; 16], version: u32) {
        self.device_uuid = uuid;
        self.driver_version = version;
    }

    /// Look up a cached pipeline.
    pub fn lookup(&mut self, hash: u64) -> Option<&[u8]> {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.hash == hash) {
            entry.touch(self.current_frame);
            self.stats.hits += 1;
            Some(&entry.data)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Insert a pipeline into the cache.
    pub fn insert(&mut self, hash: u64, data: Vec<u8>) {
        // Check if already exists
        if self.entries.iter().any(|e| e.hash == hash) {
            return;
        }

        let entry_size = data.len();

        // Evict if necessary
        while self.stats.size_bytes + entry_size > self.config.max_size_bytes
            || self.entries.len() >= self.config.max_pipelines
        {
            if !self.evict_one() {
                break;
            }
        }

        // Insert new entry
        let entry = CacheEntry::new(hash, data, self.current_frame);
        self.stats.size_bytes += entry.size;
        self.stats.pipeline_count += 1;
        self.stats.insertions += 1;
        self.entries.push(entry);
    }

    /// Evict one entry using LRU-K algorithm.
    fn evict_one(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }

        // Find entry with highest eviction score
        let mut max_score = 0u64;
        let mut evict_idx = 0;

        for (i, entry) in self.entries.iter().enumerate() {
            let score = entry.lru_score(self.current_frame);
            if score > max_score {
                max_score = score;
                evict_idx = i;
            }
        }

        let entry = self.entries.remove(evict_idx);
        self.stats.size_bytes -= entry.size;
        self.stats.pipeline_count -= 1;
        self.stats.evictions += 1;

        true
    }

    /// Record compile time.
    pub fn record_compile_time(&mut self, microseconds: u64) {
        self.stats.compile_time_us += microseconds;
    }

    /// Record saved time.
    pub fn record_saved_time(&mut self, microseconds: u64) {
        self.stats.saved_time_us += microseconds;
    }

    /// Get statistics.
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Advance to next frame.
    pub fn advance_frame(&mut self) {
        self.current_frame += 1;
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.stats.size_bytes = 0;
        self.stats.pipeline_count = 0;
    }

    /// Serialize cache to bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // Header
        let header = CacheHeader {
            magic: CACHE_MAGIC,
            version: self.config.cache_version,
            device_uuid: self.device_uuid,
            driver_version: self.driver_version,
            entry_count: self.entries.len() as u32,
        };

        // Write header
        data.extend_from_slice(&header.magic.to_le_bytes());
        data.extend_from_slice(&header.version.to_le_bytes());
        data.extend_from_slice(&header.device_uuid);
        data.extend_from_slice(&header.driver_version.to_le_bytes());
        data.extend_from_slice(&header.entry_count.to_le_bytes());

        // Write entries
        for entry in &self.entries {
            data.extend_from_slice(&entry.hash.to_le_bytes());
            data.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            data.extend_from_slice(&entry.data);
        }

        data
    }

    /// Deserialize cache from bytes.
    pub fn deserialize(&mut self, data: &[u8]) -> Result<(), CacheError> {
        if data.len() < HEADER_SIZE {
            return Err(CacheError::InvalidFormat);
        }

        let mut offset = 0;

        // Read header
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        offset += 4;

        if magic != CACHE_MAGIC {
            return Err(CacheError::InvalidMagic);
        }

        let version = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
        offset += 4;

        if version != self.config.cache_version {
            return Err(CacheError::VersionMismatch);
        }

        let mut device_uuid = [0u8; 16];
        device_uuid.copy_from_slice(&data[offset..offset + 16]);
        offset += 16;

        if self.config.validate_cache && device_uuid != self.device_uuid {
            self.stats.validation_failures += 1;
            return Err(CacheError::DeviceMismatch);
        }

        let driver_version = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
        offset += 4;

        if self.config.validate_cache && driver_version != self.driver_version {
            self.stats.validation_failures += 1;
            return Err(CacheError::DriverMismatch);
        }

        let entry_count = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
        offset += 4;

        // Read entries
        for _ in 0..entry_count {
            if offset + 12 > data.len() {
                return Err(CacheError::InvalidFormat);
            }

            let hash = u64::from_le_bytes([
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3],
                data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7],
            ]);
            offset += 8;

            let size = u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
            offset += 4;

            if offset + size > data.len() {
                return Err(CacheError::InvalidFormat);
            }

            let entry_data = data[offset..offset + size].to_vec();
            offset += size;

            // Insert into cache
            let entry = CacheEntry::new(hash, entry_data, self.current_frame);
            self.stats.size_bytes += entry.size;
            self.stats.pipeline_count += 1;
            self.entries.push(entry);
        }

        Ok(())
    }

    /// Merge another cache into this one.
    pub fn merge(&mut self, other: &PipelineCache) {
        for entry in &other.entries {
            if !self.entries.iter().any(|e| e.hash == entry.hash) {
                self.insert(entry.hash, entry.data.clone());
            }
        }
    }

    /// Get entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get current size in bytes.
    pub fn size_bytes(&self) -> usize {
        self.stats.size_bytes
    }

    /// Compute cache hash for a pipeline.
    pub fn compute_hash<T: Hash>(key: &T) -> u64 {
        let mut hasher = FnvHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

const CACHE_MAGIC: u32 = 0x4C554D49; // "LUMI"
const HEADER_SIZE: usize = 4 + 4 + 16 + 4 + 4; // magic + version + uuid + driver + count

/// Cache header.
struct CacheHeader {
    magic: u32,
    version: u32,
    device_uuid: [u8; 16],
    driver_version: u32,
    entry_count: u32,
}

/// Cache error.
#[derive(Debug, Clone)]
pub enum CacheError {
    /// Invalid format.
    InvalidFormat,
    /// Invalid magic number.
    InvalidMagic,
    /// Version mismatch.
    VersionMismatch,
    /// Device UUID mismatch.
    DeviceMismatch,
    /// Driver version mismatch.
    DriverMismatch,
    /// IO error.
    IoError(String),
}

// ============================================================================
// Pipeline State Object Cache
// ============================================================================

/// PSO cache key.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PsoCacheKey {
    /// Vertex shader hash.
    pub vertex_shader_hash: u64,
    /// Fragment shader hash.
    pub fragment_shader_hash: u64,
    /// Render state hash.
    pub render_state_hash: u64,
    /// Vertex layout hash.
    pub vertex_layout_hash: u64,
    /// Render target format hash.
    pub render_target_hash: u64,
}

impl PsoCacheKey {
    /// Create a new PSO cache key.
    pub fn new(
        vertex_shader_hash: u64,
        fragment_shader_hash: u64,
        render_state_hash: u64,
        vertex_layout_hash: u64,
        render_target_hash: u64,
    ) -> Self {
        Self {
            vertex_shader_hash,
            fragment_shader_hash,
            render_state_hash,
            vertex_layout_hash,
            render_target_hash,
        }
    }

    /// Compute combined hash.
    pub fn combined_hash(&self) -> u64 {
        let mut hasher = FnvHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// PSO cache for quick pipeline lookup.
pub struct PsoCache {
    /// Cached PSOs.
    entries: Vec<(PsoCacheKey, u32)>, // key -> pipeline handle
    /// Maximum entries.
    max_entries: usize,
}

impl PsoCache {
    /// Create a new PSO cache.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Lookup a PSO.
    pub fn lookup(&self, key: &PsoCacheKey) -> Option<u32> {
        self.entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, handle)| *handle)
    }

    /// Insert a PSO.
    pub fn insert(&mut self, key: PsoCacheKey, handle: u32) {
        // Check if exists
        if self.entries.iter().any(|(k, _)| k == &key) {
            return;
        }

        // Evict if necessary
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }

        self.entries.push((key, handle));
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for PsoCache {
    fn default() -> Self {
        Self::new(4096)
    }
}

// ============================================================================
// Shader Binary Cache
// ============================================================================

/// Shader binary cache.
pub struct ShaderBinaryCache {
    /// Cached binaries.
    entries: Vec<(u64, Vec<u8>)>,
    /// Total size.
    total_size: usize,
    /// Max size.
    max_size: usize,
}

impl ShaderBinaryCache {
    /// Create a new shader binary cache.
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            total_size: 0,
            max_size,
        }
    }

    /// Lookup a shader binary.
    pub fn lookup(&self, hash: u64) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|(h, _)| *h == hash)
            .map(|(_, data)| data.as_slice())
    }

    /// Insert a shader binary.
    pub fn insert(&mut self, hash: u64, data: Vec<u8>) {
        if self.entries.iter().any(|(h, _)| *h == hash) {
            return;
        }

        let size = data.len();

        // Evict if necessary
        while self.total_size + size > self.max_size && !self.entries.is_empty() {
            let (_, removed) = self.entries.remove(0);
            self.total_size -= removed.len();
        }

        self.total_size += size;
        self.entries.push((hash, data));
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_size = 0;
    }

    /// Get entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get total size.
    pub fn size(&self) -> usize {
        self.total_size
    }
}

impl Default for ShaderBinaryCache {
    fn default() -> Self {
        Self::new(64 * 1024 * 1024) // 64 MB
    }
}

// ============================================================================
// FNV Hasher
// ============================================================================

/// FNV-1a hasher.
struct FnvHasher {
    state: u64,
}

impl FnvHasher {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self {
            state: Self::FNV_OFFSET,
        }
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= *byte as u64;
            self.state = self.state.wrapping_mul(Self::FNV_PRIME);
        }
    }
}
