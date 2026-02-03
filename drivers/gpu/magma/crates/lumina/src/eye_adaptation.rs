//! Eye Adaptation Types for Lumina
//!
//! This module provides automatic exposure and eye adaptation
//! infrastructure for HDR rendering.

extern crate alloc;

use alloc::string::String;

// ============================================================================
// Eye Adaptation Handles
// ============================================================================

/// Eye adaptation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EyeAdaptationHandle(pub u64);

impl EyeAdaptationHandle {
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

impl Default for EyeAdaptationHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Histogram handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HistogramHandle(pub u64);

impl HistogramHandle {
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

impl Default for HistogramHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Eye Adaptation Configuration
// ============================================================================

/// Eye adaptation create info
#[derive(Clone, Debug)]
pub struct EyeAdaptationCreateInfo {
    /// Name
    pub name: String,
    /// Method
    pub method: AdaptationMethod,
    /// Mode
    pub mode: AdaptationMode,
    /// Min EV
    pub min_ev: f32,
    /// Max EV
    pub max_ev: f32,
    /// Adaptation speed (up)
    pub speed_up: f32,
    /// Adaptation speed (down)
    pub speed_down: f32,
    /// Target exposure
    pub target_exposure: f32,
}

impl EyeAdaptationCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            method: AdaptationMethod::Histogram,
            mode: AdaptationMode::Progressive,
            min_ev: -4.0,
            max_ev: 16.0,
            speed_up: 2.0,
            speed_down: 1.0,
            target_exposure: 0.0,
        }
    }

    /// Preset for indoor scenes
    pub fn indoor() -> Self {
        Self {
            min_ev: -2.0,
            max_ev: 8.0,
            speed_up: 2.0,
            speed_down: 1.5,
            ..Self::new()
        }
    }

    /// Preset for outdoor scenes
    pub fn outdoor() -> Self {
        Self {
            min_ev: 4.0,
            max_ev: 16.0,
            speed_up: 2.5,
            speed_down: 1.0,
            ..Self::new()
        }
    }

    /// Preset for high contrast (indoor/outdoor transitions)
    pub fn high_contrast() -> Self {
        Self {
            min_ev: -4.0,
            max_ev: 20.0,
            speed_up: 3.0,
            speed_down: 2.0,
            ..Self::new()
        }
    }

    /// Fast adaptation
    pub fn fast() -> Self {
        Self {
            speed_up: 5.0,
            speed_down: 3.0,
            ..Self::new()
        }
    }

    /// Slow adaptation
    pub fn slow() -> Self {
        Self {
            speed_up: 0.5,
            speed_down: 0.3,
            ..Self::new()
        }
    }

    /// With method
    pub fn with_method(mut self, method: AdaptationMethod) -> Self {
        self.method = method;
        self
    }

    /// With EV range
    pub fn with_ev_range(mut self, min: f32, max: f32) -> Self {
        self.min_ev = min;
        self.max_ev = max;
        self
    }

    /// With speed
    pub fn with_speed(mut self, up: f32, down: f32) -> Self {
        self.speed_up = up;
        self.speed_down = down;
        self
    }
}

impl Default for EyeAdaptationCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Adaptation method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AdaptationMethod {
    /// Average luminance
    Average = 0,
    /// Histogram-based
    #[default]
    Histogram = 1,
    /// Percentile-based
    Percentile = 2,
}

impl AdaptationMethod {
    /// Requires histogram
    pub const fn requires_histogram(&self) -> bool {
        matches!(self, Self::Histogram | Self::Percentile)
    }
}

/// Adaptation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AdaptationMode {
    /// Fixed exposure
    Fixed = 0,
    /// Automatic progressive
    #[default]
    Progressive = 1,
    /// Instant adaptation
    Instant = 2,
}

// ============================================================================
// Histogram Configuration
// ============================================================================

/// Histogram create info
#[derive(Clone, Debug)]
pub struct HistogramCreateInfo {
    /// Name
    pub name: String,
    /// Bin count
    pub bin_count: u32,
    /// Log luminance range
    pub log_min: f32,
    /// Log luminance max
    pub log_max: f32,
    /// Screen percentage to use
    pub screen_percentage: f32,
}

impl HistogramCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            bin_count: 64,
            log_min: -10.0,
            log_max: 10.0,
            screen_percentage: 0.9,
        }
    }

    /// High resolution histogram
    pub fn high_resolution() -> Self {
        Self {
            bin_count: 256,
            ..Self::new()
        }
    }

    /// Low resolution histogram
    pub fn low_resolution() -> Self {
        Self {
            bin_count: 32,
            ..Self::new()
        }
    }

    /// With bin count
    pub fn with_bins(mut self, count: u32) -> Self {
        self.bin_count = count;
        self
    }

    /// With range
    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.log_min = min;
        self.log_max = max;
        self
    }
}

