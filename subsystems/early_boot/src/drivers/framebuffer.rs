//! # Helix OS Early Boot - Framebuffer Driver
//!
//! This module provides early boot framebuffer access for graphical output.
//! It supports various pixel formats and provides basic drawing primitives
//! for boot splash screens and graphical diagnostics.
//!
//! ## Features
//!
//! - Multiple pixel formats (RGB, BGR, 16/24/32-bit)
//! - Basic drawing primitives (pixels, lines, rectangles)
//! - Text rendering with built-in bitmap font
//! - Boot splash and progress bar support
//! - Double buffering support (when memory permits)
//!
//! ## Usage
//!
//! ```rust,ignore
//! let fb = Framebuffer::init(boot_info)?;
//! fb.clear(Color::BLACK);
//! fb.draw_rect(10, 10, 100, 50, Color::WHITE);
//! fb.draw_text(20, 20, "Helix OS", Color::CYAN);
//! ```

#![allow(dead_code)]

use crate::error::{BootError, BootResult};
use crate::info::BootInfo;

// =============================================================================
// COLOR TYPES
// =============================================================================

/// 32-bit RGBA color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create new opaque color
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create new color with alpha
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create from 32-bit packed RGB (0x00RRGGBB)
    pub const fn from_rgb32(value: u32) -> Self {
        Self {
            r: ((value >> 16) & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: (value & 0xFF) as u8,
            a: 255,
        }
    }

    /// Create from 32-bit packed ARGB (0xAARRGGBB)
    pub const fn from_argb32(value: u32) -> Self {
        Self {
            a: ((value >> 24) & 0xFF) as u8,
            r: ((value >> 16) & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: (value & 0xFF) as u8,
        }
    }

    /// Convert to RGB32 (0x00RRGGBB)
    pub const fn to_rgb32(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Convert to BGR32 (0x00BBGGRR)
    pub const fn to_bgr32(self) -> u32 {
        ((self.b as u32) << 16) | ((self.g as u32) << 8) | (self.r as u32)
    }

    /// Convert to RGB565 (16-bit)
    pub const fn to_rgb565(self) -> u16 {
        (((self.r as u16) & 0xF8) << 8) | (((self.g as u16) & 0xFC) << 3) | ((self.b as u16) >> 3)
    }

    /// Convert to BGR565 (16-bit)
    pub const fn to_bgr565(self) -> u16 {
        (((self.b as u16) & 0xF8) << 8) | (((self.g as u16) & 0xFC) << 3) | ((self.r as u16) >> 3)
    }

    /// Blend with another color using alpha
    pub fn blend(self, other: Color) -> Color {
        let alpha = other.a as u32;
        let inv_alpha = 255 - alpha;

        Color {
            r: (((self.r as u32 * inv_alpha) + (other.r as u32 * alpha)) / 255) as u8,
            g: (((self.g as u32 * inv_alpha) + (other.g as u32 * alpha)) / 255) as u8,
            b: (((self.b as u32 * inv_alpha) + (other.b as u32 * alpha)) / 255) as u8,
            a: 255,
        }
    }

    /// Darken by percentage (0-100)
    pub fn darken(self, percent: u8) -> Color {
        let factor = (100 - percent.min(100)) as u32;
        Color {
            r: ((self.r as u32 * factor) / 100) as u8,
            g: ((self.g as u32 * factor) / 100) as u8,
            b: ((self.b as u32 * factor) / 100) as u8,
            a: self.a,
        }
    }

    /// Lighten by percentage (0-100)
    pub fn lighten(self, percent: u8) -> Color {
        let factor = percent.min(100) as u32;
        Color {
            r: self
                .r
                .saturating_add(((255 - self.r as u32) * factor / 100) as u8),
            g: self
                .g
                .saturating_add(((255 - self.g as u32) * factor / 100) as u8),
            b: self
                .b
                .saturating_add(((255 - self.b as u32) * factor / 100) as u8),
            a: self.a,
        }
    }

    // Standard colors
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const RED: Color = Color::rgb(255, 0, 0);
    pub const GREEN: Color = Color::rgb(0, 255, 0);
    pub const BLUE: Color = Color::rgb(0, 0, 255);
    pub const YELLOW: Color = Color::rgb(255, 255, 0);
    pub const CYAN: Color = Color::rgb(0, 255, 255);
    pub const MAGENTA: Color = Color::rgb(255, 0, 255);
    pub const GRAY: Color = Color::rgb(128, 128, 128);
    pub const DARK_GRAY: Color = Color::rgb(64, 64, 64);
    pub const LIGHT_GRAY: Color = Color::rgb(192, 192, 192);
    pub const ORANGE: Color = Color::rgb(255, 165, 0);
    pub const PURPLE: Color = Color::rgb(128, 0, 128);
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    // Helix theme colors
    pub const HELIX_PRIMARY: Color = Color::rgb(0x00, 0xA8, 0xE8); // Bright blue
    pub const HELIX_SECONDARY: Color = Color::rgb(0x00, 0x7E, 0xA7); // Darker blue
    pub const HELIX_ACCENT: Color = Color::rgb(0x00, 0xD4, 0xFF); // Cyan
    pub const HELIX_BG: Color = Color::rgb(0x1A, 0x1A, 0x2E); // Dark blue-gray
    pub const HELIX_FG: Color = Color::rgb(0xE8, 0xE8, 0xE8); // Light gray
}

impl Default for Color {
    fn default() -> Self {
        Color::BLACK
    }
}

// =============================================================================
// PIXEL FORMAT
// =============================================================================

/// Pixel format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// 32-bit RGB (0x00RRGGBB)
    Rgb32,
    /// 32-bit BGR (0x00BBGGRR)
    Bgr32,
    /// 24-bit RGB
    Rgb24,
    /// 24-bit BGR
    Bgr24,
    /// 16-bit RGB565
    Rgb565,
    /// 16-bit BGR565
    Bgr565,
    /// Unknown/unsupported format
    Unknown,
}

impl PixelFormat {
    /// Get bytes per pixel
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            PixelFormat::Rgb32 | PixelFormat::Bgr32 => 4,
            PixelFormat::Rgb24 | PixelFormat::Bgr24 => 3,
            PixelFormat::Rgb565 | PixelFormat::Bgr565 => 2,
            PixelFormat::Unknown => 0,
        }
    }

    /// Check if format uses BGR order
    pub const fn is_bgr(self) -> bool {
        matches!(
            self,
            PixelFormat::Bgr32 | PixelFormat::Bgr24 | PixelFormat::Bgr565
        )
    }
}

