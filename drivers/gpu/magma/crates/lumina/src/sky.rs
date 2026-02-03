//! Sky and Skybox Types for Lumina
//!
//! This module provides sky rendering infrastructure including
//! procedural skies, skyboxes, and atmospheric effects.

extern crate alloc;

use alloc::string::String;

// ============================================================================
// Sky Handles
// ============================================================================

/// Skybox handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SkyboxHandle(pub u64);

impl SkyboxHandle {
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

impl Default for SkyboxHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Procedural sky handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProceduralSkyHandle(pub u64);

impl ProceduralSkyHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ProceduralSkyHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// HDRI environment handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HdriEnvironmentHandle(pub u64);

impl HdriEnvironmentHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for HdriEnvironmentHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Sky Mode
// ============================================================================

/// Sky mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SkyMode {
    /// Solid color
    SolidColor = 0,
    /// Gradient
    Gradient = 1,
    /// Skybox (cubemap)
    #[default]
    Skybox = 2,
    /// Procedural sky
    Procedural = 3,
    /// HDRI environment
    Hdri = 4,
    /// Physical sky
    PhysicalSky = 5,
}

impl SkyMode {
    /// Requires texture
    pub const fn requires_texture(&self) -> bool {
        matches!(self, Self::Skybox | Self::Hdri)
    }

    /// Is procedural
    pub const fn is_procedural(&self) -> bool {
        matches!(self, Self::Procedural | Self::PhysicalSky)
    }
}

// ============================================================================
// Skybox Configuration
// ============================================================================

/// Skybox create info
#[derive(Clone, Debug)]
pub struct SkyboxCreateInfo {
    /// Name
    pub name: String,
    /// Mode
    pub mode: SkyMode,
    /// Cubemap texture (for Skybox mode)
    pub cubemap: u64,
    /// HDRI texture (for HDRI mode)
    pub hdri: u64,
    /// Intensity
    pub intensity: f32,
    /// Rotation (radians)
    pub rotation: f32,
    /// Blur level (for reflections)
    pub blur: f32,
    /// Tint color
    pub tint: [f32; 4],
}

impl SkyboxCreateInfo {
    /// Creates info
    pub fn new(mode: SkyMode) -> Self {
        Self {
            name: String::new(),
            mode,
            cubemap: 0,
            hdri: 0,
            intensity: 1.0,
            rotation: 0.0,
            blur: 0.0,
            tint: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With cubemap
    pub fn with_cubemap(mut self, cubemap: u64) -> Self {
        self.cubemap = cubemap;
        self.mode = SkyMode::Skybox;
        self
    }

    /// With HDRI
    pub fn with_hdri(mut self, hdri: u64) -> Self {
        self.hdri = hdri;
        self.mode = SkyMode::Hdri;
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With rotation
    pub fn with_rotation(mut self, radians: f32) -> Self {
        self.rotation = radians;
        self
    }

    /// Cubemap skybox
    pub fn cubemap(texture: u64) -> Self {
        Self::new(SkyMode::Skybox)
            .with_cubemap(texture)
    }

    /// HDRI environment
    pub fn hdri(texture: u64) -> Self {
        Self::new(SkyMode::Hdri)
            .with_hdri(texture)
    }

    /// Procedural sky
    pub fn procedural() -> Self {
        Self::new(SkyMode::Procedural)
    }
}

impl Default for SkyboxCreateInfo {
    fn default() -> Self {
        Self::new(SkyMode::Skybox)
    }
}

// ============================================================================
// Solid Color & Gradient
// ============================================================================

/// Sky solid color settings
#[derive(Clone, Copy, Debug)]
pub struct SkySolidColor {
    /// Color (linear RGB)
    pub color: [f32; 3],
    /// Intensity
    pub intensity: f32,
}

impl SkySolidColor {
    /// Creates solid color
    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self {
            color: [r, g, b],
            intensity: 1.0,
        }
    }

    /// Black sky
    pub const fn black() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }

    /// White sky
    pub const fn white() -> Self {
        Self::new(1.0, 1.0, 1.0)
    }

    /// Gray sky
    pub const fn gray(value: f32) -> Self {
        Self::new(value, value, value)
    }
}

impl Default for SkySolidColor {
    fn default() -> Self {
        Self::gray(0.1)
    }
}

/// Sky gradient settings
#[derive(Clone, Copy, Debug)]
pub struct SkyGradient {
    /// Top color (linear RGB)
    pub top_color: [f32; 3],
    /// Horizon color (linear RGB)
    pub horizon_color: [f32; 3],
    /// Bottom color (linear RGB)
    pub bottom_color: [f32; 3],
    /// Top intensity
    pub top_intensity: f32,
    /// Horizon intensity
    pub horizon_intensity: f32,
    /// Bottom intensity
    pub bottom_intensity: f32,
    /// Horizon height (normalized)
    pub horizon_height: f32,
    /// Horizon smoothness
    pub horizon_smoothness: f32,
}

impl SkyGradient {
    /// Creates gradient
    pub const fn new(top: [f32; 3], horizon: [f32; 3], bottom: [f32; 3]) -> Self {
        Self {
            top_color: top,
            horizon_color: horizon,
            bottom_color: bottom,
            top_intensity: 1.0,
            horizon_intensity: 1.0,
            bottom_intensity: 1.0,
            horizon_height: 0.5,
            horizon_smoothness: 0.2,
        }
    }

