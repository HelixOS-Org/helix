//! Lens Effects Types for Lumina
//!
//! This module provides lens effects infrastructure including
//! lens flares, chromatic aberration, and vignette effects.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Lens Effect Handles
// ============================================================================

/// Lens flare handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LensFlareHandle(pub u64);

impl LensFlareHandle {
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

impl Default for LensFlareHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Lens distortion handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LensDistortionHandle(pub u64);

impl LensDistortionHandle {
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

impl Default for LensDistortionHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Lens Flare Configuration
// ============================================================================

/// Lens flare create info
#[derive(Clone, Debug)]
pub struct LensFlareCreateInfo {
    /// Name
    pub name: String,
    /// Flare elements
    pub elements: Vec<FlareElement>,
    /// Global intensity
    pub intensity: f32,
    /// Ghost intensity
    pub ghost_intensity: f32,
    /// Halo intensity
    pub halo_intensity: f32,
    /// Occlusion
    pub occlusion: FlareOcclusion,
}

impl LensFlareCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            elements: Vec::new(),
            intensity: 1.0,
            ghost_intensity: 0.5,
            halo_intensity: 0.3,
            occlusion: FlareOcclusion::default(),
        }
    }

    /// Cinematic flare
    pub fn cinematic() -> Self {
        Self {
            elements: vec![
                FlareElement::ghost(0.2, 1.0, [1.0, 0.8, 0.6]),
                FlareElement::ghost(-0.4, 0.5, [0.8, 0.9, 1.0]),
                FlareElement::ghost(0.6, 0.3, [0.6, 0.7, 1.0]),
                FlareElement::halo(1.5, 0.3),
                FlareElement::starburst(0.8),
            ],
            intensity: 0.8,
            ghost_intensity: 0.4,
            halo_intensity: 0.25,
            ..Self::new()
        }
    }

    /// Sci-fi flare (anamorphic streak)
    pub fn anamorphic() -> Self {
        Self {
            elements: vec![
                FlareElement::anamorphic_streak(1.0, 0.8),
                FlareElement::ghost(0.3, 0.4, [0.5, 0.7, 1.0]),
                FlareElement::ghost(-0.5, 0.3, [1.0, 0.5, 0.3]),
            ],
            intensity: 1.0,
            ghost_intensity: 0.3,
            halo_intensity: 0.1,
            ..Self::new()
        }
    }

    /// Natural/photography flare
    pub fn natural() -> Self {
        Self {
            elements: vec![
                FlareElement::ghost(0.15, 0.6, [0.9, 0.85, 0.8]),
                FlareElement::ghost(-0.25, 0.4, [0.95, 0.9, 0.85]),
                FlareElement::halo(1.0, 0.2),
            ],
            intensity: 0.5,
            ghost_intensity: 0.3,
            halo_intensity: 0.2,
            ..Self::new()
        }
    }

    /// Add element
    pub fn with_element(mut self, element: FlareElement) -> Self {
        self.elements.push(element);
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for LensFlareCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Flare element
#[derive(Clone, Copy, Debug)]
pub struct FlareElement {
    /// Element type
    pub element_type: FlareElementType,
    /// Position on flare line (-1 to 1)
    pub position: f32,
    /// Size
    pub size: f32,
    /// Color tint
    pub color: [f32; 3],
    /// Intensity
    pub intensity: f32,
    /// Rotation (radians)
    pub rotation: f32,
    /// Aspect ratio
    pub aspect: f32,
}

impl FlareElement {
    /// Ghost element
    pub const fn ghost(position: f32, size: f32, color: [f32; 3]) -> Self {
        Self {
            element_type: FlareElementType::Ghost,
            position,
            size,
            color,
            intensity: 1.0,
            rotation: 0.0,
            aspect: 1.0,
        }
    }

    /// Halo element
    pub const fn halo(size: f32, intensity: f32) -> Self {
        Self {
            element_type: FlareElementType::Halo,
            position: 0.0,
            size,
            color: [1.0, 0.95, 0.9],
            intensity,
            rotation: 0.0,
            aspect: 1.0,
        }
    }

    /// Starburst element
    pub const fn starburst(intensity: f32) -> Self {
        Self {
            element_type: FlareElementType::Starburst,
            position: 0.0,
            size: 1.0,
            color: [1.0, 1.0, 1.0],
            intensity,
            rotation: 0.0,
            aspect: 1.0,
        }
    }

    /// Anamorphic streak
    pub const fn anamorphic_streak(size: f32, intensity: f32) -> Self {
        Self {
            element_type: FlareElementType::AnamorphicStreak,
            position: 0.0,
            size,
            color: [0.5, 0.7, 1.0],
            intensity,
            rotation: 0.0,
            aspect: 100.0,  // Very wide
        }
    }

    /// With rotation
    pub const fn with_rotation(mut self, radians: f32) -> Self {
        self.rotation = radians;
        self
    }

    /// With aspect
    pub const fn with_aspect(mut self, aspect: f32) -> Self {
        self.aspect = aspect;
        self
    }
}

impl Default for FlareElement {
    fn default() -> Self {
        Self::ghost(0.0, 1.0, [1.0, 1.0, 1.0])
    }
}

/// Flare element type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FlareElementType {
    /// Ghost (circular)
    #[default]
    Ghost = 0,
    /// Halo
    Halo = 1,
    /// Starburst/diffraction spikes
    Starburst = 2,
    /// Anamorphic streak
    AnamorphicStreak = 3,
    /// Ring
    Ring = 4,
    /// Custom texture
    CustomTexture = 5,
}

