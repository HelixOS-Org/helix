//! Global Illumination Types for Lumina
//!
//! This module provides global illumination infrastructure including
//! probe-based GI, voxel GI, ray-traced GI, and light propagation volumes.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// GI Handles
// ============================================================================

/// Light probe handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LightProbeHandle(pub u64);

impl LightProbeHandle {
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

impl Default for LightProbeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Probe volume handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProbeVolumeHandle(pub u64);

impl ProbeVolumeHandle {
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

impl Default for ProbeVolumeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Voxel GI handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VoxelGiHandle(pub u64);

impl VoxelGiHandle {
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

impl Default for VoxelGiHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Ray-traced GI handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RtgiHandle(pub u64);

impl RtgiHandle {
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

impl Default for RtgiHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// GI Settings
// ============================================================================

/// Global illumination settings
#[derive(Clone, Debug)]
pub struct GiSettings {
    /// GI method
    pub method: GiMethod,
    /// Intensity multiplier
    pub intensity: f32,
    /// Indirect bounces
    pub bounces: u32,
    /// Quality preset
    pub quality: GiQuality,
    /// Enable dynamic GI
    pub dynamic: bool,
    /// Update frequency (frames)
    pub update_frequency: u32,
}

impl GiSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            method: GiMethod::ProbeGrid,
            intensity: 1.0,
            bounces: 2,
            quality: GiQuality::Medium,
            dynamic: false,
            update_frequency: 1,
        }
    }

    /// Baked GI
    pub fn baked() -> Self {
        Self {
            dynamic: false,
            bounces: 3,
            quality: GiQuality::High,
            ..Self::new()
        }
    }

    /// Real-time GI
    pub fn realtime() -> Self {
        Self {
            dynamic: true,
            bounces: 1,
            quality: GiQuality::Low,
            ..Self::new()
        }
    }

    /// With method
    pub fn with_method(mut self, method: GiMethod) -> Self {
        self.method = method;
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With bounces
    pub fn with_bounces(mut self, bounces: u32) -> Self {
        self.bounces = bounces;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: GiQuality) -> Self {
        self.quality = quality;
        self
    }
}

impl Default for GiSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// GI method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GiMethod {
    /// Disabled
    None = 0,
    /// Screen-space (SSGI)
    ScreenSpace = 1,
    /// Light probe grid
    #[default]
    ProbeGrid = 2,
    /// Voxel cone tracing
    VoxelConeTracing = 3,
    /// Light propagation volumes
    Lpv = 4,
    /// Ray-traced GI
    RayTraced = 5,
    /// Hybrid (probe + SSGI)
    Hybrid = 6,
}

/// GI quality preset
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GiQuality {
    /// Low (fast)
    Low = 0,
    /// Medium
    #[default]
    Medium = 1,
    /// High
    High = 2,
    /// Ultra
    Ultra = 3,
}

impl GiQuality {
    /// Sample count
    pub fn sample_count(&self) -> u32 {
        match self {
            Self::Low => 8,
            Self::Medium => 16,
            Self::High => 32,
            Self::Ultra => 64,
        }
    }

    /// Resolution scale
    pub fn resolution_scale(&self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Medium => 0.5,
            Self::High => 1.0,
            Self::Ultra => 1.0,
        }
    }
}

// ============================================================================
// Light Probes
// ============================================================================

/// Light probe create info
#[derive(Clone, Debug)]
pub struct LightProbeCreateInfo {
    /// Position
    pub position: [f32; 3],
    /// Capture radius
    pub radius: f32,
    /// Resolution per face
    pub resolution: u32,
    /// Update mode
    pub update_mode: ProbeUpdateMode,
}

impl LightProbeCreateInfo {
    /// Creates info
    pub fn at(position: [f32; 3]) -> Self {
        Self {
            position,
            radius: 10.0,
            resolution: 64,
            update_mode: ProbeUpdateMode::Baked,
        }
    }

    /// With radius
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// Dynamic probe
    pub fn dynamic(mut self) -> Self {
        self.update_mode = ProbeUpdateMode::EveryFrame;
        self
    }
}

