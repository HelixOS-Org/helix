//! Asset Loading and Management Types for Lumina
//!
//! This module provides asset loading, caching, and management infrastructure
//! for textures, meshes, materials, shaders, and other graphics resources.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Asset Handles
// ============================================================================

/// Asset handle (type-erased)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AssetHandle(pub u64);

impl AssetHandle {
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

    /// Gets raw id
    #[inline]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for AssetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Typed asset handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypedAssetHandle<T> {
    /// Inner handle
    pub handle: AssetHandle,
    /// Phantom data
    _marker: core::marker::PhantomData<T>,
}

impl<T> TypedAssetHandle<T> {
    /// Null handle
    pub const NULL: Self = Self {
        handle: AssetHandle::NULL,
        _marker: core::marker::PhantomData,
    };

    /// Creates new handle
    pub const fn new(id: u64) -> Self {
        Self {
            handle: AssetHandle::new(id),
            _marker: core::marker::PhantomData,
        }
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.handle.is_null()
    }

    /// Gets raw id
    #[inline]
    pub const fn raw(&self) -> u64 {
        self.handle.raw()
    }
}

impl<T> Default for TypedAssetHandle<T> {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Asset Types
// ============================================================================

/// Asset type enumeration
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AssetType {
    /// Unknown type
    Unknown = 0,
    /// Texture
    Texture = 1,
    /// Mesh
    Mesh = 2,
    /// Material
    Material = 3,
    /// Shader
    Shader = 4,
    /// Pipeline
    Pipeline = 5,
    /// Buffer
    Buffer = 6,
    /// Sampler
    Sampler = 7,
    /// RenderTarget
    RenderTarget = 8,
    /// AnimationClip
    AnimationClip = 9,
    /// Skeleton
    Skeleton = 10,
    /// Font
    Font = 11,
    /// Audio
    Audio = 12,
    /// Scene
    Scene = 13,
    /// Prefab
    Prefab = 14,
}

impl AssetType {
    /// Gets file extensions for this type
    pub const fn extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Texture => &["png", "jpg", "jpeg", "bmp", "tga", "dds", "ktx", "ktx2"],
            Self::Mesh => &["obj", "gltf", "glb", "fbx"],
            Self::Material => &["mat", "material"],
            Self::Shader => &["vert", "frag", "comp", "glsl", "hlsl", "spv"],
            Self::AnimationClip => &["anim"],
            Self::Skeleton => &["skel"],
            Self::Font => &["ttf", "otf"],
            Self::Audio => &["wav", "ogg", "mp3"],
            Self::Scene => &["scene"],
            Self::Prefab => &["prefab"],
            _ => &[],
        }
    }

    /// From extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" | "jpg" | "jpeg" | "bmp" | "tga" | "dds" | "ktx" | "ktx2" => Self::Texture,
            "obj" | "gltf" | "glb" | "fbx" => Self::Mesh,
            "mat" | "material" => Self::Material,
            "vert" | "frag" | "comp" | "glsl" | "hlsl" | "spv" => Self::Shader,
            "anim" => Self::AnimationClip,
            "skel" => Self::Skeleton,
            "ttf" | "otf" => Self::Font,
            "wav" | "ogg" | "mp3" => Self::Audio,
            "scene" => Self::Scene,
            "prefab" => Self::Prefab,
            _ => Self::Unknown,
        }
    }
}

// ============================================================================
// Asset State
// ============================================================================

/// Asset load state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AssetState {
    /// Not loaded
    Unloaded = 0,
    /// Currently loading
    Loading = 1,
    /// Loaded successfully
    Loaded = 2,
    /// Failed to load
    Failed = 3,
    /// Unloading
    Unloading = 4,
}

impl AssetState {
    /// Is ready to use
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Loaded)
    }

    /// Is loading
    pub const fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Has failed
    pub const fn has_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

// ============================================================================
// Asset Metadata
// ============================================================================

