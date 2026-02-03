//! Render Graph for Lumina
//!
//! This module provides a declarative render graph system for automatic
//! resource management, barrier insertion, and pass scheduling.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Render Graph
// ============================================================================

/// Render graph for declarative rendering
#[derive(Debug, Default)]
pub struct RenderGraph {
    /// Passes
    passes: Vec<RenderGraphPass>,
    /// Resources
    resources: Vec<RenderGraphResource>,
    /// External resources (imported)
    external_resources: Vec<ExternalResource>,
    /// Compiled execution order
    execution_order: Vec<usize>,
    /// Is compiled
    compiled: bool,
}

impl RenderGraph {
    /// Creates new render graph
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            resources: Vec::new(),
            external_resources: Vec::new(),
            execution_order: Vec::new(),
            compiled: false,
        }
    }

    /// Add a render pass
    pub fn add_pass(&mut self, pass: RenderGraphPass) -> PassHandle {
        let handle = PassHandle(self.passes.len() as u32);
        self.passes.push(pass);
        self.compiled = false;
        handle
    }

    /// Create a transient texture resource
    pub fn create_texture(&mut self, info: TextureResourceInfo) -> ResourceHandle {
        let handle = ResourceHandle(self.resources.len() as u32);
        self.resources.push(RenderGraphResource::Texture(info));
        self.compiled = false;
        handle
    }

    /// Create a transient buffer resource
    pub fn create_buffer(&mut self, info: BufferResourceInfo) -> ResourceHandle {
        let handle = ResourceHandle(self.resources.len() as u32);
        self.resources.push(RenderGraphResource::Buffer(info));
        self.compiled = false;
        handle
    }

    /// Import external texture
    pub fn import_texture(&mut self, handle: u64, info: TextureResourceInfo) -> ResourceHandle {
        let resource_handle = ResourceHandle((self.resources.len() + self.external_resources.len()) as u32 | 0x8000_0000);
        self.external_resources.push(ExternalResource::Texture { handle, info });
        self.compiled = false;
        resource_handle
    }

    /// Import external buffer
    pub fn import_buffer(&mut self, handle: u64, info: BufferResourceInfo) -> ResourceHandle {
        let resource_handle = ResourceHandle((self.resources.len() + self.external_resources.len()) as u32 | 0x8000_0000);
        self.external_resources.push(ExternalResource::Buffer { handle, info });
        self.compiled = false;
        resource_handle
    }

    /// Import swapchain image
    pub fn import_swapchain(&mut self, handle: u64, width: u32, height: u32, format: TextureFormat) -> ResourceHandle {
        self.import_texture(handle, TextureResourceInfo {
            width,
            height,
            depth: 1,
            mip_levels: 1,
            array_layers: 1,
            format,
            samples: 1,
            usage: TextureUsageFlags::COLOR_ATTACHMENT,
            name: Some(String::from("swapchain")),
        })
    }

    /// Compile the render graph
    pub fn compile(&mut self) -> Result<(), RenderGraphError> {
        // Build dependency graph
        let dependencies = self.build_dependencies()?;

        // Topological sort
        self.execution_order = self.topological_sort(&dependencies)?;

        // Calculate resource lifetimes
        self.calculate_lifetimes();

        self.compiled = true;
        Ok(())
    }

    /// Build pass dependencies
    fn build_dependencies(&self) -> Result<Vec<Vec<usize>>, RenderGraphError> {
        let mut deps = vec![Vec::new(); self.passes.len()];

        for (pass_idx, pass) in self.passes.iter().enumerate() {
            for read in &pass.reads {
                // Find which pass writes to this resource
                for (other_idx, other_pass) in self.passes.iter().enumerate() {
                    if other_idx != pass_idx {
                        for write in &other_pass.writes {
                            if write.resource == read.resource {
                                deps[pass_idx].push(other_idx);
                            }
                        }
                    }
                }
            }
        }

        Ok(deps)
    }

    /// Topological sort of passes
    fn topological_sort(&self, deps: &[Vec<usize>]) -> Result<Vec<usize>, RenderGraphError> {
        let n = self.passes.len();
        let mut in_degree = vec![0usize; n];
        let mut adj = vec![Vec::new(); n];

        for (i, dep_list) in deps.iter().enumerate() {
            for &dep in dep_list {
                adj[dep].push(i);
                in_degree[i] += 1;
            }
        }

        let mut queue = Vec::new();
        for (i, &degree) in in_degree.iter().enumerate() {
            if degree == 0 {
                queue.push(i);
            }
        }

        let mut result = Vec::with_capacity(n);
        while let Some(node) = queue.pop() {
            result.push(node);
            for &next in &adj[node] {
                in_degree[next] -= 1;
                if in_degree[next] == 0 {
                    queue.push(next);
                }
            }
        }

        if result.len() != n {
            return Err(RenderGraphError::CyclicDependency);
        }

        Ok(result)
    }

    /// Calculate resource lifetimes for aliasing
    fn calculate_lifetimes(&mut self) {
        // TODO: Calculate first/last use for each resource
        // to enable memory aliasing
    }

    /// Get execution order
    pub fn execution_order(&self) -> &[usize] {
        &self.execution_order
    }

    /// Get passes
    pub fn passes(&self) -> &[RenderGraphPass] {
        &self.passes
    }

    /// Get pass by index
    pub fn pass(&self, index: usize) -> Option<&RenderGraphPass> {
        self.passes.get(index)
    }

    /// Get resource
    pub fn resource(&self, handle: ResourceHandle) -> Option<&RenderGraphResource> {
        if handle.0 & 0x8000_0000 != 0 {
            None // External resource
        } else {
            self.resources.get(handle.0 as usize)
        }
    }

    /// Is compiled
    pub fn is_compiled(&self) -> bool {
        self.compiled
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.passes.clear();
        self.resources.clear();
        self.external_resources.clear();
        self.execution_order.clear();
        self.compiled = false;
    }
}

