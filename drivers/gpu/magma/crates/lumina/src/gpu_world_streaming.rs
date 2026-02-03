//! GPU World Streaming System for Lumina
//!
//! This module provides GPU-accelerated world streaming including
//! chunk loading, terrain paging, and seamless level transitions.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// World Streaming Handles
// ============================================================================

/// GPU world streaming handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuWorldStreamingHandle(pub u64);

impl GpuWorldStreamingHandle {
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

impl Default for GpuWorldStreamingHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// World chunk handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct WorldChunkHandle(pub u64);

impl WorldChunkHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for WorldChunkHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Stream layer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StreamLayerHandle(pub u64);

impl StreamLayerHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StreamLayerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Stream region handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StreamRegionHandle(pub u64);

impl StreamRegionHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StreamRegionHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// World Streaming Creation
// ============================================================================

/// GPU world streaming create info
#[derive(Clone, Debug)]
pub struct GpuWorldStreamingCreateInfo {
    /// Name
    pub name: String,
    /// Max chunks
    pub max_chunks: u32,
    /// Max layers
    pub max_layers: u32,
    /// Max regions
    pub max_regions: u32,
    /// Chunk size
    pub chunk_size: f32,
    /// Features
    pub features: StreamingFeatures,
    /// Memory budget (bytes)
    pub memory_budget: u64,
}

impl GpuWorldStreamingCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_chunks: 1024,
            max_layers: 16,
            max_regions: 64,
            chunk_size: 256.0,
            features: StreamingFeatures::all(),
            memory_budget: 512 * 1024 * 1024, // 512MB
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max chunks
    pub fn with_max_chunks(mut self, count: u32) -> Self {
        self.max_chunks = count;
        self
    }

    /// With chunk size
    pub fn with_chunk_size(mut self, size: f32) -> Self {
        self.chunk_size = size;
        self
    }

    /// With features
    pub fn with_features(mut self, features: StreamingFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With memory budget
    pub fn with_memory_budget(mut self, budget: u64) -> Self {
        self.memory_budget = budget;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// Large world preset
    pub fn large_world() -> Self {
        Self::new()
            .with_max_chunks(4096)
            .with_memory_budget(2 * 1024 * 1024 * 1024) // 2GB
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_max_chunks(256)
            .with_memory_budget(128 * 1024 * 1024) // 128MB
            .with_features(StreamingFeatures::BASIC)
    }
}

impl Default for GpuWorldStreamingCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Streaming features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct StreamingFeatures: u32 {
        /// None
        const NONE = 0;
        /// Async loading
        const ASYNC_LOADING = 1 << 0;
        /// Priority loading
        const PRIORITY = 1 << 1;
        /// Distance-based LOD
        const DISTANCE_LOD = 1 << 2;
        /// Prefetching
        const PREFETCH = 1 << 3;
        /// Memory pooling
        const POOLING = 1 << 4;
        /// Compression
        const COMPRESSION = 1 << 5;
        /// Seamless transitions
        const SEAMLESS = 1 << 6;
        /// GPU decompression
        const GPU_DECOMPRESS = 1 << 7;
        /// Basic features
        const BASIC = Self::ASYNC_LOADING.bits() | Self::DISTANCE_LOD.bits();
        /// All
        const ALL = 0xFF;
    }
}

impl Default for StreamingFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// World Chunk
// ============================================================================

/// World chunk create info
#[derive(Clone, Debug)]
pub struct WorldChunkCreateInfo {
    /// Name
    pub name: String,
    /// Chunk coordinates
    pub coordinates: ChunkCoord,
    /// Chunk type
    pub chunk_type: ChunkType,
    /// Content
    pub content: ChunkContent,
    /// Priority
    pub priority: ChunkPriority,
    /// Dependencies
    pub dependencies: Vec<ChunkDependency>,
}

impl WorldChunkCreateInfo {
    /// Creates new info
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self {
            name: String::new(),
            coordinates: ChunkCoord::new(x, y, z),
            chunk_type: ChunkType::Static,
            content: ChunkContent::default(),
            priority: ChunkPriority::Normal,
            dependencies: Vec::new(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With type
    pub fn with_type(mut self, chunk_type: ChunkType) -> Self {
        self.chunk_type = chunk_type;
        self
    }

    /// With content
    pub fn with_content(mut self, content: ChunkContent) -> Self {
        self.content = content;
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: ChunkPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add dependency
    pub fn with_dependency(mut self, dep: ChunkDependency) -> Self {
        self.dependencies.push(dep);
        self
    }
}

impl Default for WorldChunkCreateInfo {
    fn default() -> Self {
        Self::new(0, 0, 0)
    }
}

/// Chunk coordinates
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ChunkCoord {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
    /// Z coordinate
    pub z: i32,
}

impl ChunkCoord {
    /// Creates new coordinates
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Origin
    pub const fn origin() -> Self {
        Self::new(0, 0, 0)
    }

    /// Distance to other chunk
    pub fn distance(&self, other: &Self) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        let dz = (self.z - other.z) as f32;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Manhattan distance
    pub const fn manhattan_distance(&self, other: &Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }
}

impl Default for ChunkCoord {
    fn default() -> Self {
        Self::origin()
    }
}

/// Chunk type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ChunkType {
    /// Static (never unloads)
    #[default]
    Static     = 0,
    /// Streamable
    Streamable = 1,
    /// Dynamic (runtime generated)
    Dynamic    = 2,
    /// Instanced
    Instanced  = 3,
    /// LOD only
    LodOnly    = 4,
}

/// Chunk priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ChunkPriority {
    /// Critical (always loaded)
    Critical   = 0,
    /// High
    High       = 1,
    /// Normal
    #[default]
    Normal     = 2,
    /// Low
    Low        = 3,
    /// Background
    Background = 4,
}

/// Chunk content
#[derive(Clone, Debug, Default)]
pub struct ChunkContent {
    /// Geometry data
    pub geometry: Option<ChunkGeometry>,
    /// Terrain data
    pub terrain: Option<ChunkTerrain>,
    /// Objects
    pub objects: Vec<ChunkObject>,
    /// Lights
    pub lights: Vec<ChunkLight>,
    /// Collision data
    pub collision: Option<ChunkCollision>,
}

impl ChunkContent {
    /// Creates new content
    pub fn new() -> Self {
        Self::default()
    }

