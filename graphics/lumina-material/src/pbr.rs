//! PBR Material System
//!
//! This module provides physically-based rendering materials with support
//! for metallic-roughness and specular-glossiness workflows.

use alloc::{string::String, vec::Vec};

// ============================================================================
// PBR Parameters
// ============================================================================

/// PBR material parameters.
#[derive(Debug, Clone)]
pub struct PbrParameters {
    /// Base color factor.
    pub base_color: [f32; 4],
    /// Metallic factor.
    pub metallic: f32,
    /// Roughness factor.
    pub roughness: f32,
    /// Normal scale.
    pub normal_scale: f32,
    /// Occlusion strength.
    pub occlusion_strength: f32,
    /// Emissive factor.
    pub emissive: [f32; 3],
    /// Emissive strength.
    pub emissive_strength: f32,
    /// Alpha cutoff.
    pub alpha_cutoff: f32,
    /// IOR (index of refraction).
    pub ior: f32,
}

impl Default for PbrParameters {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            emissive: [0.0, 0.0, 0.0],
            emissive_strength: 1.0,
            alpha_cutoff: 0.5,
            ior: 1.5,
        }
    }
}

impl PbrParameters {
    /// Create with base color.
    pub fn with_color(r: f32, g: f32, b: f32) -> Self {
        Self {
            base_color: [r, g, b, 1.0],
            ..Default::default()
        }
    }

    /// Create metallic material.
    pub fn metallic(metallic: f32, roughness: f32) -> Self {
        Self {
            metallic,
            roughness,
            ..Default::default()
        }
    }

    /// Create dielectric material.
    pub fn dielectric(roughness: f32) -> Self {
        Self {
            metallic: 0.0,
            roughness,
            ..Default::default()
        }
    }

    /// Convert to GPU-friendly format.
    pub fn to_gpu_data(&self) -> PbrGpuData {
        PbrGpuData {
            base_color: self.base_color,
            metallic_roughness: [self.metallic, self.roughness, 0.0, 0.0],
            emissive: [
                self.emissive[0] * self.emissive_strength,
                self.emissive[1] * self.emissive_strength,
                self.emissive[2] * self.emissive_strength,
                0.0,
            ],
            params: [
                self.normal_scale,
                self.occlusion_strength,
                self.alpha_cutoff,
                self.ior,
            ],
        }
    }
}

/// PBR GPU data layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PbrGpuData {
    /// Base color (RGBA).
    pub base_color: [f32; 4],
    /// Metallic (x), roughness (y).
    pub metallic_roughness: [f32; 4],
    /// Emissive (RGB).
    pub emissive: [f32; 4],
    /// Normal scale (x), occlusion (y), alpha cutoff (z), IOR (w).
    pub params: [f32; 4],
}

impl PbrGpuData {
    /// Size in bytes.
    pub const SIZE: usize = 64;
}

// ============================================================================
// Metallic-Roughness Workflow
// ============================================================================

/// Metallic-roughness workflow.
#[derive(Debug, Clone, Default)]
pub struct MetallicRoughness {
    /// Base color factor.
    pub base_color_factor: [f32; 4],
    /// Base color texture.
    pub base_color_texture: Option<TextureInfo>,
    /// Metallic factor.
    pub metallic_factor: f32,
    /// Roughness factor.
    pub roughness_factor: f32,
    /// Metallic-roughness texture.
    pub metallic_roughness_texture: Option<TextureInfo>,
}

impl MetallicRoughness {
    /// Create a new metallic-roughness workflow.
    pub fn new() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            ..Default::default()
        }
    }

    /// Set base color.
    pub fn base_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.base_color_factor = [r, g, b, a];
        self
    }

    /// Set metallic factor.
    pub fn metallic(mut self, metallic: f32) -> Self {
        self.metallic_factor = metallic;
        self
    }

    /// Set roughness factor.
    pub fn roughness(mut self, roughness: f32) -> Self {
        self.roughness_factor = roughness;
        self
    }
}

// ============================================================================
// Specular-Glossiness Workflow
// ============================================================================

