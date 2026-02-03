//! GPU Scene Management Types for Lumina
//!
//! This module provides GPU-accelerated scene management
//! infrastructure for efficient rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Scene Handles
// ============================================================================

/// GPU scene handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuSceneHandle(pub u64);

impl GpuSceneHandle {
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

impl Default for GpuSceneHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Scene object handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SceneObjectHandle(pub u64);

impl SceneObjectHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SceneObjectHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Scene light handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SceneLightHandle(pub u64);

impl SceneLightHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SceneLightHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Scene primitive handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ScenePrimitiveHandle(pub u64);

impl ScenePrimitiveHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ScenePrimitiveHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Instance data handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct InstanceDataHandle(pub u64);

impl InstanceDataHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for InstanceDataHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// GPU Scene Creation
// ============================================================================

/// GPU scene create info
#[derive(Clone, Debug)]
pub struct GpuSceneCreateInfo {
    /// Name
    pub name: String,
    /// Max objects
    pub max_objects: u32,
    /// Max lights
    pub max_lights: u32,
    /// Max primitives
    pub max_primitives: u32,
    /// Max instances
    pub max_instances: u32,
    /// Scene type
    pub scene_type: SceneType,
    /// Update mode
    pub update_mode: SceneUpdateMode,
    /// Features
    pub features: GpuSceneFeatures,
}

impl GpuSceneCreateInfo {
    /// Creates new info
    pub fn new(max_objects: u32) -> Self {
        Self {
            name: String::new(),
            max_objects,
            max_lights: 1024,
            max_primitives: max_objects * 4,
            max_instances: max_objects,
            scene_type: SceneType::Standard,
            update_mode: SceneUpdateMode::Incremental,
            features: GpuSceneFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max lights
    pub fn with_max_lights(mut self, max: u32) -> Self {
        self.max_lights = max;
        self
    }

    /// With max primitives
    pub fn with_max_primitives(mut self, max: u32) -> Self {
        self.max_primitives = max;
        self
    }

    /// With max instances
    pub fn with_max_instances(mut self, max: u32) -> Self {
        self.max_instances = max;
        self
    }

    /// With scene type
    pub fn with_scene_type(mut self, scene_type: SceneType) -> Self {
        self.scene_type = scene_type;
        self
    }

    /// With update mode
    pub fn with_update_mode(mut self, mode: SceneUpdateMode) -> Self {
        self.update_mode = mode;
        self
    }

    /// With features
    pub fn with_features(mut self, features: GpuSceneFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Small scene preset
    pub fn small() -> Self {
        Self::new(1024)
            .with_max_lights(256)
    }

    /// Medium scene preset
    pub fn medium() -> Self {
        Self::new(65536)
            .with_max_lights(4096)
    }

    /// Large scene preset
    pub fn large() -> Self {
        Self::new(1048576)
            .with_max_lights(32768)
    }

    /// Instancing-heavy preset
    pub fn instanced() -> Self {
        Self::new(4096)
            .with_max_instances(1048576)
            .with_features(GpuSceneFeatures::GPU_INSTANCING | GpuSceneFeatures::INSTANCE_CULLING)
    }

    /// Ray tracing scene preset
    pub fn raytracing() -> Self {
        Self::medium()
            .with_features(GpuSceneFeatures::ACCELERATION_STRUCTURE | GpuSceneFeatures::BINDLESS)
    }
}

impl Default for GpuSceneCreateInfo {
    fn default() -> Self {
        Self::medium()
    }
}

/// Scene type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SceneType {
    /// Standard scene
    #[default]
    Standard = 0,
    /// Indoor scene (optimization hints)
    Indoor = 1,
    /// Outdoor scene
    Outdoor = 2,
    /// Open world
    OpenWorld = 3,
    /// 2D scene
    Scene2D = 4,
}

/// Scene update mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SceneUpdateMode {
    /// Full rebuild each frame
    FullRebuild = 0,
    /// Incremental updates
    #[default]
    Incremental = 1,
    /// GPU-driven updates
    GpuDriven = 2,
    /// Streaming updates
    Streaming = 3,
}

bitflags::bitflags! {
    /// GPU scene features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct GpuSceneFeatures: u32 {
        /// None
        const NONE = 0;
        /// GPU instancing
        const GPU_INSTANCING = 1 << 0;
        /// Instance culling
        const INSTANCE_CULLING = 1 << 1;
        /// LOD selection
        const LOD_SELECTION = 1 << 2;
        /// Acceleration structure
        const ACCELERATION_STRUCTURE = 1 << 3;
        /// Bindless resources
        const BINDLESS = 1 << 4;
        /// Spatial hashing
        const SPATIAL_HASHING = 1 << 5;
        /// BVH
        const BVH = 1 << 6;
        /// Transform streaming
        const TRANSFORM_STREAMING = 1 << 7;
        /// Material streaming
        const MATERIAL_STREAMING = 1 << 8;
    }
}

// ============================================================================
// Scene Objects
// ============================================================================

/// Scene object create info
#[derive(Clone, Debug)]
pub struct SceneObjectCreateInfo {
    /// Name
    pub name: String,
    /// Transform
    pub transform: GpuTransform,
    /// Bounding box
    pub bounds: GpuAabb,
    /// Mesh handle
    pub mesh: u64,
    /// Material handle
    pub material: u64,
    /// Object flags
    pub flags: ObjectFlags,
    /// Layer mask
    pub layer_mask: u32,
    /// LOD group
    pub lod_group: u32,
}

impl SceneObjectCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            transform: GpuTransform::identity(),
            bounds: GpuAabb::default(),
            mesh: 0,
            material: 0,
            flags: ObjectFlags::VISIBLE | ObjectFlags::CAST_SHADOWS,
            layer_mask: 0xFFFFFFFF,
            lod_group: 0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With transform
    pub fn with_transform(mut self, transform: GpuTransform) -> Self {
        self.transform = transform;
        self
    }

