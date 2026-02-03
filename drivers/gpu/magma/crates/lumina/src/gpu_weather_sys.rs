//! GPU Weather Effects System for Lumina
//!
//! This module provides GPU-accelerated weather effects including
//! rain, snow, fog, clouds, lightning, and atmospheric phenomena.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Weather System Handles
// ============================================================================

/// GPU weather system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuWeatherSystemHandle(pub u64);

impl GpuWeatherSystemHandle {
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

impl Default for GpuWeatherSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Weather effect handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct WeatherEffectHandle(pub u64);

impl WeatherEffectHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for WeatherEffectHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Cloud layer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CloudLayerHandle(pub u64);

impl CloudLayerHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for CloudLayerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Lightning handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LightningHandle(pub u64);

impl LightningHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LightningHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Weather System Creation
// ============================================================================

/// GPU weather system create info
#[derive(Clone, Debug)]
pub struct GpuWeatherSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max precipitation particles
    pub max_particles: u32,
    /// Max cloud layers
    pub max_cloud_layers: u32,
    /// Features
    pub features: WeatherFeatures,
    /// Quality
    pub quality: WeatherQuality,
}

impl GpuWeatherSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_particles: 100000,
            max_cloud_layers: 4,
            features: WeatherFeatures::all(),
            quality: WeatherQuality::High,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max particles
    pub fn with_max_particles(mut self, count: u32) -> Self {
        self.max_particles = count;
        self
    }

    /// With max cloud layers
    pub fn with_max_cloud_layers(mut self, count: u32) -> Self {
        self.max_cloud_layers = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: WeatherFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: WeatherQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Standard
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self::new()
            .with_max_particles(500000)
            .with_quality(WeatherQuality::Ultra)
    }

    /// Mobile
    pub fn mobile() -> Self {
        Self::new()
            .with_max_particles(10000)
            .with_max_cloud_layers(2)
            .with_quality(WeatherQuality::Low)
    }
}

impl Default for GpuWeatherSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Weather features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct WeatherFeatures: u32 {
        /// None
        const NONE = 0;
        /// Rain
        const RAIN = 1 << 0;
        /// Snow
        const SNOW = 1 << 1;
        /// Fog
        const FOG = 1 << 2;
        /// Volumetric clouds
        const VOLUMETRIC_CLOUDS = 1 << 3;
        /// Lightning
        const LIGHTNING = 1 << 4;
        /// Wind effects
        const WIND = 1 << 5;
        /// Puddles
        const PUDDLES = 1 << 6;
        /// Snow accumulation
        const ACCUMULATION = 1 << 7;
        /// Wetness
        const WETNESS = 1 << 8;
        /// Dust/sand
        const DUST = 1 << 9;
        /// All
        const ALL = 0x3FF;
    }
}

impl Default for WeatherFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Weather quality level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum WeatherQuality {
    /// Low quality
    Low = 0,
    /// Medium quality
    Medium = 1,
    /// High quality
    #[default]
    High = 2,
    /// Ultra quality
    Ultra = 3,
}

// ============================================================================
// Weather State
// ============================================================================

/// Current weather state
#[derive(Clone, Copy, Debug)]
pub struct WeatherState {
    /// Precipitation type
    pub precipitation: PrecipitationType,
    /// Precipitation intensity (0-1)
    pub precipitation_intensity: f32,
    /// Cloud coverage (0-1)
    pub cloud_coverage: f32,
    /// Fog density
    pub fog_density: f32,
    /// Wind direction (radians)
    pub wind_direction: f32,
    /// Wind speed
    pub wind_speed: f32,
    /// Lightning enabled
    pub lightning: bool,
    /// Temperature (affects rain vs snow)
    pub temperature: f32,
}

impl WeatherState {
    /// Clear weather
    pub const fn clear() -> Self {
        Self {
            precipitation: PrecipitationType::None,
            precipitation_intensity: 0.0,
            cloud_coverage: 0.1,
            fog_density: 0.0,
            wind_direction: 0.0,
            wind_speed: 2.0,
            lightning: false,
            temperature: 20.0,
        }
    }

