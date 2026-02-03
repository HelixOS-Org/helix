//! Material Graph Types for Lumina
//!
//! This module provides material graph infrastructure
//! for node-based material authoring and compilation.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Graph Handles
// ============================================================================

/// Material graph handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MaterialGraphHandle(pub u64);

impl MaterialGraphHandle {
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

impl Default for MaterialGraphHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Graph node handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GraphNodeHandle(pub u64);

impl GraphNodeHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for GraphNodeHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Graph connection handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GraphConnectionHandle(pub u64);

impl GraphConnectionHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for GraphConnectionHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Compiled material handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CompiledMaterialHandle(pub u64);

impl CompiledMaterialHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for CompiledMaterialHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Material Graph Creation
// ============================================================================

/// Material graph create info
#[derive(Clone, Debug)]
pub struct MaterialGraphCreateInfo {
    /// Name
    pub name: String,
    /// Surface type
    pub surface_type: SurfaceType,
    /// Blend mode
    pub blend_mode: MaterialBlendMode,
    /// Features
    pub features: MaterialGraphFeatures,
    /// Shading model
    pub shading_model: ShadingModel,
}

impl MaterialGraphCreateInfo {
    /// Creates new info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            surface_type: SurfaceType::Opaque,
            blend_mode: MaterialBlendMode::Opaque,
            features: MaterialGraphFeatures::empty(),
            shading_model: ShadingModel::DefaultLit,
        }
    }

    /// With surface type
    pub fn with_surface_type(mut self, surface_type: SurfaceType) -> Self {
        self.surface_type = surface_type;
        self
    }

    /// With blend mode
    pub fn with_blend_mode(mut self, blend_mode: MaterialBlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }

    /// With features
    pub fn with_features(mut self, features: MaterialGraphFeatures) -> Self {
        self.features |= features;
        self
    }

    /// With shading model
    pub fn with_shading(mut self, model: ShadingModel) -> Self {
        self.shading_model = model;
        self
    }

    /// Opaque PBR preset
    pub fn opaque_pbr(name: impl Into<String>) -> Self {
        Self::new(name)
            .with_surface_type(SurfaceType::Opaque)
            .with_shading(ShadingModel::DefaultLit)
    }

    /// Transparent preset
    pub fn transparent(name: impl Into<String>) -> Self {
        Self::new(name)
            .with_surface_type(SurfaceType::Translucent)
            .with_blend_mode(MaterialBlendMode::Translucent)
    }

    /// Masked preset
    pub fn masked(name: impl Into<String>) -> Self {
        Self::new(name)
            .with_surface_type(SurfaceType::Masked)
            .with_blend_mode(MaterialBlendMode::Masked)
    }

    /// Unlit preset
    pub fn unlit(name: impl Into<String>) -> Self {
        Self::new(name)
            .with_shading(ShadingModel::Unlit)
    }

    /// Subsurface preset
    pub fn subsurface(name: impl Into<String>) -> Self {
        Self::new(name)
            .with_shading(ShadingModel::Subsurface)
            .with_features(MaterialGraphFeatures::SUBSURFACE)
    }
}

impl Default for MaterialGraphCreateInfo {
    fn default() -> Self {
        Self::opaque_pbr("DefaultMaterial")
    }
}

/// Surface type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SurfaceType {
    /// Opaque
    #[default]
    Opaque = 0,
    /// Masked (alpha test)
    Masked = 1,
    /// Translucent
    Translucent = 2,
    /// Additive
    Additive = 3,
    /// Modulate
    Modulate = 4,
}

/// Material blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MaterialBlendMode {
    /// Opaque
    #[default]
    Opaque = 0,
    /// Masked
    Masked = 1,
    /// Translucent
    Translucent = 2,
    /// Additive
    Additive = 3,
    /// Modulate
    Modulate = 4,
    /// Pre-multiplied alpha
    PreMultiplied = 5,
}

