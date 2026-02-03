//! Deferred Rendering Types for Lumina
//!
//! This module provides deferred rendering infrastructure including
//! G-Buffer management, deferred shading, and tiled/clustered lighting.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// G-Buffer Handles
// ============================================================================

/// G-Buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GBufferHandle(pub u64);

impl GBufferHandle {
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

impl Default for GBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Light culling handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct LightCullingHandle(pub u64);

impl LightCullingHandle {
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

impl Default for LightCullingHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// G-Buffer Configuration
// ============================================================================

/// G-Buffer create info
#[derive(Clone, Debug)]
pub struct GBufferCreateInfo {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Layout
    pub layout: GBufferLayout,
    /// Enable velocity buffer
    pub velocity_buffer: bool,
    /// Enable material ID buffer
    pub material_id_buffer: bool,
    /// MSAA samples
    pub samples: u32,
}

impl GBufferCreateInfo {
    /// Creates G-Buffer info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            layout: GBufferLayout::Standard,
            velocity_buffer: false,
            material_id_buffer: false,
            samples: 1,
        }
    }

    /// Full HD
    pub fn hd() -> Self {
        Self::new(1920, 1080)
    }

    /// 4K
    pub fn uhd() -> Self {
        Self::new(3840, 2160)
    }

    /// With layout
    pub fn with_layout(mut self, layout: GBufferLayout) -> Self {
        self.layout = layout;
        self
    }

    /// With velocity buffer
    pub fn with_velocity_buffer(mut self) -> Self {
        self.velocity_buffer = true;
        self
    }

    /// With material ID
    pub fn with_material_id(mut self) -> Self {
        self.material_id_buffer = true;
        self
    }

    /// With MSAA
    pub fn with_msaa(mut self, samples: u32) -> Self {
        self.samples = samples;
        self
    }

    /// Total memory size (bytes)
    pub fn memory_size(&self) -> u64 {
        self.layout.bytes_per_pixel() as u64 * self.width as u64 * self.height as u64
            + if self.velocity_buffer {
                4 * self.width as u64 * self.height as u64
            } else {
                0
            }
            + if self.material_id_buffer {
                4 * self.width as u64 * self.height as u64
            } else {
                0
            }
    }
}

impl Default for GBufferCreateInfo {
    fn default() -> Self {
        Self::hd()
    }
}

/// G-Buffer layout
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GBufferLayout {
    /// Standard layout (4 targets)
    #[default]
    Standard = 0,
    /// Compact layout (3 targets)
    Compact = 1,
    /// Extended layout (5+ targets)
    Extended = 2,
    /// Thin G-Buffer (2 targets)
    Thin = 3,
}

impl GBufferLayout {
    /// Number of render targets
    pub const fn render_target_count(&self) -> u32 {
        match self {
            Self::Thin => 2,
            Self::Compact => 3,
            Self::Standard => 4,
            Self::Extended => 5,
        }
    }

    /// Bytes per pixel (all targets combined)
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Thin => 16,
            Self::Compact => 24,
            Self::Standard => 32,
            Self::Extended => 40,
        }
    }
}

/// G-Buffer target descriptor
#[derive(Clone, Debug)]
pub struct GBufferTarget {
    /// Target name
    pub name: &'static str,
    /// Format
    pub format: GBufferFormat,
    /// Usage
    pub usage: GBufferTargetUsage,
}

impl GBufferTarget {
    /// Albedo target
    pub const ALBEDO: Self = Self {
        name: "Albedo",
        format: GBufferFormat::Rgba8Srgb,
        usage: GBufferTargetUsage::Albedo,
    };

    /// Normal target
    pub const NORMAL: Self = Self {
        name: "Normal",
        format: GBufferFormat::Rgb10A2,
        usage: GBufferTargetUsage::Normal,
    };

    /// Material target
    pub const MATERIAL: Self = Self {
        name: "Material",
        format: GBufferFormat::Rgba8Unorm,
        usage: GBufferTargetUsage::Material,
    };

    /// Emission target
    pub const EMISSION: Self = Self {
        name: "Emission",
        format: GBufferFormat::Rgba16F,
        usage: GBufferTargetUsage::Emission,
    };

    /// Velocity target
    pub const VELOCITY: Self = Self {
        name: "Velocity",
        format: GBufferFormat::Rg16F,
        usage: GBufferTargetUsage::Velocity,
    };
}

/// G-Buffer format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GBufferFormat {
    /// RGBA8 sRGB
    #[default]
    Rgba8Srgb = 0,
    /// RGBA8 linear
    Rgba8Unorm = 1,
    /// RGB10A2
    Rgb10A2 = 2,
    /// RGBA16F
    Rgba16F = 3,
    /// RG16F
    Rg16F = 4,
    /// R32F
    R32F = 5,
    /// RG32F
    Rg32F = 6,
}

