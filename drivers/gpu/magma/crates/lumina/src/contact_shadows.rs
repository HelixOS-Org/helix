//! Contact Shadows Types for Lumina
//!
//! This module provides screen-space contact shadow infrastructure
//! for accurate close-range shadowing.

extern crate alloc;

use alloc::string::String;

// ============================================================================
// Contact Shadow Handles
// ============================================================================

/// Contact shadow handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ContactShadowHandle(pub u64);

impl ContactShadowHandle {
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

impl Default for ContactShadowHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Contact Shadow Configuration
// ============================================================================

/// Contact shadow create info
#[derive(Clone, Debug)]
pub struct ContactShadowCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Quality preset
    pub quality: ContactShadowQuality,
    /// Max ray length (world units)
    pub max_ray_length: f32,
    /// Thickness
    pub thickness: f32,
}

impl ContactShadowCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            quality: ContactShadowQuality::Medium,
            max_ray_length: 0.5,
            thickness: 0.02,
        }
    }

    /// Performance preset
    pub fn performance(width: u32, height: u32) -> Self {
        Self {
            quality: ContactShadowQuality::Low,
            max_ray_length: 0.3,
            ..Self::new(width, height)
        }
    }

    /// Quality preset
    pub fn high_quality(width: u32, height: u32) -> Self {
        Self {
            quality: ContactShadowQuality::High,
            max_ray_length: 0.8,
            ..Self::new(width, height)
        }
    }

    /// With ray length
    pub fn with_ray_length(mut self, length: f32) -> Self {
        self.max_ray_length = length;
        self
    }

    /// With thickness
    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

impl Default for ContactShadowCreateInfo {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

/// Contact shadow quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ContactShadowQuality {
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

impl ContactShadowQuality {
    /// Ray steps
    pub const fn ray_steps(&self) -> u32 {
        match self {
            Self::Low => 8,
            Self::Medium => 16,
            Self::High => 32,
            Self::Ultra => 64,
        }
    }

    /// Use dithering
    pub const fn use_dithering(&self) -> bool {
        matches!(self, Self::Medium | Self::High | Self::Ultra)
    }

    /// Use TAA denoise
    pub const fn use_temporal(&self) -> bool {
        matches!(self, Self::High | Self::Ultra)
    }
}

// ============================================================================
// Contact Shadow Settings
// ============================================================================

/// Contact shadow settings
#[derive(Clone, Copy, Debug)]
pub struct ContactShadowSettings {
    /// Enable
    pub enabled: bool,
    /// Ray length (world units)
    pub ray_length: f32,
    /// Ray steps
    pub ray_steps: u32,
    /// Thickness
    pub thickness: f32,
    /// Distance fade start
    pub fade_start: f32,
    /// Distance fade end
    pub fade_end: f32,
    /// Sample jitter
    pub jitter: f32,
    /// Intensity
    pub intensity: f32,
}

impl ContactShadowSettings {
    /// Default settings
    pub const fn default_settings() -> Self {
        Self {
            enabled: true,
            ray_length: 0.5,
            ray_steps: 16,
            thickness: 0.02,
            fade_start: 5.0,
            fade_end: 10.0,
            jitter: 0.5,
            intensity: 1.0,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default_settings()
        }
    }

    /// Performance settings
    pub const fn performance() -> Self {
        Self {
            ray_length: 0.3,
            ray_steps: 8,
            fade_end: 8.0,
            ..Self::default_settings()
        }
    }

    /// High quality settings
    pub const fn quality() -> Self {
        Self {
            ray_length: 0.8,
            ray_steps: 32,
            fade_end: 15.0,
            ..Self::default_settings()
        }
    }

    /// With ray length
    pub const fn with_ray_length(mut self, length: f32) -> Self {
        self.ray_length = length;
        self
    }

    /// With steps
    pub const fn with_steps(mut self, steps: u32) -> Self {
        self.ray_steps = steps;
        self
    }

