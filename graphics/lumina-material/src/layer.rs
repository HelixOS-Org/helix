//! Layered Material System
//!
//! This module provides multi-layer material composition for complex surface
//! appearances like weathered metal, car paint, or organic materials.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Layer Types
// ============================================================================

/// Material layer.
#[derive(Debug, Clone)]
pub struct MaterialLayer {
    /// Layer name.
    pub name: String,
    /// Layer weight/opacity.
    pub weight: f32,
    /// Blend mode.
    pub blend: LayerBlend,
    /// Mask for this layer.
    pub mask: Option<LayerMask>,
    /// Layer properties.
    pub properties: LayerProperties,
    /// Whether layer is enabled.
    pub enabled: bool,
}

/// Layer blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LayerBlend {
    /// Normal blending (lerp).
    #[default]
    Normal,
    /// Additive blending.
    Add,
    /// Multiplicative blending.
    Multiply,
    /// Screen blending.
    Screen,
    /// Overlay blending.
    Overlay,
    /// Height-based blending.
    HeightBlend,
}

/// Layer mask.
#[derive(Debug, Clone)]
pub enum LayerMask {
    /// Texture mask.
    Texture(u32),
    /// Vertex color channel.
    VertexColor(u8),
    /// Height-based mask.
    Height {
        /// Height map texture.
        texture: u32,
        /// Threshold.
        threshold: f32,
        /// Blend range.
        blend: f32,
    },
    /// Noise-based mask.
    Noise {
        /// Noise frequency.
        frequency: f32,
        /// Noise threshold.
        threshold: f32,
    },
    /// Procedural mask.
    Procedural(Box<ProceduralMask>),
}

/// Procedural mask parameters.
#[derive(Debug, Clone)]
pub struct ProceduralMask {
    /// Mask type.
    pub mask_type: ProceduralMaskType,
    /// Parameters.
    pub params: [f32; 8],
}

/// Procedural mask type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProceduralMaskType {
    /// Gradient mask.
    Gradient,
    /// Edge wear.
    EdgeWear,
    /// Cavity mask.
    Cavity,
    /// Curvature mask.
    Curvature,
    /// Ambient occlusion.
    AO,
    /// World position-based.
    WorldPosition,
}

/// Layer properties.
#[derive(Debug, Clone)]
pub struct LayerProperties {
    /// Base color.
    pub base_color: [f32; 4],
    /// Metallic.
    pub metallic: f32,
    /// Roughness.
    pub roughness: f32,
    /// Normal strength.
    pub normal_strength: f32,
    /// Height scale.
    pub height_scale: f32,
    /// Emissive color.
    pub emissive: [f32; 3],
    /// Texture set.
    pub textures: LayerTextures,
}

impl Default for LayerProperties {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            normal_strength: 1.0,
            height_scale: 0.0,
            emissive: [0.0, 0.0, 0.0],
            textures: LayerTextures::default(),
        }
    }
}

/// Layer textures.
#[derive(Debug, Clone, Default)]
pub struct LayerTextures {
    /// Albedo texture.
    pub albedo: Option<u32>,
    /// Normal map.
    pub normal: Option<u32>,
    /// Metallic-roughness map.
    pub metallic_roughness: Option<u32>,
    /// Height map.
    pub height: Option<u32>,
    /// Occlusion map.
    pub occlusion: Option<u32>,
    /// Emissive map.
    pub emissive: Option<u32>,
}

impl MaterialLayer {
    /// Create a new layer.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            weight: 1.0,
            blend: LayerBlend::Normal,
            mask: None,
            properties: LayerProperties::default(),
            enabled: true,
        }
    }

    /// Set weight.
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Set blend mode.
    pub fn with_blend(mut self, blend: LayerBlend) -> Self {
        self.blend = blend;
        self
    }

    /// Set mask.
    pub fn with_mask(mut self, mask: LayerMask) -> Self {
        self.mask = Some(mask);
        self
    }

    /// Set base color.
    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.properties.base_color = color;
        self
    }

    /// Set metallic.
    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.properties.metallic = metallic;
        self
    }

    /// Set roughness.
    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.properties.roughness = roughness;
        self
    }
}

// ============================================================================
// Layered Material
// ============================================================================

/// Layered material with multiple blend layers.
#[derive(Debug, Clone)]
pub struct LayeredMaterial {
    /// Material name.
    pub name: String,
    /// Base layer (always present).
    pub base: MaterialLayer,
    /// Additional layers.
    pub layers: Vec<MaterialLayer>,
    /// Global properties.
    pub global: GlobalLayerProperties,
    /// Total layer count.
    layer_count: u32,
}