    /// Cloudy
    pub const fn cloudy() -> Self {
        Self {
            precipitation: PrecipitationType::None,
            precipitation_intensity: 0.0,
            cloud_coverage: 0.7,
            fog_density: 0.01,
            wind_direction: 0.5,
            wind_speed: 5.0,
            lightning: false,
            temperature: 15.0,
        }
    }

    /// Light rain
    pub const fn light_rain() -> Self {
        Self {
            precipitation: PrecipitationType::Rain,
            precipitation_intensity: 0.3,
            cloud_coverage: 0.8,
            fog_density: 0.02,
            wind_direction: 0.8,
            wind_speed: 8.0,
            lightning: false,
            temperature: 12.0,
        }
    }

    /// Heavy rain
    pub const fn heavy_rain() -> Self {
        Self {
            precipitation: PrecipitationType::Rain,
            precipitation_intensity: 1.0,
            cloud_coverage: 1.0,
            fog_density: 0.05,
            wind_direction: 1.0,
            wind_speed: 15.0,
            lightning: false,
            temperature: 10.0,
        }
    }

    /// Storm
    pub const fn storm() -> Self {
        Self {
            precipitation: PrecipitationType::Rain,
            precipitation_intensity: 1.0,
            cloud_coverage: 1.0,
            fog_density: 0.08,
            wind_direction: 1.2,
            wind_speed: 25.0,
            lightning: true,
            temperature: 8.0,
        }
    }

    /// Light snow
    pub const fn light_snow() -> Self {
        Self {
            precipitation: PrecipitationType::Snow,
            precipitation_intensity: 0.3,
            cloud_coverage: 0.9,
            fog_density: 0.01,
            wind_direction: 0.3,
            wind_speed: 3.0,
            lightning: false,
            temperature: -5.0,
        }
    }

    /// Heavy snow
    pub const fn heavy_snow() -> Self {
        Self {
            precipitation: PrecipitationType::Snow,
            precipitation_intensity: 1.0,
            cloud_coverage: 1.0,
            fog_density: 0.1,
            wind_direction: 0.5,
            wind_speed: 10.0,
            lightning: false,
            temperature: -10.0,
        }
    }

    /// Blizzard
    pub const fn blizzard() -> Self {
        Self {
            precipitation: PrecipitationType::Snow,
            precipitation_intensity: 1.0,
            cloud_coverage: 1.0,
            fog_density: 0.3,
            wind_direction: 0.8,
            wind_speed: 30.0,
            lightning: false,
            temperature: -15.0,
        }
    }

    /// Foggy
    pub const fn foggy() -> Self {
        Self {
            precipitation: PrecipitationType::None,
            precipitation_intensity: 0.0,
            cloud_coverage: 0.5,
            fog_density: 0.15,
            wind_direction: 0.0,
            wind_speed: 1.0,
            lightning: false,
            temperature: 10.0,
        }
    }

    /// Dense fog
    pub const fn dense_fog() -> Self {
        Self {
            precipitation: PrecipitationType::None,
            precipitation_intensity: 0.0,
            cloud_coverage: 0.6,
            fog_density: 0.5,
            wind_direction: 0.0,
            wind_speed: 0.5,
            lightning: false,
            temperature: 8.0,
        }
    }
}

impl Default for WeatherState {
    fn default() -> Self {
        Self::clear()
    }
}

/// Precipitation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PrecipitationType {
    /// No precipitation
    #[default]
    None = 0,
    /// Rain
    Rain = 1,
    /// Snow
    Snow = 2,
    /// Sleet
    Sleet = 3,
    /// Hail
    Hail = 4,
}

// ============================================================================
// Rain Settings
// ============================================================================

/// Rain settings
#[derive(Clone, Copy, Debug)]
pub struct RainSettings {
    /// Drop speed
    pub drop_speed: f32,
    /// Drop length
    pub drop_length: f32,
    /// Drop width
    pub drop_width: f32,
    /// Color
    pub color: [f32; 4],
    /// Splash enabled
    pub splash: bool,
    /// Splash intensity
    pub splash_intensity: f32,
    /// Ripples enabled
    pub ripples: bool,
    /// Collision radius
    pub collision_radius: f32,
}

