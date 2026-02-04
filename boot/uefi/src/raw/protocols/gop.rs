//! Graphics Output Protocol (GOP)
//!
//! The GOP protocol provides access to the graphics frame buffer
//! and allows setting video modes.
//!
//! # Safety
//! This module interfaces with UEFI firmware via FFI. All protocol pointers
//! are provided by the firmware and validated before use.
// codeql[rust/access-invalid-pointer] - UEFI FFI pointers validated by firmware

use core::fmt;

use crate::raw::types::*;

// =============================================================================
// GRAPHICS OUTPUT PROTOCOL
// =============================================================================

/// EFI Graphics Output Protocol
#[repr(C)]
pub struct EfiGraphicsOutputProtocol {
    /// Query mode information
    pub query_mode: unsafe extern "efiapi" fn(
        this: *mut Self,
        mode_number: u32,
        size_of_info: *mut usize,
        info: *mut *mut EfiGraphicsOutputModeInformation,
    ) -> Status,

    /// Set mode
    pub set_mode: unsafe extern "efiapi" fn(this: *mut Self, mode_number: u32) -> Status,

    /// Block transfer
    pub blt: unsafe extern "efiapi" fn(
        this: *mut Self,
        blt_buffer: *mut EfiGraphicsOutputBltPixel,
        blt_operation: EfiGraphicsOutputBltOperation,
        source_x: usize,
        source_y: usize,
        destination_x: usize,
        destination_y: usize,
        width: usize,
        height: usize,
        delta: usize,
    ) -> Status,

    /// Mode information
    pub mode: *mut EfiGraphicsOutputProtocolMode,
}

impl EfiGraphicsOutputProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::GRAPHICS_OUTPUT_PROTOCOL;

    /// Query a specific mode
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid and obtained
    /// from UEFI LocateProtocol or HandleProtocol calls.
    ///
    /// # Errors
    /// Returns status error if the mode cannot be queried.
    pub unsafe fn query_mode(
        &mut self,
        mode_number: u32,
    ) -> Result<&EfiGraphicsOutputModeInformation, Status> {
        let mut size = 0;
        let mut info = core::ptr::null_mut();

        let status = (self.query_mode)(self, mode_number, &mut size, &mut info);

        if status.is_success() && !info.is_null() {
            // SAFETY: UEFI guarantees info pointer is valid when status is SUCCESS
            Ok(&*info)
        } else {
            Err(status)
        }
    }

    /// Set the video mode
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid and obtained
    /// from UEFI LocateProtocol or HandleProtocol calls.
    pub unsafe fn set_mode(&mut self, mode_number: u32) -> Result<(), Status> {
        let status = (self.set_mode)(self, mode_number);
        status.to_status_result()
    }

    /// Block transfer operation
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer and buffer are valid.
    pub unsafe fn blt(
        &mut self,
        buffer: Option<&mut [EfiGraphicsOutputBltPixel]>,
        operation: EfiGraphicsOutputBltOperation,
        src_x: usize,
        src_y: usize,
        dst_x: usize,
        dst_y: usize,
        width: usize,
        height: usize,
        delta: usize,
    ) -> Result<(), Status> {
        let buffer_ptr = buffer
            .map(|b| b.as_mut_ptr())
            .unwrap_or(core::ptr::null_mut());

        let status = (self.blt)(
            self, buffer_ptr, operation, src_x, src_y, dst_x, dst_y, width, height, delta,
        );
        status.to_status_result()
    }

    /// Fill a rectangle with a color
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn fill_rectangle(
        &mut self,
        color: EfiGraphicsOutputBltPixel,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
    ) -> Result<(), Status> {
        let mut pixel = color;
        self.blt(
            Some(core::slice::from_mut(&mut pixel)),
            EfiGraphicsOutputBltOperation::BltVideoFill,
            0,
            0,
            x,
            y,
            width,
            height,
            0,
        )
    }

    /// Get current mode information
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn current_mode(&self) -> Option<&EfiGraphicsOutputProtocolMode> {
        if self.mode.is_null() {
            None
        } else {
            Some(&*self.mode)
        }
    }

    /// Get frame buffer base address
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn frame_buffer_base(&self) -> Option<PhysicalAddress> {
        self.current_mode().map(|m| m.frame_buffer_base)
    }

    /// Get frame buffer size
    ///
    /// # Safety
    /// The caller must ensure the protocol pointer is valid.
    pub unsafe fn frame_buffer_size(&self) -> Option<usize> {
        self.current_mode().map(|m| m.frame_buffer_size)
    }
}

impl fmt::Debug for EfiGraphicsOutputProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EfiGraphicsOutputProtocol")
            .field("mode", &self.mode)
            .finish()
    }
}

// =============================================================================
// MODE INFORMATION
// =============================================================================

/// Graphics output mode information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiGraphicsOutputModeInformation {
    /// Version of this data structure
    pub version: u32,
    /// Horizontal resolution in pixels
    pub horizontal_resolution: u32,
    /// Vertical resolution in pixels
    pub vertical_resolution: u32,
    /// Pixel format
    pub pixel_format: EfiGraphicsPixelFormat,
    /// Pixel information (for PixelBitMask format)
    pub pixel_information: EfiPixelBitmask,
    /// Pixels per scan line
    pub pixels_per_scan_line: u32,
}

