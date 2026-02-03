//! GPU Decal System Types for Lumina
//!
//! This module provides GPU-accelerated deferred decal rendering
//! infrastructure for dynamic surface decoration.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Decal System Handles
// ============================================================================

/// GPU decal system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuDecalSystemHandle(pub u64);

impl GpuDecalSystemHandle {
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

impl Default for GpuDecalSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Decal handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DecalHandle(pub u64);

impl DecalHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
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
}

impl Default for DecalAtlasHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Decal material handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DecalMaterialHandle(pub u64);

impl DecalMaterialHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for DecalMaterialHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Decal System Creation
// ============================================================================

/// GPU decal system create info
#[derive(Clone, Debug)]
pub struct GpuDecalSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max decals
    pub max_decals: u32,
    /// Max decal materials
    pub max_materials: u32,
    /// Decal method
    pub decal_method: DecalMethod,
    /// Features
    pub features: DecalFeatures,
}

impl GpuDecalSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_decals: 4096,
            max_materials: 256,
            decal_method: DecalMethod::DBuffer,
            features: DecalFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max decals
    pub fn with_max_decals(mut self, count: u32) -> Self {
        self.max_decals = count;
        self
    }

    /// With max materials
    pub fn with_max_materials(mut self, count: u32) -> Self {
        self.max_materials = count;
        self
    }

    /// With method
    pub fn with_method(mut self, method: DecalMethod) -> Self {
        self.decal_method = method;
        self
    }

    /// With features
    pub fn with_features(mut self, features: DecalFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard system
    pub fn standard() -> Self {
        Self::new()
    }

    /// High capacity
    pub fn high_capacity() -> Self {
        Self::new()
            .with_max_decals(16384)
            .with_max_materials(1024)
    }

    /// Deferred decals
    pub fn deferred() -> Self {
        Self::new()
            .with_method(DecalMethod::Deferred)
    }

    /// DBuffer decals
    pub fn dbuffer() -> Self {
        Self::new()
            .with_method(DecalMethod::DBuffer)
    }
}

impl Default for GpuDecalSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Decal method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DecalMethod {
    /// Deferred decals (render to G-buffer)
    Deferred = 0,
    /// D-buffer decals (separate buffer)
    #[default]
    DBuffer = 1,
    /// Screen-space decals
    ScreenSpace = 2,
    /// Forward decals
    Forward = 3,
}

bitflags::bitflags! {
    /// Decal features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct DecalFeatures: u32 {
        /// None
        const NONE = 0;
        /// Color
        const COLOR = 1 << 0;
        /// Normal
        const NORMAL = 1 << 1;
        /// Roughness
        const ROUGHNESS = 1 << 2;
        /// Metallic
        const METALLIC = 1 << 3;
        /// Emissive
        const EMISSIVE = 1 << 4;
        /// Ambient occlusion
        const AO = 1 << 5;
        /// Parallax/height
        const PARALLAX = 1 << 6;
        /// Animated
        const ANIMATED = 1 << 7;
        /// Fading
        const FADING = 1 << 8;
        /// All
        const ALL = 0x1FF;
    }
}

impl Default for DecalFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Decal Creation
// ============================================================================

/// Decal create info
#[derive(Clone, Debug)]
pub struct DecalCreateInfo {
    /// Name
    pub name: String,
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Size
    pub size: [f32; 3],
    /// Material
    pub material: DecalMaterialHandle,
    /// Decal type
    pub decal_type: DecalType,
    /// Fade settings
    pub fade: DecalFadeSettings,
    /// Layer mask
    pub layer_mask: u32,
    /// Sort order
    pub sort_order: i32,
    /// Flags
    pub flags: DecalFlags,
}