impl RainSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            drop_speed: 15.0,
            drop_length: 0.3,
            drop_width: 0.002,
            color: [0.7, 0.75, 0.8, 0.4],
            splash: true,
            splash_intensity: 1.0,
            ripples: true,
            collision_radius: 50.0,
        }
    }

    /// Light rain
    pub const fn light() -> Self {
        Self {
            drop_speed: 12.0,
            drop_length: 0.2,
            drop_width: 0.001,
            color: [0.7, 0.75, 0.8, 0.3],
            splash: true,
            splash_intensity: 0.5,
            ripples: true,
            collision_radius: 30.0,
        }
    }

    /// Heavy rain
    pub const fn heavy() -> Self {
        Self {
            drop_speed: 20.0,
            drop_length: 0.5,
            drop_width: 0.003,
            color: [0.6, 0.65, 0.7, 0.5],
            splash: true,
            splash_intensity: 1.5,
            ripples: true,
            collision_radius: 80.0,
        }
    }
}

impl Default for RainSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Snow Settings
// ============================================================================

/// Snow settings
#[derive(Clone, Copy, Debug)]
pub struct SnowSettings {
    /// Fall speed
    pub fall_speed: f32,
    /// Flake size range
    pub flake_size_min: f32,
    /// Flake size max
    pub flake_size_max: f32,
    /// Turbulence
    pub turbulence: f32,
    /// Color
    pub color: [f32; 4],
    /// Accumulation enabled
    pub accumulation: bool,
    /// Accumulation rate
    pub accumulation_rate: f32,
    /// Melt rate
    pub melt_rate: f32,
}

impl SnowSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            fall_speed: 2.0,
            flake_size_min: 0.01,
            flake_size_max: 0.05,
            turbulence: 1.0,
            color: [1.0, 1.0, 1.0, 0.8],
            accumulation: true,
            accumulation_rate: 0.01,
            melt_rate: 0.001,
        }
    }

    /// Light snow
    pub const fn light() -> Self {
        Self {
            fall_speed: 1.5,
            flake_size_min: 0.005,
            flake_size_max: 0.02,
            turbulence: 0.5,
            color: [1.0, 1.0, 1.0, 0.6],
            accumulation: true,
            accumulation_rate: 0.005,
            melt_rate: 0.001,
        }
    }

    /// Heavy snow
    pub const fn heavy() -> Self {
        Self {
            fall_speed: 3.0,
            flake_size_min: 0.02,
            flake_size_max: 0.08,
            turbulence: 1.5,
            color: [1.0, 1.0, 1.0, 0.9],
            accumulation: true,
            accumulation_rate: 0.03,
            melt_rate: 0.0005,
        }
    }
}

impl Default for SnowSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Fog Settings
// ============================================================================

/// Fog settings
#[derive(Clone, Copy, Debug)]
pub struct FogSettings {
    /// Fog mode
    pub mode: FogMode,
    /// Fog color
    pub color: [f32; 3],
    /// Start distance
    pub start: f32,
    /// End distance
    pub end: f32,
    /// Density (for exponential fog)
    pub density: f32,
    /// Height falloff
    pub height_falloff: f32,
    /// Max opacity
    pub max_opacity: f32,
    /// Inscattering color
    pub inscatter_color: [f32; 3],
}

