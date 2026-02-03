//! IR Type System
//!
//! This module defines all types representable in the Lumina IR.
//! The type system is designed to be close to GPU shader capabilities while
//! maintaining type safety and enabling optimization passes.

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::String, vec::Vec};
use core::fmt;

/// Type identifier for fast comparison
pub type TypeId = u32;

/// Scalar types supported by the IR
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ScalarType {
    /// Void type (no value)
    Void,
    /// Boolean type
    Bool,
    /// 8-bit signed integer
    Int8,
    /// 8-bit unsigned integer
    UInt8,
    /// 16-bit signed integer
    Int16,
    /// 16-bit unsigned integer
    UInt16,
    /// 32-bit signed integer
    Int32,
    /// 32-bit unsigned integer
    UInt32,
    /// 64-bit signed integer
    Int64,
    /// 64-bit unsigned integer
    UInt64,
    /// 16-bit floating point (half)
    Float16,
    /// 32-bit floating point
    Float32,
    /// 64-bit floating point
    Float64,
}

impl ScalarType {
    /// Get the size in bytes
    pub const fn size_bytes(self) -> u32 {
        match self {
            Self::Void => 0,
            Self::Bool => 1,
            Self::Int8 | Self::UInt8 => 1,
            Self::Int16 | Self::UInt16 | Self::Float16 => 2,
            Self::Int32 | Self::UInt32 | Self::Float32 => 4,
            Self::Int64 | Self::UInt64 | Self::Float64 => 8,
        }
    }

    /// Check if this is a floating point type
    pub const fn is_float(self) -> bool {
        matches!(self, Self::Float16 | Self::Float32 | Self::Float64)
    }

    /// Check if this is a signed integer type
    pub const fn is_signed_int(self) -> bool {
        matches!(self, Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64)
    }

    /// Check if this is an unsigned integer type
    pub const fn is_unsigned_int(self) -> bool {
        matches!(
            self,
            Self::UInt8 | Self::UInt16 | Self::UInt32 | Self::UInt64
        )
    }

    /// Check if this is any integer type
    pub const fn is_int(self) -> bool {
        self.is_signed_int() || self.is_unsigned_int()
    }

    /// Check if this is a boolean type
    pub const fn is_bool(self) -> bool {
        matches!(self, Self::Bool)
    }

    /// Get the bit width of this type
    pub const fn bit_width(self) -> u32 {
        self.size_bytes() * 8
    }
}

impl fmt::Display for ScalarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Void => write!(f, "void"),
            Self::Bool => write!(f, "bool"),
            Self::Int8 => write!(f, "i8"),
            Self::UInt8 => write!(f, "u8"),
            Self::Int16 => write!(f, "i16"),
            Self::UInt16 => write!(f, "u16"),
            Self::Int32 => write!(f, "i32"),
            Self::UInt32 => write!(f, "u32"),
            Self::Int64 => write!(f, "i64"),
            Self::UInt64 => write!(f, "u64"),
            Self::Float16 => write!(f, "f16"),
            Self::Float32 => write!(f, "f32"),
            Self::Float64 => write!(f, "f64"),
        }
    }
}

/// Vector dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VectorSize {
    Vec2 = 2,
    Vec3 = 3,
    Vec4 = 4,
}

impl VectorSize {
    /// Get the component count
    pub const fn count(self) -> u32 {
        self as u32
    }
}

/// Matrix dimensions (columns x rows)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatrixSize {
    pub columns: u8,
    pub rows: u8,
}

impl MatrixSize {
    pub const MAT2X2: Self = Self {
        columns: 2,
        rows: 2,
    };
    pub const MAT2X3: Self = Self {
        columns: 2,
        rows: 3,
    };
    pub const MAT2X4: Self = Self {
        columns: 2,
        rows: 4,
    };
    pub const MAT3X2: Self = Self {
        columns: 3,
        rows: 2,
    };
    pub const MAT3X3: Self = Self {
        columns: 3,
        rows: 3,
    };
    pub const MAT3X4: Self = Self {
        columns: 3,
        rows: 4,
    };
    pub const MAT4X2: Self = Self {
        columns: 4,
        rows: 2,
    };
    pub const MAT4X3: Self = Self {
        columns: 4,
        rows: 3,
    };
    pub const MAT4X4: Self = Self {
        columns: 4,
        rows: 4,
    };

