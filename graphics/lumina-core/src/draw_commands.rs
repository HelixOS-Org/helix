//! Draw Commands for Lumina
//!
//! This module provides draw command types for graphics pipeline rendering
//! including direct and indirect draw calls.

// ============================================================================
// Draw Command Types
// ============================================================================

/// Direct draw command
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DrawCommand {
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

impl DrawCommand {
    /// Creates new draw command
    #[inline]
    pub const fn new(
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        }
    }

    /// Simple draw (single instance, from vertex 0)
    #[inline]
    pub const fn simple(vertex_count: u32) -> Self {
        Self::new(vertex_count, 1, 0, 0)
    }

    /// Instanced draw
    #[inline]
    pub const fn instanced(vertex_count: u32, instance_count: u32) -> Self {
        Self::new(vertex_count, instance_count, 0, 0)
    }

    /// Triangle (3 vertices)
    pub const TRIANGLE: Self = Self::simple(3);
    /// Quad (4 vertices for strip)
    pub const QUAD_STRIP: Self = Self::simple(4);
    /// Quad (6 vertices for list)
    pub const QUAD_LIST: Self = Self::simple(6);
    /// Fullscreen triangle
    pub const FULLSCREEN: Self = Self::simple(3);
}

impl Default for DrawCommand {
    fn default() -> Self {
        Self::simple(0)
    }
}

/// Indexed draw command
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DrawIndexedCommand {
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

impl DrawIndexedCommand {
    /// Creates new indexed draw command
    #[inline]
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

    /// Simple indexed draw
    #[inline]
    pub const fn simple(index_count: u32) -> Self {
        Self::new(index_count, 1, 0, 0, 0)
    }

    /// Instanced indexed draw
    #[inline]
    pub const fn instanced(index_count: u32, instance_count: u32) -> Self {
        Self::new(index_count, instance_count, 0, 0, 0)
    }

    /// With vertex offset
    #[inline]
    pub const fn with_vertex_offset(mut self, offset: i32) -> Self {
        self.vertex_offset = offset;
        self
    }

    /// With first index
    #[inline]
    pub const fn with_first_index(mut self, index: u32) -> Self {
        self.first_index = index;
        self
    }
}

impl Default for DrawIndexedCommand {
    fn default() -> Self {
        Self::simple(0)
    }
}

// ============================================================================
// Indirect Draw Commands
// ============================================================================

/// Indirect draw arguments (matches GPU structure)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectDrawArgs {
    /// Vertex count
    pub vertex_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First instance
    pub first_instance: u32,
}

impl IndirectDrawArgs {
    /// Size in bytes
    pub const SIZE: usize = 16;

    /// Creates new args
    #[inline]
    pub const fn new(
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        Self {
            vertex_count,
            instance_count,
            first_vertex,
            first_instance,
        }
    }

    /// From draw command
    #[inline]
    pub const fn from_command(cmd: DrawCommand) -> Self {
        Self {
            vertex_count: cmd.vertex_count,
            instance_count: cmd.instance_count,
            first_vertex: cmd.first_vertex,
            first_instance: cmd.first_instance,
        }
    }
}

impl Default for IndirectDrawArgs {
    fn default() -> Self {
        Self::new(0, 1, 0, 0)
    }
}

/// Indexed indirect draw arguments
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectDrawIndexedArgs {
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

impl IndirectDrawIndexedArgs {
    /// Size in bytes
    pub const SIZE: usize = 20;

    /// Creates new args
    #[inline]
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

    /// From indexed draw command
    #[inline]
    pub const fn from_command(cmd: DrawIndexedCommand) -> Self {
        Self {
            index_count: cmd.index_count,
            instance_count: cmd.instance_count,
            first_index: cmd.first_index,
            vertex_offset: cmd.vertex_offset,
            first_instance: cmd.first_instance,
        }
    }
}

impl Default for IndirectDrawIndexedArgs {
    fn default() -> Self {
        Self::new(0, 1, 0, 0, 0)
    }
}

/// Indirect draw command configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectDrawConfig {
    /// Buffer containing arguments
    pub buffer: u64,
    /// Offset in buffer
    pub offset: u64,
    /// Draw count
    pub draw_count: u32,
    /// Stride between draw arguments
    pub stride: u32,
}

impl IndirectDrawConfig {
    /// Creates new config
    #[inline]
    pub const fn new(buffer: u64, offset: u64, draw_count: u32, stride: u32) -> Self {
        Self {
            buffer,
            offset,
            draw_count,
            stride,
        }
    }

