//! GPU Culling System Types for Lumina
//!
//! This module provides GPU-driven culling infrastructure
//! including frustum, occlusion, and visibility culling.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Culling Handles
// ============================================================================

/// GPU culling system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuCullingSystemHandle(pub u64);

impl GpuCullingSystemHandle {
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

impl Default for GpuCullingSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Visibility buffer handle (for culling results)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CullingVisibilityHandle(pub u64);

impl CullingVisibilityHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for CullingVisibilityHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Hi-Z pyramid handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HiZPyramidHandle(pub u64);

impl HiZPyramidHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for HiZPyramidHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Culling output buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CullingOutputHandle(pub u64);

impl CullingOutputHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for CullingOutputHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Culling System Creation
// ============================================================================

/// GPU culling system create info
#[derive(Clone, Debug)]
pub struct GpuCullingSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max objects
    pub max_objects: u32,
    /// Culling features
    pub features: CullingFeatures,
    /// Culling mode
    pub mode: CullingMode,
    /// Hi-Z settings
    pub hiz_settings: HiZSettings,
    /// Occlusion settings
    pub occlusion_settings: OcclusionSettings,
}

impl GpuCullingSystemCreateInfo {
    /// Creates new info
    pub fn new(max_objects: u32) -> Self {
        Self {
            name: String::new(),
            max_objects,
            features: CullingFeatures::FRUSTUM,
            mode: CullingMode::SinglePass,
            hiz_settings: HiZSettings::default(),
            occlusion_settings: OcclusionSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With features
    pub fn with_features(mut self, features: CullingFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With mode
    pub fn with_mode(mut self, mode: CullingMode) -> Self {
        self.mode = mode;
        self
    }

    /// With Hi-Z settings
    pub fn with_hiz(mut self, settings: HiZSettings) -> Self {
        self.hiz_settings = settings;
        self.features |= CullingFeatures::HIZ_OCCLUSION;
        self
    }

    /// With occlusion settings
    pub fn with_occlusion(mut self, settings: OcclusionSettings) -> Self {
        self.occlusion_settings = settings;
        self.features |= CullingFeatures::HIZ_OCCLUSION;
        self
    }

    /// Frustum only preset
    pub fn frustum_only(max_objects: u32) -> Self {
        Self::new(max_objects)
            .with_features(CullingFeatures::FRUSTUM)
    }

    /// Full culling preset
    pub fn full(max_objects: u32) -> Self {
        Self::new(max_objects)
            .with_features(CullingFeatures::all())
            .with_mode(CullingMode::TwoPass)
    }

    /// Performance preset
    pub fn performance(max_objects: u32) -> Self {
        Self::new(max_objects)
            .with_features(CullingFeatures::FRUSTUM | CullingFeatures::DISTANCE)
            .with_mode(CullingMode::SinglePass)
    }

    /// Quality preset (with occlusion)
    pub fn quality(max_objects: u32) -> Self {
        Self::new(max_objects)
            .with_features(CullingFeatures::FRUSTUM | CullingFeatures::HIZ_OCCLUSION | CullingFeatures::SMALL_OBJECT)
            .with_mode(CullingMode::TwoPass)
    }
}

impl Default for GpuCullingSystemCreateInfo {
    fn default() -> Self {
        Self::frustum_only(100000)
    }
}

bitflags::bitflags! {
    /// Culling features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct CullingFeatures: u32 {
        /// None
        const NONE = 0;
        /// Frustum culling
        const FRUSTUM = 1 << 0;
        /// Hi-Z occlusion culling
        const HIZ_OCCLUSION = 1 << 1;
        /// Distance culling
        const DISTANCE = 1 << 2;
        /// Small object culling
        const SMALL_OBJECT = 1 << 3;
        /// Back-face culling
        const BACKFACE = 1 << 4;
        /// LOD selection
        const LOD_SELECTION = 1 << 5;
        /// Shadow caster culling
        const SHADOW_CASTER = 1 << 6;
        /// Contribution culling
        const CONTRIBUTION = 1 << 7;
        /// Portal culling
        const PORTAL = 1 << 8;
        /// Software rasterization occlusion
        const SOFTWARE_OCCLUSION = 1 << 9;
    }
}

/// Culling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CullingMode {
    /// Single pass culling
    #[default]
    SinglePass = 0,
    /// Two-pass (first pass for occluders, second for all)
    TwoPass = 1,
    /// Persistent (use previous frame's depth)
    Persistent = 2,
    /// Temporal (blend multiple frames)
    Temporal = 3,
}

impl CullingMode {
    /// Display name
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::SinglePass => "Single Pass",
            Self::TwoPass => "Two Pass",
            Self::Persistent => "Persistent",
            Self::Temporal => "Temporal",
        }
    }

    /// Uses previous frame data
    pub const fn uses_history(&self) -> bool {
        matches!(self, Self::Persistent | Self::Temporal)
    }
}

