//! Shader Types for Lumina
//!
//! This module provides comprehensive shader type definitions, configurations,
//! and metadata for shader module management.

use core::fmt;

// ============================================================================
// Shader Handle
// ============================================================================

/// Shader module handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderHandle(pub u64);

impl ShaderHandle {
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

impl Default for ShaderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Shader Stage
// ============================================================================

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderStage {
    /// Vertex shader
    #[default]
    Vertex              = 0x00000001,
    /// Tessellation control shader
    TessellationControl = 0x00000002,
    /// Tessellation evaluation shader
    TessellationEvaluation = 0x00000004,
    /// Geometry shader
    Geometry            = 0x00000008,
    /// Fragment shader
    Fragment            = 0x00000010,
    /// Compute shader
    Compute             = 0x00000020,
    /// Task shader (mesh shading)
    Task                = 0x00000040,
    /// Mesh shader
    Mesh                = 0x00000080,
    /// Ray generation shader
    RayGeneration       = 0x00000100,
    /// Any-hit shader
    AnyHit              = 0x00000200,
    /// Closest-hit shader
    ClosestHit          = 0x00000400,
    /// Miss shader
    Miss                = 0x00000800,
    /// Intersection shader
    Intersection        = 0x00001000,
    /// Callable shader
    Callable            = 0x00002000,
}

impl ShaderStage {
    /// Is graphics stage
    #[inline]
    pub const fn is_graphics(&self) -> bool {
        matches!(
            self,
            Self::Vertex
                | Self::TessellationControl
                | Self::TessellationEvaluation
                | Self::Geometry
                | Self::Fragment
        )
    }

    /// Is compute stage
    #[inline]
    pub const fn is_compute(&self) -> bool {
        matches!(self, Self::Compute)
    }

    /// Is mesh shading stage
    #[inline]
    pub const fn is_mesh_shading(&self) -> bool {
        matches!(self, Self::Task | Self::Mesh)
    }

    /// Is ray tracing stage
    #[inline]
    pub const fn is_ray_tracing(&self) -> bool {
        matches!(
            self,
            Self::RayGeneration
                | Self::AnyHit
                | Self::ClosestHit
                | Self::Miss
                | Self::Intersection
                | Self::Callable
        )
    }

    /// Stage name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::TessellationControl => "tessellation_control",
            Self::TessellationEvaluation => "tessellation_evaluation",
            Self::Geometry => "geometry",
            Self::Fragment => "fragment",
            Self::Compute => "compute",
            Self::Task => "task",
            Self::Mesh => "mesh",
            Self::RayGeneration => "raygen",
            Self::AnyHit => "anyhit",
            Self::ClosestHit => "closesthit",
            Self::Miss => "miss",
            Self::Intersection => "intersection",
            Self::Callable => "callable",
        }
    }

    /// SPIR-V execution model
    #[inline]
    pub const fn spirv_execution_model(&self) -> u32 {
        match self {
            Self::Vertex => 0,                 // Vertex
            Self::TessellationControl => 1,    // TessellationControl
            Self::TessellationEvaluation => 2, // TessellationEvaluation
            Self::Geometry => 3,               // Geometry
            Self::Fragment => 4,               // Fragment
            Self::Compute => 5,                // GLCompute
            Self::Task => 5267,                // TaskNV / TaskEXT
            Self::Mesh => 5268,                // MeshNV / MeshEXT
            Self::RayGeneration => 5313,       // RayGenerationKHR
            Self::AnyHit => 5314,              // AnyHitKHR
            Self::ClosestHit => 5315,          // ClosestHitKHR
            Self::Miss => 5316,                // MissKHR
            Self::Intersection => 5317,        // IntersectionKHR
            Self::Callable => 5318,            // CallableKHR
        }
    }
}

