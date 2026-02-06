//! # Framebuffer Graphics Module
//!
//! This module provides advanced framebuffer graphics support including
//! text rendering, primitive drawing, and double buffering.
//!
//! ## Features
//!
//! - Text rendering with built-in font
//! - Shape drawing (lines, rectangles, circles)
//! - Double buffering for flicker-free updates
//! - Pixel format conversion
//! - Hardware cursor support

use core::ptr;

use crate::requests::{Color, Framebuffer, PixelFormat};

// =============================================================================
// Re-export Color for convenience
// =============================================================================

// Color is imported from requests::framebuffer and re-exported

// =============================================================================
// Framebuffer Console
// =============================================================================

/// Built-in 8x16 font glyph data (subset for ASCII 32-127)
/// Width of each font glyph in pixels.
const FONT_WIDTH: usize = 8;
/// Height of each font glyph in pixels.
const FONT_HEIGHT: usize = 16;

/// Simple 8x16 font data for ASCII characters
static FONT_DATA: &[u8] = include_bytes!("font8x16.bin");

/// Get font glyph for a character
fn get_glyph(c: char) -> Option<&'static [u8]> {
    let code = c as usize;
    if code >= 32 && code < 128 {
        let offset = (code - 32) * FONT_HEIGHT;
        if offset + FONT_HEIGHT <= FONT_DATA.len() {
            return Some(&FONT_DATA[offset..offset + FONT_HEIGHT]);
        }
    }
    None
}

/// Text console backed by a framebuffer
pub struct Console<'a, 'b> {
    /// Reference to the underlying framebuffer.
    fb: &'a Framebuffer<'b>,
    /// Current cursor column position (in characters).
    cursor_x: usize,
    /// Current cursor row position (in characters).
    cursor_y: usize,
    /// Foreground color for text rendering.
    foreground: Color,
    /// Background color for text rendering.
    background: Color,
    /// Number of spaces per tab character.
    tab_width: usize,
}

impl<'a, 'b> Console<'a, 'b> {
    /// Create a new console for the framebuffer
    pub fn new(fb: &'a Framebuffer<'b>) -> Self {
        Self {
            fb,
            cursor_x: 0,
            cursor_y: 0,
            foreground: Color::WHITE,
            background: Color::BLACK,
            tab_width: 4,
        }
    }

    /// Set foreground color
    pub fn set_foreground(&mut self, color: Color) {
        self.foreground = color;
    }

    /// Set background color
    pub fn set_background(&mut self, color: Color) {
        self.background = color;
    }

    /// Get columns count
    pub fn columns(&self) -> usize {
        self.fb.width() / FONT_WIDTH
    }

    /// Get rows count
    pub fn rows(&self) -> usize {
        self.fb.height() / FONT_HEIGHT
    }

    /// Move cursor to position
    pub fn move_to(&mut self, x: usize, y: usize) {
        self.cursor_x = x.min(self.columns() - 1);
        self.cursor_y = y.min(self.rows() - 1);
    }

    /// Clear the console
    pub fn clear(&mut self) {
        // Fill the framebuffer with background color
        let width = self.fb.width();
        let height = self.fb.height();
        for y in 0..height {
            for x in 0..width {
                self.fb.set_pixel(x, y, self.background);
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    /// Write a character
    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.cursor_x = 0;
                self.newline();
            },
            '\r' => {
                self.cursor_x = 0;
            },
            '\t' => {
                let spaces = self.tab_width - (self.cursor_x % self.tab_width);
                for _ in 0..spaces {
                    self.write_char(' ');
                }
            },
            '\x08' => {
                // Backspace
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
            },
            c => {
                if self.cursor_x >= self.columns() {
                    self.cursor_x = 0;
                    self.newline();
                }
                self.draw_char(c, self.cursor_x, self.cursor_y);
                self.cursor_x += 1;
            },
        }
    }