/// Asset metadata
#[derive(Clone, Debug)]
pub struct AssetMetadata {
    /// Asset path
    pub path: String,
    /// Asset type
    pub asset_type: AssetType,
    /// State
    pub state: AssetState,
    /// Size in bytes
    pub size_bytes: u64,
    /// Reference count
    pub ref_count: u32,
    /// Load timestamp
    pub load_time: u64,
    /// Last access timestamp
    pub last_access: u64,
    /// Dependencies
    pub dependencies: Vec<AssetHandle>,
    /// Custom tags
    pub tags: Vec<String>,
}

impl AssetMetadata {
    /// Creates new metadata
    pub fn new(path: &str, asset_type: AssetType) -> Self {
        Self {
            path: String::from(path),
            asset_type,
            state: AssetState::Unloaded,
            size_bytes: 0,
            ref_count: 0,
            load_time: 0,
            last_access: 0,
            dependencies: Vec::new(),
            tags: Vec::new(),
        }
    }

    /// With size
    pub fn with_size(mut self, size: u64) -> Self {
        self.size_bytes = size;
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(String::from(tag));
        self
    }

    /// Add dependency
    pub fn with_dependency(mut self, dep: AssetHandle) -> Self {
        self.dependencies.push(dep);
        self
    }
}

// ============================================================================
// Asset Load Request
// ============================================================================

/// Asset load request
#[derive(Clone, Debug)]
pub struct AssetLoadRequest {
    /// Path to load
    pub path: String,
    /// Asset type (optional, can be inferred)
    pub asset_type: Option<AssetType>,
    /// Load priority
    pub priority: LoadPriority,
    /// Load flags
    pub flags: LoadFlags,
    /// Streaming settings
    pub streaming: Option<StreamingSettings>,
}

impl AssetLoadRequest {
    /// Creates new request
    pub fn new(path: &str) -> Self {
        Self {
            path: String::from(path),
            asset_type: None,
            priority: LoadPriority::Normal,
            flags: LoadFlags::DEFAULT,
            streaming: None,
        }
    }

    /// With type
    pub fn with_type(mut self, asset_type: AssetType) -> Self {
        self.asset_type = Some(asset_type);
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: LoadPriority) -> Self {
        self.priority = priority;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: LoadFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Synchronous load
    pub fn sync(mut self) -> Self {
        self.flags = self.flags.union(LoadFlags::SYNCHRONOUS);
        self
    }

    /// Streamed load
    pub fn streamed(mut self, settings: StreamingSettings) -> Self {
        self.streaming = Some(settings);
        self
    }
}

/// Load priority
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LoadPriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    #[default]
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical (load immediately)
    Critical = 3,
}

/// Load flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct LoadFlags(pub u32);

impl LoadFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Synchronous load
    pub const SYNCHRONOUS: Self = Self(1 << 0);
    /// Keep in cache
    pub const PERSISTENT: Self = Self(1 << 1);
    /// Generate mipmaps (textures)
    pub const GENERATE_MIPMAPS: Self = Self(1 << 2);
    /// Compress (textures)
    pub const COMPRESS: Self = Self(1 << 3);
    /// Skip validation
    pub const SKIP_VALIDATION: Self = Self(1 << 4);
    /// Hot reload enabled
    pub const HOT_RELOAD: Self = Self(1 << 5);
    /// Default flags
    pub const DEFAULT: Self = Self::NONE;

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Streaming settings
#[derive(Clone, Debug)]
pub struct StreamingSettings {
    /// Minimum mip level to load first
    pub min_mip_level: u32,
    /// Priority distance
    pub priority_distance: f32,
    /// Resident mip level
    pub resident_mip: u32,
}

impl StreamingSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            min_mip_level: 4,
            priority_distance: 100.0,
            resident_mip: 0,
        }
    }

    /// With min mip
    pub fn with_min_mip(mut self, mip: u32) -> Self {
        self.min_mip_level = mip;
        self
    }
}

impl Default for StreamingSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Asset Load Result
// ============================================================================

