//! Material System for Lumina
//!
//! This module provides a comprehensive material system with PBR materials,
//! shader bindings, material instances, and property management.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Material Handle
// ============================================================================

/// Material handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MaterialHandle(pub u64);

impl MaterialHandle {
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

impl Default for MaterialHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Material instance handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MaterialInstanceHandle(pub u64);

impl MaterialInstanceHandle {
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

impl Default for MaterialInstanceHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Material Type
// ============================================================================

/// Material type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MaterialType {
    /// Opaque PBR material
    #[default]
    OpaquePbr = 0,
    /// Transparent PBR material
    TransparentPbr = 1,
    /// Unlit material
    Unlit = 2,
    /// Subsurface scattering material
    Subsurface = 3,
    /// Clear coat material
    ClearCoat = 4,
    /// Cloth/fabric material
    Cloth = 5,
    /// Hair/fur material
    Hair = 6,
    /// Eye material
    Eye = 7,
    /// Terrain material
    Terrain = 8,
    /// Water material
    Water = 9,
    /// Glass material
    Glass = 10,
    /// Emissive material
    Emissive = 11,
    /// Custom material
    Custom = 255,
}

impl MaterialType {
    /// Is transparent
    #[inline]
    pub const fn is_transparent(&self) -> bool {
        matches!(
            self,
            Self::TransparentPbr | Self::Glass | Self::Water
        )
    }

    /// Needs alpha blending
    #[inline]
    pub const fn needs_alpha_blend(&self) -> bool {
        matches!(
            self,
            Self::TransparentPbr | Self::Glass
        )
    }

