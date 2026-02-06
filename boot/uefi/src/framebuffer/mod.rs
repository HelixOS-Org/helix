//! Graphics Memory and Framebuffer Management for Helix UEFI Bootloader
//!
//! This module provides comprehensive graphics memory management,
//! framebuffer handling, and advanced rendering capabilities for
//! the UEFI boot environment.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     Graphics Memory Architecture                        │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                       Display Manager                            │  │
//! │  │  Resolution │ Color Depth │ Refresh Rate │ Multi-Monitor         │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                              │                                         │
//! │         ┌────────────────────┼────────────────────┐                    │
//! │         ▼                    ▼                    ▼                    │
//! │  ┌────────────┐     ┌────────────────┐    ┌─────────────┐             │
//! │  │Framebuffer │     │ Double Buffer  │    │   Font      │             │
//! │  │  Manager   │     │    Engine      │    │   Renderer  │             │
//! │  └────────────┘     └────────────────┘    └─────────────┘             │
//! │         │                    │                    │                    │
//! │         ▼                    ▼                    ▼                    │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                    Hardware Framebuffer                          │  │
//! │  │  Linear FB │ Tiled Memory │ Compressed │ VRAM Direct            │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - Multiple pixel formats
//! - Double buffering
//! - Font rendering
//! - Image support (BMP, PNG basics)
//! - Animated boot splash
//! - Progress bar rendering
//! - Multi-monitor support

use core::fmt;

// =============================================================================
// PIXEL FORMATS
// =============================================================================

/// Pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// RGB 888 (24-bit)
    Rgb888,
    /// BGR 888 (24-bit)
    Bgr888,
    /// RGBX 8888 (32-bit, X unused)
    Rgbx8888,
    /// BGRX 8888 (32-bit, X unused)
    Bgrx8888,
    /// RGBA 8888 (32-bit with alpha)
    Rgba8888,
    /// BGRA 8888 (32-bit with alpha)
    #[default]
    Bgra8888,
    /// RGB 565 (16-bit)
    Rgb565,
    /// BGR 565 (16-bit)
    Bgr565,
    /// RGB 555 (15-bit)
    Rgb555,
    /// BGR 555 (15-bit)
    Bgr555,
    /// ARGB 1555 (16-bit with 1-bit alpha)
    Argb1555,
    /// Grayscale 8-bit
    Gray8,
    /// Grayscale 16-bit
    Gray16,
    /// Indexed 8-bit (palette)
    Indexed8,
    /// Indexed 4-bit (palette)
    Indexed4,
    /// Bit mask format
    BitMask,
}

impl PixelFormat {
    /// Get bytes per pixel
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Rgb888 | PixelFormat::Bgr888 => 3,
            PixelFormat::Rgbx8888
            | PixelFormat::Bgrx8888
            | PixelFormat::Rgba8888
            | PixelFormat::Bgra8888 => 4,
            PixelFormat::Rgb565
            | PixelFormat::Bgr565
            | PixelFormat::Rgb555
            | PixelFormat::Bgr555
            | PixelFormat::Argb1555
            | PixelFormat::Gray16 => 2,
            PixelFormat::Gray8 | PixelFormat::Indexed8 => 1,
            PixelFormat::Indexed4 => 1, // Half byte, but use 1 for safety
            PixelFormat::BitMask => 4,  // Assume 32-bit
        }
    }

    /// Get bits per pixel
    pub const fn bits_per_pixel(&self) -> usize {
        self.bytes_per_pixel() * 8
    }

    /// Check if format has alpha channel
    pub const fn has_alpha(&self) -> bool {
        matches!(
            self,
            PixelFormat::Rgba8888 | PixelFormat::Bgra8888 | PixelFormat::Argb1555
        )
    }

    /// Check if format uses palette
    pub const fn is_indexed(&self) -> bool {
        matches!(self, PixelFormat::Indexed8 | PixelFormat::Indexed4)
    }
}

/// Pixel bit mask (for BitMask format)
#[derive(Debug, Clone, Copy, Default)]
pub struct PixelBitmask {
    /// Red mask
    pub red: u32,
    /// Green mask
    pub green: u32,
    /// Blue mask
    pub blue: u32,
    /// Reserved mask
    pub reserved: u32,
}