impl Default for LightProbeCreateInfo {
    fn default() -> Self {
        Self::at([0.0, 0.0, 0.0])
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
    /// Update periodically
    Periodic = 3,
}

/// Light probe data
#[derive(Clone, Debug)]
pub struct LightProbeData {
    /// Position
    pub position: [f32; 3],
    /// Influence radius
    pub radius: f32,
    /// Spherical harmonics coefficients (L2)
    pub sh_coefficients: [[f32; 4]; 9],
    /// Ambient cube (6 directions)
    pub ambient_cube: [[f32; 4]; 6],
    /// Validity
    pub valid: bool,
}

impl LightProbeData {
    /// Creates data
    pub fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            radius: 10.0,
            sh_coefficients: [[0.0; 4]; 9],
            ambient_cube: [[0.0; 4]; 6],
            valid: false,
        }
    }

    /// Sample ambient at direction
    pub fn sample_ambient(&self, direction: [f32; 3]) -> [f32; 3] {
        // Simple 6-direction sampling
        let mut result = [0.0f32; 3];
        let weights = [
            direction[0].max(0.0),    // +X
            (-direction[0]).max(0.0), // -X
            direction[1].max(0.0),    // +Y
            (-direction[1]).max(0.0), // -Y
            direction[2].max(0.0),    // +Z
            (-direction[2]).max(0.0), // -Z
        ];
        for (i, &w) in weights.iter().enumerate() {
            result[0] += self.ambient_cube[i][0] * w;
            result[1] += self.ambient_cube[i][1] * w;
            result[2] += self.ambient_cube[i][2] * w;
        }
        result
    }
}

impl Default for LightProbeData {
    fn default() -> Self {
        Self::new([0.0, 0.0, 0.0])
    }
}

/// Spherical harmonics (L2)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct SphericalHarmonicsL2 {
    /// Coefficients (9 RGB values)
    pub coefficients: [[f32; 4]; 9],
}

impl SphericalHarmonicsL2 {
    /// Creates SH
    pub fn new() -> Self {
        Self {
            coefficients: [[0.0; 4]; 9],
        }
    }

    /// From ambient color
    pub fn from_ambient(color: [f32; 3]) -> Self {
        let mut sh = Self::new();
        // L0 band (ambient)
        let l0_scale = 0.282095;
        sh.coefficients[0] = [color[0] * l0_scale, color[1] * l0_scale, color[2] * l0_scale, 1.0];
        sh
    }

    /// Add to SH from direction
    pub fn add_sample(&mut self, direction: [f32; 3], color: [f32; 3]) {
        let x = direction[0];
        let y = direction[1];
        let z = direction[2];

        // Basis functions
        let basis = [
            0.282095,                   // L0
            0.488603 * y,               // L1-1
            0.488603 * z,               // L10
            0.488603 * x,               // L11
            1.092548 * x * y,           // L2-2
            1.092548 * y * z,           // L2-1
            0.315392 * (3.0 * z * z - 1.0), // L20
            1.092548 * x * z,           // L21
            0.546274 * (x * x - y * y), // L22
        ];

        for (i, &b) in basis.iter().enumerate() {
            self.coefficients[i][0] += color[0] * b;
            self.coefficients[i][1] += color[1] * b;
            self.coefficients[i][2] += color[2] * b;
        }
    }
}

// ============================================================================
// Probe Volume
// ============================================================================

/// Probe volume create info
#[derive(Clone, Debug)]
pub struct ProbeVolumeCreateInfo {
    /// Bounds min
    pub bounds_min: [f32; 3],
    /// Bounds max
    pub bounds_max: [f32; 3],
    /// Probe density (probes per unit)
    pub density: f32,
    /// Resolution per axis
    pub resolution: [u32; 3],
    /// Irradiance format
    pub irradiance_format: ProbeFormat,
    /// Visibility format
    pub visibility_format: ProbeFormat,
}

impl ProbeVolumeCreateInfo {
    /// Creates info
    pub fn new(bounds_min: [f32; 3], bounds_max: [f32; 3]) -> Self {
        Self {
            bounds_min,
            bounds_max,
            density: 0.5,
            resolution: [8, 4, 8],
            irradiance_format: ProbeFormat::Octahedral8x8,
            visibility_format: ProbeFormat::Octahedral16x16,
        }
    }