    /// Day preset
    pub const fn day() -> Self {
        Self::new(
            [0.2, 0.4, 0.9],  // Blue top
            [0.8, 0.9, 1.0],  // Light horizon
            [0.3, 0.35, 0.4], // Dark ground
        )
    }

    /// Sunset preset
    pub const fn sunset() -> Self {
        Self::new(
            [0.2, 0.1, 0.4],  // Purple top
            [1.0, 0.5, 0.2],  // Orange horizon
            [0.1, 0.05, 0.05], // Dark ground
        )
    }

    /// Night preset
    pub const fn night() -> Self {
        Self::new(
            [0.0, 0.0, 0.05], // Dark blue top
            [0.02, 0.02, 0.05], // Slightly lighter horizon
            [0.0, 0.0, 0.0],  // Black ground
        )
    }

    /// Overcast preset
    pub const fn overcast() -> Self {
        Self::new(
            [0.5, 0.5, 0.55],
            [0.6, 0.6, 0.65],
            [0.3, 0.3, 0.35],
        )
    }
}

impl Default for SkyGradient {
    fn default() -> Self {
        Self::day()
    }
}

// ============================================================================
// Procedural Sky
// ============================================================================

/// Procedural sky create info
#[derive(Clone, Debug)]
pub struct ProceduralSkyCreateInfo {
    /// Name
    pub name: String,
    /// Model
    pub model: ProceduralSkyModel,
    /// Sun direction (normalized)
    pub sun_direction: [f32; 3],
    /// Sun color
    pub sun_color: [f32; 3],
    /// Sun intensity
    pub sun_intensity: f32,
    /// Ground color
    pub ground_color: [f32; 3],
    /// Exposure
    pub exposure: f32,
}

impl ProceduralSkyCreateInfo {
    /// Creates info
    pub fn new(model: ProceduralSkyModel) -> Self {
        Self {
            name: String::new(),
            model,
            sun_direction: [0.0, 0.5, 0.866],  // 30 degrees elevation
            sun_color: [1.0, 0.95, 0.9],
            sun_intensity: 1.0,
            ground_color: [0.3, 0.3, 0.3],
            exposure: 1.0,
        }
    }

    /// With sun direction
    pub fn with_sun_direction(mut self, x: f32, y: f32, z: f32) -> Self {
        let len = (x*x + y*y + z*z).sqrt();
        if len > 0.0 {
            self.sun_direction = [x/len, y/len, z/len];
        }
        self
    }

    /// With sun from time of day (0-24 hours)
    pub fn with_time_of_day(mut self, hour: f32) -> Self {
        let angle = (hour - 6.0) / 12.0 * core::f32::consts::PI;
        let y = angle.sin();
        let z = angle.cos();
        self.sun_direction = [0.0, y, z];
        self
    }

    /// Default day
    pub fn day() -> Self {
        Self::new(ProceduralSkyModel::Preetham)
            .with_time_of_day(12.0)
    }

    /// Sunrise
    pub fn sunrise() -> Self {
        Self::new(ProceduralSkyModel::HosekWilkie)
            .with_time_of_day(6.0)
    }

