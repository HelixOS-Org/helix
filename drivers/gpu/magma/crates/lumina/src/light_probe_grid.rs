//! Light Probe Grid Types for Lumina
//!
//! This module provides light probe grid management
//! for global illumination and indirect lighting.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Light Probe Grid Handles
// ============================================================================

/// Light probe grid handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LightProbeGridHandle(pub u64);

impl LightProbeGridHandle {
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

impl Default for LightProbeGridHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Probe data buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProbeDataBufferHandle(pub u64);

impl ProbeDataBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ProbeDataBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Probe ray buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProbeRayBufferHandle(pub u64);

impl ProbeRayBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ProbeRayBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Probe visibility buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ProbeVisibilityHandle(pub u64);

impl ProbeVisibilityHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ProbeVisibilityHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Light Probe Grid Creation
// ============================================================================

/// Light probe grid create info
#[derive(Clone, Debug)]
pub struct LightProbeGridCreateInfo {
    /// Name
    pub name: String,
    /// Grid type
    pub grid_type: ProbeGridType,
    /// Grid dimensions (x, y, z)
    pub dimensions: [u32; 3],
    /// World space bounds min
    pub bounds_min: [f32; 3],
    /// World space bounds max
    pub bounds_max: [f32; 3],
    /// Probe encoding
    pub encoding: ProbeEncoding,
    /// Update mode
    pub update_mode: ProbeUpdateMode,
    /// Rays per probe
    pub rays_per_probe: u32,
    /// Features
    pub features: ProbeGridFeatures,
}

impl LightProbeGridCreateInfo {
    /// Creates new info
    pub fn new(dimensions: [u32; 3], bounds_min: [f32; 3], bounds_max: [f32; 3]) -> Self {
        Self {
            name: String::new(),
            grid_type: ProbeGridType::Uniform,
            dimensions,
            bounds_min,
            bounds_max,
            encoding: ProbeEncoding::SphericalHarmonicsL2,
            update_mode: ProbeUpdateMode::RealTime,
            rays_per_probe: 256,
            features: ProbeGridFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With grid type
    pub fn with_grid_type(mut self, grid_type: ProbeGridType) -> Self {
        self.grid_type = grid_type;
        self
    }

    /// With encoding
    pub fn with_encoding(mut self, encoding: ProbeEncoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// With update mode
    pub fn with_update_mode(mut self, mode: ProbeUpdateMode) -> Self {
        self.update_mode = mode;
        self
    }

    /// With rays per probe
    pub fn with_rays(mut self, rays: u32) -> Self {
        self.rays_per_probe = rays;
        self
    }

    /// With features
    pub fn with_features(mut self, features: ProbeGridFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Room preset
    pub fn room(size: [f32; 3], resolution: u32) -> Self {
        let half_size = [size[0] / 2.0, size[1] / 2.0, size[2] / 2.0];
        let dims = [
            resolution,
            (resolution as f32 * size[1] / size[0]) as u32,
            resolution,
        ];

        Self::new(dims, [-half_size[0], 0.0, -half_size[2]], [
            half_size[0],
            size[1],
            half_size[2],
        ])
    }

    /// Outdoor preset (large, sparse)
    pub fn outdoor(area_size: f32, height: f32) -> Self {
        let half = area_size / 2.0;
        Self::new([16, 4, 16], [-half, 0.0, -half], [half, height, half])
            .with_update_mode(ProbeUpdateMode::TimeSliced)
    }

    /// DDGI preset
    pub fn ddgi(dimensions: [u32; 3], bounds_min: [f32; 3], bounds_max: [f32; 3]) -> Self {
        Self::new(dimensions, bounds_min, bounds_max)
            .with_grid_type(ProbeGridType::DDGI)
            .with_encoding(ProbeEncoding::OctahedralIrradiance)
            .with_rays(256)
            .with_features(ProbeGridFeatures::VISIBILITY | ProbeGridFeatures::RELOCATION)
    }

    /// Total probe count
    pub fn total_probes(&self) -> u32 {
        self.dimensions[0] * self.dimensions[1] * self.dimensions[2]
    }

    /// Probe spacing
    pub fn probe_spacing(&self) -> [f32; 3] {
        [
            (self.bounds_max[0] - self.bounds_min[0]) / (self.dimensions[0] - 1).max(1) as f32,
            (self.bounds_max[1] - self.bounds_min[1]) / (self.dimensions[1] - 1).max(1) as f32,
            (self.bounds_max[2] - self.bounds_min[2]) / (self.dimensions[2] - 1).max(1) as f32,
        ]
    }
}

impl Default for LightProbeGridCreateInfo {
    fn default() -> Self {
        Self::room([20.0, 5.0, 20.0], 8)
    }
}

/// Probe grid type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProbeGridType {
    /// Uniform grid
    #[default]
    Uniform     = 0,
    /// DDGI (Dynamic Diffuse GI)
    DDGI        = 1,
    /// Cascaded grid
    Cascaded    = 2,
    /// Adaptive grid
    Adaptive    = 3,
    /// Tetrahedral
    Tetrahedral = 4,
}

impl ProbeGridType {
    /// Display name
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Uniform => "Uniform Grid",
            Self::DDGI => "DDGI",
            Self::Cascaded => "Cascaded",
            Self::Adaptive => "Adaptive",
            Self::Tetrahedral => "Tetrahedral",
        }
    }

    /// Supports relocation
    pub const fn supports_relocation(&self) -> bool {
        matches!(self, Self::DDGI | Self::Adaptive)
    }
}

/// Probe encoding
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProbeEncoding {
    /// Spherical harmonics L1 (4 coefficients)
    SphericalHarmonicsL1 = 0,
    /// Spherical harmonics L2 (9 coefficients)
    #[default]
    SphericalHarmonicsL2 = 1,
    /// Spherical harmonics L3 (16 coefficients)
    SphericalHarmonicsL3 = 2,
    /// Octahedral irradiance (DDGI style)
    OctahedralIrradiance = 3,
    /// Cubemap
    Cubemap              = 4,
    /// Ambient cube (6 samples)
    AmbientCube          = 5,
}

impl ProbeEncoding {
    /// Coefficients per probe
    pub const fn coefficients(&self) -> u32 {
        match self {
            Self::SphericalHarmonicsL1 => 4,
            Self::SphericalHarmonicsL2 => 9,
            Self::SphericalHarmonicsL3 => 16,
            Self::OctahedralIrradiance => 0, // Texture-based
            Self::Cubemap => 6,
            Self::AmbientCube => 6,
        }
    }

