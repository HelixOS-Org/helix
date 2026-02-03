//! Render graph types for frame organization
//!
//! This module provides types for building a declarative render graph
//! that automatically handles resource transitions and synchronization.

use crate::barrier::{AccessFlags, ImageLayout, PipelineStageFlags};

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

/// Render pass handle in the graph
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RenderPassId(pub u32);

impl RenderPassId {
    /// Invalid pass ID
    pub const INVALID: Self = Self(!0);

    /// Creates a new pass ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Checks if valid
    pub const fn is_valid(&self) -> bool {
        self.0 != !0
    }
}

/// Resource handle in the graph
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ResourceId(pub u32);

impl ResourceId {
    /// Invalid resource ID
    pub const INVALID: Self = Self(!0);

    /// Creates a new resource ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Checks if valid
    pub const fn is_valid(&self) -> bool {
        self.0 != !0
    }
}

/// Resource type in the graph
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceType {
    /// Buffer resource
    Buffer = 0,
    /// 2D texture
    Texture2D = 1,
    /// 2D texture array
    Texture2DArray = 2,
    /// Cube texture
    TextureCube = 3,
    /// 3D texture
    Texture3D = 4,
    /// Imported external resource
    Imported = 5,
    /// Transient (render graph managed)
    Transient = 6,
}

/// Resource access type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ResourceAccess {
    /// No access
    None = 0,
    /// Read only
    Read = 1,
    /// Write only
    Write = 2,
    /// Read and write
    ReadWrite = 3,
}

impl ResourceAccess {
    /// Checks if reads
    pub const fn reads(&self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    /// Checks if writes
    pub const fn writes(&self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite)
    }

    /// Combines two accesses
    pub const fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::None, x) | (x, Self::None) => x,
            (Self::ReadWrite, _) | (_, Self::ReadWrite) => Self::ReadWrite,
            (Self::Read, Self::Write) | (Self::Write, Self::Read) => Self::ReadWrite,
            (Self::Read, Self::Read) => Self::Read,
            (Self::Write, Self::Write) => Self::Write,
        }
    }
}

/// Resource usage in a pass
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PassResourceUsage {
    /// Not used
    None = 0,
    /// Input (read in shader)
    Input = 1,
    /// Output (written in shader)
    Output = 2,
    /// Color attachment
    ColorAttachment = 3,
    /// Depth attachment
    DepthAttachment = 4,
    /// Depth read only
    DepthReadOnly = 5,
    /// Resolve attachment
    ResolveAttachment = 6,
    /// Transfer source
    TransferSrc = 7,
    /// Transfer destination
    TransferDst = 8,
    /// Storage (read/write)
    Storage = 9,
    /// Sampled
    Sampled = 10,
    /// Present
    Present = 11,
}

impl PassResourceUsage {
    /// Gets access type
    pub const fn access(&self) -> ResourceAccess {
        match self {
            Self::None => ResourceAccess::None,
            Self::Input | Self::Sampled | Self::DepthReadOnly | Self::TransferSrc => {
                ResourceAccess::Read
            }
            Self::Output | Self::TransferDst | Self::Present => ResourceAccess::Write,
            Self::ColorAttachment
            | Self::DepthAttachment
            | Self::ResolveAttachment
            | Self::Storage => ResourceAccess::ReadWrite,
        }
    }

    /// Gets image layout
    pub const fn image_layout(&self) -> ImageLayout {
        match self {
            Self::None => ImageLayout::Undefined,
            Self::Input | Self::Sampled => ImageLayout::ShaderReadOnlyOptimal,
            Self::Output | Self::Storage => ImageLayout::General,
            Self::ColorAttachment | Self::ResolveAttachment => ImageLayout::ColorAttachmentOptimal,
            Self::DepthAttachment => ImageLayout::DepthStencilAttachmentOptimal,
            Self::DepthReadOnly => ImageLayout::DepthStencilReadOnlyOptimal,
            Self::TransferSrc => ImageLayout::TransferSrcOptimal,
            Self::TransferDst => ImageLayout::TransferDstOptimal,
            Self::Present => ImageLayout::PresentSrc,
        }
    }