    /// Single draw
    #[inline]
    pub const fn single(buffer: u64, offset: u64) -> Self {
        Self::new(buffer, offset, 1, IndirectDrawArgs::SIZE as u32)
    }

    /// Multiple draws
    #[inline]
    pub const fn multi(buffer: u64, offset: u64, count: u32) -> Self {
        Self::new(buffer, offset, count, IndirectDrawArgs::SIZE as u32)
    }
}

// ============================================================================
// Indirect Count Draw
// ============================================================================

/// Indirect count draw configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectCountDrawConfig {
    /// Buffer containing arguments
    pub buffer: u64,
    /// Offset in buffer
    pub offset: u64,
    /// Count buffer
    pub count_buffer: u64,
    /// Count buffer offset
    pub count_buffer_offset: u64,
    /// Maximum draw count
    pub max_draw_count: u32,
    /// Stride between draw arguments
    pub stride: u32,
}

impl IndirectCountDrawConfig {
    /// Creates new config
    #[inline]
    pub const fn new(
        buffer: u64,
        offset: u64,
        count_buffer: u64,
        count_buffer_offset: u64,
        max_draw_count: u32,
        stride: u32,
    ) -> Self {
        Self {
            buffer,
            offset,
            count_buffer,
            count_buffer_offset,
            max_draw_count,
            stride,
        }
    }
}

// ============================================================================
// Mesh Shader Draw Commands
// ============================================================================

/// Mesh shader draw command
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DrawMeshTasksCommand {
    /// Task count X
    pub group_count_x: u32,
    /// Task count Y
    pub group_count_y: u32,
    /// Task count Z
    pub group_count_z: u32,
}

impl DrawMeshTasksCommand {
    /// Creates new command
    #[inline]
    pub const fn new(group_count_x: u32, group_count_y: u32, group_count_z: u32) -> Self {
        Self {
            group_count_x,
            group_count_y,
            group_count_z,
        }
    }

    /// 1D dispatch
    #[inline]
    pub const fn d1(count: u32) -> Self {
        Self::new(count, 1, 1)
    }

    /// 2D dispatch
    #[inline]
    pub const fn d2(x: u32, y: u32) -> Self {
        Self::new(x, y, 1)
    }

    /// Total groups
    #[inline]
    pub const fn total_groups(&self) -> u64 {
        self.group_count_x as u64 * self.group_count_y as u64 * self.group_count_z as u64
    }
}

impl Default for DrawMeshTasksCommand {
    fn default() -> Self {
        Self::d1(1)
    }
}

/// Indirect mesh tasks arguments
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndirectMeshTasksArgs {
    /// Group count X
    pub group_count_x: u32,
    /// Group count Y
    pub group_count_y: u32,
    /// Group count Z
    pub group_count_z: u32,
}

impl IndirectMeshTasksArgs {
    /// Size in bytes
    pub const SIZE: usize = 12;

    /// Creates new args
    #[inline]
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self {
            group_count_x: x,
            group_count_y: y,
            group_count_z: z,
        }
    }

    /// From command
    #[inline]
    pub const fn from_command(cmd: DrawMeshTasksCommand) -> Self {
        Self {
            group_count_x: cmd.group_count_x,
            group_count_y: cmd.group_count_y,
            group_count_z: cmd.group_count_z,
        }
    }
}

impl Default for IndirectMeshTasksArgs {
    fn default() -> Self {
        Self::new(1, 1, 1)
    }
}

// ============================================================================
// Index Buffer Binding
// ============================================================================

/// Index type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum IndexType {
    /// 16-bit indices
    Uint16 = 0,
    /// 32-bit indices
    #[default]
    Uint32 = 1,
    /// 8-bit indices (extension)
    Uint8  = 2,
    /// No indices
    None   = 0xFF,
}

impl IndexType {
    /// Size in bytes
    #[inline]
    pub const fn size(&self) -> u32 {
        match self {
            Self::Uint8 => 1,
            Self::Uint16 => 2,
            Self::Uint32 => 4,
            Self::None => 0,
        }
    }

    /// Maximum index value
    #[inline]
    pub const fn max_index(&self) -> u32 {
        match self {
            Self::Uint8 => u8::MAX as u32,
            Self::Uint16 => u16::MAX as u32,
            Self::Uint32 => u32::MAX,
            Self::None => 0,
        }
    }
}

/// Index buffer binding
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct IndexBufferBinding {
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Index type
    pub index_type: IndexType,
}