    /// Memory per probe (bytes, RGB)
    pub const fn memory_per_probe(&self) -> u32 {
        match self {
            Self::SphericalHarmonicsL1 => 4 * 3 * 4, // 4 coeffs * RGB * float
            Self::SphericalHarmonicsL2 => 9 * 3 * 4, // 9 coeffs * RGB * float
            Self::SphericalHarmonicsL3 => 16 * 3 * 4, // 16 coeffs * RGB * float
            Self::OctahedralIrradiance => 8 * 8 * 4, // 8x8 octahedral * RGBA
            Self::Cubemap => 6 * 4 * 4,              // 6 faces * RGBA * float
            Self::AmbientCube => 6 * 3 * 4,          // 6 directions * RGB * float
        }
    }
}

/// Probe update mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProbeUpdateMode {
    /// Baked (static)
    Baked      = 0,
    /// Real-time update
    #[default]
    RealTime   = 1,
    /// Time-sliced (spread across frames)
    TimeSliced = 2,
    /// On-demand
    OnDemand   = 3,
}

bitflags::bitflags! {
    /// Probe grid features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ProbeGridFeatures: u32 {
        /// None
        const NONE = 0;
        /// Visibility/depth data
        const VISIBILITY = 1 << 0;
        /// Probe relocation
        const RELOCATION = 1 << 1;
        /// Probe classification
        const CLASSIFICATION = 1 << 2;
        /// Multi-bounce
        const MULTI_BOUNCE = 1 << 3;
        /// Scroll/infinite
        const SCROLLING = 1 << 4;
        /// Ray-traced updates
        const RAY_TRACED = 1 << 5;
        /// Temporal blending
        const TEMPORAL = 1 << 6;
    }
}

// ============================================================================
// Probe Data
// ============================================================================

