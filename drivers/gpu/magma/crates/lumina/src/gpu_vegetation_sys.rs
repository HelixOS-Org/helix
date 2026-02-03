//! GPU Vegetation Rendering System for Lumina
//!
//! This module provides GPU-accelerated vegetation rendering including
//! trees, grass, foliage with wind animation, LOD, and instancing.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Vegetation System Handles
// ============================================================================

/// GPU vegetation system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuVegetationSystemHandle(pub u64);

impl GpuVegetationSystemHandle {
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

impl Default for GpuVegetationSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Vegetation asset handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VegetationAssetHandle(pub u64);

impl VegetationAssetHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for VegetationAssetHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Vegetation instance handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VegetationInstanceHandle(pub u64);

impl VegetationInstanceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for VegetationInstanceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Vegetation layer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VegetationLayerHandle(pub u64);

impl VegetationLayerHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for VegetationLayerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Vegetation System Creation
// ============================================================================

/// GPU vegetation system create info
#[derive(Clone, Debug)]
pub struct GpuVegetationSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max vegetation assets
    pub max_assets: u32,
    /// Max instances
    pub max_instances: u32,
    /// Max grass blades
    pub max_grass_blades: u32,
    /// Max layers
    pub max_layers: u32,
    /// Features
    pub features: VegetationFeatures,
    /// Quality
    pub quality: VegetationQuality,
}

impl GpuVegetationSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_assets: 256,
            max_instances: 100000,
            max_grass_blades: 1000000,
            max_layers: 32,
            features: VegetationFeatures::all(),
            quality: VegetationQuality::High,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max instances
    pub fn with_max_instances(mut self, count: u32) -> Self {
        self.max_instances = count;
        self
    }

    /// With max grass
    pub fn with_max_grass(mut self, count: u32) -> Self {
        self.max_grass_blades = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: VegetationFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: VegetationQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// High quality preset
    pub fn high_quality() -> Self {
        Self::new()
            .with_max_instances(500000)
            .with_max_grass(5000000)
            .with_quality(VegetationQuality::Ultra)
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_max_instances(10000)
            .with_max_grass(50000)
            .with_quality(VegetationQuality::Low)
            .with_features(VegetationFeatures::BASIC)
    }
}

impl Default for GpuVegetationSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Vegetation features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct VegetationFeatures: u32 {
        /// None
        const NONE = 0;
        /// GPU instancing
        const INSTANCING = 1 << 0;
        /// Wind animation
        const WIND = 1 << 1;
        /// LOD system
        const LOD = 1 << 2;
        /// Billboard fallback
        const BILLBOARD = 1 << 3;
        /// GPU culling
        const GPU_CULLING = 1 << 4;
        /// Procedural placement
        const PROCEDURAL = 1 << 5;
        /// Interaction (player footprints, etc)
        const INTERACTION = 1 << 6;
        /// Shadows
        const SHADOWS = 1 << 7;
        /// Subsurface scattering
        const SSS = 1 << 8;
        /// Tessellation
        const TESSELLATION = 1 << 9;
        /// Basic features
        const BASIC = Self::INSTANCING.bits() | Self::LOD.bits() | Self::WIND.bits();
        /// All
        const ALL = 0x3FF;
    }
}

impl Default for VegetationFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Vegetation quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VegetationQuality {
    /// Low
    Low    = 0,
    /// Medium
    Medium = 1,
    /// High
    #[default]
    High   = 2,
    /// Ultra
    Ultra  = 3,
}

// ============================================================================
// Vegetation Asset
// ============================================================================

/// Vegetation asset create info
#[derive(Clone, Debug)]
pub struct VegetationAssetCreateInfo {
    /// Name
    pub name: String,
    /// Asset type
    pub asset_type: VegetationType,
    /// LOD meshes
    pub lod_meshes: Vec<VegetationLodMesh>,
    /// Billboard texture
    pub billboard_texture: Option<u64>,
    /// Material settings
    pub material: VegetationMaterial,
    /// Wind settings
    pub wind: VegetationWindSettings,
    /// Bounds
    pub bounds: VegetationBounds,
}

