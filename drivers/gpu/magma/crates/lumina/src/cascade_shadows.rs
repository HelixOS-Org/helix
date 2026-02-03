//! Cascaded Shadow Mapping Types for Lumina
//!
//! This module provides cascaded shadow map management
//! including CSM, SDSM, and stabilization techniques.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Cascade Handles
// ============================================================================

/// Cascade shadow map handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CascadeShadowMapHandle(pub u64);

impl CascadeShadowMapHandle {
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

impl Default for CascadeShadowMapHandle {
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
}

impl Default for ShadowAtlasHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Shadow cache handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShadowCacheHandle(pub u64);

impl ShadowCacheHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ShadowCacheHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Cascade Shadow Map Creation
// ============================================================================

/// Cascade shadow map create info
#[derive(Clone, Debug)]
pub struct CascadeShadowMapCreateInfo {
    /// Name
    pub name: String,
    /// Cascade count
    pub cascade_count: u32,
    /// Shadow map resolution per cascade
    pub resolution: u32,
    /// Shadow format
    pub format: ShadowFormat,
    /// Cascade split scheme
    pub split_scheme: CascadeSplitScheme,
    /// Lambda (blend between logarithmic and uniform)
    pub split_lambda: f32,
    /// Max shadow distance
    pub max_distance: f32,
    /// Stabilization mode
    pub stabilization: CascadeStabilization,
    /// Depth bias
    pub depth_bias: f32,
    /// Normal bias
    pub normal_bias: f32,
    /// Filtering mode
    pub filtering: ShadowFilterMode,
    /// Features
    pub features: CascadeFeatures,
}

impl CascadeShadowMapCreateInfo {
    /// Creates new info
    pub fn new(cascade_count: u32, resolution: u32) -> Self {
        Self {
            name: String::new(),
            cascade_count,
            resolution,
            format: ShadowFormat::Depth16,
            split_scheme: CascadeSplitScheme::Logarithmic,
            split_lambda: 0.5,
            max_distance: 500.0,
            stabilization: CascadeStabilization::Texel,
            depth_bias: 0.0001,
            normal_bias: 0.001,
            filtering: ShadowFilterMode::Pcf3x3,
            features: CascadeFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With split scheme
    pub fn with_split_scheme(mut self, scheme: CascadeSplitScheme) -> Self {
        self.split_scheme = scheme;
        self
    }

    /// With lambda
    pub fn with_lambda(mut self, lambda: f32) -> Self {
        self.split_lambda = lambda;
        self
    }

    /// With max distance
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.max_distance = distance;
        self
    }

    /// With stabilization
    pub fn with_stabilization(mut self, stab: CascadeStabilization) -> Self {
        self.stabilization = stab;
        self
    }

    /// With bias
    pub fn with_bias(mut self, depth: f32, normal: f32) -> Self {
        self.depth_bias = depth;
        self.normal_bias = normal;
        self
    }

    /// With filtering
    pub fn with_filtering(mut self, filtering: ShadowFilterMode) -> Self {
        self.filtering = filtering;
        self
    }

    /// With features
    pub fn with_features(mut self, features: CascadeFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Low quality preset
    pub fn low_quality(max_distance: f32) -> Self {
        Self::new(2, 1024)
            .with_distance(max_distance)
            .with_filtering(ShadowFilterMode::Pcf2x2)
    }

    /// Medium quality preset
    pub fn medium_quality(max_distance: f32) -> Self {
        Self::new(3, 2048)
            .with_distance(max_distance)
            .with_filtering(ShadowFilterMode::Pcf3x3)
    }

    /// High quality preset
    pub fn high_quality(max_distance: f32) -> Self {
        Self::new(4, 2048)
            .with_distance(max_distance)
            .with_filtering(ShadowFilterMode::Pcf5x5)
            .with_features(CascadeFeatures::BLEND_CASCADES)
    }

    /// Ultra quality preset
    pub fn ultra_quality(max_distance: f32) -> Self {
        Self::new(4, 4096)
            .with_distance(max_distance)
            .with_filtering(ShadowFilterMode::Pcss)
            .with_features(CascadeFeatures::BLEND_CASCADES | CascadeFeatures::SAMPLE_DISTRIBUTION)
    }

    /// Indoor preset (short range, high detail)
    pub fn indoor() -> Self {
        Self::new(2, 2048)
            .with_distance(50.0)
            .with_lambda(0.3)
            .with_filtering(ShadowFilterMode::Pcf3x3)
    }

    /// Outdoor preset (long range)
    pub fn outdoor() -> Self {
        Self::new(4, 2048)
            .with_distance(1000.0)
            .with_lambda(0.7)
            .with_filtering(ShadowFilterMode::Pcf3x3)
    }
}

impl Default for CascadeShadowMapCreateInfo {
    fn default() -> Self {
        Self::medium_quality(500.0)
    }
}

/// Shadow format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadowFormat {
    /// 16-bit depth
    #[default]
    Depth16 = 0,
    /// 24-bit depth
    Depth24 = 1,
    /// 32-bit float depth
    Depth32F = 2,
    /// 32-bit depth + 8 stencil
    Depth24Stencil8 = 3,
    /// Variance shadow map (2 channels)
    Vsm = 4,
    /// Exponential shadow map
    Esm = 5,
    /// Moment shadow map (4 channels)
    Msm = 6,
}

impl ShadowFormat {
    /// Bits per pixel
    pub const fn bits_per_pixel(&self) -> u32 {
        match self {
            Self::Depth16 => 16,
            Self::Depth24 | Self::Depth24Stencil8 => 32,
            Self::Depth32F | Self::Esm => 32,
            Self::Vsm => 64,
            Self::Msm => 128,
        }
    }

