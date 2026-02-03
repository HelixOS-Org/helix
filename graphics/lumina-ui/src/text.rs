//! # GPU Text Rendering
//!
//! Signed Distance Field text rendering for sharp text at any scale.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::{Color, FontId, Rect, RenderCommand, TextAlign, TextStyle};

/// Text renderer using SDF
pub struct TextRenderer {
    fonts: BTreeMap<FontId, Font>,
    atlas: FontAtlas,
    config: TextConfig,
}

impl TextRenderer {
    pub fn new() -> Self {
        Self {
            fonts: BTreeMap::new(),
            atlas: FontAtlas::new(1024, 1024),
            config: TextConfig::default(),
        }
    }

    /// Add a font
    pub fn add_font(&mut self, id: FontId, font: Font) {
        self.fonts.insert(id, font);
    }

    /// Measure text dimensions
    pub fn measure(&self, text: &str, style: &TextStyle) -> [f32; 2] {
        let font = match self.fonts.get(&style.font) {
            Some(f) => f,
            None => return [0.0, 0.0],
        };

        let scale = style.size / font.size;
        let mut width = 0.0;
        let mut max_height = 0.0f32;

        for c in text.chars() {
            if let Some(glyph) = font.glyphs.get(&c) {
                width += glyph.advance * scale;
                max_height = max_height.max(glyph.height * scale);
            }
        }

        [width, max_height * style.line_height]
    }

    /// Generate render commands for text
    pub fn render(
        &self,
        text: &str,
        position: [f32; 2],
        style: &TextStyle,
        max_width: Option<f32>,
    ) -> Vec<TextQuad> {
        let font = match self.fonts.get(&style.font) {
            Some(f) => f,
            None => return Vec::new(),
        };

        let scale = style.size / font.size;
        let mut quads = Vec::new();
        let mut x = position[0];
        let mut y = position[1];

        // Word wrapping
        let lines = if let Some(max_w) = max_width {
            wrap_text(text, max_w, font, scale)
        } else {
            vec![text.into()]
        };

        for (line_idx, line) in lines.iter().enumerate() {
            // Horizontal alignment
            let line_width = self.measure_line(line, font, scale);
            x = match style.align {
                TextAlign::Left => position[0],
                TextAlign::Center => position[0] - line_width / 2.0,
                TextAlign::Right => position[0] - line_width,
            };

            for c in line.chars() {
                if let Some(glyph) = font.glyphs.get(&c) {
                    let quad = TextQuad {
                        position: [x + glyph.offset_x * scale, y + glyph.offset_y * scale],
                        size: [glyph.width * scale, glyph.height * scale],
                        uv_min: [glyph.uv_x, glyph.uv_y],
                        uv_max: [glyph.uv_x + glyph.uv_width, glyph.uv_y + glyph.uv_height],
                        color: style.color,
                    };
                    quads.push(quad);
                    x += glyph.advance * scale;
                }
            }

            y += style.size * style.line_height;
        }

        quads
    }

    fn measure_line(&self, text: &str, font: &Font, scale: f32) -> f32 {
        let mut width = 0.0;
        for c in text.chars() {
            if let Some(glyph) = font.glyphs.get(&c) {
                width += glyph.advance * scale;
            }
        }
        width
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Font data
pub struct Font {
    pub id: FontId,
    pub name: String,
    pub size: f32,
    pub line_height: f32,
    pub ascender: f32,
    pub descender: f32,
    pub glyphs: BTreeMap<char, Glyph>,
}

/// Glyph data
#[derive(Debug, Clone)]
pub struct Glyph {
    pub code: char,
    pub width: f32,
    pub height: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub advance: f32,
    pub uv_x: f32,
    pub uv_y: f32,
    pub uv_width: f32,
    pub uv_height: f32,
}

/// Text rendering quad
#[derive(Debug, Clone)]
pub struct TextQuad {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub color: Color,
}

/// Font atlas
pub struct FontAtlas {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    packer: AtlasPacker,
}

impl FontAtlas {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
            packer: AtlasPacker::new(width, height),
        }
    }

