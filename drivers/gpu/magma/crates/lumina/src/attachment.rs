//! Attachment and render pass types
//!
//! This module provides types for render pass attachments and subpasses.

/// Attachment load operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum LoadOp {
    /// Load existing contents
    #[default]
    Load = 0,
    /// Clear to specified value
    Clear = 1,
    /// Don't care about contents
    DontCare = 2,
    /// No access (for unused attachments)
    None = 3,
}

/// Attachment store operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum StoreOp {
    /// Store contents
    #[default]
    Store = 0,
    /// Don't care about contents
    DontCare = 1,
    /// No access
    None = 2,
}

/// Attachment format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum AttachmentFormat {
    /// Undefined
    Undefined = 0,

    // 8-bit formats
    /// R8 Unorm
    R8Unorm = 9,
    /// R8 Snorm
    R8Snorm = 10,
    /// R8 Uint
    R8Uint = 13,
    /// R8 Sint
    R8Sint = 14,
    /// R8 Srgb
    R8Srgb = 15,

    // 16-bit formats
    /// R8G8 Unorm
    Rg8Unorm = 16,
    /// R8G8 Snorm
    Rg8Snorm = 17,
    /// R8G8 Uint
    Rg8Uint = 20,
    /// R8G8 Sint
    Rg8Sint = 21,
    /// R16 Unorm
    R16Unorm = 70,
    /// R16 Snorm
    R16Snorm = 71,
    /// R16 Uint
    R16Uint = 74,
    /// R16 Sint
    R16Sint = 75,
    /// R16 Sfloat
    R16Sfloat = 76,

    // 32-bit formats
    /// R8G8B8A8 Unorm
    Rgba8Unorm = 37,
    /// R8G8B8A8 Snorm
    Rgba8Snorm = 38,
    /// R8G8B8A8 Uint
    Rgba8Uint = 41,
    /// R8G8B8A8 Sint
    Rgba8Sint = 42,
    /// R8G8B8A8 Srgb
    Rgba8Srgb = 43,
    /// B8G8R8A8 Unorm
    Bgra8Unorm = 44,
    /// B8G8R8A8 Snorm
    Bgra8Snorm = 45,
    /// B8G8R8A8 Srgb
    Bgra8Srgb = 50,
    /// A2R10G10B10 Unorm
    A2r10g10b10Unorm = 58,
    /// A2B10G10R10 Unorm
    A2b10g10r10Unorm = 64,
    /// R16G16 Unorm
    Rg16Unorm = 77,
    /// R16G16 Snorm
    Rg16Snorm = 78,
    /// R16G16 Uint
    Rg16Uint = 81,
    /// R16G16 Sint
    Rg16Sint = 82,
    /// R16G16 Sfloat
    Rg16Sfloat = 83,
    /// R32 Uint
    R32Uint = 98,
    /// R32 Sint
    R32Sint = 99,
    /// R32 Sfloat
    R32Sfloat = 100,
    /// B10G11R11 Ufloat
    B10g11r11Ufloat = 122,
    /// E5B9G9R9 Ufloat
    E5b9g9r9Ufloat = 123,

    // 64-bit formats
    /// R16G16B16A16 Unorm
    Rgba16Unorm = 91,
    /// R16G16B16A16 Snorm
    Rgba16Snorm = 92,
    /// R16G16B16A16 Uint
    Rgba16Uint = 95,
    /// R16G16B16A16 Sint
    Rgba16Sint = 96,
    /// R16G16B16A16 Sfloat
    Rgba16Sfloat = 97,
    /// R32G32 Uint
    Rg32Uint = 101,
    /// R32G32 Sint
    Rg32Sint = 102,
    /// R32G32 Sfloat
    Rg32Sfloat = 103,

    // 128-bit formats
    /// R32G32B32A32 Uint
    Rgba32Uint = 107,
    /// R32G32B32A32 Sint
    Rgba32Sint = 108,
    /// R32G32B32A32 Sfloat
    Rgba32Sfloat = 109,

    // Depth formats
    /// D16 Unorm
    D16Unorm = 124,
    /// X8D24 Unorm
    X8D24Unorm = 125,
    /// D32 Sfloat
    D32Sfloat = 126,
    /// S8 Uint
    S8Uint = 127,
    /// D16 Unorm S8 Uint
    D16UnormS8Uint = 128,
    /// D24 Unorm S8 Uint
    D24UnormS8Uint = 129,
    /// D32 Sfloat S8 Uint
    D32SfloatS8Uint = 130,
}

