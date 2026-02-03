//! GPU Fire and Smoke System for Lumina
//!
//! This module provides comprehensive GPU-accelerated fire and smoke simulation
//! including combustion, volumetric rendering, and heat transfer.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Fire Smoke System Handles
// ============================================================================

/// GPU fire smoke system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuFireSmokeSystemHandle(pub u64);

impl GpuFireSmokeSystemHandle {
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

impl Default for GpuFireSmokeSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Fire source handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FireSourceHandle(pub u64);

impl FireSourceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for FireSourceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Smoke volume handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SmokeVolumeHandle(pub u64);

impl SmokeVolumeHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SmokeVolumeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Explosion handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ExplosionHandle(pub u64);

impl ExplosionHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ExplosionHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Fire Smoke System Creation
// ============================================================================

/// GPU fire smoke system create info
#[derive(Clone, Debug)]
pub struct GpuFireSmokeSystemCreateInfo {
    /// Name
    pub name: String,
    /// Grid resolution
    pub grid_resolution: [u32; 3],
    /// Max particles
    pub max_particles: u32,
    /// Features
    pub features: FireSmokeFeatures,
    /// Simulation settings
    pub simulation: FireSmokeSimSettings,
    /// Rendering settings
    pub rendering: FireSmokeRenderSettings,
}

impl GpuFireSmokeSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            grid_resolution: [64, 128, 64],
            max_particles: 50_000,
            features: FireSmokeFeatures::all(),
            simulation: FireSmokeSimSettings::default(),
            rendering: FireSmokeRenderSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With grid resolution
    pub fn with_resolution(mut self, res: [u32; 3]) -> Self {
        self.grid_resolution = res;
        self
    }

    /// With max particles
    pub fn with_max_particles(mut self, count: u32) -> Self {
        self.max_particles = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: FireSmokeFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With simulation
    pub fn with_simulation(mut self, sim: FireSmokeSimSettings) -> Self {
        self.simulation = sim;
        self
    }

    /// With rendering
    pub fn with_rendering(mut self, render: FireSmokeRenderSettings) -> Self {
        self.rendering = render;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new()
            .with_resolution([128, 256, 128])
            .with_max_particles(200_000)
            .with_simulation(FireSmokeSimSettings::high_quality())
            .with_rendering(FireSmokeRenderSettings::high_quality())
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_resolution([32, 64, 32])
            .with_max_particles(10_000)
            .with_features(FireSmokeFeatures::BASIC)
            .with_simulation(FireSmokeSimSettings::mobile())
            .with_rendering(FireSmokeRenderSettings::mobile())
    }
}

impl Default for GpuFireSmokeSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Fire smoke features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct FireSmokeFeatures: u32 {
        /// None
        const NONE = 0;
        /// Fire simulation
        const FIRE = 1 << 0;
        /// Smoke simulation
        const SMOKE = 1 << 1;
        /// Heat transfer
        const HEAT = 1 << 2;
        /// Combustion
        const COMBUSTION = 1 << 3;
        /// Buoyancy
        const BUOYANCY = 1 << 4;
        /// Turbulence
        const TURBULENCE = 1 << 5;
        /// Embers/sparks
        const EMBERS = 1 << 6;
        /// Volumetric lighting
        const VOLUMETRIC = 1 << 7;
        /// Basic
        const BASIC = Self::FIRE.bits() | Self::SMOKE.bits() | Self::BUOYANCY.bits();
        /// All
        const ALL = 0xFF;
    }
}

impl Default for FireSmokeFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Simulation Settings
// ============================================================================

/// Fire smoke simulation settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FireSmokeSimSettings {
    /// Time step
    pub time_step: f32,
    /// Substeps
    pub substeps: u32,
    /// Buoyancy strength
    pub buoyancy: f32,
    /// Cooling rate
    pub cooling_rate: f32,
    /// Smoke rise rate
    pub smoke_rise: f32,
    /// Dissipation rate
    pub dissipation: f32,
    /// Turbulence strength
    pub turbulence: f32,
    /// Vorticity confinement
    pub vorticity: f32,
    /// Combustion temperature (K)
    pub combustion_temp: f32,
    /// Ignition temperature (K)
    pub ignition_temp: f32,
}

