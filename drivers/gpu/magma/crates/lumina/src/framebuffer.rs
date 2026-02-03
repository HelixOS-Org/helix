//! Framebuffer and render target types
//!
//! This module provides types for framebuffer management and render targets.

extern crate alloc;
use alloc::vec::Vec;

use crate::types::{TextureHandle, TextureViewHandle, TextureFormat};

/// Handle to a framebuffer
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FramebufferHandle(pub u64);

impl FramebufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Handle to a render pass
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RenderPassHandle(pub u64);

impl RenderPassHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Checks if this is a null handle
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Attachment load operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum AttachmentLoadOp {
    /// Load existing contents
    Load,
    /// Clear to a specified value
    #[default]
    Clear,
    /// Contents are undefined (don't care)
    DontCare,
}

/// Attachment store operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum AttachmentStoreOp {
    /// Store the results
    #[default]
    Store,
    /// Contents are undefined after (don't care)
    DontCare,
    /// No store (for transient attachments)
    None,
}

/// Sample count for multisampling
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SampleCount {
    /// 1 sample (no multisampling)
    #[default]
    S1 = 1,
    /// 2 samples
    S2 = 2,
    /// 4 samples
    S4 = 4,
    /// 8 samples
    S8 = 8,
    /// 16 samples
    S16 = 16,
    /// 32 samples
    S32 = 32,
    /// 64 samples
    S64 = 64,
}

impl SampleCount {
    /// Returns the sample count as u32
    pub const fn as_u32(&self) -> u32 {
        *self as u32
    }

    /// Checks if multisampled
    pub const fn is_multisampled(&self) -> bool {
        !matches!(self, Self::S1)
    }
}

/// Image layout for attachments
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum ImageLayout {
    /// Undefined layout
    #[default]
    Undefined,
    /// General layout (all operations)
    General,
    /// Optimal for color attachment
    ColorAttachmentOptimal,
    /// Optimal for depth/stencil attachment
    DepthStencilAttachmentOptimal,
    /// Optimal for depth/stencil read only
    DepthStencilReadOnlyOptimal,
    /// Optimal for shader read
    ShaderReadOnlyOptimal,
    /// Optimal for transfer source
    TransferSrcOptimal,
    /// Optimal for transfer destination
    TransferDstOptimal,
    /// Preinitialized
    Preinitialized,
    /// Optimal for presentation
    PresentSrcKhr,
    /// Shared present
    SharedPresentKhr,
    /// Depth read only, stencil attachment
    DepthReadOnlyStencilAttachmentOptimal,
    /// Depth attachment, stencil read only
    DepthAttachmentStencilReadOnlyOptimal,
    /// Depth attachment optimal
    DepthAttachmentOptimal,
    /// Depth read only optimal
    DepthReadOnlyOptimal,
    /// Stencil attachment optimal
    StencilAttachmentOptimal,
    /// Stencil read only optimal
    StencilReadOnlyOptimal,
    /// Read only optimal
    ReadOnlyOptimal,
    /// Attachment optimal
    AttachmentOptimal,
}

/// Attachment description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AttachmentDesc {
    /// Attachment format
    pub format: TextureFormat,
    /// Sample count
    pub samples: SampleCount,
    /// Load operation for color/depth
    pub load_op: AttachmentLoadOp,
    /// Store operation for color/depth
    pub store_op: AttachmentStoreOp,
    /// Load operation for stencil
    pub stencil_load_op: AttachmentLoadOp,
    /// Store operation for stencil
    pub stencil_store_op: AttachmentStoreOp,
    /// Initial layout
    pub initial_layout: ImageLayout,
    /// Final layout
    pub final_layout: ImageLayout,
}

