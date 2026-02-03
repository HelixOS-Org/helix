//! GPU Text Rendering Types for Lumina
//!
//! This module provides GPU-accelerated text rendering infrastructure
//! for UI, debug overlays, and in-world text display.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Text Renderer Handles
// ============================================================================

/// GPU text renderer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuTextRendererHandle(pub u64);

impl GpuTextRendererHandle {
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

impl Default for GpuTextRendererHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Font handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FontHandle(pub u64);

impl FontHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for FontHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Font atlas handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FontAtlasHandle(pub u64);

impl FontAtlasHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for FontAtlasHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Text batch handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextBatchHandle(pub u64);

impl TextBatchHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for TextBatchHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Text Renderer Creation
// ============================================================================

/// GPU text renderer create info
#[derive(Clone, Debug)]
pub struct GpuTextRendererCreateInfo {
    /// Name
    pub name: String,
    /// Max glyphs per frame
    pub max_glyphs: u32,
    /// Max text batches
    pub max_batches: u32,
    /// Default atlas size
    pub atlas_size: u32,
    /// Features
    pub features: TextRenderFeatures,
    /// Render mode
    pub render_mode: TextRenderMode,
}

impl GpuTextRendererCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_glyphs: 65536,
            max_batches: 256,
            atlas_size: 2048,
            features: TextRenderFeatures::all(),
            render_mode: TextRenderMode::Sdf,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max glyphs
    pub fn with_max_glyphs(mut self, count: u32) -> Self {
        self.max_glyphs = count;
        self
    }

    /// With max batches
    pub fn with_max_batches(mut self, count: u32) -> Self {
        self.max_batches = count;
        self
    }

    /// With atlas size
    pub fn with_atlas_size(mut self, size: u32) -> Self {
        self.atlas_size = size;
        self
    }

    /// With features
    pub fn with_features(mut self, features: TextRenderFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With render mode
    pub fn with_mode(mut self, mode: TextRenderMode) -> Self {
        self.render_mode = mode;
        self
    }

    /// Standard renderer
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality (MSDF)
    pub fn high_quality() -> Self {
        Self::new()
            .with_mode(TextRenderMode::Msdf)
            .with_atlas_size(4096)
    }

    /// Lightweight
    pub fn lightweight() -> Self {
        Self::new()
            .with_max_glyphs(16384)
            .with_max_batches(64)
            .with_atlas_size(1024)
            .with_mode(TextRenderMode::Bitmap)
    }

    /// Debug overlay
    pub fn debug_overlay() -> Self {
        Self::new()
            .with_max_glyphs(8192)
            .with_mode(TextRenderMode::Bitmap)
    }
}

impl Default for GpuTextRendererCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Text render features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct TextRenderFeatures: u32 {
        /// None
        const NONE = 0;
        /// Color per character
        const PER_CHAR_COLOR = 1 << 0;
        /// Bold
        const BOLD = 1 << 1;
        /// Italic
        const ITALIC = 1 << 2;
        /// Underline
        const UNDERLINE = 1 << 3;
        /// Strikethrough
        const STRIKETHROUGH = 1 << 4;
        /// Outline
        const OUTLINE = 1 << 5;
        /// Shadow
        const SHADOW = 1 << 6;
        /// Gradient
        const GRADIENT = 1 << 7;
        /// Kerning
        const KERNING = 1 << 8;
        /// Ligatures
        const LIGATURES = 1 << 9;
        /// Right-to-left
        const RTL = 1 << 10;
        /// Vertical text
        const VERTICAL = 1 << 11;
        /// All
        const ALL = 0xFFF;
    }
}

impl Default for TextRenderFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Text render mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextRenderMode {
    /// Bitmap (fast, low quality at large sizes)
    Bitmap = 0,
    /// Signed distance field (good quality, scalable)
    #[default]
    Sdf    = 1,
    /// Multi-channel SDF (best quality)
    Msdf   = 2,
    /// Vector (GPU path rendering)
    Vector = 3,
}

