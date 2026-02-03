//! Lighting System for Lumina
//!
//! This module provides comprehensive lighting types including
//! directional, point, spot, area lights, shadows, and environment lighting.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Light Handle
// ============================================================================

/// Light handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LightHandle(pub u64);

impl LightHandle {
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

impl Default for LightHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Light Type
// ============================================================================

/// Light type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum LightType {
    /// Directional light (sun)
    Directional = 0,
    /// Point light (omnidirectional)
    Point = 1,
    /// Spot light (cone)
    Spot = 2,
    /// Area light (rect)
    AreaRect = 3,
    /// Area light (disc)
    AreaDisc = 4,
    /// Area light (tube)
    AreaTube = 5,
    /// Sky light (ambient/IBL)
    Sky = 6,
}

impl LightType {
    /// Has position
    pub const fn has_position(&self) -> bool {
        !matches!(self, Self::Directional | Self::Sky)
    }

    /// Has direction
    pub const fn has_direction(&self) -> bool {
        matches!(self, Self::Directional | Self::Spot)
    }

    /// Has range
    pub const fn has_range(&self) -> bool {
        matches!(
            self,
            Self::Point | Self::Spot | Self::AreaRect | Self::AreaDisc | Self::AreaTube
        )
    }

    /// Is area light
    pub const fn is_area_light(&self) -> bool {
        matches!(self, Self::AreaRect | Self::AreaDisc | Self::AreaTube)
    }

    /// Can cast shadows
    pub const fn can_cast_shadows(&self) -> bool {
        !matches!(self, Self::Sky)
    }
}

// ============================================================================
// Light Create Info
// ============================================================================

/// Light create info
#[derive(Clone, Debug)]
pub struct LightCreateInfo {
    /// Light type
    pub light_type: LightType,
    /// Color (linear RGB)
    pub color: [f32; 3],
    /// Intensity (lumens for point/spot, lux for directional)
    pub intensity: f32,
    /// Range (for point/spot/area)
    pub range: f32,
    /// Inner cone angle (for spot, radians)
    pub inner_cone_angle: f32,
    /// Outer cone angle (for spot, radians)
    pub outer_cone_angle: f32,
    /// Area size (for area lights)
    pub area_size: [f32; 2],
    /// Shadow settings
    pub shadow: Option<ShadowSettings>,
    /// Light flags
    pub flags: LightFlags,
}

impl LightCreateInfo {
    /// Creates directional light
    pub fn directional(color: [f32; 3], intensity: f32) -> Self {
        Self {
            light_type: LightType::Directional,
            color,
            intensity,
            range: f32::INFINITY,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            area_size: [0.0, 0.0],
            shadow: Some(ShadowSettings::directional()),
            flags: LightFlags::DEFAULT,
        }
    }

    /// Creates point light
    pub fn point(color: [f32; 3], intensity: f32, range: f32) -> Self {
        Self {
            light_type: LightType::Point,
            color,
            intensity,
            range,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            area_size: [0.0, 0.0],
            shadow: None,
            flags: LightFlags::DEFAULT,
        }
    }

    /// Creates spot light
    pub fn spot(color: [f32; 3], intensity: f32, range: f32, inner_angle: f32, outer_angle: f32) -> Self {
        Self {
            light_type: LightType::Spot,
            color,
            intensity,
            range,
            inner_cone_angle: inner_angle,
            outer_cone_angle: outer_angle,
            area_size: [0.0, 0.0],
            shadow: None,
            flags: LightFlags::DEFAULT,
        }
    }

    /// Creates area rect light
    pub fn area_rect(color: [f32; 3], intensity: f32, width: f32, height: f32, range: f32) -> Self {
        Self {
            light_type: LightType::AreaRect,
            color,
            intensity,
            range,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            area_size: [width, height],
            shadow: None,
            flags: LightFlags::DEFAULT,
        }
    }

    /// Creates area disc light
    pub fn area_disc(color: [f32; 3], intensity: f32, radius: f32, range: f32) -> Self {
        Self {
            light_type: LightType::AreaDisc,
            color,
            intensity,
            range,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            area_size: [radius, radius],
            shadow: None,
            flags: LightFlags::DEFAULT,
        }
    }

    /// Creates sky light
    pub fn sky(intensity: f32) -> Self {
        Self {
            light_type: LightType::Sky,
            color: [1.0, 1.0, 1.0],
            intensity,
            range: 0.0,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            area_size: [0.0, 0.0],
            shadow: None,
            flags: LightFlags::DEFAULT,
        }
    }

    /// With shadow
    pub fn with_shadow(mut self, settings: ShadowSettings) -> Self {
        self.shadow = Some(settings);
        self
    }

