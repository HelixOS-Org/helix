//! # Buffer Management
//!
//! OpenGL buffer objects translated to Vulkan buffers.

use crate::context::{BufferObject, BufferUsageHint, GlContext};
use crate::enums::*;
use crate::types::*;
use alloc::vec::Vec;

// =============================================================================
// BUFFER TARGET TRANSLATION
// =============================================================================

/// Translate GL buffer usage to Vulkan memory properties
pub fn translate_usage_to_vk_flags(usage: BufferUsageHint) -> u32 {
    // Returns VkMemoryPropertyFlags
    match usage {
        BufferUsageHint::StaticDraw
        | BufferUsageHint::StaticRead
        | BufferUsageHint::StaticCopy => {
            // Device local for static data
            0x00000001 // VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT
        }
        BufferUsageHint::DynamicDraw
        | BufferUsageHint::DynamicRead
        | BufferUsageHint::DynamicCopy => {
            // Host visible + coherent for dynamic data
            0x00000002 | 0x00000004 // VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT
        }
        BufferUsageHint::StreamDraw
        | BufferUsageHint::StreamRead
        | BufferUsageHint::StreamCopy => {
            // Host visible + coherent for streaming
            0x00000002 | 0x00000004
        }
    }
}

/// Translate GL buffer target to Vulkan buffer usage flags
pub fn translate_target_to_vk_usage(target: GLenum) -> u32 {
    // Returns VkBufferUsageFlags
    match target {
        GL_ARRAY_BUFFER => 0x00000080, // VK_BUFFER_USAGE_VERTEX_BUFFER_BIT
        GL_ELEMENT_ARRAY_BUFFER => 0x00000040, // VK_BUFFER_USAGE_INDEX_BUFFER_BIT
        GL_UNIFORM_BUFFER => 0x00000010, // VK_BUFFER_USAGE_UNIFORM_BUFFER_BIT
        GL_SHADER_STORAGE_BUFFER => 0x00000020, // VK_BUFFER_USAGE_STORAGE_BUFFER_BIT
        GL_COPY_READ_BUFFER | GL_COPY_WRITE_BUFFER => {
            0x00000001 | 0x00000002 // VK_BUFFER_USAGE_TRANSFER_SRC_BIT | VK_BUFFER_USAGE_TRANSFER_DST_BIT
        }
        GL_PIXEL_PACK_BUFFER => 0x00000001, // VK_BUFFER_USAGE_TRANSFER_SRC_BIT
        GL_PIXEL_UNPACK_BUFFER => 0x00000002, // VK_BUFFER_USAGE_TRANSFER_DST_BIT
        GL_DRAW_INDIRECT_BUFFER | GL_DISPATCH_INDIRECT_BUFFER => {
            0x00000100 // VK_BUFFER_USAGE_INDIRECT_BUFFER_BIT
        }
        GL_TRANSFORM_FEEDBACK_BUFFER => {
            0x00000800 // VK_BUFFER_USAGE_TRANSFORM_FEEDBACK_BUFFER_BIT_EXT
        }
        _ => 0,
    }
}

// =============================================================================
// BUFFER OPERATIONS
// =============================================================================

/// Buffer data operation (glBufferData equivalent)
pub struct BufferDataOp {
    /// Target buffer handle
    pub buffer: u32,
    /// Size in bytes
    pub size: usize,
    /// Data pointer (may be null for allocation only)
    pub data: Option<*const u8>,
    /// Usage hint
    pub usage: BufferUsageHint,
}

/// Buffer sub-data operation (glBufferSubData equivalent)
pub struct BufferSubDataOp {
    /// Target buffer handle
    pub buffer: u32,
    /// Offset in bytes
    pub offset: usize,
    /// Size in bytes
    pub size: usize,
    /// Data pointer
    pub data: *const u8,
}

/// Buffer copy operation (glCopyBufferSubData equivalent)
pub struct BufferCopyOp {
    /// Source buffer
    pub src_buffer: u32,
    /// Destination buffer
    pub dst_buffer: u32,
    /// Source offset
    pub src_offset: usize,
    /// Destination offset
    pub dst_offset: usize,
    /// Size to copy
    pub size: usize,
}

// =============================================================================
// BUFFER MAPPING
// =============================================================================

/// Map access flags
#[derive(Debug, Clone, Copy)]
pub struct MapAccess {
    /// Read access
    pub read: bool,
    /// Write access
    pub write: bool,
    /// Invalidate range
    pub invalidate_range: bool,
    /// Invalidate buffer
    pub invalidate_buffer: bool,
    /// Flush explicit
    pub flush_explicit: bool,
    /// Unsynchronized
    pub unsynchronized: bool,
    /// Persistent
    pub persistent: bool,
    /// Coherent
    pub coherent: bool,
}

