//! Conditional rendering types
//!
//! This module provides types for GPU-driven conditional rendering.

extern crate alloc;
use alloc::vec::Vec;

use crate::buffer::BufferHandle;
use crate::query::QueryPoolHandle;

/// Conditional rendering flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ConditionalRenderingFlags(pub u32);

impl ConditionalRenderingFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Inverted condition
    pub const INVERTED: Self = Self(1 << 0);
}

impl core::ops::BitOr for ConditionalRenderingFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Conditional rendering begin info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ConditionalRenderingBeginInfo {
    /// Buffer containing the condition
    pub buffer: BufferHandle,
    /// Offset into the buffer
    pub offset: u64,
    /// Flags
    pub flags: ConditionalRenderingFlags,
}

impl ConditionalRenderingBeginInfo {
    /// Creates new conditional rendering info
    pub const fn new(buffer: BufferHandle, offset: u64) -> Self {
        Self {
            buffer,
            offset,
            flags: ConditionalRenderingFlags::NONE,
        }
    }

    /// Creates inverted conditional rendering
    pub const fn inverted(buffer: BufferHandle, offset: u64) -> Self {
        Self {
            buffer,
            offset,
            flags: ConditionalRenderingFlags::INVERTED,
        }
    }
}

/// Predicate type for conditional operations
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PredicateType {
    /// 32-bit non-zero value
    NonZero32,
    /// 64-bit non-zero value
    NonZero64,
    /// Binary (any non-zero bit)
    Binary,
}

impl Default for PredicateType {
    fn default() -> Self {
        Self::NonZero64
    }
}

/// Predicate operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PredicateOp {
    /// Clear predicate to false
    Clear,
    /// Set predicate to true
    Set,
    /// Copy from buffer
    Copy,
    /// AND with buffer value
    And,
    /// OR with buffer value
    Or,
    /// XOR with buffer value
    Xor,
}

/// Predicate handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PredicateHandle(pub u64);

impl PredicateHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Occlusion query predicate
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct OcclusionQueryPredicate {
    /// Query pool
    pub query_pool: QueryPoolHandle,
    /// First query index
    pub first_query: u32,
    /// Number of queries
    pub query_count: u32,
    /// Whether to wait for results
    pub wait: bool,
}

impl OcclusionQueryPredicate {
    /// Creates a new occlusion query predicate
    pub const fn new(pool: QueryPoolHandle, first_query: u32, query_count: u32) -> Self {
        Self {
            query_pool: pool,
            first_query,
            query_count,
            wait: true,
        }
    }

    /// Single query predicate
    pub const fn single(pool: QueryPoolHandle, query: u32) -> Self {
        Self::new(pool, query, 1)
    }
}

/// Indirect count buffer info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectCountInfo {
    /// Buffer containing draw commands
    pub command_buffer: BufferHandle,
    /// Offset to first command
    pub command_offset: u64,
    /// Buffer containing count
    pub count_buffer: BufferHandle,
    /// Offset to count value
    pub count_offset: u64,
    /// Maximum draw count
    pub max_draw_count: u32,
    /// Stride between commands
    pub stride: u32,
}

impl IndirectCountInfo {
    /// Creates new indirect count info
    pub const fn new(
        command_buffer: BufferHandle,
        count_buffer: BufferHandle,
        max_draw_count: u32,
        stride: u32,
    ) -> Self {
        Self {
            command_buffer,
            command_offset: 0,
            count_buffer,
            count_offset: 0,
            max_draw_count,
            stride,
        }
    }
}

/// Device generated commands handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IndirectCommandsLayoutHandle(pub u64);

impl IndirectCommandsLayoutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// Indirect command token type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IndirectCommandsTokenType {
    /// Shader group token
    ShaderGroup,
    /// State flags token
    StateFlags,
    /// Index buffer token
    IndexBuffer,
    /// Vertex buffer token
    VertexBuffer,
    /// Push constant token
    PushConstant,
    /// Draw indexed token
    DrawIndexed,
    /// Draw token
    Draw,
    /// Draw tasks token (mesh shader)
    DrawTasks,
    /// Draw mesh tasks token (mesh shader EXT)
    DrawMeshTasks,
    /// Dispatch token (compute)
    Dispatch,
    /// Pipeline token
    Pipeline,
}

/// Indirect commands token
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectCommandsToken {
    /// Token type
    pub token_type: IndirectCommandsTokenType,
    /// Offset in command stream
    pub offset: u32,
}

impl IndirectCommandsToken {
    /// Creates a new token
    pub const fn new(token_type: IndirectCommandsTokenType, offset: u32) -> Self {
        Self { token_type, offset }
    }

    /// Draw indexed token
    pub const fn draw_indexed(offset: u32) -> Self {
        Self::new(IndirectCommandsTokenType::DrawIndexed, offset)
    }

    /// Draw token
    pub const fn draw(offset: u32) -> Self {
        Self::new(IndirectCommandsTokenType::Draw, offset)
    }

    /// Push constant token
    pub const fn push_constant(offset: u32) -> Self {
        Self::new(IndirectCommandsTokenType::PushConstant, offset)
    }
}

