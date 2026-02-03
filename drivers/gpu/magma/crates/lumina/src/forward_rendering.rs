//! Forward Rendering Types for Lumina
//!
//! This module provides forward rendering infrastructure including
//! forward+ lighting, multi-pass rendering, and transparent object handling.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Forward Rendering Handles
// ============================================================================

/// Forward pass handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ForwardPassHandle(pub u64);

impl ForwardPassHandle {
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

impl Default for ForwardPassHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Transparent queue handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TransparentQueueHandle(pub u64);

impl TransparentQueueHandle {
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

impl Default for TransparentQueueHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Forward Rendering Settings
// ============================================================================

/// Forward rendering settings
#[derive(Clone, Debug)]
pub struct ForwardRenderingSettings {
    /// Rendering path
    pub path: ForwardPath,
    /// Max lights in shader
    pub max_lights: u32,
    /// Enable shadows
    pub shadows: bool,
    /// MSAA samples
    pub msaa_samples: u32,
    /// Alpha test threshold
    pub alpha_test_threshold: f32,
    /// Enable prepass
    pub depth_prepass: bool,
}

impl ForwardRenderingSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            path: ForwardPath::ForwardPlus,
            max_lights: 128,
            shadows: true,
            msaa_samples: 1,
            alpha_test_threshold: 0.5,
            depth_prepass: true,
        }
    }

    /// Forward+ path
    pub fn forward_plus() -> Self {
        Self {
            path: ForwardPath::ForwardPlus,
            ..Self::new()
        }
    }

    /// Simple forward (mobile)
    pub fn simple() -> Self {
        Self {
            path: ForwardPath::Simple,
            max_lights: 8,
            shadows: true,
            depth_prepass: false,
            ..Self::new()
        }
    }

    /// With MSAA
    pub fn with_msaa(mut self, samples: u32) -> Self {
        self.msaa_samples = samples;
        self
    }

    /// With max lights
    pub fn with_max_lights(mut self, count: u32) -> Self {
        self.max_lights = count;
        self
    }
}

impl Default for ForwardRenderingSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Forward rendering path
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ForwardPath {
    /// Simple forward (few lights)
    Simple = 0,
    /// Forward+ (tiled light culling)
    #[default]
    ForwardPlus = 1,
    /// Clustered forward
    ClusteredForward = 2,
}

// ============================================================================
// Multi-Pass Rendering
// ============================================================================

/// Multi-pass settings
#[derive(Clone, Debug)]
pub struct MultiPassSettings {
    /// Passes
    pub passes: Vec<RenderPassType>,
    /// Enable early-z
    pub early_z: bool,
    /// Enable stencil optimization
    pub stencil_optimization: bool,
}

impl MultiPassSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            early_z: true,
            stencil_optimization: true,
        }
    }

    /// Standard passes
    pub fn standard() -> Self {
        Self {
            passes: Vec::from([
                RenderPassType::DepthPrepass,
                RenderPassType::Opaque,
                RenderPassType::Transparent,
            ]),
            ..Self::new()
        }
    }

    /// With pass
    pub fn with_pass(mut self, pass: RenderPassType) -> Self {
        self.passes.push(pass);
        self
    }
}

impl Default for MultiPassSettings {
    fn default() -> Self {
        Self::standard()
    }
}

/// Render pass type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum RenderPassType {
    /// Depth prepass
    DepthPrepass = 0,
    /// Opaque objects
    Opaque = 1,
    /// Alpha test
    AlphaTest = 2,
    /// Transparent
    Transparent = 3,
    /// Skybox
    Skybox = 4,
    /// Post-process
    PostProcess = 5,
    /// UI
    Ui = 6,
    /// Debug
    Debug = 7,
}

/// Render pass definition
#[derive(Clone, Debug)]
pub struct RenderPassDefinition {
    /// Pass type
    pub pass_type: RenderPassType,
    /// Clear color
    pub clear_color: Option<[f32; 4]>,
    /// Clear depth
    pub clear_depth: Option<f32>,
    /// Clear stencil
    pub clear_stencil: Option<u8>,
    /// Depth test
    pub depth_test: DepthTestMode,
    /// Depth write
    pub depth_write: bool,
    /// Blending
    pub blend_mode: BlendMode,
    /// Sort order
    pub sort_order: SortOrder,
}

impl RenderPassDefinition {
    /// Opaque pass
    pub fn opaque() -> Self {
        Self {
            pass_type: RenderPassType::Opaque,
            clear_color: Some([0.0, 0.0, 0.0, 1.0]),
            clear_depth: Some(1.0),
            clear_stencil: Some(0),
            depth_test: DepthTestMode::Less,
            depth_write: true,
            blend_mode: BlendMode::Opaque,
            sort_order: SortOrder::FrontToBack,
        }
    }

    /// Transparent pass
    pub fn transparent() -> Self {
        Self {
            pass_type: RenderPassType::Transparent,
            clear_color: None,
            clear_depth: None,
            clear_stencil: None,
            depth_test: DepthTestMode::LessEqual,
            depth_write: false,
            blend_mode: BlendMode::Alpha,
            sort_order: SortOrder::BackToFront,
        }
    }

