//! Shader Compiler Types for Lumina
//!
//! This module provides shader compilation infrastructure, SPIR-V handling,
//! reflection data, and shader module management.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Shader Module Handle
// ============================================================================

/// Shader module handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderModuleHandle(pub u64);

impl ShaderModuleHandle {
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

impl Default for ShaderModuleHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Shader Module Create Info
// ============================================================================

/// Shader module create info
#[derive(Clone, Debug)]
pub struct ShaderModuleCreateInfo {
    /// SPIR-V bytecode
    pub code: Vec<u32>,
    /// Entry point (for validation)
    pub entry_point: Option<String>,
    /// Shader stage
    pub stage: ShaderStageFlags,
    /// Debug name
    pub debug_name: Option<String>,
}

impl ShaderModuleCreateInfo {
    /// Creates from SPIR-V words
    pub fn from_spirv(code: Vec<u32>) -> Self {
        Self {
            code,
            entry_point: None,
            stage: ShaderStageFlags::ALL,
            debug_name: None,
        }
    }

    /// Creates from SPIR-V bytes
    pub fn from_spirv_bytes(bytes: &[u8]) -> Result<Self, ShaderCompileError> {
        if bytes.len() % 4 != 0 {
            return Err(ShaderCompileError::InvalidSpirv("SPIR-V size not aligned to 4 bytes".into()));
        }

        let code: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        // Validate SPIR-V magic number
        if code.first() != Some(&0x07230203) {
            return Err(ShaderCompileError::InvalidSpirv("Invalid SPIR-V magic number".into()));
        }

        Ok(Self::from_spirv(code))
    }

    /// With entry point
    pub fn with_entry_point(mut self, entry: &str) -> Self {
        self.entry_point = Some(String::from(entry));
        self
    }

    /// With stage
    pub fn with_stage(mut self, stage: ShaderStageFlags) -> Self {
        self.stage = stage;
        self
    }

    /// With debug name
    pub fn with_name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }
}

impl Default for ShaderModuleCreateInfo {
    fn default() -> Self {
        Self {
            code: Vec::new(),
            entry_point: None,
            stage: ShaderStageFlags::ALL,
            debug_name: None,
        }
    }
}

// ============================================================================
// Shader Stage Flags
// ============================================================================

/// Shader stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ShaderStageFlags(pub u32);

impl ShaderStageFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Vertex
    pub const VERTEX: Self = Self(0x00000001);
    /// Tessellation control
    pub const TESSELLATION_CONTROL: Self = Self(0x00000002);
    /// Tessellation evaluation
    pub const TESSELLATION_EVALUATION: Self = Self(0x00000004);
    /// Geometry
    pub const GEOMETRY: Self = Self(0x00000008);
    /// Fragment
    pub const FRAGMENT: Self = Self(0x00000010);
    /// Compute
    pub const COMPUTE: Self = Self(0x00000020);
    /// All graphics
    pub const ALL_GRAPHICS: Self = Self(0x0000001F);
    /// All
    pub const ALL: Self = Self(0x7FFFFFFF);
    /// Task
    pub const TASK: Self = Self(0x00000040);
    /// Mesh
    pub const MESH: Self = Self(0x00000080);
    /// Ray generation
    pub const RAYGEN: Self = Self(0x00000100);
    /// Any hit
    pub const ANY_HIT: Self = Self(0x00000200);
    /// Closest hit
    pub const CLOSEST_HIT: Self = Self(0x00000400);
    /// Miss
    pub const MISS: Self = Self(0x00000800);
    /// Intersection
    pub const INTERSECTION: Self = Self(0x00001000);
    /// Callable
    pub const CALLABLE: Self = Self(0x00002000);

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

    /// To single stage (if only one)
    pub const fn to_single_stage(&self) -> Option<ShaderStage> {
        match self.0 {
            0x00000001 => Some(ShaderStage::Vertex),
            0x00000002 => Some(ShaderStage::TessellationControl),
            0x00000004 => Some(ShaderStage::TessellationEvaluation),
            0x00000008 => Some(ShaderStage::Geometry),
            0x00000010 => Some(ShaderStage::Fragment),
            0x00000020 => Some(ShaderStage::Compute),
            0x00000040 => Some(ShaderStage::Task),
            0x00000080 => Some(ShaderStage::Mesh),
            0x00000100 => Some(ShaderStage::RayGen),
            0x00000200 => Some(ShaderStage::AnyHit),
            0x00000400 => Some(ShaderStage::ClosestHit),
            0x00000800 => Some(ShaderStage::Miss),
            0x00001000 => Some(ShaderStage::Intersection),
            0x00002000 => Some(ShaderStage::Callable),
            _ => None,
        }
    }
}