/// Flare occlusion settings
#[derive(Clone, Copy, Debug)]
pub struct FlareOcclusion {
    /// Enable occlusion
    pub enabled: bool,
    /// Occlusion radius (screen space)
    pub radius: f32,
    /// Fade speed
    pub fade_speed: f32,
}

impl FlareOcclusion {
    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            radius: 0.1,
            fade_speed: 5.0,
        }
    }

    /// Default occlusion
    pub const fn default_occlusion() -> Self {
        Self {
            enabled: true,
            radius: 0.05,
            fade_speed: 5.0,
        }
    }
}

impl Default for FlareOcclusion {
    fn default() -> Self {
        Self::default_occlusion()
    }
}

// ============================================================================
// Chromatic Aberration
// ============================================================================

/// Chromatic aberration settings
#[derive(Clone, Copy, Debug)]
pub struct ChromaticAberrationSettings {
    /// Enable
    pub enabled: bool,
    /// Intensity
    pub intensity: f32,
    /// Mode
    pub mode: ChromaticMode,
    /// Falloff start (from center, 0-1)
    pub falloff_start: f32,
    /// Falloff end
    pub falloff_end: f32,
    /// Red/cyan shift
    pub red_shift: f32,
    /// Blue/yellow shift
    pub blue_shift: f32,
}

impl ChromaticAberrationSettings {
    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            intensity: 0.0,
            mode: ChromaticMode::Radial,
            falloff_start: 0.5,
            falloff_end: 1.0,
            red_shift: 1.0,
            blue_shift: 1.0,
        }
    }

    /// Subtle aberration
    pub const fn subtle() -> Self {
        Self {
            enabled: true,
            intensity: 0.15,
            mode: ChromaticMode::Radial,
            falloff_start: 0.6,
            falloff_end: 1.0,
            red_shift: 1.0,
            blue_shift: 1.0,
        }
    }

    /// Heavy aberration
    pub const fn heavy() -> Self {
        Self {
            enabled: true,
            intensity: 0.5,
            mode: ChromaticMode::Radial,
            falloff_start: 0.3,
            falloff_end: 1.0,
            red_shift: 1.0,
            blue_shift: 1.0,
        }
    }

    /// Anamorphic style
    pub const fn anamorphic() -> Self {
        Self {
            enabled: true,
            intensity: 0.3,
            mode: ChromaticMode::Horizontal,
            falloff_start: 0.4,
            falloff_end: 1.0,
            red_shift: 1.0,
            blue_shift: 0.8,
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
        Self::disabled()
    }
}

