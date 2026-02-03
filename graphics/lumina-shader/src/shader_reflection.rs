//! Shader Reflection Types for Lumina
//!
//! This module provides comprehensive shader reflection and introspection
//! for analyzing SPIR-V shaders at runtime.

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Shader Reflection Info
// ============================================================================

/// Reflected shader module information
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ShaderReflection {
    /// Shader stage
    pub stage: ShaderStage,
    /// Entry point name
    pub entry_point: String,
    /// Source language
    pub source_language: SourceLanguage,
    /// Descriptor bindings
    pub bindings: Vec<BindingReflection>,
    /// Push constant ranges
    pub push_constants: Vec<PushConstantReflection>,
    /// Input variables
    pub inputs: Vec<VariableReflection>,
    /// Output variables
    pub outputs: Vec<VariableReflection>,
    /// Specialization constants
    pub specialization_constants: Vec<SpecConstantReflection>,
    /// Workgroup size (for compute)
    pub workgroup_size: Option<WorkgroupSize>,
    /// Shader capabilities used
    pub capabilities: Vec<Capability>,
    /// Extensions used
    pub extensions: Vec<String>,
}

impl ShaderReflection {
    /// Creates a new empty reflection
    #[inline]
    pub fn new(stage: ShaderStage, entry_point: String) -> Self {
        Self {
            stage,
            entry_point,
            source_language: SourceLanguage::Unknown,
            bindings: Vec::new(),
            push_constants: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            specialization_constants: Vec::new(),
            workgroup_size: None,
            capabilities: Vec::new(),
            extensions: Vec::new(),
        }
    }

    /// Gets bindings for a specific set
    #[inline]
    pub fn bindings_for_set(&self, set: u32) -> impl Iterator<Item = &BindingReflection> {
        self.bindings.iter().filter(move |b| b.set == set)
    }

    /// Gets the maximum set index used
    #[inline]
    pub fn max_set(&self) -> u32 {
        self.bindings.iter().map(|b| b.set).max().unwrap_or(0)
    }

    /// Checks if the shader uses a specific capability
    #[inline]
    pub fn uses_capability(&self, cap: Capability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Checks if the shader uses push constants
    #[inline]
    pub fn uses_push_constants(&self) -> bool {
        !self.push_constants.is_empty()
    }

    /// Gets total push constant size
    #[inline]
    pub fn push_constant_size(&self) -> u32 {
        self.push_constants
            .iter()
            .map(|p| p.offset + p.size)
            .max()
            .unwrap_or(0)
    }

    /// Checks if this is a compute shader
    #[inline]
    pub fn is_compute(&self) -> bool {
        matches!(self.stage, ShaderStage::Compute)
    }

    /// Checks if this is a graphics shader
    #[inline]
    pub fn is_graphics(&self) -> bool {
        matches!(
            self.stage,
            ShaderStage::Vertex
                | ShaderStage::TessellationControl
                | ShaderStage::TessellationEvaluation
                | ShaderStage::Geometry
                | ShaderStage::Fragment
        )
    }

    /// Checks if this is a ray tracing shader
    #[inline]
    pub fn is_ray_tracing(&self) -> bool {
        matches!(
            self.stage,
            ShaderStage::RayGeneration
                | ShaderStage::AnyHit
                | ShaderStage::ClosestHit
                | ShaderStage::Miss
                | ShaderStage::Intersection
                | ShaderStage::Callable
        )
    }
}

/// Shader stage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ShaderStage {
    /// Vertex shader
    Vertex              = 0,
    /// Tessellation control shader
    TessellationControl = 1,
    /// Tessellation evaluation shader
    TessellationEvaluation = 2,
    /// Geometry shader
    Geometry            = 3,
    /// Fragment/pixel shader
    Fragment            = 4,
    /// Compute shader
    Compute             = 5,
    /// Task shader
    Task                = 6,
    /// Mesh shader
    Mesh                = 7,
    /// Ray generation shader
    RayGeneration       = 8,
    /// Any-hit shader
    AnyHit              = 9,
    /// Closest-hit shader
    ClosestHit          = 10,
    /// Miss shader
    Miss                = 11,
    /// Intersection shader
    Intersection        = 12,
    /// Callable shader
    Callable            = 13,
}