impl fmt::Display for ShaderStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
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
    /// No stages
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
    /// Task (mesh shading)
    pub const TASK: Self = Self(0x00000040);
    /// Mesh
    pub const MESH: Self = Self(0x00000080);
    /// Ray generation
    pub const RAY_GENERATION: Self = Self(0x00000100);
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
    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(0x0000001F);
    /// All stages
    pub const ALL: Self = Self(0x7FFFFFFF);

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

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// From stage
    #[inline]
    pub const fn from_stage(stage: ShaderStage) -> Self {
        Self(stage as u32)
    }

    /// Count stages
    #[inline]
    pub const fn count(&self) -> u32 {
        self.0.count_ones()
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

// ============================================================================
// Shader Module Configuration
// ============================================================================

/// Shader module configuration
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ShaderModuleConfig {
    /// SPIR-V bytecode
    pub code: &'static [u32],
    /// Shader stage
    pub stage: ShaderStage,
    /// Entry point name
    pub entry_point: &'static str,
    /// Flags
    pub flags: ShaderModuleFlags,
    /// Specialization info (optional)
    pub specialization: Option<SpecializationInfo>,
}

impl ShaderModuleConfig {
    /// Creates new config
    #[inline]
    pub const fn new(code: &'static [u32], stage: ShaderStage, entry_point: &'static str) -> Self {
        Self {
            code,
            stage,
            entry_point,
            flags: ShaderModuleFlags::NONE,
            specialization: None,
        }
    }

    /// Vertex shader
    #[inline]
    pub const fn vertex(code: &'static [u32]) -> Self {
        Self::new(code, ShaderStage::Vertex, "main")
    }

    /// Fragment shader
    #[inline]
    pub const fn fragment(code: &'static [u32]) -> Self {
        Self::new(code, ShaderStage::Fragment, "main")
    }

    /// Compute shader
    #[inline]
    pub const fn compute(code: &'static [u32]) -> Self {
        Self::new(code, ShaderStage::Compute, "main")
    }

    /// With entry point
    #[inline]
    pub const fn with_entry_point(mut self, entry_point: &'static str) -> Self {
        self.entry_point = entry_point;
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: ShaderModuleFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Code size in bytes
    #[inline]
    pub const fn code_size(&self) -> usize {
        self.code.len() * 4
    }
}

/// Shader module flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ShaderModuleFlags(pub u32);

impl ShaderModuleFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Allow derivatives
    pub const ALLOW_DERIVATIVES: Self = Self(1 << 0);

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

// ============================================================================
// Specialization Constants
// ============================================================================

/// Specialization info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SpecializationInfo {
    /// Map entries
    pub entries: &'static [SpecializationMapEntry],
    /// Data
    pub data: &'static [u8],
}

impl SpecializationInfo {
    /// Creates new info
    #[inline]
    pub const fn new(entries: &'static [SpecializationMapEntry], data: &'static [u8]) -> Self {
        Self { entries, data }
    }

    /// Empty
    pub const EMPTY: Self = Self {
        entries: &[],
        data: &[],
    };
}

impl Default for SpecializationInfo {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Specialization map entry
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SpecializationMapEntry {
    /// Constant ID
    pub constant_id: u32,
    /// Offset in data
    pub offset: u32,
    /// Size
    pub size: u32,
}

impl SpecializationMapEntry {
    /// Creates new entry
    #[inline]
    pub const fn new(constant_id: u32, offset: u32, size: u32) -> Self {
        Self {
            constant_id,
            offset,
            size,
        }
    }

    /// Bool entry
    #[inline]
    pub const fn bool_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// Int32 entry
    #[inline]
    pub const fn int32_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// Uint32 entry
    #[inline]
    pub const fn uint32_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// Float32 entry
    #[inline]
    pub const fn float32_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// Float64 entry
    #[inline]
    pub const fn float64_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 8)
    }
}

// ============================================================================
// Shader Source Types
// ============================================================================

/// Shader source type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderSourceType {
    /// SPIR-V binary
    #[default]
    SpirV = 0,
    /// GLSL source
    Glsl  = 1,
    /// HLSL source
    Hlsl  = 2,
    /// WGSL source
    Wgsl  = 3,
    /// Slang source
    Slang = 4,
    /// Rust (Lumina shader macro)
    Rust  = 5,
}

impl ShaderSourceType {
    /// Needs compilation
    #[inline]
    pub const fn needs_compilation(&self) -> bool {
        !matches!(self, Self::SpirV)
    }

