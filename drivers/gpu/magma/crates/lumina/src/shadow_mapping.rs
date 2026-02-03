//! Shadow Mapping Types for Lumina
//!
//! This module provides shadow mapping infrastructure including
//! cascaded shadow maps, point light shadows, and shadow filtering.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Shadow Handles
// ============================================================================

/// Shadow map handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShadowMapHandle(pub u64);

impl ShadowMapHandle {
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

impl Default for ShadowMapHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Shadow atlas handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShadowAtlasHandle(pub u64);

impl ShadowAtlasHandle {
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

impl Default for ShadowAtlasHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Shadow Settings
// ============================================================================

/// Shadow map create info
#[derive(Clone, Debug)]
pub struct ShadowMapCreateInfo {
    /// Shadow type
    pub shadow_type: ShadowType,
    /// Resolution
    pub resolution: u32,
    /// Format
    pub format: ShadowFormat,
    /// Filter mode
    pub filter: ShadowFilter,
    /// Bias
    pub bias: ShadowBias,
    /// Debug label
    pub label: Option<&'static str>,
}

impl ShadowMapCreateInfo {
    /// Creates info
    pub fn new(resolution: u32) -> Self {
        Self {
            shadow_type: ShadowType::Directional,
            resolution,
            format: ShadowFormat::Depth24,
            filter: ShadowFilter::Pcf2x2,
            bias: ShadowBias::default(),
            label: None,
        }
    }

    /// Low quality (512)
    pub fn low() -> Self {
        Self::new(512)
    }

    /// Medium quality (1024)
    pub fn medium() -> Self {
        Self::new(1024)
    }

    /// High quality (2048)
    pub fn high() -> Self {
        Self::new(2048)
    }

    /// Ultra quality (4096)
    pub fn ultra() -> Self {
        Self::new(4096)
    }

    /// With shadow type
    pub fn with_type(mut self, shadow_type: ShadowType) -> Self {
        self.shadow_type = shadow_type;
        self
    }

    /// With format
    pub fn with_format(mut self, format: ShadowFormat) -> Self {
        self.format = format;
        self
    }

    /// With filter
    pub fn with_filter(mut self, filter: ShadowFilter) -> Self {
        self.filter = filter;
        self
    }

    /// With bias
    pub fn with_bias(mut self, bias: ShadowBias) -> Self {
        self.bias = bias;
        self
    }

    /// With label
    pub fn with_label(mut self, label: &'static str) -> Self {
        self.label = Some(label);
        self
    }

    /// For point light
    pub fn point_light(resolution: u32) -> Self {
        Self::new(resolution).with_type(ShadowType::PointCube)
    }

    /// For spot light
    pub fn spot_light(resolution: u32) -> Self {
        Self::new(resolution).with_type(ShadowType::Spot)
    }
}

impl Default for ShadowMapCreateInfo {
    fn default() -> Self {
        Self::medium()
    }
}

/// Shadow type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadowType {
    /// Directional (parallel projection)
    #[default]
    Directional         = 0,
    /// Spot light (perspective projection)
    Spot                = 1,
    /// Point light cube map
    PointCube           = 2,
    /// Point light dual paraboloid
    PointDualParaboloid = 3,
}

/// Shadow map format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadowFormat {
    /// 16-bit depth
    Depth16  = 0,
    /// 24-bit depth
    #[default]
    Depth24  = 1,
    /// 32-bit float depth
    Depth32F = 2,
    /// Variance shadow map (RG32F)
    Vsm      = 3,
    /// Exponential shadow map (R32F)
    Esm      = 4,
    /// Moment shadow map
    Msm      = 5,
}

impl ShadowFormat {
    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Depth16 => 2,
            Self::Depth24 => 4, // Usually stored as D24S8
            Self::Depth32F | Self::Esm => 4,
            Self::Vsm => 8,
            Self::Msm => 16,
        }
    }

    /// Supports filtering
    pub const fn supports_filtering(&self) -> bool {
        matches!(self, Self::Vsm | Self::Esm | Self::Msm)
    }
}

/// Shadow filtering mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadowFilter {
    /// No filtering (hard shadows)
    None             = 0,
    /// 2x2 PCF
    #[default]
    Pcf2x2           = 1,
    /// 3x3 PCF
    Pcf3x3           = 2,
    /// 5x5 PCF
    Pcf5x5           = 3,
    /// 7x7 PCF
    Pcf7x7           = 4,
    /// Poisson disk PCF
    PcfPoisson       = 5,
    /// Percentage closer soft shadows
    Pcss             = 6,
    /// Variance shadow mapping
    Vsm              = 7,
    /// Exponential shadow mapping
    Esm              = 8,
    /// Contact hardening
    ContactHardening = 9,
}