impl FireSmokeSimSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            time_step: 0.016,
            substeps: 4,
            buoyancy: 1.5,
            cooling_rate: 0.8,
            smoke_rise: 2.0,
            dissipation: 0.98,
            turbulence: 0.5,
            vorticity: 0.3,
            combustion_temp: 1273.0, // 1000°C
            ignition_temp: 573.0,    // 300°C
        }
    }

    /// With buoyancy
    pub const fn with_buoyancy(mut self, buoyancy: f32) -> Self {
        self.buoyancy = buoyancy;
        self
    }

    /// With cooling
    pub const fn with_cooling(mut self, rate: f32) -> Self {
        self.cooling_rate = rate;
        self
    }

    /// With turbulence
    pub const fn with_turbulence(mut self, strength: f32) -> Self {
        self.turbulence = strength;
        self
    }

    /// High quality preset
    pub const fn high_quality() -> Self {
        Self {
            time_step: 0.008,
            substeps: 8,
            buoyancy: 1.5,
            cooling_rate: 0.9,
            smoke_rise: 2.0,
            dissipation: 0.99,
            turbulence: 0.6,
            vorticity: 0.4,
            combustion_temp: 1273.0,
            ignition_temp: 573.0,
        }
    }

    /// Mobile preset
    pub const fn mobile() -> Self {
        Self {
            time_step: 0.033,
            substeps: 2,
            buoyancy: 1.5,
            cooling_rate: 0.7,
            smoke_rise: 2.0,
            dissipation: 0.95,
            turbulence: 0.3,
            vorticity: 0.2,
            combustion_temp: 1273.0,
            ignition_temp: 573.0,
        }
    }
}

impl Default for FireSmokeSimSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Render Settings
// ============================================================================

/// Fire smoke render settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FireSmokeRenderSettings {
    /// Ray march steps
    pub ray_march_steps: u32,
    /// Shadow steps
    pub shadow_steps: u32,
    /// Density scale
    pub density_scale: f32,
    /// Fire brightness
    pub fire_brightness: f32,
    /// Smoke absorption
    pub smoke_absorption: f32,
    /// Scattering coefficient
    pub scattering: f32,
    /// Ambient light
    pub ambient: f32,
    /// Light intensity
    pub light_intensity: f32,
}

impl FireSmokeRenderSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            ray_march_steps: 64,
            shadow_steps: 16,
            density_scale: 1.0,
            fire_brightness: 2.0,
            smoke_absorption: 0.5,
            scattering: 0.3,
            ambient: 0.1,
            light_intensity: 1.0,
        }
    }

    /// With ray march steps
    pub const fn with_steps(mut self, steps: u32) -> Self {
        self.ray_march_steps = steps;
        self
    }

    /// With density scale
    pub const fn with_density(mut self, scale: f32) -> Self {
        self.density_scale = scale;
        self
    }

    /// High quality preset
    pub const fn high_quality() -> Self {
        Self {
            ray_march_steps: 128,
            shadow_steps: 32,
            density_scale: 1.0,
            fire_brightness: 2.0,
            smoke_absorption: 0.5,
            scattering: 0.4,
            ambient: 0.1,
            light_intensity: 1.2,
        }
    }

    /// Mobile preset
    pub const fn mobile() -> Self {
        Self {
            ray_march_steps: 32,
            shadow_steps: 8,
            density_scale: 1.2,
            fire_brightness: 2.5,
            smoke_absorption: 0.4,
            scattering: 0.2,
            ambient: 0.15,
            light_intensity: 1.0,
        }
    }
}

impl Default for FireSmokeRenderSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Fire Source
// ============================================================================

/// Fire source create info
#[derive(Clone, Debug)]
pub struct FireSourceCreateInfo {
    /// Name
    pub name: String,
    /// Fire type
    pub fire_type: FireType,
    /// Position
    pub position: [f32; 3],
    /// Scale
    pub scale: [f32; 3],
    /// Intensity
    pub intensity: f32,
    /// Fuel rate
    pub fuel_rate: f32,
    /// Temperature (K)
    pub temperature: f32,
    /// Fire color gradient
    pub color_gradient: FireColorGradient,
    /// Smoke properties
    pub smoke: SmokeProperties,
}

