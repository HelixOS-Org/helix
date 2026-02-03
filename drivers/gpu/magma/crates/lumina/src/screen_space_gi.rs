//! Screen-Space Global Illumination Types for Lumina
//!
//! This module provides screen-space global illumination
//! techniques including SSGI, GTAO, and bent normals.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// SSGI Handles
// ============================================================================

/// Screen-space GI handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SsgiHandle(pub u64);

impl SsgiHandle {
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

impl Default for SsgiHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// GTAO (Ground Truth AO) handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GtaoHandle(pub u64);

impl GtaoHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for GtaoHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Bent normals handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BentNormalsHandle(pub u64);

impl BentNormalsHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for BentNormalsHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// SSDO (Screen-Space Directional Occlusion) handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SsdoHandle(pub u64);

impl SsdoHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SsdoHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// SSGI Creation
// ============================================================================

/// SSGI create info
#[derive(Clone, Debug)]
pub struct SsgiCreateInfo {
    /// Name
    pub name: String,
    /// Algorithm
    pub algorithm: SsgiAlgorithm,
    /// Resolution scale (0.5 = half, 1.0 = full)
    pub resolution_scale: f32,
    /// Ray count per pixel
    pub ray_count: u32,
    /// Max ray steps
    pub max_steps: u32,
    /// Stride (for hierarchical tracing)
    pub stride: u32,
    /// Max distance
    pub max_distance: f32,
    /// Thickness
    pub thickness: f32,
    /// Intensity
    pub intensity: f32,
    /// Temporal accumulation
    pub temporal: bool,
    /// Spatial denoising
    pub denoise: bool,
    /// Quality level
    pub quality: SsgiQuality,
}

impl SsgiCreateInfo {
    /// Creates new info
    pub fn new(algorithm: SsgiAlgorithm) -> Self {
        Self {
            name: String::new(),
            algorithm,
            resolution_scale: 0.5,
            ray_count: 4,
            max_steps: 16,
            stride: 4,
            max_distance: 100.0,
            thickness: 0.1,
            intensity: 1.0,
            temporal: true,
            denoise: true,
            quality: SsgiQuality::Medium,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With resolution scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.resolution_scale = scale;
        self
    }

    /// With ray count
    pub fn with_rays(mut self, count: u32) -> Self {
        self.ray_count = count;
        self
    }

    /// With max steps
    pub fn with_steps(mut self, steps: u32) -> Self {
        self.max_steps = steps;
        self
    }

    /// With max distance
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.max_distance = distance;
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// Enable temporal
    pub fn temporal(mut self, enabled: bool) -> Self {
        self.temporal = enabled;
        self
    }

    /// Enable denoise
    pub fn denoise(mut self, enabled: bool) -> Self {
        self.denoise = enabled;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: SsgiQuality) -> Self {
        self.quality = quality;
        // Apply quality settings
        match quality {
            SsgiQuality::Low => {
                self.resolution_scale = 0.25;
                self.ray_count = 2;
                self.max_steps = 8;
            },
            SsgiQuality::Medium => {
                self.resolution_scale = 0.5;
                self.ray_count = 4;
                self.max_steps = 16;
            },
            SsgiQuality::High => {
                self.resolution_scale = 0.75;
                self.ray_count = 8;
                self.max_steps = 32;
            },
            SsgiQuality::Ultra => {
                self.resolution_scale = 1.0;
                self.ray_count = 16;
                self.max_steps = 64;
            },
        }
        self
    }

    /// Low quality preset
    pub fn low_quality() -> Self {
        Self::new(SsgiAlgorithm::HiZ).with_quality(SsgiQuality::Low)
    }

    /// Medium quality preset
    pub fn medium_quality() -> Self {
        Self::new(SsgiAlgorithm::HiZ).with_quality(SsgiQuality::Medium)
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new(SsgiAlgorithm::HiZ).with_quality(SsgiQuality::High)
    }

    /// Ultra quality preset (ray traced)
    pub fn ultra_quality() -> Self {
        Self::new(SsgiAlgorithm::RayTraced).with_quality(SsgiQuality::Ultra)
    }
}

impl Default for SsgiCreateInfo {
    fn default() -> Self {
        Self::medium_quality()
    }
}

/// SSGI algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SsgiAlgorithm {
    /// Linear depth march
    Linear    = 0,
    /// Hierarchical Z (Hi-Z) march
    #[default]
    HiZ       = 1,
    /// DDA (Digital Differential Analyzer)
    Dda       = 2,
    /// Minimum hi-Z
    MinHiZ    = 3,
    /// Ray traced (requires RT hardware)
    RayTraced = 4,
}

impl SsgiAlgorithm {
    /// Display name
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Linear => "Linear",
            Self::HiZ => "Hi-Z",
            Self::Dda => "DDA",
            Self::MinHiZ => "Min Hi-Z",
            Self::RayTraced => "Ray Traced",
        }
    }

    /// Requires ray tracing
    pub const fn requires_ray_tracing(&self) -> bool {
        matches!(self, Self::RayTraced)
    }
}

