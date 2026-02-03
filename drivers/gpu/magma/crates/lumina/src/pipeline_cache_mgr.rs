//! Pipeline Cache Manager Types for Lumina
//!
//! This module provides pipeline cache management
//! for efficient pipeline state caching.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Cache Handles
// ============================================================================

/// Pipeline cache manager handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineCacheManagerHandle(pub u64);

impl PipelineCacheManagerHandle {
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

impl Default for PipelineCacheManagerHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Cached pipeline handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CachedPipelineHandle(pub u64);

impl CachedPipelineHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for CachedPipelineHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Pipeline state hash
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct PipelineStateHash(pub u64);

impl PipelineStateHash {
    /// Null hash
    pub const NULL: Self = Self(0);
}

impl Default for PipelineStateHash {
    fn default() -> Self {
        Self::NULL
    }
}

/// Native pipeline cache handle (Vulkan VkPipelineCache, etc.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NativeCacheHandle(pub u64);

impl NativeCacheHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for NativeCacheHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Pipeline Cache Manager Creation
// ============================================================================

/// Pipeline cache manager create info
#[derive(Clone, Debug)]
pub struct PipelineCacheManagerCreateInfo {
    /// Name
    pub name: String,
    /// Cache path
    pub cache_path: String,
    /// Max cached pipelines
    pub max_pipelines: u32,
    /// Max cache size (bytes)
    pub max_size: u64,
    /// Cache policy
    pub policy: CachePolicy,
    /// Serialization format
    pub format: CacheFormat,
    /// Features
    pub features: PipelineCacheFeatures,
}

impl PipelineCacheManagerCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            cache_path: String::from("/cache/pipelines"),
            max_pipelines: 8192,
            max_size: 512 * 1024 * 1024,  // 512MB
            policy: CachePolicy::Lru,
            format: CacheFormat::Binary,
            features: PipelineCacheFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With cache path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.cache_path = path.into();
        self
    }

    /// With max pipelines
    pub fn with_max_pipelines(mut self, max: u32) -> Self {
        self.max_pipelines = max;
        self
    }

    /// With max size
    pub fn with_max_size(mut self, bytes: u64) -> Self {
        self.max_size = bytes;
        self
    }

    /// With policy
    pub fn with_policy(mut self, policy: CachePolicy) -> Self {
        self.policy = policy;
        self
    }

    /// With format
    pub fn with_format(mut self, format: CacheFormat) -> Self {
        self.format = format;
        self
    }

    /// With features
    pub fn with_features(mut self, features: PipelineCacheFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard preset
    pub fn standard() -> Self {
        Self::new()
    }

    /// Large cache preset
    pub fn large() -> Self {
        Self::new()
            .with_max_pipelines(32768)
            .with_max_size(2 * 1024 * 1024 * 1024)  // 2GB
    }

    /// Fast startup preset
    pub fn fast_startup() -> Self {
        Self::new()
            .with_features(PipelineCacheFeatures::PRELOAD | PipelineCacheFeatures::VALIDATION)
    }

    /// Hot reload preset
    pub fn hot_reload() -> Self {
        Self::new()
            .with_features(PipelineCacheFeatures::HOT_RELOAD | PipelineCacheFeatures::INVALIDATION)
    }
}

impl Default for PipelineCacheManagerCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache policy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CachePolicy {
    /// LRU eviction
    #[default]
    Lru = 0,
    /// LFU eviction
    Lfu = 1,
    /// FIFO eviction
    Fifo = 2,
    /// No eviction
    NoEviction = 3,
    /// Priority based
    Priority = 4,
}

/// Cache format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CacheFormat {
    /// Binary (native)
    #[default]
    Binary = 0,
    /// Compressed binary
    CompressedBinary = 1,
    /// JSON (debug)
    Json = 2,
}

bitflags::bitflags! {
    /// Pipeline cache features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct PipelineCacheFeatures: u32 {
        /// None
        const NONE = 0;
        /// Preload at startup
        const PRELOAD = 1 << 0;
        /// Validation
        const VALIDATION = 1 << 1;
        /// Hot reload
        const HOT_RELOAD = 1 << 2;
        /// Invalidation support
        const INVALIDATION = 1 << 3;
        /// Statistics
        const STATISTICS = 1 << 4;
        /// Async compilation
        const ASYNC_COMPILE = 1 << 5;
        /// Pipeline library
        const PIPELINE_LIBRARY = 1 << 6;
        /// Shader hot reload
        const SHADER_HOT_RELOAD = 1 << 7;
    }
}

