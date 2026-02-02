//! Pipeline state management
//!
//! This module provides types for configuring graphics and compute pipelines.
//! Most pipeline state is inferred from shader analysis, but can be overridden.

use alloc::vec::Vec;
use core::hash::{Hash, Hasher};

use crate::types::PipelineHandle;

/// Depth test comparison function
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum DepthTest {
    /// Depth testing disabled
    #[default]
    Disabled,
    /// Pass if less than
    Less,
    /// Pass if less than or equal
    LessEqual,
    /// Pass if greater than
    Greater,
    /// Pass if greater than or equal
    GreaterEqual,
    /// Pass if equal
    Equal,
    /// Pass if not equal
    NotEqual,
    /// Always pass
    Always,
    /// Never pass
    Never,
}

impl DepthTest {
    /// Returns the Vulkan compare op
    pub const fn vk_compare_op(self) -> u32 {
        match self {
            Self::Disabled | Self::Never => 0,  // VK_COMPARE_OP_NEVER
            Self::Less => 1,                     // VK_COMPARE_OP_LESS
            Self::Equal => 2,                    // VK_COMPARE_OP_EQUAL
            Self::LessEqual => 3,                // VK_COMPARE_OP_LESS_OR_EQUAL
            Self::Greater => 4,                  // VK_COMPARE_OP_GREATER
            Self::NotEqual => 5,                 // VK_COMPARE_OP_NOT_EQUAL
            Self::GreaterEqual => 6,             // VK_COMPARE_OP_GREATER_OR_EQUAL
            Self::Always => 7,                   // VK_COMPARE_OP_ALWAYS
        }
    }

    /// Returns true if depth testing is enabled
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

/// Face culling mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum CullMode {
    /// No culling
    None,
    /// Cull front faces
    Front,
    /// Cull back faces
    #[default]
    Back,
    /// Cull both (nothing rendered)
    FrontAndBack,
}

impl CullMode {
    /// Returns the Vulkan cull mode flags
    pub const fn vk_flags(self) -> u32 {
        match self {
            Self::None => 0,           // VK_CULL_MODE_NONE
            Self::Front => 1,          // VK_CULL_MODE_FRONT_BIT
            Self::Back => 2,           // VK_CULL_MODE_BACK_BIT
            Self::FrontAndBack => 3,   // VK_CULL_MODE_FRONT_AND_BACK
        }
    }
}

/// Blend mode presets
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    /// No blending (opaque)
    #[default]
    Opaque,
    /// Standard alpha blending
    Alpha,
    /// Premultiplied alpha
    PremultipliedAlpha,
    /// Additive blending
    Additive,
    /// Multiplicative blending
    Multiply,
    /// Custom blend state
    Custom(BlendState),
}

impl BlendMode {
    /// Returns the blend state for this mode
    pub const fn state(self) -> BlendState {
        match self {
            Self::Opaque => BlendState::OPAQUE,
            Self::Alpha => BlendState::ALPHA,
            Self::PremultipliedAlpha => BlendState::PREMULTIPLIED,
            Self::Additive => BlendState::ADDITIVE,
            Self::Multiply => BlendState::MULTIPLY,
            Self::Custom(state) => state,
        }
    }

    /// Returns true if blending is enabled
    pub const fn is_enabled(self) -> bool {
        !matches!(self, Self::Opaque)
    }
}

/// Detailed blend state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlendState {
    /// Enable blending
    pub enabled: bool,
    /// Source color factor
    pub src_color: BlendFactor,
    /// Destination color factor
    pub dst_color: BlendFactor,
    /// Color blend operation
    pub color_op: BlendOp,
    /// Source alpha factor
    pub src_alpha: BlendFactor,
    /// Destination alpha factor
    pub dst_alpha: BlendFactor,
    /// Alpha blend operation
    pub alpha_op: BlendOp,
}

impl BlendState {
    /// Opaque (no blending)
    pub const OPAQUE: Self = Self {
        enabled: false,
        src_color: BlendFactor::One,
        dst_color: BlendFactor::Zero,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::One,
        dst_alpha: BlendFactor::Zero,
        alpha_op: BlendOp::Add,
    };

    /// Standard alpha blending
    pub const ALPHA: Self = Self {
        enabled: true,
        src_color: BlendFactor::SrcAlpha,
        dst_color: BlendFactor::OneMinusSrcAlpha,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::One,
        dst_alpha: BlendFactor::OneMinusSrcAlpha,
        alpha_op: BlendOp::Add,
    };

    /// Premultiplied alpha
    pub const PREMULTIPLIED: Self = Self {
        enabled: true,
        src_color: BlendFactor::One,
        dst_color: BlendFactor::OneMinusSrcAlpha,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::One,
        dst_alpha: BlendFactor::OneMinusSrcAlpha,
        alpha_op: BlendOp::Add,
    };

