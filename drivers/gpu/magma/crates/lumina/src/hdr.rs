//! HDR Types for Lumina
//!
//! This module provides HDR (High Dynamic Range) display and output
//! infrastructure including tone mapping curves, color spaces, and metadata.

extern crate alloc;

use alloc::string::String;

// ============================================================================
// HDR Handles
// ============================================================================

/// HDR output handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HdrOutputHandle(pub u64);

impl HdrOutputHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Creates new handle
    #[inline]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for HdrOutputHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// HDR LUT handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HdrLutHandle(pub u64);

impl HdrLutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for HdrLutHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Color Space
// ============================================================================

/// HDR color space
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HdrColorSpace {
    /// sRGB (SDR)
    #[default]
    Srgb      = 0,
    /// scRGB (linear, extended range)
    ScRgb     = 1,
    /// Rec. 709 (HD)
    Rec709    = 2,
    /// Rec. 2020 (UHD/HDR)
    Rec2020   = 3,
    /// DCI-P3
    DciP3     = 4,
    /// Display P3
    DisplayP3 = 5,
    /// ACEScg
    AcesCg    = 6,
    /// ACES 2065-1
    Aces2065  = 7,
    /// Adobe RGB
    AdobeRgb  = 8,
}

impl HdrColorSpace {
    /// Is HDR capable color space
    pub const fn is_hdr(&self) -> bool {
        !matches!(self, Self::Srgb | Self::Rec709)
    }

    /// White point D65
    pub const fn white_point(&self) -> [f32; 2] {
        // Most use D65
        match self {
            Self::DciP3 => [0.314, 0.351], // DCI white
            _ => [0.3127, 0.3290],         // D65
        }
    }

    /// Red primary
    pub const fn red_primary(&self) -> [f32; 2] {
        match self {
            Self::Rec2020 => [0.708, 0.292],
            Self::DciP3 | Self::DisplayP3 => [0.680, 0.320],
            Self::AcesCg | Self::Aces2065 => [0.713, 0.293],
            Self::AdobeRgb => [0.640, 0.330],
            _ => [0.640, 0.330], // Rec.709/sRGB
        }
    }

    /// Green primary
    pub const fn green_primary(&self) -> [f32; 2] {
        match self {
            Self::Rec2020 => [0.170, 0.797],
            Self::DciP3 | Self::DisplayP3 => [0.265, 0.690],
            Self::AcesCg | Self::Aces2065 => [0.165, 0.830],
            Self::AdobeRgb => [0.210, 0.710],
            _ => [0.300, 0.600], // Rec.709/sRGB
        }
    }

    /// Blue primary
    pub const fn blue_primary(&self) -> [f32; 2] {
        match self {
            Self::Rec2020 => [0.131, 0.046],
            Self::DciP3 | Self::DisplayP3 => [0.150, 0.060],
            Self::AcesCg | Self::Aces2065 => [0.128, 0.044],
            Self::AdobeRgb => [0.150, 0.060],
            _ => [0.150, 0.060], // Rec.709/sRGB
        }
    }
}

// ============================================================================
// Transfer Function
// ============================================================================

/// HDR transfer function (EOTF/OETF)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HdrTransferFunction {
    /// sRGB (2.2 gamma with linear segment)
    #[default]
    Srgb    = 0,
    /// Linear
    Linear  = 1,
    /// Gamma 2.2
    Gamma22 = 2,
    /// Gamma 2.4
    Gamma24 = 3,
    /// PQ (SMPTE ST 2084)
    Pq      = 4,
    /// HLG (Hybrid Log-Gamma)
    Hlg     = 5,
    /// BT.1886 (Rec.709 display)
    Bt1886  = 6,
    /// ACEScct
    AcesCct = 7,
    /// ACEScc
    AcesCc  = 8,
    /// Log C (ARRI)
    LogC    = 9,
    /// S-Log3 (Sony)
    SLog3   = 10,
}

impl HdrTransferFunction {
    /// Is HDR transfer function
    pub const fn is_hdr(&self) -> bool {
        matches!(self, Self::Pq | Self::Hlg)
    }

    /// Is log encoding
    pub const fn is_log(&self) -> bool {
        matches!(
            self,
            Self::AcesCct | Self::AcesCc | Self::LogC | Self::SLog3
        )
    }