    /// File extension
    #[inline]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::SpirV => ".spv",
            Self::Glsl => ".glsl",
            Self::Hlsl => ".hlsl",
            Self::Wgsl => ".wgsl",
            Self::Slang => ".slang",
            Self::Rust => ".rs",
        }
    }
}

// ============================================================================
// Shader Compiler Options
// ============================================================================

/// Shader compiler options
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ShaderCompilerOptions {
    /// Source type
    pub source_type: ShaderSourceType,
    /// Target stage
    pub target_stage: ShaderStage,
    /// Optimization level
    pub optimization_level: ShaderOptimizationLevel,
    /// Generate debug info
    pub debug_info: bool,
    /// Validation enabled
    pub validation: bool,
    /// Include paths
    pub include_paths: &'static [&'static str],
    /// Defines
    pub defines: &'static [ShaderDefine],
}

impl ShaderCompilerOptions {
    /// Creates new options
    #[inline]
    pub const fn new(source_type: ShaderSourceType, stage: ShaderStage) -> Self {
        Self {
            source_type,
            target_stage: stage,
            optimization_level: ShaderOptimizationLevel::Performance,
            debug_info: false,
            validation: true,
            include_paths: &[],
            defines: &[],
        }
    }

    /// With optimization level
    #[inline]
    pub const fn with_optimization(mut self, level: ShaderOptimizationLevel) -> Self {
        self.optimization_level = level;
        self
    }

    /// With debug info
    #[inline]
    pub const fn with_debug_info(mut self) -> Self {
        self.debug_info = true;
        self
    }

    /// Without validation
    #[inline]
    pub const fn without_validation(mut self) -> Self {
        self.validation = false;
        self
    }

    /// Debug preset
    #[inline]
    pub const fn debug(source_type: ShaderSourceType, stage: ShaderStage) -> Self {
        Self {
            source_type,
            target_stage: stage,
            optimization_level: ShaderOptimizationLevel::None,
            debug_info: true,
            validation: true,
            include_paths: &[],
            defines: &[],
        }
    }

    /// Release preset
    #[inline]
    pub const fn release(source_type: ShaderSourceType, stage: ShaderStage) -> Self {
        Self {
            source_type,
            target_stage: stage,
            optimization_level: ShaderOptimizationLevel::Size,
            debug_info: false,
            validation: false,
            include_paths: &[],
            defines: &[],
        }
    }
}

impl Default for ShaderCompilerOptions {
    fn default() -> Self {
        Self::new(ShaderSourceType::SpirV, ShaderStage::Vertex)
    }
}

/// Shader optimization level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderOptimizationLevel {
    /// No optimization
    None        = 0,
    /// Basic optimization
    Basic       = 1,
    /// Performance optimization
    #[default]
    Performance = 2,
    /// Size optimization
    Size        = 3,
}

/// Shader define
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderDefine {
    /// Name
    pub name: &'static str,
    /// Value (optional)
    pub value: Option<&'static str>,
}

impl ShaderDefine {
    /// Creates new define
    #[inline]
    pub const fn new(name: &'static str, value: &'static str) -> Self {
        Self {
            name,
            value: Some(value),
        }
    }

    /// Creates define without value
    #[inline]
    pub const fn flag(name: &'static str) -> Self {
        Self { name, value: None }
    }
}

// ============================================================================
// Shader Reflection
// ============================================================================

/// Shader reflection info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ShaderReflectionInfo {
    /// Entry point name
    pub entry_point: &'static str,
    /// Stage
    pub stage: ShaderStage,
    /// Inputs
    pub inputs: &'static [ShaderInterfaceVariable],
    /// Outputs
    pub outputs: &'static [ShaderInterfaceVariable],
    /// Descriptor bindings
    pub descriptor_bindings: &'static [DescriptorBindingInfo],
    /// Push constant ranges
    pub push_constant_ranges: &'static [PushConstantRangeInfo],
    /// Workgroup size (for compute)
    pub workgroup_size: Option<WorkgroupSize>,
}

