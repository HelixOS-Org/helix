//! # Asset Streaming
//!
//! Progressive asset loading and streaming.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::{AssetError, AssetErrorKind, AssetId, AssetResult, LoadPriority};

/// Asset streaming manager
pub struct StreamingManager {
    config: StreamingConfig,
    pending_requests: Vec<StreamRequest>,
    active_streams: BTreeMap<AssetId, ActiveStream>,
    completed_chunks: Vec<CompletedChunk>,
    budget: StreamingBudget,
    stats: StreamingStats,
}

impl StreamingManager {
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            config,
            pending_requests: Vec::new(),
            active_streams: BTreeMap::new(),
            completed_chunks: Vec::new(),
            budget: StreamingBudget::new(config.max_bandwidth, config.max_memory),
            stats: StreamingStats::default(),
        }
    }

    /// Request an asset to be streamed
    pub fn request(&mut self, asset_id: AssetId, priority: StreamPriority) {
        if self.active_streams.contains_key(&asset_id) {
            // Already streaming
            return;
        }

        self.pending_requests.push(StreamRequest {
            asset_id,
            priority,
            requested_time: get_time(),
        });

        // Sort by priority
        self.pending_requests
            .sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Cancel a streaming request
    pub fn cancel(&mut self, asset_id: AssetId) {
        self.pending_requests.retain(|r| r.asset_id != asset_id);
        self.active_streams.remove(&asset_id);
    }

    /// Update streaming (call each frame)
    pub fn update(&mut self, delta_time: f32) {
        self.budget.reset_frame();

        // Process completed chunks
        for chunk in self.completed_chunks.drain(..) {
            if let Some(stream) = self.active_streams.get_mut(&chunk.asset_id) {
                stream.received_bytes += chunk.data.len() as u64;
                stream.chunks.push(chunk);

                if stream.received_bytes >= stream.total_bytes {
                    stream.state = StreamState::Complete;
                }
            }
        }

        // Start new streams if budget allows
        while let Some(request) = self.pending_requests.first() {
            if !self.budget.can_start_stream() {
                break;
            }

            let request = self.pending_requests.remove(0);
            self.start_stream(request);
        }

        // Update active streams
        for stream in self.active_streams.values_mut() {
            if stream.state == StreamState::Streaming {
                self.process_stream(stream, delta_time);
            }
        }

        // Update stats
        self.stats.active_streams = self.active_streams.len() as u32;
        self.stats.pending_requests = self.pending_requests.len() as u32;
    }

    /// Get completed assets
    pub fn get_completed(&mut self) -> Vec<(AssetId, Vec<u8>)> {
        let mut completed = Vec::new();

        let complete_ids: Vec<_> = self
            .active_streams
            .iter()
            .filter(|(_, s)| s.state == StreamState::Complete)
            .map(|(id, _)| *id)
            .collect();

        for id in complete_ids {
            if let Some(stream) = self.active_streams.remove(&id) {
                let data: Vec<u8> = stream.chunks.into_iter().flat_map(|c| c.data).collect();
                completed.push((id, data));
            }
        }

        completed
    }

    /// Get streaming progress for an asset
    pub fn progress(&self, asset_id: AssetId) -> Option<f32> {
        self.active_streams.get(&asset_id).map(|s| {
            if s.total_bytes > 0 {
                s.received_bytes as f32 / s.total_bytes as f32
            } else {
                0.0
            }
        })
    }

    /// Get streaming statistics
    pub fn stats(&self) -> &StreamingStats {
        &self.stats
    }

    fn start_stream(&mut self, request: StreamRequest) {
        self.active_streams.insert(request.asset_id, ActiveStream {
            asset_id: request.asset_id,
            priority: request.priority,
            state: StreamState::Streaming,
            total_bytes: 0, // Would be fetched from metadata
            received_bytes: 0,
            chunks: Vec::new(),
            start_time: get_time(),
        });
    }

    fn process_stream(&mut self, _stream: &mut ActiveStream, _delta_time: f32) {
        // Would process IO here
    }
}

