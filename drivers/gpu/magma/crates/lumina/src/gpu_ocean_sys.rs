//! GPU Ocean Rendering System for Lumina
//!
//! This module provides GPU-accelerated ocean and water rendering
//! with wave simulation, caustics, and underwater effects.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Ocean System Handles
// ============================================================================

/// GPU ocean system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuOceanSystemHandle(pub u64);

impl GpuOceanSystemHandle {
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

impl Default for GpuOceanSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Ocean surface handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OceanSurfaceHandle(pub u64);

impl OceanSurfaceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for OceanSurfaceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Wave spectrum handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct WaveSpectrumHandle(pub u64);

impl WaveSpectrumHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for WaveSpectrumHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Ocean foam handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct OceanFoamHandle(pub u64);

impl OceanFoamHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for OceanFoamHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Ocean System Creation
// ============================================================================

/// GPU ocean system create info
#[derive(Clone, Debug)]
pub struct GpuOceanSystemCreateInfo {
    /// Name
    pub name: String,
    /// FFT resolution
    pub fft_resolution: u32,
    /// Tile size in world units
    pub tile_size: f32,
    /// Max LOD levels
    pub max_lod_levels: u32,
    /// Features
    pub features: OceanFeatures,
    /// Quality level
    pub quality: OceanQuality,
}

impl GpuOceanSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            fft_resolution: 256,
            tile_size: 100.0,
            max_lod_levels: 8,
            features: OceanFeatures::all(),
            quality: OceanQuality::High,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With FFT resolution
    pub fn with_fft_resolution(mut self, resolution: u32) -> Self {
        self.fft_resolution = resolution.next_power_of_two();
        self
    }

    /// With tile size
    pub fn with_tile_size(mut self, size: f32) -> Self {
        self.tile_size = size;
        self
    }

    /// With max LOD levels
    pub fn with_max_lod(mut self, levels: u32) -> Self {
        self.max_lod_levels = levels;
        self
    }

    /// With features
    pub fn with_features(mut self, features: OceanFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: OceanQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Standard quality
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self::new()
            .with_fft_resolution(512)
            .with_quality(OceanQuality::Ultra)
    }

    /// Mobile quality
    pub fn mobile() -> Self {
        Self::new()
            .with_fft_resolution(128)
            .with_max_lod(4)
            .with_quality(OceanQuality::Low)
    }

    /// Simple (no FFT)
    pub fn simple() -> Self {
        Self::new()
            .with_fft_resolution(0)
            .with_quality(OceanQuality::Low)
    }
}

impl Default for GpuOceanSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Ocean features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct OceanFeatures: u32 {
        /// None
        const NONE = 0;
        /// FFT wave simulation
        const FFT_WAVES = 1 << 0;
        /// Foam generation
        const FOAM = 1 << 1;
        /// Subsurface scattering
        const SSS = 1 << 2;
        /// Reflections
        const REFLECTIONS = 1 << 3;
        /// Refractions
        const REFRACTIONS = 1 << 4;
        /// Caustics
        const CAUSTICS = 1 << 5;
        /// Underwater effects
        const UNDERWATER = 1 << 6;
        /// Tessellation
        const TESSELLATION = 1 << 7;
        /// Buoyancy physics
        const BUOYANCY = 1 << 8;
        /// Flow maps
        const FLOW_MAP = 1 << 9;
        /// All
        const ALL = 0x3FF;
    }
}

impl Default for OceanFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Ocean quality level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OceanQuality {
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
// Wave Spectrum
// ============================================================================

/// Wave spectrum create info
#[derive(Clone, Debug)]
pub struct WaveSpectrumCreateInfo {
    /// Name
    pub name: String,
    /// Spectrum type
    pub spectrum_type: WaveSpectrumType,
    /// Wind speed (m/s)
    pub wind_speed: f32,
    /// Wind direction (radians)
    pub wind_direction: f32,
    /// Fetch distance (km)
    pub fetch: f32,
    /// Depth (m, 0 = infinite)
    pub depth: f32,
    /// Scale
    pub scale: f32,
    /// Choppiness
    pub choppiness: f32,
}

impl WaveSpectrumCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            spectrum_type: WaveSpectrumType::Phillips,
            wind_speed: 10.0,
            wind_direction: 0.0,
            fetch: 500.0,
            depth: 0.0,
            scale: 1.0,
            choppiness: 1.5,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With spectrum type
    pub fn with_spectrum(mut self, spectrum: WaveSpectrumType) -> Self {
        self.spectrum_type = spectrum;
        self
    }

    /// With wind speed
    pub fn with_wind_speed(mut self, speed: f32) -> Self {
        self.wind_speed = speed;
        self
    }