impl ShaderReflectionInfo {
    /// Creates new info
    #[inline]
    pub const fn new(entry_point: &'static str, stage: ShaderStage) -> Self {
        Self {
            entry_point,
            stage,
            inputs: &[],
            outputs: &[],
            descriptor_bindings: &[],
            push_constant_ranges: &[],
            workgroup_size: None,
        }
    }

    /// Total input size
    #[inline]
    pub fn total_input_size(&self) -> u32 {
        self.inputs.iter().map(|v| v.size()).sum()
    }

    /// Total output size
    #[inline]
    pub fn total_output_size(&self) -> u32 {
        self.outputs.iter().map(|v| v.size()).sum()
    }
}

/// Shader interface variable
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderInterfaceVariable {
    /// Location
    pub location: u32,
    /// Component (for packed data)
    pub component: u32,
    /// Format
    pub format: ShaderDataFormat,
    /// Name (optional)
    pub name: Option<&'static str>,
}

impl ShaderInterfaceVariable {
    /// Creates new variable
    #[inline]
    pub const fn new(location: u32, format: ShaderDataFormat) -> Self {
        Self {
            location,
            component: 0,
            format,
            name: None,
        }
    }

    /// With component
    #[inline]
    pub const fn with_component(mut self, component: u32) -> Self {
        self.component = component;
        self
    }

    /// With name
    #[inline]
    pub const fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    /// Size
    #[inline]
    pub const fn size(&self) -> u32 {
        self.format.size()
    }
}

/// Shader data format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderDataFormat {
    /// Float
    #[default]
    Float  = 0,
    /// Vec2
    Vec2   = 1,
    /// Vec3
    Vec3   = 2,
    /// Vec4
    Vec4   = 3,
    /// Int
    Int    = 4,
    /// IVec2
    IVec2  = 5,
    /// IVec3
    IVec3  = 6,
    /// IVec4
    IVec4  = 7,
    /// Uint
    Uint   = 8,
    /// UVec2
    UVec2  = 9,
    /// UVec3
    UVec3  = 10,
    /// UVec4
    UVec4  = 11,
    /// Mat2
    Mat2   = 12,
    /// Mat3
    Mat3   = 13,
    /// Mat4
    Mat4   = 14,
    /// Double
    Double = 15,
    /// DVec2
    DVec2  = 16,
    /// DVec3
    DVec3  = 17,
    /// DVec4
    DVec4  = 18,
}

impl ShaderDataFormat {
    /// Size in bytes
    #[inline]
    pub const fn size(&self) -> u32 {
        match self {
            Self::Float | Self::Int | Self::Uint => 4,
            Self::Vec2 | Self::IVec2 | Self::UVec2 | Self::Double => 8,
            Self::Vec3 | Self::IVec3 | Self::UVec3 => 12,
            Self::Vec4 | Self::IVec4 | Self::UVec4 | Self::Mat2 | Self::DVec2 => 16,
            Self::Mat3 | Self::DVec3 => 24,
            Self::DVec4 => 32,
            Self::Mat4 => 64,
        }
    }

    /// Component count
    #[inline]
    pub const fn components(&self) -> u32 {
        match self {
            Self::Float | Self::Int | Self::Uint | Self::Double => 1,
            Self::Vec2 | Self::IVec2 | Self::UVec2 | Self::DVec2 => 2,
            Self::Vec3 | Self::IVec3 | Self::UVec3 | Self::DVec3 => 3,
            Self::Vec4 | Self::IVec4 | Self::UVec4 | Self::DVec4 | Self::Mat2 => 4,
            Self::Mat3 => 9,
            Self::Mat4 => 16,
        }
    }

    /// Is integer
    #[inline]
    pub const fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Int
                | Self::IVec2
                | Self::IVec3
                | Self::IVec4
                | Self::Uint
                | Self::UVec2
                | Self::UVec3
                | Self::UVec4
        )
    }

    /// Is double precision
    #[inline]
    pub const fn is_double(&self) -> bool {
        matches!(self, Self::Double | Self::DVec2 | Self::DVec3 | Self::DVec4)
    }
}

