//! Vertex Input Types for Lumina
//!
//! This module provides comprehensive vertex input configuration,
//! vertex attribute formats, and input assembly state.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Vertex Input State
// ============================================================================

/// Vertex input state create info
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct VertexInputStateCreateInfo {
    /// Flags
    pub flags: VertexInputStateCreateFlags,
    /// Vertex binding descriptions
    pub vertex_binding_descriptions: Vec<VertexInputBindingDescription>,
    /// Vertex attribute descriptions
    pub vertex_attribute_descriptions: Vec<VertexInputAttributeDescription>,
}

impl VertexInputStateCreateInfo {
    /// Creates new empty info
    #[inline]
    pub fn new() -> Self {
        Self {
            flags: VertexInputStateCreateFlags::NONE,
            vertex_binding_descriptions: Vec::new(),
            vertex_attribute_descriptions: Vec::new(),
        }
    }

    /// No vertex input (for generated vertices)
    pub fn empty() -> Self {
        Self::new()
    }

    /// Add binding
    #[inline]
    pub fn add_binding(mut self, binding: VertexInputBindingDescription) -> Self {
        self.vertex_binding_descriptions.push(binding);
        self
    }

    /// Add attribute
    #[inline]
    pub fn add_attribute(mut self, attribute: VertexInputAttributeDescription) -> Self {
        self.vertex_attribute_descriptions.push(attribute);
        self
    }

    /// With bindings
    #[inline]
    pub fn with_bindings(mut self, bindings: Vec<VertexInputBindingDescription>) -> Self {
        self.vertex_binding_descriptions = bindings;
        self
    }

    /// With attributes
    #[inline]
    pub fn with_attributes(mut self, attributes: Vec<VertexInputAttributeDescription>) -> Self {
        self.vertex_attribute_descriptions = attributes;
        self
    }

    /// With flags
    #[inline]
    pub fn with_flags(mut self, flags: VertexInputStateCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Position only (vec3)
    pub fn position_only() -> Self {
        Self::new()
            .add_binding(VertexInputBindingDescription::vertex(0, 12))
            .add_attribute(VertexInputAttributeDescription::new(
                0,
                0,
                VertexFormat::Float3,
                0,
            ))
    }

    /// Position and color (vec3, vec4)
    pub fn position_color() -> Self {
        Self::new()
            .add_binding(VertexInputBindingDescription::vertex(0, 28))
            .add_attribute(VertexInputAttributeDescription::new(
                0,
                0,
                VertexFormat::Float3,
                0,
            ))
            .add_attribute(VertexInputAttributeDescription::new(
                1,
                0,
                VertexFormat::Float4,
                12,
            ))
    }

    /// Position and UV (vec3, vec2)
    pub fn position_uv() -> Self {
        Self::new()
            .add_binding(VertexInputBindingDescription::vertex(0, 20))
            .add_attribute(VertexInputAttributeDescription::new(
                0,
                0,
                VertexFormat::Float3,
                0,
            ))
            .add_attribute(VertexInputAttributeDescription::new(
                1,
                0,
                VertexFormat::Float2,
                12,
            ))
    }

    /// Position, normal, UV (vec3, vec3, vec2)
    pub fn position_normal_uv() -> Self {
        Self::new()
            .add_binding(VertexInputBindingDescription::vertex(0, 32))
            .add_attribute(VertexInputAttributeDescription::new(
                0,
                0,
                VertexFormat::Float3,
                0,
            ))
            .add_attribute(VertexInputAttributeDescription::new(
                1,
                0,
                VertexFormat::Float3,
                12,
            ))
            .add_attribute(VertexInputAttributeDescription::new(
                2,
                0,
                VertexFormat::Float2,
                24,
            ))
    }

    /// Position, normal, tangent, UV (vec3, vec3, vec4, vec2)
    pub fn position_normal_tangent_uv() -> Self {
        Self::new()
            .add_binding(VertexInputBindingDescription::vertex(0, 48))
            .add_attribute(VertexInputAttributeDescription::new(
                0,
                0,
                VertexFormat::Float3,
                0,
            ))
            .add_attribute(VertexInputAttributeDescription::new(
                1,
                0,
                VertexFormat::Float3,
                12,
            ))
            .add_attribute(VertexInputAttributeDescription::new(
                2,
                0,
                VertexFormat::Float4,
                24,
            ))
            .add_attribute(VertexInputAttributeDescription::new(
                3,
                0,
                VertexFormat::Float2,
                40,
            ))
    }

    /// Full vertex (position, normal, tangent, uv0, uv1, color, joints, weights)
    pub fn full() -> Self {
        Self::new()
            .add_binding(VertexInputBindingDescription::vertex(0, 88))
            .add_attribute(VertexInputAttributeDescription::new(0, 0, VertexFormat::Float3, 0))   // position
            .add_attribute(VertexInputAttributeDescription::new(1, 0, VertexFormat::Float3, 12))  // normal
            .add_attribute(VertexInputAttributeDescription::new(2, 0, VertexFormat::Float4, 24))  // tangent
            .add_attribute(VertexInputAttributeDescription::new(3, 0, VertexFormat::Float2, 40))  // uv0
            .add_attribute(VertexInputAttributeDescription::new(4, 0, VertexFormat::Float2, 48))  // uv1
            .add_attribute(VertexInputAttributeDescription::new(5, 0, VertexFormat::Float4, 56))  // color
            .add_attribute(VertexInputAttributeDescription::new(6, 0, VertexFormat::Uint4, 72))   // joints
            .add_attribute(VertexInputAttributeDescription::new(7, 0, VertexFormat::Float4, 80))
        // weights
    }

    /// 2D UI vertex (vec2 position, vec2 uv, vec4 color)
    pub fn ui_vertex() -> Self {
        Self::new()
            .add_binding(VertexInputBindingDescription::vertex(0, 32))
            .add_attribute(VertexInputAttributeDescription::new(
                0,
                0,
                VertexFormat::Float2,
                0,
            ))
            .add_attribute(VertexInputAttributeDescription::new(
                1,
                0,
                VertexFormat::Float2,
                8,
            ))
            .add_attribute(VertexInputAttributeDescription::new(
                2,
                0,
                VertexFormat::Float4,
                16,
            ))
    }
}

/// Vertex input state create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct VertexInputStateCreateFlags(pub u32);

impl VertexInputStateCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);

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
// Vertex Input Binding Description
// ============================================================================