    /// With geometry
    pub fn with_geometry(mut self, geometry: ChunkGeometry) -> Self {
        self.geometry = Some(geometry);
        self
    }

    /// With terrain
    pub fn with_terrain(mut self, terrain: ChunkTerrain) -> Self {
        self.terrain = Some(terrain);
        self
    }

    /// Add object
    pub fn add_object(mut self, object: ChunkObject) -> Self {
        self.objects.push(object);
        self
    }

    /// Add light
    pub fn add_light(mut self, light: ChunkLight) -> Self {
        self.lights.push(light);
        self
    }
}

/// Chunk geometry
#[derive(Clone, Debug, Default)]
pub struct ChunkGeometry {
    /// Mesh handle
    pub mesh: u64,
    /// Material handle
    pub material: u64,
    /// LOD meshes
    pub lod_meshes: Vec<u64>,
}

/// Chunk terrain
#[derive(Clone, Debug)]
pub struct ChunkTerrain {
    /// Heightmap handle
    pub heightmap: u64,
    /// Splat map handle
    pub splat_map: u64,
    /// Resolution
    pub resolution: u32,
}

impl Default for ChunkTerrain {
    fn default() -> Self {
        Self {
            heightmap: 0,
            splat_map: 0,
            resolution: 256,
        }
    }
}

/// Chunk object
#[derive(Clone, Debug)]
pub struct ChunkObject {
    /// Object ID
    pub id: u64,
    /// Asset handle
    pub asset: u64,
    /// Transform
    pub transform: ChunkTransform,
}

/// Chunk light
#[derive(Clone, Debug)]
pub struct ChunkLight {
    /// Light ID
    pub id: u64,
    /// Light type
    pub light_type: ChunkLightType,
    /// Position
    pub position: [f32; 3],
    /// Color
    pub color: [f32; 3],
    /// Intensity
    pub intensity: f32,
    /// Range
    pub range: f32,
}

/// Chunk light type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ChunkLightType {
    /// Point light
    #[default]
    Point = 0,
    /// Spot light
    Spot  = 1,
    /// Area light
    Area  = 2,
}

/// Chunk collision
#[derive(Clone, Debug, Default)]
pub struct ChunkCollision {
    /// Collision mesh handle
    pub mesh: u64,
    /// Simplified collision
    pub simplified: bool,
}

/// Chunk transform
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ChunkTransform {
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
}

impl ChunkTransform {
    /// Identity
    pub const fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// At position
    pub const fn at(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}

impl Default for ChunkTransform {
    fn default() -> Self {
        Self::identity()
    }
}

/// Chunk dependency
#[derive(Clone, Copy, Debug)]
pub struct ChunkDependency {
    /// Dependent chunk
    pub chunk: WorldChunkHandle,
    /// Dependency type
    pub dep_type: DependencyType,
}

impl ChunkDependency {
    /// Creates new dependency
    pub const fn new(chunk: WorldChunkHandle, dep_type: DependencyType) -> Self {
        Self { chunk, dep_type }
    }
}

/// Dependency type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DependencyType {
    /// Hard dependency (must load first)
    #[default]
    Hard      = 0,
    /// Soft dependency (optional)
    Soft      = 1,
    /// Streaming dependency
    Streaming = 2,
}

// ============================================================================
// Stream Layer
// ============================================================================

/// Stream layer create info
#[derive(Clone, Debug)]
pub struct StreamLayerCreateInfo {
    /// Name
    pub name: String,
    /// Layer type
    pub layer_type: StreamLayerType,
    /// Priority
    pub priority: i32,
    /// Stream distance
    pub stream_distance: f32,
    /// Unload distance
    pub unload_distance: f32,
    /// LOD bias
    pub lod_bias: f32,
}

impl StreamLayerCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            layer_type: StreamLayerType::Geometry,
            priority: 0,
            stream_distance: 500.0,
            unload_distance: 600.0,
            lod_bias: 0.0,
        }
    }

    /// With type
    pub fn with_type(mut self, layer_type: StreamLayerType) -> Self {
        self.layer_type = layer_type;
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// With distances
    pub fn with_distances(mut self, stream: f32, unload: f32) -> Self {
        self.stream_distance = stream;
        self.unload_distance = unload;
        self
    }

    /// Geometry layer preset
    pub fn geometry() -> Self {
        Self::new("Geometry")
            .with_type(StreamLayerType::Geometry)
            .with_priority(0)
            .with_distances(500.0, 600.0)
    }

    /// Texture layer preset
    pub fn textures() -> Self {
        Self::new("Textures")
            .with_type(StreamLayerType::Textures)
            .with_priority(1)
            .with_distances(300.0, 400.0)
    }

    /// Collision layer preset
    pub fn collision() -> Self {
        Self::new("Collision")
            .with_type(StreamLayerType::Collision)
            .with_priority(-1)
            .with_distances(100.0, 150.0)
    }

    /// Audio layer preset
    pub fn audio() -> Self {
        Self::new("Audio")
            .with_type(StreamLayerType::Audio)
            .with_priority(2)
            .with_distances(200.0, 250.0)
    }
}

