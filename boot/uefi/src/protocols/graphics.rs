//! Graphics Output Protocol
//!
//! High-level graphics abstraction for framebuffer operations.

use super::{EnumerableProtocol, Protocol};
use crate::error::{Error, Result};
use crate::raw::protocols::gop::*;
use crate::raw::types::*;

extern crate alloc;
use alloc::vec::Vec;

/// Graphics Output Protocol GUID
const GOP_GUID: Guid = guids::GRAPHICS_OUTPUT_PROTOCOL;

// =============================================================================
// GRAPHICS OUTPUT
// =============================================================================

/// High-level graphics output abstraction
pub struct GraphicsOutput {
    /// Raw protocol pointer
    protocol: *mut EfiGraphicsOutputProtocol,
    /// Handle
    _handle: Handle,
}

impl GraphicsOutput {
    /// Create from raw protocol
    ///
    /// # Safety
    /// Protocol pointer must be valid
    pub unsafe fn from_raw(protocol: *mut EfiGraphicsOutputProtocol, handle: Handle) -> Self {
        Self {
            protocol,
            _handle: handle,
        }
    }

    /// Get current mode info
    pub fn mode_info(&self) -> Result<ModeInfo> {
        let mode = unsafe { &*(*self.protocol).mode };
        let info = unsafe { &*mode.info };

        Ok(ModeInfo {
            width: info.horizontal_resolution,
            height: info.vertical_resolution,
            pixel_format: PixelFormat::from_raw(info.pixel_format),
            pixels_per_scan_line: info.pixels_per_scan_line,
            mode_number: mode.mode,
            max_mode: mode.max_mode,
        })
    }

    /// Get framebuffer
    pub fn framebuffer(&self) -> Result<Framebuffer> {
        let mode = unsafe { &*(*self.protocol).mode };
        let info = unsafe { &*mode.info };

        Ok(Framebuffer {
            base: mode.frame_buffer_base,
            size: mode.frame_buffer_size,
            width: info.horizontal_resolution,
            height: info.vertical_resolution,
            stride: info.pixels_per_scan_line,
            format: PixelFormat::from_raw(info.pixel_format),
        })
    }

    /// Query specific mode
    pub fn query_mode(&self, mode: u32) -> Result<ModeInfo> {
        let mut size = 0usize;
        let mut info: *mut EfiGraphicsOutputModeInformation = core::ptr::null_mut();

        let result =
            unsafe { ((*self.protocol).query_mode)(self.protocol, mode, &mut size, &mut info) };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        let info_ref = unsafe { &*info };

        Ok(ModeInfo {
            width: info_ref.horizontal_resolution,
            height: info_ref.vertical_resolution,
            pixel_format: PixelFormat::from_raw(info_ref.pixel_format),
            pixels_per_scan_line: info_ref.pixels_per_scan_line,
            mode_number: mode,
            max_mode: unsafe { (*(*self.protocol).mode).max_mode },
        })
    }

    /// Get all available modes
    pub fn available_modes(&self) -> Result<Vec<ModeInfo>> {
        let mode = unsafe { &*(*self.protocol).mode };
        let max = mode.max_mode;

        let mut modes = Vec::with_capacity(max as usize);
        for i in 0..max {
            if let Ok(info) = self.query_mode(i) {
                modes.push(info);
            }
        }

        Ok(modes)
    }