/// Asset load result
#[derive(Clone, Debug)]
pub enum AssetLoadResult {
    /// Success
    Success(AssetHandle),
    /// Pending (async load in progress)
    Pending(AssetHandle),
    /// Failed
    Failed(AssetError),
}

impl AssetLoadResult {
    /// Is success
    pub const fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }

    /// Is pending
    pub const fn is_pending(&self) -> bool {
        matches!(self, Self::Pending(_))
    }

    /// Is failed
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// Gets handle if available
    pub fn handle(&self) -> Option<AssetHandle> {
        match self {
            Self::Success(h) | Self::Pending(h) => Some(*h),
            Self::Failed(_) => None,
        }
    }

    /// Gets error if failed
    pub fn error(&self) -> Option<&AssetError> {
        match self {
            Self::Failed(e) => Some(e),
            _ => None,
        }
    }
}

/// Asset error
#[derive(Clone, Debug)]
pub struct AssetError {
    /// Error kind
    pub kind: AssetErrorKind,
    /// Error message
    pub message: String,
    /// Asset path
    pub path: Option<String>,
}

impl AssetError {
    /// Creates new error
    pub fn new(kind: AssetErrorKind, message: &str) -> Self {
        Self {
            kind,
            message: String::from(message),
            path: None,
        }
    }

    /// Not found error
    pub fn not_found(path: &str) -> Self {
        Self {
            kind: AssetErrorKind::NotFound,
            message: String::from("Asset not found"),
            path: Some(String::from(path)),
        }
    }

    /// Parse error
    pub fn parse_error(path: &str, message: &str) -> Self {
        Self {
            kind: AssetErrorKind::ParseError,
            message: String::from(message),
            path: Some(String::from(path)),
        }
    }

    /// With path
    pub fn with_path(mut self, path: &str) -> Self {
        self.path = Some(String::from(path));
        self
    }
}

/// Asset error kind
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AssetErrorKind {
    /// Asset not found
    NotFound = 0,
    /// IO error
    IoError = 1,
    /// Parse error
    ParseError = 2,
    /// Invalid format
    InvalidFormat = 3,
    /// Unsupported format
    UnsupportedFormat = 4,
    /// Validation error
    ValidationError = 5,
    /// Dependency error
    DependencyError = 6,
    /// Out of memory
    OutOfMemory = 7,
    /// GPU error
    GpuError = 8,
    /// Timeout
    Timeout = 9,
    /// Cancelled
    Cancelled = 10,
}

// ============================================================================
// Asset Cache
// ============================================================================

/// Asset cache configuration
#[derive(Clone, Debug)]
pub struct AssetCacheConfig {
    /// Maximum memory budget (bytes)
    pub max_memory: u64,
    /// Maximum asset count
    pub max_assets: u32,
    /// Eviction policy
    pub eviction_policy: EvictionPolicy,
    /// Enable compression
    pub compression: bool,
    /// Streaming enabled
    pub streaming: bool,
}

impl AssetCacheConfig {
    /// Creates default config
    pub fn new() -> Self {
        Self {
            max_memory: 512 * 1024 * 1024, // 512 MB
            max_assets: 10_000,
            eviction_policy: EvictionPolicy::Lru,
            compression: false,
            streaming: true,
        }
    }

    /// With memory budget
    pub fn with_memory(mut self, bytes: u64) -> Self {
        self.max_memory = bytes;
        self
    }

    /// With max assets
    pub fn with_max_assets(mut self, count: u32) -> Self {
        self.max_assets = count;
        self
    }

    /// With eviction policy
    pub fn with_eviction(mut self, policy: EvictionPolicy) -> Self {
        self.eviction_policy = policy;
        self
    }
}

impl Default for AssetCacheConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Eviction policy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum EvictionPolicy {
    /// Least recently used
    #[default]
    Lru = 0,
    /// Least frequently used
    Lfu = 1,
    /// First in first out
    Fifo = 2,
    /// Manual only
    Manual = 3,
}