    /// Write a string
    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            self.write_char(c);
        }
    }

    /// Write a string with color
    pub fn write_colored(&mut self, s: &str, fg: Color) {
        let old_fg = self.foreground;
        self.foreground = fg;
        self.write_str(s);
        self.foreground = old_fg;
    }

    /// Scroll the console up by one line
    fn scroll_up(&mut self) {
        let line_height = FONT_HEIGHT;
        let bytes_per_pixel = self.fb.bpp() as usize / 8;
        let pitch = self.fb.pitch() as usize;
        let width = self.fb.width() as usize;
        let height = self.fb.height() as usize;

        // Copy lines up
        let line_bytes = width * bytes_per_pixel;
        let scroll_lines = height - line_height;

        unsafe {
            let base = self.fb.address() as *mut u8;
            for y in 0..scroll_lines {
                let src = base.add((y + line_height) * pitch);
                let dst = base.add(y * pitch);
                ptr::copy(src, dst, line_bytes);
            }

            // Clear bottom line
            for y in scroll_lines..height {
                let row = base.add(y * pitch);
                for x in 0..width {
                    let pixel = row.add(x * bytes_per_pixel);
                    self.write_pixel_raw(pixel, self.background);
                }
            }
        }
    }

    /// Handle newline
    fn newline(&mut self) {
        if self.cursor_y + 1 >= self.rows() {
            self.scroll_up();
        } else {
            self.cursor_y += 1;
        }
    }

    /// Draw a character at the given position
    fn draw_char(&self, c: char, col: usize, row: usize) {
        let glyph = match get_glyph(c) {
            Some(g) => g,
            None => return,
        };

        let base_x = col * FONT_WIDTH;
        let base_y = row * FONT_HEIGHT;

        for (dy, &glyph_row) in glyph.iter().enumerate() {
            for dx in 0..FONT_WIDTH {
                let pixel_on = (glyph_row >> (7 - dx)) & 1 != 0;
                let color = if pixel_on {
                    self.foreground
                } else {
                    self.background
                };
                self.fb.set_pixel(base_x + dx, base_y + dy, color);
            }
        }
    }

    /// Write pixel directly using the framebuffer's pixel format
    #[allow(dead_code)]
    unsafe fn write_pixel_raw(&self, ptr: *mut u8, color: Color) {
        let format = self.fb.pixel_format();
        let encoded = format.encode(color);
        let bpp = format.bpp as usize;

        match bpp {
            8 => {
                *ptr = encoded as u8;
            },
            16 => {
                core::ptr::write_unaligned(ptr as *mut u16, encoded as u16);
            },
            24 => {
                *ptr = encoded as u8;
                *ptr.add(1) = (encoded >> 8) as u8;
                *ptr.add(2) = (encoded >> 16) as u8;
            },
            32 => {
                core::ptr::write_unaligned(ptr as *mut u32, encoded);
            },
            _ => {},
        }
    }
}

impl<'a, 'b> core::fmt::Write for Console<'a, 'b> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        Console::write_str(self, s);
        Ok(())
    }
}

// =============================================================================
// Graphics Context
// =============================================================================

/// Point in 2D space
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    /// X coordinate (horizontal position).
    pub x: i32,
    /// Y coordinate (vertical position).
    pub y: i32,
}

impl Point {
    /// Creates a new point at the given coordinates.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Returns the origin point (0, 0).
    pub const fn origin() -> Self {
        Self { x: 0, y: 0 }
    }
}

/// Rectangle in 2D space
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    /// X coordinate of the top-left corner.
    pub x: i32,
    /// Y coordinate of the top-left corner.
    pub y: i32,
    /// Width of the rectangle in pixels.
    pub width: u32,
    /// Height of the rectangle in pixels.
    pub height: u32,
}

impl Rect {
    /// Creates a new rectangle with the given position and dimensions.
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Creates a rectangle from two opposite corner points.
    pub const fn from_points(p1: Point, p2: Point) -> Self {
        let x = if p1.x < p2.x { p1.x } else { p2.x };
        let y = if p1.y < p2.y { p1.y } else { p2.y };
        let width = (p1.x - p2.x).unsigned_abs();
        let height = (p1.y - p2.y).unsigned_abs();
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Returns the X coordinate of the right edge.
    pub const fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    /// Returns the Y coordinate of the bottom edge.
    pub const fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    /// Checks if a point is contained within this rectangle.
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x < self.right() && p.y >= self.y && p.y < self.bottom()
    }

    /// Checks if this rectangle intersects with another.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Returns the intersection of this rectangle with another, if any.
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        Some(Rect {
            x,
            y,
            width: (right - x) as u32,
            height: (bottom - y) as u32,
        })
    }
}

/// Graphics context for drawing primitives
pub struct Graphics<'a, 'b> {
    /// Reference to the underlying framebuffer.
    fb: &'a Framebuffer<'b>,
    /// Optional clipping rectangle to constrain drawing.
    clip: Option<Rect>,
}

impl<'a, 'b> Graphics<'a, 'b> {
    /// Create graphics context for framebuffer
    pub fn new(fb: &'a Framebuffer<'b>) -> Self {
        Self { fb, clip: None }
    }

    /// Set clipping rectangle
    pub fn set_clip(&mut self, rect: Rect) {
        self.clip = Some(rect);
    }