impl FireSourceCreateInfo {
    /// Creates new fire source
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fire_type: FireType::Campfire,
            position: [0.0; 3],
            scale: [1.0; 3],
            intensity: 1.0,
            fuel_rate: 1.0,
            temperature: 1273.0,
            color_gradient: FireColorGradient::default(),
            smoke: SmokeProperties::default(),
        }
    }

    /// At position
    pub fn at(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With scale
    pub fn with_scale(mut self, scale: [f32; 3]) -> Self {
        self.scale = scale;
        self
    }

    /// With fire type
    pub fn with_type(mut self, fire_type: FireType) -> Self {
        self.fire_type = fire_type;
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.temperature = temp;
        self
    }

    /// With color gradient
    pub fn with_colors(mut self, gradient: FireColorGradient) -> Self {
        self.color_gradient = gradient;
        self
    }

    /// With smoke
    pub fn with_smoke(mut self, smoke: SmokeProperties) -> Self {
        self.smoke = smoke;
        self
    }

    /// Campfire preset
    pub fn campfire(name: impl Into<String>, position: [f32; 3]) -> Self {
        Self::new(name)
            .at(position[0], position[1], position[2])
            .with_type(FireType::Campfire)
            .with_scale([0.5, 1.0, 0.5])
            .with_intensity(1.0)
    }

    /// Torch preset
    pub fn torch(name: impl Into<String>, position: [f32; 3]) -> Self {
        Self::new(name)
            .at(position[0], position[1], position[2])
            .with_type(FireType::Torch)
            .with_scale([0.15, 0.4, 0.15])
            .with_intensity(0.7)
    }

    /// Bonfire preset
    pub fn bonfire(name: impl Into<String>, position: [f32; 3]) -> Self {
        Self::new(name)
            .at(position[0], position[1], position[2])
            .with_type(FireType::Campfire)
            .with_scale([2.0, 4.0, 2.0])
            .with_intensity(3.0)
    }

    /// Industrial fire preset
    pub fn industrial(name: impl Into<String>, position: [f32; 3]) -> Self {
        Self::new(name)
            .at(position[0], position[1], position[2])
            .with_type(FireType::Industrial)
            .with_temperature(1500.0)
            .with_colors(FireColorGradient::blue_flame())
    }
}

impl Default for FireSourceCreateInfo {
    fn default() -> Self {
        Self::new("Fire")
    }
}

/// Fire type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FireType {
    /// Campfire style
    #[default]
    Campfire   = 0,
    /// Torch/candle
    Torch      = 1,
    /// Forest fire
    Wildfire   = 2,
    /// Industrial/furnace
    Industrial = 3,
    /// Gas flame
    GasFlame   = 4,
    /// Magic fire
    Magic      = 5,
}

/// Fire color gradient
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FireColorGradient {
    /// Core color (hottest)
    pub core: [f32; 4],
    /// Inner color
    pub inner: [f32; 4],
    /// Outer color
    pub outer: [f32; 4],
    /// Tip color (coolest)
    pub tip: [f32; 4],
}

impl FireColorGradient {
    /// Creates new gradient
    pub const fn new(core: [f32; 4], inner: [f32; 4], outer: [f32; 4], tip: [f32; 4]) -> Self {
        Self {
            core,
            inner,
            outer,
            tip,
        }
    }

    /// Orange fire (default)
    pub const fn orange() -> Self {
        Self {
            core: [1.0, 1.0, 0.9, 1.0],
            inner: [1.0, 0.8, 0.2, 1.0],
            outer: [1.0, 0.3, 0.0, 1.0],
            tip: [0.5, 0.1, 0.0, 0.5],
        }
    }

    /// Blue flame
    pub const fn blue_flame() -> Self {
        Self {
            core: [0.9, 0.95, 1.0, 1.0],
            inner: [0.5, 0.7, 1.0, 1.0],
            outer: [0.1, 0.3, 1.0, 1.0],
            tip: [0.0, 0.1, 0.5, 0.5],
        }
    }

    /// Green magic fire
    pub const fn green_magic() -> Self {
        Self {
            core: [0.9, 1.0, 0.9, 1.0],
            inner: [0.3, 1.0, 0.3, 1.0],
            outer: [0.0, 0.7, 0.2, 1.0],
            tip: [0.0, 0.3, 0.1, 0.5],
        }
    }

    /// Purple magic fire
    pub const fn purple_magic() -> Self {
        Self {
            core: [1.0, 0.9, 1.0, 1.0],
            inner: [0.8, 0.3, 1.0, 1.0],
            outer: [0.5, 0.0, 0.8, 1.0],
            tip: [0.2, 0.0, 0.4, 0.5],
        }
    }
}

