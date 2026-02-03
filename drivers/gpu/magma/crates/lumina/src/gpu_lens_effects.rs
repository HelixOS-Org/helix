//! GPU Lens Effects Types for Lumina
//!
//! This module provides GPU-accelerated lens effects infrastructure
//! including lens flares, anamorphic effects, and optical phenomena.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Lens Effects Handles
// ============================================================================

/// GPU lens effects system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuLensEffectsHandle(pub u64);

impl GpuLensEffectsHandle {
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

impl Default for GpuLensEffectsHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Lens flare handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LensFlareHandle(pub u64);

impl LensFlareHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LensFlareHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Lens element handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LensElementHandle(pub u64);

impl LensElementHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LensElementHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Bokeh handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BokehHandle(pub u64);

impl BokehHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for BokehHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Lens Effects System Creation
// ============================================================================

/// GPU lens effects system create info
#[derive(Clone, Debug)]
pub struct GpuLensEffectsSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max flares
    pub max_flares: u32,
    /// Max elements per flare
    pub max_elements: u32,
    /// Max light sources
    pub max_light_sources: u32,
    /// Features
    pub features: LensEffectsFeatures,
}

impl GpuLensEffectsSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_flares: 32,
            max_elements: 16,
            max_light_sources: 64,
            features: LensEffectsFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max flares
    pub fn with_max_flares(mut self, count: u32) -> Self {
        self.max_flares = count;
        self
    }

    /// With max elements
    pub fn with_max_elements(mut self, count: u32) -> Self {
        self.max_elements = count;
        self
    }

    /// With max light sources
    pub fn with_max_lights(mut self, count: u32) -> Self {
        self.max_light_sources = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: LensEffectsFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard system
    pub fn standard() -> Self {
        Self::new()
    }

    /// Cinematic
    pub fn cinematic() -> Self {
        Self::new()
            .with_max_flares(64)
            .with_max_elements(32)
            .with_features(LensEffectsFeatures::all())
    }

    /// Minimal
    pub fn minimal() -> Self {
        Self::new()
            .with_max_flares(8)
            .with_max_elements(8)
            .with_max_lights(16)
    }
}

impl Default for GpuLensEffectsSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Lens effects features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct LensEffectsFeatures: u32 {
        /// None
        const NONE = 0;
        /// Lens flares
        const LENS_FLARE = 1 << 0;
        /// Chromatic aberration
        const CHROMATIC_ABERRATION = 1 << 1;
        /// Vignette
        const VIGNETTE = 1 << 2;
        /// Lens distortion
        const LENS_DISTORTION = 1 << 3;
        /// Anamorphic streaks
        const ANAMORPHIC = 1 << 4;
        /// Bokeh
        const BOKEH = 1 << 5;
        /// Light shafts
        const LIGHT_SHAFTS = 1 << 6;
        /// Glare
        const GLARE = 1 << 7;
        /// Occlusion testing
        const OCCLUSION = 1 << 8;
        /// All
        const ALL = 0x1FF;
    }
}

impl Default for LensEffectsFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Lens Flare
// ============================================================================

/// Lens flare create info
#[derive(Clone, Debug)]
pub struct LensFlareCreateInfo {
    /// Name
    pub name: String,
    /// Elements
    pub elements: Vec<LensFlareElement>,
    /// Occlusion settings
    pub occlusion: OcclusionSettings,
    /// Fade settings
    pub fade: LensFadeSettings,
    /// Global intensity
    pub intensity: f32,
    /// Global scale
    pub scale: f32,
}