/// Asset cache statistics
#[derive(Clone, Debug, Default)]
pub struct AssetCacheStats {
    /// Total assets loaded
    pub total_assets: u32,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Assets evicted
    pub evictions: u64,
    /// Load operations
    pub loads: u64,
    /// Failed loads
    pub failures: u64,
}

impl AssetCacheStats {
    /// Hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }

    /// Memory usage percent
    pub fn memory_usage(&self, max_memory: u64) -> f32 {
        if max_memory == 0 {
            0.0
        } else {
            self.memory_used as f32 / max_memory as f32
        }
    }
}

// ============================================================================
// Asset Bundle
// ============================================================================

/// Asset bundle (collection of assets)
#[derive(Clone, Debug)]
pub struct AssetBundle {
    /// Bundle name
    pub name: String,
    /// Assets in bundle
    pub assets: Vec<AssetHandle>,
    /// Bundle state
    pub state: AssetState,
    /// Total size
    pub size_bytes: u64,
}

impl AssetBundle {
    /// Creates new bundle
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            assets: Vec::new(),
            state: AssetState::Unloaded,
            size_bytes: 0,
        }
    }

    /// Add asset
    pub fn add(&mut self, asset: AssetHandle) {
        self.assets.push(asset);
    }

    /// Asset count
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }
}

// ============================================================================
// Asset Importer
// ============================================================================

/// Asset importer trait
pub trait AssetImporter: Send + Sync {
    /// Supported extensions
    fn extensions(&self) -> &[&str];

    /// Asset type
    fn asset_type(&self) -> AssetType;

    /// Import asset from bytes
    fn import(&self, data: &[u8], settings: &ImportSettings) -> Result<ImportedAsset, AssetError>;
}

/// Import settings
#[derive(Clone, Debug)]
pub struct ImportSettings {
    /// Source path
    pub path: String,
    /// Import flags
    pub flags: ImportFlags,
    /// Texture settings
    pub texture: TextureImportSettings,
    /// Mesh settings
    pub mesh: MeshImportSettings,
}

impl ImportSettings {
    /// Creates default settings
    pub fn new(path: &str) -> Self {
        Self {
            path: String::from(path),
            flags: ImportFlags::DEFAULT,
            texture: TextureImportSettings::default(),
            mesh: MeshImportSettings::default(),
        }
    }
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self::new("")
    }
}

/// Import flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ImportFlags(pub u32);

impl ImportFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Optimize
    pub const OPTIMIZE: Self = Self(1 << 0);
    /// Generate tangents
    pub const GENERATE_TANGENTS: Self = Self(1 << 1);
    /// Generate normals
    pub const GENERATE_NORMALS: Self = Self(1 << 2);
    /// Flip UVs
    pub const FLIP_UVS: Self = Self(1 << 3);
    /// Convert to left-handed
    pub const LEFT_HANDED: Self = Self(1 << 4);
    /// Default
    pub const DEFAULT: Self = Self(Self::OPTIMIZE.0 | Self::GENERATE_TANGENTS.0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Texture import settings
#[derive(Clone, Debug)]
pub struct TextureImportSettings {
    /// Generate mipmaps
    pub generate_mipmaps: bool,
    /// Compress texture
    pub compress: bool,
    /// Compression quality
    pub compression_quality: CompressionQuality,
    /// sRGB
    pub srgb: bool,
    /// Max size
    pub max_size: u32,
    /// Force power of two
    pub power_of_two: bool,
}

impl TextureImportSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            generate_mipmaps: true,
            compress: true,
            compression_quality: CompressionQuality::Normal,
            srgb: true,
            max_size: 4096,
            power_of_two: false,
        }
    }

    /// For normal map
    pub fn normal_map() -> Self {
        Self {
            srgb: false,
            compress: false,
            ..Self::new()
        }
    }

    /// For HDR
    pub fn hdr() -> Self {
        Self {
            srgb: false,
            compress: false,
            ..Self::new()
        }
    }
}

