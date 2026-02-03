//! Post-Processing Effects for Lumina
//!
//! This module provides post-processing effect types including
//! bloom, tone mapping, color grading, anti-aliasing, and more.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Post-Process Effect Handle
// ============================================================================

/// Post-process effect handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PostProcessHandle(pub u64);

impl PostProcessHandle {
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

impl Default for PostProcessHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Post-Process Volume
// ============================================================================

/// Post-process volume containing effect settings
#[derive(Clone, Debug)]
pub struct PostProcessVolume {
    /// Priority (higher = more important)
    pub priority: i32,
    /// Blend radius (0 = infinite)
    pub blend_radius: f32,
    /// Weight
    pub weight: f32,
    /// Is enabled
    pub enabled: bool,
    /// Is global
    pub is_global: bool,
    /// Effects
    pub effects: PostProcessSettings,
}

impl PostProcessVolume {
    /// Creates new global volume
    pub fn global() -> Self {
        Self {
            priority: 0,
            blend_radius: 0.0,
            weight: 1.0,
            enabled: true,
            is_global: true,
            effects: PostProcessSettings::default(),
        }
    }

    /// Creates new local volume
    pub fn local(blend_radius: f32) -> Self {
        Self {
            priority: 0,
            blend_radius,
            weight: 1.0,
            enabled: true,
            is_global: false,
            effects: PostProcessSettings::default(),
        }
    }

    /// With priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// With weight
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// With effects
    pub fn with_effects(mut self, effects: PostProcessSettings) -> Self {
        self.effects = effects;
        self
    }
}

// ============================================================================
// Post-Process Settings
// ============================================================================

/// Post-process settings container
#[derive(Clone, Debug, Default)]
pub struct PostProcessSettings {
    /// Bloom settings
    pub bloom: Option<BloomSettings>,
    /// Tone mapping settings
    pub tone_mapping: Option<ToneMappingSettings>,
    /// Color grading settings
    pub color_grading: Option<ColorGradingSettings>,
    /// Vignette settings
    pub vignette: Option<VignetteSettings>,
    /// Chromatic aberration settings
    pub chromatic_aberration: Option<ChromaticAberrationSettings>,
    /// Depth of field settings
    pub depth_of_field: Option<DepthOfFieldSettings>,
    /// Motion blur settings
    pub motion_blur: Option<MotionBlurSettings>,
    /// Ambient occlusion settings
    pub ambient_occlusion: Option<AmbientOcclusionSettings>,
    /// Film grain settings
    pub film_grain: Option<FilmGrainSettings>,
    /// Lens flare settings
    pub lens_flare: Option<LensFlareSettings>,
    /// Auto exposure settings
    pub auto_exposure: Option<AutoExposureSettings>,
    /// Fog settings
    pub fog: Option<FogSettings>,
    /// Anti-aliasing settings
    pub anti_aliasing: Option<AntiAliasingSettings>,
    /// Sharpening settings
    pub sharpening: Option<SharpeningSettings>,
}

impl PostProcessSettings {
    /// Creates new settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Default cinematic settings
    pub fn cinematic() -> Self {
        Self {
            bloom: Some(BloomSettings::cinematic()),
            tone_mapping: Some(ToneMappingSettings::filmic()),
            color_grading: Some(ColorGradingSettings::default()),
            vignette: Some(VignetteSettings::subtle()),
            chromatic_aberration: Some(ChromaticAberrationSettings::subtle()),
            depth_of_field: None,
            motion_blur: Some(MotionBlurSettings::default()),
            ambient_occlusion: Some(AmbientOcclusionSettings::high()),
            film_grain: Some(FilmGrainSettings::subtle()),
            lens_flare: None,
            auto_exposure: Some(AutoExposureSettings::default()),
            fog: None,
            anti_aliasing: Some(AntiAliasingSettings::taa()),
            sharpening: Some(SharpeningSettings::default()),
        }
    }