impl GBufferFormat {
    /// Bytes per pixel
    pub const fn bytes_per_pixel(&self) -> u32 {
        match self {
            Self::Rgba8Srgb | Self::Rgba8Unorm | Self::Rgb10A2 => 4,
            Self::Rg16F => 4,
            Self::Rgba16F => 8,
            Self::R32F => 4,
            Self::Rg32F => 8,
        }
    }
}

/// G-Buffer target usage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GBufferTargetUsage {
    /// Albedo color
    #[default]
    Albedo = 0,
    /// World-space normal
    Normal = 1,
    /// Material properties (roughness, metallic)
    Material = 2,
    /// Emission/HDR
    Emission = 3,
    /// Motion vectors
    Velocity = 4,
    /// Material ID
    MaterialId = 5,
    /// Custom
    Custom = 6,
}

// ============================================================================
// G-Buffer Encoding
// ============================================================================

/// Normal encoding method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum NormalEncoding {
    /// Spheremap transform
    #[default]
    Spheremap = 0,
    /// Octahedron encoding
    Octahedron = 1,
    /// Stereographic
    Stereographic = 2,
    /// Raw XYZ
    Raw = 3,
}

impl NormalEncoding {
    /// Encode normal (returns packed values)
    pub fn encode(&self, x: f32, y: f32, z: f32) -> [f32; 2] {
        match self {
            Self::Spheremap => {
                let p = ((z + 1.0) * 0.5).sqrt();
                if p > 0.0 {
                    [x / (2.0 * p) + 0.5, y / (2.0 * p) + 0.5]
                } else {
                    [0.5, 0.5]
                }
            }
            Self::Octahedron => {
                let sum = x.abs() + y.abs() + z.abs();
                let ox = x / sum;
                let oy = y / sum;
                if z < 0.0 {
                    let sx = if ox >= 0.0 { 1.0 } else { -1.0 };
                    let sy = if oy >= 0.0 { 1.0 } else { -1.0 };
                    [
                        (1.0 - oy.abs()) * sx * 0.5 + 0.5,
                        (1.0 - ox.abs()) * sy * 0.5 + 0.5,
                    ]
                } else {
                    [ox * 0.5 + 0.5, oy * 0.5 + 0.5]
                }
            }
            Self::Stereographic => {
                let scale = 1.0 / (z + 1.0);
                [x * scale * 0.5 + 0.5, y * scale * 0.5 + 0.5]
            }
            Self::Raw => [x * 0.5 + 0.5, y * 0.5 + 0.5],
        }
    }
}

/// Material encoding
#[derive(Clone, Copy, Debug, Default)]
pub struct MaterialEncoding {
    /// Roughness in R
    pub roughness_channel: u8,
    /// Metallic in G
    pub metallic_channel: u8,
    /// AO in B
    pub ao_channel: u8,
    /// Special in A
    pub special_channel: u8,
}

impl MaterialEncoding {
    /// Standard (R=roughness, G=metallic, B=ao, A=flags)
    pub const STANDARD: Self = Self {
        roughness_channel: 0,
        metallic_channel: 1,
        ao_channel: 2,
        special_channel: 3,
    };

    /// Pack material values
    pub fn pack(&self, roughness: f32, metallic: f32, ao: f32, flags: u8) -> [u8; 4] {
        let mut result = [0u8; 4];
        result[self.roughness_channel as usize] = (roughness * 255.0) as u8;
        result[self.metallic_channel as usize] = (metallic * 255.0) as u8;
        result[self.ao_channel as usize] = (ao * 255.0) as u8;
        result[self.special_channel as usize] = flags;
        result
    }
}

// ============================================================================
// Deferred Shading
// ============================================================================

/// Deferred shading settings
#[derive(Clone, Debug)]
pub struct DeferredShadingSettings {
    /// Enable deferred
    pub enabled: bool,
    /// G-Buffer layout
    pub layout: GBufferLayout,
    /// Light culling method
    pub light_culling: LightCullingMethod,
    /// Max lights per tile/cluster
    pub max_lights_per_tile: u32,
    /// Enable shadows
    pub shadows: bool,
    /// Enable ambient occlusion
    pub ambient_occlusion: bool,
}