impl PixelBitmask {
    /// Create new bitmask
    pub const fn new(red: u32, green: u32, blue: u32, reserved: u32) -> Self {
        Self {
            red,
            green,
            blue,
            reserved,
        }
    }

    /// Standard RGBX 8888 mask
    pub const RGBX_8888: Self = Self::new(0x00FF_0000, 0x0000_FF00, 0x0000_00FF, 0xFF00_0000);

    /// Standard BGRX 8888 mask
    pub const BGRX_8888: Self = Self::new(0x0000_00FF, 0x0000_FF00, 0x00FF_0000, 0xFF00_0000);

    /// RGB 565 mask
    pub const RGB_565: Self = Self::new(0xF800, 0x07E0, 0x001F, 0x0000);
}

// =============================================================================
// COLOR
// =============================================================================

/// RGBA color
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255, 255 = opaque)
    pub a: u8,
}

impl Color {
    /// Create new color
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create opaque color
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create from 32-bit ARGB value
    pub const fn from_argb(value: u32) -> Self {
        Self {
            a: ((value >> 24) & 0xFF) as u8,
            r: ((value >> 16) & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: (value & 0xFF) as u8,
        }
    }

    /// Convert to 32-bit ARGB
    pub const fn to_argb(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Convert to 32-bit BGRA
    pub const fn to_bgra(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.b as u32) << 16) | ((self.g as u32) << 8) | (self.r as u32)
    }

    /// Convert to RGB 565
    pub const fn to_rgb565(&self) -> u16 {
        (((self.r as u16) >> 3) << 11) | (((self.g as u16) >> 2) << 5) | ((self.b as u16) >> 3)
    }

    /// Create from RGB 565
    pub const fn from_rgb565(value: u16) -> Self {
        Self {
            r: ((value >> 11) as u8) << 3,
            g: (((value >> 5) & 0x3F) as u8) << 2,
            b: ((value & 0x1F) as u8) << 3,
            a: 255,
        }
    }

    /// Create grayscale color
    pub const fn gray(value: u8) -> Self {
        Self {
            r: value,
            g: value,
            b: value,
            a: 255,
        }
    }

    /// Convert to grayscale
    pub const fn to_gray(&self) -> u8 {
        // Luminosity formula: 0.299*R + 0.587*G + 0.114*B
        // Approximation: (R*77 + G*150 + B*29) / 256
        ((self.r as u32 * 77 + self.g as u32 * 150 + self.b as u32 * 29) / 256) as u8
    }

    /// Blend with another color
    pub fn blend(&self, other: &Color) -> Self {
        let alpha = other.a as u32;
        let inv_alpha = 255 - alpha;
        Self {
            r: ((self.r as u32 * inv_alpha + other.r as u32 * alpha) / 255) as u8,
            g: ((self.g as u32 * inv_alpha + other.g as u32 * alpha) / 255) as u8,
            b: ((self.b as u32 * inv_alpha + other.b as u32 * alpha) / 255) as u8,
            a: 255,
        }
    }

    /// Interpolate between two colors
    pub fn lerp(&self, other: &Color, t: u8) -> Self {
        let t = t as u32;
        let inv_t = 255 - t;
        Self {
            r: ((self.r as u32 * inv_t + other.r as u32 * t) / 255) as u8,
            g: ((self.g as u32 * inv_t + other.g as u32 * t) / 255) as u8,
            b: ((self.b as u32 * inv_t + other.b as u32 * t) / 255) as u8,
            a: ((self.a as u32 * inv_t + other.a as u32 * t) / 255) as u8,
        }
    }