    /// With intensity
    pub const fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for ContactShadowSettings {
    fn default() -> Self {
        Self::default_settings()
    }
}

// ============================================================================
// Per-Light Settings
// ============================================================================

/// Per-light contact shadow settings
#[derive(Clone, Copy, Debug)]
pub struct LightContactShadowSettings {
    /// Enable for this light
    pub enabled: bool,
    /// Ray length override (0 = use default)
    pub ray_length: f32,
    /// Intensity override
    pub intensity: f32,
}

impl LightContactShadowSettings {
    /// Default settings
    pub const fn default_light() -> Self {
        Self {
            enabled: true,
            ray_length: 0.0, // Use global
            intensity: 1.0,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default_light()
        }
    }

    /// With custom length
    pub const fn with_length(mut self, length: f32) -> Self {
        self.ray_length = length;
        self
    }
}

impl Default for LightContactShadowSettings {
    fn default() -> Self {
        Self::default_light()
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// Contact shadow GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ContactShadowGpuParams {
    /// View matrix
    pub view_matrix: [[f32; 4]; 4],
    /// Projection matrix
    pub proj_matrix: [[f32; 4]; 4],
    /// Inverse view matrix
    pub inv_view_matrix: [[f32; 4]; 4],
    /// Inverse projection matrix
    pub inv_proj_matrix: [[f32; 4]; 4],
    /// Light direction (world space)
    pub light_direction: [f32; 4],
    /// Screen dimensions
    pub screen_size: [f32; 2],
    /// Ray length
    pub ray_length: f32,
    /// Ray steps
    pub ray_steps: u32,
    /// Thickness
    pub thickness: f32,
    /// Fade start
    pub fade_start: f32,
    /// Fade end
    pub fade_end: f32,
    /// Jitter
    pub jitter: f32,
    /// Intensity
    pub intensity: f32,
    /// Near plane
    pub near_plane: f32,
    /// Far plane
    pub far_plane: f32,
    /// Frame index
    pub frame_index: u32,
}

impl ContactShadowGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// ============================================================================
// Ray Marching Types
// ============================================================================

/// Ray march mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum RayMarchMode {
    /// Linear stepping
    #[default]
    Linear       = 0,
    /// Exponential stepping
    Exponential  = 1,
    /// Hierarchical (HiZ)
    Hierarchical = 2,
    /// Hybrid
    Hybrid       = 3,
}

impl RayMarchMode {
    /// Requires HiZ buffer
    pub const fn requires_hiz(&self) -> bool {
        matches!(self, Self::Hierarchical | Self::Hybrid)
    }
}

/// Ray march settings
#[derive(Clone, Copy, Debug)]
pub struct RayMarchSettings {
    /// Mode
    pub mode: RayMarchMode,
    /// Max iterations
    pub max_iterations: u32,
    /// Start step size
    pub start_step_size: f32,
    /// Step growth
    pub step_growth: f32,
    /// Binary search iterations
    pub binary_search_steps: u32,
}

impl RayMarchSettings {
    /// Default settings
    pub const fn default_march() -> Self {
        Self {
            mode: RayMarchMode::Linear,
            max_iterations: 16,
            start_step_size: 0.01,
            step_growth: 1.0,
            binary_search_steps: 4,
        }
    }

    /// Exponential march
    pub const fn exponential() -> Self {
        Self {
            mode: RayMarchMode::Exponential,
            step_growth: 1.2,
            ..Self::default_march()
        }
    }

    /// Hierarchical march
    pub const fn hierarchical() -> Self {
        Self {
            mode: RayMarchMode::Hierarchical,
            max_iterations: 32,
            binary_search_steps: 8,
            ..Self::default_march()
        }
    }
}

impl Default for RayMarchSettings {
    fn default() -> Self {
        Self::default_march()
    }
}

// ============================================================================
// HiZ (Hierarchical Z) Buffer
// ============================================================================

/// HiZ buffer create info
#[derive(Clone, Debug)]
pub struct HizBufferCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Format
    pub format: HizFormat,
}

impl HizBufferCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        let mip_levels = (width.max(height) as f32).log2().ceil() as u32;
        Self {
            name: String::new(),
            width,
            height,
            mip_levels,
            format: HizFormat::R32Float,
        }
    }

    /// With limited mips
    pub fn with_max_mips(mut self, max: u32) -> Self {
        self.mip_levels = self.mip_levels.min(max);
        self
    }

    /// Memory size
    pub fn memory_size(&self) -> u64 {
        let mut size = 0u64;
        let mut w = self.width;
        let mut h = self.height;
        let bytes = self.format.bytes_per_pixel() as u64;

        for _ in 0..self.mip_levels {
            size += (w as u64) * (h as u64) * bytes;
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        size
    }
}