impl DecalCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            size: [1.0, 1.0, 1.0],
            material: DecalMaterialHandle::NULL,
            decal_type: DecalType::Static,
            fade: DecalFadeSettings::default(),
            layer_mask: 0xFFFFFFFF,
            sort_order: 0,
            flags: DecalFlags::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With position
    pub fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With rotation
    pub fn with_rotation(mut self, rotation: [f32; 4]) -> Self {
        self.rotation = rotation;
        self
    }

    /// With size
    pub fn with_size(mut self, w: f32, h: f32, d: f32) -> Self {
        self.size = [w, h, d];
        self
    }

    /// With uniform size
    pub fn with_uniform_size(mut self, size: f32) -> Self {
        self.size = [size, size, size];
        self
    }

    /// With material
    pub fn with_material(mut self, material: DecalMaterialHandle) -> Self {
        self.material = material;
        self
    }

    /// With type
    pub fn with_type(mut self, decal_type: DecalType) -> Self {
        self.decal_type = decal_type;
        self
    }

    /// With fade
    pub fn with_fade(mut self, fade: DecalFadeSettings) -> Self {
        self.fade = fade;
        self
    }

    /// With layer mask
    pub fn with_layer_mask(mut self, mask: u32) -> Self {
        self.layer_mask = mask;
        self
    }

    /// With sort order
    pub fn with_sort_order(mut self, order: i32) -> Self {
        self.sort_order = order;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: DecalFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// Static decal
    pub fn static_decal(pos: [f32; 3], size: f32) -> Self {
        Self::new()
            .with_position(pos[0], pos[1], pos[2])
            .with_uniform_size(size)
            .with_type(DecalType::Static)
    }

    /// Dynamic decal (e.g., bullet hole)
    pub fn dynamic(pos: [f32; 3], size: f32) -> Self {
        Self::new()
            .with_position(pos[0], pos[1], pos[2])
            .with_uniform_size(size)
            .with_type(DecalType::Dynamic)
            .with_fade(DecalFadeSettings::timed(5.0, 2.0))
    }

    /// Blood splatter
    pub fn blood_splatter(pos: [f32; 3]) -> Self {
        Self::new()
            .with_position(pos[0], pos[1], pos[2])
            .with_size(0.5, 0.5, 0.2)
            .with_type(DecalType::Dynamic)
            .with_fade(DecalFadeSettings::timed(30.0, 5.0))
    }

    /// Footprint
    pub fn footprint(pos: [f32; 3], rotation: [f32; 4]) -> Self {
        Self::new()
            .with_position(pos[0], pos[1], pos[2])
            .with_rotation(rotation)
            .with_size(0.3, 0.4, 0.05)
            .with_type(DecalType::Dynamic)
            .with_fade(DecalFadeSettings::timed(10.0, 3.0))
    }

    /// Graffiti
    pub fn graffiti(pos: [f32; 3], size: f32) -> Self {
        Self::new()
            .with_position(pos[0], pos[1], pos[2])
            .with_uniform_size(size)
            .with_type(DecalType::Static)
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
    /// Static (persistent)
    #[default]
    Static = 0,
    /// Dynamic (temporary)
    Dynamic = 1,
    /// Animated
    Animated = 2,
    /// Projected (runtime)
    Projected = 3,
}

bitflags::bitflags! {
    /// Decal flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct DecalFlags: u32 {
        /// None
        const NONE = 0;
        /// Affects albedo
        const ALBEDO = 1 << 0;
        /// Affects normal
        const NORMAL = 1 << 1;
        /// Affects roughness
        const ROUGHNESS = 1 << 2;
        /// Affects metallic
        const METALLIC = 1 << 3;
        /// Project on static only
        const STATIC_ONLY = 1 << 4;
        /// Project on dynamic only
        const DYNAMIC_ONLY = 1 << 5;
        /// Wrap projection
        const WRAP = 1 << 6;
        /// Receive shadows
        const RECEIVE_SHADOWS = 1 << 7;
    }
}

/// Decal fade settings
#[derive(Clone, Copy, Debug)]
pub struct DecalFadeSettings {
    /// Fade mode
    pub mode: DecalFadeMode,
    /// Lifetime (seconds)
    pub lifetime: f32,
    /// Fade duration (seconds)
    pub fade_duration: f32,
    /// Distance fade start
    pub distance_fade_start: f32,
    /// Distance fade end
    pub distance_fade_end: f32,
    /// Angle fade start
    pub angle_fade_start: f32,
    /// Angle fade end
    pub angle_fade_end: f32,
}

