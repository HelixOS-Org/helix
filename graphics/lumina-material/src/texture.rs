//! Texture Management
//!
//! This module provides texture creation, management, and streaming.

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use core::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Texture Handle
// ============================================================================

/// Handle to a texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle {
    /// Index.
    index: u32,
    /// Generation.
    generation: u32,
}

impl TextureHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };

    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

// ============================================================================
// Texture Format
// ============================================================================

/// Texture format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormat {
    // 8-bit formats
    R8Unorm,
    R8Snorm,
    R8Uint,
    R8Sint,
    
    // 16-bit formats
    R16Unorm,
    R16Snorm,
    R16Uint,
    R16Sint,
    R16Float,
    Rg8Unorm,
    Rg8Snorm,
    Rg8Uint,
    Rg8Sint,
    
    // 32-bit formats
    R32Uint,
    R32Sint,
    R32Float,
    Rg16Unorm,
    Rg16Snorm,
    Rg16Uint,
    Rg16Sint,
    Rg16Float,
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Rgba8Snorm,
    Rgba8Uint,
    Rgba8Sint,
    Bgra8Unorm,
    Bgra8UnormSrgb,
    Rgb10a2Unorm,
    Rg11b10Float,
    
    // 64-bit formats
    Rg32Uint,
    Rg32Sint,
    Rg32Float,
    Rgba16Unorm,
    Rgba16Snorm,
    Rgba16Uint,
    Rgba16Sint,
    Rgba16Float,
    
    // 128-bit formats
    Rgba32Uint,
    Rgba32Sint,
    Rgba32Float,
    
    // Depth/stencil formats
    Depth16Unorm,
    Depth24Plus,
    Depth24PlusStencil8,
    Depth32Float,
    Depth32FloatStencil8,
    Stencil8,
    
    // Compressed formats - BC
    Bc1RgbaUnorm,
    Bc1RgbaUnormSrgb,
    Bc2RgbaUnorm,
    Bc2RgbaUnormSrgb,
    Bc3RgbaUnorm,
    Bc3RgbaUnormSrgb,
    Bc4RUnorm,
    Bc4RSnorm,
    Bc5RgUnorm,
    Bc5RgSnorm,
    Bc6hRgbUfloat,
    Bc6hRgbFloat,
    Bc7RgbaUnorm,
    Bc7RgbaUnormSrgb,
    
    // Compressed formats - ASTC
    Astc4x4RgbaUnorm,
    Astc4x4RgbaUnormSrgb,
    Astc5x4RgbaUnorm,
    Astc5x4RgbaUnormSrgb,
    Astc5x5RgbaUnorm,
    Astc5x5RgbaUnormSrgb,
    Astc6x5RgbaUnorm,
    Astc6x5RgbaUnormSrgb,
    Astc6x6RgbaUnorm,
    Astc6x6RgbaUnormSrgb,
    Astc8x5RgbaUnorm,
    Astc8x5RgbaUnormSrgb,
    Astc8x6RgbaUnorm,
    Astc8x6RgbaUnormSrgb,
    Astc8x8RgbaUnorm,
    Astc8x8RgbaUnormSrgb,
    Astc10x5RgbaUnorm,
    Astc10x5RgbaUnormSrgb,
    Astc10x6RgbaUnorm,
    Astc10x6RgbaUnormSrgb,
    Astc10x8RgbaUnorm,
    Astc10x8RgbaUnormSrgb,
    Astc10x10RgbaUnorm,
    Astc10x10RgbaUnormSrgb,
    Astc12x10RgbaUnorm,
    Astc12x10RgbaUnormSrgb,
    Astc12x12RgbaUnorm,
    Astc12x12RgbaUnormSrgb,
}

impl TextureFormat {
    /// Get bytes per pixel (for uncompressed formats).
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8Unorm | Self::R8Snorm | Self::R8Uint | Self::R8Sint | Self::Stencil8 => 1,
            
            Self::R16Unorm | Self::R16Snorm | Self::R16Uint | Self::R16Sint | Self::R16Float
            | Self::Rg8Unorm | Self::Rg8Snorm | Self::Rg8Uint | Self::Rg8Sint
            | Self::Depth16Unorm => 2,
            
            Self::Depth24Plus => 3,
            