/// Specular-glossiness workflow (KHR_materials_pbrSpecularGlossiness).
#[derive(Debug, Clone)]
pub struct SpecularGlossiness {
    /// Diffuse factor.
    pub diffuse_factor: [f32; 4],
    /// Diffuse texture.
    pub diffuse_texture: Option<TextureInfo>,
    /// Specular factor.
    pub specular_factor: [f32; 3],
    /// Glossiness factor.
    pub glossiness_factor: f32,
    /// Specular-glossiness texture.
    pub specular_glossiness_texture: Option<TextureInfo>,
}

impl Default for SpecularGlossiness {
    fn default() -> Self {
        Self {
            diffuse_factor: [1.0, 1.0, 1.0, 1.0],
            diffuse_texture: None,
            specular_factor: [1.0, 1.0, 1.0],
            glossiness_factor: 1.0,
            specular_glossiness_texture: None,
        }
    }
}

impl SpecularGlossiness {
    /// Convert to metallic-roughness workflow.
    pub fn to_metallic_roughness(&self) -> MetallicRoughness {
        // Approximate conversion
        let max_specular = self.specular_factor[0]
            .max(self.specular_factor[1])
            .max(self.specular_factor[2]);

        let metallic = max_specular;
        let roughness = 1.0 - self.glossiness_factor;

        MetallicRoughness {
            base_color_factor: self.diffuse_factor,
            metallic_factor: metallic,
            roughness_factor: roughness,
            ..Default::default()
        }
    }
}

// ============================================================================
// Clear Coat Extension
// ============================================================================

/// Clear coat extension (KHR_materials_clearcoat).
#[derive(Debug, Clone, Default)]
pub struct ClearCoat {
    /// Clear coat factor.
    pub factor: f32,
    /// Clear coat texture.
    pub texture: Option<TextureInfo>,
    /// Clear coat roughness factor.
    pub roughness_factor: f32,
    /// Clear coat roughness texture.
    pub roughness_texture: Option<TextureInfo>,
    /// Clear coat normal texture.
    pub normal_texture: Option<TextureInfo>,
    /// Normal scale.
    pub normal_scale: f32,
}

impl ClearCoat {
    /// Create a new clear coat.
    pub fn new(factor: f32, roughness: f32) -> Self {
        Self {
            factor,
            roughness_factor: roughness,
            normal_scale: 1.0,
            ..Default::default()
        }
    }

    /// Convert to GPU data.
    pub fn to_gpu_data(&self) -> ClearCoatGpuData {
        ClearCoatGpuData {
            factor: self.factor,
            roughness: self.roughness_factor,
            normal_scale: self.normal_scale,
            _padding: 0.0,
        }
    }
}

/// Clear coat GPU data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ClearCoatGpuData {
    pub factor: f32,
    pub roughness: f32,
    pub normal_scale: f32,
    pub _padding: f32,
}

// ============================================================================
// Sheen Extension
// ============================================================================

/// Sheen extension (KHR_materials_sheen).
#[derive(Debug, Clone, Default)]
pub struct Sheen {
    /// Sheen color factor.
    pub color_factor: [f32; 3],
    /// Sheen color texture.
    pub color_texture: Option<TextureInfo>,
    /// Sheen roughness factor.
    pub roughness_factor: f32,
    /// Sheen roughness texture.
    pub roughness_texture: Option<TextureInfo>,
}

impl Sheen {
    /// Create a new sheen.
    pub fn new(color: [f32; 3], roughness: f32) -> Self {
        Self {
            color_factor: color,
            roughness_factor: roughness,
            ..Default::default()
        }
    }

    /// Create white sheen (fabric-like).
    pub fn fabric(roughness: f32) -> Self {
        Self::new([1.0, 1.0, 1.0], roughness)
    }

    /// Convert to GPU data.
    pub fn to_gpu_data(&self) -> SheenGpuData {
        SheenGpuData {
            color: [
                self.color_factor[0],
                self.color_factor[1],
                self.color_factor[2],
                self.roughness_factor,
            ],
        }
    }
}

/// Sheen GPU data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SheenGpuData {
    /// RGB = color, A = roughness.
    pub color: [f32; 4],
}

// ============================================================================
// Transmission Extension
// ============================================================================

