//! Bindless Descriptor System
//!
//! This module provides a revolutionary bindless descriptor system for
//! GPU-driven rendering and virtual texturing.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// Handle Types
// ============================================================================

/// Bindless resource handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindlessHandle {
    /// Index in the global descriptor array.
    index: u32,
    /// Generation for validation.
    generation: u16,
    /// Resource type.
    resource_type: BindlessResourceType,
}

impl BindlessHandle {
    /// Invalid handle.
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
        resource_type: BindlessResourceType::None,
    };

    /// Create a new handle.
    fn new(index: u32, generation: u16, resource_type: BindlessResourceType) -> Self {
        Self {
            index,
            generation,
            resource_type,
        }
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Get the generation.
    pub fn generation(&self) -> u16 {
        self.generation
    }

    /// Get the resource type.
    pub fn resource_type(&self) -> BindlessResourceType {
        self.resource_type
    }

    /// Check if valid.
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }

    /// Pack into a u32 for shader use.
    pub fn to_shader_index(&self) -> u32 {
        self.index
    }
}

/// Bindless resource type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindlessResourceType {
    /// No resource.
    None,
    /// Sampled texture.
    SampledTexture,
    /// Storage texture.
    StorageTexture,
    /// Uniform buffer.
    UniformBuffer,
    /// Storage buffer.
    StorageBuffer,
    /// Sampler.
    Sampler,
    /// Acceleration structure.
    AccelerationStructure,
}

// ============================================================================
// Bindless Heap
// ============================================================================

/// Configuration for bindless heap.
#[derive(Debug, Clone)]
pub struct BindlessHeapConfig {
    /// Maximum sampled textures.
    pub max_sampled_textures: u32,
    /// Maximum storage textures.
    pub max_storage_textures: u32,
    /// Maximum uniform buffers.
    pub max_uniform_buffers: u32,
    /// Maximum storage buffers.
    pub max_storage_buffers: u32,
    /// Maximum samplers.
    pub max_samplers: u32,
    /// Maximum acceleration structures.
    pub max_acceleration_structures: u32,
    /// Enable debug validation.
    pub debug_validation: bool,
}

impl Default for BindlessHeapConfig {
    fn default() -> Self {
        Self {
            max_sampled_textures: 1_000_000,
            max_storage_textures: 100_000,
            max_uniform_buffers: 100_000,
            max_storage_buffers: 500_000,
            max_samplers: 4096,
            max_acceleration_structures: 65536,
            debug_validation: cfg!(debug_assertions),
        }
    }
}

impl BindlessHeapConfig {
    /// Create a minimal configuration.
    pub fn minimal() -> Self {
        Self {
            max_sampled_textures: 10_000,
            max_storage_textures: 1_000,
            max_uniform_buffers: 1_000,
            max_storage_buffers: 10_000,
            max_samplers: 256,
            max_acceleration_structures: 1024,
            debug_validation: false,
        }
    }

    /// Create a large configuration for AAA games.
    pub fn large() -> Self {
        Self {
            max_sampled_textures: 2_000_000,
            max_storage_textures: 500_000,
            max_uniform_buffers: 500_000,
            max_storage_buffers: 1_000_000,
            max_samplers: 8192,
            max_acceleration_structures: 262144,
            debug_validation: false,
        }
    }

    /// Total descriptor count.
    pub fn total_descriptors(&self) -> u32 {
        self.max_sampled_textures
            + self.max_storage_textures
            + self.max_uniform_buffers
            + self.max_storage_buffers
            + self.max_samplers
            + self.max_acceleration_structures
    }
}

/// Slot state in the free list.
#[derive(Clone)]
struct Slot {
    /// Generation counter.
    generation: u16,
    /// Whether the slot is in use.
    in_use: bool,
    /// Debug name.
    #[cfg(debug_assertions)]
    debug_name: Option<String>,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            generation: 0,
            in_use: false,
            #[cfg(debug_assertions)]
            debug_name: None,
        }
    }
}

/// Free list allocator for a resource type.
struct ResourceAllocator {
    /// Slots.
    slots: Vec<Slot>,
    /// Free indices.
    free_list: Vec<u32>,
    /// Maximum capacity.
    capacity: u32,
    /// Base offset in the global heap.
    base_offset: u32,
    /// Number of allocated slots.
    allocated: AtomicU32,
}