impl VegetationAssetCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>, asset_type: VegetationType) -> Self {
        Self {
            name: name.into(),
            asset_type,
            lod_meshes: Vec::new(),
            billboard_texture: None,
            material: VegetationMaterial::default(),
            wind: VegetationWindSettings::default(),
            bounds: VegetationBounds::default(),
        }
    }

    /// With LOD mesh
    pub fn with_lod(mut self, lod: VegetationLodMesh) -> Self {
        self.lod_meshes.push(lod);
        self
    }

    /// With billboard
    pub fn with_billboard(mut self, texture: u64) -> Self {
        self.billboard_texture = Some(texture);
        self
    }

    /// With material
    pub fn with_material(mut self, material: VegetationMaterial) -> Self {
        self.material = material;
        self
    }

    /// With wind
    pub fn with_wind(mut self, wind: VegetationWindSettings) -> Self {
        self.wind = wind;
        self
    }

    /// Tree asset
    pub fn tree(name: impl Into<String>) -> Self {
        Self::new(name, VegetationType::Tree).with_wind(VegetationWindSettings::tree())
    }

    /// Bush asset
    pub fn bush(name: impl Into<String>) -> Self {
        Self::new(name, VegetationType::Bush).with_wind(VegetationWindSettings::bush())
    }

    /// Grass asset
    pub fn grass(name: impl Into<String>) -> Self {
        Self::new(name, VegetationType::Grass).with_wind(VegetationWindSettings::grass())
    }

    /// Flower asset
    pub fn flower(name: impl Into<String>) -> Self {
        Self::new(name, VegetationType::Flower).with_wind(VegetationWindSettings::flower())
    }
}

impl Default for VegetationAssetCreateInfo {
    fn default() -> Self {
        Self::new("Vegetation", VegetationType::Tree)
    }
}

/// Vegetation type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VegetationType {
    /// Tree
    #[default]
    Tree    = 0,
    /// Bush
    Bush    = 1,
    /// Grass
    Grass   = 2,
    /// Flower
    Flower  = 3,
    /// Fern
    Fern    = 4,
    /// Vine
    Vine    = 5,
    /// Crop
    Crop    = 6,
    /// Seaweed (underwater)
    Seaweed = 7,
}

/// Vegetation LOD mesh
#[derive(Clone, Debug, Default)]
pub struct VegetationLodMesh {
    /// Mesh handle
    pub mesh: u64,
    /// LOD level
    pub lod_level: u32,
    /// Screen size threshold
    pub screen_size: f32,
    /// Vertex count
    pub vertex_count: u32,
}

impl VegetationLodMesh {
    /// Creates new LOD mesh
    pub fn new(mesh: u64, lod_level: u32, screen_size: f32) -> Self {
        Self {
            mesh,
            lod_level,
            screen_size,
            vertex_count: 0,
        }
    }
}

/// Vegetation bounds
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VegetationBounds {
    /// Min bounds
    pub min: [f32; 3],
    /// Max bounds
    pub max: [f32; 3],
    /// Radius
    pub radius: f32,
    /// Height
    pub height: f32,
}

impl VegetationBounds {
    /// Creates new bounds
    pub const fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self {
            min,
            max,
            radius: 1.0,
            height: 1.0,
        }
    }

    /// Unit bounds
    pub const fn unit() -> Self {
        Self::new([-0.5, 0.0, -0.5], [0.5, 1.0, 0.5])
    }

    /// Tree bounds
    pub const fn tree() -> Self {
        Self {
            min: [-3.0, 0.0, -3.0],
            max: [3.0, 10.0, 3.0],
            radius: 3.0,
            height: 10.0,
        }
    }

    /// Bush bounds
    pub const fn bush() -> Self {
        Self {
            min: [-1.0, 0.0, -1.0],
            max: [1.0, 1.5, 1.0],
            radius: 1.0,
            height: 1.5,
        }
    }

    /// Grass bounds
    pub const fn grass() -> Self {
        Self {
            min: [-0.1, 0.0, -0.1],
            max: [0.1, 0.5, 0.1],
            radius: 0.1,
            height: 0.5,
        }
    }
}

