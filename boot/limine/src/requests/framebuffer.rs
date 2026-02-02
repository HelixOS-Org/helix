//! # Framebuffer Request
//!
//! This module provides framebuffer support for graphical output.
//! It includes comprehensive pixel format handling, drawing primitives,
//! and multi-monitor support.

use core::{ptr, slice};

use super::{LimineRequest, ResponsePtr, SafeResponse};
use crate::protocol::raw::{RawFramebuffer, RawVideoMode};
use crate::protocol::request_ids::FRAMEBUFFER_ID;

/// Framebuffer request
///
/// Requests one or more framebuffers from the bootloader for graphical output.
///
/// # Example
///
/// ```rust,no_run
/// use helix_limine::requests::FramebufferRequest;
///
/// #[used]
/// #[link_section = ".limine_requests"]
/// static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();
///
/// fn draw_pixel(x: usize, y: usize, color: u32) {
///     if let Some(fb) = FRAMEBUFFER.response().and_then(|r| r.primary()) {
///         if let Some(pixel) = fb.pixel_mut(x, y) {
///             *pixel = color;
///         }
///     }
/// }
/// ```
#[repr(C)]
pub struct FramebufferRequest {
    /// Request identifier
    id: [u64; 4],
    /// Protocol revision
    revision: u64,
    /// Response pointer
    response: ResponsePtr<FramebufferResponse>,
}

impl FramebufferRequest {
    /// Create a new framebuffer request
    pub const fn new() -> Self {
        Self {
            id: FRAMEBUFFER_ID,
            revision: 0,
            response: ResponsePtr::null(),
        }
    }

    /// Create with specific revision
    pub const fn with_revision(revision: u64) -> Self {
        Self {
            id: FRAMEBUFFER_ID,
            revision,
            response: ResponsePtr::null(),
        }
    }
}

impl Default for FramebufferRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl LimineRequest for FramebufferRequest {
    type Response = FramebufferResponse;

    fn id(&self) -> [u64; 4] {
        self.id
    }
    fn revision(&self) -> u64 {
        self.revision
    }
    fn has_response(&self) -> bool {
        self.response.is_available()
    }
    fn response(&self) -> Option<&Self::Response> {
        unsafe { self.response.get() }
    }
}

unsafe impl Sync for FramebufferRequest {}

/// Framebuffer response
#[repr(C)]
pub struct FramebufferResponse {
    /// Response revision
    revision: u64,
    /// Number of framebuffers
    framebuffer_count: u64,
    /// Framebuffer pointers
    framebuffers: *const *const RawFramebuffer,
}

impl FramebufferResponse {
    /// Get the response revision
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Get the number of framebuffers
    pub fn count(&self) -> usize {
        self.framebuffer_count as usize
    }

    /// Get the primary (first) framebuffer
    pub fn primary(&self) -> Option<Framebuffer<'_>> {
        self.get(0)
    }

    /// Get a specific framebuffer by index
    pub fn get(&self, index: usize) -> Option<Framebuffer<'_>> {
        if index >= self.count() || self.framebuffers.is_null() {
            return None;
        }

        unsafe {
            let fb_ptr = *self.framebuffers.add(index);
            if fb_ptr.is_null() {
                None
            } else {
                Some(Framebuffer::from_raw(&*fb_ptr))
            }
        }
    }

    /// Iterate over all framebuffers
    pub fn iter(&self) -> FramebufferIterator<'_> {
        FramebufferIterator {
            response: self,
            index: 0,
        }
    }

    /// Find the largest framebuffer (by pixel count)
    pub fn largest(&self) -> Option<Framebuffer<'_>> {
        self.iter().max_by_key(|fb| fb.width() * fb.height())
    }

    /// Find a framebuffer with at least the given dimensions
    pub fn find_min_size(&self, min_width: usize, min_height: usize) -> Option<Framebuffer<'_>> {
        self.iter()
            .find(|fb| fb.width() >= min_width && fb.height() >= min_height)
    }
}

unsafe impl SafeResponse for FramebufferResponse {
    fn validate(&self) -> bool {
        self.framebuffer_count > 0 && !self.framebuffers.is_null()
    }
}

impl core::fmt::Debug for FramebufferResponse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FramebufferResponse")
            .field("count", &self.count())
            .field(
                "primary",
                &self.primary().map(|fb| (fb.width(), fb.height())),
            )
            .finish()
    }
}

