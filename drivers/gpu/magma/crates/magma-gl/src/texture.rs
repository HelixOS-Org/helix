//! # Texture Management
//!
//! OpenGL textures translated to Vulkan images.

use alloc::vec::Vec;

use crate::enums::*;
use crate::types::*;

// =============================================================================
// FORMAT TRANSLATION
// =============================================================================

/// Translate GL internal format to Vulkan format
pub fn gl_internal_format_to_vk(internal_format: GLenum) -> u32 {
    match internal_format {
        // R formats
        GL_R8 => 9,     // VK_FORMAT_R8_UNORM
        GL_R16F => 76,  // VK_FORMAT_R16_SFLOAT
        GL_R32F => 100, // VK_FORMAT_R32_SFLOAT

        // RG formats
        GL_RG8 => 16,    // VK_FORMAT_R8G8_UNORM
        GL_RG16F => 83,  // VK_FORMAT_R16G16_SFLOAT
        GL_RG32F => 103, // VK_FORMAT_R32G32_SFLOAT

        // RGB formats
        GL_RGB8 => 23,    // VK_FORMAT_R8G8B8_UNORM
        GL_RGB16F => 90,  // VK_FORMAT_R16G16B16_SFLOAT
        GL_RGB32F => 106, // VK_FORMAT_R32G32B32_SFLOAT
        GL_SRGB8 => 29,   // VK_FORMAT_R8G8B8_SRGB

        // RGBA formats
        GL_RGBA8 => 37,        // VK_FORMAT_R8G8B8A8_UNORM
        GL_RGBA16F => 97,      // VK_FORMAT_R16G16B16A16_SFLOAT
        GL_RGBA32F => 109,     // VK_FORMAT_R32G32B32A32_SFLOAT
        GL_SRGB8_ALPHA8 => 43, // VK_FORMAT_R8G8B8A8_SRGB

        // Depth formats
        GL_DEPTH_COMPONENT16 => 124,  // VK_FORMAT_D16_UNORM
        GL_DEPTH_COMPONENT24 => 125,  // VK_FORMAT_X8_D24_UNORM_PACK32
        GL_DEPTH_COMPONENT32F => 126, // VK_FORMAT_D32_SFLOAT

        // Depth-stencil formats
        GL_DEPTH24_STENCIL8 => 129,  // VK_FORMAT_D24_UNORM_S8_UINT
        GL_DEPTH32F_STENCIL8 => 130, // VK_FORMAT_D32_SFLOAT_S8_UINT

        _ => 0, // VK_FORMAT_UNDEFINED
    }
}

/// Translate GL format + type to Vulkan format
pub fn gl_format_type_to_vk(format: GLenum, type_: GLenum) -> u32 {
    match (format, type_) {
        // RGBA + unsigned byte
        (GL_RGBA, GL_UNSIGNED_BYTE) => 37, // VK_FORMAT_R8G8B8A8_UNORM
        (GL_BGRA, GL_UNSIGNED_BYTE) => 44, // VK_FORMAT_B8G8R8A8_UNORM

        // RGB + unsigned byte
        (GL_RGB, GL_UNSIGNED_BYTE) => 23, // VK_FORMAT_R8G8B8_UNORM
        (GL_BGR, GL_UNSIGNED_BYTE) => 30, // VK_FORMAT_B8G8R8_UNORM

        // RG + unsigned byte
        (GL_RG, GL_UNSIGNED_BYTE) => 16, // VK_FORMAT_R8G8_UNORM

        // R + unsigned byte
        (GL_RED, GL_UNSIGNED_BYTE) => 9, // VK_FORMAT_R8_UNORM

        // Float formats
        (GL_RGBA, GL_FLOAT) => 109, // VK_FORMAT_R32G32B32A32_SFLOAT
        (GL_RGB, GL_FLOAT) => 106,  // VK_FORMAT_R32G32B32_SFLOAT
        (GL_RG, GL_FLOAT) => 103,   // VK_FORMAT_R32G32_SFLOAT
        (GL_RED, GL_FLOAT) => 100,  // VK_FORMAT_R32_SFLOAT

        // Half float formats
        (GL_RGBA, GL_HALF_FLOAT) => 97, // VK_FORMAT_R16G16B16A16_SFLOAT
        (GL_RGB, GL_HALF_FLOAT) => 90,  // VK_FORMAT_R16G16B16_SFLOAT
        (GL_RG, GL_HALF_FLOAT) => 83,   // VK_FORMAT_R16G16_SFLOAT
        (GL_RED, GL_HALF_FLOAT) => 76,  // VK_FORMAT_R16_SFLOAT

        // Depth formats
        (GL_DEPTH_COMPONENT, GL_UNSIGNED_SHORT) => 124, // VK_FORMAT_D16_UNORM
        (GL_DEPTH_COMPONENT, GL_UNSIGNED_INT) => 125,   // VK_FORMAT_X8_D24_UNORM_PACK32
        (GL_DEPTH_COMPONENT, GL_FLOAT) => 126,          // VK_FORMAT_D32_SFLOAT

        _ => 0, // VK_FORMAT_UNDEFINED
    }
}

