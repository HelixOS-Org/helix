//! Reflection Probe Types for Lumina
//!
//! This module provides reflection probe infrastructure
//! for image-based lighting and environment reflections.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Reflection Probe Handles
// ============================================================================

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
}

impl Default for IrradianceVolumeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Environment map handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EnvironmentMapHandle(pub u64);

impl EnvironmentMapHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for EnvironmentMapHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Light probe handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LightProbeHandle(pub u64);

impl LightProbeHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LightProbeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Reflection Probe Creation
// ============================================================================

/// Reflection probe create info
#[derive(Clone, Debug)]
pub struct ReflectionProbeCreateInfo {
    /// Name
    pub name: String,
    /// Probe type
    pub probe_type: ReflectionProbeType,
    /// Position
    pub position: [f32; 3],
    /// Box size (for box probes)
    pub box_size: [f32; 3],
    /// Radius (for sphere probes)
    pub radius: f32,
    /// Resolution
    pub resolution: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Format
    pub format: ProbeFormat,
    /// Update mode
    pub update_mode: ProbeUpdateMode,
    /// Priority
    pub priority: u32,
    /// Influence blend distance
    pub blend_distance: f32,
}

impl ReflectionProbeCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            probe_type: ReflectionProbeType::Box,
            position: [0.0, 0.0, 0.0],
            box_size: [10.0, 10.0, 10.0],
            radius: 10.0,
            resolution: 256,
            mip_levels: 8,
            format: ProbeFormat::Rgba16Float,
            update_mode: ProbeUpdateMode::Baked,
            priority: 0,
            blend_distance: 1.0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With position
    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// Box probe
    pub fn box_probe(mut self, size_x: f32, size_y: f32, size_z: f32) -> Self {
        self.probe_type = ReflectionProbeType::Box;
        self.box_size = [size_x, size_y, size_z];
        self
    }

    /// Sphere probe
    pub fn sphere_probe(mut self, radius: f32) -> Self {
        self.probe_type = ReflectionProbeType::Sphere;
        self.radius = radius;
        self
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        // Calculate mip levels
        self.mip_levels = (resolution as f32).log2() as u32 + 1;
        self
    }

    /// With format
    pub fn with_format(mut self, format: ProbeFormat) -> Self {
        self.format = format;
        self
    }

    /// With update mode
    pub fn with_update_mode(mut self, mode: ProbeUpdateMode) -> Self {
        self.update_mode = mode;
        self
    }

    /// With priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// With blend distance
    pub fn with_blend_distance(mut self, distance: f32) -> Self {
        self.blend_distance = distance;
        self
    }

    /// Baked box probe
    pub fn baked_box(position: [f32; 3], size: [f32; 3]) -> Self {
        Self::new()
            .with_position(position[0], position[1], position[2])
            .box_probe(size[0], size[1], size[2])
            .with_update_mode(ProbeUpdateMode::Baked)
    }

    /// Real-time sphere probe
    pub fn realtime_sphere(position: [f32; 3], radius: f32) -> Self {
        Self::new()
            .with_position(position[0], position[1], position[2])
            .sphere_probe(radius)
            .with_update_mode(ProbeUpdateMode::RealTime)
            .with_resolution(128)
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new()
            .with_resolution(512)
            .with_format(ProbeFormat::Rgba16Float)
    }

    /// Performance preset
    pub fn performance() -> Self {
        Self::new()
            .with_resolution(128)
            .with_format(ProbeFormat::R11g11b10Float)
    }
}

impl Default for ReflectionProbeCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Reflection probe type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ReflectionProbeType {
    /// Box influence volume
    #[default]
    Box = 0,
    /// Sphere influence volume
    Sphere = 1,
    /// Infinite (sky/global)
    Infinite = 2,
}

/// Probe format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProbeFormat {
    /// RGBA8
    Rgba8 = 0,
    /// RGBA16F
    #[default]
    Rgba16Float = 1,
    /// R11G11B10F
    R11g11b10Float = 2,
    /// RGB9E5
    Rgb9e5 = 3,
}

impl ProbeFormat {
    /// Is HDR format
    pub const fn is_hdr(&self) -> bool {
        matches!(self, Self::Rgba16Float | Self::R11g11b10Float | Self::Rgb9e5)
    }

    /// Bits per pixel
    pub const fn bits_per_pixel(&self) -> u32 {
        match self {
            Self::Rgba8 => 32,
            Self::Rgba16Float => 64,
            Self::R11g11b10Float => 32,
            Self::Rgb9e5 => 32,
        }
    }
}