/// Vertex input binding description
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VertexInputBindingDescription {
    /// Binding index
    pub binding: u32,
    /// Stride in bytes
    pub stride: u32,
    /// Input rate
    pub input_rate: VertexInputRate,
}

impl VertexInputBindingDescription {
    /// Creates new binding (per-vertex)
    #[inline]
    pub const fn new(binding: u32, stride: u32, input_rate: VertexInputRate) -> Self {
        Self {
            binding,
            stride,
            input_rate,
        }
    }

    /// Per-vertex input
    #[inline]
    pub const fn vertex(binding: u32, stride: u32) -> Self {
        Self::new(binding, stride, VertexInputRate::Vertex)
    }

    /// Per-instance input
    #[inline]
    pub const fn instance(binding: u32, stride: u32) -> Self {
        Self::new(binding, stride, VertexInputRate::Instance)
    }

    /// With divisor (for instanced rendering)
    #[inline]
    pub const fn with_divisor(self, _divisor: u32) -> VertexInputBindingDivisorDescription {
        VertexInputBindingDivisorDescription {
            binding: self.binding,
            divisor: _divisor,
        }
    }
}

/// Vertex input rate
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexInputRate {
    /// Per-vertex
    #[default]
    Vertex   = 0,
    /// Per-instance
    Instance = 1,
}

/// Vertex input binding divisor description
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VertexInputBindingDivisorDescription {
    /// Binding index
    pub binding: u32,
    /// Divisor
    pub divisor: u32,
}

impl VertexInputBindingDivisorDescription {
    /// Creates new divisor description
    #[inline]
    pub const fn new(binding: u32, divisor: u32) -> Self {
        Self { binding, divisor }
    }

    /// Every vertex (divisor = 0, for per-vertex data)
    #[inline]
    pub const fn per_vertex(binding: u32) -> Self {
        Self::new(binding, 0)
    }

    /// Every instance (divisor = 1)
    #[inline]
    pub const fn per_instance(binding: u32) -> Self {
        Self::new(binding, 1)
    }
}

// ============================================================================
// Vertex Input Attribute Description
// ============================================================================

/// Vertex input attribute description
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VertexInputAttributeDescription {
    /// Location in shader
    pub location: u32,
    /// Binding index
    pub binding: u32,
    /// Format
    pub format: VertexFormat,
    /// Offset in bytes
    pub offset: u32,
}