impl ResourceAllocator {
    fn new(capacity: u32, base_offset: u32) -> Self {
        let mut slots = Vec::with_capacity(capacity as usize);
        slots.resize_with(capacity as usize, Slot::default);

        let free_list: Vec<u32> = (0..capacity).rev().collect();

        Self {
            slots,
            free_list,
            capacity,
            base_offset,
            allocated: AtomicU32::new(0),
        }
    }

    fn allocate(&mut self) -> Option<(u32, u16)> {
        let local_index = self.free_list.pop()?;
        let slot = &mut self.slots[local_index as usize];
        slot.in_use = true;
        self.allocated.fetch_add(1, Ordering::Relaxed);
        Some((local_index + self.base_offset, slot.generation))
    }

    fn free(&mut self, index: u32, generation: u16) -> bool {
        let local_index = index.checked_sub(self.base_offset)?;
        if local_index >= self.capacity {
            return false;
        }

        let slot = &mut self.slots[local_index as usize];
        if !slot.in_use || slot.generation != generation {
            return false;
        }

        slot.in_use = false;
        slot.generation = slot.generation.wrapping_add(1);
        #[cfg(debug_assertions)]
        {
            slot.debug_name = None;
        }
        self.free_list.push(local_index);
        self.allocated.fetch_sub(1, Ordering::Relaxed);
        true
    }

    fn is_valid(&self, index: u32, generation: u16) -> bool {
        let Some(local_index) = index.checked_sub(self.base_offset) else {
            return false;
        };
        if local_index >= self.capacity {
            return false;
        }

        let slot = &self.slots[local_index as usize];
        slot.in_use && slot.generation == generation
    }

    fn allocated_count(&self) -> u32 {
        self.allocated.load(Ordering::Relaxed)
    }

    #[cfg(debug_assertions)]
    fn set_debug_name(&mut self, index: u32, name: String) {
        if let Some(local_index) = index.checked_sub(self.base_offset) {
            if let Some(slot) = self.slots.get_mut(local_index as usize) {
                slot.debug_name = Some(name);
            }
        }
    }
}

/// Bindless descriptor heap.
pub struct BindlessHeap {
    /// Configuration.
    config: BindlessHeapConfig,
    /// Allocators for each resource type.
    sampled_textures: ResourceAllocator,
    storage_textures: ResourceAllocator,
    uniform_buffers: ResourceAllocator,
    storage_buffers: ResourceAllocator,
    samplers: ResourceAllocator,
    acceleration_structures: ResourceAllocator,
    /// Pending updates.
    pending_updates: Vec<BindlessUpdate>,
    /// Frame index for deferred deletion.
    frame_index: u64,
    /// Deferred deletions.
    deferred_deletions: Vec<DeferredDeletion>,
}

/// Bindless update for GPU synchronization.
#[derive(Debug, Clone)]
pub struct BindlessUpdate {
    /// Handle.
    pub handle: BindlessHandle,
    /// Update type.
    pub update_type: BindlessUpdateType,
}

/// Bindless update type.
#[derive(Debug, Clone)]
pub enum BindlessUpdateType {
    /// Bind a resource.
    Bind,
    /// Unbind a resource.
    Unbind,
}

/// Deferred deletion.
struct DeferredDeletion {
    handle: BindlessHandle,
    frame: u64,
}

impl BindlessHeap {
    /// Create a new bindless heap.
    pub fn new(config: BindlessHeapConfig) -> Self {
        let mut offset = 0u32;

        let sampled_textures = ResourceAllocator::new(config.max_sampled_textures, offset);
        offset += config.max_sampled_textures;

        let storage_textures = ResourceAllocator::new(config.max_storage_textures, offset);
        offset += config.max_storage_textures;

        let uniform_buffers = ResourceAllocator::new(config.max_uniform_buffers, offset);
        offset += config.max_uniform_buffers;

        let storage_buffers = ResourceAllocator::new(config.max_storage_buffers, offset);
        offset += config.max_storage_buffers;

        let samplers = ResourceAllocator::new(config.max_samplers, offset);
        offset += config.max_samplers;

        let acceleration_structures =
            ResourceAllocator::new(config.max_acceleration_structures, offset);

        Self {
            config,
            sampled_textures,
            storage_textures,
            uniform_buffers,
            storage_buffers,
            samplers,
            acceleration_structures,
            pending_updates: Vec::new(),
            frame_index: 0,
            deferred_deletions: Vec::new(),
        }
    }

