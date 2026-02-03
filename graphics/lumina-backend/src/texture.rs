//! GPU Texture Management
//!
//! Texture creation, views, and format support.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

use crate::device::TextureFormat;

// ============================================================================
// Texture Dimension
// ============================================================================

/// Texture dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureDimension {
    /// 1D texture.
    D1,
    /// 2D texture.
    D2,
    /// 3D texture.
    D3,
}

impl TextureDimension {
    /// Get view dimension.
    pub fn default_view_dimension(&self) -> TextureViewDimension {
        match self {
            TextureDimension::D1 => TextureViewDimension::D1,
            TextureDimension::D2 => TextureViewDimension::D2,
            TextureDimension::D3 => TextureViewDimension::D3,
        }
    }
}

impl Default for TextureDimension {
    fn default() -> Self {
        TextureDimension::D2
    }
}

// ============================================================================
// Texture View Dimension
// ============================================================================

/// Texture view dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureViewDimension {
    /// 1D texture view.
    D1,
    /// 2D texture view.
    D2,
    /// 2D array texture view.
    D2Array,
    /// Cube texture view.
    Cube,
    /// Cube array texture view.
    CubeArray,
    /// 3D texture view.
    D3,
}

impl Default for TextureViewDimension {
    fn default() -> Self {
        TextureViewDimension::D2
    }
}

// ============================================================================
// Texture Usage
// ============================================================================

bitflags! {
    /// Texture usage flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TextureUsage: u32 {
        /// Can be used as copy source.
        const COPY_SRC = 1 << 0;
        /// Can be used as copy destination.
        const COPY_DST = 1 << 1;
        /// Can be sampled in shaders.
        const TEXTURE_BINDING = 1 << 2;
        /// Can be used as storage texture.
        const STORAGE_BINDING = 1 << 3;
        /// Can be used as color attachment.
        const COLOR_ATTACHMENT = 1 << 4;
        /// Can be used as depth/stencil attachment.
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
        /// Can be used as input attachment.
        const INPUT_ATTACHMENT = 1 << 6;
        /// Transient attachment (tile-based).
        const TRANSIENT_ATTACHMENT = 1 << 7;
        /// Can be used for shading rate.
        const SHADING_RATE = 1 << 8;
    }
}

impl Default for TextureUsage {
    fn default() -> Self {
        TextureUsage::TEXTURE_BINDING | TextureUsage::COPY_DST
    }
}

// ============================================================================
// Sample Count
// ============================================================================

/// Multisampling sample count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum SampleCount {
    /// 1 sample (no multisampling).
    S1  = 1,
    /// 2 samples.
    S2  = 2,
    /// 4 samples.
    S4  = 4,
    /// 8 samples.
    S8  = 8,
    /// 16 samples.
    S16 = 16,
    /// 32 samples.
    S32 = 32,
    /// 64 samples.
    S64 = 64,
}

impl SampleCount {
    /// Get count as u32.
    pub fn count(&self) -> u32 {
        *self as u32
    }
}

impl Default for SampleCount {
    fn default() -> Self {
        SampleCount::S1
    }
}

// ============================================================================
// Texture Description
// ============================================================================

/// Description for texture creation.
#[derive(Debug, Clone)]
pub struct TextureDesc {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Depth (for 3D) or array layers.
    pub depth_or_layers: u32,
    /// Format.
    pub format: TextureFormat,
    /// Dimension.
    pub dimension: TextureDimension,
    /// Mip levels (0 = full chain).
    pub mip_levels: u32,
    /// Sample count.
    pub sample_count: SampleCount,
    /// Usage flags.
    pub usage: TextureUsage,
    /// Debug label.
    pub label: Option<String>,
}

