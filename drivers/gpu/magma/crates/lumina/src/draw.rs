//! Draw command types
//!
//! This module provides types for issuing draw commands.

use crate::types::{BufferHandle, PipelineHandle};
use crate::bind_group::BindGroupHandle;

/// Draw command for non-indexed rendering
#[derive(Clone, Copy, Debug)]
pub struct DrawCommand {
    /// Number of vertices to draw
    pub vertex_count: u32,
    /// Number of instances to draw
    pub instance_count: u32,
    /// First vertex index
    pub first_vertex: u32,
    /// First instance index
    pub first_instance: u32,
}

impl DrawCommand {
    /// Creates a simple draw command
    pub const fn new(vertex_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        }
    }

    /// Creates an instanced draw command
    pub const fn instanced(vertex_count: u32, instance_count: u32) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex: 0,
            first_instance: 0,
        }
    }

    /// Sets the first vertex
    pub const fn with_first_vertex(mut self, first: u32) -> Self {
        self.first_vertex = first;
        self
    }

    /// Sets the first instance
    pub const fn with_first_instance(mut self, first: u32) -> Self {
        self.first_instance = first;
        self
    }
}

/// Draw command for indexed rendering
#[derive(Clone, Copy, Debug)]
pub struct DrawIndexedCommand {
    /// Number of indices to draw
    pub index_count: u32,
    /// Number of instances to draw
    pub instance_count: u32,
    /// First index in the index buffer
    pub first_index: u32,
    /// Value added to each index before accessing vertices
    pub vertex_offset: i32,
    /// First instance index
    pub first_instance: u32,
}

impl DrawIndexedCommand {
    /// Creates a simple indexed draw command
    pub const fn new(index_count: u32) -> Self {
        Self {
            index_count,
            instance_count: 1,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }

    /// Creates an instanced indexed draw command
    pub const fn instanced(index_count: u32, instance_count: u32) -> Self {
        Self {
            index_count,
            instance_count,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }

    /// Sets the first index
    pub const fn with_first_index(mut self, first: u32) -> Self {
        self.first_index = first;
        self
    }

    /// Sets the vertex offset
    pub const fn with_vertex_offset(mut self, offset: i32) -> Self {
        self.vertex_offset = offset;
        self
    }

    /// Sets the first instance
    pub const fn with_first_instance(mut self, first: u32) -> Self {
        self.first_instance = first;
        self
    }
}

/// Indirect draw command (stored in GPU buffer)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DrawIndirectCommand {
    /// Number of vertices to draw
    pub vertex_count: u32,
    /// Number of instances to draw
    pub instance_count: u32,
    /// First vertex index
    pub first_vertex: u32,
    /// First instance index
    pub first_instance: u32,
}

/// Indirect indexed draw command (stored in GPU buffer)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DrawIndexedIndirectCommand {
    /// Number of indices to draw
    pub index_count: u32,
    /// Number of instances to draw
    pub instance_count: u32,
    /// First index
    pub first_index: u32,
    /// Vertex offset
    pub vertex_offset: i32,
    /// First instance
    pub first_instance: u32,
}

/// Index buffer format
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexFormat {
    /// 16-bit unsigned indices
    Uint16,
    /// 32-bit unsigned indices
    Uint32,
}

impl IndexFormat {
    /// Returns the size of one index in bytes
    pub const fn size(&self) -> usize {
        match self {
            Self::Uint16 => 2,
            Self::Uint32 => 4,
        }
    }
}

/// Primitive topology for rendering
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PrimitiveTopology {
    /// Individual points
    PointList,
    /// Individual lines
    LineList,
    /// Connected line strip
    LineStrip,
    /// Individual triangles
    #[default]
    TriangleList,
    /// Connected triangle strip
    TriangleStrip,
}

/// Front face winding order
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FrontFace {
    /// Counter-clockwise winding
    #[default]
    Ccw,
    /// Clockwise winding
    Cw,
}

/// Face culling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CullMode {
    /// No culling
    #[default]
    None,
    /// Cull front faces
    Front,
    /// Cull back faces
    Back,
}

/// Polygon fill mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PolygonMode {
    /// Fill polygons
    #[default]
    Fill,
    /// Draw lines (wireframe)
    Line,
    /// Draw points
    Point,
}

/// Viewport configuration
#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Minimum depth
    pub min_depth: f32,
    /// Maximum depth
    pub max_depth: f32,
}

impl Viewport {
    /// Creates a viewport from size
    pub const fn from_size(width: f32, height: f32) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// Creates a viewport with offset
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }

    /// Sets the depth range
    pub const fn with_depth(mut self, min: f32, max: f32) -> Self {
        self.min_depth = min;
        self.max_depth = max;
        self
    }
}

/// Scissor rectangle
#[derive(Clone, Copy, Debug)]
pub struct ScissorRect {
    /// X position
    pub x: i32,
    /// Y position
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl ScissorRect {
    /// Creates a scissor rect from size
    pub const fn from_size(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    /// Creates a scissor rect with offset
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

/// Depth bias configuration
#[derive(Clone, Copy, Debug, Default)]
pub struct DepthBias {
    /// Constant depth value added to each fragment
    pub constant_factor: f32,
    /// Maximum/minimum depth bias
    pub clamp: f32,
    /// Slope-dependent depth value added to each fragment
    pub slope_factor: f32,
}

impl DepthBias {
    /// No depth bias
    pub const NONE: Self = Self {
        constant_factor: 0.0,
        clamp: 0.0,
        slope_factor: 0.0,
    };

    /// Creates a depth bias configuration
    pub const fn new(constant_factor: f32, slope_factor: f32) -> Self {
        Self {
            constant_factor,
            clamp: 0.0,
            slope_factor,
        }
    }

    /// Sets the clamp value
    pub const fn with_clamp(mut self, clamp: f32) -> Self {
        self.clamp = clamp;
        self
    }
}

/// Multi-draw command batch
#[derive(Clone, Debug)]
pub struct MultiDrawBatch<'a> {
    /// Pipeline to use
    pub pipeline: PipelineHandle,
    /// Bind groups
    pub bind_groups: &'a [BindGroupHandle],
    /// Vertex buffers
    pub vertex_buffers: &'a [(BufferHandle, u64)],
    /// Index buffer (if indexed drawing)
    pub index_buffer: Option<(BufferHandle, u64, IndexFormat)>,
    /// Draw commands
    pub draws: MultiDrawCommands<'a>,
}

/// Multi-draw commands
#[derive(Clone, Debug)]
pub enum MultiDrawCommands<'a> {
    /// Non-indexed draws
    Draw(&'a [DrawCommand]),
    /// Indexed draws
    DrawIndexed(&'a [DrawIndexedCommand]),
    /// Indirect draws
    DrawIndirect {
        buffer: BufferHandle,
        offset: u64,
        count: u32,
    },
    /// Indirect indexed draws
    DrawIndexedIndirect {
        buffer: BufferHandle,
        offset: u64,
        count: u32,
    },
    /// Indirect draws with count buffer
    DrawIndirectCount {
        draw_buffer: BufferHandle,
        draw_offset: u64,
        count_buffer: BufferHandle,
        count_offset: u64,
        max_count: u32,
    },
}
