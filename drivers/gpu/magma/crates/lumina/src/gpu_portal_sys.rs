//! GPU Portal Rendering System for Lumina
//!
//! This module provides GPU-accelerated portal rendering for seamless
//! transitions between spaces, recursive portals, and portal effects.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Portal System Handles
// ============================================================================

/// GPU portal system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuPortalSystemHandle(pub u64);

impl GpuPortalSystemHandle {
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

impl Default for GpuPortalSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Portal handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PortalHandle(pub u64);

impl PortalHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for PortalHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Portal pair handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PortalPairHandle(pub u64);

impl PortalPairHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for PortalPairHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Portal view handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PortalViewHandle(pub u64);

impl PortalViewHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for PortalViewHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Portal System Creation
// ============================================================================

/// GPU portal system create info
#[derive(Clone, Debug)]
pub struct GpuPortalSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max portals
    pub max_portals: u32,
    /// Max portal pairs
    pub max_pairs: u32,
    /// Max recursion depth
    pub max_recursion: u32,
    /// Features
    pub features: PortalFeatures,
    /// Quality
    pub quality: PortalQuality,
}

impl GpuPortalSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_portals: 64,
            max_pairs: 32,
            max_recursion: 3,
            features: PortalFeatures::all(),
            quality: PortalQuality::High,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max portals
    pub fn with_max_portals(mut self, count: u32) -> Self {
        self.max_portals = count;
        self
    }

    /// With max recursion
    pub fn with_max_recursion(mut self, depth: u32) -> Self {
        self.max_recursion = depth;
        self
    }

    /// With features
    pub fn with_features(mut self, features: PortalFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With quality
    pub fn with_quality(mut self, quality: PortalQuality) -> Self {
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
            .with_max_recursion(5)
            .with_quality(PortalQuality::Ultra)
    }

    /// Mobile preset
    pub fn mobile() -> Self {
        Self::new()
            .with_max_recursion(1)
            .with_quality(PortalQuality::Low)
            .with_features(PortalFeatures::BASIC)
    }
}

impl Default for GpuPortalSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Portal features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct PortalFeatures: u32 {
        /// None
        const NONE = 0;
        /// Recursive rendering
        const RECURSIVE = 1 << 0;
        /// Stencil masking
        const STENCIL = 1 << 1;
        /// Oblique view frustum
        const OBLIQUE_FRUSTUM = 1 << 2;
        /// Portal effects (distortion, etc)
        const EFFECTS = 1 << 3;
        /// LOD for distant portals
        const LOD = 1 << 4;
        /// Physics teleportation
        const TELEPORT = 1 << 5;
        /// Audio occlusion
        const AUDIO = 1 << 6;
        /// Dynamic resolution
        const DYNAMIC_RES = 1 << 7;
        /// Basic features
        const BASIC = Self::STENCIL.bits() | Self::OBLIQUE_FRUSTUM.bits();
        /// All
        const ALL = 0xFF;
    }
}

impl Default for PortalFeatures {
    fn default() -> Self {
        Self::all()
    }
}

