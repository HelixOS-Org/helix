//! GPU Compute Effects Types for Lumina
//!
//! This module provides compute shader-based visual effects
//! infrastructure for post-processing and real-time effects.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Compute Effect Handles
// ============================================================================

/// Compute effect system handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ComputeEffectSystemHandle(pub u64);

impl ComputeEffectSystemHandle {
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

impl Default for ComputeEffectSystemHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Compute effect handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ComputeEffectHandle(pub u64);

impl ComputeEffectHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ComputeEffectHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Effect chain handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EffectChainHandle(pub u64);

impl EffectChainHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for EffectChainHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Effect pass handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EffectPassHandle(pub u64);

impl EffectPassHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for EffectPassHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Compute Effect System Creation
// ============================================================================

/// Compute effect system create info
#[derive(Clone, Debug)]
pub struct ComputeEffectSystemCreateInfo {
    /// Name
    pub name: String,
    /// Max effects
    pub max_effects: u32,
    /// Max chains
    pub max_chains: u32,
    /// Max temp textures
    pub max_temp_textures: u32,
    /// Features
    pub features: ComputeEffectFeatures,
}

impl ComputeEffectSystemCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            max_effects: 64,
            max_chains: 16,
            max_temp_textures: 32,
            features: ComputeEffectFeatures::all(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max effects
    pub fn with_max_effects(mut self, count: u32) -> Self {
        self.max_effects = count;
        self
    }

    /// With max chains
    pub fn with_max_chains(mut self, count: u32) -> Self {
        self.max_chains = count;
        self
    }

    /// With max temp textures
    pub fn with_max_temp(mut self, count: u32) -> Self {
        self.max_temp_textures = count;
        self
    }

    /// With features
    pub fn with_features(mut self, features: ComputeEffectFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard system
    pub fn standard() -> Self {
        Self::new()
    }

    /// Minimal system
    pub fn minimal() -> Self {
        Self::new()
            .with_max_effects(16)
            .with_max_chains(4)
            .with_max_temp(8)
    }

    /// High capacity
    pub fn high_capacity() -> Self {
        Self::new()
            .with_max_effects(256)
            .with_max_chains(64)
            .with_max_temp(128)
    }
}

impl Default for ComputeEffectSystemCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

bitflags::bitflags! {
    /// Compute effect features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ComputeEffectFeatures: u32 {
        /// None
        const NONE = 0;
        /// Async compute
        const ASYNC_COMPUTE = 1 << 0;
        /// Half precision
        const HALF_PRECISION = 1 << 1;
        /// Wave intrinsics
        const WAVE_INTRINSICS = 1 << 2;
        /// Shared memory
        const SHARED_MEMORY = 1 << 3;
        /// Indirect dispatch
        const INDIRECT = 1 << 4;
        /// Multi-pass
        const MULTI_PASS = 1 << 5;
        /// Parameter animation
        const ANIMATION = 1 << 6;
        /// All
        const ALL = 0x7F;
    }
}

impl Default for ComputeEffectFeatures {
    fn default() -> Self {
        Self::all()
    }
}

// ============================================================================
// Effect Definition
// ============================================================================

/// Compute effect create info
#[derive(Clone, Debug)]
pub struct ComputeEffectCreateInfo {
    /// Name
    pub name: String,
    /// Effect type
    pub effect_type: ComputeEffectType,
    /// Shader code (SPIR-V)
    pub shader_code: Vec<u8>,
    /// Work group size
    pub work_group_size: [u32; 3],
    /// Parameters
    pub parameters: Vec<EffectParameter>,
    /// Input textures
    pub inputs: Vec<EffectTextureInput>,
    /// Output textures
    pub outputs: Vec<EffectTextureOutput>,
}

impl ComputeEffectCreateInfo {
    /// Creates new info
    pub fn new(effect_type: ComputeEffectType) -> Self {
        Self {
            name: String::new(),
            effect_type,
            shader_code: Vec::new(),
            work_group_size: [8, 8, 1],
            parameters: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With shader
    pub fn with_shader(mut self, code: Vec<u8>) -> Self {
        self.shader_code = code;
        self
    }

    /// With work group size
    pub fn with_work_group(mut self, x: u32, y: u32, z: u32) -> Self {
        self.work_group_size = [x, y, z];
        self
    }

    /// Add parameter
    pub fn add_parameter(mut self, param: EffectParameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Add input
    pub fn add_input(mut self, input: EffectTextureInput) -> Self {
        self.inputs.push(input);
        self
    }

    /// Add output
    pub fn add_output(mut self, output: EffectTextureOutput) -> Self {
        self.outputs.push(output);
        self
    }

    /// Blur effect
    pub fn blur() -> Self {
        Self::new(ComputeEffectType::Blur)
            .with_name("Blur")
            .with_work_group(8, 8, 1)
            .add_parameter(EffectParameter::float("radius", 5.0))
            .add_parameter(EffectParameter::float("sigma", 2.0))
    }

    /// Sharpen effect
    pub fn sharpen() -> Self {
        Self::new(ComputeEffectType::Sharpen)
            .with_name("Sharpen")
            .with_work_group(8, 8, 1)
            .add_parameter(EffectParameter::float("strength", 0.5))
    }

    /// Edge detection
    pub fn edge_detect() -> Self {
        Self::new(ComputeEffectType::EdgeDetect)
            .with_name("EdgeDetect")
            .with_work_group(8, 8, 1)
            .add_parameter(EffectParameter::float("threshold", 0.1))
    }

    /// Color grading
    pub fn color_grade() -> Self {
        Self::new(ComputeEffectType::ColorGrade)
            .with_name("ColorGrade")
            .with_work_group(8, 8, 1)
            .add_parameter(EffectParameter::float("exposure", 1.0))
            .add_parameter(EffectParameter::float("contrast", 1.0))
            .add_parameter(EffectParameter::float("saturation", 1.0))
    }

    /// Bloom
    pub fn bloom() -> Self {
        Self::new(ComputeEffectType::Bloom)
            .with_name("Bloom")
            .with_work_group(8, 8, 1)
            .add_parameter(EffectParameter::float("threshold", 1.0))
            .add_parameter(EffectParameter::float("intensity", 0.5))
            .add_parameter(EffectParameter::float("radius", 4.0))
    }

    /// FXAA
    pub fn fxaa() -> Self {
        Self::new(ComputeEffectType::Fxaa)
            .with_name("FXAA")
            .with_work_group(8, 8, 1)
            .add_parameter(EffectParameter::float("subpix", 0.75))
            .add_parameter(EffectParameter::float("edge_threshold", 0.166))
    }
}

impl Default for ComputeEffectCreateInfo {
    fn default() -> Self {
        Self::new(ComputeEffectType::Custom)
    }
}

/// Compute effect type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ComputeEffectType {
    /// Custom effect
    #[default]
    Custom = 0,
    /// Gaussian blur
    Blur = 1,
    /// Sharpen
    Sharpen = 2,
    /// Edge detection
    EdgeDetect = 3,
    /// Color grading
    ColorGrade = 4,
    /// Bloom
    Bloom = 5,
    /// Vignette
    Vignette = 6,
    /// Chromatic aberration
    ChromaticAberration = 7,
    /// Film grain
    FilmGrain = 8,
    /// FXAA
    Fxaa = 9,
    /// SMAA
    Smaa = 10,
    /// Motion blur
    MotionBlur = 11,
    /// Depth of field
    DepthOfField = 12,
    /// Lens distortion
    LensDistortion = 13,
    /// HDR tonemapping
    Tonemap = 14,
    /// Histogram
    Histogram = 15,
    /// Auto exposure
    AutoExposure = 16,
    /// SSAO
    Ssao = 17,
    /// SSR
    Ssr = 18,
    /// Denoise
    Denoise = 19,
    /// Upscale
    Upscale = 20,
    /// Downsample
    Downsample = 21,
}

// ============================================================================
// Effect Parameters
// ============================================================================

/// Effect parameter
#[derive(Clone, Debug)]
pub struct EffectParameter {
    /// Name
    pub name: String,
    /// Type
    pub param_type: EffectParamType,
    /// Value
    pub value: EffectParamValue,
    /// Min value
    pub min_value: Option<EffectParamValue>,
    /// Max value
    pub max_value: Option<EffectParamValue>,
}

impl EffectParameter {
    /// Creates new parameter
    pub fn new(name: impl Into<String>, param_type: EffectParamType) -> Self {
        Self {
            name: name.into(),
            param_type,
            value: EffectParamValue::Float(0.0),
            min_value: None,
            max_value: None,
        }
    }

    /// Float parameter
    pub fn float(name: impl Into<String>, value: f32) -> Self {
        Self {
            name: name.into(),
            param_type: EffectParamType::Float,
            value: EffectParamValue::Float(value),
            min_value: None,
            max_value: None,
        }
    }

    /// Float with range
    pub fn float_range(name: impl Into<String>, value: f32, min: f32, max: f32) -> Self {
        Self {
            name: name.into(),
            param_type: EffectParamType::Float,
            value: EffectParamValue::Float(value),
            min_value: Some(EffectParamValue::Float(min)),
            max_value: Some(EffectParamValue::Float(max)),
        }
    }

    /// Int parameter
    pub fn int(name: impl Into<String>, value: i32) -> Self {
        Self {
            name: name.into(),
            param_type: EffectParamType::Int,
            value: EffectParamValue::Int(value),
            min_value: None,
            max_value: None,
        }
    }

    /// Bool parameter
    pub fn bool(name: impl Into<String>, value: bool) -> Self {
        Self {
            name: name.into(),
            param_type: EffectParamType::Bool,
            value: EffectParamValue::Bool(value),
            min_value: None,
            max_value: None,
        }
    }

    /// Vec2 parameter
    pub fn vec2(name: impl Into<String>, value: [f32; 2]) -> Self {
        Self {
            name: name.into(),
            param_type: EffectParamType::Vec2,
            value: EffectParamValue::Vec2(value),
            min_value: None,
            max_value: None,
        }
    }

    /// Vec3 parameter
    pub fn vec3(name: impl Into<String>, value: [f32; 3]) -> Self {
        Self {
            name: name.into(),
            param_type: EffectParamType::Vec3,
            value: EffectParamValue::Vec3(value),
            min_value: None,
            max_value: None,
        }
    }

    /// Vec4 parameter
    pub fn vec4(name: impl Into<String>, value: [f32; 4]) -> Self {
        Self {
            name: name.into(),
            param_type: EffectParamType::Vec4,
            value: EffectParamValue::Vec4(value),
            min_value: None,
            max_value: None,
        }
    }

    /// Color parameter
    pub fn color(name: impl Into<String>, value: [f32; 4]) -> Self {
        Self {
            name: name.into(),
            param_type: EffectParamType::Color,
            value: EffectParamValue::Vec4(value),
            min_value: None,
            max_value: None,
        }
    }
}

impl Default for EffectParameter {
    fn default() -> Self {
        Self::float("", 0.0)
    }
}

/// Effect parameter type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum EffectParamType {
    /// Float
    #[default]
    Float = 0,
    /// Int
    Int = 1,
    /// Bool
    Bool = 2,
    /// Vec2
    Vec2 = 3,
    /// Vec3
    Vec3 = 4,
    /// Vec4
    Vec4 = 5,
    /// Color
    Color = 6,
    /// Matrix
    Matrix = 7,
    /// Texture
    Texture = 8,
}

/// Effect parameter value
#[derive(Clone, Copy, Debug)]
pub enum EffectParamValue {
    /// Float
    Float(f32),
    /// Int
    Int(i32),
    /// Bool
    Bool(bool),
    /// Vec2
    Vec2([f32; 2]),
    /// Vec3
    Vec3([f32; 3]),
    /// Vec4
    Vec4([f32; 4]),
    /// Matrix
    Matrix([[f32; 4]; 4]),
    /// Texture handle
    Texture(u64),
}

impl Default for EffectParamValue {
    fn default() -> Self {
        Self::Float(0.0)
    }
}

// ============================================================================
// Effect Textures
// ============================================================================

/// Effect texture input
#[derive(Clone, Debug)]
pub struct EffectTextureInput {
    /// Name
    pub name: String,
    /// Binding
    pub binding: u32,
    /// Format
    pub format: EffectTextureFormat,
    /// Source
    pub source: EffectTextureSource,
}

impl EffectTextureInput {
    /// Creates new input
    pub fn new(name: impl Into<String>, binding: u32) -> Self {
        Self {
            name: name.into(),
            binding,
            format: EffectTextureFormat::Rgba16Float,
            source: EffectTextureSource::Previous,
        }
    }

