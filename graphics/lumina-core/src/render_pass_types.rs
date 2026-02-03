//! Render pass types and configuration
//!
//! This module provides comprehensive types for render pass creation and management.

use core::num::NonZeroU32;

use crate::attachment::{AttachmentDesc, ImageLayout, SubpassDependency, SubpassDesc};

/// Handle to a render pass
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RenderPassHandle(pub NonZeroU32);

impl RenderPassHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Render pass creation info
#[derive(Clone, Debug)]
pub struct RenderPassCreateInfo {
    /// Attachment descriptions
    pub attachments: alloc::vec::Vec<AttachmentDesc>,
    /// Subpass descriptions
    pub subpasses: alloc::vec::Vec<SubpassDesc>,
    /// Dependencies between subpasses
    pub dependencies: alloc::vec::Vec<SubpassDependency>,
    /// Flags
    pub flags: RenderPassCreateFlags,
}

use alloc::vec::Vec;

impl RenderPassCreateInfo {
    /// Creates a simple single-subpass render pass
    pub fn simple(color_attachment: AttachmentDesc) -> Self {
        Self {
            attachments: alloc::vec![color_attachment],
            subpasses: alloc::vec![SubpassDesc::simple(0)],
            dependencies: Vec::new(),
            flags: RenderPassCreateFlags::empty(),
        }
    }

    /// Creates a render pass with color and depth
    pub fn with_depth(color_attachment: AttachmentDesc, depth_attachment: AttachmentDesc) -> Self {
        let mut subpass = SubpassDesc::simple(0);
        subpass.depth_stencil_attachment = Some(crate::attachment::AttachmentReference::depth(1));

        Self {
            attachments: alloc::vec![color_attachment, depth_attachment],
            subpasses: alloc::vec![subpass],
            dependencies: Vec::new(),
            flags: RenderPassCreateFlags::empty(),
        }
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
}

bitflags::bitflags! {
    /// Render pass creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct RenderPassCreateFlags: u32 {
        /// Allow the render pass to be used with transform feedback
        const TRANSFORM_FEEDBACK_BIT = 1 << 0;
    }
}

impl RenderPassCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Framebuffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FramebufferHandle(pub NonZeroU32);

impl FramebufferHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Framebuffer creation info
#[derive(Clone, Debug)]
pub struct FramebufferCreateInfo {
    /// Associated render pass
    pub render_pass: RenderPassHandle,
    /// Image view attachments
    pub attachments: alloc::vec::Vec<ImageViewHandle>,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Layer count
    pub layers: u32,
    /// Flags
    pub flags: FramebufferCreateFlags,
}

impl FramebufferCreateInfo {
    /// Creates framebuffer info
    pub fn new(render_pass: RenderPassHandle, width: u32, height: u32) -> Self {
        Self {
            render_pass,
            attachments: Vec::new(),
            width,
            height,
            layers: 1,
            flags: FramebufferCreateFlags::empty(),
        }
    }

    /// Adds an attachment
    pub fn add_attachment(mut self, attachment: ImageViewHandle) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Sets layer count for layered rendering
    pub const fn with_layers(mut self, layers: u32) -> Self {
        self.layers = layers;
        self
    }

    /// Uses imageless framebuffer
    pub fn imageless(mut self) -> Self {
        self.flags |= FramebufferCreateFlags::IMAGELESS;
        self
    }
}

bitflags::bitflags! {
    /// Framebuffer creation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FramebufferCreateFlags: u32 {
        /// Framebuffer uses imageless attachments
        const IMAGELESS = 1 << 0;
    }
}

impl FramebufferCreateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Image view handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImageViewHandle(pub NonZeroU32);

impl ImageViewHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Render pass begin info
#[derive(Clone, Debug)]
pub struct RenderPassBeginInfo {
    /// Render pass handle
    pub render_pass: RenderPassHandle,
    /// Framebuffer handle
    pub framebuffer: FramebufferHandle,
    /// Render area
    pub render_area: RenderArea,
    /// Clear values for each attachment
    pub clear_values: alloc::vec::Vec<ClearValue>,
}

impl RenderPassBeginInfo {
    /// Creates begin info
    pub fn new(render_pass: RenderPassHandle, framebuffer: FramebufferHandle) -> Self {
        Self {
            render_pass,
            framebuffer,
            render_area: RenderArea::default(),
            clear_values: Vec::new(),
        }
    }

    /// Sets render area
    pub fn with_area(mut self, x: i32, y: i32, width: u32, height: u32) -> Self {
        self.render_area = RenderArea {
            offset: Offset2D { x, y },
            extent: Extent2D { width, height },
        };
        self
    }