// ============================================================================
// Pipeline State
// ============================================================================

/// Graphics pipeline state for caching
#[derive(Clone, Debug)]
pub struct GraphicsPipelineState {
    /// Vertex shader
    pub vertex_shader: u64,
    /// Fragment shader
    pub fragment_shader: u64,
    /// Geometry shader
    pub geometry_shader: Option<u64>,
    /// Tessellation control shader
    pub tess_control_shader: Option<u64>,
    /// Tessellation eval shader
    pub tess_eval_shader: Option<u64>,
    /// Vertex input state
    pub vertex_input: VertexInputState,
    /// Input assembly state
    pub input_assembly: InputAssemblyState,
    /// Rasterization state
    pub rasterization: RasterizationState,
    /// Multisample state
    pub multisample: MultisampleState,
    /// Depth stencil state
    pub depth_stencil: DepthStencilState,
    /// Color blend state
    pub color_blend: ColorBlendState,
    /// Dynamic state flags
    pub dynamic_state: DynamicStateFlags,
    /// Render pass compatibility hash
    pub render_pass_hash: u64,
    /// Subpass index
    pub subpass: u32,
}

impl GraphicsPipelineState {
    /// Creates new state
    pub fn new(vertex_shader: u64, fragment_shader: u64) -> Self {
        Self {
            vertex_shader,
            fragment_shader,
            geometry_shader: None,
            tess_control_shader: None,
            tess_eval_shader: None,
            vertex_input: VertexInputState::default(),
            input_assembly: InputAssemblyState::default(),
            rasterization: RasterizationState::default(),
            multisample: MultisampleState::default(),
            depth_stencil: DepthStencilState::default(),
            color_blend: ColorBlendState::default(),
            dynamic_state: DynamicStateFlags::empty(),
            render_pass_hash: 0,
            subpass: 0,
        }
    }

    /// Compute hash
    pub fn compute_hash(&self) -> PipelineStateHash {
        // Simple hash combining key elements
        let mut hash = 0u64;
        hash = hash.wrapping_mul(31).wrapping_add(self.vertex_shader);
        hash = hash.wrapping_mul(31).wrapping_add(self.fragment_shader);
        hash = hash.wrapping_mul(31).wrapping_add(self.geometry_shader.unwrap_or(0));
        hash = hash.wrapping_mul(31).wrapping_add(self.rasterization.hash());
        hash = hash.wrapping_mul(31).wrapping_add(self.depth_stencil.hash());
        hash = hash.wrapping_mul(31).wrapping_add(self.color_blend.hash());
        hash = hash.wrapping_mul(31).wrapping_add(self.render_pass_hash);
        PipelineStateHash(hash)
    }
}

impl Default for GraphicsPipelineState {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Compute pipeline state for caching
#[derive(Clone, Debug)]
pub struct ComputePipelineState {
    /// Compute shader
    pub compute_shader: u64,
    /// Specialization constants
    pub specialization: Vec<SpecializationConstant>,
}

impl ComputePipelineState {
    /// Creates new state
    pub fn new(compute_shader: u64) -> Self {
        Self {
            compute_shader,
            specialization: Vec::new(),
        }
    }