impl Default for FireColorGradient {
    fn default() -> Self {
        Self::orange()
    }
}

// ============================================================================
// Smoke Properties
// ============================================================================

/// Smoke properties
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SmokeProperties {
    /// Smoke color
    pub color: [f32; 4],
    /// Emission rate
    pub emission_rate: f32,
    /// Rise speed
    pub rise_speed: f32,
    /// Spread rate
    pub spread_rate: f32,
    /// Opacity
    pub opacity: f32,
    /// Lifetime (seconds)
    pub lifetime: f32,
}

impl SmokeProperties {
    /// Creates new properties
    pub const fn new() -> Self {
        Self {
            color: [0.2, 0.2, 0.2, 0.8],
            emission_rate: 1.0,
            rise_speed: 2.0,
            spread_rate: 0.5,
            opacity: 0.7,
            lifetime: 5.0,
        }
    }

    /// With color
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// With emission rate
    pub const fn with_rate(mut self, rate: f32) -> Self {
        self.emission_rate = rate;
        self
    }

    /// Light smoke
    pub const fn light() -> Self {
        Self {
            color: [0.4, 0.4, 0.4, 0.5],
            emission_rate: 0.5,
            rise_speed: 3.0,
            spread_rate: 0.8,
            opacity: 0.4,
            lifetime: 4.0,
        }
    }

    /// Dense smoke
    pub const fn dense() -> Self {
        Self {
            color: [0.1, 0.1, 0.1, 0.95],
            emission_rate: 2.0,
            rise_speed: 1.5,
            spread_rate: 0.3,
            opacity: 0.9,
            lifetime: 8.0,
        }
    }

    /// Steam
    pub const fn steam() -> Self {
        Self {
            color: [0.9, 0.9, 0.95, 0.4],
            emission_rate: 1.5,
            rise_speed: 4.0,
            spread_rate: 1.0,
            opacity: 0.3,
            lifetime: 2.0,
        }
    }
}

impl Default for SmokeProperties {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Smoke Volume
// ============================================================================

/// Smoke volume create info
#[derive(Clone, Debug)]
pub struct SmokeVolumeCreateInfo {
    /// Name
    pub name: String,
    /// Bounds
    pub bounds: SmokeBounds,
    /// Properties
    pub properties: SmokeProperties,
    /// Wind influence
    pub wind_influence: f32,
    /// Obstacle SDF
    pub obstacle_sdf: Option<u64>,
}

impl SmokeVolumeCreateInfo {
    /// Creates new volume
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            bounds: SmokeBounds::default(),
            properties: SmokeProperties::default(),
            wind_influence: 1.0,
            obstacle_sdf: None,
        }
    }

    /// With bounds
    pub fn with_bounds(mut self, bounds: SmokeBounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// With properties
    pub fn with_properties(mut self, properties: SmokeProperties) -> Self {
        self.properties = properties;
        self
    }

    /// With wind influence
    pub fn with_wind(mut self, influence: f32) -> Self {
        self.wind_influence = influence;
        self
    }
}

impl Default for SmokeVolumeCreateInfo {
    fn default() -> Self {
        Self::new("Smoke")
    }
}

/// Smoke bounds
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct SmokeBounds {
    /// Min corner
    pub min: [f32; 3],
    /// Max corner
    pub max: [f32; 3],
}

impl SmokeBounds {
    /// Creates new bounds
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    /// Default bounds
    pub const fn unit() -> Self {
        Self {
            min: [-5.0, 0.0, -5.0],
            max: [5.0, 20.0, 5.0],
        }
    }
}

impl Default for SmokeBounds {
    fn default() -> Self {
        Self::unit()
    }
}

// ============================================================================
// Explosion
// ============================================================================

/// Explosion create info
#[derive(Clone, Debug)]
pub struct ExplosionCreateInfo {
    /// Name
    pub name: String,
    /// Position
    pub position: [f32; 3],
    /// Explosion type
    pub explosion_type: ExplosionType,
    /// Radius
    pub radius: f32,
    /// Power
    pub power: f32,
    /// Duration (seconds)
    pub duration: f32,
    /// Fire color
    pub fire_color: FireColorGradient,
    /// Smoke properties
    pub smoke: SmokeProperties,
    /// Shockwave
    pub shockwave: bool,
}

