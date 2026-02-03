//! Decal Rendering Types for Lumina
//!
//! This module provides decal rendering infrastructure including
//! projected decals, deferred decals, and decal atlases.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Decal Handles
// ============================================================================

/// Decal handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DecalHandle(pub u64);

impl DecalHandle {
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

impl Default for DecalHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Decal atlas handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DecalAtlasHandle(pub u64);

impl DecalAtlasHandle {
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

impl Default for DecalAtlasHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Decal batch handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DecalBatchHandle(pub u64);

impl DecalBatchHandle {
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

impl Default for DecalBatchHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Decal Types
// ============================================================================

/// Decal create info
#[derive(Clone, Debug)]
pub struct DecalCreateInfo {
    /// Decal type
    pub decal_type: DecalType,
    /// Projection type
    pub projection: DecalProjection,
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Size (half extents)
    pub size: [f32; 3],
    /// Material index
    pub material: u32,
    /// Fade settings
    pub fade: DecalFade,
    /// Sort order
    pub sort_order: i32,
    /// Layers mask
    pub layers: u32,
}

impl DecalCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            decal_type: DecalType::Deferred,
            projection: DecalProjection::Box,
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            size: [1.0, 1.0, 1.0],
            material: 0,
            fade: DecalFade::default(),
            sort_order: 0,
            layers: u32::MAX,
        }
    }

    /// At position
    pub fn at(position: [f32; 3]) -> Self {
        Self {
            position,
            ..Self::new()
        }
    }

    /// With size
    pub fn with_size(mut self, x: f32, y: f32, z: f32) -> Self {
        self.size = [x, y, z];
        self
    }

    /// With rotation
    pub fn with_rotation(mut self, x: f32, y: f32, z: f32, w: f32) -> Self {
        self.rotation = [x, y, z, w];
        self
    }

    /// With material
    pub fn with_material(mut self, material: u32) -> Self {
        self.material = material;
        self
    }

    /// With projection
    pub fn with_projection(mut self, projection: DecalProjection) -> Self {
        self.projection = projection;
        self
    }

    /// With fade
    pub fn with_fade(mut self, fade: DecalFade) -> Self {
        self.fade = fade;
        self
    }

    /// With sort order
    pub fn with_sort_order(mut self, order: i32) -> Self {
        self.sort_order = order;
        self
    }
}

impl Default for DecalCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Decal type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DecalType {
    /// Deferred (modifies G-buffer)
    #[default]
    Deferred = 0,
    /// Forward (rendered with objects)
    Forward = 1,
    /// Screen space (post-process)
    ScreenSpace = 2,
}

/// Decal projection type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DecalProjection {
    /// Box projection
    #[default]
    Box = 0,
    /// Sphere projection
    Sphere = 1,
    /// Cylinder projection
    Cylinder = 2,
    /// Planar projection
    Planar = 3,
}

// ============================================================================
// Decal Fade
// ============================================================================

/// Decal fade settings
#[derive(Clone, Copy, Debug)]
pub struct DecalFade {
    /// Angle fade start (degrees)
    pub angle_start: f32,
    /// Angle fade end (degrees)
    pub angle_end: f32,
    /// Distance fade start
    pub distance_start: f32,
    /// Distance fade end
    pub distance_end: f32,
    /// Lifetime (seconds, 0 = infinite)
    pub lifetime: f32,
    /// Current age (seconds)
    pub age: f32,
}

impl DecalFade {
    /// Creates fade
    pub fn new() -> Self {
        Self {
            angle_start: 70.0,
            angle_end: 90.0,
            distance_start: 50.0,
            distance_end: 100.0,
            lifetime: 0.0,
            age: 0.0,
        }
    }

    /// No fade
    pub fn none() -> Self {
        Self {
            angle_start: 90.0,
            angle_end: 90.0,
            distance_start: 1000.0,
            distance_end: 1000.0,
            lifetime: 0.0,
            age: 0.0,
        }
    }

    /// Timed decal
    pub fn timed(lifetime: f32) -> Self {
        Self {
            lifetime,
            ..Self::new()
        }
    }

    /// Calculate angle fade factor
    pub fn angle_factor(&self, angle: f32) -> f32 {
        if angle <= self.angle_start {
            1.0
        } else if angle >= self.angle_end {
            0.0
        } else {
            1.0 - (angle - self.angle_start) / (self.angle_end - self.angle_start)
        }
    }

    /// Calculate distance fade factor
    pub fn distance_factor(&self, distance: f32) -> f32 {
        if distance <= self.distance_start {
            1.0
        } else if distance >= self.distance_end {
            0.0
        } else {
            1.0 - (distance - self.distance_start) / (self.distance_end - self.distance_start)
        }
    }

    /// Calculate lifetime fade factor
    pub fn lifetime_factor(&self) -> f32 {
        if self.lifetime <= 0.0 {
            1.0
        } else if self.age >= self.lifetime {
            0.0
        } else {
            1.0 - self.age / self.lifetime
        }
    }