impl ShaderStage {
    /// Returns the stage name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::TessellationControl => "tess_control",
            Self::TessellationEvaluation => "tess_eval",
            Self::Geometry => "geometry",
            Self::Fragment => "fragment",
            Self::Compute => "compute",
            Self::Task => "task",
            Self::Mesh => "mesh",
            Self::RayGeneration => "ray_gen",
            Self::AnyHit => "any_hit",
            Self::ClosestHit => "closest_hit",
            Self::Miss => "miss",
            Self::Intersection => "intersection",
            Self::Callable => "callable",
        }
    }

    /// Returns the SPIR-V execution model
    #[inline]
    pub const fn execution_model(&self) -> u32 {
        match self {
            Self::Vertex => 0,
            Self::TessellationControl => 1,
            Self::TessellationEvaluation => 2,
            Self::Geometry => 3,
            Self::Fragment => 4,
            Self::Compute => 5,
            Self::Task => 5267,
            Self::Mesh => 5268,
            Self::RayGeneration => 5313,
            Self::Intersection => 5314,
            Self::AnyHit => 5315,
            Self::ClosestHit => 5316,
            Self::Miss => 5317,
            Self::Callable => 5318,
        }
    }
}

/// Source language
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SourceLanguage {
    /// Unknown source
    #[default]
    Unknown      = 0,
    /// ESSL (OpenGL ES)
    Essl         = 1,
    /// GLSL
    Glsl         = 2,
    /// OpenCL C
    OpenClC      = 3,
    /// OpenCL C++
    OpenClCpp    = 4,
    /// HLSL
    Hlsl         = 5,
    /// C++ for OpenCL
    CppForOpenCl = 6,
    /// SYCL
    Sycl         = 7,
}

// ============================================================================
// Binding Reflection
// ============================================================================

/// Reflected binding information
#[derive(Clone, Debug)]
#[repr(C)]
pub struct BindingReflection {
    /// Binding name
    pub name: String,
    /// Descriptor set
    pub set: u32,
    /// Binding number
    pub binding: u32,
    /// Descriptor type
    pub descriptor_type: DescriptorType,
    /// Resource type
    pub resource_type: ResourceType,
    /// Descriptor count (for arrays)
    pub count: u32,
    /// Block size (for uniform/storage buffers)
    pub block_size: u32,
    /// Block members (for uniform/storage buffers)
    pub members: Vec<MemberReflection>,
    /// Image format (for storage images)
    pub image_format: ImageFormat,
    /// Image dimensions
    pub image_dim: ImageDim,
    /// Is array (for image arrays)
    pub is_array: bool,
    /// Is multisampled
    pub is_multisample: bool,
    /// Is depth image
    pub is_depth: bool,
    /// Access flags (for storage resources)
    pub access: AccessFlags,
}

impl BindingReflection {
    /// Creates a new binding reflection
    #[inline]
    pub fn new(name: String, set: u32, binding: u32, descriptor_type: DescriptorType) -> Self {
        Self {
            name,
            set,
            binding,
            descriptor_type,
            resource_type: ResourceType::Unknown,
            count: 1,
            block_size: 0,
            members: Vec::new(),
            image_format: ImageFormat::Unknown,
            image_dim: ImageDim::Dim2D,
            is_array: false,
            is_multisample: false,
            is_depth: false,
            access: AccessFlags::READ,
        }
    }

    /// Checks if this is a buffer
    #[inline]
    pub fn is_buffer(&self) -> bool {
        matches!(
            self.descriptor_type,
            DescriptorType::UniformBuffer
                | DescriptorType::StorageBuffer
                | DescriptorType::UniformBufferDynamic
                | DescriptorType::StorageBufferDynamic
        )
    }

    /// Checks if this is an image
    #[inline]
    pub fn is_image(&self) -> bool {
        matches!(
            self.descriptor_type,
            DescriptorType::SampledImage
                | DescriptorType::StorageImage
                | DescriptorType::CombinedImageSampler
                | DescriptorType::InputAttachment
        )
    }

    /// Checks if this is a sampler
    #[inline]
    pub fn is_sampler(&self) -> bool {
        matches!(
            self.descriptor_type,
            DescriptorType::Sampler | DescriptorType::CombinedImageSampler
        )
    }

    /// Checks if this is read-only
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.access.contains(AccessFlags::READ) && !self.access.contains(AccessFlags::WRITE)
    }

    /// Checks if this is write-only
    #[inline]
    pub fn is_write_only(&self) -> bool {
        !self.access.contains(AccessFlags::READ) && self.access.contains(AccessFlags::WRITE)
    }
}