/// Shading model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadingModel {
    /// Default lit (PBR)
    #[default]
    DefaultLit = 0,
    /// Unlit
    Unlit = 1,
    /// Subsurface
    Subsurface = 2,
    /// Subsurface profile
    SubsurfaceProfile = 3,
    /// Clear coat
    ClearCoat = 4,
    /// Cloth
    Cloth = 5,
    /// Eye
    Eye = 6,
    /// Hair
    Hair = 7,
    /// Thin translucent
    ThinTranslucent = 8,
}

bitflags::bitflags! {
    /// Material graph features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct MaterialGraphFeatures: u32 {
        /// None
        const NONE = 0;
        /// Two-sided
        const TWO_SIDED = 1 << 0;
        /// Subsurface scattering
        const SUBSURFACE = 1 << 1;
        /// Clear coat
        const CLEAR_COAT = 1 << 2;
        /// Anisotropy
        const ANISOTROPY = 1 << 3;
        /// Refraction
        const REFRACTION = 1 << 4;
        /// Displacement
        const DISPLACEMENT = 1 << 5;
        /// Tessellation
        const TESSELLATION = 1 << 6;
        /// Vertex animation
        const VERTEX_ANIMATION = 1 << 7;
        /// Pixel depth offset
        const PIXEL_DEPTH_OFFSET = 1 << 8;
    }
}

// ============================================================================
// Graph Nodes
// ============================================================================

/// Graph node create info
#[derive(Clone, Debug)]
pub struct GraphNodeCreateInfo {
    /// Name
    pub name: String,
    /// Node type
    pub node_type: GraphNodeType,
    /// Position in editor
    pub position: [f32; 2],
    /// Initial values
    pub default_values: Vec<NodeValue>,
}