    /// Is depth format
    pub const fn is_depth(&self) -> bool {
        matches!(self, Self::Depth16 | Self::Depth24 | Self::Depth32F | Self::Depth24Stencil8)
    }
}

/// Cascade split scheme
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CascadeSplitScheme {
    /// Uniform splits
    Uniform = 0,
    /// Logarithmic splits
    #[default]
    Logarithmic = 1,
    /// Practical (blend uniform + logarithmic)
    Practical = 2,
    /// Sample distribution (SDSM)
    SampleDistribution = 3,
    /// Custom splits
    Custom = 4,
}

/// Cascade stabilization
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CascadeStabilization {
    /// No stabilization
    None = 0,
    /// Texel snapping
    #[default]
    Texel = 1,
    /// Sphere fitting
    Sphere = 2,
    /// Bounding volume
    BoundingVolume = 3,
}

/// Shadow filter mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadowFilterMode {
    /// No filtering (hard shadows)
    None = 0,
    /// 2x2 PCF
    Pcf2x2 = 1,
    /// 3x3 PCF
    #[default]
    Pcf3x3 = 2,
    /// 5x5 PCF
    Pcf5x5 = 3,
    /// 7x7 PCF
    Pcf7x7 = 4,
    /// Poisson disk PCF
    PoissonPcf = 5,
    /// Percentage Closer Soft Shadows
    Pcss = 6,
    /// Variance Shadow Map filtering
    Vsm = 7,
    /// Exponential Shadow Map filtering
    Esm = 8,
}

impl ShadowFilterMode {
    /// Sample count
    pub const fn sample_count(&self) -> u32 {
        match self {
            Self::None => 1,
            Self::Pcf2x2 => 4,
            Self::Pcf3x3 => 9,
            Self::Pcf5x5 => 25,
            Self::Pcf7x7 => 49,
            Self::PoissonPcf => 16,
            Self::Pcss => 32,
            Self::Vsm | Self::Esm => 1,
        }
    }
}

bitflags::bitflags! {
    /// Cascade features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct CascadeFeatures: u32 {
        /// None
        const NONE = 0;
        /// Blend between cascades
        const BLEND_CASCADES = 1 << 0;
        /// Sample distribution shadow maps
        const SAMPLE_DISTRIBUTION = 1 << 1;
        /// Per-cascade resolution
        const VARIABLE_RESOLUTION = 1 << 2;
        /// Cached cascades
        const CACHING = 1 << 3;
        /// Async cascade updates
        const ASYNC_UPDATE = 1 << 4;
        /// Contact hardening
        const CONTACT_HARDENING = 1 << 5;
        /// Screen-space shadows
        const SCREEN_SPACE = 1 << 6;
    }
}

// ============================================================================
// Cascade Data
// ============================================================================

/// Cascade info
#[derive(Clone, Copy, Debug, Default)]
pub struct CascadeInfo {
    /// Cascade index
    pub index: u32,
    /// Near distance
    pub near_distance: f32,
    /// Far distance
    pub far_distance: f32,
    /// Resolution
    pub resolution: u32,
    /// View matrix
    pub view_matrix: [[f32; 4]; 4],
    /// Projection matrix
    pub projection_matrix: [[f32; 4]; 4],
    /// View-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Texel size
    pub texel_size: f32,
    /// Sphere center (for stabilization)
    pub sphere_center: [f32; 3],
    /// Sphere radius
    pub sphere_radius: f32,
}

impl CascadeInfo {
    /// UV scale for atlas
    pub fn uv_scale(&self, atlas_size: u32) -> f32 {
        self.resolution as f32 / atlas_size as f32
    }