    /// Max luminance (nits)
    pub const fn max_luminance(&self) -> f32 {
        match self {
            Self::Pq => 10000.0,
            Self::Hlg => 1000.0,
            Self::AcesCct | Self::AcesCc => 65504.0,
            _ => 100.0, // SDR reference white
        }
    }

    /// Reference white (nits)
    pub const fn reference_white(&self) -> f32 {
        match self {
            Self::Pq => 203.0,  // Dolby/SMPTE recommendation
            Self::Hlg => 203.0, // BBC recommendation
            _ => 80.0,          // sRGB standard
        }
    }

    /// Apply EOTF (electrical to optical)
    pub fn apply_eotf(&self, value: f32) -> f32 {
        match self {
            Self::Linear => value,
            Self::Srgb => {
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            },
            Self::Gamma22 => value.powf(2.2),
            Self::Gamma24 => value.powf(2.4),
            Self::Pq => Self::pq_eotf(value),
            Self::Hlg => Self::hlg_eotf(value),
            Self::Bt1886 => value.powf(2.4),
            _ => value.powf(2.2),
        }
    }

    /// Apply OETF (optical to electrical)
    pub fn apply_oetf(&self, value: f32) -> f32 {
        match self {
            Self::Linear => value,
            Self::Srgb => {
                if value <= 0.0031308 {
                    value * 12.92
                } else {
                    1.055 * value.powf(1.0 / 2.4) - 0.055
                }
            },
            Self::Gamma22 => value.powf(1.0 / 2.2),
            Self::Gamma24 => value.powf(1.0 / 2.4),
            Self::Pq => Self::pq_oetf(value),
            Self::Hlg => Self::hlg_oetf(value),
            Self::Bt1886 => value.powf(1.0 / 2.4),
            _ => value.powf(1.0 / 2.2),
        }
    }

    /// PQ EOTF
    fn pq_eotf(e: f32) -> f32 {
        const M1: f32 = 0.1593017578125;
        const M2: f32 = 78.84375;
        const C1: f32 = 0.8359375;
        const C2: f32 = 18.8515625;
        const C3: f32 = 18.6875;

        let e_pow = e.powf(1.0 / M2);
        let num = (e_pow - C1).max(0.0);
        let den = C2 - C3 * e_pow;

        if den > 0.0 {
            (num / den).powf(1.0 / M1) * 10000.0
        } else {
            0.0
        }
    }

    /// PQ OETF
    fn pq_oetf(y: f32) -> f32 {
        const M1: f32 = 0.1593017578125;
        const M2: f32 = 78.84375;
        const C1: f32 = 0.8359375;
        const C2: f32 = 18.8515625;
        const C3: f32 = 18.6875;

        let y_norm = (y / 10000.0).max(0.0);
        let y_pow = y_norm.powf(M1);
        let num = C1 + C2 * y_pow;
        let den = 1.0 + C3 * y_pow;

        (num / den).powf(M2)
    }

    /// HLG EOTF
    fn hlg_eotf(e: f32) -> f32 {
        const A: f32 = 0.17883277;
        const B: f32 = 0.28466892;
        const C: f32 = 0.55991073;

        if e <= 0.5 {
            (e * e) / 3.0
        } else {
            (((e - C) / A).exp() + B) / 12.0
        }
    }

    /// HLG OETF
    fn hlg_oetf(y: f32) -> f32 {
        const A: f32 = 0.17883277;
        const B: f32 = 0.28466892;
        const C: f32 = 0.55991073;

        if y <= 1.0 / 12.0 {
            (3.0 * y).sqrt()
        } else {
            A * (12.0 * y - B).ln() + C
        }
    }
}

// ============================================================================
// HDR Format
// ============================================================================

/// HDR pixel format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HdrFormat {
    /// RGBA16 Float
    #[default]
    Rgba16Float    = 0,
    /// RGBA32 Float
    Rgba32Float    = 1,
    /// RGB10A2 (10-bit)
    Rgb10A2        = 2,
    /// RGBA16 Unorm
    Rgba16Unorm    = 3,
    /// R11G11B10 Float
    R11G11B10Float = 4,
    /// RGB9E5 (shared exponent)
    Rgb9E5Float    = 5,
}

