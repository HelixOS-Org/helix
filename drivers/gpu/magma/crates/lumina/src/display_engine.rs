//! Display Engine Types for Lumina
//!
//! This module provides comprehensive display output management including
//! monitor configuration, color management, HDR, and display composition.

use alloc::vec::Vec;

// ============================================================================
// Display Handle
// ============================================================================

/// Handle to a display output
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DisplayHandle(pub u64);

impl DisplayHandle {
    /// Null handle constant
    pub const NULL: Self = Self(0);

    /// Creates a new handle from raw value
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw handle value
    #[inline]
    pub const fn as_raw(&self) -> u64 {
        self.0
    }

    /// Checks if this is a null handle
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Checks if this is a valid handle
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl Default for DisplayHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Handle to a display mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DisplayModeHandle(pub u64);

impl DisplayModeHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates from raw
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }
}

impl Default for DisplayModeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Display Properties
// ============================================================================

/// Display properties
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DisplayProperties {
    /// Display name
    pub name: DisplayName,
    /// Physical dimensions in millimeters
    pub physical_dimensions: PhysicalSize,
    /// Physical resolution
    pub physical_resolution: Resolution,
    /// Supported transforms
    pub supported_transforms: SurfaceTransformFlags,
    /// Plane reorder possible
    pub plane_reorder_possible: bool,
    /// Persistent content
    pub persistent_content: bool,
    /// Display type
    pub display_type: DisplayType,
    /// Connection type
    pub connection: ConnectionType,
    /// HDR capabilities
    pub hdr_capabilities: HdrCapabilities,
    /// Color space support
    pub supported_color_spaces: ColorSpaceFlags,
}

impl DisplayProperties {
    /// Returns the display's DPI
    #[inline]
    pub fn dpi(&self) -> (f32, f32) {
        let width_inch = self.physical_dimensions.width_mm as f32 / 25.4;
        let height_inch = self.physical_dimensions.height_mm as f32 / 25.4;

        if width_inch < 0.1 || height_inch < 0.1 {
            return (96.0, 96.0); // Default DPI
        }

        (
            self.physical_resolution.width as f32 / width_inch,
            self.physical_resolution.height as f32 / height_inch,
        )
    }

    /// Checks if this is an HDR display
    #[inline]
    pub const fn is_hdr(&self) -> bool {
        self.hdr_capabilities.supports_hdr10
            || self.hdr_capabilities.supports_dolby_vision
            || self.hdr_capabilities.supports_hlg
    }

    /// Checks if this is a wide color gamut display
    #[inline]
    pub const fn is_wide_gamut(&self) -> bool {
        self.supported_color_spaces
            .contains(ColorSpaceFlags::DISPLAY_P3)
            || self
                .supported_color_spaces
                .contains(ColorSpaceFlags::BT2020)
    }
}

/// Display name (fixed size for no_std)
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DisplayName {
    /// Name bytes
    pub bytes: [u8; 256],
    /// Name length
    pub len: u32,
}

impl DisplayName {
    /// Creates a new display name
    #[inline]
    pub const fn new() -> Self {
        Self {
            bytes: [0; 256],
            len: 0,
        }
    }

    /// Creates from a string slice
    #[inline]
    pub fn from_str(s: &str) -> Self {
        let mut name = Self::new();
        let bytes = s.as_bytes();
        let len = bytes.len().min(255);
        name.bytes[..len].copy_from_slice(&bytes[..len]);
        name.len = len as u32;
        name
    }

    /// Returns the name as a string slice
    #[inline]
    pub fn as_str(&self) -> &str {
        let slice = &self.bytes[..self.len as usize];
        core::str::from_utf8(slice).unwrap_or("Unknown")
    }
}

impl Default for DisplayName {
    fn default() -> Self {
        Self::new()
    }
}

/// Physical size in millimeters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PhysicalSize {
    /// Width in millimeters
    pub width_mm: u32,
    /// Height in millimeters
    pub height_mm: u32,
}

impl PhysicalSize {
    /// Creates a new physical size
    #[inline]
    pub const fn new(width_mm: u32, height_mm: u32) -> Self {
        Self {
            width_mm,
            height_mm,
        }
    }

