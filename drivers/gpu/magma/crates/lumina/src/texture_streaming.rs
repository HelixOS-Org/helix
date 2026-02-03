//! Texture Streaming Types for Lumina
//!
//! This module provides texture streaming infrastructure
//! for dynamic texture loading and memory management.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Streaming Handles
// ============================================================================

/// Texture streaming system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextureStreamingHandle(pub u64);

impl TextureStreamingHandle {
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

impl Default for TextureStreamingHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Streaming texture handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StreamingTextureHandle(pub u64);

impl StreamingTextureHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StreamingTextureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Streaming request handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StreamingRequestHandle(pub u64);

impl StreamingRequestHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StreamingRequestHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Mip chain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MipChainHandle(pub u64);

impl MipChainHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for MipChainHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Streaming System Creation
// ============================================================================

/// Texture streaming system create info
#[derive(Clone, Debug)]
pub struct TextureStreamingCreateInfo {
    /// Name
    pub name: String,
    /// Memory budget (bytes)
    pub memory_budget: u64,
    /// Max concurrent loads
    pub max_concurrent_loads: u32,
    /// Max textures
    pub max_textures: u32,
    /// Streaming mode
    pub mode: StreamingMode,
    /// Priority calculation
    pub priority_mode: PriorityMode,
    /// IO settings
    pub io_settings: IoSettings,
    /// Features
    pub features: StreamingFeatures,
}

impl TextureStreamingCreateInfo {
    /// Creates new info
    pub fn new(memory_budget: u64) -> Self {
        Self {
            name: String::new(),
            memory_budget,
            max_concurrent_loads: 8,
            max_textures: 4096,
            mode: StreamingMode::Async,
            priority_mode: PriorityMode::DistanceBased,
            io_settings: IoSettings::default(),
            features: StreamingFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max concurrent loads
    pub fn with_concurrent(mut self, max: u32) -> Self {
        self.max_concurrent_loads = max;
        self
    }

    /// With max textures
    pub fn with_max_textures(mut self, max: u32) -> Self {
        self.max_textures = max;
        self
    }

    /// With mode
    pub fn with_mode(mut self, mode: StreamingMode) -> Self {
        self.mode = mode;
        self
    }

    /// With priority mode
    pub fn with_priority(mut self, mode: PriorityMode) -> Self {
        self.priority_mode = mode;
        self
    }

    /// With features
    pub fn with_features(mut self, features: StreamingFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard preset (1GB budget)
    pub fn standard() -> Self {
        Self::new(1024 * 1024 * 1024).with_concurrent(8)
    }

    /// Low memory preset (256MB)
    pub fn low_memory() -> Self {
        Self::new(256 * 1024 * 1024).with_concurrent(4)
    }

    /// High memory preset (4GB)
    pub fn high_memory() -> Self {
        Self::new(4 * 1024 * 1024 * 1024).with_concurrent(16)
    }

    /// Fast storage preset (NVMe)
    pub fn fast_storage() -> Self {
        Self::standard()
            .with_concurrent(16)
            .with_features(StreamingFeatures::DIRECT_STORAGE)
    }
}

impl Default for TextureStreamingCreateInfo {
    fn default() -> Self {
        Self::standard()
    }
}

/// Streaming mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StreamingMode {
    /// Synchronous loading
    Sync       = 0,
    /// Asynchronous loading
    #[default]
    Async      = 1,
    /// On-demand (load when accessed)
    OnDemand   = 2,
    /// Predictive (prefetch based on camera)
    Predictive = 3,
}

/// Priority calculation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PriorityMode {
    /// Distance-based priority
    #[default]
    DistanceBased   = 0,
    /// Screen coverage based
    ScreenCoverage  = 1,
    /// Access frequency based
    AccessFrequency = 2,
    /// Combined
    Combined        = 3,
    /// Manual priority
    Manual          = 4,
}

bitflags::bitflags! {
    /// Streaming features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct StreamingFeatures: u32 {
        /// None
        const NONE = 0;
        /// Direct storage API (DirectStorage/Metal IO)
        const DIRECT_STORAGE = 1 << 0;
        /// GPU decompression
        const GPU_DECOMPRESSION = 1 << 1;
        /// Sparse textures
        const SPARSE_TEXTURES = 1 << 2;
        /// Virtual texturing
        const VIRTUAL_TEXTURING = 1 << 3;
        /// Feedback buffer
        const FEEDBACK_BUFFER = 1 << 4;
        /// Transcoding
        const TRANSCODING = 1 << 5;
        /// LRU eviction
        const LRU_EVICTION = 1 << 6;
    }
}

// ============================================================================
// IO Settings
// ============================================================================

/// IO settings
#[derive(Clone, Debug)]
pub struct IoSettings {
    /// Read buffer size
    pub buffer_size: u64,
    /// Max IO requests in flight
    pub max_in_flight: u32,
    /// Use memory-mapped IO
    pub memory_mapped: bool,
    /// Compression format
    pub compression: CompressionFormat,
}

impl IoSettings {
    /// Creates new settings
    pub fn new() -> Self {
        Self {
            buffer_size: 64 * 1024 * 1024,
            max_in_flight: 16,
            memory_mapped: true,
            compression: CompressionFormat::None,
        }
    }