    /// Sunset
    pub fn sunset() -> Self {
        Self::new(ProceduralSkyModel::HosekWilkie)
            .with_time_of_day(18.0)
    }
}

impl Default for ProceduralSkyCreateInfo {
    fn default() -> Self {
        Self::day()
    }
}

/// Procedural sky model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProceduralSkyModel {
    /// Preetham model (simple, fast)
    #[default]
    Preetham = 0,
    /// Hosek-Wilkie model (accurate)
    HosekWilkie = 1,
    /// Bruneton model (physical atmosphere)
    Bruneton = 2,
    /// Simple gradient-based
    Gradient = 3,
    /// Nishita (very accurate scattering)
    Nishita = 4,
}

impl ProceduralSkyModel {
    /// Complexity level
    pub const fn complexity(&self) -> u32 {
        match self {
            Self::Gradient => 1,
            Self::Preetham => 2,
            Self::HosekWilkie => 3,
            Self::Bruneton => 4,
            Self::Nishita => 5,
        }
    }

    /// Supports multiple scattering
    pub const fn supports_multiple_scattering(&self) -> bool {
        matches!(self, Self::Bruneton | Self::Nishita)
    }
}

// ============================================================================
// Physical Sky
// ============================================================================

/// Physical sky settings
#[derive(Clone, Copy, Debug)]
pub struct PhysicalSkySettings {
    /// Planet radius (km)
    pub planet_radius: f32,
    /// Atmosphere height (km)
    pub atmosphere_height: f32,
    /// Rayleigh scattering coefficient
    pub rayleigh_coefficient: [f32; 3],
    /// Rayleigh scale height (km)
    pub rayleigh_scale_height: f32,
    /// Mie scattering coefficient
    pub mie_coefficient: f32,
    /// Mie scale height (km)
    pub mie_scale_height: f32,
    /// Mie anisotropy (g)
    pub mie_anisotropy: f32,
    /// Ozone absorption
    pub ozone_absorption: [f32; 3],
    /// Ground albedo
    pub ground_albedo: [f32; 3],
    /// Sun angular diameter (degrees)
    pub sun_angular_diameter: f32,
}

impl PhysicalSkySettings {
    /// Earth-like atmosphere
    pub const fn earth() -> Self {
        Self {
            planet_radius: 6371.0,
            atmosphere_height: 100.0,
            rayleigh_coefficient: [5.802e-6, 13.558e-6, 33.1e-6],
            rayleigh_scale_height: 8.0,
            mie_coefficient: 3.996e-6,
            mie_scale_height: 1.2,
            mie_anisotropy: 0.8,
            ozone_absorption: [0.65e-6, 1.881e-6, 0.085e-6],
            ground_albedo: [0.3, 0.3, 0.3],
            sun_angular_diameter: 0.53,
        }
    }

    /// Mars-like atmosphere
    pub const fn mars() -> Self {
        Self {
            planet_radius: 3390.0,
            atmosphere_height: 200.0,
            rayleigh_coefficient: [19.918e-6, 13.57e-6, 5.75e-6],  // Reddish
            rayleigh_scale_height: 11.0,
            mie_coefficient: 21.0e-6,
            mie_scale_height: 11.0,
            mie_anisotropy: 0.76,
            ozone_absorption: [0.0, 0.0, 0.0],
            ground_albedo: [0.4, 0.2, 0.1],
            sun_angular_diameter: 0.35,
        }
    }

    /// Alien atmosphere
    pub const fn alien() -> Self {
        Self {
            rayleigh_coefficient: [33.1e-6, 13.558e-6, 5.802e-6],  // Inverted (green/purple)
            ..Self::earth()
        }
    }
}

impl Default for PhysicalSkySettings {
    fn default() -> Self {
        Self::earth()
    }
}

// ============================================================================
// Sun & Moon
// ============================================================================

/// Celestial body settings
#[derive(Clone, Copy, Debug)]
pub struct CelestialBodySettings {
    /// Direction (normalized)
    pub direction: [f32; 3],
    /// Angular diameter (degrees)
    pub angular_diameter: f32,
    /// Color (linear RGB)
    pub color: [f32; 3],
    /// Intensity
    pub intensity: f32,
    /// Corona size (for sun)
    pub corona_size: f32,
    /// Corona intensity
    pub corona_intensity: f32,
}

