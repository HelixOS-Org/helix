//! Dynamic state types
//!
//! This module provides types for dynamic pipeline state.

extern crate alloc;
use alloc::vec::Vec;

/// Dynamic state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DynamicState {
    /// Viewport
    Viewport,
    /// Scissor
    Scissor,
    /// Line width
    LineWidth,
    /// Depth bias
    DepthBias,
    /// Blend constants
    BlendConstants,
    /// Depth bounds
    DepthBounds,
    /// Stencil compare mask
    StencilCompareMask,
    /// Stencil write mask
    StencilWriteMask,
    /// Stencil reference
    StencilReference,
    /// Cull mode
    CullMode,
    /// Front face
    FrontFace,
    /// Primitive topology
    PrimitiveTopology,
    /// Viewport with count
    ViewportWithCount,
    /// Scissor with count
    ScissorWithCount,
    /// Vertex input binding stride
    VertexInputBindingStride,
    /// Depth test enable
    DepthTestEnable,
    /// Depth write enable
    DepthWriteEnable,
    /// Depth compare op
    DepthCompareOp,
    /// Depth bounds test enable
    DepthBoundsTestEnable,
    /// Stencil test enable
    StencilTestEnable,
    /// Stencil op
    StencilOp,
    /// Rasterizer discard enable
    RasterizerDiscardEnable,
    /// Depth bias enable
    DepthBiasEnable,
    /// Primitive restart enable
    PrimitiveRestartEnable,
    /// Viewport W scaling (NV)
    ViewportWScalingNV,
    /// Discard rectangle (EXT)
    DiscardRectangleEXT,
    /// Sample locations (EXT)
    SampleLocationsEXT,
    /// Ray tracing pipeline stack size (KHR)
    RayTracingPipelineStackSizeKHR,
    /// Viewport shading rate palette (NV)
    ViewportShadingRatePaletteNV,
    /// Viewport coarse sample order (NV)
    ViewportCoarseSampleOrderNV,
    /// Exclusive scissor (NV)
    ExclusiveScissorNV,
    /// Fragment shading rate (KHR)
    FragmentShadingRateKHR,
    /// Line stipple (EXT)
    LineStippleEXT,
    /// Vertex input (EXT)
    VertexInputEXT,
    /// Patch control points (EXT)
    PatchControlPointsEXT,
    /// Logic op (EXT)
    LogicOpEXT,
    /// Color write enable (EXT)
    ColorWriteEnableEXT,
    /// Tessellation domain origin (EXT)
    TessellationDomainOriginEXT,
    /// Depth clamp enable (EXT)
    DepthClampEnableEXT,
    /// Polygon mode (EXT)
    PolygonModeEXT,
    /// Rasterization samples (EXT)
    RasterizationSamplesEXT,
    /// Sample mask (EXT)
    SampleMaskEXT,
    /// Alpha to coverage enable (EXT)
    AlphaToCoverageEnableEXT,
    /// Alpha to one enable (EXT)
    AlphaToOneEnableEXT,
    /// Logic op enable (EXT)
    LogicOpEnableEXT,
    /// Color blend enable (EXT)
    ColorBlendEnableEXT,
    /// Color blend equation (EXT)
    ColorBlendEquationEXT,
    /// Color write mask (EXT)
    ColorWriteMaskEXT,
    /// Conservative rasterization mode (EXT)
    ConservativeRasterizationModeEXT,
    /// Extra primitive overestimation size (EXT)
    ExtraPrimitiveOverestimationSizeEXT,
    /// Depth clip enable (EXT)
    DepthClipEnableEXT,
    /// Sample locations enable (EXT)
    SampleLocationsEnableEXT,
    /// Color blend advanced (EXT)
    ColorBlendAdvancedEXT,
    /// Provoking vertex mode (EXT)
    ProvokingVertexModeEXT,
    /// Line rasterization mode (EXT)
    LineRasterizationModeEXT,
    /// Line stipple enable (EXT)
    LineStippleEnableEXT,
    /// Depth clip negative one to one (EXT)
    DepthClipNegativeOneToOneEXT,
    /// Viewport W scaling enable (NV)
    ViewportWScalingEnableNV,
    /// Viewport swizzle (NV)
    ViewportSwizzleNV,
    /// Coverage to color enable (NV)
    CoverageToColorEnableNV,
    /// Coverage to color location (NV)
    CoverageToColorLocationNV,
    /// Coverage modulation mode (NV)
    CoverageModulationModeNV,
    /// Coverage modulation table enable (NV)
    CoverageModulationTableEnableNV,
    /// Coverage modulation table (NV)
    CoverageModulationTableNV,
    /// Shading rate image enable (NV)
    ShadingRateImageEnableNV,
    /// Representative fragment test enable (NV)
    RepresentativeFragmentTestEnableNV,
    /// Coverage reduction mode (NV)
    CoverageReductionModeNV,
}

