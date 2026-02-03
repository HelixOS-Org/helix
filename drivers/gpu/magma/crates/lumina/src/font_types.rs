//! Font and Text Rendering Types for Lumina
//!
//! This module provides font loading, glyph management, and text
//! layout infrastructure for GPU-accelerated text rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Font Handles
// ============================================================================

/// Font handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FontHandle(pub u64);

impl FontHandle {
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

impl Default for FontAtlasHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Text mesh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TextMeshHandle(pub u64);

impl TextMeshHandle {
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

impl Default for TextMeshHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Font Create Info
// ============================================================================

/// Font create info
#[derive(Clone, Debug)]
pub struct FontCreateInfo {
    /// Font name
    pub name: String,
    /// Font data (TTF/OTF bytes)
    pub data: Vec<u8>,
    /// Font size (pixels)
    pub size: f32,
    /// DPI scale
    pub dpi_scale: f32,
    /// Rasterization mode
    pub raster_mode: FontRasterMode,
    /// Character set
    pub charset: FontCharset,
    /// Flags
    pub flags: FontFlags,
}

impl FontCreateInfo {
    /// Creates new font from data
    pub fn new(name: &str, data: Vec<u8>, size: f32) -> Self {
        Self {
            name: String::from(name),
            data,
            size,
            dpi_scale: 1.0,
            raster_mode: FontRasterMode::Standard,
            charset: FontCharset::Ascii,
            flags: FontFlags::DEFAULT,
        }
    }

    /// With DPI scale
    pub fn with_dpi_scale(mut self, scale: f32) -> Self {
        self.dpi_scale = scale;
        self
    }

    /// With raster mode
    pub fn with_raster_mode(mut self, mode: FontRasterMode) -> Self {
        self.raster_mode = mode;
        self
    }

    /// With charset
    pub fn with_charset(mut self, charset: FontCharset) -> Self {
        self.charset = charset;
        self
    }

    /// With SDF rendering
    pub fn sdf(mut self) -> Self {
        self.raster_mode = FontRasterMode::Sdf;
        self
    }

    /// With MSDF rendering
    pub fn msdf(mut self) -> Self {
        self.raster_mode = FontRasterMode::Msdf;
        self
    }
}

/// Font rasterization mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FontRasterMode {
    /// Standard rasterization
    #[default]
    Standard = 0,
    /// Signed distance field
    Sdf      = 1,
    /// Multi-channel signed distance field
    Msdf     = 2,
    /// Subpixel antialiasing
    Subpixel = 3,
}

impl FontRasterMode {
    /// Is distance field based
    pub const fn is_distance_field(&self) -> bool {
        matches!(self, Self::Sdf | Self::Msdf)
    }
}

/// Font character set
#[derive(Clone, Debug)]
pub enum FontCharset {
    /// ASCII only (32-127)
    Ascii,
    /// Extended ASCII (32-255)
    ExtendedAscii,
    /// Latin-1
    Latin1,
    /// Custom range
    Range { start: u32, end: u32 },
    /// Custom characters
    Custom(Vec<char>),
    /// Full Unicode (common planes)
    Unicode,
}

impl Default for FontCharset {
    fn default() -> Self {
        Self::Ascii
    }
}

/// Font flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FontFlags(pub u32);

impl FontFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Hinting enabled
    pub const HINTING: Self = Self(1 << 0);
    /// Anti-aliasing
    pub const ANTIALIAS: Self = Self(1 << 1);
    /// Bold
    pub const BOLD: Self = Self(1 << 2);
    /// Italic
    pub const ITALIC: Self = Self(1 << 3);
    /// Monospace metrics
    pub const MONOSPACE: Self = Self(1 << 4);
    /// Default flags
    pub const DEFAULT: Self = Self(Self::HINTING.0 | Self::ANTIALIAS.0);

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

// ============================================================================
// Font Metrics
// ============================================================================

/// Font metrics
#[derive(Clone, Copy, Debug, Default)]
pub struct FontMetrics {
    /// Units per EM
    pub units_per_em: u16,
    /// Ascender
    pub ascender: f32,
    /// Descender
    pub descender: f32,
    /// Line height
    pub line_height: f32,
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
    /// Cap height
    pub cap_height: f32,
    /// X height
    pub x_height: f32,
}

impl FontMetrics {
    /// Total line height including gap
    pub fn full_line_height(&self) -> f32 {
        self.ascender - self.descender + self.line_gap
    }