impl GraphNodeCreateInfo {
    /// Creates new info
    pub fn new(node_type: GraphNodeType) -> Self {
        Self {
            name: String::new(),
            node_type,
            position: [0.0, 0.0],
            default_values: Vec::new(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With position
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = [x, y];
        self
    }

    /// Add default value
    pub fn add_value(mut self, value: NodeValue) -> Self {
        self.default_values.push(value);
        self
    }

    /// Constant float
    pub fn constant_float(value: f32) -> Self {
        Self::new(GraphNodeType::ConstantFloat)
            .add_value(NodeValue::Float(value))
    }

    /// Constant vector3
    pub fn constant_vec3(x: f32, y: f32, z: f32) -> Self {
        Self::new(GraphNodeType::ConstantVec3)
            .add_value(NodeValue::Vec3([x, y, z]))
    }

    /// Constant color
    pub fn constant_color(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::new(GraphNodeType::ConstantVec4)
            .add_value(NodeValue::Vec4([r, g, b, a]))
    }

    /// Texture sample
    pub fn texture_sample() -> Self {
        Self::new(GraphNodeType::TextureSample)
    }

    /// Texture parameter
    pub fn texture_param(name: impl Into<String>) -> Self {
        Self::new(GraphNodeType::TextureParameter)
            .with_name(name)
    }

    /// Scalar parameter
    pub fn scalar_param(name: impl Into<String>, default: f32) -> Self {
        Self::new(GraphNodeType::ScalarParameter)
            .with_name(name)
            .add_value(NodeValue::Float(default))
    }

    /// Vector parameter
    pub fn vector_param(name: impl Into<String>, default: [f32; 4]) -> Self {
        Self::new(GraphNodeType::VectorParameter)
            .with_name(name)
            .add_value(NodeValue::Vec4(default))
    }
}

impl Default for GraphNodeCreateInfo {
    fn default() -> Self {
        Self::new(GraphNodeType::ConstantFloat)
    }
}

/// Graph node type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GraphNodeType {
    // Constants
    /// Constant float
    #[default]
    ConstantFloat = 0,
    /// Constant vec2
    ConstantVec2 = 1,
    /// Constant vec3
    ConstantVec3 = 2,
    /// Constant vec4
    ConstantVec4 = 3,

    // Parameters
    /// Scalar parameter
    ScalarParameter = 10,
    /// Vector parameter
    VectorParameter = 11,
    /// Texture parameter
    TextureParameter = 12,

    // Texture operations
    /// Texture sample
    TextureSample = 20,
    /// Texture sample bias
    TextureSampleBias = 21,
    /// Texture sample gradient
    TextureSampleGrad = 22,
    /// Texture sample level
    TextureSampleLevel = 23,

    // Math operations
    /// Add
    Add = 100,
    /// Subtract
    Subtract = 101,
    /// Multiply
    Multiply = 102,
    /// Divide
    Divide = 103,
    /// Power
    Power = 104,
    /// Sqrt
    Sqrt = 105,
    /// Abs
    Abs = 106,
    /// Floor
    Floor = 107,
    /// Ceil
    Ceil = 108,
    /// Frac
    Frac = 109,
    /// Sin
    Sin = 110,
    /// Cos
    Cos = 111,
    /// Lerp
    Lerp = 112,
    /// Clamp
    Clamp = 113,
    /// Saturate
    Saturate = 114,
    /// Step
    Step = 115,
    /// SmoothStep
    SmoothStep = 116,
    /// Min
    Min = 117,
    /// Max
    Max = 118,
    /// Dot
    Dot = 119,
    /// Cross
    Cross = 120,
    /// Normalize
    Normalize = 121,
    /// Length
    Length = 122,
    /// Distance
    Distance = 123,
    /// Reflect
    Reflect = 124,
    /// Refract
    Refract = 125,

    // Vector operations
    /// Component mask
    ComponentMask = 200,
    /// Append
    Append = 201,
    /// Break vec2
    BreakVec2 = 202,
    /// Break vec3
    BreakVec3 = 203,
    /// Break vec4
    BreakVec4 = 204,
    /// Make vec2
    MakeVec2 = 205,
    /// Make vec3
    MakeVec3 = 206,
    /// Make vec4
    MakeVec4 = 207,

    // Coordinates
    /// Texture coordinates
    TexCoord = 300,
    /// World position
    WorldPosition = 301,
    /// Object position
    ObjectPosition = 302,
    /// Camera position
    CameraPosition = 303,
    /// View direction
    ViewDirection = 304,
    /// Screen position
    ScreenPosition = 305,
    /// Vertex normal
    VertexNormal = 306,
    /// Vertex tangent
    VertexTangent = 307,
    /// Vertex color
    VertexColor = 308,

    // Utility
    /// Time
    Time = 400,
    /// Sine wave
    SineWave = 401,
    /// Noise
    Noise = 402,
    /// Fresnel
    Fresnel = 403,
    /// Normal from height
    NormalFromHeight = 404,
    /// Parallax occlusion
    ParallaxOcclusion = 405,

    // Output
    /// Material output
    MaterialOutput = 500,
}

impl GraphNodeType {
    /// Number of inputs
    pub const fn input_count(&self) -> u32 {
        match self {
            Self::ConstantFloat | Self::ConstantVec2 | Self::ConstantVec3 | Self::ConstantVec4 => 0,
            Self::ScalarParameter | Self::VectorParameter | Self::TextureParameter => 0,
            Self::TextureSample => 2,  // Texture + UV
            Self::Add | Self::Subtract | Self::Multiply | Self::Divide | Self::Dot | Self::Cross => 2,
            Self::Lerp | Self::Clamp | Self::SmoothStep => 3,
            Self::MaterialOutput => 8,  // All material outputs
            _ => 1,
        }
    }