/// Indirect commands layout create info
#[derive(Clone, Debug)]
pub struct IndirectCommandsLayoutCreateInfo {
    /// Pipeline bind point
    pub pipeline_bind_point: PipelineBindPoint,
    /// Tokens
    pub tokens: Vec<IndirectCommandsToken>,
    /// Stream stride
    pub stream_stride: u32,
    /// Flags
    pub flags: IndirectCommandsLayoutFlags,
}

impl IndirectCommandsLayoutCreateInfo {
    /// Creates for graphics
    pub fn graphics() -> Self {
        Self {
            pipeline_bind_point: PipelineBindPoint::Graphics,
            tokens: Vec::new(),
            stream_stride: 0,
            flags: IndirectCommandsLayoutFlags::NONE,
        }
    }

    /// Creates for compute
    pub fn compute() -> Self {
        Self {
            pipeline_bind_point: PipelineBindPoint::Compute,
            tokens: Vec::new(),
            stream_stride: 0,
            flags: IndirectCommandsLayoutFlags::NONE,
        }
    }

    /// Adds a token
    pub fn add_token(mut self, token: IndirectCommandsToken) -> Self {
        self.tokens.push(token);
        self
    }

    /// Sets stride
    pub fn with_stride(mut self, stride: u32) -> Self {
        self.stream_stride = stride;
        self
    }
}

/// Pipeline bind point
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PipelineBindPoint {
    /// Graphics pipeline
    Graphics,
    /// Compute pipeline
    Compute,
    /// Ray tracing pipeline
    RayTracing,
}

impl Default for PipelineBindPoint {
    fn default() -> Self {
        Self::Graphics
    }
}

/// Indirect commands layout flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct IndirectCommandsLayoutFlags(pub u32);

impl IndirectCommandsLayoutFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Unordered sequences
    pub const UNORDERED_SEQUENCES: Self = Self(1 << 0);
    /// Explicit preprocess
    pub const EXPLICIT_PREPROCESS: Self = Self(1 << 1);
}

/// Generate commands info
#[derive(Clone, Debug)]
pub struct GenerateCommandsInfo {
    /// Pipeline bind point
    pub pipeline_bind_point: PipelineBindPoint,
    /// Pipeline handle
    pub pipeline: u64,
    /// Indirect commands layout
    pub indirect_commands_layout: IndirectCommandsLayoutHandle,
    /// Stream count
    pub stream_count: u32,
    /// Input streams
    pub streams: Vec<IndirectCommandsStream>,
    /// Sequences count
    pub sequences_count: u32,
    /// Preprocess buffer
    pub preprocess_buffer: BufferHandle,
    /// Preprocess offset
    pub preprocess_offset: u64,
    /// Preprocess size
    pub preprocess_size: u64,
    /// Sequences count buffer (optional)
    pub sequences_count_buffer: BufferHandle,
    /// Sequences count offset
    pub sequences_count_offset: u64,
}

/// Indirect commands stream
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectCommandsStream {
    /// Buffer
    pub buffer: BufferHandle,
    /// Offset
    pub offset: u64,
}

impl IndirectCommandsStream {
    /// Creates a new stream
    pub const fn new(buffer: BufferHandle, offset: u64) -> Self {
        Self { buffer, offset }
    }
}

/// Indirect execution set handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IndirectExecutionSetHandle(pub u64);

impl IndirectExecutionSetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// GPU culling parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCullingParams {
    /// View-projection matrix (16 floats)
    pub view_proj: [f32; 16],
    /// Frustum planes (6 planes, 4 floats each)
    pub frustum_planes: [[f32; 4]; 6],
    /// Camera position
    pub camera_pos: [f32; 3],
    /// Padding
    pub _padding: f32,
    /// Near plane distance
    pub near_plane: f32,
    /// Far plane distance
    pub far_plane: f32,
    /// Pyramid width (for Hi-Z)
    pub pyramid_width: f32,
    /// Pyramid height
    pub pyramid_height: f32,
}

/// Draw command for indirect rendering
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawCommand {
    /// Instance data offset
    pub instance_offset: u32,
    /// Instance count
    pub instance_count: u32,
    /// Mesh index
    pub mesh_index: u32,
    /// Material index
    pub material_index: u32,
}

impl DrawCommand {
    /// Creates a new draw command
    pub const fn new(mesh_index: u32, material_index: u32, instance_count: u32) -> Self {
        Self {
            instance_offset: 0,
            instance_count,
            mesh_index,
            material_index,
        }
    }
}

/// Indirect draw indexed command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndexedIndirectCommand {
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

impl DrawIndexedIndirectCommand {
    /// Creates a new command
    pub const fn new(
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) -> Self {
        Self {
            index_count,
            instance_count,
            first_index,
            vertex_offset,
            first_instance,
        }
    }
}

/// Indirect draw command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawIndirectCommand {
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

impl DrawIndirectCommand {
    /// Creates a new command
    pub const fn new(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }
}

/// Dispatch indirect command
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DispatchIndirectCommand {
    /// Workgroup count X
    pub x: u32,
    /// Workgroup count Y
    pub y: u32,
    /// Workgroup count Z
    pub z: u32,
}

impl DispatchIndirectCommand {
    /// Creates a new command
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
}
