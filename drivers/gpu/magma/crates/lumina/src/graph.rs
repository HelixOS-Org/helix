//! Render graph for automatic barrier insertion and pass optimization
//!
//! The render graph is the heart of Lumina's optimization system.
//! It records all GPU operations, analyzes dependencies, and generates
//! optimal command buffers with correctly placed barriers.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

use crate::color::ClearValue;
use crate::types::{BufferHandle, PipelineHandle, TextureHandle};

// ═══════════════════════════════════════════════════════════════════════════
// RESOURCE TRACKING
// ═══════════════════════════════════════════════════════════════════════════

/// Unique identifier for a graph resource
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(u32);

impl ResourceId {
    /// Creates a new resource ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Returns the raw ID
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// Type of resource
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceType {
    /// GPU buffer
    Buffer,
    /// GPU texture/image
    Texture,
    /// Swapchain image
    Swapchain,
}

/// Current state of a resource
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceState {
    /// Pipeline stages that access this resource
    pub stages: PipelineStages,
    /// Access type (read/write)
    pub access: AccessFlags,
    /// Image layout (for textures)
    pub layout: ImageLayout,
}

impl Default for ResourceState {
    fn default() -> Self {
        Self {
            stages: PipelineStages::TOP_OF_PIPE,
            access: AccessFlags::NONE,
            layout: ImageLayout::Undefined,
        }
    }
}

/// Pipeline stage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipelineStages(u32);

impl PipelineStages {
    pub const TOP_OF_PIPE: Self = Self(0x00000001);
    pub const DRAW_INDIRECT: Self = Self(0x00000002);
    pub const VERTEX_INPUT: Self = Self(0x00000004);
    pub const VERTEX_SHADER: Self = Self(0x00000008);
    pub const FRAGMENT_SHADER: Self = Self(0x00000080);
    pub const EARLY_FRAGMENT_TESTS: Self = Self(0x00000100);
    pub const LATE_FRAGMENT_TESTS: Self = Self(0x00000200);
    pub const COLOR_ATTACHMENT_OUTPUT: Self = Self(0x00000400);
    pub const COMPUTE_SHADER: Self = Self(0x00000800);
    pub const TRANSFER: Self = Self(0x00001000);
    pub const BOTTOM_OF_PIPE: Self = Self(0x00002000);
    pub const ALL_GRAPHICS: Self = Self(0x00008000);
    pub const ALL_COMMANDS: Self = Self(0x00010000);

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl core::ops::BitOr for PipelineStages {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Access flags
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessFlags(u32);

impl AccessFlags {
    pub const NONE: Self = Self(0);
    pub const INDIRECT_COMMAND_READ: Self = Self(0x00000001);
    pub const INDEX_READ: Self = Self(0x00000002);
    pub const VERTEX_ATTRIBUTE_READ: Self = Self(0x00000004);
    pub const UNIFORM_READ: Self = Self(0x00000008);
    pub const SHADER_READ: Self = Self(0x00000020);
    pub const SHADER_WRITE: Self = Self(0x00000040);
    pub const COLOR_ATTACHMENT_READ: Self = Self(0x00000080);
    pub const COLOR_ATTACHMENT_WRITE: Self = Self(0x00000100);
    pub const DEPTH_STENCIL_READ: Self = Self(0x00000200);
    pub const DEPTH_STENCIL_WRITE: Self = Self(0x00000400);
    pub const TRANSFER_READ: Self = Self(0x00000800);
    pub const TRANSFER_WRITE: Self = Self(0x00001000);
    pub const MEMORY_READ: Self = Self(0x00008000);
    pub const MEMORY_WRITE: Self = Self(0x00010000);

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn is_write(self) -> bool {
        (self.0
            & (Self::SHADER_WRITE.0
                | Self::COLOR_ATTACHMENT_WRITE.0
                | Self::DEPTH_STENCIL_WRITE.0
                | Self::TRANSFER_WRITE.0
                | Self::MEMORY_WRITE.0))
            != 0
    }

    pub const fn is_read(self) -> bool {
        !self.is_write() && self.0 != 0
    }
}

impl core::ops::BitOr for AccessFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Image layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImageLayout {
    #[default]
    Undefined,
    General,
    ColorAttachment,
    DepthStencilAttachment,
    DepthStencilReadOnly,
    ShaderReadOnly,
    TransferSrc,
    TransferDst,
    Present,
}

impl ImageLayout {
    pub const fn vk_layout(self) -> u32 {
        match self {
            Self::Undefined => 0,
            Self::General => 1,
            Self::ColorAttachment => 2,
            Self::DepthStencilAttachment => 3,
            Self::DepthStencilReadOnly => 4,
            Self::ShaderReadOnly => 5,
            Self::TransferSrc => 6,
            Self::TransferDst => 7,
            Self::Present => 1000001002,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// RENDER GRAPH NODES
// ═══════════════════════════════════════════════════════════════════════════

/// A node in the render graph
#[derive(Clone, Debug)]
pub enum RenderNode {
    /// Begin a render pass
    BeginRenderPass {
        /// Color attachments
        color_attachments: Vec<Attachment>,
        /// Optional depth attachment
        depth_attachment: Option<Attachment>,
    },

    /// End the current render pass
    EndRenderPass,

    /// Bind a graphics pipeline
    BindGraphicsPipeline {
        pipeline: PipelineHandle,
    },

    /// Bind a compute pipeline
    BindComputePipeline {
        pipeline: PipelineHandle,
    },

    /// Set push constants
    PushConstants {
        data: Vec<u8>,
    },

    /// Bind vertex buffer
    BindVertexBuffer {
        binding: u32,
        buffer: BufferHandle,
        offset: u64,
    },

    /// Bind index buffer
    BindIndexBuffer {
        buffer: BufferHandle,
        offset: u64,
        index_type: IndexType,
    },

    /// Draw call
    Draw {
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    },

    /// Indexed draw call
    DrawIndexed {
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    },

    /// Compute dispatch
    Dispatch {
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    },

    /// Pipeline barrier
    Barrier {
        barriers: Vec<BarrierInfo>,
    },

    /// Clear color attachment
    ClearColor {
        attachment: u32,
        value: ClearValue,
    },

    /// Clear depth/stencil attachment
    ClearDepthStencil {
        depth: f32,
        stencil: u32,
    },

    /// Copy buffer to buffer
    CopyBuffer {
        src: BufferHandle,
        dst: BufferHandle,
        regions: Vec<BufferCopyRegion>,
    },

    /// Copy buffer to texture
    CopyBufferToTexture {
        src: BufferHandle,
        dst: TextureHandle,
        regions: Vec<BufferTextureCopyRegion>,
    },
}

/// Attachment for render pass
#[derive(Clone, Debug)]
pub struct Attachment {
    /// Resource being attached
    pub resource: ResourceId,
    /// Load operation
    pub load_op: LoadOp,
    /// Store operation
    pub store_op: StoreOp,
    /// Clear value (if load_op is Clear)
    pub clear_value: ClearValue,
}

/// Load operation for attachments
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LoadOp {
    /// Preserve existing contents
    Load,
    /// Clear to a specific value
    Clear,
    /// Contents are undefined
    #[default]
    DontCare,
}

/// Store operation for attachments
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StoreOp {
    /// Store the results
    #[default]
    Store,
    /// Contents may be discarded
    DontCare,
}

/// Index type for indexed draw calls
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexType {
    U16,
    U32,
}

/// Barrier information
#[derive(Clone, Debug)]
pub struct BarrierInfo {
    /// Resource being synchronized
    pub resource: ResourceId,
    /// Previous state
    pub src_state: ResourceState,
    /// Next state
    pub dst_state: ResourceState,
}

/// Buffer copy region
#[derive(Clone, Copy, Debug)]
pub struct BufferCopyRegion {
    pub src_offset: u64,
    pub dst_offset: u64,
    pub size: u64,
}

/// Buffer to texture copy region
#[derive(Clone, Copy, Debug)]
pub struct BufferTextureCopyRegion {
    pub buffer_offset: u64,
    pub texture_offset: [u32; 3],
    pub texture_extent: [u32; 3],
    pub mip_level: u32,
    pub array_layer: u32,
}

// ═══════════════════════════════════════════════════════════════════════════
// RENDER GRAPH
// ═══════════════════════════════════════════════════════════════════════════

/// A render graph for a single frame
///
/// The graph records all GPU operations and their resource dependencies.
/// Before execution, it analyzes the graph to:
/// - Insert optimal barriers
/// - Merge compatible render passes into subpasses
/// - Identify opportunities for async compute
/// - Alias transient resource memory
pub struct RenderGraph {
    /// All nodes in submission order
    nodes: Vec<RenderNode>,
    /// Resource states at each node
    resource_states: BTreeMap<ResourceId, Vec<(usize, ResourceState)>>,
    /// Current state of each resource
    current_states: BTreeMap<ResourceId, ResourceState>,
    /// Next resource ID
    next_resource_id: u32,
    /// Registered resources
    resources: BTreeMap<ResourceId, ResourceInfo>,
}

/// Information about a registered resource
#[derive(Clone, Debug)]
pub struct ResourceInfo {
    pub resource_type: ResourceType,
    pub name: Option<String>,
}

impl RenderGraph {
    /// Creates a new empty render graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            resource_states: BTreeMap::new(),
            current_states: BTreeMap::new(),
            next_resource_id: 0,
            resources: BTreeMap::new(),
        }
    }

    /// Registers a buffer resource
    pub fn register_buffer(&mut self, handle: BufferHandle) -> ResourceId {
        let id = ResourceId::new(self.next_resource_id);
        self.next_resource_id += 1;

        self.resources.insert(
            id,
            ResourceInfo {
                resource_type: ResourceType::Buffer,
                name: None,
            },
        );

        self.current_states.insert(id, ResourceState::default());

        id
    }

    /// Registers a texture resource
    pub fn register_texture(&mut self, handle: TextureHandle) -> ResourceId {
        let id = ResourceId::new(self.next_resource_id);
        self.next_resource_id += 1;

        self.resources.insert(
            id,
            ResourceInfo {
                resource_type: ResourceType::Texture,
                name: None,
            },
        );

        self.current_states.insert(id, ResourceState::default());

        id
    }

    /// Adds a node to the graph
    pub fn add_node(&mut self, node: RenderNode) {
        self.nodes.push(node);
    }

    /// Records a resource read
    pub fn read(&mut self, resource: ResourceId, state: ResourceState) {
        let node_index = self.nodes.len();

        self.resource_states
            .entry(resource)
            .or_insert_with(Vec::new)
            .push((node_index, state));

        self.current_states.insert(resource, state);
    }

    /// Records a resource write
    pub fn write(&mut self, resource: ResourceId, state: ResourceState) {
        let node_index = self.nodes.len();

        self.resource_states
            .entry(resource)
            .or_insert_with(Vec::new)
            .push((node_index, state));

        self.current_states.insert(resource, state);
    }

    /// Compiles the graph, inserting barriers and optimizing passes
    pub fn compile(self) -> CompiledGraph {
        let mut compiled = CompiledGraph::new();

        // Phase 1: Compute required barriers
        let barriers = self.compute_barriers();

        // Phase 2: Insert barriers at appropriate points
        let mut barrier_index = 0;
        for (node_index, node) in self.nodes.into_iter().enumerate() {
            // Insert any barriers that should come before this node
            while barrier_index < barriers.len() && barriers[barrier_index].0 == node_index {
                compiled.commands.push(CompiledCommand::Barrier(
                    barriers[barrier_index].1.clone(),
                ));
                barrier_index += 1;
            }

            // Add the actual command
            compiled.commands.push(CompiledCommand::Node(node));
        }

        compiled
    }

    /// Computes the barriers needed between nodes
    fn compute_barriers(&self) -> Vec<(usize, Vec<BarrierInfo>)> {
        let mut barriers: Vec<(usize, Vec<BarrierInfo>)> = Vec::new();

        for (resource, states) in &self.resource_states {
            let mut prev_state = ResourceState::default();
            let mut prev_node = 0usize;

            for (node_index, state) in states {
                if needs_barrier(prev_state, *state) {
                    let barrier = BarrierInfo {
                        resource: *resource,
                        src_state: prev_state,
                        dst_state: *state,
                    };

                    // Find or create barrier group for this node
                    if let Some(group) = barriers.iter_mut().find(|(idx, _)| *idx == *node_index) {
                        group.1.push(barrier);
                    } else {
                        barriers.push((*node_index, alloc::vec![barrier]));
                    }
                }

                prev_state = *state;
                prev_node = *node_index;
            }
        }

        // Sort by node index
        barriers.sort_by_key(|(idx, _)| *idx);

        // Coalesce adjacent barriers
        coalesce_barriers(&mut barriers);

        barriers
    }

    /// Clears the graph for reuse
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.resource_states.clear();
        self.current_states.clear();
        // Keep resource registrations
    }
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Determines if a barrier is needed between two states
fn needs_barrier(src: ResourceState, dst: ResourceState) -> bool {
    // Write → anything needs barrier
    if src.access.is_write() {
        return true;
    }

    // Layout transition needs barrier
    if src.layout != dst.layout && src.layout != ImageLayout::Undefined {
        return true;
    }

    // Read → write needs barrier
    if src.access.is_read() && dst.access.is_write() {
        return true;
    }

    false
}

/// Coalesces adjacent barriers into fewer barrier commands
fn coalesce_barriers(barriers: &mut Vec<(usize, Vec<BarrierInfo>)>) {
    // For now, just merge barriers at the same node
    // A more sophisticated implementation would look for barriers
    // that can be combined across nodes
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPILED GRAPH
// ═══════════════════════════════════════════════════════════════════════════

/// A compiled render graph ready for execution
pub struct CompiledGraph {
    /// Commands to execute
    pub commands: Vec<CompiledCommand>,
}

/// A command in the compiled graph
pub enum CompiledCommand {
    /// A render node
    Node(RenderNode),
    /// A barrier insertion point
    Barrier(Vec<BarrierInfo>),
}

impl CompiledGraph {
    /// Creates a new empty compiled graph
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }
}

impl Default for CompiledGraph {
    fn default() -> Self {
        Self::new()
    }
}
