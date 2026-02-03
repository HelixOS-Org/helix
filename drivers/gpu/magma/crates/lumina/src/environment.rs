//! Environment Mapping Types for Lumina
//!
//! This module provides environment mapping infrastructure including
//! reflection probes, irradiance maps, and specular IBL.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Environment Handles
// ============================================================================

/// Environment map handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EnvironmentMapHandle(pub u64);

impl EnvironmentMapHandle {
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

impl Default for EnvironmentMapHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Reflection probe handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ReflectionProbeHandle(pub u64);

impl ReflectionProbeHandle {
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

impl Default for ReflectionProbeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Irradiance volume handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct IrradianceVolumeHandle(pub u64);

impl IrradianceVolumeHandle {
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

impl Default for IrradianceVolumeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Environment Map
// ============================================================================

/// Environment map create info
#[derive(Clone, Debug)]
pub struct EnvironmentMapCreateInfo {
    /// Name
    pub name: String,
    /// Source type
    pub source: EnvironmentSource,
    /// Resolution (per face for cubemap)
    pub resolution: u32,
    /// HDR format
    pub hdr: bool,
    /// Generate irradiance map
    pub irradiance: bool,
    /// Irradiance resolution
    pub irradiance_resolution: u32,
    /// Generate specular map (prefiltered)
    pub specular: bool,
    /// Specular mip levels
    pub specular_mip_levels: u32,
    /// Generate BRDF LUT
    pub brdf_lut: bool,
}

impl EnvironmentMapCreateInfo {
    /// Creates info
    pub fn new(resolution: u32) -> Self {
        Self {
            name: String::new(),
            source: EnvironmentSource::Cubemap,
            resolution,
            hdr: true,
            irradiance: true,
            irradiance_resolution: 32,
            specular: true,
            specular_mip_levels: 8,
            brdf_lut: true,
        }
    }

    /// Low quality (fast)
    pub fn low_quality() -> Self {
        Self {
            resolution: 256,
            irradiance_resolution: 16,
            specular_mip_levels: 5,
            ..Self::new(256)
        }
    }

    /// Medium quality
    pub fn medium_quality() -> Self {
        Self::new(512)
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self {
            resolution: 1024,
            irradiance_resolution: 64,
            specular_mip_levels: 10,
            ..Self::new(1024)
        }
    }

    /// From HDRI
    pub fn from_hdri() -> Self {
        Self {
            source: EnvironmentSource::EquirectangularHdri,
            ..Self::high_quality()
        }
    }

    /// From procedural sky
    pub fn from_sky() -> Self {
        Self {
            source: EnvironmentSource::Procedural,
            ..Self::medium_quality()
        }
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// Without irradiance
    pub fn without_irradiance(mut self) -> Self {
        self.irradiance = false;
        self
    }

    /// Without specular
    pub fn without_specular(mut self) -> Self {
        self.specular = false;
        self
    }
}

impl Default for EnvironmentMapCreateInfo {
    fn default() -> Self {
        Self::medium_quality()
    }
}

/// Environment source type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum EnvironmentSource {
    /// Cubemap (6 faces)
    #[default]
    Cubemap = 0,
    /// Equirectangular HDRI
    EquirectangularHdri = 1,
    /// Dual-paraboloid
    DualParaboloid = 2,
    /// Procedural (sky model)
    Procedural = 3,
    /// Solid color
    SolidColor = 4,
    /// Gradient
    Gradient = 5,
}

// ============================================================================
// Reflection Probe
// ============================================================================

/// Reflection probe create info
#[derive(Clone, Debug)]
pub struct ReflectionProbeCreateInfo {
    /// Name
    pub name: String,
    /// Position
    pub position: [f32; 3],
    /// Influence radius
    pub radius: f32,
    /// Box projection extents (None for infinite)
    pub box_projection: Option<[f32; 3]>,
    /// Resolution
    pub resolution: u32,
    /// Update mode
    pub update_mode: ProbeUpdateMode,
    /// Priority (higher = more important)
    pub priority: i32,
    /// Blend distance (for smooth transitions)
    pub blend_distance: f32,
    /// HDR
    pub hdr: bool,
}

impl ReflectionProbeCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            position: [0.0, 0.0, 0.0],
            radius: 10.0,
            box_projection: None,
            resolution: 256,
            update_mode: ProbeUpdateMode::Baked,
            priority: 0,
            blend_distance: 1.0,
            hdr: true,
        }
    }