// ============================================================================
// Pass Handle
// ============================================================================

/// Handle to a render pass
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PassHandle(pub u32);

impl PassHandle {
    /// Invalid handle
    pub const INVALID: Self = Self(!0);

    /// Is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != !0
    }
}

impl Default for PassHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

// ============================================================================
// Resource Handle
// ============================================================================

/// Handle to a graph resource
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceHandle(pub u32);

impl ResourceHandle {
    /// Invalid handle
    pub const INVALID: Self = Self(!0);

    /// Is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != !0
    }

    /// Is external
    #[inline]
    pub const fn is_external(&self) -> bool {
        self.0 & 0x8000_0000 != 0
    }
}

impl Default for ResourceHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

// ============================================================================
// Render Graph Pass
// ============================================================================

/// A pass in the render graph
#[derive(Debug)]
pub struct RenderGraphPass {
    /// Pass name
    pub name: String,
    /// Pass type
    pub pass_type: PassType,
    /// Resources read by this pass
    pub reads: Vec<ResourceAccess>,
    /// Resources written by this pass
    pub writes: Vec<ResourceAccess>,
    /// Color attachments
    pub color_attachments: Vec<ColorAttachmentInfo>,
    /// Depth attachment
    pub depth_attachment: Option<DepthAttachmentInfo>,
    /// Render area
    pub render_area: Option<RenderArea>,
    /// Callback data (opaque)
    pub user_data: u64,
    /// Queue type
    pub queue_type: QueueType,
}

impl RenderGraphPass {
    /// Creates new graphics pass
    pub fn graphics(name: &str) -> Self {
        Self {
            name: String::from(name),
            pass_type: PassType::Graphics,
            reads: Vec::new(),
            writes: Vec::new(),
            color_attachments: Vec::new(),
            depth_attachment: None,
            render_area: None,
            user_data: 0,
            queue_type: QueueType::Graphics,
        }
    }