// ============================================================================
// Font Loading
// ============================================================================

/// Font create info
#[derive(Clone, Debug)]
pub struct FontCreateInfo {
    /// Name
    pub name: String,
    /// Font data (TTF/OTF)
    pub data: Vec<u8>,
    /// Font index in collection
    pub font_index: u32,
    /// Default size
    pub default_size: f32,
    /// Render mode
    pub render_mode: TextRenderMode,
    /// Features
    pub features: FontFeatures,
}

impl FontCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            data: Vec::new(),
            font_index: 0,
            default_size: 16.0,
            render_mode: TextRenderMode::Sdf,
            features: FontFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With data
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// With font index
    pub fn with_index(mut self, index: u32) -> Self {
        self.font_index = index;
        self
    }

    /// With default size
    pub fn with_size(mut self, size: f32) -> Self {
        self.default_size = size;
        self
    }

    /// With render mode
    pub fn with_mode(mut self, mode: TextRenderMode) -> Self {
        self.render_mode = mode;
        self
    }

    /// With features
    pub fn with_features(mut self, features: FontFeatures) -> Self {
        self.features = features;
        self
    }

    /// SDF font
    pub fn sdf(data: Vec<u8>) -> Self {
        Self::new().with_data(data).with_mode(TextRenderMode::Sdf)
    }

    /// Bitmap font
    pub fn bitmap(data: Vec<u8>) -> Self {
        Self::new()
            .with_data(data)
            .with_mode(TextRenderMode::Bitmap)
    }

    /// MSDF font
    pub fn msdf(data: Vec<u8>) -> Self {
        Self::new().with_data(data).with_mode(TextRenderMode::Msdf)
    }
}

impl Default for FontCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Font features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct FontFeatures: u32 {
        /// None
        const NONE = 0;
        /// Kerning
        const KERNING = 1 << 0;
        /// Ligatures
        const LIGATURES = 1 << 1;
        /// OpenType features
        const OPENTYPE = 1 << 2;
        /// Hinting
        const HINTING = 1 << 3;
        /// Auto-hinting
        const AUTO_HINT = 1 << 4;
        /// Subpixel positioning
        const SUBPIXEL = 1 << 5;
        /// LCD anti-aliasing
        const LCD_AA = 1 << 6;
        /// All
        const ALL = 0x7F;
    }
}

impl Default for FontFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Font Atlas
// ============================================================================

/// Font atlas create info
#[derive(Clone, Debug)]
pub struct FontAtlasCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Padding
    pub padding: u32,
    /// Render mode
    pub render_mode: TextRenderMode,
    /// SDF spread (for SDF modes)
    pub sdf_spread: u32,
}

