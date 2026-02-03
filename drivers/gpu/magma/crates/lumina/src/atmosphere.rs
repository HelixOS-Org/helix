//! Atmosphere Rendering Types for Lumina
//!
//! This module provides atmospheric scattering and sky rendering
//! including physical atmosphere models and volumetric clouds.

extern crate alloc;

use alloc::string::String;

// ============================================================================
// Atmosphere Handles
// ============================================================================

/// Atmosphere handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AtmosphereHandle(pub u64);

impl AtmosphereHandle {
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

impl Default for AtmosphereHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sky handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SkyHandle(pub u64);

impl SkyHandle {
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

impl Default for SkyHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Volumetric cloud handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CloudHandle(pub u64);

impl CloudHandle {
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

impl Default for CloudHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Atmosphere Configuration
// ============================================================================

/// Atmosphere create info
#[derive(Clone, Debug)]
pub struct AtmosphereCreateInfo {
    /// Name
    pub name: String,
    /// Atmosphere model
    pub model: AtmosphereModel,
    /// Planet radius (km)
    pub planet_radius: f32,
    /// Atmosphere height (km)
    pub atmosphere_height: f32,
    /// Rayleigh scattering coefficients
    pub rayleigh: RayleighScattering,
    /// Mie scattering coefficients
    pub mie: MieScattering,
    /// Ozone absorption
    pub ozone: OzoneAbsorption,
    /// Multi-scattering enabled
    pub multi_scattering: bool,
}

impl AtmosphereCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            model: AtmosphereModel::Bruneton,
            planet_radius: 6371.0,
            atmosphere_height: 100.0,
            rayleigh: RayleighScattering::earth(),
            mie: MieScattering::earth(),
            ozone: OzoneAbsorption::earth(),
            multi_scattering: true,
        }
    }

    /// Earth atmosphere
    pub fn earth() -> Self {
        Self::new()
    }

    /// Mars atmosphere
    pub fn mars() -> Self {
        Self {
            planet_radius: 3389.5,
            atmosphere_height: 200.0,
            rayleigh: RayleighScattering::mars(),
            mie: MieScattering::mars(),
            ozone: OzoneAbsorption::none(),
            ..Self::new()
        }
    }

    /// Alien atmosphere
    pub fn alien() -> Self {
        Self {
            rayleigh: RayleighScattering {
                coefficients: [0.1e-5, 2.0e-5, 5.0e-5],
                scale_height: 10.0,
            },
            mie: MieScattering::default(),
            ..Self::new()
        }
    }

    /// With model
    pub fn with_model(mut self, model: AtmosphereModel) -> Self {
        self.model = model;
        self
    }

    /// With planet radius
    pub fn with_radius(mut self, radius_km: f32) -> Self {
        self.planet_radius = radius_km;
        self
    }
}

impl Default for AtmosphereCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Atmosphere model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AtmosphereModel {
    /// Bruneton model (physically accurate)
    #[default]
    Bruneton = 0,
    /// Preetham model (simpler)
    Preetham = 1,
    /// Hosek-Wilkie model
    HosekWilkie = 2,
    /// Nishita model
    Nishita = 3,
    /// Simple gradient
    Gradient = 4,
}

/// Rayleigh scattering parameters
#[derive(Clone, Copy, Debug)]
pub struct RayleighScattering {
    /// Scattering coefficients at sea level (RGB)
    pub coefficients: [f32; 3],
    /// Scale height (km)
    pub scale_height: f32,
}

impl RayleighScattering {
    /// Earth Rayleigh scattering
    pub fn earth() -> Self {
        Self {
            // Wavelength-dependent coefficients (1/m)
            coefficients: [5.802e-6, 13.558e-6, 33.1e-6],
            scale_height: 8.0,
        }
    }

    /// Mars Rayleigh scattering
    pub fn mars() -> Self {
        Self {
            coefficients: [19.918e-6, 13.57e-6, 5.75e-6],
            scale_height: 11.1,
        }
    }
}

impl Default for RayleighScattering {
    fn default() -> Self {
        Self::earth()
    }
}