    /// Get the total number of components
    pub const fn component_count(&self) -> u32 {
        (self.columns as u32) * (self.rows as u32)
    }

    /// Check if this is a square matrix
    pub const fn is_square(&self) -> bool {
        self.columns == self.rows
    }
}

/// Image dimensionality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ImageDimension {
    Dim1D,
    Dim2D,
    Dim3D,
    DimCube,
    DimRect,
    DimBuffer,
    DimSubpassData,
}

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ImageFormat {
    Unknown,
    Rgba32f,
    Rgba16f,
    R32f,
    Rgba8,
    Rgba8Snorm,
    Rg32f,
    Rg16f,
    R11fG11fB10f,
    R16f,
    Rgba16,
    Rgb10A2,
    Rg16,
    Rg8,
    R16,
    R8,
    Rgba16Snorm,
    Rg16Snorm,
    Rg8Snorm,
    R16Snorm,
    R8Snorm,
    Rgba32i,
    Rgba16i,
    Rgba8i,
    R32i,
    Rg32i,
    Rg16i,
    Rg8i,
    R16i,
    R8i,
    Rgba32ui,
    Rgba16ui,
    Rgba8ui,
    R32ui,
    Rgb10A2ui,
    Rg32ui,
    Rg16ui,
    Rg8ui,
    R16ui,
    R8ui,
    R64ui,
    R64i,
}

/// Image type descriptor
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageType {
    /// Sampled type (the type of values read from the image)
    pub sampled_type: ScalarType,
    /// Dimensionality
    pub dimension: ImageDimension,
    /// Depth image
    pub depth: bool,
    /// Array image
    pub arrayed: bool,
    /// Multisampled
    pub multisampled: bool,
    /// 0 = runtime, 1 = sampled, 2 = storage
    pub sampled: u8,
    /// Image format (for storage images)
    pub format: ImageFormat,
}

impl Default for ImageType {
    fn default() -> Self {
        Self {
            sampled_type: ScalarType::Float32,
            dimension: ImageDimension::Dim2D,
            depth: false,
            arrayed: false,
            multisampled: false,
            sampled: 1,
            format: ImageFormat::Unknown,
        }
    }
}

/// Sampler type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerType {
    /// Whether this is a comparison sampler
    pub comparison: bool,
}

/// Address space for pointers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AddressSpace {
    /// Function-local (private) memory
    Private,
    /// Workgroup (shared) memory
    Workgroup,
    /// Uniform buffer memory
    Uniform,
    /// Storage buffer memory
    StorageBuffer,
    /// Push constant memory
    PushConstant,
    /// Input variables
    Input,
    /// Output variables
    Output,
    /// Image/texture memory
    Image,
    /// Generic (unspecified)
    Generic,
}

impl fmt::Display for AddressSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Private => write!(f, "private"),
            Self::Workgroup => write!(f, "workgroup"),
            Self::Uniform => write!(f, "uniform"),
            Self::StorageBuffer => write!(f, "storage"),
            Self::PushConstant => write!(f, "push_constant"),
            Self::Input => write!(f, "input"),
            Self::Output => write!(f, "output"),
            Self::Image => write!(f, "image"),
            Self::Generic => write!(f, "generic"),
        }
    }
}

/// Struct field definition
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// Field name
    pub name: String,
    /// Field type
    pub ty: IrType,
    /// Byte offset within struct
    pub offset: u32,
    /// Decorations
    pub decorations: FieldDecorations,
}

/// Field decorations
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldDecorations {
    /// Builtin kind if this is a builtin
    pub builtin: Option<BuiltinKind>,
    /// Location for interface variables
    pub location: Option<u32>,
    /// Flat interpolation
    pub flat: bool,
    /// No perspective interpolation
    pub no_perspective: bool,
    /// Centroid interpolation
    pub centroid: bool,
    /// Sample interpolation
    pub sample: bool,
    /// Row major matrix layout
    pub row_major: bool,
    /// Column major matrix layout
    pub col_major: bool,
    /// Matrix stride
    pub matrix_stride: Option<u32>,
    /// Array stride
    pub array_stride: Option<u32>,
}