/// Descriptor type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DescriptorType {
    /// Sampler
    Sampler              = 0,
    /// Combined image sampler
    CombinedImageSampler = 1,
    /// Sampled image
    #[default]
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
    /// Acceleration structure
    AccelerationStructure = 11,
}

/// Resource type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ResourceType {
    /// Unknown
    #[default]
    Unknown           = 0,
    /// Uniform buffer
    UniformBuffer     = 1,
    /// Storage buffer
    StorageBuffer     = 2,
    /// Texture 1D
    Texture1D         = 3,
    /// Texture 2D
    Texture2D         = 4,
    /// Texture 3D
    Texture3D         = 5,
    /// Texture Cube
    TextureCube       = 6,
    /// Texture 1D Array
    Texture1DArray    = 7,
    /// Texture 2D Array
    Texture2DArray    = 8,
    /// Texture Cube Array
    TextureCubeArray  = 9,
    /// Texture 2D MS
    Texture2DMS       = 10,
    /// Texture 2D MS Array
    Texture2DMSArray  = 11,
    /// Storage Texture 1D
    StorageTexture1D  = 12,
    /// Storage Texture 2D
    StorageTexture2D  = 13,
    /// Storage Texture 3D
    StorageTexture3D  = 14,
    /// Sampler
    Sampler           = 15,
    /// Comparison sampler
    SamplerComparison = 16,
    /// Acceleration structure
    AccelerationStructure = 17,
}

/// Image format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageFormat {
    /// Unknown format
    #[default]
    Unknown      = 0,
    /// RGBA32F
    Rgba32f      = 1,
    /// RGBA16F
    Rgba16f      = 2,
    /// RG32F
    Rg32f        = 3,
    /// RG16F
    Rg16f        = 4,
    /// R11fG11fB10f
    R11fG11fB10f = 5,
    /// R32F
    R32f         = 6,
    /// R16F
    R16f         = 7,
    /// RGBA16
    Rgba16       = 8,
    /// RGB10A2
    Rgb10A2      = 9,
    /// RGBA8
    Rgba8        = 10,
    /// RG16
    Rg16         = 11,
    /// RG8
    Rg8          = 12,
    /// R16
    R16          = 13,
    /// R8
    R8           = 14,
    /// RGBA16Snorm
    Rgba16Snorm  = 15,
    /// RGBA8Snorm
    Rgba8Snorm   = 16,
    /// RG16Snorm
    Rg16Snorm    = 17,
    /// RG8Snorm
    Rg8Snorm     = 18,
    /// R16Snorm
    R16Snorm     = 19,
    /// R8Snorm
    R8Snorm      = 20,
    /// RGBA32I
    Rgba32i      = 21,
    /// RGBA16I
    Rgba16i      = 22,
    /// RGBA8I
    Rgba8i       = 23,
    /// R32I
    R32i         = 24,
    /// RG32I
    Rg32i        = 25,
    /// RG16I
    Rg16i        = 26,
    /// RG8I
    Rg8i         = 27,
    /// R16I
    R16i         = 28,
    /// R8I
    R8i          = 29,
    /// RGBA32UI
    Rgba32ui     = 30,
    /// RGBA16UI
    Rgba16ui     = 31,
    /// RGBA8UI
    Rgba8ui      = 32,
    /// R32UI
    R32ui        = 33,
    /// RGB10A2UI
    Rgb10a2ui    = 34,
    /// RG32UI
    Rg32ui       = 35,
    /// RG16UI
    Rg16ui       = 36,
    /// RG8UI
    Rg8ui        = 37,
    /// R16UI
    R16ui        = 38,
    /// R8UI
    R8ui         = 39,
    /// R64UI
    R64ui        = 40,
    /// R64I
    R64i         = 41,
}

/// Image dimensions
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageDim {
    /// 1D
    Dim1D          = 0,
    /// 2D
    #[default]
    Dim2D          = 1,
    /// 3D
    Dim3D          = 2,
    /// Cube
    DimCube        = 3,
    /// Rectangle
    DimRect        = 4,
    /// Buffer
    DimBuffer      = 5,
    /// Subpass data
    DimSubpassData = 6,
}

/// Access flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct AccessFlags(pub u32);

