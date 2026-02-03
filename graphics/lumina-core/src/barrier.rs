//! Barrier and synchronization types
//!
//! This module provides memory and execution barrier primitives for GPU synchronization.

/// Pipeline stage flags for synchronization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PipelineStageFlags(pub u64);

impl PipelineStageFlags {
    /// No stage
    pub const NONE: Self = Self(0);
    /// Top of pipe (before any commands)
    pub const TOP_OF_PIPE: Self = Self(1 << 0);
    /// Draw indirect command read
    pub const DRAW_INDIRECT: Self = Self(1 << 1);
    /// Vertex input
    pub const VERTEX_INPUT: Self = Self(1 << 2);
    /// Vertex shader
    pub const VERTEX_SHADER: Self = Self(1 << 3);
    /// Tessellation control shader
    pub const TESSELLATION_CONTROL_SHADER: Self = Self(1 << 4);
    /// Tessellation evaluation shader
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
    /// Transfer operations
    pub const TRANSFER: Self = Self(1 << 12);
    /// Bottom of pipe (after all commands)
    pub const BOTTOM_OF_PIPE: Self = Self(1 << 13);
    /// Host access
    pub const HOST: Self = Self(1 << 14);
    /// All graphics stages
    pub const ALL_GRAPHICS: Self = Self(1 << 15);
    /// All commands
    pub const ALL_COMMANDS: Self = Self(1 << 16);
    /// Copy operations
    pub const COPY: Self = Self(1 << 17);
    /// Resolve operations
    pub const RESOLVE: Self = Self(1 << 18);
    /// Blit operations
    pub const BLIT: Self = Self(1 << 19);
    /// Clear operations
    pub const CLEAR: Self = Self(1 << 20);
    /// Index input
    pub const INDEX_INPUT: Self = Self(1 << 21);
    /// Vertex attribute input
    pub const VERTEX_ATTRIBUTE_INPUT: Self = Self(1 << 22);
    /// Pre-rasterization shaders
    pub const PRE_RASTERIZATION_SHADERS: Self = Self(1 << 23);
    /// Task shader
    pub const TASK_SHADER: Self = Self(1 << 24);
    /// Mesh shader
    pub const MESH_SHADER: Self = Self(1 << 25);
    /// Ray tracing shader
    pub const RAY_TRACING_SHADER: Self = Self(1 << 26);
    /// Acceleration structure build
    pub const ACCELERATION_STRUCTURE_BUILD: Self = Self(1 << 27);
    /// Acceleration structure copy
    pub const ACCELERATION_STRUCTURE_COPY: Self = Self(1 << 28);
    /// Fragment shading rate attachment
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT: Self = Self(1 << 29);
    /// Fragment density process
    pub const FRAGMENT_DENSITY_PROCESS: Self = Self(1 << 30);
    /// Conditional rendering
    pub const CONDITIONAL_RENDERING: Self = Self(1 << 31);
    /// Transform feedback
    pub const TRANSFORM_FEEDBACK: Self = Self(1 << 32);
    /// Command preprocess
    pub const COMMAND_PREPROCESS: Self = Self(1 << 33);

    /// All shader stages
    pub const ALL_SHADERS: Self = Self(
        Self::VERTEX_SHADER.0
            | Self::TESSELLATION_CONTROL_SHADER.0
            | Self::TESSELLATION_EVALUATION_SHADER.0
            | Self::GEOMETRY_SHADER.0
            | Self::FRAGMENT_SHADER.0
            | Self::COMPUTE_SHADER.0
            | Self::TASK_SHADER.0
            | Self::MESH_SHADER.0
            | Self::RAY_TRACING_SHADER.0
    );

    /// All transfer stages
    pub const ALL_TRANSFER: Self = Self(
        Self::TRANSFER.0 | Self::COPY.0 | Self::BLIT.0 | Self::RESOLVE.0 | Self::CLEAR.0
    );

