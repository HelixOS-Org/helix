//! Asset Streaming Types for Lumina
//!
//! This module provides asset streaming infrastructure including
//! texture streaming, mesh streaming, and priority-based loading.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Streaming Handles
// ============================================================================

/// Streaming asset handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StreamingAssetHandle(pub u64);

impl StreamingAssetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for StreamingAssetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Streaming pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StreamingPoolHandle(pub u64);

impl StreamingPoolHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for StreamingPoolHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Stream request handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StreamRequestHandle(pub u64);

impl StreamRequestHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for StreamRequestHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Streaming Settings
// ============================================================================

/// Streaming settings
#[derive(Clone, Debug)]
pub struct StreamingSettings {
    /// Memory budget (bytes)
    pub memory_budget: u64,
    /// Max concurrent loads
    pub max_concurrent: u32,
    /// Preload distance
    pub preload_distance: f32,
    /// Unload distance
    pub unload_distance: f32,
    /// Update frequency (frames)
    pub update_frequency: u32,
    /// Priority boost for visible
    pub visible_priority_boost: f32,
}

impl StreamingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            memory_budget: 512 * 1024 * 1024, // 512 MB
            max_concurrent: 8,
            preload_distance: 50.0,
            unload_distance: 100.0,
            update_frequency: 1,
            visible_priority_boost: 2.0,
        }
    }

    /// Low memory preset
    pub fn low_memory() -> Self {
        Self {
            memory_budget: 128 * 1024 * 1024, // 128 MB
            max_concurrent: 4,
            ..Self::new()
        }
    }

    /// High performance preset
    pub fn high_performance() -> Self {
        Self {
            memory_budget: 2 * 1024 * 1024 * 1024, // 2 GB
            max_concurrent: 16,
            preload_distance: 100.0,
            ..Self::new()
        }
    }

    /// With memory budget
    pub fn with_budget(mut self, bytes: u64) -> Self {
        self.memory_budget = bytes;
        self
    }

    /// With concurrent loads
    pub fn with_concurrent(mut self, count: u32) -> Self {
        self.max_concurrent = count;
        self
    }
}

impl Default for StreamingSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Streaming Pool
// ============================================================================

/// Streaming pool create info
#[derive(Clone, Debug)]
pub struct StreamingPoolCreateInfo {
    /// Pool type
    pub pool_type: StreamingPoolType,
    /// Pool size (bytes)
    pub size: u64,
    /// Block size
    pub block_size: u32,
    /// Alignment
    pub alignment: u32,
}

impl StreamingPoolCreateInfo {
    /// Creates info
    pub fn new(pool_type: StreamingPoolType, size: u64) -> Self {
        Self {
            pool_type,
            size,
            block_size: 64 * 1024, // 64KB blocks
            alignment: 256,
        }
    }

    /// Texture pool
    pub fn textures(size: u64) -> Self {
        Self::new(StreamingPoolType::Texture, size)
    }

    /// Mesh pool
    pub fn meshes(size: u64) -> Self {
        Self::new(StreamingPoolType::Mesh, size)
    }

    /// Block count
    pub fn block_count(&self) -> u32 {
        (self.size / self.block_size as u64) as u32
    }
}

impl Default for StreamingPoolCreateInfo {
    fn default() -> Self {
        Self::textures(256 * 1024 * 1024)
    }
}

/// Streaming pool type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StreamingPoolType {
    /// Texture streaming
    #[default]
    Texture = 0,
    /// Mesh streaming
    Mesh = 1,
    /// Audio streaming
    Audio = 2,
    /// Animation streaming
    Animation = 3,
    /// Generic
    Generic = 4,
}

/// Pool allocation
#[derive(Clone, Copy, Debug)]
pub struct PoolAllocation {
    /// Offset in pool
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Block index
    pub block: u32,
    /// Block count
    pub block_count: u32,
}

impl PoolAllocation {
    /// Creates allocation
    pub fn new(offset: u64, size: u64) -> Self {
        Self {
            offset,
            size,
            block: 0,
            block_count: 0,
        }
    }
}

// ============================================================================
// Texture Streaming
// ============================================================================

/// Texture streaming settings
#[derive(Clone, Debug)]
pub struct TextureStreamingSettings {
    /// Max mip bias
    pub max_mip_bias: f32,
    /// Min resident mips
    pub min_resident_mips: u32,
    /// Target mip latency (frames)
    pub mip_latency: u32,
    /// Use sparse textures
    pub sparse_textures: bool,
    /// Feedback buffer resolution
    pub feedback_resolution: u32,
}