    /// With position
    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.transform.position = [x, y, z, 1.0];
        self
    }

    /// With bounds
    pub fn with_bounds(mut self, bounds: GpuAabb) -> Self {
        self.bounds = bounds;
        self
    }

    /// With mesh
    pub fn with_mesh(mut self, mesh: u64) -> Self {
        self.mesh = mesh;
        self
    }

    /// With material
    pub fn with_material(mut self, material: u64) -> Self {
        self.material = material;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: ObjectFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With layer mask
    pub fn with_layer_mask(mut self, mask: u32) -> Self {
        self.layer_mask = mask;
        self
    }

    /// With LOD group
    pub fn with_lod_group(mut self, group: u32) -> Self {
        self.lod_group = group;
        self
    }

    /// Static object preset
    pub fn static_object() -> Self {
        Self::new().with_flags(ObjectFlags::VISIBLE | ObjectFlags::CAST_SHADOWS | ObjectFlags::STATIC)
    }

    /// Dynamic object preset
    pub fn dynamic_object() -> Self {
        Self::new().with_flags(ObjectFlags::VISIBLE | ObjectFlags::CAST_SHADOWS | ObjectFlags::DYNAMIC)
    }
}

impl Default for SceneObjectCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Object flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ObjectFlags: u32 {
        /// None
        const NONE = 0;
        /// Visible
        const VISIBLE = 1 << 0;
        /// Casts shadows
        const CAST_SHADOWS = 1 << 1;
        /// Receives shadows
        const RECEIVE_SHADOWS = 1 << 2;
        /// Static (doesn't move)
        const STATIC = 1 << 3;
        /// Dynamic
        const DYNAMIC = 1 << 4;
        /// Skinned mesh
        const SKINNED = 1 << 5;
        /// Instanced
        const INSTANCED = 1 << 6;
        /// Two-sided
        const TWO_SIDED = 1 << 7;
        /// Ray tracing enabled
        const RAY_TRACING = 1 << 8;
    }
}

// ============================================================================
// Scene Lights
// ============================================================================

/// Scene light create info
#[derive(Clone, Debug)]
pub struct SceneLightCreateInfo {
    /// Name
    pub name: String,
    /// Light type
    pub light_type: SceneLightType,
    /// Position
    pub position: [f32; 4],
    /// Direction
    pub direction: [f32; 4],
    /// Color
    pub color: [f32; 4],
    /// Intensity
    pub intensity: f32,
    /// Range
    pub range: f32,
    /// Inner cone angle (radians)
    pub inner_cone_angle: f32,
    /// Outer cone angle (radians)
    pub outer_cone_angle: f32,
    /// Flags
    pub flags: LightFlags,
    /// Shadow settings
    pub shadow: LightShadowSettings,
}