    /// With bloom
    pub fn with_bloom(mut self, settings: BloomSettings) -> Self {
        self.bloom = Some(settings);
        self
    }

    /// With tone mapping
    pub fn with_tone_mapping(mut self, settings: ToneMappingSettings) -> Self {
        self.tone_mapping = Some(settings);
        self
    }

    /// With color grading
    pub fn with_color_grading(mut self, settings: ColorGradingSettings) -> Self {
        self.color_grading = Some(settings);
        self
    }

    /// With vignette
    pub fn with_vignette(mut self, settings: VignetteSettings) -> Self {
        self.vignette = Some(settings);
        self
    }

    /// With ambient occlusion
    pub fn with_ao(mut self, settings: AmbientOcclusionSettings) -> Self {
        self.ambient_occlusion = Some(settings);
        self
    }

    /// Blend with another settings
    pub fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            bloom: blend_option(&self.bloom, &other.bloom, weight),
            tone_mapping: blend_option(&self.tone_mapping, &other.tone_mapping, weight),
            color_grading: blend_option(&self.color_grading, &other.color_grading, weight),
            vignette: blend_option(&self.vignette, &other.vignette, weight),
            chromatic_aberration: blend_option(&self.chromatic_aberration, &other.chromatic_aberration, weight),
            depth_of_field: blend_option(&self.depth_of_field, &other.depth_of_field, weight),
            motion_blur: blend_option(&self.motion_blur, &other.motion_blur, weight),
            ambient_occlusion: blend_option(&self.ambient_occlusion, &other.ambient_occlusion, weight),
            film_grain: blend_option(&self.film_grain, &other.film_grain, weight),
            lens_flare: blend_option(&self.lens_flare, &other.lens_flare, weight),
            auto_exposure: blend_option(&self.auto_exposure, &other.auto_exposure, weight),
            fog: blend_option(&self.fog, &other.fog, weight),
            anti_aliasing: if weight > 0.5 { other.anti_aliasing.clone() } else { self.anti_aliasing.clone() },
            sharpening: blend_option(&self.sharpening, &other.sharpening, weight),
        }
    }
}

fn blend_option<T: Blendable + Clone>(a: &Option<T>, b: &Option<T>, weight: f32) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.blend(b, weight)),
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (None, None) => None,
    }
}

/// Trait for blendable settings
pub trait Blendable {
    /// Blend with another instance
    fn blend(&self, other: &Self, weight: f32) -> Self;
}

// ============================================================================
// Bloom Settings
// ============================================================================

/// Bloom effect settings
#[derive(Clone, Debug)]
pub struct BloomSettings {
    /// Intensity
    pub intensity: f32,
    /// Threshold
    pub threshold: f32,
    /// Soft threshold
    pub soft_threshold: f32,
    /// Scatter
    pub scatter: f32,
    /// Tint color
    pub tint: [f32; 3],
    /// High quality
    pub high_quality: bool,
    /// Bloom mip count
    pub mip_count: u32,
}

impl BloomSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            intensity: 1.0,
            threshold: 1.0,
            soft_threshold: 0.5,
            scatter: 0.7,
            tint: [1.0, 1.0, 1.0],
            high_quality: true,
            mip_count: 5,
        }
    }

    /// Cinematic bloom
    pub fn cinematic() -> Self {
        Self {
            intensity: 0.8,
            threshold: 0.9,
            soft_threshold: 0.3,
            scatter: 0.65,
            tint: [1.0, 0.98, 0.95],
            high_quality: true,
            mip_count: 6,
        }
    }

    /// Subtle bloom
    pub fn subtle() -> Self {
        Self {
            intensity: 0.3,
            threshold: 1.2,
            soft_threshold: 0.2,
            scatter: 0.5,
            tint: [1.0, 1.0, 1.0],
            high_quality: false,
            mip_count: 4,
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for BloomSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            intensity: lerp(self.intensity, other.intensity, weight),
            threshold: lerp(self.threshold, other.threshold, weight),
            soft_threshold: lerp(self.soft_threshold, other.soft_threshold, weight),
            scatter: lerp(self.scatter, other.scatter, weight),
            tint: lerp_color(self.tint, other.tint, weight),
            high_quality: if weight > 0.5 { other.high_quality } else { self.high_quality },
            mip_count: if weight > 0.5 { other.mip_count } else { self.mip_count },
        }
    }
}

