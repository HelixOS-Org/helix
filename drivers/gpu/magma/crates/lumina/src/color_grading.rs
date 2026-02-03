//! Color Grading Types for Lumina
//!
//! This module provides color grading infrastructure including
//! LUTs, color curves, and cinematic color correction.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Color Grading Handles
// ============================================================================

/// LUT handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LutHandle(pub u64);

impl LutHandle {
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

impl Default for LutHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Color grading preset handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ColorGradingPresetHandle(pub u64);

impl ColorGradingPresetHandle {
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

impl Default for ColorGradingPresetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// LUT Configuration
// ============================================================================

/// LUT create info
#[derive(Clone, Debug)]
pub struct LutCreateInfo {
    /// Name
    pub name: String,
    /// LUT size (typically 16, 32, or 64)
    pub size: u32,
    /// LUT format
    pub format: LutFormat,
    /// Source data
    pub data: Vec<u8>,
}

impl LutCreateInfo {
    /// Creates info
    pub fn new(size: u32) -> Self {
        Self {
            name: String::new(),
            size,
            format: LutFormat::Rgb10A2,
            data: Vec::new(),
        }
    }

    /// Standard size (32x32x32)
    pub fn standard() -> Self {
        Self::new(32)
    }

    /// High quality (64x64x64)
    pub fn high_quality() -> Self {
        Self::new(64)
    }

    /// With data
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Memory size
    pub fn memory_size(&self) -> u64 {
        let voxels = (self.size as u64).pow(3);
        voxels * (self.format.bytes_per_voxel() as u64)
    }
}

impl Default for LutCreateInfo {
    fn default() -> Self {
        Self::standard()
    }
}

/// LUT format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LutFormat {
    /// RGB10A2 (10 bits per channel)
    #[default]
    Rgb10A2     = 0,
    /// RGBA16 float
    Rgba16Float = 1,
    /// RGBA32 float
    Rgba32Float = 2,
    /// RGBA8
    Rgba8       = 3,
}

impl LutFormat {
    /// Bytes per voxel
    pub const fn bytes_per_voxel(&self) -> u32 {
        match self {
            Self::Rgb10A2 => 4,
            Self::Rgba16Float => 8,
            Self::Rgba32Float => 16,
            Self::Rgba8 => 4,
        }
    }
}

// ============================================================================
// Color Grading Settings
// ============================================================================

/// Color grading settings
#[derive(Clone, Debug)]
pub struct ColorGradingSettings {
    /// White balance
    pub white_balance: WhiteBalance,
    /// Exposure
    pub exposure: ExposureSettings,
    /// Contrast
    pub contrast: ContrastSettings,
    /// Color adjustment
    pub color: ColorAdjustment,
    /// Shadows, midtones, highlights
    pub smh: ShadowMidtoneHighlight,
    /// Channel mixer
    pub channel_mixer: ChannelMixer,
    /// Color curves
    pub curves: ColorCurves,
    /// Split toning
    pub split_toning: SplitToning,
}

impl ColorGradingSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            white_balance: WhiteBalance::default(),
            exposure: ExposureSettings::default(),
            contrast: ContrastSettings::default(),
            color: ColorAdjustment::default(),
            smh: ShadowMidtoneHighlight::default(),
            channel_mixer: ChannelMixer::default(),
            curves: ColorCurves::default(),
            split_toning: SplitToning::default(),
        }
    }

    /// Neutral (no adjustments)
    pub fn neutral() -> Self {
        Self::new()
    }

    /// Cinematic look
    pub fn cinematic() -> Self {
        Self {
            contrast: ContrastSettings {
                contrast: 1.1,
                ..Default::default()
            },
            color: ColorAdjustment {
                saturation: 0.95,
                ..Default::default()
            },
            smh: ShadowMidtoneHighlight {
                shadows_color: [0.0, 0.0, 0.1],
                highlights_color: [0.1, 0.05, 0.0],
                ..Default::default()
            },
            split_toning: SplitToning {
                shadows_hue: 0.6,
                shadows_saturation: 0.1,
                highlights_hue: 0.1,
                highlights_saturation: 0.05,
                balance: 0.0,
            },
            ..Self::new()
        }
    }

    /// Desaturated look
    pub fn desaturated() -> Self {
        Self {
            color: ColorAdjustment {
                saturation: 0.5,
                ..Default::default()
            },
            contrast: ContrastSettings {
                contrast: 1.15,
                ..Default::default()
            },
            ..Self::new()
        }
    }

    /// Warm look
    pub fn warm() -> Self {
        Self {
            white_balance: WhiteBalance {
                temperature: 6500.0,
                tint: 0.0,
            },
            color: ColorAdjustment {
                saturation: 1.1,
                ..Default::default()
            },
            ..Self::new()
        }
    }

    /// Cool look
    pub fn cool() -> Self {
        Self {
            white_balance: WhiteBalance {
                temperature: 4500.0,
                tint: 0.0,
            },
            ..Self::new()
        }
    }
}