    /// Create with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(BindlessHeapConfig::default())
    }

    /// Allocate a sampled texture handle.
    pub fn allocate_sampled_texture(&mut self) -> Option<BindlessHandle> {
        let (index, gen) = self.sampled_textures.allocate()?;
        let handle = BindlessHandle::new(index, gen, BindlessResourceType::SampledTexture);
        self.pending_updates.push(BindlessUpdate {
            handle,
            update_type: BindlessUpdateType::Bind,
        });
        Some(handle)
    }

    /// Allocate a storage texture handle.
    pub fn allocate_storage_texture(&mut self) -> Option<BindlessHandle> {
        let (index, gen) = self.storage_textures.allocate()?;
        let handle = BindlessHandle::new(index, gen, BindlessResourceType::StorageTexture);
        self.pending_updates.push(BindlessUpdate {
            handle,
            update_type: BindlessUpdateType::Bind,
        });
        Some(handle)
    }

    /// Allocate a uniform buffer handle.
    pub fn allocate_uniform_buffer(&mut self) -> Option<BindlessHandle> {
        let (index, gen) = self.uniform_buffers.allocate()?;
        let handle = BindlessHandle::new(index, gen, BindlessResourceType::UniformBuffer);
        self.pending_updates.push(BindlessUpdate {
            handle,
            update_type: BindlessUpdateType::Bind,
        });
        Some(handle)
    }

    /// Allocate a storage buffer handle.
    pub fn allocate_storage_buffer(&mut self) -> Option<BindlessHandle> {
        let (index, gen) = self.storage_buffers.allocate()?;
        let handle = BindlessHandle::new(index, gen, BindlessResourceType::StorageBuffer);
        self.pending_updates.push(BindlessUpdate {
            handle,
            update_type: BindlessUpdateType::Bind,
        });
        Some(handle)
    }

    /// Allocate a sampler handle.
    pub fn allocate_sampler(&mut self) -> Option<BindlessHandle> {
        let (index, gen) = self.samplers.allocate()?;
        let handle = BindlessHandle::new(index, gen, BindlessResourceType::Sampler);
        self.pending_updates.push(BindlessUpdate {
            handle,
            update_type: BindlessUpdateType::Bind,
        });
        Some(handle)
    }

    /// Allocate an acceleration structure handle.
    pub fn allocate_acceleration_structure(&mut self) -> Option<BindlessHandle> {
        let (index, gen) = self.acceleration_structures.allocate()?;
        let handle = BindlessHandle::new(index, gen, BindlessResourceType::AccelerationStructure);
        self.pending_updates.push(BindlessUpdate {
            handle,
            update_type: BindlessUpdateType::Bind,
        });
        Some(handle)
    }

    /// Free a handle immediately.
    pub fn free(&mut self, handle: BindlessHandle) -> bool {
        let freed = match handle.resource_type {
            BindlessResourceType::SampledTexture => {
                self.sampled_textures.free(handle.index, handle.generation)
            },
            BindlessResourceType::StorageTexture => {
                self.storage_textures.free(handle.index, handle.generation)
            },
            BindlessResourceType::UniformBuffer => {
                self.uniform_buffers.free(handle.index, handle.generation)
            },
            BindlessResourceType::StorageBuffer => {
                self.storage_buffers.free(handle.index, handle.generation)
            },
            BindlessResourceType::Sampler => self.samplers.free(handle.index, handle.generation),
            BindlessResourceType::AccelerationStructure => self
                .acceleration_structures
                .free(handle.index, handle.generation),
            BindlessResourceType::None => false,
        };

        if freed {
            self.pending_updates.push(BindlessUpdate {
                handle,
                update_type: BindlessUpdateType::Unbind,
            });
        }

        freed
    }

    /// Free a handle with deferred deletion.
    pub fn free_deferred(&mut self, handle: BindlessHandle, frames_to_wait: u64) {
        self.deferred_deletions.push(DeferredDeletion {
            handle,
            frame: self.frame_index + frames_to_wait,
        });
    }

    /// Advance to next frame and process deferred deletions.
    pub fn advance_frame(&mut self) {
        self.frame_index += 1;

        // Process deferred deletions
        let current_frame = self.frame_index;
        let mut i = 0;
        while i < self.deferred_deletions.len() {
            if self.deferred_deletions[i].frame <= current_frame {
                let deletion = self.deferred_deletions.swap_remove(i);
                self.free(deletion.handle);
            } else {
                i += 1;
            }
        }
    }

    /// Check if a handle is valid.
    pub fn is_valid(&self, handle: BindlessHandle) -> bool {
        match handle.resource_type {
            BindlessResourceType::SampledTexture => self
                .sampled_textures
                .is_valid(handle.index, handle.generation),
            BindlessResourceType::StorageTexture => self
                .storage_textures
                .is_valid(handle.index, handle.generation),
            BindlessResourceType::UniformBuffer => self
                .uniform_buffers
                .is_valid(handle.index, handle.generation),
            BindlessResourceType::StorageBuffer => self
                .storage_buffers
                .is_valid(handle.index, handle.generation),
            BindlessResourceType::Sampler => {
                self.samplers.is_valid(handle.index, handle.generation)
            },
            BindlessResourceType::AccelerationStructure => self
                .acceleration_structures
                .is_valid(handle.index, handle.generation),
            BindlessResourceType::None => false,
        }
    }

    /// Take pending updates.
    pub fn take_pending_updates(&mut self) -> Vec<BindlessUpdate> {
        core::mem::take(&mut self.pending_updates)
    }

    /// Get statistics.
    pub fn stats(&self) -> BindlessHeapStats {
        BindlessHeapStats {
            sampled_textures_allocated: self.sampled_textures.allocated_count(),
            sampled_textures_capacity: self.config.max_sampled_textures,
            storage_textures_allocated: self.storage_textures.allocated_count(),
            storage_textures_capacity: self.config.max_storage_textures,
            uniform_buffers_allocated: self.uniform_buffers.allocated_count(),
            uniform_buffers_capacity: self.config.max_uniform_buffers,
            storage_buffers_allocated: self.storage_buffers.allocated_count(),
            storage_buffers_capacity: self.config.max_storage_buffers,
            samplers_allocated: self.samplers.allocated_count(),
            samplers_capacity: self.config.max_samplers,
            acceleration_structures_allocated: self.acceleration_structures.allocated_count(),
            acceleration_structures_capacity: self.config.max_acceleration_structures,
            pending_updates: self.pending_updates.len() as u32,
            deferred_deletions: self.deferred_deletions.len() as u32,
            frame_index: self.frame_index,
        }
    }

    /// Set debug name for a handle.
    #[cfg(debug_assertions)]
    pub fn set_debug_name(&mut self, handle: BindlessHandle, name: impl Into<String>) {
        let name = name.into();
        match handle.resource_type {
            BindlessResourceType::SampledTexture => {
                self.sampled_textures.set_debug_name(handle.index, name);
            },
            BindlessResourceType::StorageTexture => {
                self.storage_textures.set_debug_name(handle.index, name);
            },
            BindlessResourceType::UniformBuffer => {
                self.uniform_buffers.set_debug_name(handle.index, name);
            },
            BindlessResourceType::StorageBuffer => {
                self.storage_buffers.set_debug_name(handle.index, name);
            },
            BindlessResourceType::Sampler => {
                self.samplers.set_debug_name(handle.index, name);
            },
            BindlessResourceType::AccelerationStructure => {
                self.acceleration_structures
                    .set_debug_name(handle.index, name);
            },
            BindlessResourceType::None => {},
        }
    }

    #[cfg(not(debug_assertions))]
    pub fn set_debug_name(&mut self, _handle: BindlessHandle, _name: impl Into<String>) {}
}