    /// Uses subsurface scattering
    #[inline]
    pub const fn uses_subsurface(&self) -> bool {
        matches!(
            self,
            Self::Subsurface | Self::Cloth | Self::Hair | Self::Eye
        )
    }
}

// ============================================================================
// Material Create Info
// ============================================================================

/// Material create info
#[derive(Clone, Debug)]
pub struct MaterialCreateInfo {
    /// Material type
    pub material_type: MaterialType,
    /// Domain
    pub domain: MaterialDomain,
    /// Blend mode
    pub blend_mode: BlendMode,
    /// Shading model
    pub shading_model: ShadingModel,
    /// Properties
    pub properties: Vec<MaterialProperty>,
    /// Textures
    pub textures: Vec<MaterialTexture>,
    /// Shader stages
    pub shader_stages: Vec<MaterialShaderStage>,
    /// Flags
    pub flags: MaterialFlags,
    /// Debug name
    pub debug_name: Option<String>,
}

impl MaterialCreateInfo {
    /// Creates opaque PBR material
    pub fn opaque_pbr() -> Self {
        Self {
            material_type: MaterialType::OpaquePbr,
            domain: MaterialDomain::Surface,
            blend_mode: BlendMode::Opaque,
            shading_model: ShadingModel::DefaultLit,
            properties: Vec::new(),
            textures: Vec::new(),
            shader_stages: Vec::new(),
            flags: MaterialFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates transparent PBR material
    pub fn transparent_pbr() -> Self {
        Self {
            material_type: MaterialType::TransparentPbr,
            domain: MaterialDomain::Surface,
            blend_mode: BlendMode::Translucent,
            shading_model: ShadingModel::DefaultLit,
            properties: Vec::new(),
            textures: Vec::new(),
            shader_stages: Vec::new(),
            flags: MaterialFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates unlit material
    pub fn unlit() -> Self {
        Self {
            material_type: MaterialType::Unlit,
            domain: MaterialDomain::Surface,
            blend_mode: BlendMode::Opaque,
            shading_model: ShadingModel::Unlit,
            properties: Vec::new(),
            textures: Vec::new(),
            shader_stages: Vec::new(),
            flags: MaterialFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates post-process material
    pub fn post_process() -> Self {
        Self {
            material_type: MaterialType::Custom,
            domain: MaterialDomain::PostProcess,
            blend_mode: BlendMode::Opaque,
            shading_model: ShadingModel::Unlit,
            properties: Vec::new(),
            textures: Vec::new(),
            shader_stages: Vec::new(),
            flags: MaterialFlags::NONE,
            debug_name: None,
        }
    }

    /// Creates UI material
    pub fn ui() -> Self {
        Self {
            material_type: MaterialType::Unlit,
            domain: MaterialDomain::UserInterface,
            blend_mode: BlendMode::Translucent,
            shading_model: ShadingModel::Unlit,
            properties: Vec::new(),
            textures: Vec::new(),
            shader_stages: Vec::new(),
            flags: MaterialFlags::NONE,
            debug_name: None,
        }
    }

    /// With blend mode
    pub fn with_blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// With shading model
    pub fn with_shading_model(mut self, model: ShadingModel) -> Self {
        self.shading_model = model;
        self
    }

    /// Add property
    pub fn add_property(mut self, property: MaterialProperty) -> Self {
        self.properties.push(property);
        self
    }

    /// Add texture
    pub fn add_texture(mut self, texture: MaterialTexture) -> Self {
        self.textures.push(texture);
        self
    }

    /// Add shader stage
    pub fn add_shader_stage(mut self, stage: MaterialShaderStage) -> Self {
        self.shader_stages.push(stage);
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: MaterialFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With debug name
    pub fn with_name(mut self, name: &str) -> Self {
        self.debug_name = Some(String::from(name));
        self
    }

    /// Two sided
    pub fn two_sided(mut self) -> Self {
        self.flags = self.flags.union(MaterialFlags::TWO_SIDED);
        self
    }

    /// With alpha test
    pub fn alpha_tested(mut self) -> Self {
        self.blend_mode = BlendMode::Masked;
        self
    }
}

impl Default for MaterialCreateInfo {
    fn default() -> Self {
        Self::opaque_pbr()
    }
}

// ============================================================================
// Material Domain
// ============================================================================

/// Material domain
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MaterialDomain {
    /// Surface rendering
    #[default]
    Surface = 0,
    /// Deferred decals
    DeferredDecal = 1,
    /// Light function
    LightFunction = 2,
    /// Volume
    Volume = 3,
    /// Post process
    PostProcess = 4,
    /// User interface
    UserInterface = 5,
    /// Runtime virtual texture
    RuntimeVirtualTexture = 6,
}

// ============================================================================
// Blend Mode
// ============================================================================

/// Blend mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BlendMode {
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
    /// Alpha composite
    AlphaComposite = 5,
    /// Alpha hold out
    AlphaHoldOut = 6,
}

impl BlendMode {
    /// Is opaque
    #[inline]
    pub const fn is_opaque(&self) -> bool {
        matches!(self, Self::Opaque | Self::Masked)
    }

    /// Is transparent
    #[inline]
    pub const fn is_transparent(&self) -> bool {
        !self.is_opaque()
    }

    /// Needs sorting
    #[inline]
    pub const fn needs_sorting(&self) -> bool {
        self.is_transparent()
    }
}

// ============================================================================
// Shading Model
// ============================================================================

/// Shading model
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShadingModel {
    /// Unlit
    Unlit = 0,
    /// Default lit (standard PBR)
    #[default]
    DefaultLit = 1,
    /// Subsurface
    Subsurface = 2,
    /// Preintegrated skin
    PreintegratedSkin = 3,
    /// Clear coat
    ClearCoat = 4,
    /// Subsurface profile
    SubsurfaceProfile = 5,
    /// Two sided foliage
    TwoSidedFoliage = 6,
    /// Hair
    Hair = 7,
    /// Cloth
    Cloth = 8,
    /// Eye
    Eye = 9,
    /// Single layer water
    SingleLayerWater = 10,
    /// Thin translucent
    ThinTranslucent = 11,
    /// From material expression
    FromMaterialExpression = 12,
}

impl ShadingModel {
    /// Uses subsurface scattering
    #[inline]
    pub const fn uses_subsurface(&self) -> bool {
        matches!(
            self,
            Self::Subsurface
                | Self::PreintegratedSkin
                | Self::SubsurfaceProfile
                | Self::TwoSidedFoliage
        )
    }

    /// Is lit
    #[inline]
    pub const fn is_lit(&self) -> bool {
        !matches!(self, Self::Unlit)
    }
}

// ============================================================================
// Material Flags
// ============================================================================

/// Material flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MaterialFlags(pub u32);

impl MaterialFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Two sided
    pub const TWO_SIDED: Self = Self(0x00000001);
    /// Dithered LOD transition
    pub const DITHERED_LOD_TRANSITION: Self = Self(0x00000002);
    /// Responsive AA
    pub const RESPONSIVE_AA: Self = Self(0x00000004);
    /// Contact shadows
    pub const CONTACT_SHADOWS: Self = Self(0x00000008);
    /// Allow negative emissive
    pub const ALLOW_NEGATIVE_EMISSIVE: Self = Self(0x00000010);
    /// Apply fogging
    pub const APPLY_FOGGING: Self = Self(0x00000020);
    /// Compute fog per pixel
    pub const COMPUTE_FOG_PER_PIXEL: Self = Self(0x00000040);
    /// Output velocity
    pub const OUTPUT_VELOCITY: Self = Self(0x00000080);
    /// Tangent space normal
    pub const TANGENT_SPACE_NORMAL: Self = Self(0x00000100);
    /// Use lightmap directionality
    pub const USE_LIGHTMAP_DIRECTIONALITY: Self = Self(0x00000200);
    /// Use hardware line rasterization
    pub const USE_HARDWARE_LINE_RASTERIZATION: Self = Self(0x00000400);
    /// Cast dynamic shadows
    pub const CAST_DYNAMIC_SHADOWS: Self = Self(0x00000800);
    /// Cast ray traced shadows
    pub const CAST_RAY_TRACED_SHADOWS: Self = Self(0x00001000);
    /// Used with static lighting
    pub const USED_WITH_STATIC_LIGHTING: Self = Self(0x00002000);
    /// Used with skeletal mesh
    pub const USED_WITH_SKELETAL_MESH: Self = Self(0x00004000);
    /// Used with editor compositing
    pub const USED_WITH_EDITOR_COMPOSITING: Self = Self(0x00008000);
    /// Used with particle sprites
    pub const USED_WITH_PARTICLE_SPRITES: Self = Self(0x00010000);
    /// Used with beam trails
    pub const USED_WITH_BEAM_TRAILS: Self = Self(0x00020000);
    /// Used with mesh particles
    pub const USED_WITH_MESH_PARTICLES: Self = Self(0x00040000);
    /// Used with instanced static meshes
    pub const USED_WITH_INSTANCED_STATIC_MESHES: Self = Self(0x00080000);
    /// Used with landscape
    pub const USED_WITH_LANDSCAPE: Self = Self(0x00100000);
    /// Used with spline meshes
    pub const USED_WITH_SPLINE_MESHES: Self = Self(0x00200000);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for MaterialFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// ============================================================================
// Material Property
// ============================================================================

/// Material property
#[derive(Clone, Debug)]
pub struct MaterialProperty {
    /// Name
    pub name: String,
    /// Value
    pub value: MaterialPropertyValue,
    /// Property type
    pub property_type: MaterialPropertyType,
    /// Flags
    pub flags: MaterialPropertyFlags,
    /// Group (for UI organization)
    pub group: Option<String>,
    /// Display name
    pub display_name: Option<String>,
    /// Description
    pub description: Option<String>,
}

impl MaterialProperty {
    /// Creates float property
    pub fn float(name: &str, value: f32) -> Self {
        Self {
            name: String::from(name),
            value: MaterialPropertyValue::Float(value),
            property_type: MaterialPropertyType::Scalar,
            flags: MaterialPropertyFlags::NONE,
            group: None,
            display_name: None,
            description: None,
        }
    }

    /// Creates vec2 property
    pub fn vec2(name: &str, x: f32, y: f32) -> Self {
        Self {
            name: String::from(name),
            value: MaterialPropertyValue::Vec2([x, y]),
            property_type: MaterialPropertyType::Vector2,
            flags: MaterialPropertyFlags::NONE,
            group: None,
            display_name: None,
            description: None,
        }
    }

    /// Creates vec3 property
    pub fn vec3(name: &str, x: f32, y: f32, z: f32) -> Self {
        Self {
            name: String::from(name),
            value: MaterialPropertyValue::Vec3([x, y, z]),
            property_type: MaterialPropertyType::Vector3,
            flags: MaterialPropertyFlags::NONE,
            group: None,
            display_name: None,
            description: None,
        }
    }

    /// Creates vec4 property
    pub fn vec4(name: &str, x: f32, y: f32, z: f32, w: f32) -> Self {
        Self {
            name: String::from(name),
            value: MaterialPropertyValue::Vec4([x, y, z, w]),
            property_type: MaterialPropertyType::Vector4,
            flags: MaterialPropertyFlags::NONE,
            group: None,
            display_name: None,
            description: None,
        }
    }

    /// Creates color property
    pub fn color(name: &str, r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            name: String::from(name),
            value: MaterialPropertyValue::Color([r, g, b, a]),
            property_type: MaterialPropertyType::Color,
            flags: MaterialPropertyFlags::NONE,
            group: None,
            display_name: None,
            description: None,
        }
    }

    /// Creates int property
    pub fn int(name: &str, value: i32) -> Self {
        Self {
            name: String::from(name),
            value: MaterialPropertyValue::Int(value),
            property_type: MaterialPropertyType::Int,
            flags: MaterialPropertyFlags::NONE,
            group: None,
            display_name: None,
            description: None,
        }
    }

    /// Creates bool property
    pub fn bool(name: &str, value: bool) -> Self {
        Self {
            name: String::from(name),
            value: MaterialPropertyValue::Bool(value),
            property_type: MaterialPropertyType::Bool,
            flags: MaterialPropertyFlags::NONE,
            group: None,
            display_name: None,
            description: None,
        }
    }

    /// With group
    pub fn with_group(mut self, group: &str) -> Self {
        self.group = Some(String::from(group));
        self
    }

    /// With display name
    pub fn with_display_name(mut self, name: &str) -> Self {
        self.display_name = Some(String::from(name));
        self
    }

    /// With description
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(String::from(desc));
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: MaterialPropertyFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Hidden
    pub fn hidden(mut self) -> Self {
        self.flags = self.flags.union(MaterialPropertyFlags::HIDDEN);
        self
    }
}

/// Material property value
#[derive(Clone, Debug)]
pub enum MaterialPropertyValue {
    /// Float
    Float(f32),
    /// Vec2
    Vec2([f32; 2]),
    /// Vec3
    Vec3([f32; 3]),
    /// Vec4
    Vec4([f32; 4]),
    /// Color
    Color([f32; 4]),
    /// Int
    Int(i32),
    /// UInt
    UInt(u32),
    /// Bool
    Bool(bool),
    /// Matrix4x4
    Matrix4([f32; 16]),
}

impl MaterialPropertyValue {
    /// Size in bytes
    pub const fn size(&self) -> usize {
        match self {
            Self::Float(_) => 4,
            Self::Vec2(_) => 8,
            Self::Vec3(_) => 12,
            Self::Vec4(_) | Self::Color(_) => 16,
            Self::Int(_) | Self::UInt(_) | Self::Bool(_) => 4,
            Self::Matrix4(_) => 64,
        }
    }

    /// As bytes
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Float(v) => bytemuck_cast_slice(core::slice::from_ref(v)),
            Self::Vec2(v) => bytemuck_cast_slice(v),
            Self::Vec3(v) => bytemuck_cast_slice(v),
            Self::Vec4(v) | Self::Color(v) => bytemuck_cast_slice(v),
            Self::Int(v) => bytemuck_cast_slice(core::slice::from_ref(v)),
            Self::UInt(v) => bytemuck_cast_slice(core::slice::from_ref(v)),
            Self::Bool(v) => {
                let u = if *v { 1u32 } else { 0u32 };
                // Safety: This is a bit of a hack for const bool to bytes
                unsafe {
                    core::slice::from_raw_parts(&u as *const u32 as *const u8, 4)
                }
            }
            Self::Matrix4(v) => bytemuck_cast_slice(v),
        }
    }
}

// Helper function for byte casting
fn bytemuck_cast_slice<T>(slice: &[T]) -> &[u8] {
    unsafe {
        core::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            slice.len() * core::mem::size_of::<T>(),
        )
    }
}

/// Material property type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MaterialPropertyType {
    /// Scalar (float)
    #[default]
    Scalar = 0,
    /// Vector2
    Vector2 = 1,
    /// Vector3
    Vector3 = 2,
    /// Vector4
    Vector4 = 3,
    /// Color (RGBA)
    Color = 4,
    /// Int
    Int = 5,
    /// Bool
    Bool = 6,
    /// Matrix
    Matrix = 7,
}

/// Material property flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MaterialPropertyFlags(pub u32);

impl MaterialPropertyFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Hidden
    pub const HIDDEN: Self = Self(0x00000001);
    /// Read only
    pub const READ_ONLY: Self = Self(0x00000002);
    /// Per instance
    pub const PER_INSTANCE: Self = Self(0x00000004);
    /// Requires update
    pub const REQUIRES_UPDATE: Self = Self(0x00000008);
    /// HDR color
    pub const HDR_COLOR: Self = Self(0x00000010);
    /// Show alpha
    pub const SHOW_ALPHA: Self = Self(0x00000020);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl core::ops::BitOr for MaterialPropertyFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

// ============================================================================
// Material Texture
// ============================================================================

/// Material texture slot
#[derive(Clone, Debug)]
pub struct MaterialTexture {
    /// Name
    pub name: String,
    /// Slot
    pub slot: TextureSlot,
    /// Texture handle (0 for unassigned)
    pub texture: u64,
    /// Sampler settings
    pub sampler: TextureSamplerSettings,
    /// UV set index
    pub uv_set: u32,
    /// Texture coordinate transform
    pub transform: Option<TextureTransform>,
}

impl MaterialTexture {
    /// Creates new texture slot
    pub fn new(name: &str, slot: TextureSlot) -> Self {
        Self {
            name: String::from(name),
            slot,
            texture: 0,
            sampler: TextureSamplerSettings::default(),
            uv_set: 0,
            transform: None,
        }
    }