/// Descriptor binding info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DescriptorBindingInfo {
    /// Set
    pub set: u32,
    /// Binding
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Count (for arrays)
    pub count: u32,
    /// Stages
    pub stages: ShaderStageFlags,
    /// Name (optional)
    pub name: Option<&'static str>,
}

impl DescriptorBindingInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        set: u32,
        binding: u32,
        descriptor_type: DescriptorType,
        stages: ShaderStageFlags,
    ) -> Self {
        Self {
            set,
            binding,
            descriptor_type,
            count: 1,
            stages,
            name: None,
        }
    }

    /// With count
    #[inline]
    pub const fn with_count(mut self, count: u32) -> Self {
        self.count = count;
        self
    }

    /// With name
    #[inline]
    pub const fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    /// Is array
    #[inline]
    pub const fn is_array(&self) -> bool {
        self.count > 1
    }
}

/// Descriptor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DescriptorType {
    /// Sampler
    #[default]
    Sampler              = 0,
    /// Combined image sampler
    CombinedImageSampler = 1,
    /// Sampled image
    SampledImage         = 2,
    /// Storage image
    StorageImage         = 3,
    /// Uniform texel buffer
    UniformTexelBuffer   = 4,
    /// Storage texel buffer
    StorageTexelBuffer   = 5,
    /// Uniform buffer
    UniformBuffer        = 6,
    /// Storage buffer
    StorageBuffer        = 7,
    /// Dynamic uniform buffer
    UniformBufferDynamic = 8,
    /// Dynamic storage buffer
    StorageBufferDynamic = 9,
    /// Input attachment
    InputAttachment      = 10,
    /// Inline uniform block
    InlineUniformBlock   = 13,
    /// Acceleration structure
    AccelerationStructure = 1000150000,
}

/// Push constant range info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PushConstantRangeInfo {
    /// Stages
    pub stages: ShaderStageFlags,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
    /// Members
    pub members: &'static [PushConstantMember],
}

impl PushConstantRangeInfo {
    /// Creates new info
    #[inline]
    pub const fn new(stages: ShaderStageFlags, offset: u32, size: u32) -> Self {
        Self {
            stages,
            offset,
            size,
            members: &[],
        }
    }

    /// With members
    #[inline]
    pub const fn with_members(mut self, members: &'static [PushConstantMember]) -> Self {
        self.members = members;
        self
    }
}

/// Push constant member
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PushConstantMember {
    /// Name
    pub name: &'static str,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
    /// Format
    pub format: ShaderDataFormat,
}

impl PushConstantMember {
    /// Creates new member
    #[inline]
    pub const fn new(name: &'static str, offset: u32, format: ShaderDataFormat) -> Self {
        Self {
            name,
            offset,
            size: format.size(),
            format,
        }
    }
}

/// Workgroup size
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct WorkgroupSize {
    /// X dimension
    pub x: u32,
    /// Y dimension
    pub y: u32,
    /// Z dimension
    pub z: u32,
}

impl WorkgroupSize {
    /// Creates 1D workgroup
    #[inline]
    pub const fn d1(x: u32) -> Self {
        Self { x, y: 1, z: 1 }
    }

    /// Creates 2D workgroup
    #[inline]
    pub const fn d2(x: u32, y: u32) -> Self {
        Self { x, y, z: 1 }
    }

    /// Creates 3D workgroup
    #[inline]
    pub const fn d3(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Total size
    #[inline]
    pub const fn total(&self) -> u32 {
        self.x * self.y * self.z
    }

    /// Common presets
    pub const S64: Self = Self::d1(64);
    pub const S128: Self = Self::d1(128);
    pub const S256: Self = Self::d1(256);
    pub const S8X8: Self = Self::d2(8, 8);
    pub const S16X16: Self = Self::d2(16, 16);
    pub const S32X32: Self = Self::d2(32, 32);
    pub const S4X4X4: Self = Self::d3(4, 4, 4);
    pub const S8X8X8: Self = Self::d3(8, 8, 8);
}

// ============================================================================
// Shader Binary Info
// ============================================================================

/// Shader binary info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderBinaryInfo {
    /// Magic number (SPIR-V magic: 0x07230203)
    pub magic: u32,
    /// Version
    pub version: u32,
    /// Generator ID
    pub generator: u32,
    /// Bound (max ID + 1)
    pub bound: u32,
    /// Instruction schema
    pub schema: u32,
}

