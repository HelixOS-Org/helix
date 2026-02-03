//! Shader Cache Types for Lumina
//!
//! This module provides shader caching and compilation
//! infrastructure for efficient shader management.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Cache Handles
// ============================================================================

/// Shader cache handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderCacheHandle(pub u64);

impl ShaderCacheHandle {
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

impl Default for ShaderCacheHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Compiled shader handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CompiledShaderHandle(pub u64);

impl CompiledShaderHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for CompiledShaderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Shader variant handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderVariantHandle(pub u64);

impl ShaderVariantHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ShaderVariantHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Shader blob handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderBlobHandle(pub u64);

impl ShaderBlobHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ShaderBlobHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Shader Cache Creation
// ============================================================================

/// Shader cache create info
#[derive(Clone, Debug)]
pub struct ShaderCacheCreateInfo {
    /// Name
    pub name: String,
    /// Cache path
    pub cache_path: String,
    /// Max cached shaders
    pub max_cached_shaders: u32,
    /// Max memory (bytes)
    pub max_memory: u64,
    /// Cache mode
    pub cache_mode: CacheMode,
    /// Compilation mode
    pub compilation_mode: CompilationMode,
    /// Features
    pub features: ShaderCacheFeatures,
}

impl ShaderCacheCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            cache_path: String::from("/cache/shaders"),
            max_cached_shaders: 4096,
            max_memory: 256 * 1024 * 1024,  // 256MB
            cache_mode: CacheMode::ReadWrite,
            compilation_mode: CompilationMode::Async,
            features: ShaderCacheFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With cache path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.cache_path = path.into();
        self
    }

    /// With max shaders
    pub fn with_max_shaders(mut self, max: u32) -> Self {
        self.max_cached_shaders = max;
        self
    }

    /// With max memory
    pub fn with_max_memory(mut self, bytes: u64) -> Self {
        self.max_memory = bytes;
        self
    }

    /// With cache mode
    pub fn with_cache_mode(mut self, mode: CacheMode) -> Self {
        self.cache_mode = mode;
        self
    }

    /// With compilation mode
    pub fn with_compilation(mut self, mode: CompilationMode) -> Self {
        self.compilation_mode = mode;
        self
    }

    /// With features
    pub fn with_features(mut self, features: ShaderCacheFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// Large cache preset
    pub fn large() -> Self {
        Self::new()
            .with_max_shaders(16384)
            .with_max_memory(1024 * 1024 * 1024)
    }

    /// Memory only preset (no disk)
    pub fn memory_only() -> Self {
        Self::new()
            .with_cache_mode(CacheMode::MemoryOnly)
    }

    /// Read only preset
    pub fn read_only() -> Self {
        Self::new()
            .with_cache_mode(CacheMode::ReadOnly)
    }
}

impl Default for ShaderCacheCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CacheMode {
    /// Read and write
    #[default]
    ReadWrite = 0,
    /// Read only
    ReadOnly = 1,
    /// Write only (regenerate)
    WriteOnly = 2,
    /// Memory only (no disk)
    MemoryOnly = 3,
    /// Disabled
    Disabled = 4,
}

/// Compilation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompilationMode {
    /// Synchronous
    Sync = 0,
    /// Asynchronous
    #[default]
    Async = 1,
    /// Background (low priority)
    Background = 2,
    /// On demand
    OnDemand = 3,
}

bitflags::bitflags! {
    /// Shader cache features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ShaderCacheFeatures: u32 {
        /// None
        const NONE = 0;
        /// Hash validation
        const HASH_VALIDATION = 1 << 0;
        /// Compression
        const COMPRESSION = 1 << 1;
        /// Binary cache
        const BINARY_CACHE = 1 << 2;
        /// Source cache
        const SOURCE_CACHE = 1 << 3;
        /// Pipeline cache integration
        const PIPELINE_CACHE = 1 << 4;
        /// Precompilation
        const PRECOMPILATION = 1 << 5;
        /// Hot reload
        const HOT_RELOAD = 1 << 6;
    }
}

// ============================================================================
// Shader Compilation
// ============================================================================