impl Default for VegetationBounds {
    fn default() -> Self {
        Self::unit()
    }
}

// ============================================================================
// Vegetation Material
// ============================================================================

/// Vegetation material
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VegetationMaterial {
    /// Albedo color
    pub albedo: [f32; 4],
    /// Subsurface color
    pub subsurface_color: [f32; 4],
    /// Subsurface strength
    pub subsurface_strength: f32,
    /// Roughness
    pub roughness: f32,
    /// Normal strength
    pub normal_strength: f32,
    /// Alpha cutoff
    pub alpha_cutoff: f32,
    /// Two sided
    pub two_sided: bool,
    /// Use subsurface
    pub use_subsurface: bool,
}

impl VegetationMaterial {
    /// Creates new material
    pub const fn new() -> Self {
        Self {
            albedo: [0.2, 0.5, 0.1, 1.0],
            subsurface_color: [0.5, 0.8, 0.3, 1.0],
            subsurface_strength: 0.5,
            roughness: 0.8,
            normal_strength: 1.0,
            alpha_cutoff: 0.5,
            two_sided: true,
            use_subsurface: true,
        }
    }

    /// With albedo
    pub const fn with_albedo(mut self, color: [f32; 4]) -> Self {
        self.albedo = color;
        self
    }

    /// With subsurface
    pub const fn with_subsurface(mut self, color: [f32; 4], strength: f32) -> Self {
        self.subsurface_color = color;
        self.subsurface_strength = strength;
        self.use_subsurface = true;
        self
    }

    /// Leaf material preset
    pub const fn leaf() -> Self {
        Self::new()
    }

    /// Bark material preset
    pub const fn bark() -> Self {
        Self {
            albedo: [0.3, 0.2, 0.1, 1.0],
            subsurface_color: [0.0; 4],
            subsurface_strength: 0.0,
            roughness: 0.9,
            normal_strength: 1.0,
            alpha_cutoff: 0.0,
            two_sided: false,
            use_subsurface: false,
        }
    }

    /// Grass material preset
    pub const fn grass() -> Self {
        Self {
            albedo: [0.15, 0.4, 0.08, 1.0],
            subsurface_color: [0.4, 0.7, 0.2, 1.0],
            subsurface_strength: 0.6,
            roughness: 0.7,
            normal_strength: 0.5,
            alpha_cutoff: 0.3,
            two_sided: true,
            use_subsurface: true,
        }
    }

    /// Flower material preset
    pub const fn flower() -> Self {
        Self {
            albedo: [0.8, 0.3, 0.4, 1.0],
            subsurface_color: [1.0, 0.5, 0.6, 1.0],
            subsurface_strength: 0.4,
            roughness: 0.6,
            normal_strength: 0.8,
            alpha_cutoff: 0.5,
            two_sided: true,
            use_subsurface: true,
        }
    }
}

impl Default for VegetationMaterial {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Wind Animation
// ============================================================================

/// Vegetation wind settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VegetationWindSettings {
    /// Primary bend strength
    pub primary_bend: f32,
    /// Secondary bend strength
    pub secondary_bend: f32,
    /// Edge flutter strength
    pub edge_flutter: f32,
    /// Flutter frequency
    pub flutter_frequency: f32,
    /// Phase offset
    pub phase_offset: f32,
    /// Height influence (0 = base moves, 1 = only top moves)
    pub height_influence: f32,
    /// Stiffness
    pub stiffness: f32,
}