            Self::R32Uint | Self::R32Sint | Self::R32Float
            | Self::Rg16Unorm | Self::Rg16Snorm | Self::Rg16Uint | Self::Rg16Sint | Self::Rg16Float
            | Self::Rgba8Unorm | Self::Rgba8UnormSrgb | Self::Rgba8Snorm | Self::Rgba8Uint | Self::Rgba8Sint
            | Self::Bgra8Unorm | Self::Bgra8UnormSrgb
            | Self::Rgb10a2Unorm | Self::Rg11b10Float
            | Self::Depth24PlusStencil8 | Self::Depth32Float => 4,
            
            Self::Depth32FloatStencil8 => 5,
            
            Self::Rg32Uint | Self::Rg32Sint | Self::Rg32Float
            | Self::Rgba16Unorm | Self::Rgba16Snorm | Self::Rgba16Uint | Self::Rgba16Sint | Self::Rgba16Float => 8,
            
            Self::Rgba32Uint | Self::Rgba32Sint | Self::Rgba32Float => 16,
            
            // Compressed formats return block size / pixels per block
            _ => 0,
        }
    }

    /// Check if this is a compressed format.
    pub fn is_compressed(&self) -> bool {
        matches!(
            self,
            Self::Bc1RgbaUnorm | Self::Bc1RgbaUnormSrgb
                | Self::Bc2RgbaUnorm | Self::Bc2RgbaUnormSrgb
                | Self::Bc3RgbaUnorm | Self::Bc3RgbaUnormSrgb
                | Self::Bc4RUnorm | Self::Bc4RSnorm
                | Self::Bc5RgUnorm | Self::Bc5RgSnorm
                | Self::Bc6hRgbUfloat | Self::Bc6hRgbFloat
                | Self::Bc7RgbaUnorm | Self::Bc7RgbaUnormSrgb
                | Self::Astc4x4RgbaUnorm | Self::Astc4x4RgbaUnormSrgb
                | Self::Astc5x4RgbaUnorm | Self::Astc5x4RgbaUnormSrgb
                | Self::Astc5x5RgbaUnorm | Self::Astc5x5RgbaUnormSrgb
                | Self::Astc6x5RgbaUnorm | Self::Astc6x5RgbaUnormSrgb
                | Self::Astc6x6RgbaUnorm | Self::Astc6x6RgbaUnormSrgb
                | Self::Astc8x5RgbaUnorm | Self::Astc8x5RgbaUnormSrgb
                | Self::Astc8x6RgbaUnorm | Self::Astc8x6RgbaUnormSrgb
                | Self::Astc8x8RgbaUnorm | Self::Astc8x8RgbaUnormSrgb
                | Self::Astc10x5RgbaUnorm | Self::Astc10x5RgbaUnormSrgb
                | Self::Astc10x6RgbaUnorm | Self::Astc10x6RgbaUnormSrgb
                | Self::Astc10x8RgbaUnorm | Self::Astc10x8RgbaUnormSrgb
                | Self::Astc10x10RgbaUnorm | Self::Astc10x10RgbaUnormSrgb
                | Self::Astc12x10RgbaUnorm | Self::Astc12x10RgbaUnormSrgb
                | Self::Astc12x12RgbaUnorm | Self::Astc12x12RgbaUnormSrgb
        )
    }

    /// Check if this is a depth format.
    pub fn is_depth(&self) -> bool {
        matches!(
            self,
            Self::Depth16Unorm
                | Self::Depth24Plus
                | Self::Depth24PlusStencil8
                | Self::Depth32Float
                | Self::Depth32FloatStencil8
        )
    }

    /// Check if this is a stencil format.
    pub fn has_stencil(&self) -> bool {
        matches!(
            self,
            Self::Depth24PlusStencil8 | Self::Depth32FloatStencil8 | Self::Stencil8
        )
    }

    /// Check if this is an sRGB format.
    pub fn is_srgb(&self) -> bool {
        matches!(
            self,
            Self::Rgba8UnormSrgb
                | Self::Bgra8UnormSrgb
                | Self::Bc1RgbaUnormSrgb
                | Self::Bc2RgbaUnormSrgb
                | Self::Bc3RgbaUnormSrgb
                | Self::Bc7RgbaUnormSrgb
                | Self::Astc4x4RgbaUnormSrgb
                | Self::Astc5x4RgbaUnormSrgb
                | Self::Astc5x5RgbaUnormSrgb
                | Self::Astc6x5RgbaUnormSrgb
                | Self::Astc6x6RgbaUnormSrgb
                | Self::Astc8x5RgbaUnormSrgb
                | Self::Astc8x6RgbaUnormSrgb
                | Self::Astc8x8RgbaUnormSrgb
                | Self::Astc10x5RgbaUnormSrgb
                | Self::Astc10x6RgbaUnormSrgb
                | Self::Astc10x8RgbaUnormSrgb
                | Self::Astc10x10RgbaUnormSrgb
                | Self::Astc12x10RgbaUnormSrgb
                | Self::Astc12x12RgbaUnormSrgb
        )
    }

    /// Get the linear equivalent of an sRGB format.
    pub fn to_linear(&self) -> Self {
        match self {
            Self::Rgba8UnormSrgb => Self::Rgba8Unorm,
            Self::Bgra8UnormSrgb => Self::Bgra8Unorm,
            Self::Bc1RgbaUnormSrgb => Self::Bc1RgbaUnorm,
            Self::Bc2RgbaUnormSrgb => Self::Bc2RgbaUnorm,
            Self::Bc3RgbaUnormSrgb => Self::Bc3RgbaUnorm,
            Self::Bc7RgbaUnormSrgb => Self::Bc7RgbaUnorm,
            _ => *self,
        }
    }
}