/// Mie scattering parameters
#[derive(Clone, Copy, Debug)]
pub struct MieScattering {
    /// Scattering coefficient at sea level
    pub coefficient: f32,
    /// Absorption coefficient
    pub absorption: f32,
    /// Scale height (km)
    pub scale_height: f32,
    /// Phase function asymmetry (g)
    pub asymmetry: f32,
}

impl MieScattering {
    /// Earth Mie scattering
    pub fn earth() -> Self {
        Self {
            coefficient: 21e-6,
            absorption: 4.4e-6,
            scale_height: 1.2,
            asymmetry: 0.76,
        }
    }

    /// Mars Mie scattering
    pub fn mars() -> Self {
        Self {
            coefficient: 2.0e-5,
            absorption: 0.0,
            scale_height: 11.1,
            asymmetry: 0.63,
        }
    }

    /// Clear sky (low Mie)
    pub fn clear() -> Self {
        Self {
            coefficient: 5e-6,
            ..Self::earth()
        }
    }

    /// Hazy (high Mie)
    pub fn hazy() -> Self {
        Self {
            coefficient: 50e-6,
            scale_height: 1.5,
            ..Self::earth()
        }
    }
}

impl Default for MieScattering {
    fn default() -> Self {
        Self::earth()
    }
}

/// Ozone absorption parameters
#[derive(Clone, Copy, Debug)]
pub struct OzoneAbsorption {
    /// Absorption coefficients (RGB)
    pub coefficients: [f32; 3],
    /// Center altitude (km)
    pub center_altitude: f32,
    /// Layer width (km)
    pub width: f32,
}

impl OzoneAbsorption {
    /// Earth ozone layer
    pub fn earth() -> Self {
        Self {
            coefficients: [0.65e-6, 1.881e-6, 0.085e-6],
            center_altitude: 25.0,
            width: 15.0,
        }
    }

    /// No ozone
    pub fn none() -> Self {
        Self {
            coefficients: [0.0, 0.0, 0.0],
            center_altitude: 25.0,
            width: 15.0,
        }
    }
}

impl Default for OzoneAbsorption {
    fn default() -> Self {
        Self::earth()
    }
}

// ============================================================================
// Sky Configuration
// ============================================================================

/// Sky create info
#[derive(Clone, Debug)]
pub struct SkyCreateInfo {
    /// Name
    pub name: String,
    /// Sky type
    pub sky_type: SkyType,
    /// Atmosphere handle (for atmospheric sky)
    pub atmosphere: AtmosphereHandle,
    /// Sun parameters
    pub sun: SunParameters,
    /// Moon parameters
    pub moon: Option<MoonParameters>,
    /// Stars enabled
    pub stars: bool,
    /// Resolution
    pub resolution: u32,
}

impl SkyCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            sky_type: SkyType::Atmospheric,
            atmosphere: AtmosphereHandle::NULL,
            sun: SunParameters::default(),
            moon: None,
            stars: true,
            resolution: 256,
        }
    }

    /// Atmospheric sky
    pub fn atmospheric(atmosphere: AtmosphereHandle) -> Self {
        Self {
            atmosphere,
            ..Self::new()
        }
    }

    /// HDRI sky
    pub fn hdri() -> Self {
        Self {
            sky_type: SkyType::Hdri,
            ..Self::new()
        }
    }

    /// Procedural sky
    pub fn procedural() -> Self {
        Self {
            sky_type: SkyType::Procedural,
            ..Self::new()
        }
    }

    /// With sun
    pub fn with_sun(mut self, sun: SunParameters) -> Self {
        self.sun = sun;
        self
    }

    /// With moon
    pub fn with_moon(mut self, moon: MoonParameters) -> Self {
        self.moon = Some(moon);
        self
    }

    /// With stars
    pub fn with_stars(mut self, enabled: bool) -> Self {
        self.stars = enabled;
        self
    }
}

impl Default for SkyCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Sky type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SkyType {
    /// Atmospheric scattering
    #[default]
    Atmospheric = 0,
    /// HDRI cubemap
    Hdri = 1,
    /// Procedural
    Procedural = 2,
    /// Solid color
    SolidColor = 3,
    /// Gradient
    Gradient = 4,
}