    // Predefined colors
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    pub const RED: Self = Self::rgb(255, 0, 0);
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    pub const CYAN: Self = Self::rgb(0, 255, 255);
    pub const MAGENTA: Self = Self::rgb(255, 0, 255);
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);

    // Boot-related colors
    pub const BOOT_BG: Self = Self::rgb(0, 0, 42);
    pub const BOOT_TEXT: Self = Self::rgb(200, 200, 200);
    pub const BOOT_HIGHLIGHT: Self = Self::rgb(0, 120, 215);
    pub const BOOT_SUCCESS: Self = Self::rgb(0, 200, 0);
    pub const BOOT_WARNING: Self = Self::rgb(255, 200, 0);
    pub const BOOT_ERROR: Self = Self::rgb(255, 50, 50);
}

// =============================================================================
// DISPLAY MODE
// =============================================================================

/// Display mode information
#[derive(Debug, Clone, Copy)]
pub struct DisplayMode {
    /// Mode number
    pub mode_number: u32,
    /// Horizontal resolution
    pub width: u32,
    /// Vertical resolution
    pub height: u32,
    /// Pixel format
    pub pixel_format: PixelFormat,
    /// Pixels per scanline
    pub pixels_per_scanline: u32,
    /// Refresh rate (Hz, 0 if unknown)
    pub refresh_rate: u32,
    /// Bits per pixel
    pub bits_per_pixel: u32,
}

impl DisplayMode {
    /// Create new display mode
    pub const fn new(mode_number: u32, width: u32, height: u32, pixel_format: PixelFormat) -> Self {
        Self {
            mode_number,
            width,
            height,
            pixel_format,
            pixels_per_scanline: width,
            refresh_rate: 60,
            bits_per_pixel: pixel_format.bits_per_pixel() as u32,
        }
    }

    /// Get total framebuffer size in bytes
    pub const fn framebuffer_size(&self) -> usize {
        self.pixels_per_scanline as usize
            * self.height as usize
            * self.pixel_format.bytes_per_pixel()
    }

    /// Get stride (bytes per scanline)
    pub const fn stride(&self) -> usize {
        self.pixels_per_scanline as usize * self.pixel_format.bytes_per_pixel()
    }

    /// Check if mode is supported resolution
    pub const fn is_hd(&self) -> bool {
        self.width >= 1280 && self.height >= 720
    }

    /// Check if mode is Full HD
    pub const fn is_fhd(&self) -> bool {
        self.width >= 1920 && self.height >= 1080
    }

    /// Check if mode is 4K
    pub const fn is_4k(&self) -> bool {
        self.width >= 3840 && self.height >= 2160
    }
}

impl fmt::Display for DisplayMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}x{}@{}Hz ({:?})",
            self.width, self.height, self.refresh_rate, self.pixel_format
        )
    }
}

// =============================================================================
// RECTANGLE AND POINT
// =============================================================================

/// 2D point
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Point {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
}

impl Point {
    /// Create new point
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Origin point
    pub const ORIGIN: Self = Self::new(0, 0);
}

/// 2D size
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Size {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Size {
    /// Create new size
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Zero size
    pub const ZERO: Self = Self::new(0, 0);

    /// Check if size is zero
    pub const fn is_zero(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Get area
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

/// Rectangle
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rect {
    /// X position
    pub x: i32,
    /// Y position
    pub y: i32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Rect {
    /// Create new rectangle
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Create from position and size
    pub const fn from_pos_size(pos: Point, size: Size) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
        }
    }

    /// Get right edge
    pub const fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    /// Get bottom edge
    pub const fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    /// Get center point
    pub const fn center(&self) -> Point {
        Point::new(
            self.x + (self.width / 2) as i32,
            self.y + (self.height / 2) as i32,
        )
    }

    /// Check if rectangle contains point
    pub const fn contains(&self, point: Point) -> bool {
        point.x >= self.x && point.x < self.right() && point.y >= self.y && point.y < self.bottom()
    }

    /// Check if rectangles intersect
    pub const fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Get intersection of two rectangles
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }

        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        Some(Rect::new(x, y, (right - x) as u32, (bottom - y) as u32))
    }

    /// Get area
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}

// =============================================================================
// FRAMEBUFFER
// =============================================================================

/// Framebuffer information
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    /// Base address
    pub base_address: u64,
    /// Size in bytes
    pub size: usize,
    /// Display mode
    pub mode: DisplayMode,
}