/// Portal quality
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PortalQuality {
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
// Portal Creation
// ============================================================================

/// Portal create info
#[derive(Clone, Debug)]
pub struct PortalCreateInfo {
    /// Name
    pub name: String,
    /// Portal type
    pub portal_type: PortalType,
    /// Transform
    pub transform: PortalTransform,
    /// Dimensions
    pub dimensions: PortalDimensions,
    /// Linked portal
    pub linked_portal: Option<PortalHandle>,
    /// Render settings
    pub render: PortalRenderSettings,
    /// Effects
    pub effects: PortalEffects,
}

impl PortalCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            portal_type: PortalType::TwoWay,
            transform: PortalTransform::identity(),
            dimensions: PortalDimensions::default(),
            linked_portal: None,
            render: PortalRenderSettings::default(),
            effects: PortalEffects::none(),
        }
    }

    /// With transform
    pub fn with_transform(mut self, transform: PortalTransform) -> Self {
        self.transform = transform;
        self
    }

    /// With dimensions
    pub fn with_dimensions(mut self, dimensions: PortalDimensions) -> Self {
        self.dimensions = dimensions;
        self
    }

    /// With linked portal
    pub fn with_link(mut self, portal: PortalHandle) -> Self {
        self.linked_portal = Some(portal);
        self
    }

    /// With render settings
    pub fn with_render(mut self, settings: PortalRenderSettings) -> Self {
        self.render = settings;
        self
    }

    /// With effects
    pub fn with_effects(mut self, effects: PortalEffects) -> Self {
        self.effects = effects;
        self
    }

    /// Rectangular portal
    pub fn rectangle(name: impl Into<String>, width: f32, height: f32) -> Self {
        Self::new(name).with_dimensions(PortalDimensions::rectangle(width, height))
    }

    /// Circular portal
    pub fn circle(name: impl Into<String>, radius: f32) -> Self {
        Self::new(name).with_dimensions(PortalDimensions::circle(radius))
    }

    /// Standard portal pair preset
    pub fn standard_pair(name: impl Into<String>) -> Self {
        Self::rectangle(name, 2.0, 3.0)
    }
}

impl Default for PortalCreateInfo {
    fn default() -> Self {
        Self::new("Portal")
    }
}

/// Portal type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PortalType {
    /// Two way (bidirectional)
    #[default]
    TwoWay = 0,
    /// One way (entrance only)
    OneWay = 1,
    /// Window (view only, no teleport)
    Window = 2,
    /// Mirror
    Mirror = 3,
    /// Screen (displays texture)
    Screen = 4,
}

/// Portal transform
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PortalTransform {
    /// Position
    pub position: [f32; 3],
    /// Rotation (quaternion)
    pub rotation: [f32; 4],
    /// Scale
    pub scale: [f32; 3],
}

impl PortalTransform {
    /// Identity transform
    pub const fn identity() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        }
    }

    /// With position
    pub const fn with_position(mut self, x: f32, y: f32, z: f32) -> Self {
        self.position = [x, y, z];
        self
    }

    /// With rotation
    pub const fn with_rotation(mut self, x: f32, y: f32, z: f32, w: f32) -> Self {
        self.rotation = [x, y, z, w];
        self
    }

    /// With scale
    pub const fn with_scale(mut self, x: f32, y: f32, z: f32) -> Self {
        self.scale = [x, y, z];
        self
    }

    /// At position
    pub const fn at(x: f32, y: f32, z: f32) -> Self {
        Self::identity().with_position(x, y, z)
    }
}

impl Default for PortalTransform {
    fn default() -> Self {
        Self::identity()
    }
}

/// Portal dimensions
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PortalDimensions {
    /// Shape
    pub shape: PortalShape,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Border thickness
    pub border_thickness: f32,
}

impl PortalDimensions {
    /// Creates new dimensions
    pub const fn new(shape: PortalShape, width: f32, height: f32) -> Self {
        Self {
            shape,
            width,
            height,
            border_thickness: 0.05,
        }
    }

    /// Rectangle
    pub const fn rectangle(width: f32, height: f32) -> Self {
        Self::new(PortalShape::Rectangle, width, height)
    }

    /// Circle
    pub const fn circle(radius: f32) -> Self {
        Self::new(PortalShape::Circle, radius * 2.0, radius * 2.0)
    }

    /// Ellipse
    pub const fn ellipse(width: f32, height: f32) -> Self {
        Self::new(PortalShape::Ellipse, width, height)
    }

    /// With border
    pub const fn with_border(mut self, thickness: f32) -> Self {
        self.border_thickness = thickness;
        self
    }

    /// Door portal
    pub const fn door() -> Self {
        Self::rectangle(1.0, 2.2)
    }

    /// Window portal
    pub const fn window() -> Self {
        Self::rectangle(1.2, 1.0)
    }

    /// Large portal
    pub const fn large() -> Self {
        Self::rectangle(4.0, 4.0)
    }
}

