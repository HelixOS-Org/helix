//! Material Instancing
//!
//! This module provides GPU-driven material instancing for efficient
//! rendering of many objects with material variations.

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use core::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Instance ID
// ============================================================================

/// Material instance identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceId {
    /// Index.
    index: u32,
    /// Generation.
    generation: u32,
}

impl InstanceId {
    /// Invalid instance.
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
    };

    /// Create a new instance ID.
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

// ============================================================================
// Instance Data
// ============================================================================

/// Per-instance material data.
#[derive(Debug, Clone)]
pub struct InstanceData {
    /// Base material.
    pub material: u32,
    /// Parameter overrides.
    pub overrides: BTreeMap<String, ParameterOverride>,
    /// Texture overrides.
    pub texture_overrides: BTreeMap<String, u32>,
    /// Custom data (up to 16 floats).
    pub custom_data: [f32; 16],
    /// Flags.
    pub flags: InstanceFlags,
}

/// Parameter override.
#[derive(Debug, Clone)]
pub enum ParameterOverride {
    /// Float value.
    Float(f32),
    /// Vec2 value.
    Vec2([f32; 2]),
    /// Vec3 value.
    Vec3([f32; 3]),
    /// Vec4 value.
    Vec4([f32; 4]),
    /// Color value.
    Color([f32; 4]),
}

bitflags::bitflags! {
    /// Instance flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct InstanceFlags: u32 {
        /// Instance is visible.
        const VISIBLE = 1 << 0;
        /// Instance casts shadows.
        const CAST_SHADOW = 1 << 1;
        /// Instance receives shadows.
        const RECEIVE_SHADOW = 1 << 2;
        /// Instance is selected (for editor).
        const SELECTED = 1 << 3;
        /// Instance uses custom data.
        const HAS_CUSTOM_DATA = 1 << 4;
        /// Default flags.
        const DEFAULT = Self::VISIBLE.bits() | Self::CAST_SHADOW.bits() | Self::RECEIVE_SHADOW.bits();
    }
}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            material: u32::MAX,
            overrides: BTreeMap::new(),
            texture_overrides: BTreeMap::new(),
            custom_data: [0.0; 16],
            flags: InstanceFlags::DEFAULT,
        }
    }
}

impl InstanceData {
    /// Create new instance data.
    pub fn new(material: u32) -> Self {
        Self {
            material,
            ..Default::default()
        }
    }

    /// Set float override.
    pub fn override_float(&mut self, name: impl Into<String>, value: f32) {
        self.overrides.insert(name.into(), ParameterOverride::Float(value));
    }

    /// Set color override.
    pub fn override_color(&mut self, name: impl Into<String>, color: [f32; 4]) {
        self.overrides.insert(name.into(), ParameterOverride::Color(color));
    }

    /// Set texture override.
    pub fn override_texture(&mut self, name: impl Into<String>, texture: u32) {
        self.texture_overrides.insert(name.into(), texture);
    }

    /// Set custom data.
    pub fn set_custom(&mut self, index: usize, value: f32) {
        if index < 16 {
            self.custom_data[index] = value;
            self.flags |= InstanceFlags::HAS_CUSTOM_DATA;
        }
    }

    /// Check if has overrides.
    pub fn has_overrides(&self) -> bool {
        !self.overrides.is_empty() || !self.texture_overrides.is_empty()
    }
}

// ============================================================================
// GPU Instance Data
// ============================================================================

/// GPU-ready instance data layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuInstanceData {
    /// Material index.
    pub material_index: u32,
    /// Flags.
    pub flags: u32,
    /// Override mask.
    pub override_mask: u32,
    /// Custom data offset.
    pub custom_data_offset: u32,
    /// Color override.
    pub color_override: [f32; 4],
    /// Scalar overrides (metallic, roughness, etc.).
    pub scalar_overrides: [f32; 4],
    /// Texture overrides (albedo, normal, metallic-roughness, occlusion).
    pub texture_overrides: [u32; 4],
}

impl GpuInstanceData {
    /// Size in bytes.
    pub const SIZE: usize = 64;

    /// Override mask bits.
    pub const OVERRIDE_COLOR: u32 = 1 << 0;
    pub const OVERRIDE_METALLIC: u32 = 1 << 1;
    pub const OVERRIDE_ROUGHNESS: u32 = 1 << 2;
    pub const OVERRIDE_ALBEDO_TEX: u32 = 1 << 8;
    pub const OVERRIDE_NORMAL_TEX: u32 = 1 << 9;
    pub const OVERRIDE_MR_TEX: u32 = 1 << 10;
    pub const OVERRIDE_AO_TEX: u32 = 1 << 11;
}

// ============================================================================
// Material Instance
// ============================================================================