    /// Depth prepass
    pub fn depth_prepass() -> Self {
        Self {
            pass_type: RenderPassType::DepthPrepass,
            clear_color: None,
            clear_depth: Some(1.0),
            clear_stencil: None,
            depth_test: DepthTestMode::Less,
            depth_write: true,
            blend_mode: BlendMode::Opaque,
            sort_order: SortOrder::FrontToBack,
        }
    }
}

impl Default for RenderPassDefinition {
    fn default() -> Self {
        Self::opaque()
    }
}

/// Depth test mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DepthTestMode {
    /// Never pass
    Never = 0,
    /// Always pass
    Always = 1,
    /// Less than
    #[default]
    Less = 2,
    /// Less or equal
    LessEqual = 3,
    /// Greater
    Greater = 4,
    /// Greater or equal
    GreaterEqual = 5,
    /// Equal
    Equal = 6,
    /// Not equal
    NotEqual = 7,
    /// Disabled
    Disabled = 8,
}

/// Blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendMode {
    /// Opaque
    #[default]
    Opaque = 0,
    /// Alpha blending
    Alpha = 1,
    /// Premultiplied alpha
    PremultipliedAlpha = 2,
    /// Additive
    Additive = 3,
    /// Multiply
    Multiply = 4,
    /// Custom
    Custom = 5,
}

impl BlendMode {
    /// Source factor
    pub const fn source_factor(&self) -> BlendFactor {
        match self {
            Self::Opaque => BlendFactor::One,
            Self::Alpha => BlendFactor::SrcAlpha,
            Self::PremultipliedAlpha => BlendFactor::One,
            Self::Additive => BlendFactor::SrcAlpha,
            Self::Multiply => BlendFactor::DstColor,
            Self::Custom => BlendFactor::One,
        }
    }

    /// Destination factor
    pub const fn dest_factor(&self) -> BlendFactor {
        match self {
            Self::Opaque => BlendFactor::Zero,
            Self::Alpha => BlendFactor::OneMinusSrcAlpha,
            Self::PremultipliedAlpha => BlendFactor::OneMinusSrcAlpha,
            Self::Additive => BlendFactor::One,
            Self::Multiply => BlendFactor::Zero,
            Self::Custom => BlendFactor::Zero,
        }
    }
}

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendFactor {
    /// Zero
    #[default]
    Zero = 0,
    /// One
    One = 1,
    /// Src color
    SrcColor = 2,
    /// 1 - Src color
    OneMinusSrcColor = 3,
    /// Dst color
    DstColor = 4,
    /// 1 - Dst color
    OneMinusDstColor = 5,
    /// Src alpha
    SrcAlpha = 6,
    /// 1 - Src alpha
    OneMinusSrcAlpha = 7,
    /// Dst alpha
    DstAlpha = 8,
    /// 1 - Dst alpha
    OneMinusDstAlpha = 9,
}

/// Sort order
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SortOrder {
    /// No sorting
    #[default]
    None = 0,
    /// Front to back
    FrontToBack = 1,
    /// Back to front
    BackToFront = 2,
    /// By material
    ByMaterial = 3,
    /// By render order
    ByRenderOrder = 4,
}

// ============================================================================
// Transparent Rendering
// ============================================================================

/// Transparent object settings
#[derive(Clone, Debug)]
pub struct TransparentSettings {
    /// Sorting method
    pub sort_method: TransparentSortMethod,
    /// OIT method
    pub oit_method: Option<OitMethod>,
    /// Max layers (for OIT)
    pub max_layers: u32,
    /// Depth peeling passes
    pub depth_peel_passes: u32,
}

impl TransparentSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            sort_method: TransparentSortMethod::Distance,
            oit_method: None,
            max_layers: 4,
            depth_peel_passes: 4,
        }
    }

    /// With OIT
    pub fn with_oit(mut self, method: OitMethod) -> Self {
        self.oit_method = Some(method);
        self
    }

    /// With weighted blended OIT
    pub fn weighted_blended() -> Self {
        Self {
            oit_method: Some(OitMethod::WeightedBlended),
            ..Self::new()
        }
    }
}

impl Default for TransparentSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Transparent sort method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TransparentSortMethod {
    /// Distance from camera
    #[default]
    Distance = 0,
    /// Bounding box center
    BoundingBoxCenter = 1,
    /// Origin
    Origin = 2,
    /// Custom
    Custom = 3,
}

/// OIT (Order Independent Transparency) method
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum OitMethod {
    /// Weighted blended
    WeightedBlended = 0,
    /// Moment-based
    MomentBased = 1,
    /// Depth peeling
    DepthPeeling = 2,
    /// Linked list
    LinkedList = 3,
}

/// Transparent render item
#[derive(Clone, Copy, Debug)]
pub struct TransparentItem {
    /// Object ID
    pub object_id: u64,
    /// Distance from camera
    pub distance: f32,
    /// Sort key
    pub sort_key: u64,
    /// Blend mode
    pub blend_mode: BlendMode,
    /// Layer
    pub layer: u32,
}