    /// World-space coverage
    pub fn coverage(&self) -> f32 {
        self.far_distance - self.near_distance
    }
}

/// GPU cascade data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCascadeData {
    /// View-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Split distance
    pub split_distance: f32,
    /// Texel size
    pub texel_size: f32,
    /// Bias
    pub bias: f32,
    /// Normal bias
    pub normal_bias: f32,
    /// UV offset X
    pub uv_offset_x: f32,
    /// UV offset Y
    pub uv_offset_y: f32,
    /// UV scale
    pub uv_scale: f32,
    /// Padding
    pub _padding: f32,
}

impl GpuCascadeData {
    /// From cascade info
    pub fn from_cascade(info: &CascadeInfo, atlas_offset: [f32; 2], atlas_scale: f32) -> Self {
        Self {
            view_projection: info.view_projection,
            split_distance: info.far_distance,
            texel_size: info.texel_size,
            bias: 0.0,
            normal_bias: 0.0,
            uv_offset_x: atlas_offset[0],
            uv_offset_y: atlas_offset[1],
            uv_scale: atlas_scale,
            _padding: 0.0,
        }
    }
}

/// GPU shadow params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuShadowParams {
    /// Light direction
    pub light_direction: [f32; 3],
    /// Cascade count
    pub cascade_count: u32,
    /// Max shadow distance
    pub max_distance: f32,
    /// Blend distance
    pub blend_distance: f32,
    /// Shadow strength
    pub strength: f32,
    /// Filter radius
    pub filter_radius: f32,
    /// Light size (for PCSS)
    pub light_size: f32,
    /// Blocker search samples
    pub blocker_samples: u32,
    /// PCF samples
    pub pcf_samples: u32,
    /// Flags
    pub flags: u32,
}

// ============================================================================
// Shadow Atlas
// ============================================================================

/// Shadow atlas create info
#[derive(Clone, Debug)]
pub struct ShadowAtlasCreateInfo {
    /// Name
    pub name: String,
    /// Atlas resolution
    pub resolution: u32,
    /// Format
    pub format: ShadowFormat,
    /// Max shadow casters
    pub max_shadow_casters: u32,
    /// Allow resize
    pub allow_resize: bool,
}

impl ShadowAtlasCreateInfo {
    /// Creates new info
    pub fn new(resolution: u32) -> Self {
        Self {
            name: String::new(),
            resolution,
            format: ShadowFormat::Depth16,
            max_shadow_casters: 64,
            allow_resize: false,
        }
    }

    /// With format
    pub fn with_format(mut self, format: ShadowFormat) -> Self {
        self.format = format;
        self
    }

    /// With max shadow casters
    pub fn with_max_casters(mut self, max: u32) -> Self {
        self.max_shadow_casters = max;
        self
    }

    /// Small atlas preset
    pub fn small() -> Self {
        Self::new(2048).with_max_casters(16)
    }

    /// Medium atlas preset
    pub fn medium() -> Self {
        Self::new(4096).with_max_casters(64)
    }

    /// Large atlas preset
    pub fn large() -> Self {
        Self::new(8192).with_max_casters(128)
    }
}

impl Default for ShadowAtlasCreateInfo {
    fn default() -> Self {
        Self::medium()
    }
}

/// Shadow atlas slot
#[derive(Clone, Copy, Debug, Default)]
pub struct ShadowAtlasSlot {
    /// Slot index
    pub index: u32,
    /// X offset in atlas
    pub x: u32,
    /// Y offset in atlas
    pub y: u32,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Light ID using this slot
    pub light_id: u32,
    /// Priority
    pub priority: f32,
    /// Last used frame
    pub last_used_frame: u64,
}

impl ShadowAtlasSlot {
    /// UV offset
    pub fn uv_offset(&self, atlas_size: u32) -> [f32; 2] {
        [
            self.x as f32 / atlas_size as f32,
            self.y as f32 / atlas_size as f32,
        ]
    }

    /// UV scale
    pub fn uv_scale(&self, atlas_size: u32) -> f32 {
        self.width as f32 / atlas_size as f32
    }
}

// ============================================================================
// Shadow Cache
// ============================================================================

/// Shadow cache create info
#[derive(Clone, Debug)]
pub struct ShadowCacheCreateInfo {
    /// Name
    pub name: String,
    /// Max cached shadows
    pub max_cached: u32,
    /// Cache resolution
    pub resolution: u32,
    /// Format
    pub format: ShadowFormat,
    /// Invalidation mode
    pub invalidation: CacheInvalidationMode,
}