    /// Compute hash
    pub fn compute_hash(&self) -> PipelineStateHash {
        let mut hash = self.compute_shader;
        for spec in &self.specialization {
            hash = hash.wrapping_mul(31).wrapping_add(spec.id as u64);
            hash = hash.wrapping_mul(31).wrapping_add(u64::from_ne_bytes(
                spec.data.get(..8).map(|s| {
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(s);
                    arr
                }).unwrap_or([0u8; 8])
            ));
        }
        PipelineStateHash(hash)
    }
}

impl Default for ComputePipelineState {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Ray tracing pipeline state for caching
#[derive(Clone, Debug)]
pub struct RayTracingPipelineState {
    /// Ray generation shader
    pub raygen_shader: u64,
    /// Miss shaders
    pub miss_shaders: Vec<u64>,
    /// Closest hit shaders
    pub closest_hit_shaders: Vec<u64>,
    /// Any hit shaders
    pub any_hit_shaders: Vec<u64>,
    /// Intersection shaders
    pub intersection_shaders: Vec<u64>,
    /// Callable shaders
    pub callable_shaders: Vec<u64>,
    /// Max recursion depth
    pub max_recursion_depth: u32,
}

impl RayTracingPipelineState {
    /// Creates new state
    pub fn new(raygen_shader: u64) -> Self {
        Self {
            raygen_shader,
            miss_shaders: Vec::new(),
            closest_hit_shaders: Vec::new(),
            any_hit_shaders: Vec::new(),
            intersection_shaders: Vec::new(),
            callable_shaders: Vec::new(),
            max_recursion_depth: 1,
        }
    }

    /// Compute hash
    pub fn compute_hash(&self) -> PipelineStateHash {
        let mut hash = self.raygen_shader;
        for &shader in &self.miss_shaders {
            hash = hash.wrapping_mul(31).wrapping_add(shader);
        }
        for &shader in &self.closest_hit_shaders {
            hash = hash.wrapping_mul(31).wrapping_add(shader);
        }
        hash = hash.wrapping_mul(31).wrapping_add(self.max_recursion_depth as u64);
        PipelineStateHash(hash)
    }
}

impl Default for RayTracingPipelineState {
    fn default() -> Self {
        Self::new(0)
    }
}

// ============================================================================
// State Components
// ============================================================================

/// Vertex input state
#[derive(Clone, Debug, Default)]
pub struct VertexInputState {
    /// Bindings
    pub bindings: Vec<VertexBinding>,
    /// Attributes
    pub attributes: Vec<VertexAttribute>,
}

/// Vertex binding
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VertexBinding {
    /// Binding index
    pub binding: u32,
    /// Stride
    pub stride: u32,
    /// Input rate
    pub input_rate: VertexInputRate,
    /// Divisor
    pub divisor: u32,
}

/// Vertex attribute
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VertexAttribute {
    /// Location
    pub location: u32,
    /// Binding
    pub binding: u32,
    /// Format
    pub format: VertexFormat,
    /// Offset
    pub offset: u32,
}

/// Vertex input rate
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexInputRate {
    /// Per vertex
    #[default]
    Vertex = 0,
    /// Per instance
    Instance = 1,
}

/// Vertex format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum VertexFormat {
    /// Float
    #[default]
    Float = 0,
    /// Float2
    Float2 = 1,
    /// Float3
    Float3 = 2,
    /// Float4
    Float4 = 3,
    /// Int
    Int = 4,
    /// Int2
    Int2 = 5,
    /// Int3
    Int3 = 6,
    /// Int4
    Int4 = 7,
    /// UInt
    UInt = 8,
    /// UNorm8x4
    UNorm8x4 = 9,
    /// SNorm8x4
    SNorm8x4 = 10,
}

/// Input assembly state
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct InputAssemblyState {
    /// Topology
    pub topology: PrimitiveTopology,
    /// Primitive restart enable
    pub primitive_restart: bool,
}

/// Primitive topology
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PrimitiveTopology {
    /// Points
    Points = 0,
    /// Lines
    Lines = 1,
    /// Line strip
    LineStrip = 2,
    /// Triangles
    #[default]
    Triangles = 3,
    /// Triangle strip
    TriangleStrip = 4,
    /// Triangle fan
    TriangleFan = 5,
    /// Patches
    Patches = 6,
}

/// Rasterization state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RasterizationState {
    /// Polygon mode
    pub polygon_mode: PolygonMode,
    /// Cull mode
    pub cull_mode: CullMode,
    /// Front face
    pub front_face: FrontFace,
    /// Depth clamp enable
    pub depth_clamp: bool,
    /// Rasterizer discard enable
    pub rasterizer_discard: bool,
    /// Depth bias enable
    pub depth_bias_enable: bool,
    /// Depth bias constant factor
    pub depth_bias_constant: f32,
    /// Depth bias clamp
    pub depth_bias_clamp: f32,
    /// Depth bias slope factor
    pub depth_bias_slope: f32,
    /// Line width
    pub line_width: f32,
}