impl Default for ColorGradingSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// White Balance
// ============================================================================

/// White balance settings
#[derive(Clone, Copy, Debug)]
pub struct WhiteBalance {
    /// Temperature (Kelvin, 1000-40000)
    pub temperature: f32,
    /// Tint (-100 to 100)
    pub tint: f32,
}

impl WhiteBalance {
    /// Daylight
    pub const fn daylight() -> Self {
        Self {
            temperature: 5600.0,
            tint: 0.0,
        }
    }

    /// Tungsten
    pub const fn tungsten() -> Self {
        Self {
            temperature: 3200.0,
            tint: 0.0,
        }
    }

    /// Fluorescent
    pub const fn fluorescent() -> Self {
        Self {
            temperature: 4000.0,
            tint: 10.0,
        }
    }

    /// Cloudy
    pub const fn cloudy() -> Self {
        Self {
            temperature: 6500.0,
            tint: 0.0,
        }
    }

    /// Shade
    pub const fn shade() -> Self {
        Self {
            temperature: 7500.0,
            tint: 0.0,
        }
    }

    /// Temperature to RGB multipliers
    pub fn to_rgb_multipliers(&self) -> [f32; 3] {
        // Simplified Planckian locus approximation
        let temp = self.temperature.clamp(1000.0, 40000.0);

        let r = if temp <= 6600.0 {
            1.0
        } else {
            1.29293618606274 * ((temp / 100.0 - 60.0).powf(-0.1332047592))
        };

        let g = if temp <= 6600.0 {
            0.390081578769573 * (temp / 100.0 - 10.0).ln() - 0.631841443788627
        } else {
            1.12989086089529 * ((temp / 100.0 - 60.0).powf(-0.0755148492))
        };

        let b = if temp >= 6600.0 {
            1.0
        } else if temp <= 1900.0 {
            0.0
        } else {
            0.543206789110196 * (temp / 100.0 - 10.0).ln() - 1.19625408914
        };

        [r.clamp(0.0, 2.0), g.clamp(0.0, 2.0), b.clamp(0.0, 2.0)]
    }
}

impl Default for WhiteBalance {
    fn default() -> Self {
        Self::daylight()
    }
}

// ============================================================================
// Exposure Settings
// ============================================================================

/// Exposure settings
#[derive(Clone, Copy, Debug)]
pub struct ExposureSettings {
    /// Exposure compensation (EV)
    pub exposure: f32,
    /// Post-exposure (applied after tonemapping)
    pub post_exposure: f32,
}

impl ExposureSettings {
    /// Default exposure
    pub const fn default_exposure() -> Self {
        Self {
            exposure: 0.0,
            post_exposure: 0.0,
        }
    }

    /// With exposure
    pub const fn with_exposure(mut self, ev: f32) -> Self {
        self.exposure = ev;
        self
    }
}

impl Default for ExposureSettings {
    fn default() -> Self {
        Self::default_exposure()
    }
}

// ============================================================================
// Contrast Settings
// ============================================================================

/// Contrast settings
#[derive(Clone, Copy, Debug)]
pub struct ContrastSettings {
    /// Contrast multiplier (1.0 = no change)
    pub contrast: f32,
    /// Pivot point (0.18 = middle gray)
    pub pivot: f32,
}

impl ContrastSettings {
    /// Default contrast
    pub const fn default_contrast() -> Self {
        Self {
            contrast: 1.0,
            pivot: 0.18,
        }
    }

    /// High contrast
    pub const fn high() -> Self {
        Self {
            contrast: 1.25,
            pivot: 0.18,
        }
    }

    /// Low contrast
    pub const fn low() -> Self {
        Self {
            contrast: 0.8,
            pivot: 0.18,
        }
    }
}

impl Default for ContrastSettings {
    fn default() -> Self {
        Self::default_contrast()
    }
}

// ============================================================================
// Color Adjustment
// ============================================================================

/// Color adjustment
#[derive(Clone, Copy, Debug)]
pub struct ColorAdjustment {
    /// Saturation (1.0 = no change)
    pub saturation: f32,
    /// Vibrance (affects less saturated colors more)
    pub vibrance: f32,
    /// Hue shift (-1 to 1, in rotations)
    pub hue_shift: f32,
}