/// Streaming configuration
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum bandwidth per frame (bytes)
    pub max_bandwidth: u64,
    /// Maximum memory for streaming buffers
    pub max_memory: u64,
    /// Maximum concurrent streams
    pub max_concurrent: u32,
    /// Chunk size for streaming
    pub chunk_size: u32,
    /// Enable compression
    pub compression: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_bandwidth: 10 * 1024 * 1024, // 10 MB/frame
            max_memory: 256 * 1024 * 1024,   // 256 MB
            max_concurrent: 8,
            chunk_size: 64 * 1024, // 64 KB chunks
            compression: true,
        }
    }
}

/// Streaming priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StreamPriority {
    Background = 0,
    Low        = 1,
    Normal     = 2,
    High       = 3,
    Immediate  = 4,
}

/// Stream request
struct StreamRequest {
    asset_id: AssetId,
    priority: StreamPriority,
    requested_time: u64,
}

/// Active stream
struct ActiveStream {
    asset_id: AssetId,
    priority: StreamPriority,
    state: StreamState,
    total_bytes: u64,
    received_bytes: u64,
    chunks: Vec<CompletedChunk>,
    start_time: u64,
}

/// Stream state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamState {
    Pending,
    Streaming,
    Complete,
    Failed,
}

/// Completed chunk
struct CompletedChunk {
    asset_id: AssetId,
    index: u32,
    data: Vec<u8>,
}

/// Streaming budget manager
struct StreamingBudget {
    max_bandwidth: u64,
    max_memory: u64,
    used_bandwidth: u64,
    used_memory: u64,
}

impl StreamingBudget {
    fn new(max_bandwidth: u64, max_memory: u64) -> Self {
        Self {
            max_bandwidth,
            max_memory,
            used_bandwidth: 0,
            used_memory: 0,
        }
    }

    fn reset_frame(&mut self) {
        self.used_bandwidth = 0;
    }

    fn can_start_stream(&self) -> bool {
        self.used_bandwidth < self.max_bandwidth && self.used_memory < self.max_memory
    }
}

/// Streaming statistics
#[derive(Debug, Clone, Default)]
pub struct StreamingStats {
    pub active_streams: u32,
    pub pending_requests: u32,
    pub bytes_streamed: u64,
    pub bandwidth_used: u64,
    pub memory_used: u64,
}

fn get_time() -> u64 {
    0
}

/// Mipmap streaming for textures
pub struct MipmapStreamer {
    /// Loaded mip levels per texture
    loaded_mips: BTreeMap<AssetId, LoadedMips>,
    /// Target mip bias
    mip_bias: f32,
    /// Memory budget for textures
    memory_budget: u64,
    /// Current memory usage
    memory_used: u64,
}

impl MipmapStreamer {
    pub fn new(memory_budget: u64) -> Self {
        Self {
            loaded_mips: BTreeMap::new(),
            mip_bias: 0.0,
            memory_budget,
            memory_used: 0,
        }
    }

    /// Request mip level for a texture
    pub fn request_mip(&mut self, asset_id: AssetId, mip_level: u8) {
        let entry = self
            .loaded_mips
            .entry(asset_id)
            .or_insert_with(|| LoadedMips {
                lowest_loaded: u8::MAX,
                highest_loaded: 0,
                requested: mip_level,
            });

        entry.requested = entry.requested.min(mip_level);
    }

    /// Get required mip loads
    pub fn get_pending_loads(&self) -> Vec<(AssetId, u8)> {
        self.loaded_mips
            .iter()
            .filter(|(_, m)| m.requested < m.lowest_loaded)
            .map(|(id, m)| (*id, m.requested))
            .collect()
    }

    /// Mark mip as loaded
    pub fn mark_loaded(&mut self, asset_id: AssetId, mip_level: u8, size: u64) {
        if let Some(mips) = self.loaded_mips.get_mut(&asset_id) {
            mips.lowest_loaded = mips.lowest_loaded.min(mip_level);
            mips.highest_loaded = mips.highest_loaded.max(mip_level);
            self.memory_used += size;
        }
    }