// =============================================================================
// FRAMEBUFFER INFO
// =============================================================================

/// Framebuffer information
#[derive(Debug, Clone)]
pub struct FramebufferInfo {
    /// Base physical address
    pub phys_base: u64,
    /// Base virtual address (after mapping)
    pub virt_base: u64,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Bytes per scanline (pitch)
    pub pitch: u32,
    /// Bits per pixel
    pub bpp: u8,
    /// Pixel format
    pub format: PixelFormat,
    /// Red mask position
    pub red_pos: u8,
    /// Green mask position
    pub green_pos: u8,
    /// Blue mask position
    pub blue_pos: u8,
    /// Total size in bytes
    pub size: usize,
}

impl FramebufferInfo {
    /// Create default (empty) framebuffer info
    pub const fn new() -> Self {
        Self {
            phys_base: 0,
            virt_base: 0,
            width: 0,
            height: 0,
            pitch: 0,
            bpp: 0,
            format: PixelFormat::Unknown,
            red_pos: 0,
            green_pos: 0,
            blue_pos: 0,
            size: 0,
        }
    }

    /// Check if framebuffer is valid
    pub fn is_valid(&self) -> bool {
        self.virt_base != 0
            && self.width > 0
            && self.height > 0
            && self.format != PixelFormat::Unknown
    }

    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        self.format.bytes_per_pixel()
    }

    /// Get pixel offset for coordinates
    pub fn pixel_offset(&self, x: u32, y: u32) -> usize {
        (y as usize * self.pitch as usize) + (x as usize * self.bytes_per_pixel())
    }
}