impl LensFlareCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            elements: Vec::new(),
            occlusion: OcclusionSettings::default(),
            fade: LensFadeSettings::default(),
            intensity: 1.0,
            scale: 1.0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add element
    pub fn add_element(mut self, element: LensFlareElement) -> Self {
        self.elements.push(element);
        self
    }

    /// With occlusion
    pub fn with_occlusion(mut self, occlusion: OcclusionSettings) -> Self {
        self.occlusion = occlusion;
        self
    }

    /// With fade
    pub fn with_fade(mut self, fade: LensFadeSettings) -> Self {
        self.fade = fade;
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Sun flare preset
    pub fn sun() -> Self {
        Self::new()
            .with_name("SunFlare")
            .add_element(LensFlareElement::glow(1.0, [1.0, 0.9, 0.7, 0.5]))
            .add_element(LensFlareElement::ring(0.5, 0.4, [1.0, 0.8, 0.6, 0.3]))
            .add_element(LensFlareElement::streak(0.3, 0.8))
            .add_element(LensFlareElement::ghost(-0.5, 0.2, [0.8, 0.6, 0.4, 0.2]))
            .add_element(LensFlareElement::ghost(-1.0, 0.3, [0.6, 0.4, 0.3, 0.15]))
            .add_element(LensFlareElement::ghost(0.5, 0.15, [0.5, 0.4, 0.3, 0.1]))
    }

    /// Point light flare
    pub fn point_light() -> Self {
        Self::new()
            .with_name("PointLightFlare")
            .add_element(LensFlareElement::glow(0.5, [1.0, 1.0, 1.0, 0.3]))
            .add_element(LensFlareElement::ring(0.3, 0.2, [1.0, 1.0, 1.0, 0.2]))
    }

    /// Cinematic flare
    pub fn cinematic() -> Self {
        Self::new()
            .with_name("CinematicFlare")
            .add_element(LensFlareElement::glow(1.2, [1.0, 0.95, 0.9, 0.6]))
            .add_element(LensFlareElement::anamorphic(2.0, 0.1, [1.0, 0.9, 0.8, 0.4]))
            .add_element(LensFlareElement::ring(0.6, 0.5, [0.8, 0.7, 0.6, 0.25]))
            .add_element(LensFlareElement::ghost(-0.3, 0.2, [0.7, 0.5, 0.3, 0.15]))
            .add_element(LensFlareElement::ghost(-0.7, 0.25, [0.5, 0.4, 0.3, 0.1]))
            .add_element(LensFlareElement::ghost(0.4, 0.15, [0.4, 0.3, 0.2, 0.08]))
            .add_element(LensFlareElement::ghost(0.8, 0.18, [0.3, 0.25, 0.2, 0.06]))
    }
}

impl Default for LensFlareCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Lens flare element
#[derive(Clone, Copy, Debug)]
pub struct LensFlareElement {
    /// Element type
    pub element_type: LensFlareElementType,
    /// Position along flare axis (-1 to 1, 0 = light position)
    pub position: f32,
    /// Scale
    pub scale: f32,
    /// Color
    pub color: [f32; 4],
    /// Aspect ratio
    pub aspect_ratio: f32,
    /// Rotation
    pub rotation: f32,
    /// Texture index
    pub texture_index: u32,
    /// Flags
    pub flags: LensFlareElementFlags,
}

impl LensFlareElement {
    /// Creates new element
    pub fn new(element_type: LensFlareElementType) -> Self {
        Self {
            element_type,
            position: 0.0,
            scale: 1.0,
            color: [1.0, 1.0, 1.0, 1.0],
            aspect_ratio: 1.0,
            rotation: 0.0,
            texture_index: 0,
            flags: LensFlareElementFlags::empty(),
        }
    }

    /// With position
    pub fn with_position(mut self, position: f32) -> Self {
        self.position = position;
        self
    }

    /// With scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// With color
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// With aspect ratio
    pub fn with_aspect(mut self, aspect: f32) -> Self {
        self.aspect_ratio = aspect;
        self
    }

    /// With rotation
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// With texture
    pub fn with_texture(mut self, index: u32) -> Self {
        self.texture_index = index;
        self
    }

    /// Glow element
    pub fn glow(scale: f32, color: [f32; 4]) -> Self {
        Self::new(LensFlareElementType::Glow)
            .with_scale(scale)
            .with_color(color)
    }

    /// Ring element
    pub fn ring(scale: f32, thickness: f32, color: [f32; 4]) -> Self {
        Self::new(LensFlareElementType::Ring { thickness })
            .with_scale(scale)
            .with_color(color)
    }