/// Transmission extension (KHR_materials_transmission).
#[derive(Debug, Clone, Default)]
pub struct Transmission {
    /// Transmission factor.
    pub factor: f32,
    /// Transmission texture.
    pub texture: Option<TextureInfo>,
}

impl Transmission {
    /// Create a new transmission.
    pub fn new(factor: f32) -> Self {
        Self {
            factor,
            texture: None,
        }
    }

    /// Create full transmission (glass-like).
    pub fn glass() -> Self {
        Self::new(1.0)
    }
}

// ============================================================================
// Volume Extension
// ============================================================================

/// Volume extension (KHR_materials_volume).
#[derive(Debug, Clone)]
pub struct Volume {
    /// Thickness factor.
    pub thickness_factor: f32,
    /// Thickness texture.
    pub thickness_texture: Option<TextureInfo>,
    /// Attenuation distance.
    pub attenuation_distance: f32,
    /// Attenuation color.
    pub attenuation_color: [f32; 3],
}

impl Default for Volume {
    fn default() -> Self {
        Self {
            thickness_factor: 0.0,
            thickness_texture: None,
            attenuation_distance: f32::INFINITY,
            attenuation_color: [1.0, 1.0, 1.0],
        }
    }
}

impl Volume {
    /// Create a new volume.
    pub fn new(thickness: f32) -> Self {
        Self {
            thickness_factor: thickness,
            ..Default::default()
        }
    }

    /// Set attenuation.
    pub fn attenuation(mut self, distance: f32, color: [f32; 3]) -> Self {
        self.attenuation_distance = distance;
        self.attenuation_color = color;
        self
    }

    /// Convert to GPU data.
    pub fn to_gpu_data(&self) -> VolumeGpuData {
        VolumeGpuData {
            attenuation: [
                self.attenuation_color[0],
                self.attenuation_color[1],
                self.attenuation_color[2],
                self.attenuation_distance,
            ],
            thickness: self.thickness_factor,
            _padding: [0.0; 3],
        }
    }
}

/// Volume GPU data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VolumeGpuData {
    /// RGB = attenuation color, A = attenuation distance.
    pub attenuation: [f32; 4],
    /// Thickness factor.
    pub thickness: f32,
    pub _padding: [f32; 3],
}

// ============================================================================
// Iridescence Extension
// ============================================================================

/// Iridescence extension (KHR_materials_iridescence).
#[derive(Debug, Clone)]
pub struct Iridescence {
    /// Iridescence factor.
    pub factor: f32,
    /// Iridescence texture.
    pub texture: Option<TextureInfo>,
    /// IOR of the thin-film.
    pub ior: f32,
    /// Minimum thickness.
    pub thickness_min: f32,
    /// Maximum thickness.
    pub thickness_max: f32,
    /// Thickness texture.
    pub thickness_texture: Option<TextureInfo>,
}

impl Default for Iridescence {
    fn default() -> Self {
        Self {
            factor: 0.0,
            texture: None,
            ior: 1.3,
            thickness_min: 100.0,
            thickness_max: 400.0,
            thickness_texture: None,
        }
    }
}

impl Iridescence {
    /// Create a new iridescence.
    pub fn new(factor: f32) -> Self {
        Self {
            factor,
            ..Default::default()
        }
    }

    /// Create soap bubble iridescence.
    pub fn soap_bubble() -> Self {
        Self {
            factor: 1.0,
            ior: 1.33,
            thickness_min: 100.0,
            thickness_max: 500.0,
            ..Default::default()
        }
    }

    /// Create oil slick iridescence.
    pub fn oil_slick() -> Self {
        Self {
            factor: 1.0,
            ior: 1.5,
            thickness_min: 200.0,
            thickness_max: 800.0,
            ..Default::default()
        }
    }

    /// Convert to GPU data.
    pub fn to_gpu_data(&self) -> IridescenceGpuData {
        IridescenceGpuData {
            params: [self.factor, self.ior, self.thickness_min, self.thickness_max],
        }
    }
}

/// Iridescence GPU data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct IridescenceGpuData {
    /// x = factor, y = ior, z = thickness_min, w = thickness_max.
    pub params: [f32; 4],
}

// ============================================================================
// Subsurface Scattering
// ============================================================================