    /// Creates new compute pass
    pub fn compute(name: &str) -> Self {
        Self {
            name: String::from(name),
            pass_type: PassType::Compute,
            reads: Vec::new(),
            writes: Vec::new(),
            color_attachments: Vec::new(),
            depth_attachment: None,
            render_area: None,
            user_data: 0,
            queue_type: QueueType::Compute,
        }
    }

    /// Creates new transfer pass
    pub fn transfer(name: &str) -> Self {
        Self {
            name: String::from(name),
            pass_type: PassType::Transfer,
            reads: Vec::new(),
            writes: Vec::new(),
            color_attachments: Vec::new(),
            depth_attachment: None,
            render_area: None,
            user_data: 0,
            queue_type: QueueType::Transfer,
        }
    }

    /// Add read resource
    pub fn read(mut self, resource: ResourceHandle, access: ResourceAccessType) -> Self {
        self.reads.push(ResourceAccess { resource, access });
        self
    }

    /// Add write resource
    pub fn write(mut self, resource: ResourceHandle, access: ResourceAccessType) -> Self {
        self.writes.push(ResourceAccess { resource, access });
        self
    }

    /// Add color attachment (write)
    pub fn color_attachment(mut self, resource: ResourceHandle, load: LoadOp, store: StoreOp) -> Self {
        self.color_attachments.push(ColorAttachmentInfo {
            resource,
            load_op: load,
            store_op: store,
            clear_value: ClearColor::BLACK,
        });
        self.writes.push(ResourceAccess {
            resource,
            access: ResourceAccessType::ColorAttachmentWrite,
        });
        self
    }

    /// Add color attachment with clear
    pub fn color_attachment_clear(mut self, resource: ResourceHandle, clear: ClearColor) -> Self {
        self.color_attachments.push(ColorAttachmentInfo {
            resource,
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_value: clear,
        });
        self.writes.push(ResourceAccess {
            resource,
            access: ResourceAccessType::ColorAttachmentWrite,
        });
        self
    }

    /// Add depth attachment
    pub fn depth_attachment(mut self, resource: ResourceHandle, load: LoadOp, store: StoreOp) -> Self {
        self.depth_attachment = Some(DepthAttachmentInfo {
            resource,
            depth_load_op: load,
            depth_store_op: store,
            stencil_load_op: LoadOp::DontCare,
            stencil_store_op: StoreOp::DontCare,
            clear_depth: 1.0,
            clear_stencil: 0,
        });
        self.writes.push(ResourceAccess {
            resource,
            access: ResourceAccessType::DepthStencilAttachmentWrite,
        });
        self
    }

    /// Add depth attachment with clear
    pub fn depth_attachment_clear(mut self, resource: ResourceHandle, clear_depth: f32) -> Self {
        self.depth_attachment = Some(DepthAttachmentInfo {
            resource,
            depth_load_op: LoadOp::Clear,
            depth_store_op: StoreOp::Store,
            stencil_load_op: LoadOp::DontCare,
            stencil_store_op: StoreOp::DontCare,
            clear_depth,
            clear_stencil: 0,
        });
        self.writes.push(ResourceAccess {
            resource,
            access: ResourceAccessType::DepthStencilAttachmentWrite,
        });
        self
    }

    /// Set render area
    pub fn render_area(mut self, x: i32, y: i32, width: u32, height: u32) -> Self {
        self.render_area = Some(RenderArea { x, y, width, height });
        self
    }

    /// Set user data
    pub fn user_data(mut self, data: u64) -> Self {
        self.user_data = data;
        self
    }

    /// Set queue type
    pub fn queue(mut self, queue: QueueType) -> Self {
        self.queue_type = queue;
        self
    }

    /// Read texture as sampled
    pub fn sample_texture(self, resource: ResourceHandle) -> Self {
        self.read(resource, ResourceAccessType::ShaderSampledRead)
    }

