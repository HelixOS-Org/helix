//! Framebuffer Information
//!
//! Structures and types for framebuffer configuration and manipulation.

use crate::error::{Error, Result};
use crate::raw::types::*;

// =============================================================================
// PIXEL FORMAT
// =============================================================================

/// Pixel format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PixelFormat {
    /// 32-bit RGB (8 bits per channel, red in LSB)
    Rgb32   = 0,
    /// 32-bit BGR (8 bits per channel, blue in LSB)
    Bgr32   = 1,
    /// RGB with 5 bits red, 6 bits green, 5 bits blue
    Rgb565  = 2,
    /// Custom bitmask format
    Bitmask = 3,
    /// Text mode (no framebuffer)
    Text    = 4,
    /// Unknown format
    Unknown = 255,
}

impl PixelFormat {
    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            PixelFormat::Rgb32 | PixelFormat::Bgr32 => 4,
            PixelFormat::Rgb565 => 2,
            PixelFormat::Bitmask => 4, // Assume 32-bit
            PixelFormat::Text | PixelFormat::Unknown => 0,
        }
    }

    /// Get bits per pixel
    pub fn bits_per_pixel(&self) -> u32 {
        self.bytes_per_pixel() * 8
    }

    /// Check if format is linear (not planar)
    pub fn is_linear(&self) -> bool {
        matches!(
            self,
            PixelFormat::Rgb32 | PixelFormat::Bgr32 | PixelFormat::Rgb565 | PixelFormat::Bitmask
        )
    }
}

impl Default for PixelFormat {
    fn default() -> Self {
        PixelFormat::Unknown
    }
}

// =============================================================================
// PIXEL BITMASK
// =============================================================================

/// Pixel bitmask for custom formats
#[derive(Debug, Clone, Copy, Default)]
pub struct PixelBitmask {
    /// Red channel mask
    pub red_mask: u32,
    /// Green channel mask
    pub green_mask: u32,
    /// Blue channel mask
    pub blue_mask: u32,
    /// Reserved/alpha channel mask
    pub alpha_mask: u32,
}

impl PixelBitmask {
    /// Standard RGB32 bitmask
    pub const RGB32: Self = Self {
        red_mask: 0x0000_00FF,
        green_mask: 0x0000_FF00,
        blue_mask: 0x00FF_0000,
        alpha_mask: 0xFF00_0000,
    };

    /// Standard BGR32 bitmask
    pub const BGR32: Self = Self {
        red_mask: 0x00FF_0000,
        green_mask: 0x0000_FF00,
        blue_mask: 0x0000_00FF,
        alpha_mask: 0xFF00_0000,
    };

    /// RGB565 bitmask
    pub const RGB565: Self = Self {
        red_mask: 0x0000_F800,
        green_mask: 0x0000_07E0,
        blue_mask: 0x0000_001F,
        alpha_mask: 0,
    };

    /// Get shift for mask
    fn mask_shift(mask: u32) -> u8 {
        if mask == 0 {
            return 0;
        }
        mask.trailing_zeros() as u8
    }

    /// Get width for mask
    fn mask_width(mask: u32) -> u8 {
        if mask == 0 {
            return 0;
        }
        (mask >> Self::mask_shift(mask)).count_ones() as u8
    }

    /// Get red shift
    pub fn red_shift(&self) -> u8 {
        Self::mask_shift(self.red_mask)
    }

    /// Get green shift
    pub fn green_shift(&self) -> u8 {
        Self::mask_shift(self.green_mask)
    }

    /// Get blue shift
    pub fn blue_shift(&self) -> u8 {
        Self::mask_shift(self.blue_mask)
    }

    /// Get alpha shift
    pub fn alpha_shift(&self) -> u8 {
        Self::mask_shift(self.alpha_mask)
    }

    /// Get red width
    pub fn red_width(&self) -> u8 {
        Self::mask_width(self.red_mask)
    }

    /// Get green width
    pub fn green_width(&self) -> u8 {
        Self::mask_width(self.green_mask)
    }

    /// Get blue width
    pub fn blue_width(&self) -> u8 {
        Self::mask_width(self.blue_mask)
    }

    /// Get alpha width
    pub fn alpha_width(&self) -> u8 {
        Self::mask_width(self.alpha_mask)
    }