impl AttachmentDesc {
    /// Creates a color attachment description
    pub const fn color(format: TextureFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: AttachmentLoadOp::Clear,
            store_op: AttachmentStoreOp::Store,
            stencil_load_op: AttachmentLoadOp::DontCare,
            stencil_store_op: AttachmentStoreOp::DontCare,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::ColorAttachmentOptimal,
        }
    }

    /// Creates a color attachment that loads existing content
    pub const fn color_load(format: TextureFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: AttachmentLoadOp::Load,
            store_op: AttachmentStoreOp::Store,
            stencil_load_op: AttachmentLoadOp::DontCare,
            stencil_store_op: AttachmentStoreOp::DontCare,
            initial_layout: ImageLayout::ColorAttachmentOptimal,
            final_layout: ImageLayout::ColorAttachmentOptimal,
        }
    }

    /// Creates a color attachment for presentation
    pub const fn color_present(format: TextureFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: AttachmentLoadOp::Clear,
            store_op: AttachmentStoreOp::Store,
            stencil_load_op: AttachmentLoadOp::DontCare,
            stencil_store_op: AttachmentStoreOp::DontCare,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::PresentSrcKhr,
        }
    }

    /// Creates a depth attachment
    pub const fn depth(format: TextureFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: AttachmentLoadOp::Clear,
            store_op: AttachmentStoreOp::Store,
            stencil_load_op: AttachmentLoadOp::DontCare,
            stencil_store_op: AttachmentStoreOp::DontCare,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::DepthStencilAttachmentOptimal,
        }
    }

    /// Creates a depth-stencil attachment
    pub const fn depth_stencil(format: TextureFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: AttachmentLoadOp::Clear,
            store_op: AttachmentStoreOp::Store,
            stencil_load_op: AttachmentLoadOp::Clear,
            stencil_store_op: AttachmentStoreOp::Store,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::DepthStencilAttachmentOptimal,
        }
    }

    /// Creates a transient attachment (no store needed)
    pub const fn transient(format: TextureFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: AttachmentLoadOp::Clear,
            store_op: AttachmentStoreOp::DontCare,
            stencil_load_op: AttachmentLoadOp::DontCare,
            stencil_store_op: AttachmentStoreOp::DontCare,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::ColorAttachmentOptimal,
        }
    }

    /// Sets sample count
    pub const fn with_samples(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        self
    }

    /// Sets load operation
    pub const fn with_load_op(mut self, load_op: AttachmentLoadOp) -> Self {
        self.load_op = load_op;
        self
    }

    /// Sets store operation
    pub const fn with_store_op(mut self, store_op: AttachmentStoreOp) -> Self {
        self.store_op = store_op;
        self
    }

    /// Sets initial layout
    pub const fn with_initial_layout(mut self, layout: ImageLayout) -> Self {
        self.initial_layout = layout;
        self
    }

    /// Sets final layout
    pub const fn with_final_layout(mut self, layout: ImageLayout) -> Self {
        self.final_layout = layout;
        self
    }
}

impl Default for AttachmentDesc {
    fn default() -> Self {
        Self::color(TextureFormat::Bgra8Unorm)
    }
}

/// Attachment reference
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AttachmentRef {
    /// Attachment index
    pub attachment: u32,
    /// Layout during subpass
    pub layout: ImageLayout,
}

impl AttachmentRef {
    /// Unused attachment
    pub const UNUSED: Self = Self {
        attachment: u32::MAX,
        layout: ImageLayout::Undefined,
    };

    /// Creates a color attachment reference
    pub const fn color(attachment: u32) -> Self {
        Self {
            attachment,
            layout: ImageLayout::ColorAttachmentOptimal,
        }
    }

    /// Creates a depth attachment reference
    pub const fn depth(attachment: u32) -> Self {
        Self {
            attachment,
            layout: ImageLayout::DepthStencilAttachmentOptimal,
        }
    }

    /// Creates an input attachment reference
    pub const fn input(attachment: u32) -> Self {
        Self {
            attachment,
            layout: ImageLayout::ShaderReadOnlyOptimal,
        }
    }

