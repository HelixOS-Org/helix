//! GPU Terrain Rendering Types for Lumina
//!
//! This module provides GPU-accelerated terrain rendering infrastructure
//! including heightmap, clipmap, and virtual texturing support.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Terrain Handles
// ============================================================================

/// GPU terrain system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuTerrainSystemHandle(pub u64);

impl GpuTerrainSystemHandle {
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

impl Default for GpuTerrainSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Terrain tile handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TerrainTileHandle(pub u64);

impl TerrainTileHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for TerrainTileHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Heightmap handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct HeightmapHandle(pub u64);

impl HeightmapHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for HeightmapHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Terrain layer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TerrainLayerHandle(pub u64);

impl TerrainLayerHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for TerrainLayerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Terrain foliage handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TerrainFoliageHandle(pub u64);

impl TerrainFoliageHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for TerrainFoliageHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Terrain System Creation
// ============================================================================

/// GPU terrain system create info
#[derive(Clone, Debug)]
pub struct GpuTerrainSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max tiles
    pub max_tiles: u32,
    /// Tile resolution
    pub tile_resolution: u32,
    /// Max height
    pub max_height: f32,
    /// Terrain method
    pub terrain_method: TerrainMethod,
    /// LOD levels
    pub lod_levels: u32,
    /// Features
    pub features: TerrainFeatures,
}

impl GpuTerrainSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_tiles: 256,
            tile_resolution: 129,
            max_height: 1000.0,
            terrain_method: TerrainMethod::Clipmap,
            lod_levels: 8,
            features: TerrainFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max tiles
    pub fn with_max_tiles(mut self, count: u32) -> Self {
        self.max_tiles = count;
        self
    }

    /// With tile resolution
    pub fn with_tile_resolution(mut self, res: u32) -> Self {
        self.tile_resolution = res;
        self
    }

    /// With max height
    pub fn with_max_height(mut self, height: f32) -> Self {
        self.max_height = height;
        self
    }

    /// With method
    pub fn with_method(mut self, method: TerrainMethod) -> Self {
        self.terrain_method = method;
        self
    }

    /// With LOD levels
    pub fn with_lod_levels(mut self, levels: u32) -> Self {
        self.lod_levels = levels;
        self
    }

    /// With features
    pub fn with_features(mut self, features: TerrainFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard system
    pub fn standard() -> Self {
        Self::new()
    }

    /// Large world
    pub fn large_world() -> Self {
        Self::new()
            .with_max_tiles(4096)
            .with_tile_resolution(257)
            .with_lod_levels(12)
            .with_method(TerrainMethod::Clipmap)
    }

    /// Simple heightmap
    pub fn simple() -> Self {
        Self::new()
            .with_max_tiles(16)
            .with_tile_resolution(65)
            .with_lod_levels(4)
            .with_method(TerrainMethod::Heightmap)
    }

    /// Virtual texturing terrain
    pub fn virtual_textured() -> Self {
        Self::new()
            .with_method(TerrainMethod::VirtualTexture)
            .with_features(TerrainFeatures::all())
    }
}

impl Default for GpuTerrainSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Terrain method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TerrainMethod {
    /// Simple heightmap
    Heightmap = 0,
    /// Clipmap (geoclipmaps)
    #[default]
    Clipmap = 1,
    /// Quadtree LOD
    Quadtree = 2,
    /// CDLOD
    Cdlod = 3,
    /// Virtual texture
    VirtualTexture = 4,
    /// Voxel terrain
    Voxel = 5,
}

bitflags::bitflags! {
    /// Terrain features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct TerrainFeatures: u32 {
        /// None
        const NONE = 0;
        /// Normal mapping
        const NORMAL_MAPPING = 1 << 0;
        /// Tessellation
        const TESSELLATION = 1 << 1;
        /// GPU culling
        const GPU_CULLING = 1 << 2;
        /// Multi-layer texturing
        const MULTI_LAYER = 1 << 3;
        /// Detail textures
        const DETAIL_TEXTURES = 1 << 4;
        /// Procedural detail
        const PROCEDURAL_DETAIL = 1 << 5;
        /// Shadows
        const SHADOWS = 1 << 6;
        /// Foliage
        const FOLIAGE = 1 << 7;
        /// Decals
        const DECALS = 1 << 8;
        /// Holes/cutouts
        const HOLES = 1 << 9;
        /// All
        const ALL = 0x3FF;
    }
}

impl Default for TerrainFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Terrain Tile
// ============================================================================

