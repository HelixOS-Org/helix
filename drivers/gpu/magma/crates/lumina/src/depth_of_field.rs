//! Depth of Field Types for Lumina
//!
//! This module provides depth of field rendering infrastructure including
//! bokeh simulation, circle of confusion, and aperture effects.

extern crate alloc;

use alloc::string::String;

// ============================================================================
// DOF Handles
// ============================================================================

/// DOF effect handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DofHandle(pub u64);

impl DofHandle {
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

impl Default for DofHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Bokeh texture handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BokehTextureHandle(pub u64);

impl BokehTextureHandle {
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

impl Default for BokehTextureHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// DOF Configuration
// ============================================================================

/// DOF create info
#[derive(Clone, Debug)]
pub struct DofCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// DOF method
    pub method: DofMethod,
    /// Quality preset
    pub quality: DofQuality,
    /// Focus settings
    pub focus: FocusSettings,
    /// Bokeh settings
    pub bokeh: BokehSettings,
}

impl DofCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            method: DofMethod::Gather,
            quality: DofQuality::Medium,
            focus: FocusSettings::default(),
            bokeh: BokehSettings::default(),
        }
    }

    /// Cinematic DOF
    pub fn cinematic(width: u32, height: u32) -> Self {
        Self {
            method: DofMethod::GatherWithBokeh,
            quality: DofQuality::High,
            bokeh: BokehSettings::cinematic(),
            ..Self::new(width, height)
        }
    }

    /// Fast DOF
    pub fn fast(width: u32, height: u32) -> Self {
        Self {
            method: DofMethod::Gaussian,
            quality: DofQuality::Low,
            ..Self::new(width, height)
        }
    }

    /// With method
    pub fn with_method(mut self, method: DofMethod) -> Self {
        self.method = method;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: DofQuality) -> Self {
        self.quality = quality;
        self
    }

    /// With focus
    pub fn with_focus(mut self, focus: FocusSettings) -> Self {
        self.focus = focus;
        self
    }
}

impl Default for DofCreateInfo {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

/// DOF method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DofMethod {
    /// Gaussian blur (fast, low quality)
    Gaussian        = 0,
    /// Gather-based DOF
    #[default]
    Gather          = 1,
    /// Gather with bokeh sprites
    GatherWithBokeh = 2,
    /// Scatter-based DOF (high quality)
    Scatter         = 3,
    /// Physically-based (ray-traced)
    PhysicallyBased = 4,
}

impl DofMethod {
    /// Requires bokeh texture
    pub const fn requires_bokeh_texture(&self) -> bool {
        matches!(self, Self::GatherWithBokeh | Self::Scatter)
    }

    /// Is physically-based
    pub const fn is_physical(&self) -> bool {
        matches!(self, Self::PhysicallyBased)
    }

    /// Complexity level
    pub const fn complexity(&self) -> u32 {
        match self {
            Self::Gaussian => 1,
            Self::Gather => 2,
            Self::GatherWithBokeh => 3,
            Self::Scatter => 4,
            Self::PhysicallyBased => 5,
        }
    }
}

/// DOF quality preset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DofQuality {
    /// Low quality (fast)
    Low    = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High   = 2,
    /// Ultra quality
    Ultra  = 3,
}

impl DofQuality {
    /// Ring count (for gather)
    pub const fn ring_count(&self) -> u32 {
        match self {
            Self::Low => 2,
            Self::Medium => 3,
            Self::High => 4,
            Self::Ultra => 5,
        }
    }

    /// Sample count per ring
    pub const fn samples_per_ring(&self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 6,
            Self::High => 8,
            Self::Ultra => 10,
        }
    }

    /// Total samples
    pub fn total_samples(&self) -> u32 {
        let rings = self.ring_count();
        let per_ring = self.samples_per_ring();
        1 + (1..=rings).map(|r| r * per_ring).sum::<u32>()
    }
}

// ============================================================================
// Focus Settings
// ============================================================================

