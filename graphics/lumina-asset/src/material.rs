//! # Material System
//!
//! GPU material format with:
//! - PBR material model
//! - Shader variants
//! - Material instances

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::{AssetId, AssetResult};

/// Material definition
#[derive(Debug, Clone)]
pub struct Material {
    pub id: AssetId,
    pub name: String,
    pub shader: AssetId,
    pub domain: MaterialDomain,
    pub blend_mode: BlendMode,
    pub cull_mode: CullMode,
    pub properties: MaterialProperties,
    pub textures: MaterialTextures,
    pub flags: MaterialFlags,
}

/// Material domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialDomain {
    Opaque,
    Masked,
    Translucent,
    PostProcess,
    UI,
    Decal,
    Volume,
}

/// Blend mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Opaque,
    Masked,
    Additive,
    AlphaBlend,
    Premultiplied,
    Modulate,
}

/// Cull mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    None,
    Front,
    Back,
}

/// Material properties
#[derive(Debug, Clone)]
pub struct MaterialProperties {
    /// Albedo color (sRGB)
    pub albedo: [f32; 4],
    /// Metallic factor [0-1]
    pub metallic: f32,
    /// Roughness factor [0-1]
    pub roughness: f32,
    /// Reflectance (F0 for dielectrics) [0-1]
    pub reflectance: f32,
    /// Emission color (HDR)
    pub emission: [f32; 3],
    /// Emission strength
    pub emission_strength: f32,
    /// Normal map strength
    pub normal_strength: f32,
    /// Ambient occlusion strength
    pub ao_strength: f32,
    /// Height/parallax scale
    pub height_scale: f32,
    /// Alpha cutoff for masked materials
    pub alpha_cutoff: f32,
    /// Index of refraction for translucent
    pub ior: f32,
    /// Subsurface scattering color
    pub subsurface_color: [f32; 3],
    /// Subsurface scattering radius
    pub subsurface_radius: f32,
    /// Clearcoat layer strength
    pub clearcoat: f32,
    /// Clearcoat roughness
    pub clearcoat_roughness: f32,
    /// Anisotropy strength
    pub anisotropy: f32,
    /// Anisotropy rotation
    pub anisotropy_rotation: f32,
    /// Sheen color
    pub sheen_color: [f32; 3],
    /// Sheen roughness
    pub sheen_roughness: f32,
    /// UV transform
    pub uv_transform: [[f32; 3]; 2],
    /// Custom properties
    pub custom: BTreeMap<String, PropertyValue>,
}

impl Default for MaterialProperties {
    fn default() -> Self {
        Self {
            albedo: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            reflectance: 0.5,
            emission: [0.0, 0.0, 0.0],
            emission_strength: 0.0,
            normal_strength: 1.0,
            ao_strength: 1.0,
            height_scale: 0.05,
            alpha_cutoff: 0.5,
            ior: 1.5,
            subsurface_color: [1.0, 0.2, 0.1],
            subsurface_radius: 0.0,
            clearcoat: 0.0,
            clearcoat_roughness: 0.0,
            anisotropy: 0.0,
            anisotropy_rotation: 0.0,
            sheen_color: [0.0, 0.0, 0.0],
            sheen_roughness: 0.5,
            uv_transform: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            custom: BTreeMap::new(),
        }
    }
}

/// Property value
#[derive(Debug, Clone)]
pub enum PropertyValue {
    Float(f32),
    Float2([f32; 2]),
    Float3([f32; 3]),
    Float4([f32; 4]),
    Int(i32),
    Int2([i32; 2]),
    Int3([i32; 3]),
    Int4([i32; 4]),
    Bool(bool),
    Mat3([[f32; 3]; 3]),
    Mat4([[f32; 4]; 4]),
}