    /// Returns diagonal in inches
    #[inline]
    pub fn diagonal_inches(&self) -> f32 {
        let w = self.width_mm as f32 / 25.4;
        let h = self.height_mm as f32 / 25.4;
        libm::sqrtf(w * w + h * h)
    }

    /// Common monitor sizes
    pub const MONITOR_24: Self = Self {
        width_mm: 527,
        height_mm: 296,
    };
    pub const MONITOR_27: Self = Self {
        width_mm: 597,
        height_mm: 336,
    };
    pub const MONITOR_32: Self = Self {
        width_mm: 697,
        height_mm: 392,
    };
    pub const TV_55: Self = Self {
        width_mm: 1210,
        height_mm: 680,
    };
    pub const TV_65: Self = Self {
        width_mm: 1429,
        height_mm: 804,
    };
}

/// Display resolution
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct Resolution {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl Resolution {
    /// Creates a new resolution
    #[inline]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Standard resolutions
    pub const HD_720P: Self = Self {
        width: 1280,
        height: 720,
    };
    pub const HD_1080P: Self = Self {
        width: 1920,
        height: 1080,
    };
    pub const QHD_1440P: Self = Self {
        width: 2560,
        height: 1440,
    };
    pub const UHD_4K: Self = Self {
        width: 3840,
        height: 2160,
    };
    pub const UHD_5K: Self = Self {
        width: 5120,
        height: 2880,
    };
    pub const UHD_8K: Self = Self {
        width: 7680,
        height: 4320,
    };

    // Ultrawide
    pub const UWFHD: Self = Self {
        width: 2560,
        height: 1080,
    };
    pub const UWQHD: Self = Self {
        width: 3440,
        height: 1440,
    };
    pub const DUHD: Self = Self {
        width: 5120,
        height: 2160,
    };

    /// Returns the aspect ratio
    #[inline]
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Returns the total number of pixels
    #[inline]
    pub const fn pixel_count(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Checks if this is a 16:9 resolution
    #[inline]
    pub fn is_16_9(&self) -> bool {
        let ratio = self.aspect_ratio();
        (ratio - 16.0 / 9.0).abs() < 0.01
    }

    /// Checks if this is an ultrawide resolution
    #[inline]
    pub fn is_ultrawide(&self) -> bool {
        self.aspect_ratio() > 2.0
    }
}

/// Display type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DisplayType {
    /// Unknown display type
    #[default]
    Unknown    = 0,
    /// Internal display (laptop panel)
    Internal   = 1,
    /// External monitor
    External   = 2,
    /// Virtual/remote display
    Virtual    = 3,
    /// Projector
    Projector  = 4,
    /// Television
    Television = 5,
}

/// Connection type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ConnectionType {
    /// Unknown connection
    #[default]
    Unknown     = 0,
    /// HDMI
    Hdmi        = 1,
    /// DisplayPort
    DisplayPort = 2,
    /// USB-C / Thunderbolt
    UsbC        = 3,
    /// DVI
    Dvi         = 4,
    /// VGA
    Vga         = 5,
    /// eDP (embedded DisplayPort)
    Edp         = 6,
    /// LVDS
    Lvds        = 7,
    /// DSI
    Dsi         = 8,
    /// Virtual
    Virtual     = 9,
}

impl ConnectionType {
    /// Checks if connection supports HDR
    #[inline]
    pub const fn supports_hdr(&self) -> bool {
        matches!(
            self,
            Self::Hdmi | Self::DisplayPort | Self::UsbC | Self::Edp
        )
    }

    /// Returns the maximum bandwidth in Gbps
    #[inline]
    pub const fn max_bandwidth_gbps(&self) -> f32 {
        match self {
            Self::Hdmi => 48.0,        // HDMI 2.1
            Self::DisplayPort => 80.0, // DP 2.0
            Self::UsbC => 80.0,        // USB4/TB4
            Self::Dvi => 7.4,
            Self::Vga => 0.0,  // Analog
            Self::Edp => 32.4, // eDP 1.4b
            Self::Lvds => 3.4,
            Self::Dsi => 6.0,
            _ => 0.0,
        }
    }
}

/// Surface transform flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct SurfaceTransformFlags(pub u32);