impl FramebufferInfo {
    /// Create new framebuffer info
    pub const fn new(base_address: u64, mode: DisplayMode) -> Self {
        Self {
            base_address,
            size: mode.framebuffer_size(),
            mode,
        }
    }

    /// Get pixel offset for coordinates
    pub const fn pixel_offset(&self, x: u32, y: u32) -> usize {
        (y as usize * self.mode.stride()) + (x as usize * self.mode.pixel_format.bytes_per_pixel())
    }

    /// Check if coordinates are valid
    pub const fn is_valid(&self, x: u32, y: u32) -> bool {
        x < self.mode.width && y < self.mode.height
    }
}

/// Double buffer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferState {
    /// Front buffer (being displayed)
    Front,
    /// Back buffer (being drawn to)
    Back,
}

/// Double buffer info
#[derive(Debug, Clone, Copy)]
pub struct DoubleBuffer {
    /// Front buffer address
    pub front_buffer: u64,
    /// Back buffer address
    pub back_buffer: u64,
    /// Buffer size
    pub buffer_size: usize,
    /// Current draw buffer
    pub current: BufferState,
}

impl DoubleBuffer {
    /// Create new double buffer
    pub const fn new(front: u64, back: u64, size: usize) -> Self {
        Self {
            front_buffer: front,
            back_buffer: back,
            buffer_size: size,
            current: BufferState::Back,
        }
    }

    /// Get current draw buffer address
    pub const fn draw_buffer(&self) -> u64 {
        match self.current {
            BufferState::Front => self.front_buffer,
            BufferState::Back => self.back_buffer,
        }
    }

    /// Swap buffers
    pub fn swap(&mut self) {
        self.current = match self.current {
            BufferState::Front => BufferState::Back,
            BufferState::Back => BufferState::Front,
        };
    }
}

// =============================================================================
// FONT RENDERING
// =============================================================================

/// Font type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontType {
    /// Bitmap font (8x16, 8x8, etc.)
    Bitmap,
    /// PSF1 font
    Psf1,
    /// PSF2 font
    Psf2,
    /// TrueType (not rendered, just detected)
    TrueType,
}

/// Font glyph
#[derive(Debug, Clone)]
pub struct Glyph {
    /// Character code
    pub codepoint: u32,
    /// Glyph width
    pub width: u8,
    /// Glyph height
    pub height: u8,
    /// X offset
    pub offset_x: i8,
    /// Y offset
    pub offset_y: i8,
    /// Advance width
    pub advance: u8,
    /// Bitmap data (1 bit per pixel, row-major)
    pub bitmap: [u8; 64],
    /// Bitmap data length
    pub bitmap_len: usize,
}

impl Glyph {
    /// Create new empty glyph
    pub const fn new(codepoint: u32, width: u8, height: u8) -> Self {
        Self {
            codepoint,
            width,
            height,
            offset_x: 0,
            offset_y: 0,
            advance: width,
            bitmap: [0u8; 64],
            bitmap_len: 0,
        }
    }

    /// Get pixel at position (1-bit)
    pub fn get_pixel(&self, x: u8, y: u8) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let byte_index = (y as usize * ((self.width as usize + 7) / 8)) + (x as usize / 8);
        let bit_index = 7 - (x % 8);
        if byte_index < self.bitmap_len {
            (self.bitmap[byte_index] >> bit_index) & 1 != 0
        } else {
            false
        }
    }
}

/// PSF1 font header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Psf1Header {
    /// Magic bytes (0x36, 0x04)
    pub magic: [u8; 2],
    /// Font mode
    pub mode: u8,
    /// Bytes per glyph
    pub char_size: u8,
}

impl Psf1Header {
    /// PSF1 magic bytes
    pub const MAGIC: [u8; 2] = [0x36, 0x04];

    /// Check if valid PSF1 header
    pub const fn is_valid(&self) -> bool {
        self.magic[0] == Self::MAGIC[0] && self.magic[1] == Self::MAGIC[1]
    }

    /// Get number of glyphs
    pub const fn num_glyphs(&self) -> u32 {
        if self.mode & 0x01 != 0 {
            512
        } else {
            256
        }
    }
}