impl ShaderBinaryInfo {
    /// SPIR-V magic number
    pub const SPIRV_MAGIC: u32 = 0x07230203;
    /// SPIR-V magic number (reversed)
    pub const SPIRV_MAGIC_REV: u32 = 0x03022307;

    /// From SPIR-V code
    #[inline]
    pub const fn from_spirv(code: &[u32]) -> Option<Self> {
        if code.len() < 5 {
            return None;
        }

        if code[0] != Self::SPIRV_MAGIC && code[0] != Self::SPIRV_MAGIC_REV {
            return None;
        }

        Some(Self {
            magic: code[0],
            version: code[1],
            generator: code[2],
            bound: code[3],
            schema: code[4],
        })
    }

    /// Is valid SPIR-V
    #[inline]
    pub const fn is_valid_spirv(&self) -> bool {
        self.magic == Self::SPIRV_MAGIC || self.magic == Self::SPIRV_MAGIC_REV
    }

    /// Needs byte swap
    #[inline]
    pub const fn needs_byte_swap(&self) -> bool {
        self.magic == Self::SPIRV_MAGIC_REV
    }

    /// SPIR-V major version
    #[inline]
    pub const fn spirv_major(&self) -> u32 {
        (self.version >> 16) & 0xFF
    }

    /// SPIR-V minor version
    #[inline]
    pub const fn spirv_minor(&self) -> u32 {
        (self.version >> 8) & 0xFF
    }
}

// ============================================================================
// Shader Group Types
// ============================================================================

/// Shader group type (for ray tracing)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderGroupType {
    /// General (raygen, miss, callable)
    #[default]
    General            = 0,
    /// Triangles hit group
    TrianglesHitGroup  = 1,
    /// Procedural hit group
    ProceduralHitGroup = 2,
}

/// Shader group info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderGroupInfo {
    /// Group type
    pub group_type: ShaderGroupType,
    /// General shader index
    pub general_shader: u32,
    /// Closest hit shader index
    pub closest_hit_shader: u32,
    /// Any hit shader index
    pub any_hit_shader: u32,
    /// Intersection shader index
    pub intersection_shader: u32,
}

impl ShaderGroupInfo {
    /// Unused shader index
    pub const SHADER_UNUSED: u32 = u32::MAX;

    /// General group
    #[inline]
    pub const fn general(shader_index: u32) -> Self {
        Self {
            group_type: ShaderGroupType::General,
            general_shader: shader_index,
            closest_hit_shader: Self::SHADER_UNUSED,
            any_hit_shader: Self::SHADER_UNUSED,
            intersection_shader: Self::SHADER_UNUSED,
        }
    }

    /// Triangles hit group
    #[inline]
    pub const fn triangles_hit(closest_hit: u32) -> Self {
        Self {
            group_type: ShaderGroupType::TrianglesHitGroup,
            general_shader: Self::SHADER_UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: Self::SHADER_UNUSED,
            intersection_shader: Self::SHADER_UNUSED,
        }
    }

    /// Triangles hit group with any hit
    #[inline]
    pub const fn triangles_hit_with_any(closest_hit: u32, any_hit: u32) -> Self {
        Self {
            group_type: ShaderGroupType::TrianglesHitGroup,
            general_shader: Self::SHADER_UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: any_hit,
            intersection_shader: Self::SHADER_UNUSED,
        }
    }

    /// Procedural hit group
    #[inline]
    pub const fn procedural_hit(intersection: u32, closest_hit: u32) -> Self {
        Self {
            group_type: ShaderGroupType::ProceduralHitGroup,
            general_shader: Self::SHADER_UNUSED,
            closest_hit_shader: closest_hit,
            any_hit_shader: Self::SHADER_UNUSED,
            intersection_shader: intersection,
        }
    }
}

impl Default for ShaderGroupInfo {
    fn default() -> Self {
        Self::general(Self::SHADER_UNUSED)
    }
}