impl SurfaceTransformFlags {
    /// Identity transform
    pub const IDENTITY: Self = Self(1 << 0);
    /// 90 degree rotation
    pub const ROTATE_90: Self = Self(1 << 1);
    /// 180 degree rotation
    pub const ROTATE_180: Self = Self(1 << 2);
    /// 270 degree rotation
    pub const ROTATE_270: Self = Self(1 << 3);
    /// Horizontal mirror
    pub const HORIZONTAL_MIRROR: Self = Self(1 << 4);
    /// Horizontal mirror + 90 degree rotation
    pub const HORIZONTAL_MIRROR_ROTATE_90: Self = Self(1 << 5);
    /// Horizontal mirror + 180 degree rotation
    pub const HORIZONTAL_MIRROR_ROTATE_180: Self = Self(1 << 6);
    /// Horizontal mirror + 270 degree rotation
    pub const HORIZONTAL_MIRROR_ROTATE_270: Self = Self(1 << 7);
    /// Inherit from parent
    pub const INHERIT: Self = Self(1 << 8);

    /// All rotations
    pub const ALL_ROTATIONS: Self =
        Self(Self::IDENTITY.0 | Self::ROTATE_90.0 | Self::ROTATE_180.0 | Self::ROTATE_270.0);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// HDR Capabilities
// ============================================================================

/// HDR capabilities
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct HdrCapabilities {
    /// Supports HDR10
    pub supports_hdr10: bool,
    /// Supports HDR10+
    pub supports_hdr10_plus: bool,
    /// Supports Dolby Vision
    pub supports_dolby_vision: bool,
    /// Supports HLG
    pub supports_hlg: bool,
    /// Maximum luminance (nits)
    pub max_luminance: f32,
    /// Maximum frame-average luminance (nits)
    pub max_frame_average_luminance: f32,
    /// Minimum luminance (nits)
    pub min_luminance: f32,
    /// EOTF support
    pub supported_eotf: EotfFlags,
}

impl HdrCapabilities {
    /// SDR display (no HDR)
    #[inline]
    pub const fn sdr() -> Self {
        Self {
            supports_hdr10: false,
            supports_hdr10_plus: false,
            supports_dolby_vision: false,
            supports_hlg: false,
            max_luminance: 100.0,
            max_frame_average_luminance: 100.0,
            min_luminance: 0.1,
            supported_eotf: EotfFlags::SDR,
        }
    }

    /// Basic HDR10 display
    #[inline]
    pub const fn hdr10_basic() -> Self {
        Self {
            supports_hdr10: true,
            supports_hdr10_plus: false,
            supports_dolby_vision: false,
            supports_hlg: false,
            max_luminance: 400.0,
            max_frame_average_luminance: 200.0,
            min_luminance: 0.05,
            supported_eotf: EotfFlags::SDR.union(EotfFlags::PQ),
        }
    }

    /// Premium HDR display
    #[inline]
    pub const fn hdr_premium() -> Self {
        Self {
            supports_hdr10: true,
            supports_hdr10_plus: true,
            supports_dolby_vision: true,
            supports_hlg: true,
            max_luminance: 1000.0,
            max_frame_average_luminance: 600.0,
            min_luminance: 0.005,
            supported_eotf: EotfFlags::ALL,
        }
    }

    /// Returns dynamic range in stops
    #[inline]
    pub fn dynamic_range_stops(&self) -> f32 {
        if self.min_luminance <= 0.0 {
            return 0.0;
        }
        libm::log2f(self.max_luminance / self.min_luminance)
    }

    /// Checks if display meets HDR10 requirements
    #[inline]
    pub const fn meets_hdr10_requirements(&self) -> bool {
        self.supports_hdr10 && self.max_luminance >= 400.0
    }

    /// Checks if display meets Ultra HD Premium requirements
    #[inline]
    pub const fn meets_uhd_premium(&self) -> bool {
        self.max_luminance >= 1000.0 && self.min_luminance <= 0.05
    }
}

/// EOTF (Electro-Optical Transfer Function) flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct EotfFlags(pub u32);

impl EotfFlags {
    /// SDR (gamma 2.2/2.4)
    pub const SDR: Self = Self(1 << 0);
    /// Traditional HDR gamma
    pub const TRADITIONAL_HDR: Self = Self(1 << 1);
    /// PQ (SMPTE ST 2084)
    pub const PQ: Self = Self(1 << 2);
    /// HLG (Hybrid Log-Gamma)
    pub const HLG: Self = Self(1 << 3);