    /// Read/write storage buffer
    pub fn storage_buffer_rw(mut self, resource: ResourceHandle) -> Self {
        self.reads.push(ResourceAccess {
            resource,
            access: ResourceAccessType::ShaderStorageRead,
        });
        self.writes.push(ResourceAccess {
            resource,
            access: ResourceAccessType::ShaderStorageWrite,
        });
        self
    }

    /// Read uniform buffer
    pub fn uniform_buffer(self, resource: ResourceHandle) -> Self {
        self.read(resource, ResourceAccessType::UniformRead)
    }

    /// Write storage image
    pub fn storage_image_write(self, resource: ResourceHandle) -> Self {
        self.write(resource, ResourceAccessType::ShaderStorageWrite)
    }
}

// ============================================================================
// Pass Type
// ============================================================================

/// Pass type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PassType {
    /// Graphics pass
    #[default]
    Graphics,
    /// Compute pass
    Compute,
    /// Transfer pass
    Transfer,
    /// Ray tracing pass
    RayTracing,
}

/// Queue type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum QueueType {
    /// Graphics queue
    #[default]
    Graphics,
    /// Compute queue
    Compute,
    /// Transfer queue
    Transfer,
}

// ============================================================================
// Resource Access
// ============================================================================

/// Resource access info
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResourceAccess {
    /// Resource handle
    pub resource: ResourceHandle,
    /// Access type
    pub access: ResourceAccessType,
}

/// Resource access type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ResourceAccessType {
    /// Color attachment write
    ColorAttachmentWrite,
    /// Color attachment read (for blending)
    ColorAttachmentRead,
    /// Depth stencil attachment write
    DepthStencilAttachmentWrite,
    /// Depth stencil attachment read
    DepthStencilAttachmentRead,
    /// Shader sampled read
    ShaderSampledRead,
    /// Shader storage read
    ShaderStorageRead,
    /// Shader storage write
    ShaderStorageWrite,
    /// Uniform read
    UniformRead,
    /// Transfer source
    TransferSource,
    /// Transfer destination
    TransferDestination,
    /// Vertex buffer
    VertexBuffer,
    /// Index buffer
    IndexBuffer,
    /// Indirect buffer
    IndirectBuffer,
    /// Present
    Present,
    /// Input attachment
    InputAttachment,
}

impl ResourceAccessType {
    /// Is read access
    #[inline]
    pub const fn is_read(&self) -> bool {
        matches!(
            self,
            Self::ColorAttachmentRead
                | Self::DepthStencilAttachmentRead
                | Self::ShaderSampledRead
                | Self::ShaderStorageRead
                | Self::UniformRead
                | Self::TransferSource
                | Self::VertexBuffer
                | Self::IndexBuffer
                | Self::IndirectBuffer
                | Self::InputAttachment
        )
    }

    /// Is write access
    #[inline]
    pub const fn is_write(&self) -> bool {
        matches!(
            self,
            Self::ColorAttachmentWrite
                | Self::DepthStencilAttachmentWrite
                | Self::ShaderStorageWrite
                | Self::TransferDestination
        )
    }
}

// ============================================================================
// Resources
// ============================================================================

/// Render graph resource
#[derive(Clone, Debug)]
pub enum RenderGraphResource {
    /// Texture resource
    Texture(TextureResourceInfo),
    /// Buffer resource
    Buffer(BufferResourceInfo),
}

/// External resource (imported)
#[derive(Clone, Debug)]
pub enum ExternalResource {
    /// External texture
    Texture {
        /// Handle
        handle: u64,
        /// Info
        info: TextureResourceInfo,
    },
    /// External buffer
    Buffer {
        /// Handle
        handle: u64,
        /// Info
        info: BufferResourceInfo,
    },
}

/// Texture resource info
#[derive(Clone, Debug)]
pub struct TextureResourceInfo {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Array layers
    pub array_layers: u32,
    /// Format
    pub format: TextureFormat,
    /// Sample count
    pub samples: u32,
    /// Usage flags
    pub usage: TextureUsageFlags,
    /// Debug name
    pub name: Option<String>,
}