    /// With resolution
    pub fn with_resolution(mut self, x: u32, y: u32, z: u32) -> Self {
        self.resolution = [x, y, z];
        self
    }

    /// With density
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// Probe count
    pub fn probe_count(&self) -> u32 {
        self.resolution[0] * self.resolution[1] * self.resolution[2]
    }

    /// Memory size (bytes)
    pub fn memory_size(&self) -> u64 {
        let probes = self.probe_count() as u64;
        let irradiance = self.irradiance_format.texels() as u64 * 4; // RGBA16F
        let visibility = self.visibility_format.texels() as u64 * 2; // RG16F
        probes * (irradiance + visibility)
    }
}

impl Default for ProbeVolumeCreateInfo {
    fn default() -> Self {
        Self::new([-10.0, -10.0, -10.0], [10.0, 10.0, 10.0])
    }
}

/// Probe format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProbeFormat {
    /// 6x6 octahedral
    Octahedral6x6 = 0,
    /// 8x8 octahedral
    #[default]
    Octahedral8x8 = 1,
    /// 16x16 octahedral
    Octahedral16x16 = 2,
    /// Spherical harmonics L1
    ShL1 = 3,
    /// Spherical harmonics L2
    ShL2 = 4,
}

impl ProbeFormat {
    /// Texels per probe
    pub fn texels(&self) -> u32 {
        match self {
            Self::Octahedral6x6 => 36,
            Self::Octahedral8x8 => 64,
            Self::Octahedral16x16 => 256,
            Self::ShL1 => 4,
            Self::ShL2 => 9,
        }
    }
}

/// Probe volume GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ProbeVolumeGpuData {
    /// Bounds min
    pub bounds_min: [f32; 4],
    /// Bounds max
    pub bounds_max: [f32; 4],
    /// Resolution + spacing
    pub resolution: [f32; 4],
    /// Inverse spacing
    pub inv_spacing: [f32; 4],
}

// ============================================================================
// Voxel GI
// ============================================================================

/// Voxel GI create info
#[derive(Clone, Debug)]
pub struct VoxelGiCreateInfo {
    /// Bounds min
    pub bounds_min: [f32; 3],
    /// Bounds max
    pub bounds_max: [f32; 3],
    /// Voxel resolution
    pub resolution: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Anisotropic voxels
    pub anisotropic: bool,
}

impl VoxelGiCreateInfo {
    /// Creates info
    pub fn new(resolution: u32) -> Self {
        Self {
            bounds_min: [-50.0, -50.0, -50.0],
            bounds_max: [50.0, 50.0, 50.0],
            resolution,
            mip_levels: 6,
            anisotropic: true,
        }
    }

    /// Standard quality
    pub fn standard() -> Self {
        Self::new(128)
    }

    /// High quality
    pub fn high() -> Self {
        Self::new(256)
    }

    /// With bounds
    pub fn with_bounds(mut self, min: [f32; 3], max: [f32; 3]) -> Self {
        self.bounds_min = min;
        self.bounds_max = max;
        self
    }

    /// Voxel size
    pub fn voxel_size(&self) -> f32 {
        let size = [
            self.bounds_max[0] - self.bounds_min[0],
            self.bounds_max[1] - self.bounds_min[1],
            self.bounds_max[2] - self.bounds_min[2],
        ];
        size[0].max(size[1]).max(size[2]) / self.resolution as f32
    }

    /// Memory size (bytes)
    pub fn memory_size(&self) -> u64 {
        let voxels = self.resolution as u64 * self.resolution as u64 * self.resolution as u64;
        let faces = if self.anisotropic { 6 } else { 1 };
        let mut total = 0u64;
        let mut mip_size = voxels;
        for _ in 0..self.mip_levels {
            total += mip_size * faces * 4; // RGBA8 per voxel
            mip_size /= 8; // Octree reduction
        }
        total
    }
}

impl Default for VoxelGiCreateInfo {
    fn default() -> Self {
        Self::standard()
    }
}