impl VegetationWindSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            primary_bend: 0.1,
            secondary_bend: 0.05,
            edge_flutter: 0.02,
            flutter_frequency: 2.0,
            phase_offset: 0.0,
            height_influence: 0.5,
            stiffness: 0.5,
        }
    }

    /// No wind
    pub const fn none() -> Self {
        Self {
            primary_bend: 0.0,
            secondary_bend: 0.0,
            edge_flutter: 0.0,
            flutter_frequency: 0.0,
            phase_offset: 0.0,
            height_influence: 0.0,
            stiffness: 1.0,
        }
    }

    /// Tree wind preset
    pub const fn tree() -> Self {
        Self {
            primary_bend: 0.05,
            secondary_bend: 0.1,
            edge_flutter: 0.03,
            flutter_frequency: 1.5,
            phase_offset: 0.0,
            height_influence: 0.8,
            stiffness: 0.7,
        }
    }

    /// Bush wind preset
    pub const fn bush() -> Self {
        Self {
            primary_bend: 0.1,
            secondary_bend: 0.08,
            edge_flutter: 0.05,
            flutter_frequency: 2.0,
            phase_offset: 0.0,
            height_influence: 0.6,
            stiffness: 0.5,
        }
    }

    /// Grass wind preset
    pub const fn grass() -> Self {
        Self {
            primary_bend: 0.2,
            secondary_bend: 0.1,
            edge_flutter: 0.15,
            flutter_frequency: 3.0,
            phase_offset: 0.0,
            height_influence: 0.9,
            stiffness: 0.3,
        }
    }

    /// Flower wind preset
    pub const fn flower() -> Self {
        Self {
            primary_bend: 0.15,
            secondary_bend: 0.12,
            edge_flutter: 0.08,
            flutter_frequency: 2.5,
            phase_offset: 0.0,
            height_influence: 0.85,
            stiffness: 0.4,
        }
    }
}

impl Default for VegetationWindSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Global wind settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct GlobalWindSettings {
    /// Wind direction
    pub direction: [f32; 3],
    /// Wind speed
    pub speed: f32,
    /// Gust strength
    pub gust_strength: f32,
    /// Gust frequency
    pub gust_frequency: f32,
    /// Turbulence
    pub turbulence: f32,
    /// Turbulence scale
    pub turbulence_scale: f32,
}

impl GlobalWindSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            speed: 1.0,
            gust_strength: 0.3,
            gust_frequency: 0.5,
            turbulence: 0.2,
            turbulence_scale: 10.0,
        }
    }

    /// No wind
    pub const fn none() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            speed: 0.0,
            gust_strength: 0.0,
            gust_frequency: 0.0,
            turbulence: 0.0,
            turbulence_scale: 1.0,
        }
    }

    /// Light breeze
    pub const fn light_breeze() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            speed: 0.5,
            gust_strength: 0.1,
            gust_frequency: 0.3,
            turbulence: 0.1,
            turbulence_scale: 15.0,
        }
    }

    /// Moderate wind
    pub const fn moderate() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            speed: 1.5,
            gust_strength: 0.4,
            gust_frequency: 0.7,
            turbulence: 0.3,
            turbulence_scale: 10.0,
        }
    }

    /// Strong wind
    pub const fn strong() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            speed: 3.0,
            gust_strength: 0.8,
            gust_frequency: 1.0,
            turbulence: 0.5,
            turbulence_scale: 8.0,
        }
    }

    /// Storm
    pub const fn storm() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            speed: 5.0,
            gust_strength: 1.5,
            gust_frequency: 1.5,
            turbulence: 0.8,
            turbulence_scale: 5.0,
        }
    }
}

impl Default for GlobalWindSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vegetation Layer
// ============================================================================

/// Vegetation layer create info
#[derive(Clone, Debug)]
pub struct VegetationLayerCreateInfo {
    /// Name
    pub name: String,
    /// Asset
    pub asset: VegetationAssetHandle,
    /// Density
    pub density: f32,
    /// Placement settings
    pub placement: PlacementSettings,
    /// LOD settings
    pub lod: VegetationLodSettings,
    /// Render distance
    pub render_distance: f32,
}