/// Bindless heap statistics.
#[derive(Debug, Clone)]
pub struct BindlessHeapStats {
    pub sampled_textures_allocated: u32,
    pub sampled_textures_capacity: u32,
    pub storage_textures_allocated: u32,
    pub storage_textures_capacity: u32,
    pub uniform_buffers_allocated: u32,
    pub uniform_buffers_capacity: u32,
    pub storage_buffers_allocated: u32,
    pub storage_buffers_capacity: u32,
    pub samplers_allocated: u32,
    pub samplers_capacity: u32,
    pub acceleration_structures_allocated: u32,
    pub acceleration_structures_capacity: u32,
    pub pending_updates: u32,
    pub deferred_deletions: u32,
    pub frame_index: u64,
}

impl BindlessHeapStats {
    /// Total allocated descriptors.
    pub fn total_allocated(&self) -> u32 {
        self.sampled_textures_allocated
            + self.storage_textures_allocated
            + self.uniform_buffers_allocated
            + self.storage_buffers_allocated
            + self.samplers_allocated
            + self.acceleration_structures_allocated
    }

    /// Total capacity.
    pub fn total_capacity(&self) -> u32 {
        self.sampled_textures_capacity
            + self.storage_textures_capacity
            + self.uniform_buffers_capacity
            + self.storage_buffers_capacity
            + self.samplers_capacity
            + self.acceleration_structures_capacity
    }

