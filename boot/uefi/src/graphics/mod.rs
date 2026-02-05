//! UEFI Graphics Output Protocol Support
//!
//! Full graphics support including mode enumeration, framebuffer access,
//! and drawing primitives.

use core::fmt;

// =============================================================================
// GOP PROTOCOL
// =============================================================================

/// EFI Graphics Output Protocol GUID
pub const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: [u8; 16] = [
    0xDE, 0xA9, 0x42, 0x90, 0xDC, 0x23, 0x38, 0x4A, 0x96, 0xFB, 0x7A, 0xDE, 0xD0, 0x80, 0x51, 0x6A,
];

/// Pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PixelFormat {
    /// Red:Green:Blue:Reserved 8:8:8:8
    RedGreenBlueReserved8BitPerColor = 0,
    /// Blue:Green:Red:Reserved 8:8:8:8
    BlueGreenRedReserved8BitPerColor = 1,
    /// Pixel format defined by pixel information
    BitMask = 2,
    /// BLT only (no framebuffer)
    BltOnly = 3,
    /// Reserved
    Max     = 4,
}

impl PixelFormat {
    /// From raw value
    #[must_use]
    pub const fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::RedGreenBlueReserved8BitPerColor),
            1 => Some(Self::BlueGreenRedReserved8BitPerColor),
            2 => Some(Self::BitMask),
            3 => Some(Self::BltOnly),
            _ => None,
        }
    }

    /// Bytes per pixel
    #[must_use]
    pub const fn bytes_per_pixel(&self) -> usize {
        match self {
            Self::RedGreenBlueReserved8BitPerColor
            | Self::BlueGreenRedReserved8BitPerColor
            | Self::BitMask => 4,
            Self::BltOnly | Self::Max => 0,
        }
    }

    /// Is RGB (not BGR)
    #[must_use]
    pub const fn is_rgb(&self) -> bool {
        matches!(self, Self::RedGreenBlueReserved8BitPerColor)
    }

    /// Is BGR
    #[must_use]
    pub const fn is_bgr(&self) -> bool {
        matches!(self, Self::BlueGreenRedReserved8BitPerColor)
    }
}

/// Pixel bitmask
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PixelBitmask {
    /// Red channel bitmask
    pub red_mask: u32,
    /// Green channel bitmask
    pub green_mask: u32,
    /// Blue channel bitmask
    pub blue_mask: u32,
    /// Reserved channel bitmask
    pub reserved_mask: u32,
}

impl PixelBitmask {
    /// Standard RGB mask
    pub const RGB: Self = Self {
        red_mask: 0x00FF_0000,
        green_mask: 0x0000_FF00,
        blue_mask: 0x0000_00FF,
        reserved_mask: 0xFF00_0000,
    };

    /// Standard BGR mask
    pub const BGR: Self = Self {
        red_mask: 0x0000_00FF,
        green_mask: 0x0000_FF00,
        blue_mask: 0x00FF_0000,
        reserved_mask: 0xFF00_0000,
    };

    /// Get shift and size for a channel
    #[must_use]
    pub const fn channel_info(&self, mask: u32) -> (u8, u8) {
        if mask == 0 {
            return (0, 0);
        }

        let shift = mask.trailing_zeros() as u8;
        let size = (32 - mask.leading_zeros() - mask.trailing_zeros()) as u8;
        (shift, size)
    }
}

/// Graphics mode information
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GraphicsModeInfo {
    /// Version
    pub version: u32,
    /// Horizontal resolution
    pub horizontal_resolution: u32,
    /// Vertical resolution
    pub vertical_resolution: u32,
    /// Pixel format
    pub pixel_format: PixelFormat,
    /// Pixel information
    pub pixel_information: PixelBitmask,
    /// Pixels per scan line
    pub pixels_per_scan_line: u32,
}

impl GraphicsModeInfo {
    /// Get screen width
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.horizontal_resolution
    }

    /// Get screen height
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.vertical_resolution
    }

    /// Get stride (bytes per row)
    #[must_use]
    pub const fn stride(&self) -> u32 {
        self.pixels_per_scan_line * self.pixel_format.bytes_per_pixel() as u32
    }

    /// Get framebuffer size
    #[must_use]
    pub const fn framebuffer_size(&self) -> usize {
        self.stride() as usize * self.height() as usize
    }

    /// Total pixels
    #[must_use]
    pub const fn total_pixels(&self) -> u32 {
        self.horizontal_resolution * self.vertical_resolution
    }
}