// ============================================================================
// Hi-Z Settings
// ============================================================================

/// Hi-Z pyramid settings
#[derive(Clone, Debug)]
pub struct HiZSettings {
    /// Base resolution (same as depth buffer)
    pub base_resolution: [u32; 2],
    /// Max mip levels (0 = auto)
    pub max_mip_levels: u32,
    /// Reduction mode
    pub reduction_mode: HiZReductionMode,
    /// Format
    pub format: HiZFormat,
}

impl HiZSettings {
    /// Creates new settings
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            base_resolution: [width, height],
            max_mip_levels: 0,
            reduction_mode: HiZReductionMode::Max,
            format: HiZFormat::R32Float,
        }
    }

    /// With max mips
    pub fn with_max_mips(mut self, mips: u32) -> Self {
        self.max_mip_levels = mips;
        self
    }

    /// With reduction mode
    pub fn with_reduction(mut self, mode: HiZReductionMode) -> Self {
        self.reduction_mode = mode;
        self
    }

    /// Calculate mip count
    pub fn mip_count(&self) -> u32 {
        if self.max_mip_levels > 0 {
            return self.max_mip_levels;
        }
        let max_dim = self.base_resolution[0].max(self.base_resolution[1]);
        (max_dim as f32).log2().floor() as u32 + 1
    }
}

impl Default for HiZSettings {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

/// Hi-Z reduction mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HiZReductionMode {
    /// Max depth (conservative for occlusion)
    #[default]
    Max = 0,
    /// Min depth
    Min = 1,
    /// Min-Max (both stored)
    MinMax = 2,
}

/// Hi-Z format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HiZFormat {
    /// R32 float
    #[default]
    R32Float = 0,
    /// R16 float
    R16Float = 1,
    /// R32G32 float (for min-max)
    Rg32Float = 2,
}

// ============================================================================
// Occlusion Settings
// ============================================================================

/// Occlusion culling settings
#[derive(Clone, Debug)]
pub struct OcclusionSettings {
    /// Conservative depth bias
    pub depth_bias: f32,
    /// Screen size threshold (in pixels)
    pub screen_threshold: f32,
    /// Temporal stability frames
    pub stability_frames: u32,
    /// Use previous frame depth
    pub use_history: bool,
}

impl OcclusionSettings {
    /// Creates new settings
    pub fn new() -> Self {
        Self {
            depth_bias: 0.001,
            screen_threshold: 1.0,
            stability_frames: 4,
            use_history: true,
        }
    }

    /// With depth bias
    pub fn with_bias(mut self, bias: f32) -> Self {
        self.depth_bias = bias;
        self
    }

    /// With screen threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.screen_threshold = threshold;
        self
    }

    /// Conservative settings
    pub fn conservative() -> Self {
        Self::new()
            .with_bias(0.01)
            .with_threshold(2.0)
    }

    /// Aggressive settings
    pub fn aggressive() -> Self {
        Self::new()
            .with_bias(0.0001)
            .with_threshold(0.5)
    }
}

impl Default for OcclusionSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Culling Objects
// ============================================================================

/// Cullable object data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CullableObject {
    /// Bounding sphere center
    pub center: [f32; 3],
    /// Bounding sphere radius
    pub radius: f32,
    /// AABB min
    pub aabb_min: [f32; 3],
    /// Object flags
    pub flags: u32,
    /// AABB max
    pub aabb_max: [f32; 3],
    /// LOD distance scale
    pub lod_scale: f32,
    /// Object ID
    pub object_id: u32,
    /// Mesh ID
    pub mesh_id: u32,
    /// Material ID
    pub material_id: u32,
    /// LOD count
    pub lod_count: u32,
}