impl Default for StreamLayerCreateInfo {
    fn default() -> Self {
        Self::new("Layer")
    }
}

/// Stream layer type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StreamLayerType {
    /// Geometry
    #[default]
    Geometry  = 0,
    /// Textures
    Textures  = 1,
    /// Collision
    Collision = 2,
    /// Audio
    Audio     = 3,
    /// Gameplay
    Gameplay  = 4,
    /// AI
    Ai        = 5,
    /// Custom
    Custom    = 6,
}

// ============================================================================
// Stream Region
// ============================================================================

/// Stream region create info
#[derive(Clone, Debug)]
pub struct StreamRegionCreateInfo {
    /// Name
    pub name: String,
    /// Bounds
    pub bounds: RegionBounds,
    /// Region type
    pub region_type: RegionType,
    /// Priority override
    pub priority_override: Option<ChunkPriority>,
}

impl StreamRegionCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>, bounds: RegionBounds) -> Self {
        Self {
            name: name.into(),
            bounds,
            region_type: RegionType::Normal,
            priority_override: None,
        }
    }

    /// With type
    pub fn with_type(mut self, region_type: RegionType) -> Self {
        self.region_type = region_type;
        self
    }

    /// With priority override
    pub fn with_priority(mut self, priority: ChunkPriority) -> Self {
        self.priority_override = Some(priority);
        self
    }
}