    /// At position
    pub fn at(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
            ..Self::new()
        }
    }

    /// Baked (static)
    pub fn baked(position: [f32; 3], radius: f32) -> Self {
        Self {
            position,
            radius,
            update_mode: ProbeUpdateMode::Baked,
            ..Self::new()
        }
    }

    /// Realtime
    pub fn realtime(position: [f32; 3], radius: f32) -> Self {
        Self {
            position,
            radius,
            update_mode: ProbeUpdateMode::EveryFrame,
            resolution: 128, // Lower for realtime
            ..Self::new()
        }
    }

    /// Interior probe (box projection)
    pub fn interior(position: [f32; 3], extents: [f32; 3]) -> Self {
        Self {
            position,
            box_projection: Some(extents),
            radius: extents[0].max(extents[1]).max(extents[2]),
            ..Self::new()
        }
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

impl Default for ReflectionProbeCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Probe update mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProbeUpdateMode {
    /// Baked (static)
    #[default]
    Baked = 0,
    /// Update every frame
    EveryFrame = 1,
    /// Update on demand
    OnDemand = 2,
    /// Update one face per frame
    OneFacePerFrame = 3,
    /// Time sliced (one face every N frames)
    TimeSliced = 4,
}

impl ProbeUpdateMode {
    /// Is dynamic
    pub const fn is_dynamic(&self) -> bool {
        !matches!(self, Self::Baked)
    }

    /// Frames to complete update
    pub const fn frames_per_update(&self) -> u32 {
        match self {
            Self::Baked | Self::OnDemand => 1,
            Self::EveryFrame => 1,
            Self::OneFacePerFrame => 6,
            Self::TimeSliced => 12,
        }
    }
}

/// Reflection probe GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ReflectionProbeGpuData {
    /// Position
    pub position: [f32; 3],
    /// Radius
    pub radius: f32,
    /// Box min (for box projection)
    pub box_min: [f32; 3],
    /// Blend distance
    pub blend_distance: f32,
    /// Box max (for box projection)
    pub box_max: [f32; 3],
    /// Cubemap index
    pub cubemap_index: u32,
    /// Intensity
    pub intensity: f32,
    /// Use box projection
    pub use_box_projection: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Padding
    pub _padding: f32,
}

// ============================================================================
// Irradiance Volume
// ============================================================================

/// Irradiance volume create info
#[derive(Clone, Debug)]
pub struct IrradianceVolumeCreateInfo {
    /// Name
    pub name: String,
    /// Bounds min
    pub bounds_min: [f32; 3],
    /// Bounds max
    pub bounds_max: [f32; 3],
    /// Probe count per axis
    pub probe_count: [u32; 3],
    /// SH order (1 or 2)
    pub sh_order: u32,
    /// Update mode
    pub update_mode: ProbeUpdateMode,
    /// Visibility testing
    pub visibility: bool,
}

impl IrradianceVolumeCreateInfo {
    /// Creates info
    pub fn new(bounds_min: [f32; 3], bounds_max: [f32; 3], probe_count: [u32; 3]) -> Self {
        Self {
            name: String::new(),
            bounds_min,
            bounds_max,
            probe_count,
            sh_order: 2,
            update_mode: ProbeUpdateMode::Baked,
            visibility: true,
        }
    }

    /// From bounds and spacing
    pub fn from_spacing(bounds_min: [f32; 3], bounds_max: [f32; 3], spacing: f32) -> Self {
        let size = [
            bounds_max[0] - bounds_min[0],
            bounds_max[1] - bounds_min[1],
            bounds_max[2] - bounds_min[2],
        ];
        let probe_count = [
            ((size[0] / spacing).ceil() as u32).max(2),
            ((size[1] / spacing).ceil() as u32).max(2),
            ((size[2] / spacing).ceil() as u32).max(2),
        ];

        Self::new(bounds_min, bounds_max, probe_count)
    }

    /// Room volume
    pub fn room(center: [f32; 3], extents: [f32; 3], spacing: f32) -> Self {
        let bounds_min = [
            center[0] - extents[0],
            center[1] - extents[1],
            center[2] - extents[2],
        ];
        let bounds_max = [
            center[0] + extents[0],
            center[1] + extents[1],
            center[2] + extents[2],
        ];
        Self::from_spacing(bounds_min, bounds_max, spacing)
    }

    /// Total probe count
    pub fn total_probes(&self) -> u32 {
        self.probe_count[0] * self.probe_count[1] * self.probe_count[2]
    }

    /// Probe spacing
    pub fn spacing(&self) -> [f32; 3] {
        [
            (self.bounds_max[0] - self.bounds_min[0]) / (self.probe_count[0] - 1).max(1) as f32,
            (self.bounds_max[1] - self.bounds_min[1]) / (self.probe_count[1] - 1).max(1) as f32,
            (self.bounds_max[2] - self.bounds_min[2]) / (self.probe_count[2] - 1).max(1) as f32,
        ]
    }