/// Shader stage (single)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ShaderStage {
    /// Vertex
    Vertex = 0x00000001,
    /// Tessellation control
    TessellationControl = 0x00000002,
    /// Tessellation evaluation
    TessellationEvaluation = 0x00000004,
    /// Geometry
    Geometry = 0x00000008,
    /// Fragment
    Fragment = 0x00000010,
    /// Compute
    Compute = 0x00000020,
    /// Task
    Task = 0x00000040,
    /// Mesh
    Mesh = 0x00000080,
    /// Ray generation
    RayGen = 0x00000100,
    /// Any hit
    AnyHit = 0x00000200,
    /// Closest hit
    ClosestHit = 0x00000400,
    /// Miss
    Miss = 0x00000800,
    /// Intersection
    Intersection = 0x00001000,
    /// Callable
    Callable = 0x00002000,
}

impl ShaderStage {
    /// To flags
    #[inline]
    pub const fn to_flags(&self) -> ShaderStageFlags {
        ShaderStageFlags(*self as u32)
    }

    /// Get file extension
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Vertex => "vert",
            Self::TessellationControl => "tesc",
            Self::TessellationEvaluation => "tese",
            Self::Geometry => "geom",
            Self::Fragment => "frag",
            Self::Compute => "comp",
            Self::Task => "task",
            Self::Mesh => "mesh",
            Self::RayGen => "rgen",
            Self::AnyHit => "rahit",
            Self::ClosestHit => "rchit",
            Self::Miss => "rmiss",
            Self::Intersection => "rint",
            Self::Callable => "rcall",
        }
    }
}

// ============================================================================
// Shader Compile Options
// ============================================================================

/// Shader compile options
#[derive(Clone, Debug)]
pub struct ShaderCompileOptions {
    /// Source language
    pub source_language: ShaderSourceLanguage,
    /// Target environment
    pub target_env: TargetEnvironment,
    /// SPIR-V version
    pub spirv_version: SpirvVersion,
    /// Optimization level
    pub optimization_level: OptimizationLevel,
    /// Generate debug info
    pub debug_info: bool,
    /// Macro definitions
    pub definitions: Vec<ShaderDefine>,
    /// Include paths
    pub include_paths: Vec<String>,
    /// Entry point name
    pub entry_point: String,
}

impl ShaderCompileOptions {
    /// Default GLSL options
    pub fn glsl() -> Self {
        Self {
            source_language: ShaderSourceLanguage::Glsl,
            target_env: TargetEnvironment::Vulkan1_3,
            spirv_version: SpirvVersion::V1_6,
            optimization_level: OptimizationLevel::Performance,
            debug_info: false,
            definitions: Vec::new(),
            include_paths: Vec::new(),
            entry_point: String::from("main"),
        }
    }

    /// Default HLSL options
    pub fn hlsl() -> Self {
        Self {
            source_language: ShaderSourceLanguage::Hlsl,
            target_env: TargetEnvironment::Vulkan1_3,
            spirv_version: SpirvVersion::V1_6,
            optimization_level: OptimizationLevel::Performance,
            debug_info: false,
            definitions: Vec::new(),
            include_paths: Vec::new(),
            entry_point: String::from("main"),
        }
    }