impl ExplosionCreateInfo {
    /// Creates new explosion
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            position: [0.0; 3],
            explosion_type: ExplosionType::Fireball,
            radius: 5.0,
            power: 1.0,
            duration: 2.0,
            fire_color: FireColorGradient::orange(),
            smoke: SmokeProperties::dense(),
            shockwave: true,
        }
    }

    /// At position
    pub fn at(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With type
    pub fn with_type(mut self, explosion_type: ExplosionType) -> Self {
        self.explosion_type = explosion_type;
        self
    }

    /// With radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With power
    pub fn with_power(mut self, power: f32) -> Self {
        self.power = power;
        self
    }

    /// Small explosion preset
    pub fn small(name: impl Into<String>, position: [f32; 3]) -> Self {
        Self::new(name)
            .at(position[0], position[1], position[2])
            .with_radius(2.0)
            .with_power(0.5)
    }

    /// Large explosion preset
    pub fn large(name: impl Into<String>, position: [f32; 3]) -> Self {
        Self::new(name)
            .at(position[0], position[1], position[2])
            .with_radius(15.0)
            .with_power(3.0)
    }

    /// Nuclear preset
    pub fn nuclear(name: impl Into<String>, position: [f32; 3]) -> Self {
        Self::new(name)
            .at(position[0], position[1], position[2])
            .with_type(ExplosionType::Mushroom)
            .with_radius(100.0)
            .with_power(10.0)
    }
}

impl Default for ExplosionCreateInfo {
    fn default() -> Self {
        Self::new("Explosion")
    }
}

/// Explosion type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ExplosionType {
    /// Fireball
    #[default]
    Fireball = 0,
    /// Blast wave
    Blast    = 1,
    /// Mushroom cloud
    Mushroom = 2,
    /// Ground impact
    Ground   = 3,
    /// Spark burst
    Sparks   = 4,
}

// ============================================================================
// Embers and Sparks
// ============================================================================

/// Ember settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EmberSettings {
    /// Emission rate
    pub emission_rate: f32,
    /// Initial velocity
    pub velocity: f32,
    /// Velocity variation
    pub velocity_variation: f32,
    /// Lifetime (seconds)
    pub lifetime: f32,
    /// Size
    pub size: f32,
    /// Color
    pub color: [f32; 4],
    /// Gravity influence
    pub gravity: f32,
    /// Wind influence
    pub wind_influence: f32,
}

impl EmberSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            emission_rate: 100.0,
            velocity: 3.0,
            velocity_variation: 0.5,
            lifetime: 3.0,
            size: 0.02,
            color: [1.0, 0.5, 0.0, 1.0],
            gravity: 0.3,
            wind_influence: 0.8,
        }
    }

    /// With rate
    pub const fn with_rate(mut self, rate: f32) -> Self {
        self.emission_rate = rate;
        self
    }

    /// With color
    pub const fn with_color(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    /// Intense embers
    pub const fn intense() -> Self {
        Self {
            emission_rate: 500.0,
            velocity: 5.0,
            velocity_variation: 0.6,
            lifetime: 4.0,
            size: 0.025,
            color: [1.0, 0.6, 0.1, 1.0],
            gravity: 0.2,
            wind_influence: 1.0,
        }
    }

    /// Subtle embers
    pub const fn subtle() -> Self {
        Self {
            emission_rate: 30.0,
            velocity: 2.0,
            velocity_variation: 0.3,
            lifetime: 2.0,
            size: 0.015,
            color: [1.0, 0.4, 0.0, 0.8],
            gravity: 0.4,
            wind_influence: 0.5,
        }
    }
}

impl Default for EmberSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU fire cell
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuFireCell {
    /// Temperature
    pub temperature: f32,
    /// Fuel amount
    pub fuel: f32,
    /// Smoke density
    pub smoke: f32,
    /// Oxygen
    pub oxygen: f32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Heat flux
    pub heat_flux: f32,
}