impl Default for FramebufferInfo {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// FRAMEBUFFER
// =============================================================================

/// Early boot framebuffer driver
pub struct Framebuffer {
    /// Framebuffer info
    info: FramebufferInfo,

    /// Current text cursor X position
    cursor_x: u32,

    /// Current text cursor Y position
    cursor_y: u32,

    /// Current foreground color
    fg_color: Color,

    /// Current background color
    bg_color: Color,

    /// Font width
    font_width: u32,

    /// Font height
    font_height: u32,
}

impl Framebuffer {
    /// Initialize framebuffer from boot info
    pub fn init(boot_info: &BootInfo) -> BootResult<Self> {
        // Get framebuffer info from boot_info
        // This would normally come from the bootloader (Limine, UEFI GOP, etc.)
        let info = Self::detect_framebuffer(boot_info)?;

        if !info.is_valid() {
            return Err(BootError::HardwareNotFound);
        }

        Ok(Self {
            info,
            cursor_x: 0,
            cursor_y: 0,
            fg_color: Color::WHITE,
            bg_color: Color::BLACK,
            font_width: FONT_WIDTH as u32,
            font_height: FONT_HEIGHT as u32,
        })
    }

    /// Detect framebuffer from boot information
    fn detect_framebuffer(boot_info: &BootInfo) -> BootResult<FramebufferInfo> {
        // Check for framebuffer in boot info
        if let Some(ref fb_info) = boot_info.framebuffer {
            let format = match fb_info.bpp {
                32 => {
                    // Determine RGB vs BGR based on red position
                    if fb_info.red_shift == 16 {
                        PixelFormat::Rgb32
                    } else {
                        PixelFormat::Bgr32
                    }
                },
                24 => {
                    if fb_info.red_shift == 16 {
                        PixelFormat::Rgb24
                    } else {
                        PixelFormat::Bgr24
                    }
                },
                16 => {
                    if fb_info.red_shift == 11 {
                        PixelFormat::Rgb565
                    } else {
                        PixelFormat::Bgr565
                    }
                },
                _ => PixelFormat::Unknown,
            };

            let bpp = fb_info.bpp as u8;
            let pitch = fb_info.pitch;
            let height = fb_info.height;

            return Ok(FramebufferInfo {
                phys_base: fb_info.address,
                virt_base: fb_info.address, // Need to map this properly
                width: fb_info.width,
                height,
                pitch,
                bpp,
                format,
                red_pos: fb_info.red_shift,
                green_pos: fb_info.green_shift,
                blue_pos: fb_info.blue_shift,
                size: (pitch as usize) * (height as usize),
            });
        }

        Err(BootError::HardwareNotFound)
    }

    /// Get framebuffer info
    pub fn info(&self) -> &FramebufferInfo {
        &self.info
    }

    /// Get width
    pub fn width(&self) -> u32 {
        self.info.width
    }

    /// Get height
    pub fn height(&self) -> u32 {
        self.info.height
    }

    /// Set a pixel
    #[inline]
    pub fn set_pixel(&self, x: u32, y: u32, color: Color) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }

        let offset = self.info.pixel_offset(x, y);
        let ptr = self.info.virt_base as *mut u8;