/// SSGI quality level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SsgiQuality {
    /// Low quality
    Low    = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High   = 2,
    /// Ultra quality
    Ultra  = 3,
}

impl SsgiQuality {
    /// Resolution scale
    pub const fn resolution_scale(&self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 0.5,
            Self::High => 0.75,
            Self::Ultra => 1.0,
        }
    }

    /// Ray count
    pub const fn ray_count(&self) -> u32 {
        match self {
            Self::Low => 2,
            Self::Medium => 4,
            Self::High => 8,
            Self::Ultra => 16,
        }
    }
}

// ============================================================================
// GTAO (Ground Truth Ambient Occlusion)
// ============================================================================

/// GTAO create info
#[derive(Clone, Debug)]
pub struct GtaoCreateInfo {
    /// Name
    pub name: String,
    /// Resolution scale
    pub resolution_scale: f32,
    /// Direction count
    pub direction_count: u32,
    /// Steps per direction
    pub steps_per_direction: u32,
    /// Radius
    pub radius: f32,
    /// Falloff
    pub falloff: f32,
    /// Power
    pub power: f32,
    /// Intensity
    pub intensity: f32,
    /// Compute bent normals
    pub bent_normals: bool,
    /// Multi-bounce approximation
    pub multi_bounce: bool,
    /// Temporal filtering
    pub temporal: bool,
    /// Spatial blur
    pub blur: bool,
}

impl GtaoCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            resolution_scale: 1.0,
            direction_count: 4,
            steps_per_direction: 4,
            radius: 2.0,
            falloff: 1.0,
            power: 1.5,
            intensity: 1.0,
            bent_normals: false,
            multi_bounce: true,
            temporal: true,
            blur: true,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With resolution scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.resolution_scale = scale;
        self
    }

    /// With directions
    pub fn with_directions(mut self, count: u32) -> Self {
        self.direction_count = count;
        self
    }

    /// With steps
    pub fn with_steps(mut self, steps: u32) -> Self {
        self.steps_per_direction = steps;
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

    /// Enable bent normals
    pub fn with_bent_normals(mut self) -> Self {
        self.bent_normals = true;
        self
    }

    /// Low quality preset
    pub fn low_quality() -> Self {
        Self::new().with_scale(0.5).with_directions(2).with_steps(2)
    }

    /// Medium quality preset
    pub fn medium_quality() -> Self {
        Self::new().with_scale(1.0).with_directions(4).with_steps(4)
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new()
            .with_scale(1.0)
            .with_directions(6)
            .with_steps(6)
            .with_bent_normals()
    }

    /// Ultra quality preset
    pub fn ultra_quality() -> Self {
        Self::new()
            .with_scale(1.0)
            .with_directions(8)
            .with_steps(8)
            .with_bent_normals()
    }
}

impl Default for GtaoCreateInfo {
    fn default() -> Self {
        Self::medium_quality()
    }
}

// ============================================================================
// SSDO (Screen-Space Directional Occlusion)
// ============================================================================

/// SSDO create info
#[derive(Clone, Debug)]
pub struct SsdoCreateInfo {
    /// Name
    pub name: String,
    /// Sample count
    pub sample_count: u32,
    /// Radius
    pub radius: f32,
    /// Bias
    pub bias: f32,
    /// Intensity
    pub intensity: f32,
    /// Indirect intensity
    pub indirect_intensity: f32,
    /// Resolution scale
    pub resolution_scale: f32,
    /// Temporal filtering
    pub temporal: bool,
}