    /// Creates a resolve attachment reference
    pub const fn resolve(attachment: u32) -> Self {
        Self {
            attachment,
            layout: ImageLayout::ColorAttachmentOptimal,
        }
    }

    /// Checks if this is an unused attachment
    pub const fn is_unused(&self) -> bool {
        self.attachment == u32::MAX
    }
}

/// Subpass description
#[derive(Clone, Debug, Default)]
pub struct SubpassDesc {
    /// Input attachments
    pub input_attachments: Vec<AttachmentRef>,
    /// Color attachments
    pub color_attachments: Vec<AttachmentRef>,
    /// Resolve attachments (for MSAA)
    pub resolve_attachments: Vec<AttachmentRef>,
    /// Depth stencil attachment
    pub depth_stencil_attachment: Option<AttachmentRef>,
    /// Preserve attachments
    pub preserve_attachments: Vec<u32>,
}

impl SubpassDesc {
    /// Creates a new empty subpass
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a color attachment
    pub fn add_color_attachment(mut self, attachment: u32) -> Self {
        self.color_attachments.push(AttachmentRef::color(attachment));
        self
    }

    /// Adds a color attachment with resolve
    pub fn add_color_attachment_with_resolve(
        mut self,
        color: u32,
        resolve: u32,
    ) -> Self {
        self.color_attachments.push(AttachmentRef::color(color));
        self.resolve_attachments.push(AttachmentRef::resolve(resolve));
        self
    }

    /// Sets depth stencil attachment
    pub fn with_depth_stencil(mut self, attachment: u32) -> Self {
        self.depth_stencil_attachment = Some(AttachmentRef::depth(attachment));
        self
    }

    /// Adds an input attachment
    pub fn add_input_attachment(mut self, attachment: u32) -> Self {
        self.input_attachments.push(AttachmentRef::input(attachment));
        self
    }

    /// Adds a preserve attachment
    pub fn add_preserve_attachment(mut self, attachment: u32) -> Self {
        self.preserve_attachments.push(attachment);
        self
    }

    /// Simple subpass with one color attachment
    pub fn simple_color(color_attachment: u32) -> Self {
        Self::new().add_color_attachment(color_attachment)
    }

    /// Simple subpass with color and depth
    pub fn simple_color_depth(color_attachment: u32, depth_attachment: u32) -> Self {
        Self::new()
            .add_color_attachment(color_attachment)
            .with_depth_stencil(depth_attachment)
    }
}

/// Pipeline stage flags for dependencies
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineStageFlags(pub u32);

impl PipelineStageFlags {
    /// Top of pipe
    pub const TOP_OF_PIPE: Self = Self(1 << 0);
    /// Draw indirect
    pub const DRAW_INDIRECT: Self = Self(1 << 1);
    /// Vertex input
    pub const VERTEX_INPUT: Self = Self(1 << 2);
    /// Vertex shader
    pub const VERTEX_SHADER: Self = Self(1 << 3);
    /// Tessellation control
    pub const TESSELLATION_CONTROL_SHADER: Self = Self(1 << 4);
    /// Tessellation evaluation
    pub const TESSELLATION_EVALUATION_SHADER: Self = Self(1 << 5);
    /// Geometry shader
    pub const GEOMETRY_SHADER: Self = Self(1 << 6);
    /// Fragment shader
    pub const FRAGMENT_SHADER: Self = Self(1 << 7);
    /// Early fragment tests
    pub const EARLY_FRAGMENT_TESTS: Self = Self(1 << 8);
    /// Late fragment tests
    pub const LATE_FRAGMENT_TESTS: Self = Self(1 << 9);
    /// Color attachment output
    pub const COLOR_ATTACHMENT_OUTPUT: Self = Self(1 << 10);
    /// Compute shader
    pub const COMPUTE_SHADER: Self = Self(1 << 11);
    /// Transfer
    pub const TRANSFER: Self = Self(1 << 12);
    /// Bottom of pipe
    pub const BOTTOM_OF_PIPE: Self = Self(1 << 13);
    /// Host
    pub const HOST: Self = Self(1 << 14);
    /// All graphics
    pub const ALL_GRAPHICS: Self = Self(1 << 15);
    /// All commands
    pub const ALL_COMMANDS: Self = Self(1 << 16);
    /// Ray tracing shader
    pub const RAY_TRACING_SHADER: Self = Self(1 << 21);
    /// Acceleration structure build
    pub const ACCELERATION_STRUCTURE_BUILD: Self = Self(1 << 25);
}