/// Voxel cone tracing settings
#[derive(Clone, Debug)]
pub struct ConeTracingSettings {
    /// Number of cones
    pub cone_count: u32,
    /// Cone angle (degrees)
    pub cone_angle: f32,
    /// Max trace distance
    pub max_distance: f32,
    /// Step multiplier
    pub step_multiplier: f32,
    /// Occlusion factor
    pub occlusion: f32,
}

impl ConeTracingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            cone_count: 6,
            cone_angle: 45.0,
            max_distance: 50.0,
            step_multiplier: 1.0,
            occlusion: 1.0,
        }
    }

    /// Diffuse cones
    pub fn diffuse() -> Self {
        Self {
            cone_count: 6,
            cone_angle: 60.0,
            ..Self::new()
        }
    }

    /// Specular cone
    pub fn specular() -> Self {
        Self {
            cone_count: 1,
            cone_angle: 5.0,
            ..Self::new()
        }
    }
}

impl Default for ConeTracingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Voxel GI GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VoxelGiGpuParams {
    /// Bounds min + voxel size
    pub bounds_min: [f32; 4],
    /// Bounds max + resolution
    pub bounds_max: [f32; 4],
    /// Trace params (cone count, angle, max dist, step)
    pub trace_params: [f32; 4],
    /// GI params (intensity, occlusion, mip bias, 0)
    pub gi_params: [f32; 4],
}

// ============================================================================
// Light Propagation Volumes
// ============================================================================

/// LPV create info
#[derive(Clone, Debug)]
pub struct LpvCreateInfo {
    /// Grid resolution
    pub resolution: u32,
    /// Grid bounds
    pub bounds: f32,
    /// Propagation steps
    pub propagation_steps: u32,
    /// Injection multiplier
    pub injection_multiplier: f32,
}

impl LpvCreateInfo {
    /// Creates info
    pub fn new(resolution: u32) -> Self {
        Self {
            resolution,
            bounds: 50.0,
            propagation_steps: 8,
            injection_multiplier: 1.0,
        }
    }

    /// Standard quality
    pub fn standard() -> Self {
        Self::new(32)
    }

    /// Cell size
    pub fn cell_size(&self) -> f32 {
        (self.bounds * 2.0) / self.resolution as f32
    }

    /// Memory size (bytes)
    pub fn memory_size(&self) -> u64 {
        // 3 grids (R, G, B) with SH coefficients
        let cells = self.resolution as u64 * self.resolution as u64 * self.resolution as u64;
        cells * 3 * 4 * 4 // 4 SH coefficients per channel, float4
    }
}

impl Default for LpvCreateInfo {
    fn default() -> Self {
        Self::standard()
    }
}

/// LPV injection settings
#[derive(Clone, Debug)]
pub struct LpvInjectionSettings {
    /// RSM resolution
    pub rsm_resolution: u32,
    /// Flux multiplier
    pub flux_multiplier: f32,
    /// Geometry injection
    pub inject_geometry: bool,
}

impl LpvInjectionSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            rsm_resolution: 256,
            flux_multiplier: 1.0,
            inject_geometry: true,
        }
    }
}

impl Default for LpvInjectionSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Ray-Traced GI
// ============================================================================

/// RTGI create info
#[derive(Clone, Debug)]
pub struct RtgiCreateInfo {
    /// Rays per pixel
    pub rays_per_pixel: u32,
    /// Max bounces
    pub max_bounces: u32,
    /// Max ray distance
    pub max_distance: f32,
    /// Temporal accumulation
    pub temporal: bool,
    /// Denoiser
    pub denoiser: RtgiDenoiser,
}

impl RtgiCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            rays_per_pixel: 1,
            max_bounces: 2,
            max_distance: 1000.0,
            temporal: true,
            denoiser: RtgiDenoiser::Svgf,
        }
    }

    /// High quality
    pub fn high_quality() -> Self {
        Self {
            rays_per_pixel: 4,
            max_bounces: 4,
            ..Self::new()
        }
    }

    /// Performance preset
    pub fn performance() -> Self {
        Self {
            rays_per_pixel: 1,
            max_bounces: 1,
            ..Self::new()
        }
    }

    /// With rays per pixel
    pub fn with_rays(mut self, rays: u32) -> Self {
        self.rays_per_pixel = rays;
        self
    }
}