impl DecalFadeSettings {
    /// No fade
    pub const fn none() -> Self {
        Self {
            mode: DecalFadeMode::None,
            lifetime: 0.0,
            fade_duration: 0.0,
            distance_fade_start: 0.0,
            distance_fade_end: 0.0,
            angle_fade_start: 0.0,
            angle_fade_end: 0.0,
        }
    }

    /// Timed fade
    pub const fn timed(lifetime: f32, fade_duration: f32) -> Self {
        Self {
            mode: DecalFadeMode::Time,
            lifetime,
            fade_duration,
            distance_fade_start: 0.0,
            distance_fade_end: 0.0,
            angle_fade_start: 0.0,
            angle_fade_end: 0.0,
        }
    }

    /// Distance fade
    pub const fn distance(start: f32, end: f32) -> Self {
        Self {
            mode: DecalFadeMode::Distance,
            lifetime: 0.0,
            fade_duration: 0.0,
            distance_fade_start: start,
            distance_fade_end: end,
            angle_fade_start: 0.0,
            angle_fade_end: 0.0,
        }
    }

    /// Angle fade
    pub const fn angle(start: f32, end: f32) -> Self {
        Self {
            mode: DecalFadeMode::Angle,
            lifetime: 0.0,
            fade_duration: 0.0,
            distance_fade_start: 0.0,
            distance_fade_end: 0.0,
            angle_fade_start: start,
            angle_fade_end: end,
        }
    }

    /// With distance fade
    pub const fn with_distance(mut self, start: f32, end: f32) -> Self {
        self.distance_fade_start = start;
        self.distance_fade_end = end;
        self
    }

    /// With angle fade
    pub const fn with_angle(mut self, start: f32, end: f32) -> Self {
        self.angle_fade_start = start;
        self.angle_fade_end = end;
        self
    }
}

impl Default for DecalFadeSettings {
    fn default() -> Self {
        Self::none()
    }
}

/// Decal fade mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DecalFadeMode {
    /// No fade
    #[default]
    None = 0,
    /// Time based
    Time = 1,
    /// Distance based
    Distance = 2,
    /// Angle based
    Angle = 3,
    /// Combined
    Combined = 4,
}

// ============================================================================
// Decal Material
// ============================================================================

/// Decal material create info
#[derive(Clone, Debug)]
pub struct DecalMaterialCreateInfo {
    /// Name
    pub name: String,
    /// Albedo texture
    pub albedo: u64,
    /// Normal texture
    pub normal: u64,
    /// Roughness texture
    pub roughness: u64,
    /// Metallic texture
    pub metallic: u64,
    /// Emissive texture
    pub emissive: u64,
    /// Blend mode
    pub blend_mode: DecalBlendMode,
    /// Opacity
    pub opacity: f32,
    /// Normal strength
    pub normal_strength: f32,
    /// Emissive intensity
    pub emissive_intensity: f32,
    /// Affected channels
    pub affected_channels: DecalChannels,
}

impl DecalMaterialCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            albedo: 0,
            normal: 0,
            roughness: 0,
            metallic: 0,
            emissive: 0,
            blend_mode: DecalBlendMode::Normal,
            opacity: 1.0,
            normal_strength: 1.0,
            emissive_intensity: 1.0,
            affected_channels: DecalChannels::all(),
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

    /// With roughness
    pub fn with_roughness(mut self, texture: u64) -> Self {
        self.roughness = texture;
        self
    }

    /// With blend mode
    pub fn with_blend(mut self, mode: DecalBlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// With opacity
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    /// With affected channels
    pub fn with_channels(mut self, channels: DecalChannels) -> Self {
        self.affected_channels = channels;
        self
    }

    /// Color only
    pub fn color_only() -> Self {
        Self::new()
            .with_channels(DecalChannels::ALBEDO)
    }

    /// Normal only
    pub fn normal_only() -> Self {
        Self::new()
            .with_channels(DecalChannels::NORMAL)
    }

    /// Full PBR
    pub fn full_pbr() -> Self {
        Self::new()
            .with_channels(DecalChannels::all())
    }
}