impl VertexInputAttributeDescription {
    /// Creates new attribute
    #[inline]
    pub const fn new(location: u32, binding: u32, format: VertexFormat, offset: u32) -> Self {
        Self {
            location,
            binding,
            format,
            offset,
        }
    }

    /// Float attribute
    #[inline]
    pub const fn float(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Float, offset)
    }

    /// Vec2 attribute
    #[inline]
    pub const fn vec2(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Float2, offset)
    }

    /// Vec3 attribute
    #[inline]
    pub const fn vec3(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Float3, offset)
    }

    /// Vec4 attribute
    #[inline]
    pub const fn vec4(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Float4, offset)
    }

    /// Int attribute
    #[inline]
    pub const fn int(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Int, offset)
    }

    /// Uint attribute
    #[inline]
    pub const fn uint(location: u32, binding: u32, offset: u32) -> Self {
        Self::new(location, binding, VertexFormat::Uint, offset)
    }
}

// ============================================================================
// Vertex Format
// ============================================================================

/// Vertex attribute format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexFormat {
    // Undefined
    #[default]
    Undefined          = 0,

    // 8-bit formats
    /// R8 unsigned normalized
    R8Unorm            = 9,
    /// R8 signed normalized
    R8Snorm            = 10,
    /// R8 unsigned scaled
    R8Uscaled          = 11,
    /// R8 signed scaled
    R8Sscaled          = 12,
    /// R8 unsigned int
    R8Uint             = 13,
    /// R8 signed int
    R8Sint             = 14,

    /// RG8 unsigned normalized
    R8G8Unorm          = 16,
    /// RG8 signed normalized
    R8G8Snorm          = 17,
    /// RG8 unsigned int
    R8G8Uint           = 20,
    /// RG8 signed int
    R8G8Sint           = 21,

    /// RGB8 unsigned normalized
    R8G8B8Unorm        = 23,
    /// RGB8 signed normalized
    R8G8B8Snorm        = 24,
    /// RGB8 unsigned int
    R8G8B8Uint         = 27,
    /// RGB8 signed int
    R8G8B8Sint         = 28,

    /// RGBA8 unsigned normalized
    R8G8B8A8Unorm      = 37,
    /// RGBA8 signed normalized
    R8G8B8A8Snorm      = 38,
    /// RGBA8 unsigned int
    R8G8B8A8Uint       = 41,
    /// RGBA8 signed int
    R8G8B8A8Sint       = 42,

    // 16-bit formats
    /// R16 unsigned normalized
    R16Unorm           = 70,
    /// R16 signed normalized
    R16Snorm           = 71,
    /// R16 unsigned int
    R16Uint            = 74,
    /// R16 signed int
    R16Sint            = 75,
    /// R16 float
    R16Sfloat          = 76,

    /// RG16 unsigned normalized
    R16G16Unorm        = 77,
    /// RG16 signed normalized
    R16G16Snorm        = 78,
    /// RG16 unsigned int
    R16G16Uint         = 81,
    /// RG16 signed int
    R16G16Sint         = 82,
    /// RG16 float
    R16G16Sfloat       = 83,

    /// RGB16 unsigned normalized
    R16G16B16Unorm     = 84,
    /// RGB16 signed normalized
    R16G16B16Snorm     = 85,
    /// RGB16 unsigned int
    R16G16B16Uint      = 88,
    /// RGB16 signed int
    R16G16B16Sint      = 89,
    /// RGB16 float
    R16G16B16Sfloat    = 90,

    /// RGBA16 unsigned normalized
    R16G16B16A16Unorm  = 91,
    /// RGBA16 signed normalized
    R16G16B16A16Snorm  = 92,
    /// RGBA16 unsigned int
    R16G16B16A16Uint   = 95,
    /// RGBA16 signed int
    R16G16B16A16Sint   = 96,
    /// RGBA16 float
    R16G16B16A16Sfloat = 97,

    // 32-bit formats
    /// R32 unsigned int
    R32Uint            = 98,
    /// R32 signed int
    R32Sint            = 99,
    /// R32 float
    R32Sfloat          = 100,

    /// RG32 unsigned int
    R32G32Uint         = 101,
    /// RG32 signed int
    R32G32Sint         = 102,
    /// RG32 float
    R32G32Sfloat       = 103,

    /// RGB32 unsigned int
    R32G32B32Uint      = 104,
    /// RGB32 signed int
    R32G32B32Sint      = 105,
    /// RGB32 float
    R32G32B32Sfloat    = 106,

    /// RGBA32 unsigned int
    R32G32B32A32Uint   = 107,
    /// RGBA32 signed int
    R32G32B32A32Sint   = 108,
    /// RGBA32 float
    R32G32B32A32Sfloat = 109,

    // 64-bit formats
    /// R64 unsigned int
    R64Uint            = 110,
    /// R64 signed int
    R64Sint            = 111,
    /// R64 float
    R64Sfloat          = 112,

    /// RG64 unsigned int
    R64G64Uint         = 113,
    /// RG64 signed int
    R64G64Sint         = 114,
    /// RG64 float
    R64G64Sfloat       = 115,

    /// RGB64 unsigned int
    R64G64B64Uint      = 116,
    /// RGB64 signed int
    R64G64B64Sint      = 117,
    /// RGB64 float
    R64G64B64Sfloat    = 118,

    /// RGBA64 unsigned int
    R64G64B64A64Uint   = 119,
    /// RGBA64 signed int
    R64G64B64A64Sint   = 120,
    /// RGBA64 float
    R64G64B64A64Sfloat = 121,

    // Packed formats
    /// A2B10G10R10 unsigned normalized packed
    A2B10G10R10UnormPack32 = 64,
    /// A2B10G10R10 unsigned int packed
    A2B10G10R10UintPack32 = 68,

    // Common aliases
    /// Float (R32 float)
    Float              = 100,
    /// Float2 (RG32 float)
    Float2             = 103,
    /// Float3 (RGB32 float)
    Float3             = 106,
    /// Float4 (RGBA32 float)
    Float4             = 109,
    /// Int (R32 int)
    Int                = 99,
    /// Int2 (RG32 int)
    Int2               = 102,
    /// Int3 (RGB32 int)
    Int3               = 105,
    /// Int4 (RGBA32 int)
    Int4               = 108,
    /// Uint (R32 uint)
    Uint               = 98,
    /// Uint2 (RG32 uint)
    Uint2              = 101,
    /// Uint3 (RGB32 uint)
    Uint3              = 104,
    /// Uint4 (RGBA32 uint)
    Uint4              = 107,
    /// Half (R16 float)
    Half               = 76,
    /// Half2 (RG16 float)
    Half2              = 83,
    /// Half3 (RGB16 float)
    Half3              = 90,
    /// Half4 (RGBA16 float)
    Half4              = 97,
}