/// Terrain tile create info
#[derive(Clone, Debug)]
pub struct TerrainTileCreateInfo {
    /// Name
    pub name: String,
    /// Position (world XZ)
    pub position: [f32; 2],
    /// Size (world units)
    pub size: f32,
    /// Heightmap
    pub heightmap: HeightmapHandle,
    /// Min height
    pub min_height: f32,
    /// Max height
    pub max_height: f32,
    /// LOD bias
    pub lod_bias: f32,
}

impl TerrainTileCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            position: [0.0, 0.0],
            size: 1000.0,
            heightmap: HeightmapHandle::NULL,
            min_height: 0.0,
            max_height: 100.0,
            lod_bias: 0.0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With position
    pub fn with_position(mut self, x: f32, z: f32) -> Self {
        self.position = [x, z];
        self
    }

    /// With size
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// With heightmap
    pub fn with_heightmap(mut self, heightmap: HeightmapHandle) -> Self {
        self.heightmap = heightmap;
        self
    }

    /// With height range
    pub fn with_height_range(mut self, min: f32, max: f32) -> Self {
        self.min_height = min;
        self.max_height = max;
        self
    }

    /// With LOD bias
    pub fn with_lod_bias(mut self, bias: f32) -> Self {
        self.lod_bias = bias;
        self
    }
}

impl Default for TerrainTileCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Heightmap
// ============================================================================

/// Heightmap create info
#[derive(Clone, Debug)]
pub struct HeightmapCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Format
    pub format: HeightmapFormat,
    /// Height data
    pub data: Vec<u8>,
}

impl HeightmapCreateInfo {
    /// Creates new info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            name: String::new(),
            width,
            height,
            format: HeightmapFormat::R16,
            data: Vec::new(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With format
    pub fn with_format(mut self, format: HeightmapFormat) -> Self {
        self.format = format;
        self
    }

    /// With data
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// From R16 data
    pub fn from_r16(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self::new(width, height)
            .with_format(HeightmapFormat::R16)
            .with_data(data)
    }

    /// From R32F data
    pub fn from_r32f(width: u32, height: u32, data: Vec<u8>) -> Self {
        Self::new(width, height)
            .with_format(HeightmapFormat::R32F)
            .with_data(data)
    }
}

impl Default for HeightmapCreateInfo {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Heightmap format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum HeightmapFormat {
    /// R8 (8-bit)
    R8 = 0,
    /// R16 (16-bit)
    #[default]
    R16 = 1,
    /// R32F (float)
    R32F = 2,
}

impl HeightmapFormat {
    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::R8 => 1,
            Self::R16 => 2,
            Self::R32F => 4,
        }
    }
}

// ============================================================================
// Terrain Layer
// ============================================================================

/// Terrain layer create info
#[derive(Clone, Debug)]
pub struct TerrainLayerCreateInfo {
    /// Name
    pub name: String,
    /// Albedo texture
    pub albedo: u64,
    /// Normal texture
    pub normal: u64,
    /// Roughness texture
    pub roughness: u64,
    /// Height texture (for parallax)
    pub height: u64,
    /// Tiling
    pub tiling: [f32; 2],
    /// UV offset
    pub offset: [f32; 2],
    /// Blend settings
    pub blend: LayerBlendSettings,
}

impl TerrainLayerCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            albedo: 0,
            normal: 0,
            roughness: 0,
            height: 0,
            tiling: [1.0, 1.0],
            offset: [0.0, 0.0],
            blend: LayerBlendSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With albedo
    pub fn with_albedo(mut self, texture: u64) -> Self {
        self.albedo = texture;
        self
    }

    /// With normal
    pub fn with_normal(mut self, texture: u64) -> Self {
        self.normal = texture;
        self
    }

    /// With tiling
    pub fn with_tiling(mut self, u: f32, v: f32) -> Self {
        self.tiling = [u, v];
        self
    }

    /// With blend settings
    pub fn with_blend(mut self, blend: LayerBlendSettings) -> Self {
        self.blend = blend;
        self
    }

    /// Grass layer
    pub fn grass() -> Self {
        Self::new()
            .with_name("Grass")
            .with_tiling(10.0, 10.0)
    }

    /// Rock layer
    pub fn rock() -> Self {
        Self::new()
            .with_name("Rock")
            .with_tiling(5.0, 5.0)
            .with_blend(LayerBlendSettings::slope_based(0.7))
    }

    /// Sand layer
    pub fn sand() -> Self {
        Self::new()
            .with_name("Sand")
            .with_tiling(20.0, 20.0)
            .with_blend(LayerBlendSettings::height_based(10.0, 50.0))
    }

    /// Snow layer
    pub fn snow() -> Self {
        Self::new()
            .with_name("Snow")
            .with_tiling(8.0, 8.0)
            .with_blend(LayerBlendSettings::height_based(500.0, 600.0))
    }
}

