//! # LUMINA UI
//!
//! Revolutionary GPU-accelerated UI framework for LUMINA.
//!
//! ## Features
//!
//! - **Immediate Mode**: Simple, stateless UI for tools and debug
//! - **Retained Mode**: High-performance reactive UI for applications
//! - **GPU Text**: Signed distance field text rendering
//! - **Flex Layout**: CSS-like flexbox layout system
//! - **Animations**: Hardware-accelerated animations
//! - **Theming**: Complete theme system
//! - **Accessibility**: Screen reader and keyboard support

#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

pub mod animation;
pub mod immediate;
pub mod input;
pub mod layout;
pub mod render;
pub mod retained;
pub mod style;
pub mod text;
pub mod theme;
pub mod widgets;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// UI context
pub struct UiContext {
    /// Frame counter
    frame: u64,
    /// Screen dimensions
    screen_size: [f32; 2],
    /// DPI scale
    dpi_scale: f32,
    /// Input state
    input: InputState,
    /// Active theme
    theme: Theme,
    /// Font atlas
    font_atlas: FontAtlas,
    /// Render commands
    commands: Vec<RenderCommand>,
    /// Layout cache
    layout_cache: LayoutCache,
    /// Animation state
    animations: AnimationState,
    /// Widget state storage
    state: WidgetStateStorage,
    /// Next widget ID
    next_id: AtomicU64,
}

impl UiContext {
    /// Create a new UI context
    pub fn new(screen_size: [f32; 2], dpi_scale: f32) -> Self {
        Self {
            frame: 0,
            screen_size,
            dpi_scale,
            input: InputState::default(),
            theme: Theme::dark(),
            font_atlas: FontAtlas::new(),
            commands: Vec::new(),
            layout_cache: LayoutCache::new(),
            animations: AnimationState::new(),
            state: WidgetStateStorage::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Begin a new frame
    pub fn begin_frame(&mut self, input: InputState) {
        self.frame += 1;
        self.input = input;
        self.commands.clear();
        self.layout_cache.new_frame();
    }

    /// End the frame and return render commands
    pub fn end_frame(&mut self) -> &[RenderCommand] {
        self.animations.update(1.0 / 60.0);
        &self.commands
    }

    /// Set theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Get current theme
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Generate a unique widget ID
    pub fn next_id(&self) -> WidgetId {
        WidgetId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Get or create widget state
    pub fn state<T: Default + 'static>(&mut self, id: WidgetId) -> &mut T {
        self.state.get_or_insert::<T>(id)
    }

    /// Check if mouse is over a rect
    pub fn is_hovered(&self, rect: Rect) -> bool {
        rect.contains(self.input.mouse_pos)
    }

    /// Check if a rect is being clicked
    pub fn is_clicked(&self, rect: Rect) -> bool {
        self.is_hovered(rect) && self.input.mouse_clicked[0]
    }

    /// Check if a rect is being pressed
    pub fn is_pressed(&self, rect: Rect) -> bool {
        self.is_hovered(rect) && self.input.mouse_down[0]
    }

    /// Push a draw command
    pub fn draw(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }

    /// Draw a filled rectangle
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        self.commands.push(RenderCommand::Rect {
            rect,
            color,
            corner_radius: 0.0,
        });
    }

    /// Draw a rounded rectangle
    pub fn draw_rounded_rect(&mut self, rect: Rect, color: Color, radius: f32) {
        self.commands.push(RenderCommand::Rect {
            rect,
            color,
            corner_radius: radius,
        });
    }

    /// Draw text
    pub fn draw_text(&mut self, pos: [f32; 2], text: &str, style: TextStyle) {
        self.commands.push(RenderCommand::Text {
            position: pos,
            text: text.into(),
            style,
        });
    }

    /// Draw an image
    pub fn draw_image(&mut self, rect: Rect, texture: TextureId, tint: Color) {
        self.commands.push(RenderCommand::Image {
            rect,
            texture,
            uv: [0.0, 0.0, 1.0, 1.0],
            tint,
        });
    }

    /// Start an animation
    pub fn animate(&mut self, id: WidgetId, property: &str, target: f32, duration: f32) {
        self.animations.start(id, property, target, duration);
    }

    /// Get animated value
    pub fn animated_value(&self, id: WidgetId, property: &str, default: f32) -> f32 {
        self.animations.get_value(id, property).unwrap_or(default)
    }
}

/// Widget ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetId(u64);

impl WidgetId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn from_str(s: &str) -> Self {
        let mut hash = 0u64;
        for byte in s.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
        }
        Self(hash)
    }
}