/// Focus settings
#[derive(Clone, Copy, Debug)]
pub struct FocusSettings {
    /// Focus mode
    pub mode: FocusMode,
    /// Focus distance (world units)
    pub focus_distance: f32,
    /// Focus range (for DOF falloff)
    pub focus_range: f32,
    /// Near blur start
    pub near_blur_start: f32,
    /// Near blur end
    pub near_blur_end: f32,
    /// Far blur start
    pub far_blur_start: f32,
    /// Far blur end
    pub far_blur_end: f32,
    /// Max blur size (pixels)
    pub max_blur_size: f32,
}

impl FocusSettings {
    /// Default focus
    pub const fn default_focus() -> Self {
        Self {
            mode: FocusMode::Manual,
            focus_distance: 10.0,
            focus_range: 5.0,
            near_blur_start: 4.0,
            near_blur_end: 2.0,
            far_blur_start: 15.0,
            far_blur_end: 50.0,
            max_blur_size: 16.0,
        }
    }

    /// Portrait focus (shallow DOF)
    pub const fn portrait() -> Self {
        Self {
            mode: FocusMode::Manual,
            focus_distance: 2.0,
            focus_range: 0.5,
            near_blur_start: 1.5,
            near_blur_end: 0.5,
            far_blur_start: 2.5,
            far_blur_end: 10.0,
            max_blur_size: 32.0,
        }
    }

    /// Landscape focus (deep DOF)
    pub const fn landscape() -> Self {
        Self {
            mode: FocusMode::Manual,
            focus_distance: 100.0,
            focus_range: 50.0,
            near_blur_start: 50.0,
            near_blur_end: 5.0,
            far_blur_start: 150.0,
            far_blur_end: 1000.0,
            max_blur_size: 8.0,
        }
    }

    /// With focus distance
    pub const fn with_distance(mut self, distance: f32) -> Self {
        self.focus_distance = distance;
        self
    }

    /// With max blur
    pub const fn with_max_blur(mut self, size: f32) -> Self {
        self.max_blur_size = size;
        self
    }
}

impl Default for FocusSettings {
    fn default() -> Self {
        Self::default_focus()
    }
}

/// Focus mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FocusMode {
    /// Manual focus
    #[default]
    Manual     = 0,
    /// Auto focus (screen center)
    AutoCenter = 1,
    /// Auto focus (track point)
    AutoTrack  = 2,
    /// Auto focus (face detection)
    AutoFace   = 3,
    /// Physical camera
    Physical   = 4,
}

// ============================================================================
// Bokeh Settings
// ============================================================================

/// Bokeh settings
#[derive(Clone, Copy, Debug)]
pub struct BokehSettings {
    /// Bokeh shape
    pub shape: BokehShape,
    /// Blade count (for polygonal bokeh)
    pub blade_count: u32,
    /// Blade curvature (0 = straight, 1 = circular)
    pub blade_curvature: f32,
    /// Rotation angle (radians)
    pub rotation: f32,
    /// Anamorphic ratio (1 = circular)
    pub anamorphic_ratio: f32,
    /// Highlight threshold
    pub highlight_threshold: f32,
    /// Highlight boost
    pub highlight_boost: f32,
    /// Chromatic aberration
    pub chromatic_aberration: f32,
}

impl BokehSettings {
    /// Default bokeh
    pub const fn default_bokeh() -> Self {
        Self {
            shape: BokehShape::Circular,
            blade_count: 6,
            blade_curvature: 0.5,
            rotation: 0.0,
            anamorphic_ratio: 1.0,
            highlight_threshold: 1.0,
            highlight_boost: 1.0,
            chromatic_aberration: 0.0,
        }
    }

    /// Cinematic bokeh
    pub const fn cinematic() -> Self {
        Self {
            shape: BokehShape::Polygonal,
            blade_count: 9,
            blade_curvature: 0.8,
            rotation: 0.0,
            anamorphic_ratio: 1.33,
            highlight_threshold: 0.8,
            highlight_boost: 1.5,
            chromatic_aberration: 0.02,
        }
    }

    /// Vintage bokeh
    pub const fn vintage() -> Self {
        Self {
            shape: BokehShape::Polygonal,
            blade_count: 5,
            blade_curvature: 0.0,
            rotation: 0.314, // ~18 degrees
            anamorphic_ratio: 1.0,
            highlight_threshold: 0.7,
            highlight_boost: 2.0,
            chromatic_aberration: 0.05,
        }
    }

