//! Sprite Batch Rendering Types for Lumina
//!
//! This module provides efficient 2D sprite batching infrastructure
//! for UI, particles, and 2D game rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Sprite Batch Handles
// ============================================================================

/// Sprite batch renderer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SpriteBatchHandle(pub u64);

impl SpriteBatchHandle {
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

impl Default for SpriteBatchHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sprite texture handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SpriteTextureHandle(pub u64);

impl SpriteTextureHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SpriteTextureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sprite atlas handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SpriteAtlasHandle(pub u64);

impl SpriteAtlasHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SpriteAtlasHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sprite handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SpriteHandle(pub u64);

impl SpriteHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SpriteHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Sprite Batch Creation
// ============================================================================

/// Sprite batch create info
#[derive(Clone, Debug)]
pub struct SpriteBatchCreateInfo {
    /// Name
    pub name: String,
    /// Max sprites per batch
    pub max_sprites: u32,
    /// Max batches
    pub max_batches: u32,
    /// Max textures
    pub max_textures: u32,
    /// Features
    pub features: SpriteBatchFeatures,
    /// Blend mode
    pub blend_mode: SpriteBlendMode,
    /// Sort mode
    pub sort_mode: SpriteSortMode,
}

impl SpriteBatchCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_sprites: 65536,
            max_batches: 256,
            max_textures: 1024,
            features: SpriteBatchFeatures::all(),
            blend_mode: SpriteBlendMode::AlphaBlend,
            sort_mode: SpriteSortMode::Deferred,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max sprites
    pub fn with_max_sprites(mut self, count: u32) -> Self {
        self.max_sprites = count;
        self
    }

    /// With max batches
    pub fn with_max_batches(mut self, count: u32) -> Self {
        self.max_batches = count;
        self
    }

    /// With max textures
    pub fn with_max_textures(mut self, count: u32) -> Self {
        self.max_textures = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: SpriteBatchFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With blend mode
    pub fn with_blend(mut self, mode: SpriteBlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// With sort mode
    pub fn with_sort(mut self, mode: SpriteSortMode) -> Self {
        self.sort_mode = mode;
        self
    }

    /// Standard batch
    pub fn standard() -> Self {
        Self::new()
    }

    /// Lightweight
    pub fn lightweight() -> Self {
        Self::new()
            .with_max_sprites(16384)
            .with_max_batches(64)
    }

    /// High capacity
    pub fn high_capacity() -> Self {
        Self::new()
            .with_max_sprites(262144)
            .with_max_batches(1024)
    }

    /// Immediate mode
    pub fn immediate() -> Self {
        Self::new()
            .with_sort(SpriteSortMode::Immediate)
    }

    /// UI optimized
    pub fn ui() -> Self {
        Self::new()
            .with_sort(SpriteSortMode::Deferred)
            .with_blend(SpriteBlendMode::AlphaBlend)
    }

    /// Additive particles
    pub fn particles() -> Self {
        Self::new()
            .with_blend(SpriteBlendMode::Additive)
            .with_sort(SpriteSortMode::BackToFront)
    }
}

impl Default for SpriteBatchCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Sprite batch features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct SpriteBatchFeatures: u32 {
        /// None
        const NONE = 0;
        /// Rotation
        const ROTATION = 1 << 0;
        /// Scaling
        const SCALING = 1 << 1;
        /// Tinting
        const TINTING = 1 << 2;
        /// Per-vertex color
        const VERTEX_COLOR = 1 << 3;
        /// UV animation
        const UV_ANIMATION = 1 << 4;
        /// Instancing
        const INSTANCING = 1 << 5;
        /// Sorting
        const SORTING = 1 << 6;
        /// Clipping
        const CLIPPING = 1 << 7;
        /// 9-slice
        const NINE_SLICE = 1 << 8;
        /// All
        const ALL = 0x1FF;
    }
}

impl Default for SpriteBatchFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Sprite blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SpriteBlendMode {
    /// No blending (opaque)
    Opaque = 0,
    /// Alpha blend
    #[default]
    AlphaBlend = 1,
    /// Additive
    Additive = 2,
    /// Multiply
    Multiply = 3,
    /// Screen
    Screen = 4,
    /// Premultiplied alpha
    PremultipliedAlpha = 5,
    /// Subtractive
    Subtractive = 6,
    /// Custom
    Custom = 7,
}

/// Sprite sort mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SpriteSortMode {
    /// Deferred (batched)
    #[default]
    Deferred = 0,
    /// Immediate (flush each draw)
    Immediate = 1,
    /// Back to front
    BackToFront = 2,
    /// Front to back
    FrontToBack = 3,
    /// By texture
    Texture = 4,
}