    /// Clear clipping rectangle
    pub fn clear_clip(&mut self) {
        self.clip = None;
    }

    /// Get the framebuffer bounds
    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.fb.width() as u32, self.fb.height() as u32)
    }

    /// Check if point is within bounds and clip region
    fn in_bounds(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 {
            return false;
        }
        if x >= self.fb.width() as i32 || y >= self.fb.height() as i32 {
            return false;
        }
        if let Some(clip) = &self.clip {
            if !clip.contains(Point::new(x, y)) {
                return false;
            }
        }
        true
    }

    /// Set a pixel
    pub fn set_pixel(&self, x: i32, y: i32, color: Color) {
        if self.in_bounds(x, y) {
            self.fb.set_pixel(x as usize, y as usize, color);
        }
    }

    /// Draw a horizontal line
    pub fn draw_hline(&self, x1: i32, x2: i32, y: i32, color: Color) {
        let (x1, x2) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
        for x in x1..=x2 {
            self.set_pixel(x, y, color);
        }
    }

    /// Draw a vertical line
    pub fn draw_vline(&self, x: i32, y1: i32, y2: i32, color: Color) {
        let (y1, y2) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
        for y in y1..=y2 {
            self.set_pixel(x, y, color);
        }
    }

    /// Draw a line using Bresenham's algorithm
    pub fn draw_line(&self, p1: Point, p2: Point, color: Color) {
        let dx = (p2.x - p1.x).abs();
        let dy = -(p2.y - p1.y).abs();
        let sx = if p1.x < p2.x { 1 } else { -1 };
        let sy = if p1.y < p2.y { 1 } else { -1 };
        let mut err = dx + dy;
        let mut x = p1.x;
        let mut y = p1.y;

        loop {
            self.set_pixel(x, y, color);
            if x == p2.x && y == p2.y {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Draw a rectangle outline
    pub fn draw_rect(&self, rect: Rect, color: Color) {
        let x1 = rect.x;
        let y1 = rect.y;
        let x2 = rect.right() - 1;
        let y2 = rect.bottom() - 1;

        self.draw_hline(x1, x2, y1, color);
        self.draw_hline(x1, x2, y2, color);
        self.draw_vline(x1, y1, y2, color);
        self.draw_vline(x2, y1, y2, color);
    }

    /// Fill a rectangle
    pub fn fill_rect(&self, rect: Rect, color: Color) {
        for y in rect.y..rect.bottom() {
            self.draw_hline(rect.x, rect.right() - 1, y, color);
        }
    }

    /// Draw a circle outline using midpoint algorithm
    pub fn draw_circle(&self, center: Point, radius: i32, color: Color) {
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.set_pixel(center.x + x, center.y + y, color);
            self.set_pixel(center.x + y, center.y + x, color);
            self.set_pixel(center.x - y, center.y + x, color);
            self.set_pixel(center.x - x, center.y + y, color);
            self.set_pixel(center.x - x, center.y - y, color);
            self.set_pixel(center.x - y, center.y - x, color);
            self.set_pixel(center.x + y, center.y - x, color);
            self.set_pixel(center.x + x, center.y - y, color);

            y += 1;
            err += 1 + 2 * y;
            if 2 * (err - x) + 1 > 0 {
                x -= 1;
                err += 1 - 2 * x;
            }
        }
    }

    /// Fill a circle
    pub fn fill_circle(&self, center: Point, radius: i32, color: Color) {
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.draw_hline(center.x - x, center.x + x, center.y + y, color);
            self.draw_hline(center.x - x, center.x + x, center.y - y, color);
            self.draw_hline(center.x - y, center.x + y, center.y + x, color);
            self.draw_hline(center.x - y, center.x + y, center.y - x, color);

            y += 1;
            err += 1 + 2 * y;
            if 2 * (err - x) + 1 > 0 {
                x -= 1;
                err += 1 - 2 * x;
            }
        }
    }

    /// Draw a triangle outline
    pub fn draw_triangle(&self, p1: Point, p2: Point, p3: Point, color: Color) {
        self.draw_line(p1, p2, color);
        self.draw_line(p2, p3, color);
        self.draw_line(p3, p1, color);
    }

    /// Fill the entire framebuffer
    pub fn clear(&self, color: Color) {
        let width = self.fb.width() as usize;
        let height = self.fb.height() as usize;
        for y in 0..height {
            for x in 0..width {
                self.fb.set_pixel(x, y, color);
            }
        }
    }

    /// Draw a gradient from top to bottom
    pub fn fill_gradient_v(&self, rect: Rect, top: Color, bottom: Color) {
        for y in 0..rect.height {
            let t = (y * 255 / rect.height.max(1)) as u8;
            let color = Self::lerp_color(top, bottom, t);
            self.draw_hline(rect.x, rect.right() - 1, rect.y + y as i32, color);
        }
    }

    /// Draw a gradient from left to right
    pub fn fill_gradient_h(&self, rect: Rect, left: Color, right: Color) {
        for x in 0..rect.width {
            let t = (x * 255 / rect.width.max(1)) as u8;
            let color = Self::lerp_color(left, right, t);
            self.draw_vline(rect.x + x as i32, rect.y, rect.bottom() - 1, color);
        }
    }

    /// Linear interpolation between two colors
    /// t is in range 0-255, where 0 = a, 255 = b
    fn lerp_color(a: Color, b: Color, t: u8) -> Color {
        let t = u16::from(t);
        let inv_t = 255 - t;
        Color::rgb(
            ((u16::from(a.r) * inv_t + u16::from(b.r) * t) / 255) as u8,
            ((u16::from(a.g) * inv_t + u16::from(b.g) * t) / 255) as u8,
            ((u16::from(a.b) * inv_t + u16::from(b.b) * t) / 255) as u8,
        )
    }
}