impl SceneLightCreateInfo {
    /// Creates new info
    pub fn new(light_type: SceneLightType) -> Self {
        Self {
            name: String::new(),
            light_type,
            position: [0.0, 0.0, 0.0, 1.0],
            direction: [0.0, -1.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
            intensity: 1.0,
            range: 10.0,
            inner_cone_angle: 0.5,
            outer_cone_angle: 0.7,
            flags: LightFlags::ENABLED,
            shadow: LightShadowSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With position
    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z, 1.0];
        self
    }

    /// With direction
    pub fn with_direction(mut self, x: f32, y: f32, z: f32) -> Self {
        self.direction = [x, y, z, 0.0];
        self
    }

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = [r, g, b, 1.0];
        self
    }

    /// With intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    /// With range
    pub fn with_range(mut self, range: f32) -> Self {
        self.range = range;
        self
    }

    /// With shadow settings
    pub fn with_shadow(mut self, shadow: LightShadowSettings) -> Self {
        self.shadow = shadow;
        self.flags |= LightFlags::CAST_SHADOWS;
        self
    }

    /// Directional light preset
    pub fn directional() -> Self {
        Self::new(SceneLightType::Directional)
            .with_direction(0.0, -1.0, 0.0)
            .with_intensity(1.0)
    }

    /// Point light preset
    pub fn point() -> Self {
        Self::new(SceneLightType::Point)
            .with_range(10.0)
    }

    /// Spot light preset
    pub fn spot() -> Self {
        Self::new(SceneLightType::Spot)
            .with_range(20.0)
    }

    /// Sun light preset
    pub fn sun() -> Self {
        Self::directional()
            .with_color(1.0, 0.95, 0.9)
            .with_intensity(10.0)
            .with_shadow(LightShadowSettings::csm())
    }
}

impl Default for SceneLightCreateInfo {
    fn default() -> Self {
        Self::point()
    }
}

/// Scene light type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SceneLightType {
    /// Directional light
    Directional = 0,
    /// Point light
    #[default]
    Point = 1,
    /// Spot light
    Spot = 2,
    /// Area light (rectangle)
    AreaRect = 3,
    /// Area light (disk)
    AreaDisk = 4,
    /// Area light (sphere)
    AreaSphere = 5,
}

bitflags::bitflags! {
    /// Light flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct LightFlags: u32 {
        /// None
        const NONE = 0;
        /// Enabled
        const ENABLED = 1 << 0;
        /// Cast shadows
        const CAST_SHADOWS = 1 << 1;
        /// Volumetric
        const VOLUMETRIC = 1 << 2;
        /// Static
        const STATIC = 1 << 3;
        /// IES profile
        const IES_PROFILE = 1 << 4;
        /// Cookie
        const COOKIE = 1 << 5;
    }
}

/// Light shadow settings
#[derive(Clone, Debug)]
pub struct LightShadowSettings {
    /// Shadow map resolution
    pub resolution: u32,
    /// Bias
    pub bias: f32,
    /// Normal bias
    pub normal_bias: f32,
    /// Near plane
    pub near: f32,
    /// Far plane
    pub far: f32,
    /// Shadow type
    pub shadow_type: ShadowType,
}

impl LightShadowSettings {
    /// Creates new settings
    pub fn new() -> Self {
        Self {
            resolution: 1024,
            bias: 0.005,
            normal_bias: 0.05,
            near: 0.1,
            far: 100.0,
            shadow_type: ShadowType::Hard,
        }
    }

    /// With resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    /// Soft shadows preset
    pub fn soft() -> Self {
        Self::new().with_type(ShadowType::Soft)
    }

    /// CSM preset
    pub fn csm() -> Self {
        Self {
            resolution: 2048,
            bias: 0.001,
            normal_bias: 0.02,
            near: 0.1,
            far: 500.0,
            shadow_type: ShadowType::Cascaded,
        }
    }

    /// With type
    pub fn with_type(mut self, shadow_type: ShadowType) -> Self {
        self.shadow_type = shadow_type;
        self
    }
}

impl Default for LightShadowSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Shadow type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadowType {
    /// No shadows
    None = 0,
    /// Hard shadows
    #[default]
    Hard = 1,
    /// Soft shadows
    Soft = 2,
    /// Cascaded shadows
    Cascaded = 3,
    /// Ray traced shadows
    RayTraced = 4,
}

// ============================================================================
// GPU Data Structures
// ============================================================================

/// GPU transform
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTransform {
    /// Position (x, y, z, 1)
    pub position: [f32; 4],
    /// Rotation quaternion (x, y, z, w)
    pub rotation: [f32; 4],
    /// Scale (x, y, z, 1)
    pub scale: [f32; 4],
    /// Padding
    pub _pad: [f32; 4],
}