/// Dynamic state flags
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DynamicStateFlags(pub u64);

impl DynamicStateFlags {
    /// No dynamic state
    pub const NONE: Self = Self(0);
    /// Viewport
    pub const VIEWPORT: Self = Self(1 << 0);
    /// Scissor
    pub const SCISSOR: Self = Self(1 << 1);
    /// Line width
    pub const LINE_WIDTH: Self = Self(1 << 2);
    /// Depth bias
    pub const DEPTH_BIAS: Self = Self(1 << 3);
    /// Blend constants
    pub const BLEND_CONSTANTS: Self = Self(1 << 4);
    /// Depth bounds
    pub const DEPTH_BOUNDS: Self = Self(1 << 5);
    /// Stencil compare mask
    pub const STENCIL_COMPARE_MASK: Self = Self(1 << 6);
    /// Stencil write mask
    pub const STENCIL_WRITE_MASK: Self = Self(1 << 7);
    /// Stencil reference
    pub const STENCIL_REFERENCE: Self = Self(1 << 8);
    /// Cull mode
    pub const CULL_MODE: Self = Self(1 << 9);
    /// Front face
    pub const FRONT_FACE: Self = Self(1 << 10);
    /// Primitive topology
    pub const PRIMITIVE_TOPOLOGY: Self = Self(1 << 11);
    /// Depth test enable
    pub const DEPTH_TEST_ENABLE: Self = Self(1 << 12);
    /// Depth write enable
    pub const DEPTH_WRITE_ENABLE: Self = Self(1 << 13);
    /// Depth compare op
    pub const DEPTH_COMPARE_OP: Self = Self(1 << 14);
    /// Stencil test enable
    pub const STENCIL_TEST_ENABLE: Self = Self(1 << 15);
    /// Stencil op
    pub const STENCIL_OP: Self = Self(1 << 16);

    /// Common dynamic states
    pub const COMMON: Self = Self(Self::VIEWPORT.0 | Self::SCISSOR.0);

    /// All extended dynamic states
    pub const EXTENDED: Self = Self(
        Self::VIEWPORT.0
            | Self::SCISSOR.0
            | Self::CULL_MODE.0
            | Self::FRONT_FACE.0
            | Self::PRIMITIVE_TOPOLOGY.0
            | Self::DEPTH_TEST_ENABLE.0
            | Self::DEPTH_WRITE_ENABLE.0
            | Self::DEPTH_COMPARE_OP.0
            | Self::STENCIL_TEST_ENABLE.0
            | Self::STENCIL_OP.0,
    );

    /// Union of flags
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }
}

impl core::ops::BitOr for DynamicStateFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Dynamic state create info
#[derive(Clone, Debug, Default)]
pub struct DynamicStateCreateInfo {
    /// Dynamic states
    pub dynamic_states: Vec<DynamicState>,
}

impl DynamicStateCreateInfo {
    /// Creates new dynamic state info
    pub const fn new() -> Self {
        Self {
            dynamic_states: Vec::new(),
        }
    }

    /// Adds a dynamic state
    pub fn add(mut self, state: DynamicState) -> Self {
        self.dynamic_states.push(state);
        self
    }

    /// Common dynamic states (viewport + scissor)
    pub fn common() -> Self {
        Self {
            dynamic_states: alloc::vec![DynamicState::Viewport, DynamicState::Scissor],
        }
    }

    /// Extended dynamic states
    pub fn extended() -> Self {
        Self {
            dynamic_states: alloc::vec![
                DynamicState::Viewport,
                DynamicState::Scissor,
                DynamicState::CullMode,
                DynamicState::FrontFace,
                DynamicState::PrimitiveTopology,
                DynamicState::DepthTestEnable,
                DynamicState::DepthWriteEnable,
                DynamicState::DepthCompareOp,
                DynamicState::StencilTestEnable,
                DynamicState::StencilOp,
            ],
        }
    }

    /// Stencil dynamic states
    pub fn stencil() -> Self {
        Self {
            dynamic_states: alloc::vec![
                DynamicState::StencilCompareMask,
                DynamicState::StencilWriteMask,
                DynamicState::StencilReference,
            ],
        }
    }

    /// Checks if state is dynamic
    pub fn is_dynamic(&self, state: DynamicState) -> bool {
        self.dynamic_states.contains(&state)
    }
}

/// Dynamic rendering info
#[derive(Clone, Debug, Default)]
pub struct DynamicRenderingInfo {
    /// View mask
    pub view_mask: u32,
    /// Color attachment formats
    pub color_attachment_formats: Vec<ImageFormat>,
    /// Depth attachment format
    pub depth_attachment_format: Option<ImageFormat>,
    /// Stencil attachment format
    pub stencil_attachment_format: Option<ImageFormat>,
}