impl ShadowCacheCreateInfo {
    /// Creates new info
    pub fn new(max_cached: u32, resolution: u32) -> Self {
        Self {
            name: String::new(),
            max_cached,
            resolution,
            format: ShadowFormat::Depth16,
            invalidation: CacheInvalidationMode::OnLightMove,
        }
    }

    /// With invalidation mode
    pub fn with_invalidation(mut self, mode: CacheInvalidationMode) -> Self {
        self.invalidation = mode;
        self
    }

    /// Static light cache
    pub fn static_lights(max_cached: u32) -> Self {
        Self::new(max_cached, 1024)
            .with_invalidation(CacheInvalidationMode::Manual)
    }
}

impl Default for ShadowCacheCreateInfo {
    fn default() -> Self {
        Self::new(32, 1024)
    }
}

/// Cache invalidation mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CacheInvalidationMode {
    /// Manual invalidation only
    Manual = 0,
    /// Invalidate when light moves
    #[default]
    OnLightMove = 1,
    /// Invalidate when objects move
    OnObjectMove = 2,
    /// Time-based invalidation
    TimeBased = 3,
}

/// Cached shadow info
#[derive(Clone, Copy, Debug, Default)]
pub struct CachedShadowInfo {
    /// Light ID
    pub light_id: u32,
    /// Cache slot
    pub slot: u32,
    /// Light position when cached
    pub cached_position: [f32; 3],
    /// Light direction when cached
    pub cached_direction: [f32; 3],
    /// Frame cached
    pub cached_frame: u64,
    /// Is valid
    pub valid: bool,
}

// ============================================================================
// Cascade Updates
// ============================================================================

/// Cascade update request
#[derive(Clone, Debug)]
pub struct CascadeUpdateRequest {
    /// CSM handle
    pub csm: CascadeShadowMapHandle,
    /// Camera info
    pub camera_position: [f32; 3],
    /// Camera direction
    pub camera_direction: [f32; 3],
    /// Camera near
    pub camera_near: f32,
    /// Camera far
    pub camera_far: f32,
    /// Camera FOV
    pub camera_fov: f32,
    /// Camera aspect
    pub camera_aspect: f32,
    /// Light direction
    pub light_direction: [f32; 3],
    /// Custom split distances (optional)
    pub custom_splits: Option<Vec<f32>>,
}

impl CascadeUpdateRequest {
    /// Creates new request
    pub fn new(csm: CascadeShadowMapHandle) -> Self {
        Self {
            csm,
            camera_position: [0.0; 3],
            camera_direction: [0.0, 0.0, -1.0],
            camera_near: 0.1,
            camera_far: 1000.0,
            camera_fov: 60.0,
            camera_aspect: 16.0 / 9.0,
            light_direction: [0.0, -1.0, 0.0],
            custom_splits: None,
        }
    }

    /// With camera
    pub fn with_camera(mut self, pos: [f32; 3], dir: [f32; 3], near: f32, far: f32) -> Self {
        self.camera_position = pos;
        self.camera_direction = dir;
        self.camera_near = near;
        self.camera_far = far;
        self
    }

    /// With light direction
    pub fn with_light(mut self, direction: [f32; 3]) -> Self {
        self.light_direction = direction;
        self
    }

    /// With custom splits
    pub fn with_splits(mut self, splits: Vec<f32>) -> Self {
        self.custom_splits = Some(splits);
        self
    }
}

/// Cascade update result
#[derive(Clone, Debug)]
pub struct CascadeUpdateResult {
    /// Updated cascades
    pub cascades: Vec<CascadeInfo>,
    /// Total coverage distance
    pub total_coverage: f32,
    /// Effective near
    pub effective_near: f32,
    /// Effective far
    pub effective_far: f32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Cascade shadow map statistics
#[derive(Clone, Debug, Default)]
pub struct CascadeShadowStats {
    /// Cascade count
    pub cascade_count: u32,
    /// Total resolution
    pub total_resolution: u32,
    /// Render time per cascade (microseconds)
    pub cascade_render_times: Vec<u64>,
    /// Objects rendered per cascade
    pub objects_per_cascade: Vec<u32>,
    /// Triangles rendered per cascade
    pub triangles_per_cascade: Vec<u64>,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Cache hit rate
    pub cache_hit_rate: f32,
}

impl CascadeShadowStats {
    /// Total render time
    pub fn total_render_time(&self) -> u64 {
        self.cascade_render_times.iter().sum()
    }

    /// Total objects
    pub fn total_objects(&self) -> u32 {
        self.objects_per_cascade.iter().sum()
    }

    /// Total triangles
    pub fn total_triangles(&self) -> u64 {
        self.triangles_per_cascade.iter().sum()
    }
}