impl VegetationLayerCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>, asset: VegetationAssetHandle) -> Self {
        Self {
            name: name.into(),
            asset,
            density: 1.0,
            placement: PlacementSettings::default(),
            lod: VegetationLodSettings::default(),
            render_distance: 500.0,
        }
    }

    /// With density
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// With placement
    pub fn with_placement(mut self, placement: PlacementSettings) -> Self {
        self.placement = placement;
        self
    }

    /// With LOD
    pub fn with_lod(mut self, lod: VegetationLodSettings) -> Self {
        self.lod = lod;
        self
    }

    /// With render distance
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.render_distance = distance;
        self
    }

    /// Tree layer preset
    pub fn tree_layer(asset: VegetationAssetHandle) -> Self {
        Self::new("Trees", asset)
            .with_density(0.1)
            .with_distance(1000.0)
    }

    /// Grass layer preset
    pub fn grass_layer(asset: VegetationAssetHandle) -> Self {
        Self::new("Grass", asset)
            .with_density(10.0)
            .with_distance(100.0)
    }
}

/// Placement settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PlacementSettings {
    /// Min scale
    pub min_scale: f32,
    /// Max scale
    pub max_scale: f32,
    /// Min rotation (degrees)
    pub min_rotation: f32,
    /// Max rotation (degrees)
    pub max_rotation: f32,
    /// Align to normal
    pub align_to_normal: bool,
    /// Normal alignment strength
    pub normal_strength: f32,
    /// Random seed
    pub seed: u32,
    /// Spacing jitter
    pub jitter: f32,
}

impl PlacementSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            min_scale: 0.8,
            max_scale: 1.2,
            min_rotation: 0.0,
            max_rotation: 360.0,
            align_to_normal: true,
            normal_strength: 0.5,
            seed: 12345,
            jitter: 0.5,
        }
    }

    /// With scale range
    pub const fn with_scale(mut self, min: f32, max: f32) -> Self {
        self.min_scale = min;
        self.max_scale = max;
        self
    }

    /// With rotation range
    pub const fn with_rotation(mut self, min: f32, max: f32) -> Self {
        self.min_rotation = min;
        self.max_rotation = max;
        self
    }

    /// Uniform scale preset
    pub const fn uniform() -> Self {
        Self::new().with_scale(1.0, 1.0)
    }

    /// High variation preset
    pub const fn high_variation() -> Self {
        Self::new().with_scale(0.5, 1.5)
    }
}

impl Default for PlacementSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Vegetation LOD settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VegetationLodSettings {
    /// LOD distances
    pub lod_distances: [f32; 4],
    /// Crossfade range
    pub crossfade_range: f32,
    /// Billboard distance
    pub billboard_distance: f32,
    /// Cull distance
    pub cull_distance: f32,
    /// LOD bias
    pub lod_bias: f32,
}

impl VegetationLodSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            lod_distances: [20.0, 50.0, 100.0, 200.0],
            crossfade_range: 5.0,
            billboard_distance: 150.0,
            cull_distance: 500.0,
            lod_bias: 0.0,
        }
    }

    /// With distances
    pub const fn with_distances(mut self, distances: [f32; 4]) -> Self {
        self.lod_distances = distances;
        self
    }

    /// With billboard distance
    pub const fn with_billboard(mut self, distance: f32) -> Self {
        self.billboard_distance = distance;
        self
    }

    /// Close range preset
    pub const fn close_range() -> Self {
        Self::new().with_distances([10.0, 25.0, 50.0, 100.0])
    }

    /// Far range preset
    pub const fn far_range() -> Self {
        Self::new().with_distances([50.0, 150.0, 300.0, 500.0])
    }
}

impl Default for VegetationLodSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Vegetation Instance
// ============================================================================

