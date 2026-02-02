//! Color types and utilities
//!
//! This module provides color types and conversion utilities
//! for use in graphics applications.

use crate::vec::{Vec3, Vec4};

/// An RGBA color with f32 components
#[derive(Clone, Copy, Debug, PartialEq)]
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
    // Common colors
    /// Transparent black
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    /// Solid black
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    /// Solid white
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    /// Solid red
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    /// Solid green
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    /// Solid blue
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    /// Solid yellow
    pub const YELLOW: Self = Self::new(1.0, 1.0, 0.0, 1.0);
    /// Solid cyan
    pub const CYAN: Self = Self::new(0.0, 1.0, 1.0, 1.0);
    /// Solid magenta
    pub const MAGENTA: Self = Self::new(1.0, 0.0, 1.0, 1.0);
    /// Gray (50%)
    pub const GRAY: Self = Self::new(0.5, 0.5, 0.5, 1.0);
    /// Orange
    pub const ORANGE: Self = Self::new(1.0, 0.647, 0.0, 1.0);
    /// Purple
    pub const PURPLE: Self = Self::new(0.5, 0.0, 0.5, 1.0);
    /// Pink
    pub const PINK: Self = Self::new(1.0, 0.753, 0.796, 1.0);
    /// Brown
    pub const BROWN: Self = Self::new(0.647, 0.165, 0.165, 1.0);
    /// Cornflower blue (classic DirectX clear color)
    pub const CORNFLOWER_BLUE: Self = Self::new(0.392, 0.584, 0.929, 1.0);

    /// Creates a new color
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates a color from RGB with alpha = 1.0
    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    /// Creates a grayscale color
    #[inline]
    pub const fn gray(v: f32) -> Self {
        Self::new(v, v, v, 1.0)
    }

    /// Creates a color from HSV values
    /// - h: Hue (0.0 - 1.0)
    /// - s: Saturation (0.0 - 1.0)
    /// - v: Value (0.0 - 1.0)
    #[inline]
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        if s == 0.0 {
            return Self::rgb(v, v, v);
        }

        let h = h * 6.0;
        let i = h.floor();
        let f = h - i;
        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));

        let (r, g, b) = match i as i32 % 6 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };

        Self::rgb(r, g, b)
    }

    /// Converts to HSV
    /// Returns (hue, saturation, value)
    #[inline]
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let delta = max - min;

        let v = max;
        let s = if max == 0.0 { 0.0 } else { delta / max };

        let h = if delta == 0.0 {
            0.0
        } else if max == self.r {
            ((self.g - self.b) / delta).rem_euclid(6.0) / 6.0
        } else if max == self.g {
            ((self.b - self.r) / delta + 2.0) / 6.0
        } else {
            ((self.r - self.g) / delta + 4.0) / 6.0
        };

        (h, s, v)
    }

    /// Creates a color from HSL values
    /// - h: Hue (0.0 - 1.0)
    /// - s: Saturation (0.0 - 1.0)
    /// - l: Lightness (0.0 - 1.0)
    #[inline]
    pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
        if s == 0.0 {
            return Self::rgb(l, l, l);
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

        Self::rgb(r, g, b)
    }

    /// Converts to HSL
    /// Returns (hue, saturation, lightness)
    #[inline]
    pub fn to_hsl(self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let l = (max + min) / 2.0;

        if max == min {
            return (0.0, 0.0, l);
        }

        let delta = max - min;
        let s = if l > 0.5 {
            delta / (2.0 - max - min)
        } else {
            delta / (max + min)
        };

        let h = if max == self.r {
            ((self.g - self.b) / delta).rem_euclid(6.0) / 6.0
        } else if max == self.g {
            ((self.b - self.r) / delta + 2.0) / 6.0
        } else {
            ((self.r - self.g) / delta + 4.0) / 6.0
        };

        (h, s, l)
    }

    /// Creates a color from a hex value (0xRRGGBB)
    #[inline]
    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self::rgb(r, g, b)
    }

    /// Creates a color from a hex value with alpha (0xRRGGBBAA)
    #[inline]
    pub fn from_hex_rgba(hex: u32) -> Self {
        let r = ((hex >> 24) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let b = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let a = (hex & 0xFF) as f32 / 255.0;
        Self::new(r, g, b, a)
    }

    /// Converts to hex value (0xRRGGBB)
    #[inline]
    pub fn to_hex(self) -> u32 {
        let r = (self.r.clamp(0.0, 1.0) * 255.0) as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0) as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0) as u32;
        (r << 16) | (g << 8) | b
    }

    /// Converts to hex value with alpha (0xRRGGBBAA)
    #[inline]
    pub fn to_hex_rgba(self) -> u32 {
        let r = (self.r.clamp(0.0, 1.0) * 255.0) as u32;
        let g = (self.g.clamp(0.0, 1.0) * 255.0) as u32;
        let b = (self.b.clamp(0.0, 1.0) * 255.0) as u32;
        let a = (self.a.clamp(0.0, 1.0) * 255.0) as u32;
        (r << 24) | (g << 16) | (b << 8) | a
    }

    /// Creates a color from u8 components (0-255)
    #[inline]
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    /// Converts to u8 components
    #[inline]
    pub fn to_u8(self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }

    /// Converts sRGB to linear color space
    #[inline]
    pub fn to_linear(self) -> Self {
        Self::new(
            srgb_to_linear(self.r),
            srgb_to_linear(self.g),
            srgb_to_linear(self.b),
            self.a,
        )
    }

    /// Converts linear to sRGB color space
    #[inline]
    pub fn to_srgb(self) -> Self {
        Self::new(
            linear_to_srgb(self.r),
            linear_to_srgb(self.g),
            linear_to_srgb(self.b),
            self.a,
        )
    }

    /// Returns the luminance of the color (perceived brightness)
    #[inline]
    pub fn luminance(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// Returns true if the color is light (luminance > 0.5)
    #[inline]
    pub fn is_light(self) -> bool {
        self.luminance() > 0.5
    }

    /// Returns a contrasting color (black or white)
    #[inline]
    pub fn contrasting(self) -> Self {
        if self.is_light() {
            Self::BLACK
        } else {
            Self::WHITE
        }
    }

    /// Blends this color with another using alpha blending
    #[inline]
    pub fn blend(self, other: Self) -> Self {
        let a = self.a + other.a * (1.0 - self.a);
        if a == 0.0 {
            return Self::TRANSPARENT;
        }

        let inv_a = 1.0 / a;
        Self::new(
            (self.r * self.a + other.r * other.a * (1.0 - self.a)) * inv_a,
            (self.g * self.a + other.g * other.a * (1.0 - self.a)) * inv_a,
            (self.b * self.a + other.b * other.a * (1.0 - self.a)) * inv_a,
            a,
        )
    }

    /// Interpolates between two colors
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }

    /// Multiplies the RGB components by a factor
    #[inline]
    pub fn darken(self, factor: f32) -> Self {
        Self::new(self.r * factor, self.g * factor, self.b * factor, self.a)
    }

    /// Adds to the RGB components (clamped)
    #[inline]
    pub fn lighten(self, amount: f32) -> Self {
        Self::new(
            (self.r + amount).clamp(0.0, 1.0),
            (self.g + amount).clamp(0.0, 1.0),
            (self.b + amount).clamp(0.0, 1.0),
            self.a,
        )
    }

    /// Returns the color with a new alpha value
    #[inline]
    pub fn with_alpha(self, a: f32) -> Self {
        Self::new(self.r, self.g, self.b, a)
    }

    /// Returns the color with alpha multiplied by a factor
    #[inline]
    pub fn fade(self, factor: f32) -> Self {
        Self::new(self.r, self.g, self.b, self.a * factor)
    }

    /// Inverts the RGB components
    #[inline]
    pub fn invert(self) -> Self {
        Self::new(1.0 - self.r, 1.0 - self.g, 1.0 - self.b, self.a)
    }

    /// Clamps all components to [0, 1]
    #[inline]
    pub fn clamp(self) -> Self {
        Self::new(
            self.r.clamp(0.0, 1.0),
            self.g.clamp(0.0, 1.0),
            self.b.clamp(0.0, 1.0),
            self.a.clamp(0.0, 1.0),
        )
    }

    /// Returns the RGB components as Vec3
    #[inline]
    pub fn rgb_vec3(self) -> Vec3 {
        Vec3::new(self.r, self.g, self.b)
    }

    /// Returns the color as Vec4
    #[inline]
    pub fn to_vec4(self) -> Vec4 {
        Vec4::new(self.r, self.g, self.b, self.a)
    }

    /// Creates a color from Vec4
    #[inline]
    pub fn from_vec4(v: Vec4) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }

    /// Creates a color from Vec3 (alpha = 1.0)
    #[inline]
    pub fn from_vec3(v: Vec3) -> Self {
        Self::rgb(v.x, v.y, v.z)
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl From<[f32; 4]> for Color {
    fn from(arr: [f32; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}

impl From<[f32; 3]> for Color {
    fn from(arr: [f32; 3]) -> Self {
        Self::rgb(arr[0], arr[1], arr[2])
    }
}

impl From<Vec4> for Color {
    fn from(v: Vec4) -> Self {
        Self::from_vec4(v)
    }
}

impl From<Color> for Vec4 {
    fn from(c: Color) -> Self {
        c.to_vec4()
    }
}

impl core::ops::Mul<f32> for Color {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self {
        Self::new(self.r * rhs, self.g * rhs, self.b * rhs, self.a)
    }
}

impl core::ops::Add for Color {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.r + rhs.r,
            self.g + rhs.g,
            self.b + rhs.b,
            self.a + rhs.a,
        )
    }
}

/// Helper for HSL conversion
#[inline]
fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

/// Converts sRGB to linear
#[inline]
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts linear to sRGB
#[inline]
fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// A color in linear space (useful for HDR)
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct LinearColor {
    /// Red component
    pub r: f32,
    /// Green component
    pub g: f32,
    /// Blue component
    pub b: f32,
    /// Alpha component
    pub a: f32,
}

impl LinearColor {
    /// Black
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    /// White
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);

    /// Creates a new linear color
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Creates from sRGB color
    #[inline]
    pub fn from_srgb(c: Color) -> Self {
        Self::new(
            srgb_to_linear(c.r),
            srgb_to_linear(c.g),
            srgb_to_linear(c.b),
            c.a,
        )
    }

    /// Converts to sRGB color
    #[inline]
    pub fn to_srgb(self) -> Color {
        Color::new(
            linear_to_srgb(self.r),
            linear_to_srgb(self.g),
            linear_to_srgb(self.b),
            self.a,
        )
    }

    /// Returns the luminance
    #[inline]
    pub fn luminance(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// Converts to Vec4
    #[inline]
    pub fn to_vec4(self) -> Vec4 {
        Vec4::new(self.r, self.g, self.b, self.a)
    }
}

impl From<Color> for LinearColor {
    fn from(c: Color) -> Self {
        Self::from_srgb(c)
    }
}

impl From<LinearColor> for Color {
    fn from(c: LinearColor) -> Self {
        c.to_srgb()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_conversion() {
        let c = Color::from_hex(0xFF8000);
        assert!((c.r - 1.0).abs() < 1e-5);
        assert!((c.g - 0.5).abs() < 0.01);
        assert!((c.b - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_hsv_roundtrip() {
        let c = Color::RED;
        let (h, s, v) = c.to_hsv();
        let c2 = Color::from_hsv(h, s, v);
        assert!((c.r - c2.r).abs() < 1e-5);
        assert!((c.g - c2.g).abs() < 1e-5);
        assert!((c.b - c2.b).abs() < 1e-5);
    }

    #[test]
    fn test_linear_srgb_roundtrip() {
        let c = Color::new(0.5, 0.3, 0.8, 1.0);
        let linear = c.to_linear();
        let back = linear.to_srgb();
        assert!((c.r - back.r).abs() < 1e-5);
        assert!((c.g - back.g).abs() < 1e-5);
        assert!((c.b - back.b).abs() < 1e-5);
    }

    #[test]
    fn test_lerp() {
        let a = Color::BLACK;
        let b = Color::WHITE;
        let mid = a.lerp(b, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-5);
        assert!((mid.g - 0.5).abs() < 1e-5);
        assert!((mid.b - 0.5).abs() < 1e-5);
    }
}