    /// With debug info
    pub fn with_debug(mut self) -> Self {
        self.debug_info = true;
        self.optimization_level = OptimizationLevel::None;
        self
    }

    /// Add definition
    pub fn define(mut self, name: &str, value: Option<&str>) -> Self {
        self.definitions.push(ShaderDefine {
            name: String::from(name),
            value: value.map(String::from),
        });
        self
    }

    /// Add include path
    pub fn include_path(mut self, path: &str) -> Self {
        self.include_paths.push(String::from(path));
        self
    }

    /// With entry point
    pub fn entry_point(mut self, name: &str) -> Self {
        self.entry_point = String::from(name);
        self
    }

    /// With optimization level
    pub fn optimization(mut self, level: OptimizationLevel) -> Self {
        self.optimization_level = level;
        self
    }
}

impl Default for ShaderCompileOptions {
    fn default() -> Self {
        Self::glsl()
    }
}

/// Shader source language
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderSourceLanguage {
    /// GLSL
    #[default]
    Glsl = 0,
    /// HLSL
    Hlsl = 1,
    /// SPIR-V assembly
    SpirvAsm = 2,
}

/// Target environment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TargetEnvironment {
    /// Vulkan 1.0
    Vulkan1_0 = 0,
    /// Vulkan 1.1
    Vulkan1_1 = 1,
    /// Vulkan 1.2
    Vulkan1_2 = 2,
    /// Vulkan 1.3
    #[default]
    Vulkan1_3 = 3,
    /// OpenGL 4.5
    OpenGl = 10,
}

/// SPIR-V version
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SpirvVersion {
    /// SPIR-V 1.0
    V1_0 = 0x00010000,
    /// SPIR-V 1.1
    V1_1 = 0x00010100,
    /// SPIR-V 1.2
    V1_2 = 0x00010200,
    /// SPIR-V 1.3
    V1_3 = 0x00010300,
    /// SPIR-V 1.4
    V1_4 = 0x00010400,
    /// SPIR-V 1.5
    V1_5 = 0x00010500,
    /// SPIR-V 1.6
    #[default]
    V1_6 = 0x00010600,
}

/// Optimization level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OptimizationLevel {
    /// No optimization
    None = 0,
    /// Optimize for size
    Size = 1,
    /// Optimize for performance
    #[default]
    Performance = 2,
}

/// Shader define
#[derive(Clone, Debug)]
pub struct ShaderDefine {
    /// Name
    pub name: String,
    /// Value (optional)
    pub value: Option<String>,
}

// ============================================================================
// Shader Reflection
// ============================================================================

/// Shader reflection data
#[derive(Clone, Debug, Default)]
pub struct ShaderReflection {
    /// Entry points
    pub entry_points: Vec<EntryPointInfo>,
    /// Descriptor sets
    pub descriptor_sets: Vec<DescriptorSetLayoutData>,
    /// Push constant ranges
    pub push_constants: Vec<PushConstantRange>,
    /// Vertex input attributes
    pub vertex_inputs: Vec<VertexInputAttribute>,
    /// Fragment outputs
    pub fragment_outputs: Vec<FragmentOutput>,
    /// Workgroup size (for compute)
    pub workgroup_size: Option<[u32; 3]>,
    /// Specialization constants
    pub specialization_constants: Vec<SpecializationConstantInfo>,
}

impl ShaderReflection {
    /// Creates new reflection data
    pub fn new() -> Self {
        Self::default()
    }

    /// Get descriptor set by index
    pub fn descriptor_set(&self, set: u32) -> Option<&DescriptorSetLayoutData> {
        self.descriptor_sets.iter().find(|s| s.set == set)
    }

    /// Get push constant range
    pub fn push_constant_range(&self) -> Option<&PushConstantRange> {
        self.push_constants.first()
    }

    /// Has push constants
    pub fn has_push_constants(&self) -> bool {
        !self.push_constants.is_empty()
    }

    /// Total push constant size
    pub fn push_constant_size(&self) -> u32 {
        self.push_constants.iter().map(|p| p.size).sum()
    }
}