/// Shader compile request
#[derive(Clone, Debug)]
pub struct ShaderCompileRequest {
    /// Shader name
    pub name: String,
    /// Source code or path
    pub source: ShaderSource,
    /// Stage
    pub stage: ShaderStage,
    /// Entry point
    pub entry_point: String,
    /// Target profile
    pub target: ShaderTarget,
    /// Defines
    pub defines: Vec<ShaderDefine>,
    /// Include paths
    pub include_paths: Vec<String>,
    /// Compile flags
    pub flags: CompileFlags,
    /// Optimization level
    pub optimization: OptimizationLevel,
}

impl ShaderCompileRequest {
    /// Creates new request
    pub fn new(stage: ShaderStage) -> Self {
        Self {
            name: String::new(),
            source: ShaderSource::Embedded(Vec::new()),
            stage,
            entry_point: String::from("main"),
            target: ShaderTarget::SpirV,
            defines: Vec::new(),
            include_paths: Vec::new(),
            flags: CompileFlags::empty(),
            optimization: OptimizationLevel::Performance,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With source
    pub fn with_source(mut self, source: ShaderSource) -> Self {
        self.source = source;
        self
    }

    /// With entry point
    pub fn with_entry(mut self, entry: impl Into<String>) -> Self {
        self.entry_point = entry.into();
        self
    }

    /// With target
    pub fn with_target(mut self, target: ShaderTarget) -> Self {
        self.target = target;
        self
    }

    /// Add define
    pub fn add_define(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.defines.push(ShaderDefine {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// Add include path
    pub fn add_include(mut self, path: impl Into<String>) -> Self {
        self.include_paths.push(path.into());
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: CompileFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// With optimization
    pub fn with_optimization(mut self, level: OptimizationLevel) -> Self {
        self.optimization = level;
        self
    }

    /// Vertex shader preset
    pub fn vertex() -> Self {
        Self::new(ShaderStage::Vertex)
    }

    /// Fragment shader preset
    pub fn fragment() -> Self {
        Self::new(ShaderStage::Fragment)
    }

    /// Compute shader preset
    pub fn compute() -> Self {
        Self::new(ShaderStage::Compute)
    }

    /// Debug preset
    pub fn debug(stage: ShaderStage) -> Self {
        Self::new(stage)
            .with_optimization(OptimizationLevel::None)
            .with_flags(CompileFlags::DEBUG_INFO | CompileFlags::SKIP_VALIDATION)
    }
}

impl Default for ShaderCompileRequest {
    fn default() -> Self {
        Self::vertex()
    }
}

/// Shader source
#[derive(Clone, Debug)]
pub enum ShaderSource {
    /// File path
    File(String),
    /// Embedded source
    Embedded(Vec<u8>),
    /// SPIR-V binary
    SpirV(Vec<u32>),
}

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderStage {
    /// Vertex shader
    #[default]
    Vertex = 0,
    /// Fragment shader
    Fragment = 1,
    /// Compute shader
    Compute = 2,
    /// Geometry shader
    Geometry = 3,
    /// Tessellation control
    TessControl = 4,
    /// Tessellation evaluation
    TessEval = 5,
    /// Task shader (mesh shading)
    Task = 6,
    /// Mesh shader
    Mesh = 7,
    /// Ray generation
    RayGen = 8,
    /// Any hit
    AnyHit = 9,
    /// Closest hit
    ClosestHit = 10,
    /// Miss
    Miss = 11,
    /// Intersection
    Intersection = 12,
    /// Callable
    Callable = 13,
}

impl ShaderStage {
    /// Stage name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::Fragment => "fragment",
            Self::Compute => "compute",
            Self::Geometry => "geometry",
            Self::TessControl => "tesscontrol",
            Self::TessEval => "tesseval",
            Self::Task => "task",
            Self::Mesh => "mesh",
            Self::RayGen => "raygen",
            Self::AnyHit => "anyhit",
            Self::ClosestHit => "closesthit",
            Self::Miss => "miss",
            Self::Intersection => "intersection",
            Self::Callable => "callable",
        }
    }

    /// Is ray tracing stage
    pub const fn is_raytracing(&self) -> bool {
        matches!(
            self,
            Self::RayGen | Self::AnyHit | Self::ClosestHit | Self::Miss | Self::Intersection | Self::Callable
        )
    }

    /// Is mesh shading stage
    pub const fn is_mesh_shading(&self) -> bool {
        matches!(self, Self::Task | Self::Mesh)
    }
}

/// Shader target
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderTarget {
    /// SPIR-V
    #[default]
    SpirV = 0,
    /// DXIL
    Dxil = 1,
    /// Metal
    Metal = 2,
    /// GLSL
    Glsl = 3,
    /// HLSL
    Hlsl = 4,
    /// WGSL
    Wgsl = 5,
}

/// Shader define
#[derive(Clone, Debug, Default)]
pub struct ShaderDefine {
    /// Name
    pub name: String,
    /// Value
    pub value: String,
}

impl ShaderDefine {
    /// Creates new define
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Flag define (no value)
    pub fn flag(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: String::from("1"),
        }
    }
}

bitflags::bitflags! {
    /// Compile flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct CompileFlags: u32 {
        /// None
        const NONE = 0;
        /// Debug info
        const DEBUG_INFO = 1 << 0;
        /// Skip validation
        const SKIP_VALIDATION = 1 << 1;
        /// Strict mode
        const STRICT = 1 << 2;
        /// Matrix row major
        const ROW_MAJOR = 1 << 3;
        /// 16-bit types
        const ENABLE_16BIT = 1 << 4;
        /// Bindless
        const BINDLESS = 1 << 5;
        /// Warnings as errors
        const WARNINGS_AS_ERRORS = 1 << 6;
    }
}

/// Optimization level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OptimizationLevel {
    /// No optimization
    None = 0,
    /// Minimal optimization
    Minimal = 1,
    /// Size optimization
    Size = 2,
    /// Performance optimization
    #[default]
    Performance = 3,
    /// Aggressive optimization
    Aggressive = 4,
}

// ============================================================================
// Compiled Shader
// ============================================================================

/// Compiled shader info
#[derive(Clone, Debug)]
pub struct CompiledShaderInfo {
    /// Handle
    pub handle: CompiledShaderHandle,
    /// Name
    pub name: String,
    /// Stage
    pub stage: ShaderStage,
    /// Target
    pub target: ShaderTarget,
    /// Binary size (bytes)
    pub binary_size: u64,
    /// Compile time (ms)
    pub compile_time_ms: f32,
    /// Hash
    pub hash: u64,
    /// Entry point
    pub entry_point: String,
    /// Is cached
    pub is_cached: bool,
    /// Reflection data
    pub reflection: Option<ShaderReflection>,
}

impl CompiledShaderInfo {
    /// Creates new info
    pub fn new(handle: CompiledShaderHandle, stage: ShaderStage) -> Self {
        Self {
            handle,
            name: String::new(),
            stage,
            target: ShaderTarget::SpirV,
            binary_size: 0,
            compile_time_ms: 0.0,
            hash: 0,
            entry_point: String::from("main"),
            is_cached: false,
            reflection: None,
        }
    }
}

impl Default for CompiledShaderInfo {
    fn default() -> Self {
        Self::new(CompiledShaderHandle::NULL, ShaderStage::Vertex)
    }
}

/// Shader reflection
#[derive(Clone, Debug, Default)]
pub struct ShaderReflection {
    /// Input variables
    pub inputs: Vec<ShaderVariable>,
    /// Output variables
    pub outputs: Vec<ShaderVariable>,
    /// Uniform buffers
    pub uniform_buffers: Vec<ShaderBuffer>,
    /// Storage buffers
    pub storage_buffers: Vec<ShaderBuffer>,
    /// Samplers
    pub samplers: Vec<ShaderSampler>,
    /// Images
    pub images: Vec<ShaderImage>,
    /// Push constants
    pub push_constants: Option<PushConstantRange>,
    /// Workgroup size (compute)
    pub workgroup_size: [u32; 3],
}

/// Shader variable
#[derive(Clone, Debug, Default)]
pub struct ShaderVariable {
    /// Name
    pub name: String,
    /// Location
    pub location: u32,
    /// Type
    pub var_type: VariableType,
    /// Array size (0 for non-array)
    pub array_size: u32,
}

/// Variable type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VariableType {
    /// Float
    #[default]
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

/// Shader buffer
#[derive(Clone, Debug, Default)]
pub struct ShaderBuffer {
    /// Name
    pub name: String,
    /// Set
    pub set: u32,
    /// Binding
    pub binding: u32,
    /// Size (bytes)
    pub size: u64,
    /// Members
    pub members: Vec<BufferMember>,
}

/// Buffer member
#[derive(Clone, Debug, Default)]
pub struct BufferMember {
    /// Name
    pub name: String,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
    /// Type
    pub member_type: VariableType,
}

/// Shader sampler
#[derive(Clone, Debug, Default)]
pub struct ShaderSampler {
    /// Name
    pub name: String,
    /// Set
    pub set: u32,
    /// Binding
    pub binding: u32,
}

/// Shader image
#[derive(Clone, Debug, Default)]
pub struct ShaderImage {
    /// Name
    pub name: String,
    /// Set
    pub set: u32,
    /// Binding
    pub binding: u32,
    /// Dimension
    pub dimension: ImageDimension,
    /// Format
    pub format: ImageFormat,
    /// Array size
    pub array_size: u32,
}

/// Image dimension
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageDimension {
    /// 1D
    Dim1D = 0,
    /// 2D
    #[default]
    Dim2D = 1,
    /// 3D
    Dim3D = 2,
    /// Cube
    Cube = 3,
    /// 2D array
    Array2D = 4,
    /// Cube array
    CubeArray = 5,
}

/// Image format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageFormat {
    /// Unknown
    #[default]
    Unknown = 0,
    /// RGBA32F
    Rgba32F = 1,
    /// RGBA16F
    Rgba16F = 2,
    /// RGBA8
    Rgba8 = 3,
    /// R32F
    R32F = 4,
    /// R32UI
    R32UI = 5,
}