    /// Checks if contains flags
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Checks if empty
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
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

impl core::ops::BitOrAssign for PipelineStageFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Access flags for memory dependencies
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct AccessFlags(pub u64);

impl AccessFlags {
    /// No access
    pub const NONE: Self = Self(0);
    /// Indirect command read
    pub const INDIRECT_COMMAND_READ: Self = Self(1 << 0);
    /// Index buffer read
    pub const INDEX_READ: Self = Self(1 << 1);
    /// Vertex buffer read
    pub const VERTEX_ATTRIBUTE_READ: Self = Self(1 << 2);
    /// Uniform buffer read
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
    /// Shader sampled read
    pub const SHADER_SAMPLED_READ: Self = Self(1 << 17);
    /// Shader storage read
    pub const SHADER_STORAGE_READ: Self = Self(1 << 18);
    /// Shader storage write
    pub const SHADER_STORAGE_WRITE: Self = Self(1 << 19);
    /// Acceleration structure read
    pub const ACCELERATION_STRUCTURE_READ: Self = Self(1 << 20);
    /// Acceleration structure write
    pub const ACCELERATION_STRUCTURE_WRITE: Self = Self(1 << 21);
    /// Fragment density map read
    pub const FRAGMENT_DENSITY_MAP_READ: Self = Self(1 << 22);
    /// Fragment shading rate read
    pub const FRAGMENT_SHADING_RATE_ATTACHMENT_READ: Self = Self(1 << 23);
    /// Command preprocess read
    pub const COMMAND_PREPROCESS_READ: Self = Self(1 << 24);
    /// Command preprocess write
    pub const COMMAND_PREPROCESS_WRITE: Self = Self(1 << 25);
    /// Transform feedback write
    pub const TRANSFORM_FEEDBACK_WRITE: Self = Self(1 << 26);
    /// Transform feedback counter read
    pub const TRANSFORM_FEEDBACK_COUNTER_READ: Self = Self(1 << 27);
    /// Transform feedback counter write
    pub const TRANSFORM_FEEDBACK_COUNTER_WRITE: Self = Self(1 << 28);
    /// Conditional rendering read
    pub const CONDITIONAL_RENDERING_READ: Self = Self(1 << 29);

    /// All read accesses
    pub const ALL_READ: Self = Self(
        Self::INDIRECT_COMMAND_READ.0
            | Self::INDEX_READ.0
            | Self::VERTEX_ATTRIBUTE_READ.0
            | Self::UNIFORM_READ.0
            | Self::INPUT_ATTACHMENT_READ.0
            | Self::SHADER_READ.0
            | Self::COLOR_ATTACHMENT_READ.0
            | Self::DEPTH_STENCIL_ATTACHMENT_READ.0
            | Self::TRANSFER_READ.0
            | Self::HOST_READ.0
            | Self::MEMORY_READ.0
    );

    /// All write accesses
    pub const ALL_WRITE: Self = Self(
        Self::SHADER_WRITE.0
            | Self::COLOR_ATTACHMENT_WRITE.0
            | Self::DEPTH_STENCIL_ATTACHMENT_WRITE.0
            | Self::TRANSFER_WRITE.0
            | Self::HOST_WRITE.0
            | Self::MEMORY_WRITE.0
    );

    /// Checks if contains flags
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Checks if read access
    pub const fn is_read(&self) -> bool {
        (self.0 & Self::ALL_READ.0) != 0
    }

    /// Checks if write access
    pub const fn is_write(&self) -> bool {
        (self.0 & Self::ALL_WRITE.0) != 0
    }
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

/// Memory barrier (global memory dependency)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryBarrier {
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
}

impl MemoryBarrier {
    /// Creates a new memory barrier
    pub const fn new(src_access: AccessFlags, dst_access: AccessFlags) -> Self {
        Self { src_access, dst_access }
    }

    /// Full memory barrier
    pub const fn full() -> Self {
        Self {
            src_access: AccessFlags::MEMORY_WRITE,
            dst_access: AccessFlags::MEMORY_READ,
        }
    }

    /// Shader read after write
    pub const fn shader_read_after_write() -> Self {
        Self {
            src_access: AccessFlags::SHADER_WRITE,
            dst_access: AccessFlags::SHADER_READ,
        }
    }

    /// Transfer to shader read
    pub const fn transfer_to_shader() -> Self {
        Self {
            src_access: AccessFlags::TRANSFER_WRITE,
            dst_access: AccessFlags::SHADER_READ,
        }
    }

    /// Host to device
    pub const fn host_to_device() -> Self {
        Self {
            src_access: AccessFlags::HOST_WRITE,
            dst_access: AccessFlags::MEMORY_READ,
        }
    }

    /// Device to host
    pub const fn device_to_host() -> Self {
        Self {
            src_access: AccessFlags::MEMORY_WRITE,
            dst_access: AccessFlags::HOST_READ,
        }
    }
}

impl Default for MemoryBarrier {
    fn default() -> Self {
        Self::full()
    }
}

/// Buffer memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct BufferBarrier {
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
    /// Source queue family (QUEUE_FAMILY_IGNORED for no transfer)
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
    /// Buffer handle
    pub buffer: BufferHandle,
    /// Offset in buffer
    pub offset: u64,
    /// Size of region (WHOLE_SIZE for entire buffer)
    pub size: u64,
}