/// Entry point info
#[derive(Clone, Debug)]
pub struct EntryPointInfo {
    /// Name
    pub name: String,
    /// Execution model (shader stage)
    pub execution_model: ShaderStage,
    /// Workgroup size (for compute)
    pub workgroup_size: Option<[u32; 3]>,
}

/// Descriptor set layout data
#[derive(Clone, Debug)]
pub struct DescriptorSetLayoutData {
    /// Set number
    pub set: u32,
    /// Bindings
    pub bindings: Vec<DescriptorBindingData>,
}

/// Descriptor binding data
#[derive(Clone, Debug)]
pub struct DescriptorBindingData {
    /// Binding number
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Descriptor count
    pub count: u32,
    /// Stage flags
    pub stage_flags: ShaderStageFlags,
    /// Name (if available)
    pub name: Option<String>,
}

/// Descriptor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DescriptorType {
    /// Sampler
    Sampler = 0,
    /// Combined image sampler
    CombinedImageSampler = 1,
    /// Sampled image
    SampledImage = 2,
    /// Storage image
    StorageImage = 3,
    /// Uniform texel buffer
    UniformTexelBuffer = 4,
    /// Storage texel buffer
    StorageTexelBuffer = 5,
    /// Uniform buffer
    UniformBuffer = 6,
    /// Storage buffer
    StorageBuffer = 7,
    /// Dynamic uniform buffer
    UniformBufferDynamic = 8,
    /// Dynamic storage buffer
    StorageBufferDynamic = 9,
    /// Input attachment
    InputAttachment = 10,
    /// Acceleration structure
    AccelerationStructure = 1000150000,
}

/// Push constant range
#[derive(Clone, Debug)]
pub struct PushConstantRange {
    /// Stage flags
    pub stage_flags: ShaderStageFlags,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
    /// Members (if reflection available)
    pub members: Vec<PushConstantMember>,
}

/// Push constant member
#[derive(Clone, Debug)]
pub struct PushConstantMember {
    /// Name
    pub name: String,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
    /// Type
    pub member_type: ShaderDataType,
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
    /// Mat2
    Mat2 = 12,
    /// Mat3
    Mat3 = 13,
    /// Mat4
    Mat4 = 14,
    /// Bool
    Bool = 15,
    /// Struct
    Struct = 100,
    /// Array
    Array = 101,
}

impl ShaderDataType {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Float | Self::Int | Self::UInt | Self::Bool => 4,
            Self::Vec2 | Self::IVec2 | Self::UVec2 => 8,
            Self::Vec3 | Self::IVec3 | Self::UVec3 => 12,
            Self::Vec4 | Self::IVec4 | Self::UVec4 => 16,
            Self::Mat2 => 16,
            Self::Mat3 => 36,
            Self::Mat4 => 64,
            Self::Struct | Self::Array => 0, // Variable
        }
    }

    /// Alignment
    pub const fn alignment(&self) -> u32 {
        match self {
            Self::Float | Self::Int | Self::UInt | Self::Bool => 4,
            Self::Vec2 | Self::IVec2 | Self::UVec2 => 8,
            Self::Vec3 | Self::IVec3 | Self::UVec3 => 16, // Vec3 has 16-byte alignment in std140
            Self::Vec4 | Self::IVec4 | Self::UVec4 => 16,
            Self::Mat2 | Self::Mat3 | Self::Mat4 => 16,
            Self::Struct | Self::Array => 16, // Usually 16
        }
    }
}

/// Vertex input attribute
#[derive(Clone, Debug)]
pub struct VertexInputAttribute {
    /// Location
    pub location: u32,
    /// Format
    pub format: VertexFormat,
    /// Name (if available)
    pub name: Option<String>,
}

