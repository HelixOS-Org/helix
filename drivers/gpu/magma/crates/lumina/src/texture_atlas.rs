//! Texture Atlas Types for Lumina
//!
//! This module provides texture atlas management including
//! sprite atlases, font atlases, and dynamic atlas packing.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Atlas Handles
// ============================================================================

/// Texture atlas handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextureAtlasHandle(pub u64);

impl TextureAtlasHandle {
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

impl Default for TextureAtlasHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Atlas region handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AtlasRegionHandle(pub u64);

impl AtlasRegionHandle {
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

impl Default for AtlasRegionHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sprite sheet handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SpriteSheetHandle(pub u64);

impl SpriteSheetHandle {
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

impl Default for SpriteSheetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Texture Atlas
// ============================================================================

/// Texture atlas create info
#[derive(Clone, Debug)]
pub struct TextureAtlasCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Format
    pub format: AtlasFormat,
    /// Packing algorithm
    pub algorithm: PackingAlgorithm,
    /// Padding between regions
    pub padding: u32,
    /// Allow growth
    pub allow_growth: bool,
    /// Max pages (for array textures)
    pub max_pages: u32,
    /// Generate mipmaps
    pub mipmaps: bool,
}

impl TextureAtlasCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            format: AtlasFormat::Rgba8,
            algorithm: PackingAlgorithm::MaxRects,
            padding: 1,
            allow_growth: true,
            max_pages: 16,
            mipmaps: true,
        }
    }

    /// 1K atlas
    pub fn atlas_1k() -> Self {
        Self::new(1024, 1024)
    }

    /// 2K atlas
    pub fn atlas_2k() -> Self {
        Self::new(2048, 2048)
    }

    /// 4K atlas
    pub fn atlas_4k() -> Self {
        Self::new(4096, 4096)
    }

    /// Sprite atlas (RGBA)
    pub fn sprite_atlas(size: u32) -> Self {
        Self {
            format: AtlasFormat::Rgba8,
            padding: 2,
            ..Self::new(size, size)
        }
    }

    /// Font atlas (single channel)
    pub fn font_atlas(size: u32) -> Self {
        Self {
            format: AtlasFormat::R8,
            padding: 1,
            algorithm: PackingAlgorithm::Shelf,
            ..Self::new(size, size)
        }
    }

    /// Normal map atlas
    pub fn normal_atlas(size: u32) -> Self {
        Self {
            format: AtlasFormat::Rg8,
            padding: 1,
            ..Self::new(size, size)
        }
    }

    /// With format
    pub fn with_format(mut self, format: AtlasFormat) -> Self {
        self.format = format;
        self
    }

    /// With padding
    pub fn with_padding(mut self, padding: u32) -> Self {
        self.padding = padding;
        self
    }

    /// Without growth
    pub fn without_growth(mut self) -> Self {
        self.allow_growth = false;
        self
    }

    /// Without mipmaps
    pub fn without_mipmaps(mut self) -> Self {
        self.mipmaps = false;
        self
    }
}

impl Default for TextureAtlasCreateInfo {
    fn default() -> Self {
        Self::atlas_2k()
    }
}

/// Atlas format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AtlasFormat {
    /// Single channel (8-bit)
    R8 = 0,
    /// Two channels (8-bit each)
    Rg8 = 1,
    /// RGBA (8-bit each)
    #[default]
    Rgba8 = 2,
    /// RGBA sRGB
    Rgba8Srgb = 3,
    /// BC1 compressed
    Bc1 = 4,
    /// BC3 compressed
    Bc3 = 5,
    /// BC7 compressed
    Bc7 = 6,
}

impl AtlasFormat {
    /// Bytes per pixel (uncompressed)
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8 => 1,
            Self::Rg8 => 2,
            Self::Rgba8 | Self::Rgba8Srgb => 4,
            Self::Bc1 => 8, // 8 bytes per 4x4 block
            Self::Bc3 | Self::Bc7 => 16, // 16 bytes per 4x4 block
        }
    }

    /// Is compressed
    pub const fn is_compressed(&self) -> bool {
        matches!(self, Self::Bc1 | Self::Bc3 | Self::Bc7)
    }

    /// Channel count
    pub const fn channels(&self) -> u32 {
        match self {
            Self::R8 | Self::Bc1 => 1,
            Self::Rg8 => 2,
            Self::Rgba8 | Self::Rgba8Srgb | Self::Bc3 | Self::Bc7 => 4,
        }
    }
}