// ============================================================================
// Texture Usage
// ============================================================================

bitflags::bitflags! {
    /// Texture usage flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct TextureUsage: u32 {
        /// Texture can be sampled.
        const SAMPLED = 1 << 0;
        /// Texture can be used as storage.
        const STORAGE = 1 << 1;
        /// Texture can be used as render target.
        const RENDER_TARGET = 1 << 2;
        /// Texture can be used as depth stencil.
        const DEPTH_STENCIL = 1 << 3;
        /// Texture data can be copied from.
        const COPY_SRC = 1 << 4;
        /// Texture data can be copied to.
        const COPY_DST = 1 << 5;
        /// Texture supports mipmaps.
        const MIPMAPPED = 1 << 6;
        /// Transient (aliased memory).
        const TRANSIENT = 1 << 7;
    }
}

impl Default for TextureUsage {
    fn default() -> Self {
        Self::SAMPLED | Self::COPY_DST
    }
}

// ============================================================================
// Texture Dimension
// ============================================================================

/// Texture dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureDimension {
    /// 1D texture.
    D1,
    /// 2D texture.
    #[default]
    D2,
    /// 3D texture.
    D3,
}

/// Texture type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextureType {
    /// Regular 2D texture.
    #[default]
    Texture2D,
    /// 2D array.
    Texture2DArray,
    /// Cube map.
    Cube,
    /// Cube map array.
    CubeArray,
    /// 3D texture.
    Texture3D,
}

// ============================================================================
// Texture Descriptor
// ============================================================================

/// Texture descriptor.
#[derive(Debug, Clone)]
pub struct TextureDesc {
    /// Texture name.
    pub name: String,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Depth or array layers.
    pub depth_or_layers: u32,
    /// Mip levels.
    pub mip_levels: u32,
    /// Sample count.
    pub sample_count: u32,
    /// Format.
    pub format: TextureFormat,
    /// Dimension.
    pub dimension: TextureDimension,
    /// Type.
    pub texture_type: TextureType,
    /// Usage.
    pub usage: TextureUsage,
}

impl Default for TextureDesc {
    fn default() -> Self {
        Self {
            name: String::new(),
            width: 1,
            height: 1,
            depth_or_layers: 1,
            mip_levels: 1,
            sample_count: 1,
            format: TextureFormat::Rgba8Unorm,
            dimension: TextureDimension::D2,
            texture_type: TextureType::Texture2D,
            usage: TextureUsage::default(),
        }
    }
}

