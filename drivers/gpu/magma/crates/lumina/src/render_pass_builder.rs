//! Render Pass Builder Types for Lumina
//!
//! This module provides advanced render pass construction infrastructure
//! for declarative render pass definitions.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Render Pass Builder Handles
// ============================================================================

/// Render pass builder handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RenderPassBuilderHandle(pub u64);

impl RenderPassBuilderHandle {
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

impl Default for RenderPassBuilderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Subpass builder handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SubpassBuilderHandle(pub u64);

impl SubpassBuilderHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SubpassBuilderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Attachment Description
// ============================================================================

/// Attachment builder
#[derive(Clone, Debug)]
pub struct AttachmentBuilder {
    /// Index
    index: u32,
    /// Format
    format: AttachmentFormat,
    /// Sample count
    samples: SampleCount,
    /// Load operation
    load_op: LoadOp,
    /// Store operation
    store_op: StoreOp,
    /// Stencil load operation
    stencil_load_op: LoadOp,
    /// Stencil store operation
    stencil_store_op: StoreOp,
    /// Initial layout
    initial_layout: ImageLayout,
    /// Final layout
    final_layout: ImageLayout,
    /// Name
    name: String,
}

impl AttachmentBuilder {
    /// Creates new attachment builder
    pub fn new(index: u32) -> Self {
        Self {
            index,
            format: AttachmentFormat::Rgba8Srgb,
            samples: SampleCount::X1,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            stencil_load_op: LoadOp::DontCare,
            stencil_store_op: StoreOp::DontCare,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::ColorAttachmentOptimal,
            name: String::new(),
        }
    }

    /// With format
    pub fn format(mut self, format: AttachmentFormat) -> Self {
        self.format = format;
        self
    }

    /// With sample count
    pub fn samples(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        self
    }

    /// With load operation
    pub fn load_op(mut self, op: LoadOp) -> Self {
        self.load_op = op;
        self
    }

    /// With store operation
    pub fn store_op(mut self, op: StoreOp) -> Self {
        self.store_op = op;
        self
    }

    /// Clear and store
    pub fn clear_and_store(mut self) -> Self {
        self.load_op = LoadOp::Clear;
        self.store_op = StoreOp::Store;
        self
    }

    /// Load and store
    pub fn load_and_store(mut self) -> Self {
        self.load_op = LoadOp::Load;
        self.store_op = StoreOp::Store;
        self
    }

    /// Don't care (transient)
    pub fn dont_care(mut self) -> Self {
        self.load_op = LoadOp::DontCare;
        self.store_op = StoreOp::DontCare;
        self
    }

    /// With stencil ops
    pub fn stencil_ops(mut self, load: LoadOp, store: StoreOp) -> Self {
        self.stencil_load_op = load;
        self.stencil_store_op = store;
        self
    }

    /// With initial layout
    pub fn initial_layout(mut self, layout: ImageLayout) -> Self {
        self.initial_layout = layout;
        self
    }

    /// With final layout
    pub fn final_layout(mut self, layout: ImageLayout) -> Self {
        self.final_layout = layout;
        self
    }

    /// Color attachment preset
    pub fn color(index: u32) -> Self {
        Self::new(index)
            .format(AttachmentFormat::Rgba8Srgb)
            .clear_and_store()
            .initial_layout(ImageLayout::Undefined)
            .final_layout(ImageLayout::ColorAttachmentOptimal)
    }

    /// HDR color attachment
    pub fn color_hdr(index: u32) -> Self {
        Self::new(index)
            .format(AttachmentFormat::Rgba16Float)
            .clear_and_store()
            .initial_layout(ImageLayout::Undefined)
            .final_layout(ImageLayout::ColorAttachmentOptimal)
    }

    /// Depth attachment preset
    pub fn depth(index: u32) -> Self {
        Self::new(index)
            .format(AttachmentFormat::D32Float)
            .clear_and_store()
            .initial_layout(ImageLayout::Undefined)
            .final_layout(ImageLayout::DepthStencilAttachmentOptimal)
    }