    /// Albedo/base color texture
    pub fn albedo() -> Self {
        Self::new("albedo", TextureSlot::Albedo)
    }

    /// Normal map
    pub fn normal() -> Self {
        Self::new("normal", TextureSlot::Normal)
    }

    /// Metallic-roughness texture
    pub fn metallic_roughness() -> Self {
        Self::new("metallic_roughness", TextureSlot::MetallicRoughness)
    }

    /// Ambient occlusion
    pub fn ambient_occlusion() -> Self {
        Self::new("ao", TextureSlot::AmbientOcclusion)
    }

    /// Emissive
    pub fn emissive() -> Self {
        Self::new("emissive", TextureSlot::Emissive)
    }

    /// Height/displacement map
    pub fn height() -> Self {
        Self::new("height", TextureSlot::Height)
    }

    /// With texture handle
    pub fn with_texture(mut self, handle: u64) -> Self {
        self.texture = handle;
        self
    }

    /// With sampler
    pub fn with_sampler(mut self, sampler: TextureSamplerSettings) -> Self {
        self.sampler = sampler;
        self
    }

    /// With UV set
    pub fn with_uv_set(mut self, set: u32) -> Self {
        self.uv_set = set;
        self
    }

    /// With transform
    pub fn with_transform(mut self, transform: TextureTransform) -> Self {
        self.transform = Some(transform);
        self
    }
}

/// Texture slot
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum TextureSlot {
    /// Albedo/base color
    #[default]
    Albedo = 0,
    /// Normal map
    Normal = 1,
    /// Metallic-roughness (combined)
    MetallicRoughness = 2,
    /// Ambient occlusion
    AmbientOcclusion = 3,
    /// Emissive
    Emissive = 4,
    /// Height/displacement
    Height = 5,
    /// Opacity
    Opacity = 6,
    /// Specular
    Specular = 7,
    /// Glossiness
    Glossiness = 8,
    /// Subsurface
    Subsurface = 9,
    /// Clear coat
    ClearCoat = 10,
    /// Clear coat roughness
    ClearCoatRoughness = 11,
    /// Clear coat normal
    ClearCoatNormal = 12,
    /// Anisotropy
    Anisotropy = 13,
    /// Transmission
    Transmission = 14,
    /// Thickness
    Thickness = 15,
    /// Detail albedo
    DetailAlbedo = 16,
    /// Detail normal
    DetailNormal = 17,
    /// Custom 0
    Custom0 = 100,
    /// Custom 1
    Custom1 = 101,
    /// Custom 2
    Custom2 = 102,
    /// Custom 3
    Custom3 = 103,
}

/// Texture sampler settings
#[derive(Clone, Copy, Debug)]
pub struct TextureSamplerSettings {
    /// Filter mode
    pub filter: FilterMode,
    /// Address mode U
    pub address_u: AddressMode,
    /// Address mode V
    pub address_v: AddressMode,
    /// Address mode W
    pub address_w: AddressMode,
    /// Max anisotropy
    pub max_anisotropy: f32,
    /// Min LOD
    pub min_lod: f32,
    /// Max LOD
    pub max_lod: f32,
    /// LOD bias
    pub lod_bias: f32,
}

impl TextureSamplerSettings {
    /// Linear filtering, repeat
    pub const LINEAR_REPEAT: Self = Self {
        filter: FilterMode::Linear,
        address_u: AddressMode::Repeat,
        address_v: AddressMode::Repeat,
        address_w: AddressMode::Repeat,
        max_anisotropy: 1.0,
        min_lod: 0.0,
        max_lod: 1000.0,
        lod_bias: 0.0,
    };

