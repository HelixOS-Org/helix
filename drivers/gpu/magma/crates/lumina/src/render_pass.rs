//! Render pass configuration
//!
//! This module provides types for configuring render passes,
//! including color and depth attachments, load/store operations,
//! and clear values.

use crate::types::{TextureHandle, FramebufferHandle};
use crate::color::Color;

/// Configuration for a render pass
#[derive(Clone, Debug)]
pub struct RenderPassConfig<'a> {
    /// Label for debugging
    pub label: Option<&'a str>,
    /// Color attachments
    pub color_attachments: &'a [ColorAttachment],
    /// Depth/stencil attachment
    pub depth_stencil: Option<DepthStencilAttachment>,
    /// Render area (None = full framebuffer)
    pub render_area: Option<RenderArea>,
}

impl<'a> RenderPassConfig<'a> {
    /// Creates a simple render pass with one color attachment
    pub const fn single_color(attachment: &'a [ColorAttachment]) -> Self {
        Self {
            label: None,
            color_attachments: attachment,
            depth_stencil: None,
            render_area: None,
        }
    }

    /// Sets the render pass label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the depth/stencil attachment
    pub const fn with_depth_stencil(mut self, attachment: DepthStencilAttachment) -> Self {
        self.depth_stencil = Some(attachment);
        self
    }

    /// Sets the render area
    pub const fn with_render_area(mut self, area: RenderArea) -> Self {
        self.render_area = Some(area);
        self
    }
}

/// A color attachment in a render pass
#[derive(Clone, Debug)]
pub struct ColorAttachment {
    /// Target texture
    pub target: AttachmentTarget,
    /// Resolve target for MSAA
    pub resolve_target: Option<TextureHandle>,
    /// Load operation
    pub load_op: LoadOp<Color>,
    /// Store operation
    pub store_op: StoreOp,
}

impl ColorAttachment {
    /// Creates a color attachment that clears to a color
    pub fn clear(target: TextureHandle, color: Color) -> Self {
        Self {
            target: AttachmentTarget::Texture(target),
            resolve_target: None,
            load_op: LoadOp::Clear(color),
            store_op: StoreOp::Store,
        }
    }

    /// Creates a color attachment that loads existing content
    pub fn load(target: TextureHandle) -> Self {
        Self {
            target: AttachmentTarget::Texture(target),
            resolve_target: None,
            load_op: LoadOp::Load,
            store_op: StoreOp::Store,
        }
    }

    /// Creates a color attachment that doesn't care about initial content
    pub fn dont_care(target: TextureHandle) -> Self {
        Self {
            target: AttachmentTarget::Texture(target),
            resolve_target: None,
            load_op: LoadOp::DontCare,
            store_op: StoreOp::Store,
        }
    }

    /// Sets an MSAA resolve target
    pub fn with_resolve(mut self, target: TextureHandle) -> Self {
        self.resolve_target = Some(target);
        self
    }

    /// Uses the swapchain as the render target
    pub fn swapchain(clear_color: Color) -> Self {
        Self {
            target: AttachmentTarget::Swapchain,
            resolve_target: None,
            load_op: LoadOp::Clear(clear_color),
            store_op: StoreOp::Store,
        }
    }
}

/// Target for a render attachment
#[derive(Clone, Debug)]
pub enum AttachmentTarget {
    /// A specific texture
    Texture(TextureHandle),
    /// The current swapchain image
    Swapchain,
}

/// Depth/stencil attachment configuration
#[derive(Clone, Debug)]
pub struct DepthStencilAttachment {
    /// Target texture
    pub target: TextureHandle,
    /// Depth load operation
    pub depth_load_op: LoadOp<f32>,
    /// Depth store operation
    pub depth_store_op: StoreOp,
    /// Is depth read-only?
    pub depth_read_only: bool,
    /// Stencil load operation
    pub stencil_load_op: LoadOp<u32>,
    /// Stencil store operation
    pub stencil_store_op: StoreOp,
    /// Is stencil read-only?
    pub stencil_read_only: bool,
}

impl DepthStencilAttachment {
    /// Creates a depth attachment that clears to a value
    pub fn clear(target: TextureHandle, depth: f32) -> Self {
        Self {
            target,
            depth_load_op: LoadOp::Clear(depth),
            depth_store_op: StoreOp::Store,
            depth_read_only: false,
            stencil_load_op: LoadOp::Clear(0),
            stencil_store_op: StoreOp::DontCare,
            stencil_read_only: true,
        }
    }

    /// Creates a depth attachment that loads existing content
    pub fn load(target: TextureHandle) -> Self {
        Self {
            target,
            depth_load_op: LoadOp::Load,
            depth_store_op: StoreOp::Store,
            depth_read_only: false,
            stencil_load_op: LoadOp::Load,
            stencil_store_op: StoreOp::Store,
            stencil_read_only: false,
        }
    }

    /// Creates a read-only depth attachment for sampling
    pub fn read_only(target: TextureHandle) -> Self {
        Self {
            target,
            depth_load_op: LoadOp::Load,
            depth_store_op: StoreOp::DontCare,
            depth_read_only: true,
            stencil_load_op: LoadOp::Load,
            stencil_store_op: StoreOp::DontCare,
            stencil_read_only: true,
        }
    }

    /// Marks depth as read-only
    pub fn with_depth_read_only(mut self) -> Self {
        self.depth_read_only = true;
        self.depth_store_op = StoreOp::DontCare;
        self
    }
}