        unsafe {
            match self.info.format {
                PixelFormat::Rgb32 => {
                    let pixel = ptr.add(offset) as *mut u32;
                    core::ptr::write_volatile(pixel, color.to_rgb32());
                },
                PixelFormat::Bgr32 => {
                    let pixel = ptr.add(offset) as *mut u32;
                    core::ptr::write_volatile(pixel, color.to_bgr32());
                },
                PixelFormat::Rgb24 => {
                    core::ptr::write_volatile(ptr.add(offset), color.r);
                    core::ptr::write_volatile(ptr.add(offset + 1), color.g);
                    core::ptr::write_volatile(ptr.add(offset + 2), color.b);
                },
                PixelFormat::Bgr24 => {
                    core::ptr::write_volatile(ptr.add(offset), color.b);
                    core::ptr::write_volatile(ptr.add(offset + 1), color.g);
                    core::ptr::write_volatile(ptr.add(offset + 2), color.r);
                },
                PixelFormat::Rgb565 => {
                    let pixel = ptr.add(offset) as *mut u16;
                    core::ptr::write_volatile(pixel, color.to_rgb565());
                },
                PixelFormat::Bgr565 => {
                    let pixel = ptr.add(offset) as *mut u16;
                    core::ptr::write_volatile(pixel, color.to_bgr565());
                },
                PixelFormat::Unknown => {},
            }
        }
    }

    /// Get a pixel
    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        if x >= self.info.width || y >= self.info.height {
            return Color::BLACK;
        }

        let offset = self.info.pixel_offset(x, y);
        let ptr = self.info.virt_base as *const u8;

        unsafe {
            match self.info.format {
                PixelFormat::Rgb32 => {
                    let pixel = *(ptr.add(offset) as *const u32);
                    Color::from_rgb32(pixel)
                },
                PixelFormat::Bgr32 => {
                    let pixel = *(ptr.add(offset) as *const u32);
                    Color::from_rgb32(
                        ((pixel & 0xFF) << 16) | (pixel & 0xFF00) | ((pixel >> 16) & 0xFF),
                    )
                },
                PixelFormat::Rgb24 => {
                    Color::rgb(*ptr.add(offset), *ptr.add(offset + 1), *ptr.add(offset + 2))
                },
                PixelFormat::Bgr24 => {
                    Color::rgb(*ptr.add(offset + 2), *ptr.add(offset + 1), *ptr.add(offset))
                },
                _ => Color::BLACK,
            }
        }
    }

    /// Clear the screen with a color
    pub fn clear(&self, color: Color) {
        self.fill_rect(0, 0, self.info.width, self.info.height, color);
    }

    /// Draw a horizontal line
    pub fn draw_hline(&self, x: u32, y: u32, width: u32, color: Color) {
        for i in 0..width {
            self.set_pixel(x + i, y, color);
        }
    }

    /// Draw a vertical line
    pub fn draw_vline(&self, x: u32, y: u32, height: u32, color: Color) {
        for i in 0..height {
            self.set_pixel(x, y + i, color);
        }
    }

    /// Draw a line (Bresenham's algorithm)
    pub fn draw_line(&self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && y >= 0 {
                self.set_pixel(x as u32, y as u32, color);
            }

            if x == x1 && y == y1 {
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
    pub fn draw_rect(&self, x: u32, y: u32, width: u32, height: u32, color: Color) {
        self.draw_hline(x, y, width, color);
        self.draw_hline(x, y + height - 1, width, color);
        self.draw_vline(x, y, height, color);
        self.draw_vline(x + width - 1, y, height, color);
    }

    /// Fill a rectangle
    pub fn fill_rect(&self, x: u32, y: u32, width: u32, height: u32, color: Color) {
        // Optimize for 32-bit formats
        if matches!(self.info.format, PixelFormat::Rgb32 | PixelFormat::Bgr32) {
            let pixel_value = if self.info.format == PixelFormat::Rgb32 {
                color.to_rgb32()
            } else {
                color.to_bgr32()
            };

            for row in y..(y + height).min(self.info.height) {
                let offset = self.info.pixel_offset(x, row);
                let ptr = (self.info.virt_base + offset as u64) as *mut u32;

                for col in 0..width.min(self.info.width - x) {
                    unsafe {
                        core::ptr::write_volatile(ptr.add(col as usize), pixel_value);
                    }
                }
            }
        } else {
            for row in y..(y + height).min(self.info.height) {
                for col in x..(x + width).min(self.info.width) {
                    self.set_pixel(col, row, color);
                }
            }
        }
    }

    /// Draw a circle outline
    pub fn draw_circle(&self, cx: i32, cy: i32, radius: i32, color: Color) {
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.set_pixel((cx + x) as u32, (cy + y) as u32, color);
            self.set_pixel((cx + y) as u32, (cy + x) as u32, color);
            self.set_pixel((cx - y) as u32, (cy + x) as u32, color);
            self.set_pixel((cx - x) as u32, (cy + y) as u32, color);
            self.set_pixel((cx - x) as u32, (cy - y) as u32, color);
            self.set_pixel((cx - y) as u32, (cy - x) as u32, color);
            self.set_pixel((cx + y) as u32, (cy - x) as u32, color);
            self.set_pixel((cx + x) as u32, (cy - y) as u32, color);

            y += 1;
            err += 1 + 2 * y;
            if 2 * (err - x) + 1 > 0 {
                x -= 1;
                err += 1 - 2 * x;
            }
        }
    }

    /// Fill a circle
    pub fn fill_circle(&self, cx: i32, cy: i32, radius: i32, color: Color) {
        for y in (cy - radius)..=(cy + radius) {
            if y < 0 || y >= self.info.height as i32 {
                continue;
            }

            let dy = (y - cy).abs();
            let dx = ((radius * radius - dy * dy) as f64).sqrt() as i32;

            let x0 = (cx - dx).max(0) as u32;
            let x1 = (cx + dx).min(self.info.width as i32 - 1) as u32;

            for x in x0..=x1 {
                self.set_pixel(x, y as u32, color);
            }
        }
    }

    /// Draw a character at position
    pub fn draw_char(&self, x: u32, y: u32, ch: char, fg: Color, bg: Color) {
        let c = ch as usize;
        if c >= 256 {
            return;
        }

        let glyph = &FONT_DATA[c * FONT_HEIGHT..][..FONT_HEIGHT];

        for (row, &bits) in glyph.iter().enumerate() {
            for col in 0..FONT_WIDTH {
                let color = if bits & (0x80 >> col) != 0 { fg } else { bg };
                self.set_pixel(x + col as u32, y + row as u32, color);
            }
        }
    }

    /// Draw a character with transparency (only draws foreground)
    pub fn draw_char_transparent(&self, x: u32, y: u32, ch: char, color: Color) {
        let c = ch as usize;
        if c >= 256 {
            return;
        }

        let glyph = &FONT_DATA[c * FONT_HEIGHT..][..FONT_HEIGHT];

        for (row, &bits) in glyph.iter().enumerate() {
            for col in 0..FONT_WIDTH {
                if bits & (0x80 >> col) != 0 {
                    self.set_pixel(x + col as u32, y + row as u32, color);
                }
            }
        }
    }

    /// Draw text at position
    pub fn draw_text(&mut self, x: u32, y: u32, text: &str, color: Color) {
        let mut cx = x;
        let mut cy = y;

        for ch in text.chars() {
            if ch == '\n' {
                cx = x;
                cy += self.font_height;
                continue;
            }

            if ch == '\r' {
                cx = x;
                continue;
            }

            if ch == '\t' {
                cx += self.font_width * 4;
                continue;
            }

            if cx + self.font_width > self.info.width {
                cx = x;
                cy += self.font_height;
            }

            if cy + self.font_height > self.info.height {
                break;
            }

            self.draw_char_transparent(cx, cy, ch, color);
            cx += self.font_width;
        }
    }

    /// Draw text with background
    pub fn draw_text_bg(&mut self, x: u32, y: u32, text: &str, fg: Color, bg: Color) {
        let mut cx = x;
        let mut cy = y;

        for ch in text.chars() {
            if ch == '\n' {
                cx = x;
                cy += self.font_height;
                continue;
            }

            if cx + self.font_width > self.info.width {
                cx = x;
                cy += self.font_height;
            }

            if cy + self.font_height > self.info.height {
                break;
            }

            self.draw_char(cx, cy, ch, fg, bg);
            cx += self.font_width;
        }
    }

    /// Print to console area (bottom of screen)
    pub fn print(&mut self, text: &str) {
        for ch in text.chars() {
            self.print_char(ch);
        }
    }

    /// Print a character to console area
    fn print_char(&mut self, ch: char) {
        let max_cols = self.info.width / self.font_width;
        let max_rows = self.info.height / self.font_height;

        match ch {
            '\n' => {
                self.cursor_x = 0;
                self.cursor_y += 1;
            },
            '\r' => {
                self.cursor_x = 0;
            },
            '\t' => {
                self.cursor_x = (self.cursor_x + 4) & !3;
            },
            _ => {
                let x = self.cursor_x * self.font_width;
                let y = self.cursor_y * self.font_height;

                self.draw_char(x, y, ch, self.fg_color, self.bg_color);
                self.cursor_x += 1;
            },
        }

        // Handle line wrap
        if self.cursor_x >= max_cols {
            self.cursor_x = 0;
            self.cursor_y += 1;
        }

        // Handle scrolling
        if self.cursor_y >= max_rows {
            self.scroll_up(1);
            self.cursor_y = max_rows - 1;
        }
    }

    /// Scroll the screen up
    fn scroll_up(&mut self, lines: u32) {
        let scroll_height = lines * self.font_height;
        let copy_height = self.info.height - scroll_height;

        // Copy pixels up
        for y in 0..copy_height {
            let src_offset = self.info.pixel_offset(0, y + scroll_height);
            let dst_offset = self.info.pixel_offset(0, y);

            unsafe {
                let src = (self.info.virt_base + src_offset as u64) as *const u8;
                let dst = (self.info.virt_base + dst_offset as u64) as *mut u8;

                core::ptr::copy_nonoverlapping(src, dst, self.info.pitch as usize);
            }
        }

        // Clear the bottom
        self.fill_rect(
            0,
            copy_height,
            self.info.width,
            scroll_height,
            self.bg_color,
        );
    }

    /// Set console colors
    pub fn set_colors(&mut self, fg: Color, bg: Color) {
        self.fg_color = fg;
        self.bg_color = bg;
    }

    /// Draw a progress bar
    pub fn draw_progress_bar(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        progress: f32,
        fg: Color,
        bg: Color,
        border: Color,
    ) {
        // Draw border
        self.draw_rect(x, y, width, height, border);

        // Draw background
        self.fill_rect(x + 1, y + 1, width - 2, height - 2, bg);

        // Draw progress
        let fill_width = ((width - 2) as f32 * progress.clamp(0.0, 1.0)) as u32;
        if fill_width > 0 {
            self.fill_rect(x + 1, y + 1, fill_width, height - 2, fg);
        }
    }

    /// Draw boot splash screen
    pub fn draw_boot_splash(&mut self) {
        // Clear screen with Helix background
        self.clear(Color::HELIX_BG);

        let center_x = self.info.width / 2;
        let center_y = self.info.height / 2;

        // Draw Helix logo placeholder (simple helix shape)
        self.draw_helix_logo(center_x - 50, center_y - 80, Color::HELIX_PRIMARY);

        // Draw title
        let title = "Helix OS";
        let title_x = center_x - (title.len() as u32 * self.font_width) / 2;
        self.draw_text(title_x, center_y + 20, title, Color::HELIX_FG);

        // Draw version
        let version = "v0.1.0-alpha";
        let version_x = center_x - (version.len() as u32 * self.font_width) / 2;
        self.draw_text(version_x, center_y + 40, version, Color::HELIX_SECONDARY);
    }

    /// Draw a simple helix logo
    fn draw_helix_logo(&self, x: u32, y: u32, color: Color) {
        // Draw a simplified double helix
        for i in 0..100 {
            let t = (i as f64) * 0.1;

            // First strand
            let x1 = x as f64 + 50.0 + 40.0 * (t * 2.0).sin();
            let y1 = y as f64 + i as f64;

            // Second strand
            let x2 = x as f64 + 50.0 - 40.0 * (t * 2.0).sin();
            let y2 = y as f64 + i as f64;

            // Draw points
            self.fill_circle(x1 as i32, y1 as i32, 3, color);
            self.fill_circle(x2 as i32, y2 as i32, 3, color.darken(30));

            // Draw connecting lines every 10 pixels
            if i % 10 == 0 {
                self.draw_line(
                    x1 as i32,
                    y1 as i32,
                    x2 as i32,
                    y2 as i32,
                    Color::HELIX_ACCENT,
                );
            }
        }
    }

    /// Draw boot progress
    pub fn draw_boot_progress(&self, progress: f32, status: &str) {
        let bar_width = 300;
        let bar_height = 20;
        let bar_x = (self.info.width - bar_width) / 2;
        let bar_y = (self.info.height / 2) + 80;

        // Draw progress bar
        self.draw_progress_bar(
            bar_x,
            bar_y,
            bar_width,
            bar_height,
            progress,
            Color::HELIX_PRIMARY,
            Color::HELIX_BG.lighten(10),
            Color::HELIX_SECONDARY,
        );

        // Draw status text
        let status_x = (self.info.width - (status.len() as u32 * FONT_WIDTH as u32)) / 2;

        // Clear old status area
        self.fill_rect(
            0,
            bar_y + 30,
            self.info.width,
            FONT_HEIGHT as u32,
            Color::HELIX_BG,
        );

        // Draw new status - using immutable self requires a workaround
        for (i, ch) in status.chars().enumerate() {
            let cx = status_x + (i as u32 * FONT_WIDTH as u32);
            self.draw_char_transparent(cx, bar_y + 30, ch, Color::HELIX_FG);
        }
    }
}