/// Vegetation instance create info
#[derive(Clone, Debug)]
pub struct VegetationInstanceCreateInfo {
    /// Asset
    pub asset: VegetationAssetHandle,
    /// Instances
    pub instances: Vec<VegetationInstanceData>,
}

impl VegetationInstanceCreateInfo {
    /// Creates new info
    pub fn new(asset: VegetationAssetHandle) -> Self {
        Self {
            asset,
            instances: Vec::new(),
        }
    }

    /// Add instance
    pub fn add_instance(mut self, instance: VegetationInstanceData) -> Self {
        self.instances.push(instance);
        self
    }

    /// With instances
    pub fn with_instances(mut self, instances: Vec<VegetationInstanceData>) -> Self {
        self.instances = instances;
        self
    }
}

/// Vegetation instance data
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VegetationInstanceData {
    /// Position
    pub position: [f32; 3],
    /// Rotation (Y-axis, radians)
    pub rotation: f32,
    /// Scale
    pub scale: f32,
    /// Tint color
    pub tint: [f32; 4],
    /// Phase offset for wind
    pub phase_offset: f32,
}

impl VegetationInstanceData {
    /// Creates new instance
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
            rotation: 0.0,
            scale: 1.0,
            tint: [1.0, 1.0, 1.0, 1.0],
            phase_offset: 0.0,
        }
    }

    /// With rotation
    pub const fn with_rotation(mut self, radians: f32) -> Self {
        self.rotation = radians;
        self
    }

    /// With scale
    pub const fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// With tint
    pub const fn with_tint(mut self, tint: [f32; 4]) -> Self {
        self.tint = tint;
        self
    }

    /// With phase
    pub const fn with_phase(mut self, phase: f32) -> Self {
        self.phase_offset = phase;
        self
    }
}

impl Default for VegetationInstanceData {
    fn default() -> Self {
        Self::new(0.0, 0.0, 0.0)
    }
}

// ============================================================================
// Interaction
// ============================================================================

/// Vegetation interaction info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VegetationInteraction {
    /// Interactor position
    pub position: [f32; 3],
    /// Interactor radius
    pub radius: f32,
    /// Push strength
    pub push_strength: f32,
    /// Recovery speed
    pub recovery_speed: f32,
    /// Interaction type
    pub interaction_type: InteractionType,
}

impl VegetationInteraction {
    /// Creates new interaction
    pub const fn new(x: f32, y: f32, z: f32, radius: f32) -> Self {
        Self {
            position: [x, y, z],
            radius,
            push_strength: 1.0,
            recovery_speed: 2.0,
            interaction_type: InteractionType::Push,
        }
    }

    /// Player footstep
    pub const fn footstep(x: f32, y: f32, z: f32) -> Self {
        Self {
            position: [x, y, z],
            radius: 0.3,
            push_strength: 0.5,
            recovery_speed: 3.0,
            interaction_type: InteractionType::Push,
        }
    }

    /// Vehicle
    pub const fn vehicle(x: f32, y: f32, z: f32, radius: f32) -> Self {
        Self {
            position: [x, y, z],
            radius,
            push_strength: 2.0,
            recovery_speed: 1.0,
            interaction_type: InteractionType::Crush,
        }
    }
}

/// Interaction type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum InteractionType {
    /// Push
    #[default]
    Push  = 0,
    /// Crush
    Crush = 1,
    /// Cut
    Cut   = 2,
    /// Burn
    Burn  = 3,
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU vegetation instance
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuVegetationInstance {
    /// Model matrix row 0
    pub model_row0: [f32; 4],
    /// Model matrix row 1
    pub model_row1: [f32; 4],
    /// Model matrix row 2
    pub model_row2: [f32; 4],
    /// Tint color
    pub tint: [f32; 4],
    /// Wind phase offset
    pub wind_phase: f32,
    /// LOD level
    pub lod_level: u32,
    /// Asset index
    pub asset_index: u32,
    /// Flags
    pub flags: u32,
}