/// Material instance.
pub struct MaterialInstance {
    /// Instance ID.
    id: InstanceId,
    /// Instance data.
    data: InstanceData,
    /// GPU data.
    gpu_data: GpuInstanceData,
    /// Dirty flag.
    dirty: bool,
}

impl MaterialInstance {
    /// Create a new instance.
    pub fn new(id: InstanceId, material: u32) -> Self {
        let data = InstanceData::new(material);
        let mut gpu_data = GpuInstanceData::default();
        gpu_data.material_index = material;
        gpu_data.flags = InstanceFlags::DEFAULT.bits();

        Self {
            id,
            data,
            gpu_data,
            dirty: true,
        }
    }

    /// Get ID.
    pub fn id(&self) -> InstanceId {
        self.id
    }

    /// Get material.
    pub fn material(&self) -> u32 {
        self.data.material
    }

    /// Set material.
    pub fn set_material(&mut self, material: u32) {
        self.data.material = material;
        self.gpu_data.material_index = material;
        self.dirty = true;
    }

    /// Get data.
    pub fn data(&self) -> &InstanceData {
        &self.data
    }

    /// Get mutable data.
    pub fn data_mut(&mut self) -> &mut InstanceData {
        self.dirty = true;
        &mut self.data
    }

    /// Override float parameter.
    pub fn override_float(&mut self, name: &str, value: f32) {
        self.data.override_float(name, value);
        self.dirty = true;
        
        // Update GPU data
        match name {
            "metallic" => {
                self.gpu_data.scalar_overrides[0] = value;
                self.gpu_data.override_mask |= GpuInstanceData::OVERRIDE_METALLIC;
            }
            "roughness" => {
                self.gpu_data.scalar_overrides[1] = value;
                self.gpu_data.override_mask |= GpuInstanceData::OVERRIDE_ROUGHNESS;
            }
            _ => {}
        }
    }

    /// Override color.
    pub fn override_color(&mut self, color: [f32; 4]) {
        self.data.override_color("base_color", color);
        self.gpu_data.color_override = color;
        self.gpu_data.override_mask |= GpuInstanceData::OVERRIDE_COLOR;
        self.dirty = true;
    }

    /// Override texture.
    pub fn override_texture(&mut self, name: &str, texture: u32) {
        self.data.override_texture(name, texture);
        self.dirty = true;

        match name {
            "albedo" | "base_color" => {
                self.gpu_data.texture_overrides[0] = texture;
                self.gpu_data.override_mask |= GpuInstanceData::OVERRIDE_ALBEDO_TEX;
            }
            "normal" => {
                self.gpu_data.texture_overrides[1] = texture;
                self.gpu_data.override_mask |= GpuInstanceData::OVERRIDE_NORMAL_TEX;
            }
            "metallic_roughness" => {
                self.gpu_data.texture_overrides[2] = texture;
                self.gpu_data.override_mask |= GpuInstanceData::OVERRIDE_MR_TEX;
            }
            "occlusion" | "ao" => {
                self.gpu_data.texture_overrides[3] = texture;
                self.gpu_data.override_mask |= GpuInstanceData::OVERRIDE_AO_TEX;
            }
            _ => {}
        }
    }

    /// Set visibility.
    pub fn set_visible(&mut self, visible: bool) {
        if visible {
            self.data.flags |= InstanceFlags::VISIBLE;
        } else {
            self.data.flags.remove(InstanceFlags::VISIBLE);
        }
        self.gpu_data.flags = self.data.flags.bits();
        self.dirty = true;
    }

    /// Check if visible.
    pub fn is_visible(&self) -> bool {
        self.data.flags.contains(InstanceFlags::VISIBLE)
    }

    /// Get GPU data.
    pub fn gpu_data(&self) -> &GpuInstanceData {
        &self.gpu_data
    }

    /// Check if dirty.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear dirty flag.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
}

// ============================================================================
// Instance Pool
// ============================================================================

/// Slot in the pool.
struct InstanceSlot {
    instance: Option<MaterialInstance>,
    generation: u32,
}

/// Pool of material instances.
pub struct InstancePool {
    /// Slots.
    slots: Vec<InstanceSlot>,
    /// Free list.
    free_list: Vec<u32>,
    /// Next generation.
    next_generation: AtomicU32,
    /// Dirty instances.
    dirty: Vec<u32>,
    /// GPU buffer.
    gpu_buffer: Vec<GpuInstanceData>,
    /// Capacity.
    capacity: u32,
}

impl InstancePool {
    /// Create a new pool.
    pub fn new(capacity: u32) -> Self {
        let mut slots = Vec::with_capacity(capacity as usize);
        for _ in 0..capacity {
            slots.push(InstanceSlot {
                instance: None,
                generation: 0,
            });
        }

        let free_list: Vec<u32> = (0..capacity).rev().collect();
        let gpu_buffer = vec![GpuInstanceData::default(); capacity as usize];

        Self {
            slots,
            free_list,
            next_generation: AtomicU32::new(1),
            dirty: Vec::new(),
            gpu_buffer,
            capacity,
        }
    }