// ============================================================================
// Tone Mapping Settings
// ============================================================================

/// Tone mapping settings
#[derive(Clone, Debug)]
pub struct ToneMappingSettings {
    /// Tone mapping operator
    pub operator: ToneMapOperator,
    /// Exposure
    pub exposure: f32,
    /// Contrast
    pub contrast: f32,
    /// Shoulder strength (for ACES)
    pub shoulder_strength: f32,
    /// Linear strength (for ACES)
    pub linear_strength: f32,
    /// White point
    pub white_point: f32,
}

impl ToneMappingSettings {
    /// Creates with operator
    pub fn new(operator: ToneMapOperator) -> Self {
        Self {
            operator,
            exposure: 1.0,
            contrast: 1.0,
            shoulder_strength: 0.22,
            linear_strength: 0.30,
            white_point: 11.2,
        }
    }

    /// Filmic (ACES approximation)
    pub fn filmic() -> Self {
        Self::new(ToneMapOperator::Aces)
    }

    /// Reinhard
    pub fn reinhard() -> Self {
        Self::new(ToneMapOperator::Reinhard)
    }

    /// Neutral (no artistic curve)
    pub fn neutral() -> Self {
        Self::new(ToneMapOperator::Neutral)
    }

    /// With exposure
    pub fn with_exposure(mut self, exposure: f32) -> Self {
        self.exposure = exposure;
        self
    }
}

impl Default for ToneMappingSettings {
    fn default() -> Self {
        Self::filmic()
    }
}

impl Blendable for ToneMappingSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            operator: if weight > 0.5 { other.operator } else { self.operator },
            exposure: lerp(self.exposure, other.exposure, weight),
            contrast: lerp(self.contrast, other.contrast, weight),
            shoulder_strength: lerp(self.shoulder_strength, other.shoulder_strength, weight),
            linear_strength: lerp(self.linear_strength, other.linear_strength, weight),
            white_point: lerp(self.white_point, other.white_point, weight),
        }
    }
}

/// Tone mapping operator
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ToneMapOperator {
    /// Linear (no tonemapping)
    Linear = 0,
    /// Reinhard
    Reinhard = 1,
    /// Reinhard extended
    ReinhardExtended = 2,
    /// ACES (Academy Color Encoding System)
    #[default]
    Aces = 3,
    /// Uncharted 2 filmic
    Uncharted2 = 4,
    /// Neutral
    Neutral = 5,
    /// AgX
    AgX = 6,
}

// ============================================================================
// Color Grading Settings
// ============================================================================

/// Color grading settings
#[derive(Clone, Debug)]
pub struct ColorGradingSettings {
    /// Temperature offset (Kelvin, -100 to 100)
    pub temperature: f32,
    /// Tint (green-magenta, -100 to 100)
    pub tint: f32,
    /// Saturation (0 = grayscale, 1 = normal, >1 = oversaturated)
    pub saturation: f32,
    /// Vibrance (0 = no effect, 1 = normal vibrance)
    pub vibrance: f32,
    /// Brightness offset
    pub brightness: f32,
    /// Contrast
    pub contrast: f32,
    /// Gamma
    pub gamma: f32,
    /// Shadows color (RGB)
    pub shadows: [f32; 3],
    /// Midtones color (RGB)
    pub midtones: [f32; 3],
    /// Highlights color (RGB)
    pub highlights: [f32; 3],
    /// Shadow-midtone-highlight balance
    pub shadows_midtones_highlights: [f32; 3],
    /// LUT texture handle
    pub lut_texture: u64,
    /// LUT contribution
    pub lut_contribution: f32,
}