    /// Additive blending
    pub const ADDITIVE: Self = Self {
        enabled: true,
        src_color: BlendFactor::SrcAlpha,
        dst_color: BlendFactor::One,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::One,
        dst_alpha: BlendFactor::One,
        alpha_op: BlendOp::Add,
    };

    /// Multiplicative blending
    pub const MULTIPLY: Self = Self {
        enabled: true,
        src_color: BlendFactor::DstColor,
        dst_color: BlendFactor::Zero,
        color_op: BlendOp::Add,
        src_alpha: BlendFactor::DstAlpha,
        dst_alpha: BlendFactor::Zero,
        alpha_op: BlendOp::Add,
    };
}

impl Default for BlendState {
    fn default() -> Self {
        Self::OPAQUE
    }
}

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlendFactor {
    Zero,
    One,
    SrcColor,
    OneMinusSrcColor,
    DstColor,
    OneMinusDstColor,
    SrcAlpha,
    OneMinusSrcAlpha,
    DstAlpha,
    OneMinusDstAlpha,
    ConstantColor,
    OneMinusConstantColor,
    ConstantAlpha,
    OneMinusConstantAlpha,
    SrcAlphaSaturate,
}

impl BlendFactor {
    /// Returns the Vulkan blend factor
    pub const fn vk_factor(self) -> u32 {
        match self {
            Self::Zero => 0,
            Self::One => 1,
            Self::SrcColor => 2,
            Self::OneMinusSrcColor => 3,
            Self::DstColor => 4,
            Self::OneMinusDstColor => 5,
            Self::SrcAlpha => 6,
            Self::OneMinusSrcAlpha => 7,
            Self::DstAlpha => 8,
            Self::OneMinusDstAlpha => 9,
            Self::ConstantColor => 10,
            Self::OneMinusConstantColor => 11,
            Self::ConstantAlpha => 12,
            Self::OneMinusConstantAlpha => 13,
            Self::SrcAlphaSaturate => 14,
        }
    }
}

/// Blend operation
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum BlendOp {
    #[default]
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

impl BlendOp {
    /// Returns the Vulkan blend op
    pub const fn vk_op(self) -> u32 {
        match self {
            Self::Add => 0,
            Self::Subtract => 1,
            Self::ReverseSubtract => 2,
            Self::Min => 3,
            Self::Max => 4,
        }
    }
}

/// Primitive topology
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Topology {
    PointList,
    LineList,
    LineStrip,
    #[default]
    TriangleList,
    TriangleStrip,
    TriangleFan,
    LineListWithAdjacency,
    LineStripWithAdjacency,
    TriangleListWithAdjacency,
    TriangleStripWithAdjacency,
    PatchList,
}

impl Topology {
    /// Returns the Vulkan topology
    pub const fn vk_topology(self) -> u32 {
        match self {
            Self::PointList => 0,
            Self::LineList => 1,
            Self::LineStrip => 2,
            Self::TriangleList => 3,
            Self::TriangleStrip => 4,
            Self::TriangleFan => 5,
            Self::LineListWithAdjacency => 6,
            Self::LineStripWithAdjacency => 7,
            Self::TriangleListWithAdjacency => 8,
            Self::TriangleStripWithAdjacency => 9,
            Self::PatchList => 10,
        }
    }
}

/// Polygon fill mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PolygonMode {
    #[default]
    Fill,
    Line,
    Point,
}

impl PolygonMode {
    /// Returns the Vulkan polygon mode
    pub const fn vk_mode(self) -> u32 {
        match self {
            Self::Fill => 0,
            Self::Line => 1,
            Self::Point => 2,
        }
    }
}

/// Front face winding order
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FrontFace {
    #[default]
    CounterClockwise,
    Clockwise,
}