impl RasterizationState {
    /// Hash for caching
    pub fn hash(&self) -> u64 {
        let mut h = 0u64;
        h = h.wrapping_mul(31).wrapping_add(self.polygon_mode as u64);
        h = h.wrapping_mul(31).wrapping_add(self.cull_mode as u64);
        h = h.wrapping_mul(31).wrapping_add(self.front_face as u64);
        h = h.wrapping_mul(31).wrapping_add(self.depth_clamp as u64);
        h = h.wrapping_mul(31).wrapping_add(self.depth_bias_enable as u64);
        h
    }
}

impl Default for RasterizationState {
    fn default() -> Self {
        Self {
            polygon_mode: PolygonMode::Fill,
            cull_mode: CullMode::Back,
            front_face: FrontFace::CounterClockwise,
            depth_clamp: false,
            rasterizer_discard: false,
            depth_bias_enable: false,
            depth_bias_constant: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope: 0.0,
            line_width: 1.0,
        }
    }
}

/// Polygon mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PolygonMode {
    /// Fill
    #[default]
    Fill = 0,
    /// Line
    Line = 1,
    /// Point
    Point = 2,
}

/// Cull mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CullMode {
    /// None
    None = 0,
    /// Front
    Front = 1,
    /// Back
    #[default]
    Back = 2,
    /// Front and back
    FrontAndBack = 3,
}

/// Front face
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FrontFace {
    /// Counter clockwise
    #[default]
    CounterClockwise = 0,
    /// Clockwise
    Clockwise = 1,
}

/// Multisample state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MultisampleState {
    /// Sample count
    pub sample_count: u32,
    /// Sample shading enable
    pub sample_shading: bool,
    /// Min sample shading
    pub min_sample_shading: f32,
    /// Alpha to coverage enable
    pub alpha_to_coverage: bool,
    /// Alpha to one enable
    pub alpha_to_one: bool,
}

impl Default for MultisampleState {
    fn default() -> Self {
        Self {
            sample_count: 1,
            sample_shading: false,
            min_sample_shading: 1.0,
            alpha_to_coverage: false,
            alpha_to_one: false,
        }
    }
}

/// Depth stencil state
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DepthStencilState {
    /// Depth test enable
    pub depth_test: bool,
    /// Depth write enable
    pub depth_write: bool,
    /// Depth compare op
    pub depth_compare: CompareOp,
    /// Depth bounds test enable
    pub depth_bounds_test: bool,
    /// Stencil test enable
    pub stencil_test: bool,
    /// Front stencil op
    pub front_stencil: StencilOpState,
    /// Back stencil op
    pub back_stencil: StencilOpState,
    /// Min depth bounds
    pub min_depth_bounds: f32,
    /// Max depth bounds
    pub max_depth_bounds: f32,
}

impl DepthStencilState {
    /// Hash for caching
    pub fn hash(&self) -> u64 {
        let mut h = 0u64;
        h = h.wrapping_mul(31).wrapping_add(self.depth_test as u64);
        h = h.wrapping_mul(31).wrapping_add(self.depth_write as u64);
        h = h.wrapping_mul(31).wrapping_add(self.depth_compare as u64);
        h = h.wrapping_mul(31).wrapping_add(self.stencil_test as u64);
        h
    }
}

impl Default for DepthStencilState {
    fn default() -> Self {
        Self {
            depth_test: true,
            depth_write: true,
            depth_compare: CompareOp::Less,
            depth_bounds_test: false,
            stencil_test: false,
            front_stencil: StencilOpState::default(),
            back_stencil: StencilOpState::default(),
            min_depth_bounds: 0.0,
            max_depth_bounds: 1.0,
        }
    }
}

/// Compare op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum CompareOp {
    /// Never
    Never = 0,
    /// Less
    #[default]
    Less = 1,
    /// Equal
    Equal = 2,
    /// Less or equal
    LessOrEqual = 3,
    /// Greater
    Greater = 4,
    /// Not equal
    NotEqual = 5,
    /// Greater or equal
    GreaterOrEqual = 6,
    /// Always
    Always = 7,
}