impl Default for HistogramCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Eye Adaptation Settings
// ============================================================================

/// Eye adaptation settings
#[derive(Clone, Copy, Debug)]
pub struct EyeAdaptationSettings {
    /// Method
    pub method: AdaptationMethod,
    /// Mode
    pub mode: AdaptationMode,
    /// Min EV100
    pub min_ev: f32,
    /// Max EV100
    pub max_ev: f32,
    /// Exposure compensation
    pub exposure_compensation: f32,
    /// Speed up
    pub speed_up: f32,
    /// Speed down
    pub speed_down: f32,
    /// Low percentile
    pub low_percent: f32,
    /// High percentile
    pub high_percent: f32,
    /// Histogram usage percentage
    pub histogram_log_min: f32,
    /// Histogram log max
    pub histogram_log_max: f32,
}

impl EyeAdaptationSettings {
    /// Default settings
    pub const fn default_settings() -> Self {
        Self {
            method: AdaptationMethod::Histogram,
            mode: AdaptationMode::Progressive,
            min_ev: -4.0,
            max_ev: 16.0,
            exposure_compensation: 0.0,
            speed_up: 2.0,
            speed_down: 1.0,
            low_percent: 0.5,
            high_percent: 0.95,
            histogram_log_min: -10.0,
            histogram_log_max: 10.0,
        }
    }

    /// Fixed exposure
    pub const fn fixed(ev: f32) -> Self {
        Self {
            mode: AdaptationMode::Fixed,
            min_ev: ev,
            max_ev: ev,
            ..Self::default_settings()
        }
    }

    /// With compensation
    pub const fn with_compensation(mut self, ev: f32) -> Self {
        self.exposure_compensation = ev;
        self
    }

    /// With percentiles
    pub const fn with_percentiles(mut self, low: f32, high: f32) -> Self {
        self.low_percent = low;
        self.high_percent = high;
        self
    }
}

impl Default for EyeAdaptationSettings {
    fn default() -> Self {
        Self::default_settings()
    }
}

// ============================================================================
// Metering
// ============================================================================

/// Metering mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MeteringMode {
    /// Average (full frame)
    #[default]
    Average = 0,
    /// Center-weighted
    CenterWeighted = 1,
    /// Spot (center)
    Spot = 2,
    /// Evaluative/Matrix
    Evaluative = 3,
    /// Custom mask
    Custom = 4,
}

impl MeteringMode {
    /// Uses weighting
    pub const fn uses_weighting(&self) -> bool {
        !matches!(self, Self::Average | Self::Spot)
    }
}

/// Metering settings
#[derive(Clone, Copy, Debug)]
pub struct MeteringSettings {
    /// Mode
    pub mode: MeteringMode,
    /// Spot size (for spot metering)
    pub spot_size: f32,
    /// Center weight (for center-weighted)
    pub center_weight: f32,
}

impl MeteringSettings {
    /// Default metering
    pub const fn default_metering() -> Self {
        Self {
            mode: MeteringMode::CenterWeighted,
            spot_size: 0.1,
            center_weight: 0.7,
        }
    }

    /// Average metering
    pub const fn average() -> Self {
        Self {
            mode: MeteringMode::Average,
            ..Self::default_metering()
        }
    }

    /// Spot metering
    pub const fn spot() -> Self {
        Self {
            mode: MeteringMode::Spot,
            spot_size: 0.05,
            center_weight: 1.0,
        }
    }
}

impl Default for MeteringSettings {
    fn default() -> Self {
        Self::default_metering()
    }
}

// ============================================================================
// Physical Camera Settings
// ============================================================================

/// Physical camera settings for exposure
#[derive(Clone, Copy, Debug)]
pub struct PhysicalCameraSettings {
    /// ISO (sensitivity)
    pub iso: f32,
    /// Shutter speed (seconds)
    pub shutter_speed: f32,
    /// Aperture (f-number)
    pub aperture: f32,
}

impl PhysicalCameraSettings {
    /// Default camera
    pub const fn default_camera() -> Self {
        Self {
            iso: 100.0,
            shutter_speed: 1.0 / 125.0,
            aperture: 5.6,
        }
    }

    /// Daylight settings
    pub const fn daylight() -> Self {
        Self {
            iso: 100.0,
            shutter_speed: 1.0 / 250.0,
            aperture: 8.0,
        }
    }