impl ColorGradingSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            temperature: 0.0,
            tint: 0.0,
            saturation: 1.0,
            vibrance: 0.0,
            brightness: 0.0,
            contrast: 1.0,
            gamma: 1.0,
            shadows: [0.0, 0.0, 0.0],
            midtones: [0.0, 0.0, 0.0],
            highlights: [0.0, 0.0, 0.0],
            shadows_midtones_highlights: [0.0, 0.5, 1.0],
            lut_texture: 0,
            lut_contribution: 1.0,
        }
    }

    /// Warm preset
    pub fn warm() -> Self {
        Self {
            temperature: 20.0,
            ..Self::new()
        }
    }

    /// Cool preset
    pub fn cool() -> Self {
        Self {
            temperature: -20.0,
            ..Self::new()
        }
    }

    /// High contrast
    pub fn high_contrast() -> Self {
        Self {
            contrast: 1.2,
            saturation: 1.1,
            ..Self::new()
        }
    }

    /// With saturation
    pub fn with_saturation(mut self, saturation: f32) -> Self {
        self.saturation = saturation;
        self
    }

    /// With contrast
    pub fn with_contrast(mut self, contrast: f32) -> Self {
        self.contrast = contrast;
        self
    }

    /// With LUT
    pub fn with_lut(mut self, texture: u64, contribution: f32) -> Self {
        self.lut_texture = texture;
        self.lut_contribution = contribution;
        self
    }
}

impl Default for ColorGradingSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for ColorGradingSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            temperature: lerp(self.temperature, other.temperature, weight),
            tint: lerp(self.tint, other.tint, weight),
            saturation: lerp(self.saturation, other.saturation, weight),
            vibrance: lerp(self.vibrance, other.vibrance, weight),
            brightness: lerp(self.brightness, other.brightness, weight),
            contrast: lerp(self.contrast, other.contrast, weight),
            gamma: lerp(self.gamma, other.gamma, weight),
            shadows: lerp_color(self.shadows, other.shadows, weight),
            midtones: lerp_color(self.midtones, other.midtones, weight),
            highlights: lerp_color(self.highlights, other.highlights, weight),
            shadows_midtones_highlights: lerp_color(
                self.shadows_midtones_highlights,
                other.shadows_midtones_highlights,
                weight,
            ),
            lut_texture: if weight > 0.5 { other.lut_texture } else { self.lut_texture },
            lut_contribution: lerp(self.lut_contribution, other.lut_contribution, weight),
        }
    }
}

// ============================================================================
// Vignette Settings
// ============================================================================

/// Vignette effect settings
#[derive(Clone, Debug)]
pub struct VignetteSettings {
    /// Intensity (0-1)
    pub intensity: f32,
    /// Smoothness (0-1)
    pub smoothness: f32,
    /// Roundness (0 = square, 1 = round)
    pub roundness: f32,
    /// Center offset
    pub center: [f32; 2],
    /// Color
    pub color: [f32; 3],
}

impl VignetteSettings {
    /// Creates default vignette
    pub fn new() -> Self {
        Self {
            intensity: 0.3,
            smoothness: 0.5,
            roundness: 1.0,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// Subtle vignette
    pub fn subtle() -> Self {
        Self {
            intensity: 0.15,
            smoothness: 0.4,
            roundness: 1.0,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// Strong vignette
    pub fn strong() -> Self {
        Self {
            intensity: 0.5,
            smoothness: 0.3,
            roundness: 1.0,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for VignetteSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for VignetteSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            intensity: lerp(self.intensity, other.intensity, weight),
            smoothness: lerp(self.smoothness, other.smoothness, weight),
            roundness: lerp(self.roundness, other.roundness, weight),
            center: [
                lerp(self.center[0], other.center[0], weight),
                lerp(self.center[1], other.center[1], weight),
            ],
            color: lerp_color(self.color, other.color, weight),
        }
    }
}

// ============================================================================
// Chromatic Aberration Settings
// ============================================================================

/// Chromatic aberration settings
#[derive(Clone, Debug)]
pub struct ChromaticAberrationSettings {
    /// Intensity
    pub intensity: f32,
    /// Max samples
    pub max_samples: u32,
}

impl ChromaticAberrationSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            intensity: 0.1,
            max_samples: 8,
        }
    }

    /// Subtle effect
    pub fn subtle() -> Self {
        Self {
            intensity: 0.05,
            max_samples: 4,
        }
    }

    /// Strong effect
    pub fn strong() -> Self {
        Self {
            intensity: 0.3,
            max_samples: 16,
        }
    }
}

impl Default for ChromaticAberrationSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for ChromaticAberrationSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            intensity: lerp(self.intensity, other.intensity, weight),
            max_samples: if weight > 0.5 { other.max_samples } else { self.max_samples },
        }
    }
}