impl Default for RtgiCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// RTGI denoiser
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum RtgiDenoiser {
    /// None
    None = 0,
    /// Temporal
    Temporal = 1,
    /// SVGF
    #[default]
    Svgf = 2,
    /// ReBLUR
    Reblur = 3,
    /// NRD
    Nrd = 4,
}

/// RTGI GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct RtgiGpuParams {
    /// Rays per pixel + bounces
    pub ray_params: [u32; 4],
    /// Max distance + intensity
    pub distance_params: [f32; 4],
    /// Temporal params
    pub temporal_params: [f32; 4],
}

// ============================================================================
// Ambient Occlusion
// ============================================================================

/// Ambient occlusion bake settings
#[derive(Clone, Debug)]
pub struct AoBakeSettings {
    /// Samples per texel
    pub samples: u32,
    /// Max ray distance
    pub max_distance: f32,
    /// Output resolution scale
    pub resolution_scale: f32,
    /// Use GPU
    pub gpu_accelerated: bool,
}

impl AoBakeSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            samples: 256,
            max_distance: 1.0,
            resolution_scale: 1.0,
            gpu_accelerated: true,
        }
    }

    /// Preview quality
    pub fn preview() -> Self {
        Self {
            samples: 32,
            ..Self::new()
        }
    }

    /// Production quality
    pub fn production() -> Self {
        Self {
            samples: 1024,
            ..Self::new()
        }
    }
}

impl Default for AoBakeSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Lightmap
// ============================================================================

/// Lightmap create info
#[derive(Clone, Debug)]
pub struct LightmapCreateInfo {
    /// Resolution
    pub resolution: u32,
    /// Texels per unit
    pub texels_per_unit: f32,
    /// Format
    pub format: LightmapFormat,
    /// Include directional
    pub directional: bool,
    /// Padding
    pub padding: u32,
}

impl LightmapCreateInfo {
    /// Creates info
    pub fn new(resolution: u32) -> Self {
        Self {
            resolution,
            texels_per_unit: 10.0,
            format: LightmapFormat::Rgbm,
            directional: false,
            padding: 2,
        }
    }

    /// With directional
    pub fn with_directional(mut self) -> Self {
        self.directional = true;
        self
    }

    /// Memory size (bytes)
    pub fn memory_size(&self) -> u64 {
        let texels = self.resolution as u64 * self.resolution as u64;
        let bpp = self.format.bytes_per_pixel() as u64;
        let directional = if self.directional { 2 } else { 1 };
        texels * bpp * directional
    }
}

impl Default for LightmapCreateInfo {
    fn default() -> Self {
        Self::new(1024)
    }
}

/// Lightmap format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LightmapFormat {
    /// RGBM (8-bit with multiplier)
    #[default]
    Rgbm = 0,
    /// RGBD (8-bit with divisor)
    Rgbd = 1,
    /// RGB9E5 (shared exponent)
    Rgb9e5 = 2,
    /// BC6H (compressed HDR)
    Bc6h = 3,
    /// RGBA16F
    Rgba16f = 4,
}

impl LightmapFormat {
    /// Bytes per pixel
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Rgbm | Self::Rgbd | Self::Rgb9e5 => 4,
            Self::Bc6h => 1, // Compressed
            Self::Rgba16f => 8,
        }
    }
}

/// Lightmap UV rect
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct LightmapUvRect {
    /// UV offset
    pub offset: [f32; 2],
    /// UV scale
    pub scale: [f32; 2],
    /// Lightmap index
    pub index: u32,
    /// Padding
    pub _padding: [u32; 3],
}

// ============================================================================
// Statistics
// ============================================================================

/// GI statistics
#[derive(Clone, Debug, Default)]
pub struct GiStats {
    /// Active probes
    pub probe_count: u32,
    /// Probes updated this frame
    pub probes_updated: u32,
    /// Voxels
    pub voxel_count: u64,
    /// Rays traced
    pub rays_traced: u64,
    /// GI GPU time (microseconds)
    pub gpu_time_us: u64,
    /// Memory usage (bytes)
    pub memory_bytes: u64,
}