impl TextureResourceInfo {
    /// Creates 2D texture info
    pub fn d2(width: u32, height: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            depth: 1,
            mip_levels: 1,
            array_layers: 1,
            format,
            samples: 1,
            usage: TextureUsageFlags::SAMPLED,
            name: None,
        }
    }

    /// Creates render target info
    pub fn render_target(width: u32, height: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            depth: 1,
            mip_levels: 1,
            array_layers: 1,
            format,
            samples: 1,
            usage: TextureUsageFlags::COLOR_ATTACHMENT.union(TextureUsageFlags::SAMPLED),
            name: None,
        }
    }

    /// Creates depth buffer info
    pub fn depth_buffer(width: u32, height: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            depth: 1,
            mip_levels: 1,
            array_layers: 1,
            format,
            samples: 1,
            usage: TextureUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            name: None,
        }
    }

    /// With MSAA
    pub fn with_samples(mut self, samples: u32) -> Self {
        self.samples = samples;
        self
    }

    /// With mip levels
    pub fn with_mips(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// With usage
    pub fn with_usage(mut self, usage: TextureUsageFlags) -> Self {
        self.usage = usage;
        self
    }

    /// With name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(String::from(name));
        self
    }
}

/// Buffer resource info
#[derive(Clone, Debug)]
pub struct BufferResourceInfo {
    /// Size in bytes
    pub size: u64,
    /// Usage flags
    pub usage: BufferUsageFlags,
    /// Debug name
    pub name: Option<String>,
}

impl BufferResourceInfo {
    /// Creates new buffer info
    pub fn new(size: u64, usage: BufferUsageFlags) -> Self {
        Self {
            size,
            usage,
            name: None,
        }
    }

    /// Creates uniform buffer info
    pub fn uniform(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::UNIFORM,
            name: None,
        }
    }

    /// Creates storage buffer info
    pub fn storage(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::STORAGE,
            name: None,
        }
    }

    /// With name
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(String::from(name));
        self
    }
}

// ============================================================================
// Texture Format
// ============================================================================

/// Texture format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextureFormat {
    /// Undefined
    #[default]
    Undefined = 0,
    /// RGBA8 unsigned normalized
    Rgba8Unorm = 37,
    /// RGBA8 sRGB
    Rgba8Srgb = 43,
    /// BGRA8 unsigned normalized
    Bgra8Unorm = 44,
    /// BGRA8 sRGB
    Bgra8Srgb = 50,
    /// RGBA16 float
    Rgba16Float = 97,
    /// RGBA32 float
    Rgba32Float = 109,
    /// R10G10B10A2 unsigned normalized
    Rgb10A2Unorm = 64,
    /// R11G11B10 float
    Rg11B10Float = 122,
    /// D16 unsigned normalized
    D16Unorm = 124,
    /// D32 float
    D32Float = 126,
    /// D24 S8
    D24UnormS8Uint = 129,
    /// D32 S8
    D32FloatS8Uint = 130,
    /// R8 unsigned normalized
    R8Unorm = 9,
    /// RG8 unsigned normalized
    Rg8Unorm = 16,
    /// R16 float
    R16Float = 76,
    /// RG16 float
    Rg16Float = 83,
    /// R32 float
    R32Float = 100,
    /// RG32 float
    Rg32Float = 103,
}

impl TextureFormat {
    /// Is depth format
    #[inline]
    pub const fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::D16Unorm | Self::D32Float | Self::D24UnormS8Uint | Self::D32FloatS8Uint
        )
    }

    /// Is stencil format
    #[inline]
    pub const fn is_stencil(&self) -> bool {
        matches!(self, Self::D24UnormS8Uint | Self::D32FloatS8Uint)
    }

    /// Is color format
    #[inline]
    pub const fn is_color(&self) -> bool {
        !self.is_depth()
    }
}