impl core::ops::BitOr for PipelineStageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for PipelineStageFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Access flags for dependencies
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AccessFlags(pub u32);

impl AccessFlags {
    /// No access
    pub const NONE: Self = Self(0);
    /// Indirect command read
    pub const INDIRECT_COMMAND_READ: Self = Self(1 << 0);
    /// Index read
    pub const INDEX_READ: Self = Self(1 << 1);
    /// Vertex attribute read
    pub const VERTEX_ATTRIBUTE_READ: Self = Self(1 << 2);
    /// Uniform read
    pub const UNIFORM_READ: Self = Self(1 << 3);
    /// Input attachment read
    pub const INPUT_ATTACHMENT_READ: Self = Self(1 << 4);
    /// Shader read
    pub const SHADER_READ: Self = Self(1 << 5);
    /// Shader write
    pub const SHADER_WRITE: Self = Self(1 << 6);
    /// Color attachment read
    pub const COLOR_ATTACHMENT_READ: Self = Self(1 << 7);
    /// Color attachment write
    pub const COLOR_ATTACHMENT_WRITE: Self = Self(1 << 8);
    /// Depth stencil attachment read
    pub const DEPTH_STENCIL_ATTACHMENT_READ: Self = Self(1 << 9);
    /// Depth stencil attachment write
    pub const DEPTH_STENCIL_ATTACHMENT_WRITE: Self = Self(1 << 10);
    /// Transfer read
    pub const TRANSFER_READ: Self = Self(1 << 11);
    /// Transfer write
    pub const TRANSFER_WRITE: Self = Self(1 << 12);
    /// Host read
    pub const HOST_READ: Self = Self(1 << 13);
    /// Host write
    pub const HOST_WRITE: Self = Self(1 << 14);
    /// Memory read
    pub const MEMORY_READ: Self = Self(1 << 15);
    /// Memory write
    pub const MEMORY_WRITE: Self = Self(1 << 16);
    /// Acceleration structure read
    pub const ACCELERATION_STRUCTURE_READ: Self = Self(1 << 21);
    /// Acceleration structure write
    pub const ACCELERATION_STRUCTURE_WRITE: Self = Self(1 << 22);
}

impl core::ops::BitOr for AccessFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for AccessFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Dependency flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DependencyFlags(pub u32);

impl DependencyFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// By region (framebuffer-local)
    pub const BY_REGION: Self = Self(1 << 0);
    /// View local
    pub const VIEW_LOCAL: Self = Self(1 << 1);
    /// Device group
    pub const DEVICE_GROUP: Self = Self(1 << 2);
}

impl core::ops::BitOr for DependencyFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Subpass dependency
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SubpassDependency {
    /// Source subpass (EXTERNAL = before render pass)
    pub src_subpass: u32,
    /// Destination subpass (EXTERNAL = after render pass)
    pub dst_subpass: u32,
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags,
    /// Source access mask
    pub src_access_mask: AccessFlags,
    /// Destination access mask
    pub dst_access_mask: AccessFlags,
    /// Dependency flags
    pub dependency_flags: DependencyFlags,
}

impl SubpassDependency {
    /// External subpass index
    pub const EXTERNAL: u32 = u32::MAX;

    /// Creates a dependency from external to first subpass
    pub const fn external_to_first() -> Self {
        Self {
            src_subpass: Self::EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: PipelineStageFlags::BOTTOM_OF_PIPE,
            dst_stage_mask: PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: AccessFlags::MEMORY_READ,
            dst_access_mask: AccessFlags::COLOR_ATTACHMENT_WRITE,
            dependency_flags: DependencyFlags::BY_REGION,
        }
    }