impl DeferredShadingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            layout: GBufferLayout::Standard,
            light_culling: LightCullingMethod::Tiled,
            max_lights_per_tile: 256,
            shadows: true,
            ambient_occlusion: true,
        }
    }

    /// With clustered lighting
    pub fn with_clustered(mut self) -> Self {
        self.light_culling = LightCullingMethod::Clustered;
        self
    }

    /// With tiled lighting
    pub fn with_tiled(mut self) -> Self {
        self.light_culling = LightCullingMethod::Tiled;
        self
    }
}

impl Default for DeferredShadingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Light culling method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LightCullingMethod {
    /// No culling
    None = 0,
    /// Tiled (2D screen tiles)
    #[default]
    Tiled = 1,
    /// Clustered (3D froxels)
    Clustered = 2,
    /// GPU-driven
    GpuDriven = 3,
}

// ============================================================================
// Tiled Lighting
// ============================================================================

/// Tiled lighting settings
#[derive(Clone, Debug)]
pub struct TiledLightingSettings {
    /// Tile size (pixels)
    pub tile_size: u32,
    /// Max lights per tile
    pub max_lights_per_tile: u32,
    /// Use depth bounds
    pub use_depth_bounds: bool,
}

impl TiledLightingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            tile_size: 16,
            max_lights_per_tile: 256,
            use_depth_bounds: true,
        }
    }

    /// Tile count for resolution
    pub fn tile_count(&self, width: u32, height: u32) -> (u32, u32) {
        let tiles_x = (width + self.tile_size - 1) / self.tile_size;
        let tiles_y = (height + self.tile_size - 1) / self.tile_size;
        (tiles_x, tiles_y)
    }

    /// Total tiles
    pub fn total_tiles(&self, width: u32, height: u32) -> u32 {
        let (x, y) = self.tile_count(width, height);
        x * y
    }

    /// Light list buffer size (bytes)
    pub fn light_list_size(&self, width: u32, height: u32) -> u64 {
        self.total_tiles(width, height) as u64 * self.max_lights_per_tile as u64 * 4
    }
}

impl Default for TiledLightingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Tile data (for compute shader)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct TileData {
    /// Min depth
    pub min_depth: f32,
    /// Max depth
    pub max_depth: f32,
    /// Light count
    pub light_count: u32,
    /// Light list offset
    pub light_offset: u32,
}

// ============================================================================
// Clustered Lighting
// ============================================================================

/// Clustered lighting settings
#[derive(Clone, Debug)]
pub struct ClusteredLightingSettings {
    /// Tile size X (pixels)
    pub tile_size_x: u32,
    /// Tile size Y (pixels)
    pub tile_size_y: u32,
    /// Depth slices
    pub depth_slices: u32,
    /// Near plane
    pub near_plane: f32,
    /// Far plane
    pub far_plane: f32,
    /// Logarithmic slice distribution
    pub logarithmic: bool,
    /// Max lights per cluster
    pub max_lights_per_cluster: u32,
}

impl ClusteredLightingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            tile_size_x: 16,
            tile_size_y: 16,
            depth_slices: 24,
            near_plane: 0.1,
            far_plane: 1000.0,
            logarithmic: true,
            max_lights_per_cluster: 128,
        }
    }

    /// Cluster count
    pub fn cluster_count(&self, width: u32, height: u32) -> (u32, u32, u32) {
        let clusters_x = (width + self.tile_size_x - 1) / self.tile_size_x;
        let clusters_y = (height + self.tile_size_y - 1) / self.tile_size_y;
        (clusters_x, clusters_y, self.depth_slices)
    }

    /// Total clusters
    pub fn total_clusters(&self, width: u32, height: u32) -> u32 {
        let (x, y, z) = self.cluster_count(width, height);
        x * y * z
    }

    /// Slice depth at index
    pub fn slice_depth(&self, slice: u32) -> f32 {
        let t = slice as f32 / self.depth_slices as f32;
        if self.logarithmic {
            self.near_plane * (self.far_plane / self.near_plane).powf(t)
        } else {
            self.near_plane + t * (self.far_plane - self.near_plane)
        }
    }

    /// Cluster index to world position
    pub fn cluster_to_world(
        &self,
        cluster_x: u32,
        cluster_y: u32,
        slice: u32,
        screen_width: u32,
        screen_height: u32,
    ) -> ([f32; 3], [f32; 3]) {
        let x0 = cluster_x as f32 * self.tile_size_x as f32 / screen_width as f32;
        let y0 = cluster_y as f32 * self.tile_size_y as f32 / screen_height as f32;
        let x1 = (cluster_x + 1) as f32 * self.tile_size_x as f32 / screen_width as f32;
        let y1 = (cluster_y + 1) as f32 * self.tile_size_y as f32 / screen_height as f32;
        let z0 = self.slice_depth(slice);
        let z1 = self.slice_depth(slice + 1);

        ([x0, y0, z0], [x1, y1, z1])
    }
}