/// GPU vegetation constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuVegetationConstants {
    /// View projection matrix
    pub view_proj: [[f32; 4]; 4],
    /// Camera position
    pub camera_position: [f32; 3],
    /// Time
    pub time: f32,
    /// Wind direction
    pub wind_direction: [f32; 3],
    /// Wind speed
    pub wind_speed: f32,
    /// Gust strength
    pub gust_strength: f32,
    /// Gust frequency
    pub gust_frequency: f32,
    /// Turbulence
    pub turbulence: f32,
    /// Turbulence scale
    pub turbulence_scale: f32,
    /// LOD distances
    pub lod_distances: [f32; 4],
    /// Crossfade range
    pub crossfade_range: f32,
    /// Billboard distance
    pub billboard_distance: f32,
    /// Pad
    pub _pad: [f32; 2],
}

impl Default for GpuVegetationConstants {
    fn default() -> Self {
        Self {
            view_proj: [[0.0; 4]; 4],
            camera_position: [0.0; 3],
            time: 0.0,
            wind_direction: [1.0, 0.0, 0.0],
            wind_speed: 1.0,
            gust_strength: 0.3,
            gust_frequency: 0.5,
            turbulence: 0.2,
            turbulence_scale: 10.0,
            lod_distances: [20.0, 50.0, 100.0, 200.0],
            crossfade_range: 5.0,
            billboard_distance: 150.0,
            _pad: [0.0; 2],
        }
    }
}

/// GPU vegetation material params
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuVegetationMaterialParams {
    /// Albedo
    pub albedo: [f32; 4],
    /// Subsurface color
    pub subsurface_color: [f32; 4],
    /// Subsurface strength
    pub subsurface_strength: f32,
    /// Roughness
    pub roughness: f32,
    /// Normal strength
    pub normal_strength: f32,
    /// Alpha cutoff
    pub alpha_cutoff: f32,
    /// Primary bend
    pub primary_bend: f32,
    /// Secondary bend
    pub secondary_bend: f32,
    /// Edge flutter
    pub edge_flutter: f32,
    /// Flutter frequency
    pub flutter_frequency: f32,
}

impl Default for GpuVegetationMaterialParams {
    fn default() -> Self {
        Self {
            albedo: [0.2, 0.5, 0.1, 1.0],
            subsurface_color: [0.5, 0.8, 0.3, 1.0],
            subsurface_strength: 0.5,
            roughness: 0.8,
            normal_strength: 1.0,
            alpha_cutoff: 0.5,
            primary_bend: 0.1,
            secondary_bend: 0.05,
            edge_flutter: 0.02,
            flutter_frequency: 2.0,
        }
    }
}

// ============================================================================
// Vegetation Statistics
// ============================================================================

/// Vegetation system statistics
#[derive(Clone, Debug, Default)]
pub struct GpuVegetationStats {
    /// Total instances
    pub total_instances: u32,
    /// Visible instances
    pub visible_instances: u32,
    /// Instances by LOD
    pub instances_by_lod: [u32; 4],
    /// Billboard instances
    pub billboard_instances: u32,
    /// Total triangles
    pub total_triangles: u32,
    /// Draw calls
    pub draw_calls: u32,
    /// Culling time (ms)
    pub culling_time_ms: f32,
    /// Render time (ms)
    pub render_time_ms: f32,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

impl GpuVegetationStats {
    /// Visibility ratio
    pub fn visibility_ratio(&self) -> f32 {
        if self.total_instances > 0 {
            self.visible_instances as f32 / self.total_instances as f32
        } else {
            0.0
        }
    }

    /// Average triangles per instance
    pub fn avg_triangles_per_instance(&self) -> f32 {
        if self.visible_instances > 0 {
            self.total_triangles as f32 / self.visible_instances as f32
        } else {
            0.0
        }
    }

    /// Memory in MB
    pub fn memory_mb(&self) -> f32 {
        self.memory_usage as f32 / (1024.0 * 1024.0)
    }
}
