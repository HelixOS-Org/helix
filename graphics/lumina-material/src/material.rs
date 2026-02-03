//! Core Material System
//!
//! This module provides the core material abstraction and management.

use alloc::{string::String, vec::Vec, collections::BTreeMap, boxed::Box};
use core::hash::{Hash, Hasher};
use core::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Material Handle
// ============================================================================

/// Handle to a material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialHandle {
    /// Index.
    index: u32,
    /// Generation.
    generation: u32,
}

impl MaterialHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };

    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

// ============================================================================
// Blend Mode
// ============================================================================

/// Material blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    /// Opaque (no blending).
    #[default]
    Opaque,
    /// Alpha blend (standard transparency).
    AlphaBlend,
    /// Premultiplied alpha.
    Premultiplied,
    /// Additive blending.
    Additive,
    /// Multiply blending.
    Multiply,
    /// Custom blend mode.
    Custom,
}

/// Alpha handling mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlphaMode {
    /// Fully opaque.
    #[default]
    Opaque,
    /// Alpha cutout (binary transparency).
    Mask,
    /// Alpha blending.
    Blend,
    /// Hashed alpha (stochastic transparency).
    Hashed,
}

bitflags::bitflags! {
    /// Material flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MaterialFlags: u32 {
        /// Material is double-sided.
        const DOUBLE_SIDED = 1 << 0;
        /// Material casts shadows.
        const CAST_SHADOW = 1 << 1;
        /// Material receives shadows.
        const RECEIVE_SHADOW = 1 << 2;
        /// Material is visible.
        const VISIBLE = 1 << 3;
        /// Material uses vertex colors.
        const USE_VERTEX_COLOR = 1 << 4;
        /// Material uses instancing.
        const INSTANCED = 1 << 5;
        /// Material is unlit.
        const UNLIT = 1 << 6;
        /// Material uses subsurface scattering.
        const SUBSURFACE = 1 << 7;
        /// Material uses clear coat.
        const CLEAR_COAT = 1 << 8;
        /// Material uses sheen.
        const SHEEN = 1 << 9;
        /// Material uses transmission.
        const TRANSMISSION = 1 << 10;
        /// Material uses volume.
        const VOLUME = 1 << 11;
        /// Material uses iridescence.
        const IRIDESCENCE = 1 << 12;
        /// Material is decal.
        const DECAL = 1 << 13;
        /// Material uses displacement.
        const DISPLACEMENT = 1 << 14;
        /// Material uses parallax occlusion.
        const PARALLAX = 1 << 15;
        /// Default flags.
        const DEFAULT = Self::CAST_SHADOW.bits() | Self::RECEIVE_SHADOW.bits() | Self::VISIBLE.bits();
    }
}

impl Default for MaterialFlags {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ============================================================================
// Material Descriptor
// ============================================================================

/// Material descriptor.
#[derive(Debug, Clone)]
pub struct MaterialDesc {
    /// Material name.
    pub name: String,
    /// Shader name/path.
    pub shader: String,
    /// Blend mode.
    pub blend_mode: BlendMode,
    /// Alpha mode.
    pub alpha_mode: AlphaMode,
    /// Alpha cutoff.
    pub alpha_cutoff: f32,
    /// Flags.
    pub flags: MaterialFlags,
    /// Render queue priority.
    pub queue_priority: i32,
    /// Stencil reference.
    pub stencil_ref: u8,
    /// Custom tags.
    pub tags: Vec<String>,
}

impl Default for MaterialDesc {
    fn default() -> Self {
        Self {
            name: String::new(),
            shader: String::from("pbr"),
            blend_mode: BlendMode::Opaque,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            flags: MaterialFlags::DEFAULT,
            queue_priority: 0,
            stencil_ref: 0,
            tags: Vec::new(),
        }
    }
}

impl MaterialDesc {
    /// Create a new descriptor.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set shader.
    pub fn shader(mut self, shader: impl Into<String>) -> Self {
        self.shader = shader.into();
        self
    }

    /// Set blend mode.
    pub fn blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    /// Set alpha mode.
    pub fn alpha_mode(mut self, mode: AlphaMode) -> Self {
        self.alpha_mode = mode;
        self
    }

    /// Set alpha cutoff.
    pub fn alpha_cutoff(mut self, cutoff: f32) -> Self {
        self.alpha_cutoff = cutoff;
        self
    }

    /// Set double-sided.
    pub fn double_sided(mut self, enabled: bool) -> Self {
        if enabled {
            self.flags |= MaterialFlags::DOUBLE_SIDED;
        } else {
            self.flags.remove(MaterialFlags::DOUBLE_SIDED);
        }
        self
    }