impl Default for TerrainLayerCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Layer blend settings
#[derive(Clone, Copy, Debug)]
pub struct LayerBlendSettings {
    /// Blend mode
    pub mode: LayerBlendMode,
    /// Height min
    pub height_min: f32,
    /// Height max
    pub height_max: f32,
    /// Slope threshold
    pub slope_threshold: f32,
    /// Blend sharpness
    pub sharpness: f32,
    /// Noise amplitude
    pub noise_amplitude: f32,
}

impl LayerBlendSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            mode: LayerBlendMode::Painted,
            height_min: 0.0,
            height_max: 1000.0,
            slope_threshold: 0.0,
            sharpness: 1.0,
            noise_amplitude: 0.0,
        }
    }

    /// Painted (splatmap)
    pub const fn painted() -> Self {
        Self::new()
    }

    /// Height based
    pub const fn height_based(min: f32, max: f32) -> Self {
        Self {
            mode: LayerBlendMode::Height,
            height_min: min,
            height_max: max,
            slope_threshold: 0.0,
            sharpness: 1.0,
            noise_amplitude: 0.1,
        }
    }

    /// Slope based
    pub const fn slope_based(threshold: f32) -> Self {
        Self {
            mode: LayerBlendMode::Slope,
            height_min: 0.0,
            height_max: 1000.0,
            slope_threshold: threshold,
            sharpness: 2.0,
            noise_amplitude: 0.05,
        }
    }

    /// With sharpness
    pub const fn with_sharpness(mut self, sharpness: f32) -> Self {
        self.sharpness = sharpness;
        self
    }

    /// With noise
    pub const fn with_noise(mut self, amplitude: f32) -> Self {
        self.noise_amplitude = amplitude;
        self
    }
}

impl Default for LayerBlendSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Layer blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LayerBlendMode {
    /// Painted via splatmap
    #[default]
    Painted = 0,
    /// Height based
    Height = 1,
    /// Slope based
    Slope = 2,
    /// Curvature based
    Curvature = 3,
    /// Combined
    Combined = 4,
}

// ============================================================================
// Terrain Foliage
// ============================================================================

/// Terrain foliage create info
#[derive(Clone, Debug)]
pub struct TerrainFoliageCreateInfo {
    /// Name
    pub name: String,
    /// Foliage type
    pub foliage_type: FoliageType,
    /// Mesh handle
    pub mesh: u64,
    /// Density
    pub density: f32,
    /// Min/max scale
    pub scale_range: [f32; 2],
    /// Min/max height
    pub height_range: [f32; 2],
    /// Max slope
    pub max_slope: f32,
    /// Align to terrain
    pub align_to_terrain: f32,
    /// Random rotation
    pub random_rotation: bool,
    /// Cull distance
    pub cull_distance: f32,
    /// Cast shadows
    pub cast_shadows: bool,
}

impl TerrainFoliageCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            foliage_type: FoliageType::Grass,
            mesh: 0,
            density: 10.0,
            scale_range: [0.8, 1.2],
            height_range: [0.0, 1000.0],
            max_slope: 0.7,
            align_to_terrain: 0.5,
            random_rotation: true,
            cull_distance: 100.0,
            cast_shadows: false,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With type
    pub fn with_type(mut self, foliage_type: FoliageType) -> Self {
        self.foliage_type = foliage_type;
        self
    }

    /// With mesh
    pub fn with_mesh(mut self, mesh: u64) -> Self {
        self.mesh = mesh;
        self
    }

    /// With density
    pub fn with_density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// With scale range
    pub fn with_scale(mut self, min: f32, max: f32) -> Self {
        self.scale_range = [min, max];
        self
    }

    /// With height range
    pub fn with_height(mut self, min: f32, max: f32) -> Self {
        self.height_range = [min, max];
        self
    }

    /// With cull distance
    pub fn with_cull_distance(mut self, distance: f32) -> Self {
        self.cull_distance = distance;
        self
    }

    /// Grass preset
    pub fn grass() -> Self {
        Self::new()
            .with_name("Grass")
            .with_type(FoliageType::Grass)
            .with_density(50.0)
            .with_scale(0.7, 1.3)
            .with_cull_distance(50.0)
    }

    /// Tree preset
    pub fn tree() -> Self {
        Self::new()
            .with_name("Tree")
            .with_type(FoliageType::Tree)
            .with_density(0.1)
            .with_scale(0.8, 1.5)
            .with_cull_distance(500.0)
    }

    /// Rock preset
    pub fn rock() -> Self {
        Self::new()
            .with_name("Rock")
            .with_type(FoliageType::Rock)
            .with_density(0.5)
            .with_scale(0.5, 2.0)
            .with_cull_distance(200.0)
    }
}

