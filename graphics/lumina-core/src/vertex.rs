//! Vertex input and vertex buffer types
//!
//! This module provides types for vertex buffer layouts and input state.

/// Vertex buffer binding
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VertexBinding {
    /// Binding index
    pub binding: u32,
    /// Stride in bytes
    pub stride: u32,
    /// Input rate
    pub input_rate: VertexInputRate,
    /// Divisor (for instanced rendering)
    pub divisor: u32,
}

impl VertexBinding {
    /// Creates per-vertex binding
    pub const fn per_vertex(binding: u32, stride: u32) -> Self {
        Self {
            binding,
            stride,
            input_rate: VertexInputRate::Vertex,
            divisor: 1,
        }
    }

    /// Creates per-instance binding
    pub const fn per_instance(binding: u32, stride: u32) -> Self {
        Self {
            binding,
            stride,
            input_rate: VertexInputRate::Instance,
            divisor: 1,
        }
    }

    /// With divisor
    pub const fn with_divisor(mut self, divisor: u32) -> Self {
        self.divisor = divisor;
        self
    }
}

/// Vertex input rate
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum VertexInputRate {
    /// Per vertex
    #[default]
    Vertex = 0,
    /// Per instance
    Instance = 1,
}

/// Vertex attribute
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct VertexAttribute {
    /// Location in shader
    pub location: u32,
    /// Binding index
    pub binding: u32,
    /// Attribute format
    pub format: VertexFormat,
    /// Offset in bytes from start of vertex
    pub offset: u32,
}

impl VertexAttribute {
    /// Creates new attribute
    pub const fn new(location: u32, binding: u32, format: VertexFormat, offset: u32) -> Self {
        Self {
            location,
            binding,
            format,
            offset,
        }
    }
}

/// Vertex format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum VertexFormat {
    // Single-component formats
    /// 8-bit unsigned normalized
    R8Unorm = 0,
    /// 8-bit signed normalized
    R8Snorm = 1,
    /// 8-bit unsigned int
    R8Uint = 2,
    /// 8-bit signed int
    R8Sint = 3,
    /// 16-bit unsigned normalized
    R16Unorm = 4,
    /// 16-bit signed normalized
    R16Snorm = 5,
    /// 16-bit unsigned int
    R16Uint = 6,
    /// 16-bit signed int
    R16Sint = 7,
    /// 16-bit float
    R16Float = 8,
    /// 32-bit unsigned int
    R32Uint = 9,
    /// 32-bit signed int
    R32Sint = 10,
    /// 32-bit float
    R32Float = 11,
    /// 64-bit float (double)
    R64Float = 12,

    // Two-component formats
    /// 2x 8-bit unsigned normalized
    Rg8Unorm = 20,
    /// 2x 8-bit signed normalized
    Rg8Snorm = 21,
    /// 2x 8-bit unsigned int
    Rg8Uint = 22,
    /// 2x 8-bit signed int
    Rg8Sint = 23,
    /// 2x 16-bit unsigned normalized
    Rg16Unorm = 24,
    /// 2x 16-bit signed normalized
    Rg16Snorm = 25,
    /// 2x 16-bit unsigned int
    Rg16Uint = 26,
    /// 2x 16-bit signed int
    Rg16Sint = 27,
    /// 2x 16-bit float
    Rg16Float = 28,
    /// 2x 32-bit unsigned int
    Rg32Uint = 29,
    /// 2x 32-bit signed int
    Rg32Sint = 30,
    /// 2x 32-bit float
    Rg32Float = 31,
    /// 2x 64-bit float
    Rg64Float = 32,

    // Three-component formats
    /// 3x 8-bit unsigned normalized
    Rgb8Unorm = 40,
    /// 3x 8-bit signed normalized
    Rgb8Snorm = 41,
    /// 3x 8-bit unsigned int
    Rgb8Uint = 42,
    /// 3x 8-bit signed int
    Rgb8Sint = 43,
    /// 3x 16-bit unsigned normalized
    Rgb16Unorm = 44,
    /// 3x 16-bit signed normalized
    Rgb16Snorm = 45,
    /// 3x 16-bit unsigned int
    Rgb16Uint = 46,
    /// 3x 16-bit signed int
    Rgb16Sint = 47,
    /// 3x 16-bit float
    Rgb16Float = 48,
    /// 3x 32-bit unsigned int
    Rgb32Uint = 49,
    /// 3x 32-bit signed int
    Rgb32Sint = 50,
    /// 3x 32-bit float
    Rgb32Float = 51,
    /// 3x 64-bit float
    Rgb64Float = 52,

    // Four-component formats
    /// 4x 8-bit unsigned normalized
    Rgba8Unorm = 60,
    /// 4x 8-bit signed normalized
    Rgba8Snorm = 61,
    /// 4x 8-bit unsigned int
    Rgba8Uint = 62,
    /// 4x 8-bit signed int
    Rgba8Sint = 63,
    /// 4x 16-bit unsigned normalized
    Rgba16Unorm = 64,
    /// 4x 16-bit signed normalized
    Rgba16Snorm = 65,
    /// 4x 16-bit unsigned int
    Rgba16Uint = 66,
    /// 4x 16-bit signed int
    Rgba16Sint = 67,
    /// 4x 16-bit float
    Rgba16Float = 68,
    /// 4x 32-bit unsigned int
    Rgba32Uint = 69,
    /// 4x 32-bit signed int
    Rgba32Sint = 70,
    /// 4x 32-bit float
    Rgba32Float = 71,
    /// 4x 64-bit float
    Rgba64Float = 72,

    // Special formats
    /// RGB 10-bit, A 2-bit unsigned normalized
    Rgb10A2Unorm = 80,
    /// RGB 10-bit, A 2-bit unsigned int
    Rgb10A2Uint = 81,
    /// RG 11-bit float, B 10-bit float
    Rg11B10Float = 82,

    // Packed integer formats
    /// BGRA 8-bit unsigned normalized
    Bgra8Unorm = 90,
    /// BGRA 8-bit sRGB
    Bgra8Srgb = 91,
}