/// Packing algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PackingAlgorithm {
    /// MaxRects (best quality)
    #[default]
    MaxRects = 0,
    /// Shelf (fast, good for similar sized items)
    Shelf = 1,
    /// Guillotine
    Guillotine = 2,
    /// Skyline
    Skyline = 3,
    /// Grid (fixed cell size)
    Grid = 4,
}

impl PackingAlgorithm {
    /// Is suitable for dynamic updates
    pub const fn is_dynamic(&self) -> bool {
        matches!(self, Self::Shelf | Self::Skyline)
    }
}

// ============================================================================
// Atlas Region
// ============================================================================

/// Atlas region
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AtlasRegion {
    /// X position in atlas
    pub x: u32,
    /// Y position in atlas
    pub y: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Page index (for array textures)
    pub page: u32,
    /// Rotation (90 degrees)
    pub rotated: bool,
}

impl AtlasRegion {
    /// Creates region
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            page: 0,
            rotated: false,
        }
    }

    /// Get UV coordinates (normalized)
    pub fn uv(&self, atlas_width: u32, atlas_height: u32) -> AtlasUv {
        let inv_w = 1.0 / atlas_width as f32;
        let inv_h = 1.0 / atlas_height as f32;

        AtlasUv {
            u0: self.x as f32 * inv_w,
            v0: self.y as f32 * inv_h,
            u1: (self.x + self.width) as f32 * inv_w,
            v1: (self.y + self.height) as f32 * inv_h,
            page: self.page,
            rotated: self.rotated,
        }
    }

    /// Get padded UV (half-pixel inset)
    pub fn uv_padded(&self, atlas_width: u32, atlas_height: u32, half_pixel: f32) -> AtlasUv {
        let mut uv = self.uv(atlas_width, atlas_height);
        uv.u0 += half_pixel / atlas_width as f32;
        uv.v0 += half_pixel / atlas_height as f32;
        uv.u1 -= half_pixel / atlas_width as f32;
        uv.v1 -= half_pixel / atlas_height as f32;
        uv
    }

    /// Area in pixels
    pub const fn area(&self) -> u32 {
        self.width * self.height
    }
}

/// Atlas UV coordinates
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AtlasUv {
    /// U min
    pub u0: f32,
    /// V min
    pub v0: f32,
    /// U max
    pub u1: f32,
    /// V max
    pub v1: f32,
    /// Page index
    pub page: u32,
    /// Is rotated
    pub rotated: bool,
}

impl AtlasUv {
    /// Creates UV
    pub const fn new(u0: f32, v0: f32, u1: f32, v1: f32) -> Self {
        Self {
            u0,
            v0,
            u1,
            v1,
            page: 0,
            rotated: false,
        }
    }

    /// Full texture
    pub const fn full() -> Self {
        Self::new(0.0, 0.0, 1.0, 1.0)
    }

    /// UV width
    pub fn width(&self) -> f32 {
        self.u1 - self.u0
    }

    /// UV height
    pub fn height(&self) -> f32 {
        self.v1 - self.v0
    }

    /// Sample at normalized position
    pub fn sample(&self, u: f32, v: f32) -> [f32; 2] {
        if self.rotated {
            [self.u0 + v * self.width(), self.v0 + (1.0 - u) * self.height()]
        } else {
            [self.u0 + u * self.width(), self.v0 + v * self.height()]
        }
    }
}

// ============================================================================
// Atlas Builder
// ============================================================================

/// Atlas builder settings
#[derive(Clone, Debug)]
pub struct AtlasBuilderSettings {
    /// Max width
    pub max_width: u32,
    /// Max height
    pub max_height: u32,
    /// Power of two
    pub power_of_two: bool,
    /// Square
    pub square: bool,
    /// Border mode
    pub border_mode: BorderMode,
    /// Trim transparent pixels
    pub trim: bool,
    /// Extrude edges (for bleeding prevention)
    pub extrude: u32,
}

