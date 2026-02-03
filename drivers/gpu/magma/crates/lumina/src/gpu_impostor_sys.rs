//! GPU Impostor Rendering System for Lumina
//!
//! This module provides GPU-accelerated impostor (billboard) rendering
//! for efficient LOD, distant objects, and vegetation rendering.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Impostor System Handles
// ============================================================================

/// GPU impostor system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuImpostorSystemHandle(pub u64);

impl GpuImpostorSystemHandle {
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

impl Default for GpuImpostorSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Impostor atlas handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImpostorAtlasHandle(pub u64);

impl ImpostorAtlasHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ImpostorAtlasHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Impostor instance handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImpostorInstanceHandle(pub u64);

impl ImpostorInstanceHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ImpostorInstanceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Impostor group handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ImpostorGroupHandle(pub u64);

impl ImpostorGroupHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ImpostorGroupHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Impostor System Creation
// ============================================================================

/// GPU impostor system create info
#[derive(Clone, Debug)]
pub struct GpuImpostorSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max atlases
    pub max_atlases: u32,
    /// Max instances
    pub max_instances: u32,
    /// Max groups
    pub max_groups: u32,
    /// Features
    pub features: ImpostorFeatures,
}

impl GpuImpostorSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_atlases: 64,
            max_instances: 100000,
            max_groups: 256,
            features: ImpostorFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max atlases
    pub fn with_max_atlases(mut self, count: u32) -> Self {
        self.max_atlases = count;
        self
    }

    /// With max instances
    pub fn with_max_instances(mut self, count: u32) -> Self {
        self.max_instances = count;
        self
    }

    /// With max groups
    pub fn with_max_groups(mut self, count: u32) -> Self {
        self.max_groups = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: ImpostorFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard
    pub fn standard() -> Self {
        Self::new()
    }

    /// Massive vegetation
    pub fn massive() -> Self {
        Self::new()
            .with_max_atlases(256)
            .with_max_instances(1000000)
            .with_max_groups(1024)
    }

    /// Mobile
    pub fn mobile() -> Self {
        Self::new()
            .with_max_atlases(16)
            .with_max_instances(10000)
            .with_max_groups(64)
    }
}

impl Default for GpuImpostorSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Impostor features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ImpostorFeatures: u32 {
        /// None
        const NONE = 0;
        /// Octahedral impostors
        const OCTAHEDRAL = 1 << 0;
        /// Billboard impostors
        const BILLBOARD = 1 << 1;
        /// Parallax mapping
        const PARALLAX = 1 << 2;
        /// Normal mapping
        const NORMAL_MAP = 1 << 3;
        /// Wind animation
        const WIND = 1 << 4;
        /// GPU instancing
        const INSTANCING = 1 << 5;
        /// LOD crossfade
        const CROSSFADE = 1 << 6;
        /// Shadow casting
        const SHADOWS = 1 << 7;
        /// All
        const ALL = 0xFF;
    }
}

impl Default for ImpostorFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Impostor Atlas
// ============================================================================

/// Impostor atlas create info
#[derive(Clone, Debug)]
pub struct ImpostorAtlasCreateInfo {
    /// Name
    pub name: String,
    /// Atlas type
    pub atlas_type: ImpostorAtlasType,
    /// Resolution per frame
    pub frame_resolution: u32,
    /// Horizontal frames
    pub frames_x: u32,
    /// Vertical frames
    pub frames_y: u32,
    /// Channels
    pub channels: ImpostorChannels,
    /// Source mesh (for baking)
    pub source_mesh: u64,
}