/// Sun parameters
#[derive(Clone, Copy, Debug)]
pub struct SunParameters {
    /// Direction (normalized)
    pub direction: [f32; 3],
    /// Angular diameter (degrees)
    pub angular_diameter: f32,
    /// Color temperature (Kelvin)
    pub temperature: f32,
    /// Intensity multiplier
    pub intensity: f32,
    /// Enabled
    pub enabled: bool,
}

impl SunParameters {
    /// Creates parameters
    pub fn new() -> Self {
        Self {
            direction: [0.0, -1.0, 0.0],
            angular_diameter: 0.53,
            temperature: 5778.0,
            intensity: 1.0,
            enabled: true,
        }
    }

    /// From elevation and azimuth (degrees)
    pub fn from_angles(elevation: f32, azimuth: f32) -> Self {
        let elev_rad = elevation.to_radians();
        let azim_rad = azimuth.to_radians();

        Self {
            direction: [
                elev_rad.cos() * azim_rad.sin(),
                -elev_rad.sin(),
                elev_rad.cos() * azim_rad.cos(),
            ],
            ..Self::new()
        }
    }

    /// Noon sun
    pub fn noon() -> Self {
        Self {
            direction: [0.0, -1.0, 0.0],
            ..Self::new()
        }
    }

    /// Sunset sun
    pub fn sunset() -> Self {
        Self {
            direction: [0.707, -0.1, 0.707],
            temperature: 3000.0,
            ..Self::new()
        }
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for SunParameters {
    fn default() -> Self {
        Self::new()
    }
}

/// Moon parameters
#[derive(Clone, Copy, Debug)]
pub struct MoonParameters {
    /// Direction (normalized)
    pub direction: [f32; 3],
    /// Angular diameter (degrees)
    pub angular_diameter: f32,
    /// Phase (0.0 = new, 0.5 = full, 1.0 = new)
    pub phase: f32,
    /// Intensity
    pub intensity: f32,
    /// Enabled
    pub enabled: bool,
}

impl MoonParameters {
    /// Creates parameters
    pub fn new() -> Self {
        Self {
            direction: [0.0, -1.0, 0.0],
            angular_diameter: 0.52,
            phase: 0.5,
            intensity: 0.02,
            enabled: true,
        }
    }

    /// Full moon
    pub fn full() -> Self {
        Self { phase: 0.5, ..Self::new() }
    }

    /// New moon
    pub fn new_moon() -> Self {
        Self { phase: 0.0, ..Self::new() }
    }
}

impl Default for MoonParameters {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Atmosphere GPU Data
// ============================================================================

/// Atmosphere GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct AtmosphereGpuData {
    /// Sun direction
    pub sun_direction: [f32; 3],
    /// Sun intensity
    pub sun_intensity: f32,
    /// Rayleigh scattering
    pub rayleigh_scattering: [f32; 3],
    /// Rayleigh scale height
    pub rayleigh_scale_height: f32,
    /// Mie scattering
    pub mie_scattering: f32,
    /// Mie absorption
    pub mie_absorption: f32,
    /// Mie scale height
    pub mie_scale_height: f32,
    /// Mie asymmetry (g)
    pub mie_asymmetry: f32,
    /// Ozone absorption
    pub ozone_absorption: [f32; 3],
    /// Planet radius (km)
    pub planet_radius: f32,
    /// Atmosphere height (km)
    pub atmosphere_height: f32,
    /// Multi-scattering factor
    pub multi_scattering_factor: f32,
    /// Padding
    pub _padding: [f32; 2],
}

impl AtmosphereGpuData {
    /// Creates GPU data from create info
    pub fn from_info(info: &AtmosphereCreateInfo) -> Self {
        Self {
            sun_direction: [0.0, -1.0, 0.0],
            sun_intensity: 1.0,
            rayleigh_scattering: info.rayleigh.coefficients,
            rayleigh_scale_height: info.rayleigh.scale_height,
            mie_scattering: info.mie.coefficient,
            mie_absorption: info.mie.absorption,
            mie_scale_height: info.mie.scale_height,
            mie_asymmetry: info.mie.asymmetry,
            ozone_absorption: info.ozone.coefficients,
            planet_radius: info.planet_radius,
            atmosphere_height: info.atmosphere_height,
            multi_scattering_factor: if info.multi_scattering { 1.0 } else { 0.0 },
            _padding: [0.0; 2],
        }
    }
}

// ============================================================================
// Volumetric Clouds
// ============================================================================

/// Cloud create info
#[derive(Clone, Debug)]
pub struct CloudCreateInfo {
    /// Name
    pub name: String,
    /// Cloud type
    pub cloud_type: CloudType,
    /// Cloud layer
    pub layer: CloudLayer,
    /// Weather map resolution
    pub weather_resolution: u32,
    /// Shape noise resolution
    pub shape_resolution: u32,
    /// Detail noise resolution
    pub detail_resolution: u32,
    /// Quality
    pub quality: CloudQuality,
}

impl CloudCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            cloud_type: CloudType::Volumetric,
            layer: CloudLayer::default(),
            weather_resolution: 512,
            shape_resolution: 128,
            detail_resolution: 32,
            quality: CloudQuality::Medium,
        }
    }

    /// High quality volumetric clouds
    pub fn high_quality() -> Self {
        Self {
            weather_resolution: 1024,
            shape_resolution: 256,
            detail_resolution: 64,
            quality: CloudQuality::High,
            ..Self::new()
        }
    }

    /// Performance clouds
    pub fn performance() -> Self {
        Self {
            weather_resolution: 256,
            shape_resolution: 64,
            detail_resolution: 32,
            quality: CloudQuality::Low,
            ..Self::new()
        }
    }

    /// With layer
    pub fn with_layer(mut self, layer: CloudLayer) -> Self {
        self.layer = layer;
        self
    }
}

