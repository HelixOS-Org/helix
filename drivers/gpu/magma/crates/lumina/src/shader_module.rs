//! Shader module types and utilities
//!
//! This module provides types for shader modules and pipeline creation.

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ShaderStage {
    /// Vertex shader
    Vertex = 0x00000001,
    /// Tessellation control shader
    TessellationControl = 0x00000002,
    /// Tessellation evaluation shader
    TessellationEvaluation = 0x00000004,
    /// Geometry shader
    Geometry = 0x00000008,
    /// Fragment shader
    Fragment = 0x00000010,
    /// Compute shader
    Compute = 0x00000020,
    /// Task shader
    Task = 0x00000040,
    /// Mesh shader
    Mesh = 0x00000080,
    /// Ray generation shader
    RayGen = 0x00000100,
    /// Any hit shader
    AnyHit = 0x00000200,
    /// Closest hit shader
    ClosestHit = 0x00000400,
    /// Miss shader
    Miss = 0x00000800,
    /// Intersection shader
    Intersection = 0x00001000,
    /// Callable shader
    Callable = 0x00002000,
}

impl ShaderStage {
    /// Is graphics stage
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

    /// Is mesh shading stage
    pub const fn is_mesh_shading(&self) -> bool {
        matches!(self, Self::Task | Self::Mesh)
    }

    /// Is ray tracing stage
    pub const fn is_ray_tracing(&self) -> bool {
        matches!(
            self,
            Self::RayGen
                | Self::AnyHit
                | Self::ClosestHit
                | Self::Miss
                | Self::Intersection
                | Self::Callable
        )
    }
}

/// Shader module handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShaderModuleHandle(pub u64);

impl ShaderModuleHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is valid
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for ShaderModuleHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Shader module create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderModuleCreateInfo {
    /// Code size in bytes
    pub code_size: usize,
    /// Pointer to SPIR-V code
    pub code: *const u32,
}

impl Default for ShaderModuleCreateInfo {
    fn default() -> Self {
        Self {
            code_size: 0,
            code: core::ptr::null(),
        }
    }
}

impl ShaderModuleCreateInfo {
    /// Creates from SPIR-V slice
    pub fn from_spirv(spirv: &[u32]) -> Self {
        Self {
            code_size: spirv.len() * 4,
            code: spirv.as_ptr(),
        }
    }
}

/// Shader stage create info
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ShaderStageCreateInfo {
    /// Shader stage
    pub stage: ShaderStage,
    /// Shader module
    pub module: ShaderModuleHandle,
    /// Entry point name offset
    pub entry_point_offset: u32,
    /// Entry point name length
    pub entry_point_len: u32,
    /// Specialization info
    pub specialization: SpecializationInfo,
}

impl Default for ShaderStageCreateInfo {
    fn default() -> Self {
        Self {
            stage: ShaderStage::Vertex,
            module: ShaderModuleHandle::NULL,
            entry_point_offset: 0,
            entry_point_len: 4, // "main"
            specialization: SpecializationInfo::default(),
        }
    }
}

impl ShaderStageCreateInfo {
    /// Creates vertex shader stage
    pub const fn vertex(module: ShaderModuleHandle) -> Self {
        Self {
            stage: ShaderStage::Vertex,
            module,
            entry_point_offset: 0,
            entry_point_len: 4,
            specialization: SpecializationInfo::empty(),
        }
    }

    /// Creates fragment shader stage
    pub const fn fragment(module: ShaderModuleHandle) -> Self {
        Self {
            stage: ShaderStage::Fragment,
            module,
            entry_point_offset: 0,
            entry_point_len: 4,
            specialization: SpecializationInfo::empty(),
        }
    }

    /// Creates compute shader stage
    pub const fn compute(module: ShaderModuleHandle) -> Self {
        Self {
            stage: ShaderStage::Compute,
            module,
            entry_point_offset: 0,
            entry_point_len: 4,
            specialization: SpecializationInfo::empty(),
        }
    }