impl ColorAdjustment {
    /// Default adjustment
    pub const fn default_adjustment() -> Self {
        Self {
            saturation: 1.0,
            vibrance: 0.0,
            hue_shift: 0.0,
        }
    }

    /// Vibrant
    pub const fn vibrant() -> Self {
        Self {
            saturation: 1.0,
            vibrance: 0.3,
            hue_shift: 0.0,
        }
    }

    /// Muted
    pub const fn muted() -> Self {
        Self {
            saturation: 0.7,
            vibrance: -0.2,
            hue_shift: 0.0,
        }
    }
}

impl Default for ColorAdjustment {
    fn default() -> Self {
        Self::default_adjustment()
    }
}

// ============================================================================
// Shadow/Midtone/Highlight
// ============================================================================

/// Shadow, midtone, highlight adjustment
#[derive(Clone, Copy, Debug)]
pub struct ShadowMidtoneHighlight {
    /// Shadows lift
    pub shadows_lift: f32,
    /// Shadows color tint
    pub shadows_color: [f32; 3],
    /// Midtones gamma
    pub midtones_gamma: f32,
    /// Midtones color tint
    pub midtones_color: [f32; 3],
    /// Highlights gain
    pub highlights_gain: f32,
    /// Highlights color tint
    pub highlights_color: [f32; 3],
}

impl ShadowMidtoneHighlight {
    /// Default SMH
    pub const fn default_smh() -> Self {
        Self {
            shadows_lift: 0.0,
            shadows_color: [0.0, 0.0, 0.0],
            midtones_gamma: 1.0,
            midtones_color: [0.0, 0.0, 0.0],
            highlights_gain: 1.0,
            highlights_color: [0.0, 0.0, 0.0],
        }
    }
}

impl Default for ShadowMidtoneHighlight {
    fn default() -> Self {
        Self::default_smh()
    }
}

// ============================================================================
// Channel Mixer
// ============================================================================

/// Channel mixer
#[derive(Clone, Copy, Debug)]
pub struct ChannelMixer {
    /// Red output
    pub red: [f32; 3],
    /// Green output
    pub green: [f32; 3],
    /// Blue output
    pub blue: [f32; 3],
}

impl ChannelMixer {
    /// Identity mixer
    pub const fn identity() -> Self {
        Self {
            red: [1.0, 0.0, 0.0],
            green: [0.0, 1.0, 0.0],
            blue: [0.0, 0.0, 1.0],
        }
    }

    /// Grayscale (luminance)
    pub const fn grayscale() -> Self {
        Self {
            red: [0.2126, 0.7152, 0.0722],
            green: [0.2126, 0.7152, 0.0722],
            blue: [0.2126, 0.7152, 0.0722],
        }
    }

    /// Sepia
    pub const fn sepia() -> Self {
        Self {
            red: [0.393, 0.769, 0.189],
            green: [0.349, 0.686, 0.168],
            blue: [0.272, 0.534, 0.131],
        }
    }

    /// To 3x3 matrix
    pub const fn to_matrix(&self) -> [[f32; 3]; 3] {
        [self.red, self.green, self.blue]
    }
}

impl Default for ChannelMixer {
    fn default() -> Self {
        Self::identity()
    }
}

// ============================================================================
// Color Curves
// ============================================================================

/// Color curves
#[derive(Clone, Debug)]
pub struct ColorCurves {
    /// Master curve
    pub master: CurveData,
    /// Red curve
    pub red: CurveData,
    /// Green curve
    pub green: CurveData,
    /// Blue curve
    pub blue: CurveData,
    /// Hue vs Hue
    pub hue_vs_hue: CurveData,
    /// Hue vs Saturation
    pub hue_vs_sat: CurveData,
    /// Saturation vs Saturation
    pub sat_vs_sat: CurveData,
}

impl ColorCurves {
    /// Creates default curves
    pub fn new() -> Self {
        Self {
            master: CurveData::linear(),
            red: CurveData::linear(),
            green: CurveData::linear(),
            blue: CurveData::linear(),
            hue_vs_hue: CurveData::linear(),
            hue_vs_sat: CurveData::linear(),
            sat_vs_sat: CurveData::linear(),
        }
    }
}

impl Default for ColorCurves {
    fn default() -> Self {
        Self::new()
    }
}

/// Curve data
#[derive(Clone, Debug)]
pub struct CurveData {
    /// Control points (x, y pairs)
    pub points: Vec<[f32; 2]>,
    /// LUT resolution
    pub lut_resolution: u32,
}

impl CurveData {
    /// Linear curve (no adjustment)
    pub fn linear() -> Self {
        Self {
            points: vec![[0.0, 0.0], [1.0, 1.0]],
            lut_resolution: 256,
        }
    }