    /// Gets pipeline stage flags
    pub const fn pipeline_stage(&self) -> PipelineStageFlags {
        match self {
            Self::None => PipelineStageFlags::TOP_OF_PIPE,
            Self::Input | Self::Sampled => PipelineStageFlags::ALL_SHADERS,
            Self::Output | Self::Storage => PipelineStageFlags::ALL_SHADERS,
            Self::ColorAttachment | Self::ResolveAttachment => {
                PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
            }
            Self::DepthAttachment | Self::DepthReadOnly => PipelineStageFlags(
                PipelineStageFlags::EARLY_FRAGMENT_TESTS.0
                    | PipelineStageFlags::LATE_FRAGMENT_TESTS.0,
            ),
            Self::TransferSrc | Self::TransferDst => PipelineStageFlags::TRANSFER,
            Self::Present => PipelineStageFlags::BOTTOM_OF_PIPE,
        }
    }

    /// Gets access flags
    pub const fn access_flags(&self) -> AccessFlags {
        match self {
            Self::None => AccessFlags::NONE,
            Self::Input | Self::Sampled => AccessFlags::SHADER_READ,
            Self::Output => AccessFlags::SHADER_WRITE,
            Self::Storage => AccessFlags(AccessFlags::SHADER_READ.0 | AccessFlags::SHADER_WRITE.0),
            Self::ColorAttachment | Self::ResolveAttachment => AccessFlags(
                AccessFlags::COLOR_ATTACHMENT_READ.0 | AccessFlags::COLOR_ATTACHMENT_WRITE.0,
            ),
            Self::DepthAttachment => AccessFlags(
                AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ.0
                    | AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE.0,
            ),
            Self::DepthReadOnly => AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ,
            Self::TransferSrc => AccessFlags::TRANSFER_READ,
            Self::TransferDst => AccessFlags::TRANSFER_WRITE,
            Self::Present => AccessFlags::NONE,
        }
    }
}

/// Texture format for render graph resources
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum TextureFormat {
    /// RGBA8 unorm
    Rgba8Unorm = 0,
    /// RGBA8 sRGB
    Rgba8Srgb = 1,
    /// BGRA8 unorm
    Bgra8Unorm = 2,
    /// BGRA8 sRGB
    Bgra8Srgb = 3,
    /// RGBA16 float
    Rgba16Float = 4,
    /// RGBA32 float
    Rgba32Float = 5,
    /// RGB10A2 unorm
    Rgb10a2Unorm = 6,
    /// RG11B10 float
    Rg11b10Float = 7,
    /// R8 unorm
    R8Unorm = 8,
    /// RG8 unorm
    Rg8Unorm = 9,
    /// R16 float
    R16Float = 10,
    /// RG16 float
    Rg16Float = 11,
    /// R32 float
    R32Float = 12,
    /// RG32 float
    Rg32Float = 13,
    /// Depth16
    Depth16 = 14,
    /// Depth24
    Depth24 = 15,
    /// Depth32 float
    Depth32Float = 16,
    /// Depth24 stencil8
    Depth24Stencil8 = 17,
    /// Depth32 stencil8
    Depth32Stencil8 = 18,
}

impl TextureFormat {
    /// Checks if depth format
    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::Depth16
                | Self::Depth24
                | Self::Depth32Float
                | Self::Depth24Stencil8
                | Self::Depth32Stencil8
        )
    }

    /// Checks if stencil format
    pub const fn is_stencil(&self) -> bool {
        matches!(self, Self::Depth24Stencil8 | Self::Depth32Stencil8)
    }

    /// Checks if sRGB format
    pub const fn is_srgb(&self) -> bool {
        matches!(self, Self::Rgba8Srgb | Self::Bgra8Srgb)
    }

    /// Gets bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8Unorm => 1,
            Self::Rg8Unorm | Self::R16Float | Self::Depth16 => 2,
            Self::Depth24 => 3,
            Self::Rgba8Unorm
            | Self::Rgba8Srgb
            | Self::Bgra8Unorm
            | Self::Bgra8Srgb
            | Self::Rgb10a2Unorm
            | Self::Rg11b10Float
            | Self::Rg16Float
            | Self::R32Float
            | Self::Depth32Float
            | Self::Depth24Stencil8 => 4,
            Self::Depth32Stencil8 => 5,
            Self::Rgba16Float | Self::Rg32Float => 8,
            Self::Rgba32Float => 16,
        }
    }
}

/// Texture description for render graph
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TextureDesc {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth (for 3D) or array layers
    pub depth_or_layers: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Format
    pub format: TextureFormat,
    /// Sample count
    pub samples: u32,
}

impl Default for TextureDesc {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            depth_or_layers: 1,
            mip_levels: 1,
            format: TextureFormat::Rgba8Unorm,
            samples: 1,
        }
    }
}