    /// Linear filtering, clamp
    pub const LINEAR_CLAMP: Self = Self {
        filter: FilterMode::Linear,
        address_u: AddressMode::ClampToEdge,
        address_v: AddressMode::ClampToEdge,
        address_w: AddressMode::ClampToEdge,
        max_anisotropy: 1.0,
        min_lod: 0.0,
        max_lod: 1000.0,
        lod_bias: 0.0,
    };

    /// Nearest filtering, repeat
    pub const NEAREST_REPEAT: Self = Self {
        filter: FilterMode::Nearest,
        address_u: AddressMode::Repeat,
        address_v: AddressMode::Repeat,
        address_w: AddressMode::Repeat,
        max_anisotropy: 1.0,
        min_lod: 0.0,
        max_lod: 1000.0,
        lod_bias: 0.0,
    };

    /// Anisotropic filtering
    pub const ANISOTROPIC_16X: Self = Self {
        filter: FilterMode::Linear,
        address_u: AddressMode::Repeat,
        address_v: AddressMode::Repeat,
        address_w: AddressMode::Repeat,
        max_anisotropy: 16.0,
        min_lod: 0.0,
        max_lod: 1000.0,
        lod_bias: 0.0,
    };

    /// With anisotropy
    pub const fn with_anisotropy(mut self, level: f32) -> Self {
        self.max_anisotropy = level;
        self
    }