impl TextureDesc {
    /// Create a new 2D texture descriptor.
    pub fn new_2d(width: u32, height: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            format,
            ..Default::default()
        }
    }

    /// Create a new 2D array texture descriptor.
    pub fn new_2d_array(width: u32, height: u32, layers: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            depth_or_layers: layers,
            format,
            texture_type: TextureType::Texture2DArray,
            ..Default::default()
        }
    }

    /// Create a new cube map descriptor.
    pub fn new_cube(size: u32, format: TextureFormat) -> Self {
        Self {
            width: size,
            height: size,
            depth_or_layers: 6,
            format,
            texture_type: TextureType::Cube,
            ..Default::default()
        }
    }

    /// Create a new 3D texture descriptor.
    pub fn new_3d(width: u32, height: u32, depth: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            depth_or_layers: depth,
            format,
            dimension: TextureDimension::D3,
            texture_type: TextureType::Texture3D,
            ..Default::default()
        }
    }

    /// Create render target descriptor.
    pub fn render_target(width: u32, height: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            format,
            usage: TextureUsage::RENDER_TARGET | TextureUsage::SAMPLED,
            ..Default::default()
        }
    }

    /// Create depth buffer descriptor.
    pub fn depth_buffer(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            format: TextureFormat::Depth32Float,
            usage: TextureUsage::DEPTH_STENCIL | TextureUsage::SAMPLED,
            ..Default::default()
        }
    }

    /// Set name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set mip levels.
    pub fn mip_levels(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self.usage |= TextureUsage::MIPMAPPED;
        self
    }

    /// Auto-calculate mip levels.
    pub fn auto_mips(mut self) -> Self {
        self.mip_levels = calculate_mip_levels(self.width, self.height);
        self.usage |= TextureUsage::MIPMAPPED;
        self
    }

    /// Set sample count.
    pub fn sample_count(mut self, count: u32) -> Self {
        self.sample_count = count;
        self
    }

    /// Add usage flags.
    pub fn usage(mut self, usage: TextureUsage) -> Self {
        self.usage |= usage;
        self
    }

    /// Calculate memory size.
    pub fn memory_size(&self) -> u64 {
        let bpp = self.format.bytes_per_pixel() as u64;
        let base_size = self.width as u64 * self.height as u64 * self.depth_or_layers as u64 * bpp;

        if self.mip_levels > 1 {
            // Approximate: full mip chain is ~1.33x base size
            (base_size * 4) / 3
        } else {
            base_size
        }
    }
}

/// Calculate mip levels for a texture.
pub fn calculate_mip_levels(width: u32, height: u32) -> u32 {
    let max_dim = width.max(height);
    (32 - max_dim.leading_zeros()).max(1)
}

// ============================================================================
// Texture View
// ============================================================================

/// Handle to a texture view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureViewHandle {
    /// Index.
    index: u32,
    /// Generation.
    generation: u32,
}

impl TextureViewHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };
}

/// Texture view descriptor.
#[derive(Debug, Clone)]
pub struct TextureViewDesc {
    /// Base mip level.
    pub base_mip: u32,
    /// Mip level count.
    pub mip_count: u32,
    /// Base array layer.
    pub base_layer: u32,
    /// Array layer count.
    pub layer_count: u32,
    /// View type override.
    pub view_type: Option<TextureType>,
    /// Format override.
    pub format: Option<TextureFormat>,
}

impl Default for TextureViewDesc {
    fn default() -> Self {
        Self {
            base_mip: 0,
            mip_count: u32::MAX, // All mips
            base_layer: 0,
            layer_count: u32::MAX, // All layers
            view_type: None,
            format: None,
        }
    }
}

impl TextureViewDesc {
    /// Create a single mip view.
    pub fn single_mip(level: u32) -> Self {
        Self {
            base_mip: level,
            mip_count: 1,
            ..Default::default()
        }
    }

    /// Create a single layer view.
    pub fn single_layer(layer: u32) -> Self {
        Self {
            base_layer: layer,
            layer_count: 1,
            ..Default::default()
        }
    }

    /// Create a cube face view.
    pub fn cube_face(face: u32) -> Self {
        Self {
            base_layer: face,
            layer_count: 1,
            view_type: Some(TextureType::Texture2D),
            ..Default::default()
        }
    }
}

/// Texture view.
pub struct TextureView {
    /// Handle.
    handle: TextureViewHandle,
    /// Source texture.
    texture: TextureHandle,
    /// Descriptor.
    desc: TextureViewDesc,
}

impl TextureView {
    /// Create a new view.
    pub fn new(handle: TextureViewHandle, texture: TextureHandle, desc: TextureViewDesc) -> Self {
        Self {
            handle,
            texture,
            desc,
        }
    }

    /// Get handle.
    pub fn handle(&self) -> TextureViewHandle {
        self.handle
    }

    /// Get source texture.
    pub fn texture(&self) -> TextureHandle {
        self.texture
    }

    /// Get descriptor.
    pub fn desc(&self) -> &TextureViewDesc {
        &self.desc
    }
}

// ============================================================================
// Texture
// ============================================================================

/// Texture resource.
pub struct Texture {
    /// Handle.
    handle: TextureHandle,
    /// Descriptor.
    desc: TextureDesc,
    /// Current state.
    state: TextureState,
    /// Bindless index.
    bindless_index: Option<u32>,
}