impl Default for TextureImportSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompressionQuality {
    /// Fast compression
    Fast = 0,
    /// Normal compression
    #[default]
    Normal = 1,
    /// High quality
    High = 2,
    /// Best quality
    Best = 3,
}

/// Mesh import settings
#[derive(Clone, Debug)]
pub struct MeshImportSettings {
    /// Scale factor
    pub scale: f32,
    /// Generate tangents
    pub generate_tangents: bool,
    /// Generate normals
    pub generate_normals: bool,
    /// Optimize mesh
    pub optimize: bool,
    /// Merge meshes
    pub merge_meshes: bool,
    /// Import materials
    pub import_materials: bool,
    /// Import animations
    pub import_animations: bool,
    /// Import skeleton
    pub import_skeleton: bool,
}

impl MeshImportSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            generate_tangents: true,
            generate_normals: true,
            optimize: true,
            merge_meshes: false,
            import_materials: true,
            import_animations: true,
            import_skeleton: true,
        }
    }
}

impl Default for MeshImportSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Imported asset
#[derive(Clone, Debug)]
pub struct ImportedAsset {
    /// Asset type
    pub asset_type: AssetType,
    /// Asset data
    pub data: AssetData,
    /// Sub-assets
    pub sub_assets: Vec<ImportedAsset>,
    /// Metadata
    pub metadata: AssetMetadata,
}

/// Asset data (type-specific)
#[derive(Clone, Debug)]
pub enum AssetData {
    /// Texture data
    Texture(TextureData),
    /// Mesh data
    Mesh(MeshData),
    /// Shader data
    Shader(ShaderData),
    /// Raw bytes
    Raw(Vec<u8>),
}

/// Texture data
#[derive(Clone, Debug)]
pub struct TextureData {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Array layers
    pub array_layers: u32,
    /// Format
    pub format: TextureDataFormat,
    /// Pixel data (per mip, per layer)
    pub pixels: Vec<Vec<u8>>,
}

/// Texture data format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextureDataFormat {
    /// R8
    R8 = 0,
    /// RG8
    Rg8 = 1,
    /// RGB8
    Rgb8 = 2,
    /// RGBA8
    #[default]
    Rgba8 = 3,
    /// R16F
    R16Float = 4,
    /// RG16F
    Rg16Float = 5,
    /// RGBA16F
    Rgba16Float = 6,
    /// R32F
    R32Float = 7,
    /// RGBA32F
    Rgba32Float = 8,
    /// BC1
    Bc1 = 100,
    /// BC3
    Bc3 = 101,
    /// BC5
    Bc5 = 102,
    /// BC7
    Bc7 = 103,
}

/// Mesh data
#[derive(Clone, Debug)]
pub struct MeshData {
    /// Vertices
    pub vertices: Vec<MeshVertex>,
    /// Indices
    pub indices: Vec<u32>,
    /// Sub-meshes
    pub sub_meshes: Vec<SubMeshData>,
    /// Bounding box min
    pub bounds_min: [f32; 3],
    /// Bounding box max
    pub bounds_max: [f32; 3],
}

/// Mesh vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshVertex {
    /// Position
    pub position: [f32; 3],
    /// Normal
    pub normal: [f32; 3],
    /// Tangent
    pub tangent: [f32; 4],
    /// UV0
    pub uv0: [f32; 2],
    /// UV1
    pub uv1: [f32; 2],
    /// Color
    pub color: [f32; 4],
    /// Bone indices
    pub bone_indices: [u32; 4],
    /// Bone weights
    pub bone_weights: [f32; 4],
}

/// Sub-mesh data
#[derive(Clone, Debug)]
pub struct SubMeshData {
    /// Name
    pub name: String,
    /// Start index
    pub start_index: u32,
    /// Index count
    pub index_count: u32,
    /// Material index
    pub material_index: u32,
}