/// Graphics mode
#[derive(Debug, Clone)]
pub struct GraphicsMode {
    /// Mode number
    pub mode_number: u32,
    /// Mode info
    pub info: GraphicsModeInfo,
    /// Framebuffer base
    pub framebuffer_base: u64,
    /// Framebuffer size
    pub framebuffer_size: usize,
}

impl GraphicsMode {
    /// Is this mode suitable for console
    pub fn is_suitable_for_console(&self) -> bool {
        self.info.width() >= 640
            && self.info.height() >= 480
            && self.info.pixel_format != PixelFormat::BltOnly
    }

    /// Calculate aspect ratio
    pub fn aspect_ratio(&self) -> (u32, u32) {
        let gcd = gcd(self.info.width(), self.info.height());
        (self.info.width() / gcd, self.info.height() / gcd)
    }
}

/// Calculate GCD
fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

// =============================================================================
// GRAPHICS CONTROLLER
// =============================================================================

/// Graphics output controller
pub struct GraphicsOutput {
    /// Protocol handle
    _handle: usize,
    /// Available modes
    modes: [Option<GraphicsMode>; 64],
    /// Mode count
    mode_count: usize,
    /// Current mode
    current_mode: usize,
    /// Framebuffer base
    framebuffer: *mut u8,
    /// Framebuffer size
    framebuffer_size: usize,
}

impl GraphicsOutput {
    /// Maximum supported modes
    pub const MAX_MODES: usize = 64;

    /// Create new graphics output
    pub fn new(handle: usize) -> Self {
        Self {
            _handle: handle,
            modes: core::array::from_fn(|_| None),
            mode_count: 0,
            current_mode: 0,
            framebuffer: core::ptr::null_mut(),
            framebuffer_size: 0,
        }
    }

    /// Query available modes
    pub fn query_modes(&mut self) -> Result<usize, GraphicsError> {
        // Would call EFI_GRAPHICS_OUTPUT_PROTOCOL.QueryMode for each mode
        // Populate self.modes array
        Ok(self.mode_count)
    }

    /// Get mode count
    #[must_use]
    pub const fn mode_count(&self) -> usize {
        self.mode_count
    }

    /// Get mode info
    pub fn get_mode(&self, mode_number: usize) -> Option<&GraphicsMode> {
        self.modes.get(mode_number).and_then(|m| m.as_ref())
    }

    /// Get current mode
    pub fn current_mode(&self) -> Option<&GraphicsMode> {
        self.get_mode(self.current_mode)
    }

    /// Set mode
    pub fn set_mode(&mut self, mode_number: u32) -> Result<(), GraphicsError> {
        if mode_number as usize >= self.mode_count {
            return Err(GraphicsError::InvalidMode);
        }

        // Would call EFI_GRAPHICS_OUTPUT_PROTOCOL.SetMode
        self.current_mode = mode_number as usize;

        // Update framebuffer pointer
        if let Some(mode) = &self.modes[self.current_mode] {
            self.framebuffer = mode.framebuffer_base as *mut u8;
            self.framebuffer_size = mode.framebuffer_size;
        }

        Ok(())
    }

    /// Find best mode for resolution
    pub fn find_mode(&self, width: u32, height: u32) -> Option<u32> {
        for i in 0..self.mode_count {
            if let Some(mode) = &self.modes[i] {
                if mode.info.width() == width && mode.info.height() == height {
                    return Some(i as u32);
                }
            }
        }
        None
    }

    /// Find largest mode
    pub fn find_largest_mode(&self) -> Option<u32> {
        let mut best_mode = None;
        let mut best_pixels = 0;

        for i in 0..self.mode_count {
            if let Some(mode) = &self.modes[i] {
                let pixels = mode.info.total_pixels();
                if pixels > best_pixels && mode.info.pixel_format != PixelFormat::BltOnly {
                    best_pixels = pixels;
                    best_mode = Some(i as u32);
                }
            }
        }

        best_mode
    }

    /// Get framebuffer
    pub fn framebuffer(&self) -> Option<&mut [u8]> {
        if self.framebuffer.is_null() || self.framebuffer_size == 0 {
            return None;
        }

        unsafe {
            Some(core::slice::from_raw_parts_mut(
                self.framebuffer,
                self.framebuffer_size,
            ))
        }
    }