impl VertexFormat {
    /// Size in bytes
    #[inline]
    pub const fn size(&self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::R8Unorm
            | Self::R8Snorm
            | Self::R8Uscaled
            | Self::R8Sscaled
            | Self::R8Uint
            | Self::R8Sint => 1,
            Self::R8G8Unorm | Self::R8G8Snorm | Self::R8G8Uint | Self::R8G8Sint => 2,
            Self::R8G8B8Unorm | Self::R8G8B8Snorm | Self::R8G8B8Uint | Self::R8G8B8Sint => 3,
            Self::R8G8B8A8Unorm | Self::R8G8B8A8Snorm | Self::R8G8B8A8Uint | Self::R8G8B8A8Sint => {
                4
            },
            Self::R16Unorm
            | Self::R16Snorm
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Sfloat
            | Self::Half => 2,
            Self::R16G16Unorm
            | Self::R16G16Snorm
            | Self::R16G16Uint
            | Self::R16G16Sint
            | Self::R16G16Sfloat
            | Self::Half2 => 4,
            Self::R16G16B16Unorm
            | Self::R16G16B16Snorm
            | Self::R16G16B16Uint
            | Self::R16G16B16Sint
            | Self::R16G16B16Sfloat
            | Self::Half3 => 6,
            Self::R16G16B16A16Unorm
            | Self::R16G16B16A16Snorm
            | Self::R16G16B16A16Uint
            | Self::R16G16B16A16Sint
            | Self::R16G16B16A16Sfloat
            | Self::Half4 => 8,
            Self::R32Uint
            | Self::R32Sint
            | Self::R32Sfloat
            | Self::Float
            | Self::Int
            | Self::Uint => 4,
            Self::R32G32Uint
            | Self::R32G32Sint
            | Self::R32G32Sfloat
            | Self::Float2
            | Self::Int2
            | Self::Uint2 => 8,
            Self::R32G32B32Uint
            | Self::R32G32B32Sint
            | Self::R32G32B32Sfloat
            | Self::Float3
            | Self::Int3
            | Self::Uint3 => 12,
            Self::R32G32B32A32Uint
            | Self::R32G32B32A32Sint
            | Self::R32G32B32A32Sfloat
            | Self::Float4
            | Self::Int4
            | Self::Uint4 => 16,
            Self::R64Uint | Self::R64Sint | Self::R64Sfloat => 8,
            Self::R64G64Uint | Self::R64G64Sint | Self::R64G64Sfloat => 16,
            Self::R64G64B64Uint | Self::R64G64B64Sint | Self::R64G64B64Sfloat => 24,
            Self::R64G64B64A64Uint | Self::R64G64B64A64Sint | Self::R64G64B64A64Sfloat => 32,
            Self::A2B10G10R10UnormPack32 | Self::A2B10G10R10UintPack32 => 4,
        }
    }

    /// Component count
    #[inline]
    pub const fn component_count(&self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::R8Unorm
            | Self::R8Snorm
            | Self::R8Uscaled
            | Self::R8Sscaled
            | Self::R8Uint
            | Self::R8Sint
            | Self::R16Unorm
            | Self::R16Snorm
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Sfloat
            | Self::Half
            | Self::R32Uint
            | Self::R32Sint
            | Self::R32Sfloat
            | Self::Float
            | Self::Int
            | Self::Uint
            | Self::R64Uint
            | Self::R64Sint
            | Self::R64Sfloat => 1,
            Self::R8G8Unorm
            | Self::R8G8Snorm
            | Self::R8G8Uint
            | Self::R8G8Sint
            | Self::R16G16Unorm
            | Self::R16G16Snorm
            | Self::R16G16Uint
            | Self::R16G16Sint
            | Self::R16G16Sfloat
            | Self::Half2
            | Self::R32G32Uint
            | Self::R32G32Sint
            | Self::R32G32Sfloat
            | Self::Float2
            | Self::Int2
            | Self::Uint2
            | Self::R64G64Uint
            | Self::R64G64Sint
            | Self::R64G64Sfloat => 2,
            Self::R8G8B8Unorm
            | Self::R8G8B8Snorm
            | Self::R8G8B8Uint
            | Self::R8G8B8Sint
            | Self::R16G16B16Unorm
            | Self::R16G16B16Snorm
            | Self::R16G16B16Uint
            | Self::R16G16B16Sint
            | Self::R16G16B16Sfloat
            | Self::Half3
            | Self::R32G32B32Uint
            | Self::R32G32B32Sint
            | Self::R32G32B32Sfloat
            | Self::Float3
            | Self::Int3
            | Self::Uint3
            | Self::R64G64B64Uint
            | Self::R64G64B64Sint
            | Self::R64G64B64Sfloat => 3,
            Self::R8G8B8A8Unorm
            | Self::R8G8B8A8Snorm
            | Self::R8G8B8A8Uint
            | Self::R8G8B8A8Sint
            | Self::R16G16B16A16Unorm
            | Self::R16G16B16A16Snorm
            | Self::R16G16B16A16Uint
            | Self::R16G16B16A16Sint
            | Self::R16G16B16A16Sfloat
            | Self::Half4
            | Self::R32G32B32A32Uint
            | Self::R32G32B32A32Sint
            | Self::R32G32B32A32Sfloat
            | Self::Float4
            | Self::Int4
            | Self::Uint4
            | Self::R64G64B64A64Uint
            | Self::R64G64B64A64Sint
            | Self::R64G64B64A64Sfloat
            | Self::A2B10G10R10UnormPack32
            | Self::A2B10G10R10UintPack32 => 4,
        }
    }

    /// Is float format
    #[inline]
    pub const fn is_float(&self) -> bool {
        matches!(
            self,
            Self::R16Sfloat
                | Self::R16G16Sfloat
                | Self::R16G16B16Sfloat
                | Self::R16G16B16A16Sfloat
                | Self::R32Sfloat
                | Self::R32G32Sfloat
                | Self::R32G32B32Sfloat
                | Self::R32G32B32A32Sfloat
                | Self::R64Sfloat
                | Self::R64G64Sfloat
                | Self::R64G64B64Sfloat
                | Self::R64G64B64A64Sfloat
                | Self::Float
                | Self::Float2
                | Self::Float3
                | Self::Float4
                | Self::Half
                | Self::Half2
                | Self::Half3
                | Self::Half4
        )
    }

    /// Is integer format
    #[inline]
    pub const fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::R8Uint
                | Self::R8Sint
                | Self::R8G8Uint
                | Self::R8G8Sint
                | Self::R8G8B8Uint
                | Self::R8G8B8Sint
                | Self::R8G8B8A8Uint
                | Self::R8G8B8A8Sint
                | Self::R16Uint
                | Self::R16Sint
                | Self::R16G16Uint
                | Self::R16G16Sint
                | Self::R16G16B16Uint
                | Self::R16G16B16Sint
                | Self::R16G16B16A16Uint
                | Self::R16G16B16A16Sint
                | Self::R32Uint
                | Self::R32Sint
                | Self::R32G32Uint
                | Self::R32G32Sint
                | Self::R32G32B32Uint
                | Self::R32G32B32Sint
                | Self::R32G32B32A32Uint
                | Self::R32G32B32A32Sint
                | Self::R64Uint
                | Self::R64Sint
                | Self::R64G64Uint
                | Self::R64G64Sint
                | Self::R64G64B64Uint
                | Self::R64G64B64Sint
                | Self::R64G64B64A64Uint
                | Self::R64G64B64A64Sint
                | Self::Int
                | Self::Int2
                | Self::Int3
                | Self::Int4
                | Self::Uint
                | Self::Uint2
                | Self::Uint3
                | Self::Uint4
                | Self::A2B10G10R10UintPack32
        )
    }

    /// Is normalized format
    #[inline]
    pub const fn is_normalized(&self) -> bool {
        matches!(
            self,
            Self::R8Unorm
                | Self::R8Snorm
                | Self::R8G8Unorm
                | Self::R8G8Snorm
                | Self::R8G8B8Unorm
                | Self::R8G8B8Snorm
                | Self::R8G8B8A8Unorm
                | Self::R8G8B8A8Snorm
                | Self::R16Unorm
                | Self::R16Snorm
                | Self::R16G16Unorm
                | Self::R16G16Snorm
                | Self::R16G16B16Unorm
                | Self::R16G16B16Snorm
                | Self::R16G16B16A16Unorm
                | Self::R16G16B16A16Snorm
                | Self::A2B10G10R10UnormPack32
        )
    }
}