    /// Output type
    pub const fn output_type(&self) -> ValueType {
        match self {
            Self::ConstantFloat | Self::ScalarParameter | Self::Dot | Self::Length | Self::Distance => ValueType::Float,
            Self::ConstantVec2 | Self::TexCoord => ValueType::Vec2,
            Self::ConstantVec3 | Self::WorldPosition | Self::Cross | Self::Normalize | Self::Reflect => ValueType::Vec3,
            Self::ConstantVec4 | Self::VectorParameter | Self::TextureSample | Self::VertexColor => ValueType::Vec4,
            Self::MaterialOutput => ValueType::Void,
            _ => ValueType::Float,
        }
    }
}

/// Node value
#[derive(Clone, Debug)]
pub enum NodeValue {
    /// Float
    Float(f32),
    /// Vec2
    Vec2([f32; 2]),
    /// Vec3
    Vec3([f32; 3]),
    /// Vec4
    Vec4([f32; 4]),
    /// Bool
    Bool(bool),
    /// Int
    Int(i32),
    /// Texture reference
    Texture(u64),
}

impl Default for NodeValue {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

/// Value type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ValueType {
    /// Float
    #[default]
    Float = 0,
    /// Vec2
    Vec2 = 1,
    /// Vec3
    Vec3 = 2,
    /// Vec4
    Vec4 = 3,
    /// Bool
    Bool = 4,
    /// Int
    Int = 5,
    /// Texture2D
    Texture2D = 6,
    /// TextureCube
    TextureCube = 7,
    /// Void
    Void = 100,
}

// ============================================================================
// Graph Connections
// ============================================================================

/// Graph connection
#[derive(Clone, Debug, Default)]
pub struct GraphConnection {
    /// Handle
    pub handle: GraphConnectionHandle,
    /// Source node
    pub source_node: GraphNodeHandle,
    /// Source output index
    pub source_output: u32,
    /// Target node
    pub target_node: GraphNodeHandle,
    /// Target input index
    pub target_input: u32,
}

impl GraphConnection {
    /// Creates new connection
    pub fn new(
        source_node: GraphNodeHandle,
        source_output: u32,
        target_node: GraphNodeHandle,
        target_input: u32,
    ) -> Self {
        Self {
            handle: GraphConnectionHandle::NULL,
            source_node,
            source_output,
            target_node,
            target_input,
        }
    }
}

// ============================================================================
// Material Outputs
// ============================================================================

/// Material output pin
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MaterialOutput {
    /// Base color
    #[default]
    BaseColor = 0,
    /// Metallic
    Metallic = 1,
    /// Specular
    Specular = 2,
    /// Roughness
    Roughness = 3,
    /// Emissive color
    EmissiveColor = 4,
    /// Opacity
    Opacity = 5,
    /// Opacity mask
    OpacityMask = 6,
    /// Normal
    Normal = 7,
    /// World position offset
    WorldPositionOffset = 8,
    /// Ambient occlusion
    AmbientOcclusion = 9,
    /// Refraction
    Refraction = 10,
    /// Pixel depth offset
    PixelDepthOffset = 11,
    /// Subsurface color
    SubsurfaceColor = 12,
    /// Clear coat
    ClearCoat = 13,
    /// Clear coat roughness
    ClearCoatRoughness = 14,
    /// Anisotropy
    Anisotropy = 15,
    /// Tangent
    Tangent = 16,
    /// Shading model
    ShadingModelFromMaterial = 17,
}

impl MaterialOutput {
    /// Expected value type
    pub const fn value_type(&self) -> ValueType {
        match self {
            Self::BaseColor | Self::EmissiveColor | Self::SubsurfaceColor => ValueType::Vec3,
            Self::Normal | Self::WorldPositionOffset | Self::Tangent => ValueType::Vec3,
            Self::Metallic | Self::Specular | Self::Roughness | Self::Opacity => ValueType::Float,
            Self::OpacityMask | Self::AmbientOcclusion | Self::ClearCoat => ValueType::Float,
            Self::ClearCoatRoughness | Self::PixelDepthOffset | Self::Anisotropy => ValueType::Float,
            Self::Refraction => ValueType::Float,
            Self::ShadingModelFromMaterial => ValueType::Int,
        }
    }