impl HdrFormat {
    /// Bits per component
    pub const fn bits_per_component(&self) -> u32 {
        match self {
            Self::Rgba16Float | Self::Rgba16Unorm => 16,
            Self::Rgba32Float => 32,
            Self::Rgb10A2 => 10,
            Self::R11G11B10Float => 11, // Approximate
            Self::Rgb9E5Float => 9,
        }
    }

    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Rgba16Float | Self::Rgba16Unorm => 8,
            Self::Rgba32Float => 16,
            Self::Rgb10A2 | Self::R11G11B10Float | Self::Rgb9E5Float => 4,
        }
    }

    /// Supports negative values
    pub const fn supports_negative(&self) -> bool {
        matches!(self, Self::Rgba16Float | Self::Rgba32Float)
    }
}

// ============================================================================
// HDR Metadata
// ============================================================================

/// HDR10 static metadata (SMPTE ST 2086)
#[derive(Clone, Copy, Debug)]
pub struct Hdr10Metadata {
    /// Display primaries - red X
    pub red_primary_x: f32,
    /// Display primaries - red Y
    pub red_primary_y: f32,
    /// Display primaries - green X
    pub green_primary_x: f32,
    /// Display primaries - green Y
    pub green_primary_y: f32,
    /// Display primaries - blue X
    pub blue_primary_x: f32,
    /// Display primaries - blue Y
    pub blue_primary_y: f32,
    /// White point X
    pub white_point_x: f32,
    /// White point Y
    pub white_point_y: f32,
    /// Max luminance (nits)
    pub max_luminance: f32,
    /// Min luminance (nits)
    pub min_luminance: f32,
    /// Max content light level (nits)
    pub max_cll: f32,
    /// Max frame average light level (nits)
    pub max_fall: f32,
}

impl Hdr10Metadata {
    /// Creates with DCI-P3 primaries
    pub const fn dci_p3() -> Self {
        Self {
            red_primary_x: 0.680,
            red_primary_y: 0.320,
            green_primary_x: 0.265,
            green_primary_y: 0.690,
            blue_primary_x: 0.150,
            blue_primary_y: 0.060,
            white_point_x: 0.3127,
            white_point_y: 0.3290,
            max_luminance: 1000.0,
            min_luminance: 0.001,
            max_cll: 1000.0,
            max_fall: 400.0,
        }
    }

    /// Creates with Rec. 2020 primaries
    pub const fn rec2020() -> Self {
        Self {
            red_primary_x: 0.708,
            red_primary_y: 0.292,
            green_primary_x: 0.170,
            green_primary_y: 0.797,
            blue_primary_x: 0.131,
            blue_primary_y: 0.046,
            white_point_x: 0.3127,
            white_point_y: 0.3290,
            max_luminance: 1000.0,
            min_luminance: 0.0001,
            max_cll: 1000.0,
            max_fall: 400.0,
        }
    }

    /// Cinema preset (4000 nit peak)
    pub const fn cinema() -> Self {
        Self {
            max_luminance: 4000.0,
            max_cll: 4000.0,
            max_fall: 1000.0,
            ..Self::rec2020()
        }
    }

    /// TV preset (1000 nit peak)
    pub const fn tv() -> Self {
        Self::rec2020()
    }

    /// With max luminance
    pub const fn with_max_luminance(mut self, nits: f32) -> Self {
        self.max_luminance = nits;
        self
    }

    /// With content light levels
    pub const fn with_cll(mut self, max_cll: f32, max_fall: f32) -> Self {
        self.max_cll = max_cll;
        self.max_fall = max_fall;
        self
    }
}

impl Default for Hdr10Metadata {
    fn default() -> Self {
        Self::rec2020()
    }
}

/// Dolby Vision metadata
#[derive(Clone, Copy, Debug)]
pub struct DolbyVisionMetadata {
    /// Profile
    pub profile: u8,
    /// Level
    pub level: u8,
    /// RPU present
    pub rpu_present: bool,
    /// EL present
    pub el_present: bool,
    /// BL present
    pub bl_present: bool,
    /// BL signal compatibility ID
    pub bl_signal_compatibility_id: u8,
}

impl DolbyVisionMetadata {
    /// Profile 5 (single layer PQ)
    pub const fn profile_5() -> Self {
        Self {
            profile: 5,
            level: 6,
            rpu_present: true,
            el_present: false,
            bl_present: true,
            bl_signal_compatibility_id: 0,
        }
    }

    /// Profile 8 (dual layer)
    pub const fn profile_8() -> Self {
        Self {
            profile: 8,
            level: 6,
            rpu_present: true,
            el_present: true,
            bl_present: true,
            bl_signal_compatibility_id: 1,
        }
    }
}

