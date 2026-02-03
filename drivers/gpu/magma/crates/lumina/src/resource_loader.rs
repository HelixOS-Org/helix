//! Async Resource Loader Types for Lumina
//!
//! This module provides asynchronous resource loading
//! infrastructure for efficient asset management.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Loader Handles
// ============================================================================

/// Resource loader handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ResourceLoaderHandle(pub u64);

impl ResourceLoaderHandle {
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

impl Default for ResourceLoaderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Load request handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LoadRequestHandle(pub u64);

impl LoadRequestHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LoadRequestHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Loaded resource handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LoadedResourceHandle(pub u64);

impl LoadedResourceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LoadedResourceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Resource bundle handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ResourceBundleHandle(pub u64);

impl ResourceBundleHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ResourceBundleHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Resource Loader Creation
// ============================================================================

/// Resource loader create info
#[derive(Clone, Debug)]
pub struct ResourceLoaderCreateInfo {
    /// Name
    pub name: String,
    /// Max concurrent loads
    pub max_concurrent: u32,
    /// Max pending requests
    pub max_pending: u32,
    /// IO thread count
    pub io_threads: u32,
    /// Staging buffer size
    pub staging_size: u64,
    /// Loading mode
    pub mode: LoadingMode,
    /// Priority mode
    pub priority_mode: PriorityMode,
    /// Features
    pub features: LoaderFeatures,
}

impl ResourceLoaderCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_concurrent: 8,
            max_pending: 256,
            io_threads: 2,
            staging_size: 128 * 1024 * 1024,  // 128MB
            mode: LoadingMode::Async,
            priority_mode: PriorityMode::Fifo,
            features: LoaderFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max concurrent
    pub fn with_concurrent(mut self, max: u32) -> Self {
        self.max_concurrent = max;
        self
    }

    /// With max pending
    pub fn with_pending(mut self, max: u32) -> Self {
        self.max_pending = max;
        self
    }

    /// With IO threads
    pub fn with_io_threads(mut self, count: u32) -> Self {
        self.io_threads = count;
        self
    }

    /// With staging size
    pub fn with_staging_size(mut self, bytes: u64) -> Self {
        self.staging_size = bytes;
        self
    }

    /// With mode
    pub fn with_mode(mut self, mode: LoadingMode) -> Self {
        self.mode = mode;
        self
    }

    /// With priority mode
    pub fn with_priority(mut self, mode: PriorityMode) -> Self {
        self.priority_mode = mode;
        self
    }

    /// With features
    pub fn with_features(mut self, features: LoaderFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// Fast storage preset (NVMe)
    pub fn fast_storage() -> Self {
        Self::new()
            .with_concurrent(16)
            .with_io_threads(4)
            .with_features(LoaderFeatures::DIRECT_STORAGE)
    }

    /// Streaming preset
    pub fn streaming() -> Self {
        Self::new()
            .with_concurrent(4)
            .with_priority(PriorityMode::Priority)
            .with_features(LoaderFeatures::STREAMING | LoaderFeatures::DECOMPRESSION)
    }

    /// Background loading preset
    pub fn background() -> Self {
        Self::new()
            .with_concurrent(2)
            .with_mode(LoadingMode::Background)
    }
}

impl Default for ResourceLoaderCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Loading mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LoadingMode {
    /// Synchronous loading
    Sync = 0,
    /// Asynchronous loading
    #[default]
    Async = 1,
    /// Background loading (low priority)
    Background = 2,
    /// On-demand loading
    OnDemand = 3,
}

/// Priority mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PriorityMode {
    /// First in first out
    #[default]
    Fifo = 0,
    /// Priority based
    Priority = 1,
    /// Round robin
    RoundRobin = 2,
    /// Deadline based
    Deadline = 3,
}

bitflags::bitflags! {
    /// Loader features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct LoaderFeatures: u32 {
        /// None
        const NONE = 0;
        /// Direct storage (DirectStorage/Metal IO)
        const DIRECT_STORAGE = 1 << 0;
        /// Streaming support
        const STREAMING = 1 << 1;
        /// Decompression
        const DECOMPRESSION = 1 << 2;
        /// GPU decompression
        const GPU_DECOMPRESSION = 1 << 3;
        /// Prefetching
        const PREFETCHING = 1 << 4;
        /// Caching
        const CACHING = 1 << 5;
        /// Hot reload
        const HOT_RELOAD = 1 << 6;
        /// Dependency tracking
        const DEPENDENCIES = 1 << 7;
    }
}