// =============================================================================
// Double Buffering
// =============================================================================

/// Double-buffered framebuffer for flicker-free rendering
pub struct DoubleBuffer {
    /// Pointer to the front (visible) buffer.
    front: *mut u8,
    /// Pointer to the back (drawing) buffer.
    back: *mut u8,
    /// Total size of the buffer in bytes.
    size: usize,
    /// Width of the framebuffer in pixels.
    width: usize,
    /// Height of the framebuffer in pixels.
    height: usize,
    /// Number of bytes per scanline.
    pitch: usize,
    /// Bits per pixel.
    bpp: usize,
    /// Pixel format used by the framebuffer.
    format: PixelFormat,
    /// Whether this buffer owns the back buffer memory.
    owns_back: bool,
}

impl DoubleBuffer {
    /// Create double buffer from framebuffer
    ///
    /// # Safety
    ///
    /// The back buffer must be a valid memory region of at least `pitch * height` bytes.
    pub unsafe fn new(fb: &Framebuffer, back_buffer: *mut u8) -> Self {
        let size = fb.pitch() as usize * fb.height() as usize;
        Self {
            front: fb.address() as *mut u8,
            back: back_buffer,
            size,
            width: fb.width() as usize,
            height: fb.height() as usize,
            pitch: fb.pitch() as usize,
            bpp: fb.bpp() as usize,
            format: fb.pixel_format(),
            owns_back: false,
        }
    }

    /// Get width
    pub fn width(&self) -> usize {
        self.width
    }

    /// Get height
    pub fn height(&self) -> usize {
        self.height
    }

    /// Get pitch
    pub fn pitch(&self) -> usize {
        self.pitch
    }

    /// Get bits per pixel
    pub fn bpp(&self) -> usize {
        self.bpp
    }

    /// Get back buffer address
    pub fn back_buffer(&self) -> *mut u8 {
        self.back
    }

    /// Set a pixel in the back buffer
    pub fn set_pixel(&self, x: usize, y: usize, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }

        let bytes_per_pixel = self.bpp / 8;
        let offset = y * self.pitch + x * bytes_per_pixel;
        let encoded = self.format.encode(color);

        // SAFETY: ptr is within the back buffer allocation, and we use
        // write_unaligned to handle potentially unaligned framebuffer addresses.
        unsafe {
            let ptr = self.back.add(offset);
            match bytes_per_pixel {
                1 => *ptr = encoded as u8,
                2 => core::ptr::write_unaligned(ptr as *mut u16, encoded as u16),
                3 => {
                    *ptr = encoded as u8;
                    *ptr.add(1) = (encoded >> 8) as u8;
                    *ptr.add(2) = (encoded >> 16) as u8;
                },
                4 => core::ptr::write_unaligned(ptr as *mut u32, encoded),
                _ => {},
            }
        }
    }

    /// Fill the back buffer with a color
    pub fn clear(&self, color: Color) {
        let bytes_per_pixel = self.bpp / 8;
        let encoded = self.format.encode(color);

        // SAFETY: ptr is within the back buffer allocation, and we use
        // write_unaligned to handle potentially unaligned framebuffer addresses.
        unsafe {
            for y in 0..self.height {
                for x in 0..self.width {
                    let offset = y * self.pitch + x * bytes_per_pixel;
                    let ptr = self.back.add(offset);
                    match bytes_per_pixel {
                        1 => *ptr = encoded as u8,
                        2 => core::ptr::write_unaligned(ptr as *mut u16, encoded as u16),
                        3 => {
                            *ptr = encoded as u8;
                            *ptr.add(1) = (encoded >> 8) as u8;
                            *ptr.add(2) = (encoded >> 16) as u8;
                        },
                        4 => core::ptr::write_unaligned(ptr as *mut u32, encoded),
                        _ => {},
                    }
                }
            }
        }
    }

    /// Swap buffers (copy back to front)
    pub fn swap(&self) {
        unsafe {
            ptr::copy_nonoverlapping(self.back, self.front, self.size);
        }
    }

    /// Swap only a dirty region
    pub fn swap_region(&self, rect: Rect) {
        let bytes_per_pixel = self.bpp / 8;
        let x = rect.x.max(0) as usize;
        let y = rect.y.max(0) as usize;
        let width = (rect.width as usize).min(self.width - x);
        let height = (rect.height as usize).min(self.height - y);
        let row_bytes = width * bytes_per_pixel;

        unsafe {
            for row in y..y + height {
                let offset = row * self.pitch + x * bytes_per_pixel;
                ptr::copy_nonoverlapping(self.back.add(offset), self.front.add(offset), row_bytes);
            }
        }
    }
}