    /// Depth-stencil attachment preset
    pub fn depth_stencil(index: u32) -> Self {
        Self::new(index)
            .format(AttachmentFormat::D24UnormS8Uint)
            .clear_and_store()
            .stencil_ops(LoadOp::Clear, StoreOp::Store)
            .initial_layout(ImageLayout::Undefined)
            .final_layout(ImageLayout::DepthStencilAttachmentOptimal)
    }

    /// Resolve attachment preset
    pub fn resolve(index: u32) -> Self {
        Self::new(index)
            .format(AttachmentFormat::Rgba8Srgb)
            .load_op(LoadOp::DontCare)
            .store_op(StoreOp::Store)
            .initial_layout(ImageLayout::Undefined)
            .final_layout(ImageLayout::PresentSrc)
    }

    /// Swapchain attachment preset
    pub fn swapchain(index: u32) -> Self {
        Self::new(index)
            .format(AttachmentFormat::Bgra8Srgb)
            .clear_and_store()
            .initial_layout(ImageLayout::Undefined)
            .final_layout(ImageLayout::PresentSrc)
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Build into description
    pub fn build(self) -> AttachmentDescription {
        AttachmentDescription {
            index: self.index,
            format: self.format,
            samples: self.samples,
            load_op: self.load_op,
            store_op: self.store_op,
            stencil_load_op: self.stencil_load_op,
            stencil_store_op: self.stencil_store_op,
            initial_layout: self.initial_layout,
            final_layout: self.final_layout,
            name: self.name,
        }
    }
}

impl Default for AttachmentBuilder {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Attachment description
#[derive(Clone, Debug)]
pub struct AttachmentDescription {
    /// Index
    pub index: u32,
    /// Format
    pub format: AttachmentFormat,
    /// Sample count
    pub samples: SampleCount,
    /// Load operation
    pub load_op: LoadOp,
    /// Store operation
    pub store_op: StoreOp,
    /// Stencil load operation
    pub stencil_load_op: LoadOp,
    /// Stencil store operation
    pub stencil_store_op: StoreOp,
    /// Initial layout
    pub initial_layout: ImageLayout,
    /// Final layout
    pub final_layout: ImageLayout,
    /// Name
    pub name: String,
}

impl Default for AttachmentDescription {
    fn default() -> Self {
        AttachmentBuilder::new(0).build()
    }
}

// ============================================================================
// Attachment Types
// ============================================================================

/// Attachment format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AttachmentFormat {
    /// R8 unorm
    R8Unorm = 0,
    /// RG8 unorm
    Rg8Unorm = 1,
    /// RGBA8 unorm
    Rgba8Unorm = 2,
    /// RGBA8 sRGB
    #[default]
    Rgba8Srgb = 3,
    /// BGRA8 unorm
    Bgra8Unorm = 4,
    /// BGRA8 sRGB
    Bgra8Srgb = 5,
    /// RGB10A2 unorm
    Rgb10a2Unorm = 6,
    /// R16 float
    R16Float = 7,
    /// RG16 float
    Rg16Float = 8,
    /// RGBA16 float
    Rgba16Float = 9,
    /// R32 float
    R32Float = 10,
    /// RG32 float
    Rg32Float = 11,
    /// RGBA32 float
    Rgba32Float = 12,
    /// R11G11B10 float
    R11g11b10Float = 13,
    /// D16 unorm
    D16Unorm = 20,
    /// D24 unorm
    D24Unorm = 21,
    /// D32 float
    D32Float = 22,
    /// D24 unorm S8 uint
    D24UnormS8Uint = 23,
    /// D32 float S8 uint
    D32FloatS8Uint = 24,
    /// S8 uint
    S8Uint = 25,
}

impl AttachmentFormat {
    /// Is depth format
    pub const fn is_depth(&self) -> bool {
        matches!(self, Self::D16Unorm | Self::D24Unorm | Self::D32Float |
                 Self::D24UnormS8Uint | Self::D32FloatS8Uint)
    }