impl AtlasBuilderSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            max_width: 4096,
            max_height: 4096,
            power_of_two: true,
            square: false,
            border_mode: BorderMode::Transparent,
            trim: true,
            extrude: 1,
        }
    }

    /// For sprites (with extrusion)
    pub fn sprites() -> Self {
        Self {
            extrude: 2,
            trim: true,
            ..Self::new()
        }
    }

    /// For UI (no extrusion needed)
    pub fn ui() -> Self {
        Self {
            extrude: 0,
            trim: false,
            ..Self::new()
        }
    }

    /// With max size
    pub fn with_max_size(mut self, width: u32, height: u32) -> Self {
        self.max_width = width;
        self.max_height = height;
        self
    }

    /// Require square
    pub fn require_square(mut self) -> Self {
        self.square = true;
        self
    }
}

impl Default for AtlasBuilderSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Border mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BorderMode {
    /// Transparent
    #[default]
    Transparent = 0,
    /// Clamp edge
    Clamp = 1,
    /// Solid color
    Solid = 2,
}

/// Atlas entry (for building)
#[derive(Clone, Debug)]
pub struct AtlasEntry {
    /// Entry name/ID
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Data (RGBA bytes)
    pub data: Vec<u8>,
    /// User data
    pub user_data: u64,
}

impl AtlasEntry {
    /// Creates entry
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        Self {
            name: String::from(name),
            width,
            height,
            data: Vec::new(),
            user_data: 0,
        }
    }

    /// With data
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }
}

impl Default for AtlasEntry {
    fn default() -> Self {
        Self::new("entry", 0, 0)
    }
}

/// Atlas build result
#[derive(Clone, Debug)]
pub struct AtlasBuildResult {
    /// Atlas width
    pub width: u32,
    /// Atlas height
    pub height: u32,
    /// Pages
    pub page_count: u32,
    /// Regions
    pub regions: Vec<(String, AtlasRegion)>,
    /// Efficiency (0-1)
    pub efficiency: f32,
}

impl AtlasBuildResult {
    /// Creates result
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            page_count: 1,
            regions: Vec::new(),
            efficiency: 0.0,
        }
    }

    /// Find region by name
    pub fn find(&self, name: &str) -> Option<&AtlasRegion> {
        self.regions
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| r)
    }
}

impl Default for AtlasBuildResult {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

// ============================================================================
// Sprite Sheet
// ============================================================================

/// Sprite sheet create info
#[derive(Clone, Debug)]
pub struct SpriteSheetCreateInfo {
    /// Name
    pub name: String,
    /// Atlas handle
    pub atlas: TextureAtlasHandle,
    /// Sprites
    pub sprites: Vec<SpriteDefinition>,
    /// Animations
    pub animations: Vec<SpriteAnimation>,
}

impl SpriteSheetCreateInfo {
    /// Creates info
    pub fn new(atlas: TextureAtlasHandle) -> Self {
        Self {
            name: String::new(),
            atlas,
            sprites: Vec::new(),
            animations: Vec::new(),
        }
    }

    /// From grid (uniform grid of sprites)
    pub fn from_grid(
        atlas: TextureAtlasHandle,
        cell_width: u32,
        cell_height: u32,
        columns: u32,
        rows: u32,
    ) -> Self {
        let mut sprites = Vec::new();

        for row in 0..rows {
            for col in 0..columns {
                let idx = row * columns + col;
                sprites.push(SpriteDefinition {
                    name: alloc::format!("sprite_{}", idx),
                    region: AtlasRegion::new(
                        col * cell_width,
                        row * cell_height,
                        cell_width,
                        cell_height,
                    ),
                    pivot: [0.5, 0.5],
                    border: [0, 0, 0, 0],
                });
            }
        }

        Self {
            sprites,
            ..Self::new(atlas)
        }
    }

    /// Add sprite
    pub fn with_sprite(mut self, sprite: SpriteDefinition) -> Self {
        self.sprites.push(sprite);
        self
    }

    /// Add animation
    pub fn with_animation(mut self, animation: SpriteAnimation) -> Self {
        self.animations.push(animation);
        self
    }
}

impl Default for SpriteSheetCreateInfo {
    fn default() -> Self {
        Self::new(TextureAtlasHandle::NULL)
    }
}

/// Sprite definition
#[derive(Clone, Debug)]
pub struct SpriteDefinition {
    /// Name
    pub name: String,
    /// Region in atlas
    pub region: AtlasRegion,
    /// Pivot point (normalized, 0-1)
    pub pivot: [f32; 2],
    /// 9-slice border (left, right, top, bottom)
    pub border: [u32; 4],
}

impl SpriteDefinition {
    /// Creates definition
    pub fn new(name: &str, region: AtlasRegion) -> Self {
        Self {
            name: String::from(name),
            region,
            pivot: [0.5, 0.5],
            border: [0, 0, 0, 0],
        }
    }