    /// Adds a clear value
    pub fn add_clear_value(mut self, value: ClearValue) -> Self {
        self.clear_values.push(value);
        self
    }
}

/// 2D offset
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Offset2D {
    /// X offset
    pub x: i32,
    /// Y offset
    pub y: i32,
}

/// 2D extent
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Extent2D {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

/// Render area
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct RenderArea {
    /// Offset
    pub offset: Offset2D,
    /// Extent
    pub extent: Extent2D,
}

impl RenderArea {
    /// Creates from extent
    pub const fn from_extent(width: u32, height: u32) -> Self {
        Self {
            offset: Offset2D { x: 0, y: 0 },
            extent: Extent2D { width, height },
        }
    }
}

/// Clear value union
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum ClearValue {
    /// Color clear value
    Color(ClearColorValue),
    /// Depth-stencil clear value
    DepthStencil(ClearDepthStencilValue),
}

impl ClearValue {
    /// Black color
    pub const BLACK: Self = Self::Color(ClearColorValue::Float([0.0, 0.0, 0.0, 1.0]));
    /// White color
    pub const WHITE: Self = Self::Color(ClearColorValue::Float([1.0, 1.0, 1.0, 1.0]));
    /// Default depth/stencil
    pub const DEPTH_ONE: Self = Self::DepthStencil(ClearDepthStencilValue {
        depth: 1.0,
        stencil: 0,
    });
    /// Reversed Z depth
    pub const DEPTH_ZERO: Self = Self::DepthStencil(ClearDepthStencilValue {
        depth: 0.0,
        stencil: 0,
    });
}

/// Clear color value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum ClearColorValue {
    /// Float color
    Float([f32; 4]),
    /// Integer color
    Int([i32; 4]),
    /// Unsigned integer color
    Uint([u32; 4]),
}

/// Clear depth-stencil value
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ClearDepthStencilValue {
    /// Depth value (0.0 - 1.0)
    pub depth: f32,
    /// Stencil value
    pub stencil: u32,
}

/// Subpass contents flag
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SubpassContents {
    /// Commands are recorded inline
    #[default]
    Inline = 0,
    /// Commands come from secondary command buffers
    SecondaryCommandBuffers = 1,
    /// Commands can be either (VK 1.3+)
    InlineAndSecondary = 2,
}

/// Render pass attachment begin info (for imageless framebuffers)
#[derive(Clone, Debug)]
pub struct RenderPassAttachmentBeginInfo {
    /// Attachments to use
    pub attachments: alloc::vec::Vec<ImageViewHandle>,
}

/// Render pass multiview create info
#[derive(Clone, Debug)]
pub struct RenderPassMultiviewCreateInfo {
    /// View masks for each subpass
    pub view_masks: alloc::vec::Vec<u32>,
    /// View offsets for each dependency
    pub view_offsets: alloc::vec::Vec<i32>,
    /// Correlation masks
    pub correlation_masks: alloc::vec::Vec<u32>,
}

impl RenderPassMultiviewCreateInfo {
    /// Creates multiview info for stereo rendering (2 views)
    pub fn stereo() -> Self {
        Self {
            view_masks: alloc::vec![0b11], // Both eyes active
            view_offsets: Vec::new(),
            correlation_masks: alloc::vec![0b11], // Views are correlated
        }
    }

    /// Creates for specified view count
    pub fn for_views(view_count: u32) -> Self {
        let mask = (1u32 << view_count) - 1;
        Self {
            view_masks: alloc::vec![mask],
            view_offsets: Vec::new(),
            correlation_masks: alloc::vec![mask],
        }
    }
}

/// Render pass input attachment aspect create info
#[derive(Clone, Debug)]
pub struct InputAttachmentAspectReference {
    /// Subpass index
    pub subpass: u32,
    /// Input attachment index
    pub input_attachment_index: u32,
    /// Aspect mask
    pub aspect_mask: ImageAspectFlags,
}

bitflags::bitflags! {
    /// Image aspect flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ImageAspectFlags: u32 {
        /// Color aspect
        const COLOR = 1 << 0;
        /// Depth aspect
        const DEPTH = 1 << 1;
        /// Stencil aspect
        const STENCIL = 1 << 2;
        /// Metadata aspect
        const METADATA = 1 << 3;
        /// Plane 0
        const PLANE_0 = 1 << 4;
        /// Plane 1
        const PLANE_1 = 1 << 5;
        /// Plane 2
        const PLANE_2 = 1 << 6;
    }
}