/// GPU smoke particle
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuSmokeParticle {
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

/// GPU ember particle
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuEmberParticle {
    /// Position
    pub position: [f32; 3],
    /// Age
    pub age: f32,
    /// Velocity
    pub velocity: [f32; 3],
    /// Lifetime
    pub lifetime: f32,
    /// Color
    pub color: [f32; 4],
}

/// GPU fire smoke constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuFireSmokeConstants {
    /// Grid dimensions
    pub grid_dims: [u32; 3],
    /// Cell size
    pub cell_size: f32,
    /// Time
    pub time: f32,
    /// Delta time
    pub dt: f32,
    /// Buoyancy
    pub buoyancy: f32,
    /// Cooling rate
    pub cooling_rate: f32,
    /// Dissipation
    pub dissipation: f32,
    /// Turbulence
    pub turbulence: f32,
    /// Vorticity
    pub vorticity: f32,
    /// Combustion temperature
    pub combustion_temp: f32,
    /// Wind
    pub wind: [f32; 3],
    /// Ambient temperature
    pub ambient_temp: f32,
    /// Particle count
    pub particle_count: u32,
    /// Ember count
    pub ember_count: u32,
    /// Flags
    pub flags: u32,
    /// Pad
    pub _pad: f32,
}

impl Default for GpuFireSmokeConstants {
    fn default() -> Self {
        Self {
            grid_dims: [64, 128, 64],
            cell_size: 0.1,
            time: 0.0,
            dt: 0.016,
            buoyancy: 1.5,
            cooling_rate: 0.8,
            dissipation: 0.98,
            turbulence: 0.5,
            vorticity: 0.3,
            combustion_temp: 1273.0,
            wind: [0.0; 3],
            ambient_temp: 293.0,
            particle_count: 0,
            ember_count: 0,
            flags: 0,
            _pad: 0.0,
        }
    }
}

/// GPU fire render constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuFireRenderConstants {
    /// Camera position
    pub camera_position: [f32; 3],
    /// Ray march steps
    pub ray_march_steps: u32,
    /// Shadow steps
    pub shadow_steps: u32,
    /// Density scale
    pub density_scale: f32,
    /// Fire brightness
    pub fire_brightness: f32,
    /// Smoke absorption
    pub smoke_absorption: f32,
    /// Fire color core
    pub fire_core: [f32; 4],
    /// Fire color outer
    pub fire_outer: [f32; 4],
    /// Smoke color
    pub smoke_color: [f32; 4],
    /// Light direction
    pub light_dir: [f32; 3],
    /// Light intensity
    pub light_intensity: f32,
}

impl Default for GpuFireRenderConstants {
    fn default() -> Self {
        Self {
            camera_position: [0.0; 3],
            ray_march_steps: 64,
            shadow_steps: 16,
            density_scale: 1.0,
            fire_brightness: 2.0,
            smoke_absorption: 0.5,
            fire_core: [1.0, 1.0, 0.9, 1.0],
            fire_outer: [1.0, 0.3, 0.0, 1.0],
            smoke_color: [0.2, 0.2, 0.2, 0.8],
            light_dir: [0.0, 1.0, 0.0],
            light_intensity: 1.0,
        }
    }
}

// ============================================================================
// Fire Smoke Statistics
// ============================================================================

/// Fire smoke statistics
#[derive(Clone, Debug, Default)]
pub struct GpuFireSmokeStats {
    /// Active fire sources
    pub fire_sources: u32,
    /// Active smoke particles
    pub smoke_particles: u32,
    /// Active embers
    pub ember_particles: u32,
    /// Grid cells
    pub grid_cells: u32,
    /// Active fire cells
    pub active_fire_cells: u32,
    /// Simulation time (ms)
    pub sim_time_ms: f32,
    /// Render time (ms)
    pub render_time_ms: f32,
    /// Average temperature (K)
    pub avg_temperature: f32,
    /// Max temperature (K)
    pub max_temperature: f32,
    /// Total heat output (W)
    pub heat_output: f32,
}

impl GpuFireSmokeStats {
    /// Fire cell ratio
    pub fn fire_cell_ratio(&self) -> f32 {
        if self.grid_cells > 0 {
            self.active_fire_cells as f32 / self.grid_cells as f32
        } else {
            0.0
        }
    }

    /// Total particles
    pub fn total_particles(&self) -> u32 {
        self.smoke_particles + self.ember_particles
    }

    /// Total time (ms)
    pub fn total_time_ms(&self) -> f32 {
        self.sim_time_ms + self.render_time_ms
    }

    /// Average temperature in Celsius
    pub fn avg_temp_celsius(&self) -> f32 {
        self.avg_temperature - 273.15
    }
}
