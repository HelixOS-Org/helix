//! Atmosphere Scattering Types for Lumina
//!
//! This module provides advanced atmospheric scattering
//! simulation including Rayleigh, Mie, and multiple scattering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Atmosphere Handles
// ============================================================================

/// Atmosphere handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AtmosphereScatteringHandle(pub u64);

impl AtmosphereScatteringHandle {
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

impl Default for AtmosphereScatteringHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Transmittance LUT handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TransmittanceLutHandle(pub u64);

impl TransmittanceLutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for TransmittanceLutHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Scattering LUT handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ScatteringLutHandle(pub u64);

impl ScatteringLutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ScatteringLutHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sky view LUT handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SkyViewLutHandle(pub u64);

impl SkyViewLutHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SkyViewLutHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Atmosphere Creation
// ============================================================================

/// Atmosphere create info
#[derive(Clone, Debug)]
pub struct AtmosphereCreateInfo {
    /// Name
    pub name: String,
    /// Atmosphere model
    pub model: AtmosphereModel,
    /// Planet parameters
    pub planet: PlanetParams,
    /// Rayleigh scattering
    pub rayleigh: RayleighParams,
    /// Mie scattering
    pub mie: MieParams,
    /// Ozone absorption
    pub ozone: OzoneParams,
    /// Quality settings
    pub quality: AtmosphereQuality,
    /// Features
    pub features: AtmosphereFeatures,
}

impl AtmosphereCreateInfo {
    /// Creates new info
    pub fn new(model: AtmosphereModel) -> Self {
        Self {
            name: String::new(),
            model,
            planet: PlanetParams::earth(),
            rayleigh: RayleighParams::earth(),
            mie: MieParams::earth(),
            ozone: OzoneParams::earth(),
            quality: AtmosphereQuality::default(),
            features: AtmosphereFeatures::SINGLE_SCATTERING,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With planet
    pub fn with_planet(mut self, planet: PlanetParams) -> Self {
        self.planet = planet;
        self
    }

    /// With rayleigh
    pub fn with_rayleigh(mut self, rayleigh: RayleighParams) -> Self {
        self.rayleigh = rayleigh;
        self
    }

    /// With mie
    pub fn with_mie(mut self, mie: MieParams) -> Self {
        self.mie = mie;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: AtmosphereQuality) -> Self {
        self.quality = quality;
        self
    }

    /// With features
    pub fn with_features(mut self, features: AtmosphereFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Earth atmosphere preset
    pub fn earth() -> Self {
        Self::new(AtmosphereModel::Bruneton)
            .with_planet(PlanetParams::earth())
            .with_rayleigh(RayleighParams::earth())
            .with_mie(MieParams::earth())
            .with_features(AtmosphereFeatures::SINGLE_SCATTERING | AtmosphereFeatures::MULTIPLE_SCATTERING)
    }

    /// Mars atmosphere preset
    pub fn mars() -> Self {
        Self::new(AtmosphereModel::Bruneton)
            .with_planet(PlanetParams::mars())
            .with_rayleigh(RayleighParams::mars())
            .with_mie(MieParams::mars())
    }

    /// Alien world preset (purple sky)
    pub fn alien_purple() -> Self {
        Self::new(AtmosphereModel::Bruneton)
            .with_planet(PlanetParams {
                radius: 6360000.0,
                atmosphere_height: 100000.0,
            })
            .with_rayleigh(RayleighParams {
                scattering: [0.00002, 0.000055, 0.000033],  // Purple tint
                scale_height: 8500.0,
            })
    }

    /// Performance preset
    pub fn performance() -> Self {
        Self::earth()
            .with_quality(AtmosphereQuality::low())
            .with_features(AtmosphereFeatures::SINGLE_SCATTERING)
    }

    /// Quality preset
    pub fn quality() -> Self {
        Self::earth()
            .with_quality(AtmosphereQuality::high())
            .with_features(AtmosphereFeatures::all())
    }
}

impl Default for AtmosphereCreateInfo {
    fn default() -> Self {
        Self::earth()
    }
}

/// Atmosphere model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AtmosphereModel {
    /// Bruneton model (physically-based)
    #[default]
    Bruneton = 0,
    /// Preetham model (analytic)
    Preetham = 1,
    /// Hosek-Wilkie model
    HosekWilkie = 2,
    /// O'Neal model
    ONeal = 3,
    /// Hillaire model (UE4/5 style)
    Hillaire = 4,
}

impl AtmosphereModel {
    /// Display name
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Bruneton => "Bruneton",
            Self::Preetham => "Preetham",
            Self::HosekWilkie => "Hosek-Wilkie",
            Self::ONeal => "O'Neal",
            Self::Hillaire => "Hillaire",
        }
    }