/// Builtin kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BuiltinKind {
    Position,
    PointSize,
    ClipDistance,
    CullDistance,
    VertexId,
    InstanceId,
    PrimitiveId,
    InvocationId,
    Layer,
    ViewportIndex,
    TessLevelOuter,
    TessLevelInner,
    TessCoord,
    PatchVertices,
    FragCoord,
    PointCoord,
    FrontFacing,
    SampleId,
    SamplePosition,
    SampleMask,
    FragDepth,
    HelperInvocation,
    NumWorkgroups,
    WorkgroupSize,
    WorkgroupId,
    LocalInvocationId,
    GlobalInvocationId,
    LocalInvocationIndex,
    DrawIndex,
    BaseVertex,
    BaseInstance,
    ViewIndex,
    DeviceIndex,
    SubgroupSize,
    SubgroupInvocationId,
    SubgroupLocalInvocationId,
    SubgroupEqMask,
    SubgroupGeMask,
    SubgroupGtMask,
    SubgroupLeMask,
    SubgroupLtMask,
    NumSubgroups,
    SubgroupId,
    TaskCountNV,
    PrimitiveCountNV,
    PrimitiveIndicesNV,
    ClipDistancePerViewNV,
    CullDistancePerViewNV,
    LayerPerViewNV,
    MeshViewCountNV,
    MeshViewIndicesNV,
    LaunchIdKHR,
    LaunchSizeKHR,
    WorldRayOriginKHR,
    WorldRayDirectionKHR,
    ObjectRayOriginKHR,
    ObjectRayDirectionKHR,
    RayTminKHR,
    RayTmaxKHR,
    InstanceCustomIndexKHR,
    ObjectToWorldKHR,
    WorldToObjectKHR,
    HitKindKHR,
    HitTKHR,
    IncomingRayFlagsKHR,
    RayGeometryIndexKHR,
    CullMaskKHR,
}

/// Struct type definition
#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    /// Struct name
    pub name: String,
    /// Fields
    pub fields: Vec<StructField>,
    /// Total size in bytes
    pub size: u32,
    /// Alignment in bytes
    pub alignment: u32,
    /// Is this a block-decorated struct (for UBO/SSBO)
    pub block: bool,
    /// Is this a buffer block (SSBO)
    pub buffer_block: bool,
}

impl StructType {
    /// Create a new struct type
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
            size: 0,
            alignment: 1,
            block: false,
            buffer_block: false,
        }
    }

    /// Add a field to the struct
    pub fn add_field(&mut self, name: impl Into<String>, ty: IrType) {
        let field_size = ty.size_bytes();
        let field_align = ty.alignment();

        // Align current offset
        let offset = (self.size + field_align - 1) & !(field_align - 1);

        self.fields.push(StructField {
            name: name.into(),
            ty,
            offset,
            decorations: FieldDecorations::default(),
        });

        self.size = offset + field_size;
        self.alignment = self.alignment.max(field_align);
    }

    /// Get field by name
    pub fn get_field(&self, name: &str) -> Option<(usize, &StructField)> {
        self.fields.iter().enumerate().find(|(_, f)| f.name == name)
    }

    /// Get field by index
    pub fn get_field_by_index(&self, index: usize) -> Option<&StructField> {
        self.fields.get(index)
    }
}

/// Array type
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayType {
    /// Element type
    pub element: Box<IrType>,
    /// Array length (None for runtime arrays)
    pub length: Option<u32>,
    /// Explicit stride (for buffer arrays)
    pub stride: Option<u32>,
}

/// Pointer type
#[derive(Debug, Clone, PartialEq)]
pub struct PointerType {
    /// Pointee type
    pub pointee: Box<IrType>,
    /// Address space
    pub address_space: AddressSpace,
}

/// Function type
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    /// Return type
    pub return_type: Box<IrType>,
    /// Parameter types
    pub parameters: Vec<IrType>,
}

/// Acceleration structure type (for ray tracing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AccelerationStructureType;