// ============================================================================
// Depth of Field Settings
// ============================================================================

/// Depth of field settings
#[derive(Clone, Debug)]
pub struct DepthOfFieldSettings {
    /// Focus mode
    pub mode: FocusMode,
    /// Focus distance (meters)
    pub focus_distance: f32,
    /// Focal length (mm)
    pub focal_length: f32,
    /// Aperture (f-stop)
    pub aperture: f32,
    /// Blade count (for bokeh shape)
    pub blade_count: u32,
    /// Blade curvature
    pub blade_curvature: f32,
    /// Near blur start
    pub near_blur_start: f32,
    /// Near blur end
    pub near_blur_end: f32,
    /// Far blur start
    pub far_blur_start: f32,
    /// Far blur end
    pub far_blur_end: f32,
    /// High quality bokeh
    pub high_quality: bool,
}

impl DepthOfFieldSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            mode: FocusMode::Manual,
            focus_distance: 10.0,
            focal_length: 50.0,
            aperture: 2.8,
            blade_count: 5,
            blade_curvature: 1.0,
            near_blur_start: 0.0,
            near_blur_end: 4.0,
            far_blur_start: 10.0,
            far_blur_end: 30.0,
            high_quality: true,
        }
    }

    /// Physical camera settings
    pub fn physical(focal_length: f32, aperture: f32, focus_distance: f32) -> Self {
        Self {
            mode: FocusMode::Physical,
            focus_distance,
            focal_length,
            aperture,
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
}

impl Default for DepthOfFieldSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for DepthOfFieldSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            mode: if weight > 0.5 { other.mode } else { self.mode },
            focus_distance: lerp(self.focus_distance, other.focus_distance, weight),
            focal_length: lerp(self.focal_length, other.focal_length, weight),
            aperture: lerp(self.aperture, other.aperture, weight),
            blade_count: if weight > 0.5 { other.blade_count } else { self.blade_count },
            blade_curvature: lerp(self.blade_curvature, other.blade_curvature, weight),
            near_blur_start: lerp(self.near_blur_start, other.near_blur_start, weight),
            near_blur_end: lerp(self.near_blur_end, other.near_blur_end, weight),
            far_blur_start: lerp(self.far_blur_start, other.far_blur_start, weight),
            far_blur_end: lerp(self.far_blur_end, other.far_blur_end, weight),
            high_quality: if weight > 0.5 { other.high_quality } else { self.high_quality },
        }
    }
}

/// Focus mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FocusMode {
    /// Manual focus distance
    #[default]
    Manual = 0,
    /// Physical camera model
    Physical = 1,
    /// Auto focus on depth
    Auto = 2,
}

// ============================================================================
// Motion Blur Settings
// ============================================================================

/// Motion blur settings
#[derive(Clone, Debug)]
pub struct MotionBlurSettings {
    /// Intensity (shutter angle / 360)
    pub intensity: f32,
    /// Max samples
    pub max_samples: u32,
    /// Quality
    pub quality: MotionBlurQuality,
}