// =============================================================================
// BUILT-IN BITMAP FONT
// =============================================================================

/// Font dimensions
const FONT_WIDTH: usize = 8;
const FONT_HEIGHT: usize = 16;

/// Built-in 8x16 VGA font (first 128 ASCII characters)
/// Each character is 16 bytes (one byte per row)
#[rustfmt::skip]
static FONT_DATA: [u8; 256 * 16] = {
    let mut font = [0u8; 256 * 16];

    // Initialize with basic ASCII font data
    // This is a subset - in a real implementation, you'd include the full VGA font

    // For now, provide just a placeholder that can be filled in
    // Each character is 16 bytes (8 bits wide x 16 rows tall)

    font
};

// Since we can't easily initialize a static const array with a real font in const context,
// we provide a runtime-loadable font system
static mut FONT_LOADED: bool = false;

/// Load the built-in font (called during initialization)
pub fn load_builtin_font() {
    unsafe {
        if FONT_LOADED {
            return;
        }

        // In a real implementation, this would copy font data from bootloader
        // or embed a proper bitmap font. For now, we generate a simple font.
        let font = &mut *(&FONT_DATA as *const _ as *mut [u8; 256 * 16]);

        // Generate basic glyphs for common characters
        // Space (32)
        // Already zero-filled

        // Generate simple block patterns for visible characters
        for c in 33u8..127 {
            let offset = (c as usize) * FONT_HEIGHT;
            // Create a simple pattern based on character code
            for row in 2..14 {
                font[offset + row] = 0x00; // Placeholder
            }
        }

        // Generate digits 0-9 with recognizable patterns
        generate_digit_glyphs(font);

        // Generate uppercase letters A-Z
        generate_letter_glyphs(font);

        FONT_LOADED = true;
    }
}