    /// With format
    pub fn with_format(mut self, format: EffectTextureFormat) -> Self {
        self.format = format;
        self
    }

    /// With source
    pub fn with_source(mut self, source: EffectTextureSource) -> Self {
        self.source = source;
        self
    }

    /// Color input
    pub fn color(binding: u32) -> Self {
        Self::new("color", binding)
            .with_source(EffectTextureSource::SceneColor)
    }

    /// Depth input
    pub fn depth(binding: u32) -> Self {
        Self::new("depth", binding)
            .with_format(EffectTextureFormat::R32Float)
            .with_source(EffectTextureSource::SceneDepth)
    }

    /// Normal input
    pub fn normal(binding: u32) -> Self {
        Self::new("normal", binding)
            .with_source(EffectTextureSource::GBufferNormal)
    }
}

impl Default for EffectTextureInput {
    fn default() -> Self {
        Self::new("", 0)
    }
}

/// Effect texture output
#[derive(Clone, Debug)]
pub struct EffectTextureOutput {
    /// Name
    pub name: String,
    /// Binding
    pub binding: u32,
    /// Format
    pub format: EffectTextureFormat,
    /// Scale (relative to input)
    pub scale: f32,
}

impl EffectTextureOutput {
    /// Creates new output
    pub fn new(name: impl Into<String>, binding: u32) -> Self {
        Self {
            name: name.into(),
            binding,
            format: EffectTextureFormat::Rgba16Float,
            scale: 1.0,
        }
    }