impl Drop for DoubleBuffer {
    fn drop(&mut self) {
        // If we allocated the back buffer, we'd free it here
        // For now, we assume the caller manages the back buffer
        if self.owns_back {
            // Would free self.back here if we owned it
        }
    }
}

// =============================================================================
// Bitmap Support
// =============================================================================

/// Simple bitmap image (raw pixel data)
pub struct Bitmap {
    /// Width of the bitmap in pixels.
    width: u32,
    /// Height of the bitmap in pixels.
    height: u32,
    /// Pointer to raw RGBA pixel data.
    data: *const u32,
}

impl Bitmap {
    /// Create bitmap from raw RGBA pixel data
    ///
    /// # Safety
    ///
    /// Data must point to valid pixel data of size width * height * 4 bytes.
    pub const unsafe fn from_raw(width: u32, height: u32, data: *const u32) -> Self {
        Self {
            width,
            height,
            data,
        }
    }

    /// Get width
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Get height
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Get pixel at position
    pub fn pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }

        unsafe {
            let offset = y * self.width + x;
            let value = *self.data.add(offset as usize);
            Some(Color::from_u32(value))
        }
    }
}

/// Draws a bitmap to the graphics context at the specified position.
pub fn draw_bitmap(gfx: &Graphics, bitmap: &Bitmap, pos: Point) {
    for y in 0..bitmap.height() {
        for x in 0..bitmap.width() {
            if let Some(color) = bitmap.pixel(x, y) {
                if color.a > 0 {
                    gfx.set_pixel(pos.x + x as i32, pos.y + y as i32, color);
                }
            }
        }
    }
}

/// Draws a bitmap scaled to fit the destination rectangle.
pub fn draw_bitmap_scaled(gfx: &Graphics, bitmap: &Bitmap, dest: Rect) {
    let scale_x = bitmap.width() as f32 / dest.width as f32;
    let scale_y = bitmap.height() as f32 / dest.height as f32;

    for dy in 0..dest.height {
        for dx in 0..dest.width {
            let sx = (dx as f32 * scale_x) as u32;
            let sy = (dy as f32 * scale_y) as u32;
            if let Some(color) = bitmap.pixel(sx, sy) {
                if color.a > 0 {
                    gfx.set_pixel(dest.x + dx as i32, dest.y + dy as i32, color);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_blend() {
        let red = Color::rgba(255, 0, 0, 128);
        let blue = Color::rgb(0, 0, 255);
        let blended = red.blend_over(blue);
        assert!(blended.r > 0);
        assert!(blended.b > 0);
    }

    #[test]
    fn test_color_lerp() {
        let black = Color::BLACK;
        let white = Color::WHITE;
        let gray = black.lerp(white, 0.5);
        assert_eq!(gray.r, 127);
        assert_eq!(gray.g, 127);
        assert_eq!(gray.b, 127);
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10, 10, 100, 100);
        assert!(rect.contains(Point::new(50, 50)));
        assert!(!rect.contains(Point::new(5, 50)));
    }

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::new(0, 0, 100, 100);
        let r2 = Rect::new(50, 50, 100, 100);
        let intersection = r1.intersection(&r2).unwrap();
        assert_eq!(intersection.x, 50);
        assert_eq!(intersection.y, 50);
        assert_eq!(intersection.width, 50);
        assert_eq!(intersection.height, 50);
    }
}
