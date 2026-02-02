//! Color types and utilities
//!
//! Provides color representations for use in rendering operations
//! such as clear colors and constant colors.

use crate::output::Rgba8;
use lumina_math::Vec4;

/// An RGBA color with floating-point components
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct Color {
    /// Red component (0.0 - 1.0)
    pub r: f32,
    /// Green component (0.0 - 1.0)
    pub g: f32,
    /// Blue component (0.0 - 1.0)
    pub b: f32,
    /// Alpha component (0.0 - 1.0)
    pub a: f32,
}

impl Color {
    // ═══════════════════════════════════════════════════════════════════════
    // PREDEFINED COLORS
    // ═══════════════════════════════════════════════════════════════════════

    /// Black (#000000)
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    /// White (#FFFFFF)
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    /// Red (#FF0000)
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    /// Green (#00FF00)
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    /// Blue (#0000FF)
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    /// Yellow (#FFFF00)
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    /// Cyan (#00FFFF)
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    /// Magenta (#FF00FF)
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
    /// Transparent (0, 0, 0, 0)
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    /// Cornflower blue (#6495ED) - classic clear color
    pub const CORNFLOWER_BLUE: Self = Self::rgb(0.392, 0.584, 0.929);
    /// Dark gray (#333333)
    pub const DARK_GRAY: Self = Self::rgb(0.2, 0.2, 0.2);
    /// Light gray (#CCCCCC)
    pub const LIGHT_GRAY: Self = Self::rgb(0.8, 0.8, 0.8);

    // ═══════════════════════════════════════════════════════════════════════
    // CONSTRUCTORS
    // ═══════════════════════════════════════════════════════════════════════

    /// Creates a new color with the given RGBA components
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates a new opaque color with the given RGB components
    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Creates a color from 8-bit RGBA values (0-255)
    #[inline]
    pub const fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    /// Creates a color from a 32-bit hex value (0xRRGGBBAA)
    #[inline]
    pub const fn from_hex(hex: u32) -> Self {
        Self::from_rgba8(
            ((hex >> 24) & 0xFF) as u8,
            ((hex >> 16) & 0xFF) as u8,
            ((hex >> 8) & 0xFF) as u8,
            (hex & 0xFF) as u8,
        )
    }

    /// Creates an opaque color from a 24-bit hex value (0xRRGGBB)
    #[inline]
    pub const fn from_rgb_hex(hex: u32) -> Self {
        Self::from_rgba8(
            ((hex >> 16) & 0xFF) as u8,
            ((hex >> 8) & 0xFF) as u8,
            (hex & 0xFF) as u8,
            255,
        )
    }

    /// Creates a grayscale color
    #[inline]
    pub const fn gray(value: f32) -> Self {
        Self::rgb(value, value, value)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // CONVERSIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Converts to a Vec4
    #[inline]
    pub const fn to_vec4(self) -> Vec4 {
        Vec4::new(self.r, self.g, self.b, self.a)
    }

    /// Converts to an array
    #[inline]
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Converts to Rgba8 format
    #[inline]
    pub fn to_rgba8(self) -> Rgba8 {
        Rgba8 {
            r: (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            g: (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            b: (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            a: (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════

    /// Returns this color with a different alpha value
    #[inline]
    pub const fn with_alpha(self, a: f32) -> Self {
        Self::new(self.r, self.g, self.b, a)
    }

    /// Linearly interpolates between two colors
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }

    /// Returns the color in linear color space (from sRGB)
    #[inline]
    pub fn to_linear(self) -> Self {
        Self::new(
            srgb_to_linear(self.r),
            srgb_to_linear(self.g),
            srgb_to_linear(self.b),
            self.a, // Alpha is already linear
        )
    }

    /// Returns the color in sRGB color space (from linear)
    #[inline]
    pub fn to_srgb(self) -> Self {
        Self::new(
            linear_to_srgb(self.r),
            linear_to_srgb(self.g),
            linear_to_srgb(self.b),
            self.a, // Alpha is already linear
        )
    }

    /// Premultiplies alpha into the color channels
    #[inline]
    pub fn premultiply(self) -> Self {
        Self::new(self.r * self.a, self.g * self.a, self.b * self.a, self.a)
    }

    /// Returns the luminance of the color
    #[inline]
    pub fn luminance(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }
}

impl From<Vec4> for Color {
    #[inline]
    fn from(v: Vec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }
}

impl From<Color> for Vec4 {
    #[inline]
    fn from(c: Color) -> Self {
        c.to_vec4()
    }
}

impl From<[f32; 4]> for Color {
    #[inline]
    fn from(arr: [f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

impl From<Color> for [f32; 4] {
    #[inline]
    fn from(c: Color) -> Self {
        c.to_array()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// COLOR SPACE CONVERSIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Converts a single sRGB component to linear
#[inline]
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts a single linear component to sRGB
#[inline]
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// CLEAR VALUES
// ═══════════════════════════════════════════════════════════════════════════

/// Value used for clearing render targets
#[derive(Clone, Copy, Debug)]
pub enum ClearValue {
    /// Clear color attachment
    Color(Color),
    /// Clear depth attachment
    Depth(f32),
    /// Clear stencil attachment
    Stencil(u32),
    /// Clear both depth and stencil
    DepthStencil(f32, u32),
}

impl Default for ClearValue {
    fn default() -> Self {
        Self::Color(Color::BLACK)
    }
}

impl From<Color> for ClearValue {
    fn from(c: Color) -> Self {
        Self::Color(c)
    }
}