    /// Creates a dependency from last subpass to external
    pub const fn last_to_external(last_subpass: u32) -> Self {
        Self {
            src_subpass: last_subpass,
            dst_subpass: Self::EXTERNAL,
            src_stage_mask: PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: PipelineStageFlags::BOTTOM_OF_PIPE,
            src_access_mask: AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_access_mask: AccessFlags::MEMORY_READ,
            dependency_flags: DependencyFlags::BY_REGION,
        }
    }

    /// Creates a dependency between subpasses
    pub const fn between_subpasses(src: u32, dst: u32) -> Self {
        Self {
            src_subpass: src,
            dst_subpass: dst,
            src_stage_mask: PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: PipelineStageFlags::FRAGMENT_SHADER,
            src_access_mask: AccessFlags::COLOR_ATTACHMENT_WRITE,
            dst_access_mask: AccessFlags::INPUT_ATTACHMENT_READ,
            dependency_flags: DependencyFlags::BY_REGION,
        }
    }
}

/// Render pass description
#[derive(Clone, Debug, Default)]
pub struct RenderPassDesc {
    /// Attachment descriptions
    pub attachments: Vec<AttachmentDesc>,
    /// Subpass descriptions
    pub subpasses: Vec<SubpassDesc>,
    /// Subpass dependencies
    pub dependencies: Vec<SubpassDependency>,
}

impl RenderPassDesc {
    /// Creates a new empty render pass description
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an attachment
    pub fn add_attachment(mut self, attachment: AttachmentDesc) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Adds a subpass
    pub fn add_subpass(mut self, subpass: SubpassDesc) -> Self {
        self.subpasses.push(subpass);
        self
    }

    /// Adds a dependency
    pub fn add_dependency(mut self, dependency: SubpassDependency) -> Self {
        self.dependencies.push(dependency);
        self
    }

    /// Creates a simple render pass with one color attachment
    pub fn simple_color(format: TextureFormat) -> Self {
        Self::new()
            .add_attachment(AttachmentDesc::color_present(format))
            .add_subpass(SubpassDesc::simple_color(0))
    }

    /// Creates a render pass with color and depth
    pub fn color_depth(color_format: TextureFormat, depth_format: TextureFormat) -> Self {
        Self::new()
            .add_attachment(AttachmentDesc::color_present(color_format))
            .add_attachment(AttachmentDesc::depth(depth_format))
            .add_subpass(SubpassDesc::simple_color_depth(0, 1))
    }

    /// Creates a render pass with MSAA
    pub fn msaa_color_depth(
        color_format: TextureFormat,
        depth_format: TextureFormat,
        samples: SampleCount,
    ) -> Self {
        Self::new()
            // MSAA color attachment
            .add_attachment(
                AttachmentDesc::color(color_format)
                    .with_samples(samples)
                    .with_store_op(AttachmentStoreOp::DontCare),
            )
            // Resolve attachment
            .add_attachment(AttachmentDesc::color_present(color_format))
            // Depth attachment
            .add_attachment(AttachmentDesc::depth(depth_format).with_samples(samples))
            .add_subpass(
                SubpassDesc::new()
                    .add_color_attachment_with_resolve(0, 1)
                    .with_depth_stencil(2),
            )
    }
}

/// Framebuffer description
#[derive(Clone, Debug)]
pub struct FramebufferDesc {
    /// Render pass this framebuffer is compatible with
    pub render_pass: RenderPassHandle,
    /// Attachment image views
    pub attachments: Vec<TextureViewHandle>,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Number of layers
    pub layers: u32,
}

impl FramebufferDesc {
    /// Creates a new framebuffer description
    pub fn new(render_pass: RenderPassHandle, width: u32, height: u32) -> Self {
        Self {
            render_pass,
            attachments: Vec::new(),
            width,
            height,
            layers: 1,
        }
    }