/// Subsurface scattering.
#[derive(Debug, Clone)]
pub struct Subsurface {
    /// Subsurface color.
    pub color: [f32; 3],
    /// Scattering radius per color channel.
    pub radius: [f32; 3],
    /// Subsurface factor.
    pub factor: f32,
    /// Thickness texture.
    pub thickness_texture: Option<TextureInfo>,
}

impl Default for Subsurface {
    fn default() -> Self {
        Self {
            color: [1.0, 0.2, 0.1],
            radius: [1.0, 0.2, 0.1],
            factor: 0.0,
            thickness_texture: None,
        }
    }
}

impl Subsurface {
    /// Create skin-like subsurface.
    pub fn skin() -> Self {
        Self {
            color: [0.9, 0.4, 0.3],
            radius: [1.0, 0.4, 0.25],
            factor: 1.0,
            ..Default::default()
        }
    }

    /// Create marble-like subsurface.
    pub fn marble() -> Self {
        Self {
            color: [0.9, 0.9, 0.95],
            radius: [0.8, 0.8, 0.8],
            factor: 0.5,
            ..Default::default()
        }
    }

    /// Create jade-like subsurface.
    pub fn jade() -> Self {
        Self {
            color: [0.4, 0.8, 0.4],
            radius: [0.5, 0.6, 0.3],
            factor: 0.8,
            ..Default::default()
        }
    }

    /// Convert to GPU data.
    pub fn to_gpu_data(&self) -> SubsurfaceGpuData {
        SubsurfaceGpuData {
            color: [self.color[0], self.color[1], self.color[2], self.factor],
            radius: [self.radius[0], self.radius[1], self.radius[2], 0.0],
        }
    }
}

/// Subsurface GPU data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SubsurfaceGpuData {
    /// RGB = color, A = factor.
    pub color: [f32; 4],
    /// RGB = radius.
    pub radius: [f32; 4],
}

// ============================================================================
// Texture Info
// ============================================================================

/// Texture information.
#[derive(Debug, Clone)]
pub struct TextureInfo {
    /// Texture handle.
    pub index: u32,
    /// Texture coordinate set.
    pub tex_coord: u32,
    /// Transform.
    pub transform: Option<TextureTransform>,
}

impl TextureInfo {
    /// Create a new texture info.
    pub fn new(index: u32) -> Self {
        Self {
            index,
            tex_coord: 0,
            transform: None,
        }
    }

    /// Set texture coordinate set.
    pub fn tex_coord(mut self, coord: u32) -> Self {
        self.tex_coord = coord;
        self
    }

    /// Set transform.
    pub fn transform(mut self, transform: TextureTransform) -> Self {
        self.transform = Some(transform);
        self
    }
}

/// Texture transform (KHR_texture_transform).
#[derive(Debug, Clone, Copy)]
pub struct TextureTransform {
    /// Offset.
    pub offset: [f32; 2],
    /// Rotation in radians.
    pub rotation: f32,
    /// Scale.
    pub scale: [f32; 2],
}

impl Default for TextureTransform {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            rotation: 0.0,
            scale: [1.0, 1.0],
        }
    }
}

// ============================================================================
// PBR Material
// ============================================================================

/// Complete PBR material.
#[derive(Debug, Clone, Default)]
pub struct PbrMaterial {
    /// Material name.
    pub name: String,
    /// Base PBR parameters.
    pub params: PbrParameters,
    /// Metallic-roughness workflow.
    pub metallic_roughness: MetallicRoughness,
    /// Normal texture.
    pub normal_texture: Option<TextureInfo>,
    /// Occlusion texture.
    pub occlusion_texture: Option<TextureInfo>,
    /// Emissive texture.
    pub emissive_texture: Option<TextureInfo>,
    /// Clear coat extension.
    pub clear_coat: Option<ClearCoat>,
    /// Sheen extension.
    pub sheen: Option<Sheen>,
    /// Transmission extension.
    pub transmission: Option<Transmission>,
    /// Volume extension.
    pub volume: Option<Volume>,
    /// Iridescence extension.
    pub iridescence: Option<Iridescence>,
    /// Subsurface scattering.
    pub subsurface: Option<Subsurface>,
    /// Alpha mode.
    pub alpha_mode: AlphaMode,
    /// Alpha cutoff.
    pub alpha_cutoff: f32,
    /// Double-sided.
    pub double_sided: bool,
    /// Unlit.
    pub unlit: bool,
}