impl FogSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            mode: FogMode::ExponentialSquared,
            color: [0.7, 0.75, 0.8],
            start: 0.0,
            end: 500.0,
            density: 0.01,
            height_falloff: 0.1,
            max_opacity: 1.0,
            inscatter_color: [0.8, 0.85, 0.9],
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            mode: FogMode::None,
            color: [0.0, 0.0, 0.0],
            start: 0.0,
            end: 0.0,
            density: 0.0,
            height_falloff: 0.0,
            max_opacity: 0.0,
            inscatter_color: [0.0, 0.0, 0.0],
        }
    }

    /// Light fog
    pub const fn light() -> Self {
        Self {
            mode: FogMode::ExponentialSquared,
            color: [0.8, 0.85, 0.9],
            start: 50.0,
            end: 1000.0,
            density: 0.005,
            height_falloff: 0.05,
            max_opacity: 0.7,
            inscatter_color: [0.9, 0.92, 0.95],
        }
    }

    /// Dense fog
    pub const fn dense() -> Self {
        Self {
            mode: FogMode::ExponentialSquared,
            color: [0.6, 0.65, 0.7],
            start: 5.0,
            end: 100.0,
            density: 0.1,
            height_falloff: 0.2,
            max_opacity: 1.0,
            inscatter_color: [0.7, 0.72, 0.75],
        }
    }

    /// Height fog
    pub const fn height_fog() -> Self {
        Self {
            mode: FogMode::HeightExp,
            color: [0.75, 0.8, 0.85],
            start: 0.0,
            end: 500.0,
            density: 0.02,
            height_falloff: 0.5,
            max_opacity: 0.9,
            inscatter_color: [0.85, 0.88, 0.92],
        }
    }
}

impl Default for FogSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Fog mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FogMode {
    /// No fog
    None = 0,
    /// Linear fog
    Linear = 1,
    /// Exponential fog
    Exponential = 2,
    /// Exponential squared
    #[default]
    ExponentialSquared = 3,
    /// Height-based exponential
    HeightExp = 4,
}

// ============================================================================
// Cloud Settings
// ============================================================================

/// Cloud layer create info
#[derive(Clone, Debug)]
pub struct CloudLayerCreateInfo {
    /// Name
    pub name: String,
    /// Cloud type
    pub cloud_type: CloudType,
    /// Base altitude
    pub altitude: f32,
    /// Thickness
    pub thickness: f32,
    /// Coverage (0-1)
    pub coverage: f32,
    /// Density
    pub density: f32,
    /// Wind velocity
    pub wind_velocity: [f32; 2],
    /// Color
    pub color: [f32; 3],
    /// Ambient color
    pub ambient_color: [f32; 3],
}

impl CloudLayerCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            cloud_type: CloudType::Cumulus,
            altitude: 1000.0,
            thickness: 500.0,
            coverage: 0.5,
            density: 0.5,
            wind_velocity: [5.0, 0.0],
            color: [1.0, 1.0, 1.0],
            ambient_color: [0.8, 0.85, 0.9],
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With cloud type
    pub fn with_type(mut self, cloud_type: CloudType) -> Self {
        self.cloud_type = cloud_type;
        self
    }

    /// With altitude
    pub fn with_altitude(mut self, altitude: f32) -> Self {
        self.altitude = altitude;
        self
    }

    /// With thickness
    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    /// With coverage
    pub fn with_coverage(mut self, coverage: f32) -> Self {
        self.coverage = coverage.clamp(0.0, 1.0);
        self
    }

    /// With density
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// Cumulus preset
    pub fn cumulus() -> Self {
        Self::new()
            .with_name("Cumulus")
            .with_type(CloudType::Cumulus)
            .with_altitude(1500.0)
            .with_thickness(800.0)
            .with_coverage(0.4)
    }

    /// Stratus preset
    pub fn stratus() -> Self {
        Self::new()
            .with_name("Stratus")
            .with_type(CloudType::Stratus)
            .with_altitude(500.0)
            .with_thickness(300.0)
            .with_coverage(0.8)
    }

    /// Cirrus preset
    pub fn cirrus() -> Self {
        Self::new()
            .with_name("Cirrus")
            .with_type(CloudType::Cirrus)
            .with_altitude(8000.0)
            .with_thickness(200.0)
            .with_coverage(0.3)
            .with_density(0.2)
    }

    /// Storm clouds preset
    pub fn storm_clouds() -> Self {
        Self::new()
            .with_name("StormClouds")
            .with_type(CloudType::Cumulonimbus)
            .with_altitude(500.0)
            .with_thickness(5000.0)
            .with_coverage(0.9)
            .with_density(0.8)
    }
}