    /// With buffer size
    pub fn with_buffer_size(mut self, size: u64) -> Self {
        self.buffer_size = size;
        self
    }

    /// With compression
    pub fn with_compression(mut self, format: CompressionFormat) -> Self {
        self.compression = format;
        self
    }
}

impl Default for IoSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompressionFormat {
    /// No compression
    #[default]
    None     = 0,
    /// LZ4 compression
    Lz4      = 1,
    /// ZSTD compression
    Zstd     = 2,
    /// GDeflate (GPU-friendly)
    GDeflate = 3,
}

// ============================================================================
// Streaming Texture
// ============================================================================

/// Streaming texture create info
#[derive(Clone, Debug)]
pub struct StreamingTextureCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Format
    pub format: TextureFormat,
    /// Resident mip level (always loaded)
    pub resident_mip: u32,
    /// Source path
    pub source_path: String,
    /// Priority
    pub priority: f32,
    /// Flags
    pub flags: StreamingTextureFlags,
}

impl StreamingTextureCreateInfo {
    /// Creates new info
    pub fn new(width: u32, height: u32, mip_levels: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            mip_levels,
            format: TextureFormat::Rgba8,
            resident_mip: mip_levels.saturating_sub(4), // Keep top 4 mips resident
            source_path: String::new(),
            priority: 1.0,
            flags: StreamingTextureFlags::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With format
    pub fn with_format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }

    /// With source path
    pub fn with_source(mut self, path: impl Into<String>) -> Self {
        self.source_path = path.into();
        self
    }

    /// With resident mip
    pub fn with_resident_mip(mut self, mip: u32) -> Self {
        self.resident_mip = mip;
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: StreamingTextureFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// Total mip size estimate (bytes)
    pub fn total_size(&self) -> u64 {
        let bpp = self.format.bytes_per_pixel();
        let mut total = 0u64;
        let mut w = self.width;
        let mut h = self.height;

        for _ in 0..self.mip_levels {
            total += (w as u64 * h as u64 * bpp as u64);
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }

        total
    }

    /// Resident mip size (bytes)
    pub fn resident_size(&self) -> u64 {
        let bpp = self.format.bytes_per_pixel();
        let mut total = 0u64;
        let mut w = self.width >> self.resident_mip;
        let mut h = self.height >> self.resident_mip;

        for _ in self.resident_mip..self.mip_levels {
            total += (w as u64 * h as u64 * bpp as u64);
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }

        total
    }
}

impl Default for StreamingTextureCreateInfo {
    fn default() -> Self {
        Self::new(1024, 1024, 10)
    }
}

/// Texture format (simplified)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextureFormat {
    /// RGBA8
    #[default]
    Rgba8   = 0,
    /// RGBA16F
    Rgba16F = 1,
    /// BC1 (DXT1)
    Bc1     = 2,
    /// BC3 (DXT5)
    Bc3     = 3,
    /// BC5 (normal maps)
    Bc5     = 4,
    /// BC7
    Bc7     = 5,
    /// ASTC 4x4
    Astc4x4 = 6,
    /// ASTC 8x8
    Astc8x8 = 7,
}

impl TextureFormat {
    /// Bytes per pixel (or per block for compressed)
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Rgba8 => 4,
            Self::Rgba16F => 8,
            Self::Bc1 => 1,                         // 8 bytes per 4x4 block = 0.5 bpp
            Self::Bc3 | Self::Bc5 | Self::Bc7 => 1, // 16 bytes per 4x4 block = 1 bpp
            Self::Astc4x4 => 1,
            Self::Astc8x8 => 1,
        }
    }

    /// Is block compressed
    pub const fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::Bc1 | Self::Bc3 | Self::Bc5 | Self::Bc7 | Self::Astc4x4 | Self::Astc8x8
        )
    }
}

bitflags::bitflags! {
    /// Streaming texture flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct StreamingTextureFlags: u32 {
        /// None
        const NONE = 0;
        /// High priority
        const HIGH_PRIORITY = 1 << 0;
        /// Keep fully resident
        const FULLY_RESIDENT = 1 << 1;
        /// Can be evicted
        const EVICTABLE = 1 << 2;
        /// Preload
        const PRELOAD = 1 << 3;
        /// UI texture (always highest priority)
        const UI = 1 << 4;
    }
}

// ============================================================================
// Streaming State
// ============================================================================

/// Texture streaming state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StreamingState {
    /// Not loaded
    #[default]
    NotLoaded       = 0,
    /// Partially loaded (some mips)
    PartiallyLoaded = 1,
    /// Fully loaded
    FullyLoaded     = 2,
    /// Loading
    Loading         = 3,
    /// Evicting
    Evicting        = 4,
    /// Error
    Error           = 5,
}