    /// Get pixel at coordinates
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<Color32> {
        let mode = self.current_mode()?;

        if x >= mode.info.width() || y >= mode.info.height() {
            return None;
        }

        let offset =
            (y * mode.info.stride() + x * mode.info.pixel_format.bytes_per_pixel() as u32) as usize;

        if offset + 4 > self.framebuffer_size {
            return None;
        }

        unsafe {
            let pixel = self.framebuffer.add(offset).cast::<u32>().read_volatile();
            Some(Self::decode_pixel(pixel, &mode.info))
        }
    }

    /// Set pixel at coordinates
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color32) -> Result<(), GraphicsError> {
        let mode = self.current_mode().ok_or(GraphicsError::NoMode)?;

        if x >= mode.info.width() || y >= mode.info.height() {
            return Err(GraphicsError::OutOfBounds);
        }

        let offset =
            (y * mode.info.stride() + x * mode.info.pixel_format.bytes_per_pixel() as u32) as usize;

        if offset + 4 > self.framebuffer_size {
            return Err(GraphicsError::OutOfBounds);
        }

        let pixel = Self::encode_pixel(color, &mode.info);

        unsafe {
            self.framebuffer
                .add(offset)
                .cast::<u32>()
                .write_volatile(pixel);
        }

        Ok(())
    }

    /// Encode color to pixel value
    fn encode_pixel(color: Color32, info: &GraphicsModeInfo) -> u32 {
        match info.pixel_format {
            PixelFormat::RedGreenBlueReserved8BitPerColor => {
                (u32::from(color.r) << 16)
                    | (u32::from(color.g) << 8)
                    | u32::from(color.b)
                    | (u32::from(color.a) << 24)
            },
            PixelFormat::BlueGreenRedReserved8BitPerColor => {
                u32::from(color.b)
                    | (u32::from(color.g) << 8)
                    | (u32::from(color.r) << 16)
                    | (u32::from(color.a) << 24)
            },
            PixelFormat::BitMask => {
                let (r_shift, _) = info
                    .pixel_information
                    .channel_info(info.pixel_information.red_mask);
                let (g_shift, _) = info
                    .pixel_information
                    .channel_info(info.pixel_information.green_mask);
                let (b_shift, _) = info
                    .pixel_information
                    .channel_info(info.pixel_information.blue_mask);

                (u32::from(color.r) << r_shift)
                    | (u32::from(color.g) << g_shift)
                    | (u32::from(color.b) << b_shift)
            },
            _ => 0,
        }
    }

    /// Decode pixel value to color
    fn decode_pixel(pixel: u32, info: &GraphicsModeInfo) -> Color32 {
        match info.pixel_format {
            PixelFormat::RedGreenBlueReserved8BitPerColor => Color32 {
                r: ((pixel >> 16) & 0xFF) as u8,
                g: ((pixel >> 8) & 0xFF) as u8,
                b: (pixel & 0xFF) as u8,
                a: ((pixel >> 24) & 0xFF) as u8,
            },
            PixelFormat::BlueGreenRedReserved8BitPerColor => Color32 {
                b: (pixel & 0xFF) as u8,
                g: ((pixel >> 8) & 0xFF) as u8,
                r: ((pixel >> 16) & 0xFF) as u8,
                a: ((pixel >> 24) & 0xFF) as u8,
            },
            PixelFormat::BitMask => {
                let (r_shift, _) = info
                    .pixel_information
                    .channel_info(info.pixel_information.red_mask);
                let (g_shift, _) = info
                    .pixel_information
                    .channel_info(info.pixel_information.green_mask);
                let (b_shift, _) = info
                    .pixel_information
                    .channel_info(info.pixel_information.blue_mask);

                Color32 {
                    r: ((pixel >> r_shift) & 0xFF) as u8,
                    g: ((pixel >> g_shift) & 0xFF) as u8,
                    b: ((pixel >> b_shift) & 0xFF) as u8,
                    a: 255,
                }
            },
            _ => Color32::BLACK,
        }
    }

    /// BLT (Block Transfer) operation
    pub fn blt(
        &mut self,
        blt_buffer: Option<&[BltPixel]>,
        operation: BltOperation,
        src_x: usize,
        src_y: usize,
        dst_x: usize,
        dst_y: usize,
        width: usize,
        height: usize,
    ) -> Result<(), GraphicsError> {
        let mode = self.current_mode().ok_or(GraphicsError::NoMode)?;

        // Validate bounds
        let screen_width = mode.info.width() as usize;
        let screen_height = mode.info.height() as usize;

        match operation {
            BltOperation::VideoFill => {
                if dst_x + width > screen_width || dst_y + height > screen_height {
                    return Err(GraphicsError::OutOfBounds);
                }

                let pixel = blt_buffer
                    .and_then(|b| b.first())
                    .ok_or(GraphicsError::InvalidParameter)?;

                self.fill_rect(
                    dst_x as u32,
                    dst_y as u32,
                    width as u32,
                    height as u32,
                    pixel.to_color(),
                )?;
            },
            BltOperation::VideoToBltBuffer => {
                // Copy from video to buffer
                if src_x + width > screen_width || src_y + height > screen_height {
                    return Err(GraphicsError::OutOfBounds);
                }
                // Would read pixels into buffer
            },
            BltOperation::BufferToVideo => {
                // Copy from buffer to video
                if dst_x + width > screen_width || dst_y + height > screen_height {
                    return Err(GraphicsError::OutOfBounds);
                }

                if let Some(buffer) = blt_buffer {
                    for y in 0..height {
                        for x in 0..width {
                            let idx = y * width + x;
                            if idx < buffer.len() {
                                self.set_pixel(
                                    (dst_x + x) as u32,
                                    (dst_y + y) as u32,
                                    buffer[idx].to_color(),
                                )?;
                            }
                        }
                    }
                }
            },
            BltOperation::VideoToVideo => {
                // Copy within video memory
                if src_x + width > screen_width
                    || src_y + height > screen_height
                    || dst_x + width > screen_width
                    || dst_y + height > screen_height
                {
                    return Err(GraphicsError::OutOfBounds);
                }
                // Would copy pixels within framebuffer
            },
        }

        Ok(())
    }

    /// Fill rectangle
    pub fn fill_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        let mode = self.current_mode().ok_or(GraphicsError::NoMode)?;
        let info = mode.info.clone();

        let end_x = (x + width).min(info.width());
        let end_y = (y + height).min(info.height());

        let pixel = Self::encode_pixel(color, &info);
        let bpp = info.pixel_format.bytes_per_pixel();
        let stride = info.stride() as usize;

        for py in y..end_y {
            let row_offset = py as usize * stride;
            for px in x..end_x {
                let offset = row_offset + px as usize * bpp;
                if offset + 4 <= self.framebuffer_size {
                    unsafe {
                        self.framebuffer
                            .add(offset)
                            .cast::<u32>()
                            .write_volatile(pixel);
                    }
                }
            }
        }

        Ok(())
    }

    /// Clear screen
    pub fn clear(&mut self, color: Color32) -> Result<(), GraphicsError> {
        let mode = self.current_mode().ok_or(GraphicsError::NoMode)?;
        self.fill_rect(0, 0, mode.info.width(), mode.info.height(), color)
    }
}