    /// Evict mip levels to stay within budget
    pub fn evict_if_needed(&mut self) -> Vec<(AssetId, u8)> {
        let mut evictions = Vec::new();

        while self.memory_used > self.memory_budget {
            // Find texture with most loaded mips
            if let Some((&id, mips)) = self
                .loaded_mips
                .iter()
                .filter(|(_, m)| m.lowest_loaded < m.highest_loaded)
                .max_by_key(|(_, m)| m.highest_loaded - m.lowest_loaded)
            {
                evictions.push((id, mips.highest_loaded));

                if let Some(m) = self.loaded_mips.get_mut(&id) {
                    m.highest_loaded -= 1;
                }

                // Approximate size reduction
                self.memory_used = self.memory_used.saturating_sub(1024);
            } else {
                break;
            }
        }

        evictions
    }
}

/// Loaded mip levels
struct LoadedMips {
    lowest_loaded: u8,
    highest_loaded: u8,
    requested: u8,
}

/// LOD streaming for meshes
pub struct LodStreamer {
    loaded_lods: BTreeMap<AssetId, LoadedLods>,
    distance_thresholds: Vec<f32>,
}

impl LodStreamer {
    pub fn new() -> Self {
        Self {
            loaded_lods: BTreeMap::new(),
            distance_thresholds: vec![10.0, 30.0, 100.0, 500.0],
        }
    }

    /// Calculate required LOD for distance
    pub fn lod_for_distance(&self, distance: f32) -> u8 {
        for (i, &threshold) in self.distance_thresholds.iter().enumerate() {
            if distance < threshold {
                return i as u8;
            }
        }
        self.distance_thresholds.len() as u8
    }

    /// Request LOD level
    pub fn request_lod(&mut self, asset_id: AssetId, lod_level: u8) {
        let entry = self
            .loaded_lods
            .entry(asset_id)
            .or_insert_with(|| LoadedLods {
                loaded: 0,
                requested: lod_level,
            });

        entry.requested = entry.requested.min(lod_level);
    }

    /// Get pending LOD loads
    pub fn get_pending_loads(&self) -> Vec<(AssetId, u8)> {
        self.loaded_lods
            .iter()
            .filter(|(_, l)| l.requested < l.loaded || l.loaded == 0)
            .map(|(id, l)| (*id, l.requested))
            .collect()
    }
}

impl Default for LodStreamer {
    fn default() -> Self {
        Self::new()
    }
}

/// Loaded LOD levels
struct LoadedLods {
    loaded: u8,
    requested: u8,
}

/// Prefetch system for predictive loading
pub struct Prefetcher {
    predictions: Vec<PrefetchPrediction>,
    history: Vec<AssetId>,
    max_history: usize,
}

impl Prefetcher {
    pub fn new(max_history: usize) -> Self {
        Self {
            predictions: Vec::new(),
            history: Vec::new(),
            max_history,
        }
    }

    /// Record asset access
    pub fn record_access(&mut self, asset_id: AssetId) {
        self.history.push(asset_id);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    /// Add a prefetch prediction
    pub fn add_prediction(&mut self, prediction: PrefetchPrediction) {
        self.predictions.push(prediction);
    }

    /// Get assets to prefetch
    pub fn get_prefetch_list(&self) -> Vec<(AssetId, f32)> {
        self.predictions
            .iter()
            .filter(|p| p.probability > 0.5)
            .map(|p| (p.asset_id, p.probability))
            .collect()
    }

    /// Learn from access patterns
    pub fn learn(&mut self) {
        // Would analyze history to update predictions
    }
}

/// Prefetch prediction
#[derive(Debug, Clone)]
pub struct PrefetchPrediction {
    pub asset_id: AssetId,
    pub probability: f32,
    pub estimated_use_time: f32,
}