    /// Encode RGB to pixel value
    pub fn encode(&self, r: u8, g: u8, b: u8) -> u32 {
        let r = (r as u32) >> (8 - self.red_width());
        let g = (g as u32) >> (8 - self.green_width());
        let b = (b as u32) >> (8 - self.blue_width());

        (r << self.red_shift()) | (g << self.green_shift()) | (b << self.blue_shift())
    }

    /// Encode RGBA to pixel value
    pub fn encode_rgba(&self, r: u8, g: u8, b: u8, a: u8) -> u32 {
        let rgb = self.encode(r, g, b);
        if self.alpha_mask != 0 {
            let a = (a as u32) >> (8 - self.alpha_width());
            rgb | (a << self.alpha_shift())
        } else {
            rgb
        }
    }

    /// Decode pixel value to RGB
    pub fn decode(&self, pixel: u32) -> (u8, u8, u8) {
        let r = ((pixel & self.red_mask) >> self.red_shift()) as u8;
        let g = ((pixel & self.green_mask) >> self.green_shift()) as u8;
        let b = ((pixel & self.blue_mask) >> self.blue_shift()) as u8;

        // Scale up to 8 bits
        let r = r << (8 - self.red_width());
        let g = g << (8 - self.green_width());
        let b = b << (8 - self.blue_width());

        (r, g, b)
    }
}

// =============================================================================
// FRAMEBUFFER INFO
// =============================================================================

/// Framebuffer information
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    /// Physical address of framebuffer
    pub address: PhysicalAddress,
    /// Size of framebuffer in bytes
    pub size: u64,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Stride (bytes per scanline)
    pub stride: u32,
    /// Pixel format
    pub format: PixelFormat,
    /// Pixel bitmask (for custom formats)
    pub bitmask: PixelBitmask,
    /// Bits per pixel
    pub bpp: u8,
}

impl FramebufferInfo {
    /// Calculate framebuffer size from dimensions
    pub fn calculate_size(_width: u32, height: u32, stride: u32) -> u64 {
        (height as u64) * (stride as u64)
    }

    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> u32 {
        (self.bpp as u32 + 7) / 8
    }

    /// Get pixel offset for coordinates
    pub fn pixel_offset(&self, x: u32, y: u32) -> Option<u64> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let offset =
            (y as u64) * (self.stride as u64) + (x as u64) * (self.bytes_per_pixel() as u64);
        Some(offset)
    }

    /// Check if coordinates are valid
    pub fn contains(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height
    }

    /// Get total pixels
    pub fn total_pixels(&self) -> u64 {
        (self.width as u64) * (self.height as u64)
    }

    /// Encode color to pixel value
    pub fn encode_pixel(&self, r: u8, g: u8, b: u8) -> u32 {
        match self.format {
            PixelFormat::Rgb32 => (r as u32) | ((g as u32) << 8) | ((b as u32) << 16),
            PixelFormat::Bgr32 => (b as u32) | ((g as u32) << 8) | ((r as u32) << 16),
            PixelFormat::Rgb565 => {
                ((r as u32 & 0xF8) << 8) | ((g as u32 & 0xFC) << 3) | ((b as u32 & 0xF8) >> 3)
            },
            PixelFormat::Bitmask => self.bitmask.encode(r, g, b),
            _ => 0,
        }
    }
}

impl Default for FramebufferInfo {
    fn default() -> Self {
        Self {
            address: PhysicalAddress(0),
            size: 0,
            width: 0,
            height: 0,
            stride: 0,
            format: PixelFormat::Unknown,
            bitmask: PixelBitmask::default(),
            bpp: 0,
        }
    }
}

// =============================================================================
// FRAMEBUFFER WRITER
// =============================================================================

/// Framebuffer writer for pixel manipulation
pub struct FramebufferWriter<'a> {
    /// Framebuffer info
    info: FramebufferInfo,
    /// Framebuffer memory
    buffer: &'a mut [u8],
}

impl<'a> FramebufferWriter<'a> {
    /// Create new framebuffer writer
    pub fn new(info: FramebufferInfo, buffer: &'a mut [u8]) -> Self {
        Self { info, buffer }
    }

    /// Get framebuffer info
    pub fn info(&self) -> &FramebufferInfo {
        &self.info
    }