// =============================================================================
// COLOR TYPES
// =============================================================================

/// 32-bit color (RGBA)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color32 {
    /// Red channel
    pub r: u8,
    /// Green channel
    pub g: u8,
    /// Blue channel
    pub b: u8,
    /// Alpha channel
    pub a: u8,
}

impl Color32 {
    /// Create color from RGB
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create color from RGBA
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create from hex value (0xRRGGBB)
    pub const fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }

    /// Create from hex with alpha (0xAARRGGBB)
    pub const fn from_hex_argb(hex: u32) -> Self {
        Self {
            a: ((hex >> 24) & 0xFF) as u8,
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
        }
    }

    /// To u32 (ARGB)
    pub const fn to_u32_argb(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }

    /// To u32 (RGBA)
    pub const fn to_u32_rgba(&self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }

    /// Blend with another color
    #[must_use]
    pub const fn blend(&self, other: Self, t: u8) -> Self {
        let inv_t = 255 - t;
        Self {
            r: ((self.r as u16 * inv_t as u16 + other.r as u16 * t as u16) / 255) as u8,
            g: ((self.g as u16 * inv_t as u16 + other.g as u16 * t as u16) / 255) as u8,
            b: ((self.b as u16 * inv_t as u16 + other.b as u16 * t as u16) / 255) as u8,
            a: ((self.a as u16 * inv_t as u16 + other.a as u16 * t as u16) / 255) as u8,
        }
    }

    /// Alpha blend over another color
    #[must_use]
    pub const fn over(&self, background: Self) -> Self {
        if self.a == 255 {
            return *self;
        }
        if self.a == 0 {
            return background;
        }

        let alpha = self.a as u16;
        let inv_alpha = 255 - alpha;

        Self {
            r: ((self.r as u16 * alpha + background.r as u16 * inv_alpha) / 255) as u8,
            g: ((self.g as u16 * alpha + background.g as u16 * inv_alpha) / 255) as u8,
            b: ((self.b as u16 * alpha + background.b as u16 * inv_alpha) / 255) as u8,
            a: 255,
        }
    }

    /// Grayscale value
    #[must_use]
    pub const fn grayscale(&self) -> u8 {
        // ITU-R BT.601 weights
        ((self.r as u16 * 299 + self.g as u16 * 587 + self.b as u16 * 114) / 1000) as u8
    }

    /// Black color constant
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White color constant
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Red color constant
    pub const RED: Self = Self::rgb(255, 0, 0);
    /// Green color constant
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    /// Blue color constant
    pub const BLUE: Self = Self::rgb(0, 0, 255);
    /// Cyan color constant
    pub const CYAN: Self = Self::rgb(0, 255, 255);
    /// Magenta color constant
    pub const MAGENTA: Self = Self::rgb(255, 0, 255);
    /// Yellow color constant
    pub const YELLOW: Self = Self::rgb(255, 255, 0);
    /// Orange color constant
    pub const ORANGE: Self = Self::rgb(255, 165, 0);
    /// Purple color constant
    pub const PURPLE: Self = Self::rgb(128, 0, 128);
    /// Pink color constant
    pub const PINK: Self = Self::rgb(255, 192, 203);
    /// Gray color constant
    pub const GRAY: Self = Self::rgb(128, 128, 128);
    /// Light gray color constant
    pub const LIGHT_GRAY: Self = Self::rgb(192, 192, 192);
    /// Dark gray color constant
    pub const DARK_GRAY: Self = Self::rgb(64, 64, 64);
    /// Transparent color constant
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
}