impl MotionBlurSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            intensity: 0.5,
            max_samples: 16,
            quality: MotionBlurQuality::Medium,
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for MotionBlurSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for MotionBlurSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            intensity: lerp(self.intensity, other.intensity, weight),
            max_samples: if weight > 0.5 { other.max_samples } else { self.max_samples },
            quality: if weight > 0.5 { other.quality } else { self.quality },
        }
    }
}

/// Motion blur quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MotionBlurQuality {
    /// Low quality
    Low = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High = 2,
}

// ============================================================================
// Ambient Occlusion Settings
// ============================================================================

/// Ambient occlusion settings
#[derive(Clone, Debug)]
pub struct AmbientOcclusionSettings {
    /// AO technique
    pub technique: AoTechnique,
    /// Intensity
    pub intensity: f32,
    /// Radius (world units)
    pub radius: f32,
    /// Bias
    pub bias: f32,
    /// Direct lighting influence
    pub direct_lighting_strength: f32,
    /// Sample count
    pub sample_count: u32,
    /// Temporal accumulation
    pub temporal: bool,
}

impl AmbientOcclusionSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            technique: AoTechnique::Ssao,
            intensity: 1.0,
            radius: 0.5,
            bias: 0.025,
            direct_lighting_strength: 0.0,
            sample_count: 16,
            temporal: true,
        }
    }

    /// High quality preset
    pub fn high() -> Self {
        Self {
            technique: AoTechnique::Gtao,
            intensity: 1.0,
            radius: 0.5,
            bias: 0.02,
            direct_lighting_strength: 0.25,
            sample_count: 32,
            temporal: true,
        }
    }

    /// Performance preset
    pub fn performance() -> Self {
        Self {
            technique: AoTechnique::Ssao,
            intensity: 0.8,
            radius: 0.3,
            bias: 0.03,
            direct_lighting_strength: 0.0,
            sample_count: 8,
            temporal: true,
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for AmbientOcclusionSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for AmbientOcclusionSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            technique: if weight > 0.5 { other.technique } else { self.technique },
            intensity: lerp(self.intensity, other.intensity, weight),
            radius: lerp(self.radius, other.radius, weight),
            bias: lerp(self.bias, other.bias, weight),
            direct_lighting_strength: lerp(self.direct_lighting_strength, other.direct_lighting_strength, weight),
            sample_count: if weight > 0.5 { other.sample_count } else { self.sample_count },
            temporal: if weight > 0.5 { other.temporal } else { self.temporal },
        }
    }
}

/// AO technique
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AoTechnique {
    /// SSAO
    #[default]
    Ssao = 0,
    /// HBAO+
    Hbao = 1,
    /// GTAO (ground truth AO)
    Gtao = 2,
    /// Ray traced AO
    RayTraced = 3,
}

// ============================================================================
// Film Grain Settings
// ============================================================================

/// Film grain settings
#[derive(Clone, Debug)]
pub struct FilmGrainSettings {
    /// Intensity
    pub intensity: f32,
    /// Response curve
    pub response: f32,
    /// Grain type
    pub grain_type: FilmGrainType,
}

impl FilmGrainSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            intensity: 0.2,
            response: 0.8,
            grain_type: FilmGrainType::Fine,
        }
    }

    /// Subtle grain
    pub fn subtle() -> Self {
        Self {
            intensity: 0.1,
            response: 0.9,
            grain_type: FilmGrainType::Fine,
        }
    }

    /// Film-like grain
    pub fn filmic() -> Self {
        Self {
            intensity: 0.3,
            response: 0.7,
            grain_type: FilmGrainType::Medium,
        }
    }
}

impl Default for FilmGrainSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for FilmGrainSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            intensity: lerp(self.intensity, other.intensity, weight),
            response: lerp(self.response, other.response, weight),
            grain_type: if weight > 0.5 { other.grain_type } else { self.grain_type },
        }
    }
}