/// Ray query type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RayQueryType;

/// The main IR type enum
#[derive(Debug, Clone, PartialEq)]
pub enum IrType {
    /// Scalar types
    Scalar(ScalarType),
    /// Vector types
    Vector {
        element: ScalarType,
        size: VectorSize,
    },
    /// Matrix types
    Matrix {
        element: ScalarType,
        size: MatrixSize,
    },
    /// Array types
    Array(ArrayType),
    /// Pointer types
    Pointer(PointerType),
    /// Struct types
    Struct(StructType),
    /// Image types
    Image(ImageType),
    /// Sampler types
    Sampler(SamplerType),
    /// Sampled image (combined image+sampler)
    SampledImage(Box<ImageType>),
    /// Function types
    Function(FunctionType),
    /// Acceleration structure (ray tracing)
    AccelerationStructure(AccelerationStructureType),
    /// Ray query type
    RayQuery(RayQueryType),
}

impl IrType {
    // ========== Constructors ==========

    /// Create a void type
    pub const fn void() -> Self {
        Self::Scalar(ScalarType::Void)
    }

    /// Create a bool type
    pub const fn bool() -> Self {
        Self::Scalar(ScalarType::Bool)
    }

    /// Create an i32 type
    pub const fn i32() -> Self {
        Self::Scalar(ScalarType::Int32)
    }

    /// Create a u32 type
    pub const fn u32() -> Self {
        Self::Scalar(ScalarType::UInt32)
    }

    /// Create an f32 type
    pub const fn f32() -> Self {
        Self::Scalar(ScalarType::Float32)
    }

    /// Create an f64 type
    pub const fn f64() -> Self {
        Self::Scalar(ScalarType::Float64)
    }

    /// Create a vec2<f32> type
    pub const fn vec2f() -> Self {
        Self::Vector {
            element: ScalarType::Float32,
            size: VectorSize::Vec2,
        }
    }

    /// Create a vec3<f32> type
    pub const fn vec3f() -> Self {
        Self::Vector {
            element: ScalarType::Float32,
            size: VectorSize::Vec3,
        }
    }

    /// Create a vec4<f32> type
    pub const fn vec4f() -> Self {
        Self::Vector {
            element: ScalarType::Float32,
            size: VectorSize::Vec4,
        }
    }

    /// Create a vec2<i32> type
    pub const fn vec2i() -> Self {
        Self::Vector {
            element: ScalarType::Int32,
            size: VectorSize::Vec2,
        }
    }

    /// Create a vec3<i32> type
    pub const fn vec3i() -> Self {
        Self::Vector {
            element: ScalarType::Int32,
            size: VectorSize::Vec3,
        }
    }

    /// Create a vec4<i32> type
    pub const fn vec4i() -> Self {
        Self::Vector {
            element: ScalarType::Int32,
            size: VectorSize::Vec4,
        }
    }

    /// Create a vec2<u32> type
    pub const fn vec2u() -> Self {
        Self::Vector {
            element: ScalarType::UInt32,
            size: VectorSize::Vec2,
        }
    }

    /// Create a vec3<u32> type
    pub const fn vec3u() -> Self {
        Self::Vector {
            element: ScalarType::UInt32,
            size: VectorSize::Vec3,
        }
    }

    /// Create a vec4<u32> type
    pub const fn vec4u() -> Self {
        Self::Vector {
            element: ScalarType::UInt32,
            size: VectorSize::Vec4,
        }
    }

    /// Create a mat2x2<f32> type
    pub const fn mat2f() -> Self {
        Self::Matrix {
            element: ScalarType::Float32,
            size: MatrixSize::MAT2X2,
        }
    }

    /// Create a mat3x3<f32> type
    pub const fn mat3f() -> Self {
        Self::Matrix {
            element: ScalarType::Float32,
            size: MatrixSize::MAT3X3,
        }
    }

    /// Create a mat4x4<f32> type
    pub const fn mat4f() -> Self {
        Self::Matrix {
            element: ScalarType::Float32,
            size: MatrixSize::MAT4X4,
        }
    }