impl AccessFlags {
    /// No access
    pub const NONE: Self = Self(0);
    /// Read access
    pub const READ: Self = Self(1 << 0);
    /// Write access
    pub const WRITE: Self = Self(1 << 1);
    /// Read and write
    pub const READ_WRITE: Self = Self(Self::READ.0 | Self::WRITE.0);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Member Reflection
// ============================================================================

/// Reflected struct member information
#[derive(Clone, Debug)]
#[repr(C)]
pub struct MemberReflection {
    /// Member name
    pub name: String,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
    /// Type
    pub member_type: TypeReflection,
    /// Array stride (0 if not array)
    pub array_stride: u32,
    /// Matrix stride (0 if not matrix)
    pub matrix_stride: u32,
    /// Row major matrix (false = column major)
    pub row_major: bool,
}

impl MemberReflection {
    /// Creates a new member reflection
    #[inline]
    pub fn new(name: String, offset: u32, size: u32, member_type: TypeReflection) -> Self {
        Self {
            name,
            offset,
            size,
            member_type,
            array_stride: 0,
            matrix_stride: 0,
            row_major: false,
        }
    }

    /// Checks if this is an array
    #[inline]
    pub const fn is_array(&self) -> bool {
        self.array_stride > 0 || matches!(self.member_type.base, BaseType::Array)
    }

    /// Checks if this is a matrix
    #[inline]
    pub const fn is_matrix(&self) -> bool {
        self.matrix_stride > 0 || matches!(self.member_type.base, BaseType::Matrix)
    }
}

/// Type reflection
#[derive(Clone, Debug)]
#[repr(C)]
pub struct TypeReflection {
    /// Base type
    pub base: BaseType,
    /// Vector size (1-4)
    pub vec_size: u8,
    /// Matrix columns (1-4)
    pub columns: u8,
    /// Array size (0 = runtime array)
    pub array_size: u32,
    /// Bit width (8, 16, 32, 64)
    pub bit_width: u8,
    /// Nested type (for structs/arrays)
    pub nested: Option<alloc::boxed::Box<TypeReflection>>,
    /// Struct members (for struct types)
    pub members: Vec<MemberReflection>,
}

impl TypeReflection {
    /// Creates a scalar type
    #[inline]
    pub const fn scalar(base: BaseType, bit_width: u8) -> Self {
        Self {
            base,
            vec_size: 1,
            columns: 1,
            array_size: 1,
            bit_width,
            nested: None,
            members: Vec::new(),
        }
    }

    /// Creates a vector type
    #[inline]
    pub const fn vector(base: BaseType, size: u8, bit_width: u8) -> Self {
        Self {
            base: BaseType::Vector,
            vec_size: size,
            columns: 1,
            array_size: 1,
            bit_width,
            nested: None,
            members: Vec::new(),
        }
    }

    /// Creates a matrix type
    #[inline]
    pub const fn matrix(rows: u8, cols: u8, bit_width: u8) -> Self {
        Self {
            base: BaseType::Matrix,
            vec_size: rows,
            columns: cols,
            array_size: 1,
            bit_width,
            nested: None,
            members: Vec::new(),
        }
    }

    /// float
    pub const FLOAT: Self = Self::scalar(BaseType::Float, 32);
    /// vec2
    pub const VEC2: Self = Self::vector(BaseType::Float, 2, 32);
    /// vec3
    pub const VEC3: Self = Self::vector(BaseType::Float, 3, 32);
    /// vec4
    pub const VEC4: Self = Self::vector(BaseType::Float, 4, 32);
    /// mat3
    pub const MAT3: Self = Self::matrix(3, 3, 32);
    /// mat4
    pub const MAT4: Self = Self::matrix(4, 4, 32);
    /// int
    pub const INT: Self = Self::scalar(BaseType::Int, 32);
    /// uint
    pub const UINT: Self = Self::scalar(BaseType::UInt, 32);
    /// bool
    pub const BOOL: Self = Self::scalar(BaseType::Bool, 32);

    /// Size in bytes (for std140 layout)
    #[inline]
    pub const fn size_std140(&self) -> u32 {
        let component_size = (self.bit_width as u32 + 7) / 8;
        match self.base {
            BaseType::Bool => 4, // Bools are 4 bytes in GLSL
            BaseType::Int | BaseType::UInt | BaseType::Float => component_size,
            BaseType::Vector => {
                let vec_size = self.vec_size as u32;
                if vec_size == 3 {
                    4 * component_size // vec3 is padded to vec4
                } else {
                    vec_size * component_size
                }
            },
            BaseType::Matrix => {
                let col_size = if self.vec_size == 3 {
                    4
                } else {
                    self.vec_size as u32
                };
                col_size * component_size * self.columns as u32
            },
            BaseType::Array | BaseType::Struct => 0, // Requires nested type info
            _ => 0,
        }
    }