/// PSF2 font header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Psf2Header {
    /// Magic bytes
    pub magic: [u8; 4],
    /// Version
    pub version: u32,
    /// Header size
    pub header_size: u32,
    /// Flags
    pub flags: u32,
    /// Number of glyphs
    pub num_glyphs: u32,
    /// Bytes per glyph
    pub bytes_per_glyph: u32,
    /// Height
    pub height: u32,
    /// Width
    pub width: u32,
}

impl Psf2Header {
    /// PSF2 magic bytes
    pub const MAGIC: [u8; 4] = [0x72, 0xB5, 0x4A, 0x86];

    /// Check if valid PSF2 header
    pub const fn is_valid(&self) -> bool {
        self.magic[0] == Self::MAGIC[0]
            && self.magic[1] == Self::MAGIC[1]
            && self.magic[2] == Self::MAGIC[2]
            && self.magic[3] == Self::MAGIC[3]
    }

    /// Check if has unicode table
    pub const fn has_unicode_table(&self) -> bool {
        self.flags & 0x01 != 0
    }
}

// =============================================================================
// IMAGE FORMATS
// =============================================================================

/// Image format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// BMP (Windows Bitmap)
    Bmp,
    /// PNG
    Png,
    /// TGA
    Tga,
    /// JPEG
    Jpeg,
    /// ICO (Windows Icon)
    Ico,
    /// Raw pixel data
    Raw,
}

impl ImageFormat {
    /// Detect format from magic bytes
    pub const fn from_magic(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        // BMP: "BM"
        if data[0] == b'B' && data[1] == b'M' {
            return Some(ImageFormat::Bmp);
        }

        // PNG: 0x89 'P' 'N' 'G' 0x0D 0x0A 0x1A 0x0A
        if data[0] == 0x89 && data[1] == b'P' && data[2] == b'N' && data[3] == b'G' {
            return Some(ImageFormat::Png);
        }

        // JPEG: 0xFF 0xD8 0xFF
        if data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
            return Some(ImageFormat::Jpeg);
        }

        // ICO: 0x00 0x00 0x01 0x00
        if data[0] == 0x00 && data[1] == 0x00 && data[2] == 0x01 && data[3] == 0x00 {
            return Some(ImageFormat::Ico);
        }

        None
    }
}

/// BMP file header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct BmpFileHeader {
    /// Magic ("BM")
    pub magic: [u8; 2],
    /// File size
    pub file_size: u32,
    /// Reserved
    pub reserved: u32,
    /// Offset to pixel data
    pub pixel_offset: u32,
}

/// BMP info header (BITMAPINFOHEADER)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct BmpInfoHeader {
    /// Header size
    pub header_size: u32,
    /// Image width
    pub width: i32,
    /// Image height (negative = top-down)
    pub height: i32,
    /// Color planes (1)
    pub planes: u16,
    /// Bits per pixel
    pub bits_per_pixel: u16,
    /// Compression
    pub compression: u32,
    /// Image size
    pub image_size: u32,
    /// Horizontal resolution (pixels per meter)
    pub x_pixels_per_meter: i32,
    /// Vertical resolution (pixels per meter)
    pub y_pixels_per_meter: i32,
    /// Colors used
    pub colors_used: u32,
    /// Important colors
    pub colors_important: u32,
}

impl BmpInfoHeader {
    /// No compression
    pub const BI_RGB: u32 = 0;
    /// RLE 8-bit
    pub const BI_RLE8: u32 = 1;
    /// RLE 4-bit
    pub const BI_RLE4: u32 = 2;
    /// Bitfields
    pub const BI_BITFIELDS: u32 = 3;

    /// Check if top-down orientation
    pub const fn is_top_down(&self) -> bool {
        self.height < 0
    }

    /// Get absolute height
    pub const fn abs_height(&self) -> u32 {
        if self.height < 0 {
            (-self.height) as u32
        } else {
            self.height as u32
        }
    }
}