// ============================================================================
// Sprite Definition
// ============================================================================

/// Sprite definition
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Sprite {
    /// Position (x, y)
    pub position: [f32; 2],
    /// Size (width, height)
    pub size: [f32; 2],
    /// Origin (pivot point, 0-1)
    pub origin: [f32; 2],
    /// Rotation (radians)
    pub rotation: f32,
    /// Color tint
    pub color: SpriteColor,
    /// UV rectangle (x, y, w, h)
    pub uv_rect: [f32; 4],
    /// Texture index
    pub texture_index: u32,
    /// Depth (for sorting)
    pub depth: f32,
    /// Flags
    pub flags: SpriteFlags,
}

impl Sprite {
    /// Creates new sprite
    pub const fn new(position: [f32; 2], size: [f32; 2]) -> Self {
        Self {
            position,
            size,
            origin: [0.5, 0.5],
            rotation: 0.0,
            color: SpriteColor::WHITE,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            texture_index: 0,
            depth: 0.0,
            flags: SpriteFlags::NONE,
        }
    }

    /// With origin
    pub const fn with_origin(mut self, x: f32, y: f32) -> Self {
        self.origin = [x, y];
        self
    }

    /// With rotation
    pub const fn with_rotation(mut self, radians: f32) -> Self {
        self.rotation = radians;
        self
    }

    /// With color
    pub const fn with_color(mut self, color: SpriteColor) -> Self {
        self.color = color;
        self
    }

    /// With UV rect
    pub const fn with_uv(mut self, uv: [f32; 4]) -> Self {
        self.uv_rect = uv;
        self
    }

    /// With texture
    pub const fn with_texture(mut self, index: u32) -> Self {
        self.texture_index = index;
        self
    }

    /// With depth
    pub const fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }

    /// With flags
    pub const fn with_flags(mut self, flags: SpriteFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Flip horizontal
    pub fn flip_h(mut self) -> Self {
        self.flags |= SpriteFlags::FLIP_H;
        self
    }

    /// Flip vertical
    pub fn flip_v(mut self) -> Self {
        self.flags |= SpriteFlags::FLIP_V;
        self
    }

    /// Center origin
    pub const fn centered(mut self) -> Self {
        self.origin = [0.5, 0.5];
        self
    }

    /// Top-left origin
    pub const fn top_left(mut self) -> Self {
        self.origin = [0.0, 0.0];
        self
    }

    /// Simple sprite at position
    pub const fn at(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::new([x, y], [w, h])
    }
}

impl Default for Sprite {
    fn default() -> Self {
        Self::new([0.0, 0.0], [1.0, 1.0])
    }
}

/// Sprite color
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct SpriteColor {
    /// Red
    pub r: f32,
    /// Green
    pub g: f32,
    /// Blue
    pub b: f32,
    /// Alpha
    pub a: f32,
}

impl SpriteColor {
    /// White
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    /// Black
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    /// Red
    pub const RED: Self = Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    /// Green
    pub const GREEN: Self = Self { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    /// Blue
    pub const BLUE: Self = Self { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    /// Transparent
    pub const TRANSPARENT: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 0.0 };

    /// Creates new color
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// From RGB
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// From u8 values
    pub const fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// With alpha
    pub const fn with_alpha(mut self, a: f32) -> Self {
        self.a = a;
        self
    }

    /// To array
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for SpriteColor {
    fn default() -> Self {
        Self::WHITE
    }
}

bitflags::bitflags! {
    /// Sprite flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct SpriteFlags: u32 {
        /// None
        const NONE = 0;
        /// Flip horizontal
        const FLIP_H = 1 << 0;
        /// Flip vertical
        const FLIP_V = 1 << 1;
        /// Billboard (face camera)
        const BILLBOARD = 1 << 2;
        /// Ignore depth
        const NO_DEPTH = 1 << 3;
        /// Shadow caster
        const CAST_SHADOW = 1 << 4;
        /// Lit sprite
        const LIT = 1 << 5;
    }
}

// ============================================================================
// Sprite Atlas
// ============================================================================

/// Sprite atlas create info
#[derive(Clone, Debug)]
pub struct SpriteAtlasCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Padding
    pub padding: u32,
    /// Allow resize
    pub allow_resize: bool,
    /// Max size
    pub max_size: u32,
}