    /// Adds an attachment
    pub fn add_attachment(mut self, view: TextureViewHandle) -> Self {
        self.attachments.push(view);
        self
    }

    /// Sets the number of layers
    pub fn with_layers(mut self, layers: u32) -> Self {
        self.layers = layers;
        self
    }
}

/// Clear color value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub union ClearColorValue {
    /// Float values
    pub float32: [f32; 4],
    /// Signed integer values
    pub int32: [i32; 4],
    /// Unsigned integer values
    pub uint32: [u32; 4],
}

impl Default for ClearColorValue {
    fn default() -> Self {
        Self {
            float32: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

impl ClearColorValue {
    /// Black
    pub const BLACK: Self = Self {
        float32: [0.0, 0.0, 0.0, 1.0],
    };

    /// White
    pub const WHITE: Self = Self {
        float32: [1.0, 1.0, 1.0, 1.0],
    };

    /// Transparent
    pub const TRANSPARENT: Self = Self {
        float32: [0.0, 0.0, 0.0, 0.0],
    };

    /// Creates a float clear color
    pub const fn float(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            float32: [r, g, b, a],
        }
    }

    /// Creates an integer clear color
    pub const fn int(r: i32, g: i32, b: i32, a: i32) -> Self {
        Self {
            int32: [r, g, b, a],
        }
    }

    /// Creates an unsigned integer clear color
    pub const fn uint(r: u32, g: u32, b: u32, a: u32) -> Self {
        Self {
            uint32: [r, g, b, a],
        }
    }
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

impl ClearDepthStencilValue {
    /// Default clear (depth = 1.0, stencil = 0)
    pub const DEFAULT: Self = Self {
        depth: 1.0,
        stencil: 0,
    };

    /// Reversed depth clear (depth = 0.0)
    pub const REVERSED: Self = Self {
        depth: 0.0,
        stencil: 0,
    };

    /// Creates a new clear value
    pub const fn new(depth: f32, stencil: u32) -> Self {
        Self { depth, stencil }
    }
}

/// Clear value for an attachment
#[derive(Clone, Copy)]
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
            color: ClearColorValue::BLACK,
        }
    }
}

impl ClearValue {
    /// Creates a color clear value
    pub const fn color(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: ClearColorValue::float(r, g, b, a),
        }
    }

    /// Creates a depth stencil clear value
    pub const fn depth_stencil(depth: f32, stencil: u32) -> Self {
        Self {
            depth_stencil: ClearDepthStencilValue::new(depth, stencil),
        }
    }
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
    /// Creates a render area from dimensions
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    /// Creates a render area with offset
    pub const fn with_offset(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Render pass begin info
#[derive(Clone, Debug)]
pub struct RenderPassBeginInfo {
    /// Render pass
    pub render_pass: RenderPassHandle,
    /// Framebuffer
    pub framebuffer: FramebufferHandle,
    /// Render area
    pub render_area: RenderArea,
    /// Clear values for each attachment
    pub clear_values: Vec<ClearValue>,
}

impl RenderPassBeginInfo {
    /// Creates a new render pass begin info
    pub fn new(
        render_pass: RenderPassHandle,
        framebuffer: FramebufferHandle,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            render_pass,
            framebuffer,
            render_area: RenderArea::new(width, height),
            clear_values: Vec::new(),
        }
    }

    /// Adds a clear value
    pub fn add_clear_value(mut self, value: ClearValue) -> Self {
        self.clear_values.push(value);
        self
    }

    /// Sets the render area
    pub fn with_render_area(mut self, area: RenderArea) -> Self {
        self.render_area = area;
        self
    }
}

/// Subpass contents
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SubpassContents {
    /// Commands are recorded inline
    #[default]
    Inline,
    /// Commands are in secondary command buffers
    SecondaryCommandBuffers,
}