    /// Set unlit.
    pub fn unlit(mut self, enabled: bool) -> Self {
        if enabled {
            self.flags |= MaterialFlags::UNLIT;
        } else {
            self.flags.remove(MaterialFlags::UNLIT);
        }
        self
    }

    /// Add tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

// ============================================================================
// Material
// ============================================================================

/// Material definition.
pub struct Material {
    /// Handle.
    handle: MaterialHandle,
    /// Descriptor.
    desc: MaterialDesc,
    /// Parameter values.
    parameters: BTreeMap<String, ParameterValue>,
    /// Texture bindings.
    textures: BTreeMap<String, TextureBinding>,
    /// Compiled shader variant.
    shader_variant: Option<ShaderVariant>,
    /// Dirty flag.
    dirty: bool,
    /// Hash for change detection.
    hash: u64,
}

/// Texture binding.
#[derive(Debug, Clone)]
pub struct TextureBinding {
    /// Texture handle.
    pub texture: u32,
    /// Sampler handle.
    pub sampler: u32,
    /// UV channel.
    pub uv_channel: u8,
    /// Transform.
    pub transform: TextureTransform,
}

impl Default for TextureBinding {
    fn default() -> Self {
        Self {
            texture: u32::MAX,
            sampler: 0,
            uv_channel: 0,
            transform: TextureTransform::default(),
        }
    }
}

/// Texture transform.
#[derive(Debug, Clone, Copy)]
pub struct TextureTransform {
    /// Offset.
    pub offset: [f32; 2],
    /// Scale.
    pub scale: [f32; 2],
    /// Rotation in radians.
    pub rotation: f32,
}

impl Default for TextureTransform {
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            scale: [1.0, 1.0],
            rotation: 0.0,
        }
    }
}

impl TextureTransform {
    /// Create identity transform.
    pub fn identity() -> Self {
        Self::default()
    }

    /// Check if identity.
    pub fn is_identity(&self) -> bool {
        self.offset == [0.0, 0.0] && self.scale == [1.0, 1.0] && self.rotation == 0.0
    }

    /// Create from offset.
    pub fn with_offset(x: f32, y: f32) -> Self {
        Self {
            offset: [x, y],
            ..Default::default()
        }
    }

    /// Create from scale.
    pub fn with_scale(x: f32, y: f32) -> Self {
        Self {
            scale: [x, y],
            ..Default::default()
        }
    }

    /// Create tiled transform.
    pub fn tiled(repeat_x: f32, repeat_y: f32) -> Self {
        Self::with_scale(repeat_x, repeat_y)
    }
}

/// Parameter value.
#[derive(Debug, Clone)]
pub enum ParameterValue {
    /// Boolean.
    Bool(bool),
    /// Integer.
    Int(i32),
    /// Unsigned integer.
    Uint(u32),
    /// Float.
    Float(f32),
    /// Float2.
    Float2([f32; 2]),
    /// Float3.
    Float3([f32; 3]),
    /// Float4.
    Float4([f32; 4]),
    /// Color (linear RGB).
    Color([f32; 4]),
}

impl ParameterValue {
    /// Get as float.
    pub fn as_float(&self) -> Option<f32> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// Get as color.
    pub fn as_color(&self) -> Option<[f32; 4]> {
        match self {
            Self::Color(v) | Self::Float4(v) => Some(*v),
            _ => None,
        }
    }
}

/// Shader variant.
#[derive(Debug, Clone)]
pub struct ShaderVariant {
    /// Variant key.
    pub key: u64,
    /// Feature flags.
    pub features: u32,
    /// Pipeline handle.
    pub pipeline: u32,
}

impl Material {
    /// Create a new material.
    pub fn new(handle: MaterialHandle, desc: MaterialDesc) -> Self {
        Self {
            handle,
            desc,
            parameters: BTreeMap::new(),
            textures: BTreeMap::new(),
            shader_variant: None,
            dirty: true,
            hash: 0,
        }
    }

    /// Get handle.
    pub fn handle(&self) -> MaterialHandle {
        self.handle
    }

    /// Get descriptor.
    pub fn desc(&self) -> &MaterialDesc {
        &self.desc
    }

    /// Get name.
    pub fn name(&self) -> &str {
        &self.desc.name
    }

    /// Get blend mode.
    pub fn blend_mode(&self) -> BlendMode {
        self.desc.blend_mode
    }

    /// Get alpha mode.
    pub fn alpha_mode(&self) -> AlphaMode {
        self.desc.alpha_mode
    }