    /// Add glyph to atlas
    pub fn add_glyph(&mut self, width: u32, height: u32, data: &[u8]) -> Option<[u32; 2]> {
        let pos = self.packer.pack(width, height)?;

        for y in 0..height {
            for x in 0..width {
                let src_idx = (y * width + x) as usize;
                let dst_idx = ((pos[1] + y) * self.width + pos[0] + x) as usize;
                self.pixels[dst_idx] = data[src_idx];
            }
        }

        Some(pos)
    }
}

/// Simple bin packer for atlas
struct AtlasPacker {
    width: u32,
    height: u32,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
}

impl AtlasPacker {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            cursor_x: 1,
            cursor_y: 1,
            row_height: 0,
        }
    }

    fn pack(&mut self, width: u32, height: u32) -> Option<[u32; 2]> {
        // Check if we need a new row
        if self.cursor_x + width + 1 > self.width {
            self.cursor_x = 1;
            self.cursor_y += self.row_height + 1;
            self.row_height = 0;
        }

        // Check if we have space
        if self.cursor_y + height + 1 > self.height {
            return None;
        }

        let pos = [self.cursor_x, self.cursor_y];
        self.cursor_x += width + 1;
        self.row_height = self.row_height.max(height);

        Some(pos)
    }
}

/// Text configuration
#[derive(Debug, Clone)]
pub struct TextConfig {
    pub sdf_pixel_range: f32,
    pub gamma: f32,
    pub subpixel: bool,
}

impl Default for TextConfig {
    fn default() -> Self {
        Self {
            sdf_pixel_range: 4.0,
            gamma: 1.0,
            subpixel: true,
        }
    }
}

fn wrap_text(text: &str, max_width: f32, font: &Font, scale: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0.0;

    for word in text.split_whitespace() {
        let word_width = measure_word(word, font, scale);
        let space_width = font
            .glyphs
            .get(&' ')
            .map(|g| g.advance * scale)
            .unwrap_or(4.0);

        if current_width + word_width > max_width && !current_line.is_empty() {
            lines.push(current_line);
            current_line = String::new();
            current_width = 0.0;
        }

        if !current_line.is_empty() {
            current_line.push(' ');
            current_width += space_width;
        }

        current_line.push_str(word);
        current_width += word_width;
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

fn measure_word(word: &str, font: &Font, scale: f32) -> f32 {
    let mut width = 0.0;
    for c in word.chars() {
        if let Some(glyph) = font.glyphs.get(&c) {
            width += glyph.advance * scale;
        }
    }
    width
}

/// Rich text with formatting
#[derive(Debug, Clone)]
pub struct RichText {
    pub spans: Vec<TextSpan>,
}

impl RichText {
    pub fn new() -> Self {
        Self { spans: Vec::new() }
    }

    pub fn plain(text: &str) -> Self {
        Self {
            spans: vec![TextSpan {
                text: text.into(),
                style: SpanStyle::default(),
            }],
        }
    }

    pub fn push(&mut self, text: &str, style: SpanStyle) {
        self.spans.push(TextSpan {
            text: text.into(),
            style,
        });
    }

    pub fn bold(mut self, text: &str) -> Self {
        self.spans.push(TextSpan {
            text: text.into(),
            style: SpanStyle {
                bold: true,
                ..Default::default()
            },
        });
        self
    }

    pub fn italic(mut self, text: &str) -> Self {
        self.spans.push(TextSpan {
            text: text.into(),
            style: SpanStyle {
                italic: true,
                ..Default::default()
            },
        });
        self
    }

    pub fn colored(mut self, text: &str, color: Color) -> Self {
        self.spans.push(TextSpan {
            text: text.into(),
            style: SpanStyle {
                color: Some(color),
                ..Default::default()
            },
        });
        self
    }
}

impl Default for RichText {
    fn default() -> Self {
        Self::new()
    }
}

/// Text span
#[derive(Debug, Clone)]
pub struct TextSpan {
    pub text: String,
    pub style: SpanStyle,
}

/// Span style
#[derive(Debug, Clone, Default)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub color: Option<Color>,
    pub size: Option<f32>,
    pub font: Option<FontId>,
}