/// Global properties affecting all layers.
#[derive(Debug, Clone)]
pub struct GlobalLayerProperties {
    /// UV scale.
    pub uv_scale: [f32; 2],
    /// UV offset.
    pub uv_offset: [f32; 2],
    /// Triplanar mapping.
    pub triplanar: bool,
    /// Triplanar blend sharpness.
    pub triplanar_sharpness: f32,
    /// Height blend contrast.
    pub height_blend_contrast: f32,
}

impl Default for GlobalLayerProperties {
    fn default() -> Self {
        Self {
            uv_scale: [1.0, 1.0],
            uv_offset: [0.0, 0.0],
            triplanar: false,
            triplanar_sharpness: 1.0,
            height_blend_contrast: 1.0,
        }
    }
}

impl LayeredMaterial {
    /// Create a new layered material.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base: MaterialLayer::new("Base"),
            layers: Vec::new(),
            global: GlobalLayerProperties::default(),
            layer_count: 1,
        }
    }

    /// Add a layer.
    pub fn add_layer(&mut self, layer: MaterialLayer) -> usize {
        let index = self.layers.len();
        self.layers.push(layer);
        self.layer_count += 1;
        index
    }

    /// Remove a layer.
    pub fn remove_layer(&mut self, index: usize) -> Option<MaterialLayer> {
        if index < self.layers.len() {
            self.layer_count -= 1;
            Some(self.layers.remove(index))
        } else {
            None
        }
    }

    /// Get layer count.
    pub fn layer_count(&self) -> u32 {
        self.layer_count
    }

    /// Get a layer.
    pub fn get_layer(&self, index: usize) -> Option<&MaterialLayer> {
        self.layers.get(index)
    }

    /// Get mutable layer.
    pub fn get_layer_mut(&mut self, index: usize) -> Option<&mut MaterialLayer> {
        self.layers.get_mut(index)
    }

    /// Set base layer.
    pub fn set_base(&mut self, layer: MaterialLayer) {
        self.base = layer;
    }

    /// Get base layer.
    pub fn base(&self) -> &MaterialLayer {
        &self.base
    }

    /// Enable triplanar mapping.
    pub fn enable_triplanar(&mut self, sharpness: f32) {
        self.global.triplanar = true;
        self.global.triplanar_sharpness = sharpness;
    }

    /// Calculate blended properties at a point.
    pub fn blend_at(&self, masks: &[f32]) -> LayerProperties {
        let mut result = self.base.properties.clone();

        for (i, layer) in self.layers.iter().enumerate() {
            if !layer.enabled {
                continue;
            }

            let mask = masks.get(i).copied().unwrap_or(1.0);
            let weight = layer.weight * mask;

            if weight <= 0.0 {
                continue;
            }

            result = Self::blend_properties(&result, &layer.properties, weight, layer.blend);
        }

        result
    }

    /// Blend two property sets.
    fn blend_properties(
        a: &LayerProperties,
        b: &LayerProperties,
        weight: f32,
        blend: LayerBlend,
    ) -> LayerProperties {
        let blend_scalar = |a: f32, b: f32| match blend {
            LayerBlend::Normal => Self::lerp(a, b, weight),
            LayerBlend::Add => a + b * weight,
            LayerBlend::Multiply => a * Self::lerp(1.0, b, weight),
            LayerBlend::Screen => 1.0 - (1.0 - a) * (1.0 - b * weight),
            LayerBlend::Overlay => {
                if a < 0.5 {
                    Self::lerp(a, 2.0 * a * b, weight)
                } else {
                    Self::lerp(a, 1.0 - 2.0 * (1.0 - a) * (1.0 - b), weight)
                }
            },
            LayerBlend::HeightBlend => Self::lerp(a, b, weight),
        };

        LayerProperties {
            base_color: [
                blend_scalar(a.base_color[0], b.base_color[0]),
                blend_scalar(a.base_color[1], b.base_color[1]),
                blend_scalar(a.base_color[2], b.base_color[2]),
                blend_scalar(a.base_color[3], b.base_color[3]),
            ],
            metallic: blend_scalar(a.metallic, b.metallic),
            roughness: blend_scalar(a.roughness, b.roughness),
            normal_strength: blend_scalar(a.normal_strength, b.normal_strength),
            height_scale: blend_scalar(a.height_scale, b.height_scale),
            emissive: [
                blend_scalar(a.emissive[0], b.emissive[0]),
                blend_scalar(a.emissive[1], b.emissive[1]),
                blend_scalar(a.emissive[2], b.emissive[2]),
            ],
            textures: a.textures.clone(), // Texture blending is done in shader
        }
    }

    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }
}

// ============================================================================
// Layer Presets
// ============================================================================

/// Common layer presets.
pub struct LayerPresets;

impl LayerPresets {
    /// Create a rust/corrosion layer.
    pub fn rust() -> MaterialLayer {
        MaterialLayer::new("Rust")
            .with_color([0.5, 0.2, 0.1, 1.0])
            .with_metallic(0.0)
            .with_roughness(0.9)
            .with_blend(LayerBlend::HeightBlend)
    }