/// Rectangle
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_min_max(min: [f32; 2], max: [f32; 2]) -> Self {
        Self {
            x: min[0],
            y: min[1],
            width: max[0] - min[0],
            height: max[1] - min[1],
        }
    }

    pub fn min(&self) -> [f32; 2] {
        [self.x, self.y]
    }

    pub fn max(&self) -> [f32; 2] {
        [self.x + self.width, self.y + self.height]
    }

    pub fn center(&self) -> [f32; 2] {
        [self.x + self.width * 0.5, self.y + self.height * 0.5]
    }

    pub fn contains(&self, point: [f32; 2]) -> bool {
        point[0] >= self.x
            && point[0] < self.x + self.width
            && point[1] >= self.y
            && point[1] < self.y + self.height
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }

    pub fn expand(&self, amount: f32) -> Rect {
        Rect {
            x: self.x - amount,
            y: self.y - amount,
            width: self.width + amount * 2.0,
            height: self.height + amount * 2.0,
        }
    }

    pub fn shrink(&self, amount: f32) -> Rect {
        self.expand(-amount)
    }
}

/// Color (RGBA)
#[derive(Debug, Clone, Copy, Default)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as f32 / 255.0,
            g: ((hex >> 8) & 0xFF) as f32 / 255.0,
            b: (hex & 0xFF) as f32 / 255.0,
            a: 1.0,
        }
    }

    pub fn with_alpha(self, alpha: f32) -> Self {
        Self { a: alpha, ..self }
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    // Common colors
    pub const WHITE: Self = Self::rgba(1.0, 1.0, 1.0, 1.0);
    pub const BLACK: Self = Self::rgba(0.0, 0.0, 0.0, 1.0);
    pub const RED: Self = Self::rgba(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::rgba(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::rgba(0.0, 0.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
}

/// Input state
#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub mouse_pos: [f32; 2],
    pub mouse_delta: [f32; 2],
    pub mouse_down: [bool; 3],
    pub mouse_clicked: [bool; 3],
    pub mouse_released: [bool; 3],
    pub scroll_delta: [f32; 2],
    pub keys_down: [bool; 256],
    pub keys_pressed: [bool; 256],
    pub keys_released: [bool; 256],
    pub modifiers: Modifiers,
    pub text_input: String,
}

/// Keyboard modifiers
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub super_key: bool,
}

/// Render command
#[derive(Debug, Clone)]
pub enum RenderCommand {
    Rect {
        rect: Rect,
        color: Color,
        corner_radius: f32,
    },
    RectOutline {
        rect: Rect,
        color: Color,
        thickness: f32,
        corner_radius: f32,
    },
    Text {
        position: [f32; 2],
        text: String,
        style: TextStyle,
    },
    Image {
        rect: Rect,
        texture: TextureId,
        uv: [f32; 4],
        tint: Color,
    },
    Line {
        start: [f32; 2],
        end: [f32; 2],
        color: Color,
        thickness: f32,
    },
    Triangle {
        points: [[f32; 2]; 3],
        color: Color,
    },
    Circle {
        center: [f32; 2],
        radius: f32,
        color: Color,
    },
    Clip {
        rect: Rect,
    },
    PopClip,
    Custom {
        id: u32,
        data: Vec<u8>,
    },
}

/// Text style
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub font: FontId,
    pub size: f32,
    pub color: Color,
    pub align: TextAlign,
    pub line_height: f32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: FontId(0),
            size: 14.0,
            color: Color::WHITE,
            align: TextAlign::Left,
            line_height: 1.2,
        }
    }
}

/// Text alignment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Font ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontId(pub u32);

/// Texture ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureId(pub u64);

/// Theme
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: ThemeColors,
    pub sizes: ThemeSizes,
    pub fonts: ThemeFonts,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            name: "Dark".into(),
            colors: ThemeColors {
                background: Color::hex(0x1E1E1E),
                surface: Color::hex(0x252526),
                primary: Color::hex(0x007ACC),
                secondary: Color::hex(0x3C3C3C),
                text: Color::hex(0xCCCCCC),
                text_secondary: Color::hex(0x808080),
                border: Color::hex(0x474747),
                error: Color::hex(0xF44747),
                warning: Color::hex(0xCCA700),
                success: Color::hex(0x4EC9B0),
            },
            sizes: ThemeSizes::default(),
            fonts: ThemeFonts::default(),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "Light".into(),
            colors: ThemeColors {
                background: Color::hex(0xFFFFFF),
                surface: Color::hex(0xF3F3F3),
                primary: Color::hex(0x0078D4),
                secondary: Color::hex(0xE1E1E1),
                text: Color::hex(0x1E1E1E),
                text_secondary: Color::hex(0x6E6E6E),
                border: Color::hex(0xCCCCCC),
                error: Color::hex(0xE51400),
                warning: Color::hex(0xF7630C),
                success: Color::hex(0x107C10),
            },
            sizes: ThemeSizes::default(),
            fonts: ThemeFonts::default(),
        }
    }
}