impl EfiGraphicsOutputModeInformation {
    /// Get the resolution as a tuple
    pub fn resolution(&self) -> (u32, u32) {
        (self.horizontal_resolution, self.vertical_resolution)
    }

    /// Calculate bytes per pixel based on format
    pub fn bytes_per_pixel(&self) -> usize {
        match self.pixel_format {
            EfiGraphicsPixelFormat::PixelRedGreenBlueReserved8BitPerColor => 4,
            EfiGraphicsPixelFormat::PixelBlueGreenRedReserved8BitPerColor => 4,
            EfiGraphicsPixelFormat::PixelBitMask => {
                // Calculate from bitmask
                let mask = self.pixel_information.red_mask
                    | self.pixel_information.green_mask
                    | self.pixel_information.blue_mask
                    | self.pixel_information.reserved_mask;
                ((64 - mask.leading_zeros()) / 8) as usize
            },
            EfiGraphicsPixelFormat::PixelBltOnly => 0,
        }
    }

    /// Calculate stride in bytes
    pub fn stride(&self) -> usize {
        self.pixels_per_scan_line as usize * self.bytes_per_pixel()
    }

    /// Calculate total frame buffer size needed
    pub fn frame_buffer_size(&self) -> usize {
        self.stride() * self.vertical_resolution as usize
    }
}

/// Graphics output protocol mode
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiGraphicsOutputProtocolMode {
    /// Maximum mode number (modes are 0 to max_mode-1)
    pub max_mode: u32,
    /// Current mode number
    pub mode: u32,
    /// Pointer to mode information
    pub info: *mut EfiGraphicsOutputModeInformation,
    /// Size of mode information structure
    pub size_of_info: usize,
    /// Physical address of frame buffer
    pub frame_buffer_base: PhysicalAddress,
    /// Size of frame buffer in bytes
    pub frame_buffer_size: usize,
}

impl EfiGraphicsOutputProtocolMode {
    /// Get the current mode information
    ///
    /// # Safety
    /// The caller must ensure the info pointer is valid.
    pub unsafe fn current_info(&self) -> Option<&EfiGraphicsOutputModeInformation> {
        if self.info.is_null() {
            None
        } else {
            Some(&*self.info)
        }
    }
}

// =============================================================================
// PIXEL FORMAT
// =============================================================================

/// Pixel format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiGraphicsPixelFormat {
    /// RGBX 8-bit per color
    PixelRedGreenBlueReserved8BitPerColor = 0,
    /// BGRX 8-bit per color
    PixelBlueGreenRedReserved8BitPerColor = 1,
    /// Custom bit mask
    PixelBitMask = 2,
    /// BLT only (no direct frame buffer access)
    PixelBltOnly = 3,
}

impl EfiGraphicsPixelFormat {
    /// Check if this format allows direct frame buffer access
    pub fn has_frame_buffer(&self) -> bool {
        *self != Self::PixelBltOnly
    }

    /// Check if this is RGB format
    pub fn is_rgb(&self) -> bool {
        *self == Self::PixelRedGreenBlueReserved8BitPerColor
    }

    /// Check if this is BGR format
    pub fn is_bgr(&self) -> bool {
        *self == Self::PixelBlueGreenRedReserved8BitPerColor
    }
}

/// Pixel bitmask for custom formats
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct EfiPixelBitmask {
    /// Red mask
    pub red_mask: u32,
    /// Green mask
    pub green_mask: u32,
    /// Blue mask
    pub blue_mask: u32,
    /// Reserved mask
    pub reserved_mask: u32,
}

impl EfiPixelBitmask {
    /// Standard RGB888 mask
    pub const RGB888: Self = Self {
        red_mask: 0x00FF0000,
        green_mask: 0x0000FF00,
        blue_mask: 0x000000FF,
        reserved_mask: 0xFF000000,
    };

    /// Standard BGR888 mask
    pub const BGR888: Self = Self {
        red_mask: 0x000000FF,
        green_mask: 0x0000FF00,
        blue_mask: 0x00FF0000,
        reserved_mask: 0xFF000000,
    };

    /// RGB565 mask
    pub const RGB565: Self = Self {
        red_mask: 0xF800,
        green_mask: 0x07E0,
        blue_mask: 0x001F,
        reserved_mask: 0x0000,
    };
}

// =============================================================================
// BLT OPERATIONS
// =============================================================================

/// Block transfer operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum EfiGraphicsOutputBltOperation {
    /// Fill rectangle with single color from buffer
    BltVideoFill        = 0,
    /// Copy from video to buffer
    BltVideoToBltBuffer = 1,
    /// Copy from buffer to video
    BltBufferToVideo    = 2,
    /// Copy within video memory
    BltVideoToVideo     = 3,
}

/// BLT pixel structure
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct EfiGraphicsOutputBltPixel {
    /// Blue component
    pub blue: u8,
    /// Green component
    pub green: u8,
    /// Red component
    pub red: u8,
    /// Reserved (alpha)
    pub reserved: u8,
}

