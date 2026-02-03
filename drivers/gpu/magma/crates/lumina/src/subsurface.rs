//! Subsurface Scattering Types for Lumina
//!
//! This module provides subsurface scattering (SSS) infrastructure for
//! realistic skin, wax, marble, and organic material rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// SSS Handles
// ============================================================================

/// Subsurface profile handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SubsurfaceProfileHandle(pub u64);

impl SubsurfaceProfileHandle {
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

impl Default for SubsurfaceProfileHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// SSS material handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SssMaterialHandle(pub u64);

impl SssMaterialHandle {
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

impl Default for SssMaterialHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// SSS Model
// ============================================================================

/// Subsurface scattering model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SssModel {
    /// Separable SSS (screen-space)
    #[default]
    Separable = 0,
    /// Pre-integrated SSS
    PreIntegrated = 1,
    /// Burley normalized diffusion
    BurleyDiffusion = 2,
    /// Random walk SSS (most accurate)
    RandomWalk = 3,
    /// Christensen-Burley SSS
    ChristensenBurley = 4,
    /// Custom diffusion profile
    CustomProfile = 5,
}

impl SssModel {
    /// Is screen-space method
    pub const fn is_screen_space(&self) -> bool {
        matches!(self, Self::Separable | Self::PreIntegrated)
    }

    /// Is path-traced method
    pub const fn is_path_traced(&self) -> bool {
        matches!(self, Self::RandomWalk)
    }

    /// Complexity rating (1-5)
    pub const fn complexity(&self) -> u32 {
        match self {
            Self::PreIntegrated => 1,
            Self::Separable => 2,
            Self::BurleyDiffusion => 3,
            Self::ChristensenBurley => 4,
            Self::RandomWalk => 5,
            Self::CustomProfile => 3,
        }
    }
}

// ============================================================================
// Subsurface Profile
// ============================================================================

/// Subsurface profile create info
#[derive(Clone, Debug)]
pub struct SubsurfaceProfileCreateInfo {
    /// Name
    pub name: String,
    /// SSS model
    pub model: SssModel,
    /// Scatter color (subsurface color)
    pub scatter_color: [f32; 3],
    /// Scatter radius (in world units)
    pub scatter_radius: f32,
    /// Transmission tint
    pub transmission_tint: [f32; 3],
    /// Diffusion profile
    pub diffusion_profile: DiffusionProfile,
    /// Quality settings
    pub quality: SssQuality,
}

impl SubsurfaceProfileCreateInfo {
    /// Creates default profile
    pub fn new() -> Self {
        Self {
            name: String::new(),
            model: SssModel::Separable,
            scatter_color: [1.0, 0.4, 0.25],
            scatter_radius: 0.01,
            transmission_tint: [1.0, 0.4, 0.2],
            diffusion_profile: DiffusionProfile::skin(),
            quality: SssQuality::Medium,
        }
    }

    /// Skin profile
    pub fn skin() -> Self {
        Self {
            name: String::from("Skin"),
            model: SssModel::BurleyDiffusion,
            scatter_color: [0.8, 0.35, 0.2],
            scatter_radius: 0.012,
            transmission_tint: [1.0, 0.4, 0.25],
            diffusion_profile: DiffusionProfile::skin(),
            quality: SssQuality::High,
        }
    }

    /// Wax profile
    pub fn wax() -> Self {
        Self {
            name: String::from("Wax"),
            model: SssModel::Separable,
            scatter_color: [0.9, 0.7, 0.4],
            scatter_radius: 0.02,
            transmission_tint: [1.0, 0.8, 0.5],
            diffusion_profile: DiffusionProfile::wax(),
            quality: SssQuality::Medium,
        }
    }

    /// Marble profile
    pub fn marble() -> Self {
        Self {
            name: String::from("Marble"),
            model: SssModel::Separable,
            scatter_color: [0.95, 0.93, 0.88],
            scatter_radius: 0.05,
            transmission_tint: [0.95, 0.92, 0.85],
            diffusion_profile: DiffusionProfile::marble(),
            quality: SssQuality::Medium,
        }
    }