    /// Creates mesh shader stage
    pub const fn mesh(module: ShaderModuleHandle) -> Self {
        Self {
            stage: ShaderStage::Mesh,
            module,
            entry_point_offset: 0,
            entry_point_len: 4,
            specialization: SpecializationInfo::empty(),
        }
    }

    /// Creates task shader stage
    pub const fn task(module: ShaderModuleHandle) -> Self {
        Self {
            stage: ShaderStage::Task,
            module,
            entry_point_offset: 0,
            entry_point_len: 4,
            specialization: SpecializationInfo::empty(),
        }
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
    pub size: usize,
}

impl SpecializationMapEntry {
    /// Creates bool entry
    pub const fn bool_entry(constant_id: u32, offset: u32) -> Self {
        Self {
            constant_id,
            offset,
            size: 4, // VkBool32
        }
    }

    /// Creates i32 entry
    pub const fn i32_entry(constant_id: u32, offset: u32) -> Self {
        Self {
            constant_id,
            offset,
            size: 4,
        }
    }

    /// Creates u32 entry
    pub const fn u32_entry(constant_id: u32, offset: u32) -> Self {
        Self {
            constant_id,
            offset,
            size: 4,
        }
    }

    /// Creates f32 entry
    pub const fn f32_entry(constant_id: u32, offset: u32) -> Self {
        Self {
            constant_id,
            offset,
            size: 4,
        }
    }

    /// Creates f64 entry
    pub const fn f64_entry(constant_id: u32, offset: u32) -> Self {
        Self {
            constant_id,
            offset,
            size: 8,
        }
    }
}

/// Specialization info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SpecializationInfo {
    /// Map entry count
    pub map_entry_count: u32,
    /// Map entries pointer
    pub map_entries: *const SpecializationMapEntry,
    /// Data size
    pub data_size: usize,
    /// Data pointer
    pub data: *const u8,
}

impl SpecializationInfo {
    /// Empty specialization
    pub const fn empty() -> Self {
        Self {
            map_entry_count: 0,
            map_entries: core::ptr::null(),
            data_size: 0,
            data: core::ptr::null(),
        }
    }

    /// Is empty
    pub const fn is_empty(&self) -> bool {
        self.map_entry_count == 0
    }
}

/// Required subgroup size
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RequiredSubgroupSize {
    /// Use default
    Default = 0,
    /// 8 threads
    Size8 = 8,
    /// 16 threads
    Size16 = 16,
    /// 32 threads
    Size32 = 32,
    /// 64 threads
    Size64 = 64,
    /// 128 threads
    Size128 = 128,
}

/// Pipeline shader stage required subgroup size
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ShaderRequiredSubgroupSize {
    /// Required subgroup size
    pub required_subgroup_size: u32,
}

/// Shader resource binding
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ShaderResourceBinding {
    /// Binding set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: u32,
    /// Descriptor count
    pub descriptor_count: u32,
    /// Stage flags
    pub stage_flags: u32,
}

/// Shader push constant range
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ShaderPushConstantRange {
    /// Stage flags
    pub stage_flags: u32,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
}

/// Shader reflection info
#[derive(Clone, Debug, Default)]
pub struct ShaderReflection {
    /// Input attributes
    pub inputs: [ShaderInput; 16],
    /// Input count
    pub input_count: u32,
    /// Output attributes
    pub outputs: [ShaderOutput; 16],
    /// Output count
    pub output_count: u32,
    /// Resource bindings
    pub bindings: [ShaderResourceBinding; 32],
    /// Binding count
    pub binding_count: u32,
    /// Push constant ranges
    pub push_constants: [ShaderPushConstantRange; 4],
    /// Push constant count
    pub push_constant_count: u32,
    /// Local size X (compute/mesh)
    pub local_size_x: u32,
    /// Local size Y (compute/mesh)
    pub local_size_y: u32,
    /// Local size Z (compute/mesh)
    pub local_size_z: u32,
}

/// Shader input attribute
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ShaderInput {
    /// Location
    pub location: u32,
    /// Format
    pub format: u32,
    /// Component count
    pub component_count: u32,
}