impl CelestialBodySettings {
    /// Sun default
    pub const fn sun() -> Self {
        Self {
            direction: [0.0, 0.5, 0.866],
            angular_diameter: 0.53,
            color: [1.0, 0.95, 0.85],
            intensity: 100000.0,  // Lux on clear day
            corona_size: 1.5,
            corona_intensity: 0.1,
        }
    }

    /// Moon default
    pub const fn moon() -> Self {
        Self {
            direction: [0.0, -0.5, 0.866],
            angular_diameter: 0.52,
            color: [0.9, 0.92, 1.0],
            intensity: 0.3,  // Lux on full moon
            corona_size: 1.2,
            corona_intensity: 0.02,
        }
    }

    /// With direction
    pub const fn with_direction(mut self, x: f32, y: f32, z: f32) -> Self {
        self.direction = [x, y, z];
        self
    }
}

impl Default for CelestialBodySettings {
    fn default() -> Self {
        Self::sun()
    }
}

/// Stars settings
#[derive(Clone, Copy, Debug)]
pub struct StarsSettings {
    /// Enable stars
    pub enabled: bool,
    /// Density
    pub density: f32,
    /// Brightness
    pub brightness: f32,
    /// Twinkle intensity
    pub twinkle: f32,
    /// Twinkle speed
    pub twinkle_speed: f32,
    /// Star size
    pub size: f32,
}

impl StarsSettings {
    /// Default stars
    pub const fn default_stars() -> Self {
        Self {
            enabled: true,
            density: 1.0,
            brightness: 1.0,
            twinkle: 0.5,
            twinkle_speed: 1.0,
            size: 1.0,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default_stars()
        }
    }

    /// Dense stars
    pub const fn dense() -> Self {
        Self {
            density: 3.0,
            ..Self::default_stars()
        }
    }
}

impl Default for StarsSettings {
    fn default() -> Self {
        Self::default_stars()
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// Sky GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SkyGpuParams {
    /// Sky mode
    pub mode: u32,
    /// Intensity
    pub intensity: f32,
    /// Rotation
    pub rotation: f32,
    /// Blur
    pub blur: f32,
    /// Tint color
    pub tint: [f32; 4],
    /// Sun direction
    pub sun_direction: [f32; 4],
    /// Sun color
    pub sun_color: [f32; 4],
    /// Ground color
    pub ground_color: [f32; 4],
    /// Top color (gradient)
    pub top_color: [f32; 4],
    /// Horizon color (gradient)
    pub horizon_color: [f32; 4],
    /// Bottom color (gradient)
    pub bottom_color: [f32; 4],
    /// Exposure
    pub exposure: f32,
    /// Time
    pub time: f32,
    /// Padding
    pub _padding: [f32; 2],
}

impl SkyGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// Physical sky GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PhysicalSkyGpuParams {
    /// Planet radius
    pub planet_radius: f32,
    /// Atmosphere radius
    pub atmosphere_radius: f32,
    /// Rayleigh scale height
    pub rayleigh_scale_height: f32,
    /// Mie scale height
    pub mie_scale_height: f32,
    /// Rayleigh coefficient
    pub rayleigh_coefficient: [f32; 4],
    /// Mie coefficient
    pub mie_coefficient: f32,
    /// Mie anisotropy
    pub mie_anisotropy: f32,
    /// Ozone density
    pub ozone_density: f32,
    /// Sun intensity
    pub sun_intensity: f32,
    /// Sun direction
    pub sun_direction: [f32; 4],
    /// Sun color
    pub sun_color: [f32; 4],
    /// Ground albedo
    pub ground_albedo: [f32; 4],
    /// Camera height
    pub camera_height: f32,
    /// Multiple scattering
    pub multiple_scattering: u32,
    /// Padding
    pub _padding: [f32; 2],
}

impl PhysicalSkyGpuParams {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// ============================================================================
// Statistics
// ============================================================================

/// Sky rendering statistics
#[derive(Clone, Debug, Default)]
pub struct SkyStats {
    /// Render time (microseconds)
    pub render_time_us: u64,
    /// LUT generation time (microseconds)
    pub lut_time_us: u64,
    /// Transmittance samples
    pub transmittance_samples: u32,
    /// Scattering samples
    pub scattering_samples: u32,
    /// Star count rendered
    pub stars_rendered: u32,
}