    /// Usage percentage.
    pub fn usage_percent(&self) -> f32 {
        if self.total_capacity() == 0 {
            0.0
        } else {
            (self.total_allocated() as f32 / self.total_capacity() as f32) * 100.0
        }
    }
}

// ============================================================================
// Bindless Table
// ============================================================================

/// GPU-visible bindless table for shader access.
#[derive(Debug, Clone)]
pub struct BindlessTable {
    /// Entries.
    entries: Vec<BindlessTableEntry>,
    /// Dirty range.
    dirty_start: u32,
    dirty_end: u32,
}

/// Bindless table entry.
#[derive(Debug, Clone, Copy, Default)]
pub struct BindlessTableEntry {
    /// Resource handle (GPU pointer or index).
    pub handle: u64,
    /// Additional data (e.g., view parameters).
    pub data: u64,
}

impl BindlessTable {
    /// Create a new table.
    pub fn new(capacity: u32) -> Self {
        Self {
            entries: vec![BindlessTableEntry::default(); capacity as usize],
            dirty_start: u32::MAX,
            dirty_end: 0,
        }
    }

    /// Set an entry.
    pub fn set(&mut self, index: u32, entry: BindlessTableEntry) {
        if let Some(slot) = self.entries.get_mut(index as usize) {
            *slot = entry;
            self.dirty_start = self.dirty_start.min(index);
            self.dirty_end = self.dirty_end.max(index + 1);
        }
    }

    /// Get an entry.
    pub fn get(&self, index: u32) -> Option<&BindlessTableEntry> {
        self.entries.get(index as usize)
    }

    /// Clear an entry.
    pub fn clear(&mut self, index: u32) {
        self.set(index, BindlessTableEntry::default());
    }

    /// Get dirty range.
    pub fn dirty_range(&self) -> Option<(u32, u32)> {
        if self.dirty_start < self.dirty_end {
            Some((self.dirty_start, self.dirty_end))
        } else {
            None
        }
    }

    /// Get dirty entries as bytes.
    pub fn dirty_data(&self) -> &[BindlessTableEntry] {
        if self.dirty_start < self.dirty_end {
            &self.entries[self.dirty_start as usize..self.dirty_end as usize]
        } else {
            &[]
        }
    }

    /// Clear dirty flag.
    pub fn clear_dirty(&mut self) {
        self.dirty_start = u32::MAX;
        self.dirty_end = 0;
    }

    /// Get all entries.
    pub fn entries(&self) -> &[BindlessTableEntry] {
        &self.entries
    }
}

// ============================================================================
// Material Table
// ============================================================================

/// Material ID for GPU-driven rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId(pub u32);

impl MaterialId {
    /// Invalid material.
    pub const INVALID: Self = Self(u32::MAX);