// ============================================================================
// Usage Flags
// ============================================================================

/// Texture usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct TextureUsageFlags(pub u32);

impl TextureUsageFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 0);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 1);
    /// Sampled
    pub const SAMPLED: Self = Self(1 << 2);
    /// Storage
    pub const STORAGE: Self = Self(1 << 3);
    /// Color attachment
    pub const COLOR_ATTACHMENT: Self = Self(1 << 4);
    /// Depth stencil attachment
    pub const DEPTH_STENCIL_ATTACHMENT: Self = Self(1 << 5);
    /// Input attachment
    pub const INPUT_ATTACHMENT: Self = Self(1 << 7);

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

/// Buffer usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct BufferUsageFlags(pub u32);

impl BufferUsageFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 0);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 1);
    /// Uniform texel buffer
    pub const UNIFORM_TEXEL: Self = Self(1 << 2);
    /// Storage texel buffer
    pub const STORAGE_TEXEL: Self = Self(1 << 3);
    /// Uniform buffer
    pub const UNIFORM: Self = Self(1 << 4);
    /// Storage buffer
    pub const STORAGE: Self = Self(1 << 5);
    /// Index buffer
    pub const INDEX: Self = Self(1 << 6);
    /// Vertex buffer
    pub const VERTEX: Self = Self(1 << 7);
    /// Indirect buffer
    pub const INDIRECT: Self = Self(1 << 8);

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
// Attachment Info
// ============================================================================

/// Color attachment info
#[derive(Clone, Copy, Debug)]
pub struct ColorAttachmentInfo {
    /// Resource handle
    pub resource: ResourceHandle,
    /// Load op
    pub load_op: LoadOp,
    /// Store op
    pub store_op: StoreOp,
    /// Clear value
    pub clear_value: ClearColor,
}

/// Depth attachment info
#[derive(Clone, Copy, Debug)]
pub struct DepthAttachmentInfo {
    /// Resource handle
    pub resource: ResourceHandle,
    /// Depth load op
    pub depth_load_op: LoadOp,
    /// Depth store op
    pub depth_store_op: StoreOp,
    /// Stencil load op
    pub stencil_load_op: LoadOp,
    /// Stencil store op
    pub stencil_store_op: StoreOp,
    /// Clear depth
    pub clear_depth: f32,
    /// Clear stencil
    pub clear_stencil: u32,
}

/// Load operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LoadOp {
    /// Load existing contents
    Load = 0,
    /// Clear to a value
    #[default]
    Clear = 1,
    /// Don't care
    DontCare = 2,
}

/// Store operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StoreOp {
    /// Store contents
    #[default]
    Store = 0,
    /// Don't care
    DontCare = 1,
}

/// Clear color
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClearColor {
    /// Red
    pub r: f32,
    /// Green
    pub g: f32,
    /// Blue
    pub b: f32,
    /// Alpha
    pub a: f32,
}

impl ClearColor {
    /// Black
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    /// White
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    /// Transparent
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
    /// Red
    pub const RED: Self = Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    /// Green
    pub const GREEN: Self = Self { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    /// Blue
    pub const BLUE: Self = Self { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    /// Cornflower blue
    pub const CORNFLOWER_BLUE: Self = Self { r: 0.392, g: 0.584, b: 0.929, a: 1.0 };

    /// Creates new clear color
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// From RGB (alpha = 1)
    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

impl Default for ClearColor {
    fn default() -> Self {
        Self::BLACK
    }
}

/// Render area
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
    #[inline]
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// From size only
    #[inline]
    pub const fn from_size(width: u32, height: u32) -> Self {
        Self { x: 0, y: 0, width, height }
    }
}

// ============================================================================
// Errors
// ============================================================================

/// Render graph error
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderGraphError {
    /// Cyclic dependency detected
    CyclicDependency,
    /// Invalid resource handle
    InvalidResource,
    /// Invalid pass handle
    InvalidPass,
    /// Resource not found
    ResourceNotFound,
    /// Pass not found
    PassNotFound,
    /// Graph not compiled
    NotCompiled,
}

impl core::fmt::Display for RenderGraphError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CyclicDependency => write!(f, "Cyclic dependency detected in render graph"),
            Self::InvalidResource => write!(f, "Invalid resource handle"),
            Self::InvalidPass => write!(f, "Invalid pass handle"),
            Self::ResourceNotFound => write!(f, "Resource not found"),
            Self::PassNotFound => write!(f, "Pass not found"),
            Self::NotCompiled => write!(f, "Render graph not compiled"),
        }
    }
}