    /// Set mode
    pub fn set_mode(&self, mode: u32) -> Result<()> {
        let result = unsafe { ((*self.protocol).set_mode)(self.protocol, mode) };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Set best mode (highest resolution)
    pub fn set_best_mode(&self) -> Result<ModeInfo> {
        let modes = self.available_modes()?;

        let best = modes
            .iter()
            .max_by_key(|m| (m.width as u64) * (m.height as u64))
            .cloned()
            .ok_or(Error::NotFound)?;

        self.set_mode(best.mode_number)?;
        Ok(best)
    }

    /// Find mode by resolution
    pub fn find_mode(&self, width: u32, height: u32) -> Result<u32> {
        let modes = self.available_modes()?;

        modes
            .iter()
            .find(|m| m.width == width && m.height == height)
            .map(|m| m.mode_number)
            .ok_or(Error::NotFound)
    }

    /// Set mode by resolution
    pub fn set_resolution(&self, width: u32, height: u32) -> Result<()> {
        let mode = self.find_mode(width, height)?;
        self.set_mode(mode)
    }

    // =========================================================================
    // DRAWING OPERATIONS
    // =========================================================================

    /// Fill rectangle with color
    pub fn fill_rect(&self, x: u32, y: u32, width: u32, height: u32, color: Pixel) -> Result<()> {
        // Use BLT to fill
        let blt_pixel = EfiGraphicsOutputBltPixel {
            blue: color.b,
            green: color.g,
            red: color.r,
            reserved: 0,
        };

        let result = unsafe {
            ((*self.protocol).blt)(
                self.protocol,
                &blt_pixel as *const _ as *mut _,
                EfiGraphicsOutputBltOperation::BltVideoFill,
                0,
                0,
                x as usize,
                y as usize,
                width as usize,
                height as usize,
                0,
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Clear screen with color
    pub fn clear(&self, color: Pixel) -> Result<()> {
        let mode = self.mode_info()?;
        self.fill_rect(0, 0, mode.width, mode.height, color)
    }

    /// Draw single pixel
    pub fn draw_pixel(&self, x: u32, y: u32, color: Pixel) -> Result<()> {
        self.fill_rect(x, y, 1, 1, color)
    }

    /// Draw horizontal line
    pub fn draw_hline(&self, x: u32, y: u32, length: u32, color: Pixel) -> Result<()> {
        self.fill_rect(x, y, length, 1, color)
    }

    /// Draw vertical line
    pub fn draw_vline(&self, x: u32, y: u32, length: u32, color: Pixel) -> Result<()> {
        self.fill_rect(x, y, 1, length, color)
    }

    /// Draw rectangle outline
    pub fn draw_rect(&self, x: u32, y: u32, width: u32, height: u32, color: Pixel) -> Result<()> {
        self.draw_hline(x, y, width, color)?;
        self.draw_hline(x, y + height - 1, width, color)?;
        self.draw_vline(x, y, height, color)?;
        self.draw_vline(x + width - 1, y, height, color)?;
        Ok(())
    }

    /// Draw line (Bresenham's algorithm)
    pub fn draw_line(&self, x0: i32, y0: i32, x1: i32, y1: i32, color: Pixel) -> Result<()> {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx: i32 = if x0 < x1 { 1 } else { -1 };
        let sy: i32 = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            if x >= 0 && y >= 0 {
                self.draw_pixel(x as u32, y as u32, color)?;
            }

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                if x == x1 {
                    break;
                }
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                if y == y1 {
                    break;
                }
                err += dx;
                y += sy;
            }
        }

        Ok(())
    }

    /// Draw circle outline (Midpoint algorithm)
    pub fn draw_circle(&self, cx: i32, cy: i32, radius: i32, color: Pixel) -> Result<()> {
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.draw_pixel((cx + x) as u32, (cy + y) as u32, color)?;
            self.draw_pixel((cx + y) as u32, (cy + x) as u32, color)?;
            self.draw_pixel((cx - y) as u32, (cy + x) as u32, color)?;
            self.draw_pixel((cx - x) as u32, (cy + y) as u32, color)?;
            self.draw_pixel((cx - x) as u32, (cy - y) as u32, color)?;
            self.draw_pixel((cx - y) as u32, (cy - x) as u32, color)?;
            self.draw_pixel((cx + y) as u32, (cy - x) as u32, color)?;
            self.draw_pixel((cx + x) as u32, (cy - y) as u32, color)?;

            if err <= 0 {
                y += 1;
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }

        Ok(())
    }

    /// Integer square root approximation
    fn isqrt(n: i32) -> i32 {
        if n <= 0 {
            return 0;
        }
        let mut x = n;
        let mut y = (x + 1) / 2;
        while y < x {
            x = y;
            y = (x + n / x) / 2;
        }
        x
    }

    /// Fill circle
    pub fn fill_circle(&self, cx: i32, cy: i32, radius: i32, color: Pixel) -> Result<()> {
        for y in -radius..=radius {
            let half_width = Self::isqrt(radius * radius - y * y);
            let start_x = cx - half_width;
            let py = cy + y;
            if py >= 0 {
                self.draw_hline(start_x as u32, py as u32, (half_width * 2) as u32, color)?;
            }
        }
        Ok(())
    }

    // =========================================================================
    // BLT OPERATIONS
    // =========================================================================

    /// Copy buffer to screen
    pub fn blt_buffer_to_video(
        &self,
        buffer: &[Pixel],
        src_x: usize,
        src_y: usize,
        dest_x: usize,
        dest_y: usize,
        width: usize,
        height: usize,
        delta: usize,
    ) -> Result<()> {
        let result = unsafe {
            ((*self.protocol).blt)(
                self.protocol,
                buffer.as_ptr() as *mut EfiGraphicsOutputBltPixel,
                EfiGraphicsOutputBltOperation::BltBufferToVideo,
                src_x,
                src_y,
                dest_x,
                dest_y,
                width,
                height,
                delta,
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Copy screen to buffer
    pub fn blt_video_to_buffer(
        &self,
        buffer: &mut [Pixel],
        src_x: usize,
        src_y: usize,
        dest_x: usize,
        dest_y: usize,
        width: usize,
        height: usize,
        delta: usize,
    ) -> Result<()> {
        let result = unsafe {
            ((*self.protocol).blt)(
                self.protocol,
                buffer.as_mut_ptr() as *mut EfiGraphicsOutputBltPixel,
                EfiGraphicsOutputBltOperation::BltVideoToBltBuffer,
                src_x,
                src_y,
                dest_x,
                dest_y,
                width,
                height,
                delta,
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Copy screen region to another location
    pub fn blt_video_to_video(
        &self,
        src_x: usize,
        src_y: usize,
        dest_x: usize,
        dest_y: usize,
        width: usize,
        height: usize,
    ) -> Result<()> {
        let result = unsafe {
            ((*self.protocol).blt)(
                self.protocol,
                core::ptr::null_mut(),
                EfiGraphicsOutputBltOperation::BltVideoToVideo,
                src_x,
                src_y,
                dest_x,
                dest_y,
                width,
                height,
                0,
            )
        };

        if result == Status::SUCCESS {
            Ok(())
        } else {
            Err(Error::from_status(result))
        }
    }

    /// Scroll screen up
    pub fn scroll_up(&self, lines: u32, fill_color: Pixel) -> Result<()> {
        let mode = self.mode_info()?;
        let h = mode.height;
        let w = mode.width;

        if lines >= h {
            return self.clear(fill_color);
        }

        // Copy region up
        self.blt_video_to_video(0, lines as usize, 0, 0, w as usize, (h - lines) as usize)?;

        // Fill bottom
        self.fill_rect(0, h - lines, w, lines, fill_color)?;

        Ok(())
    }

    // =========================================================================
    // IMAGE OPERATIONS
    // =========================================================================

    /// Draw image from pixel buffer
    pub fn draw_image(
        &self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        pixels: &[Pixel],
    ) -> Result<()> {
        if pixels.len() < (width * height) as usize {
            return Err(Error::InvalidParameter);
        }

        self.blt_buffer_to_video(
            pixels,
            0,
            0,
            x as usize,
            y as usize,
            width as usize,
            height as usize,
            width as usize * core::mem::size_of::<Pixel>(),
        )
    }

    /// Capture screen region to buffer
    pub fn capture_region(&self, x: u32, y: u32, width: u32, height: u32) -> Result<Vec<Pixel>> {
        let mut buffer = alloc::vec![Pixel::black(); (width * height) as usize];

        self.blt_video_to_buffer(
            &mut buffer,
            x as usize,
            y as usize,
            0,
            0,
            width as usize,
            height as usize,
            width as usize * core::mem::size_of::<Pixel>(),
        )?;

        Ok(buffer)
    }

    /// Take screenshot
    pub fn screenshot(&self) -> Result<Vec<Pixel>> {
        let mode = self.mode_info()?;
        self.capture_region(0, 0, mode.width, mode.height)
    }
}

impl Protocol for GraphicsOutput {
    const GUID: Guid = GOP_GUID;

    fn open(handle: Handle) -> Result<Self> {
        use crate::services::boot_services;

        let bs = unsafe { boot_services() };
        let image = crate::services::image_handle().ok_or(Error::NotReady)?;

        let mut protocol: *mut core::ffi::c_void = core::ptr::null_mut();
        let result = unsafe {
            ((*bs).open_protocol)(
                handle,
                &Self::GUID as *const Guid,
                &mut protocol,
                image,
                Handle(core::ptr::null_mut()),
                0x00000002, // BY_HANDLE_PROTOCOL
            )
        };

        if result != Status::SUCCESS {
            return Err(Error::from_status(result));
        }

        Ok(unsafe { Self::from_raw(protocol as *mut EfiGraphicsOutputProtocol, handle) })
    }
}

impl EnumerableProtocol for GraphicsOutput {
    fn enumerate() -> Result<Vec<Self>> {
        super::ProtocolLocator::locate_all::<Self>()
            .map(|handles| handles.into_iter().map(|h| h.leak()).collect())
    }
}

// =============================================================================
// MODE INFO
// =============================================================================

/// Graphics mode information
#[derive(Debug, Clone)]
pub struct ModeInfo {
    /// Horizontal resolution in pixels
    pub width: u32,
    /// Vertical resolution in pixels
    pub height: u32,
    /// Pixel format
    pub pixel_format: PixelFormat,
    /// Pixels per scan line
    pub pixels_per_scan_line: u32,
    /// Mode number
    pub mode_number: u32,
    /// Maximum mode number
    pub max_mode: u32,
}

impl ModeInfo {
    /// Get resolution as tuple
    pub fn resolution(&self) -> Resolution {
        Resolution {
            width: self.width,
            height: self.height,
        }
    }

    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Calculate bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        match self.pixel_format {
            PixelFormat::Rgb | PixelFormat::Bgr | PixelFormat::Bitmask => 4,
            PixelFormat::BltOnly => 0,
        }
    }

    /// Calculate bytes per scan line
    pub fn bytes_per_scan_line(&self) -> usize {
        self.pixels_per_scan_line as usize * self.bytes_per_pixel()
    }

    /// Calculate total framebuffer size
    pub fn framebuffer_size(&self) -> usize {
        self.bytes_per_scan_line() * self.height as usize
    }
}

// =============================================================================
// RESOLUTION
// =============================================================================

/// Screen resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl Resolution {
    /// Common resolutions
    pub const VGA: Self = Self {
        width: 640,
        height: 480,
    };
    pub const SVGA: Self = Self {
        width: 800,
        height: 600,
    };
    pub const XGA: Self = Self {
        width: 1024,
        height: 768,
    };
    pub const HD: Self = Self {
        width: 1280,
        height: 720,
    };
    pub const WXGA: Self = Self {
        width: 1366,
        height: 768,
    };
    pub const SXGA: Self = Self {
        width: 1280,
        height: 1024,
    };
    pub const FULL_HD: Self = Self {
        width: 1920,
        height: 1080,
    };
    pub const QHD: Self = Self {
        width: 2560,
        height: 1440,
    };
    pub const UHD: Self = Self {
        width: 3840,
        height: 2160,
    };

    /// Get pixel count
    pub fn pixels(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Check if 16:9
    pub fn is_16_9(&self) -> bool {
        let ratio = self.aspect_ratio();
        (ratio - 16.0 / 9.0).abs() < 0.01
    }

    /// Check if 16:10
    pub fn is_16_10(&self) -> bool {
        let ratio = self.aspect_ratio();
        (ratio - 16.0 / 10.0).abs() < 0.01
    }

    /// Check if 4:3
    pub fn is_4_3(&self) -> bool {
        let ratio = self.aspect_ratio();
        (ratio - 4.0 / 3.0).abs() < 0.01
    }
}

impl core::fmt::Display for Resolution {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

// =============================================================================
// PIXEL FORMAT
// =============================================================================

/// Pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// RGBX (32-bit)
    Rgb,
    /// BGRX (32-bit)
    Bgr,
    /// Custom bitmask
    Bitmask,
    /// BLT only (no framebuffer access)
    BltOnly,
}

impl PixelFormat {
    /// Create from raw UEFI pixel format
    pub fn from_raw(raw: EfiGraphicsPixelFormat) -> Self {
        match raw {
            EfiGraphicsPixelFormat::PixelRedGreenBlueReserved8BitPerColor => Self::Rgb,
            EfiGraphicsPixelFormat::PixelBlueGreenRedReserved8BitPerColor => Self::Bgr,
            EfiGraphicsPixelFormat::PixelBitMask => Self::Bitmask,
            EfiGraphicsPixelFormat::PixelBltOnly => Self::BltOnly,
        }
    }

    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::Rgb | Self::Bgr | Self::Bitmask => 4,
            Self::BltOnly => 0,
        }
    }

    /// Check if framebuffer is accessible
    pub fn has_framebuffer(&self) -> bool {
        !matches!(self, Self::BltOnly)
    }
}

// =============================================================================
// PIXEL
// =============================================================================

/// 32-bit BGRA pixel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C)]
pub struct Pixel {
    /// Blue component
    pub b: u8,
    /// Green component
    pub g: u8,
    /// Red component
    pub r: u8,
    /// Reserved/Alpha
    pub a: u8,
}

impl Pixel {
    /// Create from RGB values
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 0 }
    }

    /// Create from RGBA values
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create from 32-bit ARGB value
    pub const fn from_argb32(argb: u32) -> Self {
        Self {
            a: ((argb >> 24) & 0xFF) as u8,
            r: ((argb >> 16) & 0xFF) as u8,
            g: ((argb >> 8) & 0xFF) as u8,
            b: (argb & 0xFF) as u8,
        }
    }

    /// Convert to 32-bit ARGB
    pub const fn to_argb32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// Create grayscale
    pub const fn gray(value: u8) -> Self {
        Self::rgb(value, value, value)
    }

    // Common colors
    pub const fn black() -> Self {
        Self::rgb(0, 0, 0)
    }
    pub const fn white() -> Self {
        Self::rgb(255, 255, 255)
    }
    pub const fn red() -> Self {
        Self::rgb(255, 0, 0)
    }
    pub const fn green() -> Self {
        Self::rgb(0, 255, 0)
    }
    pub const fn blue() -> Self {
        Self::rgb(0, 0, 255)
    }
    pub const fn cyan() -> Self {
        Self::rgb(0, 255, 255)
    }
    pub const fn magenta() -> Self {
        Self::rgb(255, 0, 255)
    }
    pub const fn yellow() -> Self {
        Self::rgb(255, 255, 0)
    }
    pub const fn orange() -> Self {
        Self::rgb(255, 165, 0)
    }
    pub const fn pink() -> Self {
        Self::rgb(255, 192, 203)
    }
    pub const fn purple() -> Self {
        Self::rgb(128, 0, 128)
    }
    pub const fn brown() -> Self {
        Self::rgb(139, 69, 19)
    }

    /// Blend two pixels
    pub fn blend(&self, other: &Self, alpha: u8) -> Self {
        let a = alpha as u16;
        let inv_a = 255 - a;

        Self {
            r: ((self.r as u16 * inv_a + other.r as u16 * a) / 255) as u8,
            g: ((self.g as u16 * inv_a + other.g as u16 * a) / 255) as u8,
            b: ((self.b as u16 * inv_a + other.b as u16 * a) / 255) as u8,
            a: ((self.a as u16 * inv_a + other.a as u16 * a) / 255) as u8,
        }
    }

    /// Lighten color
    pub fn lighten(&self, amount: u8) -> Self {
        Self {
            r: self.r.saturating_add(amount),
            g: self.g.saturating_add(amount),
            b: self.b.saturating_add(amount),
            a: self.a,
        }
    }

    /// Darken color
    pub fn darken(&self, amount: u8) -> Self {
        Self {
            r: self.r.saturating_sub(amount),
            g: self.g.saturating_sub(amount),
            b: self.b.saturating_sub(amount),
            a: self.a,
        }
    }

    /// Invert color
    pub fn invert(&self) -> Self {
        Self {
            r: 255 - self.r,
            g: 255 - self.g,
            b: 255 - self.b,
            a: self.a,
        }
    }

    /// Convert to grayscale
    pub fn to_grayscale(&self) -> Self {
        let gray = (self.r as u16 * 30 + self.g as u16 * 59 + self.b as u16 * 11) / 100;
        Self::gray(gray as u8)
    }
}

// =============================================================================
// FRAMEBUFFER
// =============================================================================

/// Framebuffer information and access
#[derive(Debug, Clone)]
pub struct Framebuffer {
    /// Physical base address
    pub base: PhysicalAddress,
    /// Size in bytes
    pub size: usize,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Stride (pixels per scan line)
    pub stride: u32,
    /// Pixel format
    pub format: PixelFormat,
}

impl Framebuffer {
    /// Get raw pointer to framebuffer
    ///
    /// # Safety
    /// Returned pointer must only be used while framebuffer is valid
    pub unsafe fn as_ptr(&self) -> *mut u8 {
        self.base.0 as *mut u8
    }

    /// Get mutable slice view of framebuffer
    ///
    /// # Safety
    /// Must ensure exclusive access
    pub unsafe fn as_mut_slice(&self) -> &mut [u8] {
        core::slice::from_raw_parts_mut(self.as_ptr(), self.size)
    }

    /// Get pixel slice (assumes 32-bit pixels)
    ///
    /// # Safety
    /// Must ensure exclusive access
    pub unsafe fn pixels_mut(&self) -> &mut [Pixel] {
        core::slice::from_raw_parts_mut(
            self.as_ptr() as *mut Pixel,
            self.size / core::mem::size_of::<Pixel>(),
        )
    }

    /// Calculate offset for pixel coordinates
    pub fn pixel_offset(&self, x: u32, y: u32) -> usize {
        ((y * self.stride + x) as usize) * self.format.bytes_per_pixel()
    }

    /// Write pixel directly (fast, no bounds check)
    ///
    /// # Safety
    /// Coordinates must be valid
    pub unsafe fn write_pixel_unchecked(&self, x: u32, y: u32, pixel: Pixel) {
        let offset = self.pixel_offset(x, y);
        let ptr = self.as_ptr().add(offset) as *mut Pixel;
        core::ptr::write_volatile(ptr, pixel);
    }

    /// Write pixel with bounds check
    pub fn write_pixel(&self, x: u32, y: u32, pixel: Pixel) -> bool {
        if x < self.width && y < self.height {
            unsafe { self.write_pixel_unchecked(x, y, pixel) };
            true
        } else {
            false
        }
    }

    /// Read pixel
    pub fn read_pixel(&self, x: u32, y: u32) -> Option<Pixel> {
        if x < self.width && y < self.height {
            let offset = self.pixel_offset(x, y);
            unsafe {
                let ptr = self.as_ptr().add(offset) as *const Pixel;
                Some(core::ptr::read_volatile(ptr))
            }
        } else {
            None
        }
    }

    /// Fill entire framebuffer
    pub fn fill(&self, pixel: Pixel) {
        unsafe {
            let pixels = self.pixels_mut();
            for p in pixels.iter_mut() {
                *p = pixel;
            }
        }
    }

    /// Fill rectangle
    pub fn fill_rect(&self, x: u32, y: u32, width: u32, height: u32, pixel: Pixel) {
        for py in y..(y + height).min(self.height) {
            for px in x..(x + width).min(self.width) {
                self.write_pixel(px, py, pixel);
            }
        }
    }

    /// Get resolution
    pub fn resolution(&self) -> Resolution {
        Resolution {
            width: self.width,
            height: self.height,
        }
    }

    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        self.format.bytes_per_pixel()
    }

    /// Get bytes per row
    pub fn bytes_per_row(&self) -> usize {
        self.stride as usize * self.bytes_per_pixel()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_creation() {
        let p = Pixel::rgb(255, 128, 64);
        assert_eq!(p.r, 255);
        assert_eq!(p.g, 128);
        assert_eq!(p.b, 64);
    }

    #[test]
    fn test_pixel_blend() {
        let black = Pixel::black();
        let white = Pixel::white();
        let gray = black.blend(&white, 128);
        assert!(gray.r > 100 && gray.r < 150);
    }

    #[test]
    fn test_pixel_invert() {
        let black = Pixel::black();
        let inverted = black.invert();
        assert_eq!(inverted, Pixel::white());
    }

    #[test]
    fn test_resolution() {
        assert!(Resolution::FULL_HD.is_16_9());
        assert!(Resolution::XGA.is_4_3());
    }
}