impl EfiGraphicsOutputBltPixel {
    /// Black color
    pub const BLACK: Self = Self::new(0, 0, 0);
    /// White color
    pub const WHITE: Self = Self::new(255, 255, 255);
    /// Red color
    pub const RED: Self = Self::new(255, 0, 0);
    /// Green color
    pub const GREEN: Self = Self::new(0, 255, 0);
    /// Blue color
    pub const BLUE: Self = Self::new(0, 0, 255);
    /// Yellow color
    pub const YELLOW: Self = Self::new(255, 255, 0);
    /// Cyan color
    pub const CYAN: Self = Self::new(0, 255, 255);
    /// Magenta color
    pub const MAGENTA: Self = Self::new(255, 0, 255);

    /// Create a new pixel from RGB values
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self {
            red,
            green,
            blue,
            reserved: 0,
        }
    }

    /// Create a new pixel with alpha
    pub const fn with_alpha(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            reserved: alpha,
        }
    }

    /// Create from a 32-bit ARGB value
    pub const fn from_argb(argb: u32) -> Self {
        Self {
            red: ((argb >> 16) & 0xFF) as u8,
            green: ((argb >> 8) & 0xFF) as u8,
            blue: (argb & 0xFF) as u8,
            reserved: ((argb >> 24) & 0xFF) as u8,
        }
    }

    /// Convert to 32-bit ARGB value
    pub const fn to_argb(self) -> u32 {
        ((self.reserved as u32) << 24)
            | ((self.red as u32) << 16)
            | ((self.green as u32) << 8)
            | (self.blue as u32)
    }

    /// Convert to 32-bit ABGR value (for BGR formats)
    pub const fn to_abgr(self) -> u32 {
        ((self.reserved as u32) << 24)
            | ((self.blue as u32) << 16)
            | ((self.green as u32) << 8)
            | (self.red as u32)
    }

    /// Blend with another pixel
    pub fn blend(self, other: Self, alpha: u8) -> Self {
        let a = alpha as u16;
        let inv_a = 255 - a;

        Self {
            red: ((self.red as u16 * inv_a + other.red as u16 * a) / 255) as u8,
            green: ((self.green as u16 * inv_a + other.green as u16 * a) / 255) as u8,
            blue: ((self.blue as u16 * inv_a + other.blue as u16 * a) / 255) as u8,
            reserved: 255,
        }
    }
}

// =============================================================================
// EDID PROTOCOLS
// =============================================================================

/// EDID Discovered Protocol
#[repr(C)]
pub struct EfiEdidDiscoveredProtocol {
    /// Size of EDID data
    pub size_of_edid: u32,
    /// Pointer to EDID data
    pub edid: *mut u8,
}

impl EfiEdidDiscoveredProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::EDID_DISCOVERED_PROTOCOL;

    /// Get EDID data
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn edid_data(&self) -> Option<&[u8]> {
        if self.edid.is_null() || self.size_of_edid == 0 {
            None
        } else {
            Some(core::slice::from_raw_parts(
                self.edid,
                self.size_of_edid as usize,
            ))
        }
    }
}

/// EDID Active Protocol
#[repr(C)]
pub struct EfiEdidActiveProtocol {
    /// Size of EDID data
    pub size_of_edid: u32,
    /// Pointer to EDID data
    pub edid: *mut u8,
}

impl EfiEdidActiveProtocol {
    /// Protocol GUID
    pub const GUID: Guid = guids::EDID_ACTIVE_PROTOCOL;

    /// Get EDID data
    ///
    /// # Safety
    /// The caller must ensure the pointer is valid.
    pub unsafe fn edid_data(&self) -> Option<&[u8]> {
        if self.edid.is_null() || self.size_of_edid == 0 {
            None
        } else {
            Some(core::slice::from_raw_parts(
                self.edid,
                self.size_of_edid as usize,
            ))
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
    fn test_pixel_creation() {
        let pixel = EfiGraphicsOutputBltPixel::new(255, 128, 64);
        assert_eq!(pixel.red, 255);
        assert_eq!(pixel.green, 128);
        assert_eq!(pixel.blue, 64);
    }

    #[test]
    fn test_pixel_argb_conversion() {
        let pixel = EfiGraphicsOutputBltPixel::from_argb(0xFF804020);
        assert_eq!(pixel.red, 0x80);
        assert_eq!(pixel.green, 0x40);
        assert_eq!(pixel.blue, 0x20);
        assert_eq!(pixel.reserved, 0xFF);
    }

    #[test]
    fn test_mode_info_bytes_per_pixel() {
        let info = EfiGraphicsOutputModeInformation {
            version: 0,
            horizontal_resolution: 1920,
            vertical_resolution: 1080,
            pixel_format: EfiGraphicsPixelFormat::PixelBlueGreenRedReserved8BitPerColor,
            pixel_information: EfiPixelBitmask::default(),
            pixels_per_scan_line: 1920,
        };

        assert_eq!(info.bytes_per_pixel(), 4);
        assert_eq!(info.stride(), 1920 * 4);
    }
}
