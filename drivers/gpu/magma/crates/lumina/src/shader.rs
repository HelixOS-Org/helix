//! Shader compilation and reflection
//!
//! This module provides types for shader management.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;

use crate::types::ShaderHandle;
use crate::compute::{ShaderStage, ShaderStageFlags, TextureFormat};

/// Shader module descriptor
#[derive(Clone, Debug)]
pub struct ShaderModuleDesc<'a> {
    /// Debug label
    pub label: Option<&'a str>,
    /// Shader source
    pub source: ShaderSource<'a>,
    /// Entry point name
    pub entry_point: &'a str,
    /// Shader stage
    pub stage: ShaderStage,
}

impl<'a> ShaderModuleDesc<'a> {
    /// Creates a vertex shader descriptor
    pub const fn vertex(spirv: &'a [u32], entry_point: &'a str) -> Self {
        Self {
            label: None,
            source: ShaderSource::SpirV(spirv),
            entry_point,
            stage: ShaderStage::Vertex,
        }
    }

    /// Creates a fragment shader descriptor
    pub const fn fragment(spirv: &'a [u32], entry_point: &'a str) -> Self {
        Self {
            label: None,
            source: ShaderSource::SpirV(spirv),
            entry_point,
            stage: ShaderStage::Fragment,
        }
    }