impl ShadowFilter {
    /// Sample count
    pub const fn sample_count(&self) -> u32 {
        match self {
            Self::None => 1,
            Self::Pcf2x2 => 4,
            Self::Pcf3x3 => 9,
            Self::Pcf5x5 => 25,
            Self::Pcf7x7 => 49,
            Self::PcfPoisson => 16,
            Self::Pcss => 32,
            Self::Vsm | Self::Esm => 1,
            Self::ContactHardening => 24,
        }
    }
}

/// Shadow bias settings
#[derive(Clone, Copy, Debug)]
pub struct ShadowBias {
    /// Constant bias
    pub constant: f32,
    /// Slope-scaled bias
    pub slope_scale: f32,
    /// Normal offset
    pub normal_offset: f32,
}

impl ShadowBias {
    /// Creates bias
    pub fn new(constant: f32, slope_scale: f32) -> Self {
        Self {
            constant,
            slope_scale,
            normal_offset: 0.0,
        }
    }

    /// Low bias
    pub fn low() -> Self {
        Self::new(0.0001, 1.0)
    }

    /// Medium bias
    pub fn medium() -> Self {
        Self::new(0.0005, 2.0)
    }

    /// High bias
    pub fn high() -> Self {
        Self::new(0.001, 3.0)
    }

    /// With normal offset
    pub fn with_normal_offset(mut self, offset: f32) -> Self {
        self.normal_offset = offset;
        self
    }
}

impl Default for ShadowBias {
    fn default() -> Self {
        Self::medium()
    }
}

// ============================================================================
// Cascaded Shadow Maps
// ============================================================================

/// Cascaded shadow map settings
#[derive(Clone, Debug)]
pub struct CascadeSettings {
    /// Number of cascades
    pub cascade_count: u32,
    /// Resolution per cascade
    pub resolution: u32,
    /// Split distribution (0 = linear, 1 = logarithmic)
    pub split_lambda: f32,
    /// Max shadow distance
    pub max_distance: f32,
    /// Cascade blend range
    pub blend_range: f32,
    /// Stabilize cascades
    pub stabilize: bool,
}

impl CascadeSettings {
    /// Creates settings
    pub fn new(cascade_count: u32) -> Self {
        Self {
            cascade_count,
            resolution: 2048,
            split_lambda: 0.9,
            max_distance: 200.0,
            blend_range: 0.1,
            stabilize: true,
        }
    }

    /// Low quality (2 cascades)
    pub fn low() -> Self {
        Self {
            cascade_count: 2,
            resolution: 1024,
            max_distance: 100.0,
            ..Self::new(2)
        }
    }

    /// Medium quality (3 cascades)
    pub fn medium() -> Self {
        Self {
            cascade_count: 3,
            resolution: 2048,
            max_distance: 150.0,
            ..Self::new(3)
        }
    }

    /// High quality (4 cascades)
    pub fn high() -> Self {
        Self {
            cascade_count: 4,
            resolution: 2048,
            max_distance: 200.0,
            ..Self::new(4)
        }
    }

    /// Ultra quality (4 cascades, high res)
    pub fn ultra() -> Self {
        Self {
            cascade_count: 4,
            resolution: 4096,
            max_distance: 300.0,
            ..Self::new(4)
        }
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// With max distance
    pub fn with_max_distance(mut self, distance: f32) -> Self {
        self.max_distance = distance;
        self
    }

    /// With split lambda
    pub fn with_split_lambda(mut self, lambda: f32) -> Self {
        self.split_lambda = lambda.clamp(0.0, 1.0);
        self
    }

    /// Calculate cascade splits
    pub fn calculate_splits(&self, near: f32, far: f32) -> Vec<f32> {
        let far = far.min(self.max_distance);
        let mut splits = Vec::with_capacity(self.cascade_count as usize + 1);
        splits.push(near);

        for i in 1..self.cascade_count {
            let t = i as f32 / self.cascade_count as f32;

            // Logarithmic distribution
            let log_split = near * (far / near).powf(t);
            // Linear distribution
            let lin_split = near + (far - near) * t;
            // Blend between them
            let split = self.split_lambda * log_split + (1.0 - self.split_lambda) * lin_split;

            splits.push(split);
        }

        splits.push(far);
        splits
    }

    /// Total memory usage
    pub fn memory_usage(&self) -> u64 {
        (self.resolution as u64) * (self.resolution as u64) * 4 * (self.cascade_count as u64)
    }
}

impl Default for CascadeSettings {
    fn default() -> Self {
        Self::medium()
    }
}

/// Cascade data
#[derive(Clone, Copy, Debug, Default)]
pub struct CascadeData {
    /// Cascade index
    pub index: u32,
    /// Split near
    pub near: f32,
    /// Split far
    pub far: f32,
    /// View-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Texel size
    pub texel_size: f32,
}

impl CascadeData {
    /// Creates cascade data
    pub fn new(index: u32, near: f32, far: f32) -> Self {
        Self {
            index,
            near,
            far,
            view_projection: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            texel_size: 1.0,
        }
    }