/// Queue family ignored constant
pub const QUEUE_FAMILY_IGNORED: u32 = !0;

/// Whole size constant
pub const WHOLE_SIZE: u64 = !0;

/// Buffer handle (opaque)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BufferHandle(pub u64);

impl BufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates a new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Checks if null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl BufferBarrier {
    /// Creates a new buffer barrier
    pub const fn new(
        buffer: BufferHandle,
        src_access: AccessFlags,
        dst_access: AccessFlags,
    ) -> Self {
        Self {
            src_access,
            dst_access,
            src_queue_family: QUEUE_FAMILY_IGNORED,
            dst_queue_family: QUEUE_FAMILY_IGNORED,
            buffer,
            offset: 0,
            size: WHOLE_SIZE,
        }
    }

    /// With queue family transfer
    pub const fn with_queue_transfer(mut self, src: u32, dst: u32) -> Self {
        self.src_queue_family = src;
        self.dst_queue_family = dst;
        self
    }

    /// With specific region
    pub const fn with_region(mut self, offset: u64, size: u64) -> Self {
        self.offset = offset;
        self.size = size;
        self
    }
}

/// Image handle (opaque)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImageHandle(pub u64);

impl ImageHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates a new handle
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Checks if null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

/// Image layout for layout transitions
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImageLayout {
    /// Undefined (contents not preserved)
    #[default]
    Undefined = 0,
    /// General (all access types)
    General = 1,
    /// Color attachment optimal
    ColorAttachmentOptimal = 2,
    /// Depth stencil attachment optimal
    DepthStencilAttachmentOptimal = 3,
    /// Depth stencil read only optimal
    DepthStencilReadOnlyOptimal = 4,
    /// Shader read only optimal
    ShaderReadOnlyOptimal = 5,
    /// Transfer source optimal
    TransferSrcOptimal = 6,
    /// Transfer destination optimal
    TransferDstOptimal = 7,
    /// Preinitialized
    Preinitialized = 8,
    /// Present source
    PresentSrc = 9,
    /// Shared present
    SharedPresent = 10,
    /// Depth read only stencil attachment
    DepthReadOnlyStencilAttachmentOptimal = 11,
    /// Depth attachment stencil read only
    DepthAttachmentStencilReadOnlyOptimal = 12,
    /// Depth attachment optimal
    DepthAttachmentOptimal = 13,
    /// Depth read only optimal
    DepthReadOnlyOptimal = 14,
    /// Stencil attachment optimal
    StencilAttachmentOptimal = 15,
    /// Stencil read only optimal
    StencilReadOnlyOptimal = 16,
    /// Read only optimal
    ReadOnlyOptimal = 17,
    /// Attachment optimal
    AttachmentOptimal = 18,
    /// Fragment shading rate attachment
    FragmentShadingRateAttachmentOptimal = 19,
    /// Fragment density map optimal
    FragmentDensityMapOptimal = 20,
}

impl ImageLayout {
    /// Checks if this is a depth/stencil layout
    pub const fn is_depth_stencil(&self) -> bool {
        matches!(
            self,
            Self::DepthStencilAttachmentOptimal
                | Self::DepthStencilReadOnlyOptimal
                | Self::DepthReadOnlyStencilAttachmentOptimal
                | Self::DepthAttachmentStencilReadOnlyOptimal
                | Self::DepthAttachmentOptimal
                | Self::DepthReadOnlyOptimal
                | Self::StencilAttachmentOptimal
                | Self::StencilReadOnlyOptimal
        )
    }

    /// Checks if read-only
    pub const fn is_read_only(&self) -> bool {
        matches!(
            self,
            Self::ShaderReadOnlyOptimal
                | Self::DepthStencilReadOnlyOptimal
                | Self::TransferSrcOptimal
                | Self::DepthReadOnlyOptimal
                | Self::StencilReadOnlyOptimal
                | Self::ReadOnlyOptimal
                | Self::PresentSrc
        )
    }
}

/// Image aspect flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ImageAspectFlags(pub u32);