impl Default for ClusteredLightingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Cluster data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ClusterData {
    /// AABB min
    pub aabb_min: [f32; 3],
    /// Light count
    pub light_count: u32,
    /// AABB max
    pub aabb_max: [f32; 3],
    /// Light offset
    pub light_offset: u32,
}

/// Cluster GPU params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ClusterGpuParams {
    /// Cluster dimensions
    pub cluster_dims: [u32; 4],
    /// Near/far planes
    pub depth_params: [f32; 4],
    /// Tile size
    pub tile_size: [f32; 4],
}

// ============================================================================
// Light Indexing
// ============================================================================

/// Light index buffer
#[derive(Clone, Debug)]
pub struct LightIndexBuffer {
    /// Light indices
    pub indices: Vec<u32>,
    /// Max lights
    pub max_lights: u32,
    /// Current count
    pub count: u32,
}

impl LightIndexBuffer {
    /// Creates buffer
    pub fn new(max_lights: u32) -> Self {
        Self {
            indices: Vec::with_capacity(max_lights as usize),
            max_lights,
            count: 0,
        }
    }

    /// Clear
    pub fn clear(&mut self) {
        self.indices.clear();
        self.count = 0;
    }

    /// Add light
    pub fn add(&mut self, index: u32) -> bool {
        if self.count < self.max_lights {
            self.indices.push(index);
            self.count += 1;
            true
        } else {
            false
        }
    }

    /// As bytes
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self.indices.as_ptr() as *const u8,
                self.indices.len() * core::mem::size_of::<u32>(),
            )
        }
    }
}

// ============================================================================
// Deferred Light Types
// ============================================================================

/// Deferred light GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DeferredLightGpu {
    /// Position
    pub position: [f32; 3],
    /// Range
    pub range: f32,
    /// Color
    pub color: [f32; 3],
    /// Intensity
    pub intensity: f32,
    /// Direction (for spot)
    pub direction: [f32; 3],
    /// Spot angles (inner, outer)
    pub spot_angles: f32,
    /// Light type
    pub light_type: u32,
    /// Shadow index
    pub shadow_index: i32,
    /// Padding
    pub _padding: [f32; 2],
}

/// Light volume type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LightVolumeType {
    /// Sphere (point light)
    #[default]
    Sphere = 0,
    /// Cone (spot light)
    Cone = 1,
    /// Fullscreen (directional)
    Fullscreen = 2,
}

// ============================================================================
// G-Buffer Pass
// ============================================================================

/// G-Buffer pass settings
#[derive(Clone, Debug)]
pub struct GBufferPassSettings {
    /// Clear color
    pub clear_color: [f32; 4],
    /// Clear depth
    pub clear_depth: f32,
    /// Clear stencil
    pub clear_stencil: u8,
    /// Alpha test threshold
    pub alpha_test_threshold: f32,
    /// Two-sided rendering
    pub two_sided: bool,
}

impl GBufferPassSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            clear_color: [0.0, 0.0, 0.0, 0.0],
            clear_depth: 1.0,
            clear_stencil: 0,
            alpha_test_threshold: 0.5,
            two_sided: false,
        }
    }
}

impl Default for GBufferPassSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Lighting pass settings
#[derive(Clone, Debug)]
pub struct LightingPassSettings {
    /// Enable direct lighting
    pub direct_lighting: bool,
    /// Enable indirect lighting
    pub indirect_lighting: bool,
    /// Enable ambient
    pub ambient: bool,
    /// Ambient color
    pub ambient_color: [f32; 3],
    /// Ambient intensity
    pub ambient_intensity: f32,
}

impl LightingPassSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            direct_lighting: true,
            indirect_lighting: true,
            ambient: true,
            ambient_color: [0.03, 0.03, 0.03],
            ambient_intensity: 1.0,
        }
    }
}

impl Default for LightingPassSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Deferred rendering statistics
#[derive(Clone, Debug, Default)]
pub struct DeferredStats {
    /// G-Buffer width
    pub gbuffer_width: u32,
    /// G-Buffer height
    pub gbuffer_height: u32,
    /// G-Buffer memory (bytes)
    pub gbuffer_memory: u64,
    /// Light count
    pub light_count: u32,
    /// Visible lights (after culling)
    pub visible_lights: u32,
    /// Tile/cluster count
    pub tile_count: u32,
    /// Max lights in tile
    pub max_lights_in_tile: u32,
    /// GPU time (microseconds)
    pub gpu_time_us: u64,
}