    /// Is expired
    pub fn is_expired(&self) -> bool {
        self.lifetime > 0.0 && self.age >= self.lifetime
    }
}

impl Default for DecalFade {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Decal Material
// ============================================================================

/// Decal material
#[derive(Clone, Debug)]
pub struct DecalMaterial {
    /// Albedo texture
    pub albedo: u64,
    /// Normal texture
    pub normal: u64,
    /// Metallic-roughness texture
    pub metallic_roughness: u64,
    /// Emission texture
    pub emission: u64,
    /// Blend mode
    pub blend_mode: DecalBlendMode,
    /// Color tint
    pub color: [f32; 4],
    /// Emission intensity
    pub emission_intensity: f32,
    /// Normal strength
    pub normal_strength: f32,
    /// Metallic override
    pub metallic: f32,
    /// Roughness override
    pub roughness: f32,
    /// Channels to modify
    pub channels: DecalChannels,
}

impl DecalMaterial {
    /// Creates material
    pub fn new() -> Self {
        Self {
            albedo: 0,
            normal: 0,
            metallic_roughness: 0,
            emission: 0,
            blend_mode: DecalBlendMode::Normal,
            color: [1.0, 1.0, 1.0, 1.0],
            emission_intensity: 0.0,
            normal_strength: 1.0,
            metallic: 0.0,
            roughness: 0.5,
            channels: DecalChannels::ALL,
        }
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

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }

    /// With blend mode
    pub fn with_blend_mode(mut self, mode: DecalBlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// With channels
    pub fn with_channels(mut self, channels: DecalChannels) -> Self {
        self.channels = channels;
        self
    }
}

impl Default for DecalMaterial {
    fn default() -> Self {
        Self::new()
    }
}

/// Decal blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DecalBlendMode {
    /// Normal (alpha blend)
    #[default]
    Normal = 0,
    /// Multiply
    Multiply = 1,
    /// Additive
    Additive = 2,
    /// Screen
    Screen = 3,
    /// Overlay
    Overlay = 4,
}

/// Decal channels to modify
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DecalChannels(pub u32);

impl DecalChannels {
    /// None
    pub const NONE: Self = Self(0);
    /// Albedo
    pub const ALBEDO: Self = Self(1 << 0);
    /// Normal
    pub const NORMAL: Self = Self(1 << 1);
    /// Metallic
    pub const METALLIC: Self = Self(1 << 2);
    /// Roughness
    pub const ROUGHNESS: Self = Self(1 << 3);
    /// Emission
    pub const EMISSION: Self = Self(1 << 4);
    /// All channels
    pub const ALL: Self = Self(0x1F);

    /// Has channel
    pub const fn has(&self, channel: Self) -> bool {
        (self.0 & channel.0) != 0
    }
}

impl Default for DecalChannels {
    fn default() -> Self {
        Self::ALL
    }
}

impl core::ops::BitOr for DecalChannels {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

// ============================================================================
// Decal Atlas
// ============================================================================

/// Decal atlas create info
#[derive(Clone, Debug)]
pub struct DecalAtlasCreateInfo {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Layers
    pub layers: u32,
    /// Format
    pub format: DecalAtlasFormat,
    /// Include normal atlas
    pub has_normal: bool,
    /// Include material atlas
    pub has_material: bool,
}

impl DecalAtlasCreateInfo {
    /// Creates info
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            layers: 1,
            format: DecalAtlasFormat::Bc7,
            has_normal: true,
            has_material: false,
        }
    }

    /// 2K atlas
    pub fn atlas_2k() -> Self {
        Self::new(2048, 2048)
    }

    /// 4K atlas
    pub fn atlas_4k() -> Self {
        Self::new(4096, 4096)
    }

    /// With layers
    pub fn with_layers(mut self, layers: u32) -> Self {
        self.layers = layers;
        self
    }

    /// Memory size (bytes)
    pub fn memory_size(&self) -> u64 {
        let base = self.format.bytes_per_block() as u64
            * (self.width as u64 / 4)
            * (self.height as u64 / 4)
            * self.layers as u64;
        let normal = if self.has_normal { base } else { 0 };
        let material = if self.has_material { base } else { 0 };
        base + normal + material
    }
}

impl Default for DecalAtlasCreateInfo {
    fn default() -> Self {
        Self::atlas_2k()
    }
}

/// Decal atlas format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DecalAtlasFormat {
    /// BC1 (DXT1)
    Bc1 = 0,
    /// BC3 (DXT5)
    Bc3 = 1,
    /// BC7
    #[default]
    Bc7 = 2,
    /// RGBA8
    Rgba8 = 3,
}

impl DecalAtlasFormat {
    /// Bytes per block
    pub const fn bytes_per_block(&self) -> u32 {
        match self {
            Self::Bc1 => 8,
            Self::Bc3 | Self::Bc7 => 16,
            Self::Rgba8 => 64, // 4x4 block = 16 pixels * 4 bytes
        }
    }
}