    /// Memory size estimate (bytes)
    pub fn memory_size(&self) -> u64 {
        let probes = self.total_probes() as u64;
        let sh_coeffs = if self.sh_order == 1 { 4 } else { 9 };
        // 3 channels * 4 bytes * sh_coeffs per probe
        probes * 3 * 4 * sh_coeffs
    }
}

impl Default for IrradianceVolumeCreateInfo {
    fn default() -> Self {
        Self::new([-10.0, 0.0, -10.0], [10.0, 5.0, 10.0], [5, 3, 5])
    }
}

/// Irradiance volume GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct IrradianceVolumeGpuData {
    /// Bounds min
    pub bounds_min: [f32; 3],
    /// Probe count X
    pub probe_count_x: u32,
    /// Bounds max
    pub bounds_max: [f32; 3],
    /// Probe count Y
    pub probe_count_y: u32,
    /// Spacing
    pub spacing: [f32; 3],
    /// Probe count Z
    pub probe_count_z: u32,
    /// SH texture index
    pub sh_texture_index: u32,
    /// Visibility texture index
    pub visibility_texture_index: u32,
    /// Padding
    pub _padding: [u32; 2],
}

// ============================================================================
// BRDF Integration
// ============================================================================

/// BRDF LUT create info
#[derive(Clone, Copy, Debug)]
pub struct BrdfLutCreateInfo {
    /// Resolution
    pub resolution: u32,
    /// Sample count
    pub sample_count: u32,
    /// BRDF type
    pub brdf_type: BrdfType,
}

impl BrdfLutCreateInfo {
    /// Creates info
    pub fn new(resolution: u32) -> Self {
        Self {
            resolution,
            sample_count: 1024,
            brdf_type: BrdfType::GgxSmith,
        }
    }

    /// Low quality
    pub fn low_quality() -> Self {
        Self {
            resolution: 128,
            sample_count: 256,
            brdf_type: BrdfType::GgxSmith,
        }
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self {
            resolution: 512,
            sample_count: 4096,
            brdf_type: BrdfType::GgxSmith,
        }
    }
}

impl Default for BrdfLutCreateInfo {
    fn default() -> Self {
        Self::new(256)
    }
}

/// BRDF type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BrdfType {
    /// GGX with Smith geometry
    #[default]
    GgxSmith = 0,
    /// GGX with height-correlated Smith
    GgxHeightCorrelated = 1,
    /// Beckmann
    Beckmann = 2,
    /// Blinn-Phong
    BlinnPhong = 3,
}

// ============================================================================
// Spherical Harmonics
// ============================================================================

/// Spherical harmonics (L2)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SphericalHarmonicsL2 {
    /// Coefficients (9 per channel, 27 total)
    pub coefficients: [[f32; 3]; 9],
}

impl SphericalHarmonicsL2 {
    /// Creates empty SH
    pub const fn new() -> Self {
        Self {
            coefficients: [[0.0; 3]; 9],
        }
    }

    /// From ambient color
    pub fn from_ambient(color: [f32; 3]) -> Self {
        let mut sh = Self::new();
        // L0 band (constant term)
        let factor = 0.282095; // Y_0^0
        sh.coefficients[0] = [
            color[0] / factor,
            color[1] / factor,
            color[2] / factor,
        ];
        sh
    }

    /// Evaluate at direction
    pub fn evaluate(&self, direction: [f32; 3]) -> [f32; 3] {
        let x = direction[0];
        let y = direction[1];
        let z = direction[2];

        // SH basis functions
        let basis = [
            0.282095,                       // Y_0^0
            0.488603 * y,                   // Y_1^-1
            0.488603 * z,                   // Y_1^0
            0.488603 * x,                   // Y_1^1
            1.092548 * x * y,               // Y_2^-2
            1.092548 * y * z,               // Y_2^-1
            0.315392 * (3.0 * z * z - 1.0), // Y_2^0
            1.092548 * x * z,               // Y_2^1
            0.546274 * (x * x - y * y),     // Y_2^2
        ];

        let mut result = [0.0f32; 3];
        for (i, b) in basis.iter().enumerate() {
            result[0] += self.coefficients[i][0] * b;
            result[1] += self.coefficients[i][1] * b;
            result[2] += self.coefficients[i][2] * b;
        }
        result
    }