/// Iterator over framebuffers
pub struct FramebufferIterator<'a> {
    response: &'a FramebufferResponse,
    index: usize,
}

impl<'a> Iterator for FramebufferIterator<'a> {
    type Item = Framebuffer<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let fb = self.response.get(self.index)?;
        self.index += 1;
        Some(fb)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.response.count() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for FramebufferIterator<'a> {}

/// A single framebuffer
pub struct Framebuffer<'a> {
    raw: &'a RawFramebuffer,
}

impl<'a> Framebuffer<'a> {
    fn from_raw(raw: &'a RawFramebuffer) -> Self {
        Self { raw }
    }

    /// Get the framebuffer address
    pub fn address(&self) -> *mut u8 {
        self.raw.address
    }

    /// Get the width in pixels
    pub fn width(&self) -> usize {
        self.raw.width as usize
    }

    /// Get the height in pixels
    pub fn height(&self) -> usize {
        self.raw.height as usize
    }

    /// Get the pitch (bytes per row)
    pub fn pitch(&self) -> usize {
        self.raw.pitch as usize
    }

    /// Get the bits per pixel
    pub fn bpp(&self) -> u16 {
        self.raw.bpp
    }

    /// Get the bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        (self.raw.bpp as usize + 7) / 8
    }

    /// Get the total size in bytes
    pub fn size(&self) -> usize {
        self.pitch() * self.height()
    }

    /// Get the pixel format
    pub fn pixel_format(&self) -> PixelFormat {
        PixelFormat {
            bpp: self.raw.bpp,
            red_mask_size: self.raw.red_mask_size,
            red_mask_shift: self.raw.red_mask_shift,
            green_mask_size: self.raw.green_mask_size,
            green_mask_shift: self.raw.green_mask_shift,
            blue_mask_size: self.raw.blue_mask_size,
            blue_mask_shift: self.raw.blue_mask_shift,
        }
    }

    /// Check if this is an RGB framebuffer
    pub fn is_rgb(&self) -> bool {
        self.raw.memory_model == 1
    }

    /// Get the EDID data (if available)
    pub fn edid(&self) -> Option<&[u8]> {
        if self.raw.edid.is_null() || self.raw.edid_size == 0 {
            return None;
        }
        unsafe {
            Some(slice::from_raw_parts(
                self.raw.edid,
                self.raw.edid_size as usize,
            ))
        }
    }

    /// Get available video modes (revision 1+)
    pub fn video_modes(&self) -> VideoModeIterator<'_> {
        VideoModeIterator {
            framebuffer: self,
            index: 0,
        }
    }

    /// Get the number of video modes
    pub fn mode_count(&self) -> usize {
        self.raw.mode_count as usize
    }

    /// Get the framebuffer as a mutable byte slice
    ///
    /// # Safety
    ///
    /// This is safe because the bootloader guarantees the framebuffer
    /// memory is accessible and properly mapped.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.raw.address, self.size()) }
    }

    /// Get the framebuffer as a mutable byte slice
    pub fn as_slice_mut(&self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.raw.address, self.size()) }
    }

    /// Get a pixel as a mutable reference (32-bit)
    ///
    /// Returns None if coordinates are out of bounds.
    pub fn pixel_mut(&self, x: usize, y: usize) -> Option<&mut u32> {
        if x >= self.width() || y >= self.height() {
            return None;
        }

        let offset = y * self.pitch() + x * 4;
        unsafe { Some(&mut *(self.raw.address.add(offset) as *mut u32)) }
    }

    /// Set a pixel to a specific color
    pub fn set_pixel(&self, x: usize, y: usize, color: Color) {
        if let Some(pixel) = self.pixel_mut(x, y) {
            *pixel = self.pixel_format().encode(color);
        }
    }

    /// Fill the entire framebuffer with a color
    pub fn clear(&self, color: Color) {
        let encoded = self.pixel_format().encode(color);
        let buffer = self.as_slice_mut();

        // Fast path for 32bpp
        if self.bpp() == 32 {
            let pixels = unsafe {
                slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut u32, buffer.len() / 4)
            };
            for pixel in pixels {
                *pixel = encoded;
            }
        } else {
            // Generic path
            for y in 0..self.height() {
                for x in 0..self.width() {
                    self.set_pixel(x, y, color);
                }
            }
        }
    }

    /// Fill a rectangle with a color
    pub fn fill_rect(&self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        let encoded = self.pixel_format().encode(color);
        let max_x = (x + width).min(self.width());
        let max_y = (y + height).min(self.height());

        for py in y..max_y {
            for px in x..max_x {
                if let Some(pixel) = self.pixel_mut(px, py) {
                    *pixel = encoded;
                }
            }
        }
    }

    /// Draw a horizontal line
    pub fn hline(&self, x: usize, y: usize, length: usize, color: Color) {
        self.fill_rect(x, y, length, 1, color);
    }

    /// Draw a vertical line
    pub fn vline(&self, x: usize, y: usize, length: usize, color: Color) {
        self.fill_rect(x, y, 1, length, color);
    }

    /// Draw a rectangle outline
    pub fn draw_rect(&self, x: usize, y: usize, width: usize, height: usize, color: Color) {
        self.hline(x, y, width, color);
        self.hline(x, y + height - 1, width, color);
        self.vline(x, y, height, color);
        self.vline(x + width - 1, y, height, color);
    }

    /// Copy raw data to the framebuffer
    pub fn copy_from_slice(&self, x: usize, y: usize, data: &[u8], width: usize, height: usize) {
        let bytes_per_pixel = self.bytes_per_pixel();
        let src_pitch = width * bytes_per_pixel;

        for row in 0..height {
            if y + row >= self.height() {
                break;
            }

            let src_offset = row * src_pitch;
            let dst_offset = (y + row) * self.pitch() + x * bytes_per_pixel;
            let copy_len = src_pitch.min(self.pitch() - x * bytes_per_pixel);

            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr().add(src_offset),
                    self.raw.address.add(dst_offset),
                    copy_len,
                );
            }
        }
    }

    /// Scroll the framebuffer up by the given number of pixels
    pub fn scroll_up(&self, pixels: usize) {
        if pixels >= self.height() {
            self.clear(Color::BLACK);
            return;
        }

        let src_offset = pixels * self.pitch();
        let copy_len = (self.height() - pixels) * self.pitch();

        unsafe {
            ptr::copy(self.raw.address.add(src_offset), self.raw.address, copy_len);
        }

        // Clear the bottom portion
        let clear_offset = copy_len;
        let clear_len = pixels * self.pitch();
        unsafe {
            ptr::write_bytes(self.raw.address.add(clear_offset), 0, clear_len);
        }
    }
}