// ============================================================================
// Load Requests
// ============================================================================

/// Load request
#[derive(Clone, Debug)]
pub struct LoadRequest {
    /// Resource ID
    pub resource_id: String,
    /// Resource type
    pub resource_type: ResourceType,
    /// Source path
    pub source: ResourceSource,
    /// Priority
    pub priority: LoadPriority,
    /// Flags
    pub flags: LoadFlags,
    /// Callback data
    pub callback_data: u64,
    /// Deadline (frame number)
    pub deadline: Option<u64>,
}

impl LoadRequest {
    /// Creates new request
    pub fn new(resource_type: ResourceType) -> Self {
        Self {
            resource_id: String::new(),
            resource_type,
            source: ResourceSource::File(String::new()),
            priority: LoadPriority::Normal,
            flags: LoadFlags::empty(),
            callback_data: 0,
            deadline: None,
        }
    }

    /// With resource ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.resource_id = id.into();
        self
    }

    /// With source
    pub fn with_source(mut self, source: ResourceSource) -> Self {
        self.source = source;
        self
    }

    /// With file path
    pub fn with_file(mut self, path: impl Into<String>) -> Self {
        self.source = ResourceSource::File(path.into());
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: LoadPriority) -> Self {
        self.priority = priority;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: LoadFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// With callback data
    pub fn with_callback(mut self, data: u64) -> Self {
        self.callback_data = data;
        self
    }

    /// With deadline
    pub fn with_deadline(mut self, frame: u64) -> Self {
        self.deadline = Some(frame);
        self
    }

    /// Texture request
    pub fn texture(path: impl Into<String>) -> Self {
        Self::new(ResourceType::Texture)
            .with_file(path)
    }

    /// Mesh request
    pub fn mesh(path: impl Into<String>) -> Self {
        Self::new(ResourceType::Mesh)
            .with_file(path)
    }

    /// Shader request
    pub fn shader(path: impl Into<String>) -> Self {
        Self::new(ResourceType::Shader)
            .with_file(path)
    }

    /// High priority request
    pub fn high_priority(mut self) -> Self {
        self.priority = LoadPriority::High;
        self
    }

    /// Immediate request
    pub fn immediate(mut self) -> Self {
        self.priority = LoadPriority::Immediate;
        self.flags |= LoadFlags::BLOCKING;
        self
    }
}

impl Default for LoadRequest {
    fn default() -> Self {
        Self::new(ResourceType::Generic)
    }
}

/// Resource type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ResourceType {
    /// Generic
    #[default]
    Generic = 0,
    /// Texture
    Texture = 1,
    /// Mesh
    Mesh = 2,
    /// Shader
    Shader = 3,
    /// Material
    Material = 4,
    /// Animation
    Animation = 5,
    /// Audio
    Audio = 6,
    /// Font
    Font = 7,
    /// Scene
    Scene = 8,
    /// Prefab
    Prefab = 9,
}

impl ResourceType {
    /// Default priority
    pub const fn default_priority(&self) -> LoadPriority {
        match self {
            Self::Shader => LoadPriority::High,
            Self::Texture | Self::Mesh => LoadPriority::Normal,
            Self::Audio | Self::Animation => LoadPriority::Low,
            _ => LoadPriority::Normal,
        }
    }

    /// Is GPU resource
    pub const fn is_gpu_resource(&self) -> bool {
        matches!(self, Self::Texture | Self::Mesh | Self::Shader)
    }
}

/// Resource source
#[derive(Clone, Debug)]
pub enum ResourceSource {
    /// File path
    File(String),
    /// Memory blob
    Memory(Vec<u8>),
    /// Bundle reference
    Bundle { bundle: ResourceBundleHandle, entry: String },
    /// Network URL
    Network(String),
    /// Generated
    Generated,
}

/// Load priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(u32)]
pub enum LoadPriority {
    /// Background (lowest)
    Background = 0,
    /// Low
    Low = 1,
    /// Normal
    #[default]
    Normal = 2,
    /// High
    High = 3,
    /// Immediate (highest)
    Immediate = 4,
}

bitflags::bitflags! {
    /// Load flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct LoadFlags: u32 {
        /// None
        const NONE = 0;
        /// Blocking load
        const BLOCKING = 1 << 0;
        /// Skip cache
        const SKIP_CACHE = 1 << 1;
        /// Force reload
        const FORCE_RELOAD = 1 << 2;
        /// Prefetch only
        const PREFETCH_ONLY = 1 << 3;
        /// Keep in memory
        const KEEP_IN_MEMORY = 1 << 4;
        /// Compress in memory
        const COMPRESS = 1 << 5;
        /// GPU upload
        const GPU_UPLOAD = 1 << 6;
    }
}