impl TextureStreamingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            max_mip_bias: 2.0,
            min_resident_mips: 4,
            mip_latency: 3,
            sparse_textures: false,
            feedback_resolution: 128,
        }
    }

    /// With sparse textures
    pub fn with_sparse(mut self) -> Self {
        self.sparse_textures = true;
        self
    }
}

impl Default for TextureStreamingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming texture info
#[derive(Clone, Debug)]
pub struct StreamingTextureInfo {
    /// Asset handle
    pub asset: StreamingAssetHandle,
    /// Full resolution
    pub full_resolution: [u32; 2],
    /// Total mips
    pub total_mips: u32,
    /// Resident mips
    pub resident_mips: u32,
    /// Required mip (based on usage)
    pub required_mip: u32,
    /// Memory per mip
    pub mip_sizes: [u64; 16],
    /// Priority
    pub priority: f32,
}

impl StreamingTextureInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        let total_mips = ((width.max(height) as f32).log2() as u32 + 1).min(16);
        Self {
            asset: StreamingAssetHandle::NULL,
            full_resolution: [width, height],
            total_mips,
            resident_mips: 1,
            required_mip: total_mips,
            mip_sizes: [0; 16],
            priority: 0.0,
        }
    }

    /// Resident memory
    pub fn resident_memory(&self) -> u64 {
        let start = self.total_mips.saturating_sub(self.resident_mips);
        self.mip_sizes[start as usize..self.total_mips as usize]
            .iter()
            .sum()
    }

    /// Memory needed for target mip
    pub fn memory_for_mip(&self, mip: u32) -> u64 {
        self.mip_sizes[mip.min(self.total_mips - 1) as usize]
    }

    /// Is fully resident
    pub fn is_fully_resident(&self) -> bool {
        self.resident_mips >= self.total_mips
    }
}

impl Default for StreamingTextureInfo {
    fn default() -> Self {
        Self::new(1024, 1024)
    }
}

/// Mip feedback data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MipFeedback {
    /// Texture ID
    pub texture_id: u32,
    /// Required mip level
    pub required_mip: u32,
    /// Screen area (pixels)
    pub screen_area: u32,
    /// Flags
    pub flags: u32,
}

// ============================================================================
// Mesh Streaming
// ============================================================================

/// Mesh streaming settings
#[derive(Clone, Debug)]
pub struct MeshStreamingSettings {
    /// Min LOD always resident
    pub min_resident_lod: u32,
    /// Target LOD latency (frames)
    pub lod_latency: u32,
    /// Cluster size
    pub cluster_size: u32,
}

impl MeshStreamingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            min_resident_lod: 2,
            lod_latency: 5,
            cluster_size: 256,
        }
    }
}

impl Default for MeshStreamingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming mesh info
#[derive(Clone, Debug)]
pub struct StreamingMeshInfo {
    /// Asset handle
    pub asset: StreamingAssetHandle,
    /// LOD levels
    pub lod_count: u32,
    /// Resident LODs (bitmask)
    pub resident_lods: u32,
    /// Required LOD
    pub required_lod: u32,
    /// Memory per LOD
    pub lod_sizes: [u64; 8],
    /// Priority
    pub priority: f32,
}

impl StreamingMeshInfo {
    /// Creates info
    pub fn new(lod_count: u32) -> Self {
        Self {
            asset: StreamingAssetHandle::NULL,
            lod_count: lod_count.min(8),
            resident_lods: 1, // LOD 0 only
            required_lod: 0,
            lod_sizes: [0; 8],
            priority: 0.0,
        }
    }

    /// Is LOD resident
    pub fn is_lod_resident(&self, lod: u32) -> bool {
        (self.resident_lods & (1 << lod)) != 0
    }

    /// Resident memory
    pub fn resident_memory(&self) -> u64 {
        let mut total = 0u64;
        for i in 0..self.lod_count {
            if self.is_lod_resident(i) {
                total += self.lod_sizes[i as usize];
            }
        }
        total
    }
}

impl Default for StreamingMeshInfo {
    fn default() -> Self {
        Self::new(4)
    }
}

// ============================================================================
// Stream Request
// ============================================================================