/// Material textures
#[derive(Debug, Clone, Default)]
pub struct MaterialTextures {
    /// Base color / albedo map
    pub albedo: Option<TextureRef>,
    /// Normal map (tangent space)
    pub normal: Option<TextureRef>,
    /// Metallic map
    pub metallic: Option<TextureRef>,
    /// Roughness map
    pub roughness: Option<TextureRef>,
    /// Ambient occlusion map
    pub ao: Option<TextureRef>,
    /// Emission map
    pub emission: Option<TextureRef>,
    /// Height / parallax map
    pub height: Option<TextureRef>,
    /// Combined metallic-roughness (glTF style)
    pub metallic_roughness: Option<TextureRef>,
    /// Combined ORM (occlusion-roughness-metallic)
    pub orm: Option<TextureRef>,
    /// Detail albedo
    pub detail_albedo: Option<TextureRef>,
    /// Detail normal
    pub detail_normal: Option<TextureRef>,
    /// Clearcoat normal
    pub clearcoat_normal: Option<TextureRef>,
    /// Subsurface color map
    pub subsurface: Option<TextureRef>,
    /// Anisotropy direction map
    pub anisotropy: Option<TextureRef>,
    /// Custom textures
    pub custom: BTreeMap<String, TextureRef>,
}

/// Texture reference
#[derive(Debug, Clone)]
pub struct TextureRef {
    pub asset_id: AssetId,
    pub uv_channel: u8,
    pub sampler: SamplerSettings,
}

/// Sampler settings
#[derive(Debug, Clone)]
pub struct SamplerSettings {
    pub wrap_u: WrapMode,
    pub wrap_v: WrapMode,
    pub filter_min: FilterMode,
    pub filter_mag: FilterMode,
    pub filter_mip: MipFilterMode,
    pub anisotropy: u8,
}

impl Default for SamplerSettings {
    fn default() -> Self {
        Self {
            wrap_u: WrapMode::Repeat,
            wrap_v: WrapMode::Repeat,
            filter_min: FilterMode::Linear,
            filter_mag: FilterMode::Linear,
            filter_mip: MipFilterMode::Linear,
            anisotropy: 16,
        }
    }
}

/// Wrap mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    Repeat,
    Mirror,
    Clamp,
    Border,
}

/// Filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Nearest,
    Linear,
}

/// Mip filter mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MipFilterMode {
    None,
    Nearest,
    Linear,
}

/// Material flags
#[derive(Debug, Clone, Copy, Default)]
pub struct MaterialFlags {
    pub double_sided: bool,
    pub cast_shadows: bool,
    pub receive_shadows: bool,
    pub unlit: bool,
    pub vertex_colors: bool,
    pub fog: bool,
    pub depth_write: bool,
    pub depth_test: bool,
}

/// Material instance (runtime variation)
#[derive(Debug, Clone)]
pub struct MaterialInstance {
    pub base_material: AssetId,
    pub property_overrides: BTreeMap<String, PropertyValue>,
    pub texture_overrides: BTreeMap<String, TextureRef>,
}

impl MaterialInstance {
    pub fn new(base_material: AssetId) -> Self {
        Self {
            base_material,
            property_overrides: BTreeMap::new(),
            texture_overrides: BTreeMap::new(),
        }
    }

    pub fn set_property(&mut self, name: &str, value: PropertyValue) {
        self.property_overrides.insert(name.into(), value);
    }

    pub fn set_texture(&mut self, name: &str, texture: TextureRef) {
        self.texture_overrides.insert(name.into(), texture);
    }
}

/// Material compiler
pub struct MaterialCompiler {
    templates: BTreeMap<String, MaterialTemplate>,
}

impl MaterialCompiler {
    pub fn new() -> Self {
        Self {
            templates: BTreeMap::new(),
        }
    }

    /// Register a material template
    pub fn register_template(&mut self, name: &str, template: MaterialTemplate) {
        self.templates.insert(name.into(), template);
    }