    /// Scaled metrics
    pub fn scaled(&self, scale: f32) -> Self {
        Self {
            units_per_em: self.units_per_em,
            ascender: self.ascender * scale,
            descender: self.descender * scale,
            line_height: self.line_height * scale,
            line_gap: self.line_gap * scale,
            underline_position: self.underline_position * scale,
            underline_thickness: self.underline_thickness * scale,
            strikeout_position: self.strikeout_position * scale,
            strikeout_thickness: self.strikeout_thickness * scale,
            cap_height: self.cap_height * scale,
            x_height: self.x_height * scale,
        }
    }
}

// ============================================================================
// Glyph
// ============================================================================

/// Glyph info
#[derive(Clone, Copy, Debug, Default)]
pub struct GlyphInfo {
    /// Glyph ID
    pub id: u32,
    /// Unicode codepoint
    pub codepoint: u32,
    /// Advance width
    pub advance_x: f32,
    /// Advance height
    pub advance_y: f32,
    /// Bearing X (left side)
    pub bearing_x: f32,
    /// Bearing Y (top)
    pub bearing_y: f32,
    /// Glyph width
    pub width: f32,
    /// Glyph height
    pub height: f32,
    /// Atlas UV coordinates
    pub uv: GlyphUV,
    /// Atlas page
    pub atlas_page: u32,
}

impl GlyphInfo {
    /// Creates empty glyph
    pub const fn empty() -> Self {
        Self {
            id: 0,
            codepoint: 0,
            advance_x: 0.0,
            advance_y: 0.0,
            bearing_x: 0.0,
            bearing_y: 0.0,
            width: 0.0,
            height: 0.0,
            uv: GlyphUV::ZERO,
            atlas_page: 0,
        }
    }

    /// Is whitespace
    pub fn is_whitespace(&self) -> bool {
        self.width == 0.0 && self.height == 0.0
    }

    /// Quad positions (x0, y0, x1, y1)
    pub fn quad(&self, x: f32, y: f32) -> [f32; 4] {
        [
            x + self.bearing_x,
            y - self.bearing_y,
            x + self.bearing_x + self.width,
            y - self.bearing_y + self.height,
        ]
    }
}

/// Glyph UV coordinates
#[derive(Clone, Copy, Debug, Default)]
pub struct GlyphUV {
    /// Left U
    pub u0: f32,
    /// Top V
    pub v0: f32,
    /// Right U
    pub u1: f32,
    /// Bottom V
    pub v1: f32,
}

impl GlyphUV {
    /// Zero UVs
    pub const ZERO: Self = Self {
        u0: 0.0,
        v0: 0.0,
        u1: 0.0,
        v1: 0.0,
    };

    /// Creates UV from rect in atlas
    pub fn from_rect(x: u32, y: u32, w: u32, h: u32, atlas_w: u32, atlas_h: u32) -> Self {
        let inv_w = 1.0 / atlas_w as f32;
        let inv_h = 1.0 / atlas_h as f32;
        Self {
            u0: x as f32 * inv_w,
            v0: y as f32 * inv_h,
            u1: (x + w) as f32 * inv_w,
            v1: (y + h) as f32 * inv_h,
        }
    }

    /// To array
    pub const fn to_array(&self) -> [f32; 4] {
        [self.u0, self.v0, self.u1, self.v1]
    }
}

// ============================================================================
// Font Atlas
// ============================================================================

/// Font atlas configuration
#[derive(Clone, Debug)]
pub struct FontAtlasConfig {
    /// Atlas width
    pub width: u32,
    /// Atlas height
    pub height: u32,
    /// Padding between glyphs
    pub padding: u32,
    /// Oversample X
    pub oversample_x: u32,
    /// Oversample Y
    pub oversample_y: u32,
    /// SDF spread (if using SDF)
    pub sdf_spread: u32,
    /// Allow multiple pages
    pub multi_page: bool,
}

impl FontAtlasConfig {
    /// Creates default config
    pub fn new() -> Self {
        Self {
            width: 1024,
            height: 1024,
            padding: 2,
            oversample_x: 1,
            oversample_y: 1,
            sdf_spread: 8,
            multi_page: true,
        }
    }

    /// Small atlas (512x512)
    pub fn small() -> Self {
        Self {
            width: 512,
            height: 512,
            ..Self::new()
        }
    }