    /// With LOD bias
    pub const fn with_lod_bias(mut self, bias: f32) -> Self {
        self.lod_bias = bias;
        self
    }
}

impl Default for TextureSamplerSettings {
    fn default() -> Self {
        Self::LINEAR_REPEAT
    }
}

/// Filter mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum FilterMode {
    /// Nearest
    Nearest = 0,
    /// Linear
    #[default]
    Linear = 1,
}

/// Address mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AddressMode {
    /// Repeat
    #[default]
    Repeat = 0,
    /// Mirrored repeat
    MirroredRepeat = 1,
    /// Clamp to edge
    ClampToEdge = 2,
    /// Clamp to border
    ClampToBorder = 3,
    /// Mirror clamp to edge
    MirrorClampToEdge = 4,
}

/// Texture transform
#[derive(Clone, Copy, Debug)]
pub struct TextureTransform {
    /// Offset
    pub offset: [f32; 2],
    /// Scale
    pub scale: [f32; 2],
    /// Rotation (radians)
    pub rotation: f32,
}

impl TextureTransform {
    /// Identity transform
    pub const IDENTITY: Self = Self {
        offset: [0.0, 0.0],
        scale: [1.0, 1.0],
        rotation: 0.0,
    };

    /// Creates new transform
    pub const fn new(offset: [f32; 2], scale: [f32; 2], rotation: f32) -> Self {
        Self { offset, scale, rotation }
    }