impl MapAccess {
    /// Parse GL map access flags
    pub fn from_gl_flags(access: GLbitfield) -> Self {
        const GL_MAP_READ_BIT: u32 = 0x0001;
        const GL_MAP_WRITE_BIT: u32 = 0x0002;
        const GL_MAP_INVALIDATE_RANGE_BIT: u32 = 0x0004;
        const GL_MAP_INVALIDATE_BUFFER_BIT: u32 = 0x0008;
        const GL_MAP_FLUSH_EXPLICIT_BIT: u32 = 0x0010;
        const GL_MAP_UNSYNCHRONIZED_BIT: u32 = 0x0020;
        const GL_MAP_PERSISTENT_BIT: u32 = 0x0040;
        const GL_MAP_COHERENT_BIT: u32 = 0x0080;

        Self {
            read: (access & GL_MAP_READ_BIT) != 0,
            write: (access & GL_MAP_WRITE_BIT) != 0,
            invalidate_range: (access & GL_MAP_INVALIDATE_RANGE_BIT) != 0,
            invalidate_buffer: (access & GL_MAP_INVALIDATE_BUFFER_BIT) != 0,
            flush_explicit: (access & GL_MAP_FLUSH_EXPLICIT_BIT) != 0,
            unsynchronized: (access & GL_MAP_UNSYNCHRONIZED_BIT) != 0,
            persistent: (access & GL_MAP_PERSISTENT_BIT) != 0,
            coherent: (access & GL_MAP_COHERENT_BIT) != 0,
        }
    }
}

// =============================================================================
// VERTEX ATTRIBUTE FORMAT
// =============================================================================

/// Vertex attribute format info
#[derive(Debug, Clone, Copy)]
pub struct VertexAttribFormat {
    /// Number of components (1-4)
    pub components: u8,
    /// Component type
    pub component_type: GLenum,
    /// Normalize integer to float
    pub normalized: bool,
    /// Size of one element in bytes
    pub element_size: u8,
}

impl VertexAttribFormat {
    /// Calculate format from GL parameters
    pub fn from_gl(size: i32, type_: GLenum, normalized: bool) -> Self {
        let component_size = match type_ {
            GL_BYTE | GL_UNSIGNED_BYTE => 1,
            GL_SHORT | GL_UNSIGNED_SHORT | GL_HALF_FLOAT => 2,
            GL_INT | GL_UNSIGNED_INT | GL_FLOAT | GL_FIXED => 4,
            GL_DOUBLE => 8,
            _ => 4, // Default
        };

        Self {
            components: size as u8,
            component_type: type_,
            normalized,
            element_size: component_size * size as u8,
        }
    }