/// Image data
#[derive(Debug, Clone)]
pub struct Image {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Pixel format
    pub format: PixelFormat,
    /// Pixel data (limited size for no_std)
    pub data: [u8; 65536],
    /// Actual data length
    pub data_len: usize,
}

impl Image {
    /// Create new empty image
    pub const fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        Self {
            width,
            height,
            format,
            data: [0u8; 65536],
            data_len: 0,
        }
    }

    /// Get pixel at coordinates
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Color> {
        if x >= self.width || y >= self.height {
            return None;
        }

        let bpp = self.format.bytes_per_pixel();
        let offset = (y as usize * self.width as usize + x as usize) * bpp;

        if offset + bpp > self.data_len {
            return None;
        }

        match self.format {
            PixelFormat::Rgba8888 => Some(Color::new(
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
                self.data[offset + 3],
            )),
            PixelFormat::Bgra8888 => Some(Color::new(
                self.data[offset + 2],
                self.data[offset + 1],
                self.data[offset],
                self.data[offset + 3],
            )),
            PixelFormat::Rgb888 => Some(Color::rgb(
                self.data[offset],
                self.data[offset + 1],
                self.data[offset + 2],
            )),
            PixelFormat::Bgr888 => Some(Color::rgb(
                self.data[offset + 2],
                self.data[offset + 1],
                self.data[offset],
            )),
            _ => None,
        }
    }

    /// Set pixel at coordinates
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }

        let bpp = self.format.bytes_per_pixel();
        let offset = (y as usize * self.width as usize + x as usize) * bpp;

        if offset + bpp > self.data.len() {
            return false;
        }

        match self.format {
            PixelFormat::Rgba8888 => {
                self.data[offset] = color.r;
                self.data[offset + 1] = color.g;
                self.data[offset + 2] = color.b;
                self.data[offset + 3] = color.a;
            },
            PixelFormat::Bgra8888 => {
                self.data[offset] = color.b;
                self.data[offset + 1] = color.g;
                self.data[offset + 2] = color.r;
                self.data[offset + 3] = color.a;
            },
            _ => return false,
        }

        if offset + bpp > self.data_len {
            self.data_len = offset + bpp;
        }

        true
    }
}

// =============================================================================
// PROGRESS BAR
// =============================================================================

/// Progress bar style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressBarStyle {
    /// Simple solid bar
    Solid,
    /// Gradient bar
    Gradient,
    /// Striped bar
    Striped,
    /// Animated pulse
    Pulse,
    /// Segmented bar
    Segmented,
}

/// Progress bar configuration
#[derive(Debug, Clone, Copy)]
pub struct ProgressBar {
    /// Position and size
    pub rect: Rect,
    /// Progress (0-100)
    pub progress: u8,
    /// Style
    pub style: ProgressBarStyle,
    /// Foreground color
    pub fg_color: Color,
    /// Background color
    pub bg_color: Color,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: u8,
    /// Show percentage text
    pub show_text: bool,
}

impl ProgressBar {
    /// Create new progress bar
    pub const fn new(rect: Rect) -> Self {
        Self {
            rect,
            progress: 0,
            style: ProgressBarStyle::Solid,
            fg_color: Color::BOOT_HIGHLIGHT,
            bg_color: Color::gray(40),
            border_color: Color::gray(80),
            border_width: 1,
            show_text: true,
        }
    }

    /// Set progress (0-100)
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = progress.min(100);
    }

    /// Get filled width
    pub const fn filled_width(&self) -> u32 {
        (self.rect.width * self.progress as u32) / 100
    }
}

// =============================================================================
// BOOT SPLASH
// =============================================================================

/// Boot splash type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplashType {
    /// Simple logo
    Logo,
    /// Animated spinner
    Spinner,
    /// Progress with logo
    LogoProgress,
    /// Fullscreen animation
    Animation,
}

/// Splash configuration
#[derive(Debug, Clone, Copy)]
pub struct SplashConfig {
    /// Splash type
    pub splash_type: SplashType,
    /// Background color
    pub bg_color: Color,
    /// Logo position (center if None)
    pub logo_pos: Option<Point>,
    /// Progress bar position
    pub progress_pos: Option<Rect>,
    /// Show text under logo
    pub show_text: bool,
    /// Animation frame rate (FPS)
    pub frame_rate: u8,
}