    /// With format
    pub fn with_format(mut self, format: EffectTextureFormat) -> Self {
        self.format = format;
        self
    }

    /// With scale
    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Main output
    pub fn main(binding: u32) -> Self {
        Self::new("output", binding)
    }

    /// Half resolution
    pub fn half_res(binding: u32) -> Self {
        Self::new("output", binding).with_scale(0.5)
    }

    /// Quarter resolution
    pub fn quarter_res(binding: u32) -> Self {
        Self::new("output", binding).with_scale(0.25)
    }
}

impl Default for EffectTextureOutput {
    fn default() -> Self {
        Self::new("", 0)
    }
}

/// Effect texture format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum EffectTextureFormat {
    /// RGBA8 unorm
    Rgba8Unorm = 0,
    /// RGBA8 sRGB
    Rgba8Srgb = 1,
    /// RGBA16 float
    #[default]
    Rgba16Float = 2,
    /// RGBA32 float
    Rgba32Float = 3,
    /// R16 float
    R16Float = 4,
    /// R32 float
    R32Float = 5,
    /// RG16 float
    Rg16Float = 6,
    /// RG32 float
    Rg32Float = 7,
    /// R11G11B10 float
    R11G11B10Float = 8,
}

/// Effect texture source
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum EffectTextureSource {
    /// Previous effect output
    #[default]
    Previous = 0,
    /// Scene color
    SceneColor = 1,
    /// Scene depth
    SceneDepth = 2,
    /// G-buffer normal
    GBufferNormal = 3,
    /// G-buffer albedo
    GBufferAlbedo = 4,
    /// G-buffer material
    GBufferMaterial = 5,
    /// Velocity buffer
    Velocity = 6,
    /// Custom texture
    Custom = 7,
}

// ============================================================================
// Effect Chain
// ============================================================================

/// Effect chain create info
#[derive(Clone, Debug)]
pub struct EffectChainCreateInfo {
    /// Name
    pub name: String,
    /// Effects in chain
    pub effects: Vec<ChainedEffect>,
    /// Input resolution
    pub input_resolution: [u32; 2],
    /// Output resolution
    pub output_resolution: [u32; 2],
    /// Enabled
    pub enabled: bool,
}

impl EffectChainCreateInfo {
    /// Creates new info
    pub fn new() -> Self {
        Self {
            name: String::new(),
            effects: Vec::new(),
            input_resolution: [0, 0],
            output_resolution: [0, 0],
            enabled: true,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Add effect
    pub fn add_effect(mut self, effect: ChainedEffect) -> Self {
        self.effects.push(effect);
        self
    }

    /// With resolution
    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.input_resolution = [width, height];
        self.output_resolution = [width, height];
        self
    }

    /// Post-processing chain
    pub fn post_process() -> Self {
        Self::new()
            .with_name("PostProcess")
            .add_effect(ChainedEffect::effect(ComputeEffectType::Bloom))
            .add_effect(ChainedEffect::effect(ComputeEffectType::ColorGrade))
            .add_effect(ChainedEffect::effect(ComputeEffectType::Tonemap))
            .add_effect(ChainedEffect::effect(ComputeEffectType::Fxaa))
    }

    /// HDR chain
    pub fn hdr() -> Self {
        Self::new()
            .with_name("HDR")
            .add_effect(ChainedEffect::effect(ComputeEffectType::AutoExposure))
            .add_effect(ChainedEffect::effect(ComputeEffectType::Bloom))
            .add_effect(ChainedEffect::effect(ComputeEffectType::Tonemap))
    }
}

impl Default for EffectChainCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Chained effect
#[derive(Clone, Debug)]
pub struct ChainedEffect {
    /// Effect type
    pub effect_type: ComputeEffectType,
    /// Effect handle (if custom)
    pub effect_handle: ComputeEffectHandle,
    /// Parameter overrides
    pub parameters: Vec<(String, EffectParamValue)>,
    /// Enabled
    pub enabled: bool,
    /// Blend mode
    pub blend_mode: EffectBlendMode,
    /// Blend factor
    pub blend_factor: f32,
}

impl ChainedEffect {
    /// Creates from effect type
    pub fn effect(effect_type: ComputeEffectType) -> Self {
        Self {
            effect_type,
            effect_handle: ComputeEffectHandle::NULL,
            parameters: Vec::new(),
            enabled: true,
            blend_mode: EffectBlendMode::Replace,
            blend_factor: 1.0,
        }
    }