/// Decal atlas entry
#[derive(Clone, Copy, Debug)]
pub struct DecalAtlasEntry {
    /// UV rect (x, y, width, height)
    pub uv_rect: [f32; 4],
    /// Layer
    pub layer: u32,
    /// Decal ID
    pub decal_id: u32,
}

impl DecalAtlasEntry {
    /// Creates entry
    pub fn new(x: f32, y: f32, width: f32, height: f32, layer: u32) -> Self {
        Self {
            uv_rect: [x, y, width, height],
            layer,
            decal_id: 0,
        }
    }
}

// ============================================================================
// Decal Batch
// ============================================================================

/// Decal batch create info
#[derive(Clone, Debug)]
pub struct DecalBatchCreateInfo {
    /// Max decals
    pub max_decals: u32,
    /// Atlas
    pub atlas: DecalAtlasHandle,
    /// Dynamic (allow updates)
    pub dynamic: bool,
}

impl DecalBatchCreateInfo {
    /// Creates info
    pub fn new(max_decals: u32) -> Self {
        Self {
            max_decals,
            atlas: DecalAtlasHandle::NULL,
            dynamic: true,
        }
    }

    /// Static batch
    pub fn static_batch(max_decals: u32) -> Self {
        Self {
            max_decals,
            dynamic: false,
            ..Self::new(max_decals)
        }
    }

    /// With atlas
    pub fn with_atlas(mut self, atlas: DecalAtlasHandle) -> Self {
        self.atlas = atlas;
        self
    }
}

impl Default for DecalBatchCreateInfo {
    fn default() -> Self {
        Self::new(1024)
    }
}

/// Decal instance (for batching)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DecalInstance {
    /// World matrix row 0
    pub world_0: [f32; 4],
    /// World matrix row 1
    pub world_1: [f32; 4],
    /// World matrix row 2
    pub world_2: [f32; 4],
    /// UV rect
    pub uv_rect: [f32; 4],
    /// Color
    pub color: [f32; 4],
    /// Fade params (angle_start, angle_end, lifetime_factor, layer)
    pub fade_params: [f32; 4],
}

impl DecalInstance {
    /// Creates instance
    pub fn new() -> Self {
        Self {
            world_0: [1.0, 0.0, 0.0, 0.0],
            world_1: [0.0, 1.0, 0.0, 0.0],
            world_2: [0.0, 0.0, 1.0, 0.0],
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
            fade_params: [70.0, 90.0, 1.0, 0.0],
        }
    }

    /// With position
    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.world_0[3] = x;
        self.world_1[3] = y;
        self.world_2[3] = z;
        self
    }

    /// With color
    pub fn with_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.color = [r, g, b, a];
        self
    }

    /// With UV rect
    pub fn with_uv(mut self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.uv_rect = [x, y, w, h];
        self
    }
}

// ============================================================================
// Decal GPU Data
// ============================================================================

/// Decal GPU data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DecalGpuData {
    /// Inverse world matrix row 0
    pub inv_world_0: [f32; 4],
    /// Inverse world matrix row 1
    pub inv_world_1: [f32; 4],
    /// Inverse world matrix row 2
    pub inv_world_2: [f32; 4],
    /// UV rect
    pub uv_rect: [f32; 4],
    /// Color
    pub color: [f32; 4],
    /// Parameters (blend mode, channels, angle_fade, dist_fade)
    pub params: [f32; 4],
}

/// Decal render params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DecalRenderParams {
    /// Decal count
    pub decal_count: u32,
    /// Atlas size
    pub atlas_size: [f32; 2],
    /// Padding
    pub _padding: f32,
}

// ============================================================================
// Decal Projector
// ============================================================================

/// Decal projector
#[derive(Clone, Debug)]
pub struct DecalProjector {
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Size
    pub size: [f32; 3],
    /// Projection matrix
    pub projection: [[f32; 4]; 4],
    /// View matrix
    pub view: [[f32; 4]; 4],
}

impl DecalProjector {
    /// Creates projector
    pub fn new(position: [f32; 3], size: [f32; 3]) -> Self {
        Self {
            position,
            rotation: [0.0, 0.0, 0.0, 1.0],
            size,
            projection: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            view: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// World to decal space matrix
    pub fn world_to_decal(&self) -> [[f32; 4]; 4] {
        // Simplified - combine view and projection
        let mut result = [[0.0f32; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += self.projection[i][k] * self.view[k][j];
                }
            }
        }
        result
    }
}

impl Default for DecalProjector {
    fn default() -> Self {
        Self::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Decal statistics
#[derive(Clone, Debug, Default)]
pub struct DecalStats {
    /// Total decals
    pub decal_count: u32,
    /// Visible decals
    pub visible_decals: u32,
    /// Batches
    pub batch_count: u32,
    /// Draw calls
    pub draw_calls: u32,
    /// GPU time (microseconds)
    pub gpu_time_us: u64,
}