    /// Jade profile
    pub fn jade() -> Self {
        Self {
            name: String::from("Jade"),
            model: SssModel::BurleyDiffusion,
            scatter_color: [0.3, 0.7, 0.4],
            scatter_radius: 0.03,
            transmission_tint: [0.2, 0.6, 0.3],
            diffusion_profile: DiffusionProfile::jade(),
            quality: SssQuality::High,
        }
    }

    /// Milk profile
    pub fn milk() -> Self {
        Self {
            name: String::from("Milk"),
            model: SssModel::ChristensenBurley,
            scatter_color: [0.98, 0.97, 0.95],
            scatter_radius: 0.015,
            transmission_tint: [0.95, 0.9, 0.85],
            diffusion_profile: DiffusionProfile::milk(),
            quality: SssQuality::Medium,
        }
    }

    /// Leaf/plant profile
    pub fn leaf() -> Self {
        Self {
            name: String::from("Leaf"),
            model: SssModel::Separable,
            scatter_color: [0.5, 0.8, 0.3],
            scatter_radius: 0.005,
            transmission_tint: [0.6, 0.9, 0.2],
            diffusion_profile: DiffusionProfile::leaf(),
            quality: SssQuality::Low,
        }
    }

    /// With scatter color
    pub fn with_scatter_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.scatter_color = [r, g, b];
        self
    }

    /// With scatter radius
    pub fn with_scatter_radius(mut self, radius: f32) -> Self {
        self.scatter_radius = radius;
        self
    }

    /// With model
    pub fn with_model(mut self, model: SssModel) -> Self {
        self.model = model;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: SssQuality) -> Self {
        self.quality = quality;
        self
    }
}

impl Default for SubsurfaceProfileCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Diffusion Profile
// ============================================================================

/// Diffusion profile for SSS
#[derive(Clone, Debug)]
pub struct DiffusionProfile {
    /// Falloff weights
    pub falloff: [f32; 6],
    /// Scatter distances per channel
    pub scatter_distance: [f32; 3],
    /// Sharpness
    pub sharpness: f32,
    /// World scale
    pub world_scale: f32,
}

impl DiffusionProfile {
    /// Skin diffusion profile
    pub fn skin() -> Self {
        Self {
            falloff: [0.233, 0.455, 0.649, 0.0, 0.0, 0.0],
            scatter_distance: [1.0, 0.4, 0.25],
            sharpness: 0.4,
            world_scale: 1.0,
        }
    }

    /// Wax diffusion profile
    pub fn wax() -> Self {
        Self {
            falloff: [0.5, 0.3, 0.15, 0.05, 0.0, 0.0],
            scatter_distance: [2.0, 1.5, 1.0],
            sharpness: 0.3,
            world_scale: 1.0,
        }
    }

    /// Marble diffusion profile
    pub fn marble() -> Self {
        Self {
            falloff: [0.4, 0.35, 0.2, 0.05, 0.0, 0.0],
            scatter_distance: [2.5, 2.3, 2.0],
            sharpness: 0.5,
            world_scale: 1.0,
        }
    }

    /// Jade diffusion profile
    pub fn jade() -> Self {
        Self {
            falloff: [0.4, 0.4, 0.15, 0.05, 0.0, 0.0],
            scatter_distance: [1.5, 2.0, 0.8],
            sharpness: 0.6,
            world_scale: 1.0,
        }
    }

    /// Milk diffusion profile
    pub fn milk() -> Self {
        Self {
            falloff: [0.35, 0.35, 0.25, 0.05, 0.0, 0.0],
            scatter_distance: [1.8, 1.7, 1.5],
            sharpness: 0.25,
            world_scale: 1.0,
        }
    }

    /// Leaf diffusion profile
    pub fn leaf() -> Self {
        Self {
            falloff: [0.6, 0.3, 0.1, 0.0, 0.0, 0.0],
            scatter_distance: [0.5, 0.8, 0.3],
            sharpness: 0.7,
            world_scale: 1.0,
        }
    }

    /// Custom profile
    pub fn custom(scatter_distance: [f32; 3], sharpness: f32) -> Self {
        Self {
            falloff: [0.3, 0.3, 0.25, 0.1, 0.05, 0.0],
            scatter_distance,
            sharpness,
            world_scale: 1.0,
        }
    }

    /// With world scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.world_scale = scale;
        self
    }
}

impl Default for DiffusionProfile {
    fn default() -> Self {
        Self::skin()
    }
}