impl TransparentItem {
    /// Creates item
    pub fn new(object_id: u64, distance: f32) -> Self {
        Self {
            object_id,
            distance,
            sort_key: distance.to_bits() as u64,
            blend_mode: BlendMode::Alpha,
            layer: 0,
        }
    }

    /// With blend mode
    pub fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }
}

/// Transparent queue
#[derive(Clone, Debug)]
pub struct TransparentQueue {
    /// Items
    pub items: Vec<TransparentItem>,
    /// Is sorted
    pub sorted: bool,
}

impl TransparentQueue {
    /// Creates queue
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            sorted: false,
        }
    }

    /// With capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            sorted: false,
        }
    }

    /// Clear
    pub fn clear(&mut self) {
        self.items.clear();
        self.sorted = false;
    }

    /// Add item
    pub fn add(&mut self, item: TransparentItem) {
        self.items.push(item);
        self.sorted = false;
    }

    /// Sort back to front
    pub fn sort_back_to_front(&mut self) {
        // Sort by distance descending (farthest first)
        self.items.sort_by(|a, b| {
            b.distance
                .partial_cmp(&a.distance)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        self.sorted = true;
    }

    /// Sort front to back
    pub fn sort_front_to_back(&mut self) {
        self.items.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        self.sorted = true;
    }

    /// Count
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for TransparentQueue {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Forward Light Structures
// ============================================================================

/// Forward light data (for shader)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ForwardLightGpu {
    /// Position
    pub position: [f32; 3],
    /// Range
    pub range: f32,
    /// Color
    pub color: [f32; 3],
    /// Intensity
    pub intensity: f32,
    /// Direction
    pub direction: [f32; 3],
    /// Type
    pub light_type: u32,
    /// Spot inner angle
    pub spot_inner: f32,
    /// Spot outer angle
    pub spot_outer: f32,
    /// Shadow index
    pub shadow_index: i32,
    /// Padding
    pub _padding: f32,
}

/// Forward lighting params
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ForwardLightingParams {
    /// Light count
    pub light_count: u32,
    /// Shadow count
    pub shadow_count: u32,
    /// Ambient color
    pub ambient: [f32; 3],
    /// Padding
    pub _padding: f32,
}

// ============================================================================
// Depth Prepass
// ============================================================================

/// Depth prepass settings
#[derive(Clone, Debug)]
pub struct DepthPrepassSettings {
    /// Enable
    pub enabled: bool,
    /// Include alpha tested
    pub alpha_tested: bool,
    /// Generate motion vectors
    pub motion_vectors: bool,
    /// Generate normals
    pub normals: bool,
}

impl DepthPrepassSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            alpha_tested: true,
            motion_vectors: false,
            normals: false,
        }
    }

    /// Disabled
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::new()
        }
    }

    /// With motion vectors
    pub fn with_motion_vectors(mut self) -> Self {
        self.motion_vectors = true;
        self
    }
}

impl Default for DepthPrepassSettings {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Render Order
// ============================================================================

/// Render order
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(i32)]
pub enum RenderOrder {
    /// Background (-1000)
    Background = -1000,
    /// Geometry (0)
    #[default]
    Geometry = 0,
    /// Alpha test (2450)
    AlphaTest = 2450,
    /// Transparent (3000)
    Transparent = 3000,
    /// Overlay (4000)
    Overlay = 4000,
}

impl RenderOrder {
    /// Custom value
    pub fn custom(value: i32) -> i32 {
        value
    }
}

/// Sort key builder
#[derive(Clone, Copy, Debug, Default)]
pub struct SortKeyBuilder {
    /// Key value
    key: u64,
}

impl SortKeyBuilder {
    /// Creates builder
    pub const fn new() -> Self {
        Self { key: 0 }
    }

    /// With render order (top 16 bits)
    pub const fn with_render_order(mut self, order: i32) -> Self {
        self.key |= ((order as u64 + 32768) & 0xFFFF) << 48;
        self
    }

    /// With material (next 24 bits)
    pub const fn with_material(mut self, material_id: u32) -> Self {
        self.key |= ((material_id as u64) & 0xFFFFFF) << 24;
        self
    }

    /// With depth (bottom 24 bits)
    pub fn with_depth(mut self, depth: f32) -> Self {
        // Convert depth to 24-bit integer
        let depth_int = ((depth.clamp(0.0, 1.0) * 0xFFFFFF as f32) as u64) & 0xFFFFFF;
        self.key |= depth_int;
        self
    }

    /// Build key
    pub const fn build(self) -> u64 {
        self.key
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Forward rendering statistics
#[derive(Clone, Debug, Default)]
pub struct ForwardStats {
    /// Draw calls
    pub draw_calls: u32,
    /// Triangles
    pub triangles: u64,
    /// Vertices
    pub vertices: u64,
    /// Opaque objects
    pub opaque_objects: u32,
    /// Transparent objects
    pub transparent_objects: u32,
    /// Light count
    pub lights: u32,
    /// Shadow casters
    pub shadow_casters: u32,
    /// GPU time (microseconds)
    pub gpu_time_us: u64,
}