/// Alpha mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlphaMode {
    #[default]
    Opaque,
    Mask,
    Blend,
}

impl PbrMaterial {
    /// Create a new PBR material.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            alpha_cutoff: 0.5,
            ..Default::default()
        }
    }

    /// Set base color.
    pub fn base_color(mut self, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.params.base_color = [r, g, b, a];
        self.metallic_roughness.base_color_factor = [r, g, b, a];
        self
    }

    /// Set metallic factor.
    pub fn metallic(mut self, metallic: f32) -> Self {
        self.params.metallic = metallic;
        self.metallic_roughness.metallic_factor = metallic;
        self
    }

    /// Set roughness factor.
    pub fn roughness(mut self, roughness: f32) -> Self {
        self.params.roughness = roughness;
        self.metallic_roughness.roughness_factor = roughness;
        self
    }

    /// Set emissive.
    pub fn emissive(mut self, r: f32, g: f32, b: f32) -> Self {
        self.params.emissive = [r, g, b];
        self
    }

    /// Enable clear coat.
    pub fn with_clear_coat(mut self, factor: f32, roughness: f32) -> Self {
        self.clear_coat = Some(ClearCoat::new(factor, roughness));
        self
    }

    /// Enable sheen.
    pub fn with_sheen(mut self, color: [f32; 3], roughness: f32) -> Self {
        self.sheen = Some(Sheen::new(color, roughness));
        self
    }

    /// Enable transmission.
    pub fn with_transmission(mut self, factor: f32) -> Self {
        self.transmission = Some(Transmission::new(factor));
        self
    }

    /// Enable subsurface.
    pub fn with_subsurface(mut self, subsurface: Subsurface) -> Self {
        self.subsurface = Some(subsurface);
        self
    }

    /// Set double-sided.
    pub fn double_sided(mut self, enabled: bool) -> Self {
        self.double_sided = enabled;
        self
    }

    /// Set unlit.
    pub fn unlit(mut self, enabled: bool) -> Self {
        self.unlit = enabled;
        self
    }

    /// Check if has extensions.
    pub fn has_extensions(&self) -> bool {
        self.clear_coat.is_some()
            || self.sheen.is_some()
            || self.transmission.is_some()
            || self.volume.is_some()
            || self.iridescence.is_some()
            || self.subsurface.is_some()
    }

    /// Get feature flags for shader variant selection.
    pub fn feature_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.metallic_roughness.base_color_texture.is_some() {
            flags |= 1 << 0;
        }
        if self.normal_texture.is_some() {
            flags |= 1 << 1;
        }
        if self.metallic_roughness.metallic_roughness_texture.is_some() {
            flags |= 1 << 2;
        }
        if self.occlusion_texture.is_some() {
            flags |= 1 << 3;
        }
        if self.emissive_texture.is_some() {
            flags |= 1 << 4;
        }
        if self.clear_coat.is_some() {
            flags |= 1 << 5;
        }
        if self.sheen.is_some() {
            flags |= 1 << 6;
        }
        if self.transmission.is_some() {
            flags |= 1 << 7;
        }
        if self.volume.is_some() {
            flags |= 1 << 8;
        }
        if self.iridescence.is_some() {
            flags |= 1 << 9;
        }
        if self.subsurface.is_some() {
            flags |= 1 << 10;
        }
        if self.double_sided {
            flags |= 1 << 11;
        }
        if self.unlit {
            flags |= 1 << 12;
        }
        flags
    }

    /// Convert to GPU material data.
    pub fn to_gpu_material(&self) -> PbrGpuMaterial {
        PbrGpuMaterial {
            base: self.params.to_gpu_data(),
            clear_coat: self.clear_coat.as_ref().map(|c| c.to_gpu_data()).unwrap_or_default(),
            sheen: self.sheen.as_ref().map(|s| s.to_gpu_data()).unwrap_or_default(),
            subsurface: self.subsurface.as_ref().map(|s| s.to_gpu_data()).unwrap_or_default(),
            volume: self.volume.as_ref().map(|v| v.to_gpu_data()).unwrap_or_default(),
            iridescence: self.iridescence.as_ref().map(|i| i.to_gpu_data()).unwrap_or_default(),
            transmission: self.transmission.as_ref().map(|t| t.factor).unwrap_or(0.0),
            feature_flags: self.feature_flags(),
            _padding: [0.0; 2],
        }
    }
}