// ============================================================================
// SSS Quality
// ============================================================================

/// SSS quality preset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SssQuality {
    /// Low quality (fast)
    Low = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High = 2,
    /// Ultra quality (slow)
    Ultra = 3,
}

impl SssQuality {
    /// Sample count
    pub const fn sample_count(&self) -> u32 {
        match self {
            Self::Low => 11,
            Self::Medium => 21,
            Self::High => 35,
            Self::Ultra => 55,
        }
    }

    /// Blur passes
    pub const fn blur_passes(&self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Ultra => 4,
        }
    }
}

// ============================================================================
// SSS Material
// ============================================================================

/// SSS material create info
#[derive(Clone, Debug)]
pub struct SssMaterialCreateInfo {
    /// Name
    pub name: String,
    /// Profile
    pub profile: SubsurfaceProfileHandle,
    /// SSS intensity
    pub intensity: f32,
    /// Normal influence (0 = ignore normals)
    pub normal_influence: f32,
    /// Curvature influence
    pub curvature_influence: f32,
    /// Thickness
    pub thickness: f32,
    /// Back-lighting strength
    pub back_lighting: f32,
}

impl SssMaterialCreateInfo {
    /// Creates info
    pub fn new(profile: SubsurfaceProfileHandle) -> Self {
        Self {
            name: String::new(),
            profile,
            intensity: 1.0,
            normal_influence: 0.8,
            curvature_influence: 0.5,
            thickness: 1.0,
            back_lighting: 0.5,
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With back-lighting
    pub fn with_back_lighting(mut self, strength: f32) -> Self {
        self.back_lighting = strength;
        self
    }

    /// With thickness
    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

impl Default for SssMaterialCreateInfo {
    fn default() -> Self {
        Self::new(SubsurfaceProfileHandle::NULL)
    }
}

// ============================================================================
// SSS GPU Data
// ============================================================================

/// SSS GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SssGpuParams {
    /// Scatter color
    pub scatter_color: [f32; 4],
    /// Transmission tint
    pub transmission_tint: [f32; 4],
    /// Scatter radius
    pub scatter_radius: f32,
    /// Intensity
    pub intensity: f32,
    /// Normal influence
    pub normal_influence: f32,
    /// Curvature influence
    pub curvature_influence: f32,
    /// Thickness
    pub thickness: f32,
    /// Back-lighting
    pub back_lighting: f32,
    /// World scale
    pub world_scale: f32,
    /// Sharpness
    pub sharpness: f32,
}

impl SssGpuParams {
    /// Memory size
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// From profile and material
    pub fn from_profile_material(
        profile: &SubsurfaceProfileCreateInfo,
        material: &SssMaterialCreateInfo,
    ) -> Self {
        Self {
            scatter_color: [
                profile.scatter_color[0],
                profile.scatter_color[1],
                profile.scatter_color[2],
                1.0,
            ],
            transmission_tint: [
                profile.transmission_tint[0],
                profile.transmission_tint[1],
                profile.transmission_tint[2],
                1.0,
            ],
            scatter_radius: profile.scatter_radius,
            intensity: material.intensity,
            normal_influence: material.normal_influence,
            curvature_influence: material.curvature_influence,
            thickness: material.thickness,
            back_lighting: material.back_lighting,
            world_scale: profile.diffusion_profile.world_scale,
            sharpness: profile.diffusion_profile.sharpness,
        }
    }
}

// ============================================================================
// Separable SSS Kernel
// ============================================================================

/// Separable SSS kernel
#[derive(Clone, Debug)]
pub struct SeparableSssKernel {
    /// Kernel samples
    pub samples: Vec<SssSample>,
    /// Strength
    pub strength: [f32; 3],
    /// Falloff
    pub falloff: [f32; 3],
}

impl SeparableSssKernel {
    /// Maximum kernel size
    pub const MAX_SAMPLES: usize = 25;

    /// Creates kernel
    pub fn new(num_samples: u32, strength: [f32; 3], falloff: [f32; 3]) -> Self {
        let samples = Self::calculate_samples(num_samples, strength, falloff);
        Self {
            samples,
            strength,
            falloff,
        }
    }

    /// Skin kernel
    pub fn skin() -> Self {
        Self::new(21, [0.48, 0.41, 0.28], [1.0, 0.37, 0.3])
    }

    /// Generic kernel
    pub fn generic(radius: f32) -> Self {
        Self::new(
            21,
            [radius, radius * 0.8, radius * 0.6],
            [1.0, 0.5, 0.4],
        )
    }

    /// Calculate kernel samples
    fn calculate_samples(num_samples: u32, strength: [f32; 3], falloff: [f32; 3]) -> Vec<SssSample> {
        let mut samples = Vec::with_capacity(num_samples as usize);
        let range = 2.0;
        let exponent = 2.0;

        for i in 0..num_samples {
            // Calculate offset using exponential distribution
            let t = if num_samples > 1 {
                i as f32 / (num_samples - 1) as f32
            } else {
                0.0
            };
            let offset = range * (2.0 * t - 1.0).abs().powf(exponent) * if t < 0.5 { -1.0 } else { 1.0 };

            // Calculate weights per channel
            let weight_r = Self::gaussian(offset, falloff[0]) * strength[0];
            let weight_g = Self::gaussian(offset, falloff[1]) * strength[1];
            let weight_b = Self::gaussian(offset, falloff[2]) * strength[2];

            samples.push(SssSample {
                offset,
                weight: [weight_r, weight_g, weight_b],
            });
        }

        // Normalize weights
        let mut sum = [0.0f32; 3];
        for sample in &samples {
            sum[0] += sample.weight[0];
            sum[1] += sample.weight[1];
            sum[2] += sample.weight[2];
        }

        for sample in &mut samples {
            if sum[0] > 0.0 {
                sample.weight[0] /= sum[0];
            }
            if sum[1] > 0.0 {
                sample.weight[1] /= sum[1];
            }
            if sum[2] > 0.0 {
                sample.weight[2] /= sum[2];
            }
        }

        samples
    }

    /// Gaussian function
    fn gaussian(x: f32, variance: f32) -> f32 {
        (-x * x / (2.0 * variance * variance)).exp()
    }
}

/// SSS sample
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SssSample {
    /// Offset
    pub offset: f32,
    /// Weight per channel
    pub weight: [f32; 3],
}

// ============================================================================
// Pre-Integrated SSS
// ============================================================================

/// Pre-integrated SSS LUT create info
#[derive(Clone, Copy, Debug)]
pub struct PreIntegratedSssLutInfo {
    /// LUT resolution
    pub resolution: u32,
    /// Number of wrap angles
    pub num_wrap_angles: u32,
    /// Scatter width
    pub scatter_width: f32,
}

impl PreIntegratedSssLutInfo {
    /// Low quality
    pub const fn low() -> Self {
        Self {
            resolution: 128,
            num_wrap_angles: 8,
            scatter_width: 0.5,
        }
    }

    /// Medium quality
    pub const fn medium() -> Self {
        Self {
            resolution: 256,
            num_wrap_angles: 16,
            scatter_width: 0.5,
        }
    }

    /// High quality
    pub const fn high() -> Self {
        Self {
            resolution: 512,
            num_wrap_angles: 32,
            scatter_width: 0.5,
        }
    }

    /// With scatter width
    pub const fn with_scatter_width(mut self, width: f32) -> Self {
        self.scatter_width = width;
        self
    }
}

impl Default for PreIntegratedSssLutInfo {
    fn default() -> Self {
        Self::medium()
    }
}

// ============================================================================
// Random Walk SSS
// ============================================================================

/// Random walk SSS settings
#[derive(Clone, Copy, Debug)]
pub struct RandomWalkSssSettings {
    /// Maximum ray depth
    pub max_bounces: u32,
    /// Samples per pixel
    pub samples_per_pixel: u32,
    /// Extinction coefficient scale
    pub extinction_scale: f32,
    /// Use IOR for fresnel
    pub use_fresnel: bool,
    /// Index of refraction
    pub ior: f32,
}

impl RandomWalkSssSettings {
    /// Fast settings
    pub const fn fast() -> Self {
        Self {
            max_bounces: 8,
            samples_per_pixel: 1,
            extinction_scale: 1.0,
            use_fresnel: true,
            ior: 1.4,
        }
    }

    /// Quality settings
    pub const fn quality() -> Self {
        Self {
            max_bounces: 32,
            samples_per_pixel: 4,
            extinction_scale: 1.0,
            use_fresnel: true,
            ior: 1.4,
        }
    }

    /// Reference settings
    pub const fn reference() -> Self {
        Self {
            max_bounces: 64,
            samples_per_pixel: 16,
            extinction_scale: 1.0,
            use_fresnel: true,
            ior: 1.4,
        }
    }

    /// With bounces
    pub const fn with_bounces(mut self, bounces: u32) -> Self {
        self.max_bounces = bounces;
        self
    }

    /// With samples
    pub const fn with_samples(mut self, samples: u32) -> Self {
        self.samples_per_pixel = samples;
        self
    }
}

impl Default for RandomWalkSssSettings {
    fn default() -> Self {
        Self::quality()
    }
}

// ============================================================================
// Transmission
// ============================================================================

/// Transmission settings
#[derive(Clone, Copy, Debug)]
pub struct TransmissionSettings {
    /// Enable transmission
    pub enabled: bool,
    /// Transmission color
    pub color: [f32; 3],
    /// Transmission weight
    pub weight: f32,
    /// Depth scale (for thin objects)
    pub depth_scale: f32,
    /// Shadow bias
    pub shadow_bias: f32,
}

impl TransmissionSettings {
    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            color: [1.0, 1.0, 1.0],
            weight: 0.0,
            depth_scale: 1.0,
            shadow_bias: 0.001,
        }
    }