    /// Create a new material ID.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID.
    pub fn raw(&self) -> u32 {
        self.0
    }
}

/// GPU material data.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct GpuMaterial {
    /// Albedo texture index.
    pub albedo_index: u32,
    /// Normal texture index.
    pub normal_index: u32,
    /// Roughness-metallic texture index.
    pub roughness_metallic_index: u32,
    /// Occlusion texture index.
    pub occlusion_index: u32,
    /// Emissive texture index.
    pub emissive_index: u32,
    /// Base color factor.
    pub base_color: [f32; 4],
    /// Metallic factor.
    pub metallic: f32,
    /// Roughness factor.
    pub roughness: f32,
    /// Emissive factor.
    pub emissive: [f32; 3],
    /// Alpha cutoff.
    pub alpha_cutoff: f32,
    /// Flags.
    pub flags: u32,
    /// Padding.
    _padding: [u32; 3],
}

impl GpuMaterial {
    /// Size in bytes.
    pub const SIZE: usize = 80;

    /// Create a default material.
    pub fn default_material() -> Self {
        Self {
            albedo_index: u32::MAX,
            normal_index: u32::MAX,
            roughness_metallic_index: u32::MAX,
            occlusion_index: u32::MAX,
            emissive_index: u32::MAX,
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0, 0.0, 0.0],
            alpha_cutoff: 0.5,
            flags: 0,
            _padding: [0; 3],
        }
    }
}

/// Material table for GPU-driven rendering.
pub struct MaterialTable {
    /// Materials.
    materials: Vec<GpuMaterial>,
    /// Free list.
    free_list: Vec<u32>,
    /// Name to ID mapping.
    name_map: BTreeMap<String, MaterialId>,
    /// Dirty materials.
    dirty: Vec<u32>,
}

impl MaterialTable {
    /// Create a new material table.
    pub fn new(capacity: u32) -> Self {
        let mut materials = Vec::with_capacity(capacity as usize);
        materials.resize_with(capacity as usize, GpuMaterial::default_material);

        let free_list: Vec<u32> = (0..capacity).rev().collect();

        Self {
            materials,
            free_list,
            name_map: BTreeMap::new(),
            dirty: Vec::new(),
        }
    }

    /// Allocate a material.
    pub fn allocate(&mut self) -> Option<MaterialId> {
        let index = self.free_list.pop()?;
        self.materials[index as usize] = GpuMaterial::default_material();
        self.dirty.push(index);
        Some(MaterialId(index))
    }

    /// Allocate a named material.
    pub fn allocate_named(&mut self, name: impl Into<String>) -> Option<MaterialId> {
        let id = self.allocate()?;
        self.name_map.insert(name.into(), id);
        Some(id)
    }

    /// Free a material.
    pub fn free(&mut self, id: MaterialId) {
        if (id.0 as usize) < self.materials.len() {
            self.free_list.push(id.0);
            self.name_map.retain(|_, v| *v != id);
        }
    }

    /// Get material by name.
    pub fn get_by_name(&self, name: &str) -> Option<MaterialId> {
        self.name_map.get(name).copied()
    }

    /// Get material data.
    pub fn get(&self, id: MaterialId) -> Option<&GpuMaterial> {
        self.materials.get(id.0 as usize)
    }

    /// Get mutable material data.
    pub fn get_mut(&mut self, id: MaterialId) -> Option<&mut GpuMaterial> {
        if (id.0 as usize) < self.materials.len() {
            self.dirty.push(id.0);
            Some(&mut self.materials[id.0 as usize])
        } else {
            None
        }
    }

    /// Update material.
    pub fn update(&mut self, id: MaterialId, material: GpuMaterial) {
        if let Some(slot) = self.materials.get_mut(id.0 as usize) {
            *slot = material;
            self.dirty.push(id.0);
        }
    }

    /// Take dirty list.
    pub fn take_dirty(&mut self) -> Vec<u32> {
        core::mem::take(&mut self.dirty)
    }

    /// Get all materials.
    pub fn materials(&self) -> &[GpuMaterial] {
        &self.materials
    }

    /// Get material count.
    pub fn count(&self) -> u32 {
        (self.materials.len() - self.free_list.len()) as u32
    }
}

