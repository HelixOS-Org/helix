//! Shader specialization and reflection types
//!
//! This module provides types for shader specialization constants and reflection.

extern crate alloc;
use alloc::vec::Vec;

/// Specialization constant entry
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SpecializationMapEntry {
    /// Constant ID
    pub constant_id: u32,
    /// Offset in data
    pub offset: u32,
    /// Size of constant
    pub size: u32,
}

impl SpecializationMapEntry {
    /// Creates a new entry
    pub const fn new(constant_id: u32, offset: u32, size: u32) -> Self {
        Self {
            constant_id,
            offset,
            size,
        }
    }

    /// Creates a bool entry
    pub const fn bool_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// Creates an i32 entry
    pub const fn i32_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// Creates a u32 entry
    pub const fn u32_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// Creates an f32 entry
    pub const fn f32_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 4)
    }

    /// Creates an f64 entry
    pub const fn f64_entry(constant_id: u32, offset: u32) -> Self {
        Self::new(constant_id, offset, 8)
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
    pub const fn new() -> Self {
        Self {
            map_entries: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Adds a bool constant
    pub fn add_bool(mut self, constant_id: u32, value: bool) -> Self {
        let offset = self.data.len() as u32;
        let val: u32 = if value { 1 } else { 0 };
        self.data.extend_from_slice(&val.to_le_bytes());
        self.map_entries
            .push(SpecializationMapEntry::bool_entry(constant_id, offset));
        self
    }

    /// Adds an i32 constant
    pub fn add_i32(mut self, constant_id: u32, value: i32) -> Self {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.map_entries
            .push(SpecializationMapEntry::i32_entry(constant_id, offset));
        self
    }

    /// Adds a u32 constant
    pub fn add_u32(mut self, constant_id: u32, value: u32) -> Self {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.map_entries
            .push(SpecializationMapEntry::u32_entry(constant_id, offset));
        self
    }

    /// Adds an f32 constant
    pub fn add_f32(mut self, constant_id: u32, value: f32) -> Self {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.map_entries
            .push(SpecializationMapEntry::f32_entry(constant_id, offset));
        self
    }

    /// Adds an f64 constant
    pub fn add_f64(mut self, constant_id: u32, value: f64) -> Self {
        let offset = self.data.len() as u32;
        self.data.extend_from_slice(&value.to_le_bytes());
        self.map_entries
            .push(SpecializationMapEntry::f64_entry(constant_id, offset));
        self
    }
}

/// Shader reflection data
#[derive(Clone, Debug, Default)]
pub struct ShaderReflection {
    /// Entry points
    pub entry_points: Vec<EntryPointInfo>,
    /// Input variables
    pub inputs: Vec<ReflectedVariable>,
    /// Output variables
    pub outputs: Vec<ReflectedVariable>,
    /// Uniform buffers
    pub uniform_buffers: Vec<ReflectedBuffer>,
    /// Storage buffers
    pub storage_buffers: Vec<ReflectedBuffer>,
    /// Sampled images
    pub sampled_images: Vec<ReflectedImage>,
    /// Storage images
    pub storage_images: Vec<ReflectedImage>,
    /// Push constants
    pub push_constants: Vec<ReflectedPushConstant>,
    /// Specialization constants
    pub spec_constants: Vec<ReflectedSpecConstant>,
}

impl ShaderReflection {
    /// Creates empty reflection
    pub const fn new() -> Self {
        Self {
            entry_points: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            uniform_buffers: Vec::new(),
            storage_buffers: Vec::new(),
            sampled_images: Vec::new(),
            storage_images: Vec::new(),
            push_constants: Vec::new(),
            spec_constants: Vec::new(),
        }
    }

    /// Gets total binding count
    pub fn total_bindings(&self) -> usize {
        self.uniform_buffers.len()
            + self.storage_buffers.len()
            + self.sampled_images.len()
            + self.storage_images.len()
    }
}

/// Entry point info
#[derive(Clone, Debug)]
pub struct EntryPointInfo {
    /// Name
    pub name: Vec<u8>,
    /// Execution model
    pub execution_model: ExecutionModel,
    /// Workgroup size (for compute)
    pub workgroup_size: Option<[u32; 3]>,
}

/// Execution model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ExecutionModel {
    /// Vertex shader
    Vertex,
    /// Tessellation control
    TessellationControl,
    /// Tessellation evaluation
    TessellationEvaluation,
    /// Geometry shader
    Geometry,
    /// Fragment shader
    Fragment,
    /// Compute shader
    GLCompute,
    /// Kernel (OpenCL)
    Kernel,
    /// Task shader
    TaskNV,
    /// Mesh shader
    MeshNV,
    /// Ray generation
    RayGenerationKHR,
    /// Intersection
    IntersectionKHR,
    /// Any hit
    AnyHitKHR,
    /// Closest hit
    ClosestHitKHR,
    /// Miss
    MissKHR,
    /// Callable
    CallableKHR,
}

/// Reflected variable
#[derive(Clone, Debug)]
pub struct ReflectedVariable {
    /// Name
    pub name: Vec<u8>,
    /// Location
    pub location: u32,
    /// Type
    pub var_type: ReflectedType,
}

/// Reflected buffer
#[derive(Clone, Debug)]
pub struct ReflectedBuffer {
    /// Name
    pub name: Vec<u8>,
    /// Set
    pub set: u32,
    /// Binding
    pub binding: u32,
    /// Size
    pub size: u64,
    /// Members
    pub members: Vec<ReflectedMember>,
}

/// Reflected member
#[derive(Clone, Debug)]
pub struct ReflectedMember {
    /// Name
    pub name: Vec<u8>,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
    /// Type
    pub member_type: ReflectedType,
}

/// Reflected image
#[derive(Clone, Debug)]
pub struct ReflectedImage {
    /// Name
    pub name: Vec<u8>,
    /// Set
    pub set: u32,
    /// Binding
    pub binding: u32,
    /// Dimension
    pub dimension: ImageDimension,
    /// Arrayed
    pub arrayed: bool,
    /// Multisampled
    pub multisampled: bool,
    /// Sampled
    pub sampled: ImageSampled,
    /// Format
    pub format: ImageFormat,
}

/// Image dimension
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ImageDimension {
    /// 1D
    Dim1D,
    /// 2D
    Dim2D,
    /// 3D
    Dim3D,
    /// Cube
    Cube,
    /// Rectangle
    Rect,
    /// Buffer
    Buffer,
    /// Subpass data
    SubpassData,
}

/// Image sampled type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ImageSampled {
    /// Runtime
    Runtime,
    /// With sampler
    WithSampler,
    /// Without sampler
    WithoutSampler,
}