    /// Creates from handle
    pub fn custom(handle: ComputeEffectHandle) -> Self {
        Self {
            effect_type: ComputeEffectType::Custom,
            effect_handle: handle,
            parameters: Vec::new(),
            enabled: true,
            blend_mode: EffectBlendMode::Replace,
            blend_factor: 1.0,
        }
    }

    /// Set parameter
    pub fn set_param(mut self, name: impl Into<String>, value: EffectParamValue) -> Self {
        self.parameters.push((name.into(), value));
        self
    }

    /// With blend
    pub fn with_blend(mut self, mode: EffectBlendMode, factor: f32) -> Self {
        self.blend_mode = mode;
        self.blend_factor = factor;
        self
    }

    /// Disabled
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

impl Default for ChainedEffect {
    fn default() -> Self {
        Self::effect(ComputeEffectType::Custom)
    }
}

/// Effect blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum EffectBlendMode {
    /// Replace
    #[default]
    Replace = 0,
    /// Additive
    Additive = 1,
    /// Multiply
    Multiply = 2,
    /// Lerp with original
    Lerp = 3,
}

// ============================================================================
// GPU Structures
// ============================================================================

/// GPU effect constants
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuEffectConstants {
    /// Input size
    pub input_size: [f32; 2],
    /// Output size
    pub output_size: [f32; 2],
    /// Texel size (1/size)
    pub texel_size: [f32; 2],
    /// Time
    pub time: f32,
    /// Delta time
    pub delta_time: f32,
    /// Frame index
    pub frame_index: u32,
    /// Effect-specific params
    pub params: [f32; 15],
}

/// GPU dispatch info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuDispatchInfo {
    /// Group count X
    pub group_count_x: u32,
    /// Group count Y
    pub group_count_y: u32,
    /// Group count Z
    pub group_count_z: u32,
    /// Flags
    pub flags: u32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Compute effect statistics
#[derive(Clone, Debug, Default)]
pub struct ComputeEffectStats {
    /// Effects executed
    pub effects_executed: u32,
    /// Chains executed
    pub chains_executed: u32,
    /// Dispatches
    pub dispatches: u32,
    /// Barriers
    pub barriers: u32,
    /// Temp textures used
    pub temp_textures_used: u32,
    /// Temp texture memory
    pub temp_memory: u64,
    /// Total GPU time (ms)
    pub gpu_time_ms: f32,
}

impl ComputeEffectStats {
    /// Average time per effect
    pub fn avg_time_per_effect(&self) -> f32 {
        if self.effects_executed == 0 {
            0.0
        } else {
            self.gpu_time_ms / self.effects_executed as f32
        }
    }
}