    /// Without shadow
    pub fn without_shadow(mut self) -> Self {
        self.shadow = None;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: LightFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Calculate effective radius (for culling)
    pub fn effective_radius(&self) -> f32 {
        match self.light_type {
            LightType::Directional | LightType::Sky => f32::INFINITY,
            _ => self.range,
        }
    }
}

// ============================================================================
// Light Flags
// ============================================================================

/// Light flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct LightFlags(pub u32);

impl LightFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Enabled
    pub const ENABLED: Self = Self(1 << 0);
    /// Affects specular
    pub const AFFECTS_SPECULAR: Self = Self(1 << 1);
    /// Affects diffuse
    pub const AFFECTS_DIFFUSE: Self = Self(1 << 2);
    /// Cast shadows
    pub const CAST_SHADOWS: Self = Self(1 << 3);
    /// Use temperature
    pub const USE_TEMPERATURE: Self = Self(1 << 4);
    /// Affects translucency
    pub const AFFECTS_TRANSLUCENCY: Self = Self(1 << 5);
    /// Affects volumetrics
    pub const AFFECTS_VOLUMETRICS: Self = Self(1 << 6);
    /// Use light function
    pub const USE_LIGHT_FUNCTION: Self = Self(1 << 7);
    /// Use IES profile
    pub const USE_IES_PROFILE: Self = Self(1 << 8);
    /// Default flags
    pub const DEFAULT: Self = Self(
        Self::ENABLED.0
            | Self::AFFECTS_SPECULAR.0
            | Self::AFFECTS_DIFFUSE.0
            | Self::CAST_SHADOWS.0
            | Self::AFFECTS_VOLUMETRICS.0,
    );

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for LightFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// ============================================================================
// Shadow Settings
// ============================================================================

/// Shadow settings
#[derive(Clone, Debug)]
pub struct ShadowSettings {
    /// Shadow map resolution
    pub resolution: u32,
    /// Shadow bias
    pub bias: f32,
    /// Normal bias
    pub normal_bias: f32,
    /// Shadow distance
    pub distance: f32,
    /// Cascade count (for directional)
    pub cascade_count: u32,
    /// Cascade split lambda
    pub cascade_split_lambda: f32,
    /// Shadow filter mode
    pub filter_mode: ShadowFilterMode,
    /// Shadow quality
    pub quality: ShadowQuality,
    /// Fade start (fraction of distance)
    pub fade_start: f32,
    /// Contact shadows
    pub contact_shadows: bool,
    /// Ray traced shadows
    pub ray_traced: bool,
}

impl ShadowSettings {
    /// Default for directional light
    pub fn directional() -> Self {
        Self {
            resolution: 2048,
            bias: 0.0001,
            normal_bias: 0.01,
            distance: 200.0,
            cascade_count: 4,
            cascade_split_lambda: 0.75,
            filter_mode: ShadowFilterMode::Pcf5x5,
            quality: ShadowQuality::High,
            fade_start: 0.9,
            contact_shadows: true,
            ray_traced: false,
        }
    }

    /// Default for point light
    pub fn point() -> Self {
        Self {
            resolution: 512,
            bias: 0.001,
            normal_bias: 0.02,
            distance: 50.0,
            cascade_count: 1,
            cascade_split_lambda: 0.0,
            filter_mode: ShadowFilterMode::Pcf3x3,
            quality: ShadowQuality::Medium,
            fade_start: 0.8,
            contact_shadows: false,
            ray_traced: false,
        }
    }

    /// Default for spot light
    pub fn spot() -> Self {
        Self {
            resolution: 1024,
            bias: 0.0005,
            normal_bias: 0.015,
            distance: 100.0,
            cascade_count: 1,
            cascade_split_lambda: 0.0,
            filter_mode: ShadowFilterMode::Pcf3x3,
            quality: ShadowQuality::High,
            fade_start: 0.85,
            contact_shadows: false,
            ray_traced: false,
        }
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// With cascade count
    pub fn with_cascades(mut self, count: u32) -> Self {
        self.cascade_count = count;
        self
    }

    /// With filter mode
    pub fn with_filter(mut self, mode: ShadowFilterMode) -> Self {
        self.filter_mode = mode;
        self
    }

    /// With ray tracing
    pub fn with_ray_tracing(mut self, enabled: bool) -> Self {
        self.ray_traced = enabled;
        self
    }
}

/// Shadow filter mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadowFilterMode {
    /// No filtering (hard shadows)
    None = 0,
    /// PCF 3x3
    Pcf3x3 = 1,
    /// PCF 5x5
    #[default]
    Pcf5x5 = 2,
    /// PCF 7x7
    Pcf7x7 = 3,
    /// PCSS (percentage closer soft shadows)
    Pcss = 4,
    /// VSM (variance shadow maps)
    Vsm = 5,
    /// ESM (exponential shadow maps)
    Esm = 6,
    /// MSM (moment shadow maps)
    Msm = 7,
}

/// Shadow quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadowQuality {
    /// Low quality
    Low = 0,
    /// Medium quality
    #[default]
    Medium = 1,
    /// High quality
    High = 2,
    /// Ultra quality
    Ultra = 3,
}

