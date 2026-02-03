//! Indirect Arguments Types for Lumina
//!
//! This module provides indirect draw and dispatch
//! argument structures for GPU-driven rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Indirect Handles
// ============================================================================

/// Indirect argument buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IndirectBufferHandle(pub u64);

impl IndirectBufferHandle {
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

impl Default for IndirectBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Indirect command signature handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CommandSignatureHandle(pub u64);

impl CommandSignatureHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for CommandSignatureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Indirect builder handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IndirectBuilderHandle(pub u64);

impl IndirectBuilderHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for IndirectBuilderHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Indirect Buffer Creation
// ============================================================================

/// Indirect buffer create info
#[derive(Clone, Debug)]
pub struct IndirectBufferCreateInfo {
    /// Name
    pub name: String,
    /// Max commands
    pub max_commands: u32,
    /// Command type
    pub command_type: IndirectCommandType,
    /// Features
    pub features: IndirectBufferFeatures,
    /// Initial count (0 = dynamic count)
    pub initial_count: u32,
}

impl IndirectBufferCreateInfo {
    /// Creates new info
    pub fn new(command_type: IndirectCommandType, max_commands: u32) -> Self {
        Self {
            name: String::new(),
            max_commands,
            command_type,
            features: IndirectBufferFeatures::empty(),
            initial_count: 0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With features
    pub fn with_features(mut self, features: IndirectBufferFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With initial count
    pub fn with_count(mut self, count: u32) -> Self {
        self.initial_count = count;
        self
    }

    /// Draw indirect buffer
    pub fn draw(max_commands: u32) -> Self {
        Self::new(IndirectCommandType::Draw, max_commands)
    }

    /// Draw indexed indirect buffer
    pub fn draw_indexed(max_commands: u32) -> Self {
        Self::new(IndirectCommandType::DrawIndexed, max_commands)
    }

    /// Dispatch indirect buffer
    pub fn dispatch(max_commands: u32) -> Self {
        Self::new(IndirectCommandType::Dispatch, max_commands)
    }

    /// Mesh draw indirect buffer
    pub fn mesh_draw(max_commands: u32) -> Self {
        Self::new(IndirectCommandType::MeshDraw, max_commands)
    }

    /// GPU-driven preset
    pub fn gpu_driven(max_commands: u32) -> Self {
        Self::draw_indexed(max_commands)
            .with_features(IndirectBufferFeatures::COUNT_BUFFER | IndirectBufferFeatures::GPU_WRITABLE)
    }
}

impl Default for IndirectBufferCreateInfo {
    fn default() -> Self {
        Self::draw(1024)
    }
}

/// Indirect command type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum IndirectCommandType {
    /// Draw (non-indexed)
    Draw = 0,
    /// Draw indexed
    #[default]
    DrawIndexed = 1,
    /// Dispatch compute
    Dispatch = 2,
    /// Dispatch rays
    DispatchRays = 3,
    /// Mesh draw
    MeshDraw = 4,
    /// Multi-draw indirect count
    MultiDrawCount = 5,
    /// Custom command
    Custom = 100,
}

impl IndirectCommandType {
    /// Command size (bytes)
    pub const fn command_size(&self) -> u32 {
        match self {
            Self::Draw => 16,           // 4 * u32
            Self::DrawIndexed => 20,    // 5 * u32
            Self::Dispatch => 12,       // 3 * u32
            Self::DispatchRays => 24,   // 3 * u64
            Self::MeshDraw => 12,       // 3 * u32
            Self::MultiDrawCount => 8,  // 2 * u32
            Self::Custom => 0,
        }
    }
}

bitflags::bitflags! {
    /// Indirect buffer features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct IndirectBufferFeatures: u32 {
        /// None
        const NONE = 0;
        /// Has count buffer
        const COUNT_BUFFER = 1 << 0;
        /// GPU writable
        const GPU_WRITABLE = 1 << 1;
        /// CPU readable
        const CPU_READABLE = 1 << 2;
        /// Pre-validated
        const PRE_VALIDATED = 1 << 3;
        /// Persistent
        const PERSISTENT = 1 << 4;
    }
}

// ============================================================================
// Draw Arguments
// ============================================================================

/// Draw indirect arguments
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndirectArgs {
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

impl DrawIndirectArgs {
    /// Creates new args
    pub const fn new(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }

    /// Single draw
    pub const fn single(vertex_count: u32) -> Self {
        Self::new(vertex_count, 1)
    }

    /// Instanced draw
    pub const fn instanced(vertex_count: u32, instances: u32) -> Self {
        Self::new(vertex_count, instances)
    }

    /// With first vertex
    pub const fn with_first_vertex(mut self, first: u32) -> Self {
        self.first_vertex = first;
        self
    }

    /// With first instance
    pub const fn with_first_instance(mut self, first: u32) -> Self {
        self.first_instance = first;
        self
    }

    /// Total vertices drawn
    pub const fn total_vertices(&self) -> u32 {
        self.vertex_count * self.instance_count
    }
}

/// Draw indexed indirect arguments
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndexedIndirectArgs {
    /// Index count
    pub index_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First index
    pub first_index: u32,
    /// Vertex offset
    pub vertex_offset: i32,
    /// First instance
    pub first_instance: u32,
}

impl DrawIndexedIndirectArgs {
    /// Creates new args
    pub const fn new(index_count: u32, instance_count: u32) -> Self {
        Self {
            index_count,
            instance_count,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }

    /// Single draw
    pub const fn single(index_count: u32) -> Self {
        Self::new(index_count, 1)
    }

    /// Instanced draw
    pub const fn instanced(index_count: u32, instances: u32) -> Self {
        Self::new(index_count, instances)
    }

    /// With first index
    pub const fn with_first_index(mut self, first: u32) -> Self {
        self.first_index = first;
        self
    }

    /// With vertex offset
    pub const fn with_vertex_offset(mut self, offset: i32) -> Self {
        self.vertex_offset = offset;
        self
    }

    /// With first instance
    pub const fn with_first_instance(mut self, first: u32) -> Self {
        self.first_instance = first;
        self
    }

    /// Total indices drawn
    pub const fn total_indices(&self) -> u32 {
        self.index_count * self.instance_count
    }
}

// ============================================================================
// Dispatch Arguments
// ============================================================================

/// Dispatch indirect arguments
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DispatchIndirectArgs {
    /// X dimension
    pub x: u32,
    /// Y dimension
    pub y: u32,
    /// Z dimension
    pub z: u32,
}

impl DispatchIndirectArgs {
    /// Creates new args
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    /// 1D dispatch
    pub const fn d1(x: u32) -> Self {
        Self::new(x, 1, 1)
    }

    /// 2D dispatch
    pub const fn d2(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }

    /// 3D dispatch
    pub const fn d3(x: u32, y: u32, z: u32) -> Self {
        Self::new(x, y, z)
    }

    /// From total threads and workgroup size
    pub fn from_threads(total_x: u32, total_y: u32, total_z: u32, wg_x: u32, wg_y: u32, wg_z: u32) -> Self {
        Self {
            x: (total_x + wg_x - 1) / wg_x,
            y: (total_y + wg_y - 1) / wg_y,
            z: (total_z + wg_z - 1) / wg_z,
        }
    }

    /// Total workgroups
    pub const fn total_workgroups(&self) -> u32 {
        self.x * self.y * self.z
    }
}

/// Dispatch rays indirect arguments
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DispatchRaysIndirectArgs {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
    /// Padding
    pub _pad: u32,
}

impl DispatchRaysIndirectArgs {
    /// Creates new args
    pub const fn new(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
            _pad: 0,
        }
    }

    /// 2D dispatch rays
    pub const fn d2(width: u32, height: u32) -> Self {
        Self::new(width, height, 1)
    }

    /// Full screen
    pub fn fullscreen(width: u32, height: u32) -> Self {
        Self::d2(width, height)
    }

    /// Total rays
    pub const fn total_rays(&self) -> u64 {
        self.width as u64 * self.height as u64 * self.depth as u64
    }
}

// ============================================================================
// Mesh Shading Arguments
// ============================================================================

/// Mesh draw indirect arguments
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshDrawIndirectArgs {
    /// Task/amplification shader group count X
    pub group_count_x: u32,
    /// Task/amplification shader group count Y
    pub group_count_y: u32,
    /// Task/amplification shader group count Z
    pub group_count_z: u32,
}

impl MeshDrawIndirectArgs {
    /// Creates new args
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            group_count_x: x,
            group_count_y: y,
            group_count_z: z,
        }
    }