impl GpuTransform {
    /// Identity transform
    pub const fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0, 1.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0, 1.0],
            _pad: [0.0; 4],
        }
    }

    /// From position
    pub fn from_position(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z, 1.0],
            ..Self::identity()
        }
    }

    /// From scale
    pub fn from_scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            scale: [x, y, z, 1.0],
            ..Self::identity()
        }
    }

    /// From uniform scale
    pub fn from_uniform_scale(s: f32) -> Self {
        Self::from_scale(s, s, s)
    }
}

/// GPU AABB
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuAabb {
    /// Min bounds
    pub min: [f32; 4],
    /// Max bounds
    pub max: [f32; 4],
}

impl GpuAabb {
    /// Empty AABB
    pub const fn empty() -> Self {
        Self {
            min: [f32::MAX, f32::MAX, f32::MAX, 0.0],
            max: [f32::MIN, f32::MIN, f32::MIN, 0.0],
        }
    }

    /// From min/max
    pub fn from_min_max(min: [f32; 3], max: [f32; 3]) -> Self {
        Self {
            min: [min[0], min[1], min[2], 0.0],
            max: [max[0], max[1], max[2], 0.0],
        }
    }

    /// From center and extents
    pub fn from_center_extents(center: [f32; 3], extents: [f32; 3]) -> Self {
        Self::from_min_max(
            [center[0] - extents[0], center[1] - extents[1], center[2] - extents[2]],
            [center[0] + extents[0], center[1] + extents[1], center[2] + extents[2]],
        )
    }

    /// Center
    pub fn center(&self) -> [f32; 3] {
        [
            (self.min[0] + self.max[0]) * 0.5,
            (self.min[1] + self.max[1]) * 0.5,
            (self.min[2] + self.max[2]) * 0.5,
        ]
    }

    /// Extents
    pub fn extents(&self) -> [f32; 3] {
        [
            (self.max[0] - self.min[0]) * 0.5,
            (self.max[1] - self.min[1]) * 0.5,
            (self.max[2] - self.min[2]) * 0.5,
        ]
    }

    /// Size
    pub fn size(&self) -> [f32; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }
}

/// GPU object data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuObjectData {
    /// World matrix row 0
    pub world_row0: [f32; 4],
    /// World matrix row 1
    pub world_row1: [f32; 4],
    /// World matrix row 2
    pub world_row2: [f32; 4],
    /// Mesh index
    pub mesh_index: u32,
    /// Material index
    pub material_index: u32,
    /// Flags
    pub flags: u32,
    /// LOD level
    pub lod_level: u32,
    /// Bounds min
    pub bounds_min: [f32; 4],
    /// Bounds max
    pub bounds_max: [f32; 4],
}

/// GPU light data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuLightData {
    /// Position and type
    pub position_type: [f32; 4],
    /// Direction and range
    pub direction_range: [f32; 4],
    /// Color and intensity
    pub color_intensity: [f32; 4],
    /// Spot params and flags
    pub spot_params_flags: [f32; 4],
}

impl GpuLightData {
    /// From light create info
    pub fn from_create_info(info: &SceneLightCreateInfo) -> Self {
        Self {
            position_type: [info.position[0], info.position[1], info.position[2], info.light_type as u32 as f32],
            direction_range: [info.direction[0], info.direction[1], info.direction[2], info.range],
            color_intensity: [info.color[0], info.color[1], info.color[2], info.intensity],
            spot_params_flags: [info.inner_cone_angle, info.outer_cone_angle, info.flags.bits() as f32, 0.0],
        }
    }
}

// ============================================================================
// Scene Statistics
// ============================================================================

/// GPU scene statistics
#[derive(Clone, Debug, Default)]
pub struct GpuSceneStats {
    /// Total objects
    pub total_objects: u32,
    /// Visible objects
    pub visible_objects: u32,
    /// Culled objects
    pub culled_objects: u32,
    /// Total lights
    pub total_lights: u32,
    /// Active lights
    pub active_lights: u32,
    /// Total primitives
    pub total_primitives: u32,
    /// Total instances
    pub total_instances: u32,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Update time (ms)
    pub update_time_ms: f32,
    /// Cull time (ms)
    pub cull_time_ms: f32,
}

impl GpuSceneStats {
    /// Visibility ratio
    pub fn visibility_ratio(&self) -> f32 {
        if self.total_objects == 0 {
            return 0.0;
        }
        self.visible_objects as f32 / self.total_objects as f32
    }

    /// Cull ratio
    pub fn cull_ratio(&self) -> f32 {
        if self.total_objects == 0 {
            return 0.0;
        }
        self.culled_objects as f32 / self.total_objects as f32
    }
}