    /// Default transmission
    pub const fn default_transmission() -> Self {
        Self {
            enabled: true,
            color: [1.0, 0.8, 0.6],
            weight: 1.0,
            depth_scale: 1.0,
            shadow_bias: 0.001,
        }
    }

    /// Thin surface
    pub const fn thin() -> Self {
        Self {
            enabled: true,
            color: [1.0, 1.0, 1.0],
            weight: 1.0,
            depth_scale: 0.1,
            shadow_bias: 0.0001,
        }
    }

    /// With color
    pub const fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = [r, g, b];
        self
    }

    /// With weight
    pub const fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }
}

impl Default for TransmissionSettings {
    fn default() -> Self {
        Self::disabled()
    }
}

// ============================================================================
// SSS Pass Configuration
// ============================================================================

/// SSS pass configuration
#[derive(Clone, Debug)]
pub struct SssPassConfig {
    /// Model to use
    pub model: SssModel,
    /// Quality
    pub quality: SssQuality,
    /// Enable stencil test
    pub use_stencil: bool,
    /// Stencil reference value
    pub stencil_ref: u8,
    /// Follow surface
    pub follow_surface: bool,
    /// Sample radius in pixels
    pub sample_radius_pixels: f32,
    /// Enable transmission
    pub transmission: bool,
}