impl ImageAspectFlags {
    /// No aspect
    pub const NONE: Self = Self(0);
    /// Color aspect
    pub const COLOR: Self = Self(1 << 0);
    /// Depth aspect
    pub const DEPTH: Self = Self(1 << 1);
    /// Stencil aspect
    pub const STENCIL: Self = Self(1 << 2);
    /// Metadata aspect
    pub const METADATA: Self = Self(1 << 3);
    /// Plane 0 aspect (multi-planar formats)
    pub const PLANE_0: Self = Self(1 << 4);
    /// Plane 1 aspect
    pub const PLANE_1: Self = Self(1 << 5);
    /// Plane 2 aspect
    pub const PLANE_2: Self = Self(1 << 6);
    /// Memory plane 0
    pub const MEMORY_PLANE_0: Self = Self(1 << 7);
    /// Memory plane 1
    pub const MEMORY_PLANE_1: Self = Self(1 << 8);
    /// Memory plane 2
    pub const MEMORY_PLANE_2: Self = Self(1 << 9);
    /// Memory plane 3
    pub const MEMORY_PLANE_3: Self = Self(1 << 10);

    /// Depth and stencil
    pub const DEPTH_STENCIL: Self = Self(Self::DEPTH.0 | Self::STENCIL.0);

    /// Checks if contains
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for ImageAspectFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Image subresource range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageSubresourceRange {
    /// Aspect mask
    pub aspect_mask: ImageAspectFlags,
    /// Base mip level
    pub base_mip_level: u32,
    /// Mip level count
    pub level_count: u32,
    /// Base array layer
    pub base_array_layer: u32,
    /// Layer count
    pub layer_count: u32,
}

impl Default for ImageSubresourceRange {
    fn default() -> Self {
        Self::color()
    }
}

impl ImageSubresourceRange {
    /// All mip levels constant
    pub const REMAINING_MIP_LEVELS: u32 = !0;
    /// All array layers constant
    pub const REMAINING_ARRAY_LAYERS: u32 = !0;

    /// Creates a color image range (all mips, all layers)
    pub const fn color() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: Self::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: Self::REMAINING_ARRAY_LAYERS,
        }
    }

    /// Creates a depth image range
    pub const fn depth() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH,
            base_mip_level: 0,
            level_count: Self::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: Self::REMAINING_ARRAY_LAYERS,
        }
    }

    /// Creates a depth-stencil image range
    pub const fn depth_stencil() -> Self {
        Self {
            aspect_mask: ImageAspectFlags::DEPTH_STENCIL,
            base_mip_level: 0,
            level_count: Self::REMAINING_MIP_LEVELS,
            base_array_layer: 0,
            layer_count: Self::REMAINING_ARRAY_LAYERS,
        }
    }

    /// With specific mip levels
    pub const fn with_mips(mut self, base: u32, count: u32) -> Self {
        self.base_mip_level = base;
        self.level_count = count;
        self
    }

    /// With specific array layers
    pub const fn with_layers(mut self, base: u32, count: u32) -> Self {
        self.base_array_layer = base;
        self.layer_count = count;
        self
    }

    /// Single mip level
    pub const fn single_mip(mut self, level: u32) -> Self {
        self.base_mip_level = level;
        self.level_count = 1;
        self
    }

    /// Single array layer
    pub const fn single_layer(mut self, layer: u32) -> Self {
        self.base_array_layer = layer;
        self.layer_count = 1;
        self
    }
}

/// Image memory barrier
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImageBarrier {
    /// Source access mask
    pub src_access: AccessFlags,
    /// Destination access mask
    pub dst_access: AccessFlags,
    /// Old layout
    pub old_layout: ImageLayout,
    /// New layout
    pub new_layout: ImageLayout,
    /// Source queue family
    pub src_queue_family: u32,
    /// Destination queue family
    pub dst_queue_family: u32,
    /// Image handle
    pub image: ImageHandle,
    /// Subresource range
    pub subresource_range: ImageSubresourceRange,
}

impl ImageBarrier {
    /// Creates a new image barrier
    pub const fn new(
        image: ImageHandle,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
    ) -> Self {
        Self {
            src_access: AccessFlags::NONE,
            dst_access: AccessFlags::NONE,
            old_layout,
            new_layout,
            src_queue_family: QUEUE_FAMILY_IGNORED,
            dst_queue_family: QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: ImageSubresourceRange::color(),
        }
    }

    /// With access flags
    pub const fn with_access(mut self, src: AccessFlags, dst: AccessFlags) -> Self {
        self.src_access = src;
        self.dst_access = dst;
        self
    }

    /// With queue family transfer
    pub const fn with_queue_transfer(mut self, src: u32, dst: u32) -> Self {
        self.src_queue_family = src;
        self.dst_queue_family = dst;
        self
    }

    /// With subresource range
    pub const fn with_subresource(mut self, range: ImageSubresourceRange) -> Self {
        self.subresource_range = range;
        self
    }