/// Image format for storage images
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ImageFormat {
    /// Unknown
    Unknown,
    /// RGBA32F
    Rgba32f,
    /// RGBA16F
    Rgba16f,
    /// R32F
    R32f,
    /// RGBA8
    Rgba8,
    /// RGBA8 Snorm
    Rgba8Snorm,
    /// RG32F
    Rg32f,
    /// RG16F
    Rg16f,
    /// R11G11B10F
    R11fG11fB10f,
    /// R16F
    R16f,
    /// RGBA16
    Rgba16,
    /// RGB10A2
    Rgb10A2,
    /// RG16
    Rg16,
    /// RG8
    Rg8,
    /// R16
    R16,
    /// R8
    R8,
    /// RGBA16 Snorm
    Rgba16Snorm,
    /// RG16 Snorm
    Rg16Snorm,
    /// RG8 Snorm
    Rg8Snorm,
    /// R16 Snorm
    R16Snorm,
    /// R8 Snorm
    R8Snorm,
    /// RGBA32I
    Rgba32i,
    /// RGBA16I
    Rgba16i,
    /// RGBA8I
    Rgba8i,
    /// R32I
    R32i,
    /// RG32I
    Rg32i,
    /// RG16I
    Rg16i,
    /// RG8I
    Rg8i,
    /// R16I
    R16i,
    /// R8I
    R8i,
    /// RGBA32UI
    Rgba32ui,
    /// RGBA16UI
    Rgba16ui,
    /// RGBA8UI
    Rgba8ui,
    /// R32UI
    R32ui,
    /// RGB10A2UI
    Rgb10a2ui,
    /// RG32UI
    Rg32ui,
    /// RG16UI
    Rg16ui,
    /// RG8UI
    Rg8ui,
    /// R16UI
    R16ui,
    /// R8UI
    R8ui,
    /// R64UI
    R64ui,
    /// R64I
    R64i,
}

/// Reflected push constant
#[derive(Clone, Debug)]
pub struct ReflectedPushConstant {
    /// Name
    pub name: Vec<u8>,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
    /// Stage flags
    pub stages: ShaderStages,
}

/// Shader stages
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ShaderStages(pub u32);