    /// Alignment in bytes (for std140 layout)
    #[inline]
    pub const fn alignment_std140(&self) -> u32 {
        let component_size = (self.bit_width as u32 + 7) / 8;
        match self.base {
            BaseType::Bool | BaseType::Int | BaseType::UInt | BaseType::Float => component_size,
            BaseType::Vector => {
                let vec_size = self.vec_size as u32;
                if vec_size == 2 {
                    2 * component_size
                } else {
                    4 * component_size // vec3 and vec4 align to 16
                }
            },
            BaseType::Matrix | BaseType::Array | BaseType::Struct => 16, // Round up to vec4
            _ => component_size,
        }
    }
}

/// Base type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BaseType {
    /// Unknown type
    #[default]
    Unknown      = 0,
    /// Void
    Void         = 1,
    /// Boolean
    Bool         = 2,
    /// Signed integer
    Int          = 3,
    /// Unsigned integer
    UInt         = 4,
    /// Floating point
    Float        = 5,
    /// Double precision float
    Double       = 6,
    /// Vector
    Vector       = 7,
    /// Matrix
    Matrix       = 8,
    /// Image
    Image        = 9,
    /// Sampler
    Sampler      = 10,
    /// Sampled image
    SampledImage = 11,
    /// Array
    Array        = 12,
    /// Struct
    Struct       = 13,
    /// Pointer
    Pointer      = 14,
    /// Acceleration structure
    AccelerationStructure = 15,
    /// Ray query
    RayQuery     = 16,
}

impl BaseType {
    /// Checks if this is a numeric type
    #[inline]
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::UInt | Self::Float | Self::Double)
    }

    /// Checks if this is an integer type
    #[inline]
    pub const fn is_integer(&self) -> bool {
        matches!(self, Self::Int | Self::UInt)
    }

    /// Checks if this is a floating-point type
    #[inline]
    pub const fn is_float(&self) -> bool {
        matches!(self, Self::Float | Self::Double)
    }
}

// ============================================================================
// Variable Reflection
// ============================================================================

/// Reflected variable (input/output)
#[derive(Clone, Debug)]
#[repr(C)]
pub struct VariableReflection {
    /// Variable name
    pub name: String,
    /// Location
    pub location: u32,
    /// Component (for sub-location)
    pub component: u32,
    /// Variable type
    pub var_type: TypeReflection,
    /// Semantic (for HLSL)
    pub semantic: Option<String>,
    /// Built-in type (if applicable)
    pub builtin: Option<BuiltIn>,
    /// Is flat (no interpolation)
    pub is_flat: bool,
    /// Interpolation type
    pub interpolation: Interpolation,
}

impl VariableReflection {
    /// Creates a new variable reflection
    #[inline]
    pub fn new(name: String, location: u32, var_type: TypeReflection) -> Self {
        Self {
            name,
            location,
            component: 0,
            var_type,
            semantic: None,
            builtin: None,
            is_flat: false,
            interpolation: Interpolation::Smooth,
        }
    }

    /// Checks if this is a built-in
    #[inline]
    pub const fn is_builtin(&self) -> bool {
        self.builtin.is_some()
    }
}

/// Built-in variable
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum BuiltIn {
    /// Position
    Position             = 0,
    /// Point size
    PointSize            = 1,
    /// Clip distance
    ClipDistance         = 3,
    /// Cull distance
    CullDistance         = 4,
    /// Vertex ID
    VertexId             = 5,
    /// Instance ID
    InstanceId           = 6,
    /// Primitive ID
    PrimitiveId          = 7,
    /// Invocation ID
    InvocationId         = 8,
    /// Layer
    Layer                = 9,
    /// Viewport index
    ViewportIndex        = 10,
    /// Tess level outer
    TessLevelOuter       = 11,
    /// Tess level inner
    TessLevelInner       = 12,
    /// Tess coord
    TessCoord            = 13,
    /// Patch vertices
    PatchVertices        = 14,
    /// Fragment coord
    FragCoord            = 15,
    /// Point coord
    PointCoord           = 16,
    /// Front facing
    FrontFacing          = 17,
    /// Sample ID
    SampleId             = 18,
    /// Sample position
    SamplePosition       = 19,
    /// Sample mask
    SampleMask           = 20,
    /// Fragment depth
    FragDepth            = 22,
    /// Helper invocation
    HelperInvocation     = 23,
    /// Local invocation ID
    LocalInvocationId    = 27,
    /// Global invocation ID
    GlobalInvocationId   = 28,
    /// Workgroup ID
    WorkgroupId          = 26,
    /// Workgroup size
    WorkgroupSize        = 25,
    /// Num workgroups
    NumWorkgroups        = 24,
    /// Local invocation index
    LocalInvocationIndex = 29,
    /// Vertex index
    VertexIndex          = 42,
    /// Instance index
    InstanceIndex        = 43,
    /// Base vertex
    BaseVertex           = 4424,
    /// Base instance
    BaseInstance         = 4425,
    /// Draw index
    DrawIndex            = 4426,
    /// View index
    ViewIndex            = 4440,
    /// Launch ID
    LaunchIdKHR          = 5319,
    /// Launch size
    LaunchSizeKHR        = 5320,
    /// World ray origin
    WorldRayOriginKHR    = 5321,
    /// World ray direction
    WorldRayDirectionKHR = 5322,
    /// Object ray origin
    ObjectRayOriginKHR   = 5323,
    /// Object ray direction
    ObjectRayDirectionKHR = 5324,
    /// Ray T min
    RayTminKHR           = 5325,
    /// Ray T max
    RayTmaxKHR           = 5326,
    /// Incoming ray flags
    IncomingRayFlagsKHR  = 5351,
    /// Hit kind
    HitKindKHR           = 5333,
}