/// Chromatic aberration mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ChromaticMode {
    /// Radial (from center)
    #[default]
    Radial = 0,
    /// Horizontal only
    Horizontal = 1,
    /// Vertical only
    Vertical = 2,
    /// Uniform (screen-space)
    Uniform = 3,
}

// ============================================================================
// Vignette
// ============================================================================

/// Vignette settings
#[derive(Clone, Copy, Debug)]
pub struct VignetteSettings {
    /// Enable
    pub enabled: bool,
    /// Intensity
    pub intensity: f32,
    /// Smoothness
    pub smoothness: f32,
    /// Roundness
    pub roundness: f32,
    /// Center offset
    pub center: [f32; 2],
    /// Color (usually black)
    pub color: [f32; 3],
}

impl VignetteSettings {
    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            intensity: 0.0,
            smoothness: 0.4,
            roundness: 1.0,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// Subtle vignette
    pub const fn subtle() -> Self {
        Self {
            enabled: true,
            intensity: 0.25,
            smoothness: 0.5,
            roundness: 1.0,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// Heavy vignette
    pub const fn heavy() -> Self {
        Self {
            enabled: true,
            intensity: 0.5,
            smoothness: 0.3,
            roundness: 1.0,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// Cinematic vignette
    pub const fn cinematic() -> Self {
        Self {
            enabled: true,
            intensity: 0.35,
            smoothness: 0.4,
            roundness: 0.8,
            center: [0.5, 0.5],
            color: [0.0, 0.0, 0.0],
        }
    }

    /// With intensity
    pub const fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With center
    pub const fn with_center(mut self, x: f32, y: f32) -> Self {
        self.center = [x, y];
        self
    }
}

impl Default for VignetteSettings {
    fn default() -> Self {
        Self::disabled()
    }
}

// ============================================================================
// Lens Distortion
// ============================================================================

/// Lens distortion settings
#[derive(Clone, Copy, Debug)]
pub struct LensDistortionSettings {
    /// Enable
    pub enabled: bool,
    /// Intensity (-1 to 1)
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
            intensity,
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
            intensity: -intensity,
            x_multiplier: 1.0,
            y_multiplier: 1.0,
            center: [0.5, 0.5],
            scale: 1.0,
        }
    }

    /// Anamorphic distortion
    pub const fn anamorphic() -> Self {
        Self {
            enabled: true,
            intensity: 0.2,
            x_multiplier: 1.5,
            y_multiplier: 1.0,
            center: [0.5, 0.5],
            scale: 0.95,
        }
    }
}

impl Default for LensDistortionSettings {
    fn default() -> Self {
        Self::disabled()
    }
}

// ============================================================================
// Combined Lens Effects
// ============================================================================

/// Combined lens effects settings
#[derive(Clone, Debug)]
pub struct LensEffectsSettings {
    /// Lens flare
    pub lens_flare: Option<LensFlareCreateInfo>,
    /// Chromatic aberration
    pub chromatic_aberration: ChromaticAberrationSettings,
    /// Vignette
    pub vignette: VignetteSettings,
    /// Lens distortion
    pub distortion: LensDistortionSettings,
    /// Lens dirt
    pub lens_dirt: LensDirtSettings,
}

impl LensEffectsSettings {
    /// Creates default settings
    pub fn new() -> Self {
        Self {
            lens_flare: None,
            chromatic_aberration: ChromaticAberrationSettings::disabled(),
            vignette: VignetteSettings::disabled(),
            distortion: LensDistortionSettings::disabled(),
            lens_dirt: LensDirtSettings::disabled(),
        }
    }

    /// Cinematic settings
    pub fn cinematic() -> Self {
        Self {
            lens_flare: Some(LensFlareCreateInfo::cinematic()),
            chromatic_aberration: ChromaticAberrationSettings::subtle(),
            vignette: VignetteSettings::cinematic(),
            distortion: LensDistortionSettings::disabled(),
            lens_dirt: LensDirtSettings::subtle(),
        }
    }

    /// Clean (no effects)
    pub fn clean() -> Self {
        Self::new()
    }
}

impl Default for LensEffectsSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Lens Dirt
// ============================================================================

/// Lens dirt settings
#[derive(Clone, Copy, Debug)]
pub struct LensDirtSettings {
    /// Enable
    pub enabled: bool,
    /// Intensity
    pub intensity: f32,
    /// Threshold
    pub threshold: f32,
}

impl LensDirtSettings {
    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            intensity: 0.0,
            threshold: 0.8,
        }
    }