impl Default for PortalDimensions {
    fn default() -> Self {
        Self::rectangle(2.0, 3.0)
    }
}

/// Portal shape
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PortalShape {
    /// Rectangle
    #[default]
    Rectangle  = 0,
    /// Circle
    Circle     = 1,
    /// Ellipse
    Ellipse    = 2,
    /// Custom mesh
    CustomMesh = 3,
}

// ============================================================================
// Portal Rendering
// ============================================================================

/// Portal render settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PortalRenderSettings {
    /// Resolution scale
    pub resolution_scale: f32,
    /// Max recursion depth override
    pub max_recursion: u32,
    /// Near clip plane
    pub near_clip: f32,
    /// Far clip plane
    pub far_clip: f32,
    /// FOV override (0 = use camera FOV)
    pub fov_override: f32,
    /// Render layers
    pub render_layers: u32,
    /// Render order
    pub render_order: i32,
    /// Flags
    pub flags: PortalRenderFlags,
}

impl PortalRenderSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            resolution_scale: 1.0,
            max_recursion: 0,
            near_clip: 0.01,
            far_clip: 1000.0,
            fov_override: 0.0,
            render_layers: 0xFFFFFFFF,
            render_order: 0,
            flags: PortalRenderFlags::empty(),
        }
    }

    /// With resolution scale
    pub const fn with_resolution_scale(mut self, scale: f32) -> Self {
        self.resolution_scale = scale;
        self
    }

    /// With max recursion
    pub const fn with_max_recursion(mut self, depth: u32) -> Self {
        self.max_recursion = depth;
        self
    }

    /// With clip planes
    pub const fn with_clip(mut self, near: f32, far: f32) -> Self {
        self.near_clip = near;
        self.far_clip = far;
        self
    }

    /// High quality preset
    pub const fn high_quality() -> Self {
        Self::new().with_resolution_scale(1.0)
    }

    /// Low quality preset
    pub const fn low_quality() -> Self {
        Self::new().with_resolution_scale(0.5).with_max_recursion(1)
    }
}

impl Default for PortalRenderSettings {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Portal render flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct PortalRenderFlags: u32 {
        /// None
        const NONE = 0;
        /// Cast shadows
        const CAST_SHADOW = 1 << 0;
        /// Receive shadows
        const RECEIVE_SHADOW = 1 << 1;
        /// Include in reflections
        const REFLECTIONS = 1 << 2;
        /// Include in refractions
        const REFRACTIONS = 1 << 3;
        /// Render in VR
        const VR = 1 << 4;
    }
}

// ============================================================================
// Portal Effects
// ============================================================================

/// Portal effects
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PortalEffects {
    /// Edge glow color
    pub edge_glow_color: [f32; 4],
    /// Edge glow intensity
    pub edge_glow_intensity: f32,
    /// Edge glow width
    pub edge_glow_width: f32,
    /// Ripple effect
    pub ripple_settings: RippleSettings,
    /// Distortion effect
    pub distortion_settings: DistortionSettings,
    /// Transition effect
    pub transition_effect: TransitionEffect,
    /// Particle effects enabled
    pub particles_enabled: bool,
}

impl PortalEffects {
    /// Creates new effects
    pub const fn new() -> Self {
        Self {
            edge_glow_color: [0.0, 0.5, 1.0, 1.0],
            edge_glow_intensity: 1.0,
            edge_glow_width: 0.1,
            ripple_settings: RippleSettings::none(),
            distortion_settings: DistortionSettings::none(),
            transition_effect: TransitionEffect::None,
            particles_enabled: false,
        }
    }

    /// No effects
    pub const fn none() -> Self {
        Self {
            edge_glow_color: [0.0; 4],
            edge_glow_intensity: 0.0,
            edge_glow_width: 0.0,
            ripple_settings: RippleSettings::none(),
            distortion_settings: DistortionSettings::none(),
            transition_effect: TransitionEffect::None,
            particles_enabled: false,
        }
    }