/// Vertex format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexFormat {
    /// R32 float
    #[default]
    R32Sfloat = 100,
    /// RG32 float
    R32G32Sfloat = 103,
    /// RGB32 float
    R32G32B32Sfloat = 106,
    /// RGBA32 float
    R32G32B32A32Sfloat = 109,
    /// R32 int
    R32Sint = 99,
    /// RG32 int
    R32G32Sint = 102,
    /// RGB32 int
    R32G32B32Sint = 105,
    /// RGBA32 int
    R32G32B32A32Sint = 108,
    /// R32 uint
    R32Uint = 98,
    /// RG32 uint
    R32G32Uint = 101,
    /// RGB32 uint
    R32G32B32Uint = 104,
    /// RGBA32 uint
    R32G32B32A32Uint = 107,
}

impl VertexFormat {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::R32Sfloat | Self::R32Sint | Self::R32Uint => 4,
            Self::R32G32Sfloat | Self::R32G32Sint | Self::R32G32Uint => 8,
            Self::R32G32B32Sfloat | Self::R32G32B32Sint | Self::R32G32B32Uint => 12,
            Self::R32G32B32A32Sfloat | Self::R32G32B32A32Sint | Self::R32G32B32A32Uint => 16,
        }
    }
}

/// Fragment output
#[derive(Clone, Debug)]
pub struct FragmentOutput {
    /// Location
    pub location: u32,
    /// Component count
    pub components: u32,
    /// Type
    pub output_type: ShaderDataType,
    /// Name (if available)
    pub name: Option<String>,
}

/// Specialization constant info
#[derive(Clone, Debug)]
pub struct SpecializationConstantInfo {
    /// Constant ID
    pub constant_id: u32,
    /// Name
    pub name: Option<String>,
    /// Type
    pub data_type: ShaderDataType,
    /// Default value (as bytes)
    pub default_value: [u8; 8],
}

// ============================================================================
// Shader Compile Error
// ============================================================================

/// Shader compile error
#[derive(Clone, Debug)]
pub enum ShaderCompileError {
    /// Invalid SPIR-V
    InvalidSpirv(String),
    /// Compilation failed
    CompilationFailed(String),
    /// Linking failed
    LinkingFailed(String),
    /// Unsupported feature
    UnsupportedFeature(String),
    /// IO error
    IoError(String),
    /// Reflection failed
    ReflectionFailed(String),
}

impl core::fmt::Display for ShaderCompileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidSpirv(msg) => write!(f, "Invalid SPIR-V: {}", msg),
            Self::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            Self::LinkingFailed(msg) => write!(f, "Linking failed: {}", msg),
            Self::UnsupportedFeature(msg) => write!(f, "Unsupported feature: {}", msg),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::ReflectionFailed(msg) => write!(f, "Reflection failed: {}", msg),
        }
    }
}

// ============================================================================
// Shader Pipeline Stage Create Info
// ============================================================================

/// Shader pipeline stage create info
#[derive(Clone, Debug)]
pub struct ShaderStageCreateInfo {
    /// Stage
    pub stage: ShaderStage,
    /// Module handle
    pub module: ShaderModuleHandle,
    /// Entry point name
    pub entry_point: String,
    /// Specialization info
    pub specialization_info: Option<SpecializationInfo>,
}

impl ShaderStageCreateInfo {
    /// Creates new stage info
    pub fn new(stage: ShaderStage, module: ShaderModuleHandle, entry: &str) -> Self {
        Self {
            stage,
            module,
            entry_point: String::from(entry),
            specialization_info: None,
        }
    }

    /// Vertex shader
    pub fn vertex(module: ShaderModuleHandle, entry: &str) -> Self {
        Self::new(ShaderStage::Vertex, module, entry)
    }

    /// Fragment shader
    pub fn fragment(module: ShaderModuleHandle, entry: &str) -> Self {
        Self::new(ShaderStage::Fragment, module, entry)
    }

    /// Compute shader
    pub fn compute(module: ShaderModuleHandle, entry: &str) -> Self {
        Self::new(ShaderStage::Compute, module, entry)
    }

    /// With specialization
    pub fn with_specialization(mut self, info: SpecializationInfo) -> Self {
        self.specialization_info = Some(info);
        self
    }
}

