//! Core type definitions for Lumina
//!
//! This module provides fundamental types used throughout the Lumina API,
//! including GPU-compatible primitives and handle types.

use core::marker::PhantomData;

/// A typed handle to a GPU resource
///
/// Handles are type-safe wrappers around raw indices, ensuring that
/// a buffer handle cannot be used where a texture handle is expected.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle<T> {
    id: u32,
    generation: u32,
    _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    /// Creates a new handle with the given id and generation
    pub(crate) const fn new(id: u32, generation: u32) -> Self {
        Self {
            id,
            generation,
            _marker: PhantomData,
        }
    }

    /// Returns the raw id of this handle
    pub const fn id(&self) -> u32 {
        self.id
    }

    /// Returns the generation of this handle
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Creates a null handle
    pub const fn null() -> Self {
        Self::new(u32::MAX, 0)
    }

    /// Returns true if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.id == u32::MAX
    }
}

impl<T> core::fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Handle")
            .field("id", &self.id)
            .field("generation", &self.generation)
            .finish()
    }
}

/// Marker type for buffer handles
pub struct BufferMarker;
/// Marker type for texture handles
pub struct TextureMarker;
/// Marker type for sampler handles
pub struct SamplerMarker;
/// Marker type for pipeline handles
pub struct PipelineMarker;
/// Marker type for render pass handles
pub struct RenderPassMarker;
/// Marker type for framebuffer handles
pub struct FramebufferMarker;
/// Marker type for shader handles
pub struct ShaderMarker;

/// Handle to a GPU buffer
pub type BufferHandle = Handle<BufferMarker>;
/// Handle to a GPU texture
pub type TextureHandle = Handle<TextureMarker>;
/// Handle to a sampler
pub type SamplerHandle = Handle<SamplerMarker>;
/// Handle to a graphics or compute pipeline
pub type PipelineHandle = Handle<PipelineMarker>;
/// Handle to a render pass
pub type RenderPassHandle = Handle<RenderPassMarker>;
/// Handle to a framebuffer
pub type FramebufferHandle = Handle<FramebufferMarker>;
/// Handle to a shader module
pub type ShaderHandle = Handle<ShaderMarker>;

/// Trait for types that can be stored in GPU buffers
///
/// This trait is automatically derived with `#[derive(GpuData)]`.
/// It ensures that the type has a well-defined memory layout compatible
/// with GPU access patterns.
///
/// # Safety
///
/// Implementors must guarantee that:
/// - The type is `Copy` and `Pod` (plain old data)
/// - The type has consistent alignment on both CPU and GPU
/// - The type does not contain any pointers or references
pub unsafe trait GpuData: Copy + Sized + 'static {
    /// The size of this type in bytes
    const SIZE: usize = core::mem::size_of::<Self>();

    /// The alignment of this type in bytes
    const ALIGN: usize = core::mem::align_of::<Self>();

    /// Returns the byte representation of this value
    fn as_bytes(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }
}

// Implement GpuData for primitive types
unsafe impl GpuData for f32 {}
unsafe impl GpuData for f64 {}
unsafe impl GpuData for i8 {}
unsafe impl GpuData for i16 {}
unsafe impl GpuData for i32 {}
unsafe impl GpuData for i64 {}
unsafe impl GpuData for u8 {}
unsafe impl GpuData for u16 {}
unsafe impl GpuData for u32 {}
unsafe impl GpuData for u64 {}

// Implement for arrays of GpuData
unsafe impl<T: GpuData, const N: usize> GpuData for [T; N] {}

/// Trait for types that can be used as vertex attributes
///
/// This trait is automatically derived with `#[derive(GpuVertex)]`.
/// It provides the vertex format description needed for pipeline creation.
pub trait GpuVertex: GpuData {
    /// Returns the vertex attribute descriptions
    fn attributes() -> &'static [VertexAttribute];

    /// Returns the stride (size) of one vertex
    fn stride() -> u32 {
        Self::SIZE as u32
    }
}

/// Description of a vertex attribute
#[derive(Clone, Copy, Debug)]
pub struct VertexAttribute {
    /// Location in the shader (layout(location = N))
    pub location: u32,
    /// Offset from the start of the vertex
    pub offset: u32,
    /// Format of the attribute
    pub format: AttributeFormat,
}