impl SplashConfig {
    /// Create default splash config
    pub const fn new() -> Self {
        Self {
            splash_type: SplashType::LogoProgress,
            bg_color: Color::BOOT_BG,
            logo_pos: None,
            progress_pos: None,
            show_text: true,
            frame_rate: 30,
        }
    }
}

impl Default for SplashConfig {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Graphics error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsError {
    /// Framebuffer not available
    NoFramebuffer,
    /// Invalid resolution
    InvalidResolution,
    /// Invalid pixel format
    InvalidPixelFormat,
    /// Mode not supported
    ModeNotSupported,
    /// Out of memory
    OutOfMemory,
    /// Invalid coordinates
    InvalidCoordinates,
    /// Buffer overflow
    BufferOverflow,
    /// Font not found
    FontNotFound,
    /// Invalid image format
    InvalidImageFormat,
    /// Image too large
    ImageTooLarge,
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphicsError::NoFramebuffer => write!(f, "No framebuffer available"),
            GraphicsError::InvalidResolution => write!(f, "Invalid resolution"),
            GraphicsError::InvalidPixelFormat => write!(f, "Invalid pixel format"),
            GraphicsError::ModeNotSupported => write!(f, "Mode not supported"),
            GraphicsError::OutOfMemory => write!(f, "Out of memory"),
            GraphicsError::InvalidCoordinates => write!(f, "Invalid coordinates"),
            GraphicsError::BufferOverflow => write!(f, "Buffer overflow"),
            GraphicsError::FontNotFound => write!(f, "Font not found"),
            GraphicsError::InvalidImageFormat => write!(f, "Invalid image format"),
            GraphicsError::ImageTooLarge => write!(f, "Image too large"),
        }
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
        assert_eq!(PixelFormat::Bgra8888.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Rgb565.bytes_per_pixel(), 2);
        assert!(PixelFormat::Rgba8888.has_alpha());
        assert!(!PixelFormat::Rgb888.has_alpha());
    }

    #[test]
    fn test_color() {
        let color = Color::rgb(255, 128, 64);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 255);

        let argb = color.to_argb();
        let restored = Color::from_argb(argb);
        assert_eq!(restored.r, color.r);
        assert_eq!(restored.g, color.g);
        assert_eq!(restored.b, color.b);
    }

    #[test]
    fn test_color_rgb565() {
        let color = Color::rgb(255, 128, 64);
        let rgb565 = color.to_rgb565();
        let restored = Color::from_rgb565(rgb565);
        // Note: Some precision loss expected
        assert!(restored.r > 200);
        assert!(restored.g > 100);
    }

    #[test]
    fn test_rect() {
        let rect = Rect::new(10, 20, 100, 50);
        assert_eq!(rect.right(), 110);
        assert_eq!(rect.bottom(), 70);
        assert!(rect.contains(Point::new(50, 40)));
        assert!(!rect.contains(Point::new(5, 40)));
    }

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::new(0, 0, 100, 100);
        let r2 = Rect::new(50, 50, 100, 100);
        let intersection = r1.intersection(&r2);
        assert!(intersection.is_some());
        let i = intersection.unwrap();
        assert_eq!(i.x, 50);
        assert_eq!(i.y, 50);
        assert_eq!(i.width, 50);
        assert_eq!(i.height, 50);
    }

    #[test]
    fn test_display_mode() {
        let mode = DisplayMode::new(0, 1920, 1080, PixelFormat::Bgra8888);
        assert!(mode.is_hd());
        assert!(mode.is_fhd());
        assert!(!mode.is_4k());
        assert_eq!(mode.stride(), 1920 * 4);
    }

    #[test]
    fn test_progress_bar() {
        let mut bar = ProgressBar::new(Rect::new(100, 500, 400, 20));
        bar.set_progress(50);
        assert_eq!(bar.progress, 50);
        assert_eq!(bar.filled_width(), 200);
    }
}