    /// 1D mesh draw
    pub const fn d1(x: u32) -> Self {
        Self::new(x, 1, 1)
    }

    /// Total task groups
    pub const fn total_groups(&self) -> u32 {
        self.group_count_x * self.group_count_y * self.group_count_z
    }
}

/// Mesh draw indexed indirect arguments (extended)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MeshDrawIndexedIndirectArgs {
    /// Group count X
    pub group_count_x: u32,
    /// Group count Y
    pub group_count_y: u32,
    /// Group count Z
    pub group_count_z: u32,
    /// First task
    pub first_task: u32,
}

// ============================================================================
// Multi-Draw Count
// ============================================================================

/// Multi-draw indirect count arguments
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MultiDrawCountArgs {
    /// Draw count (actual number of draws)
    pub draw_count: u32,
    /// Stride between draw commands
    pub stride: u32,
}

impl MultiDrawCountArgs {
    /// Creates new args
    pub const fn new(draw_count: u32, stride: u32) -> Self {
        Self { draw_count, stride }
    }

    /// For draw commands
    pub const fn draw(count: u32) -> Self {
        Self::new(count, core::mem::size_of::<DrawIndirectArgs>() as u32)
    }

    /// For draw indexed commands
    pub const fn draw_indexed(count: u32) -> Self {
        Self::new(count, core::mem::size_of::<DrawIndexedIndirectArgs>() as u32)
    }
}