    /// Scale only
    pub const fn scaled(x: f32, y: f32) -> Self {
        Self {
            offset: [0.0, 0.0],
            scale: [x, y],
            rotation: 0.0,
        }
    }

    /// Offset only
    pub const fn offset(x: f32, y: f32) -> Self {
        Self {
            offset: [x, y],
            scale: [1.0, 1.0],
            rotation: 0.0,
        }
    }
}

impl Default for TextureTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

// ============================================================================
// Material Shader Stage
// ============================================================================

/// Material shader stage
#[derive(Clone, Debug)]
pub struct MaterialShaderStage {
    /// Stage
    pub stage: ShaderStageType,
    /// Shader module handle
    pub module: u64,
    /// Entry point
    pub entry_point: String,
}

impl MaterialShaderStage {
    /// Creates new shader stage
    pub fn new(stage: ShaderStageType, module: u64, entry: &str) -> Self {
        Self {
            stage,
            module,
            entry_point: String::from(entry),
        }
    }

    /// Vertex stage
    pub fn vertex(module: u64, entry: &str) -> Self {
        Self::new(ShaderStageType::Vertex, module, entry)
    }

    /// Fragment stage
    pub fn fragment(module: u64, entry: &str) -> Self {
        Self::new(ShaderStageType::Fragment, module, entry)
    }
}