impl VertexFormat {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::R8Unorm
            | Self::R8Snorm
            | Self::R8Uint
            | Self::R8Sint => 1,

            Self::Rg8Unorm
            | Self::Rg8Snorm
            | Self::Rg8Uint
            | Self::Rg8Sint
            | Self::R16Unorm
            | Self::R16Snorm
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Float => 2,

            Self::Rgb8Unorm
            | Self::Rgb8Snorm
            | Self::Rgb8Uint
            | Self::Rgb8Sint => 3,

            Self::Rgba8Unorm
            | Self::Rgba8Snorm
            | Self::Rgba8Uint
            | Self::Rgba8Sint
            | Self::Bgra8Unorm
            | Self::Bgra8Srgb
            | Self::Rg16Unorm
            | Self::Rg16Snorm
            | Self::Rg16Uint
            | Self::Rg16Sint
            | Self::Rg16Float
            | Self::R32Uint
            | Self::R32Sint
            | Self::R32Float
            | Self::Rgb10A2Unorm
            | Self::Rgb10A2Uint
            | Self::Rg11B10Float => 4,

            Self::Rgb16Unorm
            | Self::Rgb16Snorm
            | Self::Rgb16Uint
            | Self::Rgb16Sint
            | Self::Rgb16Float => 6,

            Self::Rgba16Unorm
            | Self::Rgba16Snorm
            | Self::Rgba16Uint
            | Self::Rgba16Sint
            | Self::Rgba16Float
            | Self::Rg32Uint
            | Self::Rg32Sint
            | Self::Rg32Float
            | Self::R64Float => 8,

            Self::Rgb32Uint
            | Self::Rgb32Sint
            | Self::Rgb32Float => 12,

            Self::Rgba32Uint
            | Self::Rgba32Sint
            | Self::Rgba32Float
            | Self::Rg64Float => 16,