/// Theme colors
#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub background: Color,
    pub surface: Color,
    pub primary: Color,
    pub secondary: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub border: Color,
    pub error: Color,
    pub warning: Color,
    pub success: Color,
}

/// Theme sizes
#[derive(Debug, Clone)]
pub struct ThemeSizes {
    pub border_radius: f32,
    pub border_width: f32,
    pub padding: f32,
    pub spacing: f32,
    pub icon_size: f32,
}

impl Default for ThemeSizes {
    fn default() -> Self {
        Self {
            border_radius: 4.0,
            border_width: 1.0,
            padding: 8.0,
            spacing: 4.0,
            icon_size: 16.0,
        }
    }
}

/// Theme fonts
#[derive(Debug, Clone)]
pub struct ThemeFonts {
    pub regular: FontId,
    pub bold: FontId,
    pub italic: FontId,
    pub mono: FontId,
    pub heading: FontId,
}

impl Default for ThemeFonts {
    fn default() -> Self {
        Self {
            regular: FontId(0),
            bold: FontId(1),
            italic: FontId(2),
            mono: FontId(3),
            heading: FontId(4),
        }
    }
}

/// Font atlas for GPU text rendering
pub struct FontAtlas {
    width: u32,
    height: u32,
    glyphs: BTreeMap<(FontId, char), GlyphInfo>,
}

impl FontAtlas {
    pub fn new() -> Self {
        Self {
            width: 1024,
            height: 1024,
            glyphs: BTreeMap::new(),
        }
    }

    pub fn get_glyph(&self, font: FontId, c: char) -> Option<&GlyphInfo> {
        self.glyphs.get(&(font, c))
    }
}

impl Default for FontAtlas {
    fn default() -> Self {
        Self::new()
    }
}

/// Glyph information
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    pub uv: [f32; 4],
    pub size: [f32; 2],
    pub offset: [f32; 2],
    pub advance: f32,
}

/// Layout cache
struct LayoutCache {
    entries: BTreeMap<WidgetId, CachedLayout>,
    frame: u64,
}

impl LayoutCache {
    fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            frame: 0,
        }
    }

    fn new_frame(&mut self) {
        self.frame += 1;
        // Remove stale entries
        self.entries.retain(|_, e| e.frame >= self.frame - 2);
    }
}

struct CachedLayout {
    rect: Rect,
    frame: u64,
}

/// Animation state
struct AnimationState {
    animations: BTreeMap<(WidgetId, u64), Animation>,
}

impl AnimationState {
    fn new() -> Self {
        Self {
            animations: BTreeMap::new(),
        }
    }

    fn start(&mut self, id: WidgetId, property: &str, target: f32, duration: f32) {
        let key = (id, hash_str(property));
        let current = self.get_value(id, property).unwrap_or(0.0);

        self.animations.insert(key, Animation {
            start: current,
            target,
            duration,
            elapsed: 0.0,
        });
    }

    fn get_value(&self, id: WidgetId, property: &str) -> Option<f32> {
        let key = (id, hash_str(property));
        self.animations.get(&key).map(|a| {
            let t = (a.elapsed / a.duration).min(1.0);
            let t = ease_out_cubic(t);
            a.start + (a.target - a.start) * t
        })
    }

    fn update(&mut self, dt: f32) {
        for anim in self.animations.values_mut() {
            anim.elapsed += dt;
        }
        self.animations.retain(|_, a| a.elapsed < a.duration);
    }
}

struct Animation {
    start: f32,
    target: f32,
    duration: f32,
    elapsed: f32,
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn hash_str(s: &str) -> u64 {
    let mut hash = 0u64;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    hash
}

/// Widget state storage
struct WidgetStateStorage {
    states: BTreeMap<(WidgetId, core::any::TypeId), Box<dyn core::any::Any>>,
}

impl WidgetStateStorage {
    fn new() -> Self {
        Self {
            states: BTreeMap::new(),
        }
    }

    fn get_or_insert<T: Default + 'static>(&mut self, _id: WidgetId) -> &mut T {
        // Simplified - would use proper type erasure
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10.0, 10.0, 100.0, 50.0);
        assert!(rect.contains([50.0, 30.0]));
        assert!(!rect.contains([0.0, 0.0]));
        assert!(!rect.contains([200.0, 100.0]));
    }

    #[test]
    fn test_color_lerp() {
        let white = Color::WHITE;
        let black = Color::BLACK;
        let gray = white.lerp(black, 0.5);
        assert!((gray.r - 0.5).abs() < 0.01);
    }
}