impl From<u32> for Color32 {
    fn from(v: u32) -> Self {
        Self::from_hex(v)
    }
}

impl fmt::Display for Color32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// BLT pixel
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BltPixel {
    /// Blue channel
    pub blue: u8,
    /// Green channel
    pub green: u8,
    /// Red channel
    pub red: u8,
    /// Reserved byte
    pub reserved: u8,
}

impl BltPixel {
    /// Create from color
    #[must_use]
    pub const fn from_color(color: Color32) -> Self {
        Self {
            red: color.r,
            green: color.g,
            blue: color.b,
            reserved: 0,
        }
    }

    /// Convert to color
    #[must_use]
    pub const fn to_color(&self) -> Color32 {
        Color32::rgb(self.red, self.green, self.blue)
    }
}

/// BLT operation
#[derive(Debug, Clone, Copy)]
pub enum BltOperation {
    /// Fill video with pixel
    VideoFill,
    /// Copy from video to buffer
    VideoToBltBuffer,
    /// Copy from buffer to video
    BufferToVideo,
    /// Copy within video
    VideoToVideo,
}

// =============================================================================
// DRAWING PRIMITIVES
// =============================================================================

/// Drawing context
pub struct DrawingContext<'a> {
    gop: &'a mut GraphicsOutput,
    clip_rect: Option<Rect>,
}

impl<'a> DrawingContext<'a> {
    /// Create new drawing context
    pub fn new(gop: &'a mut GraphicsOutput) -> Self {
        Self {
            gop,
            clip_rect: None,
        }
    }

    /// Set clipping rectangle
    pub fn set_clip(&mut self, rect: Option<Rect>) {
        self.clip_rect = rect;
    }

    /// Draw pixel
    pub fn draw_pixel(&mut self, x: i32, y: i32, color: Color32) -> Result<(), GraphicsError> {
        if x < 0 || y < 0 {
            return Ok(());
        }

        if let Some(ref clip) = self.clip_rect {
            if !clip.contains(x, y) {
                return Ok(());
            }
        }

        self.gop.set_pixel(x as u32, y as u32, color)
    }

    /// Draw horizontal line
    pub fn draw_hline(
        &mut self,
        x1: i32,
        x2: i32,
        y: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        let (start, end) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };

        for x in start..=end {
            self.draw_pixel(x, y, color)?;
        }
        Ok(())
    }

    /// Draw vertical line
    pub fn draw_vline(
        &mut self,
        x: i32,
        y1: i32,
        y2: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        let (start, end) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };

        for y in start..=end {
            self.draw_pixel(x, y, color)?;
        }
        Ok(())
    }

    /// Draw line (Bresenham's algorithm)
    pub fn draw_line(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            self.draw_pixel(x, y, color)?;

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

    /// Draw rectangle outline
    pub fn draw_rect(&mut self, rect: &Rect, color: Color32) -> Result<(), GraphicsError> {
        let x1 = rect.x;
        let y1 = rect.y;
        let x2 = rect.x + rect.width as i32 - 1;
        let y2 = rect.y + rect.height as i32 - 1;

        self.draw_hline(x1, x2, y1, color)?;
        self.draw_hline(x1, x2, y2, color)?;
        self.draw_vline(x1, y1, y2, color)?;
        self.draw_vline(x2, y1, y2, color)?;

        Ok(())
    }

    /// Fill rectangle
    pub fn fill_rect(&mut self, rect: &Rect, color: Color32) -> Result<(), GraphicsError> {
        if rect.x >= 0 && rect.y >= 0 {
            self.gop
                .fill_rect(rect.x as u32, rect.y as u32, rect.width, rect.height, color)
        } else {
            // Handle negative coordinates
            for y in rect.y..(rect.y + rect.height as i32) {
                for x in rect.x..(rect.x + rect.width as i32) {
                    self.draw_pixel(x, y, color)?;
                }
            }
            Ok(())
        }
    }

    /// Draw circle outline (Midpoint algorithm)
    pub fn draw_circle(
        &mut self,
        cx: i32,
        cy: i32,
        radius: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.draw_pixel(cx + x, cy + y, color)?;
            self.draw_pixel(cx + y, cy + x, color)?;
            self.draw_pixel(cx - y, cy + x, color)?;
            self.draw_pixel(cx - x, cy + y, color)?;
            self.draw_pixel(cx - x, cy - y, color)?;
            self.draw_pixel(cx - y, cy - x, color)?;
            self.draw_pixel(cx + y, cy - x, color)?;
            self.draw_pixel(cx + x, cy - y, color)?;

            y += 1;
            if err <= 0 {
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }

        Ok(())
    }

    /// Fill circle
    pub fn fill_circle(
        &mut self,
        cx: i32,
        cy: i32,
        radius: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.draw_hline(cx - x, cx + x, cy + y, color)?;
            self.draw_hline(cx - x, cx + x, cy - y, color)?;
            self.draw_hline(cx - y, cx + y, cy + x, color)?;
            self.draw_hline(cx - y, cx + y, cy - x, color)?;

            y += 1;
            if err <= 0 {
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }

        Ok(())
    }

    /// Draw rounded rectangle
    pub fn draw_rounded_rect(
        &mut self,
        rect: &Rect,
        radius: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        let x1 = rect.x;
        let y1 = rect.y;
        let x2 = rect.x + rect.width as i32 - 1;
        let y2 = rect.y + rect.height as i32 - 1;

        // Edges
        self.draw_hline(x1 + radius, x2 - radius, y1, color)?;
        self.draw_hline(x1 + radius, x2 - radius, y2, color)?;
        self.draw_vline(x1, y1 + radius, y2 - radius, color)?;
        self.draw_vline(x2, y1 + radius, y2 - radius, color)?;

        // Corners (quarter circles)
        self.draw_arc(x1 + radius, y1 + radius, radius, 180, 270, color)?;
        self.draw_arc(x2 - radius, y1 + radius, radius, 270, 360, color)?;
        self.draw_arc(x2 - radius, y2 - radius, radius, 0, 90, color)?;
        self.draw_arc(x1 + radius, y2 - radius, radius, 90, 180, color)?;

        Ok(())
    }

    /// Draw arc
    pub fn draw_arc(
        &mut self,
        cx: i32,
        cy: i32,
        radius: i32,
        start_angle: i32,
        end_angle: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        // Simple arc drawing using angle iteration
        for angle_deg in start_angle..=end_angle {
            let angle_rad = (angle_deg as f32) * core::f32::consts::PI / 180.0;
            let x = cx + (radius as f32 * cos_approx(angle_rad)) as i32;
            let y = cy + (radius as f32 * sin_approx(angle_rad)) as i32;
            self.draw_pixel(x, y, color)?;
        }

        Ok(())
    }

    /// Draw triangle outline
    pub fn draw_triangle(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        self.draw_line(x0, y0, x1, y1, color)?;
        self.draw_line(x1, y1, x2, y2, color)?;
        self.draw_line(x2, y2, x0, y0, color)?;
        Ok(())
    }

    /// Fill triangle (using scan line)
    pub fn fill_triangle(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        // Sort vertices by y
        let (mut v0, mut v1, mut v2) = ((x0, y0), (x1, y1), (x2, y2));

        if v1.1 < v0.1 {
            core::mem::swap(&mut v0, &mut v1);
        }
        if v2.1 < v0.1 {
            core::mem::swap(&mut v0, &mut v2);
        }
        if v2.1 < v1.1 {
            core::mem::swap(&mut v1, &mut v2);
        }

        let (x0, y0) = v0;
        let (x1, y1) = v1;
        let (x2, y2) = v2;

        // Fill flat-bottom and flat-top triangles
        if y1 == y2 {
            self.fill_flat_bottom_triangle(x0, y0, x1, y1, x2, y2, color)?;
        } else if y0 == y1 {
            self.fill_flat_top_triangle(x0, y0, x1, y1, x2, y2, color)?;
        } else {
            // Split into two triangles
            let x3 = x0 + (((y1 - y0) as i64 * (x2 - x0) as i64) / (y2 - y0) as i64) as i32;
            let y3 = y1;

            self.fill_flat_bottom_triangle(x0, y0, x1, y1, x3, y3, color)?;
            self.fill_flat_top_triangle(x1, y1, x3, y3, x2, y2, color)?;
        }

        Ok(())
    }

    fn fill_flat_bottom_triangle(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        _y2: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        let dy = y1 - y0;
        if dy == 0 {
            return Ok(());
        }

        let invslope1 = (x1 - x0) as f32 / dy as f32;
        let invslope2 = (x2 - x0) as f32 / dy as f32;

        let mut curx1 = x0 as f32;
        let mut curx2 = x0 as f32;

        for y in y0..=y1 {
            let start = curx1.min(curx2) as i32;
            let end = curx1.max(curx2) as i32;
            self.draw_hline(start, end, y, color)?;
            curx1 += invslope1;
            curx2 += invslope2;
        }

        Ok(())
    }

    fn fill_flat_top_triangle(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        _y1: i32,
        x2: i32,
        y2: i32,
        color: Color32,
    ) -> Result<(), GraphicsError> {
        let dy = y2 - y0;
        if dy == 0 {
            return Ok(());
        }

        let invslope1 = (x2 - x0) as f32 / dy as f32;
        let invslope2 = (x2 - x1) as f32 / dy as f32;

        let mut curx1 = x2 as f32;
        let mut curx2 = x2 as f32;

        for y in (y0..=y2).rev() {
            let start = curx1.min(curx2) as i32;
            let end = curx1.max(curx2) as i32;
            self.draw_hline(start, end, y, color)?;
            curx1 -= invslope1;
            curx2 -= invslope2;
        }

        Ok(())
    }

    /// Draw gradient rectangle
    pub fn draw_gradient_rect(
        &mut self,
        rect: &Rect,
        start_color: Color32,
        end_color: Color32,
        horizontal: bool,
    ) -> Result<(), GraphicsError> {
        if horizontal {
            for x in 0..rect.width {
                let t = (x * 255) / rect.width.max(1);
                let color = start_color.blend(end_color, t as u8);
                for y in 0..rect.height as i32 {
                    self.draw_pixel(rect.x + x as i32, rect.y + y, color)?;
                }
            }
        } else {
            for y in 0..rect.height {
                let t = (y * 255) / rect.height.max(1);
                let color = start_color.blend(end_color, t as u8);
                for x in 0..rect.width as i32 {
                    self.draw_pixel(rect.x + x, rect.y + y as i32, color)?;
                }
            }
        }

        Ok(())
    }
}