    /// Check if transparent.
    pub fn is_transparent(&self) -> bool {
        !matches!(self.desc.blend_mode, BlendMode::Opaque)
            || matches!(self.desc.alpha_mode, AlphaMode::Blend | AlphaMode::Hashed)
    }

    /// Check if double-sided.
    pub fn is_double_sided(&self) -> bool {
        self.desc.flags.contains(MaterialFlags::DOUBLE_SIDED)
    }

    /// Set parameter.
    pub fn set_parameter(&mut self, name: impl Into<String>, value: ParameterValue) {
        self.parameters.insert(name.into(), value);
        self.dirty = true;
    }

    /// Get parameter.
    pub fn parameter(&self, name: &str) -> Option<&ParameterValue> {
        self.parameters.get(name)
    }

    /// Set texture.
    pub fn set_texture(&mut self, name: impl Into<String>, binding: TextureBinding) {
        self.textures.insert(name.into(), binding);
        self.dirty = true;
    }

    /// Get texture.
    pub fn texture(&self, name: &str) -> Option<&TextureBinding> {
        self.textures.get(name)
    }

    /// Check if dirty.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear dirty flag.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Get shader variant.
    pub fn shader_variant(&self) -> Option<&ShaderVariant> {
        self.shader_variant.as_ref()
    }

    /// Set shader variant.
    pub fn set_shader_variant(&mut self, variant: ShaderVariant) {
        self.shader_variant = Some(variant);
    }

    /// Compute hash.
    pub fn compute_hash(&mut self) {
        let mut hasher = FnvHasher::new();
        self.desc.name.hash(&mut hasher);
        self.desc.shader.hash(&mut hasher);
        self.hash = hasher.finish();
    }
}

// ============================================================================
// Material Manager
// ============================================================================

/// Slot in material storage.
struct MaterialSlot {
    material: Option<Material>,
    generation: u32,
}

/// Material manager.
pub struct MaterialManager {
    /// Materials.
    materials: Vec<MaterialSlot>,
    /// Free list.
    free_list: Vec<u32>,
    /// Name to handle mapping.
    name_map: BTreeMap<String, MaterialHandle>,
    /// Default material.
    default_material: MaterialHandle,
    /// Stats.
    stats: MaterialStats,
    /// Next generation.
    next_generation: AtomicU32,
}

/// Material statistics.
#[derive(Debug, Clone, Default)]
pub struct MaterialStats {
    /// Total materials.
    pub total: u32,
    /// Opaque materials.
    pub opaque: u32,
    /// Transparent materials.
    pub transparent: u32,
    /// Dirty materials.
    pub dirty: u32,
}

impl MaterialManager {
    /// Create a new manager.
    pub fn new(capacity: u32) -> Self {
        let mut manager = Self {
            materials: Vec::with_capacity(capacity as usize),
            free_list: Vec::new(),
            name_map: BTreeMap::new(),
            default_material: MaterialHandle::INVALID,
            stats: MaterialStats::default(),
            next_generation: AtomicU32::new(1),
        };

        // Create default material
        let desc = MaterialDesc::new("default")
            .shader("pbr");
        manager.default_material = manager.create(desc).unwrap_or(MaterialHandle::INVALID);

        manager
    }

    /// Create a material.
    pub fn create(&mut self, desc: MaterialDesc) -> Option<MaterialHandle> {
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);

        let index = if let Some(index) = self.free_list.pop() {
            let slot = &mut self.materials[index as usize];
            slot.generation = generation;
            index
        } else {
            let index = self.materials.len() as u32;
            self.materials.push(MaterialSlot {
                material: None,
                generation,
            });
            index
        };

        let handle = MaterialHandle::new(index, generation);
        let name = desc.name.clone();

        let material = Material::new(handle, desc);
        self.materials[index as usize].material = Some(material);

        if !name.is_empty() {
            self.name_map.insert(name, handle);
        }