/// Region bounds
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RegionBounds {
    /// Min corner
    pub min: [f32; 3],
    /// Max corner
    pub max: [f32; 3],
}

impl RegionBounds {
    /// Creates new bounds
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// From center and extents
    pub const fn from_center(center: [f32; 3], extents: [f32; 3]) -> Self {
        Self {
            min: [
                center[0] - extents[0],
                center[1] - extents[1],
                center[2] - extents[2],
            ],
            max: [
                center[0] + extents[0],
                center[1] + extents[1],
                center[2] + extents[2],
            ],
        }
    }

    /// Contains point
    pub fn contains(&self, point: [f32; 3]) -> bool {
        point[0] >= self.min[0]
            && point[0] <= self.max[0]
            && point[1] >= self.min[1]
            && point[1] <= self.max[1]
            && point[2] >= self.min[2]
            && point[2] <= self.max[2]
    }

    /// Center
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Extents
    pub fn extents(&self) -> [f32; 3] {
        [
            (self.max[0] - self.min[0]) * 0.5,
            (self.max[1] - self.min[1]) * 0.5,
            (self.max[2] - self.min[2]) * 0.5,
        ]
    }
}

impl Default for RegionBounds {
    fn default() -> Self {
        Self::new([-100.0; 3], [100.0; 3])
    }
}

/// Region type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum RegionType {
    /// Normal region
    #[default]
    Normal       = 0,
    /// Always loaded
    AlwaysLoaded = 1,
    /// Never unload once loaded
    Persistent   = 2,
    /// High detail region
    HighDetail   = 3,
    /// Low detail region
    LowDetail    = 4,
}

// ============================================================================
// Streaming State
// ============================================================================

/// Chunk state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ChunkState {
    /// Unloaded
    #[default]
    Unloaded      = 0,
    /// Pending load
    PendingLoad   = 1,
    /// Loading
    Loading       = 2,
    /// Loaded
    Loaded        = 3,
    /// Active
    Active        = 4,
    /// Pending unload
    PendingUnload = 5,
    /// Unloading
    Unloading     = 6,
    /// Error
    Error         = 7,
}

/// Stream request
#[derive(Clone, Debug)]
pub struct StreamRequest {
    /// Chunk
    pub chunk: WorldChunkHandle,
    /// Request type
    pub request_type: StreamRequestType,
    /// Priority
    pub priority: ChunkPriority,
    /// Callback ID
    pub callback_id: u64,
}

/// Stream request type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StreamRequestType {
    /// Load
    #[default]
    Load     = 0,
    /// Unload
    Unload   = 1,
    /// Reload
    Reload   = 2,
    /// Prefetch
    Prefetch = 3,
}

/// Stream result
#[derive(Clone, Debug)]
pub struct StreamResult {
    /// Chunk
    pub chunk: WorldChunkHandle,
    /// Success
    pub success: bool,
    /// New state
    pub state: ChunkState,
    /// Load time (ms)
    pub load_time_ms: f32,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Error message
    pub error: Option<String>,
}

// ============================================================================
// Streaming View
// ============================================================================

/// Streaming view
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StreamingView {
    /// View position
    pub position: [f32; 3],
    /// View direction
    pub direction: [f32; 3],
    /// Priority multiplier
    pub priority_multiplier: f32,
    /// Stream distance override
    pub stream_distance: f32,
}

impl StreamingView {
    /// Creates new view
    pub const fn new(position: [f32; 3], direction: [f32; 3]) -> Self {
        Self {
            position,
            direction,
            priority_multiplier: 1.0,
            stream_distance: 0.0,
        }
    }

    /// With priority
    pub const fn with_priority(mut self, multiplier: f32) -> Self {
        self.priority_multiplier = multiplier;
        self
    }

    /// With distance
    pub const fn with_distance(mut self, distance: f32) -> Self {
        self.stream_distance = distance;
        self
    }

    /// Player view
    pub const fn player(position: [f32; 3], direction: [f32; 3]) -> Self {
        Self::new(position, direction).with_priority(1.0)
    }

    /// Camera view
    pub const fn camera(position: [f32; 3], direction: [f32; 3]) -> Self {
        Self::new(position, direction).with_priority(0.8)
    }
}

