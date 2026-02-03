//! # Asset Cache
//!
//! Content-addressed asset caching.

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::String,
    vec::Vec,
};
use crate::{AssetId, AssetMetadata, AssetResult, AssetError, AssetErrorKind};

/// Asset cache for built assets
pub struct AssetCache {
    path: String,
    index: CacheIndex,
    memory_cache: MemoryCache,
    stats: CacheStats,
}

impl AssetCache {
    /// Create a new asset cache
    pub fn new(path: &str) -> AssetResult<Self> {
        Ok(Self {
            path: path.into(),
            index: CacheIndex::new(),
            memory_cache: MemoryCache::new(64 * 1024 * 1024), // 64 MB
            stats: CacheStats::default(),
        })
    }
    
    /// Check if asset is cached
    pub fn contains(&self, id: AssetId) -> bool {
        self.index.entries.contains_key(&id)
    }
    
    /// Get asset metadata
    pub fn get_metadata(&self, id: AssetId) -> AssetResult<AssetMetadata> {
        self.index.entries.get(&id)
            .map(|e| e.metadata.clone())
            .ok_or_else(|| AssetError::new(AssetErrorKind::NotFound, "Asset not in cache"))
    }
    
    /// Load asset data
    pub fn load_data(&mut self, id: AssetId) -> AssetResult<Vec<u8>> {
        // Check memory cache first
        if let Some(data) = self.memory_cache.get(id) {
            self.stats.memory_hits += 1;
            return Ok(data.clone());
        }
        
        self.stats.memory_misses += 1;
        
        // Load from disk
        let entry = self.index.entries.get(&id)
            .ok_or_else(|| AssetError::new(AssetErrorKind::NotFound, "Asset not in cache"))?;
        
        let data = self.load_from_disk(&entry.file_path)?;
        self.stats.disk_reads += 1;
        
        // Add to memory cache
        self.memory_cache.insert(id, data.clone());
        
        Ok(data)
    }
    
    /// Store asset in cache
    pub fn store(&mut self, id: AssetId, data: &[u8], metadata: &AssetMetadata) -> AssetResult<()> {
        let file_path = self.generate_path(id);
        
        // Save to disk
        self.save_to_disk(&file_path, data)?;
        self.stats.disk_writes += 1;
        
        // Update index
        self.index.entries.insert(id, CacheEntry {
            metadata: metadata.clone(),
            file_path,
            size: data.len() as u64,
            timestamp: get_time(),
        });
        
        // Add to memory cache
        self.memory_cache.insert(id, data.to_vec());
        
        Ok(())
    }
    
    /// Remove asset from cache
    pub fn remove(&mut self, id: AssetId) -> AssetResult<()> {
        if let Some(entry) = self.index.entries.remove(&id) {
            self.delete_from_disk(&entry.file_path)?;
        }
        self.memory_cache.remove(id);
        Ok(())
    }
    
    /// Clear entire cache
    pub fn clear(&mut self) -> AssetResult<()> {
        self.index.entries.clear();
        self.memory_cache.clear();
        // Would delete all files from disk
        Ok(())
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }
    
    /// Compact cache (remove unreferenced entries)
    pub fn compact(&mut self, referenced: &[AssetId]) -> AssetResult<u64> {
        let mut freed = 0u64;
        
        let to_remove: Vec<_> = self.index.entries.keys()
            .filter(|id| !referenced.contains(id))
            .copied()
            .collect();
        
        for id in to_remove {
            if let Some(entry) = self.index.entries.remove(&id) {
                freed += entry.size;
                let _ = self.delete_from_disk(&entry.file_path);
            }
        }
        
        Ok(freed)
    }
    
    fn generate_path(&self, id: AssetId) -> String {
        let hex = id.to_hex();
        alloc::format!("{}/{}/{}", self.path, &hex[0..2], hex)
    }
    
    fn load_from_disk(&self, _path: &str) -> AssetResult<Vec<u8>> {
        // Would read from filesystem
        Ok(Vec::new())
    }
    
    fn save_to_disk(&self, _path: &str, _data: &[u8]) -> AssetResult<()> {
        // Would write to filesystem
        Ok(())
    }
    
    fn delete_from_disk(&self, _path: &str) -> AssetResult<()> {
        // Would delete from filesystem
        Ok(())
    }
}

/// Cache index
struct CacheIndex {
    entries: BTreeMap<AssetId, CacheEntry>,
}

impl CacheIndex {
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

/// Cache entry
#[derive(Debug, Clone)]
struct CacheEntry {
    metadata: AssetMetadata,
    file_path: String,
    size: u64,
    timestamp: u64,
}

/// In-memory cache with LRU eviction
struct MemoryCache {
    entries: BTreeMap<AssetId, MemoryCacheEntry>,
    max_size: u64,
    current_size: u64,
    access_counter: u64,
}

impl MemoryCache {
    fn new(max_size: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_size,
            current_size: 0,
            access_counter: 0,
        }
    }
    