/// Stencil op state
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct StencilOpState {
    /// Fail op
    pub fail_op: StencilOp,
    /// Pass op
    pub pass_op: StencilOp,
    /// Depth fail op
    pub depth_fail_op: StencilOp,
    /// Compare op
    pub compare_op: CompareOp,
    /// Compare mask
    pub compare_mask: u32,
    /// Write mask
    pub write_mask: u32,
    /// Reference
    pub reference: u32,
}

/// Stencil op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StencilOp {
    /// Keep
    #[default]
    Keep = 0,
    /// Zero
    Zero = 1,
    /// Replace
    Replace = 2,
    /// Increment and clamp
    IncrementClamp = 3,
    /// Decrement and clamp
    DecrementClamp = 4,
    /// Invert
    Invert = 5,
    /// Increment and wrap
    IncrementWrap = 6,
    /// Decrement and wrap
    DecrementWrap = 7,
}

/// Color blend state
#[derive(Clone, Debug)]
pub struct ColorBlendState {
    /// Logic op enable
    pub logic_op_enable: bool,
    /// Logic op
    pub logic_op: LogicOp,
    /// Attachments
    pub attachments: Vec<ColorBlendAttachment>,
    /// Blend constants
    pub blend_constants: [f32; 4],
}

impl ColorBlendState {
    /// Hash for caching
    pub fn hash(&self) -> u64 {
        let mut h = 0u64;
        h = h.wrapping_mul(31).wrapping_add(self.logic_op_enable as u64);
        h = h.wrapping_mul(31).wrapping_add(self.logic_op as u64);
        h = h.wrapping_mul(31).wrapping_add(self.attachments.len() as u64);
        for att in &self.attachments {
            h = h.wrapping_mul(31).wrapping_add(att.blend_enable as u64);
            h = h.wrapping_mul(31).wrapping_add(att.src_color_blend as u64);
            h = h.wrapping_mul(31).wrapping_add(att.dst_color_blend as u64);
        }
        h
    }
}

impl Default for ColorBlendState {
    fn default() -> Self {
        Self {
            logic_op_enable: false,
            logic_op: LogicOp::Copy,
            attachments: Vec::new(),
            blend_constants: [0.0; 4],
        }
    }
}

/// Logic op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LogicOp {
    /// Clear
    Clear = 0,
    /// And
    And = 1,
    /// And reverse
    AndReverse = 2,
    /// Copy
    #[default]
    Copy = 3,
    /// And inverted
    AndInverted = 4,
    /// No op
    NoOp = 5,
    /// Xor
    Xor = 6,
    /// Or
    Or = 7,
    /// Nor
    Nor = 8,
    /// Equivalent
    Equivalent = 9,
    /// Invert
    Invert = 10,
    /// Or reverse
    OrReverse = 11,
    /// Copy inverted
    CopyInverted = 12,
    /// Or inverted
    OrInverted = 13,
    /// Nand
    Nand = 14,
    /// Set
    Set = 15,
}

/// Color blend attachment
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ColorBlendAttachment {
    /// Blend enable
    pub blend_enable: bool,
    /// Source color blend factor
    pub src_color_blend: BlendFactor,
    /// Destination color blend factor
    pub dst_color_blend: BlendFactor,
    /// Color blend op
    pub color_blend_op: BlendOp,
    /// Source alpha blend factor
    pub src_alpha_blend: BlendFactor,
    /// Destination alpha blend factor
    pub dst_alpha_blend: BlendFactor,
    /// Alpha blend op
    pub alpha_blend_op: BlendOp,
    /// Color write mask
    pub color_write_mask: ColorWriteFlags,
}

impl Default for ColorBlendAttachment {
    fn default() -> Self {
        Self {
            blend_enable: false,
            src_color_blend: BlendFactor::One,
            dst_color_blend: BlendFactor::Zero,
            color_blend_op: BlendOp::Add,
            src_alpha_blend: BlendFactor::One,
            dst_alpha_blend: BlendFactor::Zero,
            alpha_blend_op: BlendOp::Add,
            color_write_mask: ColorWriteFlags::all(),
        }
    }
}