// ============================================================================
// Load Status
// ============================================================================

/// Load status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LoadStatus {
    /// Not started
    #[default]
    NotStarted = 0,
    /// Queued
    Queued = 1,
    /// Loading from disk
    Loading = 2,
    /// Processing/parsing
    Processing = 3,
    /// Uploading to GPU
    Uploading = 4,
    /// Completed
    Completed = 5,
    /// Failed
    Failed = 6,
    /// Cancelled
    Cancelled = 7,
}

impl LoadStatus {
    /// Is in progress
    pub const fn is_in_progress(&self) -> bool {
        matches!(self, Self::Queued | Self::Loading | Self::Processing | Self::Uploading)
    }

    /// Is done
    pub const fn is_done(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Is success
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Completed)
    }
}

/// Load progress
#[derive(Clone, Debug, Default)]
pub struct LoadProgress {
    /// Request handle
    pub request: LoadRequestHandle,
    /// Status
    pub status: LoadStatus,
    /// Bytes loaded
    pub bytes_loaded: u64,
    /// Total bytes
    pub total_bytes: u64,
    /// Progress (0.0 - 1.0)
    pub progress: f32,
    /// Error message
    pub error: Option<String>,
    /// Elapsed time (ms)
    pub elapsed_ms: f32,
}

impl LoadProgress {
    /// Creates new progress
    pub fn new(request: LoadRequestHandle) -> Self {
        Self {
            request,
            ..Default::default()
        }
    }

    /// Is complete
    pub fn is_complete(&self) -> bool {
        self.status == LoadStatus::Completed
    }

    /// Is failed
    pub fn is_failed(&self) -> bool {
        self.status == LoadStatus::Failed
    }

    /// Remaining bytes
    pub fn remaining_bytes(&self) -> u64 {
        self.total_bytes.saturating_sub(self.bytes_loaded)
    }

    /// Estimated time remaining (ms)
    pub fn estimated_remaining_ms(&self) -> f32 {
        if self.progress <= 0.0 || self.elapsed_ms <= 0.0 {
            return 0.0;
        }
        let remaining_progress = 1.0 - self.progress;
        (self.elapsed_ms / self.progress) * remaining_progress
    }
}

// ============================================================================
// Loaded Resource
// ============================================================================

/// Loaded resource info
#[derive(Clone, Debug)]
pub struct LoadedResourceInfo {
    /// Handle
    pub handle: LoadedResourceHandle,
    /// Resource ID
    pub resource_id: String,
    /// Resource type
    pub resource_type: ResourceType,
    /// Size (bytes)
    pub size: u64,
    /// GPU size (bytes, if applicable)
    pub gpu_size: u64,
    /// Load time (ms)
    pub load_time_ms: f32,
    /// Reference count
    pub ref_count: u32,
    /// Is loaded
    pub is_loaded: bool,
    /// Is GPU uploaded
    pub is_gpu_uploaded: bool,
}

impl LoadedResourceInfo {
    /// Creates new info
    pub fn new(handle: LoadedResourceHandle) -> Self {
        Self {
            handle,
            resource_id: String::new(),
            resource_type: ResourceType::Generic,
            size: 0,
            gpu_size: 0,
            load_time_ms: 0.0,
            ref_count: 1,
            is_loaded: false,
            is_gpu_uploaded: false,
        }
    }
}

impl Default for LoadedResourceInfo {
    fn default() -> Self {
        Self::new(LoadedResourceHandle::NULL)
    }
}

// ============================================================================
// Resource Bundles
// ============================================================================

/// Resource bundle create info
#[derive(Clone, Debug)]
pub struct ResourceBundleCreateInfo {
    /// Name
    pub name: String,
    /// Bundle path
    pub path: String,
    /// Bundle type
    pub bundle_type: BundleType,
    /// Compression
    pub compression: BundleCompression,
    /// Features
    pub features: BundleFeatures,
}

impl ResourceBundleCreateInfo {
    /// Creates new info
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            name: String::new(),
            path: path.into(),
            bundle_type: BundleType::Archive,
            compression: BundleCompression::None,
            features: BundleFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With bundle type
    pub fn with_type(mut self, bundle_type: BundleType) -> Self {
        self.bundle_type = bundle_type;
        self
    }

