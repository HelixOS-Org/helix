//! Screen Space Effects Types for Lumina
//!
//! This module provides screen-space effect infrastructure including
//! ambient occlusion, screen-space reflections, and contact shadows.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Effect Handles
// ============================================================================

/// SSAO handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SsaoHandle(pub u64);

impl SsaoHandle {
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

impl Default for SsaoHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// SSR handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SsrHandle(pub u64);

impl SsrHandle {
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

impl Default for SsrHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Screen-Space Ambient Occlusion (SSAO)
// ============================================================================

/// SSAO settings
#[derive(Clone, Debug)]
pub struct SsaoSettings {
    /// Enable SSAO
    pub enabled: bool,
    /// SSAO method
    pub method: SsaoMethod,
    /// Quality
    pub quality: SsaoQuality,
    /// Radius (world units)
    pub radius: f32,
    /// Intensity
    pub intensity: f32,
    /// Bias
    pub bias: f32,
    /// Power (contrast)
    pub power: f32,
    /// Blur settings
    pub blur: SsaoBlurSettings,
}

impl SsaoSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            method: SsaoMethod::Hbao,
            quality: SsaoQuality::Medium,
            radius: 0.5,
            intensity: 1.0,
            bias: 0.025,
            power: 2.0,
            blur: SsaoBlurSettings::default(),
        }
    }

    /// Low quality
    pub fn low() -> Self {
        Self {
            quality: SsaoQuality::Low,
            ..Self::new()
        }
    }

    /// High quality
    pub fn high() -> Self {
        Self {
            quality: SsaoQuality::High,
            method: SsaoMethod::Gtao,
            ..Self::new()
        }
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    /// With method
    pub fn with_method(mut self, method: SsaoMethod) -> Self {
        self.method = method;
        self
    }

    /// With radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for SsaoSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// SSAO method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SsaoMethod {
    /// Basic SSAO
    Basic = 0,
    /// HBAO (Horizon-Based AO)
    #[default]
    Hbao = 1,
    /// HBAO+
    HbaoPlus = 2,
    /// GTAO (Ground Truth AO)
    Gtao = 3,
    /// XeGTAO
    XeGtao = 4,
    /// RTAO (Ray-traced)
    Rtao = 5,
}

impl SsaoMethod {
    /// Is ray traced
    pub const fn is_ray_traced(&self) -> bool {
        matches!(self, Self::Rtao)
    }
}

/// SSAO quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SsaoQuality {
    /// Low (quarter res, few samples)
    Low = 0,
    /// Medium (half res)
    #[default]
    Medium = 1,
    /// High (full res)
    High = 2,
    /// Ultra (high sample count)
    Ultra = 3,
}

impl SsaoQuality {
    /// Resolution scale
    pub const fn resolution_scale(&self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 0.5,
            Self::High => 1.0,
            Self::Ultra => 1.0,
        }
    }

    /// Sample count
    pub const fn sample_count(&self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 8,
            Self::High => 16,
            Self::Ultra => 32,
        }
    }
}

/// SSAO blur settings
#[derive(Clone, Copy, Debug)]
pub struct SsaoBlurSettings {
    /// Enable blur
    pub enabled: bool,
    /// Blur radius
    pub radius: u32,
    /// Edge-aware sharpness
    pub sharpness: f32,
}

impl SsaoBlurSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            radius: 3,
            sharpness: 10.0,
        }
    }

    /// No blur
    pub fn none() -> Self {
        Self {
            enabled: false,
            radius: 0,
            sharpness: 0.0,
        }
    }
}

impl Default for SsaoBlurSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// SSAO GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SsaoGpuParams {
    /// Projection info (for reconstructing position)
    pub projection_info: [f32; 4],
    /// Radius and bias
    pub radius_bias: [f32; 4],
    /// Intensity and power
    pub intensity_power: [f32; 4],
    /// Screen size
    pub screen_size: [f32; 4],
}

// ============================================================================
// Screen-Space Reflections (SSR)
// ============================================================================

/// SSR settings
#[derive(Clone, Debug)]
pub struct SsrSettings {
    /// Enable SSR
    pub enabled: bool,
    /// SSR method
    pub method: SsrMethod,
    /// Quality
    pub quality: SsrQuality,
    /// Max ray distance
    pub max_distance: f32,
    /// Thickness (for depth comparison)
    pub thickness: f32,
    /// Step stride
    pub stride: f32,
    /// Fade settings
    pub fade: SsrFadeSettings,
}

impl SsrSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            method: SsrMethod::HiZ,
            quality: SsrQuality::Medium,
            max_distance: 100.0,
            thickness: 0.1,
            stride: 1.0,
            fade: SsrFadeSettings::default(),
        }
    }

    /// Low quality
    pub fn low() -> Self {
        Self {
            quality: SsrQuality::Low,
            max_distance: 50.0,
            ..Self::new()
        }
    }

    /// High quality
    pub fn high() -> Self {
        Self {
            quality: SsrQuality::High,
            max_distance: 200.0,
            ..Self::new()
        }
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    /// With method
    pub fn with_method(mut self, method: SsrMethod) -> Self {
        self.method = method;
        self
    }

    /// With max distance
    pub fn with_max_distance(mut self, distance: f32) -> Self {
        self.max_distance = distance;
        self
    }
}