    /// Subtle
    pub const fn subtle() -> Self {
        Self {
            enabled: true,
            intensity: 0.2,
            threshold: 0.7,
        }
    }

    /// Heavy
    pub const fn heavy() -> Self {
        Self {
            enabled: true,
            intensity: 0.5,
            threshold: 0.5,
        }
    }
}

impl Default for LensDirtSettings {
    fn default() -> Self {
        Self::disabled()
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// Lens effects GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct LensEffectsGpuParams {
    /// Screen dimensions
    pub screen_size: [f32; 2],
    /// Chromatic aberration intensity
    pub ca_intensity: f32,
    /// Chromatic aberration mode
    pub ca_mode: u32,
    /// CA falloff start
    pub ca_falloff_start: f32,
    /// CA falloff end
    pub ca_falloff_end: f32,
    /// Red shift
    pub ca_red_shift: f32,
    /// Blue shift
    pub ca_blue_shift: f32,
    /// Vignette intensity
    pub vignette_intensity: f32,
    /// Vignette smoothness
    pub vignette_smoothness: f32,
    /// Vignette roundness
    pub vignette_roundness: f32,
    /// Padding
    pub _padding0: f32,
    /// Vignette center
    pub vignette_center: [f32; 2],
    /// Vignette color
    pub vignette_color: [f32; 2],
    /// Distortion intensity
    pub distortion_intensity: f32,
    /// Distortion X mul
    pub distortion_x_mul: f32,
    /// Distortion Y mul
    pub distortion_y_mul: f32,
    /// Distortion scale
    pub distortion_scale: f32,
    /// Distortion center
    pub distortion_center: [f32; 2],
    /// Flags
    pub flags: u32,
    /// Padding
    pub _padding1: u32,
}

impl LensEffectsGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Chromatic aberration flag
    pub const FLAG_CHROMATIC_ABERRATION: u32 = 1 << 0;
    /// Vignette flag
    pub const FLAG_VIGNETTE: u32 = 1 << 1;
    /// Distortion flag
    pub const FLAG_DISTORTION: u32 = 1 << 2;
    /// Lens dirt flag
    pub const FLAG_LENS_DIRT: u32 = 1 << 3;
}

/// Lens flare GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FlareElementGpuData {
    /// Position
    pub position: f32,
    /// Size
    pub size: f32,
    /// Intensity
    pub intensity: f32,
    /// Element type
    pub element_type: u32,
    /// Color
    pub color: [f32; 4],
    /// Rotation
    pub rotation: f32,
    /// Aspect
    pub aspect: f32,
    /// Padding
    pub _padding: [f32; 2],
}

impl FlareElementGpuData {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// ============================================================================
// Statistics
// ============================================================================

/// Lens effects statistics
#[derive(Clone, Debug, Default)]
pub struct LensEffectsStats {
    /// Lens flare pass time (microseconds)
    pub flare_time_us: u64,
    /// Other effects time (microseconds)
    pub effects_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}