impl DynamicRenderingInfo {
    /// Creates new info
    pub const fn new() -> Self {
        Self {
            view_mask: 0,
            color_attachment_formats: Vec::new(),
            depth_attachment_format: None,
            stencil_attachment_format: None,
        }
    }

    /// Adds color attachment
    pub fn add_color(mut self, format: ImageFormat) -> Self {
        self.color_attachment_formats.push(format);
        self
    }

    /// With depth attachment
    pub fn with_depth(mut self, format: ImageFormat) -> Self {
        self.depth_attachment_format = Some(format);
        self
    }

    /// With depth/stencil attachment
    pub fn with_depth_stencil(mut self, format: ImageFormat) -> Self {
        self.depth_attachment_format = Some(format);
        self.stencil_attachment_format = Some(format);
        self
    }

    /// With view mask (multiview)
    pub const fn with_view_mask(mut self, mask: u32) -> Self {
        self.view_mask = mask;
        self
    }
}

/// Image format (simplified for dynamic rendering)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ImageFormat {
    /// Undefined
    #[default]
    Undefined = 0,
    /// R8 unorm
    R8Unorm = 9,
    /// R8G8 unorm
    R8G8Unorm = 16,
    /// R8G8B8 unorm
    R8G8B8Unorm = 23,
    /// R8G8B8A8 unorm
    R8G8B8A8Unorm = 37,
    /// R8G8B8A8 sRGB
    R8G8B8A8Srgb = 43,
    /// B8G8R8A8 unorm
    B8G8R8A8Unorm = 44,
    /// B8G8R8A8 sRGB
    B8G8R8A8Srgb = 50,
    /// R16 float
    R16Float = 76,
    /// R16G16 float
    R16G16Float = 83,
    /// R16G16B16A16 float
    R16G16B16A16Float = 97,
    /// R32 float
    R32Float = 100,
    /// R32G32 float
    R32G32Float = 103,
    /// R32G32B32 float
    R32G32B32Float = 106,
    /// R32G32B32A32 float
    R32G32B32A32Float = 109,
    /// D16 unorm
    D16Unorm = 124,
    /// D32 float
    D32Float = 126,
    /// D24 unorm S8 uint
    D24UnormS8Uint = 129,
    /// D32 float S8 uint
    D32FloatS8Uint = 130,
}

impl ImageFormat {
    /// Is depth format
    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::D16Unorm | Self::D32Float | Self::D24UnormS8Uint | Self::D32FloatS8Uint
        )
    }

    /// Is stencil format
    pub const fn is_stencil(&self) -> bool {
        matches!(self, Self::D24UnormS8Uint | Self::D32FloatS8Uint)
    }

    /// Is depth/stencil format
    pub const fn is_depth_stencil(&self) -> bool {
        self.is_depth() || self.is_stencil()
    }

    /// Is sRGB format
    pub const fn is_srgb(&self) -> bool {
        matches!(self, Self::R8G8B8A8Srgb | Self::B8G8R8A8Srgb)
    }

    /// Is color format
    pub const fn is_color(&self) -> bool {
        !self.is_depth_stencil() && !matches!(self, Self::Undefined)
    }

    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::R8Unorm => 1,
            Self::R8G8Unorm | Self::R16Float | Self::D16Unorm => 2,
            Self::R8G8B8Unorm => 3,
            Self::R8G8B8A8Unorm
            | Self::R8G8B8A8Srgb
            | Self::B8G8R8A8Unorm
            | Self::B8G8R8A8Srgb
            | Self::R16G16Float
            | Self::R32Float
            | Self::D32Float
            | Self::D24UnormS8Uint => 4,
            Self::D32FloatS8Uint => 5,
            Self::R16G16B16A16Float | Self::R32G32Float => 8,
            Self::R32G32B32Float => 12,
            Self::R32G32B32A32Float => 16,
        }
    }
}

/// Pipeline rendering create info
#[derive(Clone, Debug, Default)]
pub struct PipelineRenderingCreateInfo {
    /// View mask
    pub view_mask: u32,
    /// Color attachment count
    pub color_attachment_count: u32,
    /// Color attachment formats
    pub color_attachment_formats: Vec<ImageFormat>,
    /// Depth attachment format
    pub depth_attachment_format: ImageFormat,
    /// Stencil attachment format
    pub stencil_attachment_format: ImageFormat,
}

impl PipelineRenderingCreateInfo {
    /// Creates new info
    pub const fn new() -> Self {
        Self {
            view_mask: 0,
            color_attachment_count: 0,
            color_attachment_formats: Vec::new(),
            depth_attachment_format: ImageFormat::Undefined,
            stencil_attachment_format: ImageFormat::Undefined,
        }
    }

    /// Adds color attachment format
    pub fn add_color_format(mut self, format: ImageFormat) -> Self {
        self.color_attachment_formats.push(format);
        self.color_attachment_count += 1;
        self
    }