/// Shader output attribute
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ShaderOutput {
    /// Location
    pub location: u32,
    /// Format
    pub format: u32,
    /// Component count
    pub component_count: u32,
}

/// Shader binary type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShaderBinaryType {
    /// SPIR-V
    SpirV = 0,
    /// DXIL
    Dxil = 1,
    /// Metal
    Metal = 2,
    /// GLSL source
    GlslSource = 3,
    /// HLSL source
    HlslSource = 4,
}

/// Shader compile options
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ShaderCompileOptions {
    /// Optimization level (0-3)
    pub optimization_level: u8,
    /// Generate debug info
    pub debug_info: bool,
    /// Validate SPIR-V
    pub validate: bool,
    /// Target Vulkan version (e.g., 0x00401000 for 1.1)
    pub target_vulkan_version: u32,
    /// Target SPIR-V version
    pub target_spirv_version: u32,
}

impl ShaderCompileOptions {
    /// Debug build options
    pub const DEBUG: Self = Self {
        optimization_level: 0,
        debug_info: true,
        validate: true,
        target_vulkan_version: 0x00401000,
        target_spirv_version: 0x00010300,
    };

    /// Release build options
    pub const RELEASE: Self = Self {
        optimization_level: 3,
        debug_info: false,
        validate: false,
        target_vulkan_version: 0x00401000,
        target_spirv_version: 0x00010300,
    };
}

/// Shader identifier (for ray tracing)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ShaderIdentifier {
    /// Identifier data (32 bytes)
    pub data: [u8; 32],
}

impl ShaderIdentifier {
    /// Size in bytes
    pub const SIZE: usize = 32;

    /// Creates from bytes
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { data: bytes }
    }

    /// Is null
    pub fn is_null(&self) -> bool {
        self.data.iter().all(|&b| b == 0)
    }
}

/// Pipeline handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineHandle(pub u64);

impl PipelineHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is valid
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for PipelineHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Pipeline cache handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineCacheHandle(pub u64);

impl PipelineCacheHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is valid
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for PipelineCacheHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Pipeline create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineCreateFlags(pub u32);

impl PipelineCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Disable optimization
    pub const DISABLE_OPTIMIZATION: Self = Self(1 << 0);
    /// Allow derivatives
    pub const ALLOW_DERIVATIVES: Self = Self(1 << 1);
    /// Derivative pipeline
    pub const DERIVATIVE: Self = Self(1 << 2);
    /// Fail on pipeline compile required
    pub const FAIL_ON_COMPILE_REQUIRED: Self = Self(1 << 8);
    /// Early return on failure
    pub const EARLY_RETURN_ON_FAILURE: Self = Self(1 << 9);
    /// Link time optimization
    pub const LINK_TIME_OPTIMIZATION: Self = Self(1 << 10);
    /// Retain link time optimization info
    pub const RETAIN_LINK_TIME_OPTIMIZATION_INFO: Self = Self(1 << 23);
    /// Library
    pub const LIBRARY: Self = Self(1 << 11);
    /// Ray tracing skip triangles
    pub const RAY_TRACING_SKIP_TRIANGLES: Self = Self(1 << 12);
    /// Ray tracing skip AABBs
    pub const RAY_TRACING_SKIP_AABBS: Self = Self(1 << 13);
    /// Ray tracing no null any hit shaders
    pub const RAY_TRACING_NO_NULL_ANY_HIT_SHADERS: Self = Self(1 << 14);
    /// Ray tracing no null closest hit shaders
    pub const RAY_TRACING_NO_NULL_CLOSEST_HIT_SHADERS: Self = Self(1 << 15);
    /// Ray tracing no null miss shaders
    pub const RAY_TRACING_NO_NULL_MISS_SHADERS: Self = Self(1 << 16);
    /// Ray tracing no null intersection shaders
    pub const RAY_TRACING_NO_NULL_INTERSECTION_SHADERS: Self = Self(1 << 17);
}

impl core::ops::BitOr for PipelineCreateFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