    /// Allocate an instance.
    pub fn allocate(&mut self, material: u32) -> Option<InstanceId> {
        let index = self.free_list.pop()?;
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);

        let id = InstanceId::new(index, generation);
        let instance = MaterialInstance::new(id, material);

        let slot = &mut self.slots[index as usize];
        slot.generation = generation;
        slot.instance = Some(instance);

        self.dirty.push(index);
        Some(id)
    }

    /// Free an instance.
    pub fn free(&mut self, id: InstanceId) -> bool {
        if let Some(slot) = self.slots.get_mut(id.index as usize) {
            if slot.generation == id.generation {
                slot.instance = None;
                self.free_list.push(id.index);
                return true;
            }
        }
        false
    }

    /// Get instance.
    pub fn get(&self, id: InstanceId) -> Option<&MaterialInstance> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        slot.instance.as_ref()
    }

    /// Get mutable instance.
    pub fn get_mut(&mut self, id: InstanceId) -> Option<&mut MaterialInstance> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        if slot.instance.is_some() {
            self.dirty.push(id.index);
        }
        slot.instance.as_mut()
    }

    /// Update GPU buffer.
    pub fn update_gpu_buffer(&mut self) {
        for &index in &self.dirty {
            if let Some(slot) = self.slots.get_mut(index as usize) {
                if let Some(instance) = &mut slot.instance {
                    self.gpu_buffer[index as usize] = *instance.gpu_data();
                    instance.clear_dirty();
                }
            }
        }
        self.dirty.clear();
    }

    /// Get GPU buffer.
    pub fn gpu_buffer(&self) -> &[GpuInstanceData] {
        &self.gpu_buffer
    }

    /// Get dirty count.
    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }

    /// Get allocated count.
    pub fn allocated_count(&self) -> u32 {
        self.capacity - self.free_list.len() as u32
    }

    /// Get capacity.
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Iterate over instances.
    pub fn iter(&self) -> impl Iterator<Item = &MaterialInstance> {
        self.slots
            .iter()
            .filter_map(|slot| slot.instance.as_ref())
    }

    /// Iterate over visible instances.
    pub fn iter_visible(&self) -> impl Iterator<Item = &MaterialInstance> {
        self.iter().filter(|i| i.is_visible())
    }
}

// ============================================================================
// Instance Batch
// ============================================================================

/// Batch of instances sharing the same material.
#[derive(Debug, Clone)]
pub struct InstanceBatch {
    /// Material handle.
    pub material: u32,
    /// Instance indices.
    pub instances: Vec<u32>,
    /// Instance count.
    pub count: u32,
}

impl InstanceBatch {
    /// Create a new batch.
    pub fn new(material: u32) -> Self {
        Self {
            material,
            instances: Vec::new(),
            count: 0,
        }
    }

    /// Add an instance.
    pub fn add(&mut self, index: u32) {
        self.instances.push(index);
        self.count += 1;
    }

    /// Clear the batch.
    pub fn clear(&mut self) {
        self.instances.clear();
        self.count = 0;
    }
}

/// Batch instances by material.
pub fn batch_instances(pool: &InstancePool) -> Vec<InstanceBatch> {
    let mut batches: BTreeMap<u32, InstanceBatch> = BTreeMap::new();

    for instance in pool.iter_visible() {
        let material = instance.material();
        batches
            .entry(material)
            .or_insert_with(|| InstanceBatch::new(material))
            .add(instance.id().index());
    }

    batches.into_values().collect()
}

// ============================================================================
// Instancing Stats
// ============================================================================

/// Instancing statistics.
#[derive(Debug, Clone, Default)]
pub struct InstancingStats {
    /// Total instances.
    pub total_instances: u32,
    /// Visible instances.
    pub visible_instances: u32,
    /// Unique materials.
    pub unique_materials: u32,
    /// Batches.
    pub batches: u32,
    /// Instances with overrides.
    pub override_instances: u32,
    /// Memory usage (bytes).
    pub memory_usage: u64,
}

/// Calculate instancing stats.
pub fn calculate_stats(pool: &InstancePool) -> InstancingStats {
    let mut stats = InstancingStats::default();
    let mut materials = alloc::collections::BTreeSet::new();

    for instance in pool.iter() {
        stats.total_instances += 1;
        if instance.is_visible() {
            stats.visible_instances += 1;
        }
        materials.insert(instance.material());
        if instance.data().has_overrides() {
            stats.override_instances += 1;
        }
    }

    stats.unique_materials = materials.len() as u32;
    stats.batches = stats.unique_materials;
    stats.memory_usage = (pool.capacity() as u64) * (GpuInstanceData::SIZE as u64);

    stats
}