impl ImpostorAtlasCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            atlas_type: ImpostorAtlasType::Octahedral,
            frame_resolution: 256,
            frames_x: 8,
            frames_y: 8,
            channels: ImpostorChannels::all(),
            source_mesh: 0,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With atlas type
    pub fn with_type(mut self, atlas_type: ImpostorAtlasType) -> Self {
        self.atlas_type = atlas_type;
        self
    }

    /// With frame resolution
    pub fn with_resolution(mut self, resolution: u32) -> Self {
        self.frame_resolution = resolution;
        self
    }

    /// With frame count
    pub fn with_frames(mut self, x: u32, y: u32) -> Self {
        self.frames_x = x;
        self.frames_y = y;
        self
    }

    /// With channels
    pub fn with_channels(mut self, channels: ImpostorChannels) -> Self {
        self.channels = channels;
        self
    }

    /// With source mesh
    pub fn from_mesh(mut self, mesh: u64) -> Self {
        self.source_mesh = mesh;
        self
    }

    /// Billboard preset
    pub fn billboard() -> Self {
        Self::new()
            .with_type(ImpostorAtlasType::Billboard)
            .with_frames(1, 1)
    }

    /// Octahedral 8x8 preset
    pub fn octahedral_8x8() -> Self {
        Self::new()
            .with_type(ImpostorAtlasType::Octahedral)
            .with_frames(8, 8)
    }

    /// Octahedral 16x16 preset (high quality)
    pub fn octahedral_16x16() -> Self {
        Self::new()
            .with_type(ImpostorAtlasType::Octahedral)
            .with_frames(16, 16)
            .with_resolution(512)
    }

    /// Flipbook preset
    pub fn flipbook(frames: u32) -> Self {
        Self::new()
            .with_type(ImpostorAtlasType::Flipbook)
            .with_frames(frames, 1)
    }

    /// Total atlas size
    pub fn atlas_size(&self) -> (u32, u32) {
        (
            self.frame_resolution * self.frames_x,
            self.frame_resolution * self.frames_y,
        )
    }
}

impl Default for ImpostorAtlasCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Impostor atlas type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ImpostorAtlasType {
    /// Simple billboard (1 view)
    Billboard = 0,
    /// Octahedral mapping
    #[default]
    Octahedral = 1,
    /// Hemisphere mapping
    Hemisphere = 2,
    /// Flipbook (animation)
    Flipbook = 3,
    /// Full sphere
    FullSphere = 4,
}

bitflags::bitflags! {
    /// Impostor channels
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ImpostorChannels: u32 {
        /// None
        const NONE = 0;
        /// Albedo/diffuse
        const ALBEDO = 1 << 0;
        /// Normal map
        const NORMAL = 1 << 1;
        /// Depth
        const DEPTH = 1 << 2;
        /// Emission
        const EMISSION = 1 << 3;
        /// Alpha mask
        const ALPHA = 1 << 4;
        /// All
        const ALL = 0x1F;
    }
}

impl Default for ImpostorChannels {
    fn default() -> Self {
        Self::ALBEDO | Self::NORMAL | Self::ALPHA
    }
}

// ============================================================================
// Impostor Instance
// ============================================================================

/// Impostor instance create info
#[derive(Clone, Debug)]
pub struct ImpostorInstanceCreateInfo {
    /// Atlas
    pub atlas: ImpostorAtlasHandle,
    /// Position
    pub position: [f32; 3],
    /// Scale
    pub scale: [f32; 3],
    /// Rotation (Y axis, radians)
    pub rotation: f32,
    /// Tint color
    pub tint: [f32; 4],
    /// Wind influence
    pub wind_influence: f32,
    /// LOD bias
    pub lod_bias: f32,
}

impl ImpostorInstanceCreateInfo {
    /// Creates new info
    pub fn new(atlas: ImpostorAtlasHandle) -> Self {
        Self {
            atlas,
            position: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            rotation: 0.0,
            tint: [1.0, 1.0, 1.0, 1.0],
            wind_influence: 1.0,
            lod_bias: 0.0,
        }
    }

    /// At position
    pub fn at_position(mut self, position: [f32; 3]) -> Self {
        self.position = position;
        self
    }

    /// With scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = [scale, scale, scale];
        self
    }

    /// With non-uniform scale
    pub fn with_scale_xyz(mut self, scale: [f32; 3]) -> Self {
        self.scale = scale;
        self
    }

    /// With rotation
    pub fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// With tint
    pub fn with_tint(mut self, tint: [f32; 4]) -> Self {
        self.tint = tint;
        self
    }

    /// With wind influence
    pub fn with_wind(mut self, influence: f32) -> Self {
        self.wind_influence = influence;
        self
    }

    /// With LOD bias
    pub fn with_lod_bias(mut self, bias: f32) -> Self {
        self.lod_bias = bias;
        self
    }
}

impl Default for ImpostorInstanceCreateInfo {
    fn default() -> Self {
        Self::new(ImpostorAtlasHandle::NULL)
    }
}

// ============================================================================
// Impostor Group
// ============================================================================

/// Impostor group create info
#[derive(Clone, Debug)]
pub struct ImpostorGroupCreateInfo {
    /// Name
    pub name: String,
    /// Atlas
    pub atlas: ImpostorAtlasHandle,
    /// Instances
    pub instances: Vec<ImpostorGroupInstance>,
    /// Culling settings
    pub culling: ImpostorCulling,
    /// LOD settings
    pub lod: ImpostorLodSettings,
}