impl ShadowQuality {
    /// Get resolution multiplier
    pub const fn resolution_scale(&self) -> f32 {
        match self {
            Self::Low => 0.5,
            Self::Medium => 1.0,
            Self::High => 1.5,
            Self::Ultra => 2.0,
        }
    }

    /// Get sample count
    pub const fn sample_count(&self) -> u32 {
        match self {
            Self::Low => 4,
            Self::Medium => 8,
            Self::High => 16,
            Self::Ultra => 32,
        }
    }
}

// ============================================================================
// Light Data (GPU)
// ============================================================================

/// GPU light data structure
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct LightData {
    /// Position (world space, w = range)
    pub position_range: [f32; 4],
    /// Direction (w = cos outer angle for spot)
    pub direction_outer: [f32; 4],
    /// Color (w = intensity)
    pub color_intensity: [f32; 4],
    /// Packed data: type, inner angle, area width, area height
    pub packed: [f32; 4],
    /// Shadow matrix (for spot/directional)
    pub shadow_matrix: [[f32; 4]; 4],
    /// Shadow params: bias, normal_bias, near, far
    pub shadow_params: [f32; 4],
}

impl LightData {
    /// Creates from light info
    pub fn from_info(
        info: &LightCreateInfo,
        position: [f32; 3],
        direction: [f32; 3],
    ) -> Self {
        let shadow_bias = info
            .shadow
            .as_ref()
            .map(|s| s.bias)
            .unwrap_or(0.0);
        let normal_bias = info
            .shadow
            .as_ref()
            .map(|s| s.normal_bias)
            .unwrap_or(0.0);

        Self {
            position_range: [position[0], position[1], position[2], info.range],
            direction_outer: [
                direction[0],
                direction[1],
                direction[2],
                info.outer_cone_angle.cos(),
            ],
            color_intensity: [info.color[0], info.color[1], info.color[2], info.intensity],
            packed: [
                info.light_type as u32 as f32,
                info.inner_cone_angle.cos(),
                info.area_size[0],
                info.area_size[1],
            ],
            shadow_matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            shadow_params: [shadow_bias, normal_bias, 0.1, info.range],
        }
    }

    /// Creates null light data
    pub const fn null() -> Self {
        Self {
            position_range: [0.0, 0.0, 0.0, 0.0],
            direction_outer: [0.0, -1.0, 0.0, 1.0],
            color_intensity: [0.0, 0.0, 0.0, 0.0],
            packed: [0.0, 0.0, 0.0, 0.0],
            shadow_matrix: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]],
            shadow_params: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

impl Default for LightData {
    fn default() -> Self {
        Self::null()
    }
}

// ============================================================================
// Cascade Shadow Map Data
// ============================================================================

/// Cascade shadow map data
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct CascadeData {
    /// View projection matrices for each cascade
    pub matrices: [[[f32; 4]; 4]; 4],
    /// Split distances (z values in view space)
    pub split_depths: [f32; 4],
    /// Cascade count
    pub cascade_count: u32,
    /// Shadow distance
    pub shadow_distance: f32,
    /// Fade range
    pub fade_range: f32,
    /// Padding
    pub _padding: f32,
}

impl CascadeData {
    /// Creates default cascade data
    pub fn new(cascade_count: u32, shadow_distance: f32) -> Self {
        Self {
            matrices: [[[0.0; 4]; 4]; 4],
            split_depths: [0.0; 4],
            cascade_count,
            shadow_distance,
            fade_range: shadow_distance * 0.1,
            _padding: 0.0,
        }
    }