    /// Create a snow layer.
    pub fn snow() -> MaterialLayer {
        MaterialLayer::new("Snow")
            .with_color([0.95, 0.95, 0.98, 1.0])
            .with_metallic(0.0)
            .with_roughness(0.8)
            .with_blend(LayerBlend::Normal)
    }

    /// Create a dirt layer.
    pub fn dirt() -> MaterialLayer {
        MaterialLayer::new("Dirt")
            .with_color([0.3, 0.25, 0.2, 1.0])
            .with_metallic(0.0)
            .with_roughness(1.0)
            .with_blend(LayerBlend::HeightBlend)
    }

    /// Create a moss layer.
    pub fn moss() -> MaterialLayer {
        MaterialLayer::new("Moss")
            .with_color([0.2, 0.35, 0.15, 1.0])
            .with_metallic(0.0)
            .with_roughness(0.85)
            .with_blend(LayerBlend::Normal)
    }

    /// Create a water/wetness layer.
    pub fn wet() -> MaterialLayer {
        MaterialLayer::new("Wet")
            .with_color([0.02, 0.02, 0.02, 0.5])
            .with_metallic(0.0)
            .with_roughness(0.1)
            .with_blend(LayerBlend::Multiply)
    }

    /// Create an edge wear layer.
    pub fn edge_wear() -> MaterialLayer {
        MaterialLayer::new("Edge Wear")
            .with_color([0.8, 0.8, 0.8, 1.0])
            .with_metallic(1.0)
            .with_roughness(0.3)
            .with_blend(LayerBlend::Normal)
            .with_mask(LayerMask::Procedural(Box::new(ProceduralMask {
                mask_type: ProceduralMaskType::EdgeWear,
                params: [0.5, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            })))
    }
}

// ============================================================================
// GPU Layer Data
// ============================================================================

/// GPU-ready layer data.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuLayerData {
    /// Base color.
    pub base_color: [f32; 4],
    /// Metallic, roughness, normal strength, height scale.
    pub properties: [f32; 4],
    /// Emissive + padding.
    pub emissive: [f32; 4],
    /// Mask parameters.
    pub mask_params: [f32; 4],
    /// Texture indices.
    pub texture_indices: [u32; 4],
    /// Blend mode, weight, flags, padding.
    pub blend_info: [f32; 4],
}

impl GpuLayerData {
    /// Size in bytes.
    pub const SIZE: usize = 96;

    /// Create from layer.
    pub fn from_layer(layer: &MaterialLayer) -> Self {
        Self {
            base_color: layer.properties.base_color,
            properties: [
                layer.properties.metallic,
                layer.properties.roughness,
                layer.properties.normal_strength,
                layer.properties.height_scale,
            ],
            emissive: [
                layer.properties.emissive[0],
                layer.properties.emissive[1],
                layer.properties.emissive[2],
                0.0,
            ],
            mask_params: [0.0; 4],
            texture_indices: [
                layer.properties.textures.albedo.unwrap_or(u32::MAX),
                layer.properties.textures.normal.unwrap_or(u32::MAX),
                layer
                    .properties
                    .textures
                    .metallic_roughness
                    .unwrap_or(u32::MAX),
                layer.properties.textures.height.unwrap_or(u32::MAX),
            ],
            blend_info: [
                layer.blend as u32 as f32,
                layer.weight,
                if layer.enabled { 1.0 } else { 0.0 },
                0.0,
            ],
        }
    }
}

/// GPU-ready layered material.
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct GpuLayeredMaterial {
    /// Global properties.
    pub global: [f32; 8],
    /// Layer count.
    pub layer_count: u32,
    /// Padding.
    pub _padding: [u32; 3],
    /// Layers (max 8).
    pub layers: [GpuLayerData; 8],
}

impl GpuLayeredMaterial {
    /// Size in bytes.
    pub const SIZE: usize = 32 + 8 * GpuLayerData::SIZE;

    /// Create from layered material.
    pub fn from_material(material: &LayeredMaterial) -> Self {
        let mut result = Self {
            global: [
                material.global.uv_scale[0],
                material.global.uv_scale[1],
                material.global.uv_offset[0],
                material.global.uv_offset[1],
                if material.global.triplanar { 1.0 } else { 0.0 },
                material.global.triplanar_sharpness,
                material.global.height_blend_contrast,
                0.0,
            ],
            layer_count: material.layer_count,
            _padding: [0; 3],
            layers: [GpuLayerData::default(); 8],
        };

        // Base layer
        result.layers[0] = GpuLayerData::from_layer(&material.base);

        // Additional layers
        for (i, layer) in material.layers.iter().take(7).enumerate() {
            result.layers[i + 1] = GpuLayerData::from_layer(layer);
        }

        result
    }
}