    /// With blade count
    pub const fn with_blades(mut self, count: u32) -> Self {
        self.blade_count = count;
        self
    }

    /// With anamorphic
    pub const fn with_anamorphic(mut self, ratio: f32) -> Self {
        self.anamorphic_ratio = ratio;
        self
    }
}

impl Default for BokehSettings {
    fn default() -> Self {
        Self::default_bokeh()
    }
}

/// Bokeh shape
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BokehShape {
    /// Circular
    #[default]
    Circular  = 0,
    /// Polygonal (aperture blades)
    Polygonal = 1,
    /// Custom texture
    Custom    = 2,
    /// Cat's eye (optical vignette)
    CatsEye   = 3,
}

// ============================================================================
// Circle of Confusion
// ============================================================================

/// Circle of confusion calculator
#[derive(Clone, Copy, Debug)]
pub struct CocCalculator {
    /// Focal length (mm)
    pub focal_length: f32,
    /// Aperture (f-number)
    pub aperture: f32,
    /// Sensor size (mm)
    pub sensor_width: f32,
    /// Focus distance (world units)
    pub focus_distance: f32,
}

impl CocCalculator {
    /// Creates calculator
    pub const fn new(focal_length: f32, aperture: f32, focus_distance: f32) -> Self {
        Self {
            focal_length,
            aperture,
            sensor_width: 36.0, // Full frame
            focus_distance,
        }
    }

    /// 50mm f/1.8
    pub const fn standard_50mm() -> Self {
        Self::new(50.0, 1.8, 10.0)
    }

    /// 85mm f/1.4 (portrait)
    pub const fn portrait_85mm() -> Self {
        Self::new(85.0, 1.4, 3.0)
    }

    /// 35mm f/2.8 (landscape)
    pub const fn landscape_35mm() -> Self {
        Self::new(35.0, 2.8, 100.0)
    }

    /// Calculate CoC for given distance
    pub fn calculate_coc(&self, distance: f32) -> f32 {
        // Simplified thin lens CoC formula
        let ms = self.focal_length / (self.focus_distance * 1000.0 - self.focal_length);
        let mp = self.focal_length / (distance * 1000.0 - self.focal_length);

        let coc_mm = ((ms - mp) * self.focal_length / self.aperture).abs();

        // Convert to normalized (0-1) based on sensor
        coc_mm / self.sensor_width
    }

    /// Hyperfocal distance
    pub fn hyperfocal_distance(&self) -> f32 {
        // Circle of confusion for acceptable sharpness (typically ~0.03mm for full frame)
        let coc = 0.03;
        (self.focal_length * self.focal_length) / (self.aperture * coc) / 1000.0
    }

    /// Near focus limit
    pub fn near_focus_limit(&self) -> f32 {
        let h = self.hyperfocal_distance();
        (self.focus_distance * h) / (h + self.focus_distance)
    }

    /// Far focus limit
    pub fn far_focus_limit(&self) -> f32 {
        let h = self.hyperfocal_distance();
        if self.focus_distance >= h {
            f32::INFINITY
        } else {
            (self.focus_distance * h) / (h - self.focus_distance)
        }
    }
}

impl Default for CocCalculator {
    fn default() -> Self {
        Self::standard_50mm()
    }
}

// ============================================================================
// DOF Pass Configuration
// ============================================================================

/// DOF pass config
#[derive(Clone, Debug)]
pub struct DofPassConfig {
    /// Input color
    pub color: u64,
    /// Input depth
    pub depth: u64,
    /// Output
    pub output: u64,
    /// DOF settings
    pub settings: DofSettings,
}

impl DofPassConfig {
    /// Creates config
    pub fn new(color: u64, depth: u64, output: u64) -> Self {
        Self {
            color,
            depth,
            output,
            settings: DofSettings::default(),
        }
    }

    /// With settings
    pub fn with_settings(mut self, settings: DofSettings) -> Self {
        self.settings = settings;
        self
    }
}

impl Default for DofPassConfig {
    fn default() -> Self {
        Self::new(0, 0, 0)
    }
}