    /// Large atlas (2048x2048)
    pub fn large() -> Self {
        Self {
            width: 2048,
            height: 2048,
            ..Self::new()
        }
    }

    /// With SDF
    pub fn with_sdf(mut self, spread: u32) -> Self {
        self.sdf_spread = spread;
        self
    }

    /// With oversampling
    pub fn with_oversample(mut self, x: u32, y: u32) -> Self {
        self.oversample_x = x;
        self.oversample_y = y;
        self
    }
}

impl Default for FontAtlasConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Font atlas page
#[derive(Clone, Debug)]
pub struct FontAtlasPage {
    /// Page index
    pub index: u32,
    /// Texture handle
    pub texture: u64,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Dirty region (needs upload)
    pub dirty: Option<AtlasDirtyRect>,
}

/// Atlas dirty rectangle
#[derive(Clone, Copy, Debug)]
pub struct AtlasDirtyRect {
    /// X offset
    pub x: u32,
    /// Y offset
    pub y: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

// ============================================================================
// Text Layout
// ============================================================================

/// Text layout settings
#[derive(Clone, Debug)]
pub struct TextLayoutSettings {
    /// Font handle
    pub font: FontHandle,
    /// Font size
    pub font_size: f32,
    /// Line height multiplier
    pub line_height: f32,
    /// Letter spacing
    pub letter_spacing: f32,
    /// Word spacing
    pub word_spacing: f32,
    /// Tab width (in spaces)
    pub tab_width: u32,
    /// Horizontal alignment
    pub h_align: TextHAlign,
    /// Vertical alignment
    pub v_align: TextVAlign,
    /// Text direction
    pub direction: TextDirection,
    /// Wrap mode
    pub wrap: TextWrap,
    /// Max width (for wrapping)
    pub max_width: Option<f32>,
    /// Max height (for truncation)
    pub max_height: Option<f32>,
    /// Truncation
    pub truncation: TextTruncation,
}

impl TextLayoutSettings {
    /// Creates default settings
    pub fn new(font: FontHandle, font_size: f32) -> Self {
        Self {
            font,
            font_size,
            line_height: 1.0,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            tab_width: 4,
            h_align: TextHAlign::Left,
            v_align: TextVAlign::Top,
            direction: TextDirection::LeftToRight,
            wrap: TextWrap::Word,
            max_width: None,
            max_height: None,
            truncation: TextTruncation::None,
        }
    }

    /// With alignment
    pub fn with_align(mut self, h: TextHAlign, v: TextVAlign) -> Self {
        self.h_align = h;
        self.v_align = v;
        self
    }

    /// Centered
    pub fn centered(mut self) -> Self {
        self.h_align = TextHAlign::Center;
        self.v_align = TextVAlign::Center;
        self
    }

    /// With line height
    pub fn with_line_height(mut self, multiplier: f32) -> Self {
        self.line_height = multiplier;
        self
    }

    /// With letter spacing
    pub fn with_letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = spacing;
        self
    }

    /// With max width
    pub fn with_max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// With wrap mode
    pub fn with_wrap(mut self, wrap: TextWrap) -> Self {
        self.wrap = wrap;
        self
    }

    /// No wrapping
    pub fn no_wrap(mut self) -> Self {
        self.wrap = TextWrap::None;
        self
    }

    /// With truncation
    pub fn with_truncation(mut self, truncation: TextTruncation) -> Self {
        self.truncation = truncation;
        self
    }
}

/// Horizontal text alignment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextHAlign {
    /// Left aligned
    #[default]
    Left    = 0,
    /// Center aligned
    Center  = 1,
    /// Right aligned
    Right   = 2,
    /// Justified
    Justify = 3,
}

/// Vertical text alignment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextVAlign {
    /// Top aligned
    #[default]
    Top      = 0,
    /// Center aligned
    Center   = 1,
    /// Bottom aligned
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
}

/// Text wrap mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextWrap {
    /// No wrapping
    None            = 0,
    /// Wrap at word boundaries
    #[default]
    Word            = 1,
    /// Wrap at character boundaries
    Character       = 2,
    /// Wrap at word, then character
    WordOrCharacter = 3,
}

/// Text truncation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextTruncation {
    /// No truncation
    #[default]
    None     = 0,
    /// Clip
    Clip     = 1,
    /// Ellipsis at end
    Ellipsis = 2,
    /// Fade out
    Fade     = 3,
}