/// Shader stage type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum ShaderStageType {
    /// Vertex
    #[default]
    Vertex = 0,
    /// Fragment
    Fragment = 1,
    /// Geometry
    Geometry = 2,
    /// Tessellation control
    TessellationControl = 3,
    /// Tessellation evaluation
    TessellationEvaluation = 4,
    /// Compute
    Compute = 5,
    /// Task
    Task = 6,
    /// Mesh
    Mesh = 7,
}

// ============================================================================
// PBR Material Data
// ============================================================================

/// Standard PBR material data
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PbrMaterialData {
    /// Base color
    pub base_color: [f32; 4],
    /// Emissive color
    pub emissive: [f32; 4],
    /// Metallic
    pub metallic: f32,
    /// Roughness
    pub roughness: f32,
    /// Ambient occlusion
    pub ambient_occlusion: f32,
    /// Normal scale
    pub normal_scale: f32,
    /// Alpha cutoff
    pub alpha_cutoff: f32,
    /// UV scale X
    pub uv_scale_x: f32,
    /// UV scale Y
    pub uv_scale_y: f32,
    /// Padding
    pub _padding: f32,
}

impl PbrMaterialData {
    /// Default white material
    pub const DEFAULT: Self = Self {
        base_color: [1.0, 1.0, 1.0, 1.0],
        emissive: [0.0, 0.0, 0.0, 1.0],
        metallic: 0.0,
        roughness: 0.5,
        ambient_occlusion: 1.0,
        normal_scale: 1.0,
        alpha_cutoff: 0.5,
        uv_scale_x: 1.0,
        uv_scale_y: 1.0,
        _padding: 0.0,
    };