    /// All EOTFs
    pub const ALL: Self = Self(Self::SDR.0 | Self::TRADITIONAL_HDR.0 | Self::PQ.0 | Self::HLG.0);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Color Space
// ============================================================================

/// Color space flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ColorSpaceFlags(pub u32);

impl ColorSpaceFlags {
    /// sRGB
    pub const SRGB: Self = Self(1 << 0);
    /// Display P3
    pub const DISPLAY_P3: Self = Self(1 << 1);
    /// Adobe RGB
    pub const ADOBE_RGB: Self = Self(1 << 2);
    /// BT.709
    pub const BT709: Self = Self(1 << 3);
    /// BT.2020
    pub const BT2020: Self = Self(1 << 4);
    /// DCI-P3
    pub const DCI_P3: Self = Self(1 << 5);
    /// ACES
    pub const ACES: Self = Self(1 << 6);

    /// SDR color spaces
    pub const SDR_SPACES: Self = Self(Self::SRGB.0 | Self::BT709.0);

    /// HDR color spaces
    pub const HDR_SPACES: Self = Self(Self::DISPLAY_P3.0 | Self::BT2020.0 | Self::DCI_P3.0);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Combines flags
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Color space definition
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ColorSpace {
    /// sRGB
    #[default]
    Srgb               = 0,
    /// sRGB non-linear
    SrgbNonlinear      = 1,
    /// Display P3 (linear)
    DisplayP3Linear    = 2,
    /// Display P3 (non-linear)
    DisplayP3Nonlinear = 3,
    /// BT.709 (linear)
    Bt709Linear        = 4,
    /// BT.2020 (linear)
    Bt2020Linear       = 5,
    /// BT.2020 PQ (HDR10)
    Bt2020Pq           = 6,
    /// BT.2020 HLG
    Bt2020Hlg          = 7,
    /// DCI-P3 (linear)
    DciP3Linear        = 8,
    /// Extended sRGB (scRGB)
    ExtendedSrgb       = 9,
    /// HDR10 (BT.2020 with PQ)
    Hdr10              = 10,
    /// Dolby Vision
    DolbyVision        = 11,
}

impl ColorSpace {
    /// Returns the white point
    #[inline]
    pub const fn white_point(&self) -> WhitePoint {
        match self {
            Self::DciP3Linear => WhitePoint::DCI,
            _ => WhitePoint::D65,
        }
    }

    /// Checks if this is an HDR color space
    #[inline]
    pub const fn is_hdr(&self) -> bool {
        matches!(
            self,
            Self::Bt2020Pq | Self::Bt2020Hlg | Self::ExtendedSrgb | Self::Hdr10 | Self::DolbyVision
        )
    }

    /// Checks if this is a linear color space
    #[inline]
    pub const fn is_linear(&self) -> bool {
        matches!(
            self,
            Self::DisplayP3Linear | Self::Bt709Linear | Self::Bt2020Linear | Self::DciP3Linear
        )
    }