impl FontAtlasCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            width: 2048,
            height: 2048,
            padding: 2,
            render_mode: TextRenderMode::Sdf,
            sdf_spread: 8,
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

    /// With render mode
    pub fn with_mode(mut self, mode: TextRenderMode) -> Self {
        self.render_mode = mode;
        self
    }

    /// With SDF spread
    pub fn with_sdf_spread(mut self, spread: u32) -> Self {
        self.sdf_spread = spread;
        self
    }

    /// Standard atlas
    pub fn standard() -> Self {
        Self::new()
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

impl Default for FontAtlasCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Glyph info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GlyphInfo {
    /// Unicode codepoint
    pub codepoint: u32,
    /// Atlas X
    pub atlas_x: u16,
    /// Atlas Y
    pub atlas_y: u16,
    /// Glyph width
    pub width: u16,
    /// Glyph height
    pub height: u16,
    /// X offset
    pub offset_x: i16,
    /// Y offset
    pub offset_y: i16,
    /// Advance width
    pub advance: f32,
}

impl GlyphInfo {
    /// UV min
    pub fn uv_min(&self, atlas_size: f32) -> [f32; 2] {
        [
            self.atlas_x as f32 / atlas_size,
            self.atlas_y as f32 / atlas_size,
        ]
    }

    /// UV max
    pub fn uv_max(&self, atlas_size: f32) -> [f32; 2] {
        [
            (self.atlas_x + self.width) as f32 / atlas_size,
            (self.atlas_y + self.height) as f32 / atlas_size,
        ]
    }
}

// ============================================================================
// Text Drawing
// ============================================================================

/// Text draw info
#[derive(Clone, Debug)]
pub struct TextDrawInfo {
    /// Text content
    pub text: String,
    /// Font
    pub font: FontHandle,
    /// Position
    pub position: TextPosition,
    /// Font size
    pub font_size: f32,
    /// Color
    pub color: TextColor,
    /// Style
    pub style: TextStyle,
    /// Layout
    pub layout: TextLayout,
}

impl TextDrawInfo {
    /// Creates new info
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font: FontHandle::NULL,
            position: TextPosition::default(),
            font_size: 16.0,
            color: TextColor::Solid([1.0, 1.0, 1.0, 1.0]),
            style: TextStyle::default(),
            layout: TextLayout::default(),
        }
    }

    /// With font
    pub fn with_font(mut self, font: FontHandle) -> Self {
        self.font = font;
        self
    }

    /// At screen position
    pub fn at_screen(mut self, x: f32, y: f32) -> Self {
        self.position = TextPosition::Screen { x, y };
        self
    }

    /// At world position
    pub fn at_world(mut self, pos: [f32; 3]) -> Self {
        self.position = TextPosition::World { position: pos };
        self
    }

    /// With size
    pub fn with_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = TextColor::Solid([r, g, b, a]);
        self
    }

    /// With gradient
    pub fn with_gradient(mut self, top: [f32; 4], bottom: [f32; 4]) -> Self {
        self.color = TextColor::Gradient { top, bottom };
        self
    }

    /// With style
    pub fn with_style(mut self, style: TextStyle) -> Self {
        self.style = style;
        self
    }

    /// With layout
    pub fn with_layout(mut self, layout: TextLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Bold
    pub fn bold(mut self) -> Self {
        self.style.flags |= TextStyleFlags::BOLD;
        self
    }

    /// Italic
    pub fn italic(mut self) -> Self {
        self.style.flags |= TextStyleFlags::ITALIC;
        self
    }

    /// Simple white text
    pub fn simple(text: impl Into<String>, x: f32, y: f32, size: f32) -> Self {
        Self::new(text).at_screen(x, y).with_size(size)
    }
}

impl Default for TextDrawInfo {
    fn default() -> Self {
        Self::new("")
    }
}

/// Text position
#[derive(Clone, Copy, Debug)]
pub enum TextPosition {
    /// Screen space (pixels)
    Screen { x: f32, y: f32 },
    /// Normalized screen (0-1)
    Normalized { x: f32, y: f32 },
    /// World space (3D)
    World { position: [f32; 3] },
}

impl Default for TextPosition {
    fn default() -> Self {
        Self::Screen { x: 0.0, y: 0.0 }
    }
}

/// Text color
#[derive(Clone, Copy, Debug)]
pub enum TextColor {
    /// Solid color
    Solid([f32; 4]),
    /// Vertical gradient
    Gradient { top: [f32; 4], bottom: [f32; 4] },
    /// Per-character (array index)
    PerCharacter(u32),
}

impl Default for TextColor {
    fn default() -> Self {
        Self::Solid([1.0, 1.0, 1.0, 1.0])
    }
}

/// Text style
#[derive(Clone, Copy, Debug, Default)]
pub struct TextStyle {
    /// Style flags
    pub flags: TextStyleFlags,
    /// Outline color
    pub outline_color: [f32; 4],
    /// Outline width
    pub outline_width: f32,
    /// Shadow color
    pub shadow_color: [f32; 4],
    /// Shadow offset
    pub shadow_offset: [f32; 2],
    /// Letter spacing
    pub letter_spacing: f32,
    /// Line height
    pub line_height: f32,
}