    /// With compression
    pub fn with_compression(mut self, compression: BundleCompression) -> Self {
        self.compression = compression;
        self
    }

    /// With features
    pub fn with_features(mut self, features: BundleFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Archive preset
    pub fn archive(path: impl Into<String>) -> Self {
        Self::new(path)
            .with_type(BundleType::Archive)
    }

    /// Packed preset (optimized)
    pub fn packed(path: impl Into<String>) -> Self {
        Self::new(path)
            .with_type(BundleType::Packed)
            .with_compression(BundleCompression::Lz4)
    }
}

impl Default for ResourceBundleCreateInfo {
    fn default() -> Self {
        Self::new("")
    }
}

/// Bundle type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BundleType {
    /// Archive (ZIP-like)
    #[default]
    Archive = 0,
    /// Packed (optimized for streaming)
    Packed = 1,
    /// Directory (loose files)
    Directory = 2,
    /// Memory mapped
    MemoryMapped = 3,
}

/// Bundle compression
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BundleCompression {
    /// None
    #[default]
    None = 0,
    /// LZ4
    Lz4 = 1,
    /// ZSTD
    Zstd = 2,
    /// Per-entry compression
    PerEntry = 3,
}

bitflags::bitflags! {
    /// Bundle features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct BundleFeatures: u32 {
        /// None
        const NONE = 0;
        /// Encryption
        const ENCRYPTION = 1 << 0;
        /// Integrity check
        const INTEGRITY_CHECK = 1 << 1;
        /// Delta patches
        const DELTA_PATCHES = 1 << 2;
        /// Streaming
        const STREAMING = 1 << 3;
    }
}

/// Bundle entry
#[derive(Clone, Debug, Default)]
pub struct BundleEntry {
    /// Entry name
    pub name: String,
    /// Resource type
    pub resource_type: ResourceType,
    /// Offset in bundle
    pub offset: u64,
    /// Compressed size
    pub compressed_size: u64,
    /// Uncompressed size
    pub uncompressed_size: u64,
    /// Hash
    pub hash: u64,
}

/// Bundle info
#[derive(Clone, Debug, Default)]
pub struct ResourceBundleInfo {
    /// Handle
    pub handle: ResourceBundleHandle,
    /// Name
    pub name: String,
    /// Path
    pub path: String,
    /// Bundle type
    pub bundle_type: BundleType,
    /// Entry count
    pub entry_count: u32,
    /// Total size (bytes)
    pub total_size: u64,
    /// Is loaded
    pub is_loaded: bool,
}

// ============================================================================
// Prefetch
// ============================================================================

/// Prefetch request
#[derive(Clone, Debug)]
pub struct PrefetchRequest {
    /// Resource IDs
    pub resources: Vec<String>,
    /// Priority
    pub priority: LoadPriority,
    /// Deadline (frame)
    pub deadline: Option<u64>,
}

impl PrefetchRequest {
    /// Creates new request
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
            priority: LoadPriority::Background,
            deadline: None,
        }
    }

    /// Add resource
    pub fn add_resource(mut self, id: impl Into<String>) -> Self {
        self.resources.push(id.into());
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: LoadPriority) -> Self {
        self.priority = priority;
        self
    }

    /// With deadline
    pub fn with_deadline(mut self, frame: u64) -> Self {
        self.deadline = Some(frame);
        self
    }
}

impl Default for PrefetchRequest {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Resource loader statistics
#[derive(Clone, Debug, Default)]
pub struct ResourceLoaderStats {
    /// Total resources loaded
    pub total_loaded: u32,
    /// Resources in memory
    pub in_memory: u32,
    /// Pending requests
    pub pending_requests: u32,
    /// Active loads
    pub active_loads: u32,
    /// Bytes loaded
    pub bytes_loaded: u64,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// GPU memory used (bytes)
    pub gpu_memory_used: u64,
    /// Average load time (ms)
    pub avg_load_time_ms: f32,
    /// Load bandwidth (bytes/second)
    pub load_bandwidth: f64,
    /// Cache hit rate
    pub cache_hit_rate: f32,
}

impl ResourceLoaderStats {
    /// Total memory
    pub fn total_memory(&self) -> u64 {
        self.memory_used + self.gpu_memory_used
    }

    /// Is loading
    pub fn is_loading(&self) -> bool {
        self.active_loads > 0
    }
}