    /// Create a vector type
    pub const fn vector(element: ScalarType, size: VectorSize) -> Self {
        Self::Vector { element, size }
    }

    /// Create a matrix type
    pub const fn matrix(element: ScalarType, size: MatrixSize) -> Self {
        Self::Matrix { element, size }
    }

    /// Create a fixed-size array type
    pub fn array(element: IrType, length: u32) -> Self {
        Self::Array(ArrayType {
            element: Box::new(element),
            length: Some(length),
            stride: None,
        })
    }

    /// Create a runtime array type
    pub fn runtime_array(element: IrType) -> Self {
        Self::Array(ArrayType {
            element: Box::new(element),
            length: None,
            stride: None,
        })
    }

    /// Create a pointer type
    pub fn pointer(pointee: IrType, address_space: AddressSpace) -> Self {
        Self::Pointer(PointerType {
            pointee: Box::new(pointee),
            address_space,
        })
    }

    /// Create a 2D texture type
    pub fn texture2d(sampled_type: ScalarType) -> Self {
        Self::Image(ImageType {
            sampled_type,
            dimension: ImageDimension::Dim2D,
            depth: false,
            arrayed: false,
            multisampled: false,
            sampled: 1,
            format: ImageFormat::Unknown,
        })
    }

    /// Create a depth texture type
    pub fn depth_texture2d() -> Self {
        Self::Image(ImageType {
            sampled_type: ScalarType::Float32,
            dimension: ImageDimension::Dim2D,
            depth: true,
            arrayed: false,
            multisampled: false,
            sampled: 1,
            format: ImageFormat::Unknown,
        })
    }

    /// Create a cube texture type
    pub fn texture_cube(sampled_type: ScalarType) -> Self {
        Self::Image(ImageType {
            sampled_type,
            dimension: ImageDimension::DimCube,
            depth: false,
            arrayed: false,
            multisampled: false,
            sampled: 1,
            format: ImageFormat::Unknown,
        })
    }

    /// Create a 3D texture type
    pub fn texture3d(sampled_type: ScalarType) -> Self {
        Self::Image(ImageType {
            sampled_type,
            dimension: ImageDimension::Dim3D,
            depth: false,
            arrayed: false,
            multisampled: false,
            sampled: 1,
            format: ImageFormat::Unknown,
        })
    }

    /// Create a storage image type
    pub fn storage_image_2d(format: ImageFormat) -> Self {
        Self::Image(ImageType {
            sampled_type: ScalarType::Float32,
            dimension: ImageDimension::Dim2D,
            depth: false,
            arrayed: false,
            multisampled: false,
            sampled: 2, // storage image
            format,
        })
    }

    /// Create a sampler type
    pub fn sampler() -> Self {
        Self::Sampler(SamplerType { comparison: false })
    }

    /// Create a comparison sampler type
    pub fn comparison_sampler() -> Self {
        Self::Sampler(SamplerType { comparison: true })
    }

    /// Create an acceleration structure type
    pub fn acceleration_structure() -> Self {
        Self::AccelerationStructure(AccelerationStructureType)
    }

    /// Create a ray query type
    pub fn ray_query() -> Self {
        Self::RayQuery(RayQueryType)
    }

    // ========== Type Queries ==========

    /// Get the size in bytes
    pub fn size_bytes(&self) -> u32 {
        match self {
            Self::Scalar(s) => s.size_bytes(),
            Self::Vector { element, size } => element.size_bytes() * size.count(),
            Self::Matrix { element, size } => element.size_bytes() * size.component_count(),
            Self::Array(arr) => {
                if let Some(len) = arr.length {
                    let elem_size = arr.element.size_bytes();
                    let elem_align = arr.element.alignment();
                    let stride = arr
                        .stride
                        .unwrap_or((elem_size + elem_align - 1) & !(elem_align - 1));
                    stride * len
                } else {
                    0 // Runtime arrays have no static size
                }
            },
            Self::Struct(s) => s.size,
            Self::Pointer(_) => 8, // Assume 64-bit pointers
            Self::Image(_) | Self::Sampler(_) | Self::SampledImage(_) => 0, // Opaque types
            Self::Function(_) => 0, // Functions have no size
            Self::AccelerationStructure(_) | Self::RayQuery(_) => 0, // Opaque types
        }
    }