    /// Creates a compute shader descriptor
    pub const fn compute(spirv: &'a [u32], entry_point: &'a str) -> Self {
        Self {
            label: None,
            source: ShaderSource::SpirV(spirv),
            entry_point,
            stage: ShaderStage::Compute,
        }
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

/// Shader source
#[derive(Clone, Debug)]
pub enum ShaderSource<'a> {
    /// SPIR-V binary
    SpirV(&'a [u32]),
    /// SPIR-V binary (owned)
    SpirVOwned(Vec<u32>),
    /// GLSL source (requires runtime compilation)
    Glsl {
        code: &'a str,
        stage: ShaderStage,
        defines: &'a [(&'a str, &'a str)],
    },
    /// HLSL source (requires runtime compilation)
    Hlsl {
        code: &'a str,
        stage: ShaderStage,
        entry_point: &'a str,
        defines: &'a [(&'a str, &'a str)],
    },
}

/// Shader reflection info
#[derive(Clone, Debug, Default)]
pub struct ShaderReflection {
    /// Entry points
    pub entry_points: Vec<EntryPoint>,
    /// Descriptor bindings
    pub bindings: Vec<ReflectedBinding>,
    /// Push constant ranges
    pub push_constants: Vec<PushConstantRange>,
    /// Input variables (for vertex shaders)
    pub inputs: Vec<ShaderVariable>,
    /// Output variables (for fragment shaders)
    pub outputs: Vec<ShaderVariable>,
    /// Specialization constants
    pub spec_constants: Vec<SpecializationConstant>,
    /// Workgroup size (for compute shaders)
    pub workgroup_size: Option<[u32; 3]>,
}

/// Entry point info
#[derive(Clone, Debug)]
pub struct EntryPoint {
    /// Name
    pub name: String,
    /// Shader stage
    pub stage: ShaderStage,
    /// Workgroup size (for compute)
    pub workgroup_size: Option<[u32; 3]>,
}

/// Reflected binding
#[derive(Clone, Debug)]
pub struct ReflectedBinding {
    /// Set index
    pub set: u32,
    /// Binding index
    pub binding: u32,
    /// Binding type
    pub binding_type: ReflectedBindingType,
    /// Array count (1 for non-array)
    pub count: u32,
    /// Stages that use this binding
    pub stages: ShaderStageFlags,
    /// Name (if available)
    pub name: Option<String>,
}

/// Reflected binding type
#[derive(Clone, Debug)]
pub enum ReflectedBindingType {
    /// Uniform buffer
    UniformBuffer {
        size: u64,
        dynamic: bool,
    },
    /// Storage buffer
    StorageBuffer {
        size: u64,
        dynamic: bool,
        read_only: bool,
    },
    /// Sampler
    Sampler,
    /// Sampled texture
    SampledTexture {
        dimension: TextureDimension,
        multisampled: bool,
    },
    /// Storage texture
    StorageTexture {
        dimension: TextureDimension,
        format: TextureFormat,
        read_only: bool,
    },
    /// Combined texture/sampler
    CombinedTextureSampler {
        dimension: TextureDimension,
        multisampled: bool,
    },
    /// Input attachment
    InputAttachment {
        index: u32,
    },
    /// Acceleration structure (ray tracing)
    AccelerationStructure,
}

/// Texture dimension
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureDimension {
    /// 1D texture
    D1,
    /// 2D texture
    D2,
    /// 3D texture
    D3,
    /// Cube texture
    Cube,
    /// 2D array texture
    D2Array,
    /// Cube array texture
    CubeArray,
}

/// Push constant range
#[derive(Clone, Debug)]
pub struct PushConstantRange {
    /// Shader stages
    pub stages: ShaderStageFlags,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

/// Shader variable (input/output)
#[derive(Clone, Debug)]
pub struct ShaderVariable {
    /// Location
    pub location: u32,
    /// Component (for partial locations)
    pub component: u32,
    /// Format
    pub format: VertexFormat,
    /// Name (if available)
    pub name: Option<String>,
}

/// Vertex format
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VertexFormat {
    /// f32
    Float32,
    /// vec2
    Float32x2,
    /// vec3
    Float32x3,
    /// vec4
    Float32x4,
    /// i32
    Int32,
    /// ivec2
    Int32x2,
    /// ivec3
    Int32x3,
    /// ivec4
    Int32x4,
    /// u32
    Uint32,
    /// uvec2
    Uint32x2,
    /// uvec3
    Uint32x3,
    /// uvec4
    Uint32x4,
    /// f64
    Float64,
    /// dvec2
    Float64x2,
    /// dvec3
    Float64x3,
    /// dvec4
    Float64x4,
    /// f16
    Float16,
    /// f16x2
    Float16x2,
    /// f16x4
    Float16x4,
    /// Normalized u8
    Unorm8,
    /// Normalized u8x2
    Unorm8x2,
    /// Normalized u8x4
    Unorm8x4,
    /// Normalized i8
    Snorm8,
    /// Normalized i8x2
    Snorm8x2,
    /// Normalized i8x4
    Snorm8x4,
    /// Normalized u16
    Unorm16,
    /// Normalized u16x2
    Unorm16x2,
    /// Normalized u16x4
    Unorm16x4,
    /// Normalized i16
    Snorm16,
    /// Normalized i16x2
    Snorm16x2,
    /// Normalized i16x4
    Snorm16x4,
}

impl VertexFormat {
    /// Returns the size in bytes
    pub const fn size(self) -> u32 {
        match self {
            Self::Float32 => 4,
            Self::Float32x2 => 8,
            Self::Float32x3 => 12,
            Self::Float32x4 => 16,
            Self::Int32 => 4,
            Self::Int32x2 => 8,
            Self::Int32x3 => 12,
            Self::Int32x4 => 16,
            Self::Uint32 => 4,
            Self::Uint32x2 => 8,
            Self::Uint32x3 => 12,
            Self::Uint32x4 => 16,
            Self::Float64 => 8,
            Self::Float64x2 => 16,
            Self::Float64x3 => 24,
            Self::Float64x4 => 32,
            Self::Float16 => 2,
            Self::Float16x2 => 4,
            Self::Float16x4 => 8,
            Self::Unorm8 | Self::Snorm8 => 1,
            Self::Unorm8x2 | Self::Snorm8x2 => 2,
            Self::Unorm8x4 | Self::Snorm8x4 => 4,
            Self::Unorm16 | Self::Snorm16 => 2,
            Self::Unorm16x2 | Self::Snorm16x2 => 4,
            Self::Unorm16x4 | Self::Snorm16x4 => 8,
        }
    }

    /// Returns the number of components
    pub const fn components(self) -> u32 {
        match self {
            Self::Float32 | Self::Int32 | Self::Uint32 | Self::Float64 | Self::Float16 |
            Self::Unorm8 | Self::Snorm8 | Self::Unorm16 | Self::Snorm16 => 1,
            Self::Float32x2 | Self::Int32x2 | Self::Uint32x2 | Self::Float64x2 | Self::Float16x2 |
            Self::Unorm8x2 | Self::Snorm8x2 | Self::Unorm16x2 | Self::Snorm16x2 => 2,
            Self::Float32x3 | Self::Int32x3 | Self::Uint32x3 | Self::Float64x3 => 3,
            Self::Float32x4 | Self::Int32x4 | Self::Uint32x4 | Self::Float64x4 | Self::Float16x4 |
            Self::Unorm8x4 | Self::Snorm8x4 | Self::Unorm16x4 | Self::Snorm16x4 => 4,
        }
    }
}

/// Specialization constant
#[derive(Clone, Debug)]
pub struct SpecializationConstant {
    /// Constant ID
    pub id: u32,
    /// Name (if available)
    pub name: Option<String>,
    /// Default value
    pub default_value: SpecConstValue,
}

/// Specialization constant value
#[derive(Clone, Copy, Debug)]
pub enum SpecConstValue {
    /// Boolean
    Bool(bool),
    /// Integer
    Int(i32),
    /// Unsigned integer
    Uint(u32),
    /// Float
    Float(f32),
}

/// Specialization info
#[derive(Clone, Debug)]
pub struct SpecializationInfo {
    /// Entries
    pub entries: Vec<SpecializationEntry>,
    /// Data
    pub data: Vec<u8>,
}

impl SpecializationInfo {
    /// Creates empty specialization info
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Adds a boolean constant
    pub fn add_bool(&mut self, id: u32, value: bool) {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&(value as u32).to_le_bytes());
        self.entries.push(SpecializationEntry {
            constant_id: id,
            offset,
            size: 4,
        });
    }

    /// Adds an integer constant
    pub fn add_int(&mut self, id: u32, value: i32) {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.entries.push(SpecializationEntry {
            constant_id: id,
            offset,
            size: 4,
        });
    }

    /// Adds an unsigned integer constant
    pub fn add_uint(&mut self, id: u32, value: u32) {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.entries.push(SpecializationEntry {
            constant_id: id,
            offset,
            size: 4,
        });
    }

    /// Adds a float constant
    pub fn add_float(&mut self, id: u32, value: f32) {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.entries.push(SpecializationEntry {
            constant_id: id,
            offset,
            size: 4,
        });
    }
}

impl Default for SpecializationInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Specialization entry
#[derive(Clone, Copy, Debug)]
pub struct SpecializationEntry {
    /// Constant ID
    pub constant_id: u32,
    /// Offset in data
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
}

/// Shader compiler options
#[derive(Clone, Debug, Default)]
pub struct ShaderCompilerOptions {
    /// Optimization level
    pub optimization_level: OptimizationLevel,
    /// Generate debug info
    pub debug_info: bool,
    /// Validate SPIR-V
    pub validate: bool,
    /// Target SPIR-V version
    pub target_spirv_version: Option<SpirvVersion>,
    /// Defines
    pub defines: Vec<(String, String)>,
    /// Include paths
    pub include_paths: Vec<String>,
}

/// Optimization level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum OptimizationLevel {
    /// No optimization
    None,
    /// Basic optimization
    #[default]
    Performance,
    /// Aggressive optimization
    Aggressive,
    /// Optimize for size
    Size,
}

/// SPIR-V version
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpirvVersion {
    /// Major version
    pub major: u8,
    /// Minor version
    pub minor: u8,
}

impl SpirvVersion {
    /// SPIR-V 1.0
    pub const V1_0: Self = Self { major: 1, minor: 0 };
    /// SPIR-V 1.1
    pub const V1_1: Self = Self { major: 1, minor: 1 };
    /// SPIR-V 1.2
    pub const V1_2: Self = Self { major: 1, minor: 2 };
    /// SPIR-V 1.3
    pub const V1_3: Self = Self { major: 1, minor: 3 };
    /// SPIR-V 1.4
    pub const V1_4: Self = Self { major: 1, minor: 4 };
    /// SPIR-V 1.5
    pub const V1_5: Self = Self { major: 1, minor: 5 };
    /// SPIR-V 1.6
    pub const V1_6: Self = Self { major: 1, minor: 6 };