impl<'a> core::fmt::Debug for Framebuffer<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Framebuffer")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("bpp", &self.bpp())
            .field("pitch", &self.pitch())
            .field("pixel_format", &self.pixel_format())
            .finish()
    }
}

/// Video mode information
pub struct VideoMode<'a> {
    raw: &'a RawVideoMode,
}

impl<'a> VideoMode<'a> {
    fn from_raw(raw: &'a RawVideoMode) -> Self {
        Self { raw }
    }

    /// Get the mode width
    pub fn width(&self) -> usize {
        self.raw.width as usize
    }

    /// Get the mode height
    pub fn height(&self) -> usize {
        self.raw.height as usize
    }

    /// Get the mode pitch
    pub fn pitch(&self) -> usize {
        self.raw.pitch as usize
    }

    /// Get the bits per pixel
    pub fn bpp(&self) -> u16 {
        self.raw.bpp
    }

    /// Get the pixel format
    pub fn pixel_format(&self) -> PixelFormat {
        PixelFormat {
            bpp: self.raw.bpp,
            red_mask_size: self.raw.red_mask_size,
            red_mask_shift: self.raw.red_mask_shift,
            green_mask_size: self.raw.green_mask_size,
            green_mask_shift: self.raw.green_mask_shift,
            blue_mask_size: self.raw.blue_mask_size,
            blue_mask_shift: self.raw.blue_mask_shift,
        }
    }
}

impl<'a> core::fmt::Debug for VideoMode<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VideoMode")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("bpp", &self.bpp())
            .finish()
    }
}

/// Iterator over video modes
pub struct VideoModeIterator<'a> {
    framebuffer: &'a Framebuffer<'a>,
    index: usize,
}

impl<'a> Iterator for VideoModeIterator<'a> {
    type Item = VideoMode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.framebuffer.mode_count() {
            return None;
        }

        if self.framebuffer.raw.modes.is_null() {
            return None;
        }