impl Default for DolbyVisionMetadata {
    fn default() -> Self {
        Self::profile_5()
    }
}

// ============================================================================
// HDR Output Configuration
// ============================================================================

/// HDR output create info
#[derive(Clone, Debug)]
pub struct HdrOutputCreateInfo {
    /// Name
    pub name: String,
    /// Color space
    pub color_space: HdrColorSpace,
    /// Transfer function
    pub transfer_function: HdrTransferFunction,
    /// Format
    pub format: HdrFormat,
    /// HDR10 metadata
    pub metadata: Option<Hdr10Metadata>,
    /// Max output luminance (nits)
    pub max_luminance: f32,
    /// Paper white luminance (nits)
    pub paper_white: f32,
}

impl HdrOutputCreateInfo {
    /// Creates SDR config
    pub fn sdr() -> Self {
        Self {
            name: String::new(),
            color_space: HdrColorSpace::Srgb,
            transfer_function: HdrTransferFunction::Srgb,
            format: HdrFormat::Rgb10A2,
            metadata: None,
            max_luminance: 100.0,
            paper_white: 80.0,
        }
    }

    /// Creates HDR10 config
    pub fn hdr10() -> Self {
        Self {
            name: String::new(),
            color_space: HdrColorSpace::Rec2020,
            transfer_function: HdrTransferFunction::Pq,
            format: HdrFormat::Rgb10A2,
            metadata: Some(Hdr10Metadata::rec2020()),
            max_luminance: 1000.0,
            paper_white: 203.0,
        }
    }

    /// Creates scRGB config
    pub fn scrgb() -> Self {
        Self {
            name: String::new(),
            color_space: HdrColorSpace::ScRgb,
            transfer_function: HdrTransferFunction::Linear,
            format: HdrFormat::Rgba16Float,
            metadata: None,
            max_luminance: 10000.0,
            paper_white: 80.0,
        }
    }

    /// Creates Dolby Vision config
    pub fn dolby_vision() -> Self {
        Self {
            name: String::new(),
            color_space: HdrColorSpace::Rec2020,
            transfer_function: HdrTransferFunction::Pq,
            format: HdrFormat::Rgba16Float,
            metadata: Some(Hdr10Metadata::cinema()),
            max_luminance: 4000.0,
            paper_white: 203.0,
        }
    }

    /// With max luminance
    pub fn with_max_luminance(mut self, nits: f32) -> Self {
        self.max_luminance = nits;
        self
    }

    /// With paper white
    pub fn with_paper_white(mut self, nits: f32) -> Self {
        self.paper_white = nits;
        self
    }
}

impl Default for HdrOutputCreateInfo {
    fn default() -> Self {
        Self::sdr()
    }
}

// ============================================================================
// HDR Tone Mapping
// ============================================================================

/// HDR to SDR tone mapping mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HdrTonemapMode {
    /// None (clip)
    None             = 0,
    /// Reinhard
    #[default]
    Reinhard         = 1,
    /// Extended Reinhard
    ReinhardExtended = 2,
    /// Filmic (ACES)
    Aces             = 3,
    /// Uncharted 2
    Uncharted2       = 4,
    /// AgX
    Agx              = 5,
    /// Neutral (Khronos PBR)
    Neutral          = 6,
    /// Custom LUT
    CustomLut        = 7,
}

/// HDR tonemapping settings
#[derive(Clone, Copy, Debug)]
pub struct HdrTonemapSettings {
    /// Mode
    pub mode: HdrTonemapMode,
    /// Input max luminance
    pub input_max_luminance: f32,
    /// Output max luminance
    pub output_max_luminance: f32,
    /// White point
    pub white_point: f32,
    /// Contrast
    pub contrast: f32,
    /// Saturation
    pub saturation: f32,
    /// Shoulder strength
    pub shoulder_strength: f32,
}

impl HdrTonemapSettings {
    /// Default settings
    pub const fn default_tonemap() -> Self {
        Self {
            mode: HdrTonemapMode::Aces,
            input_max_luminance: 10000.0,
            output_max_luminance: 100.0,
            white_point: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            shoulder_strength: 1.0,
        }
    }

    /// For HDR display
    pub const fn hdr_display(max_nits: f32) -> Self {
        Self {
            mode: HdrTonemapMode::None,
            input_max_luminance: max_nits,
            output_max_luminance: max_nits,
            ..Self::default_tonemap()
        }
    }