impl Default for HizBufferCreateInfo {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

/// HiZ format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HizFormat {
    /// R32 float (standard)
    #[default]
    R32Float = 0,
    /// R16 float (compact)
    R16Float = 1,
    /// D32 float (depth)
    D32Float = 2,
}

impl HizFormat {
    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R32Float | Self::D32Float => 4,
            Self::R16Float => 2,
        }
    }
}

// ============================================================================
// Pass Configuration
// ============================================================================

/// Contact shadow pass config
#[derive(Clone, Debug)]
pub struct ContactShadowPassConfig {
    /// Depth buffer
    pub depth: u64,
    /// Normal buffer (optional)
    pub normal: u64,
    /// HiZ buffer (optional)
    pub hiz: u64,
    /// Output shadow mask
    pub output: u64,
    /// Light direction
    pub light_direction: [f32; 3],
    /// Settings
    pub settings: ContactShadowSettings,
    /// March settings
    pub march: RayMarchSettings,
    /// Frame index (for jitter)
    pub frame_index: u32,
}

impl ContactShadowPassConfig {
    /// Creates config
    pub fn new(depth: u64, output: u64) -> Self {
        Self {
            depth,
            normal: 0,
            hiz: 0,
            output,
            light_direction: [0.0, -1.0, 0.0],
            settings: ContactShadowSettings::default(),
            march: RayMarchSettings::default(),
            frame_index: 0,
        }
    }

    /// With light direction
    pub fn with_light_direction(mut self, x: f32, y: f32, z: f32) -> Self {
        self.light_direction = [x, y, z];
        self
    }

    /// With settings
    pub fn with_settings(mut self, settings: ContactShadowSettings) -> Self {
        self.settings = settings;
        self
    }

    /// With HiZ
    pub fn with_hiz(mut self, hiz: u64) -> Self {
        self.hiz = hiz;
        self.march = RayMarchSettings::hierarchical();
        self
    }
}

impl Default for ContactShadowPassConfig {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

// ============================================================================
// Denoising
// ============================================================================

/// Contact shadow denoise settings
#[derive(Clone, Copy, Debug)]
pub struct ContactShadowDenoiseSettings {
    /// Enable temporal filtering
    pub temporal: bool,
    /// Temporal weight
    pub temporal_weight: f32,
    /// Enable spatial filter
    pub spatial: bool,
    /// Spatial radius
    pub spatial_radius: u32,
    /// Normal weight
    pub normal_weight: f32,
    /// Depth weight
    pub depth_weight: f32,
}

impl ContactShadowDenoiseSettings {
    /// Default settings
    pub const fn default_denoise() -> Self {
        Self {
            temporal: true,
            temporal_weight: 0.9,
            spatial: true,
            spatial_radius: 2,
            normal_weight: 1.0,
            depth_weight: 1.0,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            temporal: false,
            spatial: false,
            ..Self::default_denoise()
        }
    }

    /// Temporal only
    pub const fn temporal_only() -> Self {
        Self {
            spatial: false,
            ..Self::default_denoise()
        }
    }
}

impl Default for ContactShadowDenoiseSettings {
    fn default() -> Self {
        Self::default_denoise()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Contact shadow statistics
#[derive(Clone, Debug, Default)]
pub struct ContactShadowStats {
    /// Average ray steps
    pub avg_steps: f32,
    /// Shadow coverage
    pub shadow_coverage: f32,
    /// Pass time (microseconds)
    pub pass_time_us: u64,
    /// Denoise time (microseconds)
    pub denoise_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