/// Push constant range
#[derive(Clone, Debug, Default)]
pub struct PushConstantRange {
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
    /// Stage flags
    pub stages: u32,
}

// ============================================================================
// Cache Entry
// ============================================================================

/// Shader cache entry
#[derive(Clone, Debug)]
pub struct ShaderCacheEntry {
    /// Hash
    pub hash: u64,
    /// Shader handle
    pub shader: CompiledShaderHandle,
    /// Stage
    pub stage: ShaderStage,
    /// Target
    pub target: ShaderTarget,
    /// Binary size
    pub binary_size: u64,
    /// Last accessed
    pub last_accessed: u64,
    /// Access count
    pub access_count: u32,
    /// Is compressed
    pub is_compressed: bool,
}

impl ShaderCacheEntry {
    /// Creates new entry
    pub fn new(hash: u64, shader: CompiledShaderHandle) -> Self {
        Self {
            hash,
            shader,
            stage: ShaderStage::Vertex,
            target: ShaderTarget::SpirV,
            binary_size: 0,
            last_accessed: 0,
            access_count: 1,
            is_compressed: false,
        }
    }
}

impl Default for ShaderCacheEntry {
    fn default() -> Self {
        Self::new(0, CompiledShaderHandle::NULL)
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Shader cache statistics
#[derive(Clone, Debug, Default)]
pub struct ShaderCacheStats {
    /// Total shaders
    pub total_shaders: u32,
    /// Cached shaders
    pub cached_shaders: u32,
    /// Cache hits
    pub cache_hits: u32,
    /// Cache misses
    pub cache_misses: u32,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Disk used (bytes)
    pub disk_used: u64,
    /// Compile time total (ms)
    pub compile_time_ms: f32,
    /// Pending compilations
    pub pending_compilations: u32,
}

impl ShaderCacheStats {
    /// Cache hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.cache_hits as f32 / total as f32
    }

    /// Average compile time
    pub fn average_compile_time(&self) -> f32 {
        if self.total_shaders == 0 {
            return 0.0;
        }
        self.compile_time_ms / self.total_shaders as f32
    }
}