impl Default for StreamingView {
    fn default() -> Self {
        Self::new([0.0; 3], [0.0, 0.0, 1.0])
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU chunk data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuChunkData {
    /// Chunk bounds min
    pub bounds_min: [f32; 3],
    /// Chunk state
    pub state: u32,
    /// Chunk bounds max
    pub bounds_max: [f32; 3],
    /// LOD level
    pub lod_level: u32,
    /// World offset
    pub world_offset: [f32; 3],
    /// Priority
    pub priority: u32,
    /// Coordinates
    pub coords: [i32; 3],
    /// Flags
    pub flags: u32,
}

/// GPU streaming constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuStreamingConstants {
    /// Camera position
    pub camera_position: [f32; 3],
    /// Time
    pub time: f32,
    /// Chunk size
    pub chunk_size: f32,
    /// Stream distance
    pub stream_distance: f32,
    /// Fade distance
    pub fade_distance: f32,
    /// LOD bias
    pub lod_bias: f32,
    /// Active chunk count
    pub active_chunks: u32,
    /// Total chunks
    pub total_chunks: u32,
    /// Pad
    pub _pad: [u32; 2],
}

impl Default for GpuStreamingConstants {
    fn default() -> Self {
        Self {
            camera_position: [0.0; 3],
            time: 0.0,
            chunk_size: 256.0,
            stream_distance: 500.0,
            fade_distance: 50.0,
            lod_bias: 0.0,
            active_chunks: 0,
            total_chunks: 0,
            _pad: [0; 2],
        }
    }
}

// ============================================================================
// Streaming Statistics
// ============================================================================

/// World streaming statistics
#[derive(Clone, Debug, Default)]
pub struct GpuWorldStreamingStats {
    /// Total chunks
    pub total_chunks: u32,
    /// Loaded chunks
    pub loaded_chunks: u32,
    /// Active chunks
    pub active_chunks: u32,
    /// Pending loads
    pub pending_loads: u32,
    /// Pending unloads
    pub pending_unloads: u32,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Memory budget (bytes)
    pub memory_budget: u64,
    /// Load time this frame (ms)
    pub frame_load_time_ms: f32,
    /// Chunks loaded this frame
    pub chunks_loaded_frame: u32,
    /// Chunks unloaded this frame
    pub chunks_unloaded_frame: u32,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
}

impl GpuWorldStreamingStats {
    /// Memory usage ratio
    pub fn memory_usage_ratio(&self) -> f32 {
        if self.memory_budget > 0 {
            self.memory_used as f32 / self.memory_budget as f32
        } else {
            0.0
        }
    }

    /// Load ratio
    pub fn load_ratio(&self) -> f32 {
        if self.total_chunks > 0 {
            self.loaded_chunks as f32 / self.total_chunks as f32
        } else {
            0.0
        }
    }

    /// Cache hit rate
    pub fn cache_hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hits as f32 / total as f32
        } else {
            0.0
        }
    }

    /// Memory used in MB
    pub fn memory_mb(&self) -> f32 {
        self.memory_used as f32 / (1024.0 * 1024.0)
    }

    /// Memory budget in MB
    pub fn budget_mb(&self) -> f32 {
        self.memory_budget as f32 / (1024.0 * 1024.0)
    }
}

// ============================================================================
// Streaming Events
// ============================================================================

/// Streaming event
#[derive(Clone, Debug)]
pub enum StreamingEvent {
    /// Chunk loaded
    ChunkLoaded {
        /// Chunk handle
        chunk: WorldChunkHandle,
        /// Load time
        load_time_ms: f32,
    },
    /// Chunk unloaded
    ChunkUnloaded {
        /// Chunk handle
        chunk: WorldChunkHandle,
    },
    /// Chunk error
    ChunkError {
        /// Chunk handle
        chunk: WorldChunkHandle,
        /// Error message
        error: String,
    },
    /// Memory pressure
    MemoryPressure {
        /// Usage ratio
        usage_ratio: f32,
    },
    /// Region entered
    RegionEntered {
        /// Region handle
        region: StreamRegionHandle,
    },
    /// Region exited
    RegionExited {
        /// Region handle
        region: StreamRegionHandle,
    },
}

/// Streaming event listener
pub type StreamingEventCallback = fn(StreamingEvent);