impl ImpostorGroupCreateInfo {
    /// Creates new info
    pub fn new(atlas: ImpostorAtlasHandle) -> Self {
        Self {
            name: String::new(),
            atlas,
            instances: Vec::new(),
            culling: ImpostorCulling::default(),
            lod: ImpostorLodSettings::default(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add instance
    pub fn add_instance(mut self, instance: ImpostorGroupInstance) -> Self {
        self.instances.push(instance);
        self
    }

    /// With instances
    pub fn with_instances(mut self, instances: Vec<ImpostorGroupInstance>) -> Self {
        self.instances = instances;
        self
    }

    /// With culling
    pub fn with_culling(mut self, culling: ImpostorCulling) -> Self {
        self.culling = culling;
        self
    }

    /// With LOD settings
    pub fn with_lod(mut self, lod: ImpostorLodSettings) -> Self {
        self.lod = lod;
        self
    }
}

impl Default for ImpostorGroupCreateInfo {
    fn default() -> Self {
        Self::new(ImpostorAtlasHandle::NULL)
    }
}

/// Impostor group instance
#[derive(Clone, Copy, Debug)]
pub struct ImpostorGroupInstance {
    /// Position
    pub position: [f32; 3],
    /// Scale
    pub scale: f32,
    /// Rotation
    pub rotation: f32,
    /// Tint
    pub tint: [f32; 4],
}

impl ImpostorGroupInstance {
    /// Creates new instance
    pub const fn new(position: [f32; 3]) -> Self {
        Self {
            position,
            scale: 1.0,
            rotation: 0.0,
            tint: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// With scale
    pub const fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// With rotation
    pub const fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// With tint
    pub const fn with_tint(mut self, tint: [f32; 4]) -> Self {
        self.tint = tint;
        self
    }
}

impl Default for ImpostorGroupInstance {
    fn default() -> Self {
        Self::new([0.0, 0.0, 0.0])
    }
}

// ============================================================================
// Culling Settings
// ============================================================================

/// Impostor culling settings
#[derive(Clone, Copy, Debug)]
pub struct ImpostorCulling {
    /// Frustum culling
    pub frustum: bool,
    /// Occlusion culling
    pub occlusion: bool,
    /// Distance culling
    pub distance: bool,
    /// Max distance
    pub max_distance: f32,
    /// Screen size threshold
    pub screen_threshold: f32,
}

impl ImpostorCulling {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            frustum: true,
            occlusion: true,
            distance: true,
            max_distance: 1000.0,
            screen_threshold: 1.0,
        }
    }

    /// No culling
    pub const fn none() -> Self {
        Self {
            frustum: false,
            occlusion: false,
            distance: false,
            max_distance: 0.0,
            screen_threshold: 0.0,
        }
    }

    /// Frustum only
    pub const fn frustum_only() -> Self {
        Self {
            frustum: true,
            occlusion: false,
            distance: false,
            max_distance: 0.0,
            screen_threshold: 0.0,
        }
    }

    /// With max distance
    pub const fn with_max_distance(mut self, distance: f32) -> Self {
        self.max_distance = distance;
        self
    }
}

impl Default for ImpostorCulling {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LOD Settings
// ============================================================================

/// Impostor LOD settings
#[derive(Clone, Copy, Debug)]
pub struct ImpostorLodSettings {
    /// LOD enabled
    pub enabled: bool,
    /// Mesh to impostor distance
    pub mesh_distance: f32,
    /// High to low impostor distance
    pub lod_distance: f32,
    /// Crossfade range
    pub crossfade_range: f32,
    /// Dither crossfade
    pub dither: bool,
}

impl ImpostorLodSettings {
    /// Default settings
    pub const fn new() -> Self {
        Self {
            enabled: true,
            mesh_distance: 50.0,
            lod_distance: 200.0,
            crossfade_range: 5.0,
            dither: true,
        }
    }

    /// Disabled
    pub const fn disabled() -> Self {
        Self {
            enabled: false,
            mesh_distance: 0.0,
            lod_distance: 0.0,
            crossfade_range: 0.0,
            dither: false,
        }
    }

    /// Close range
    pub const fn close_range() -> Self {
        Self {
            enabled: true,
            mesh_distance: 20.0,
            lod_distance: 100.0,
            crossfade_range: 3.0,
            dither: true,
        }
    }

    /// Far range
    pub const fn far_range() -> Self {
        Self {
            enabled: true,
            mesh_distance: 100.0,
            lod_distance: 500.0,
            crossfade_range: 10.0,
            dither: true,
        }
    }

    /// With mesh distance
    pub const fn with_mesh_distance(mut self, distance: f32) -> Self {
        self.mesh_distance = distance;
        self
    }

    /// With LOD distance
    pub const fn with_lod_distance(mut self, distance: f32) -> Self {
        self.lod_distance = distance;
        self
    }
}

impl Default for ImpostorLodSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Wind Settings
// ============================================================================

/// Impostor wind settings
#[derive(Clone, Copy, Debug)]
pub struct ImpostorWindSettings {
    /// Wind direction
    pub direction: [f32; 3],
    /// Wind strength
    pub strength: f32,
    /// Turbulence
    pub turbulence: f32,
    /// Frequency
    pub frequency: f32,
    /// Amplitude
    pub amplitude: f32,
}

impl ImpostorWindSettings {
    /// No wind
    pub const fn none() -> Self {
        Self {
            direction: [0.0, 0.0, 0.0],
            strength: 0.0,
            turbulence: 0.0,
            frequency: 0.0,
            amplitude: 0.0,
        }
    }

    /// Light breeze
    pub const fn light() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            strength: 1.0,
            turbulence: 0.2,
            frequency: 1.0,
            amplitude: 0.05,
        }
    }

    /// Moderate wind
    pub const fn moderate() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            strength: 3.0,
            turbulence: 0.5,
            frequency: 1.5,
            amplitude: 0.1,
        }
    }