/// DOF settings
#[derive(Clone, Copy, Debug)]
pub struct DofSettings {
    /// Method
    pub method: DofMethod,
    /// Quality
    pub quality: DofQuality,
    /// Focus distance
    pub focus_distance: f32,
    /// Aperture
    pub aperture: f32,
    /// Focal length
    pub focal_length: f32,
    /// Max blur radius (pixels)
    pub max_blur_radius: f32,
    /// Near transition
    pub near_transition: f32,
    /// Far transition
    pub far_transition: f32,
}

impl DofSettings {
    /// Default settings
    pub const fn default_settings() -> Self {
        Self {
            method: DofMethod::Gather,
            quality: DofQuality::Medium,
            focus_distance: 10.0,
            aperture: 2.8,
            focal_length: 50.0,
            max_blur_radius: 16.0,
            near_transition: 0.3,
            far_transition: 0.5,
        }
    }

    /// With focus
    pub const fn with_focus(mut self, distance: f32) -> Self {
        self.focus_distance = distance;
        self
    }

    /// With aperture
    pub const fn with_aperture(mut self, f_stop: f32) -> Self {
        self.aperture = f_stop;
        self
    }
}

impl Default for DofSettings {
    fn default() -> Self {
        Self::default_settings()
    }
}

/// DOF GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DofGpuParams {
    /// Screen dimensions
    pub screen_size: [f32; 2],
    /// Focus distance
    pub focus_distance: f32,
    /// Aperture
    pub aperture: f32,
    /// Focal length
    pub focal_length: f32,
    /// Sensor width
    pub sensor_width: f32,
    /// Max blur radius
    pub max_blur_radius: f32,
    /// Near transition
    pub near_transition: f32,
    /// Far transition
    pub far_transition: f32,
    /// Blade count
    pub blade_count: u32,
    /// Blade curvature
    pub blade_curvature: f32,
    /// Anamorphic ratio
    pub anamorphic_ratio: f32,
    /// Highlight threshold
    pub highlight_threshold: f32,
    /// Highlight boost
    pub highlight_boost: f32,
    /// Chromatic aberration
    pub chromatic_aberration: f32,
    /// Flags
    pub flags: u32,
}

impl DofGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Use bokeh sprites flag
    pub const FLAG_BOKEH_SPRITES: u32 = 1 << 0;
    /// Use custom bokeh texture flag
    pub const FLAG_CUSTOM_BOKEH: u32 = 1 << 1;
    /// Near blur enabled flag
    pub const FLAG_NEAR_BLUR: u32 = 1 << 2;
    /// Far blur enabled flag
    pub const FLAG_FAR_BLUR: u32 = 1 << 3;
}

// ============================================================================
// Bokeh Sprite Generation
// ============================================================================

/// Bokeh sprite generation config
#[derive(Clone, Debug)]
pub struct BokehSpriteConfig {
    /// Color input
    pub color: u64,
    /// CoC buffer
    pub coc: u64,
    /// Sprite output buffer
    pub output: u64,
    /// Max sprites
    pub max_sprites: u32,
    /// Threshold for sprite generation
    pub luminance_threshold: f32,
    /// Minimum CoC for sprite
    pub min_coc: f32,
}

impl BokehSpriteConfig {
    /// Creates config
    pub fn new(color: u64, coc: u64, output: u64) -> Self {
        Self {
            color,
            coc,
            output,
            max_sprites: 10000,
            luminance_threshold: 1.0,
            min_coc: 4.0,
        }
    }
}

impl Default for BokehSpriteConfig {
    fn default() -> Self {
        Self::new(0, 0, 0)
    }
}

/// Bokeh sprite data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BokehSprite {
    /// Position (screen space)
    pub position: [f32; 2],
    /// Size (CoC diameter)
    pub size: f32,
    /// Rotation
    pub rotation: f32,
    /// Color
    pub color: [f32; 4],
}

impl BokehSprite {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// ============================================================================
// Statistics
// ============================================================================

/// DOF statistics
#[derive(Clone, Debug, Default)]
pub struct DofStats {
    /// Samples per pixel (average)
    pub avg_samples: f32,
    /// Bokeh sprites generated
    pub bokeh_sprites: u32,
    /// Near blur pixels
    pub near_blur_pixels: u64,
    /// Far blur pixels
    pub far_blur_pixels: u64,
    /// In-focus pixels
    pub in_focus_pixels: u64,
    /// DOF pass time (microseconds)
    pub pass_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