impl Default for SsrSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// SSR method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SsrMethod {
    /// Linear ray march
    Linear = 0,
    /// Hierarchical Z-buffer
    #[default]
    HiZ = 1,
    /// Stochastic SSR
    Stochastic = 2,
}

/// SSR quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SsrQuality {
    /// Low (fewer steps)
    Low = 0,
    /// Medium
    #[default]
    Medium = 1,
    /// High (more steps, refinement)
    High = 2,
    /// Ultra
    Ultra = 3,
}

impl SsrQuality {
    /// Max steps
    pub const fn max_steps(&self) -> u32 {
        match self {
            Self::Low => 16,
            Self::Medium => 32,
            Self::High => 64,
            Self::Ultra => 128,
        }
    }

    /// Binary search steps
    pub const fn refinement_steps(&self) -> u32 {
        match self {
            Self::Low => 0,
            Self::Medium => 4,
            Self::High => 8,
            Self::Ultra => 16,
        }
    }
}

/// SSR fade settings
#[derive(Clone, Copy, Debug)]
pub struct SsrFadeSettings {
    /// Screen edge fade
    pub edge_fade: f32,
    /// Distance fade start
    pub distance_fade_start: f32,
    /// Distance fade end
    pub distance_fade_end: f32,
    /// Roughness fade
    pub roughness_fade: f32,
}

impl SsrFadeSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            edge_fade: 0.1,
            distance_fade_start: 50.0,
            distance_fade_end: 100.0,
            roughness_fade: 0.5,
        }
    }
}

impl Default for SsrFadeSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// SSR GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SsrGpuParams {
    /// Projection matrix
    pub projection: [[f32; 4]; 4],
    /// Inverse projection
    pub inv_projection: [[f32; 4]; 4],
    /// View matrix
    pub view: [[f32; 4]; 4],
    /// Ray params (max_dist, thickness, stride, fade)
    pub ray_params: [f32; 4],
    /// Screen size
    pub screen_size: [f32; 4],
}

// ============================================================================
// Contact Shadows
// ============================================================================

/// Contact shadow settings
#[derive(Clone, Debug)]
pub struct ContactShadowSettings {
    /// Enable contact shadows
    pub enabled: bool,
    /// Ray length
    pub ray_length: f32,
    /// Sample count
    pub samples: u32,
    /// Thickness
    pub thickness: f32,
    /// Fade distance
    pub fade_distance: f32,
}

impl ContactShadowSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            ray_length: 0.5,
            samples: 8,
            thickness: 0.1,
            fade_distance: 50.0,
        }
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    /// With ray length
    pub fn with_ray_length(mut self, length: f32) -> Self {
        self.ray_length = length;
        self
    }

    /// With samples
    pub fn with_samples(mut self, samples: u32) -> Self {
        self.samples = samples;
        self
    }
}

impl Default for ContactShadowSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Screen-Space Global Illumination (SSGI)
// ============================================================================

/// SSGI settings
#[derive(Clone, Debug)]
pub struct SsgiSettings {
    /// Enable SSGI
    pub enabled: bool,
    /// Method
    pub method: SsgiMethod,
    /// Quality
    pub quality: SsgiQuality,
    /// Intensity
    pub intensity: f32,
    /// Max distance
    pub max_distance: f32,
    /// Thickness
    pub thickness: f32,
}

impl SsgiSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: false,
            method: SsgiMethod::Ssgi,
            quality: SsgiQuality::Medium,
            intensity: 1.0,
            max_distance: 10.0,
            thickness: 0.5,
        }
    }

    /// Enabled
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::new()
        }
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for SsgiSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// SSGI method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SsgiMethod {
    /// Screen-space GI
    #[default]
    Ssgi = 0,
    /// SSDO (Screen-Space Directional Occlusion)
    Ssdo = 1,
    /// RTGI (Ray-traced)
    Rtgi = 2,
}

/// SSGI quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SsgiQuality {
    /// Low
    Low = 0,
    /// Medium
    #[default]
    Medium = 1,
    /// High
    High = 2,
}

// ============================================================================
// Screen-Space Subsurface Scattering
// ============================================================================

/// Screen-space SSS settings
#[derive(Clone, Debug)]
pub struct ScreenSpaceSssSettings {
    /// Enable
    pub enabled: bool,
    /// Quality
    pub quality: SssQuality,
    /// Samples
    pub samples: u32,
    /// Follow surface
    pub follow_surface: bool,
}

impl ScreenSpaceSssSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: false,
            quality: SssQuality::Medium,
            samples: 11,
            follow_surface: true,
        }
    }

    /// Enabled
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::new()
        }
    }
}

impl Default for ScreenSpaceSssSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// SSS quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SssQuality {
    /// Low
    Low = 0,
    /// Medium
    #[default]
    Medium = 1,
    /// High
    High = 2,
}

