//! Light Clustering Types for Lumina
//!
//! This module provides light clustering infrastructure
//! for Forward+ and clustered deferred rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Light Cluster Handles
// ============================================================================

/// Light cluster handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LightClusterHandle(pub u64);

impl LightClusterHandle {
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

impl Default for LightClusterHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Light buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LightBufferHandle(pub u64);

impl LightBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LightBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Light tile buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LightTileBufferHandle(pub u64);

impl LightTileBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for LightTileBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Light Cluster Configuration
// ============================================================================

/// Light cluster create info
#[derive(Clone, Debug)]
pub struct LightClusterCreateInfo {
    /// Name
    pub name: String,
    /// Clustering mode
    pub mode: LightClusteringMode,
    /// Screen width
    pub screen_width: u32,
    /// Screen height
    pub screen_height: u32,
    /// Tile size (for tiled modes)
    pub tile_size: u32,
    /// Depth slices (for 3D clustering)
    pub depth_slices: u32,
    /// Max lights per cluster
    pub max_lights_per_cluster: u32,
    /// Total max lights
    pub max_total_lights: u32,
    /// Near plane
    pub near_plane: f32,
    /// Far plane
    pub far_plane: f32,
    /// Slice distribution
    pub slice_distribution: SliceDistribution,
}

impl LightClusterCreateInfo {
    /// Creates new info
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            name: String::new(),
            mode: LightClusteringMode::Clustered3D,
            screen_width,
            screen_height,
            tile_size: 16,
            depth_slices: 24,
            max_lights_per_cluster: 256,
            max_total_lights: 4096,
            near_plane: 0.1,
            far_plane: 1000.0,
            slice_distribution: SliceDistribution::Exponential,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With mode
    pub fn with_mode(mut self, mode: LightClusteringMode) -> Self {
        self.mode = mode;
        self
    }

    /// With tile size
    pub fn with_tile_size(mut self, size: u32) -> Self {
        self.tile_size = size;
        self
    }

    /// With depth slices
    pub fn with_depth_slices(mut self, slices: u32) -> Self {
        self.depth_slices = slices;
        self
    }

    /// With max lights per cluster
    pub fn with_max_lights_per_cluster(mut self, max: u32) -> Self {
        self.max_lights_per_cluster = max;
        self
    }

    /// With max total lights
    pub fn with_max_total_lights(mut self, max: u32) -> Self {
        self.max_total_lights = max;
        self
    }

    /// With depth range
    pub fn with_depth_range(mut self, near: f32, far: f32) -> Self {
        self.near_plane = near;
        self.far_plane = far;
        self
    }

    /// With slice distribution
    pub fn with_distribution(mut self, distribution: SliceDistribution) -> Self {
        self.slice_distribution = distribution;
        self
    }

    /// Tiled (2D) clustering
    pub fn tiled(screen_width: u32, screen_height: u32) -> Self {
        Self::new(screen_width, screen_height)
            .with_mode(LightClusteringMode::Tiled2D)
            .with_tile_size(16)
    }

    /// Clustered (3D) clustering - standard
    pub fn clustered(screen_width: u32, screen_height: u32) -> Self {
        Self::new(screen_width, screen_height)
            .with_mode(LightClusteringMode::Clustered3D)
            .with_tile_size(16)
            .with_depth_slices(24)
    }

    /// High quality preset
    pub fn high_quality(screen_width: u32, screen_height: u32) -> Self {
        Self::new(screen_width, screen_height)
            .with_mode(LightClusteringMode::Clustered3D)
            .with_tile_size(8)
            .with_depth_slices(32)
            .with_max_lights_per_cluster(512)
            .with_max_total_lights(8192)
    }

    /// Performance preset
    pub fn performance(screen_width: u32, screen_height: u32) -> Self {
        Self::new(screen_width, screen_height)
            .with_mode(LightClusteringMode::Tiled2D)
            .with_tile_size(32)
            .with_max_lights_per_cluster(128)
            .with_max_total_lights(1024)
    }

    /// Tile count X
    pub fn tiles_x(&self) -> u32 {
        (self.screen_width + self.tile_size - 1) / self.tile_size
    }