impl Default for CloudLayerCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Cloud type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CloudType {
    /// Cumulus
    #[default]
    Cumulus = 0,
    /// Stratus
    Stratus = 1,
    /// Cirrus
    Cirrus = 2,
    /// Cumulonimbus (storm)
    Cumulonimbus = 3,
    /// Stratocumulus
    Stratocumulus = 4,
}

// ============================================================================
// Lightning
// ============================================================================

/// Lightning settings
#[derive(Clone, Copy, Debug)]
pub struct LightningSettings {
    /// Enabled
    pub enabled: bool,
    /// Flash interval min
    pub interval_min: f32,
    /// Flash interval max
    pub interval_max: f32,
    /// Flash duration
    pub flash_duration: f32,
    /// Branch probability
    pub branch_probability: f32,
    /// Color
    pub color: [f32; 3],
    /// Intensity
    pub intensity: f32,
    /// Audio enabled
    pub audio: bool,
}

impl LightningSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: true,
            interval_min: 5.0,
            interval_max: 20.0,
            flash_duration: 0.2,
            branch_probability: 0.6,
            color: [0.9, 0.95, 1.0],
            intensity: 10.0,
            audio: true,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            interval_min: 0.0,
            interval_max: 0.0,
            flash_duration: 0.0,
            branch_probability: 0.0,
            color: [0.0, 0.0, 0.0],
            intensity: 0.0,
            audio: false,
        }
    }

    /// Frequent
    pub const fn frequent() -> Self {
        Self {
            enabled: true,
            interval_min: 1.0,
            interval_max: 5.0,
            flash_duration: 0.3,
            branch_probability: 0.7,
            color: [0.9, 0.95, 1.0],
            intensity: 15.0,
            audio: true,
        }
    }
}

impl Default for LightningSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU weather constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuWeatherConstants {
    /// Time
    pub time: f32,
    /// Precipitation intensity
    pub precipitation_intensity: f32,
    /// Precipitation type
    pub precipitation_type: u32,
    /// Wind speed
    pub wind_speed: f32,
    /// Wind direction
    pub wind_direction: [f32; 2],
    /// Temperature
    pub temperature: f32,
    /// Wetness
    pub wetness: f32,
    /// Snow coverage
    pub snow_coverage: f32,
    /// Cloud coverage
    pub cloud_coverage: f32,
    /// Lightning flash
    pub lightning_flash: f32,
    /// Padding
    pub _pad: f32,
}

/// GPU fog constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuFogConstants {
    /// Fog color
    pub color: [f32; 4],
    /// Inscatter color
    pub inscatter_color: [f32; 4],
    /// Start distance
    pub start: f32,
    /// End distance
    pub end: f32,
    /// Density
    pub density: f32,
    /// Height falloff
    pub height_falloff: f32,
    /// Max opacity
    pub max_opacity: f32,
    /// Mode
    pub mode: u32,
    /// Padding
    pub _pad: [f32; 2],
}

/// GPU cloud layer data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCloudLayer {
    /// Altitude
    pub altitude: f32,
    /// Thickness
    pub thickness: f32,
    /// Coverage
    pub coverage: f32,
    /// Density
    pub density: f32,
    /// Wind velocity
    pub wind_velocity: [f32; 2],
    /// Cloud type
    pub cloud_type: u32,
    /// Padding
    pub _pad: f32,
    /// Color
    pub color: [f32; 4],
    /// Ambient color
    pub ambient_color: [f32; 4],
}

/// GPU precipitation particle
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuPrecipitationParticle {
    /// Position
    pub position: [f32; 3],
    /// Lifetime
    pub lifetime: f32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Size
    pub size: f32,
    /// Color
    pub color: [f32; 4],
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU weather statistics
#[derive(Clone, Debug, Default)]
pub struct GpuWeatherStats {
    /// Active particles
    pub active_particles: u32,
    /// Spawned this frame
    pub particles_spawned: u32,
    /// Died this frame
    pub particles_died: u32,
    /// Cloud layers
    pub cloud_layers: u32,
    /// Lightning strikes
    pub lightning_strikes: u32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
}