    /// Streak element
    pub fn streak(scale: f32, length: f32) -> Self {
        Self::new(LensFlareElementType::Streak { length })
            .with_scale(scale)
            .with_color([1.0, 1.0, 1.0, 0.5])
    }

    /// Ghost element
    pub fn ghost(position: f32, scale: f32, color: [f32; 4]) -> Self {
        Self::new(LensFlareElementType::Ghost)
            .with_position(position)
            .with_scale(scale)
            .with_color(color)
    }

    /// Halo element
    pub fn halo(scale: f32, color: [f32; 4]) -> Self {
        Self::new(LensFlareElementType::Halo)
            .with_scale(scale)
            .with_color(color)
    }

    /// Anamorphic streak
    pub fn anamorphic(width: f32, height: f32, color: [f32; 4]) -> Self {
        Self::new(LensFlareElementType::Anamorphic)
            .with_scale(width)
            .with_aspect(width / height)
            .with_color(color)
    }

    /// Starburst
    pub fn starburst(scale: f32, rays: u32, color: [f32; 4]) -> Self {
        Self::new(LensFlareElementType::Starburst { rays })
            .with_scale(scale)
            .with_color(color)
    }
}

impl Default for LensFlareElement {
    fn default() -> Self {
        Self::new(LensFlareElementType::Glow)
    }
}

/// Lens flare element type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LensFlareElementType {
    /// Soft glow
    Glow,
    /// Ring
    Ring { thickness: f32 },
    /// Streak
    Streak { length: f32 },
    /// Ghost (reflected element)
    Ghost,
    /// Halo
    Halo,
    /// Anamorphic streak
    Anamorphic,
    /// Starburst pattern
    Starburst { rays: u32 },
    /// Custom texture
    Custom,
}

impl Default for LensFlareElementType {
    fn default() -> Self {
        Self::Glow
    }
}

bitflags::bitflags! {
    /// Lens flare element flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct LensFlareElementFlags: u32 {
        /// None
        const NONE = 0;
        /// Rotate with light position
        const ROTATE_WITH_POSITION = 1 << 0;
        /// Scale with light intensity
        const SCALE_WITH_INTENSITY = 1 << 1;
        /// Chromatic shift
        const CHROMATIC = 1 << 2;
        /// Additive blend
        const ADDITIVE = 1 << 3;
    }
}

// ============================================================================
// Occlusion Settings
// ============================================================================

/// Occlusion settings
#[derive(Clone, Copy, Debug)]
pub struct OcclusionSettings {
    /// Occlusion mode
    pub mode: OcclusionMode,
    /// Query radius
    pub query_radius: f32,
    /// Sample count
    pub sample_count: u32,
    /// Fade speed
    pub fade_speed: f32,
}

impl OcclusionSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            mode: OcclusionMode::DepthQuery,
            query_radius: 0.02,
            sample_count: 8,
            fade_speed: 5.0,
        }
    }

    /// No occlusion
    pub const fn none() -> Self {
        Self {
            mode: OcclusionMode::None,
            query_radius: 0.0,
            sample_count: 0,
            fade_speed: 0.0,
        }
    }

    /// Depth query
    pub const fn depth_query() -> Self {
        Self::new()
    }

    /// Raytraced
    pub const fn raytraced() -> Self {
        Self {
            mode: OcclusionMode::Raytraced,
            query_radius: 0.01,
            sample_count: 16,
            fade_speed: 10.0,
        }
    }

    /// With radius
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.query_radius = radius;
        self
    }

    /// With fade speed
    pub const fn with_fade_speed(mut self, speed: f32) -> Self {
        self.fade_speed = speed;
        self
    }
}

impl Default for OcclusionSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Occlusion mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OcclusionMode {
    /// No occlusion
    None = 0,
    /// Depth buffer query
    #[default]
    DepthQuery = 1,
    /// Raytraced occlusion
    Raytraced = 2,
}

// ============================================================================
// Lens Fade Settings
// ============================================================================