    /// S-curve (adds contrast)
    pub fn s_curve(strength: f32) -> Self {
        let s = strength * 0.25;
        Self {
            points: vec![
                [0.0, 0.0],
                [0.25, 0.25 - s],
                [0.5, 0.5],
                [0.75, 0.75 + s],
                [1.0, 1.0],
            ],
            lut_resolution: 256,
        }
    }

    /// Add point
    pub fn with_point(mut self, x: f32, y: f32) -> Self {
        self.points.push([x, y]);
        self.points.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
        self
    }

    /// Evaluate curve at x
    pub fn evaluate(&self, x: f32) -> f32 {
        if self.points.is_empty() {
            return x;
        }

        let x = x.clamp(0.0, 1.0);

        // Find surrounding points
        let mut i = 0;
        while i < self.points.len() - 1 && self.points[i + 1][0] < x {
            i += 1;
        }

        if i >= self.points.len() - 1 {
            return self.points.last().map(|p| p[1]).unwrap_or(x);
        }

        // Linear interpolation
        let p0 = self.points[i];
        let p1 = self.points[i + 1];
        let t = (x - p0[0]) / (p1[0] - p0[0]).max(0.0001);
        p0[1] + t * (p1[1] - p0[1])
    }
}

impl Default for CurveData {
    fn default() -> Self {
        Self::linear()
    }
}

// ============================================================================
// Split Toning
// ============================================================================

/// Split toning
#[derive(Clone, Copy, Debug)]
pub struct SplitToning {
    /// Shadows hue (0-1)
    pub shadows_hue: f32,
    /// Shadows saturation (0-1)
    pub shadows_saturation: f32,
    /// Highlights hue (0-1)
    pub highlights_hue: f32,
    /// Highlights saturation (0-1)
    pub highlights_saturation: f32,
    /// Balance (-1 to 1)
    pub balance: f32,
}

impl SplitToning {
    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            shadows_hue: 0.0,
            shadows_saturation: 0.0,
            highlights_hue: 0.0,
            highlights_saturation: 0.0,
            balance: 0.0,
        }
    }

    /// Orange/teal
    pub const fn orange_teal() -> Self {
        Self {
            shadows_hue: 0.55, // Teal
            shadows_saturation: 0.15,
            highlights_hue: 0.08, // Orange
            highlights_saturation: 0.1,
            balance: 0.0,
        }
    }

    /// Vintage
    pub const fn vintage() -> Self {
        Self {
            shadows_hue: 0.6, // Blue
            shadows_saturation: 0.1,
            highlights_hue: 0.15, // Yellow
            highlights_saturation: 0.15,
            balance: -0.2,
        }
    }
}

impl Default for SplitToning {
    fn default() -> Self {
        Self::disabled()
    }
}

// ============================================================================
// Color Grading GPU Data
// ============================================================================

/// Color grading GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ColorGradingGpuParams {
    /// White balance multipliers
    pub white_balance: [f32; 4],
    /// Exposure (linear multiplier)
    pub exposure: f32,
    /// Contrast
    pub contrast: f32,
    /// Contrast pivot
    pub contrast_pivot: f32,
    /// Saturation
    pub saturation: f32,
    /// Vibrance
    pub vibrance: f32,
    /// Hue shift
    pub hue_shift: f32,
    /// Padding
    pub _padding0: [f32; 2],
    /// Channel mixer matrix (row major)
    pub channel_mixer: [[f32; 4]; 3],
    /// Shadows lift
    pub shadows_lift: [f32; 4],
    /// Midtones gamma
    pub midtones_gamma: [f32; 4],
    /// Highlights gain
    pub highlights_gain: [f32; 4],
    /// Split toning shadows
    pub split_shadows: [f32; 4],
    /// Split toning highlights
    pub split_highlights: [f32; 4],
    /// Split toning balance
    pub split_balance: f32,
    /// Flags
    pub flags: u32,
    /// Padding
    pub _padding1: [f32; 2],
}

impl ColorGradingGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Use LUT flag
    pub const FLAG_USE_LUT: u32 = 1 << 0;
    /// Use curves flag
    pub const FLAG_USE_CURVES: u32 = 1 << 1;
    /// Use split toning flag
    pub const FLAG_USE_SPLIT_TONING: u32 = 1 << 2;
}

// ============================================================================
// Statistics
// ============================================================================

/// Color grading statistics
#[derive(Clone, Debug, Default)]
pub struct ColorGradingStats {
    /// LUTs loaded
    pub lut_count: u32,
    /// Grading pass time (microseconds)
    pub pass_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