impl TextureDesc {
    /// Creates a 2D texture description
    pub const fn new_2d(width: u32, height: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            depth_or_layers: 1,
            mip_levels: 1,
            format,
            samples: 1,
        }
    }

    /// Creates a render target description
    pub const fn render_target(width: u32, height: u32, format: TextureFormat) -> Self {
        Self::new_2d(width, height, format)
    }

    /// Creates a depth buffer description
    pub const fn depth_buffer(width: u32, height: u32) -> Self {
        Self::new_2d(width, height, TextureFormat::Depth32Float)
    }

    /// With mip levels
    pub const fn with_mips(mut self, mips: u32) -> Self {
        self.mip_levels = mips;
        self
    }

    /// With sample count
    pub const fn with_samples(mut self, samples: u32) -> Self {
        self.samples = samples;
        self
    }

    /// Calculate full mip chain
    pub fn with_full_mip_chain(mut self) -> Self {
        let max_dim = self.width.max(self.height);
        self.mip_levels = (max_dim as f32).log2().floor() as u32 + 1;
        self
    }

    /// Half resolution
    pub const fn half_res(self) -> Self {
        Self {
            width: (self.width / 2).max(1),
            height: (self.height / 2).max(1),
            ..self
        }
    }

    /// Quarter resolution
    pub const fn quarter_res(self) -> Self {
        Self {
            width: (self.width / 4).max(1),
            height: (self.height / 4).max(1),
            ..self
        }
    }
}

/// Buffer description for render graph
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferDesc {
    /// Size in bytes
    pub size: u64,
    /// Usage flags
    pub usage: BufferUsageFlags,
}

impl Default for BufferDesc {
    fn default() -> Self {
        Self {
            size: 0,
            usage: BufferUsageFlags::STORAGE,
        }
    }
}

impl BufferDesc {
    /// Creates a new buffer description
    pub const fn new(size: u64, usage: BufferUsageFlags) -> Self {
        Self { size, usage }
    }

    /// Storage buffer
    pub const fn storage(size: u64) -> Self {
        Self::new(size, BufferUsageFlags::STORAGE)
    }

    /// Uniform buffer
    pub const fn uniform(size: u64) -> Self {
        Self::new(size, BufferUsageFlags::UNIFORM)
    }

    /// Vertex buffer
    pub const fn vertex(size: u64) -> Self {
        Self::new(size, BufferUsageFlags::VERTEX)
    }

    /// Index buffer
    pub const fn index(size: u64) -> Self {
        Self::new(size, BufferUsageFlags::INDEX)
    }
}

/// Buffer usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BufferUsageFlags(pub u32);

impl BufferUsageFlags {
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 0);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 1);
    /// Uniform buffer
    pub const UNIFORM: Self = Self(1 << 2);
    /// Storage buffer
    pub const STORAGE: Self = Self(1 << 3);
    /// Index buffer
    pub const INDEX: Self = Self(1 << 4);
    /// Vertex buffer
    pub const VERTEX: Self = Self(1 << 5);
    /// Indirect buffer
    pub const INDIRECT: Self = Self(1 << 6);
    /// Shader binding table
    pub const SHADER_BINDING_TABLE: Self = Self(1 << 7);
    /// Acceleration structure storage
    pub const ACCELERATION_STRUCTURE: Self = Self(1 << 8);

    /// Common GPU-only buffer
    pub const GPU_ONLY: Self = Self(Self::STORAGE.0 | Self::TRANSFER_DST.0);

    /// Common staging buffer
    pub const STAGING: Self = Self(Self::TRANSFER_SRC.0 | Self::TRANSFER_DST.0);
}

impl core::ops::BitOr for BufferUsageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Pass type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PassType {
    /// Graphics pass (rasterization)
    Graphics = 0,
    /// Compute pass
    Compute = 1,
    /// Transfer pass
    Transfer = 2,
    /// Ray tracing pass
    RayTracing = 3,
    /// Present pass
    Present = 4,
}

/// Render pass description
#[derive(Clone, Debug)]
pub struct RenderPassDesc {
    /// Pass type
    pub pass_type: PassType,
    /// Queue type hint
    pub queue_type: QueueType,
    /// Color attachment count
    pub color_attachment_count: u32,
    /// Has depth attachment
    pub has_depth: bool,
    /// Render area width
    pub width: u32,
    /// Render area height
    pub height: u32,
}

impl Default for RenderPassDesc {
    fn default() -> Self {
        Self {
            pass_type: PassType::Graphics,
            queue_type: QueueType::Graphics,
            color_attachment_count: 1,
            has_depth: true,
            width: 0,
            height: 0,
        }
    }
}

/// Queue type for pass execution
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum QueueType {
    /// Graphics queue (supports all operations)
    Graphics = 0,
    /// Compute queue
    Compute = 1,
    /// Transfer/copy queue
    Transfer = 2,
}