/// Lens fade settings
#[derive(Clone, Copy, Debug)]
pub struct LensFadeSettings {
    /// Edge fade start
    pub edge_fade_start: f32,
    /// Edge fade end
    pub edge_fade_end: f32,
    /// Angle fade start
    pub angle_fade_start: f32,
    /// Angle fade end
    pub angle_fade_end: f32,
    /// Distance fade start
    pub distance_fade_start: f32,
    /// Distance fade end
    pub distance_fade_end: f32,
}

impl LensFadeSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            edge_fade_start: 0.7,
            edge_fade_end: 1.0,
            angle_fade_start: 0.0,
            angle_fade_end: 90.0,
            distance_fade_start: 0.0,
            distance_fade_end: 0.0,
        }
    }

    /// No fade
    pub const fn none() -> Self {
        Self {
            edge_fade_start: 1.0,
            edge_fade_end: 1.0,
            angle_fade_start: 0.0,
            angle_fade_end: 180.0,
            distance_fade_start: 0.0,
            distance_fade_end: 0.0,
        }
    }

    /// With edge fade
    pub const fn with_edge_fade(mut self, start: f32, end: f32) -> Self {
        self.edge_fade_start = start;
        self.edge_fade_end = end;
        self
    }

    /// With angle fade
    pub const fn with_angle_fade(mut self, start: f32, end: f32) -> Self {
        self.angle_fade_start = start;
        self.angle_fade_end = end;
        self
    }

    /// With distance fade
    pub const fn with_distance_fade(mut self, start: f32, end: f32) -> Self {
        self.distance_fade_start = start;
        self.distance_fade_end = end;
        self
    }
}

impl Default for LensFadeSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Chromatic Aberration
// ============================================================================

/// Chromatic aberration settings
#[derive(Clone, Copy, Debug)]
pub struct ChromaticAberrationSettings {
    /// Enabled
    pub enabled: bool,
    /// Intensity
    pub intensity: f32,
    /// Red offset
    pub red_offset: [f32; 2],
    /// Green offset (usually 0)
    pub green_offset: [f32; 2],
    /// Blue offset
    pub blue_offset: [f32; 2],
    /// Radial falloff
    pub radial: bool,
}

impl ChromaticAberrationSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: true,
            intensity: 0.5,
            red_offset: [0.005, 0.0],
            green_offset: [0.0, 0.0],
            blue_offset: [-0.005, 0.0],
            radial: true,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            intensity: 0.0,
            red_offset: [0.0, 0.0],
            green_offset: [0.0, 0.0],
            blue_offset: [0.0, 0.0],
            radial: false,
        }
    }

    /// Subtle
    pub const fn subtle() -> Self {
        Self {
            enabled: true,
            intensity: 0.3,
            red_offset: [0.002, 0.0],
            green_offset: [0.0, 0.0],
            blue_offset: [-0.002, 0.0],
            radial: true,
        }
    }

    /// Strong
    pub const fn strong() -> Self {
        Self {
            enabled: true,
            intensity: 1.0,
            red_offset: [0.01, 0.0],
            green_offset: [0.0, 0.0],
            blue_offset: [-0.01, 0.0],
            radial: true,
        }
    }

    /// With intensity
    pub const fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for ChromaticAberrationSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vignette
// ============================================================================

/// Vignette settings
#[derive(Clone, Copy, Debug)]
pub struct VignetteSettings {
    /// Enabled
    pub enabled: bool,
    /// Intensity
    pub intensity: f32,
    /// Smoothness
    pub smoothness: f32,
    /// Roundness
    pub roundness: f32,
    /// Center
    pub center: [f32; 2],
    /// Color
    pub color: [f32; 3],
}