/// Interpolation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum Interpolation {
    /// Smooth interpolation
    #[default]
    Smooth        = 0,
    /// Flat (no interpolation)
    Flat          = 1,
    /// No perspective
    NoPerspective = 2,
    /// Centroid
    Centroid      = 3,
    /// Sample
    Sample        = 4,
}

// ============================================================================
// Push Constant Reflection
// ============================================================================

/// Reflected push constant
#[derive(Clone, Debug)]
#[repr(C)]
pub struct PushConstantReflection {
    /// Name
    pub name: String,
    /// Offset in bytes
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
    /// Members
    pub members: Vec<MemberReflection>,
    /// Stage flags
    pub stages: ShaderStageFlags,
}

impl PushConstantReflection {
    /// Creates a new push constant reflection
    #[inline]
    pub fn new(name: String, offset: u32, size: u32) -> Self {
        Self {
            name,
            offset,
            size,
            members: Vec::new(),
            stages: ShaderStageFlags::ALL,
        }
    }
}

/// Shader stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ShaderStageFlags(pub u32);

impl ShaderStageFlags {
    /// No stages
    pub const NONE: Self = Self(0);
    /// Vertex
    pub const VERTEX: Self = Self(1 << 0);
    /// Tessellation control
    pub const TESS_CONTROL: Self = Self(1 << 1);
    /// Tessellation evaluation
    pub const TESS_EVALUATION: Self = Self(1 << 2);
    /// Geometry
    pub const GEOMETRY: Self = Self(1 << 3);
    /// Fragment
    pub const FRAGMENT: Self = Self(1 << 4);
    /// Compute
    pub const COMPUTE: Self = Self(1 << 5);
    /// Task
    pub const TASK: Self = Self(1 << 6);
    /// Mesh
    pub const MESH: Self = Self(1 << 7);
    /// Ray generation
    pub const RAY_GEN: Self = Self(1 << 8);
    /// Any hit
    pub const ANY_HIT: Self = Self(1 << 9);
    /// Closest hit
    pub const CLOSEST_HIT: Self = Self(1 << 10);
    /// Miss
    pub const MISS: Self = Self(1 << 11);
    /// Intersection
    pub const INTERSECTION: Self = Self(1 << 12);
    /// Callable
    pub const CALLABLE: Self = Self(1 << 13);

    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(
        Self::VERTEX.0
            | Self::TESS_CONTROL.0
            | Self::TESS_EVALUATION.0
            | Self::GEOMETRY.0
            | Self::FRAGMENT.0,
    );

    /// All ray tracing stages
    pub const ALL_RAY_TRACING: Self = Self(
        Self::RAY_GEN.0
            | Self::ANY_HIT.0
            | Self::CLOSEST_HIT.0
            | Self::MISS.0
            | Self::INTERSECTION.0
            | Self::CALLABLE.0,
    );