    /// Undefined to transfer destination
    pub const fn undefined_to_transfer_dst(image: ImageHandle) -> Self {
        Self::new(image, ImageLayout::Undefined, ImageLayout::TransferDstOptimal)
            .with_access(AccessFlags::NONE, AccessFlags::TRANSFER_WRITE)
    }

    /// Transfer destination to shader read
    pub const fn transfer_dst_to_shader_read(image: ImageHandle) -> Self {
        Self::new(image, ImageLayout::TransferDstOptimal, ImageLayout::ShaderReadOnlyOptimal)
            .with_access(AccessFlags::TRANSFER_WRITE, AccessFlags::SHADER_READ)
    }

    /// Color attachment to present
    pub const fn color_to_present(image: ImageHandle) -> Self {
        Self::new(image, ImageLayout::ColorAttachmentOptimal, ImageLayout::PresentSrc)
            .with_access(AccessFlags::COLOR_ATTACHMENT_WRITE, AccessFlags::NONE)
    }

    /// Present to color attachment
    pub const fn present_to_color(image: ImageHandle) -> Self {
        Self::new(image, ImageLayout::PresentSrc, ImageLayout::ColorAttachmentOptimal)
            .with_access(AccessFlags::NONE, AccessFlags::COLOR_ATTACHMENT_WRITE)
    }

    /// Undefined to color attachment
    pub const fn undefined_to_color(image: ImageHandle) -> Self {
        Self::new(image, ImageLayout::Undefined, ImageLayout::ColorAttachmentOptimal)
            .with_access(AccessFlags::NONE, AccessFlags::COLOR_ATTACHMENT_WRITE)
    }

    /// Undefined to depth attachment
    pub fn undefined_to_depth(image: ImageHandle) -> Self {
        Self::new(image, ImageLayout::Undefined, ImageLayout::DepthStencilAttachmentOptimal)
            .with_access(
                AccessFlags::NONE,
                AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            )
            .with_subresource(ImageSubresourceRange::depth())
    }
}

/// Dependency flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DependencyFlags(pub u32);

impl DependencyFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// By region (framebuffer-local)
    pub const BY_REGION: Self = Self(1 << 0);
    /// Device group
    pub const DEVICE_GROUP: Self = Self(1 << 1);
    /// View local (multiview)
    pub const VIEW_LOCAL: Self = Self(1 << 2);
    /// Feedback loop
    pub const FEEDBACK_LOOP: Self = Self(1 << 3);
}

impl core::ops::BitOr for DependencyFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Pipeline barrier command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PipelineBarrier {
    /// Source stage mask
    pub src_stage_mask: PipelineStageFlags,
    /// Destination stage mask
    pub dst_stage_mask: PipelineStageFlags,
    /// Dependency flags
    pub dependency_flags: DependencyFlags,
}

impl PipelineBarrier {
    /// Creates a new pipeline barrier
    pub const fn new(src_stage: PipelineStageFlags, dst_stage: PipelineStageFlags) -> Self {
        Self {
            src_stage_mask: src_stage,
            dst_stage_mask: dst_stage,
            dependency_flags: DependencyFlags::NONE,
        }
    }

    /// With dependency flags
    pub const fn with_flags(mut self, flags: DependencyFlags) -> Self {
        self.dependency_flags = flags;
        self
    }

    /// Full pipeline barrier
    pub const fn full() -> Self {
        Self::new(
            PipelineStageFlags::ALL_COMMANDS,
            PipelineStageFlags::ALL_COMMANDS,
        )
    }

    /// Compute to compute
    pub const fn compute_to_compute() -> Self {
        Self::new(
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::COMPUTE_SHADER,
        )
    }

    /// Compute to graphics
    pub const fn compute_to_graphics() -> Self {
        Self::new(
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::ALL_GRAPHICS,
        )
    }

    /// Graphics to compute
    pub const fn graphics_to_compute() -> Self {
        Self::new(
            PipelineStageFlags::ALL_GRAPHICS,
            PipelineStageFlags::COMPUTE_SHADER,
        )
    }

    /// Transfer to shader
    pub const fn transfer_to_shader() -> Self {
        Self::new(
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::VERTEX_SHADER
                .union(PipelineStageFlags::FRAGMENT_SHADER)
                .union(PipelineStageFlags::COMPUTE_SHADER),
        )
    }

    /// Render to present
    pub const fn render_to_present() -> Self {
        Self::new(
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            PipelineStageFlags::BOTTOM_OF_PIPE,
        )
    }
}