impl IndexBufferBinding {
    /// Creates new binding
    #[inline]
    pub const fn new(buffer: u64, offset: u64, index_type: IndexType) -> Self {
        Self {
            buffer,
            offset,
            index_type,
        }
    }

    /// Uint16 indices
    #[inline]
    pub const fn uint16(buffer: u64, offset: u64) -> Self {
        Self::new(buffer, offset, IndexType::Uint16)
    }

    /// Uint32 indices
    #[inline]
    pub const fn uint32(buffer: u64, offset: u64) -> Self {
        Self::new(buffer, offset, IndexType::Uint32)
    }
}

impl Default for IndexBufferBinding {
    fn default() -> Self {
        Self::new(0, 0, IndexType::Uint32)
    }
}

// ============================================================================
// Vertex Buffer Binding
// ============================================================================

/// Vertex buffer binding
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexBufferBinding {
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Size (0 = whole buffer)
    pub size: u64,
    /// Stride (0 = use pipeline stride)
    pub stride: u64,
}

impl VertexBufferBinding {
    /// Creates new binding
    #[inline]
    pub const fn new(buffer: u64, offset: u64) -> Self {
        Self {
            buffer,
            offset,
            size: 0,
            stride: 0,
        }
    }

    /// With size
    #[inline]
    pub const fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    /// With stride
    #[inline]
    pub const fn with_stride(mut self, stride: u64) -> Self {
        self.stride = stride;
        self
    }
}

impl Default for VertexBufferBinding {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Multiple vertex buffer bindings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VertexBufferBindings {
    /// First binding index
    pub first_binding: u32,
    /// Bindings
    pub bindings: [VertexBufferBinding; 16],
    /// Binding count
    pub binding_count: u32,
}

impl VertexBufferBindings {
    /// Creates new bindings
    #[inline]
    pub const fn new() -> Self {
        Self {
            first_binding: 0,
            bindings: [VertexBufferBinding::new(0, 0); 16],
            binding_count: 0,
        }
    }

    /// Single binding at index 0
    #[inline]
    pub const fn single(buffer: u64, offset: u64) -> Self {
        let mut bindings = Self::new();
        bindings.bindings[0] = VertexBufferBinding::new(buffer, offset);
        bindings.binding_count = 1;
        bindings
    }

    /// Add binding
    pub fn add(&mut self, binding: VertexBufferBinding) {
        if self.binding_count < 16 {
            self.bindings[self.binding_count as usize] = binding;
            self.binding_count += 1;
        }
    }
}

impl Default for VertexBufferBindings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Draw State
// ============================================================================

/// Draw state flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DrawStateFlags(pub u32);

impl DrawStateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Indexed draw
    pub const INDEXED: Self = Self(1 << 0);
    /// Instanced draw
    pub const INSTANCED: Self = Self(1 << 1);
    /// Indirect draw
    pub const INDIRECT: Self = Self(1 << 2);
    /// Count from buffer
    pub const COUNT_FROM_BUFFER: Self = Self(1 << 3);
    /// Mesh shader draw
    pub const MESH_SHADER: Self = Self(1 << 4);
    /// Conditional rendering
    pub const CONDITIONAL: Self = Self(1 << 5);
    /// Multi-draw
    pub const MULTI_DRAW: Self = Self(1 << 6);

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

/// Current draw state
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DrawState {
    /// Flags
    pub flags: DrawStateFlags,
    /// Vertex count (for non-indexed)
    pub vertex_count: u32,
    /// Index count (for indexed)
    pub index_count: u32,
    /// Instance count
    pub instance_count: u32,
    /// First vertex
    pub first_vertex: u32,
    /// First index
    pub first_index: u32,
    /// Vertex offset
    pub vertex_offset: i32,
    /// First instance
    pub first_instance: u32,
}

impl DrawState {
    /// Creates new state
    #[inline]
    pub const fn new() -> Self {
        Self {
            flags: DrawStateFlags::NONE,
            vertex_count: 0,
            index_count: 0,
            instance_count: 1,
            first_vertex: 0,
            first_index: 0,
            vertex_offset: 0,
            first_instance: 0,
        }
    }

    /// From draw command
    #[inline]
    pub const fn from_draw(cmd: DrawCommand) -> Self {
        Self {
            flags: DrawStateFlags::NONE,
            vertex_count: cmd.vertex_count,
            index_count: 0,
            instance_count: cmd.instance_count,
            first_vertex: cmd.first_vertex,
            first_index: 0,
            vertex_offset: 0,
            first_instance: cmd.first_instance,
        }
    }