    /// All stages
    pub const ALL: Self = Self(0x3FFF);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Specialization Constant Reflection
// ============================================================================

/// Reflected specialization constant
#[derive(Clone, Debug)]
#[repr(C)]
pub struct SpecConstantReflection {
    /// Constant name
    pub name: String,
    /// Constant ID
    pub constant_id: u32,
    /// Type
    pub constant_type: TypeReflection,
    /// Default value (as u64, interpret based on type)
    pub default_value: u64,
}

impl SpecConstantReflection {
    /// Creates a new spec constant reflection
    #[inline]
    pub fn new(name: String, constant_id: u32, constant_type: TypeReflection) -> Self {
        Self {
            name,
            constant_id,
            constant_type,
            default_value: 0,
        }
    }

    /// Gets the default as a float
    #[inline]
    pub fn default_as_f32(&self) -> f32 {
        f32::from_bits(self.default_value as u32)
    }

    /// Gets the default as an int
    #[inline]
    pub fn default_as_i32(&self) -> i32 {
        self.default_value as i32
    }

    /// Gets the default as a uint
    #[inline]
    pub fn default_as_u32(&self) -> u32 {
        self.default_value as u32
    }

    /// Gets the default as a bool
    #[inline]
    pub fn default_as_bool(&self) -> bool {
        self.default_value != 0
    }
}

// ============================================================================
// Workgroup Size
// ============================================================================

/// Workgroup size for compute shaders
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct WorkgroupSize {
    /// X dimension
    pub x: u32,
    /// Y dimension
    pub y: u32,
    /// Z dimension
    pub z: u32,
    /// Specialization constant IDs (0 = not a spec constant)
    pub x_id: u32,
    /// Y spec constant ID
    pub y_id: u32,
    /// Z spec constant ID
    pub z_id: u32,
}

impl WorkgroupSize {
    /// Creates a new workgroup size
    #[inline]
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            x,
            y,
            z,
            x_id: 0,
            y_id: 0,
            z_id: 0,
        }
    }

    /// 1D workgroup (64)
    pub const WORKGROUP_64: Self = Self::new(64, 1, 1);
    /// 1D workgroup (128)
    pub const WORKGROUP_128: Self = Self::new(128, 1, 1);
    /// 1D workgroup (256)
    pub const WORKGROUP_256: Self = Self::new(256, 1, 1);
    /// 2D workgroup (8x8)
    pub const WORKGROUP_8X8: Self = Self::new(8, 8, 1);
    /// 2D workgroup (16x16)
    pub const WORKGROUP_16X16: Self = Self::new(16, 16, 1);
    /// 3D workgroup (4x4x4)
    pub const WORKGROUP_4X4X4: Self = Self::new(4, 4, 4);
    /// 3D workgroup (8x8x8)
    pub const WORKGROUP_8X8X8: Self = Self::new(8, 8, 8);

    /// Total invocations
    #[inline]
    pub const fn total_invocations(&self) -> u32 {
        self.x * self.y * self.z
    }

    /// Checks if any dimension uses a spec constant
    #[inline]
    pub const fn uses_spec_constants(&self) -> bool {
        self.x_id != 0 || self.y_id != 0 || self.z_id != 0
    }
}

impl Default for WorkgroupSize {
    fn default() -> Self {
        Self::new(1, 1, 1)
    }
}

// ============================================================================
// Capability
// ============================================================================