// ============================================================================
// Input Assembly State
// ============================================================================

/// Input assembly state create info
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct InputAssemblyStateCreateInfo {
    /// Flags
    pub flags: InputAssemblyStateCreateFlags,
    /// Topology
    pub topology: PrimitiveTopology,
    /// Primitive restart enable
    pub primitive_restart_enable: bool,
}

impl InputAssemblyStateCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(topology: PrimitiveTopology) -> Self {
        Self {
            flags: InputAssemblyStateCreateFlags::NONE,
            topology,
            primitive_restart_enable: false,
        }
    }

    /// Triangle list (most common)
    pub const TRIANGLE_LIST: Self = Self::new(PrimitiveTopology::TriangleList);

    /// Triangle strip
    pub const TRIANGLE_STRIP: Self = Self {
        flags: InputAssemblyStateCreateFlags::NONE,
        topology: PrimitiveTopology::TriangleStrip,
        primitive_restart_enable: true,
    };

    /// Triangle fan
    pub const TRIANGLE_FAN: Self = Self::new(PrimitiveTopology::TriangleFan);

    /// Line list
    pub const LINE_LIST: Self = Self::new(PrimitiveTopology::LineList);

    /// Line strip
    pub const LINE_STRIP: Self = Self {
        flags: InputAssemblyStateCreateFlags::NONE,
        topology: PrimitiveTopology::LineStrip,
        primitive_restart_enable: true,
    };

    /// Point list
    pub const POINT_LIST: Self = Self::new(PrimitiveTopology::PointList);

    /// With primitive restart
    #[inline]
    pub const fn with_primitive_restart(mut self) -> Self {
        self.primitive_restart_enable = true;
        self
    }

    /// Without primitive restart
    #[inline]
    pub const fn without_primitive_restart(mut self) -> Self {
        self.primitive_restart_enable = false;
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: InputAssemblyStateCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for InputAssemblyStateCreateInfo {
    fn default() -> Self {
        Self::TRIANGLE_LIST
    }
}

/// Input assembly state create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct InputAssemblyStateCreateFlags(pub u32);

impl InputAssemblyStateCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);

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
// Primitive Topology
// ============================================================================