impl ShaderStages {
    /// Vertex
    pub const VERTEX: Self = Self(1 << 0);
    /// Tessellation control
    pub const TESS_CONTROL: Self = Self(1 << 1);
    /// Tessellation evaluation
    pub const TESS_EVAL: Self = Self(1 << 2);
    /// Geometry
    pub const GEOMETRY: Self = Self(1 << 3);
    /// Fragment
    pub const FRAGMENT: Self = Self(1 << 4);
    /// Compute
    pub const COMPUTE: Self = Self(1 << 5);
    /// All graphics
    pub const ALL_GRAPHICS: Self = Self(0x1F);
    /// All
    pub const ALL: Self = Self(0x7FFFFFFF);

    /// Combines stages
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Reflected specialization constant
#[derive(Clone, Debug)]
pub struct ReflectedSpecConstant {
    /// Name
    pub name: Vec<u8>,
    /// Constant ID
    pub constant_id: u32,
    /// Type
    pub constant_type: ReflectedType,
    /// Default value
    pub default_value: SpecConstantDefault,
}

/// Specialization constant default value
#[derive(Clone, Copy, Debug)]
pub enum SpecConstantDefault {
    /// Bool
    Bool(bool),
    /// Int
    Int(i32),
    /// UInt
    UInt(u32),
    /// Float
    Float(f32),
    /// Double
    Double(f64),
}

/// Reflected type
#[derive(Clone, Debug)]
pub enum ReflectedType {
    /// Void
    Void,
    /// Bool
    Bool,
    /// Int
    Int { width: u32, signed: bool },
    /// Float
    Float { width: u32 },
    /// Vector
    Vector {
        component: Box<ReflectedType>,
        count: u32,
    },
    /// Matrix
    Matrix {
        column_type: Box<ReflectedType>,
        column_count: u32,
    },
    /// Array
    Array {
        element: Box<ReflectedType>,
        length: u32,
    },
    /// Runtime array
    RuntimeArray { element: Box<ReflectedType> },
    /// Struct
    Struct {
        name: Vec<u8>,
        members: Vec<ReflectedMember>,
    },
    /// Image
    Image { dimension: ImageDimension },
    /// Sampler
    Sampler,
    /// Sampled image
    SampledImage { image: Box<ReflectedType> },
}

impl ReflectedType {
    /// Creates a vec2 type
    pub fn vec2() -> Self {
        Self::Vector {
            component: Box::new(Self::Float { width: 32 }),
            count: 2,
        }
    }

    /// Creates a vec3 type
    pub fn vec3() -> Self {
        Self::Vector {
            component: Box::new(Self::Float { width: 32 }),
            count: 3,
        }
    }

    /// Creates a vec4 type
    pub fn vec4() -> Self {
        Self::Vector {
            component: Box::new(Self::Float { width: 32 }),
            count: 4,
        }
    }

    /// Creates a mat4 type
    pub fn mat4() -> Self {
        Self::Matrix {
            column_type: Box::new(Self::vec4()),
            column_count: 4,
        }
    }

    /// Size in bytes
    pub fn size_bytes(&self) -> u32 {
        match self {
            Self::Void => 0,
            Self::Bool => 4,
            Self::Int { width, .. } | Self::Float { width } => width / 8,
            Self::Vector { component, count } => component.size_bytes() * count,
            Self::Matrix {
                column_type,
                column_count,
            } => column_type.size_bytes() * column_count,
            Self::Array { element, length } => element.size_bytes() * length,
            Self::RuntimeArray { .. } => 0,
            Self::Struct { members, .. } => members.iter().map(|m| m.size).sum(),
            Self::Image { .. } | Self::Sampler | Self::SampledImage { .. } => 0,
        }
    }
}

/// Vertex attribute format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VertexFormat {
    /// Float32
    Float32,
    /// Float32x2
    Float32x2,
    /// Float32x3
    Float32x3,
    /// Float32x4
    Float32x4,
    /// Int32
    Int32,
    /// Int32x2
    Int32x2,
    /// Int32x3
    Int32x3,
    /// Int32x4
    Int32x4,
    /// UInt32
    UInt32,
    /// UInt32x2
    UInt32x2,
    /// UInt32x3
    UInt32x3,
    /// UInt32x4
    UInt32x4,
    /// Sint8x2
    Sint8x2,
    /// Sint8x4
    Sint8x4,
    /// Uint8x2
    Uint8x2,
    /// Uint8x4
    Uint8x4,
    /// Snorm8x2
    Snorm8x2,
    /// Snorm8x4
    Snorm8x4,
    /// Unorm8x2
    Unorm8x2,
    /// Unorm8x4
    Unorm8x4,
    /// Sint16x2
    Sint16x2,
    /// Sint16x4
    Sint16x4,
    /// Uint16x2
    Uint16x2,
    /// Uint16x4
    Uint16x4,
    /// Snorm16x2
    Snorm16x2,
    /// Snorm16x4
    Snorm16x4,
    /// Unorm16x2
    Unorm16x2,
    /// Unorm16x4
    Unorm16x4,
    /// Float16x2
    Float16x2,
    /// Float16x4
    Float16x4,
}