    /// With edge glow
    pub const fn with_edge_glow(mut self, color: [f32; 4], intensity: f32, width: f32) -> Self {
        self.edge_glow_color = color;
        self.edge_glow_intensity = intensity;
        self.edge_glow_width = width;
        self
    }

    /// With ripple
    pub const fn with_ripple(mut self, settings: RippleSettings) -> Self {
        self.ripple_settings = settings;
        self
    }

    /// With distortion
    pub const fn with_distortion(mut self, settings: DistortionSettings) -> Self {
        self.distortion_settings = settings;
        self
    }

    /// Sci-fi portal preset
    pub const fn scifi() -> Self {
        Self::new()
            .with_edge_glow([0.0, 0.8, 1.0, 1.0], 2.0, 0.15)
            .with_ripple(RippleSettings::continuous())
    }

    /// Magic portal preset
    pub const fn magic() -> Self {
        Self::new()
            .with_edge_glow([0.8, 0.2, 1.0, 1.0], 1.5, 0.2)
            .with_distortion(DistortionSettings::subtle())
    }

    /// Clean portal preset
    pub const fn clean() -> Self {
        Self::new()
    }
}

impl Default for PortalEffects {
    fn default() -> Self {
        Self::new()
    }
}

/// Ripple settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RippleSettings {
    /// Enabled
    pub enabled: bool,
    /// Frequency
    pub frequency: f32,
    /// Amplitude
    pub amplitude: f32,
    /// Speed
    pub speed: f32,
    /// Rings
    pub rings: u32,
}

impl RippleSettings {
    /// None
    pub const fn none() -> Self {
        Self {
            enabled: false,
            frequency: 0.0,
            amplitude: 0.0,
            speed: 0.0,
            rings: 0,
        }
    }

    /// Continuous ripple
    pub const fn continuous() -> Self {
        Self {
            enabled: true,
            frequency: 2.0,
            amplitude: 0.02,
            speed: 1.0,
            rings: 3,
        }
    }

    /// On touch ripple
    pub const fn on_touch() -> Self {
        Self {
            enabled: true,
            frequency: 4.0,
            amplitude: 0.05,
            speed: 2.0,
            rings: 5,
        }
    }
}

impl Default for RippleSettings {
    fn default() -> Self {
        Self::none()
    }
}

/// Distortion settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DistortionSettings {
    /// Enabled
    pub enabled: bool,
    /// Strength
    pub strength: f32,
    /// Scale
    pub scale: f32,
    /// Speed
    pub speed: f32,
}

impl DistortionSettings {
    /// None
    pub const fn none() -> Self {
        Self {
            enabled: false,
            strength: 0.0,
            scale: 1.0,
            speed: 0.0,
        }
    }

    /// Subtle distortion
    pub const fn subtle() -> Self {
        Self {
            enabled: true,
            strength: 0.01,
            scale: 3.0,
            speed: 0.5,
        }
    }

    /// Strong distortion
    pub const fn strong() -> Self {
        Self {
            enabled: true,
            strength: 0.05,
            scale: 2.0,
            speed: 1.0,
        }
    }
}

impl Default for DistortionSettings {
    fn default() -> Self {
        Self::none()
    }
}

/// Transition effect
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TransitionEffect {
    /// None
    #[default]
    None     = 0,
    /// Fade
    Fade     = 1,
    /// Blur
    Blur     = 2,
    /// Warp
    Warp     = 3,
    /// Dissolve
    Dissolve = 4,
}

// ============================================================================
// Portal Pair
// ============================================================================

/// Portal pair create info
#[derive(Clone, Debug)]
pub struct PortalPairCreateInfo {
    /// Name
    pub name: String,
    /// Portal A info
    pub portal_a: PortalCreateInfo,
    /// Portal B info
    pub portal_b: PortalCreateInfo,
    /// Pair settings
    pub settings: PortalPairSettings,
}