    /// Calculate cascade splits
    pub fn calculate_splits(&mut self, near: f32, far: f32, lambda: f32) {
        let range = far.min(self.shadow_distance);

        for i in 0..self.cascade_count as usize {
            let p = (i as f32 + 1.0) / self.cascade_count as f32;

            let log = near * (range / near).powf(p);
            let uniform = near + (range - near) * p;

            self.split_depths[i] = lambda * log + (1.0 - lambda) * uniform;
        }
    }
}

impl Default for CascadeData {
    fn default() -> Self {
        Self::new(4, 200.0)
    }
}

// ============================================================================
// Environment Light
// ============================================================================

/// Environment light (IBL)
#[derive(Clone, Debug)]
pub struct EnvironmentLight {
    /// Diffuse irradiance cubemap handle
    pub irradiance_map: u64,
    /// Specular prefiltered cubemap handle
    pub prefiltered_map: u64,
    /// BRDF LUT handle
    pub brdf_lut: u64,
    /// Intensity
    pub intensity: f32,
    /// Rotation (radians)
    pub rotation: f32,
    /// Enabled
    pub enabled: bool,
}

impl EnvironmentLight {
    /// Creates new environment light
    pub fn new() -> Self {
        Self {
            irradiance_map: 0,
            prefiltered_map: 0,
            brdf_lut: 0,
            intensity: 1.0,
            rotation: 0.0,
            enabled: true,
        }
    }

    /// With maps
    pub fn with_maps(mut self, irradiance: u64, prefiltered: u64, brdf_lut: u64) -> Self {
        self.irradiance_map = irradiance;
        self.prefiltered_map = prefiltered;
        self.brdf_lut = brdf_lut;
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

    /// Is valid
    pub fn is_valid(&self) -> bool {
        self.irradiance_map != 0 && self.prefiltered_map != 0 && self.brdf_lut != 0
    }
}

impl Default for EnvironmentLight {
    fn default() -> Self {
        Self::new()
    }
}

/// Environment light GPU data
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct EnvironmentLightData {
    /// Intensity
    pub intensity: f32,
    /// Rotation
    pub rotation: f32,
    /// Prefiltered map mip count
    pub prefiltered_mip_count: u32,
    /// Enabled
    pub enabled: u32,
}

impl Default for EnvironmentLightData {
    fn default() -> Self {
        Self {
            intensity: 1.0,
            rotation: 0.0,
            prefiltered_mip_count: 5,
            enabled: 0,
        }
    }
}

// ============================================================================
// Light Buffer
// ============================================================================

/// Light buffer for GPU
#[derive(Clone, Debug)]
pub struct LightBuffer {
    /// Directional lights
    pub directional_lights: Vec<LightData>,
    /// Point lights
    pub point_lights: Vec<LightData>,
    /// Spot lights
    pub spot_lights: Vec<LightData>,
    /// Area lights
    pub area_lights: Vec<LightData>,
    /// Max lights per type
    pub max_per_type: u32,
}

impl LightBuffer {
    /// Creates new light buffer
    pub fn new(max_per_type: u32) -> Self {
        Self {
            directional_lights: Vec::with_capacity(max_per_type as usize),
            point_lights: Vec::with_capacity(max_per_type as usize),
            spot_lights: Vec::with_capacity(max_per_type as usize),
            area_lights: Vec::with_capacity(max_per_type as usize),
            max_per_type,
        }
    }

    /// Clear all lights
    pub fn clear(&mut self) {
        self.directional_lights.clear();
        self.point_lights.clear();
        self.spot_lights.clear();
        self.area_lights.clear();
    }

    /// Add light
    pub fn add_light(&mut self, light: LightData, light_type: LightType) -> bool {
        let (vec, max) = match light_type {
            LightType::Directional => (&mut self.directional_lights, self.max_per_type),
            LightType::Point => (&mut self.point_lights, self.max_per_type),
            LightType::Spot => (&mut self.spot_lights, self.max_per_type),
            LightType::AreaRect | LightType::AreaDisc | LightType::AreaTube => {
                (&mut self.area_lights, self.max_per_type)
            }
            LightType::Sky => return false,
        };

        if vec.len() < max as usize {
            vec.push(light);
            true
        } else {
            false
        }
    }

    /// Total light count
    pub fn total_count(&self) -> usize {
        self.directional_lights.len()
            + self.point_lights.len()
            + self.spot_lights.len()
            + self.area_lights.len()
    }
}

impl Default for LightBuffer {
    fn default() -> Self {
        Self::new(16)
    }
}

/// Light counts for shader
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct LightCounts {
    /// Directional light count
    pub directional_count: u32,
    /// Point light count
    pub point_count: u32,
    /// Spot light count
    pub spot_count: u32,
    /// Area light count
    pub area_count: u32,
}

impl LightCounts {
    /// From buffer
    pub fn from_buffer(buffer: &LightBuffer) -> Self {
        Self {
            directional_count: buffer.directional_lights.len() as u32,
            point_count: buffer.point_lights.len() as u32,
            spot_count: buffer.spot_lights.len() as u32,
            area_count: buffer.area_lights.len() as u32,
        }
    }
}

// ============================================================================
// IES Profile
// ============================================================================

/// IES light profile
#[derive(Clone, Debug)]
pub struct IesProfile {
    /// Profile name
    pub name: String,
    /// Vertical angles (radians)
    pub vertical_angles: Vec<f32>,
    /// Horizontal angles (radians)
    pub horizontal_angles: Vec<f32>,
    /// Candela values (2D array)
    pub candela_values: Vec<f32>,
    /// Max candela value
    pub max_candela: f32,
}

impl IesProfile {
    /// Creates new profile
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            vertical_angles: Vec::new(),
            horizontal_angles: Vec::new(),
            candela_values: Vec::new(),
            max_candela: 1.0,
        }
    }