    /// Indoor settings
    pub const fn indoor() -> Self {
        Self {
            iso: 800.0,
            shutter_speed: 1.0 / 60.0,
            aperture: 2.8,
        }
    }

    /// Night settings
    pub const fn night() -> Self {
        Self {
            iso: 3200.0,
            shutter_speed: 1.0 / 30.0,
            aperture: 1.4,
        }
    }

    /// Calculate EV100
    pub fn ev100(&self) -> f32 {
        let n2 = self.aperture * self.aperture;
        let t = self.shutter_speed;
        (100.0 * n2 / (t * self.iso)).log2()
    }

    /// Calculate exposure from EV100
    pub fn exposure_from_ev100(ev100: f32) -> f32 {
        1.0 / (1.2 * 2.0_f32.powf(ev100))
    }

    /// Calculate exposure
    pub fn exposure(&self) -> f32 {
        Self::exposure_from_ev100(self.ev100())
    }
}

impl Default for PhysicalCameraSettings {
    fn default() -> Self {
        Self::default_camera()
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// Eye adaptation GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct EyeAdaptationGpuParams {
    /// Screen dimensions
    pub screen_size: [u32; 2],
    /// Min EV
    pub min_ev: f32,
    /// Max EV
    pub max_ev: f32,
    /// Speed up
    pub speed_up: f32,
    /// Speed down
    pub speed_down: f32,
    /// Exposure compensation
    pub exposure_compensation: f32,
    /// Delta time
    pub delta_time: f32,
    /// Low percentile
    pub low_percent: f32,
    /// High percentile
    pub high_percent: f32,
    /// Histogram log min
    pub histogram_log_min: f32,
    /// Histogram log max
    pub histogram_log_max: f32,
    /// Bin count
    pub bin_count: u32,
    /// Method
    pub method: u32,
    /// Mode
    pub mode: u32,
    /// Flags
    pub flags: u32,
}

impl EyeAdaptationGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Reset flag
    pub const FLAG_RESET: u32 = 1 << 0;
    /// First frame flag
    pub const FLAG_FIRST_FRAME: u32 = 1 << 1;
}

/// Histogram GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct HistogramGpuParams {
    /// Screen dimensions
    pub screen_size: [u32; 2],
    /// Bin count
    pub bin_count: u32,
    /// Log min
    pub log_min: f32,
    /// Log max
    pub log_max: f32,
    /// Screen percentage
    pub screen_percentage: f32,
    /// Padding
    pub _padding: [f32; 2],
}

impl HistogramGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// Current exposure result
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExposureResult {
    /// Current exposure
    pub exposure: f32,
    /// Current EV100
    pub ev100: f32,
    /// Average luminance
    pub avg_luminance: f32,
    /// Target exposure
    pub target_exposure: f32,
}

impl ExposureResult {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// ============================================================================
// Pass Configuration
// ============================================================================

/// Eye adaptation pass config
#[derive(Clone, Debug)]
pub struct EyeAdaptationPassConfig {
    /// Input color
    pub color: u64,
    /// Histogram buffer
    pub histogram: HistogramHandle,
    /// Current exposure buffer
    pub current_exposure: u64,
    /// Previous exposure buffer
    pub previous_exposure: u64,
    /// Settings
    pub settings: EyeAdaptationSettings,
    /// Metering
    pub metering: MeteringSettings,
    /// Delta time
    pub delta_time: f32,
    /// Is first frame
    pub first_frame: bool,
}

impl EyeAdaptationPassConfig {
    /// Creates config
    pub fn new(color: u64) -> Self {
        Self {
            color,
            histogram: HistogramHandle::NULL,
            current_exposure: 0,
            previous_exposure: 0,
            settings: EyeAdaptationSettings::default(),
            metering: MeteringSettings::default(),
            delta_time: 0.016,
            first_frame: false,
        }
    }

    /// With settings
    pub fn with_settings(mut self, settings: EyeAdaptationSettings) -> Self {
        self.settings = settings;
        self
    }

    /// With metering
    pub fn with_metering(mut self, metering: MeteringSettings) -> Self {
        self.metering = metering;
        self
    }
}

impl Default for EyeAdaptationPassConfig {
    fn default() -> Self {
        Self::new(0)
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Eye adaptation statistics
#[derive(Clone, Debug, Default)]
pub struct EyeAdaptationStats {
    /// Current exposure
    pub current_exposure: f32,
    /// Current EV100
    pub current_ev100: f32,
    /// Average luminance
    pub avg_luminance: f32,
    /// Histogram peak bin
    pub peak_bin: u32,
    /// Pass time (microseconds)
    pub pass_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