/// Texture state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureState {
    /// Undefined state.
    Undefined,
    /// Ready for sampling.
    ShaderRead,
    /// Ready for storage write.
    ShaderWrite,
    /// Render target.
    RenderTarget,
    /// Depth write.
    DepthWrite,
    /// Depth read.
    DepthRead,
    /// Copy source.
    CopySrc,
    /// Copy destination.
    CopyDst,
    /// Present.
    Present,
}

impl Texture {
    /// Create a new texture.
    pub fn new(handle: TextureHandle, desc: TextureDesc) -> Self {
        Self {
            handle,
            desc,
            state: TextureState::Undefined,
            bindless_index: None,
        }
    }

    /// Get handle.
    pub fn handle(&self) -> TextureHandle {
        self.handle
    }

    /// Get descriptor.
    pub fn desc(&self) -> &TextureDesc {
        &self.desc
    }

    /// Get width.
    pub fn width(&self) -> u32 {
        self.desc.width
    }

    /// Get height.
    pub fn height(&self) -> u32 {
        self.desc.height
    }

    /// Get format.
    pub fn format(&self) -> TextureFormat {
        self.desc.format
    }

    /// Get state.
    pub fn state(&self) -> TextureState {
        self.state
    }

    /// Set state.
    pub fn set_state(&mut self, state: TextureState) {
        self.state = state;
    }

    /// Get bindless index.
    pub fn bindless_index(&self) -> Option<u32> {
        self.bindless_index
    }

    /// Set bindless index.
    pub fn set_bindless_index(&mut self, index: u32) {
        self.bindless_index = Some(index);
    }

    /// Get mip size.
    pub fn mip_size(&self, level: u32) -> (u32, u32) {
        let w = (self.desc.width >> level).max(1);
        let h = (self.desc.height >> level).max(1);
        (w, h)
    }
}

// ============================================================================
// Texture Manager
// ============================================================================

/// Texture slot.
struct TextureSlot {
    texture: Option<Texture>,
    generation: u32,
}

/// Texture manager.
pub struct TextureManager {
    /// Textures.
    textures: Vec<TextureSlot>,
    /// Free list.
    free_list: Vec<u32>,
    /// Name map.
    name_map: BTreeMap<String, TextureHandle>,
    /// Next generation.
    next_generation: AtomicU32,
    /// Default textures.
    defaults: DefaultTextures,
    /// Stats.
    stats: TextureStats,
}

/// Default textures.
#[derive(Debug, Clone, Copy)]
pub struct DefaultTextures {
    /// White texture.
    pub white: TextureHandle,
    /// Black texture.
    pub black: TextureHandle,
    /// Normal map (flat).
    pub normal: TextureHandle,
    /// Fallback texture (magenta checkerboard).
    pub fallback: TextureHandle,
}

impl Default for DefaultTextures {
    fn default() -> Self {
        Self {
            white: TextureHandle::INVALID,
            black: TextureHandle::INVALID,
            normal: TextureHandle::INVALID,
            fallback: TextureHandle::INVALID,
        }
    }
}

/// Texture statistics.
#[derive(Debug, Clone, Default)]
pub struct TextureStats {
    /// Total textures.
    pub total: u32,
    /// Total memory (bytes).
    pub memory: u64,
    /// Compressed textures.
    pub compressed: u32,
    /// Render targets.
    pub render_targets: u32,
}

impl TextureManager {
    /// Create a new manager.
    pub fn new(capacity: u32) -> Self {
        let mut manager = Self {
            textures: Vec::with_capacity(capacity as usize),
            free_list: Vec::new(),
            name_map: BTreeMap::new(),
            next_generation: AtomicU32::new(1),
            defaults: DefaultTextures::default(),
            stats: TextureStats::default(),
        };

        // Create default textures
        manager.create_default_textures();
        manager
    }

    /// Create default textures.
    fn create_default_textures(&mut self) {
        // White
        let desc = TextureDesc::new_2d(1, 1, TextureFormat::Rgba8Unorm).name("white");
        self.defaults.white = self.create(desc).unwrap_or(TextureHandle::INVALID);

        // Black
        let desc = TextureDesc::new_2d(1, 1, TextureFormat::Rgba8Unorm).name("black");
        self.defaults.black = self.create(desc).unwrap_or(TextureHandle::INVALID);

        // Normal
        let desc = TextureDesc::new_2d(1, 1, TextureFormat::Rgba8Unorm).name("normal");
        self.defaults.normal = self.create(desc).unwrap_or(TextureHandle::INVALID);

        // Fallback
        let desc = TextureDesc::new_2d(2, 2, TextureFormat::Rgba8Unorm).name("fallback");
        self.defaults.fallback = self.create(desc).unwrap_or(TextureHandle::INVALID);
    }