/// Format of a vertex attribute
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttributeFormat {
    /// Single 32-bit float
    Float,
    /// Two 32-bit floats
    Vec2,
    /// Three 32-bit floats
    Vec3,
    /// Four 32-bit floats
    Vec4,
    /// Single 32-bit signed integer
    Int,
    /// Two 32-bit signed integers
    IVec2,
    /// Three 32-bit signed integers
    IVec3,
    /// Four 32-bit signed integers
    IVec4,
    /// Single 32-bit unsigned integer
    UInt,
    /// Two 32-bit unsigned integers
    UVec2,
    /// Three 32-bit unsigned integers
    UVec3,
    /// Four 32-bit unsigned integers
    UVec4,
    /// Four 8-bit unsigned normalized values
    Rgba8,
}

impl AttributeFormat {
    /// Returns the size of this format in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Float | Self::Int | Self::UInt => 4,
            Self::Vec2 | Self::IVec2 | Self::UVec2 => 8,
            Self::Vec3 | Self::IVec3 | Self::UVec3 => 12,
            Self::Vec4 | Self::IVec4 | Self::UVec4 | Self::Rgba8 => 16,
        }
    }

    /// Returns the Vulkan format constant
    pub const fn vk_format(&self) -> u32 {
        match self {
            Self::Float => 100,  // VK_FORMAT_R32_SFLOAT
            Self::Vec2 => 103,   // VK_FORMAT_R32G32_SFLOAT
            Self::Vec3 => 106,   // VK_FORMAT_R32G32B32_SFLOAT
            Self::Vec4 => 109,   // VK_FORMAT_R32G32B32A32_SFLOAT
            Self::Int => 98,     // VK_FORMAT_R32_SINT
            Self::IVec2 => 101,  // VK_FORMAT_R32G32_SINT
            Self::IVec3 => 104,  // VK_FORMAT_R32G32B32_SINT
            Self::IVec4 => 107,  // VK_FORMAT_R32G32B32A32_SINT
            Self::UInt => 99,    // VK_FORMAT_R32_UINT
            Self::UVec2 => 102,  // VK_FORMAT_R32G32_UINT
            Self::UVec3 => 105,  // VK_FORMAT_R32G32B32_UINT
            Self::UVec4 => 108,  // VK_FORMAT_R32G32B32A32_UINT
            Self::Rgba8 => 37,   // VK_FORMAT_R8G8B8A8_UNORM
        }
    }
}

/// Trait for types that can be used as uniform blocks
///
/// This trait is automatically derived with `#[derive(GpuUniforms)]`.
/// Types implementing this trait can be passed to shaders as uniform data.
pub trait GpuUniforms: GpuData {
    /// Returns the layout of uniform members
    fn layout() -> &'static [UniformMember];
}

/// Description of a uniform block member
#[derive(Clone, Copy, Debug)]
pub struct UniformMember {
    /// Name of the member
    pub name: &'static str,
    /// Offset from the start of the block
    pub offset: u32,
    /// Size in bytes
    pub size: u32,
    /// Type of the member
    pub ty: UniformType,
}

/// Type of a uniform member
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniformType {
    /// 32-bit float
    Float,
    /// 2D float vector
    Vec2,
    /// 3D float vector
    Vec3,
    /// 4D float vector
    Vec4,
    /// 32-bit signed integer
    Int,
    /// 32-bit unsigned integer
    UInt,
    /// 2x2 matrix
    Mat2,
    /// 3x3 matrix
    Mat3,
    /// 4x4 matrix
    Mat4,
}

/// Builtin shader inputs
#[derive(Clone, Copy, Debug)]
pub enum Builtin {
    /// Vertex index (gl_VertexIndex)
    VertexIndex,
    /// Instance index (gl_InstanceIndex)
    InstanceIndex,
    /// Fragment coordinates (gl_FragCoord)
    FragCoord,
    /// Front-facing (gl_FrontFacing)
    FrontFacing,
    /// Point coordinates (gl_PointCoord)
    PointCoord,
    /// Global invocation ID (compute)
    GlobalId,
    /// Local invocation ID (compute)
    LocalId,
    /// Workgroup ID (compute)
    WorkgroupId,
}

/// Unsigned 3D vector for compute dispatch
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct UVec3 {
    /// X component
    pub x: u32,
    /// Y component
    pub y: u32,
    /// Z component
    pub z: u32,
}

impl UVec3 {
    /// Creates a new UVec3
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// Creates a UVec3 with all components set to the same value
    pub const fn splat(v: u32) -> Self {
        Self { x: v, y: v, z: v }
    }
}