    /// Sample intensity at angle
    pub fn sample(&self, vertical: f32, horizontal: f32) -> f32 {
        // Simplified bilinear sampling
        if self.vertical_angles.is_empty() || self.horizontal_angles.is_empty() {
            return 1.0;
        }

        let v_idx = self.find_angle_index(&self.vertical_angles, vertical);
        let h_idx = self.find_angle_index(&self.horizontal_angles, horizontal);

        let h_count = self.horizontal_angles.len();
        let idx = v_idx * h_count + h_idx;

        if idx < self.candela_values.len() {
            self.candela_values[idx] / self.max_candela
        } else {
            1.0
        }
    }

    fn find_angle_index(&self, angles: &[f32], angle: f32) -> usize {
        for (i, &a) in angles.iter().enumerate() {
            if angle <= a {
                return i;
            }
        }
        angles.len().saturating_sub(1)
    }
}

// ============================================================================
// Light Temperature
// ============================================================================

/// Convert color temperature (Kelvin) to RGB
pub fn temperature_to_rgb(kelvin: f32) -> [f32; 3] {
    let temp = kelvin.clamp(1000.0, 40000.0) / 100.0;

    let r = if temp <= 66.0 {
        1.0
    } else {
        let r = 329.698727446 * (temp - 60.0).powf(-0.1332047592);
        (r / 255.0).clamp(0.0, 1.0)
    };

    let g = if temp <= 66.0 {
        let g = 99.4708025861 * temp.ln() - 161.1195681661;
        (g / 255.0).clamp(0.0, 1.0)
    } else {
        let g = 288.1221695283 * (temp - 60.0).powf(-0.0755148492);
        (g / 255.0).clamp(0.0, 1.0)
    };

    let b = if temp >= 66.0 {
        1.0
    } else if temp <= 19.0 {
        0.0
    } else {
        let b = 138.5177312231 * (temp - 10.0).ln() - 305.0447927307;
        (b / 255.0).clamp(0.0, 1.0)
    };

    [r, g, b]
}

/// Common light temperatures
pub mod temperatures {
    /// Candle (1900K)
    pub const CANDLE: f32 = 1900.0;
    /// Incandescent (2700K)
    pub const INCANDESCENT: f32 = 2700.0;
    /// Warm white (3000K)
    pub const WARM_WHITE: f32 = 3000.0;
    /// Neutral (4000K)
    pub const NEUTRAL: f32 = 4000.0;
    /// Cool white (5000K)
    pub const COOL_WHITE: f32 = 5000.0;
    /// Daylight (5500K)
    pub const DAYLIGHT: f32 = 5500.0;
    /// Noon sun (5780K)
    pub const NOON_SUN: f32 = 5780.0;
    /// Overcast sky (6500K)
    pub const OVERCAST: f32 = 6500.0;
    /// Blue sky (10000K)
    pub const BLUE_SKY: f32 = 10000.0;
}

// ============================================================================
// Light Attenuation
// ============================================================================

/// Calculate light attenuation
pub fn calculate_attenuation(distance: f32, range: f32) -> f32 {
    if distance >= range {
        return 0.0;
    }

    // Inverse square with smooth falloff
    let d = distance / range;
    let d2 = d * d;
    let d4 = d2 * d2;

    let smooth = (1.0 - d2).max(0.0);
    smooth * smooth / (1.0 + d4)
}

/// Calculate spot light attenuation
pub fn calculate_spot_attenuation(
    cos_angle: f32,
    cos_inner: f32,
    cos_outer: f32,
) -> f32 {
    if cos_angle <= cos_outer {
        return 0.0;
    }
    if cos_angle >= cos_inner {
        return 1.0;
    }

    let t = (cos_angle - cos_outer) / (cos_inner - cos_outer);
    t * t * (3.0 - 2.0 * t) // Smoothstep
}