            Self::Rgb64Float => 24,
            Self::Rgba64Float => 32,
        }
    }

    /// Component count
    pub const fn components(&self) -> u32 {
        match self {
            Self::R8Unorm
            | Self::R8Snorm
            | Self::R8Uint
            | Self::R8Sint
            | Self::R16Unorm
            | Self::R16Snorm
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Float
            | Self::R32Uint
            | Self::R32Sint
            | Self::R32Float
            | Self::R64Float => 1,

            Self::Rg8Unorm
            | Self::Rg8Snorm
            | Self::Rg8Uint
            | Self::Rg8Sint
            | Self::Rg16Unorm
            | Self::Rg16Snorm
            | Self::Rg16Uint
            | Self::Rg16Sint
            | Self::Rg16Float
            | Self::Rg32Uint
            | Self::Rg32Sint
            | Self::Rg32Float
            | Self::Rg64Float => 2,

            Self::Rgb8Unorm
            | Self::Rgb8Snorm
            | Self::Rgb8Uint
            | Self::Rgb8Sint
            | Self::Rgb16Unorm
            | Self::Rgb16Snorm
            | Self::Rgb16Uint
            | Self::Rgb16Sint
            | Self::Rgb16Float
            | Self::Rgb32Uint
            | Self::Rgb32Sint
            | Self::Rgb32Float
            | Self::Rgb64Float
            | Self::Rg11B10Float => 3,

            Self::Rgba8Unorm
            | Self::Rgba8Snorm
            | Self::Rgba8Uint
            | Self::Rgba8Sint
            | Self::Bgra8Unorm
            | Self::Bgra8Srgb
            | Self::Rgba16Unorm
            | Self::Rgba16Snorm
            | Self::Rgba16Uint
            | Self::Rgba16Sint
            | Self::Rgba16Float
            | Self::Rgba32Uint
            | Self::Rgba32Sint
            | Self::Rgba32Float
            | Self::Rgba64Float
            | Self::Rgb10A2Unorm
            | Self::Rgb10A2Uint => 4,
        }
    }

    /// Is normalized format
    pub const fn is_normalized(&self) -> bool {
        matches!(
            self,
            Self::R8Unorm
                | Self::R8Snorm
                | Self::Rg8Unorm
                | Self::Rg8Snorm
                | Self::Rgb8Unorm
                | Self::Rgb8Snorm
                | Self::Rgba8Unorm
                | Self::Rgba8Snorm
                | Self::Bgra8Unorm
                | Self::Bgra8Srgb
                | Self::R16Unorm
                | Self::R16Snorm
                | Self::Rg16Unorm
                | Self::Rg16Snorm
                | Self::Rgb16Unorm
                | Self::Rgb16Snorm
                | Self::Rgba16Unorm
                | Self::Rgba16Snorm
                | Self::Rgb10A2Unorm
        )
    }

    /// Is float format
    pub const fn is_float(&self) -> bool {
        matches!(
            self,
            Self::R16Float
                | Self::Rg16Float
                | Self::Rgb16Float
                | Self::Rgba16Float
                | Self::R32Float
                | Self::Rg32Float
                | Self::Rgb32Float
                | Self::Rgba32Float
                | Self::R64Float
                | Self::Rg64Float
                | Self::Rgb64Float
                | Self::Rgba64Float
                | Self::Rg11B10Float
        )
    }
}

/// Common vertex formats
pub mod formats {
    use super::VertexFormat;

    /// Position only (Vec3)
    pub const POSITION: VertexFormat = VertexFormat::Rgb32Float;
    /// Position with W (Vec4)
    pub const POSITION_W: VertexFormat = VertexFormat::Rgba32Float;
    /// Normal (Vec3)
    pub const NORMAL: VertexFormat = VertexFormat::Rgb32Float;
    /// Tangent (Vec4)
    pub const TANGENT: VertexFormat = VertexFormat::Rgba32Float;
    /// UV (Vec2)
    pub const UV: VertexFormat = VertexFormat::Rg32Float;
    /// Color RGBA (Vec4)
    pub const COLOR: VertexFormat = VertexFormat::Rgba32Float;
    /// Color RGBA normalized (u8x4)
    pub const COLOR_UNORM: VertexFormat = VertexFormat::Rgba8Unorm;
    /// Joint indices (u16x4)
    pub const JOINTS: VertexFormat = VertexFormat::Rgba16Uint;
    /// Joint weights (f32x4)
    pub const WEIGHTS: VertexFormat = VertexFormat::Rgba32Float;
    /// Joint weights normalized (u8x4)
    pub const WEIGHTS_UNORM: VertexFormat = VertexFormat::Rgba8Unorm;
}