    /// Strong wind
    pub const fn strong() -> Self {
        Self {
            direction: [1.0, 0.0, 0.0],
            strength: 8.0,
            turbulence: 1.0,
            frequency: 2.0,
            amplitude: 0.2,
        }
    }

    /// With direction
    pub const fn with_direction(mut self, direction: [f32; 3]) -> Self {
        self.direction = direction;
        self
    }

    /// With strength
    pub const fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength;
        self
    }
}

impl Default for ImpostorWindSettings {
    fn default() -> Self {
        Self::light()
    }
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU impostor instance
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuImpostorInstance {
    /// Position
    pub position: [f32; 3],
    /// Scale
    pub scale: f32,
    /// Rotation
    pub rotation: f32,
    /// Atlas index
    pub atlas_index: u32,
    /// Wind influence
    pub wind_influence: f32,
    /// LOD factor
    pub lod_factor: f32,
    /// Tint
    pub tint: [f32; 4],
}

/// GPU impostor constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuImpostorConstants {
    /// Time
    pub time: f32,
    /// Frames X
    pub frames_x: u32,
    /// Frames Y
    pub frames_y: u32,
    /// Atlas type
    pub atlas_type: u32,
    /// Camera position
    pub camera_position: [f32; 3],
    /// Crossfade range
    pub crossfade_range: f32,
    /// Wind direction
    pub wind_direction: [f32; 3],
    /// Wind strength
    pub wind_strength: f32,
    /// Wind turbulence
    pub wind_turbulence: f32,
    /// Wind frequency
    pub wind_frequency: f32,
    /// Wind amplitude
    pub wind_amplitude: f32,
    /// Instance count
    pub instance_count: u32,
}

/// GPU impostor atlas info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuImpostorAtlasInfo {
    /// Atlas size
    pub atlas_size: [f32; 2],
    /// Frame size
    pub frame_size: [f32; 2],
    /// Frames X
    pub frames_x: u32,
    /// Frames Y
    pub frames_y: u32,
    /// Atlas type
    pub atlas_type: u32,
    /// Channels
    pub channels: u32,
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU impostor statistics
#[derive(Clone, Debug, Default)]
pub struct GpuImpostorStats {
    /// Active atlases
    pub active_atlases: u32,
    /// Total instances
    pub total_instances: u32,
    /// Visible instances
    pub visible_instances: u32,
    /// Culled instances
    pub culled_instances: u32,
    /// Draw calls
    pub draw_calls: u32,
    /// GPU time (ms)
    pub gpu_time_ms: f32,
}

impl GpuImpostorStats {
    /// Cull ratio
    pub fn cull_ratio(&self) -> f32 {
        if self.total_instances == 0 {
            0.0
        } else {
            self.culled_instances as f32 / self.total_instances as f32
        }
    }
}