// ============================================================================
// Render Graph Builder
// ============================================================================

/// Render graph builder for fluent API
#[derive(Debug, Default)]
pub struct RenderGraphBuilder {
    graph: RenderGraph,
}

impl RenderGraphBuilder {
    /// Creates new builder
    pub fn new() -> Self {
        Self {
            graph: RenderGraph::new(),
        }
    }

    /// Add a graphics pass
    pub fn graphics_pass(mut self, name: &str) -> PassBuilder {
        PassBuilder {
            graph_builder: self,
            pass: RenderGraphPass::graphics(name),
        }
    }

    /// Add a compute pass
    pub fn compute_pass(mut self, name: &str) -> PassBuilder {
        PassBuilder {
            graph_builder: self,
            pass: RenderGraphPass::compute(name),
        }
    }

    /// Add a transfer pass
    pub fn transfer_pass(mut self, name: &str) -> PassBuilder {
        PassBuilder {
            graph_builder: self,
            pass: RenderGraphPass::transfer(name),
        }
    }

    /// Create texture
    pub fn create_texture(&mut self, info: TextureResourceInfo) -> ResourceHandle {
        self.graph.create_texture(info)
    }

    /// Create buffer
    pub fn create_buffer(&mut self, info: BufferResourceInfo) -> ResourceHandle {
        self.graph.create_buffer(info)
    }

    /// Import texture
    pub fn import_texture(&mut self, handle: u64, info: TextureResourceInfo) -> ResourceHandle {
        self.graph.import_texture(handle, info)
    }

    /// Import swapchain
    pub fn import_swapchain(&mut self, handle: u64, width: u32, height: u32, format: TextureFormat) -> ResourceHandle {
        self.graph.import_swapchain(handle, width, height, format)
    }

    /// Build and compile
    pub fn build(mut self) -> Result<RenderGraph, RenderGraphError> {
        self.graph.compile()?;
        Ok(self.graph)
    }
}

/// Pass builder
pub struct PassBuilder {
    graph_builder: RenderGraphBuilder,
    pass: RenderGraphPass,
}

impl PassBuilder {
    /// Read resource
    pub fn read(mut self, resource: ResourceHandle, access: ResourceAccessType) -> Self {
        self.pass = self.pass.read(resource, access);
        self
    }

    /// Write resource
    pub fn write(mut self, resource: ResourceHandle, access: ResourceAccessType) -> Self {
        self.pass = self.pass.write(resource, access);
        self
    }

    /// Color attachment
    pub fn color_attachment(mut self, resource: ResourceHandle, load: LoadOp, store: StoreOp) -> Self {
        self.pass = self.pass.color_attachment(resource, load, store);
        self
    }

    /// Depth attachment
    pub fn depth_attachment(mut self, resource: ResourceHandle, load: LoadOp, store: StoreOp) -> Self {
        self.pass = self.pass.depth_attachment(resource, load, store);
        self
    }

    /// Sample texture
    pub fn sample_texture(mut self, resource: ResourceHandle) -> Self {
        self.pass = self.pass.sample_texture(resource);
        self
    }

    /// Finish pass and return to graph builder
    pub fn finish(mut self) -> RenderGraphBuilder {
        self.graph_builder.graph.add_pass(self.pass);
        self.graph_builder
    }
}