    /// Tile count Y
    pub fn tiles_y(&self) -> u32 {
        (self.screen_height + self.tile_size - 1) / self.tile_size
    }

    /// Total cluster count
    pub fn total_clusters(&self) -> u32 {
        let slices = if matches!(self.mode, LightClusteringMode::Tiled2D) {
            1
        } else {
            self.depth_slices
        };
        self.tiles_x() * self.tiles_y() * slices
    }
}

impl Default for LightClusterCreateInfo {
    fn default() -> Self {
        Self::clustered(1920, 1080)
    }
}

/// Light clustering mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LightClusteringMode {
    /// No clustering
    None = 0,
    /// 2D tiled (Forward+)
    Tiled2D = 1,
    /// 3D clustered (Clustered Forward)
    #[default]
    Clustered3D = 2,
    /// Hierarchical
    Hierarchical = 3,
    /// View space slicing
    ViewSpaceSlicing = 4,
}

impl LightClusteringMode {
    /// Display name
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Tiled2D => "Tiled 2D (Forward+)",
            Self::Clustered3D => "Clustered 3D",
            Self::Hierarchical => "Hierarchical",
            Self::ViewSpaceSlicing => "View Space Slicing",
        }
    }

    /// Is 3D clustering
    pub const fn is_3d(&self) -> bool {
        matches!(self, Self::Clustered3D | Self::Hierarchical | Self::ViewSpaceSlicing)
    }
}

/// Slice distribution
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SliceDistribution {
    /// Linear distribution
    Linear = 0,
    /// Exponential distribution (more slices near camera)
    #[default]
    Exponential = 1,
    /// Logarithmic distribution
    Logarithmic = 2,
    /// Custom distribution
    Custom = 3,
}

// ============================================================================
// Light Data
// ============================================================================

/// Light type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LightType {
    /// Point light
    #[default]
    Point = 0,
    /// Spot light
    Spot = 1,
    /// Directional light
    Directional = 2,
    /// Area light (rect)
    AreaRect = 3,
    /// Area light (disk)
    AreaDisk = 4,
    /// Area light (sphere)
    AreaSphere = 5,
    /// Area light (tube)
    AreaTube = 6,
}

impl LightType {
    /// Is area light
    pub const fn is_area(&self) -> bool {
        matches!(self, Self::AreaRect | Self::AreaDisk | Self::AreaSphere | Self::AreaTube)
    }

    /// Is positional
    pub const fn is_positional(&self) -> bool {
        !matches!(self, Self::Directional)
    }

    /// Has attenuation
    pub const fn has_attenuation(&self) -> bool {
        !matches!(self, Self::Directional)
    }
}

/// GPU light data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuLightData {
    /// Position (xyz) + type (w)
    pub position_type: [f32; 4],
    /// Direction (xyz) + inner angle (w)
    pub direction_inner_angle: [f32; 4],
    /// Color (xyz) + intensity (w)
    pub color_intensity: [f32; 4],
    /// Range (x), outer angle (y), shadow index (z), flags (w)
    pub range_angle_shadow_flags: [f32; 4],
}

impl GpuLightData {
    /// Creates point light
    pub fn point(position: [f32; 3], color: [f32; 3], intensity: f32, range: f32) -> Self {
        Self {
            position_type: [position[0], position[1], position[2], LightType::Point as u32 as f32],
            direction_inner_angle: [0.0, 0.0, 0.0, 0.0],
            color_intensity: [color[0], color[1], color[2], intensity],
            range_angle_shadow_flags: [range, 0.0, -1.0, 0.0],
        }
    }

    /// Creates spot light
    pub fn spot(
        position: [f32; 3],
        direction: [f32; 3],
        color: [f32; 3],
        intensity: f32,
        range: f32,
        inner_angle: f32,
        outer_angle: f32,
    ) -> Self {
        Self {
            position_type: [position[0], position[1], position[2], LightType::Spot as u32 as f32],
            direction_inner_angle: [direction[0], direction[1], direction[2], inner_angle],
            color_intensity: [color[0], color[1], color[2], intensity],
            range_angle_shadow_flags: [range, outer_angle, -1.0, 0.0],
        }
    }