impl VignetteSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: true,
            intensity: 0.3,
            smoothness: 0.5,
            roundness: 1.0,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            intensity: 0.0,
            smoothness: 0.0,
            roundness: 0.0,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// Subtle
    pub const fn subtle() -> Self {
        Self::new()
    }

    /// Strong
    pub const fn strong() -> Self {
        Self {
            enabled: true,
            intensity: 0.6,
            smoothness: 0.4,
            roundness: 1.0,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// Colored
    pub const fn colored(color: [f32; 3]) -> Self {
        Self {
            enabled: true,
            intensity: 0.4,
            smoothness: 0.5,
            roundness: 1.0,
            center: [0.5, 0.5],
            color,
        }
    }

    /// With intensity
    pub const fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With smoothness
    pub const fn with_smoothness(mut self, smoothness: f32) -> Self {
        self.smoothness = smoothness;
        self
    }
}

impl Default for VignetteSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Lens Distortion
// ============================================================================

/// Lens distortion settings
#[derive(Clone, Copy, Debug)]
pub struct LensDistortionSettings {
    /// Enabled
    pub enabled: bool,
    /// Barrel/pincushion distortion
    pub intensity: f32,
    /// X multiplier
    pub x_multiplier: f32,
    /// Y multiplier
    pub y_multiplier: f32,
    /// Center
    pub center: [f32; 2],
    /// Scale
    pub scale: f32,
}

impl LensDistortionSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: true,
            intensity: 0.0,
            x_multiplier: 1.0,
            y_multiplier: 1.0,
            center: [0.5, 0.5],
            scale: 1.0,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            intensity: 0.0,
            x_multiplier: 1.0,
            y_multiplier: 1.0,
            center: [0.5, 0.5],
            scale: 1.0,
        }
    }

    /// Barrel distortion
    pub const fn barrel(intensity: f32) -> Self {
        Self {
            enabled: true,
            intensity: intensity.abs(),
            x_multiplier: 1.0,
            y_multiplier: 1.0,
            center: [0.5, 0.5],
            scale: 1.0,
        }
    }

    /// Pincushion distortion
    pub const fn pincushion(intensity: f32) -> Self {
        Self {
            enabled: true,
            intensity: -intensity.abs(),
            x_multiplier: 1.0,
            y_multiplier: 1.0,
            center: [0.5, 0.5],
            scale: 1.0,
        }
    }

    /// With intensity
    pub const fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for LensDistortionSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU lens flare data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuLensFlareData {
    /// Screen position
    pub screen_pos: [f32; 2],
    /// Intensity
    pub intensity: f32,
    /// Occlusion
    pub occlusion: f32,
    /// Color
    pub color: [f32; 4],
    /// Scale
    pub scale: f32,
    /// Element count
    pub element_count: u32,
    /// Padding
    pub _pad: [f32; 2],
}

/// GPU lens element data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuLensElementData {
    /// Position offset
    pub position: f32,
    /// Scale
    pub scale: f32,
    /// Aspect ratio
    pub aspect_ratio: f32,
    /// Rotation
    pub rotation: f32,
    /// Color
    pub color: [f32; 4],
    /// Element type
    pub element_type: u32,
    /// Texture index
    pub texture_index: u32,
    /// Flags
    pub flags: u32,
    /// Extra param
    pub extra: f32,
}

/// GPU lens effects constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuLensEffectsConstants {
    /// Screen size
    pub screen_size: [f32; 2],
    /// Time
    pub time: f32,
    /// Global intensity
    pub global_intensity: f32,
    /// Chromatic aberration
    pub chromatic_intensity: f32,
    /// Chromatic red offset
    pub chromatic_red: [f32; 2],
    /// Chromatic blue offset
    pub chromatic_blue: [f32; 2],
    /// Vignette params (intensity, smoothness, roundness)
    pub vignette: [f32; 4],
    /// Lens distortion
    pub distortion: [f32; 4],
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU lens effects statistics
#[derive(Clone, Debug, Default)]
pub struct GpuLensEffectsStats {
    /// Active flares
    pub active_flares: u32,
    /// Visible flares
    pub visible_flares: u32,
    /// Total elements
    pub total_elements: u32,
    /// Occlusion queries
    pub occlusion_queries: u32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
}

impl GpuLensEffectsStats {
    /// Elements per flare
    pub fn elements_per_flare(&self) -> f32 {
        if self.visible_flares == 0 {
            0.0
        } else {
            self.total_elements as f32 / self.visible_flares as f32
        }
    }
}