    /// Returns the primaries
    #[inline]
    pub const fn primaries(&self) -> ColorPrimaries {
        match self {
            Self::Srgb | Self::SrgbNonlinear | Self::ExtendedSrgb => ColorPrimaries::BT709,
            Self::DisplayP3Linear | Self::DisplayP3Nonlinear => ColorPrimaries::DisplayP3,
            Self::Bt709Linear => ColorPrimaries::BT709,
            Self::Bt2020Linear | Self::Bt2020Pq | Self::Bt2020Hlg | Self::Hdr10 => {
                ColorPrimaries::BT2020
            },
            Self::DciP3Linear => ColorPrimaries::DCIP3,
            Self::DolbyVision => ColorPrimaries::BT2020,
        }
    }
}

/// White point
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum WhitePoint {
    /// D65 (standard daylight)
    #[default]
    D65 = 0,
    /// D50 (horizon light)
    D50 = 1,
    /// DCI (cinema)
    DCI = 2,
}

impl WhitePoint {
    /// Returns the CIE xy coordinates
    #[inline]
    pub const fn xy(&self) -> (f32, f32) {
        match self {
            Self::D65 => (0.31271, 0.32902),
            Self::D50 => (0.34567, 0.35850),
            Self::DCI => (0.31400, 0.35100),
        }
    }
}

/// Color primaries
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ColorPrimaries {
    /// BT.709 / sRGB
    #[default]
    BT709     = 0,
    /// BT.2020 / Rec. 2020
    BT2020    = 1,
    /// Display P3
    DisplayP3 = 2,
    /// DCI-P3
    DCIP3     = 3,
    /// Adobe RGB
    AdobeRGB  = 4,
}

impl ColorPrimaries {
    /// Returns the coverage of BT.709
    #[inline]
    pub const fn bt709_coverage(&self) -> f32 {
        match self {
            Self::BT709 => 1.0,
            Self::DisplayP3 => 0.84,
            Self::DCIP3 => 0.87,
            Self::BT2020 => 0.58,
            Self::AdobeRGB => 0.67,
        }
    }
}

// ============================================================================
// Display Mode
// ============================================================================

/// Display mode parameters
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DisplayMode {
    /// Handle
    pub handle: DisplayModeHandle,
    /// Resolution
    pub resolution: Resolution,
    /// Refresh rate in millihertz (e.g., 60000 = 60 Hz)
    pub refresh_rate_mhz: u32,
    /// Pixel clock in Hz
    pub pixel_clock: u64,
    /// Horizontal timing
    pub h_timing: DisplayTiming,
    /// Vertical timing
    pub v_timing: DisplayTiming,
    /// Mode flags
    pub flags: DisplayModeFlags,
}

impl DisplayMode {
    /// Returns the refresh rate in Hz
    #[inline]
    pub fn refresh_rate_hz(&self) -> f32 {
        self.refresh_rate_mhz as f32 / 1000.0
    }

    /// Returns frame time in milliseconds
    #[inline]
    pub fn frame_time_ms(&self) -> f32 {
        1000.0 / self.refresh_rate_hz()
    }

    /// Returns the required bandwidth in bits per second
    #[inline]
    pub const fn bandwidth_bps(&self, bits_per_pixel: u32) -> u64 {
        self.resolution.pixel_count() * bits_per_pixel as u64 * self.refresh_rate_mhz as u64 / 1000
    }

    /// Creates a standard mode
    #[inline]
    pub const fn standard(resolution: Resolution, refresh_mhz: u32) -> Self {
        Self {
            handle: DisplayModeHandle::NULL,
            resolution,
            refresh_rate_mhz: refresh_mhz,
            pixel_clock: 0,
            h_timing: DisplayTiming::default_const(),
            v_timing: DisplayTiming::default_const(),
            flags: DisplayModeFlags::PREFERRED,
        }
    }

    /// 1080p @ 60 Hz
    pub const FHD_60: Self = Self::standard(Resolution::HD_1080P, 60000);
    /// 1080p @ 120 Hz
    pub const FHD_120: Self = Self::standard(Resolution::HD_1080P, 120000);
    /// 1080p @ 144 Hz
    pub const FHD_144: Self = Self::standard(Resolution::HD_1080P, 144000);
    /// 1440p @ 60 Hz
    pub const QHD_60: Self = Self::standard(Resolution::QHD_1440P, 60000);
    /// 1440p @ 144 Hz
    pub const QHD_144: Self = Self::standard(Resolution::QHD_1440P, 144000);
    /// 1440p @ 165 Hz
    pub const QHD_165: Self = Self::standard(Resolution::QHD_1440P, 165000);
    /// 4K @ 60 Hz
    pub const UHD_60: Self = Self::standard(Resolution::UHD_4K, 60000);
    /// 4K @ 120 Hz
    pub const UHD_120: Self = Self::standard(Resolution::UHD_4K, 120000);
}

/// Display timing parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DisplayTiming {
    /// Active pixels/lines
    pub active: u32,
    /// Front porch
    pub front_porch: u32,
    /// Sync width
    pub sync_width: u32,
    /// Back porch
    pub back_porch: u32,
    /// Sync polarity (true = positive)
    pub sync_positive: bool,
}

impl DisplayTiming {
    /// Creates a new timing
    #[inline]
    pub const fn new(active: u32, front_porch: u32, sync_width: u32, back_porch: u32) -> Self {
        Self {
            active,
            front_porch,
            sync_width,
            back_porch,
            sync_positive: true,
        }
    }