    fn get(&mut self, id: AssetId) -> Option<&Vec<u8>> {
        self.access_counter += 1;
        
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.last_access = self.access_counter;
            Some(&entry.data)
        } else {
            None
        }
    }
    
    fn insert(&mut self, id: AssetId, data: Vec<u8>) {
        let size = data.len() as u64;
        
        // Evict if needed
        while self.current_size + size > self.max_size && !self.entries.is_empty() {
            self.evict_lru();
        }
        
        self.access_counter += 1;
        
        if let Some(old) = self.entries.insert(id, MemoryCacheEntry {
            data,
            last_access: self.access_counter,
        }) {
            self.current_size -= old.data.len() as u64;
        }
        
        self.current_size += size;
    }
    
    fn remove(&mut self, id: AssetId) {
        if let Some(entry) = self.entries.remove(&id) {
            self.current_size -= entry.data.len() as u64;
        }
    }
    
    fn clear(&mut self) {
        self.entries.clear();
        self.current_size = 0;
    }
    
    fn evict_lru(&mut self) {
        if let Some((&id, _)) = self.entries.iter()
            .min_by_key(|(_, e)| e.last_access)
        {
            if let Some(entry) = self.entries.remove(&id) {
                self.current_size -= entry.data.len() as u64;
            }
        }
    }
}

/// Memory cache entry
struct MemoryCacheEntry {
    data: Vec<u8>,
    last_access: u64,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub memory_hits: u64,
    pub memory_misses: u64,
    pub disk_reads: u64,
    pub disk_writes: u64,
    pub total_cached_bytes: u64,
    pub total_cached_assets: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f32 {
        let total = self.memory_hits + self.memory_misses;
        if total > 0 {
            self.memory_hits as f32 / total as f32
        } else {
            0.0
        }
    }
}

fn get_time() -> u64 {
    0
}

/// Deduplication for asset storage
pub struct AssetDeduplicator {
    chunk_size: usize,
    chunks: BTreeMap<[u8; 32], u64>,
    next_chunk_id: u64,
}

impl AssetDeduplicator {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            chunks: BTreeMap::new(),
            next_chunk_id: 1,
        }
    }
    
    /// Deduplicate data into chunks
    pub fn deduplicate(&mut self, data: &[u8]) -> DeduplicatedAsset {
        let mut chunk_refs = Vec::new();
        
        for chunk in data.chunks(self.chunk_size) {
            let hash = hash_chunk(chunk);
            
            let chunk_id = if let Some(&id) = self.chunks.get(&hash) {
                id
            } else {
                let id = self.next_chunk_id;
                self.next_chunk_id += 1;
                self.chunks.insert(hash, id);
                id
            };
            
            chunk_refs.push(ChunkRef {
                id: chunk_id,
                hash,
                size: chunk.len() as u32,
            });
        }
        
        DeduplicatedAsset {
            total_size: data.len() as u64,
            chunks: chunk_refs,
        }
    }
    
    /// Get deduplication ratio
    pub fn dedup_ratio(&self) -> f32 {
        // Would calculate actual ratio
        1.0
    }
}

/// Deduplicated asset reference
#[derive(Debug, Clone)]
pub struct DeduplicatedAsset {
    pub total_size: u64,
    pub chunks: Vec<ChunkRef>,
}

/// Chunk reference
#[derive(Debug, Clone)]
pub struct ChunkRef {
    pub id: u64,
    pub hash: [u8; 32],
    pub size: u32,
}

fn hash_chunk(data: &[u8]) -> [u8; 32] {
    let mut hash = [0u8; 32];
    for (i, &byte) in data.iter().enumerate() {
        hash[i % 32] ^= byte;
        hash[(i + 1) % 32] = hash[(i + 1) % 32].wrapping_add(byte);
    }
    hash
}

/// Asset bundle for grouped loading
#[derive(Debug, Clone)]
pub struct AssetBundle {
    pub name: String,
    pub assets: Vec<AssetId>,
    pub dependencies: Vec<String>,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

impl AssetBundle {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            assets: Vec::new(),
            dependencies: Vec::new(),
            compressed_size: 0,
            uncompressed_size: 0,
        }
    }
    
    pub fn add_asset(&mut self, id: AssetId) {
        if !self.assets.contains(&id) {
            self.assets.push(id);
        }
    }
    
    pub fn add_dependency(&mut self, bundle_name: &str) {
        if !self.dependencies.contains(&bundle_name.into()) {
            self.dependencies.push(bundle_name.into());
        }
    }
}

/// Bundle builder
pub struct BundleBuilder {
    bundles: BTreeMap<String, AssetBundle>,
}

impl BundleBuilder {
    pub fn new() -> Self {
        Self {
            bundles: BTreeMap::new(),
        }
    }
    
    /// Create or get a bundle
    pub fn bundle(&mut self, name: &str) -> &mut AssetBundle {
        self.bundles.entry(name.into())
            .or_insert_with(|| AssetBundle::new(name))
    }
    
    /// Build all bundles
    pub fn build(self) -> Vec<AssetBundle> {
        self.bundles.into_values().collect()
    }
}

impl Default for BundleBuilder {
    fn default() -> Self {
        Self::new()
    }
}