    /// SDR compatible
    pub const fn sdr_compatible() -> Self {
        Self {
            output_max_luminance: 100.0,
            ..Self::default_tonemap()
        }
    }

    /// With mode
    pub const fn with_mode(mut self, mode: HdrTonemapMode) -> Self {
        self.mode = mode;
        self
    }
}

impl Default for HdrTonemapSettings {
    fn default() -> Self {
        Self::default_tonemap()
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// HDR GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct HdrGpuParams {
    /// Color matrix (3x4)
    pub color_matrix: [[f32; 4]; 3],
    /// Input color space
    pub input_color_space: u32,
    /// Output color space
    pub output_color_space: u32,
    /// Input transfer function
    pub input_transfer: u32,
    /// Output transfer function
    pub output_transfer: u32,
    /// Max input luminance (nits)
    pub max_input_luminance: f32,
    /// Max output luminance (nits)
    pub max_output_luminance: f32,
    /// Paper white (nits)
    pub paper_white: f32,
    /// Tonemap mode
    pub tonemap_mode: u32,
    /// Contrast
    pub contrast: f32,
    /// Saturation
    pub saturation: f32,
    /// White point
    pub white_point: f32,
    /// Shoulder strength
    pub shoulder_strength: f32,
}

impl HdrGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Identity color matrix
    pub const fn identity_matrix() -> [[f32; 4]; 3] {
        [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [
            0.0, 0.0, 1.0, 0.0,
        ]]
    }
}

// ============================================================================
// Display Capabilities
// ============================================================================

/// HDR display capabilities
#[derive(Clone, Debug)]
pub struct HdrDisplayCapabilities {
    /// Supports HDR
    pub hdr_supported: bool,
    /// Supports Dolby Vision
    pub dolby_vision: bool,
    /// Supports HDR10+
    pub hdr10_plus: bool,
    /// Supports HLG
    pub hlg: bool,
    /// Supports 10-bit
    pub supports_10bit: bool,
    /// Supports 12-bit
    pub supports_12bit: bool,
    /// Max luminance (nits)
    pub max_luminance: f32,
    /// Min luminance (nits)
    pub min_luminance: f32,
    /// Max full frame luminance (nits)
    pub max_full_frame_luminance: f32,
    /// Color gamut coverage (% of Rec.2020)
    pub rec2020_coverage: f32,
    /// Color gamut coverage (% of DCI-P3)
    pub dci_p3_coverage: f32,
}

impl HdrDisplayCapabilities {
    /// SDR only
    pub const fn sdr_only() -> Self {
        Self {
            hdr_supported: false,
            dolby_vision: false,
            hdr10_plus: false,
            hlg: false,
            supports_10bit: false,
            supports_12bit: false,
            max_luminance: 300.0,
            min_luminance: 0.5,
            max_full_frame_luminance: 250.0,
            rec2020_coverage: 0.68,
            dci_p3_coverage: 0.92,
        }
    }

    /// HDR TV (typical)
    pub const fn hdr_tv() -> Self {
        Self {
            hdr_supported: true,
            dolby_vision: true,
            hdr10_plus: true,
            hlg: true,
            supports_10bit: true,
            supports_12bit: false,
            max_luminance: 1000.0,
            min_luminance: 0.05,
            max_full_frame_luminance: 500.0,
            rec2020_coverage: 0.72,
            dci_p3_coverage: 0.98,
        }
    }

    /// HDR monitor (high-end)
    pub const fn hdr_monitor() -> Self {
        Self {
            hdr_supported: true,
            dolby_vision: false,
            hdr10_plus: false,
            hlg: false,
            supports_10bit: true,
            supports_12bit: true,
            max_luminance: 1400.0,
            min_luminance: 0.1,
            max_full_frame_luminance: 600.0,
            rec2020_coverage: 0.75,
            dci_p3_coverage: 0.99,
        }
    }
}

impl Default for HdrDisplayCapabilities {
    fn default() -> Self {
        Self::sdr_only()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// HDR statistics
#[derive(Clone, Debug, Default)]
pub struct HdrStats {
    /// Current max luminance
    pub current_max_luminance: f32,
    /// Current average luminance
    pub current_avg_luminance: f32,
    /// Pixels clipped (percentage)
    pub pixels_clipped: f32,
    /// Color volume used (percentage)
    pub color_volume_used: f32,
    /// Processing time (microseconds)
    pub processing_time_us: u64,
}