    /// With wind direction
    pub fn with_wind_direction(mut self, direction: f32) -> Self {
        self.wind_direction = direction;
        self
    }

    /// With fetch
    pub fn with_fetch(mut self, fetch: f32) -> Self {
        self.fetch = fetch;
        self
    }

    /// With depth
    pub fn with_depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }

    /// With scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// With choppiness
    pub fn with_choppiness(mut self, choppiness: f32) -> Self {
        self.choppiness = choppiness;
        self
    }

    /// Calm sea preset
    pub fn calm() -> Self {
        Self::new()
            .with_name("CalmSea")
            .with_wind_speed(5.0)
            .with_choppiness(0.5)
    }

    /// Moderate sea preset
    pub fn moderate() -> Self {
        Self::new()
            .with_name("ModerateSea")
            .with_wind_speed(10.0)
            .with_choppiness(1.2)
    }

    /// Stormy sea preset
    pub fn stormy() -> Self {
        Self::new()
            .with_name("StormySea")
            .with_wind_speed(25.0)
            .with_choppiness(2.5)
    }

    /// Lake preset
    pub fn lake() -> Self {
        Self::new()
            .with_name("Lake")
            .with_wind_speed(3.0)
            .with_fetch(10.0)
            .with_choppiness(0.3)
    }

    /// River preset
    pub fn river() -> Self {
        Self::new()
            .with_name("River")
            .with_spectrum(WaveSpectrumType::Gerstner)
            .with_wind_speed(2.0)
            .with_choppiness(0.2)
    }
}

impl Default for WaveSpectrumCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Wave spectrum type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum WaveSpectrumType {
    /// Phillips spectrum
    #[default]
    Phillips = 0,
    /// JONSWAP spectrum
    Jonswap = 1,
    /// TMA spectrum (shallow water)
    Tma = 2,
    /// Pierson-Moskowitz spectrum
    PiersonMoskowitz = 3,
    /// Gerstner waves (analytical)
    Gerstner = 4,
    /// Custom
    Custom = 5,
}

// ============================================================================
// Ocean Surface
// ============================================================================

/// Ocean surface create info
#[derive(Clone, Debug)]
pub struct OceanSurfaceCreateInfo {
    /// Name
    pub name: String,
    /// Spectrum
    pub spectrum: WaveSpectrumHandle,
    /// Water color
    pub water_color: [f32; 3],
    /// Scatter color
    pub scatter_color: [f32; 3],
    /// Absorption color
    pub absorption_color: [f32; 3],
    /// Specular power
    pub specular_power: f32,
    /// Fresnel power
    pub fresnel_power: f32,
    /// Refraction strength
    pub refraction_strength: f32,
    /// Normal strength
    pub normal_strength: f32,
}

impl OceanSurfaceCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            spectrum: WaveSpectrumHandle::NULL,
            water_color: [0.02, 0.08, 0.15],
            scatter_color: [0.05, 0.2, 0.3],
            absorption_color: [0.5, 0.2, 0.05],
            specular_power: 1.0,
            fresnel_power: 5.0,
            refraction_strength: 0.1,
            normal_strength: 1.0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With spectrum
    pub fn with_spectrum(mut self, spectrum: WaveSpectrumHandle) -> Self {
        self.spectrum = spectrum;
        self
    }

    /// With water color
    pub fn with_water_color(mut self, color: [f32; 3]) -> Self {
        self.water_color = color;
        self
    }

    /// With scatter color
    pub fn with_scatter_color(mut self, color: [f32; 3]) -> Self {
        self.scatter_color = color;
        self
    }

    /// With absorption color
    pub fn with_absorption_color(mut self, color: [f32; 3]) -> Self {
        self.absorption_color = color;
        self
    }

    /// Deep ocean preset
    pub fn deep_ocean() -> Self {
        Self::new()
            .with_name("DeepOcean")
            .with_water_color([0.01, 0.04, 0.12])
            .with_scatter_color([0.02, 0.12, 0.25])
    }

    /// Tropical preset
    pub fn tropical() -> Self {
        Self::new()
            .with_name("TropicalOcean")
            .with_water_color([0.05, 0.2, 0.3])
            .with_scatter_color([0.1, 0.4, 0.5])
    }

    /// Murky preset
    pub fn murky() -> Self {
        Self::new()
            .with_name("MurkyWater")
            .with_water_color([0.08, 0.12, 0.1])
            .with_scatter_color([0.12, 0.15, 0.1])
            .with_absorption_color([0.6, 0.5, 0.3])
    }

    /// Arctic preset
    pub fn arctic() -> Self {
        Self::new()
            .with_name("ArcticOcean")
            .with_water_color([0.02, 0.06, 0.1])
            .with_scatter_color([0.1, 0.2, 0.3])
    }
}