        unsafe {
            let mode_ptr = *self.framebuffer.raw.modes.add(self.index);
            if mode_ptr.is_null() {
                None
            } else {
                self.index += 1;
                Some(VideoMode::from_raw(&*mode_ptr))
            }
        }
    }
}

/// Pixel format descriptor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelFormat {
    /// Bits per pixel
    pub bpp: u16,
    /// Red mask size
    pub red_mask_size: u8,
    /// Red mask shift
    pub red_mask_shift: u8,
    /// Green mask size
    pub green_mask_size: u8,
    /// Green mask shift
    pub green_mask_shift: u8,
    /// Blue mask size
    pub blue_mask_size: u8,
    /// Blue mask shift
    pub blue_mask_shift: u8,
}

impl PixelFormat {
    /// Common BGRA 32-bit format
    pub const BGRA32: Self = Self {
        bpp: 32,
        red_mask_size: 8,
        red_mask_shift: 16,
        green_mask_size: 8,
        green_mask_shift: 8,
        blue_mask_size: 8,
        blue_mask_shift: 0,
    };

    /// Common RGBA 32-bit format
    pub const RGBA32: Self = Self {
        bpp: 32,
        red_mask_size: 8,
        red_mask_shift: 0,
        green_mask_size: 8,
        green_mask_shift: 8,
        blue_mask_size: 8,
        blue_mask_shift: 16,
    };

    /// RGB 565 format (16-bit)
    pub const RGB565: Self = Self {
        bpp: 16,
        red_mask_size: 5,
        red_mask_shift: 11,
        green_mask_size: 6,
        green_mask_shift: 5,
        blue_mask_size: 5,
        blue_mask_shift: 0,
    };

    /// Encode a color using this format
    pub fn encode(&self, color: Color) -> u32 {
        let r = (color.r >> (8 - self.red_mask_size)) as u32;
        let g = (color.g >> (8 - self.green_mask_size)) as u32;
        let b = (color.b >> (8 - self.blue_mask_size)) as u32;

        (r << self.red_mask_shift) | (g << self.green_mask_shift) | (b << self.blue_mask_shift)
    }

    /// Decode a pixel value to a color
    pub fn decode(&self, pixel: u32) -> Color {
        let r_mask = ((1u32 << self.red_mask_size) - 1) << self.red_mask_shift;
        let g_mask = ((1u32 << self.green_mask_size) - 1) << self.green_mask_shift;
        let b_mask = ((1u32 << self.blue_mask_size) - 1) << self.blue_mask_shift;

        let r = ((pixel & r_mask) >> self.red_mask_shift) << (8 - self.red_mask_size);
        let g = ((pixel & g_mask) >> self.green_mask_shift) << (8 - self.green_mask_size);
        let b = ((pixel & b_mask) >> self.blue_mask_shift) << (8 - self.blue_mask_size);

        Color::rgb(r as u8, g as u8, b as u8)
    }

    /// Check if this is a common BGRA format
    pub fn is_bgra(&self) -> bool {
        self.bpp == 32
            && self.red_mask_shift == 16
            && self.green_mask_shift == 8
            && self.blue_mask_shift == 0
    }

    /// Check if this is a common RGBA format
    pub fn is_rgba(&self) -> bool {
        self.bpp == 32
            && self.red_mask_shift == 0
            && self.green_mask_shift == 8
            && self.blue_mask_shift == 16
    }
}

/// RGB color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255)
    pub a: u8,
}

impl Color {
    /// Black color
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White color
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Red color
    pub const RED: Self = Self::rgb(255, 0, 0);
    /// Green color
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    /// Blue color
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    /// Yellow color
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    /// Cyan color
    pub const CYAN: Self = Self::rgb(0, 255, 255);
    /// Magenta color
    pub const MAGENTA: Self = Self::rgb(255, 0, 255);
    /// Gray color
    pub const GRAY: Self = Self::rgb(128, 128, 128);
    /// Dark gray color
    pub const DARK_GRAY: Self = Self::rgb(64, 64, 64);
    /// Light gray color
    pub const LIGHT_GRAY: Self = Self::rgb(192, 192, 192);

    /// Create a color from RGB values
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create a color from RGBA values
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a color from a 24-bit RGB value
    pub const fn from_rgb24(rgb: u32) -> Self {
        Self::rgb(
            ((rgb >> 16) & 0xFF) as u8,
            ((rgb >> 8) & 0xFF) as u8,
            (rgb & 0xFF) as u8,
        )
    }