    /// Creates directional light
    pub fn directional(direction: [f32; 3], color: [f32; 3], intensity: f32) -> Self {
        Self {
            position_type: [0.0, 0.0, 0.0, LightType::Directional as u32 as f32],
            direction_inner_angle: [direction[0], direction[1], direction[2], 0.0],
            color_intensity: [color[0], color[1], color[2], intensity],
            range_angle_shadow_flags: [f32::MAX, 0.0, -1.0, 0.0],
        }
    }

    /// With shadow index
    pub fn with_shadow_index(mut self, index: i32) -> Self {
        self.range_angle_shadow_flags[2] = index as f32;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: u32) -> Self {
        self.range_angle_shadow_flags[3] = flags as f32;
        self
    }

    /// Light type
    pub fn light_type(&self) -> LightType {
        match self.position_type[3] as u32 {
            0 => LightType::Point,
            1 => LightType::Spot,
            2 => LightType::Directional,
            3 => LightType::AreaRect,
            4 => LightType::AreaDisk,
            5 => LightType::AreaSphere,
            6 => LightType::AreaTube,
            _ => LightType::Point,
        }
    }

    /// Size in bytes
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

/// Light flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct LightFlags(pub u32);

impl LightFlags {
    /// Enabled
    pub const ENABLED: Self = Self(1 << 0);
    /// Cast shadows
    pub const CAST_SHADOWS: Self = Self(1 << 1);
    /// Volumetric
    pub const VOLUMETRIC: Self = Self(1 << 2);
    /// Affect specular
    pub const AFFECT_SPECULAR: Self = Self(1 << 3);
    /// IES profile
    pub const IES_PROFILE: Self = Self(1 << 4);
    /// Cookie texture
    pub const COOKIE: Self = Self(1 << 5);

    /// Default flags
    pub const DEFAULT: Self = Self(
        Self::ENABLED.0 |
        Self::AFFECT_SPECULAR.0
    );
}

// ============================================================================
// Light Buffer
// ============================================================================

/// Light buffer create info
#[derive(Clone, Debug)]
pub struct LightBufferCreateInfo {
    /// Name
    pub name: String,
    /// Max lights
    pub max_lights: u32,
    /// Dynamic
    pub dynamic: bool,
    /// Include shadow data
    pub include_shadow_data: bool,
}

impl LightBufferCreateInfo {
    /// Creates new info
    pub fn new(max_lights: u32) -> Self {
        Self {
            name: String::new(),
            max_lights,
            dynamic: true,
            include_shadow_data: true,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Static buffer
    pub fn static_buffer(mut self) -> Self {
        self.dynamic = false;
        self
    }

    /// Without shadow data
    pub fn without_shadows(mut self) -> Self {
        self.include_shadow_data = false;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new(4096)
    }

    /// Large scene preset
    pub fn large_scene() -> Self {
        Self::new(16384)
    }
}

impl Default for LightBufferCreateInfo {
    fn default() -> Self {
        Self::standard()
    }
}

// ============================================================================
// Cluster Update
// ============================================================================

/// Cluster update info
#[derive(Clone, Debug)]
pub struct ClusterUpdateInfo {
    /// Light cluster
    pub cluster: LightClusterHandle,
    /// View matrix
    pub view_matrix: [[f32; 4]; 4],
    /// Projection matrix
    pub projection_matrix: [[f32; 4]; 4],
    /// Light buffer
    pub light_buffer: LightBufferHandle,
    /// Light count
    pub light_count: u32,
    /// Near plane (override)
    pub near_plane: Option<f32>,
    /// Far plane (override)
    pub far_plane: Option<f32>,
}

impl ClusterUpdateInfo {
    /// Creates new info
    pub fn new(cluster: LightClusterHandle) -> Self {
        Self {
            cluster,
            view_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            projection_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            light_buffer: LightBufferHandle::NULL,
            light_count: 0,
            near_plane: None,
            far_plane: None,
        }
    }

    /// With view matrix
    pub fn with_view(mut self, view: [[f32; 4]; 4]) -> Self {
        self.view_matrix = view;
        self
    }

    /// With projection matrix
    pub fn with_projection(mut self, projection: [[f32; 4]; 4]) -> Self {
        self.projection_matrix = projection;
        self
    }

    /// With light buffer
    pub fn with_lights(mut self, buffer: LightBufferHandle, count: u32) -> Self {
        self.light_buffer = buffer;
        self.light_count = count;
        self
    }

    /// With depth planes
    pub fn with_depth_planes(mut self, near: f32, far: f32) -> Self {
        self.near_plane = Some(near);
        self.far_plane = Some(far);
        self
    }
}

impl Default for ClusterUpdateInfo {
    fn default() -> Self {
        Self::new(LightClusterHandle::NULL)
    }
}

// ============================================================================
// Cluster GPU Data
// ============================================================================

/// Cluster GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ClusterGpuParams {
    /// Cluster dimensions (tiles_x, tiles_y, slices, 0)
    pub dimensions: [u32; 4],
    /// Screen size (width, height, 1/width, 1/height)
    pub screen_size: [f32; 4],
    /// Depth params (near, far, scale, bias)
    pub depth_params: [f32; 4],
    /// Max lights per cluster
    pub max_lights_per_cluster: u32,
    /// Total light count
    pub total_light_count: u32,
    /// Tile size
    pub tile_size: u32,
    /// Flags
    pub flags: u32,
}

impl ClusterGpuParams {
    /// Creates params from create info
    pub fn from_create_info(info: &LightClusterCreateInfo) -> Self {
        let tiles_x = info.tiles_x();
        let tiles_y = info.tiles_y();
        let slices = if matches!(info.mode, LightClusteringMode::Tiled2D) {
            1
        } else {
            info.depth_slices
        };

        // Calculate logarithmic depth scale and bias
        let near = info.near_plane;
        let far = info.far_plane;
        let scale = slices as f32 / (far / near).ln();
        let bias = -(slices as f32 * near.ln()) / (far / near).ln();

        Self {
            dimensions: [tiles_x, tiles_y, slices, 0],
            screen_size: [
                info.screen_width as f32,
                info.screen_height as f32,
                1.0 / info.screen_width as f32,
                1.0 / info.screen_height as f32,
            ],
            depth_params: [near, far, scale, bias],
            max_lights_per_cluster: info.max_lights_per_cluster,
            total_light_count: 0,
            tile_size: info.tile_size,
            flags: 0,
        }
    }