    /// Sets depth format
    pub const fn with_depth_format(mut self, format: ImageFormat) -> Self {
        self.depth_attachment_format = format;
        self
    }

    /// Sets stencil format
    pub const fn with_stencil_format(mut self, format: ImageFormat) -> Self {
        self.stencil_attachment_format = format;
        self
    }

    /// Sets depth/stencil format
    pub const fn with_depth_stencil_format(mut self, format: ImageFormat) -> Self {
        self.depth_attachment_format = format;
        self.stencil_attachment_format = format;
        self
    }
}

/// Color attachment rendering info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RenderingAttachmentInfo {
    /// Image view handle
    pub image_view: u64,
    /// Image layout
    pub image_layout: ImageLayout,
    /// Resolve mode
    pub resolve_mode: ResolveMode,
    /// Resolve image view
    pub resolve_image_view: u64,
    /// Resolve image layout
    pub resolve_image_layout: ImageLayout,
    /// Load op
    pub load_op: AttachmentLoadOp,
    /// Store op
    pub store_op: AttachmentStoreOp,
    /// Clear value
    pub clear_value: ClearValue,
}

impl Default for RenderingAttachmentInfo {
    fn default() -> Self {
        Self {
            image_view: 0,
            image_layout: ImageLayout::ColorAttachmentOptimal,
            resolve_mode: ResolveMode::None,
            resolve_image_view: 0,
            resolve_image_layout: ImageLayout::ColorAttachmentOptimal,
            load_op: AttachmentLoadOp::Clear,
            store_op: AttachmentStoreOp::Store,
            clear_value: ClearValue::color(0.0, 0.0, 0.0, 1.0),
        }
    }
}

/// Image layout
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined = 0,
    /// General
    General = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth stencil read only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer src optimal
    TransferSrcOptimal = 6,
    /// Transfer dst optimal
    TransferDstOptimal = 7,
    /// Preinitialized
    Preinitialized = 8,
    /// Present src
    PresentSrc = 1000001002,
    /// Read only optimal
    ReadOnlyOptimal = 1000314000,
    /// Attachment optimal
    AttachmentOptimal = 1000314001,
}

/// Resolve mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ResolveMode {
    /// No resolve
    #[default]
    None,
    /// Sample zero
    SampleZero,
    /// Average
    Average,
    /// Min
    Min,
    /// Max
    Max,
}

/// Attachment load op
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AttachmentLoadOp {
    /// Load existing contents
    Load,
    /// Clear to value
    #[default]
    Clear,
    /// Don't care
    DontCare,
    /// None
    NoneEXT,
}

/// Attachment store op
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AttachmentStoreOp {
    /// Store contents
    #[default]
    Store,
    /// Don't care
    DontCare,
    /// None
    NoneEXT,
}

/// Clear value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub union ClearValue {
    /// Color clear value
    pub color: ClearColorValue,
    /// Depth stencil clear value
    pub depth_stencil: ClearDepthStencilValue,
}

impl Default for ClearValue {
    fn default() -> Self {
        Self {
            color: ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        }
    }
}

impl ClearValue {
    /// Creates color clear value
    pub const fn color(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: ClearColorValue {
                float32: [r, g, b, a],
            },
        }
    }

    /// Creates depth clear value
    pub const fn depth(depth: f32) -> Self {
        Self {
            depth_stencil: ClearDepthStencilValue { depth, stencil: 0 },
        }
    }

    /// Creates depth/stencil clear value
    pub const fn depth_stencil(depth: f32, stencil: u32) -> Self {
        Self {
            depth_stencil: ClearDepthStencilValue { depth, stencil },
        }
    }
}

/// Clear color value
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub union ClearColorValue {
    /// Float values
    pub float32: [f32; 4],
    /// Int values
    pub int32: [i32; 4],
    /// Uint values
    pub uint32: [u32; 4],
}

/// Clear depth stencil value
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ClearDepthStencilValue {
    /// Depth value
    pub depth: f32,
    /// Stencil value
    pub stencil: u32,
}

/// Rendering info
#[derive(Clone, Debug, Default)]
pub struct RenderingInfo {
    /// Render area
    pub render_area: RenderArea,
    /// Layer count
    pub layer_count: u32,
    /// View mask
    pub view_mask: u32,
    /// Color attachments
    pub color_attachments: Vec<RenderingAttachmentInfo>,
    /// Depth attachment
    pub depth_attachment: Option<RenderingAttachmentInfo>,
    /// Stencil attachment
    pub stencil_attachment: Option<RenderingAttachmentInfo>,
}

/// Render area
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RenderArea {
    /// X offset
    pub x: i32,
    /// Y offset
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl RenderArea {
    /// Creates new render area
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Creates from extent
    pub const fn from_extent(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}