    /// With view projection
    pub fn with_view_projection(mut self, vp: [[f32; 4]; 4]) -> Self {
        self.view_projection = vp;
        self
    }
}

/// Cascade GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CascadeGpuData {
    /// View-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Split distances (xyz = near/far/blend, w = texel_size)
    pub split_data: [f32; 4],
}

// ============================================================================
// Shadow Atlas
// ============================================================================

/// Shadow atlas create info
#[derive(Clone, Debug)]
pub struct ShadowAtlasCreateInfo {
    /// Atlas size
    pub size: u32,
    /// Format
    pub format: ShadowFormat,
    /// Max shadows
    pub max_shadows: u32,
    /// Min tile size
    pub min_tile_size: u32,
}

impl ShadowAtlasCreateInfo {
    /// Creates info
    pub fn new(size: u32) -> Self {
        Self {
            size,
            format: ShadowFormat::Depth24,
            max_shadows: 64,
            min_tile_size: 64,
        }
    }

    /// 2K atlas
    pub fn atlas_2k() -> Self {
        Self::new(2048)
    }

    /// 4K atlas
    pub fn atlas_4k() -> Self {
        Self::new(4096)
    }

    /// 8K atlas
    pub fn atlas_8k() -> Self {
        Self::new(8192)
    }

    /// With format
    pub fn with_format(mut self, format: ShadowFormat) -> Self {
        self.format = format;
        self
    }

    /// With max shadows
    pub fn with_max_shadows(mut self, count: u32) -> Self {
        self.max_shadows = count;
        self
    }

    /// Memory size
    pub fn memory_size(&self) -> u64 {
        (self.size as u64) * (self.size as u64) * (self.format.bytes_per_pixel() as u64)
    }
}

impl Default for ShadowAtlasCreateInfo {
    fn default() -> Self {
        Self::atlas_4k()
    }
}

/// Shadow atlas tile
#[derive(Clone, Copy, Debug, Default)]
pub struct ShadowAtlasTile {
    /// X offset in atlas
    pub x: u32,
    /// Y offset in atlas
    pub y: u32,
    /// Size
    pub size: u32,
    /// Light index
    pub light_index: u32,
    /// Is allocated
    pub allocated: bool,
}

impl ShadowAtlasTile {
    /// Creates tile
    pub fn new(x: u32, y: u32, size: u32) -> Self {
        Self {
            x,
            y,
            size,
            light_index: 0,
            allocated: false,
        }
    }

    /// UV rect (x, y, width, height normalized)
    pub fn uv_rect(&self, atlas_size: u32) -> [f32; 4] {
        let inv_size = 1.0 / atlas_size as f32;
        [
            self.x as f32 * inv_size,
            self.y as f32 * inv_size,
            self.size as f32 * inv_size,
            self.size as f32 * inv_size,
        ]
    }

    /// Viewport
    pub fn viewport(&self) -> ShadowViewport {
        ShadowViewport {
            x: self.x,
            y: self.y,
            width: self.size,
            height: self.size,
        }
    }
}

/// Shadow viewport
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ShadowViewport {
    /// X offset
    pub x: u32,
    /// Y offset
    pub y: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
}

// ============================================================================
// Point Light Shadows
// ============================================================================

/// Point light shadow settings
#[derive(Clone, Debug)]
pub struct PointShadowSettings {
    /// Resolution per face
    pub resolution: u32,
    /// Format
    pub format: ShadowFormat,
    /// Filter
    pub filter: ShadowFilter,
    /// Near plane
    pub near: f32,
    /// Use dual paraboloid instead of cube
    pub dual_paraboloid: bool,
}