impl TextureDesc {
    /// Create a new 2D texture description.
    pub fn new_2d(width: u32, height: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            depth_or_layers: 1,
            format,
            dimension: TextureDimension::D2,
            mip_levels: 1,
            sample_count: SampleCount::S1,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::COPY_DST,
            label: None,
        }
    }

    /// Create render target description.
    pub fn render_target(width: u32, height: u32, format: TextureFormat) -> Self {
        Self::new_2d(width, height, format)
            .with_usage(TextureUsage::COLOR_ATTACHMENT | TextureUsage::TEXTURE_BINDING)
    }

    /// Create depth target description.
    pub fn depth_target(width: u32, height: u32, format: TextureFormat) -> Self {
        Self::new_2d(width, height, format)
            .with_usage(TextureUsage::DEPTH_STENCIL_ATTACHMENT | TextureUsage::TEXTURE_BINDING)
    }

    /// Create storage texture description.
    pub fn storage(width: u32, height: u32, format: TextureFormat) -> Self {
        Self::new_2d(width, height, format)
            .with_usage(TextureUsage::STORAGE_BINDING | TextureUsage::TEXTURE_BINDING)
    }

    /// Create cube texture description.
    pub fn cube(size: u32, format: TextureFormat) -> Self {
        Self {
            width: size,
            height: size,
            depth_or_layers: 6,
            format,
            dimension: TextureDimension::D2,
            mip_levels: 1,
            sample_count: SampleCount::S1,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::COPY_DST,
            label: None,
        }
    }

    /// Create 3D texture description.
    pub fn new_3d(width: u32, height: u32, depth: u32, format: TextureFormat) -> Self {
        Self {
            width,
            height,
            depth_or_layers: depth,
            format,
            dimension: TextureDimension::D3,
            mip_levels: 1,
            sample_count: SampleCount::S1,
            usage: TextureUsage::TEXTURE_BINDING | TextureUsage::COPY_DST,
            label: None,
        }
    }

    /// Set usage.
    pub fn with_usage(mut self, usage: TextureUsage) -> Self {
        self.usage = usage;
        self
    }

    /// Set mip levels.
    pub fn with_mip_levels(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// Set sample count.
    pub fn with_sample_count(mut self, count: SampleCount) -> Self {
        self.sample_count = count;
        self
    }

    /// Set label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Calculate max mip levels.
    pub fn max_mip_levels(&self) -> u32 {
        let max_dim = self
            .width
            .max(self.height)
            .max(if self.dimension == TextureDimension::D3 {
                self.depth_or_layers
            } else {
                1
            });
        (32 - max_dim.leading_zeros()).max(1)
    }

    /// Get actual mip levels.
    pub fn actual_mip_levels(&self) -> u32 {
        if self.mip_levels == 0 {
            self.max_mip_levels()
        } else {
            self.mip_levels
        }
    }
}

// ============================================================================
// Texture Handle
// ============================================================================

/// Handle to a texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(Handle<Texture>);

impl TextureHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.0.generation()
    }
}

// ============================================================================
// Texture State
// ============================================================================

/// Texture resource state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureState {
    /// Undefined state.
    Undefined,
    /// General state.
    General,
    /// Color attachment.
    ColorAttachment,
    /// Depth/stencil attachment.
    DepthStencilAttachment,
    /// Depth/stencil read-only.
    DepthStencilReadOnly,
    /// Shader read-only.
    ShaderReadOnly,
    /// Copy source.
    CopySrc,
    /// Copy destination.
    CopyDst,
    /// Present.
    Present,
    /// Resolve source.
    ResolveSrc,
    /// Resolve destination.
    ResolveDst,
    /// Shading rate.
    ShadingRate,
}

// ============================================================================
// Texture
// ============================================================================

/// A GPU texture.
pub struct Texture {
    /// Handle.
    pub handle: TextureHandle,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Depth or array layers.
    pub depth_or_layers: u32,
    /// Format.
    pub format: TextureFormat,
    /// Dimension.
    pub dimension: TextureDimension,
    /// Mip levels.
    pub mip_levels: u32,
    /// Sample count.
    pub sample_count: SampleCount,
    /// Usage.
    pub usage: TextureUsage,
    /// Current state.
    pub state: TextureState,
    /// Debug label.
    pub label: Option<String>,
}

impl Texture {
    /// Create a new texture.
    pub fn new(handle: TextureHandle, desc: &TextureDesc) -> Self {
        Self {
            handle,
            width: desc.width,
            height: desc.height,
            depth_or_layers: desc.depth_or_layers,
            format: desc.format,
            dimension: desc.dimension,
            mip_levels: desc.actual_mip_levels(),
            sample_count: desc.sample_count,
            usage: desc.usage,
            state: TextureState::Undefined,
            label: desc.label.clone(),
        }
    }

    /// Get size at mip level.
    pub fn mip_size(&self, level: u32) -> (u32, u32, u32) {
        let width = (self.width >> level).max(1);
        let height = (self.height >> level).max(1);
        let depth = if self.dimension == TextureDimension::D3 {
            (self.depth_or_layers >> level).max(1)
        } else {
            self.depth_or_layers
        };
        (width, height, depth)
    }

    /// Check if multisampled.
    pub fn is_multisampled(&self) -> bool {
        self.sample_count.count() > 1
    }

    /// Check if has depth.
    pub fn is_depth(&self) -> bool {
        self.format.is_depth()
    }

    /// Get aspect ratio.
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

// ============================================================================
// Texture View Description
// ============================================================================

/// Description for texture view creation.
#[derive(Debug, Clone)]
pub struct TextureViewDesc {
    /// Format (None = texture format).
    pub format: Option<TextureFormat>,
    /// View dimension.
    pub dimension: TextureViewDimension,
    /// Base mip level.
    pub base_mip_level: u32,
    /// Mip level count (0 = remaining).
    pub mip_level_count: u32,
    /// Base array layer.
    pub base_array_layer: u32,
    /// Array layer count (0 = remaining).
    pub array_layer_count: u32,
    /// Debug label.
    pub label: Option<String>,
}

impl Default for TextureViewDesc {
    fn default() -> Self {
        Self {
            format: None,
            dimension: TextureViewDimension::D2,
            base_mip_level: 0,
            mip_level_count: 0,
            base_array_layer: 0,
            array_layer_count: 0,
            label: None,
        }
    }
}

// ============================================================================
// Texture View Handle
// ============================================================================

/// Handle to a texture view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureViewHandle(Handle<TextureView>);

impl TextureViewHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }
}