/// Single probe data (SH L2)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ProbeDataSH {
    /// SH coefficients (L0, L1, L2 for RGB)
    pub coefficients: [[f32; 3]; 9],
}

impl ProbeDataSH {
    /// Evaluate SH at direction
    pub fn evaluate(&self, direction: [f32; 3]) -> [f32; 3] {
        let x = direction[0];
        let y = direction[1];
        let z = direction[2];

        // SH basis functions
        let y0 = 0.282095;
        let y1 = 0.488603 * y;
        let y2 = 0.488603 * z;
        let y3 = 0.488603 * x;
        let y4 = 1.092548 * x * y;
        let y5 = 1.092548 * y * z;
        let y6 = 0.315392 * (3.0 * z * z - 1.0);
        let y7 = 1.092548 * x * z;
        let y8 = 0.546274 * (x * x - y * y);

        let basis = [y0, y1, y2, y3, y4, y5, y6, y7, y8];

        let mut result = [0.0; 3];
        for i in 0..9 {
            result[0] += self.coefficients[i][0] * basis[i];
            result[1] += self.coefficients[i][1] * basis[i];
            result[2] += self.coefficients[i][2] * basis[i];
        }
        result
    }
}

/// Probe state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProbeState {
    /// Active probe
    #[default]
    Active          = 0,
    /// Inactive (off, relocating)
    Inactive        = 1,
    /// Just activated
    NewlyActive     = 2,
    /// Outside geometry
    OutsideGeometry = 3,
}

/// GPU probe data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuProbeData {
    /// World position
    pub position: [f32; 3],
    /// State
    pub state: u32,
    /// Offset for relocation
    pub offset: [f32; 3],
    /// Padding
    pub _padding: u32,
}

// ============================================================================
// Grid Parameters
// ============================================================================

/// GPU probe grid params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuProbeGridParams {
    /// Grid dimensions
    pub dimensions: [u32; 3],
    /// Total probes
    pub probe_count: u32,
    /// Grid bounds min
    pub bounds_min: [f32; 3],
    /// Rays per probe
    pub rays_per_probe: u32,
    /// Grid bounds max
    pub bounds_max: [f32; 3],
    /// Probe spacing
    pub probe_spacing: [f32; 3],
    /// Update flags
    pub flags: u32,
    /// Frame index
    pub frame_index: u32,
    /// Hysteresis
    pub hysteresis: f32,
    /// Max ray distance
    pub max_ray_distance: f32,
    /// Irradiance encoding
    pub irradiance_resolution: u32,
    /// Visibility encoding
    pub visibility_resolution: u32,
    /// Padding
    pub _padding: [u32; 2],
}

impl GpuProbeGridParams {
    /// From create info
    pub fn from_create_info(info: &LightProbeGridCreateInfo, frame: u32) -> Self {
        let spacing = info.probe_spacing();
        Self {
            dimensions: info.dimensions,
            probe_count: info.total_probes(),
            bounds_min: info.bounds_min,
            rays_per_probe: info.rays_per_probe,
            bounds_max: info.bounds_max,
            probe_spacing: spacing,
            flags: info.features.bits(),
            frame_index: frame,
            hysteresis: 0.97,
            max_ray_distance: 100.0,
            irradiance_resolution: 8,
            visibility_resolution: 16,
            _padding: [0; 2],
        }
    }

    /// Probe index to world position
    pub fn index_to_world(&self, index: u32) -> [f32; 3] {
        let x = index % self.dimensions[0];
        let y = (index / self.dimensions[0]) % self.dimensions[1];
        let z = index / (self.dimensions[0] * self.dimensions[1]);

        [
            self.bounds_min[0] + x as f32 * self.probe_spacing[0],
            self.bounds_min[1] + y as f32 * self.probe_spacing[1],
            self.bounds_min[2] + z as f32 * self.probe_spacing[2],
        ]
    }

    /// World position to grid coordinates
    pub fn world_to_grid(&self, position: [f32; 3]) -> [f32; 3] {
        [
            (position[0] - self.bounds_min[0]) / self.probe_spacing[0],
            (position[1] - self.bounds_min[1]) / self.probe_spacing[1],
            (position[2] - self.bounds_min[2]) / self.probe_spacing[2],
        ]
    }
}