/// Film grain type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FilmGrainType {
    /// Fine grain
    #[default]
    Fine = 0,
    /// Medium grain
    Medium = 1,
    /// Coarse grain
    Coarse = 2,
}

// ============================================================================
// Lens Flare Settings
// ============================================================================

/// Lens flare settings
#[derive(Clone, Debug)]
pub struct LensFlareSettings {
    /// Intensity
    pub intensity: f32,
    /// Threshold
    pub threshold: f32,
    /// Ghost count
    pub ghost_count: u32,
    /// Ghost spacing
    pub ghost_spacing: f32,
    /// Halo width
    pub halo_width: f32,
    /// Chromatic distortion
    pub chromatic_distortion: f32,
}

impl LensFlareSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            intensity: 1.0,
            threshold: 1.5,
            ghost_count: 8,
            ghost_spacing: 0.125,
            halo_width: 0.6,
            chromatic_distortion: 2.0,
        }
    }
}

impl Default for LensFlareSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for LensFlareSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            intensity: lerp(self.intensity, other.intensity, weight),
            threshold: lerp(self.threshold, other.threshold, weight),
            ghost_count: if weight > 0.5 { other.ghost_count } else { self.ghost_count },
            ghost_spacing: lerp(self.ghost_spacing, other.ghost_spacing, weight),
            halo_width: lerp(self.halo_width, other.halo_width, weight),
            chromatic_distortion: lerp(self.chromatic_distortion, other.chromatic_distortion, weight),
        }
    }
}

// ============================================================================
// Auto Exposure Settings
// ============================================================================

/// Auto exposure settings
#[derive(Clone, Debug)]
pub struct AutoExposureSettings {
    /// Minimum exposure
    pub min_exposure: f32,
    /// Maximum exposure
    pub max_exposure: f32,
    /// Adaptation speed (up)
    pub speed_up: f32,
    /// Adaptation speed (down)
    pub speed_down: f32,
    /// Exposure compensation (EV)
    pub compensation: f32,
    /// Metering mode
    pub metering_mode: MeteringMode,
}

impl AutoExposureSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            min_exposure: -10.0,
            max_exposure: 10.0,
            speed_up: 3.0,
            speed_down: 1.0,
            compensation: 0.0,
            metering_mode: MeteringMode::Average,
        }
    }

    /// With exposure range
    pub fn with_range(mut self, min: f32, max: f32) -> Self {
        self.min_exposure = min;
        self.max_exposure = max;
        self
    }
}

impl Default for AutoExposureSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for AutoExposureSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            min_exposure: lerp(self.min_exposure, other.min_exposure, weight),
            max_exposure: lerp(self.max_exposure, other.max_exposure, weight),
            speed_up: lerp(self.speed_up, other.speed_up, weight),
            speed_down: lerp(self.speed_down, other.speed_down, weight),
            compensation: lerp(self.compensation, other.compensation, weight),
            metering_mode: if weight > 0.5 { other.metering_mode } else { self.metering_mode },
        }
    }
}

/// Metering mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MeteringMode {
    /// Average metering
    #[default]
    Average = 0,
    /// Center weighted
    CenterWeighted = 1,
    /// Spot metering
    Spot = 2,
}

// ============================================================================
// Fog Settings
// ============================================================================

/// Fog settings
#[derive(Clone, Debug)]
pub struct FogSettings {
    /// Fog mode
    pub mode: FogMode,
    /// Fog color
    pub color: [f32; 3],
    /// Density (for exponential)
    pub density: f32,
    /// Start distance (for linear)
    pub start: f32,
    /// End distance (for linear)
    pub end: f32,
    /// Height fog start
    pub height_start: f32,
    /// Height fog falloff
    pub height_falloff: f32,
    /// Max opacity
    pub max_opacity: f32,
    /// Scattering
    pub scattering: f32,
}