    /// Write pixel at coordinates
    pub fn write_pixel(&mut self, pos_x: u32, pos_y: u32, red: u8, green: u8, blue: u8) -> Result<()> {
        let offset = self.info.pixel_offset(pos_x, pos_y).ok_or(Error::OutOfBounds)?;

        let pixel = self.info.encode_pixel(red, green, blue);
        let offset = offset as usize;
        let bpp = self.info.bytes_per_pixel() as usize;

        if offset + bpp > self.buffer.len() {
            return Err(Error::OutOfBounds);
        }

        let bytes = pixel.to_le_bytes();
        self.buffer[offset..offset + bpp].copy_from_slice(&bytes[..bpp]);

        Ok(())
    }

    /// Fill rectangle
    pub fn fill_rect(&mut self, pos_x: u32, pos_y: u32, width: u32, height: u32, red: u8, green: u8, blue: u8) -> Result<()> {
        let pixel = self.info.encode_pixel(red, green, blue);
        let bpp = self.info.bytes_per_pixel() as usize;
        let bytes = pixel.to_le_bytes();

        for dy in 0..height {
            for dx in 0..width {
                let px = pos_x + dx;
                let py = pos_y + dy;

                if let Some(offset) = self.info.pixel_offset(px, py) {
                    let offset = offset as usize;
                    if offset + bpp <= self.buffer.len() {
                        self.buffer[offset..offset + bpp].copy_from_slice(&bytes[..bpp]);
                    }
                }
            }
        }

        Ok(())
    }

    /// Clear framebuffer with color
    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        let _ = self.fill_rect(0, 0, self.info.width, self.info.height, r, g, b);
    }

    /// Draw horizontal line
    pub fn draw_hline(&mut self, pos_x: u32, pos_y: u32, line_width: u32, red: u8, green: u8, blue: u8) -> Result<()> {
        self.fill_rect(pos_x, pos_y, line_width, 1, red, green, blue)
    }

    /// Draw vertical line
    pub fn draw_vline(&mut self, pos_x: u32, pos_y: u32, line_height: u32, red: u8, green: u8, blue: u8) -> Result<()> {
        self.fill_rect(pos_x, pos_y, 1, line_height, red, green, blue)
    }

    /// Draw rectangle outline
    pub fn draw_rect(&mut self, pos_x: u32, pos_y: u32, width: u32, height: u32, red: u8, green: u8, blue: u8) -> Result<()> {
        self.draw_hline(pos_x, pos_y, width, red, green, blue)?;
        self.draw_hline(pos_x, pos_y + height.saturating_sub(1), width, red, green, blue)?;
        self.draw_vline(pos_x, pos_y, height, red, green, blue)?;
        self.draw_vline(pos_x + width.saturating_sub(1), pos_y, height, red, green, blue)?;
        Ok(())
    }
}

// =============================================================================
// FONT RENDERING
// =============================================================================

/// Basic 8x16 console font
pub mod font {
    /// Font width
    pub const FONT_WIDTH: u32 = 8;
    /// Font height
    pub const FONT_HEIGHT: u32 = 16;