    /// Get the alignment in bytes
    pub fn alignment(&self) -> u32 {
        match self {
            Self::Scalar(s) => s.size_bytes().max(1),
            Self::Vector { element, size } => {
                let base = element.size_bytes();
                // vec3 aligns to vec4
                if *size == VectorSize::Vec3 {
                    base * 4
                } else {
                    base * size.count()
                }
            },
            Self::Matrix { element, size } => {
                // Matrix columns align like vectors
                let col_size = size.rows;
                let base = element.size_bytes();
                if col_size == 3 {
                    base * 4
                } else {
                    base * col_size as u32
                }
            },
            Self::Array(arr) => arr.element.alignment(),
            Self::Struct(s) => s.alignment,
            Self::Pointer(_) => 8,
            Self::Image(_) | Self::Sampler(_) | Self::SampledImage(_) => 0,
            Self::Function(_) => 0,
            Self::AccelerationStructure(_) | Self::RayQuery(_) => 0,
        }
    }

    /// Check if this is a scalar type
    pub const fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar(_))
    }

    /// Check if this is a vector type
    pub const fn is_vector(&self) -> bool {
        matches!(self, Self::Vector { .. })
    }

    /// Check if this is a matrix type
    pub const fn is_matrix(&self) -> bool {
        matches!(self, Self::Matrix { .. })
    }

    /// Check if this is an array type
    pub const fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    /// Check if this is a struct type
    pub const fn is_struct(&self) -> bool {
        matches!(self, Self::Struct(_))
    }

    /// Check if this is a pointer type
    pub const fn is_pointer(&self) -> bool {
        matches!(self, Self::Pointer(_))
    }

    /// Check if this is an image type
    pub const fn is_image(&self) -> bool {
        matches!(self, Self::Image(_))
    }

    /// Check if this is a sampler type
    pub const fn is_sampler(&self) -> bool {
        matches!(self, Self::Sampler(_))
    }

    /// Check if this is an opaque type (image, sampler, etc.)
    pub const fn is_opaque(&self) -> bool {
        matches!(
            self,
            Self::Image(_)
                | Self::Sampler(_)
                | Self::SampledImage(_)
                | Self::AccelerationStructure(_)
                | Self::RayQuery(_)
        )
    }

    /// Check if this is a void type
    pub const fn is_void(&self) -> bool {
        matches!(self, Self::Scalar(ScalarType::Void))
    }

    /// Check if this type contains floating point components
    pub fn is_float_based(&self) -> bool {
        match self {
            Self::Scalar(s) => s.is_float(),
            Self::Vector { element, .. } | Self::Matrix { element, .. } => element.is_float(),
            _ => false,
        }
    }

    /// Check if this type contains integer components
    pub fn is_int_based(&self) -> bool {
        match self {
            Self::Scalar(s) => s.is_int(),
            Self::Vector { element, .. } | Self::Matrix { element, .. } => element.is_int(),
            _ => false,
        }
    }

    /// Get the scalar element type if applicable
    pub fn scalar_type(&self) -> Option<ScalarType> {
        match self {
            Self::Scalar(s) => Some(*s),
            Self::Vector { element, .. } | Self::Matrix { element, .. } => Some(*element),
            _ => None,
        }
    }

    /// Get the component count
    pub fn component_count(&self) -> u32 {
        match self {
            Self::Scalar(_) => 1,
            Self::Vector { size, .. } => size.count(),
            Self::Matrix { size, .. } => size.component_count(),
            _ => 1,
        }
    }

    /// Get the pointee type for pointer types
    pub fn pointee(&self) -> Option<&IrType> {
        match self {
            Self::Pointer(p) => Some(&p.pointee),
            _ => None,
        }
    }

    /// Get the element type for array types
    pub fn element_type(&self) -> Option<&IrType> {
        match self {
            Self::Array(arr) => Some(&arr.element),
            _ => None,
        }
    }

    /// Check if two types are compatible for assignment
    pub fn is_compatible_with(&self, other: &IrType) -> bool {
        match (self, other) {
            (Self::Scalar(a), Self::Scalar(b)) => a == b,
            (
                Self::Vector {
                    element: e1,
                    size: s1,
                },
                Self::Vector {
                    element: e2,
                    size: s2,
                },
            ) => e1 == e2 && s1 == s2,
            (
                Self::Matrix {
                    element: e1,
                    size: s1,
                },
                Self::Matrix {
                    element: e2,
                    size: s2,
                },
            ) => e1 == e2 && s1 == s2,
            (Self::Array(a), Self::Array(b)) => {
                a.length == b.length && a.element.is_compatible_with(&b.element)
            },
            (Self::Pointer(a), Self::Pointer(b)) => {
                a.address_space == b.address_space && a.pointee.is_compatible_with(&b.pointee)
            },
            (Self::Struct(a), Self::Struct(b)) => a.name == b.name,
            (Self::Image(a), Self::Image(b)) => a == b,
            (Self::Sampler(a), Self::Sampler(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scalar(s) => write!(f, "{}", s),
            Self::Vector { element, size } => write!(f, "vec{}<{}>", size.count(), element),
            Self::Matrix { element, size } => {
                write!(f, "mat{}x{}<{}>", size.columns, size.rows, element)
            },
            Self::Array(arr) => {
                if let Some(len) = arr.length {
                    write!(f, "[{}; {}]", arr.element, len)
                } else {
                    write!(f, "[{}]", arr.element)
                }
            },
            Self::Pointer(p) => write!(f, "*{} {}", p.address_space, p.pointee),
            Self::Struct(s) => write!(f, "struct {}", s.name),
            Self::Image(img) => {
                let dim = match img.dimension {
                    ImageDimension::Dim1D => "1D",
                    ImageDimension::Dim2D => "2D",
                    ImageDimension::Dim3D => "3D",
                    ImageDimension::DimCube => "Cube",
                    ImageDimension::DimRect => "Rect",
                    ImageDimension::DimBuffer => "Buffer",
                    ImageDimension::DimSubpassData => "SubpassData",
                };
                if img.arrayed {
                    write!(f, "texture{}Array<{}>", dim, img.sampled_type)
                } else {
                    write!(f, "texture{}<{}>", dim, img.sampled_type)
                }
            },
            Self::Sampler(s) => {
                if s.comparison {
                    write!(f, "samplerComparison")
                } else {
                    write!(f, "sampler")
                }
            },
            Self::SampledImage(img) => write!(f, "sampledImage<{:?}>", img.dimension),
            Self::Function(fun) => {
                write!(f, "fn(")?;
                for (i, param) in fun.parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", fun.return_type)
            },
            Self::AccelerationStructure(_) => write!(f, "accelerationStructure"),
            Self::RayQuery(_) => write!(f, "rayQuery"),
        }
    }
}

/// Type registry for managing complex types
#[derive(Debug, Default)]
pub struct TypeRegistry {
    /// Registered struct types
    structs: Vec<StructType>,
    /// Type ID counter
    next_id: TypeId,
}

impl TypeRegistry {
    /// Create a new type registry
    pub fn new() -> Self {
        Self {
            structs: Vec::new(),
            next_id: 0,
        }
    }

    /// Register a new struct type
    pub fn register_struct(&mut self, struct_type: StructType) -> TypeId {
        let id = self.next_id;
        self.next_id += 1;
        self.structs.push(struct_type);
        id
    }

    /// Get a struct type by ID
    pub fn get_struct(&self, id: TypeId) -> Option<&StructType> {
        self.structs.get(id as usize)
    }

    /// Get a struct type by name
    pub fn get_struct_by_name(&self, name: &str) -> Option<(TypeId, &StructType)> {
        self.structs
            .iter()
            .enumerate()
            .find(|(_, s)| s.name == name)
            .map(|(i, s)| (i as TypeId, s))
    }

    /// Iterate over all struct types
    pub fn structs(&self) -> impl Iterator<Item = (TypeId, &StructType)> {
        self.structs
            .iter()
            .enumerate()
            .map(|(i, s)| (i as TypeId, s))
    }
}

/// Layout rules for struct members
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutRules {
    /// Standard Vulkan layout (std140 for UBO, std430 for SSBO)
    Std140,
    /// Relaxed layout (std430)
    Std430,
    /// Scalar layout (no padding)
    Scalar,
    /// C layout
    C,
}

impl LayoutRules {
    /// Calculate the base alignment for a type
    pub fn base_alignment(&self, ty: &IrType) -> u32 {
        match self {
            Self::Std140 => self.std140_alignment(ty),
            Self::Std430 => self.std430_alignment(ty),
            Self::Scalar => ty.scalar_type().map(|s| s.size_bytes()).unwrap_or(4),
            Self::C => ty.alignment(),
        }
    }

    fn std140_alignment(&self, ty: &IrType) -> u32 {
        match ty {
            IrType::Scalar(s) => s.size_bytes(),
            IrType::Vector { element, size } => {
                let n = size.count();
                let base = element.size_bytes();
                if n == 2 {
                    base * 2
                } else {
                    base * 4 // vec3 and vec4 align to vec4
                }
            },
            IrType::Matrix { element, size } => {
                // Matrix columns are treated as arrays of vectors
                // Round up to vec4 alignment
                let base = element.size_bytes();
                let col_size = size.rows;
                if col_size <= 2 { base * 2 } else { base * 4 }.max(16) // Minimum 16 bytes for arrays/matrices
            },
            IrType::Array(arr) => {
                // Array element alignment rounded up to 16 bytes
                self.std140_alignment(&arr.element).max(16)
            },
            IrType::Struct(s) => {
                // Struct alignment is rounded up to 16 bytes
                s.alignment.max(16)
            },
            _ => 4,
        }
    }

    fn std430_alignment(&self, ty: &IrType) -> u32 {
        match ty {
            IrType::Scalar(s) => s.size_bytes(),
            IrType::Vector { element, size } => {
                let n = size.count();
                let base = element.size_bytes();
                if n == 2 {
                    base * 2
                } else {
                    base * 4
                }
            },
            IrType::Matrix { element, size } => {
                let base = element.size_bytes();
                let col_size = size.rows;
                if col_size <= 2 {
                    base * 2
                } else {
                    base * 4
                }
            },
            IrType::Array(arr) => self.std430_alignment(&arr.element),
            IrType::Struct(s) => s.alignment,
            _ => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_types() {
        assert_eq!(ScalarType::Float32.size_bytes(), 4);
        assert_eq!(ScalarType::Int32.size_bytes(), 4);
        assert_eq!(ScalarType::Float64.size_bytes(), 8);
        assert!(ScalarType::Float32.is_float());
        assert!(ScalarType::Int32.is_signed_int());
        assert!(ScalarType::UInt32.is_unsigned_int());
    }

    #[test]
    fn test_vector_types() {
        let vec4f = IrType::vec4f();
        assert_eq!(vec4f.size_bytes(), 16);
        assert_eq!(vec4f.component_count(), 4);
        assert!(vec4f.is_vector());
        assert!(vec4f.is_float_based());
    }

    #[test]
    fn test_matrix_types() {
        let mat4 = IrType::mat4f();
        assert_eq!(mat4.size_bytes(), 64);
        assert_eq!(mat4.component_count(), 16);
        assert!(mat4.is_matrix());
    }

    #[test]
    fn test_struct_type() {
        let mut s = StructType::new("TestStruct");
        s.add_field("position", IrType::vec3f());
        s.add_field("normal", IrType::vec3f());
        s.add_field("uv", IrType::vec2f());

        assert_eq!(s.fields.len(), 3);
        assert!(s.get_field("position").is_some());
    }

    #[test]
    fn test_array_type() {
        let arr = IrType::array(IrType::f32(), 16);
        assert!(arr.is_array());
        assert_eq!(arr.size_bytes(), 64);
    }

    #[test]
    fn test_type_display() {
        assert_eq!(format!("{}", IrType::f32()), "f32");
        assert_eq!(format!("{}", IrType::vec4f()), "vec4<f32>");
        assert_eq!(format!("{}", IrType::mat4f()), "mat4x4<f32>");
    }
}