impl Default for AttachmentFormat {
    fn default() -> Self {
        Self::Undefined
    }
}

impl AttachmentFormat {
    /// Is depth format
    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::D16Unorm
                | Self::X8D24Unorm
                | Self::D32Sfloat
                | Self::D16UnormS8Uint
                | Self::D24UnormS8Uint
                | Self::D32SfloatS8Uint
        )
    }

    /// Is stencil format
    pub const fn is_stencil(&self) -> bool {
        matches!(
            self,
            Self::S8Uint
                | Self::D16UnormS8Uint
                | Self::D24UnormS8Uint
                | Self::D32SfloatS8Uint
        )
    }

    /// Is depth-stencil format
    pub const fn is_depth_stencil(&self) -> bool {
        self.is_depth() || self.is_stencil()
    }

    /// Is color format
    pub const fn is_color(&self) -> bool {
        !self.is_depth_stencil() && !matches!(self, Self::Undefined)
    }

    /// Is sRGB format
    pub const fn is_srgb(&self) -> bool {
        matches!(self, Self::R8Srgb | Self::Rgba8Srgb | Self::Bgra8Srgb)
    }

    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::R8Unorm | Self::R8Snorm | Self::R8Uint | Self::R8Sint | Self::R8Srgb => 1,
            Self::Rg8Unorm
            | Self::Rg8Snorm
            | Self::Rg8Uint
            | Self::Rg8Sint
            | Self::R16Unorm
            | Self::R16Snorm
            | Self::R16Uint
            | Self::R16Sint
            | Self::R16Sfloat
            | Self::D16Unorm => 2,
            Self::D16UnormS8Uint => 3,
            Self::Rgba8Unorm
            | Self::Rgba8Snorm
            | Self::Rgba8Uint
            | Self::Rgba8Sint
            | Self::Rgba8Srgb
            | Self::Bgra8Unorm
            | Self::Bgra8Snorm
            | Self::Bgra8Srgb
            | Self::A2r10g10b10Unorm
            | Self::A2b10g10r10Unorm
            | Self::Rg16Unorm
            | Self::Rg16Snorm
            | Self::Rg16Uint
            | Self::Rg16Sint
            | Self::Rg16Sfloat
            | Self::R32Uint
            | Self::R32Sint
            | Self::R32Sfloat
            | Self::B10g11r11Ufloat
            | Self::E5b9g9r9Ufloat
            | Self::X8D24Unorm
            | Self::D32Sfloat
            | Self::D24UnormS8Uint => 4,
            Self::S8Uint => 1,
            Self::D32SfloatS8Uint => 5,
            Self::Rgba16Unorm
            | Self::Rgba16Snorm
            | Self::Rgba16Uint
            | Self::Rgba16Sint
            | Self::Rgba16Sfloat
            | Self::Rg32Uint
            | Self::Rg32Sint
            | Self::Rg32Sfloat => 8,
            Self::Rgba32Uint | Self::Rgba32Sint | Self::Rgba32Sfloat => 16,
        }
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum SampleCount {
    /// 1 sample
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

/// Attachment description
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AttachmentDesc {
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
}

impl Default for AttachmentDesc {
    fn default() -> Self {
        Self::color(AttachmentFormat::Rgba8Unorm)
    }
}

impl AttachmentDesc {
    /// Creates color attachment
    pub const fn color(format: AttachmentFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            stencil_load_op: LoadOp::DontCare,
            stencil_store_op: StoreOp::DontCare,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::ColorAttachment,
        }
    }

    /// Creates depth attachment
    pub const fn depth(format: AttachmentFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            stencil_load_op: LoadOp::DontCare,
            stencil_store_op: StoreOp::DontCare,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::DepthStencilAttachment,
        }
    }

    /// Creates depth-stencil attachment
    pub const fn depth_stencil(format: AttachmentFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            stencil_load_op: LoadOp::Clear,
            stencil_store_op: StoreOp::Store,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::DepthStencilAttachment,
        }
    }

    /// Creates present attachment
    pub const fn present(format: AttachmentFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            stencil_load_op: LoadOp::DontCare,
            stencil_store_op: StoreOp::DontCare,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::Present,
        }
    }

    /// Creates transient attachment
    pub const fn transient(format: AttachmentFormat) -> Self {
        Self {
            format,
            samples: SampleCount::S1,
            load_op: LoadOp::DontCare,
            store_op: StoreOp::DontCare,
            stencil_load_op: LoadOp::DontCare,
            stencil_store_op: StoreOp::DontCare,
            initial_layout: ImageLayout::Undefined,
            final_layout: ImageLayout::ColorAttachment,
        }
    }

    /// With sample count
    pub const fn with_samples(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        self
    }

    /// With load op
    pub const fn with_load_op(mut self, op: LoadOp) -> Self {
        self.load_op = op;
        self
    }

    /// With store op
    pub const fn with_store_op(mut self, op: StoreOp) -> Self {
        self.store_op = op;
        self
    }

    /// With layouts
    pub const fn with_layouts(mut self, initial: ImageLayout, final_: ImageLayout) -> Self {
        self.initial_layout = initial;
        self.final_layout = final_;
        self
    }
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined layout
    #[default]
    Undefined = 0,
    /// General layout (any access)
    General = 1,
    /// Color attachment
    ColorAttachment = 2,
    /// Depth-stencil attachment
    DepthStencilAttachment = 3,
    /// Depth-stencil read-only
    DepthStencilReadOnly = 4,
    /// Shader read-only
    ShaderReadOnly = 5,
    /// Transfer source
    TransferSrc = 6,
    /// Transfer destination
    TransferDst = 7,
    /// Preinitialized
    Preinitialized = 8,
    /// Present
    Present = 1000001002,
    /// Depth read-only stencil attachment
    DepthReadOnlyStencilAttachment = 1000117000,
    /// Depth attachment stencil read-only
    DepthAttachmentStencilReadOnly = 1000117001,
    /// Depth attachment
    DepthAttachment = 1000241000,
    /// Depth read-only
    DepthReadOnly = 1000241001,
    /// Stencil attachment
    StencilAttachment = 1000241002,
    /// Stencil read-only
    StencilReadOnly = 1000241003,
    /// Read-only
    ReadOnly = 1000314000,
    /// Attachment
    Attachment = 1000314001,
    /// Fragment density map
    FragmentDensityMap = 1000218000,
    /// Fragment shading rate attachment
    FragmentShadingRateAttachment = 1000164003,
}