impl PointShadowSettings {
    /// Creates settings
    pub fn new(resolution: u32) -> Self {
        Self {
            resolution,
            format: ShadowFormat::Depth24,
            filter: ShadowFilter::Pcf2x2,
            near: 0.1,
            dual_paraboloid: false,
        }
    }

    /// Low quality
    pub fn low() -> Self {
        Self::new(256)
    }

    /// Medium quality
    pub fn medium() -> Self {
        Self::new(512)
    }

    /// High quality
    pub fn high() -> Self {
        Self::new(1024)
    }

    /// Use dual paraboloid
    pub fn use_dual_paraboloid(mut self) -> Self {
        self.dual_paraboloid = true;
        self
    }

    /// Memory size (all faces)
    pub fn memory_size(&self) -> u64 {
        let faces = if self.dual_paraboloid { 2 } else { 6 };
        (self.resolution as u64)
            * (self.resolution as u64)
            * (self.format.bytes_per_pixel() as u64)
            * faces
    }
}

impl Default for PointShadowSettings {
    fn default() -> Self {
        Self::medium()
    }
}

/// Cube shadow face
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum CubeFace {
    /// +X
    PositiveX = 0,
    /// -X
    NegativeX = 1,
    /// +Y
    PositiveY = 2,
    /// -Y
    NegativeY = 3,
    /// +Z
    PositiveZ = 4,
    /// -Z
    NegativeZ = 5,
}

impl CubeFace {
    /// All faces
    pub const ALL: [Self; 6] = [
        Self::PositiveX,
        Self::NegativeX,
        Self::PositiveY,
        Self::NegativeY,
        Self::PositiveZ,
        Self::NegativeZ,
    ];

    /// Forward direction
    pub fn forward(&self) -> [f32; 3] {
        match self {
            Self::PositiveX => [1.0, 0.0, 0.0],
            Self::NegativeX => [-1.0, 0.0, 0.0],
            Self::PositiveY => [0.0, 1.0, 0.0],
            Self::NegativeY => [0.0, -1.0, 0.0],
            Self::PositiveZ => [0.0, 0.0, 1.0],
            Self::NegativeZ => [0.0, 0.0, -1.0],
        }
    }

    /// Up direction
    pub fn up(&self) -> [f32; 3] {
        match self {
            Self::PositiveX | Self::NegativeX => [0.0, -1.0, 0.0],
            Self::PositiveY => [0.0, 0.0, 1.0],
            Self::NegativeY => [0.0, 0.0, -1.0],
            Self::PositiveZ | Self::NegativeZ => [0.0, -1.0, 0.0],
        }
    }
}

// ============================================================================
// Shadow GPU Data
// ============================================================================

/// Shadow light GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ShadowLightGpuData {
    /// Light view-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Atlas UV rect (xy = offset, zw = size)
    pub atlas_rect: [f32; 4],
    /// Bias and softness
    pub params: [f32; 4],
}

impl ShadowLightGpuData {
    /// Creates data
    pub fn new(vp: [[f32; 4]; 4], atlas_rect: [f32; 4], bias: f32, softness: f32) -> Self {
        Self {
            view_projection: vp,
            atlas_rect,
            params: [bias, softness, 0.0, 0.0],
        }
    }
}

/// Shadow cascade GPU data (packed for shader)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ShadowCascadesGpu {
    /// Cascade view-projection matrices
    pub view_projections: [[[f32; 4]; 4]; 4],
    /// Cascade split distances
    pub splits: [f32; 4],
    /// Cascade count and params
    pub params: [f32; 4],
}

// ============================================================================
// Shadow Statistics
// ============================================================================

/// Shadow statistics
#[derive(Clone, Debug, Default)]
pub struct ShadowStats {
    /// Active shadow maps
    pub active_maps: u32,
    /// Cascade renders
    pub cascade_renders: u32,
    /// Point light renders
    pub point_light_renders: u32,
    /// Spot light renders
    pub spot_light_renders: u32,
    /// Shadow draw calls
    pub draw_calls: u32,
    /// Triangles rendered
    pub triangles: u64,
    /// Atlas utilization
    pub atlas_utilization: f32,
    /// GPU time (microseconds)
    pub gpu_time_us: u64,
}

impl ShadowStats {
    /// Total renders
    pub fn total_renders(&self) -> u32 {
        self.cascade_renders + self.point_light_renders + self.spot_light_renders
    }
}