impl SpriteAtlasCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            width: 2048,
            height: 2048,
            padding: 2,
            allow_resize: true,
            max_size: 8192,
        }
    }

    /// With size
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// With padding
    pub fn with_padding(mut self, padding: u32) -> Self {
        self.padding = padding;
        self
    }

    /// Small atlas
    pub fn small() -> Self {
        Self::new().with_size(1024, 1024)
    }

    /// Large atlas
    pub fn large() -> Self {
        Self::new().with_size(4096, 4096)
    }
}

impl Default for SpriteAtlasCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Sprite region (in atlas)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SpriteRegion {
    /// X position in atlas
    pub x: u32,
    /// Y position in atlas
    pub y: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Original width (before packing)
    pub original_width: u32,
    /// Original height
    pub original_height: u32,
    /// Offset X (trimmed whitespace)
    pub offset_x: i32,
    /// Offset Y
    pub offset_y: i32,
    /// Is rotated 90 degrees
    pub rotated: bool,
}

impl SpriteRegion {
    /// UV rect
    pub fn uv_rect(&self, atlas_width: u32, atlas_height: u32) -> [f32; 4] {
        let w = atlas_width as f32;
        let h = atlas_height as f32;
        [
            self.x as f32 / w,
            self.y as f32 / h,
            self.width as f32 / w,
            self.height as f32 / h,
        ]
    }
}

// ============================================================================
// Nine-Slice (9-patch)
// ============================================================================

/// Nine-slice definition
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct NineSlice {
    /// Left border
    pub left: f32,
    /// Right border
    pub right: f32,
    /// Top border
    pub top: f32,
    /// Bottom border
    pub bottom: f32,
}

impl NineSlice {
    /// Creates new nine-slice
    pub const fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self { left, right, top, bottom }
    }

    /// Uniform borders
    pub const fn uniform(border: f32) -> Self {
        Self::new(border, border, border, border)
    }

    /// Horizontal only
    pub const fn horizontal(left: f32, right: f32) -> Self {
        Self::new(left, right, 0.0, 0.0)
    }

    /// Vertical only
    pub const fn vertical(top: f32, bottom: f32) -> Self {
        Self::new(0.0, 0.0, top, bottom)
    }
}

impl Default for NineSlice {
    fn default() -> Self {
        Self::uniform(0.0)
    }
}

/// Nine-slice sprite
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct NineSliceSprite {
    /// Position
    pub position: [f32; 2],
    /// Size
    pub size: [f32; 2],
    /// UV rect
    pub uv_rect: [f32; 4],
    /// Slice borders
    pub slice: NineSlice,
    /// Color
    pub color: SpriteColor,
    /// Texture index
    pub texture_index: u32,
    /// Depth
    pub depth: f32,
}

impl NineSliceSprite {
    /// Creates new nine-slice sprite
    pub const fn new(position: [f32; 2], size: [f32; 2], slice: NineSlice) -> Self {
        Self {
            position,
            size,
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            slice,
            color: SpriteColor::WHITE,
            texture_index: 0,
            depth: 0.0,
        }
    }

    /// With color
    pub const fn with_color(mut self, color: SpriteColor) -> Self {
        self.color = color;
        self
    }

    /// With UV
    pub const fn with_uv(mut self, uv: [f32; 4]) -> Self {
        self.uv_rect = uv;
        self
    }

    /// With texture
    pub const fn with_texture(mut self, index: u32) -> Self {
        self.texture_index = index;
        self
    }
}

impl Default for NineSliceSprite {
    fn default() -> Self {
        Self::new([0.0, 0.0], [1.0, 1.0], NineSlice::default())
    }
}

// ============================================================================
// Animation
// ============================================================================

/// Sprite animation
#[derive(Clone, Debug)]
pub struct SpriteAnimation {
    /// Name
    pub name: String,
    /// Frames
    pub frames: Vec<SpriteFrame>,
    /// Playback mode
    pub mode: AnimationMode,
    /// Frame rate
    pub fps: f32,
}