impl VertexFormat {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Float32 | Self::Int32 | Self::UInt32 => 4,
            Self::Float32x2 | Self::Int32x2 | Self::UInt32x2 => 8,
            Self::Float32x3 | Self::Int32x3 | Self::UInt32x3 => 12,
            Self::Float32x4 | Self::Int32x4 | Self::UInt32x4 => 16,
            Self::Sint8x2 | Self::Uint8x2 | Self::Snorm8x2 | Self::Unorm8x2 => 2,
            Self::Sint8x4 | Self::Uint8x4 | Self::Snorm8x4 | Self::Unorm8x4 => 4,
            Self::Sint16x2
            | Self::Uint16x2
            | Self::Snorm16x2
            | Self::Unorm16x2
            | Self::Float16x2 => 4,
            Self::Sint16x4
            | Self::Uint16x4
            | Self::Snorm16x4
            | Self::Unorm16x4
            | Self::Float16x4 => 8,
        }
    }

    /// Component count
    pub const fn component_count(&self) -> u32 {
        match self {
            Self::Float32 | Self::Int32 | Self::UInt32 => 1,
            Self::Float32x2
            | Self::Int32x2
            | Self::UInt32x2
            | Self::Sint8x2
            | Self::Uint8x2
            | Self::Snorm8x2
            | Self::Unorm8x2
            | Self::Sint16x2
            | Self::Uint16x2
            | Self::Snorm16x2
            | Self::Unorm16x2
            | Self::Float16x2 => 2,
            Self::Float32x3 | Self::Int32x3 | Self::UInt32x3 => 3,
            Self::Float32x4
            | Self::Int32x4
            | Self::UInt32x4
            | Self::Sint8x4
            | Self::Uint8x4
            | Self::Snorm8x4
            | Self::Unorm8x4
            | Self::Sint16x4
            | Self::Uint16x4
            | Self::Snorm16x4
            | Self::Unorm16x4
            | Self::Float16x4 => 4,
        }
    }
}

/// Vertex input rate
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VertexInputRate {
    /// Per vertex
    #[default]
    Vertex,
    /// Per instance
    Instance,
}

/// Vertex binding description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexBindingDescription {
    /// Binding index
    pub binding: u32,
    /// Stride
    pub stride: u32,
    /// Input rate
    pub input_rate: VertexInputRate,
}

impl VertexBindingDescription {
    /// Creates a new binding
    pub const fn new(binding: u32, stride: u32) -> Self {
        Self {
            binding,
            stride,
            input_rate: VertexInputRate::Vertex,
        }
    }

    /// Per instance
    pub const fn per_instance(mut self) -> Self {
        self.input_rate = VertexInputRate::Instance;
        self
    }
}

/// Vertex attribute description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexAttributeDescription {
    /// Location
    pub location: u32,
    /// Binding
    pub binding: u32,
    /// Format
    pub format: VertexFormat,
    /// Offset
    pub offset: u32,
}

impl VertexAttributeDescription {
    /// Creates a new attribute
    pub const fn new(location: u32, binding: u32, format: VertexFormat, offset: u32) -> Self {
        Self {
            location,
            binding,
            format,
            offset,
        }
    }
}

/// Vertex input state
#[derive(Clone, Debug, Default)]
pub struct VertexInputState {
    /// Bindings
    pub bindings: Vec<VertexBindingDescription>,
    /// Attributes
    pub attributes: Vec<VertexAttributeDescription>,
}

impl VertexInputState {
    /// Creates new vertex input state
    pub const fn new() -> Self {
        Self {
            bindings: Vec::new(),
            attributes: Vec::new(),
        }
    }

    /// Adds a binding
    pub fn add_binding(mut self, binding: VertexBindingDescription) -> Self {
        self.bindings.push(binding);
        self
    }

    /// Adds an attribute
    pub fn add_attribute(mut self, attribute: VertexAttributeDescription) -> Self {
        self.attributes.push(attribute);
        self
    }
}