    /// Is LUT-based
    pub const fn is_lut_based(&self) -> bool {
        matches!(self, Self::Bruneton | Self::Hillaire)
    }
}

bitflags::bitflags! {
    /// Atmosphere features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct AtmosphereFeatures: u32 {
        /// None
        const NONE = 0;
        /// Single scattering
        const SINGLE_SCATTERING = 1 << 0;
        /// Multiple scattering
        const MULTIPLE_SCATTERING = 1 << 1;
        /// Ozone absorption
        const OZONE = 1 << 2;
        /// Aerial perspective
        const AERIAL_PERSPECTIVE = 1 << 3;
        /// Shadow from ground
        const GROUND_SHADOW = 1 << 4;
        /// Sun disk
        const SUN_DISK = 1 << 5;
        /// Moon
        const MOON = 1 << 6;
        /// Stars
        const STARS = 1 << 7;
        /// Clouds integration
        const CLOUDS = 1 << 8;
    }
}

// ============================================================================
// Planet Parameters
// ============================================================================

/// Planet parameters
#[derive(Clone, Copy, Debug)]
pub struct PlanetParams {
    /// Planet radius (meters)
    pub radius: f32,
    /// Atmosphere height (meters)
    pub atmosphere_height: f32,
}

impl PlanetParams {
    /// Earth parameters
    pub const fn earth() -> Self {
        Self {
            radius: 6360000.0,          // 6360 km
            atmosphere_height: 100000.0, // 100 km
        }
    }

    /// Mars parameters
    pub const fn mars() -> Self {
        Self {
            radius: 3389500.0,          // 3389.5 km
            atmosphere_height: 200000.0, // Thin but extends higher
        }
    }

    /// Top of atmosphere
    pub fn top_of_atmosphere(&self) -> f32 {
        self.radius + self.atmosphere_height
    }
}

impl Default for PlanetParams {
    fn default() -> Self {
        Self::earth()
    }
}

// ============================================================================
// Scattering Parameters
// ============================================================================

/// Rayleigh scattering parameters
#[derive(Clone, Copy, Debug)]
pub struct RayleighParams {
    /// Scattering coefficients (1/m) for RGB
    pub scattering: [f32; 3],
    /// Scale height (meters)
    pub scale_height: f32,
}

impl RayleighParams {
    /// Earth Rayleigh
    pub const fn earth() -> Self {
        Self {
            scattering: [0.0000055, 0.000013, 0.0000224],  // Blue dominant
            scale_height: 8500.0,
        }
    }

    /// Mars Rayleigh (reddish)
    pub const fn mars() -> Self {
        Self {
            scattering: [0.00002, 0.000012, 0.000006],  // Red dominant
            scale_height: 11000.0,
        }
    }
}

impl Default for RayleighParams {
    fn default() -> Self {
        Self::earth()
    }
}

/// Mie scattering parameters
#[derive(Clone, Copy, Debug)]
pub struct MieParams {
    /// Scattering coefficient (1/m)
    pub scattering: f32,
    /// Absorption coefficient (1/m)
    pub absorption: f32,
    /// Scale height (meters)
    pub scale_height: f32,
    /// Phase function asymmetry (g)
    pub asymmetry: f32,
}

impl MieParams {
    /// Earth Mie
    pub const fn earth() -> Self {
        Self {
            scattering: 0.0000035,
            absorption: 0.00000044,
            scale_height: 1200.0,
            asymmetry: 0.8,  // Forward scattering
        }
    }

    /// Mars Mie (dusty)
    pub const fn mars() -> Self {
        Self {
            scattering: 0.00002,
            absorption: 0.000004,
            scale_height: 11000.0,
            asymmetry: 0.76,
        }
    }

    /// Hazy atmosphere
    pub const fn hazy() -> Self {
        Self {
            scattering: 0.000021,
            absorption: 0.00000044,
            scale_height: 1200.0,
            asymmetry: 0.76,
        }
    }

    /// Clear atmosphere
    pub const fn clear() -> Self {
        Self {
            scattering: 0.000002,
            absorption: 0.0000002,
            scale_height: 1200.0,
            asymmetry: 0.85,
        }
    }
}

impl Default for MieParams {
    fn default() -> Self {
        Self::earth()
    }
}