impl SsdoCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            sample_count: 16,
            radius: 1.0,
            bias: 0.01,
            intensity: 1.0,
            indirect_intensity: 1.0,
            resolution_scale: 1.0,
            temporal: true,
        }
    }

    /// With samples
    pub fn with_samples(mut self, count: u32) -> Self {
        self.sample_count = count;
        self
    }

    /// With radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, direct: f32, indirect: f32) -> Self {
        self.intensity = direct;
        self.indirect_intensity = indirect;
        self
    }

    /// Low quality preset
    pub fn low_quality() -> Self {
        Self::new().with_samples(8)
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new().with_samples(32)
    }
}

impl Default for SsdoCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU SSGI params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSsgiParams {
    /// Resolution (width, height)
    pub resolution: [u32; 2],
    /// Ray count
    pub ray_count: u32,
    /// Max steps
    pub max_steps: u32,
    /// Stride
    pub stride: u32,
    /// Max distance
    pub max_distance: f32,
    /// Thickness
    pub thickness: f32,
    /// Intensity
    pub intensity: f32,
    /// Frame index
    pub frame_index: u32,
    /// Jitter offset
    pub jitter: [f32; 2],
    /// Padding
    pub _padding: f32,
}

impl GpuSsgiParams {
    /// From create info
    pub fn from_create_info(info: &SsgiCreateInfo, width: u32, height: u32, frame: u32) -> Self {
        let scaled_width = (width as f32 * info.resolution_scale) as u32;
        let scaled_height = (height as f32 * info.resolution_scale) as u32;

        Self {
            resolution: [scaled_width, scaled_height],
            ray_count: info.ray_count,
            max_steps: info.max_steps,
            stride: info.stride,
            max_distance: info.max_distance,
            thickness: info.thickness,
            intensity: info.intensity,
            frame_index: frame,
            jitter: [0.0, 0.0],
            _padding: 0.0,
        }
    }
}

/// GPU GTAO params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuGtaoParams {
    /// Resolution (width, height)
    pub resolution: [u32; 2],
    /// Direction count
    pub direction_count: u32,
    /// Steps per direction
    pub steps_per_direction: u32,
    /// Radius
    pub radius: f32,
    /// Falloff
    pub falloff: f32,
    /// Power
    pub power: f32,
    /// Intensity
    pub intensity: f32,
    /// Frame index
    pub frame_index: u32,
    /// Multi-bounce enabled
    pub multi_bounce: u32,
    /// Bent normals enabled
    pub bent_normals: u32,
    /// Padding
    pub _padding: u32,
}

impl GpuGtaoParams {
    /// From create info
    pub fn from_create_info(info: &GtaoCreateInfo, width: u32, height: u32, frame: u32) -> Self {
        let scaled_width = (width as f32 * info.resolution_scale) as u32;
        let scaled_height = (height as f32 * info.resolution_scale) as u32;

        Self {
            resolution: [scaled_width, scaled_height],
            direction_count: info.direction_count,
            steps_per_direction: info.steps_per_direction,
            radius: info.radius,
            falloff: info.falloff,
            power: info.power,
            intensity: info.intensity,
            frame_index: frame,
            multi_bounce: if info.multi_bounce { 1 } else { 0 },
            bent_normals: if info.bent_normals { 1 } else { 0 },
            _padding: 0,
        }
    }
}

/// GPU SSDO params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSsdoParams {
    /// Resolution
    pub resolution: [u32; 2],
    /// Sample count
    pub sample_count: u32,
    /// Radius
    pub radius: f32,
    /// Bias
    pub bias: f32,
    /// Intensity
    pub intensity: f32,
    /// Indirect intensity
    pub indirect_intensity: f32,
    /// Frame index
    pub frame_index: u32,
}

// ============================================================================
// Bent Normals
// ============================================================================

/// Bent normals create info
#[derive(Clone, Debug)]
pub struct BentNormalsCreateInfo {
    /// Name
    pub name: String,
    /// Resolution scale
    pub resolution_scale: f32,
    /// Sample count
    pub sample_count: u32,
    /// Radius
    pub radius: f32,
    /// Use for specular occlusion
    pub specular_occlusion: bool,
}