    /// Create a texture.
    pub fn create(&mut self, desc: TextureDesc) -> Option<TextureHandle> {
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);

        let index = if let Some(index) = self.free_list.pop() {
            let slot = &mut self.textures[index as usize];
            slot.generation = generation;
            index
        } else {
            let index = self.textures.len() as u32;
            self.textures.push(TextureSlot {
                texture: None,
                generation,
            });
            index
        };

        let handle = TextureHandle::new(index, generation);
        let name = desc.name.clone();

        let texture = Texture::new(handle, desc);
        self.textures[index as usize].texture = Some(texture);

        if !name.is_empty() {
            self.name_map.insert(name, handle);
        }

        self.update_stats();
        Some(handle)
    }

    /// Get texture.
    pub fn get(&self, handle: TextureHandle) -> Option<&Texture> {
        let slot = self.textures.get(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.texture.as_ref()
    }

    /// Get mutable texture.
    pub fn get_mut(&mut self, handle: TextureHandle) -> Option<&mut Texture> {
        let slot = self.textures.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.texture.as_mut()
    }

    /// Get texture by name.
    pub fn get_by_name(&self, name: &str) -> Option<&Texture> {
        let handle = *self.name_map.get(name)?;
        self.get(handle)
    }

    /// Get handle by name.
    pub fn handle_by_name(&self, name: &str) -> Option<TextureHandle> {
        self.name_map.get(name).copied()
    }

    /// Destroy texture.
    pub fn destroy(&mut self, handle: TextureHandle) {
        if let Some(slot) = self.textures.get_mut(handle.index as usize) {
            if slot.generation == handle.generation {
                if let Some(texture) = slot.texture.take() {
                    self.name_map.remove(&texture.desc.name);
                }
                self.free_list.push(handle.index);
                self.update_stats();
            }
        }
    }

    /// Get default textures.
    pub fn defaults(&self) -> &DefaultTextures {
        &self.defaults
    }

    /// Get stats.
    pub fn stats(&self) -> &TextureStats {
        &self.stats
    }

    /// Update stats.
    fn update_stats(&mut self) {
        let mut stats = TextureStats::default();
        for slot in &self.textures {
            if let Some(texture) = &slot.texture {
                stats.total += 1;
                stats.memory += texture.desc.memory_size();
                if texture.desc.format.is_compressed() {
                    stats.compressed += 1;
                }
                if texture.desc.usage.contains(TextureUsage::RENDER_TARGET) {
                    stats.render_targets += 1;
                }
            }
        }
        self.stats = stats;
    }
}

// ============================================================================
// Texture Streaming
// ============================================================================

/// Texture streaming priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum StreamingPriority {
    /// Low priority.
    Low = 0,
    /// Normal priority.
    Normal = 1,
    /// High priority.
    High = 2,
    /// Critical (needed immediately).
    Critical = 3,
}

/// Streaming request.
#[derive(Debug, Clone)]
pub struct StreamingRequest {
    /// Texture handle.
    pub texture: TextureHandle,
    /// Target mip level.
    pub target_mip: u32,
    /// Priority.
    pub priority: StreamingPriority,
    /// Distance to camera.
    pub distance: f32,
    /// Screen coverage.
    pub screen_coverage: f32,
}

/// Texture streaming state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamingState {
    /// Not loaded.
    NotLoaded,
    /// Loading.
    Loading,
    /// Partially loaded.
    Partial { current_mip: u32, target_mip: u32 },
    /// Fully loaded.
    FullyLoaded,
}

/// Streaming budget.
#[derive(Debug, Clone)]
pub struct StreamingBudget {
    /// Memory budget (bytes).
    pub memory_budget: u64,
    /// Current memory usage (bytes).
    pub memory_used: u64,
    /// Bandwidth budget (bytes per frame).
    pub bandwidth_budget: u64,
}

impl StreamingBudget {
    /// Check if within budget.
    pub fn within_budget(&self) -> bool {
        self.memory_used <= self.memory_budget
    }

    /// Get remaining memory.
    pub fn remaining_memory(&self) -> u64 {
        self.memory_budget.saturating_sub(self.memory_used)
    }
}