impl Default for CloudCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Cloud type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CloudType {
    /// Full volumetric
    #[default]
    Volumetric = 0,
    /// Billboard (2D)
    Billboard = 1,
    /// Skybox texture
    Skybox = 2,
}

/// Cloud quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CloudQuality {
    /// Low quality (fast)
    Low = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High = 2,
    /// Ultra quality
    Ultra = 3,
}

impl CloudQuality {
    /// Primary ray march steps
    pub fn primary_steps(&self) -> u32 {
        match self {
            Self::Low => 64,
            Self::Medium => 128,
            Self::High => 256,
            Self::Ultra => 512,
        }
    }

    /// Light ray march steps
    pub fn light_steps(&self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 6,
            Self::High => 8,
            Self::Ultra => 12,
        }
    }
}

/// Cloud layer
#[derive(Clone, Copy, Debug)]
pub struct CloudLayer {
    /// Bottom altitude (km)
    pub altitude_bottom: f32,
    /// Top altitude (km)
    pub altitude_top: f32,
    /// Coverage (0-1)
    pub coverage: f32,
    /// Density
    pub density: f32,
    /// Wind direction
    pub wind_direction: [f32; 2],
    /// Wind speed (km/h)
    pub wind_speed: f32,
    /// Type weights (cumulus, stratus, stratocumulus, cumulonimbus)
    pub type_weights: [f32; 4],
}

impl CloudLayer {
    /// Creates layer
    pub fn new(altitude_bottom: f32, altitude_top: f32) -> Self {
        Self {
            altitude_bottom,
            altitude_top,
            coverage: 0.5,
            density: 1.0,
            wind_direction: [1.0, 0.0],
            wind_speed: 10.0,
            type_weights: [1.0, 0.0, 0.0, 0.0], // Default cumulus
        }
    }

    /// Low clouds
    pub fn low() -> Self {
        Self::new(1.5, 3.0)
    }

    /// Medium clouds
    pub fn medium() -> Self {
        Self::new(3.0, 6.0)
    }

    /// High clouds
    pub fn high() -> Self {
        Self::new(6.0, 12.0)
    }