impl ImageLayout {
    /// Is read-only layout
    pub const fn is_read_only(&self) -> bool {
        matches!(
            self,
            Self::ShaderReadOnly
                | Self::DepthStencilReadOnly
                | Self::DepthReadOnly
                | Self::StencilReadOnly
                | Self::ReadOnly
                | Self::TransferSrc
        )
    }
}

/// Attachment reference
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct AttachmentReference {
    /// Attachment index (u32::MAX for unused)
    pub attachment: u32,
    /// Layout during subpass
    pub layout: ImageLayout,
    /// Aspect mask
    pub aspect_mask: u32,
}

impl AttachmentReference {
    /// Unused attachment
    pub const UNUSED: Self = Self {
        attachment: u32::MAX,
        layout: ImageLayout::Undefined,
        aspect_mask: 0,
    };

    /// Creates color attachment reference
    pub const fn color(attachment: u32) -> Self {
        Self {
            attachment,
            layout: ImageLayout::ColorAttachment,
            aspect_mask: 1, // COLOR
        }
    }

    /// Creates depth attachment reference
    pub const fn depth(attachment: u32) -> Self {
        Self {
            attachment,
            layout: ImageLayout::DepthStencilAttachment,
            aspect_mask: 2, // DEPTH
        }
    }

    /// Creates depth-stencil attachment reference
    pub const fn depth_stencil(attachment: u32) -> Self {
        Self {
            attachment,
            layout: ImageLayout::DepthStencilAttachment,
            aspect_mask: 3, // DEPTH | STENCIL
        }
    }

    /// Creates input attachment reference
    pub const fn input(attachment: u32, aspect_mask: u32) -> Self {
        Self {
            attachment,
            layout: ImageLayout::ShaderReadOnly,
            aspect_mask,
        }
    }