/// SPIR-V capability
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum Capability {
    /// Matrix
    Matrix               = 0,
    /// Shader
    Shader               = 1,
    /// Geometry
    Geometry             = 2,
    /// Tessellation
    Tessellation         = 3,
    /// Addresses
    Addresses            = 4,
    /// Linkage
    Linkage              = 5,
    /// Kernel
    Kernel               = 6,
    /// Float16
    Float16              = 9,
    /// Float64
    Float64              = 10,
    /// Int64
    Int64                = 11,
    /// Groups
    Groups               = 18,
    /// AtomicStorage
    AtomicStorage        = 21,
    /// Int16
    Int16                = 22,
    /// TessellationPointSize
    TessellationPointSize = 23,
    /// GeometryPointSize
    GeometryPointSize    = 24,
    /// ImageGatherExtended
    ImageGatherExtended  = 25,
    /// StorageImageMultisample
    StorageImageMultisample = 27,
    /// UniformBufferArrayDynamicIndexing
    UniformBufferArrayDynamicIndexing = 28,
    /// SampledImageArrayDynamicIndexing
    SampledImageArrayDynamicIndexing = 29,
    /// StorageBufferArrayDynamicIndexing
    StorageBufferArrayDynamicIndexing = 30,
    /// StorageImageArrayDynamicIndexing
    StorageImageArrayDynamicIndexing = 31,
    /// ClipDistance
    ClipDistance         = 32,
    /// CullDistance
    CullDistance         = 33,
    /// SampleRateShading
    SampleRateShading    = 35,
    /// SampledRect
    SampledRect          = 37,
    /// InputAttachment
    InputAttachment      = 40,
    /// SparseResidency
    SparseResidency      = 41,
    /// MinLod
    MinLod               = 42,
    /// SampledCubeArray
    SampledCubeArray     = 45,
    /// ImageMSArray
    ImageMSArray         = 48,
    /// StorageImageExtendedFormats
    StorageImageExtendedFormats = 49,
    /// ImageQuery
    ImageQuery           = 50,
    /// DerivativeControl
    DerivativeControl    = 51,
    /// InterpolationFunction
    InterpolationFunction = 52,
    /// TransformFeedback
    TransformFeedback    = 53,
    /// StorageImageReadWithoutFormat
    StorageImageReadWithoutFormat = 55,
    /// StorageImageWriteWithoutFormat
    StorageImageWriteWithoutFormat = 56,
    /// MultiViewport
    MultiViewport        = 57,
    /// SubgroupBallotKHR
    SubgroupBallotKHR    = 4423,
    /// DrawParameters
    DrawParameters       = 4427,
    /// SubgroupVoteKHR
    SubgroupVoteKHR      = 4431,
    /// StorageUniformBufferBlock16
    StorageUniformBufferBlock16 = 4433,
    /// StoragePushConstant16
    StoragePushConstant16 = 4435,
    /// StorageInputOutput16
    StorageInputOutput16 = 4436,
    /// DeviceGroup
    DeviceGroup          = 4437,
    /// MultiView
    MultiView            = 4439,
    /// VariablePointersStorageBuffer
    VariablePointersStorageBuffer = 4441,
    /// VariablePointers
    VariablePointers     = 4442,
    /// Int8
    Int8                 = 4448,
    /// InputAttachmentArrayDynamicIndexing
    InputAttachmentArrayDynamicIndexing = 4449,
    /// UniformTexelBufferArrayDynamicIndexing
    UniformTexelBufferArrayDynamicIndexing = 4450,
    /// StorageTexelBufferArrayDynamicIndexing
    StorageTexelBufferArrayDynamicIndexing = 4451,
    /// UniformBufferArrayNonUniformIndexing
    UniformBufferArrayNonUniformIndexing = 4452,
    /// SampledImageArrayNonUniformIndexing
    SampledImageArrayNonUniformIndexing = 4453,
    /// StorageBufferArrayNonUniformIndexing
    StorageBufferArrayNonUniformIndexing = 4454,
    /// StorageImageArrayNonUniformIndexing
    StorageImageArrayNonUniformIndexing = 4455,
    /// RuntimeDescriptorArray
    RuntimeDescriptorArray = 4457,
    /// RayTracingKHR
    RayTracingKHR        = 4479,
    /// RayQueryKHR
    RayQueryKHR          = 4472,
    /// Float16ImageAMD
    Float16ImageAMD      = 5008,
    /// ImageGatherBiasLodAMD
    ImageGatherBiasLodAMD = 5009,
    /// FragmentMaskAMD
    FragmentMaskAMD      = 5010,
    /// StencilExportEXT
    StencilExportEXT     = 5013,
    /// ImageReadWriteLodAMD
    ImageReadWriteLodAMD = 5015,
    /// Int64ImageEXT
    Int64ImageEXT        = 5016,
    /// ShaderClockKHR
    ShaderClockKHR       = 5055,
    /// FragmentShadingRateKHR
    FragmentShadingRateKHR = 4422,
    /// MeshShadingNV
    MeshShadingNV        = 5266,
    /// MeshShadingEXT
    MeshShadingEXT       = 5283,
    /// WorkgroupMemoryExplicitLayoutKHR
    WorkgroupMemoryExplicitLayoutKHR = 4428,
}

impl Capability {
    /// Returns the capability name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Matrix => "Matrix",
            Self::Shader => "Shader",
            Self::Geometry => "Geometry",
            Self::Tessellation => "Tessellation",
            Self::Float16 => "Float16",
            Self::Float64 => "Float64",
            Self::Int64 => "Int64",
            Self::Int16 => "Int16",
            Self::Int8 => "Int8",
            Self::RayTracingKHR => "RayTracingKHR",
            Self::RayQueryKHR => "RayQueryKHR",
            Self::MeshShadingEXT => "MeshShadingEXT",
            Self::MeshShadingNV => "MeshShadingNV",
            _ => "Unknown",
        }
    }
}