/// Complete PBR GPU material data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PbrGpuMaterial {
    /// Base PBR data.
    pub base: PbrGpuData,
    /// Clear coat data.
    pub clear_coat: ClearCoatGpuData,
    /// Sheen data.
    pub sheen: SheenGpuData,
    /// Subsurface data.
    pub subsurface: SubsurfaceGpuData,
    /// Volume data.
    pub volume: VolumeGpuData,
    /// Iridescence data.
    pub iridescence: IridescenceGpuData,
    /// Transmission factor.
    pub transmission: f32,
    /// Feature flags.
    pub feature_flags: u32,
    pub _padding: [f32; 2],
}

impl PbrGpuMaterial {
    /// Size in bytes.
    pub const SIZE: usize = 192;
}

// ============================================================================
// Material Presets
// ============================================================================

/// Material presets.
pub struct MaterialPresets;

impl MaterialPresets {
    /// Default white material.
    pub fn default_white() -> PbrMaterial {
        PbrMaterial::new("default_white")
            .base_color(1.0, 1.0, 1.0, 1.0)
            .metallic(0.0)
            .roughness(0.5)
    }

    /// Gold material.
    pub fn gold() -> PbrMaterial {
        PbrMaterial::new("gold")
            .base_color(1.0, 0.766, 0.336, 1.0)
            .metallic(1.0)
            .roughness(0.3)
    }

    /// Silver material.
    pub fn silver() -> PbrMaterial {
        PbrMaterial::new("silver")
            .base_color(0.972, 0.960, 0.915, 1.0)
            .metallic(1.0)
            .roughness(0.3)
    }

    /// Copper material.
    pub fn copper() -> PbrMaterial {
        PbrMaterial::new("copper")
            .base_color(0.955, 0.637, 0.538, 1.0)
            .metallic(1.0)
            .roughness(0.35)
    }

    /// Iron material.
    pub fn iron() -> PbrMaterial {
        PbrMaterial::new("iron")
            .base_color(0.56, 0.57, 0.58, 1.0)
            .metallic(1.0)
            .roughness(0.5)
    }

    /// Plastic material.
    pub fn plastic(color: [f32; 3]) -> PbrMaterial {
        PbrMaterial::new("plastic")
            .base_color(color[0], color[1], color[2], 1.0)
            .metallic(0.0)
            .roughness(0.4)
    }

    /// Rubber material.
    pub fn rubber(color: [f32; 3]) -> PbrMaterial {
        PbrMaterial::new("rubber")
            .base_color(color[0], color[1], color[2], 1.0)
            .metallic(0.0)
            .roughness(0.9)
    }

    /// Glass material.
    pub fn glass() -> PbrMaterial {
        PbrMaterial::new("glass")
            .base_color(1.0, 1.0, 1.0, 1.0)
            .metallic(0.0)
            .roughness(0.0)
            .with_transmission(1.0)
    }

    /// Fabric material.
    pub fn fabric(color: [f32; 3]) -> PbrMaterial {
        PbrMaterial::new("fabric")
            .base_color(color[0], color[1], color[2], 1.0)
            .metallic(0.0)
            .roughness(0.8)
            .with_sheen([1.0, 1.0, 1.0], 0.5)
    }

    /// Car paint material.
    pub fn car_paint(color: [f32; 3]) -> PbrMaterial {
        PbrMaterial::new("car_paint")
            .base_color(color[0], color[1], color[2], 1.0)
            .metallic(0.9)
            .roughness(0.1)
            .with_clear_coat(1.0, 0.03)
    }

    /// Skin material.
    pub fn skin() -> PbrMaterial {
        PbrMaterial::new("skin")
            .base_color(0.9, 0.7, 0.6, 1.0)
            .metallic(0.0)
            .roughness(0.5)
            .with_subsurface(Subsurface::skin())
    }
}