/// Primitive topology
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PrimitiveTopology {
    /// Point list
    PointList     = 0,
    /// Line list
    LineList      = 1,
    /// Line strip
    LineStrip     = 2,
    /// Triangle list
    #[default]
    TriangleList  = 3,
    /// Triangle strip
    TriangleStrip = 4,
    /// Triangle fan
    TriangleFan   = 5,
    /// Line list with adjacency
    LineListWithAdjacency = 6,
    /// Line strip with adjacency
    LineStripWithAdjacency = 7,
    /// Triangle list with adjacency
    TriangleListWithAdjacency = 8,
    /// Triangle strip with adjacency
    TriangleStripWithAdjacency = 9,
    /// Patch list (for tessellation)
    PatchList     = 10,
}

impl PrimitiveTopology {
    /// Is point primitive
    #[inline]
    pub const fn is_point(&self) -> bool {
        matches!(self, Self::PointList)
    }

    /// Is line primitive
    #[inline]
    pub const fn is_line(&self) -> bool {
        matches!(
            self,
            Self::LineList
                | Self::LineStrip
                | Self::LineListWithAdjacency
                | Self::LineStripWithAdjacency
        )
    }

    /// Is triangle primitive
    #[inline]
    pub const fn is_triangle(&self) -> bool {
        matches!(
            self,
            Self::TriangleList
                | Self::TriangleStrip
                | Self::TriangleFan
                | Self::TriangleListWithAdjacency
                | Self::TriangleStripWithAdjacency
        )
    }