// ============================================================================
// Update Requests
// ============================================================================

/// Probe grid update request
#[derive(Clone, Debug)]
pub struct ProbeGridUpdateRequest {
    /// Grid handle
    pub grid: LightProbeGridHandle,
    /// Probes to update (None = all)
    pub probes_to_update: Option<Vec<u32>>,
    /// Max probes per frame
    pub max_probes_per_frame: u32,
    /// Ray budget
    pub ray_budget: u32,
    /// Use ray tracing
    pub ray_traced: bool,
}

impl ProbeGridUpdateRequest {
    /// Creates new request (update all)
    pub fn all(grid: LightProbeGridHandle) -> Self {
        Self {
            grid,
            probes_to_update: None,
            max_probes_per_frame: u32::MAX,
            ray_budget: u32::MAX,
            ray_traced: true,
        }
    }

    /// Time-sliced update
    pub fn time_sliced(grid: LightProbeGridHandle, max_per_frame: u32) -> Self {
        Self {
            grid,
            probes_to_update: None,
            max_probes_per_frame: max_per_frame,
            ray_budget: max_per_frame * 256,
            ray_traced: true,
        }
    }

    /// Specific probes
    pub fn specific(grid: LightProbeGridHandle, probes: Vec<u32>) -> Self {
        Self {
            grid,
            probes_to_update: Some(probes),
            max_probes_per_frame: u32::MAX,
            ray_budget: u32::MAX,
            ray_traced: true,
        }
    }
}

/// Probe sampling request
#[derive(Clone, Debug)]
pub struct ProbeSampleRequest {
    /// Grid handle
    pub grid: LightProbeGridHandle,
    /// World position
    pub position: [f32; 3],
    /// Normal
    pub normal: [f32; 3],
    /// Trilinear interpolation
    pub trilinear: bool,
}

impl ProbeSampleRequest {
    /// Creates new request
    pub fn new(grid: LightProbeGridHandle, position: [f32; 3], normal: [f32; 3]) -> Self {
        Self {
            grid,
            position,
            normal,
            trilinear: true,
        }
    }
}

// ============================================================================
// Cascaded Grid
// ============================================================================

/// Cascaded probe grid info
#[derive(Clone, Debug)]
pub struct CascadedProbeGridInfo {
    /// Cascade count
    pub cascade_count: u32,
    /// Base resolution
    pub base_resolution: [u32; 3],
    /// Scale factor between cascades
    pub scale_factor: f32,
    /// Center position
    pub center: [f32; 3],
}

impl CascadedProbeGridInfo {
    /// Creates new info
    pub fn new(cascades: u32, base_res: [u32; 3], center: [f32; 3]) -> Self {
        Self {
            cascade_count: cascades,
            base_resolution: base_res,
            scale_factor: 2.0,
            center,
        }
    }

    /// Total probes across all cascades
    pub fn total_probes(&self) -> u32 {
        let per_cascade =
            self.base_resolution[0] * self.base_resolution[1] * self.base_resolution[2];
        per_cascade * self.cascade_count
    }
}

impl Default for CascadedProbeGridInfo {
    fn default() -> Self {
        Self::new(4, [8, 4, 8], [0.0, 0.0, 0.0])
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Probe grid statistics
#[derive(Clone, Debug, Default)]
pub struct ProbeGridStats {
    /// Total probes
    pub total_probes: u32,
    /// Active probes
    pub active_probes: u32,
    /// Probes updated this frame
    pub probes_updated: u32,
    /// Rays traced this frame
    pub rays_traced: u64,
    /// Update time (microseconds)
    pub update_time_us: u64,
    /// Sample time (microseconds)
    pub sample_time_us: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Average rays per second
    pub rays_per_second: f32,
}

impl ProbeGridStats {
    /// Active probe ratio
    pub fn active_ratio(&self) -> f32 {
        if self.total_probes == 0 {
            return 0.0;
        }
        self.active_probes as f32 / self.total_probes as f32
    }

    /// Rays per probe
    pub fn rays_per_probe(&self) -> f32 {
        if self.probes_updated == 0 {
            return 0.0;
        }
        self.rays_traced as f32 / self.probes_updated as f32
    }
}