/// Probe update mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ProbeUpdateMode {
    /// Baked (offline)
    #[default]
    Baked = 0,
    /// Updated on scene load
    OnLoad = 1,
    /// Real-time (every frame)
    RealTime = 2,
    /// Time-sliced (spread across frames)
    TimeSliced = 3,
    /// Manual update
    Manual = 4,
}

// ============================================================================
// Irradiance Volume
// ============================================================================

/// Irradiance volume create info
#[derive(Clone, Debug)]
pub struct IrradianceVolumeCreateInfo {
    /// Name
    pub name: String,
    /// Position (min corner)
    pub position: [f32; 3],
    /// Size
    pub size: [f32; 3],
    /// Probe count per axis
    pub probe_count: [u32; 3],
    /// Encoding
    pub encoding: IrradianceEncoding,
    /// Update mode
    pub update_mode: ProbeUpdateMode,
    /// Include visibility
    pub visibility: bool,
}

impl IrradianceVolumeCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            position: [0.0, 0.0, 0.0],
            size: [20.0, 10.0, 20.0],
            probe_count: [8, 4, 8],
            encoding: IrradianceEncoding::SphericalHarmonics,
            update_mode: ProbeUpdateMode::Baked,
            visibility: true,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With bounds
    pub fn with_bounds(mut self, position: [f32; 3], size: [f32; 3]) -> Self {
        self.position = position;
        self.size = size;
        self
    }

    /// With probe count
    pub fn with_probe_count(mut self, x: u32, y: u32, z: u32) -> Self {
        self.probe_count = [x, y, z];
        self
    }

    /// With encoding
    pub fn with_encoding(mut self, encoding: IrradianceEncoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// With update mode
    pub fn with_update_mode(mut self, mode: ProbeUpdateMode) -> Self {
        self.update_mode = mode;
        self
    }

    /// Without visibility
    pub fn without_visibility(mut self) -> Self {
        self.visibility = false;
        self
    }

    /// Total probe count
    pub fn total_probes(&self) -> u32 {
        self.probe_count[0] * self.probe_count[1] * self.probe_count[2]
    }

    /// Probe spacing
    pub fn probe_spacing(&self) -> [f32; 3] {
        [
            self.size[0] / (self.probe_count[0] - 1).max(1) as f32,
            self.size[1] / (self.probe_count[1] - 1).max(1) as f32,
            self.size[2] / (self.probe_count[2] - 1).max(1) as f32,
        ]
    }

    /// Room-sized preset
    pub fn room() -> Self {
        Self::new()
            .with_bounds([0.0, 0.0, 0.0], [10.0, 3.0, 10.0])
            .with_probe_count(8, 4, 8)
    }

    /// Large area preset
    pub fn large_area() -> Self {
        Self::new()
            .with_bounds([0.0, 0.0, 0.0], [100.0, 20.0, 100.0])
            .with_probe_count(16, 8, 16)
    }
}

impl Default for IrradianceVolumeCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Irradiance encoding
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum IrradianceEncoding {
    /// Spherical harmonics (L2)
    #[default]
    SphericalHarmonics = 0,
    /// Octahedral encoding
    Octahedral = 1,
    /// Ambient cube
    AmbientCube = 2,
    /// Irradiance volume texture
    VolumeTexture = 3,
}

impl IrradianceEncoding {
    /// Coefficients per probe
    pub const fn coefficients_per_probe(&self) -> u32 {
        match self {
            Self::SphericalHarmonics => 9 * 3,  // L2, 3 color channels
            Self::Octahedral => 8 * 8 * 3,     // 8x8 octahedral map
            Self::AmbientCube => 6 * 3,        // 6 faces
            Self::VolumeTexture => 1,          // Single sample
        }
    }
}

// ============================================================================
// Light Probe
// ============================================================================

/// Light probe create info
#[derive(Clone, Debug)]
pub struct LightProbeCreateInfo {
    /// Name
    pub name: String,
    /// Position
    pub position: [f32; 3],
    /// Encoding
    pub encoding: LightProbeEncoding,
    /// Resolution (for cubemap)
    pub resolution: u32,
}