/// Stream request
#[derive(Clone, Debug)]
pub struct StreamRequest {
    /// Request handle
    pub handle: StreamRequestHandle,
    /// Asset
    pub asset: StreamingAssetHandle,
    /// Request type
    pub request_type: StreamRequestType,
    /// Priority
    pub priority: f32,
    /// Target level (mip or LOD)
    pub target_level: u32,
    /// Data offset
    pub data_offset: u64,
    /// Data size
    pub data_size: u64,
    /// State
    pub state: StreamRequestState,
}

impl StreamRequest {
    /// Creates request
    pub fn new(asset: StreamingAssetHandle, request_type: StreamRequestType) -> Self {
        Self {
            handle: StreamRequestHandle::NULL,
            asset,
            request_type,
            priority: 1.0,
            target_level: 0,
            data_offset: 0,
            data_size: 0,
            state: StreamRequestState::Pending,
        }
    }

    /// Load mip request
    pub fn load_mip(asset: StreamingAssetHandle, mip: u32, priority: f32) -> Self {
        Self {
            request_type: StreamRequestType::LoadMip,
            target_level: mip,
            priority,
            ..Self::new(asset, StreamRequestType::LoadMip)
        }
    }

    /// Load LOD request
    pub fn load_lod(asset: StreamingAssetHandle, lod: u32, priority: f32) -> Self {
        Self {
            request_type: StreamRequestType::LoadLod,
            target_level: lod,
            priority,
            ..Self::new(asset, StreamRequestType::LoadLod)
        }
    }

    /// Is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.state, StreamRequestState::Complete)
    }

    /// Is failed
    pub fn is_failed(&self) -> bool {
        matches!(self.state, StreamRequestState::Failed)
    }
}

/// Stream request type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StreamRequestType {
    /// Load mip level
    #[default]
    LoadMip = 0,
    /// Unload mip level
    UnloadMip = 1,
    /// Load LOD
    LoadLod = 2,
    /// Unload LOD
    UnloadLod = 3,
    /// Load chunk
    LoadChunk = 4,
    /// Prefetch
    Prefetch = 5,
}

/// Stream request state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StreamRequestState {
    /// Pending
    #[default]
    Pending = 0,
    /// Queued
    Queued = 1,
    /// Loading
    Loading = 2,
    /// Uploading
    Uploading = 3,
    /// Complete
    Complete = 4,
    /// Failed
    Failed = 5,
    /// Cancelled
    Cancelled = 6,
}

// ============================================================================
// Priority System
// ============================================================================

/// Priority calculator
#[derive(Clone, Debug)]
pub struct PriorityCalculator {
    /// Camera position
    pub camera_position: [f32; 3],
    /// Camera direction
    pub camera_direction: [f32; 3],
    /// Distance weight
    pub distance_weight: f32,
    /// Screen size weight
    pub screen_size_weight: f32,
    /// Visibility weight
    pub visibility_weight: f32,
    /// Time weight (recently used)
    pub time_weight: f32,
}

impl PriorityCalculator {
    /// Creates calculator
    pub fn new() -> Self {
        Self {
            camera_position: [0.0, 0.0, 0.0],
            camera_direction: [0.0, 0.0, 1.0],
            distance_weight: 1.0,
            screen_size_weight: 2.0,
            visibility_weight: 3.0,
            time_weight: 0.5,
        }
    }

    /// Update camera
    pub fn update_camera(&mut self, position: [f32; 3], direction: [f32; 3]) {
        self.camera_position = position;
        self.camera_direction = direction;
    }

    /// Calculate priority
    pub fn calculate(
        &self,
        object_position: [f32; 3],
        screen_coverage: f32,
        is_visible: bool,
        last_used_frame: u64,
        current_frame: u64,
    ) -> f32 {
        // Distance factor
        let dx = object_position[0] - self.camera_position[0];
        let dy = object_position[1] - self.camera_position[1];
        let dz = object_position[2] - self.camera_position[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();
        let distance_factor = 1.0 / (1.0 + distance * 0.01);

        // Screen size factor
        let screen_factor = screen_coverage;

        // Visibility factor
        let visibility_factor = if is_visible { 1.0 } else { 0.2 };

        // Time factor
        let frames_since_used = current_frame.saturating_sub(last_used_frame);
        let time_factor = 1.0 / (1.0 + frames_since_used as f32 * 0.01);

        // Combine
        distance_factor * self.distance_weight
            + screen_factor * self.screen_size_weight
            + visibility_factor * self.visibility_weight
            + time_factor * self.time_weight
    }
}

impl Default for PriorityCalculator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Streaming Manager
// ============================================================================

/// Streaming budget
#[derive(Clone, Debug)]
pub struct StreamingBudget {
    /// Total budget (bytes)
    pub total: u64,
    /// Used (bytes)
    pub used: u64,
    /// Reserved (bytes)
    pub reserved: u64,
    /// Per-type budgets
    pub type_budgets: [u64; 5],
    /// Per-type usage
    pub type_usage: [u64; 5],
}

impl StreamingBudget {
    /// Creates budget
    pub fn new(total: u64) -> Self {
        Self {
            total,
            used: 0,
            reserved: 0,
            type_budgets: [
                total * 60 / 100, // Textures (60%)
                total * 30 / 100, // Meshes (30%)
                total * 5 / 100,  // Audio (5%)
                total * 3 / 100,  // Animation (3%)
                total * 2 / 100,  // Generic (2%)
            ],
            type_usage: [0; 5],
        }
    }