impl Default for TerrainFoliageCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Foliage type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FoliageType {
    /// Grass
    #[default]
    Grass = 0,
    /// Flowers
    Flowers = 1,
    /// Shrubs
    Shrubs = 2,
    /// Trees
    Tree = 3,
    /// Rocks
    Rock = 4,
    /// Custom mesh
    Custom = 5,
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU terrain tile data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTerrainTile {
    /// World position (x, y, z, size)
    pub position_size: [f32; 4],
    /// Height scale (min, max, scale, bias)
    pub height_params: [f32; 4],
    /// UV offset and scale
    pub uv_params: [f32; 4],
    /// LOD parameters
    pub lod_params: [f32; 4],
    /// Layer indices
    pub layer_indices: [u32; 4],
    /// Flags
    pub flags: u32,
    /// Padding
    pub _pad: [u32; 3],
}

/// GPU terrain constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTerrainConstants {
    /// Camera position
    pub camera_pos: [f32; 3],
    /// LOD distance scale
    pub lod_distance_scale: f32,
    /// World offset
    pub world_offset: [f32; 2],
    /// Terrain size
    pub terrain_size: [f32; 2],
    /// Height range (min, max)
    pub height_range: [f32; 2],
    /// Tessellation factor
    pub tessellation_factor: f32,
    /// Max tessellation
    pub max_tessellation: f32,
}

/// GPU terrain layer
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuTerrainLayer {
    /// Tiling
    pub tiling: [f32; 2],
    /// Offset
    pub offset: [f32; 2],
    /// Height range
    pub height_range: [f32; 2],
    /// Slope threshold
    pub slope_threshold: f32,
    /// Blend sharpness
    pub blend_sharpness: f32,
    /// Texture indices
    pub texture_indices: [u32; 4],
}

/// GPU foliage instance
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuFoliageInstance {
    /// Position (x, y, z)
    pub position: [f32; 3],
    /// Scale
    pub scale: f32,
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Color tint
    pub color: [f32; 4],
}

// ============================================================================
// Terrain Query
// ============================================================================

/// Terrain height query
#[derive(Clone, Copy, Debug, Default)]
pub struct TerrainHeightQuery {
    /// World X position
    pub x: f32,
    /// World Z position
    pub z: f32,
}

impl TerrainHeightQuery {
    /// New query
    pub const fn new(x: f32, z: f32) -> Self {
        Self { x, z }
    }
}

/// Terrain height result
#[derive(Clone, Copy, Debug, Default)]
pub struct TerrainHeightResult {
    /// Height
    pub height: f32,
    /// Normal
    pub normal: [f32; 3],
    /// Valid
    pub valid: bool,
}

/// Terrain raycast result
#[derive(Clone, Copy, Debug, Default)]
pub struct TerrainRaycastResult {
    /// Hit position
    pub position: [f32; 3],
    /// Hit normal
    pub normal: [f32; 3],
    /// Distance
    pub distance: f32,
    /// Hit
    pub hit: bool,
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU terrain statistics
#[derive(Clone, Debug, Default)]
pub struct GpuTerrainStats {
    /// Active tiles
    pub active_tiles: u32,
    /// Visible tiles
    pub visible_tiles: u32,
    /// Total triangles
    pub triangles: u64,
    /// Foliage instances
    pub foliage_instances: u64,
    /// Height samples
    pub height_samples: u32,
    /// GPU memory
    pub gpu_memory: u64,
    /// Render time (ms)
    pub render_time_ms: f32,
}

impl GpuTerrainStats {
    /// Triangles per tile
    pub fn triangles_per_tile(&self) -> f32 {
        if self.visible_tiles == 0 {
            0.0
        } else {
            self.triangles as f32 / self.visible_tiles as f32
        }
    }
}