impl ImageAspectFlags {
    /// Depth and stencil combined
    pub const DEPTH_STENCIL: Self =
        Self::from_bits_truncate(Self::DEPTH.bits() | Self::STENCIL.bits());
}

/// Dynamic rendering attachment info (VK_KHR_dynamic_rendering)
#[derive(Clone, Debug)]
pub struct RenderingAttachmentInfo {
    /// Image view
    pub image_view: ImageViewHandle,
    /// Image layout during rendering
    pub image_layout: ImageLayout,
    /// Resolve mode
    pub resolve_mode: ResolveMode,
    /// Resolve image view
    pub resolve_image_view: Option<ImageViewHandle>,
    /// Resolve image layout
    pub resolve_image_layout: ImageLayout,
    /// Load operation
    pub load_op: LoadOp,
    /// Store operation
    pub store_op: StoreOp,
    /// Clear value
    pub clear_value: ClearValue,
}

/// Load operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum LoadOp {
    /// Load existing contents
    #[default]
    Load = 0,
    /// Clear to a specific value
    Clear = 1,
    /// Contents are undefined
    DontCare = 2,
    /// No operation (VK 1.3+)
    None = 3,
}

/// Store operation
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum StoreOp {
    /// Store contents
    #[default]
    Store = 0,
    /// Contents may be discarded
    DontCare = 1,
    /// No operation (VK 1.3+)
    None = 2,
}

/// Resolve mode
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum ResolveMode {
    /// No resolve
    #[default]
    None = 0,
    /// Average samples
    Average = 1,
    /// Use sample 0
    SampleZero = 2,
    /// Minimum value
    Min = 4,
    /// Maximum value
    Max = 8,
}

/// Rendering info (for dynamic rendering)
#[derive(Clone, Debug)]
pub struct RenderingInfo {
    /// Flags
    pub flags: RenderingFlags,
    /// Render area
    pub render_area: RenderArea,
    /// Layer count
    pub layer_count: u32,
    /// View mask for multiview
    pub view_mask: u32,
    /// Color attachments
    pub color_attachments: alloc::vec::Vec<RenderingAttachmentInfo>,
    /// Depth attachment
    pub depth_attachment: Option<RenderingAttachmentInfo>,
    /// Stencil attachment
    pub stencil_attachment: Option<RenderingAttachmentInfo>,
}

impl RenderingInfo {
    /// Creates rendering info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            flags: RenderingFlags::empty(),
            render_area: RenderArea::from_extent(width, height),
            layer_count: 1,
            view_mask: 0,
            color_attachments: Vec::new(),
            depth_attachment: None,
            stencil_attachment: None,
        }
    }

    /// Adds a color attachment
    pub fn add_color_attachment(mut self, attachment: RenderingAttachmentInfo) -> Self {
        self.color_attachments.push(attachment);
        self
    }

    /// Sets depth attachment
    pub fn with_depth(mut self, attachment: RenderingAttachmentInfo) -> Self {
        self.depth_attachment = Some(attachment);
        self
    }

    /// Sets stencil attachment
    pub fn with_stencil(mut self, attachment: RenderingAttachmentInfo) -> Self {
        self.stencil_attachment = Some(attachment);
        self
    }

    /// Enables suspending
    pub fn suspending(mut self) -> Self {
        self.flags |= RenderingFlags::SUSPENDING;
        self
    }

    /// Enables resuming
    pub fn resuming(mut self) -> Self {
        self.flags |= RenderingFlags::RESUMING;
        self
    }
}

bitflags::bitflags! {
    /// Rendering flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct RenderingFlags: u32 {
        /// Rendering is incomplete and will be resumed
        const SUSPENDING = 1 << 0;
        /// Rendering resumes a previous pass
        const RESUMING = 1 << 1;
        /// Contents may be preserved
        const CONTENTS_SECONDARY_COMMAND_BUFFERS = 1 << 2;
    }
}

impl RenderingFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Fragment shading rate attachment info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FragmentShadingRateAttachmentInfo {
    /// Image view
    pub image_view: ImageViewHandle,
    /// Image layout
    pub image_layout: ImageLayout,
    /// Shading rate attachment texel size
    pub texel_size: Extent2D,
}

/// Tile properties for tile-based renderers
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TileProperties {
    /// Tile size
    pub tile_size: Extent2D,
    /// Maximum tile viewport coverage
    pub max_viewport_coverage: Extent2D,
    /// Whether optimal tile layout is supported
    pub optimal_layout_supported: bool,
}

/// Subpass shading max workgroup size
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SubpassShadingMaxWorkgroupSize {
    /// Maximum workgroup size
    pub max_workgroup_size: [u32; 2],
}