impl CullableObject {
    /// Creates new object
    pub fn new(center: [f32; 3], radius: f32) -> Self {
        Self {
            center,
            radius,
            aabb_min: [center[0] - radius, center[1] - radius, center[2] - radius],
            aabb_max: [center[0] + radius, center[1] + radius, center[2] + radius],
            flags: 0,
            lod_scale: 1.0,
            object_id: 0,
            mesh_id: 0,
            material_id: 0,
            lod_count: 1,
        }
    }

    /// With AABB
    pub fn with_aabb(mut self, min: [f32; 3], max: [f32; 3]) -> Self {
        self.aabb_min = min;
        self.aabb_max = max;
        self
    }

    /// With IDs
    pub fn with_ids(mut self, object: u32, mesh: u32, material: u32) -> Self {
        self.object_id = object;
        self.mesh_id = mesh;
        self.material_id = material;
        self
    }

    /// With LOD
    pub fn with_lod(mut self, count: u32, scale: f32) -> Self {
        self.lod_count = count;
        self.lod_scale = scale;
        self
    }

    /// Is shadow caster
    pub fn set_shadow_caster(&mut self, caster: bool) {
        if caster {
            self.flags |= 1;
        } else {
            self.flags &= !1;
        }
    }
}

bitflags::bitflags! {
    /// Cullable object flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct CullableFlags: u32 {
        /// None
        const NONE = 0;
        /// Shadow caster
        const SHADOW_CASTER = 1 << 0;
        /// Occluder
        const OCCLUDER = 1 << 1;
        /// Static object
        const STATIC = 1 << 2;
        /// Always visible
        const ALWAYS_VISIBLE = 1 << 3;
        /// Two-sided
        const TWO_SIDED = 1 << 4;
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU culling params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuCullingParams {
    /// View-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Frustum planes (6 planes)
    pub frustum_planes: [[f32; 4]; 6],
    /// Camera position
    pub camera_position: [f32; 3],
    /// Object count
    pub object_count: u32,
    /// Near plane
    pub near_plane: f32,
    /// Far plane
    pub far_plane: f32,
    /// Screen width
    pub screen_width: f32,
    /// Screen height
    pub screen_height: f32,
    /// Hi-Z mip count
    pub hiz_mip_count: u32,
    /// Depth bias
    pub depth_bias: f32,
    /// Screen threshold
    pub screen_threshold: f32,
    /// Max distance
    pub max_distance: f32,
    /// Feature flags
    pub features: u32,
    /// LOD bias
    pub lod_bias: f32,
    /// Small object threshold
    pub small_object_threshold: f32,
    /// Frame index
    pub frame_index: u32,
}

impl GpuCullingParams {
    /// From create info
    pub fn from_create_info(info: &GpuCullingSystemCreateInfo) -> Self {
        Self {
            features: info.features.bits(),
            depth_bias: info.occlusion_settings.depth_bias,
            screen_threshold: info.occlusion_settings.screen_threshold,
            hiz_mip_count: info.hiz_settings.mip_count(),
            screen_width: info.hiz_settings.base_resolution[0] as f32,
            screen_height: info.hiz_settings.base_resolution[1] as f32,
            ..Default::default()
        }
    }
}

/// Culling output data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct CullingOutput {
    /// Visible object indices
    pub visible_count: u32,
    /// Frustum culled count
    pub frustum_culled: u32,
    /// Occlusion culled count
    pub occlusion_culled: u32,
    /// Distance culled count
    pub distance_culled: u32,
    /// Small object culled count
    pub small_object_culled: u32,
    /// Total processed
    pub total_processed: u32,
    /// Padding
    pub _padding: [u32; 2],
}

impl CullingOutput {
    /// Total culled
    pub fn total_culled(&self) -> u32 {
        self.frustum_culled + self.occlusion_culled + self.distance_culled + self.small_object_culled
    }

    /// Visibility ratio
    pub fn visibility_ratio(&self) -> f32 {
        if self.total_processed == 0 {
            return 0.0;
        }
        self.visible_count as f32 / self.total_processed as f32
    }
}