    /// Const default for use in const contexts
    #[inline]
    pub const fn default_const() -> Self {
        Self {
            active: 0,
            front_porch: 0,
            sync_width: 0,
            back_porch: 0,
            sync_positive: true,
        }
    }

    /// Returns the total size
    #[inline]
    pub const fn total(&self) -> u32 {
        self.active + self.front_porch + self.sync_width + self.back_porch
    }

    /// Returns the blanking period
    #[inline]
    pub const fn blanking(&self) -> u32 {
        self.front_porch + self.sync_width + self.back_porch
    }
}

/// Display mode flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DisplayModeFlags(pub u32);

impl DisplayModeFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Preferred mode
    pub const PREFERRED: Self = Self(1 << 0);
    /// Current mode
    pub const CURRENT: Self = Self(1 << 1);
    /// Stereo 3D
    pub const STEREO_3D: Self = Self(1 << 2);
    /// Interlaced
    pub const INTERLACED: Self = Self(1 << 3);
    /// Double scan
    pub const DOUBLE_SCAN: Self = Self(1 << 4);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// Display Configuration
// ============================================================================

/// Display configuration for multi-monitor setup
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DisplayConfiguration {
    /// Display handle
    pub display: DisplayHandle,
    /// Mode to use
    pub mode: DisplayModeHandle,
    /// Position in virtual desktop
    pub position: DisplayPosition,
    /// Rotation
    pub rotation: DisplayRotation,
    /// Scaling factor
    pub scale: f32,
    /// Is primary display
    pub is_primary: bool,
    /// Is enabled
    pub is_enabled: bool,
    /// Color profile
    pub color_profile: Option<ColorProfileHandle>,
    /// HDR mode
    pub hdr_mode: HdrMode,
}

impl DisplayConfiguration {
    /// Creates a new configuration
    #[inline]
    pub const fn new(display: DisplayHandle, mode: DisplayModeHandle) -> Self {
        Self {
            display,
            mode,
            position: DisplayPosition { x: 0, y: 0 },
            rotation: DisplayRotation::Normal,
            scale: 1.0,
            is_primary: false,
            is_enabled: true,
            color_profile: None,
            hdr_mode: HdrMode::Off,
        }
    }

    /// Sets as primary display
    #[inline]
    pub const fn primary(mut self) -> Self {
        self.is_primary = true;
        self
    }

    /// Sets position
    #[inline]
    pub const fn at_position(mut self, x: i32, y: i32) -> Self {
        self.position = DisplayPosition { x, y };
        self
    }

    /// Sets rotation
    #[inline]
    pub const fn with_rotation(mut self, rotation: DisplayRotation) -> Self {
        self.rotation = rotation;
        self
    }

    /// Sets scale
    #[inline]
    pub const fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Enables HDR
    #[inline]
    pub const fn with_hdr(mut self, mode: HdrMode) -> Self {
        self.hdr_mode = mode;
        self
    }
}

/// Display position in virtual desktop
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DisplayPosition {
    /// X coordinate
    pub x: i32,
    /// Y coordinate
    pub y: i32,
}

impl DisplayPosition {
    /// Origin position
    pub const ORIGIN: Self = Self { x: 0, y: 0 };

    /// Creates a new position
    #[inline]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Position to the right of another display
    #[inline]
    pub const fn right_of(width: u32) -> Self {
        Self {
            x: width as i32,
            y: 0,
        }
    }

    /// Position below another display
    #[inline]
    pub const fn below(height: u32) -> Self {
        Self {
            x: 0,
            y: height as i32,
        }
    }
}

/// Display rotation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DisplayRotation {
    /// Normal (0 degrees)
    #[default]
    Normal    = 0,
    /// 90 degrees clockwise
    Rotate90  = 1,
    /// 180 degrees
    Rotate180 = 2,
    /// 270 degrees clockwise (90 CCW)
    Rotate270 = 3,
}

impl DisplayRotation {
    /// Returns the angle in degrees
    #[inline]
    pub const fn degrees(&self) -> u32 {
        match self {
            Self::Normal => 0,
            Self::Rotate90 => 90,
            Self::Rotate180 => 180,
            Self::Rotate270 => 270,
        }
    }