    /// Storm clouds
    pub fn storm() -> Self {
        Self {
            type_weights: [0.0, 0.0, 0.0, 1.0], // Cumulonimbus
            coverage: 0.8,
            density: 2.0,
            ..Self::new(0.5, 15.0)
        }
    }

    /// With coverage
    pub fn with_coverage(mut self, coverage: f32) -> Self {
        self.coverage = coverage.clamp(0.0, 1.0);
        self
    }

    /// With wind
    pub fn with_wind(mut self, direction: [f32; 2], speed: f32) -> Self {
        self.wind_direction = direction;
        self.wind_speed = speed;
        self
    }
}

impl Default for CloudLayer {
    fn default() -> Self {
        Self::low()
    }
}

/// Cloud GPU parameters
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CloudGpuParams {
    /// Layer bottom altitude
    pub altitude_bottom: f32,
    /// Layer top altitude
    pub altitude_top: f32,
    /// Coverage
    pub coverage: f32,
    /// Density
    pub density: f32,
    /// Wind direction and speed
    pub wind: [f32; 3],
    /// Time
    pub time: f32,
    /// Type weights
    pub type_weights: [f32; 4],
    /// Scattering coefficients
    pub scattering: [f32; 3],
    /// Absorption
    pub absorption: f32,
    /// Eccentricity (HG phase function)
    pub eccentricity: f32,
    /// Silver intensity
    pub silver_intensity: f32,
    /// Silver spread
    pub silver_spread: f32,
    /// Ray march steps
    pub primary_steps: u32,
}

// ============================================================================
// Time of Day
// ============================================================================

/// Time of day settings
#[derive(Clone, Copy, Debug)]
pub struct TimeOfDay {
    /// Hour (0-24)
    pub hour: f32,
    /// Day of year (1-365)
    pub day_of_year: u32,
    /// Latitude (degrees)
    pub latitude: f32,
    /// Longitude (degrees)
    pub longitude: f32,
    /// Time zone offset (hours)
    pub timezone: f32,
}

impl TimeOfDay {
    /// Creates time
    pub fn new(hour: f32) -> Self {
        Self {
            hour,
            day_of_year: 172, // Summer solstice
            latitude: 45.0,
            longitude: 0.0,
            timezone: 0.0,
        }
    }

    /// Noon
    pub fn noon() -> Self {
        Self::new(12.0)
    }

    /// Sunset
    pub fn sunset() -> Self {
        Self::new(19.0)
    }

    /// Night
    pub fn night() -> Self {
        Self::new(23.0)
    }

    /// Calculate sun elevation angle
    pub fn sun_elevation(&self) -> f32 {
        // Simplified solar position calculation
        let day_angle = 2.0 * core::f32::consts::PI * (self.day_of_year as f32 - 1.0) / 365.0;
        let declination = 23.45_f32.to_radians() * (day_angle + 10.0_f32.to_radians()).sin();

        let solar_time = self.hour + self.longitude / 15.0 - self.timezone;
        let hour_angle = (solar_time - 12.0) * 15.0_f32.to_radians();

        let lat_rad = self.latitude.to_radians();
        let elevation =
            (lat_rad.sin() * declination.sin() + lat_rad.cos() * declination.cos() * hour_angle.cos()).asin();

        elevation.to_degrees()
    }

    /// Calculate sun direction
    pub fn sun_direction(&self) -> [f32; 3] {
        let elevation = self.sun_elevation().to_radians();
        // Simplified - using hour angle for azimuth approximation
        let hour_angle = (self.hour - 12.0) * 15.0_f32.to_radians();

        [
            hour_angle.sin() * elevation.cos(),
            -elevation.sin(),
            hour_angle.cos() * elevation.cos(),
        ]
    }

    /// Is daytime
    pub fn is_day(&self) -> bool {
        self.sun_elevation() > 0.0
    }
}

impl Default for TimeOfDay {
    fn default() -> Self {
        Self::noon()
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
    /// Cloud render time (microseconds)
    pub cloud_render_time_us: u64,
    /// Total ray march steps
    pub ray_march_steps: u64,
}