    /// From indexed draw command
    #[inline]
    pub const fn from_indexed_draw(cmd: DrawIndexedCommand) -> Self {
        Self {
            flags: DrawStateFlags::INDEXED,
            vertex_count: 0,
            index_count: cmd.index_count,
            instance_count: cmd.instance_count,
            first_vertex: 0,
            first_index: cmd.first_index,
            vertex_offset: cmd.vertex_offset,
            first_instance: cmd.first_instance,
        }
    }

    /// Is indexed
    #[inline]
    pub const fn is_indexed(&self) -> bool {
        self.flags.contains(DrawStateFlags::INDEXED)
    }

    /// Is instanced
    #[inline]
    pub const fn is_instanced(&self) -> bool {
        self.instance_count > 1
    }

    /// Primitive count
    #[inline]
    pub const fn primitive_count(&self) -> u32 {
        if self.is_indexed() {
            self.index_count
        } else {
            self.vertex_count
        }
    }

    /// Total vertices processed
    #[inline]
    pub const fn total_vertices(&self) -> u64 {
        if self.is_indexed() {
            self.index_count as u64 * self.instance_count as u64
        } else {
            self.vertex_count as u64 * self.instance_count as u64
        }
    }
}

// ============================================================================
// Multi-Draw
// ============================================================================

/// Multi-draw info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultiDrawInfo {
    /// Draw commands
    pub draws: &'static [DrawCommand],
}

impl MultiDrawInfo {
    /// Creates new info
    #[inline]
    pub const fn new(draws: &'static [DrawCommand]) -> Self {
        Self { draws }
    }

    /// Draw count
    #[inline]
    pub const fn draw_count(&self) -> usize {
        self.draws.len()
    }
}

/// Multi-draw indexed info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultiDrawIndexedInfo {
    /// Draw commands
    pub draws: &'static [DrawIndexedCommand],
    /// Vertex offsets (optional, one per draw)
    pub vertex_offsets: Option<&'static [i32]>,
}

impl MultiDrawIndexedInfo {
    /// Creates new info
    #[inline]
    pub const fn new(draws: &'static [DrawIndexedCommand]) -> Self {
        Self {
            draws,
            vertex_offsets: None,
        }
    }

    /// With vertex offsets
    #[inline]
    pub const fn with_vertex_offsets(mut self, offsets: &'static [i32]) -> Self {
        self.vertex_offsets = Some(offsets);
        self
    }

    /// Draw count
    #[inline]
    pub const fn draw_count(&self) -> usize {
        self.draws.len()
    }
}

// ============================================================================
// Conditional Rendering
// ============================================================================

/// Conditional rendering info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ConditionalRenderingInfo {
    /// Buffer containing condition
    pub buffer: u64,
    /// Offset in buffer
    pub offset: u64,
    /// Flags
    pub flags: ConditionalRenderingFlags,
}

impl ConditionalRenderingInfo {
    /// Creates new info
    #[inline]
    pub const fn new(buffer: u64, offset: u64) -> Self {
        Self {
            buffer,
            offset,
            flags: ConditionalRenderingFlags::NONE,
        }
    }

    /// Inverted condition
    #[inline]
    pub const fn inverted(mut self) -> Self {
        self.flags = ConditionalRenderingFlags::INVERTED;
        self
    }
}

/// Conditional rendering flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ConditionalRenderingFlags(pub u32);

impl ConditionalRenderingFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Inverted condition
    pub const INVERTED: Self = Self(1 << 0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Primitive Restart
// ============================================================================

/// Primitive restart value
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct PrimitiveRestartIndex {
    /// Index type
    pub index_type: IndexType,
    /// Custom restart index (0 = use default)
    pub custom_index: u32,
}

impl PrimitiveRestartIndex {
    /// Default restart index for type
    #[inline]
    pub const fn default_for_type(index_type: IndexType) -> Self {
        Self {
            index_type,
            custom_index: 0,
        }
    }

    /// Custom restart index
    #[inline]
    pub const fn custom(index_type: IndexType, index: u32) -> Self {
        Self {
            index_type,
            custom_index: index,
        }
    }

    /// Effective restart index
    #[inline]
    pub const fn effective_index(&self) -> u32 {
        if self.custom_index != 0 {
            self.custom_index
        } else {
            self.index_type.max_index()
        }
    }
}

impl Default for PrimitiveRestartIndex {
    fn default() -> Self {
        Self::default_for_type(IndexType::Uint32)
    }
}