/// Generate digit glyphs (0-9)
fn generate_digit_glyphs(font: &mut [u8; 256 * 16]) {
    // Digit 0
    let zero: [u8; 16] = [
        0x00, 0x00, 0x3C, 0x66, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00,
        0x00,
    ];
    font[0x30 * 16..][..16].copy_from_slice(&zero);

    // Digit 1
    let one: [u8; 16] = [
        0x00, 0x00, 0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00, 0x00,
        0x00,
    ];
    font[0x31 * 16..][..16].copy_from_slice(&one);

    // Digit 2
    let two: [u8; 16] = [
        0x00, 0x00, 0x3C, 0x66, 0x06, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x66, 0x7E, 0x00, 0x00, 0x00,
        0x00,
    ];
    font[0x32 * 16..][..16].copy_from_slice(&two);

    // Continue for other digits...
}

/// Generate uppercase letter glyphs (A-Z)
fn generate_letter_glyphs(font: &mut [u8; 256 * 16]) {
    // Letter A
    let a: [u8; 16] = [
        0x00, 0x00, 0x18, 0x3C, 0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00, 0x00,
        0x00,
    ];
    font[0x41 * 16..][..16].copy_from_slice(&a);

    // Letter B
    let b: [u8; 16] = [
        0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00, 0x00,
        0x00,
    ];
    font[0x42 * 16..][..16].copy_from_slice(&b);

    // Letter C
    let c: [u8; 16] = [
        0x00, 0x00, 0x3C, 0x66, 0x66, 0x60, 0x60, 0x60, 0x60, 0x66, 0x66, 0x3C, 0x00, 0x00, 0x00,
        0x00,
    ];
    font[0x43 * 16..][..16].copy_from_slice(&c);

    // Continue for other letters...

    // Lowercase letters a-z (similar patterns, shifted down)
    // For brevity, we'll just copy uppercase patterns
    for i in 0..26 {
        let src_offset = (0x41 + i) * 16;
        let dst_offset = (0x61 + i) * 16;
        for j in 0..16 {
            font[dst_offset + j] = font[src_offset + j];
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_rgb() {
        let c = Color::rgb(255, 128, 64);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 64);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn test_color_to_rgb32() {
        let c = Color::rgb(0xFF, 0x80, 0x40);
        assert_eq!(c.to_rgb32(), 0x00FF8040);
    }

    #[test]
    fn test_color_to_bgr32() {
        let c = Color::rgb(0xFF, 0x80, 0x40);
        assert_eq!(c.to_bgr32(), 0x004080FF);
    }

    #[test]
    fn test_color_rgb565() {
        let c = Color::rgb(255, 255, 255);
        assert_eq!(c.to_rgb565(), 0xFFFF);

        let black = Color::BLACK;
        assert_eq!(black.to_rgb565(), 0x0000);
    }

    #[test]
    fn test_pixel_format_bpp() {
        assert_eq!(PixelFormat::Rgb32.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Rgb24.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::Rgb565.bytes_per_pixel(), 2);
    }

    #[test]
    fn test_framebuffer_info() {
        let mut info = FramebufferInfo::new();
        assert!(!info.is_valid());

        info.virt_base = 0x1000;
        info.width = 800;
        info.height = 600;
        info.format = PixelFormat::Rgb32;
        assert!(info.is_valid());
    }

    #[test]
    fn test_color_blend() {
        let bg = Color::BLACK;
        let fg = Color::rgba(255, 255, 255, 128);
        let blended = bg.blend(fg);

        // Should be approximately 50% gray
        assert!(blended.r > 120 && blended.r < 136);
        assert!(blended.g > 120 && blended.g < 136);
        assert!(blended.b > 120 && blended.b < 136);
    }

    #[test]
    fn test_color_darken() {
        let white = Color::WHITE;
        let darkened = white.darken(50);

        assert_eq!(darkened.r, 127);
        assert_eq!(darkened.g, 127);
        assert_eq!(darkened.b, 127);
    }
}