/// Check if format has depth component
pub fn is_depth_format(internal_format: GLenum) -> bool {
    matches!(
        internal_format,
        GL_DEPTH_COMPONENT16
            | GL_DEPTH_COMPONENT24
            | GL_DEPTH_COMPONENT32F
            | GL_DEPTH24_STENCIL8
            | GL_DEPTH32F_STENCIL8
    )
}

/// Check if format has stencil component
pub fn is_stencil_format(internal_format: GLenum) -> bool {
    matches!(internal_format, GL_DEPTH24_STENCIL8 | GL_DEPTH32F_STENCIL8)
}

/// Check if format is compressed
pub fn is_compressed_format(internal_format: GLenum) -> bool {
    // BC/DXT formats
    (internal_format >= 0x83F0 && internal_format <= 0x83F3) // S3TC
        || (internal_format >= 0x8C4C && internal_format <= 0x8C4F) // RGTC
        || (internal_format >= 0x8E8C && internal_format <= 0x8E8F) // BPTC
        || (internal_format >= 0x9274 && internal_format <= 0x9279) // ETC2/EAC
}

// =============================================================================
// IMAGE TYPE TRANSLATION
// =============================================================================

/// Translate GL texture target to Vulkan image type
pub fn gl_target_to_vk_image_type(target: GLenum) -> u32 {
    match target {
        GL_TEXTURE_1D | GL_TEXTURE_1D_ARRAY => 0, // VK_IMAGE_TYPE_1D
        GL_TEXTURE_2D
        | GL_TEXTURE_2D_ARRAY
        | GL_TEXTURE_2D_MULTISAMPLE
        | GL_TEXTURE_2D_MULTISAMPLE_ARRAY
        | GL_TEXTURE_CUBE_MAP
        | GL_TEXTURE_CUBE_MAP_ARRAY
        | GL_TEXTURE_RECTANGLE => 1, // VK_IMAGE_TYPE_2D
        GL_TEXTURE_3D => 2,                       // VK_IMAGE_TYPE_3D
        _ => 1,                                   // Default to 2D
    }
}

/// Translate GL texture target to Vulkan image view type
pub fn gl_target_to_vk_view_type(target: GLenum) -> u32 {
    match target {
        GL_TEXTURE_1D => 0,                        // VK_IMAGE_VIEW_TYPE_1D
        GL_TEXTURE_2D | GL_TEXTURE_RECTANGLE => 1, // VK_IMAGE_VIEW_TYPE_2D
        GL_TEXTURE_3D => 2,                        // VK_IMAGE_VIEW_TYPE_3D
        GL_TEXTURE_CUBE_MAP => 3,                  // VK_IMAGE_VIEW_TYPE_CUBE
        GL_TEXTURE_1D_ARRAY => 4,                  // VK_IMAGE_VIEW_TYPE_1D_ARRAY
        GL_TEXTURE_2D_ARRAY | GL_TEXTURE_2D_MULTISAMPLE_ARRAY => 5, // VK_IMAGE_VIEW_TYPE_2D_ARRAY
        GL_TEXTURE_CUBE_MAP_ARRAY => 6,            // VK_IMAGE_VIEW_TYPE_CUBE_ARRAY
        _ => 1,
    }
}

// =============================================================================
// SAMPLER TRANSLATION
// =============================================================================

/// Translate GL filter to Vulkan filter
pub fn gl_filter_to_vk(filter: GLenum) -> u32 {
    match filter {
        GL_NEAREST | GL_NEAREST_MIPMAP_NEAREST | GL_NEAREST_MIPMAP_LINEAR => 0, // VK_FILTER_NEAREST
        GL_LINEAR | GL_LINEAR_MIPMAP_NEAREST | GL_LINEAR_MIPMAP_LINEAR => 1,    // VK_FILTER_LINEAR
        _ => 0,
    }
}