    /// Default value
    pub fn default_value(&self) -> NodeValue {
        match self {
            Self::BaseColor => NodeValue::Vec3([0.5, 0.5, 0.5]),
            Self::Metallic => NodeValue::Float(0.0),
            Self::Specular => NodeValue::Float(0.5),
            Self::Roughness => NodeValue::Float(0.5),
            Self::EmissiveColor => NodeValue::Vec3([0.0, 0.0, 0.0]),
            Self::Opacity => NodeValue::Float(1.0),
            Self::OpacityMask => NodeValue::Float(1.0),
            Self::Normal => NodeValue::Vec3([0.0, 0.0, 1.0]),
            Self::AmbientOcclusion => NodeValue::Float(1.0),
            Self::ClearCoat => NodeValue::Float(0.0),
            Self::ClearCoatRoughness => NodeValue::Float(0.0),
            _ => NodeValue::Float(0.0),
        }
    }
}

// ============================================================================
// Compilation
// ============================================================================

/// Material compile request
#[derive(Clone, Debug)]
pub struct MaterialCompileRequest {
    /// Graph handle
    pub graph: MaterialGraphHandle,
    /// Target API
    pub target: ShaderTarget,
    /// Optimization level
    pub optimization: OptimizationLevel,
    /// Features
    pub features: CompileFeatures,
}

impl MaterialCompileRequest {
    /// Creates new request
    pub fn new(graph: MaterialGraphHandle) -> Self {
        Self {
            graph,
            target: ShaderTarget::SpirV,
            optimization: OptimizationLevel::Performance,
            features: CompileFeatures::empty(),
        }
    }

    /// With target
    pub fn with_target(mut self, target: ShaderTarget) -> Self {
        self.target = target;
        self
    }

    /// With optimization
    pub fn with_optimization(mut self, level: OptimizationLevel) -> Self {
        self.optimization = level;
        self
    }

    /// Debug build
    pub fn debug(graph: MaterialGraphHandle) -> Self {
        Self::new(graph)
            .with_optimization(OptimizationLevel::None)
    }
}

impl Default for MaterialCompileRequest {
    fn default() -> Self {
        Self::new(MaterialGraphHandle::NULL)
    }
}

/// Shader target
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderTarget {
    /// SPIR-V
    #[default]
    SpirV = 0,
    /// HLSL
    Hlsl = 1,
    /// GLSL
    Glsl = 2,
    /// Metal
    Metal = 3,
}

/// Optimization level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum OptimizationLevel {
    /// None
    None = 0,
    /// Size
    Size = 1,
    /// Performance
    #[default]
    Performance = 2,
}

bitflags::bitflags! {
    /// Compile features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct CompileFeatures: u32 {
        /// None
        const NONE = 0;
        /// Debug info
        const DEBUG_INFO = 1 << 0;
        /// Validation
        const VALIDATION = 1 << 1;
        /// Strip reflection
        const STRIP_REFLECTION = 1 << 2;
    }
}

/// Compiled material info
#[derive(Clone, Debug, Default)]
pub struct CompiledMaterialInfo {
    /// Handle
    pub handle: CompiledMaterialHandle,
    /// Name
    pub name: String,
    /// Vertex shader size
    pub vertex_shader_size: u64,
    /// Fragment shader size
    pub fragment_shader_size: u64,
    /// Parameter count
    pub parameter_count: u32,
    /// Texture count
    pub texture_count: u32,
    /// Compile time (ms)
    pub compile_time_ms: f32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Material graph statistics
#[derive(Clone, Debug, Default)]
pub struct MaterialGraphStats {
    /// Node count
    pub node_count: u32,
    /// Connection count
    pub connection_count: u32,
    /// Parameter count
    pub parameter_count: u32,
    /// Texture count
    pub texture_count: u32,
    /// Instruction count (estimated)
    pub instruction_count: u32,
    /// Texture samples
    pub texture_samples: u32,
    /// Has vertex animation
    pub has_vertex_animation: bool,
    /// Is valid
    pub is_valid: bool,
    /// Error message
    pub error: Option<String>,
}