    /// Returns version as u32 (e.g., 0x00010300 for 1.3)
    pub const fn as_u32(self) -> u32 {
        ((self.major as u32) << 16) | ((self.minor as u32) << 8)
    }
}

/// Shader cache key
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShaderCacheKey {
    /// Source hash
    pub source_hash: u64,
    /// Stage
    pub stage: ShaderStage,
    /// Entry point
    pub entry_point: String,
    /// Specialization hash
    pub specialization_hash: u64,
}

/// Shader library (collection of shader modules)
#[derive(Clone, Debug, Default)]
pub struct ShaderLibrary {
    /// Shader modules
    pub modules: Vec<ShaderLibraryEntry>,
}

impl ShaderLibrary {
    /// Creates an empty shader library
    pub const fn new() -> Self {
        Self { modules: Vec::new() }
    }

    /// Adds a shader module
    pub fn add(&mut self, name: String, handle: ShaderHandle, stage: ShaderStage) {
        self.modules.push(ShaderLibraryEntry { name, handle, stage });
    }

    /// Finds a shader by name
    pub fn find(&self, name: &str) -> Option<&ShaderLibraryEntry> {
        self.modules.iter().find(|m| m.name == name)
    }

    /// Finds a shader by stage
    pub fn find_by_stage(&self, stage: ShaderStage) -> Option<&ShaderLibraryEntry> {
        self.modules.iter().find(|m| m.stage == stage)
    }
}

/// Shader library entry
#[derive(Clone, Debug)]
pub struct ShaderLibraryEntry {
    /// Name
    pub name: String,
    /// Handle
    pub handle: ShaderHandle,
    /// Stage
    pub stage: ShaderStage,
}

/// Shader program (linked shaders)
#[derive(Clone, Debug)]
pub struct ShaderProgram {
    /// Vertex shader
    pub vertex: Option<ShaderHandle>,
    /// Fragment shader
    pub fragment: Option<ShaderHandle>,
    /// Geometry shader
    pub geometry: Option<ShaderHandle>,
    /// Tessellation control shader
    pub tessellation_control: Option<ShaderHandle>,
    /// Tessellation evaluation shader
    pub tessellation_evaluation: Option<ShaderHandle>,
    /// Compute shader
    pub compute: Option<ShaderHandle>,
}

impl ShaderProgram {
    /// Creates an empty shader program
    pub const fn new() -> Self {
        Self {
            vertex: None,
            fragment: None,
            geometry: None,
            tessellation_control: None,
            tessellation_evaluation: None,
            compute: None,
        }
    }

    /// Creates a graphics program (vertex + fragment)
    pub const fn graphics(vertex: ShaderHandle, fragment: ShaderHandle) -> Self {
        Self {
            vertex: Some(vertex),
            fragment: Some(fragment),
            geometry: None,
            tessellation_control: None,
            tessellation_evaluation: None,
            compute: None,
        }
    }

    /// Creates a compute program
    pub const fn compute(shader: ShaderHandle) -> Self {
        Self {
            vertex: None,
            fragment: None,
            geometry: None,
            tessellation_control: None,
            tessellation_evaluation: None,
            compute: Some(shader),
        }
    }

    /// Checks if this is a compute program
    pub const fn is_compute(&self) -> bool {
        self.compute.is_some()
    }

    /// Checks if this is a graphics program
    pub const fn is_graphics(&self) -> bool {
        self.vertex.is_some()
    }
}

impl Default for ShaderProgram {
    fn default() -> Self {
        Self::new()
    }
}