/// Ozone absorption parameters
#[derive(Clone, Copy, Debug)]
pub struct OzoneParams {
    /// Absorption coefficients (1/m) for RGB
    pub absorption: [f32; 3],
    /// Layer altitude (meters)
    pub altitude: f32,
    /// Layer thickness (meters)
    pub thickness: f32,
}

impl OzoneParams {
    /// Earth ozone
    pub const fn earth() -> Self {
        Self {
            absorption: [0.00000065, 0.0000019, 0.000000085],
            altitude: 25000.0,   // 25 km
            thickness: 30000.0,  // 30 km layer
        }
    }
}

impl Default for OzoneParams {
    fn default() -> Self {
        Self::earth()
    }
}

// ============================================================================
// Quality Settings
// ============================================================================

/// Atmosphere quality settings
#[derive(Clone, Debug)]
pub struct AtmosphereQuality {
    /// Transmittance LUT size
    pub transmittance_lut_size: [u32; 2],
    /// Multi-scattering LUT size
    pub multi_scatter_lut_size: [u32; 2],
    /// Sky view LUT size
    pub sky_view_lut_size: [u32; 2],
    /// Aerial perspective volume size
    pub aerial_volume_size: [u32; 3],
    /// View ray samples
    pub view_samples: u32,
    /// Light ray samples
    pub light_samples: u32,
    /// Multi-scattering iterations
    pub multi_scatter_iterations: u32,
}

impl AtmosphereQuality {
    /// Low quality
    pub fn low() -> Self {
        Self {
            transmittance_lut_size: [128, 32],
            multi_scatter_lut_size: [16, 16],
            sky_view_lut_size: [128, 64],
            aerial_volume_size: [16, 16, 16],
            view_samples: 16,
            light_samples: 4,
            multi_scatter_iterations: 2,
        }
    }

    /// Medium quality
    pub fn medium() -> Self {
        Self {
            transmittance_lut_size: [256, 64],
            multi_scatter_lut_size: [32, 32],
            sky_view_lut_size: [192, 104],
            aerial_volume_size: [32, 32, 32],
            view_samples: 32,
            light_samples: 8,
            multi_scatter_iterations: 4,
        }
    }

    /// High quality
    pub fn high() -> Self {
        Self {
            transmittance_lut_size: [256, 64],
            multi_scatter_lut_size: [32, 32],
            sky_view_lut_size: [256, 128],
            aerial_volume_size: [32, 32, 32],
            view_samples: 40,
            light_samples: 12,
            multi_scatter_iterations: 8,
        }
    }

    /// Ultra quality
    pub fn ultra() -> Self {
        Self {
            transmittance_lut_size: [512, 128],
            multi_scatter_lut_size: [64, 64],
            sky_view_lut_size: [384, 192],
            aerial_volume_size: [64, 64, 64],
            view_samples: 64,
            light_samples: 16,
            multi_scatter_iterations: 16,
        }
    }
}