/// Specialization info
#[derive(Clone, Debug, Default)]
pub struct SpecializationInfo {
    /// Map entries
    pub map_entries: Vec<SpecializationMapEntry>,
    /// Data
    pub data: Vec<u8>,
}

impl SpecializationInfo {
    /// Creates new specialization info
    pub fn new() -> Self {
        Self::default()
    }

    /// Add bool constant
    pub fn add_bool(mut self, constant_id: u32, value: bool) -> Self {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&(value as u32).to_le_bytes());
        self.map_entries.push(SpecializationMapEntry {
            constant_id,
            offset,
            size: 4,
        });
        self
    }

    /// Add i32 constant
    pub fn add_i32(mut self, constant_id: u32, value: i32) -> Self {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.map_entries.push(SpecializationMapEntry {
            constant_id,
            offset,
            size: 4,
        });
        self
    }

    /// Add u32 constant
    pub fn add_u32(mut self, constant_id: u32, value: u32) -> Self {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.map_entries.push(SpecializationMapEntry {
            constant_id,
            offset,
            size: 4,
        });
        self
    }

    /// Add f32 constant
    pub fn add_f32(mut self, constant_id: u32, value: f32) -> Self {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.map_entries.push(SpecializationMapEntry {
            constant_id,
            offset,
            size: 4,
        });
        self
    }
}

/// Specialization map entry
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct SpecializationMapEntry {
    /// Constant ID
    pub constant_id: u32,
    /// Offset in data
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

// ============================================================================
// Shader Variant
// ============================================================================

/// Shader variant for permutation handling
#[derive(Clone, Debug)]
pub struct ShaderVariant {
    /// Base shader name
    pub base_name: String,
    /// Variant keywords
    pub keywords: Vec<String>,
    /// Module handle
    pub module: ShaderModuleHandle,
}

impl ShaderVariant {
    /// Creates new variant
    pub fn new(base_name: &str, keywords: &[&str], module: ShaderModuleHandle) -> Self {
        Self {
            base_name: String::from(base_name),
            keywords: keywords.iter().map(|s| String::from(*s)).collect(),
            module,
        }
    }

    /// Variant key for lookup
    pub fn variant_key(&self) -> String {
        let mut key = self.base_name.clone();
        for keyword in &self.keywords {
            key.push('_');
            key.push_str(keyword);
        }
        key
    }

    /// Has keyword
    pub fn has_keyword(&self, keyword: &str) -> bool {
        self.keywords.iter().any(|k| k == keyword)
    }
}

// ============================================================================
// Shader Library
// ============================================================================

/// Shader library for managing compiled shaders
#[derive(Debug, Default)]
pub struct ShaderLibrary {
    /// Shaders by name
    shaders: Vec<(String, ShaderModuleHandle)>,
    /// Variants
    variants: Vec<ShaderVariant>,
}

impl ShaderLibrary {
    /// Creates new library
    pub fn new() -> Self {
        Self::default()
    }

    /// Add shader
    pub fn add(&mut self, name: &str, module: ShaderModuleHandle) {
        self.shaders.push((String::from(name), module));
    }

    /// Add variant
    pub fn add_variant(&mut self, variant: ShaderVariant) {
        self.variants.push(variant);
    }

    /// Get shader by name
    pub fn get(&self, name: &str) -> Option<ShaderModuleHandle> {
        self.shaders.iter().find(|(n, _)| n == name).map(|(_, h)| *h)
    }

    /// Get variant
    pub fn get_variant(&self, base_name: &str, keywords: &[&str]) -> Option<ShaderModuleHandle> {
        for variant in &self.variants {
            if variant.base_name == base_name {
                let matches = keywords.iter().all(|k| variant.has_keyword(k));
                if matches && variant.keywords.len() == keywords.len() {
                    return Some(variant.module);
                }
            }
        }
        None
    }

    /// Shader count
    pub fn len(&self) -> usize {
        self.shaders.len() + self.variants.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.shaders.is_empty() && self.variants.is_empty()
    }
}