    /// Metal material
    pub const METAL: Self = Self {
        metallic: 1.0,
        roughness: 0.3,
        ..Self::DEFAULT
    };

    /// Plastic material
    pub const PLASTIC: Self = Self {
        metallic: 0.0,
        roughness: 0.4,
        ..Self::DEFAULT
    };

    /// Rough material
    pub const ROUGH: Self = Self {
        metallic: 0.0,
        roughness: 0.9,
        ..Self::DEFAULT
    };

    /// Creates with base color
    pub const fn with_base_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.base_color = [r, g, b, a];
        self
    }

    /// Creates with metallic-roughness
    pub const fn with_metallic_roughness(mut self, metallic: f32, roughness: f32) -> Self {
        self.metallic = metallic;
        self.roughness = roughness;
        self
    }

    /// With emissive
    pub const fn with_emissive(mut self, r: f32, g: f32, b: f32) -> Self {
        self.emissive = [r, g, b, 1.0];
        self
    }
}

impl Default for PbrMaterialData {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ============================================================================
// Material Instance
// ============================================================================

/// Material instance data
#[derive(Clone, Debug, Default)]
pub struct MaterialInstanceData {
    /// Parent material
    pub parent: MaterialHandle,
    /// Property overrides
    pub property_overrides: Vec<(String, MaterialPropertyValue)>,
    /// Texture overrides
    pub texture_overrides: Vec<(TextureSlot, u64)>,
}

impl MaterialInstanceData {
    /// Creates new instance
    pub fn new(parent: MaterialHandle) -> Self {
        Self {
            parent,
            property_overrides: Vec::new(),
            texture_overrides: Vec::new(),
        }
    }

    /// Set property
    pub fn set_property(mut self, name: &str, value: MaterialPropertyValue) -> Self {
        self.property_overrides.push((String::from(name), value));
        self
    }

    /// Set float
    pub fn set_float(self, name: &str, value: f32) -> Self {
        self.set_property(name, MaterialPropertyValue::Float(value))
    }

    /// Set color
    pub fn set_color(self, name: &str, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.set_property(name, MaterialPropertyValue::Color([r, g, b, a]))
    }

    /// Set texture
    pub fn set_texture(mut self, slot: TextureSlot, texture: u64) -> Self {
        self.texture_overrides.push((slot, texture));
        self
    }
}

// ============================================================================
// Material Library
// ============================================================================

/// Material library
#[derive(Debug, Default)]
pub struct MaterialLibrary {
    /// Materials by name
    materials: Vec<(String, MaterialHandle)>,
}

impl MaterialLibrary {
    /// Creates new library
    pub fn new() -> Self {
        Self::default()
    }

    /// Add material
    pub fn add(&mut self, name: &str, handle: MaterialHandle) {
        self.materials.push((String::from(name), handle));
    }

    /// Get material by name
    pub fn get(&self, name: &str) -> Option<MaterialHandle> {
        self.materials.iter().find(|(n, _)| n == name).map(|(_, h)| *h)
    }

    /// Material count
    pub fn len(&self) -> usize {
        self.materials.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }
}