impl PortalPairCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            portal_a: PortalCreateInfo::default(),
            portal_b: PortalCreateInfo::default(),
            settings: PortalPairSettings::default(),
        }
    }

    /// With portals
    pub fn with_portals(mut self, a: PortalCreateInfo, b: PortalCreateInfo) -> Self {
        self.portal_a = a;
        self.portal_b = b;
        self
    }

    /// With settings
    pub fn with_settings(mut self, settings: PortalPairSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Standard pair
    pub fn standard(name: impl Into<String>, pos_a: [f32; 3], pos_b: [f32; 3]) -> Self {
        Self::new(name).with_portals(
            PortalCreateInfo::standard_pair("PortalA")
                .with_transform(PortalTransform::at(pos_a[0], pos_a[1], pos_a[2])),
            PortalCreateInfo::standard_pair("PortalB")
                .with_transform(PortalTransform::at(pos_b[0], pos_b[1], pos_b[2])),
        )
    }
}

/// Portal pair settings
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PortalPairSettings {
    /// Scale factor between portals
    pub scale_factor: f32,
    /// Rotation offset (degrees)
    pub rotation_offset: f32,
    /// Bidirectional
    pub bidirectional: bool,
    /// Teleport enabled
    pub teleport_enabled: bool,
    /// Seamless rendering
    pub seamless: bool,
}

impl PortalPairSettings {
    /// Creates new settings
    pub const fn new() -> Self {
        Self {
            scale_factor: 1.0,
            rotation_offset: 0.0,
            bidirectional: true,
            teleport_enabled: true,
            seamless: true,
        }
    }

    /// With scale factor
    pub const fn with_scale(mut self, scale: f32) -> Self {
        self.scale_factor = scale;
        self
    }

    /// With rotation
    pub const fn with_rotation(mut self, degrees: f32) -> Self {
        self.rotation_offset = degrees;
        self
    }

    /// One way only
    pub const fn one_way() -> Self {
        Self::new()
    }
}

impl Default for PortalPairSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Portal View
// ============================================================================

/// Portal view info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PortalViewInfo {
    /// Source portal
    pub source: PortalHandle,
    /// Destination portal
    pub destination: PortalHandle,
    /// Recursion level
    pub recursion_level: u32,
    /// View matrix
    pub view_matrix: [[f32; 4]; 4],
    /// Projection matrix
    pub projection_matrix: [[f32; 4]; 4],
    /// Clip plane
    pub clip_plane: [f32; 4],
    /// Stencil value
    pub stencil_value: u8,
    /// Resolution scale
    pub resolution_scale: f32,
}

impl Default for PortalViewInfo {
    fn default() -> Self {
        Self {
            source: PortalHandle::NULL,
            destination: PortalHandle::NULL,
            recursion_level: 0,
            view_matrix: [[0.0; 4]; 4],
            projection_matrix: [[0.0; 4]; 4],
            clip_plane: [0.0; 4],
            stencil_value: 0,
            resolution_scale: 1.0,
        }
    }
}

// ============================================================================
// GPU Parameters
// ============================================================================

/// GPU portal data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C, align(16))]
pub struct GpuPortalData {
    /// Portal model matrix
    pub model_matrix: [[f32; 4]; 4],
    /// Portal plane (normal + distance)
    pub portal_plane: [f32; 4],
    /// Dimensions (width, height, border, shape)
    pub dimensions: [f32; 4],
    /// Portal ID
    pub portal_id: u32,
    /// Linked portal ID
    pub linked_id: u32,
    /// Portal type
    pub portal_type: u32,
    /// Flags
    pub flags: u32,
}

/// GPU portal view data
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuPortalViewData {
    /// View matrix
    pub view_matrix: [[f32; 4]; 4],
    /// Projection matrix
    pub proj_matrix: [[f32; 4]; 4],
    /// Inverse view projection
    pub inv_view_proj: [[f32; 4]; 4],
    /// Clip plane (oblique near plane)
    pub clip_plane: [f32; 4],
    /// Camera position
    pub camera_pos: [f32; 3],
    /// Recursion level
    pub recursion_level: u32,
    /// Source portal
    pub source_portal: u32,
    /// Dest portal
    pub dest_portal: u32,
    /// Resolution scale
    pub resolution_scale: f32,
    /// Stencil ref
    pub stencil_ref: u32,
}