// ============================================================================
// Command Signature
// ============================================================================

/// Command signature create info
#[derive(Clone, Debug)]
pub struct CommandSignatureCreateInfo {
    /// Name
    pub name: String,
    /// Arguments
    pub arguments: Vec<IndirectArgumentDesc>,
    /// Stride (0 = auto)
    pub stride: u32,
}

impl CommandSignatureCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            arguments: Vec::new(),
            stride: 0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add argument
    pub fn add_argument(mut self, arg: IndirectArgumentDesc) -> Self {
        self.arguments.push(arg);
        self
    }

    /// With stride
    pub fn with_stride(mut self, stride: u32) -> Self {
        self.stride = stride;
        self
    }

    /// Simple draw
    pub fn simple_draw() -> Self {
        Self::new()
            .add_argument(IndirectArgumentDesc::draw())
    }

    /// Simple draw indexed
    pub fn simple_draw_indexed() -> Self {
        Self::new()
            .add_argument(IndirectArgumentDesc::draw_indexed())
    }

    /// Simple dispatch
    pub fn simple_dispatch() -> Self {
        Self::new()
            .add_argument(IndirectArgumentDesc::dispatch())
    }

    /// With vertex buffer
    pub fn with_vertex_buffer(self, slot: u32) -> Self {
        self.add_argument(IndirectArgumentDesc::vertex_buffer(slot))
    }

    /// With index buffer
    pub fn with_index_buffer(self) -> Self {
        self.add_argument(IndirectArgumentDesc::index_buffer())
    }

    /// With push constants
    pub fn with_push_constants(self, offset: u32, size: u32) -> Self {
        self.add_argument(IndirectArgumentDesc::push_constants(offset, size))
    }
}

impl Default for CommandSignatureCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Indirect argument descriptor
#[derive(Clone, Debug)]
pub struct IndirectArgumentDesc {
    /// Argument type
    pub arg_type: IndirectArgumentType,
    /// Slot or offset
    pub slot: u32,
    /// Size (for constants)
    pub size: u32,
}

impl IndirectArgumentDesc {
    /// Creates new desc
    pub fn new(arg_type: IndirectArgumentType) -> Self {
        Self {
            arg_type,
            slot: 0,
            size: 0,
        }
    }

    /// Draw argument
    pub fn draw() -> Self {
        Self::new(IndirectArgumentType::Draw)
    }