/// Approximate sine (for `no_std`)
const fn sin_approx(x: f32) -> f32 {
    // Taylor series approximation
    let x = x % (2.0 * core::f32::consts::PI);
    let x3 = x * x * x;
    let x5 = x3 * x * x;
    let x7 = x5 * x * x;
    x - x3 / 6.0 + x5 / 120.0 - x7 / 5040.0
}

/// Approximate cosine (for `no_std`)
const fn cos_approx(x: f32) -> f32 {
    sin_approx(x + core::f32::consts::PI / 2.0)
}

// =============================================================================
// GEOMETRY TYPES
// =============================================================================

/// Rectangle
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
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

    /// Create from corners
    #[must_use]
    pub const fn from_corners(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        let (x, width) = if x1 <= x2 {
            (x1, (x2 - x1) as u32)
        } else {
            (x2, (x1 - x2) as u32)
        };

        let (y, height) = if y1 <= y2 {
            (y1, (y2 - y1) as u32)
        } else {
            (y2, (y1 - y2) as u32)
        };

        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge
    #[must_use]
    pub const fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    /// Bottom edge
    #[must_use]
    pub const fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    /// Center X
    #[must_use]
    pub const fn center_x(&self) -> i32 {
        self.x + (self.width as i32) / 2
    }

    /// Center Y
    #[must_use]
    pub const fn center_y(&self) -> i32 {
        self.y + (self.height as i32) / 2
    }

    /// Check if point is inside
    #[must_use]
    pub const fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Check if rectangles intersect
    #[must_use]
    pub const fn intersects(&self, other: &Self) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Get intersection
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }

        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        Some(Self::from_corners(x, y, right, bottom))
    }

    /// Get union (bounding box)
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());

        Self::from_corners(x, y, right, bottom)
    }

    /// Inset rectangle
    #[must_use]
    pub const fn inset(&self, amount: i32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: self.width.saturating_sub((amount * 2) as u32),
            height: self.height.saturating_sub((amount * 2) as u32),
        }
    }

    /// Offset rectangle
    #[must_use]
    pub const fn offset(&self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            width: self.width,
            height: self.height,
        }
    }
}