impl SssQuality {
    /// Sample count
    pub const fn sample_count(&self) -> u32 {
        match self {
            Self::Low => 7,
            Self::Medium => 11,
            Self::High => 21,
        }
    }
}

/// SSS profile
#[derive(Clone, Copy, Debug)]
pub struct SssProfile {
    /// Scatter color
    pub scatter_color: [f32; 3],
    /// Scatter distance
    pub scatter_distance: f32,
    /// Falloff
    pub falloff: [f32; 3],
    /// Strength
    pub strength: f32,
}

impl SssProfile {
    /// Creates profile
    pub fn new(scatter_color: [f32; 3], scatter_distance: f32) -> Self {
        Self {
            scatter_color,
            scatter_distance,
            falloff: [1.0, 0.37, 0.12],
            strength: 1.0,
        }
    }

    /// Skin preset
    pub fn skin() -> Self {
        Self::new([0.8, 0.3, 0.2], 2.0)
    }

    /// Marble preset
    pub fn marble() -> Self {
        Self::new([0.9, 0.85, 0.8], 5.0)
    }

    /// Jade preset
    pub fn jade() -> Self {
        Self::new([0.4, 0.8, 0.4], 3.0)
    }

    /// With strength
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }
}

impl Default for SssProfile {
    fn default() -> Self {
        Self::skin()
    }
}

// ============================================================================
// Depth of Field
// ============================================================================

/// Depth of field settings
#[derive(Clone, Debug)]
pub struct DepthOfFieldSettings {
    /// Enable DoF
    pub enabled: bool,
    /// Method
    pub method: DofMethod,
    /// Focus distance
    pub focus_distance: f32,
    /// Aperture (f-stop)
    pub aperture: f32,
    /// Focal length (mm)
    pub focal_length: f32,
    /// Max blur radius
    pub max_blur_radius: f32,
    /// Near blur
    pub near_blur: bool,
}

impl DepthOfFieldSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: false,
            method: DofMethod::Bokeh,
            focus_distance: 10.0,
            aperture: 2.8,
            focal_length: 50.0,
            max_blur_radius: 10.0,
            near_blur: true,
        }
    }

    /// Enabled
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::new()
        }
    }

    /// With focus distance
    pub fn with_focus_distance(mut self, distance: f32) -> Self {
        self.focus_distance = distance;
        self
    }

    /// With aperture
    pub fn with_aperture(mut self, aperture: f32) -> Self {
        self.aperture = aperture;
        self
    }

    /// Circle of confusion size at distance
    pub fn coc_at_distance(&self, distance: f32) -> f32 {
        let focal = self.focal_length * 0.001; // mm to m
        let aperture_size = focal / self.aperture;
        let focus = self.focus_distance;

        if distance <= 0.0 || focus <= 0.0 {
            return 0.0;
        }

        let coc = aperture_size * focal * (distance - focus).abs() / (distance * (focus - focal));
        coc.abs().min(self.max_blur_radius)
    }
}

impl Default for DepthOfFieldSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// DoF method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DofMethod {
    /// Gaussian blur
    Gaussian = 0,
    /// Bokeh (hex/circular)
    #[default]
    Bokeh = 1,
    /// Scatter bokeh
    ScatterBokeh = 2,
}

/// DoF GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DofGpuParams {
    /// Focus distance and aperture
    pub focus_params: [f32; 4],
    /// Focal length, sensor size
    pub lens_params: [f32; 4],
    /// Screen size
    pub screen_size: [f32; 4],
}

// ============================================================================
// Motion Blur
// ============================================================================

/// Motion blur settings
#[derive(Clone, Debug)]
pub struct MotionBlurSettings {
    /// Enable motion blur
    pub enabled: bool,
    /// Intensity
    pub intensity: f32,
    /// Sample count
    pub samples: u32,
    /// Max blur velocity (pixels)
    pub max_velocity: f32,
    /// Camera motion blur
    pub camera_motion: bool,
    /// Object motion blur
    pub object_motion: bool,
}

impl MotionBlurSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: false,
            intensity: 1.0,
            samples: 8,
            max_velocity: 32.0,
            camera_motion: true,
            object_motion: true,
        }
    }

    /// Enabled
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::new()
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With samples
    pub fn with_samples(mut self, samples: u32) -> Self {
        self.samples = samples;
        self
    }
}

impl Default for MotionBlurSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Screen space effects statistics
#[derive(Clone, Debug, Default)]
pub struct ScreenSpaceStats {
    /// SSAO enabled
    pub ssao_enabled: bool,
    /// SSR enabled
    pub ssr_enabled: bool,
    /// SSGI enabled
    pub ssgi_enabled: bool,
    /// Contact shadows enabled
    pub contact_shadows_enabled: bool,
    /// Total GPU time (microseconds)
    pub gpu_time_us: u64,
    /// SSAO time
    pub ssao_time_us: u64,
    /// SSR time
    pub ssr_time_us: u64,
}