    /// Is unused
    pub const fn is_unused(&self) -> bool {
        self.attachment == u32::MAX
    }
}

/// Subpass description
#[derive(Clone, Debug, Default)]
pub struct SubpassDesc {
    /// Input attachments
    pub input_attachments: [AttachmentReference; 8],
    /// Input attachment count
    pub input_count: u32,
    /// Color attachments
    pub color_attachments: [AttachmentReference; 8],
    /// Color attachment count
    pub color_count: u32,
    /// Resolve attachments
    pub resolve_attachments: [AttachmentReference; 8],
    /// Depth-stencil attachment
    pub depth_stencil_attachment: AttachmentReference,
    /// Preserve attachments
    pub preserve_attachments: [u32; 8],
    /// Preserve count
    pub preserve_count: u32,
}

impl SubpassDesc {
    /// Creates simple subpass with color and depth
    pub fn simple(color: u32, depth: Option<u32>) -> Self {
        let mut desc = Self::default();
        desc.color_attachments[0] = AttachmentReference::color(color);
        desc.color_count = 1;
        if let Some(d) = depth {
            desc.depth_stencil_attachment = AttachmentReference::depth(d);
        } else {
            desc.depth_stencil_attachment = AttachmentReference::UNUSED;
        }
        desc
    }

    /// Adds color attachment
    pub fn add_color(&mut self, attachment: u32) -> &mut Self {
        if (self.color_count as usize) < 8 {
            self.color_attachments[self.color_count as usize] = AttachmentReference::color(attachment);
            self.color_count += 1;
        }
        self
    }

    /// Sets depth attachment
    pub fn set_depth(&mut self, attachment: u32) -> &mut Self {
        self.depth_stencil_attachment = AttachmentReference::depth(attachment);
        self
    }

    /// Adds input attachment
    pub fn add_input(&mut self, attachment: u32, aspect_mask: u32) -> &mut Self {
        if (self.input_count as usize) < 8 {
            self.input_attachments[self.input_count as usize] = AttachmentReference::input(attachment, aspect_mask);
            self.input_count += 1;
        }
        self
    }
}

/// Subpass dependency
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SubpassDependency {
    /// Source subpass (u32::MAX for external)
    pub src_subpass: u32,
    /// Destination subpass (u32::MAX for external)
    pub dst_subpass: u32,
    /// Source stage mask
    pub src_stage_mask: u32,
    /// Destination stage mask
    pub dst_stage_mask: u32,
    /// Source access mask
    pub src_access_mask: u32,
    /// Destination access mask
    pub dst_access_mask: u32,
    /// Dependency flags
    pub dependency_flags: u32,
}

impl SubpassDependency {
    /// External subpass
    pub const EXTERNAL: u32 = u32::MAX;

    /// Creates external to first subpass dependency
    pub const fn external_to_first() -> Self {
        Self {
            src_subpass: Self::EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: 0x2000, // BOTTOM_OF_PIPE
            dst_stage_mask: 0x80,   // COLOR_ATTACHMENT_OUTPUT
            src_access_mask: 0,
            dst_access_mask: 0x100, // COLOR_ATTACHMENT_WRITE
            dependency_flags: 1,    // BY_REGION
        }
    }

    /// Creates last subpass to external dependency
    pub const fn last_to_external(last_subpass: u32) -> Self {
        Self {
            src_subpass: last_subpass,
            dst_subpass: Self::EXTERNAL,
            src_stage_mask: 0x80,   // COLOR_ATTACHMENT_OUTPUT
            dst_stage_mask: 0x2000, // BOTTOM_OF_PIPE
            src_access_mask: 0x100, // COLOR_ATTACHMENT_WRITE
            dst_access_mask: 0,
            dependency_flags: 1, // BY_REGION
        }
    }

    /// Creates subpass to subpass dependency
    pub const fn between(src: u32, dst: u32) -> Self {
        Self {
            src_subpass: src,
            dst_subpass: dst,
            src_stage_mask: 0x80,  // COLOR_ATTACHMENT_OUTPUT
            dst_stage_mask: 0x20,  // FRAGMENT_SHADER
            src_access_mask: 0x100, // COLOR_ATTACHMENT_WRITE
            dst_access_mask: 0x20,  // SHADER_READ
            dependency_flags: 1,   // BY_REGION
        }
    }
}