impl SssPassConfig {
    /// Screen-space SSS
    pub fn screen_space() -> Self {
        Self {
            model: SssModel::Separable,
            quality: SssQuality::Medium,
            use_stencil: true,
            stencil_ref: 1,
            follow_surface: true,
            sample_radius_pixels: 20.0,
            transmission: false,
        }
    }

    /// Full SSS with transmission
    pub fn full() -> Self {
        Self {
            model: SssModel::BurleyDiffusion,
            quality: SssQuality::High,
            use_stencil: true,
            stencil_ref: 1,
            follow_surface: true,
            sample_radius_pixels: 30.0,
            transmission: true,
        }
    }

    /// Performance config
    pub fn performance() -> Self {
        Self {
            model: SssModel::PreIntegrated,
            quality: SssQuality::Low,
            use_stencil: true,
            stencil_ref: 1,
            follow_surface: false,
            sample_radius_pixels: 15.0,
            transmission: false,
        }
    }

    /// With model
    pub fn with_model(mut self, model: SssModel) -> Self {
        self.model = model;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: SssQuality) -> Self {
        self.quality = quality;
        self
    }
}

impl Default for SssPassConfig {
    fn default() -> Self {
        Self::screen_space()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// SSS statistics
#[derive(Clone, Debug, Default)]
pub struct SssStats {
    /// Active profiles
    pub profile_count: u32,
    /// SSS materials
    pub material_count: u32,
    /// Pixels affected by SSS
    pub affected_pixels: u64,
    /// SSS pass time (microseconds)
    pub pass_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