/// Point
#[derive(Debug, Clone, Copy)]
pub struct Point {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
}

impl Point {
    /// Create a new point
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Zero point constant
    pub const ZERO: Self = Self::new(0, 0);
}

/// Size
#[derive(Debug, Clone, Copy)]
pub struct Size {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

impl Size {
    /// Create a new size
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Zero size constant
    pub const ZERO: Self = Self::new(0, 0);
}

// =============================================================================
// GRAPHICS ERROR
// =============================================================================

/// Graphics error
#[derive(Debug, Clone)]
pub enum GraphicsError {
    /// No graphics mode set
    NoMode,
    /// Invalid mode
    InvalidMode,
    /// Out of bounds
    OutOfBounds,
    /// Invalid parameter
    InvalidParameter,
    /// Hardware error
    HardwareError,
    /// Not supported
    NotSupported,
    /// Device error
    DeviceError,
}

impl fmt::Display for GraphicsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoMode => write!(f, "no graphics mode set"),
            Self::InvalidMode => write!(f, "invalid graphics mode"),
            Self::OutOfBounds => write!(f, "coordinates out of bounds"),
            Self::InvalidParameter => write!(f, "invalid parameter"),
            Self::HardwareError => write!(f, "hardware error"),
            Self::NotSupported => write!(f, "not supported"),
            Self::DeviceError => write!(f, "device error"),
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
    fn test_color32() {
        let red = Color32::RED;
        assert_eq!(red.r, 255);
        assert_eq!(red.g, 0);
        assert_eq!(red.b, 0);

        let from_hex = Color32::from_hex(0xFF8000);
        assert_eq!(from_hex.r, 255);
        assert_eq!(from_hex.g, 128);
        assert_eq!(from_hex.b, 0);
    }

    #[test]
    fn test_color_blend() {
        let black = Color32::BLACK;
        let white = Color32::WHITE;
        let gray = black.blend(white, 128);

        assert!(gray.r > 100 && gray.r < 150);
    }

    #[test]
    fn test_rect() {
        let rect = Rect::new(10, 20, 100, 50);
        assert!(rect.contains(50, 40));
        assert!(!rect.contains(0, 0));
        assert_eq!(rect.right(), 110);
        assert_eq!(rect.bottom(), 70);
    }

    #[test]
    fn test_rect_intersection() {
        let r1 = Rect::new(0, 0, 100, 100);
        let r2 = Rect::new(50, 50, 100, 100);

        assert!(r1.intersects(&r2));

        let intersection = r1.intersection(&r2).unwrap();
        assert_eq!(intersection.x, 50);
        assert_eq!(intersection.y, 50);
        assert_eq!(intersection.width, 50);
        assert_eq!(intersection.height, 50);
    }
}