    /// Compile a material to GPU data
    pub fn compile(&self, material: &Material) -> AssetResult<CompiledMaterial> {
        // Pack properties into uniform buffer
        let uniform_data = pack_properties(&material.properties);

        // Generate texture bindings
        let mut texture_bindings = Vec::new();

        if let Some(ref tex) = material.textures.albedo {
            texture_bindings.push(TextureBinding {
                slot: 0,
                asset_id: tex.asset_id,
                sampler: tex.sampler.clone(),
            });
        }
        if let Some(ref tex) = material.textures.normal {
            texture_bindings.push(TextureBinding {
                slot: 1,
                asset_id: tex.asset_id,
                sampler: tex.sampler.clone(),
            });
        }
        if let Some(ref tex) = material.textures.metallic_roughness {
            texture_bindings.push(TextureBinding {
                slot: 2,
                asset_id: tex.asset_id,
                sampler: tex.sampler.clone(),
            });
        }
        if let Some(ref tex) = material.textures.ao {
            texture_bindings.push(TextureBinding {
                slot: 3,
                asset_id: tex.asset_id,
                sampler: tex.sampler.clone(),
            });
        }
        if let Some(ref tex) = material.textures.emission {
            texture_bindings.push(TextureBinding {
                slot: 4,
                asset_id: tex.asset_id,
                sampler: tex.sampler.clone(),
            });
        }

        // Determine shader variant
        let shader_variant = determine_shader_variant(material);

        Ok(CompiledMaterial {
            shader: material.shader,
            shader_variant,
            uniform_data,
            texture_bindings,
            blend_state: compile_blend_state(material.blend_mode),
            depth_stencil_state: compile_depth_stencil_state(material.domain),
            rasterizer_state: compile_rasterizer_state(material.cull_mode, material.flags),
        })
    }
}

impl Default for MaterialCompiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Material template
#[derive(Debug, Clone)]
pub struct MaterialTemplate {
    pub shader: AssetId,
    pub default_properties: MaterialProperties,
    pub default_textures: MaterialTextures,
    pub variants: Vec<ShaderVariant>,
}

/// Shader variant
#[derive(Debug, Clone)]
pub struct ShaderVariant {
    pub name: String,
    pub defines: Vec<String>,
    pub features: MaterialFeatures,
}

/// Material features bitflags
#[derive(Debug, Clone, Copy, Default)]
pub struct MaterialFeatures {
    pub normal_mapping: bool,
    pub parallax_mapping: bool,
    pub emission: bool,
    pub clearcoat: bool,
    pub subsurface: bool,
    pub anisotropy: bool,
    pub sheen: bool,
    pub vertex_animation: bool,
}

/// Compiled material
#[derive(Debug, Clone)]
pub struct CompiledMaterial {
    pub shader: AssetId,
    pub shader_variant: u32,
    pub uniform_data: Vec<u8>,
    pub texture_bindings: Vec<TextureBinding>,
    pub blend_state: BlendState,
    pub depth_stencil_state: DepthStencilState,
    pub rasterizer_state: RasterizerState,
}

/// Texture binding
#[derive(Debug, Clone)]
pub struct TextureBinding {
    pub slot: u32,
    pub asset_id: AssetId,
    pub sampler: SamplerSettings,
}

/// Blend state
#[derive(Debug, Clone)]
pub struct BlendState {
    pub enabled: bool,
    pub src_color: BlendFactor,
    pub dst_color: BlendFactor,
    pub color_op: BlendOp,
    pub src_alpha: BlendFactor,
    pub dst_alpha: BlendFactor,
    pub alpha_op: BlendOp,
}

/// Blend factor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// Blend operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendOp {
    Add,
    Subtract,
    ReverseSubtract,
    Min,
    Max,
}

/// Depth stencil state
#[derive(Debug, Clone)]
pub struct DepthStencilState {
    pub depth_test: bool,
    pub depth_write: bool,
    pub depth_func: CompareFunc,
}

/// Compare function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareFunc {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

/// Rasterizer state
#[derive(Debug, Clone)]
pub struct RasterizerState {
    pub cull_mode: CullMode,
    pub front_face: FrontFace,
    pub polygon_mode: PolygonMode,
    pub depth_bias: f32,
    pub depth_bias_slope: f32,
}

/// Front face winding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontFace {
    CounterClockwise,
    Clockwise,
}

/// Polygon mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolygonMode {
    Fill,
    Line,
    Point,
}

fn pack_properties(props: &MaterialProperties) -> Vec<u8> {
    let mut data = Vec::with_capacity(256);

    // Pack albedo
    for &v in &props.albedo {
        data.extend_from_slice(&v.to_le_bytes());
    }

    // Pack metallic/roughness/reflectance
    data.extend_from_slice(&props.metallic.to_le_bytes());
    data.extend_from_slice(&props.roughness.to_le_bytes());
    data.extend_from_slice(&props.reflectance.to_le_bytes());
    data.extend_from_slice(&0.0f32.to_le_bytes()); // Padding

    // Pack emission
    for &v in &props.emission {
        data.extend_from_slice(&v.to_le_bytes());
    }
    data.extend_from_slice(&props.emission_strength.to_le_bytes());

    // Pack various strengths
    data.extend_from_slice(&props.normal_strength.to_le_bytes());
    data.extend_from_slice(&props.ao_strength.to_le_bytes());
    data.extend_from_slice(&props.height_scale.to_le_bytes());
    data.extend_from_slice(&props.alpha_cutoff.to_le_bytes());

    data
}