impl BentNormalsCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            resolution_scale: 1.0,
            sample_count: 8,
            radius: 2.0,
            specular_occlusion: true,
        }
    }

    /// With samples
    pub fn with_samples(mut self, count: u32) -> Self {
        self.sample_count = count;
        self
    }

    /// With radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

impl Default for BentNormalsCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Render Requests
// ============================================================================

/// SSGI render request
#[derive(Clone, Debug)]
pub struct SsgiRenderRequest {
    /// SSGI handle
    pub ssgi: SsgiHandle,
    /// Depth buffer
    pub depth_buffer: u64,
    /// Normal buffer
    pub normal_buffer: u64,
    /// Color buffer (for reflection)
    pub color_buffer: u64,
    /// Output buffer
    pub output_buffer: u64,
    /// Hi-Z pyramid (optional)
    pub hiz_buffer: Option<u64>,
    /// Motion vectors (for temporal)
    pub motion_buffer: Option<u64>,
    /// Previous frame output (for temporal)
    pub history_buffer: Option<u64>,
}

impl SsgiRenderRequest {
    /// Creates new request
    pub fn new(ssgi: SsgiHandle, depth: u64, normal: u64, color: u64, output: u64) -> Self {
        Self {
            ssgi,
            depth_buffer: depth,
            normal_buffer: normal,
            color_buffer: color,
            output_buffer: output,
            hiz_buffer: None,
            motion_buffer: None,
            history_buffer: None,
        }
    }

    /// With Hi-Z
    pub fn with_hiz(mut self, hiz: u64) -> Self {
        self.hiz_buffer = Some(hiz);
        self
    }

    /// With temporal
    pub fn with_temporal(mut self, motion: u64, history: u64) -> Self {
        self.motion_buffer = Some(motion);
        self.history_buffer = Some(history);
        self
    }
}

/// GTAO render request
#[derive(Clone, Debug)]
pub struct GtaoRenderRequest {
    /// GTAO handle
    pub gtao: GtaoHandle,
    /// Depth buffer
    pub depth_buffer: u64,
    /// Normal buffer
    pub normal_buffer: u64,
    /// AO output
    pub ao_output: u64,
    /// Bent normal output (optional)
    pub bent_normal_output: Option<u64>,
    /// Motion vectors (for temporal)
    pub motion_buffer: Option<u64>,
    /// Previous frame (for temporal)
    pub history_buffer: Option<u64>,
}

impl GtaoRenderRequest {
    /// Creates new request
    pub fn new(gtao: GtaoHandle, depth: u64, normal: u64, output: u64) -> Self {
        Self {
            gtao,
            depth_buffer: depth,
            normal_buffer: normal,
            ao_output: output,
            bent_normal_output: None,
            motion_buffer: None,
            history_buffer: None,
        }
    }

    /// With bent normals
    pub fn with_bent_normals(mut self, output: u64) -> Self {
        self.bent_normal_output = Some(output);
        self
    }

    /// With temporal
    pub fn with_temporal(mut self, motion: u64, history: u64) -> Self {
        self.motion_buffer = Some(motion);
        self.history_buffer = Some(history);
        self
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// SSGI statistics
#[derive(Clone, Debug, Default)]
pub struct SsgiStats {
    /// Render time (microseconds)
    pub render_time_us: u64,
    /// Trace time (microseconds)
    pub trace_time_us: u64,
    /// Denoise time (microseconds)
    pub denoise_time_us: u64,
    /// Rays traced
    pub rays_traced: u64,
    /// Ray hit rate
    pub hit_rate: f32,
    /// Average ray steps
    pub avg_steps: f32,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

impl SsgiStats {
    /// Total time
    pub fn total_time_us(&self) -> u64 {
        self.render_time_us
    }
}

/// GTAO statistics
#[derive(Clone, Debug, Default)]
pub struct GtaoStats {
    /// Render time (microseconds)
    pub render_time_us: u64,
    /// Blur time (microseconds)
    pub blur_time_us: u64,
    /// Pixels processed
    pub pixels_processed: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