impl Default for GpuPortalViewData {
    fn default() -> Self {
        Self {
            view_matrix: [[0.0; 4]; 4],
            proj_matrix: [[0.0; 4]; 4],
            inv_view_proj: [[0.0; 4]; 4],
            clip_plane: [0.0; 4],
            camera_pos: [0.0; 3],
            recursion_level: 0,
            source_portal: 0,
            dest_portal: 0,
            resolution_scale: 1.0,
            stencil_ref: 0,
        }
    }
}

/// GPU portal constants
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuPortalConstants {
    /// Camera view matrix
    pub camera_view: [[f32; 4]; 4],
    /// Camera projection
    pub camera_proj: [[f32; 4]; 4],
    /// Camera position
    pub camera_position: [f32; 3],
    /// Time
    pub time: f32,
    /// Portal count
    pub portal_count: u32,
    /// Max recursion
    pub max_recursion: u32,
    /// Current recursion
    pub current_recursion: u32,
    /// Flags
    pub flags: u32,
    /// Screen size
    pub screen_size: [f32; 2],
    /// Pad
    pub _pad: [f32; 2],
}

impl Default for GpuPortalConstants {
    fn default() -> Self {
        Self {
            camera_view: [[0.0; 4]; 4],
            camera_proj: [[0.0; 4]; 4],
            camera_position: [0.0; 3],
            time: 0.0,
            portal_count: 0,
            max_recursion: 3,
            current_recursion: 0,
            flags: 0,
            screen_size: [1920.0, 1080.0],
            _pad: [0.0; 2],
        }
    }
}

/// GPU portal effect params
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct GpuPortalEffectParams {
    /// Edge glow color
    pub edge_glow_color: [f32; 4],
    /// Edge glow intensity
    pub edge_glow_intensity: f32,
    /// Edge glow width
    pub edge_glow_width: f32,
    /// Ripple frequency
    pub ripple_frequency: f32,
    /// Ripple amplitude
    pub ripple_amplitude: f32,
    /// Ripple speed
    pub ripple_speed: f32,
    /// Distortion strength
    pub distortion_strength: f32,
    /// Distortion scale
    pub distortion_scale: f32,
    /// Distortion speed
    pub distortion_speed: f32,
    /// Time
    pub time: f32,
    /// Effect flags
    pub effect_flags: u32,
    /// Pad
    pub _pad: [f32; 2],
}

impl Default for GpuPortalEffectParams {
    fn default() -> Self {
        Self {
            edge_glow_color: [0.0, 0.5, 1.0, 1.0],
            edge_glow_intensity: 1.0,
            edge_glow_width: 0.1,
            ripple_frequency: 2.0,
            ripple_amplitude: 0.02,
            ripple_speed: 1.0,
            distortion_strength: 0.0,
            distortion_scale: 1.0,
            distortion_speed: 0.0,
            time: 0.0,
            effect_flags: 0,
            _pad: [0.0; 2],
        }
    }
}

// ============================================================================
// Portal Statistics
// ============================================================================

/// Portal system statistics
#[derive(Clone, Debug, Default)]
pub struct GpuPortalStats {
    /// Active portals
    pub active_portals: u32,
    /// Visible portals
    pub visible_portals: u32,
    /// Portal pairs
    pub portal_pairs: u32,
    /// Total renders
    pub total_renders: u32,
    /// Recursive renders
    pub recursive_renders: u32,
    /// Max recursion reached
    pub max_recursion_reached: u32,
    /// Render time (ms)
    pub render_time_ms: f32,
    /// Teleports this frame
    pub teleports: u32,
}

impl GpuPortalStats {
    /// Average renders per visible portal
    pub fn avg_renders_per_portal(&self) -> f32 {
        if self.visible_portals > 0 {
            self.total_renders as f32 / self.visible_portals as f32
        } else {
            0.0
        }
    }
}