    /// Size in bytes
    pub const fn size() -> usize {
        core::mem::size_of::<Self>()
    }
}

// ============================================================================
// Light Tile Data
// ============================================================================

/// Light tile data (indices per tile)
#[derive(Clone, Debug, Default)]
pub struct LightTileData {
    /// Offset into light index buffer
    pub offset: u32,
    /// Light count in this tile
    pub count: u32,
}

/// Cluster light indices
#[derive(Clone, Debug)]
pub struct ClusterLightIndices {
    /// Light indices per cluster
    pub indices: Vec<Vec<u32>>,
    /// Total clusters
    pub cluster_count: u32,
    /// Total indexed lights
    pub total_indexed_lights: u32,
}

impl Default for ClusterLightIndices {
    fn default() -> Self {
        Self {
            indices: Vec::new(),
            cluster_count: 0,
            total_indexed_lights: 0,
        }
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Light clustering statistics
#[derive(Clone, Debug, Default)]
pub struct LightClusterStats {
    /// Total clusters
    pub total_clusters: u32,
    /// Active clusters (with lights)
    pub active_clusters: u32,
    /// Total lights
    pub total_lights: u32,
    /// Max lights in single cluster
    pub max_lights_in_cluster: u32,
    /// Average lights per active cluster
    pub avg_lights_per_cluster: f32,
    /// Cluster update time (microseconds)
    pub update_time_us: u64,
    /// Light assignment time (microseconds)
    pub assignment_time_us: u64,
    /// Overflow count (clusters exceeding max)
    pub overflow_count: u32,
}

impl LightClusterStats {
    /// Cluster utilization (0.0 - 1.0)
    pub fn cluster_utilization(&self) -> f32 {
        if self.total_clusters == 0 {
            return 0.0;
        }
        self.active_clusters as f32 / self.total_clusters as f32
    }
}