impl QueueType {
    /// Checks if supports graphics
    pub const fn supports_graphics(&self) -> bool {
        matches!(self, Self::Graphics)
    }

    /// Checks if supports compute
    pub const fn supports_compute(&self) -> bool {
        matches!(self, Self::Graphics | Self::Compute)
    }

    /// Checks if supports transfer
    pub const fn supports_transfer(&self) -> bool {
        true
    }
}

/// Resource binding in a pass
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PassResourceBinding {
    /// Resource ID
    pub resource: ResourceId,
    /// Usage in this pass
    pub usage: PassResourceUsage,
    /// Binding slot (for shaders)
    pub binding: u32,
    /// Set (for descriptor sets)
    pub set: u32,
}

impl PassResourceBinding {
    /// Creates a new binding
    pub const fn new(resource: ResourceId, usage: PassResourceUsage) -> Self {
        Self {
            resource,
            usage,
            binding: 0,
            set: 0,
        }
    }

    /// With binding location
    pub const fn at(mut self, set: u32, binding: u32) -> Self {
        self.set = set;
        self.binding = binding;
        self
    }

    /// As sampled texture
    pub const fn sampled(resource: ResourceId) -> Self {
        Self::new(resource, PassResourceUsage::Sampled)
    }

    /// As storage image
    pub const fn storage(resource: ResourceId) -> Self {
        Self::new(resource, PassResourceUsage::Storage)
    }

    /// As color attachment
    pub const fn color(resource: ResourceId) -> Self {
        Self::new(resource, PassResourceUsage::ColorAttachment)
    }

    /// As depth attachment
    pub const fn depth(resource: ResourceId) -> Self {
        Self::new(resource, PassResourceUsage::DepthAttachment)
    }
}

/// Load operation for attachments
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum LoadOp {
    /// Load existing contents
    Load = 0,
    /// Clear to specified value
    #[default]
    Clear = 1,
    /// Don't care (undefined contents)
    DontCare = 2,
}

/// Store operation for attachments
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum StoreOp {
    /// Store contents
    #[default]
    Store = 0,
    /// Don't care (contents may be discarded)
    DontCare = 1,
    /// No store (for MSAA resolve)
    None = 2,
}

/// Attachment operations
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AttachmentOps {
    /// Load operation
    pub load: LoadOp,
    /// Store operation
    pub store: StoreOp,
    /// Stencil load (for depth-stencil)
    pub stencil_load: LoadOp,
    /// Stencil store (for depth-stencil)
    pub stencil_store: StoreOp,
}

impl AttachmentOps {
    /// Clear and store
    pub const CLEAR_STORE: Self = Self {
        load: LoadOp::Clear,
        store: StoreOp::Store,
        stencil_load: LoadOp::DontCare,
        stencil_store: StoreOp::DontCare,
    };

    /// Load and store
    pub const LOAD_STORE: Self = Self {
        load: LoadOp::Load,
        store: StoreOp::Store,
        stencil_load: LoadOp::DontCare,
        stencil_store: StoreOp::DontCare,
    };

    /// Clear, discard (transient)
    pub const CLEAR_DISCARD: Self = Self {
        load: LoadOp::Clear,
        store: StoreOp::DontCare,
        stencil_load: LoadOp::DontCare,
        stencil_store: StoreOp::DontCare,
    };

    /// Don't care (fully transient)
    pub const DONT_CARE: Self = Self {
        load: LoadOp::DontCare,
        store: StoreOp::DontCare,
        stencil_load: LoadOp::DontCare,
        stencil_store: StoreOp::DontCare,
    };
}

/// Dependency between passes
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PassDependency {
    /// Source pass
    pub src_pass: RenderPassId,
    /// Destination pass
    pub dst_pass: RenderPassId,
    /// Source stage
    pub src_stage: PipelineStageFlags,
    /// Destination stage
    pub dst_stage: PipelineStageFlags,
}

impl PassDependency {
    /// Creates a new dependency
    pub const fn new(src: RenderPassId, dst: RenderPassId) -> Self {
        Self {
            src_pass: src,
            dst_pass: dst,
            src_stage: PipelineStageFlags::ALL_COMMANDS,
            dst_stage: PipelineStageFlags::ALL_COMMANDS,
        }
    }

    /// With specific stages
    pub const fn with_stages(
        mut self,
        src_stage: PipelineStageFlags,
        dst_stage: PipelineStageFlags,
    ) -> Self {
        self.src_stage = src_stage;
        self.dst_stage = dst_stage;
        self
    }
}