    /// With centered pivot
    pub fn centered(mut self) -> Self {
        self.pivot = [0.5, 0.5];
        self
    }

    /// With bottom-center pivot
    pub fn bottom_center(mut self) -> Self {
        self.pivot = [0.5, 1.0];
        self
    }

    /// With 9-slice border
    pub fn with_border(mut self, left: u32, right: u32, top: u32, bottom: u32) -> Self {
        self.border = [left, right, top, bottom];
        self
    }

    /// Has 9-slice border
    pub fn has_border(&self) -> bool {
        self.border.iter().any(|&b| b > 0)
    }
}

impl Default for SpriteDefinition {
    fn default() -> Self {
        Self::new("sprite", AtlasRegion::default())
    }
}

/// Sprite animation
#[derive(Clone, Debug)]
pub struct SpriteAnimation {
    /// Name
    pub name: String,
    /// Frames (sprite indices)
    pub frames: Vec<AnimationFrame>,
    /// Loop mode
    pub loop_mode: AnimationLoopMode,
    /// FPS
    pub fps: f32,
}

impl SpriteAnimation {
    /// Creates animation
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            frames: Vec::new(),
            loop_mode: AnimationLoopMode::Loop,
            fps: 12.0,
        }
    }

    /// From sprite range
    pub fn from_range(name: &str, start: u32, end: u32, fps: f32) -> Self {
        let frames = (start..=end)
            .map(|i| AnimationFrame {
                sprite_index: i,
                duration: 1.0,
            })
            .collect();

        Self {
            name: String::from(name),
            frames,
            fps,
            ..Self::new(name)
        }
    }

    /// Add frame
    pub fn with_frame(mut self, sprite_index: u32, duration: f32) -> Self {
        self.frames.push(AnimationFrame {
            sprite_index,
            duration,
        });
        self
    }

    /// Total duration in seconds
    pub fn duration(&self) -> f32 {
        let total_weight: f32 = self.frames.iter().map(|f| f.duration).sum();
        total_weight / self.fps
    }

    /// Get frame at time
    pub fn frame_at(&self, time: f32) -> u32 {
        if self.frames.is_empty() {
            return 0;
        }

        let duration = self.duration();
        let t = match self.loop_mode {
            AnimationLoopMode::Once => time.clamp(0.0, duration),
            AnimationLoopMode::Loop => time % duration,
            AnimationLoopMode::PingPong => {
                let t = time % (duration * 2.0);
                if t > duration {
                    duration * 2.0 - t
                } else {
                    t
                }
            }
        };

        let frame_time = t * self.fps;
        let mut accumulated = 0.0;

        for frame in &self.frames {
            accumulated += frame.duration;
            if frame_time < accumulated {
                return frame.sprite_index;
            }
        }

        self.frames.last().map(|f| f.sprite_index).unwrap_or(0)
    }
}

impl Default for SpriteAnimation {
    fn default() -> Self {
        Self::new("animation")
    }
}

/// Animation frame
#[derive(Clone, Copy, Debug)]
pub struct AnimationFrame {
    /// Sprite index
    pub sprite_index: u32,
    /// Relative duration
    pub duration: f32,
}

impl Default for AnimationFrame {
    fn default() -> Self {
        Self {
            sprite_index: 0,
            duration: 1.0,
        }
    }
}

/// Animation loop mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AnimationLoopMode {
    /// Play once
    Once = 0,
    /// Loop
    #[default]
    Loop = 1,
    /// Ping-pong
    PingPong = 2,
}

// ============================================================================
// Statistics
// ============================================================================

/// Atlas statistics
#[derive(Clone, Debug, Default)]
pub struct AtlasStats {
    /// Total atlases
    pub atlas_count: u32,
    /// Total pages
    pub page_count: u32,
    /// Total regions
    pub region_count: u32,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Average efficiency
    pub average_efficiency: f32,
}