/// Translate GL filter to Vulkan mipmap mode
pub fn gl_filter_to_vk_mipmap_mode(filter: GLenum) -> u32 {
    match filter {
        GL_NEAREST_MIPMAP_NEAREST | GL_LINEAR_MIPMAP_NEAREST => 0, // VK_SAMPLER_MIPMAP_MODE_NEAREST
        GL_NEAREST_MIPMAP_LINEAR | GL_LINEAR_MIPMAP_LINEAR => 1,   // VK_SAMPLER_MIPMAP_MODE_LINEAR
        _ => 0,
    }
}

/// Check if filter uses mipmaps
pub fn filter_uses_mipmaps(filter: GLenum) -> bool {
    matches!(
        filter,
        GL_NEAREST_MIPMAP_NEAREST
            | GL_LINEAR_MIPMAP_NEAREST
            | GL_NEAREST_MIPMAP_LINEAR
            | GL_LINEAR_MIPMAP_LINEAR
    )
}

/// Translate GL wrap mode to Vulkan address mode
pub fn gl_wrap_to_vk(wrap: GLenum) -> u32 {
    match wrap {
        GL_REPEAT => 0,               // VK_SAMPLER_ADDRESS_MODE_REPEAT
        GL_MIRRORED_REPEAT => 1,      // VK_SAMPLER_ADDRESS_MODE_MIRRORED_REPEAT
        GL_CLAMP_TO_EDGE => 2,        // VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE
        GL_CLAMP_TO_BORDER => 3,      // VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_BORDER
        GL_MIRROR_CLAMP_TO_EDGE => 4, // VK_SAMPLER_ADDRESS_MODE_MIRROR_CLAMP_TO_EDGE
        _ => 0,
    }
}

/// Translate GL compare func to Vulkan compare op
pub fn gl_compare_to_vk(func: GLenum) -> u32 {
    match func {
        GL_NEVER => 0,    // VK_COMPARE_OP_NEVER
        GL_LESS => 1,     // VK_COMPARE_OP_LESS
        GL_EQUAL => 2,    // VK_COMPARE_OP_EQUAL
        GL_LEQUAL => 3,   // VK_COMPARE_OP_LESS_OR_EQUAL
        GL_GREATER => 4,  // VK_COMPARE_OP_GREATER
        GL_NOTEQUAL => 5, // VK_COMPARE_OP_NOT_EQUAL
        GL_GEQUAL => 6,   // VK_COMPARE_OP_GREATER_OR_EQUAL
        GL_ALWAYS => 7,   // VK_COMPARE_OP_ALWAYS
        _ => 0,
    }
}

// =============================================================================
// SAMPLER STATE
// =============================================================================

/// Sampler state for hashing/caching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SamplerState {
    /// Minification filter
    pub min_filter: GLenum,
    /// Magnification filter
    pub mag_filter: GLenum,
    /// S wrap mode
    pub wrap_s: GLenum,
    /// T wrap mode
    pub wrap_t: GLenum,
    /// R wrap mode
    pub wrap_r: GLenum,
    /// Compare mode enabled
    pub compare_enabled: bool,
    /// Compare function
    pub compare_func: GLenum,
    /// Anisotropy enabled
    pub anisotropy_enabled: bool,
    /// Max anisotropy (as fixed point * 16)
    pub max_anisotropy_x16: u8,
    /// Min LOD (as fixed point * 16)
    pub min_lod_x16: i16,
    /// Max LOD (as fixed point * 16)
    pub max_lod_x16: i16,
    /// LOD bias (as fixed point * 256)
    pub lod_bias_x256: i16,
    /// Border color type
    pub border_color: BorderColorType,
}

/// Border color types (Vulkan only supports predefined colors)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorderColorType {
    /// Transparent black (0, 0, 0, 0)
    TransparentBlack,
    /// Opaque black (0, 0, 0, 1)
    OpaqueBlack,
    /// Opaque white (1, 1, 1, 1)
    OpaqueWhite,
    /// Custom color (requires extension)
    Custom,
}

impl Default for SamplerState {
    fn default() -> Self {
        Self {
            min_filter: GL_NEAREST_MIPMAP_LINEAR,
            mag_filter: GL_LINEAR,
            wrap_s: GL_REPEAT,
            wrap_t: GL_REPEAT,
            wrap_r: GL_REPEAT,
            compare_enabled: false,
            compare_func: GL_LEQUAL,
            anisotropy_enabled: false,
            max_anisotropy_x16: 16, // 1.0
            min_lod_x16: -16000,    // -1000.0
            max_lod_x16: 16000,     // 1000.0
            lod_bias_x256: 0,
            border_color: BorderColorType::TransparentBlack,
        }
    }
}