fn determine_shader_variant(material: &Material) -> u32 {
    let mut variant = 0u32;

    if material.textures.normal.is_some() {
        variant |= 1 << 0;
    }
    if material.textures.emission.is_some() || material.properties.emission_strength > 0.0 {
        variant |= 1 << 1;
    }
    if material.properties.clearcoat > 0.0 {
        variant |= 1 << 2;
    }
    if material.properties.subsurface_radius > 0.0 {
        variant |= 1 << 3;
    }
    if material.properties.anisotropy != 0.0 {
        variant |= 1 << 4;
    }

    variant
}

fn compile_blend_state(mode: BlendMode) -> BlendState {
    match mode {
        BlendMode::Opaque | BlendMode::Masked => BlendState {
            enabled: false,
            src_color: BlendFactor::One,
            dst_color: BlendFactor::Zero,
            color_op: BlendOp::Add,
            src_alpha: BlendFactor::One,
            dst_alpha: BlendFactor::Zero,
            alpha_op: BlendOp::Add,
        },
        BlendMode::AlphaBlend => BlendState {
            enabled: true,
            src_color: BlendFactor::SrcAlpha,
            dst_color: BlendFactor::OneMinusSrcAlpha,
            color_op: BlendOp::Add,
            src_alpha: BlendFactor::One,
            dst_alpha: BlendFactor::OneMinusSrcAlpha,
            alpha_op: BlendOp::Add,
        },
        BlendMode::Additive => BlendState {
            enabled: true,
            src_color: BlendFactor::One,
            dst_color: BlendFactor::One,
            color_op: BlendOp::Add,
            src_alpha: BlendFactor::One,
            dst_alpha: BlendFactor::One,
            alpha_op: BlendOp::Add,
        },
        BlendMode::Premultiplied => BlendState {
            enabled: true,
            src_color: BlendFactor::One,
            dst_color: BlendFactor::OneMinusSrcAlpha,
            color_op: BlendOp::Add,
            src_alpha: BlendFactor::One,
            dst_alpha: BlendFactor::OneMinusSrcAlpha,
            alpha_op: BlendOp::Add,
        },
        BlendMode::Modulate => BlendState {
            enabled: true,
            src_color: BlendFactor::DstColor,
            dst_color: BlendFactor::Zero,
            color_op: BlendOp::Add,
            src_alpha: BlendFactor::DstAlpha,
            dst_alpha: BlendFactor::Zero,
            alpha_op: BlendOp::Add,
        },
    }
}

fn compile_depth_stencil_state(domain: MaterialDomain) -> DepthStencilState {
    match domain {
        MaterialDomain::Opaque | MaterialDomain::Masked => DepthStencilState {
            depth_test: true,
            depth_write: true,
            depth_func: CompareFunc::Less,
        },
        MaterialDomain::Translucent => DepthStencilState {
            depth_test: true,
            depth_write: false,
            depth_func: CompareFunc::Less,
        },
        MaterialDomain::PostProcess | MaterialDomain::UI => DepthStencilState {
            depth_test: false,
            depth_write: false,
            depth_func: CompareFunc::Always,
        },
        MaterialDomain::Decal => DepthStencilState {
            depth_test: true,
            depth_write: false,
            depth_func: CompareFunc::LessEqual,
        },
        MaterialDomain::Volume => DepthStencilState {
            depth_test: false,
            depth_write: false,
            depth_func: CompareFunc::Always,
        },
    }
}

fn compile_rasterizer_state(cull: CullMode, flags: MaterialFlags) -> RasterizerState {
    let cull_mode = if flags.double_sided {
        CullMode::None
    } else {
        cull
    };

    RasterizerState {
        cull_mode,
        front_face: FrontFace::CounterClockwise,
        polygon_mode: PolygonMode::Fill,
        depth_bias: 0.0,
        depth_bias_slope: 0.0,
    }
}