        self.update_stats();
        Some(handle)
    }

    /// Get material.
    pub fn get(&self, handle: MaterialHandle) -> Option<&Material> {
        let slot = self.materials.get(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.material.as_ref()
    }

    /// Get mutable material.
    pub fn get_mut(&mut self, handle: MaterialHandle) -> Option<&mut Material> {
        let slot = self.materials.get_mut(handle.index as usize)?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.material.as_mut()
    }

    /// Get material by name.
    pub fn get_by_name(&self, name: &str) -> Option<&Material> {
        let handle = *self.name_map.get(name)?;
        self.get(handle)
    }

    /// Get material handle by name.
    pub fn handle_by_name(&self, name: &str) -> Option<MaterialHandle> {
        self.name_map.get(name).copied()
    }

    /// Destroy material.
    pub fn destroy(&mut self, handle: MaterialHandle) {
        if let Some(slot) = self.materials.get_mut(handle.index as usize) {
            if slot.generation == handle.generation {
                if let Some(material) = slot.material.take() {
                    self.name_map.remove(&material.desc.name);
                }
                self.free_list.push(handle.index);
                self.update_stats();
            }
        }
    }

    /// Get default material.
    pub fn default_material(&self) -> MaterialHandle {
        self.default_material
    }

    /// Iterate over all materials.
    pub fn iter(&self) -> impl Iterator<Item = &Material> {
        self.materials
            .iter()
            .filter_map(|slot| slot.material.as_ref())
    }

    /// Iterate over dirty materials.
    pub fn iter_dirty(&self) -> impl Iterator<Item = &Material> {
        self.iter().filter(|m| m.is_dirty())
    }

    /// Get stats.
    pub fn stats(&self) -> &MaterialStats {
        &self.stats
    }

    /// Update stats.
    fn update_stats(&mut self) {
        let mut stats = MaterialStats::default();
        for slot in &self.materials {
            if let Some(material) = &slot.material {
                stats.total += 1;
                if material.is_transparent() {
                    stats.transparent += 1;
                } else {
                    stats.opaque += 1;
                }
                if material.is_dirty() {
                    stats.dirty += 1;
                }
            }
        }
        self.stats = stats;
    }

    /// Clear dirty flags.
    pub fn clear_dirty(&mut self) {
        for slot in &mut self.materials {
            if let Some(material) = &mut slot.material {
                material.clear_dirty();
            }
        }
        self.stats.dirty = 0;
    }
}

// ============================================================================
// Material Builder
// ============================================================================

/// Material builder.
pub struct MaterialBuilder {
    desc: MaterialDesc,
    parameters: BTreeMap<String, ParameterValue>,
    textures: BTreeMap<String, TextureBinding>,
}

impl MaterialBuilder {
    /// Create a new builder.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            desc: MaterialDesc::new(name),
            parameters: BTreeMap::new(),
            textures: BTreeMap::new(),
        }
    }

    /// Set shader.
    pub fn shader(mut self, shader: impl Into<String>) -> Self {
        self.desc.shader = shader.into();
        self
    }

    /// Set blend mode.
    pub fn blend_mode(mut self, mode: BlendMode) -> Self {
        self.desc.blend_mode = mode;
        self
    }

    /// Set alpha mode.
    pub fn alpha_mode(mut self, mode: AlphaMode) -> Self {
        self.desc.alpha_mode = mode;
        self
    }

    /// Set alpha cutoff.
    pub fn alpha_cutoff(mut self, cutoff: f32) -> Self {
        self.desc.alpha_cutoff = cutoff;
        self
    }

    /// Set double-sided.
    pub fn double_sided(mut self, enabled: bool) -> Self {
        self.desc = self.desc.double_sided(enabled);
        self
    }

    /// Set unlit.
    pub fn unlit(mut self) -> Self {
        self.desc = self.desc.unlit(true);
        self
    }

    /// Add parameter.
    pub fn param(mut self, name: impl Into<String>, value: ParameterValue) -> Self {
        self.parameters.insert(name.into(), value);
        self
    }

    /// Add float parameter.
    pub fn param_float(self, name: impl Into<String>, value: f32) -> Self {
        self.param(name, ParameterValue::Float(value))
    }

    /// Add color parameter.
    pub fn param_color(self, name: impl Into<String>, r: f32, g: f32, b: f32, a: f32) -> Self {
        self.param(name, ParameterValue::Color([r, g, b, a]))
    }

    /// Add texture.
    pub fn texture(mut self, name: impl Into<String>, texture: u32) -> Self {
        self.textures.insert(
            name.into(),
            TextureBinding {
                texture,
                ..Default::default()
            },
        );
        self
    }

    /// Build into manager.
    pub fn build(self, manager: &mut MaterialManager) -> Option<MaterialHandle> {
        let handle = manager.create(self.desc)?;
        let material = manager.get_mut(handle)?;

        for (name, value) in self.parameters {
            material.set_parameter(name, value);
        }
        for (name, binding) in self.textures {
            material.set_texture(name, binding);
        }

        Some(handle)
    }
}

// ============================================================================
// FNV Hasher
// ============================================================================

/// FNV-1a hasher.
struct FnvHasher {
    state: u64,
}

impl FnvHasher {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self {
            state: Self::FNV_OFFSET,
        }
    }
}

impl Hasher for FnvHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= *byte as u64;
            self.state = self.state.wrapping_mul(Self::FNV_PRIME);
        }
    }
}