    /// Draw indexed argument
    pub fn draw_indexed() -> Self {
        Self::new(IndirectArgumentType::DrawIndexed)
    }

    /// Dispatch argument
    pub fn dispatch() -> Self {
        Self::new(IndirectArgumentType::Dispatch)
    }

    /// Mesh draw argument
    pub fn mesh_draw() -> Self {
        Self::new(IndirectArgumentType::MeshDraw)
    }

    /// Vertex buffer argument
    pub fn vertex_buffer(slot: u32) -> Self {
        Self {
            arg_type: IndirectArgumentType::VertexBuffer,
            slot,
            size: 0,
        }
    }

    /// Index buffer argument
    pub fn index_buffer() -> Self {
        Self::new(IndirectArgumentType::IndexBuffer)
    }

    /// Push constants argument
    pub fn push_constants(offset: u32, size: u32) -> Self {
        Self {
            arg_type: IndirectArgumentType::PushConstants,
            slot: offset,
            size,
        }
    }

    /// Descriptor table argument
    pub fn descriptor_table(root_index: u32) -> Self {
        Self {
            arg_type: IndirectArgumentType::DescriptorTable,
            slot: root_index,
            size: 0,
        }
    }
}

/// Indirect argument type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum IndirectArgumentType {
    /// Draw
    #[default]
    Draw = 0,
    /// Draw indexed
    DrawIndexed = 1,
    /// Dispatch
    Dispatch = 2,
    /// Mesh draw
    MeshDraw = 3,
    /// Dispatch rays
    DispatchRays = 4,
    /// Vertex buffer
    VertexBuffer = 10,
    /// Index buffer
    IndexBuffer = 11,
    /// Push constants
    PushConstants = 12,
    /// Descriptor table
    DescriptorTable = 13,
    /// Constant buffer view
    ConstantBufferView = 14,
    /// Shader resource view
    ShaderResourceView = 15,
    /// Unordered access view
    UnorderedAccessView = 16,
}

// ============================================================================
// Indirect Builder
// ============================================================================

/// Indirect command builder
#[derive(Clone, Debug, Default)]
pub struct IndirectCommandBuilder {
    /// Commands
    pub draw_commands: Vec<DrawIndexedIndirectArgs>,
    /// Dispatch commands
    pub dispatch_commands: Vec<DispatchIndirectArgs>,
    /// Mesh commands
    pub mesh_commands: Vec<MeshDrawIndirectArgs>,
}

impl IndirectCommandBuilder {
    /// Creates new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Add draw indexed command
    pub fn add_draw_indexed(&mut self, args: DrawIndexedIndirectArgs) -> &mut Self {
        self.draw_commands.push(args);
        self
    }

    /// Add dispatch command
    pub fn add_dispatch(&mut self, args: DispatchIndirectArgs) -> &mut Self {
        self.dispatch_commands.push(args);
        self
    }

    /// Add mesh draw command
    pub fn add_mesh_draw(&mut self, args: MeshDrawIndirectArgs) -> &mut Self {
        self.mesh_commands.push(args);
        self
    }

    /// Draw command count
    pub fn draw_count(&self) -> u32 {
        self.draw_commands.len() as u32
    }

    /// Dispatch command count
    pub fn dispatch_count(&self) -> u32 {
        self.dispatch_commands.len() as u32
    }

    /// Clear all commands
    pub fn clear(&mut self) {
        self.draw_commands.clear();
        self.dispatch_commands.clear();
        self.mesh_commands.clear();
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Indirect buffer statistics
#[derive(Clone, Debug, Default)]
pub struct IndirectBufferStats {
    /// Total commands
    pub total_commands: u32,
    /// Max commands
    pub max_commands: u32,
    /// Buffer size (bytes)
    pub buffer_size: u64,
    /// Commands written this frame
    pub commands_this_frame: u32,
    /// Commands executed this frame
    pub commands_executed: u32,
    /// Draw calls saved (batching efficiency)
    pub draw_calls_saved: u32,
}

impl IndirectBufferStats {
    /// Usage ratio
    pub fn usage_ratio(&self) -> f32 {
        if self.max_commands == 0 {
            return 0.0;
        }
        self.total_commands as f32 / self.max_commands as f32
    }
}