impl Default for AtmosphereQuality {
    fn default() -> Self {
        Self::medium()
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU atmosphere params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuAtmosphereParams {
    /// Planet radius
    pub planet_radius: f32,
    /// Atmosphere height
    pub atmosphere_height: f32,
    /// Rayleigh scale height
    pub rayleigh_scale_height: f32,
    /// Mie scale height
    pub mie_scale_height: f32,
    /// Rayleigh scattering
    pub rayleigh_scattering: [f32; 3],
    /// Mie asymmetry
    pub mie_asymmetry: f32,
    /// Mie scattering
    pub mie_scattering: f32,
    /// Mie absorption
    pub mie_absorption: f32,
    /// Ozone altitude
    pub ozone_altitude: f32,
    /// Ozone thickness
    pub ozone_thickness: f32,
    /// Ozone absorption
    pub ozone_absorption: [f32; 3],
    /// View samples
    pub view_samples: u32,
    /// Light samples
    pub light_samples: u32,
    /// Features
    pub features: u32,
    /// Padding
    pub _padding: [f32; 2],
}

impl GpuAtmosphereParams {
    /// From create info
    pub fn from_create_info(info: &AtmosphereCreateInfo) -> Self {
        Self {
            planet_radius: info.planet.radius,
            atmosphere_height: info.planet.atmosphere_height,
            rayleigh_scale_height: info.rayleigh.scale_height,
            mie_scale_height: info.mie.scale_height,
            rayleigh_scattering: info.rayleigh.scattering,
            mie_asymmetry: info.mie.asymmetry,
            mie_scattering: info.mie.scattering,
            mie_absorption: info.mie.absorption,
            ozone_altitude: info.ozone.altitude,
            ozone_thickness: info.ozone.thickness,
            ozone_absorption: info.ozone.absorption,
            view_samples: info.quality.view_samples,
            light_samples: info.quality.light_samples,
            features: info.features.bits(),
            _padding: [0.0; 2],
        }
    }
}

/// GPU sky render params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSkyRenderParams {
    /// Sun direction
    pub sun_direction: [f32; 3],
    /// Sun intensity
    pub sun_intensity: f32,
    /// Sun angular radius
    pub sun_angular_radius: f32,
    /// Camera height
    pub camera_height: f32,
    /// Camera position (relative to planet center)
    pub camera_position: [f32; 3],
    /// Exposure
    pub exposure: f32,
}

// ============================================================================
// Render Requests
// ============================================================================

/// Atmosphere LUT generation request
#[derive(Clone, Debug)]
pub struct AtmosphereLutRequest {
    /// Atmosphere handle
    pub atmosphere: AtmosphereScatteringHandle,
    /// Generate transmittance LUT
    pub generate_transmittance: bool,
    /// Generate multi-scatter LUT
    pub generate_multi_scatter: bool,
    /// Generate sky view LUT
    pub generate_sky_view: bool,
}

impl AtmosphereLutRequest {
    /// Creates new request (all LUTs)
    pub fn all(atmosphere: AtmosphereScatteringHandle) -> Self {
        Self {
            atmosphere,
            generate_transmittance: true,
            generate_multi_scatter: true,
            generate_sky_view: true,
        }
    }

    /// Only sky view (for per-frame update)
    pub fn sky_view_only(atmosphere: AtmosphereScatteringHandle) -> Self {
        Self {
            atmosphere,
            generate_transmittance: false,
            generate_multi_scatter: false,
            generate_sky_view: true,
        }
    }
}

/// Sky render request
#[derive(Clone, Debug)]
pub struct SkyRenderRequest {
    /// Atmosphere handle
    pub atmosphere: AtmosphereScatteringHandle,
    /// Output texture
    pub output: u64,
    /// Depth texture (for aerial perspective)
    pub depth: Option<u64>,
    /// Sun direction
    pub sun_direction: [f32; 3],
    /// Sun intensity
    pub sun_intensity: f32,
    /// Camera position
    pub camera_position: [f32; 3],
    /// Camera height above ground
    pub camera_height: f32,
    /// View-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Inverse view-projection matrix
    pub inv_view_projection: [[f32; 4]; 4],
}

impl SkyRenderRequest {
    /// Creates new request
    pub fn new(atmosphere: AtmosphereScatteringHandle, output: u64) -> Self {
        Self {
            atmosphere,
            output,
            depth: None,
            sun_direction: [0.0, 1.0, 0.0],
            sun_intensity: 1.0,
            camera_position: [0.0, 0.0, 0.0],
            camera_height: 0.0,
            view_projection: [[0.0; 4]; 4],
            inv_view_projection: [[0.0; 4]; 4],
        }
    }

    /// With depth for aerial perspective
    pub fn with_depth(mut self, depth: u64) -> Self {
        self.depth = Some(depth);
        self
    }

    /// With sun
    pub fn with_sun(mut self, direction: [f32; 3], intensity: f32) -> Self {
        self.sun_direction = direction;
        self.sun_intensity = intensity;
        self
    }

    /// With camera
    pub fn with_camera(mut self, position: [f32; 3], height: f32) -> Self {
        self.camera_position = position;
        self.camera_height = height;
        self
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Atmosphere statistics
#[derive(Clone, Debug, Default)]
pub struct AtmosphereStats {
    /// LUT generation time (microseconds)
    pub lut_generation_time_us: u64,
    /// Sky render time (microseconds)
    pub sky_render_time_us: u64,
    /// Aerial perspective time (microseconds)
    pub aerial_perspective_time_us: u64,
    /// LUT memory usage (bytes)
    pub lut_memory_usage: u64,
    /// Transmittance LUT size
    pub transmittance_lut_size: [u32; 2],
    /// Multi-scatter LUT size
    pub multi_scatter_lut_size: [u32; 2],
    /// Sky view LUT size
    pub sky_view_lut_size: [u32; 2],
}

impl AtmosphereStats {
    /// Total render time
    pub fn total_time_us(&self) -> u64 {
        self.lut_generation_time_us + self.sky_render_time_us + self.aerial_perspective_time_us
    }
}