    /// Add sample
    pub fn add_sample(&mut self, direction: [f32; 3], color: [f32; 3], weight: f32) {
        let x = direction[0];
        let y = direction[1];
        let z = direction[2];

        let basis = [
            0.282095,
            0.488603 * y,
            0.488603 * z,
            0.488603 * x,
            1.092548 * x * y,
            1.092548 * y * z,
            0.315392 * (3.0 * z * z - 1.0),
            1.092548 * x * z,
            0.546274 * (x * x - y * y),
        ];

        for (i, b) in basis.iter().enumerate() {
            let w = weight * b;
            self.coefficients[i][0] += color[0] * w;
            self.coefficients[i][1] += color[1] * w;
            self.coefficients[i][2] += color[2] * w;
        }
    }

    /// Scale
    pub fn scale(&mut self, factor: f32) {
        for coef in &mut self.coefficients {
            coef[0] *= factor;
            coef[1] *= factor;
            coef[2] *= factor;
        }
    }

    /// Convolve with cosine lobe (for irradiance)
    pub fn convolve_cosine(&mut self) {
        // Convolution factors for cosine lobe
        let factors = [
            3.141593,  // L0
            2.094395,  // L1
            2.094395,
            2.094395,
            0.785398,  // L2
            0.785398,
            0.785398,
            0.785398,
            0.785398,
        ];

        for (i, f) in factors.iter().enumerate() {
            self.coefficients[i][0] *= f;
            self.coefficients[i][1] *= f;
            self.coefficients[i][2] *= f;
        }
    }
}

// ============================================================================
// Prefiltered Environment Map
// ============================================================================

/// Prefiltered environment settings
#[derive(Clone, Copy, Debug)]
pub struct PrefilteredEnvSettings {
    /// Resolution per face
    pub resolution: u32,
    /// Mip levels to generate
    pub mip_levels: u32,
    /// Samples per mip
    pub samples: u32,
    /// Roughness for each mip
    pub roughness_per_mip: bool,
}

impl PrefilteredEnvSettings {
    /// Creates settings
    pub fn new(resolution: u32) -> Self {
        let mip_levels = (resolution as f32).log2() as u32 + 1;
        Self {
            resolution,
            mip_levels,
            samples: 1024,
            roughness_per_mip: true,
        }
    }

    /// Low quality
    pub fn low_quality() -> Self {
        Self {
            resolution: 128,
            mip_levels: 5,
            samples: 256,
            roughness_per_mip: true,
        }
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self {
            resolution: 512,
            mip_levels: 9,
            samples: 4096,
            roughness_per_mip: true,
        }
    }

    /// Roughness at mip level
    pub fn roughness_at_mip(&self, mip: u32) -> f32 {
        if self.roughness_per_mip && self.mip_levels > 1 {
            mip as f32 / (self.mip_levels - 1) as f32
        } else {
            0.0
        }
    }
}

impl Default for PrefilteredEnvSettings {
    fn default() -> Self {
        Self::new(256)
    }
}

// ============================================================================
// Environment Blending
// ============================================================================

/// Environment blend info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct EnvironmentBlendInfo {
    /// Probe indices (up to 4)
    pub probe_indices: [u32; 4],
    /// Blend weights
    pub weights: [f32; 4],
    /// Number of active probes
    pub probe_count: u32,
}

impl EnvironmentBlendInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            probe_indices: [0; 4],
            weights: [0.0; 4],
            probe_count: 0,
        }
    }

    /// Single probe
    pub fn single(probe_index: u32) -> Self {
        Self {
            probe_indices: [probe_index, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
            probe_count: 1,
        }
    }

    /// Two probe blend
    pub fn blend_two(probe_a: u32, probe_b: u32, weight: f32) -> Self {
        Self {
            probe_indices: [probe_a, probe_b, 0, 0],
            weights: [1.0 - weight, weight, 0.0, 0.0],
            probe_count: 2,
        }
    }

    /// Add probe
    pub fn add(&mut self, probe_index: u32, weight: f32) {
        if self.probe_count < 4 {
            let idx = self.probe_count as usize;
            self.probe_indices[idx] = probe_index;
            self.weights[idx] = weight;
            self.probe_count += 1;
        }
    }

    /// Normalize weights
    pub fn normalize(&mut self) {
        let sum: f32 = self.weights[..self.probe_count as usize].iter().sum();
        if sum > 0.0 {
            for w in &mut self.weights[..self.probe_count as usize] {
                *w /= sum;
            }
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Environment statistics
#[derive(Clone, Debug, Default)]
pub struct EnvironmentStats {
    /// Environment maps
    pub environment_count: u32,
    /// Reflection probes
    pub probe_count: u32,
    /// Irradiance volumes
    pub volume_count: u32,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Update time (microseconds)
    pub update_time_us: u64,
}