impl FrontFace {
    /// Returns the Vulkan front face
    pub const fn vk_front_face(self) -> u32 {
        match self {
            Self::CounterClockwise => 0,
            Self::Clockwise => 1,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PIPELINE DESCRIPTION
// ═══════════════════════════════════════════════════════════════════════════

/// Description of a graphics pipeline
#[derive(Clone, Debug)]
pub struct GraphicsPipelineDesc {
    /// Vertex shader SPIR-V
    pub vertex_spirv: Vec<u32>,
    /// Fragment shader SPIR-V
    pub fragment_spirv: Vec<u32>,
    /// Vertex input layout
    pub vertex_layout: VertexLayout,
    /// Topology
    pub topology: Topology,
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Front face
    pub front_face: FrontFace,
    /// Depth test
    pub depth_test: DepthTest,
    /// Depth write enable
    pub depth_write: bool,
    /// Blend state per attachment
    pub blend_states: Vec<BlendState>,
    /// Color attachment formats
    pub color_formats: Vec<u32>,
    /// Depth format (0 for none)
    pub depth_format: u32,
    /// Sample count
    pub samples: u32,
}

impl Default for GraphicsPipelineDesc {
    fn default() -> Self {
        Self {
            vertex_spirv: Vec::new(),
            fragment_spirv: Vec::new(),
            vertex_layout: VertexLayout::default(),
            topology: Topology::default(),
            polygon_mode: PolygonMode::default(),
            cull_mode: CullMode::default(),
            front_face: FrontFace::default(),
            depth_test: DepthTest::default(),
            depth_write: true,
            blend_states: Vec::new(),
            color_formats: Vec::new(),
            depth_format: 0,
            samples: 1,
        }
    }
}

/// Vertex input layout
#[derive(Clone, Debug, Default)]
pub struct VertexLayout {
    /// Bindings
    pub bindings: Vec<VertexBinding>,
    /// Attributes
    pub attributes: Vec<VertexAttributeDesc>,
}

/// Vertex binding description
#[derive(Clone, Copy, Debug)]
pub struct VertexBinding {
    /// Binding index
    pub binding: u32,
    /// Stride in bytes
    pub stride: u32,
    /// Input rate
    pub input_rate: VertexInputRate,
}

/// Vertex input rate
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum VertexInputRate {
    #[default]
    Vertex,
    Instance,
}

/// Vertex attribute description
#[derive(Clone, Copy, Debug)]
pub struct VertexAttributeDesc {
    /// Location in shader
    pub location: u32,
    /// Binding index
    pub binding: u32,
    /// Format
    pub format: u32,
    /// Offset in bytes
    pub offset: u32,
}

// ═══════════════════════════════════════════════════════════════════════════
// COMPUTE PIPELINE
// ═══════════════════════════════════════════════════════════════════════════

/// Description of a compute pipeline
#[derive(Clone, Debug)]
pub struct ComputePipelineDesc {
    /// Compute shader SPIR-V
    pub spirv: Vec<u32>,
    /// Workgroup size
    pub workgroup_size: [u32; 3],
}

impl Default for ComputePipelineDesc {
    fn default() -> Self {
        Self {
            spirv: Vec::new(),
            workgroup_size: [1, 1, 1],
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PIPELINE CACHE
// ═══════════════════════════════════════════════════════════════════════════

/// Key for pipeline cache lookup
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    /// Hash of shaders
    pub shader_hash: u64,
    /// Hash of vertex layout
    pub layout_hash: u64,
    /// Render state hash
    pub state_hash: u64,
}

/// Cached pipeline information
pub struct CachedPipeline {
    /// Handle to the pipeline
    pub handle: PipelineHandle,
    /// Last frame this pipeline was used
    pub last_used_frame: u64,
}

/// Pipeline cache for avoiding redundant compilations
pub struct PipelineCache {
    /// Cached pipelines
    pipelines: alloc::collections::BTreeMap<u64, CachedPipeline>,
    /// Current frame number
    current_frame: u64,
    /// Maximum cache size
    max_size: usize,
}

impl PipelineCache {
    /// Creates a new pipeline cache
    pub fn new(max_size: usize) -> Self {
        Self {
            pipelines: alloc::collections::BTreeMap::new(),
            current_frame: 0,
            max_size,
        }
    }

    /// Looks up a pipeline by key
    pub fn get(&mut self, key: &PipelineKey) -> Option<PipelineHandle> {
        let hash = self.hash_key(key);

        if let Some(cached) = self.pipelines.get_mut(&hash) {
            cached.last_used_frame = self.current_frame;
            Some(cached.handle)
        } else {
            None
        }
    }

    /// Inserts a pipeline into the cache
    pub fn insert(&mut self, key: &PipelineKey, handle: PipelineHandle) {
        // Evict old entries if at capacity
        if self.pipelines.len() >= self.max_size {
            self.evict_lru();
        }

        let hash = self.hash_key(key);
        self.pipelines.insert(
            hash,
            CachedPipeline {
                handle,
                last_used_frame: self.current_frame,
            },
        );
    }

    /// Advances to the next frame
    pub fn next_frame(&mut self) {
        self.current_frame += 1;
    }

    /// Evicts the least recently used entry
    fn evict_lru(&mut self) {
        if let Some((&key, _)) = self
            .pipelines
            .iter()
            .min_by_key(|(_, v)| v.last_used_frame)
        {
            self.pipelines.remove(&key);
        }
    }

    /// Computes hash for a key
    fn hash_key(&self, key: &PipelineKey) -> u64 {
        use core::hash::Hasher;
        let mut hasher = FnvHasher::default();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self::new(1024)
    }
}

/// FNV-1a hasher for pipeline keys
#[derive(Default)]
struct FnvHasher {
    state: u64,
}

impl Hasher for FnvHasher {
    fn write(&mut self, bytes: &[u8]) {
        const FNV_PRIME: u64 = 0x100000001b3;

        for byte in bytes {
            self.state ^= *byte as u64;
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}