impl Default for OceanSurfaceCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Foam Settings
// ============================================================================

/// Ocean foam create info
#[derive(Clone, Debug)]
pub struct OceanFoamCreateInfo {
    /// Name
    pub name: String,
    /// Foam threshold
    pub threshold: f32,
    /// Foam intensity
    pub intensity: f32,
    /// Foam fade speed
    pub fade_speed: f32,
    /// Foam texture scale
    pub texture_scale: f32,
    /// Foam color
    pub color: [f32; 4],
    /// Shoreline foam
    pub shoreline: ShorelineFoam,
}

impl OceanFoamCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            threshold: 0.3,
            intensity: 1.0,
            fade_speed: 1.0,
            texture_scale: 10.0,
            color: [1.0, 1.0, 1.0, 0.8],
            shoreline: ShorelineFoam::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With fade speed
    pub fn with_fade_speed(mut self, speed: f32) -> Self {
        self.fade_speed = speed;
        self
    }

    /// With shoreline foam
    pub fn with_shoreline(mut self, shoreline: ShorelineFoam) -> Self {
        self.shoreline = shoreline;
        self
    }

    /// Light foam preset
    pub fn light() -> Self {
        Self::new()
            .with_threshold(0.5)
            .with_intensity(0.5)
    }

    /// Heavy foam preset
    pub fn heavy() -> Self {
        Self::new()
            .with_threshold(0.2)
            .with_intensity(1.5)
    }
}

impl Default for OceanFoamCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Shoreline foam settings
#[derive(Clone, Copy, Debug)]
pub struct ShorelineFoam {
    /// Enabled
    pub enabled: bool,
    /// Distance
    pub distance: f32,
    /// Intensity
    pub intensity: f32,
    /// Speed
    pub speed: f32,
}

impl ShorelineFoam {
    /// Default
    pub const fn new() -> Self {
        Self {
            enabled: true,
            distance: 5.0,
            intensity: 1.0,
            speed: 1.0,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            distance: 0.0,
            intensity: 0.0,
            speed: 0.0,
        }
    }
}

impl Default for ShorelineFoam {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Caustics
// ============================================================================

/// Caustics settings
#[derive(Clone, Copy, Debug)]
pub struct CausticsSettings {
    /// Enabled
    pub enabled: bool,
    /// Intensity
    pub intensity: f32,
    /// Scale
    pub scale: f32,
    /// Speed
    pub speed: f32,
    /// Chromatic split
    pub chromatic: f32,
    /// Depth fade start
    pub depth_fade_start: f32,
    /// Depth fade end
    pub depth_fade_end: f32,
}

impl CausticsSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: true,
            intensity: 1.0,
            scale: 1.0,
            speed: 1.0,
            chromatic: 0.02,
            depth_fade_start: 0.0,
            depth_fade_end: 50.0,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            intensity: 0.0,
            scale: 0.0,
            speed: 0.0,
            chromatic: 0.0,
            depth_fade_start: 0.0,
            depth_fade_end: 0.0,
        }
    }

    /// Subtle
    pub const fn subtle() -> Self {
        Self {
            enabled: true,
            intensity: 0.5,
            scale: 1.0,
            speed: 0.5,
            chromatic: 0.01,
            depth_fade_start: 0.0,
            depth_fade_end: 30.0,
        }
    }

    /// Strong
    pub const fn strong() -> Self {
        Self {
            enabled: true,
            intensity: 2.0,
            scale: 1.5,
            speed: 1.0,
            chromatic: 0.04,
            depth_fade_start: 0.0,
            depth_fade_end: 80.0,
        }
    }

    /// With intensity
    pub const fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for CausticsSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Underwater Effects
// ============================================================================

/// Underwater effects settings
#[derive(Clone, Copy, Debug)]
pub struct UnderwaterEffects {
    /// Enabled
    pub enabled: bool,
    /// Fog density
    pub fog_density: f32,
    /// Fog color
    pub fog_color: [f32; 3],
    /// Light absorption
    pub absorption: [f32; 3],
    /// Distortion strength
    pub distortion: f32,
    /// God rays enabled
    pub god_rays: bool,
    /// God rays intensity
    pub god_rays_intensity: f32,
}