    /// Is strip topology
    #[inline]
    pub const fn is_strip(&self) -> bool {
        matches!(
            self,
            Self::LineStrip
                | Self::TriangleStrip
                | Self::LineStripWithAdjacency
                | Self::TriangleStripWithAdjacency
        )
    }

    /// Is list topology
    #[inline]
    pub const fn is_list(&self) -> bool {
        matches!(
            self,
            Self::PointList
                | Self::LineList
                | Self::TriangleList
                | Self::LineListWithAdjacency
                | Self::TriangleListWithAdjacency
        )
    }

    /// Has adjacency data
    #[inline]
    pub const fn has_adjacency(&self) -> bool {
        matches!(
            self,
            Self::LineListWithAdjacency
                | Self::LineStripWithAdjacency
                | Self::TriangleListWithAdjacency
                | Self::TriangleStripWithAdjacency
        )
    }

    /// Is patch topology
    #[inline]
    pub const fn is_patch(&self) -> bool {
        matches!(self, Self::PatchList)
    }

    /// Vertices per primitive (for lists)
    #[inline]
    pub const fn vertices_per_primitive(&self) -> u32 {
        match self {
            Self::PointList => 1,
            Self::LineList => 2,
            Self::TriangleList | Self::TriangleFan => 3,
            Self::LineListWithAdjacency => 4,
            Self::TriangleListWithAdjacency => 6,
            Self::LineStrip
            | Self::TriangleStrip
            | Self::LineStripWithAdjacency
            | Self::TriangleStripWithAdjacency
            | Self::PatchList => 0, // Variable
        }
    }
}

// ============================================================================
// Tessellation State
// ============================================================================

/// Tessellation state create info
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct TessellationStateCreateInfo {
    /// Flags
    pub flags: TessellationStateCreateFlags,
    /// Patch control points
    pub patch_control_points: u32,
}

impl TessellationStateCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(patch_control_points: u32) -> Self {
        Self {
            flags: TessellationStateCreateFlags::NONE,
            patch_control_points,
        }
    }

    /// Triangle patches (3 control points)
    pub const TRIANGLE: Self = Self::new(3);

    /// Quad patches (4 control points)
    pub const QUAD: Self = Self::new(4);

    /// Isoline patches (2 control points)
    pub const ISOLINE: Self = Self::new(2);

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: TessellationStateCreateFlags) -> Self {
        self.flags = flags;
        self
    }
}

impl Default for TessellationStateCreateInfo {
    fn default() -> Self {
        Self::TRIANGLE
    }
}

/// Tessellation state create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct TessellationStateCreateFlags(pub u32);

impl TessellationStateCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);

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

/// Tessellation domain origin
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TessellationDomainOrigin {
    /// Upper left
    #[default]
    UpperLeft = 0,
    /// Lower left
    LowerLeft = 1,
}