// ============================================================================
// Texture View
// ============================================================================

/// A view into a texture.
pub struct TextureView {
    /// Handle.
    pub handle: TextureViewHandle,
    /// Source texture.
    pub texture: TextureHandle,
    /// Format.
    pub format: TextureFormat,
    /// Dimension.
    pub dimension: TextureViewDimension,
    /// Base mip level.
    pub base_mip_level: u32,
    /// Mip level count.
    pub mip_level_count: u32,
    /// Base array layer.
    pub base_array_layer: u32,
    /// Array layer count.
    pub array_layer_count: u32,
}

impl TextureView {
    /// Create a new view.
    pub fn new(handle: TextureViewHandle, texture: &Texture, desc: &TextureViewDesc) -> Self {
        Self {
            handle,
            texture: texture.handle,
            format: desc.format.unwrap_or(texture.format),
            dimension: desc.dimension,
            base_mip_level: desc.base_mip_level,
            mip_level_count: if desc.mip_level_count == 0 {
                texture.mip_levels - desc.base_mip_level
            } else {
                desc.mip_level_count
            },
            base_array_layer: desc.base_array_layer,
            array_layer_count: if desc.array_layer_count == 0 {
                texture.depth_or_layers - desc.base_array_layer
            } else {
                desc.array_layer_count
            },
        }
    }
}

// ============================================================================
// Texture Manager
// ============================================================================

/// Manages texture resources.
pub struct TextureManager {
    /// Textures.
    textures: Vec<Option<Texture>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Total memory used.
    memory_used: AtomicU64,
    /// Texture count.
    texture_count: u32,
}

impl TextureManager {
    /// Create a new texture manager.
    pub fn new() -> Self {
        Self {
            textures: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            memory_used: AtomicU64::new(0),
            texture_count: 0,
        }
    }

    /// Create a texture.
    pub fn create(&mut self, desc: &TextureDesc) -> TextureHandle {
        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.textures.len() as u32;
            self.textures.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = TextureHandle::new(index, generation);
        let texture = Texture::new(handle, desc);

        // Calculate memory size
        let size = self.calculate_texture_size(desc);
        self.memory_used.fetch_add(size, Ordering::Relaxed);

        self.textures[index as usize] = Some(texture);
        self.texture_count += 1;

        handle
    }

    /// Calculate texture memory size.
    fn calculate_texture_size(&self, desc: &TextureDesc) -> u64 {
        let bpp = desc.format.bytes_per_pixel() as u64;
        let layers = desc.depth_or_layers as u64;
        let samples = desc.sample_count.count() as u64;
        let mips = desc.actual_mip_levels();

        let mut size = 0u64;
        for mip in 0..mips {
            let w = ((desc.width >> mip) as u64).max(1);
            let h = ((desc.height >> mip) as u64).max(1);
            size += w * h * bpp * layers * samples;
        }

        size
    }

    /// Get a texture.
    pub fn get(&self, handle: TextureHandle) -> Option<&Texture> {
        let index = handle.index() as usize;
        if index >= self.textures.len() {
            return None;
        }
        if self.generations.get(index) != Some(&handle.generation()) {
            return None;
        }
        self.textures[index].as_ref()
    }

    /// Get mutable texture.
    pub fn get_mut(&mut self, handle: TextureHandle) -> Option<&mut Texture> {
        let index = handle.index() as usize;
        if index >= self.textures.len() {
            return None;
        }
        if self.generations.get(index) != Some(&handle.generation()) {
            return None;
        }
        self.textures[index].as_mut()
    }

    /// Destroy a texture.
    pub fn destroy(&mut self, handle: TextureHandle) {
        let index = handle.index() as usize;
        if index >= self.textures.len() {
            return;
        }
        if self.generations.get(index) != Some(&handle.generation()) {
            return;
        }

        if let Some(_texture) = self.textures[index].take() {
            // Note: Would need to track individual texture sizes for proper accounting
            self.texture_count -= 1;
        }

        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);
    }

    /// Get total memory used.
    pub fn memory_used(&self) -> u64 {
        self.memory_used.load(Ordering::Relaxed)
    }

    /// Get texture count.
    pub fn count(&self) -> u32 {
        self.texture_count
    }
}

impl Default for TextureManager {
    fn default() -> Self {
        Self::new()
    }
}