impl LightProbeCreateInfo {
    /// Creates new info
    pub fn new(position: [f32; 3]) -> Self {
        Self {
            name: String::new(),
            position,
            encoding: LightProbeEncoding::SphericalHarmonicsL2,
            resolution: 64,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With encoding
    pub fn with_encoding(mut self, encoding: LightProbeEncoding) -> Self {
        self.encoding = encoding;
        self
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl Default for LightProbeCreateInfo {
    fn default() -> Self {
        Self::new([0.0, 0.0, 0.0])
    }
}

/// Light probe encoding
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LightProbeEncoding {
    /// L1 spherical harmonics (4 coefficients)
    SphericalHarmonicsL1 = 0,
    /// L2 spherical harmonics (9 coefficients)
    #[default]
    SphericalHarmonicsL2 = 1,
    /// L3 spherical harmonics (16 coefficients)
    SphericalHarmonicsL3 = 2,
    /// Cubemap
    Cubemap = 3,
}

impl LightProbeEncoding {
    /// Coefficient count (per color channel)
    pub const fn coefficient_count(&self) -> u32 {
        match self {
            Self::SphericalHarmonicsL1 => 4,
            Self::SphericalHarmonicsL2 => 9,
            Self::SphericalHarmonicsL3 => 16,
            Self::Cubemap => 0,
        }
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
    pub source_type: EnvironmentSourceType,
    /// Resolution
    pub resolution: u32,
    /// Mip levels
    pub mip_levels: u32,
    /// Format
    pub format: ProbeFormat,
    /// Generate diffuse irradiance
    pub generate_irradiance: bool,
    /// Generate specular prefilter
    pub generate_prefilter: bool,
}

impl EnvironmentMapCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            source_type: EnvironmentSourceType::Cubemap,
            resolution: 512,
            mip_levels: 9,
            format: ProbeFormat::Rgba16Float,
            generate_irradiance: true,
            generate_prefilter: true,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With source type
    pub fn with_source(mut self, source: EnvironmentSourceType) -> Self {
        self.source_type = source;
        self
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self.mip_levels = (resolution as f32).log2() as u32 + 1;
        self
    }

    /// With format
    pub fn with_format(mut self, format: ProbeFormat) -> Self {
        self.format = format;
        self
    }

    /// Without irradiance
    pub fn without_irradiance(mut self) -> Self {
        self.generate_irradiance = false;
        self
    }

    /// Without prefilter
    pub fn without_prefilter(mut self) -> Self {
        self.generate_prefilter = false;
        self
    }

    /// From HDRI
    pub fn from_hdri() -> Self {
        Self::new()
            .with_source(EnvironmentSourceType::Equirectangular)
            .with_resolution(1024)
    }

    /// From cubemap
    pub fn from_cubemap() -> Self {
        Self::new()
            .with_source(EnvironmentSourceType::Cubemap)
    }

    /// Procedural sky
    pub fn procedural() -> Self {
        Self::new()
            .with_source(EnvironmentSourceType::Procedural)
    }
}

impl Default for EnvironmentMapCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Environment source type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum EnvironmentSourceType {
    /// Cubemap texture
    #[default]
    Cubemap = 0,
    /// Equirectangular (HDRI)
    Equirectangular = 1,
    /// Procedural (sky)
    Procedural = 2,
    /// Dual paraboloid
    DualParaboloid = 3,
}

// ============================================================================
// GPU Data Structures
// ============================================================================

/// GPU reflection probe data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuReflectionProbeData {
    /// Position (xyz) + probe type (w)
    pub position_type: [f32; 4],
    /// Box min or center - radius
    pub bounds_min: [f32; 4],
    /// Box max or center + radius
    pub bounds_max: [f32; 4],
    /// Texture index + mip count + blend distance + priority
    pub params: [f32; 4],
}

impl GpuReflectionProbeData {
    /// Creates from create info
    pub fn from_create_info(info: &ReflectionProbeCreateInfo, texture_index: u32) -> Self {
        let (bounds_min, bounds_max) = match info.probe_type {
            ReflectionProbeType::Box => (
                [
                    info.position[0] - info.box_size[0] * 0.5,
                    info.position[1] - info.box_size[1] * 0.5,
                    info.position[2] - info.box_size[2] * 0.5,
                    0.0,
                ],
                [
                    info.position[0] + info.box_size[0] * 0.5,
                    info.position[1] + info.box_size[1] * 0.5,
                    info.position[2] + info.box_size[2] * 0.5,
                    0.0,
                ],
            ),
            ReflectionProbeType::Sphere => (
                [info.position[0], info.position[1], info.position[2], info.radius],
                [info.position[0], info.position[1], info.position[2], info.radius],
            ),
            ReflectionProbeType::Infinite => (
                [f32::NEG_INFINITY; 4],
                [f32::INFINITY; 4],
            ),
        };

        Self {
            position_type: [
                info.position[0],
                info.position[1],
                info.position[2],
                info.probe_type as u32 as f32,
            ],
            bounds_min,
            bounds_max,
            params: [
                texture_index as f32,
                info.mip_levels as f32,
                info.blend_distance,
                info.priority as f32,
            ],
        }
    }

    /// Size in bytes
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

/// GPU spherical harmonics data (L2)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSphericalHarmonicsL2 {
    /// SH coefficients for red channel
    pub r: [f32; 9],
    /// SH coefficients for green channel
    pub g: [f32; 9],
    /// SH coefficients for blue channel
    pub b: [f32; 9],
    /// Padding
    pub _padding: [f32; 5],
}

impl GpuSphericalHarmonicsL2 {
    /// Creates black SH
    pub const fn black() -> Self {
        Self {
            r: [0.0; 9],
            g: [0.0; 9],
            b: [0.0; 9],
            _padding: [0.0; 5],
        }
    }

    /// Size in bytes
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

// ============================================================================
// Probe Update
// ============================================================================

/// Probe capture info
#[derive(Clone, Debug)]
pub struct ProbeCaptureInfo {
    /// Probe to capture
    pub probe: ReflectionProbeHandle,
    /// Capture position (override)
    pub position: Option<[f32; 3]>,
    /// Near plane
    pub near_plane: f32,
    /// Far plane
    pub far_plane: f32,
    /// Render layers
    pub render_layers: u32,
    /// Face to render (None = all faces)
    pub face: Option<CubemapFace>,
    /// Clear color
    pub clear_color: [f32; 4],
}

impl ProbeCaptureInfo {
    /// Creates new info
    pub fn new(probe: ReflectionProbeHandle) -> Self {
        Self {
            probe,
            position: None,
            near_plane: 0.1,
            far_plane: 1000.0,
            render_layers: u32::MAX,
            face: None,
            clear_color: [0.0, 0.0, 0.0, 1.0],
        }
    }

    /// With position override
    pub fn with_position(mut self, position: [f32; 3]) -> Self {
        self.position = Some(position);
        self
    }

    /// With depth range
    pub fn with_depth_range(mut self, near: f32, far: f32) -> Self {
        self.near_plane = near;
        self.far_plane = far;
        self
    }

    /// With render layers
    pub fn with_layers(mut self, layers: u32) -> Self {
        self.render_layers = layers;
        self
    }

    /// Single face
    pub fn single_face(mut self, face: CubemapFace) -> Self {
        self.face = Some(face);
        self
    }

    /// With clear color
    pub fn with_clear_color(mut self, color: [f32; 4]) -> Self {
        self.clear_color = color;
        self
    }
}

impl Default for ProbeCaptureInfo {
    fn default() -> Self {
        Self::new(ReflectionProbeHandle::NULL)
    }
}

/// Cubemap face
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CubemapFace {
    /// Positive X
    #[default]
    PositiveX = 0,
    /// Negative X
    NegativeX = 1,
    /// Positive Y
    PositiveY = 2,
    /// Negative Y
    NegativeY = 3,
    /// Positive Z
    PositiveZ = 4,
    /// Negative Z
    NegativeZ = 5,
}

impl CubemapFace {
    /// Face count
    pub const COUNT: usize = 6;

    /// All faces
    pub const ALL: [Self; 6] = [
        Self::PositiveX,
        Self::NegativeX,
        Self::PositiveY,
        Self::NegativeY,
        Self::PositiveZ,
        Self::NegativeZ,
    ];

    /// Face direction
    pub const fn direction(&self) -> [f32; 3] {
        match self {
            Self::PositiveX => [1.0, 0.0, 0.0],
            Self::NegativeX => [-1.0, 0.0, 0.0],
            Self::PositiveY => [0.0, 1.0, 0.0],
            Self::NegativeY => [0.0, -1.0, 0.0],
            Self::PositiveZ => [0.0, 0.0, 1.0],
            Self::NegativeZ => [0.0, 0.0, -1.0],
        }
    }

    /// Face up vector
    pub const fn up(&self) -> [f32; 3] {
        match self {
            Self::PositiveX => [0.0, -1.0, 0.0],
            Self::NegativeX => [0.0, -1.0, 0.0],
            Self::PositiveY => [0.0, 0.0, 1.0],
            Self::NegativeY => [0.0, 0.0, -1.0],
            Self::PositiveZ => [0.0, -1.0, 0.0],
            Self::NegativeZ => [0.0, -1.0, 0.0],
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Reflection probe statistics
#[derive(Clone, Debug, Default)]
pub struct ReflectionProbeStats {
    /// Total probes
    pub total_probes: u32,
    /// Active probes
    pub active_probes: u32,
    /// Real-time probes
    pub realtime_probes: u32,
    /// Total memory (bytes)
    pub total_memory: u64,
    /// Probes updated this frame
    pub probes_updated: u32,
    /// Faces rendered this frame
    pub faces_rendered: u32,
    /// Update time (microseconds)
    pub update_time_us: u64,
}