/// Blend factor
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendFactor {
    /// Zero
    Zero = 0,
    /// One
    #[default]
    One = 1,
    /// Source color
    SrcColor = 2,
    /// One minus source color
    OneMinusSrcColor = 3,
    /// Destination color
    DstColor = 4,
    /// One minus destination color
    OneMinusDstColor = 5,
    /// Source alpha
    SrcAlpha = 6,
    /// One minus source alpha
    OneMinusSrcAlpha = 7,
    /// Destination alpha
    DstAlpha = 8,
    /// One minus destination alpha
    OneMinusDstAlpha = 9,
}

/// Blend op
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendOp {
    /// Add
    #[default]
    Add = 0,
    /// Subtract
    Subtract = 1,
    /// Reverse subtract
    ReverseSubtract = 2,
    /// Min
    Min = 3,
    /// Max
    Max = 4,
}

bitflags::bitflags! {
    /// Color write flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct ColorWriteFlags: u32 {
        /// None
        const NONE = 0;
        /// R
        const R = 1 << 0;
        /// G
        const G = 1 << 1;
        /// B
        const B = 1 << 2;
        /// A
        const A = 1 << 3;
    }
}

bitflags::bitflags! {
    /// Dynamic state flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct DynamicStateFlags: u32 {
        /// None
        const NONE = 0;
        /// Viewport
        const VIEWPORT = 1 << 0;
        /// Scissor
        const SCISSOR = 1 << 1;
        /// Line width
        const LINE_WIDTH = 1 << 2;
        /// Depth bias
        const DEPTH_BIAS = 1 << 3;
        /// Blend constants
        const BLEND_CONSTANTS = 1 << 4;
        /// Depth bounds
        const DEPTH_BOUNDS = 1 << 5;
        /// Stencil compare mask
        const STENCIL_COMPARE_MASK = 1 << 6;
        /// Stencil write mask
        const STENCIL_WRITE_MASK = 1 << 7;
        /// Stencil reference
        const STENCIL_REFERENCE = 1 << 8;
    }
}

/// Specialization constant
#[derive(Clone, Debug, Default)]
pub struct SpecializationConstant {
    /// Constant ID
    pub id: u32,
    /// Data
    pub data: Vec<u8>,
}

// ============================================================================
// Cache Entry
// ============================================================================

/// Pipeline cache entry
#[derive(Clone, Debug)]
pub struct PipelineCacheEntry {
    /// Hash
    pub hash: PipelineStateHash,
    /// Pipeline handle
    pub pipeline: CachedPipelineHandle,
    /// Pipeline type
    pub pipeline_type: PipelineType,
    /// Size (bytes)
    pub size: u64,
    /// Compile time (ms)
    pub compile_time_ms: f32,
    /// Last accessed frame
    pub last_accessed: u64,
    /// Access count
    pub access_count: u32,
}

impl Default for PipelineCacheEntry {
    fn default() -> Self {
        Self {
            hash: PipelineStateHash::NULL,
            pipeline: CachedPipelineHandle::NULL,
            pipeline_type: PipelineType::Graphics,
            size: 0,
            compile_time_ms: 0.0,
            last_accessed: 0,
            access_count: 0,
        }
    }
}

/// Pipeline type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum PipelineType {
    /// Graphics
    #[default]
    Graphics = 0,
    /// Compute
    Compute = 1,
    /// Ray tracing
    RayTracing = 2,
    /// Mesh shading
    MeshShading = 3,
}

// ============================================================================
// Statistics
// ============================================================================

/// Pipeline cache statistics
#[derive(Clone, Debug, Default)]
pub struct PipelineCacheStats {
    /// Total pipelines
    pub total_pipelines: u32,
    /// Graphics pipelines
    pub graphics_pipelines: u32,
    /// Compute pipelines
    pub compute_pipelines: u32,
    /// Ray tracing pipelines
    pub raytracing_pipelines: u32,
    /// Cache hits
    pub cache_hits: u32,
    /// Cache misses
    pub cache_misses: u32,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Max memory (bytes)
    pub max_memory: u64,
    /// Total compile time (ms)
    pub compile_time_ms: f32,
    /// Evictions
    pub evictions: u32,
}

impl PipelineCacheStats {
    /// Cache hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.cache_hits as f32 / total as f32
    }

    /// Memory usage ratio
    pub fn memory_usage_ratio(&self) -> f32 {
        if self.max_memory == 0 {
            return 0.0;
        }
        self.memory_used as f32 / self.max_memory as f32
    }
}