/// Vertex layout descriptor
#[derive(Clone, Debug, Default)]
pub struct VertexLayout {
    /// Bindings
    bindings: [Option<VertexBinding>; 16],
    /// Binding count
    binding_count: u32,
    /// Attributes
    attributes: [Option<VertexAttribute>; 32],
    /// Attribute count
    attribute_count: u32,
}

impl VertexLayout {
    /// Creates empty layout
    pub const fn new() -> Self {
        Self {
            bindings: [None; 16],
            binding_count: 0,
            attributes: [None; 32],
            attribute_count: 0,
        }
    }

    /// Adds a binding
    pub fn add_binding(&mut self, binding: VertexBinding) -> &mut Self {
        if (binding.binding as usize) < 16 {
            self.bindings[binding.binding as usize] = Some(binding);
            self.binding_count = self.binding_count.max(binding.binding + 1);
        }
        self
    }

    /// Adds an attribute
    pub fn add_attribute(&mut self, attr: VertexAttribute) -> &mut Self {
        if (self.attribute_count as usize) < 32 {
            self.attributes[self.attribute_count as usize] = Some(attr);
            self.attribute_count += 1;
        }
        self
    }

    /// Binding count
    pub const fn binding_count(&self) -> u32 {
        self.binding_count
    }

    /// Attribute count
    pub const fn attribute_count(&self) -> u32 {
        self.attribute_count
    }

    /// Total vertex size for binding
    pub fn binding_stride(&self, binding: u32) -> u32 {
        self.bindings
            .get(binding as usize)
            .and_then(|b| b.map(|b| b.stride))
            .unwrap_or(0)
    }
}

/// Standard vertex layouts
pub mod layouts {
    use super::*;

    /// Position only layout
    pub fn position_only() -> VertexLayout {
        let mut layout = VertexLayout::new();
        layout.add_binding(VertexBinding::per_vertex(0, 12));
        layout.add_attribute(VertexAttribute::new(0, 0, VertexFormat::Rgb32Float, 0));
        layout
    }

    /// Position + UV layout
    pub fn position_uv() -> VertexLayout {
        let mut layout = VertexLayout::new();
        layout.add_binding(VertexBinding::per_vertex(0, 20));
        layout.add_attribute(VertexAttribute::new(0, 0, VertexFormat::Rgb32Float, 0));
        layout.add_attribute(VertexAttribute::new(1, 0, VertexFormat::Rg32Float, 12));
        layout
    }

    /// Position + Normal + UV layout
    pub fn position_normal_uv() -> VertexLayout {
        let mut layout = VertexLayout::new();
        layout.add_binding(VertexBinding::per_vertex(0, 32));
        layout.add_attribute(VertexAttribute::new(0, 0, VertexFormat::Rgb32Float, 0));
        layout.add_attribute(VertexAttribute::new(1, 0, VertexFormat::Rgb32Float, 12));
        layout.add_attribute(VertexAttribute::new(2, 0, VertexFormat::Rg32Float, 24));
        layout
    }

    /// Position + Normal + Tangent + UV layout
    pub fn position_normal_tangent_uv() -> VertexLayout {
        let mut layout = VertexLayout::new();
        layout.add_binding(VertexBinding::per_vertex(0, 48));
        layout.add_attribute(VertexAttribute::new(0, 0, VertexFormat::Rgb32Float, 0));
        layout.add_attribute(VertexAttribute::new(1, 0, VertexFormat::Rgb32Float, 12));
        layout.add_attribute(VertexAttribute::new(2, 0, VertexFormat::Rgba32Float, 24));
        layout.add_attribute(VertexAttribute::new(3, 0, VertexFormat::Rg32Float, 40));
        layout
    }

    /// Skinned vertex layout (position + normal + joints + weights)
    pub fn skinned() -> VertexLayout {
        let mut layout = VertexLayout::new();
        layout.add_binding(VertexBinding::per_vertex(0, 48));
        layout.add_attribute(VertexAttribute::new(0, 0, VertexFormat::Rgb32Float, 0));
        layout.add_attribute(VertexAttribute::new(1, 0, VertexFormat::Rgb32Float, 12));
        layout.add_attribute(VertexAttribute::new(2, 0, VertexFormat::Rgba16Uint, 24));
        layout.add_attribute(VertexAttribute::new(3, 0, VertexFormat::Rgba32Float, 32));
        layout
    }
}