    /// Checks if dimensions are swapped
    #[inline]
    pub const fn swaps_dimensions(&self) -> bool {
        matches!(self, Self::Rotate90 | Self::Rotate270)
    }
}

/// Color profile handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ColorProfileHandle(pub u64);

impl ColorProfileHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

/// HDR mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HdrMode {
    /// HDR off
    #[default]
    Off         = 0,
    /// HDR10
    Hdr10       = 1,
    /// HDR10+
    Hdr10Plus   = 2,
    /// Dolby Vision
    DolbyVision = 3,
    /// HLG
    Hlg         = 4,
}

// ============================================================================
// Display Plane
// ============================================================================

/// Display plane properties
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DisplayPlane {
    /// Plane index
    pub index: u32,
    /// Supported displays
    pub supported_displays: Vec<DisplayHandle>,
    /// Current display
    pub current_display: DisplayHandle,
    /// Current z-order
    pub current_z: u32,
    /// Plane capabilities
    pub capabilities: PlaneCapabilities,
}

/// Plane capabilities
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PlaneCapabilities {
    /// Supported alpha modes
    pub supported_alpha: PlaneAlphaFlags,
    /// Minimum source size
    pub min_src_size: Resolution,
    /// Maximum source size
    pub max_src_size: Resolution,
    /// Minimum destination size
    pub min_dst_size: Resolution,
    /// Maximum destination size
    pub max_dst_size: Resolution,
    /// Minimum source position
    pub min_src_position: DisplayPosition,
    /// Maximum source position
    pub max_src_position: DisplayPosition,
    /// Supported transforms
    pub supported_transforms: SurfaceTransformFlags,
}

/// Plane alpha flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PlaneAlphaFlags(pub u32);

impl PlaneAlphaFlags {
    /// Opaque
    pub const OPAQUE: Self = Self(1 << 0);
    /// Global alpha
    pub const GLOBAL: Self = Self(1 << 1);
    /// Per-pixel alpha
    pub const PER_PIXEL: Self = Self(1 << 2);
    /// Pre-multiplied alpha
    pub const PRE_MULTIPLIED: Self = Self(1 << 3);
    /// Post-multiplied alpha
    pub const POST_MULTIPLIED: Self = Self(1 << 4);

    /// Checks if flag is set
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

// ============================================================================
// VRR (Variable Refresh Rate)
// ============================================================================

/// Variable refresh rate capabilities
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct VrrCapabilities {
    /// VRR is supported
    pub supported: bool,
    /// Minimum refresh rate in mHz
    pub min_refresh_mhz: u32,
    /// Maximum refresh rate in mHz
    pub max_refresh_mhz: u32,
    /// Supports AMD FreeSync
    pub freesync: bool,
    /// FreeSync tier
    pub freesync_tier: FreeSyncTier,
    /// Supports NVIDIA G-Sync
    pub gsync: bool,
    /// G-Sync type
    pub gsync_type: GSyncType,
    /// Supports HDMI VRR
    pub hdmi_vrr: bool,
}

impl VrrCapabilities {
    /// SDR display without VRR
    pub const fn none() -> Self {
        Self {
            supported: false,
            min_refresh_mhz: 0,
            max_refresh_mhz: 0,
            freesync: false,
            freesync_tier: FreeSyncTier::None,
            gsync: false,
            gsync_type: GSyncType::None,
            hdmi_vrr: false,
        }
    }

    /// Returns the VRR range in Hz
    #[inline]
    pub fn range_hz(&self) -> (f32, f32) {
        (
            self.min_refresh_mhz as f32 / 1000.0,
            self.max_refresh_mhz as f32 / 1000.0,
        )
    }
}

/// FreeSync tier
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FreeSyncTier {
    /// No FreeSync
    #[default]
    None       = 0,
    /// Basic FreeSync
    Basic      = 1,
    /// FreeSync Premium
    Premium    = 2,
    /// FreeSync Premium Pro (HDR)
    PremiumPro = 3,
}

/// G-Sync type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GSyncType {
    /// No G-Sync
    #[default]
    None       = 0,
    /// G-Sync Compatible (Adaptive-Sync)
    Compatible = 1,
    /// G-Sync (dedicated module)
    Native     = 2,
    /// G-Sync Ultimate
    Ultimate   = 3,
}