impl Default for DecalMaterialCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Decal blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DecalBlendMode {
    /// Normal blend
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

bitflags::bitflags! {
    /// Decal affected channels
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct DecalChannels: u32 {
        /// None
        const NONE = 0;
        /// Albedo
        const ALBEDO = 1 << 0;
        /// Normal
        const NORMAL = 1 << 1;
        /// Roughness
        const ROUGHNESS = 1 << 2;
        /// Metallic
        const METALLIC = 1 << 3;
        /// AO
        const AO = 1 << 4;
        /// Emissive
        const EMISSIVE = 1 << 5;
        /// All
        const ALL = 0x3F;
    }
}

impl Default for DecalChannels {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Decal Atlas
// ============================================================================

/// Decal atlas create info
#[derive(Clone, Debug)]
pub struct DecalAtlasCreateInfo {
    /// Name
    pub name: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Layers
    pub layers: u32,
    /// Format
    pub format: DecalAtlasFormat,
}

impl DecalAtlasCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            width: 2048,
            height: 2048,
            layers: 4,
            format: DecalAtlasFormat::Bc7,
        }
    }

    /// With size
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// With layers
    pub fn with_layers(mut self, layers: u32) -> Self {
        self.layers = layers;
        self
    }

    /// Standard atlas
    pub fn standard() -> Self {
        Self::new()
    }

    /// Large atlas
    pub fn large() -> Self {
        Self::new()
            .with_size(4096, 4096)
            .with_layers(8)
    }
}

impl Default for DecalAtlasCreateInfo {
    fn default() -> Self {
        Self::new()
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

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU decal data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDecalData {
    /// World to decal matrix
    pub world_to_decal: [[f32; 4]; 4],
    /// Decal to world matrix (inverse)
    pub decal_to_world: [[f32; 4]; 4],
    /// UV rect (for atlas)
    pub uv_rect: [f32; 4],
    /// Color/tint
    pub color: [f32; 4],
    /// Blend params (opacity, normal strength, etc.)
    pub blend_params: [f32; 4],
    /// Fade params
    pub fade_params: [f32; 4],
    /// Material index
    pub material_index: u32,
    /// Flags
    pub flags: u32,
    /// Layer mask
    pub layer_mask: u32,
    /// Sort order
    pub sort_order: i32,
}

/// GPU decal constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDecalConstants {
    /// View-projection inverse
    pub view_proj_inv: [[f32; 4]; 4],
    /// Camera position
    pub camera_pos: [f32; 3],
    /// Time
    pub time: f32,
    /// Screen size
    pub screen_size: [f32; 2],
    /// Decal count
    pub decal_count: u32,
    /// Padding
    pub _pad: u32,
}

/// GPU decal material
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDecalMaterial {
    /// Albedo texture index
    pub albedo_index: u32,
    /// Normal texture index
    pub normal_index: u32,
    /// Roughness texture index
    pub roughness_index: u32,
    /// Metallic texture index
    pub metallic_index: u32,
    /// Emissive texture index
    pub emissive_index: u32,
    /// Blend mode
    pub blend_mode: u32,
    /// Affected channels
    pub affected_channels: u32,
    /// Padding
    pub _pad: u32,
    /// Parameters
    pub params: [f32; 4],
}

// ============================================================================
// Statistics
// ============================================================================

/// GPU decal statistics
#[derive(Clone, Debug, Default)]
pub struct GpuDecalStats {
    /// Active decals
    pub active_decals: u32,
    /// Visible decals
    pub visible_decals: u32,
    /// Materials used
    pub materials_used: u32,
    /// Draw calls
    pub draw_calls: u32,
    /// Pixels affected
    pub pixels_affected: u64,
    /// GPU memory
    pub gpu_memory: u64,
    /// Render time (ms)
    pub render_time_ms: f32,
}

impl GpuDecalStats {
    /// Decals per draw call
    pub fn decals_per_draw(&self) -> f32 {
        if self.draw_calls == 0 {
            0.0
        } else {
            self.visible_decals as f32 / self.draw_calls as f32
        }
    }
}