// ============================================================================
// Instance Table
// ============================================================================

/// Instance ID for GPU-driven rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InstanceId(pub u32);

impl InstanceId {
    /// Invalid instance.
    pub const INVALID: Self = Self(u32::MAX);
}

/// GPU instance data.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct GpuInstance {
    /// World transform (4x3 matrix, row-major).
    pub transform: [[f32; 4]; 3],
    /// Previous frame transform for motion vectors.
    pub prev_transform: [[f32; 4]; 3],
    /// Mesh index.
    pub mesh_index: u32,
    /// Material ID.
    pub material_id: u32,
    /// Flags.
    pub flags: u32,
    /// Custom data.
    pub custom: u32,
}

impl GpuInstance {
    /// Size in bytes.
    pub const SIZE: usize = 112;

    /// Instance flags.
    pub const FLAG_VISIBLE: u32 = 1 << 0;
    pub const FLAG_CAST_SHADOW: u32 = 1 << 1;
    pub const FLAG_RECEIVE_SHADOW: u32 = 1 << 2;
    pub const FLAG_STATIC: u32 = 1 << 3;
    pub const FLAG_SKINNED: u32 = 1 << 4;

    /// Create a default instance.
    pub fn new() -> Self {
        Self {
            transform: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [
                0.0, 0.0, 1.0, 0.0,
            ]],
            prev_transform: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [
                0.0, 0.0, 1.0, 0.0,
            ]],
            mesh_index: u32::MAX,
            material_id: u32::MAX,
            flags: Self::FLAG_VISIBLE | Self::FLAG_CAST_SHADOW | Self::FLAG_RECEIVE_SHADOW,
            custom: 0,
        }
    }

    /// Set transform from matrix.
    pub fn set_transform(&mut self, m: [[f32; 4]; 4]) {
        self.prev_transform = self.transform;
        self.transform = [m[0], m[1], m[2]];
    }
}

/// Instance table for GPU-driven rendering.
pub struct InstanceTable {
    /// Instances.
    instances: Vec<GpuInstance>,
    /// Free list.
    free_list: Vec<u32>,
    /// Dirty instances.
    dirty: Vec<u32>,
}

impl InstanceTable {
    /// Create a new instance table.
    pub fn new(capacity: u32) -> Self {
        let mut instances = Vec::with_capacity(capacity as usize);
        instances.resize_with(capacity as usize, GpuInstance::new);

        let free_list: Vec<u32> = (0..capacity).rev().collect();

        Self {
            instances,
            free_list,
            dirty: Vec::new(),
        }
    }

    /// Allocate an instance.
    pub fn allocate(&mut self) -> Option<InstanceId> {
        let index = self.free_list.pop()?;
        self.instances[index as usize] = GpuInstance::new();
        self.dirty.push(index);
        Some(InstanceId(index))
    }

    /// Free an instance.
    pub fn free(&mut self, id: InstanceId) {
        if (id.0 as usize) < self.instances.len() {
            self.free_list.push(id.0);
        }
    }

    /// Get instance.
    pub fn get(&self, id: InstanceId) -> Option<&GpuInstance> {
        self.instances.get(id.0 as usize)
    }

    /// Get mutable instance.
    pub fn get_mut(&mut self, id: InstanceId) -> Option<&mut GpuInstance> {
        if (id.0 as usize) < self.instances.len() {
            self.dirty.push(id.0);
            Some(&mut self.instances[id.0 as usize])
        } else {
            None
        }
    }

    /// Update instance.
    pub fn update(&mut self, id: InstanceId, instance: GpuInstance) {
        if let Some(slot) = self.instances.get_mut(id.0 as usize) {
            *slot = instance;
            self.dirty.push(id.0);
        }
    }

    /// Take dirty list.
    pub fn take_dirty(&mut self) -> Vec<u32> {
        core::mem::take(&mut self.dirty)
    }

    /// Get all instances.
    pub fn instances(&self) -> &[GpuInstance] {
        &self.instances
    }

    /// Get instance count.
    pub fn count(&self) -> u32 {
        (self.instances.len() - self.free_list.len()) as u32
    }
}