    /// Create a color from a 32-bit RGBA value
    pub const fn from_rgba32(rgba: u32) -> Self {
        Self::rgba(
            ((rgba >> 24) & 0xFF) as u8,
            ((rgba >> 16) & 0xFF) as u8,
            ((rgba >> 8) & 0xFF) as u8,
            (rgba & 0xFF) as u8,
        )
    }

    /// Create a color from a 32-bit value (ARGB format)
    pub const fn from_u32(value: u32) -> Self {
        Self::rgba(
            ((value >> 16) & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
            ((value >> 24) & 0xFF) as u8,
        )
    }

    /// Convert to 24-bit RGB
    pub const fn to_rgb24(&self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Convert to 32-bit RGBA
    pub const fn to_rgba32(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    /// Blend two colors using alpha
    pub fn blend(&self, other: Color, alpha: u8) -> Self {
        let a = alpha as u16;
        let inv_a = 255 - a;

        Self {
            r: ((self.r as u16 * inv_a + other.r as u16 * a) / 255) as u8,
            g: ((self.g as u16 * inv_a + other.g as u16 * a) / 255) as u8,
            b: ((self.b as u16 * inv_a + other.b as u16 * a) / 255) as u8,
            a: 255,
        }
    }

    /// Darken the color by a factor (0-255)
    pub fn darken(&self, factor: u8) -> Self {
        let f = factor as u16;
        Self {
            r: ((self.r as u16 * f) / 255) as u8,
            g: ((self.g as u16 * f) / 255) as u8,
            b: ((self.b as u16 * f) / 255) as u8,
            a: self.a,
        }
    }

    /// Lighten the color by a factor (0-255)
    pub fn lighten(&self, factor: u8) -> Self {
        let f = factor as u16;
        Self {
            r: (self.r as u16 + ((255 - self.r as u16) * f) / 255) as u8,
            g: (self.g as u16 + ((255 - self.g as u16) * f) / 255) as u8,
            b: (self.b as u16 + ((255 - self.b as u16) * f) / 255) as u8,
            a: self.a,
        }
    }

    /// Linear interpolation between two colors
    /// t is in range 0.0-1.0, where 0.0 = self, 1.0 = other
    pub fn lerp(&self, other: Color, t: f32) -> Self {
        let t_fixed = (t.clamp(0.0, 1.0) * 255.0) as u16;
        let inv_t = 255 - t_fixed;

        Self {
            r: ((self.r as u16 * inv_t + other.r as u16 * t_fixed) / 255) as u8,
            g: ((self.g as u16 * inv_t + other.g as u16 * t_fixed) / 255) as u8,
            b: ((self.b as u16 * inv_t + other.b as u16 * t_fixed) / 255) as u8,
            a: ((self.a as u16 * inv_t + other.a as u16 * t_fixed) / 255) as u8,
        }
    }

    /// Blend this color over another using alpha compositing (Porter-Duff over)
    pub fn blend_over(&self, other: Color) -> Self {
        let src_a = self.a as u16;
        let dst_a = other.a as u16;
        let out_a = src_a + (dst_a * (255 - src_a)) / 255;

        if out_a == 0 {
            return Color::rgba(0, 0, 0, 0);
        }

        let r =
            ((self.r as u16 * src_a + other.r as u16 * dst_a * (255 - src_a) / 255) / out_a) as u8;
        let g =
            ((self.g as u16 * src_a + other.g as u16 * dst_a * (255 - src_a) / 255) / out_a) as u8;
        let b =
            ((self.b as u16 * src_a + other.b as u16 * dst_a * (255 - src_a) / 255) / out_a) as u8;

        Self {
            r,
            g,
            b,
            a: out_a as u8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let color = Color::rgb(255, 128, 64);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 255);
    }

    #[test]
    fn test_pixel_format_encode() {
        let format = PixelFormat::BGRA32;
        let color = Color::rgb(255, 128, 64);
        let encoded = format.encode(color);

        // BGRA: Blue at 0, Green at 8, Red at 16
        assert_eq!(encoded, 0x00FF8040);
    }

    #[test]
    fn test_color_blend() {
        let black = Color::BLACK;
        let white = Color::WHITE;
        let gray = black.blend(white, 128);

        assert!(gray.r > 120 && gray.r < 136);
        assert!(gray.g > 120 && gray.g < 136);
        assert!(gray.b > 120 && gray.b < 136);
    }
}