// ============================================================================
// Laid Out Text
// ============================================================================

/// Laid out text result
#[derive(Clone, Debug)]
pub struct LaidOutText {
    /// Positioned glyphs
    pub glyphs: Vec<PositionedGlyph>,
    /// Lines
    pub lines: Vec<TextLine>,
    /// Total width
    pub width: f32,
    /// Total height
    pub height: f32,
    /// Caret positions
    pub caret_positions: Vec<CaretPosition>,
}

impl LaidOutText {
    /// Creates empty layout
    pub fn empty() -> Self {
        Self {
            glyphs: Vec::new(),
            lines: Vec::new(),
            width: 0.0,
            height: 0.0,
            caret_positions: Vec::new(),
        }
    }

    /// Glyph count
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// Line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Hit test (returns character index)
    pub fn hit_test(&self, x: f32, y: f32) -> Option<usize> {
        // Find line
        let line_idx = self
            .lines
            .iter()
            .position(|line| y >= line.y && y < line.y + line.height)?;

        let line = &self.lines[line_idx];

        // Find glyph in line
        for i in line.glyph_start..line.glyph_end {
            let glyph = &self.glyphs[i];
            if x >= glyph.x && x < glyph.x + glyph.advance_x {
                return Some(glyph.char_index);
            }
        }

        // Return end of line
        if line.glyph_end > 0 {
            Some(self.glyphs[line.glyph_end - 1].char_index + 1)
        } else {
            None
        }
    }

    /// Gets caret position for character index
    pub fn caret_at(&self, char_index: usize) -> Option<CaretPosition> {
        self.caret_positions.get(char_index).copied()
    }
}

/// Positioned glyph
#[derive(Clone, Copy, Debug)]
pub struct PositionedGlyph {
    /// Glyph info
    pub glyph: GlyphInfo,
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Advance X
    pub advance_x: f32,
    /// Character index in source string
    pub char_index: usize,
    /// Color (if per-glyph coloring)
    pub color: [f32; 4],
}

impl PositionedGlyph {
    /// Gets quad corners
    pub fn quad_positions(&self) -> [[f32; 2]; 4] {
        let [x0, y0, x1, y1] = self.glyph.quad(self.x, self.y);
        [
            [x0, y0], // Top-left
            [x1, y0], // Top-right
            [x1, y1], // Bottom-right
            [x0, y1], // Bottom-left
        ]
    }

    /// Gets UV coordinates
    pub fn quad_uvs(&self) -> [[f32; 2]; 4] {
        let uv = &self.glyph.uv;
        [
            [uv.u0, uv.v0], // Top-left
            [uv.u1, uv.v0], // Top-right
            [uv.u1, uv.v1], // Bottom-right
            [uv.u0, uv.v1], // Bottom-left
        ]
    }
}

/// Text line info
#[derive(Clone, Copy, Debug)]
pub struct TextLine {
    /// Start glyph index
    pub glyph_start: usize,
    /// End glyph index (exclusive)
    pub glyph_end: usize,
    /// Y position
    pub y: f32,
    /// Line width
    pub width: f32,
    /// Line height
    pub height: f32,
    /// Baseline Y offset
    pub baseline: f32,
}

/// Caret position
#[derive(Clone, Copy, Debug)]
pub struct CaretPosition {
    /// X position
    pub x: f32,
    /// Y position (top)
    pub y: f32,
    /// Height
    pub height: f32,
    /// Line index
    pub line: usize,
}

// ============================================================================
// Text Vertex
// ============================================================================

/// Text vertex for GPU rendering
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TextVertex {
    /// Position
    pub position: [f32; 2],
    /// UV coordinates
    pub uv: [f32; 2],
    /// Color
    pub color: [f32; 4],
}

impl TextVertex {
    /// Creates vertex
    pub const fn new(x: f32, y: f32, u: f32, v: f32, color: [f32; 4]) -> Self {
        Self {
            position: [x, y],
            uv: [u, v],
            color,
        }
    }
}

/// Text draw data
#[derive(Clone, Debug, Default)]
pub struct TextDrawData {
    /// Vertices
    pub vertices: Vec<TextVertex>,
    /// Indices
    pub indices: Vec<u32>,
    /// Atlas pages used
    pub atlas_pages: Vec<u32>,
}