impl FogSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            mode: FogMode::Linear,
            color: [0.7, 0.8, 0.9],
            density: 0.02,
            start: 10.0,
            end: 100.0,
            height_start: 0.0,
            height_falloff: 5.0,
            max_opacity: 1.0,
            scattering: 0.5,
        }
    }

    /// Atmospheric fog
    pub fn atmospheric() -> Self {
        Self {
            mode: FogMode::Exponential2,
            color: [0.7, 0.8, 1.0],
            density: 0.005,
            start: 0.0,
            end: 1000.0,
            height_start: -50.0,
            height_falloff: 100.0,
            max_opacity: 0.9,
            scattering: 0.7,
        }
    }

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = [r, g, b];
        self
    }
}

impl Default for FogSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for FogSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            mode: if weight > 0.5 { other.mode } else { self.mode },
            color: lerp_color(self.color, other.color, weight),
            density: lerp(self.density, other.density, weight),
            start: lerp(self.start, other.start, weight),
            end: lerp(self.end, other.end, weight),
            height_start: lerp(self.height_start, other.height_start, weight),
            height_falloff: lerp(self.height_falloff, other.height_falloff, weight),
            max_opacity: lerp(self.max_opacity, other.max_opacity, weight),
            scattering: lerp(self.scattering, other.scattering, weight),
        }
    }
}

/// Fog mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FogMode {
    /// Linear fog
    #[default]
    Linear = 0,
    /// Exponential fog
    Exponential = 1,
    /// Exponential squared fog
    Exponential2 = 2,
    /// Height-based fog
    Height = 3,
}

// ============================================================================
// Anti-Aliasing Settings
// ============================================================================

/// Anti-aliasing settings
#[derive(Clone, Debug)]
pub struct AntiAliasingSettings {
    /// AA technique
    pub technique: AaTechnique,
    /// Quality
    pub quality: AaQuality,
    /// Sharpness (for TAA)
    pub sharpness: f32,
}

impl AntiAliasingSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            technique: AaTechnique::Taa,
            quality: AaQuality::High,
            sharpness: 0.25,
        }
    }

    /// TAA settings
    pub fn taa() -> Self {
        Self {
            technique: AaTechnique::Taa,
            quality: AaQuality::High,
            sharpness: 0.25,
        }
    }

    /// FXAA settings
    pub fn fxaa() -> Self {
        Self {
            technique: AaTechnique::Fxaa,
            quality: AaQuality::Medium,
            sharpness: 0.0,
        }
    }

    /// SMAA settings
    pub fn smaa() -> Self {
        Self {
            technique: AaTechnique::Smaa,
            quality: AaQuality::High,
            sharpness: 0.0,
        }
    }
}

impl Default for AntiAliasingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// AA technique
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AaTechnique {
    /// None
    None = 0,
    /// FXAA
    Fxaa = 1,
    /// SMAA
    Smaa = 2,
    /// TAA
    #[default]
    Taa = 3,
    /// MSAA
    Msaa = 4,
}

/// AA quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AaQuality {
    /// Low
    Low = 0,
    /// Medium
    #[default]
    Medium = 1,
    /// High
    High = 2,
    /// Ultra
    Ultra = 3,
}

// ============================================================================
// Sharpening Settings
// ============================================================================

/// Sharpening settings
#[derive(Clone, Debug)]
pub struct SharpeningSettings {
    /// Intensity
    pub intensity: f32,
    /// Technique
    pub technique: SharpeningTechnique,
}

impl SharpeningSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            intensity: 0.5,
            technique: SharpeningTechnique::Cas,
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for SharpeningSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl Blendable for SharpeningSettings {
    fn blend(&self, other: &Self, weight: f32) -> Self {
        Self {
            intensity: lerp(self.intensity, other.intensity, weight),
            technique: if weight > 0.5 { other.technique } else { self.technique },
        }
    }
}

/// Sharpening technique
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SharpeningTechnique {
    /// CAS (contrast adaptive sharpening)
    #[default]
    Cas = 0,
    /// Unsharp mask
    UnsharpMask = 1,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Linear interpolation
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Color interpolation
fn lerp_color(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    ]
}