impl TextStyle {
    /// Creates new style
    pub const fn new() -> Self {
        Self {
            flags: TextStyleFlags::NONE,
            outline_color: [0.0, 0.0, 0.0, 1.0],
            outline_width: 0.0,
            shadow_color: [0.0, 0.0, 0.0, 0.5],
            shadow_offset: [2.0, 2.0],
            letter_spacing: 0.0,
            line_height: 1.2,
        }
    }

    /// With outline
    pub fn with_outline(mut self, color: [f32; 4], width: f32) -> Self {
        self.flags |= TextStyleFlags::OUTLINE;
        self.outline_color = color;
        self.outline_width = width;
        self
    }

    /// With shadow
    pub fn with_shadow(mut self, color: [f32; 4], offset: [f32; 2]) -> Self {
        self.flags |= TextStyleFlags::SHADOW;
        self.shadow_color = color;
        self.shadow_offset = offset;
        self
    }

    /// With letter spacing
    pub fn with_letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = spacing;
        self
    }

    /// With line height
    pub fn with_line_height(mut self, height: f32) -> Self {
        self.line_height = height;
        self
    }
}

bitflags::bitflags! {
    /// Text style flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct TextStyleFlags: u32 {
        /// None
        const NONE = 0;
        /// Bold
        const BOLD = 1 << 0;
        /// Italic
        const ITALIC = 1 << 1;
        /// Underline
        const UNDERLINE = 1 << 2;
        /// Strikethrough
        const STRIKETHROUGH = 1 << 3;
        /// Outline
        const OUTLINE = 1 << 4;
        /// Shadow
        const SHADOW = 1 << 5;
        /// Small caps
        const SMALL_CAPS = 1 << 6;
        /// Superscript
        const SUPERSCRIPT = 1 << 7;
        /// Subscript
        const SUBSCRIPT = 1 << 8;
    }
}

/// Text layout
#[derive(Clone, Copy, Debug)]
pub struct TextLayout {
    /// Horizontal alignment
    pub h_align: TextHAlign,
    /// Vertical alignment
    pub v_align: TextVAlign,
    /// Text direction
    pub direction: TextDirection,
    /// Wrap mode
    pub wrap: TextWrap,
    /// Max width (0 = no limit)
    pub max_width: f32,
    /// Max height (0 = no limit)
    pub max_height: f32,
}

impl TextLayout {
    /// Creates new layout
    pub const fn new() -> Self {
        Self {
            h_align: TextHAlign::Left,
            v_align: TextVAlign::Top,
            direction: TextDirection::LeftToRight,
            wrap: TextWrap::None,
            max_width: 0.0,
            max_height: 0.0,
        }
    }

    /// With horizontal align
    pub const fn with_h_align(mut self, align: TextHAlign) -> Self {
        self.h_align = align;
        self
    }

    /// With vertical align
    pub const fn with_v_align(mut self, align: TextVAlign) -> Self {
        self.v_align = align;
        self
    }

    /// With wrap
    pub const fn with_wrap(mut self, wrap: TextWrap) -> Self {
        self.wrap = wrap;
        self
    }

    /// With max width
    pub const fn with_max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    /// Centered
    pub const fn centered() -> Self {
        Self::new()
            .with_h_align(TextHAlign::Center)
            .with_v_align(TextVAlign::Center)
    }

    /// Right aligned
    pub const fn right() -> Self {
        Self::new().with_h_align(TextHAlign::Right)
    }
}

impl Default for TextLayout {
    fn default() -> Self {
        Self::new()
    }
}

/// Horizontal alignment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextHAlign {
    /// Left
    #[default]
    Left      = 0,
    /// Center
    Center    = 1,
    /// Right
    Right     = 2,
    /// Justified
    Justified = 3,
}