/// Primitive topology
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PrimitiveTopology {
    /// Point list
    PointList = 0,
    /// Line list
    LineList = 1,
    /// Line strip
    LineStrip = 2,
    /// Triangle list
    #[default]
    TriangleList = 3,
    /// Triangle strip
    TriangleStrip = 4,
    /// Triangle fan
    TriangleFan = 5,
    /// Line list with adjacency
    LineListWithAdjacency = 6,
    /// Line strip with adjacency
    LineStripWithAdjacency = 7,
    /// Triangle list with adjacency
    TriangleListWithAdjacency = 8,
    /// Triangle strip with adjacency
    TriangleStripWithAdjacency = 9,
    /// Patch list (for tessellation)
    PatchList = 10,
}

impl PrimitiveTopology {
    /// Has adjacency info
    pub const fn has_adjacency(&self) -> bool {
        matches!(
            self,
            Self::LineListWithAdjacency
                | Self::LineStripWithAdjacency
                | Self::TriangleListWithAdjacency
                | Self::TriangleStripWithAdjacency
        )
    }

    /// Is strip topology
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

    /// Vertices per primitive
    pub const fn vertices_per_primitive(&self) -> u32 {
        match self {
            Self::PointList => 1,
            Self::LineList | Self::LineStrip => 2,
            Self::TriangleList | Self::TriangleStrip | Self::TriangleFan => 3,
            Self::LineListWithAdjacency | Self::LineStripWithAdjacency => 4,
            Self::TriangleListWithAdjacency | Self::TriangleStripWithAdjacency => 6,
            Self::PatchList => 0, // Variable
        }
    }
}

/// Index type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum IndexType {
    /// 16-bit indices
    Uint16 = 0,
    /// 32-bit indices
    #[default]
    Uint32 = 1,
    /// 8-bit indices (requires extension)
    Uint8 = 2,
}

impl IndexType {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Uint8 => 1,
            Self::Uint16 => 2,
            Self::Uint32 => 4,
        }
    }

    /// Maximum index value
    pub const fn max_index(&self) -> u32 {
        match self {
            Self::Uint8 => 255,
            Self::Uint16 => 65535,
            Self::Uint32 => u32::MAX,
        }
    }
}

/// Primitive restart
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PrimitiveRestart {
    /// Disabled
    #[default]
    Disabled = 0,
    /// Enabled with 0xFFFF or 0xFFFFFFFF
    Enabled = 1,
    /// Enabled with custom index
    Custom = 2,
}

/// Input assembly state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct InputAssemblyState {
    /// Primitive topology
    pub topology: PrimitiveTopology,
    /// Primitive restart
    pub primitive_restart: PrimitiveRestart,
    /// Patch control points (for tessellation)
    pub patch_control_points: u32,
}

impl Default for InputAssemblyState {
    fn default() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleList,
            primitive_restart: PrimitiveRestart::Disabled,
            patch_control_points: 0,
        }
    }
}

impl InputAssemblyState {
    /// Creates for triangle list
    pub const fn triangles() -> Self {
        Self {
            topology: PrimitiveTopology::TriangleList,
            primitive_restart: PrimitiveRestart::Disabled,
            patch_control_points: 0,
        }
    }

    /// Creates for line list
    pub const fn lines() -> Self {
        Self {
            topology: PrimitiveTopology::LineList,
            primitive_restart: PrimitiveRestart::Disabled,
            patch_control_points: 0,
        }
    }

    /// Creates for points
    pub const fn points() -> Self {
        Self {
            topology: PrimitiveTopology::PointList,
            primitive_restart: PrimitiveRestart::Disabled,
            patch_control_points: 0,
        }
    }

    /// Creates for patches
    pub const fn patches(control_points: u32) -> Self {
        Self {
            topology: PrimitiveTopology::PatchList,
            primitive_restart: PrimitiveRestart::Disabled,
            patch_control_points: control_points,
        }
    }

    /// With primitive restart
    pub const fn with_primitive_restart(mut self, enable: bool) -> Self {
        self.primitive_restart = if enable {
            PrimitiveRestart::Enabled
        } else {
            PrimitiveRestart::Disabled
        };
        self
    }
}