    /// Is stencil format
    pub const fn is_stencil(&self) -> bool {
        matches!(self, Self::D24UnormS8Uint | Self::D32FloatS8Uint | Self::S8Uint)
    }

    /// Is depth-stencil format
    pub const fn is_depth_stencil(&self) -> bool {
        matches!(self, Self::D24UnormS8Uint | Self::D32FloatS8Uint)
    }

    /// Is HDR format
    pub const fn is_hdr(&self) -> bool {
        matches!(self, Self::R16Float | Self::Rg16Float | Self::Rgba16Float |
                 Self::R32Float | Self::Rg32Float | Self::Rgba32Float |
                 Self::R11g11b10Float)
    }

    /// Bits per pixel
    pub const fn bits_per_pixel(&self) -> u32 {
        match self {
            Self::R8Unorm => 8,
            Self::Rg8Unorm => 16,
            Self::Rgba8Unorm | Self::Rgba8Srgb | Self::Bgra8Unorm | Self::Bgra8Srgb |
            Self::Rgb10a2Unorm | Self::R11g11b10Float => 32,
            Self::R16Float => 16,
            Self::Rg16Float | Self::R32Float | Self::D32Float => 32,
            Self::Rgba16Float | Self::Rg32Float => 64,
            Self::Rgba32Float => 128,
            Self::D16Unorm => 16,
            Self::D24Unorm => 24,
            Self::D24UnormS8Uint | Self::D32FloatS8Uint => 32,
            Self::S8Uint => 8,
        }
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SampleCount {
    /// 1 sample
    #[default]
    X1 = 1,
    /// 2 samples
    X2 = 2,
    /// 4 samples
    X4 = 4,
    /// 8 samples
    X8 = 8,
    /// 16 samples
    X16 = 16,
    /// 32 samples
    X32 = 32,
    /// 64 samples
    X64 = 64,
}

impl SampleCount {
    /// Count value
    pub const fn count(&self) -> u32 {
        *self as u32
    }

    /// Is MSAA
    pub const fn is_msaa(&self) -> bool {
        !matches!(self, Self::X1)
    }
}

/// Load operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LoadOp {
    /// Load previous contents
    Load = 0,
    /// Clear to value
    #[default]
    Clear = 1,
    /// Don't care about contents
    DontCare = 2,
}

/// Store operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StoreOp {
    /// Store contents
    #[default]
    Store = 0,
    /// Don't care (can discard)
    DontCare = 1,
    /// Resolve (MSAA)
    Resolve = 2,
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined
    #[default]
    Undefined = 0,
    /// General
    General = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth-stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth-stencil read-only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read-only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer src optimal
    TransferSrcOptimal = 6,
    /// Transfer dst optimal
    TransferDstOptimal = 7,
    /// Preinitialized
    Preinitialized = 8,
    /// Present src
    PresentSrc = 9,
    /// Depth read-only stencil attachment
    DepthReadOnlyStencilAttachment = 10,
    /// Depth attachment stencil read-only
    DepthAttachmentStencilReadOnly = 11,
    /// Depth attachment optimal
    DepthAttachmentOptimal = 12,
    /// Depth read-only optimal
    DepthReadOnlyOptimal = 13,
    /// Stencil attachment optimal
    StencilAttachmentOptimal = 14,
    /// Stencil read-only optimal
    StencilReadOnlyOptimal = 15,
    /// Read-only optimal
    ReadOnlyOptimal = 16,
    /// Attachment optimal
    AttachmentOptimal = 17,
}

// ============================================================================
// Subpass Builder
// ============================================================================

/// Subpass builder
#[derive(Clone, Debug)]
pub struct SubpassBuilder {
    /// Index
    index: u32,
    /// Name
    name: String,
    /// Color attachments
    color_attachments: Vec<AttachmentReference>,
    /// Input attachments
    input_attachments: Vec<AttachmentReference>,
    /// Resolve attachments
    resolve_attachments: Vec<AttachmentReference>,
    /// Depth-stencil attachment
    depth_stencil_attachment: Option<AttachmentReference>,
    /// Preserve attachments
    preserve_attachments: Vec<u32>,
    /// Pipeline bind point
    bind_point: PipelineBindPoint,
}

impl SubpassBuilder {
    /// Creates new subpass builder
    pub fn new(index: u32) -> Self {
        Self {
            index,
            name: String::new(),
            color_attachments: Vec::new(),
            input_attachments: Vec::new(),
            resolve_attachments: Vec::new(),
            depth_stencil_attachment: None,
            preserve_attachments: Vec::new(),
            bind_point: PipelineBindPoint::Graphics,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add color attachment
    pub fn color_attachment(mut self, index: u32, layout: ImageLayout) -> Self {
        self.color_attachments.push(AttachmentReference {
            index,
            layout,
            aspect: ImageAspect::Color,
        });
        self
    }

    /// Add color attachment with default layout
    pub fn color(mut self, index: u32) -> Self {
        self.color_attachments.push(AttachmentReference {
            index,
            layout: ImageLayout::ColorAttachmentOptimal,
            aspect: ImageAspect::Color,
        });
        self
    }

    /// Add input attachment
    pub fn input_attachment(mut self, index: u32, layout: ImageLayout) -> Self {
        self.input_attachments.push(AttachmentReference {
            index,
            layout,
            aspect: ImageAspect::Color,
        });
        self
    }

    /// Add input attachment with default layout
    pub fn input(mut self, index: u32) -> Self {
        self.input_attachments.push(AttachmentReference {
            index,
            layout: ImageLayout::ShaderReadOnlyOptimal,
            aspect: ImageAspect::Color,
        });
        self
    }

    /// Add resolve attachment
    pub fn resolve_attachment(mut self, index: u32) -> Self {
        self.resolve_attachments.push(AttachmentReference {
            index,
            layout: ImageLayout::ColorAttachmentOptimal,
            aspect: ImageAspect::Color,
        });
        self
    }

    /// Set depth-stencil attachment
    pub fn depth_stencil(mut self, index: u32, layout: ImageLayout) -> Self {
        self.depth_stencil_attachment = Some(AttachmentReference {
            index,
            layout,
            aspect: ImageAspect::DepthStencil,
        });
        self
    }

    /// Set depth attachment
    pub fn depth(mut self, index: u32) -> Self {
        self.depth_stencil_attachment = Some(AttachmentReference {
            index,
            layout: ImageLayout::DepthStencilAttachmentOptimal,
            aspect: ImageAspect::Depth,
        });
        self
    }

    /// Preserve attachment
    pub fn preserve(mut self, index: u32) -> Self {
        self.preserve_attachments.push(index);
        self
    }

    /// Preserve attachments
    pub fn preserve_all(mut self, indices: impl IntoIterator<Item = u32>) -> Self {
        self.preserve_attachments.extend(indices);
        self
    }

    /// Graphics bind point
    pub fn graphics(mut self) -> Self {
        self.bind_point = PipelineBindPoint::Graphics;
        self
    }

    /// Build into description
    pub fn build(self) -> SubpassDescription {
        SubpassDescription {
            index: self.index,
            name: self.name,
            color_attachments: self.color_attachments,
            input_attachments: self.input_attachments,
            resolve_attachments: self.resolve_attachments,
            depth_stencil_attachment: self.depth_stencil_attachment,
            preserve_attachments: self.preserve_attachments,
            bind_point: self.bind_point,
        }
    }
}

impl Default for SubpassBuilder {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Subpass description
#[derive(Clone, Debug, Default)]
pub struct SubpassDescription {
    /// Index
    pub index: u32,
    /// Name
    pub name: String,
    /// Color attachments
    pub color_attachments: Vec<AttachmentReference>,
    /// Input attachments
    pub input_attachments: Vec<AttachmentReference>,
    /// Resolve attachments
    pub resolve_attachments: Vec<AttachmentReference>,
    /// Depth-stencil attachment
    pub depth_stencil_attachment: Option<AttachmentReference>,
    /// Preserve attachments
    pub preserve_attachments: Vec<u32>,
    /// Pipeline bind point
    pub bind_point: PipelineBindPoint,
}

/// Attachment reference
#[derive(Clone, Copy, Debug, Default)]
pub struct AttachmentReference {
    /// Attachment index
    pub index: u32,
    /// Image layout
    pub layout: ImageLayout,
    /// Aspect
    pub aspect: ImageAspect,
}

impl AttachmentReference {
    /// Creates new reference
    pub const fn new(index: u32, layout: ImageLayout) -> Self {
        Self {
            index,
            layout,
            aspect: ImageAspect::Color,
        }
    }

    /// Color attachment
    pub const fn color(index: u32) -> Self {
        Self {
            index,
            layout: ImageLayout::ColorAttachmentOptimal,
            aspect: ImageAspect::Color,
        }
    }

    /// Depth attachment
    pub const fn depth(index: u32) -> Self {
        Self {
            index,
            layout: ImageLayout::DepthStencilAttachmentOptimal,
            aspect: ImageAspect::Depth,
        }
    }

    /// Unused attachment
    pub const UNUSED: Self = Self {
        index: u32::MAX,
        layout: ImageLayout::Undefined,
        aspect: ImageAspect::Color,
    };
}

/// Image aspect
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageAspect {
    /// Color
    #[default]
    Color = 0,
    /// Depth
    Depth = 1,
    /// Stencil
    Stencil = 2,
    /// Depth and stencil
    DepthStencil = 3,
}

/// Pipeline bind point
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PipelineBindPoint {
    /// Graphics pipeline
    #[default]
    Graphics = 0,
    /// Compute pipeline
    Compute = 1,
    /// Ray tracing pipeline
    RayTracing = 2,
}

// ============================================================================
// Subpass Dependency Builder
// ============================================================================

/// Subpass dependency builder
#[derive(Clone, Debug)]
pub struct DependencyBuilder {
    /// Source subpass
    src_subpass: u32,
    /// Destination subpass
    dst_subpass: u32,
    /// Source stage mask
    src_stage_mask: PipelineStageFlags,
    /// Destination stage mask
    dst_stage_mask: PipelineStageFlags,
    /// Source access mask
    src_access_mask: AccessFlags,
    /// Destination access mask
    dst_access_mask: AccessFlags,
    /// Dependency flags
    flags: DependencyFlags,
}

impl DependencyBuilder {
    /// Creates new dependency
    pub fn new(src: u32, dst: u32) -> Self {
        Self {
            src_subpass: src,
            dst_subpass: dst,
            src_stage_mask: PipelineStageFlags::BOTTOM_OF_PIPE,
            dst_stage_mask: PipelineStageFlags::TOP_OF_PIPE,
            src_access_mask: AccessFlags::NONE,
            dst_access_mask: AccessFlags::NONE,
            flags: DependencyFlags::empty(),
        }
    }

    /// External to subpass 0
    pub fn external_to_first() -> Self {
        Self::new(u32::MAX, 0)
    }

    /// Last subpass to external
    pub fn last_to_external(last: u32) -> Self {
        Self::new(last, u32::MAX)
    }

    /// Between subpasses
    pub fn between(src: u32, dst: u32) -> Self {
        Self::new(src, dst)
    }

    /// With stage masks
    pub fn stages(mut self, src: PipelineStageFlags, dst: PipelineStageFlags) -> Self {
        self.src_stage_mask = src;
        self.dst_stage_mask = dst;
        self
    }

    /// With access masks
    pub fn access(mut self, src: AccessFlags, dst: AccessFlags) -> Self {
        self.src_access_mask = src;
        self.dst_access_mask = dst;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: DependencyFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// By region
    pub fn by_region(mut self) -> Self {
        self.flags |= DependencyFlags::BY_REGION;
        self
    }

    /// Color attachment dependency
    pub fn color_attachment(self) -> Self {
        self.stages(
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ).access(
            AccessFlags::COLOR_ATTACHMENT_WRITE,
            AccessFlags::COLOR_ATTACHMENT_READ | AccessFlags::COLOR_ATTACHMENT_WRITE,
        )
    }

    /// Depth attachment dependency
    pub fn depth_attachment(self) -> Self {
        self.stages(
            PipelineStageFlags::EARLY_FRAGMENT_TESTS | PipelineStageFlags::LATE_FRAGMENT_TESTS,
            PipelineStageFlags::EARLY_FRAGMENT_TESTS | PipelineStageFlags::LATE_FRAGMENT_TESTS,
        ).access(
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        )
    }

    /// Input attachment dependency
    pub fn input_attachment(self) -> Self {
        self.stages(
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::FRAGMENT_SHADER,
        ).access(
            AccessFlags::COLOR_ATTACHMENT_WRITE,
            AccessFlags::INPUT_ATTACHMENT_READ,
        )
    }

    /// Build into description
    pub fn build(self) -> SubpassDependency {
        SubpassDependency {
            src_subpass: self.src_subpass,
            dst_subpass: self.dst_subpass,
            src_stage_mask: self.src_stage_mask,
            dst_stage_mask: self.dst_stage_mask,
            src_access_mask: self.src_access_mask,
            dst_access_mask: self.dst_access_mask,
            flags: self.flags,
        }
    }
}

impl Default for DependencyBuilder {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Subpass dependency
#[derive(Clone, Copy, Debug, Default)]
pub struct SubpassDependency {
    /// Source subpass (u32::MAX = external)
    pub src_subpass: u32,
    /// Destination subpass (u32::MAX = external)
    pub dst_subpass: u32,
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags,
    /// Source access mask
    pub src_access_mask: AccessFlags,
    /// Destination access mask
    pub dst_access_mask: AccessFlags,
    /// Flags
    pub flags: DependencyFlags,
}

bitflags::bitflags! {
    /// Pipeline stage flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct PipelineStageFlags: u32 {
        /// None
        const NONE = 0;
        /// Top of pipe
        const TOP_OF_PIPE = 1 << 0;
        /// Draw indirect
        const DRAW_INDIRECT = 1 << 1;
        /// Vertex input
        const VERTEX_INPUT = 1 << 2;
        /// Vertex shader
        const VERTEX_SHADER = 1 << 3;
        /// Tessellation control shader
        const TESSELLATION_CONTROL_SHADER = 1 << 4;
        /// Tessellation evaluation shader
        const TESSELLATION_EVALUATION_SHADER = 1 << 5;
        /// Geometry shader
        const GEOMETRY_SHADER = 1 << 6;
        /// Fragment shader
        const FRAGMENT_SHADER = 1 << 7;
        /// Early fragment tests
        const EARLY_FRAGMENT_TESTS = 1 << 8;
        /// Late fragment tests
        const LATE_FRAGMENT_TESTS = 1 << 9;
        /// Color attachment output
        const COLOR_ATTACHMENT_OUTPUT = 1 << 10;
        /// Compute shader
        const COMPUTE_SHADER = 1 << 11;
        /// Transfer
        const TRANSFER = 1 << 12;
        /// Bottom of pipe
        const BOTTOM_OF_PIPE = 1 << 13;
        /// Host
        const HOST = 1 << 14;
        /// All graphics
        const ALL_GRAPHICS = 1 << 15;
        /// All commands
        const ALL_COMMANDS = 1 << 16;
    }
}

bitflags::bitflags! {
    /// Access flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct AccessFlags: u32 {
        /// None
        const NONE = 0;
        /// Indirect command read
        const INDIRECT_COMMAND_READ = 1 << 0;
        /// Index read
        const INDEX_READ = 1 << 1;
        /// Vertex attribute read
        const VERTEX_ATTRIBUTE_READ = 1 << 2;
        /// Uniform read
        const UNIFORM_READ = 1 << 3;
        /// Input attachment read
        const INPUT_ATTACHMENT_READ = 1 << 4;
        /// Shader read
        const SHADER_READ = 1 << 5;
        /// Shader write
        const SHADER_WRITE = 1 << 6;
        /// Color attachment read
        const COLOR_ATTACHMENT_READ = 1 << 7;
        /// Color attachment write
        const COLOR_ATTACHMENT_WRITE = 1 << 8;
        /// Depth stencil attachment read
        const DEPTH_STENCIL_ATTACHMENT_READ = 1 << 9;
        /// Depth stencil attachment write
        const DEPTH_STENCIL_ATTACHMENT_WRITE = 1 << 10;
        /// Transfer read
        const TRANSFER_READ = 1 << 11;
        /// Transfer write
        const TRANSFER_WRITE = 1 << 12;
        /// Host read
        const HOST_READ = 1 << 13;
        /// Host write
        const HOST_WRITE = 1 << 14;
        /// Memory read
        const MEMORY_READ = 1 << 15;
        /// Memory write
        const MEMORY_WRITE = 1 << 16;
    }
}

bitflags::bitflags! {
    /// Dependency flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct DependencyFlags: u32 {
        /// None
        const NONE = 0;
        /// By region
        const BY_REGION = 1 << 0;
        /// Device group
        const DEVICE_GROUP = 1 << 1;
        /// View local
        const VIEW_LOCAL = 1 << 2;
    }
}

// ============================================================================
// Render Pass Builder
// ============================================================================

/// Render pass builder
#[derive(Clone, Debug)]
pub struct RenderPassBuilder {
    /// Name
    name: String,
    /// Attachments
    attachments: Vec<AttachmentDescription>,
    /// Subpasses
    subpasses: Vec<SubpassDescription>,
    /// Dependencies
    dependencies: Vec<SubpassDependency>,
}

impl RenderPassBuilder {
    /// Creates new render pass builder
    pub fn new() -> Self {
        Self {
            name: String::new(),
            attachments: Vec::new(),
            subpasses: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add attachment
    pub fn attachment(mut self, attachment: AttachmentBuilder) -> Self {
        self.attachments.push(attachment.build());
        self
    }

    /// Add subpass
    pub fn subpass(mut self, subpass: SubpassBuilder) -> Self {
        self.subpasses.push(subpass.build());
        self
    }

    /// Add dependency
    pub fn dependency(mut self, dependency: DependencyBuilder) -> Self {
        self.dependencies.push(dependency.build());
        self
    }

    /// Add external dependency for first subpass
    pub fn with_external_dependency(mut self) -> Self {
        self.dependencies.push(
            DependencyBuilder::external_to_first()
                .color_attachment()
                .build()
        );
        self
    }

    /// Simple forward pass (color + depth)
    pub fn forward_pass() -> Self {
        Self::new()
            .with_name("Forward Pass")
            .attachment(AttachmentBuilder::color(0))
            .attachment(AttachmentBuilder::depth(1))
            .subpass(SubpassBuilder::new(0).color(0).depth(1))
            .with_external_dependency()
    }

    /// Deferred G-buffer pass
    pub fn gbuffer_pass() -> Self {
        Self::new()
            .with_name("G-Buffer Pass")
            .attachment(AttachmentBuilder::color_hdr(0).with_name("Albedo"))
            .attachment(AttachmentBuilder::color_hdr(1).with_name("Normal"))
            .attachment(AttachmentBuilder::color_hdr(2).with_name("Material"))
            .attachment(AttachmentBuilder::depth(3))
            .subpass(SubpassBuilder::new(0)
                .color(0).color(1).color(2).depth(3))
    }

    /// Build render pass description
    pub fn build(self) -> RenderPassCreateInfo {
        RenderPassCreateInfo {
            name: self.name,
            attachments: self.attachments,
            subpasses: self.subpasses,
            dependencies: self.dependencies,
        }
    }
}

impl Default for RenderPassBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Render pass create info
#[derive(Clone, Debug, Default)]
pub struct RenderPassCreateInfo {
    /// Name
    pub name: String,
    /// Attachments
    pub attachments: Vec<AttachmentDescription>,
    /// Subpasses
    pub subpasses: Vec<SubpassDescription>,
    /// Dependencies
    pub dependencies: Vec<SubpassDependency>,
}