    /// Translate to Vulkan format (VkFormat)
    pub fn to_vk_format(&self) -> u32 {
        match (self.component_type, self.components, self.normalized) {
            // Float formats
            (GL_FLOAT, 1, _) => 100, // VK_FORMAT_R32_SFLOAT
            (GL_FLOAT, 2, _) => 103, // VK_FORMAT_R32G32_SFLOAT
            (GL_FLOAT, 3, _) => 106, // VK_FORMAT_R32G32B32_SFLOAT
            (GL_FLOAT, 4, _) => 109, // VK_FORMAT_R32G32B32A32_SFLOAT

            // Half float formats
            (GL_HALF_FLOAT, 1, _) => 76, // VK_FORMAT_R16_SFLOAT
            (GL_HALF_FLOAT, 2, _) => 83, // VK_FORMAT_R16G16_SFLOAT
            (GL_HALF_FLOAT, 3, _) => 90, // VK_FORMAT_R16G16B16_SFLOAT
            (GL_HALF_FLOAT, 4, _) => 97, // VK_FORMAT_R16G16B16A16_SFLOAT

            // Unsigned byte normalized
            (GL_UNSIGNED_BYTE, 1, true) => 9,  // VK_FORMAT_R8_UNORM
            (GL_UNSIGNED_BYTE, 2, true) => 16, // VK_FORMAT_R8G8_UNORM
            (GL_UNSIGNED_BYTE, 3, true) => 23, // VK_FORMAT_R8G8B8_UNORM
            (GL_UNSIGNED_BYTE, 4, true) => 37, // VK_FORMAT_R8G8B8A8_UNORM

            // Unsigned byte integer
            (GL_UNSIGNED_BYTE, 1, false) => 13, // VK_FORMAT_R8_UINT
            (GL_UNSIGNED_BYTE, 2, false) => 20, // VK_FORMAT_R8G8_UINT
            (GL_UNSIGNED_BYTE, 3, false) => 27, // VK_FORMAT_R8G8B8_UINT
            (GL_UNSIGNED_BYTE, 4, false) => 41, // VK_FORMAT_R8G8B8A8_UINT

            // Signed byte normalized
            (GL_BYTE, 1, true) => 10,  // VK_FORMAT_R8_SNORM
            (GL_BYTE, 2, true) => 17,  // VK_FORMAT_R8G8_SNORM
            (GL_BYTE, 3, true) => 24,  // VK_FORMAT_R8G8B8_SNORM
            (GL_BYTE, 4, true) => 38,  // VK_FORMAT_R8G8B8A8_SNORM

            // Signed byte integer
            (GL_BYTE, 1, false) => 14, // VK_FORMAT_R8_SINT
            (GL_BYTE, 2, false) => 21, // VK_FORMAT_R8G8_SINT
            (GL_BYTE, 3, false) => 28, // VK_FORMAT_R8G8B8_SINT
            (GL_BYTE, 4, false) => 42, // VK_FORMAT_R8G8B8A8_SINT

            // Unsigned short normalized
            (GL_UNSIGNED_SHORT, 1, true) => 70, // VK_FORMAT_R16_UNORM
            (GL_UNSIGNED_SHORT, 2, true) => 77, // VK_FORMAT_R16G16_UNORM
            (GL_UNSIGNED_SHORT, 3, true) => 84, // VK_FORMAT_R16G16B16_UNORM
            (GL_UNSIGNED_SHORT, 4, true) => 91, // VK_FORMAT_R16G16B16A16_UNORM

            // Unsigned short integer
            (GL_UNSIGNED_SHORT, 1, false) => 74, // VK_FORMAT_R16_UINT
            (GL_UNSIGNED_SHORT, 2, false) => 81, // VK_FORMAT_R16G16_UINT
            (GL_UNSIGNED_SHORT, 3, false) => 88, // VK_FORMAT_R16G16B16_UINT
            (GL_UNSIGNED_SHORT, 4, false) => 95, // VK_FORMAT_R16G16B16A16_UINT

            // Signed short normalized
            (GL_SHORT, 1, true) => 71, // VK_FORMAT_R16_SNORM
            (GL_SHORT, 2, true) => 78, // VK_FORMAT_R16G16_SNORM
            (GL_SHORT, 3, true) => 85, // VK_FORMAT_R16G16B16_SNORM
            (GL_SHORT, 4, true) => 92, // VK_FORMAT_R16G16B16A16_SNORM

            // Signed short integer
            (GL_SHORT, 1, false) => 75, // VK_FORMAT_R16_SINT
            (GL_SHORT, 2, false) => 82, // VK_FORMAT_R16G16_SINT
            (GL_SHORT, 3, false) => 89, // VK_FORMAT_R16G16B16_SINT
            (GL_SHORT, 4, false) => 96, // VK_FORMAT_R16G16B16A16_SINT

            // Unsigned int
            (GL_UNSIGNED_INT, 1, _) => 98,  // VK_FORMAT_R32_UINT
            (GL_UNSIGNED_INT, 2, _) => 101, // VK_FORMAT_R32G32_UINT
            (GL_UNSIGNED_INT, 3, _) => 104, // VK_FORMAT_R32G32B32_UINT
            (GL_UNSIGNED_INT, 4, _) => 107, // VK_FORMAT_R32G32B32A32_UINT

            // Signed int
            (GL_INT, 1, _) => 99,  // VK_FORMAT_R32_SINT
            (GL_INT, 2, _) => 102, // VK_FORMAT_R32G32_SINT
            (GL_INT, 3, _) => 105, // VK_FORMAT_R32G32B32_SINT
            (GL_INT, 4, _) => 108, // VK_FORMAT_R32G32B32A32_SINT

            // Double
            (GL_DOUBLE, 1, _) => 114, // VK_FORMAT_R64_SFLOAT
            (GL_DOUBLE, 2, _) => 117, // VK_FORMAT_R64G64_SFLOAT
            (GL_DOUBLE, 3, _) => 120, // VK_FORMAT_R64G64B64_SFLOAT
            (GL_DOUBLE, 4, _) => 123, // VK_FORMAT_R64G64B64A64_SFLOAT

            _ => 0, // VK_FORMAT_UNDEFINED
        }
    }
}

// =============================================================================
// INDEX TYPE TRANSLATION
// =============================================================================

/// Translate GL index type to Vulkan index type
pub fn translate_index_type(type_: GLenum) -> u32 {
    match type_ {
        GL_UNSIGNED_BYTE => 1000265000, // VK_INDEX_TYPE_UINT8_EXT
        GL_UNSIGNED_SHORT => 0,         // VK_INDEX_TYPE_UINT16
        GL_UNSIGNED_INT => 1,           // VK_INDEX_TYPE_UINT32
        _ => 0,
    }
}