    /// Available memory
    pub fn available(&self) -> u64 {
        self.total.saturating_sub(self.used + self.reserved)
    }

    /// Available for type
    pub fn available_for_type(&self, pool_type: StreamingPoolType) -> u64 {
        let idx = pool_type as usize;
        self.type_budgets[idx].saturating_sub(self.type_usage[idx])
    }

    /// Usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.used as f64 / self.total as f64 * 100.0) as f32
        }
    }
}

impl Default for StreamingBudget {
    fn default() -> Self {
        Self::new(512 * 1024 * 1024)
    }
}

/// Streaming queue
#[derive(Clone, Debug)]
pub struct StreamingQueue {
    /// Pending requests
    pub pending: Vec<StreamRequest>,
    /// Active requests
    pub active: Vec<StreamRequest>,
    /// Max concurrent
    pub max_concurrent: u32,
}

impl StreamingQueue {
    /// Creates queue
    pub fn new(max_concurrent: u32) -> Self {
        Self {
            pending: Vec::new(),
            active: Vec::new(),
            max_concurrent,
        }
    }

    /// Enqueue request
    pub fn enqueue(&mut self, request: StreamRequest) {
        self.pending.push(request);
    }

    /// Sort by priority
    pub fn sort_by_priority(&mut self) {
        self.pending.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
    }

    /// Can start more
    pub fn can_start_more(&self) -> bool {
        (self.active.len() as u32) < self.max_concurrent && !self.pending.is_empty()
    }

    /// Pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Active count
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

impl Default for StreamingQueue {
    fn default() -> Self {
        Self::new(8)
    }
}

// ============================================================================
// I/O
// ============================================================================

/// Streaming I/O settings
#[derive(Clone, Debug)]
pub struct StreamingIoSettings {
    /// Read buffer size
    pub read_buffer_size: u32,
    /// Max read size per frame
    pub max_read_per_frame: u64,
    /// Use async I/O
    pub async_io: bool,
    /// Compression
    pub compression: StreamingCompression,
}

impl StreamingIoSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            read_buffer_size: 256 * 1024, // 256KB
            max_read_per_frame: 16 * 1024 * 1024, // 16MB
            async_io: true,
            compression: StreamingCompression::Lz4,
        }
    }
}

impl Default for StreamingIoSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Streaming compression
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StreamingCompression {
    /// None
    None = 0,
    /// LZ4
    #[default]
    Lz4 = 1,
    /// Zstd
    Zstd = 2,
    /// Oodle
    Oodle = 3,
}

// ============================================================================
// Statistics
// ============================================================================

/// Streaming statistics
#[derive(Clone, Debug, Default)]
pub struct StreamingStats {
    /// Requests completed this frame
    pub completed_requests: u32,
    /// Requests pending
    pub pending_requests: u32,
    /// Requests active
    pub active_requests: u32,
    /// Bytes loaded this frame
    pub bytes_loaded: u64,
    /// Bytes evicted this frame
    pub bytes_evicted: u64,
    /// Memory used
    pub memory_used: u64,
    /// Memory budget
    pub memory_budget: u64,
    /// Textures streaming
    pub textures_streaming: u32,
    /// Meshes streaming
    pub meshes_streaming: u32,
    /// IO bandwidth (bytes/second)
    pub io_bandwidth: u64,
}

impl StreamingStats {
    /// Memory usage percentage
    pub fn memory_usage_percent(&self) -> f32 {
        if self.memory_budget == 0 {
            0.0
        } else {
            (self.memory_used as f64 / self.memory_budget as f64 * 100.0) as f32
        }
    }
}