impl UnderwaterEffects {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: true,
            fog_density: 0.02,
            fog_color: [0.02, 0.08, 0.12],
            absorption: [0.5, 0.25, 0.1],
            distortion: 0.02,
            god_rays: true,
            god_rays_intensity: 0.5,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            fog_density: 0.0,
            fog_color: [0.0, 0.0, 0.0],
            absorption: [0.0, 0.0, 0.0],
            distortion: 0.0,
            god_rays: false,
            god_rays_intensity: 0.0,
        }
    }

    /// Clear water
    pub const fn clear() -> Self {
        Self {
            enabled: true,
            fog_density: 0.01,
            fog_color: [0.05, 0.15, 0.25],
            absorption: [0.3, 0.15, 0.05],
            distortion: 0.01,
            god_rays: true,
            god_rays_intensity: 0.8,
        }
    }

    /// Murky water
    pub const fn murky() -> Self {
        Self {
            enabled: true,
            fog_density: 0.1,
            fog_color: [0.05, 0.08, 0.05],
            absorption: [0.8, 0.6, 0.4],
            distortion: 0.03,
            god_rays: false,
            god_rays_intensity: 0.0,
        }
    }
}

impl Default for UnderwaterEffects {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Buoyancy
// ============================================================================

/// Buoyancy query request
#[derive(Clone, Copy, Debug)]
pub struct BuoyancyQuery {
    /// World position
    pub position: [f32; 3],
    /// Query radius
    pub radius: f32,
}

impl BuoyancyQuery {
    /// Creates new query
    pub const fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            radius: 0.1,
        }
    }

    /// With radius
    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }
}

/// Buoyancy query result
#[derive(Clone, Copy, Debug, Default)]
pub struct BuoyancyResult {
    /// Water height at position
    pub water_height: f32,
    /// Normal at position
    pub normal: [f32; 3],
    /// Displacement
    pub displacement: [f32; 3],
    /// Flow velocity
    pub flow_velocity: [f32; 2],
    /// Is underwater
    pub is_underwater: bool,
    /// Submerged ratio (0-1)
    pub submerged_ratio: f32,
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU ocean tile data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuOceanTile {
    /// World position
    pub world_position: [f32; 2],
    /// Tile size
    pub tile_size: f32,
    /// LOD level
    pub lod_level: u32,
    /// UV offset
    pub uv_offset: [f32; 2],
    /// UV scale
    pub uv_scale: f32,
    /// Morph factor
    pub morph_factor: f32,
}

/// GPU ocean constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuOceanConstants {
    /// Time
    pub time: f32,
    /// Wind speed
    pub wind_speed: f32,
    /// Wind direction
    pub wind_direction: [f32; 2],
    /// Water color
    pub water_color: [f32; 4],
    /// Scatter color
    pub scatter_color: [f32; 4],
    /// Absorption color
    pub absorption_color: [f32; 4],
    /// Specular power
    pub specular_power: f32,
    /// Fresnel power
    pub fresnel_power: f32,
    /// Choppiness
    pub choppiness: f32,
    /// Normal strength
    pub normal_strength: f32,
    /// Foam threshold
    pub foam_threshold: f32,
    /// Foam intensity
    pub foam_intensity: f32,
    /// Foam texture scale
    pub foam_texture_scale: f32,
    /// Refraction strength
    pub refraction_strength: f32,
    /// Tile size
    pub tile_size: f32,
    /// FFT resolution
    pub fft_resolution: u32,
    /// Padding
    pub _pad: [f32; 2],
}

/// GPU caustics constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCausticsConstants {
    /// Intensity
    pub intensity: f32,
    /// Scale
    pub scale: f32,
    /// Speed
    pub speed: f32,
    /// Chromatic
    pub chromatic: f32,
    /// Depth fade
    pub depth_fade: [f32; 2],
    /// Time
    pub time: f32,
    /// Padding
    pub _pad: f32,
}

/// GPU underwater constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuUnderwaterConstants {
    /// Fog density
    pub fog_density: f32,
    /// Distortion
    pub distortion: f32,
    /// God rays intensity
    pub god_rays_intensity: f32,
    /// Water surface height
    pub water_height: f32,
    /// Fog color
    pub fog_color: [f32; 4],
    /// Absorption
    pub absorption: [f32; 4],
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU ocean statistics
#[derive(Clone, Debug, Default)]
pub struct GpuOceanStats {
    /// Visible tiles
    pub visible_tiles: u32,
    /// Total triangles
    pub total_triangles: u32,
    /// FFT dispatch count
    pub fft_dispatches: u32,
    /// Buoyancy queries
    pub buoyancy_queries: u32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
    /// FFT time (ms)
    pub fft_time_ms: f32,
}

impl GpuOceanStats {
    /// Average triangles per tile
    pub fn triangles_per_tile(&self) -> u32 {
        if self.visible_tiles == 0 {
            0
        } else {
            self.total_triangles / self.visible_tiles
        }
    }
}