/// Load operation for an attachment
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LoadOp<T> {
    /// Load existing content
    Load,
    /// Clear to a specific value
    Clear(T),
    /// Content is undefined (don't care)
    DontCare,
}

impl<T: Default> Default for LoadOp<T> {
    fn default() -> Self {
        Self::DontCare
    }
}

/// Store operation for an attachment
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreOp {
    /// Store the result
    Store,
    /// Discard the result
    DontCare,
}

impl Default for StoreOp {
    fn default() -> Self {
        Self::Store
    }
}

/// Render area for a render pass
#[derive(Clone, Copy, Debug)]
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
    /// Creates a new render area
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Creates a render area from origin with given size
    pub const fn from_size(width: u32, height: u32) -> Self {
        Self::new(0, 0, width, height)
    }
}

/// Framebuffer configuration
#[derive(Clone, Debug)]
pub struct FramebufferConfig<'a> {
    /// Label for debugging
    pub label: Option<&'a str>,
    /// Color attachments
    pub color_attachments: &'a [TextureHandle],
    /// Depth/stencil attachment
    pub depth_stencil: Option<TextureHandle>,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Number of layers (for layered rendering)
    pub layers: u32,
}

impl<'a> FramebufferConfig<'a> {
    /// Creates a framebuffer with a single color attachment
    pub const fn single(color: &'a [TextureHandle], width: u32, height: u32) -> Self {
        Self {
            label: None,
            color_attachments: color,
            depth_stencil: None,
            width,
            height,
            layers: 1,
        }
    }

    /// Adds a depth/stencil attachment
    pub const fn with_depth_stencil(mut self, depth: TextureHandle) -> Self {
        self.depth_stencil = Some(depth);
        self
    }

    /// Sets the label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

/// Subpass dependency for synchronization
#[derive(Clone, Copy, Debug)]
pub struct SubpassDependency {
    /// Source subpass (or EXTERNAL)
    pub src_subpass: SubpassRef,
    /// Destination subpass (or EXTERNAL)
    pub dst_subpass: SubpassRef,
    /// Source stage mask
    pub src_stage_mask: PipelineStage,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStage,
    /// Source access mask
    pub src_access_mask: AccessFlags,
    /// Destination access mask
    pub dst_access_mask: AccessFlags,
}

/// Reference to a subpass
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubpassRef {
    /// External (before or after render pass)
    External,
    /// Subpass by index
    Index(u32),
}

/// Pipeline stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct PipelineStage(pub u32);

impl PipelineStage {
    /// Top of pipe
    pub const TOP_OF_PIPE: Self = Self(0x0001);
    /// Draw indirect commands
    pub const DRAW_INDIRECT: Self = Self(0x0002);
    /// Vertex input
    pub const VERTEX_INPUT: Self = Self(0x0004);
    /// Vertex shader
    pub const VERTEX_SHADER: Self = Self(0x0008);
    /// Fragment shader
    pub const FRAGMENT_SHADER: Self = Self(0x0080);
    /// Early fragment tests
    pub const EARLY_FRAGMENT_TESTS: Self = Self(0x0100);
    /// Late fragment tests
    pub const LATE_FRAGMENT_TESTS: Self = Self(0x0200);
    /// Color attachment output
    pub const COLOR_ATTACHMENT_OUTPUT: Self = Self(0x0400);
    /// Compute shader
    pub const COMPUTE_SHADER: Self = Self(0x0800);
    /// Transfer operations
    pub const TRANSFER: Self = Self(0x1000);
    /// Bottom of pipe
    pub const BOTTOM_OF_PIPE: Self = Self(0x2000);
    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(0x8000);
    /// All commands
    pub const ALL_COMMANDS: Self = Self(0x10000);
}

impl core::ops::BitOr for PipelineStage {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Access flags for synchronization
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct AccessFlags(pub u32);

impl AccessFlags {
    /// No access
    pub const NONE: Self = Self(0);
    /// Indirect command read
    pub const INDIRECT_COMMAND_READ: Self = Self(0x0001);
    /// Index read
    pub const INDEX_READ: Self = Self(0x0002);
    /// Vertex attribute read
    pub const VERTEX_ATTRIBUTE_READ: Self = Self(0x0004);
    /// Uniform read
    pub const UNIFORM_READ: Self = Self(0x0008);
    /// Input attachment read
    pub const INPUT_ATTACHMENT_READ: Self = Self(0x0010);
    /// Shader read
    pub const SHADER_READ: Self = Self(0x0020);
    /// Shader write
    pub const SHADER_WRITE: Self = Self(0x0040);
    /// Color attachment read
    pub const COLOR_ATTACHMENT_READ: Self = Self(0x0080);
    /// Color attachment write
    pub const COLOR_ATTACHMENT_WRITE: Self = Self(0x0100);
    /// Depth/stencil attachment read
    pub const DEPTH_STENCIL_ATTACHMENT_READ: Self = Self(0x0200);
    /// Depth/stencil attachment write
    pub const DEPTH_STENCIL_ATTACHMENT_WRITE: Self = Self(0x0400);
    /// Transfer read
    pub const TRANSFER_READ: Self = Self(0x0800);
    /// Transfer write
    pub const TRANSFER_WRITE: Self = Self(0x1000);
    /// Memory read
    pub const MEMORY_READ: Self = Self(0x8000);
    /// Memory write
    pub const MEMORY_WRITE: Self = Self(0x10000);
}

impl core::ops::BitOr for AccessFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