// ============================================================================
// Display Output
// ============================================================================

/// Display output configuration
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DisplayOutput {
    /// Display handle
    pub display: DisplayHandle,
    /// Output format
    pub format: OutputFormat,
    /// Color depth
    pub color_depth: ColorDepth,
    /// HDR metadata
    pub hdr_metadata: Option<HdrMetadata>,
    /// VRR enabled
    pub vrr_enabled: bool,
    /// Target frame rate (for VRR)
    pub target_fps: u32,
}

impl DisplayOutput {
    /// Creates a new output
    #[inline]
    pub const fn new(display: DisplayHandle) -> Self {
        Self {
            display,
            format: OutputFormat::Rgb,
            color_depth: ColorDepth::Bit8,
            hdr_metadata: None,
            vrr_enabled: false,
            target_fps: 60,
        }
    }

    /// Enables HDR10
    #[inline]
    pub fn with_hdr10(mut self, metadata: HdrMetadata) -> Self {
        self.format = OutputFormat::Rgb;
        self.color_depth = ColorDepth::Bit10;
        self.hdr_metadata = Some(metadata);
        self
    }

    /// Enables VRR
    #[inline]
    pub const fn with_vrr(mut self, target_fps: u32) -> Self {
        self.vrr_enabled = true;
        self.target_fps = target_fps;
        self
    }
}

/// Output format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OutputFormat {
    /// RGB
    #[default]
    Rgb      = 0,
    /// YCbCr 4:4:4
    Ycbcr444 = 1,
    /// YCbCr 4:2:2
    Ycbcr422 = 2,
    /// YCbCr 4:2:0
    Ycbcr420 = 3,
}

/// Color depth
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ColorDepth {
    /// 8 bits per channel
    #[default]
    Bit8  = 8,
    /// 10 bits per channel
    Bit10 = 10,
    /// 12 bits per channel
    Bit12 = 12,
    /// 16 bits per channel
    Bit16 = 16,
}

impl ColorDepth {
    /// Returns the bits per pixel for RGB
    #[inline]
    pub const fn bpp_rgb(&self) -> u32 {
        match self {
            Self::Bit8 => 24,
            Self::Bit10 => 30,
            Self::Bit12 => 36,
            Self::Bit16 => 48,
        }
    }
}

/// HDR metadata for output
#[derive(Clone, Debug)]
#[repr(C)]
pub struct HdrMetadata {
    /// Display primaries (red xy, green xy, blue xy)
    pub display_primaries: [(f32, f32); 3],
    /// White point xy
    pub white_point: (f32, f32),
    /// Maximum display luminance (nits)
    pub max_luminance: f32,
    /// Minimum display luminance (nits)
    pub min_luminance: f32,
    /// Maximum content light level (nits)
    pub max_cll: u32,
    /// Maximum frame-average light level (nits)
    pub max_fall: u32,
}

impl HdrMetadata {
    /// Standard HDR10 metadata for BT.2020
    #[inline]
    pub const fn bt2020_hdr10() -> Self {
        Self {
            display_primaries: [
                (0.708, 0.292), // Red
                (0.170, 0.797), // Green
                (0.131, 0.046), // Blue
            ],
            white_point: (0.3127, 0.3290), // D65
            max_luminance: 1000.0,
            min_luminance: 0.0001,
            max_cll: 1000,
            max_fall: 400,
        }
    }

    /// Display P3 metadata
    #[inline]
    pub const fn display_p3() -> Self {
        Self {
            display_primaries: [
                (0.680, 0.320), // Red
                (0.265, 0.690), // Green
                (0.150, 0.060), // Blue
            ],
            white_point: (0.3127, 0.3290), // D65
            max_luminance: 500.0,
            min_luminance: 0.001,
            max_cll: 500,
            max_fall: 250,
        }
    }
}

impl Default for HdrMetadata {
    fn default() -> Self {
        Self::bt2020_hdr10()
    }
}