impl SpriteAnimation {
    /// Creates new animation
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            frames: Vec::new(),
            mode: AnimationMode::Loop,
            fps: 12.0,
        }
    }

    /// Add frame
    pub fn add_frame(mut self, frame: SpriteFrame) -> Self {
        self.frames.push(frame);
        self
    }

    /// With mode
    pub fn with_mode(mut self, mode: AnimationMode) -> Self {
        self.mode = mode;
        self
    }

    /// With FPS
    pub fn with_fps(mut self, fps: f32) -> Self {
        self.fps = fps;
        self
    }

    /// Duration
    pub fn duration(&self) -> f32 {
        self.frames.len() as f32 / self.fps
    }

    /// Frame at time
    pub fn frame_at(&self, time: f32) -> Option<&SpriteFrame> {
        if self.frames.is_empty() {
            return None;
        }

        let frame_index = match self.mode {
            AnimationMode::Once => {
                let idx = (time * self.fps) as usize;
                idx.min(self.frames.len() - 1)
            }
            AnimationMode::Loop => {
                let idx = (time * self.fps) as usize;
                idx % self.frames.len()
            }
            AnimationMode::PingPong => {
                let total = self.frames.len() * 2 - 2;
                if total == 0 {
                    0
                } else {
                    let idx = (time * self.fps) as usize % total;
                    if idx < self.frames.len() {
                        idx
                    } else {
                        total - idx
                    }
                }
            }
        };

        self.frames.get(frame_index)
    }
}

impl Default for SpriteAnimation {
    fn default() -> Self {
        Self::new("")
    }
}

/// Sprite frame
#[derive(Clone, Copy, Debug, Default)]
pub struct SpriteFrame {
    /// UV rect
    pub uv_rect: [f32; 4],
    /// Texture index
    pub texture_index: u32,
    /// Duration override (0 = use animation fps)
    pub duration: f32,
}

impl SpriteFrame {
    /// Creates new frame
    pub const fn new(uv_rect: [f32; 4]) -> Self {
        Self {
            uv_rect,
            texture_index: 0,
            duration: 0.0,
        }
    }

    /// With texture
    pub const fn with_texture(mut self, index: u32) -> Self {
        self.texture_index = index;
        self
    }

    /// With duration
    pub const fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }
}

/// Animation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AnimationMode {
    /// Play once
    Once = 0,
    /// Loop
    #[default]
    Loop = 1,
    /// Ping-pong
    PingPong = 2,
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU sprite vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSpriteVertex {
    /// Position (x, y)
    pub position: [f32; 2],
    /// UV
    pub uv: [f32; 2],
    /// Color
    pub color: [f32; 4],
}

/// GPU sprite instance
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSpriteInstance {
    /// Position (x, y)
    pub position: [f32; 2],
    /// Size (w, h)
    pub size: [f32; 2],
    /// Origin (pivot)
    pub origin: [f32; 2],
    /// Rotation (radians)
    pub rotation: f32,
    /// Texture index
    pub texture_index: u32,
    /// UV rect
    pub uv_rect: [f32; 4],
    /// Color
    pub color: [f32; 4],
    /// Flags (flip, etc)
    pub flags: u32,
    /// Depth
    pub depth: f32,
    /// Padding
    pub _pad: [f32; 2],
}

/// GPU sprite batch constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSpriteBatchConstants {
    /// Projection matrix
    pub projection: [[f32; 4]; 4],
    /// Screen size
    pub screen_size: [f32; 2],
    /// Time
    pub time: f32,
    /// Padding
    pub _pad: f32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Sprite batch statistics
#[derive(Clone, Debug, Default)]
pub struct SpriteBatchStats {
    /// Sprites drawn
    pub sprites_drawn: u32,
    /// Batches submitted
    pub batches_submitted: u32,
    /// Draw calls
    pub draw_calls: u32,
    /// Texture switches
    pub texture_switches: u32,
    /// Vertices generated
    pub vertices: u32,
    /// Indices generated
    pub indices: u32,
    /// Buffer memory usage
    pub buffer_memory: u64,
}

impl SpriteBatchStats {
    /// Sprites per batch
    pub fn sprites_per_batch(&self) -> f32 {
        if self.batches_submitted == 0 {
            0.0
        } else {
            self.sprites_drawn as f32 / self.batches_submitted as f32
        }
    }

    /// Sprites per draw call
    pub fn sprites_per_draw(&self) -> f32 {
        if self.draw_calls == 0 {
            0.0
        } else {
            self.sprites_drawn as f32 / self.draw_calls as f32
        }
    }
}