/// Vertical alignment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextVAlign {
    /// Top
    #[default]
    Top      = 0,
    /// Center
    Center   = 1,
    /// Bottom
    Bottom   = 2,
    /// Baseline
    Baseline = 3,
}

/// Text direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextDirection {
    /// Left to right
    #[default]
    LeftToRight = 0,
    /// Right to left
    RightToLeft = 1,
    /// Top to bottom
    TopToBottom = 2,
    /// Bottom to top
    BottomToTop = 3,
}

/// Text wrap mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextWrap {
    /// No wrap
    #[default]
    None      = 0,
    /// Word wrap
    Word      = 1,
    /// Character wrap
    Character = 2,
    /// Word with hyphenation
    Hyphenate = 3,
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU text vertex
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTextVertex {
    /// Position (x, y)
    pub position: [f32; 2],
    /// UV coordinates
    pub uv: [f32; 2],
    /// Color
    pub color: [f32; 4],
}

/// GPU text glyph
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTextGlyph {
    /// Position (x, y)
    pub position: [f32; 2],
    /// Size (width, height)
    pub size: [f32; 2],
    /// UV min
    pub uv_min: [f32; 2],
    /// UV max
    pub uv_max: [f32; 2],
    /// Color
    pub color: [f32; 4],
}

/// GPU text constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTextConstants {
    /// Transform matrix
    pub transform: [[f32; 4]; 4],
    /// Screen size
    pub screen_size: [f32; 2],
    /// Atlas size
    pub atlas_size: f32,
    /// SDF smoothing
    pub sdf_smoothing: f32,
    /// Outline params (color in .xyz, width in .w)
    pub outline: [f32; 4],
    /// Shadow params (color in .xyz, offset in extra field)
    pub shadow_color: [f32; 4],
    /// Shadow offset
    pub shadow_offset: [f32; 2],
    /// Padding
    pub _pad: [f32; 2],
}

// ============================================================================
// Text Measurement
// ============================================================================

/// Text metrics
#[derive(Clone, Copy, Debug, Default)]
pub struct TextMetrics {
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Ascent
    pub ascent: f32,
    /// Descent
    pub descent: f32,
    /// Line height
    pub line_height: f32,
    /// Number of lines
    pub line_count: u32,
    /// Character count
    pub char_count: u32,
}

impl TextMetrics {
    /// Bounding box
    pub fn bounds(&self) -> [f32; 4] {
        [0.0, -self.ascent, self.width, self.height]
    }
}

/// Font metrics
#[derive(Clone, Copy, Debug, Default)]
pub struct FontMetrics {
    /// Units per EM
    pub units_per_em: u32,
    /// Ascender
    pub ascender: f32,
    /// Descender
    pub descender: f32,
    /// Line gap
    pub line_gap: f32,
    /// Underline position
    pub underline_position: f32,
    /// Underline thickness
    pub underline_thickness: f32,
    /// Strikeout position
    pub strikeout_position: f32,
    /// Strikeout thickness
    pub strikeout_thickness: f32,
}

impl FontMetrics {
    /// Line height
    pub fn line_height(&self) -> f32 {
        self.ascender - self.descender + self.line_gap
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Text renderer statistics
#[derive(Clone, Debug, Default)]
pub struct TextRendererStats {
    /// Glyphs rendered
    pub glyphs_rendered: u32,
    /// Batches submitted
    pub batches_submitted: u32,
    /// Draw calls
    pub draw_calls: u32,
    /// Atlas textures
    pub atlas_count: u32,
    /// Atlas memory usage
    pub atlas_memory: u64,
    /// Vertex buffer usage
    pub vertex_memory: u64,
    /// Cache hits
    pub cache_hits: u32,
    /// Cache misses
    pub cache_misses: u32,
}

impl TextRendererStats {
    /// Cache hit rate
    pub fn cache_hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            1.0
        } else {
            self.cache_hits as f32 / total as f32
        }
    }
}