// ============================================================================
// Culling Requests
// ============================================================================

/// Culling request
#[derive(Clone, Debug)]
pub struct CullingRequest {
    /// Culling system
    pub system: GpuCullingSystemHandle,
    /// Objects buffer
    pub objects_buffer: u64,
    /// Object count
    pub object_count: u32,
    /// View-projection matrix
    pub view_projection: [[f32; 4]; 4],
    /// Camera position
    pub camera_position: [f32; 3],
    /// Near plane
    pub near_plane: f32,
    /// Far plane
    pub far_plane: f32,
    /// Hi-Z texture (for occlusion)
    pub hiz_texture: Option<u64>,
    /// Previous frame depth (for temporal)
    pub prev_depth: Option<u64>,
    /// Output buffer
    pub output_buffer: u64,
    /// Visible indices buffer
    pub visible_indices_buffer: u64,
}

impl CullingRequest {
    /// Creates new request
    pub fn new(system: GpuCullingSystemHandle, objects: u64, count: u32) -> Self {
        Self {
            system,
            objects_buffer: objects,
            object_count: count,
            view_projection: [[0.0; 4]; 4],
            camera_position: [0.0; 3],
            near_plane: 0.1,
            far_plane: 1000.0,
            hiz_texture: None,
            prev_depth: None,
            output_buffer: 0,
            visible_indices_buffer: 0,
        }
    }

    /// With camera
    pub fn with_camera(mut self, vp: [[f32; 4]; 4], pos: [f32; 3], near: f32, far: f32) -> Self {
        self.view_projection = vp;
        self.camera_position = pos;
        self.near_plane = near;
        self.far_plane = far;
        self
    }

    /// With Hi-Z
    pub fn with_hiz(mut self, hiz: u64) -> Self {
        self.hiz_texture = Some(hiz);
        self
    }

    /// With output buffers
    pub fn with_output(mut self, output: u64, visible: u64) -> Self {
        self.output_buffer = output;
        self.visible_indices_buffer = visible;
        self
    }
}

/// Hi-Z generation request
#[derive(Clone, Debug)]
pub struct HiZGenerateRequest {
    /// Depth buffer input
    pub depth_buffer: u64,
    /// Hi-Z pyramid output
    pub hiz_pyramid: u64,
    /// Resolution
    pub resolution: [u32; 2],
    /// Reduction mode
    pub reduction_mode: HiZReductionMode,
}

impl HiZGenerateRequest {
    /// Creates new request
    pub fn new(depth: u64, hiz: u64, width: u32, height: u32) -> Self {
        Self {
            depth_buffer: depth,
            hiz_pyramid: hiz,
            resolution: [width, height],
            reduction_mode: HiZReductionMode::Max,
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Culling statistics
#[derive(Clone, Debug, Default)]
pub struct CullingStats {
    /// Total objects processed
    pub total_objects: u32,
    /// Visible objects
    pub visible_objects: u32,
    /// Frustum culled
    pub frustum_culled: u32,
    /// Occlusion culled
    pub occlusion_culled: u32,
    /// Distance culled
    pub distance_culled: u32,
    /// Small object culled
    pub small_object_culled: u32,
    /// Culling time (microseconds)
    pub culling_time_us: u64,
    /// Hi-Z generation time
    pub hiz_time_us: u64,
    /// Triangles before culling
    pub triangles_before: u64,
    /// Triangles after culling
    pub triangles_after: u64,
}

impl CullingStats {
    /// Visibility ratio
    pub fn visibility_ratio(&self) -> f32 {
        if self.total_objects == 0 {
            return 0.0;
        }
        self.visible_objects as f32 / self.total_objects as f32
    }

    /// Triangle reduction ratio
    pub fn triangle_reduction(&self) -> f32 {
        if self.triangles_before == 0 {
            return 0.0;
        }
        1.0 - (self.triangles_after as f32 / self.triangles_before as f32)
    }

    /// Occlusion culling effectiveness
    pub fn occlusion_effectiveness(&self) -> f32 {
        let total_culled = self.frustum_culled + self.occlusion_culled + self.distance_culled + self.small_object_culled;
        if total_culled == 0 {
            return 0.0;
        }
        self.occlusion_culled as f32 / total_culled as f32
    }
}