/// Mip level state
#[derive(Clone, Copy, Debug, Default)]
pub struct MipLevelState {
    /// Mip level
    pub level: u32,
    /// Is loaded
    pub loaded: bool,
    /// Is loading
    pub loading: bool,
    /// Size (bytes)
    pub size: u64,
    /// Load time (milliseconds)
    pub load_time_ms: f32,
}

/// Texture streaming info
#[derive(Clone, Debug, Default)]
pub struct TextureStreamingInfo {
    /// Texture handle
    pub texture: StreamingTextureHandle,
    /// Current state
    pub state: StreamingState,
    /// Current loaded mip
    pub loaded_mip: u32,
    /// Target mip
    pub target_mip: u32,
    /// Resident mip
    pub resident_mip: u32,
    /// Total mips
    pub total_mips: u32,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Priority
    pub priority: f32,
    /// Last access frame
    pub last_access_frame: u64,
}

impl TextureStreamingInfo {
    /// Is fully loaded
    pub fn is_fully_loaded(&self) -> bool {
        self.loaded_mip == 0
    }

    /// Needs streaming
    pub fn needs_streaming(&self) -> bool {
        self.loaded_mip > self.target_mip
    }

    /// Load progress (0.0 - 1.0)
    pub fn load_progress(&self) -> f32 {
        if self.total_mips == 0 {
            return 1.0;
        }
        1.0 - (self.loaded_mip as f32 / self.total_mips as f32)
    }
}

// ============================================================================
// Streaming Requests
// ============================================================================

/// Mip load request
#[derive(Clone, Debug)]
pub struct MipLoadRequest {
    /// Texture
    pub texture: StreamingTextureHandle,
    /// Target mip level
    pub target_mip: u32,
    /// Priority
    pub priority: f32,
    /// Deadline (frame number)
    pub deadline: Option<u64>,
}

impl MipLoadRequest {
    /// Creates new request
    pub fn new(texture: StreamingTextureHandle, target_mip: u32) -> Self {
        Self {
            texture,
            target_mip,
            priority: 1.0,
            deadline: None,
        }
    }

    /// With priority
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }

    /// With deadline
    pub fn with_deadline(mut self, frame: u64) -> Self {
        self.deadline = Some(frame);
        self
    }

    /// Immediate request
    pub fn immediate(texture: StreamingTextureHandle) -> Self {
        Self::new(texture, 0).with_priority(f32::MAX)
    }
}

/// Eviction request
#[derive(Clone, Debug)]
pub struct EvictionRequest {
    /// Texture
    pub texture: StreamingTextureHandle,
    /// Target mip level (evict to this level)
    pub target_mip: u32,
    /// Force eviction
    pub force: bool,
}

impl EvictionRequest {
    /// Creates new request
    pub fn new(texture: StreamingTextureHandle, target_mip: u32) -> Self {
        Self {
            texture,
            target_mip,
            force: false,
        }
    }

    /// Force eviction
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Evict to resident only
    pub fn to_resident(texture: StreamingTextureHandle, resident_mip: u32) -> Self {
        Self::new(texture, resident_mip)
    }
}

// ============================================================================
// Feedback
// ============================================================================

/// Streaming feedback data (from GPU)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct StreamingFeedback {
    /// Texture ID
    pub texture_id: u32,
    /// Requested mip level
    pub requested_mip: u32,
    /// Screen coverage
    pub screen_coverage: f32,
    /// Frame index
    pub frame_index: u32,
}

/// Feedback buffer info
#[derive(Clone, Debug)]
pub struct FeedbackBufferInfo {
    /// Buffer handle
    pub buffer: u64,
    /// Max entries
    pub max_entries: u32,
    /// Current entry count
    pub entry_count: u32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Texture streaming statistics
#[derive(Clone, Debug, Default)]
pub struct TextureStreamingStats {
    /// Total textures registered
    pub total_textures: u32,
    /// Fully loaded textures
    pub fully_loaded: u32,
    /// Partially loaded textures
    pub partially_loaded: u32,
    /// Textures loading
    pub loading: u32,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Memory budget (bytes)
    pub memory_budget: u64,
    /// Pending requests
    pub pending_requests: u32,
    /// Active loads
    pub active_loads: u32,
    /// Bytes loaded this frame
    pub bytes_loaded_this_frame: u64,
    /// Bytes evicted this frame
    pub bytes_evicted_this_frame: u64,
    /// Load bandwidth (bytes/second)
    pub load_bandwidth: f64,
    /// Cache hit rate
    pub cache_hit_rate: f32,
}

impl TextureStreamingStats {
    /// Memory usage ratio
    pub fn memory_usage_ratio(&self) -> f32 {
        if self.memory_budget == 0 {
            return 0.0;
        }
        self.memory_used as f32 / self.memory_budget as f32
    }

    /// Load ratio
    pub fn load_ratio(&self) -> f32 {
        if self.total_textures == 0 {
            return 0.0;
        }
        self.fully_loaded as f32 / self.total_textures as f32
    }

    /// Is at budget
    pub fn is_at_budget(&self) -> bool {
        self.memory_used >= self.memory_budget
    }
}