    /// Get glyph bitmap for character
    pub fn get_glyph(c: char) -> &'static [u8; 16] {
        let index = c as usize;
        if index < 256 {
            &FONT_DATA[index]
        } else {
            &FONT_DATA[0] // Fallback to null character
        }
    }

    /// Basic 8x16 font data (256 characters)
    /// Each character is 16 bytes (16 rows of 8 pixels)
    static FONT_DATA: [[u8; 16]; 256] = {
        let mut data = [[0u8; 16]; 256];

        // Space (32)
        data[32] = [0; 16];

        // ! (33)
        data[33] = [
            0x00, 0x00, 0x18, 0x3C, 0x3C, 0x3C, 0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00,
            0x00, 0x00,
        ];

        // " (34)
        data[34] = [
            0x00, 0x66, 0x66, 0x66, 0x24, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];

        // # (35)
        data[35] = [
            0x00, 0x00, 0x00, 0x6C, 0x6C, 0xFE, 0x6C, 0x6C, 0x6C, 0xFE, 0x6C, 0x6C, 0x00, 0x00,
            0x00, 0x00,
        ];

        // 0-9 digits
        data[48] = [
            // 0
            0x00, 0x00, 0x38, 0x6C, 0xC6, 0xC6, 0xD6, 0xD6, 0xC6, 0xC6, 0x6C, 0x38, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[49] = [
            // 1
            0x00, 0x00, 0x18, 0x38, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[50] = [
            // 2
            0x00, 0x00, 0x7C, 0xC6, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0xC6, 0xFE, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[51] = [
            // 3
            0x00, 0x00, 0x7C, 0xC6, 0x06, 0x06, 0x3C, 0x06, 0x06, 0x06, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[52] = [
            // 4
            0x00, 0x00, 0x0C, 0x1C, 0x3C, 0x6C, 0xCC, 0xFE, 0x0C, 0x0C, 0x0C, 0x1E, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[53] = [
            // 5
            0x00, 0x00, 0xFE, 0xC0, 0xC0, 0xC0, 0xFC, 0x06, 0x06, 0x06, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[54] = [
            // 6
            0x00, 0x00, 0x38, 0x60, 0xC0, 0xC0, 0xFC, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[55] = [
            // 7
            0x00, 0x00, 0xFE, 0xC6, 0x06, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x30, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[56] = [
            // 8
            0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0x7C, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[57] = [
            // 9
            0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0x7E, 0x06, 0x06, 0x06, 0x0C, 0x78, 0x00, 0x00,
            0x00, 0x00,
        ];

        // A-Z uppercase letters
        data[65] = [
            // A
            0x00, 0x00, 0x10, 0x38, 0x6C, 0xC6, 0xC6, 0xFE, 0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[66] = [
            // B
            0x00, 0x00, 0xFC, 0x66, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x66, 0x66, 0xFC, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[67] = [
            // C
            0x00, 0x00, 0x3C, 0x66, 0xC2, 0xC0, 0xC0, 0xC0, 0xC0, 0xC2, 0x66, 0x3C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[68] = [
            // D
            0x00, 0x00, 0xF8, 0x6C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x6C, 0xF8, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[69] = [
            // E
            0x00, 0x00, 0xFE, 0x66, 0x62, 0x68, 0x78, 0x68, 0x60, 0x62, 0x66, 0xFE, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[70] = [
            // F
            0x00, 0x00, 0xFE, 0x66, 0x62, 0x68, 0x78, 0x68, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[71] = [
            // G
            0x00, 0x00, 0x3C, 0x66, 0xC2, 0xC0, 0xC0, 0xDE, 0xC6, 0xC6, 0x66, 0x3A, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[72] = [
            // H
            0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xFE, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[73] = [
            // I
            0x00, 0x00, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[74] = [
            // J
            0x00, 0x00, 0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0xCC, 0xCC, 0xCC, 0x78, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[75] = [
            // K
            0x00, 0x00, 0xE6, 0x66, 0x66, 0x6C, 0x78, 0x78, 0x6C, 0x66, 0x66, 0xE6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[76] = [
            // L
            0x00, 0x00, 0xF0, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x62, 0x66, 0xFE, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[77] = [
            // M
            0x00, 0x00, 0xC6, 0xEE, 0xFE, 0xFE, 0xD6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[78] = [
            // N
            0x00, 0x00, 0xC6, 0xE6, 0xF6, 0xFE, 0xDE, 0xCE, 0xC6, 0xC6, 0xC6, 0xC6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[79] = [
            // O
            0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[80] = [
            // P
            0x00, 0x00, 0xFC, 0x66, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[81] = [
            // Q
            0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xD6, 0xDE, 0x7C, 0x0C, 0x0E,
            0x00, 0x00,
        ];
        data[82] = [
            // R
            0x00, 0x00, 0xFC, 0x66, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x66, 0xE6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[83] = [
            // S
            0x00, 0x00, 0x7C, 0xC6, 0xC6, 0x60, 0x38, 0x0C, 0x06, 0xC6, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[84] = [
            // T
            0x00, 0x00, 0x7E, 0x7E, 0x5A, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[85] = [
            // U
            0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[86] = [
            // V
            0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x6C, 0x38, 0x10, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[87] = [
            // W
            0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xD6, 0xD6, 0xD6, 0xFE, 0xEE, 0x6C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[88] = [
            // X
            0x00, 0x00, 0xC6, 0xC6, 0x6C, 0x7C, 0x38, 0x38, 0x7C, 0x6C, 0xC6, 0xC6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[89] = [
            // Y
            0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[90] = [
            // Z
            0x00, 0x00, 0xFE, 0xC6, 0x86, 0x0C, 0x18, 0x30, 0x60, 0xC2, 0xC6, 0xFE, 0x00, 0x00,
            0x00, 0x00,
        ];

        // a-z lowercase letters
        data[97] = [
            // a
            0x00, 0x00, 0x00, 0x00, 0x00, 0x78, 0x0C, 0x7C, 0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[98] = [
            // b
            0x00, 0x00, 0xE0, 0x60, 0x60, 0x78, 0x6C, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[99] = [
            // c
            0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0xC0, 0xC0, 0xC0, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[100] = [
            // d
            0x00, 0x00, 0x1C, 0x0C, 0x0C, 0x3C, 0x6C, 0xCC, 0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[101] = [
            // e
            0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0xFE, 0xC0, 0xC0, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[102] = [
            // f
            0x00, 0x00, 0x38, 0x6C, 0x64, 0x60, 0xF0, 0x60, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[103] = [
            // g
            0x00, 0x00, 0x00, 0x00, 0x00, 0x76, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x7C, 0x0C, 0xCC,
            0x78, 0x00,
        ];
        data[104] = [
            // h
            0x00, 0x00, 0xE0, 0x60, 0x60, 0x6C, 0x76, 0x66, 0x66, 0x66, 0x66, 0xE6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[105] = [
            // i
            0x00, 0x00, 0x18, 0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[106] = [
            // j
            0x00, 0x00, 0x06, 0x06, 0x00, 0x0E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x66, 0x66,
            0x3C, 0x00,
        ];
        data[107] = [
            // k
            0x00, 0x00, 0xE0, 0x60, 0x60, 0x66, 0x6C, 0x78, 0x78, 0x6C, 0x66, 0xE6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[108] = [
            // l
            0x00, 0x00, 0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[109] = [
            // m
            0x00, 0x00, 0x00, 0x00, 0x00, 0xEC, 0xFE, 0xD6, 0xD6, 0xD6, 0xD6, 0xC6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[110] = [
            // n
            0x00, 0x00, 0x00, 0x00, 0x00, 0xDC, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[111] = [
            // o
            0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[112] = [
            // p
            0x00, 0x00, 0x00, 0x00, 0x00, 0xDC, 0x66, 0x66, 0x66, 0x66, 0x66, 0x7C, 0x60, 0x60,
            0xF0, 0x00,
        ];
        data[113] = [
            // q
            0x00, 0x00, 0x00, 0x00, 0x00, 0x76, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x7C, 0x0C, 0x0C,
            0x1E, 0x00,
        ];
        data[114] = [
            // r
            0x00, 0x00, 0x00, 0x00, 0x00, 0xDC, 0x76, 0x66, 0x60, 0x60, 0x60, 0xF0, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[115] = [
            // s
            0x00, 0x00, 0x00, 0x00, 0x00, 0x7C, 0xC6, 0x60, 0x38, 0x0C, 0xC6, 0x7C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[116] = [
            // t
            0x00, 0x00, 0x10, 0x30, 0x30, 0xFC, 0x30, 0x30, 0x30, 0x30, 0x36, 0x1C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[117] = [
            // u
            0x00, 0x00, 0x00, 0x00, 0x00, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0xCC, 0x76, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[118] = [
            // v
            0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[119] = [
            // w
            0x00, 0x00, 0x00, 0x00, 0x00, 0xC6, 0xC6, 0xD6, 0xD6, 0xD6, 0xFE, 0x6C, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[120] = [
            // x
            0x00, 0x00, 0x00, 0x00, 0x00, 0xC6, 0x6C, 0x38, 0x38, 0x38, 0x6C, 0xC6, 0x00, 0x00,
            0x00, 0x00,
        ];
        data[121] = [
            // y
            0x00, 0x00, 0x00, 0x00, 0x00, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0xC6, 0x7E, 0x06, 0x0C,
            0xF8, 0x00,
        ];
        data[122] = [
            // z
            0x00, 0x00, 0x00, 0x00, 0x00, 0xFE, 0xCC, 0x18, 0x30, 0x60, 0xC6, 0xFE, 0x00, 0x00,
            0x00, 0x00,
        ];

        data
    };
}

/// Console for text rendering on framebuffer
pub struct FramebufferConsole<'a> {
    writer: FramebufferWriter<'a>,
    cursor_x: u32,
    cursor_y: u32,
    fg_color: (u8, u8, u8),
    bg_color: (u8, u8, u8),
}

impl<'a> FramebufferConsole<'a> {
    /// Create new console
    pub fn new(writer: FramebufferWriter<'a>) -> Self {
        Self {
            writer,
            cursor_x: 0,
            cursor_y: 0,
            fg_color: (255, 255, 255),
            bg_color: (0, 0, 0),
        }
    }

    /// Get columns
    pub fn columns(&self) -> u32 {
        self.writer.info.width / font::FONT_WIDTH
    }

    /// Get rows
    pub fn rows(&self) -> u32 {
        self.writer.info.height / font::FONT_HEIGHT
    }

    /// Set foreground color
    pub fn set_fg(&mut self, r: u8, g: u8, b: u8) {
        self.fg_color = (r, g, b);
    }

    /// Set background color
    pub fn set_bg(&mut self, r: u8, g: u8, b: u8) {
        self.bg_color = (r, g, b);
    }

    /// Clear console
    pub fn clear(&mut self) {
        self.writer
            .clear(self.bg_color.0, self.bg_color.1, self.bg_color.2);
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    /// Write character
    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => {
                self.cursor_x = 0;
                self.cursor_y += 1;
                if self.cursor_y >= self.rows() {
                    self.scroll();
                }
            },
            '\r' => {
                self.cursor_x = 0;
            },
            '\t' => {
                self.cursor_x = (self.cursor_x + 8) & !7;
                if self.cursor_x >= self.columns() {
                    self.write_char('\n');
                }
            },
            c => {
                if self.cursor_x >= self.columns() {
                    self.write_char('\n');
                }

                self.draw_char(c, self.cursor_x, self.cursor_y);
                self.cursor_x += 1;
            },
        }
    }

    /// Write string
    pub fn write_str(&mut self, s: &str) {
        for c in s.chars() {
            self.write_char(c);
        }
    }

    /// Draw character at position
    fn draw_char(&mut self, c: char, col: u32, row: u32) {
        let glyph = font::get_glyph(c);
        let x = col * font::FONT_WIDTH;
        let y = row * font::FONT_HEIGHT;

        for (dy, &byte) in glyph.iter().enumerate() {
            for dx in 0..8 {
                let pixel = if (byte >> (7 - dx)) & 1 != 0 {
                    self.fg_color
                } else {
                    self.bg_color
                };

                let _ = self
                    .writer
                    .write_pixel(x + dx, y + dy as u32, pixel.0, pixel.1, pixel.2);
            }
        }
    }

    /// Scroll up one line
    fn scroll(&mut self) {
        // Simple scroll: just reset to top
        // A real implementation would copy framebuffer contents
        self.cursor_y = self.rows() - 1;
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_format() {
        assert_eq!(PixelFormat::Rgb32.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Rgb565.bytes_per_pixel(), 2);
        assert!(PixelFormat::Rgb32.is_linear());
    }

    #[test]
    fn test_bitmask() {
        let mask = PixelBitmask::RGB32;
        assert_eq!(mask.red_shift(), 0);
        assert_eq!(mask.green_shift(), 8);
        assert_eq!(mask.blue_shift(), 16);

        let pixel = mask.encode(255, 128, 64);
        assert_eq!(pixel & 0xFF, 255);
        assert_eq!((pixel >> 8) & 0xFF, 128);
        assert_eq!((pixel >> 16) & 0xFF, 64);
    }

    #[test]
    fn test_framebuffer_info() {
        let fb = FramebufferInfo {
            address: 0xFD000000,
            size: 1920 * 1080 * 4,
            width: 1920,
            height: 1080,
            stride: 1920 * 4,
            format: PixelFormat::Bgr32,
            bitmask: PixelBitmask::BGR32,
            bpp: 32,
        };

        assert_eq!(fb.bytes_per_pixel(), 4);
        assert_eq!(fb.total_pixels(), 1920 * 1080);
        assert!(fb.contains(100, 100));
        assert!(!fb.contains(2000, 100));
    }
}