/// Shader data
#[derive(Clone, Debug)]
pub struct ShaderData {
    /// Stage
    pub stage: ShaderStage,
    /// SPIR-V bytecode
    pub spirv: Vec<u32>,
    /// Entry point
    pub entry_point: String,
    /// Reflection data
    pub reflection: Option<ShaderReflection>,
}

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ShaderStage {
    /// Vertex
    Vertex = 0,
    /// Fragment
    Fragment = 1,
    /// Compute
    Compute = 2,
    /// Geometry
    Geometry = 3,
    /// TessControl
    TessControl = 4,
    /// TessEval
    TessEval = 5,
    /// RayGen
    RayGen = 6,
    /// RayMiss
    RayMiss = 7,
    /// RayClosestHit
    RayClosestHit = 8,
    /// RayAnyHit
    RayAnyHit = 9,
    /// RayIntersection
    RayIntersection = 10,
}

/// Shader reflection data
#[derive(Clone, Debug, Default)]
pub struct ShaderReflection {
    /// Inputs
    pub inputs: Vec<ShaderInput>,
    /// Outputs
    pub outputs: Vec<ShaderOutput>,
    /// Uniforms
    pub uniforms: Vec<ShaderUniform>,
    /// Push constants
    pub push_constants: Vec<ShaderPushConstant>,
}

/// Shader input
#[derive(Clone, Debug)]
pub struct ShaderInput {
    /// Name
    pub name: String,
    /// Location
    pub location: u32,
    /// Type
    pub data_type: ShaderDataType,
}

/// Shader output
#[derive(Clone, Debug)]
pub struct ShaderOutput {
    /// Name
    pub name: String,
    /// Location
    pub location: u32,
    /// Type
    pub data_type: ShaderDataType,
}

/// Shader uniform
#[derive(Clone, Debug)]
pub struct ShaderUniform {
    /// Name
    pub name: String,
    /// Set
    pub set: u32,
    /// Binding
    pub binding: u32,
    /// Type
    pub uniform_type: UniformType,
}

/// Uniform type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum UniformType {
    /// Uniform buffer
    UniformBuffer = 0,
    /// Storage buffer
    StorageBuffer = 1,
    /// Sampler
    Sampler = 2,
    /// Sampled image
    SampledImage = 3,
    /// Storage image
    StorageImage = 4,
    /// Combined image sampler
    CombinedImageSampler = 5,
    /// Input attachment
    InputAttachment = 6,
    /// Acceleration structure
    AccelerationStructure = 7,
}

/// Shader push constant
#[derive(Clone, Debug)]
pub struct ShaderPushConstant {
    /// Name
    pub name: String,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
}

/// Shader data type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ShaderDataType {
    /// Float
    Float = 0,
    /// Vec2
    Vec2 = 1,
    /// Vec3
    Vec3 = 2,
    /// Vec4
    Vec4 = 3,
    /// Int
    Int = 4,
    /// IVec2
    IVec2 = 5,
    /// IVec3
    IVec3 = 6,
    /// IVec4
    IVec4 = 7,
    /// UInt
    UInt = 8,
    /// UVec2
    UVec2 = 9,
    /// UVec3
    UVec3 = 10,
    /// UVec4
    UVec4 = 11,
    /// Mat3
    Mat3 = 12,
    /// Mat4
    Mat4 = 13,
}

// ============================================================================
// Asset Reference
// ============================================================================

/// Weak asset reference
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WeakAssetRef {
    /// Handle
    pub handle: AssetHandle,
    /// Generation
    pub generation: u32,
}

impl WeakAssetRef {
    /// Creates new weak ref
    pub const fn new(handle: AssetHandle, generation: u32) -> Self {
        Self { handle, generation }
    }

    /// Is valid
    pub fn is_valid(&self, current_gen: u32) -> bool {
        !self.handle.is_null() && self.generation == current_gen
    }
}

impl Default for WeakAssetRef {
    fn default() -> Self {
        Self {
            handle: AssetHandle::NULL,
            generation: 0,
        }
    }
}