impl SamplerState {
    /// Convert border color RGBA to type
    pub fn border_color_from_rgba(rgba: [f32; 4]) -> BorderColorType {
        if rgba == [0.0, 0.0, 0.0, 0.0] {
            BorderColorType::TransparentBlack
        } else if rgba == [0.0, 0.0, 0.0, 1.0] {
            BorderColorType::OpaqueBlack
        } else if rgba == [1.0, 1.0, 1.0, 1.0] {
            BorderColorType::OpaqueWhite
        } else {
            BorderColorType::Custom
        }
    }

    /// Get Vulkan border color enum
    pub fn vk_border_color(&self, is_int: bool) -> u32 {
        match (self.border_color, is_int) {
            (BorderColorType::TransparentBlack, false) => 0, // VK_BORDER_COLOR_FLOAT_TRANSPARENT_BLACK
            (BorderColorType::TransparentBlack, true) => 1, // VK_BORDER_COLOR_INT_TRANSPARENT_BLACK
            (BorderColorType::OpaqueBlack, false) => 2,     // VK_BORDER_COLOR_FLOAT_OPAQUE_BLACK
            (BorderColorType::OpaqueBlack, true) => 3,      // VK_BORDER_COLOR_INT_OPAQUE_BLACK
            (BorderColorType::OpaqueWhite, false) => 4,     // VK_BORDER_COLOR_FLOAT_OPAQUE_WHITE
            (BorderColorType::OpaqueWhite, true) => 5,      // VK_BORDER_COLOR_INT_OPAQUE_WHITE
            (BorderColorType::Custom, _) => 0, // Would need custom border color extension
        }
    }
}

// =============================================================================
// TEXTURE OPERATIONS
// =============================================================================

/// Texture upload parameters
#[derive(Debug)]
pub struct TextureUploadParams {
    /// Target texture
    pub texture: u32,
    /// Mip level
    pub level: u32,
    /// X offset
    pub x_offset: u32,
    /// Y offset
    pub y_offset: u32,
    /// Z offset (for 3D/array textures)
    pub z_offset: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth (for 3D/array textures)
    pub depth: u32,
    /// Pixel format
    pub format: GLenum,
    /// Pixel type
    pub type_: GLenum,
    /// Data pointer
    pub data: *const u8,
    /// Row length (0 = tightly packed)
    pub row_length: u32,
    /// Image height (0 = tightly packed)
    pub image_height: u32,
    /// Skip pixels
    pub skip_pixels: u32,
    /// Skip rows
    pub skip_rows: u32,
    /// Skip images
    pub skip_images: u32,
    /// Alignment
    pub alignment: u32,
}

/// Calculate mip level dimensions
pub fn mip_dimensions(
    base_width: u32,
    base_height: u32,
    base_depth: u32,
    level: u32,
) -> (u32, u32, u32) {
    let width = (base_width >> level).max(1);
    let height = (base_height >> level).max(1);
    let depth = (base_depth >> level).max(1);
    (width, height, depth)
}

/// Calculate number of mip levels for dimensions
pub fn mip_level_count(width: u32, height: u32, depth: u32) -> u32 {
    let max_dim = width.max(height).max(depth);
    if max_dim == 0 {
        1
    } else {
        32 - max_dim.leading_zeros()
    }
}

/// Calculate texture data size
pub fn texture_size(width: u32, height: u32, depth: u32, format: GLenum, type_: GLenum) -> usize {
    let components = match format {
        GL_RED | GL_DEPTH_COMPONENT | GL_STENCIL_INDEX => 1,
        GL_RG | GL_DEPTH_STENCIL => 2,
        GL_RGB | GL_BGR => 3,
        GL_RGBA | GL_BGRA => 4,
        _ => 4,
    };

    let component_size = match type_ {
        GL_UNSIGNED_BYTE | GL_BYTE => 1,
        GL_UNSIGNED_SHORT | GL_SHORT | GL_HALF_FLOAT => 2,
        GL_UNSIGNED_INT | GL_INT | GL_FLOAT | GL_FIXED => 4,
        GL_DOUBLE => 8,
        _ => 1,
    };

    (width as usize) * (height as usize) * (depth as usize) * components * component_size
}