impl TextDrawData {
    /// Creates empty draw data
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears draw data
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
        self.atlas_pages.clear();
    }

    /// Adds quad
    pub fn add_quad(&mut self, glyph: &PositionedGlyph) {
        let base = self.vertices.len() as u32;
        let positions = glyph.quad_positions();
        let uvs = glyph.quad_uvs();

        for i in 0..4 {
            self.vertices.push(TextVertex {
                position: positions[i],
                uv: uvs[i],
                color: glyph.color,
            });
        }

        // Two triangles
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

        if !self.atlas_pages.contains(&glyph.glyph.atlas_page) {
            self.atlas_pages.push(glyph.glyph.atlas_page);
        }
    }

    /// Builds from laid out text
    pub fn build(&mut self, text: &LaidOutText) {
        self.clear();
        for glyph in &text.glyphs {
            if !glyph.glyph.is_whitespace() {
                self.add_quad(glyph);
            }
        }
    }

    /// Vertex count
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Index count
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }
}

// ============================================================================
// Rich Text
// ============================================================================

/// Rich text span
#[derive(Clone, Debug)]
pub struct TextSpan {
    /// Text content
    pub text: String,
    /// Style
    pub style: TextStyle,
}

impl TextSpan {
    /// Creates span
    pub fn new(text: &str, style: TextStyle) -> Self {
        Self {
            text: String::from(text),
            style,
        }
    }

    /// Plain span
    pub fn plain(text: &str) -> Self {
        Self::new(text, TextStyle::default())
    }

    /// Bold span
    pub fn bold(text: &str) -> Self {
        Self::new(text, TextStyle::default().bold())
    }

    /// Italic span
    pub fn italic(text: &str) -> Self {
        Self::new(text, TextStyle::default().italic())
    }

    /// Colored span
    pub fn colored(text: &str, color: [f32; 4]) -> Self {
        Self::new(text, TextStyle::default().with_color(color))
    }
}

/// Text style
#[derive(Clone, Debug)]
pub struct TextStyle {
    /// Font (None = inherit)
    pub font: Option<FontHandle>,
    /// Font size (None = inherit)
    pub font_size: Option<f32>,
    /// Color
    pub color: [f32; 4],
    /// Bold
    pub is_bold: bool,
    /// Italic
    pub is_italic: bool,
    /// Underline
    pub underline: bool,
    /// Strikethrough
    pub strikethrough: bool,
    /// Superscript
    pub superscript: bool,
    /// Subscript
    pub subscript: bool,
}

impl TextStyle {
    /// Default style
    pub fn new() -> Self {
        Self {
            font: None,
            font_size: None,
            color: [1.0, 1.0, 1.0, 1.0],
            is_bold: false,
            is_italic: false,
            underline: false,
            strikethrough: false,
            superscript: false,
            subscript: false,
        }
    }

    /// With font
    pub fn with_font(mut self, font: FontHandle) -> Self {
        self.font = Some(font);
        self
    }

    /// With font size
    pub fn with_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    /// With color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Bold
    pub fn bold(mut self) -> Self {
        self.is_bold = true;
        self
    }

    /// Italic
    pub fn italic(mut self) -> Self {
        self.is_italic = true;
        self
    }

    /// Underline
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Strikethrough
    pub fn strikethrough(mut self) -> Self {
        self.strikethrough = true;
        self
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self::new()
    }
}

/// Rich text document
#[derive(Clone, Debug, Default)]
pub struct RichText {
    /// Spans
    pub spans: Vec<TextSpan>,
}

impl RichText {
    /// Creates empty rich text
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    /// Adds span
    pub fn push(&mut self, span: TextSpan) {
        self.spans.push(span);
    }

    /// Adds plain text
    pub fn text(mut self, text: &str) -> Self {
        self.push(TextSpan::plain(text));
        self
    }

    /// Adds bold text
    pub fn bold(mut self, text: &str) -> Self {
        self.push(TextSpan::bold(text));
        self
    }

    /// Adds italic text
    pub fn italic(mut self, text: &str) -> Self {
        self.push(TextSpan::italic(text));
        self
    }

    /// Adds colored text
    pub fn colored(mut self, text: &str, color: [f32; 4]) -> Self {
        self.push(TextSpan::colored(text, color));
        self
    }

    /// Total character count
    pub fn len(&self) -> usize {
        self.spans.iter().map(|s| s.text.len()).sum()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty() || self.len() == 0
    }
}
