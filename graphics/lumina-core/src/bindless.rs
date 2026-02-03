//! Bindless Resources for LUMINA
//!
//! Bindless resources allow shaders to access resources through indices rather
//! than fixed descriptor bindings. This enables:
//!
//! - GPU-driven rendering with dynamic resource selection
//! - Reduced CPU overhead for descriptor updates
//! - Massive resource arrays (millions of textures/buffers)
//!
//! ## Architecture
//!
//! ```text
//! Traditional Binding:
//!   Shader: layout(binding = 0) uniform sampler2D tex;
//!   CPU: Bind texture to slot 0 before each draw
//!
//! Bindless:
//!   Shader: layout(binding = 0) uniform sampler2D textures[];
//!   GPU: textures[materialData.textureIndex]
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Create bindless descriptor heap
//! let heap = BindlessHeap::new(device, BindlessHeapConfig {
//!     max_samplers: 2048,
//!     max_sampled_images: 1_000_000,
//!     max_storage_images: 100_000,
//!     max_storage_buffers: 500_000,
//!     max_uniform_buffers: 10_000,
//! });
//!
//! // Allocate a handle
//! let tex_handle = heap.allocate_sampled_image(image_view, sampler);
//!
//! // Use in shader via push constant or buffer
//! material_data.albedo_index = tex_handle.index();
//! ```

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::error::{Error, Result};

// ============================================================================
// Bindless Handles
// ============================================================================

/// Handle to a bindless resource
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BindlessHandle {
    /// Index into the descriptor array
    index: u32,
    /// Generation for validation
    generation: u16,
    /// Resource type
    resource_type: BindlessResourceType,
}

impl BindlessHandle {
    /// Invalid handle constant
    pub const INVALID: Self = Self {
        index: u32::MAX,
        generation: 0,
        resource_type: BindlessResourceType::SampledImage,
    };

    /// Create a new handle
    pub(crate) const fn new(
        index: u32,
        generation: u16,
        resource_type: BindlessResourceType,
    ) -> Self {
        Self {
            index,
            generation,
            resource_type,
        }
    }

    /// Get the index for use in shaders
    #[inline]
    pub const fn index(&self) -> u32 {
        self.index
    }

    /// Get the generation
    #[inline]
    pub const fn generation(&self) -> u16 {
        self.generation
    }

    /// Get the resource type
    #[inline]
    pub const fn resource_type(&self) -> BindlessResourceType {
        self.resource_type
    }

    /// Check if handle is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }

    /// Pack into a u64 for storage
    #[inline]
    pub const fn pack(&self) -> u64 {
        ((self.resource_type as u64) << 48) | ((self.generation as u64) << 32) | (self.index as u64)
    }

    /// Unpack from a u64
    #[inline]
    pub const fn unpack(packed: u64) -> Self {
        Self {
            index: packed as u32,
            generation: (packed >> 32) as u16,
            resource_type: match (packed >> 48) as u8 {
                0 => BindlessResourceType::Sampler,
                1 => BindlessResourceType::SampledImage,
                2 => BindlessResourceType::StorageImage,
                3 => BindlessResourceType::UniformBuffer,
                4 => BindlessResourceType::StorageBuffer,
                5 => BindlessResourceType::AccelerationStructure,
                _ => BindlessResourceType::SampledImage,
            },
        }
    }
}

impl Default for BindlessHandle {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Type of bindless resource
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BindlessResourceType {
    /// Sampler
    Sampler       = 0,
    /// Combined image sampler or sampled image
    SampledImage  = 1,
    /// Storage image (read/write)
    StorageImage  = 2,
    /// Uniform buffer
    UniformBuffer = 3,
    /// Storage buffer
    StorageBuffer = 4,
    /// Acceleration structure (ray tracing)
    AccelerationStructure = 5,
}

// ============================================================================
// Bindless Heap Configuration
// ============================================================================

/// Configuration for bindless descriptor heap
#[derive(Clone, Debug)]
pub struct BindlessHeapConfig {
    /// Maximum number of samplers
    pub max_samplers: u32,
    /// Maximum number of sampled images
    pub max_sampled_images: u32,
    /// Maximum number of storage images
    pub max_storage_images: u32,
    /// Maximum number of uniform buffers
    pub max_uniform_buffers: u32,
    /// Maximum number of storage buffers
    pub max_storage_buffers: u32,
    /// Maximum number of acceleration structures
    pub max_acceleration_structures: u32,
    /// Enable update-after-bind
    pub update_after_bind: bool,
    /// Enable partially bound descriptors
    pub partially_bound: bool,
}

impl Default for BindlessHeapConfig {
    fn default() -> Self {
        Self {
            max_samplers: 2048,
            max_sampled_images: 500_000,
            max_storage_images: 50_000,
            max_uniform_buffers: 10_000,
            max_storage_buffers: 100_000,
            max_acceleration_structures: 1000,
            update_after_bind: true,
            partially_bound: true,
        }
    }
}

impl BindlessHeapConfig {
    /// Configuration for small heaps (mobile, embedded)
    pub fn small() -> Self {
        Self {
            max_samplers: 256,
            max_sampled_images: 4096,
            max_storage_images: 256,
            max_uniform_buffers: 256,
            max_storage_buffers: 1024,
            max_acceleration_structures: 0,
            update_after_bind: true,
            partially_bound: true,
        }
    }

    /// Configuration for large heaps (desktop, high-end)
    pub fn large() -> Self {
        Self {
            max_samplers: 4096,
            max_sampled_images: 1_000_000,
            max_storage_images: 100_000,
            max_uniform_buffers: 50_000,
            max_storage_buffers: 500_000,
            max_acceleration_structures: 10_000,
            update_after_bind: true,
            partially_bound: true,
        }
    }

    /// Total descriptors in this configuration
    pub fn total_descriptors(&self) -> u32 {
        self.max_samplers
            + self.max_sampled_images
            + self.max_storage_images
            + self.max_uniform_buffers
            + self.max_storage_buffers
            + self.max_acceleration_structures
    }
}

// ============================================================================
// Slot Allocator
// ============================================================================

/// Slot state in the free list
#[derive(Clone, Copy, Debug)]
struct Slot {
    /// Next free slot index (or u32::MAX if end of list)
    next_free: u32,
    /// Generation counter for this slot
    generation: u16,
    /// Whether slot is currently allocated
    allocated: bool,
}

impl Default for Slot {
    fn default() -> Self {
        Self {
            next_free: u32::MAX,
            generation: 0,
            allocated: false,
        }
    }
}

/// Free-list based slot allocator for a single resource type
#[derive(Debug)]
pub struct SlotAllocator {
    /// Slots array
    slots: [Slot; Self::MAX_SLOTS],
    /// Number of slots
    capacity: u32,
    /// Head of free list
    free_head: u32,
    /// Number of allocated slots
    allocated_count: u32,
    /// Resource type
    resource_type: BindlessResourceType,
}

impl SlotAllocator {
    /// Maximum slots per allocator
    pub const MAX_SLOTS: usize = 4096;

    /// Create a new allocator
    pub fn new(capacity: u32, resource_type: BindlessResourceType) -> Self {
        let capacity = capacity.min(Self::MAX_SLOTS as u32);
        let mut slots = [Slot::default(); Self::MAX_SLOTS];

        // Initialize free list
        for i in 0..capacity as usize {
            slots[i].next_free = if i + 1 < capacity as usize {
                (i + 1) as u32
            } else {
                u32::MAX
            };
        }

        Self {
            slots,
            capacity,
            free_head: if capacity > 0 { 0 } else { u32::MAX },
            allocated_count: 0,
            resource_type,
        }
    }

    /// Allocate a slot
    pub fn allocate(&mut self) -> Result<BindlessHandle> {
        if self.free_head == u32::MAX {
            return Err(Error::OutOfMemory);
        }

        let index = self.free_head;
        let slot = &mut self.slots[index as usize];

        self.free_head = slot.next_free;
        slot.allocated = true;
        slot.next_free = u32::MAX;
        self.allocated_count += 1;

        Ok(BindlessHandle::new(
            index,
            slot.generation,
            self.resource_type,
        ))
    }

    /// Free a slot
    pub fn free(&mut self, handle: BindlessHandle) -> Result<()> {
        if handle.index >= self.capacity {
            return Err(Error::InvalidHandle);
        }

        let slot = &mut self.slots[handle.index as usize];

        if !slot.allocated {
            return Err(Error::InvalidHandle);
        }
        if slot.generation != handle.generation {
            return Err(Error::InvalidHandle);
        }

        slot.allocated = false;
        slot.generation = slot.generation.wrapping_add(1);
        slot.next_free = self.free_head;
        self.free_head = handle.index;
        self.allocated_count -= 1;

        Ok(())
    }

    /// Check if a handle is valid
    pub fn is_valid(&self, handle: BindlessHandle) -> bool {
        if handle.index >= self.capacity {
            return false;
        }
        let slot = &self.slots[handle.index as usize];
        slot.allocated && slot.generation == handle.generation
    }

    /// Get allocated count
    pub fn allocated_count(&self) -> u32 {
        self.allocated_count
    }

    /// Get capacity
    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Get available slots
    pub fn available(&self) -> u32 {
        self.capacity - self.allocated_count
    }
}

// ============================================================================
// Bindless Heap
// ============================================================================

/// Bindless descriptor heap
///
/// This manages large arrays of descriptors that can be indexed in shaders.
#[derive(Debug)]
pub struct BindlessHeap {
    /// Configuration
    config: BindlessHeapConfig,
    /// Descriptor set layout handle
    layout: u64,
    /// Descriptor pool handle
    pool: u64,
    /// Descriptor set handle
    descriptor_set: u64,
    /// Allocators for each resource type
    sampler_allocator: SlotAllocator,
    sampled_image_allocator: SlotAllocator,
    storage_image_allocator: SlotAllocator,
    uniform_buffer_allocator: SlotAllocator,
    storage_buffer_allocator: SlotAllocator,
    accel_struct_allocator: SlotAllocator,
}

impl BindlessHeap {
    /// Create a new bindless heap
    pub fn new(config: BindlessHeapConfig) -> Self {
        Self {
            sampler_allocator: SlotAllocator::new(
                config.max_samplers.min(SlotAllocator::MAX_SLOTS as u32),
                BindlessResourceType::Sampler,
            ),
            sampled_image_allocator: SlotAllocator::new(
                config
                    .max_sampled_images
                    .min(SlotAllocator::MAX_SLOTS as u32),
                BindlessResourceType::SampledImage,
            ),
            storage_image_allocator: SlotAllocator::new(
                config
                    .max_storage_images
                    .min(SlotAllocator::MAX_SLOTS as u32),
                BindlessResourceType::StorageImage,
            ),
            uniform_buffer_allocator: SlotAllocator::new(
                config
                    .max_uniform_buffers
                    .min(SlotAllocator::MAX_SLOTS as u32),
                BindlessResourceType::UniformBuffer,
            ),
            storage_buffer_allocator: SlotAllocator::new(
                config
                    .max_storage_buffers
                    .min(SlotAllocator::MAX_SLOTS as u32),
                BindlessResourceType::StorageBuffer,
            ),
            accel_struct_allocator: SlotAllocator::new(
                config
                    .max_acceleration_structures
                    .min(SlotAllocator::MAX_SLOTS as u32),
                BindlessResourceType::AccelerationStructure,
            ),
            config,
            layout: 0,
            pool: 0,
            descriptor_set: 0,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &BindlessHeapConfig {
        &self.config
    }

    /// Get the descriptor set for binding
    pub fn descriptor_set(&self) -> u64 {
        self.descriptor_set
    }

    /// Allocate a sampler slot
    pub fn allocate_sampler(&mut self) -> Result<BindlessHandle> {
        self.sampler_allocator.allocate()
    }

    /// Allocate a sampled image slot
    pub fn allocate_sampled_image(&mut self) -> Result<BindlessHandle> {
        self.sampled_image_allocator.allocate()
    }

    /// Allocate a storage image slot
    pub fn allocate_storage_image(&mut self) -> Result<BindlessHandle> {
        self.storage_image_allocator.allocate()
    }

    /// Allocate a uniform buffer slot
    pub fn allocate_uniform_buffer(&mut self) -> Result<BindlessHandle> {
        self.uniform_buffer_allocator.allocate()
    }

    /// Allocate a storage buffer slot
    pub fn allocate_storage_buffer(&mut self) -> Result<BindlessHandle> {
        self.storage_buffer_allocator.allocate()
    }

    /// Allocate an acceleration structure slot
    pub fn allocate_acceleration_structure(&mut self) -> Result<BindlessHandle> {
        self.accel_struct_allocator.allocate()
    }

    /// Free a resource handle
    pub fn free(&mut self, handle: BindlessHandle) -> Result<()> {
        match handle.resource_type {
            BindlessResourceType::Sampler => self.sampler_allocator.free(handle),
            BindlessResourceType::SampledImage => self.sampled_image_allocator.free(handle),
            BindlessResourceType::StorageImage => self.storage_image_allocator.free(handle),
            BindlessResourceType::UniformBuffer => self.uniform_buffer_allocator.free(handle),
            BindlessResourceType::StorageBuffer => self.storage_buffer_allocator.free(handle),
            BindlessResourceType::AccelerationStructure => self.accel_struct_allocator.free(handle),
        }
    }

    /// Check if a handle is valid
    pub fn is_valid(&self, handle: BindlessHandle) -> bool {
        match handle.resource_type {
            BindlessResourceType::Sampler => self.sampler_allocator.is_valid(handle),
            BindlessResourceType::SampledImage => self.sampled_image_allocator.is_valid(handle),
            BindlessResourceType::StorageImage => self.storage_image_allocator.is_valid(handle),
            BindlessResourceType::UniformBuffer => self.uniform_buffer_allocator.is_valid(handle),
            BindlessResourceType::StorageBuffer => self.storage_buffer_allocator.is_valid(handle),
            BindlessResourceType::AccelerationStructure => {
                self.accel_struct_allocator.is_valid(handle)
            },
        }
    }

    /// Get statistics
    pub fn stats(&self) -> BindlessHeapStats {
        BindlessHeapStats {
            samplers_allocated: self.sampler_allocator.allocated_count(),
            samplers_capacity: self.sampler_allocator.capacity(),
            sampled_images_allocated: self.sampled_image_allocator.allocated_count(),
            sampled_images_capacity: self.sampled_image_allocator.capacity(),
            storage_images_allocated: self.storage_image_allocator.allocated_count(),
            storage_images_capacity: self.storage_image_allocator.capacity(),
            uniform_buffers_allocated: self.uniform_buffer_allocator.allocated_count(),
            uniform_buffers_capacity: self.uniform_buffer_allocator.capacity(),
            storage_buffers_allocated: self.storage_buffer_allocator.allocated_count(),
            storage_buffers_capacity: self.storage_buffer_allocator.capacity(),
            accel_structs_allocated: self.accel_struct_allocator.allocated_count(),
            accel_structs_capacity: self.accel_struct_allocator.capacity(),
        }
    }
}

/// Statistics for a bindless heap
#[derive(Clone, Copy, Debug, Default)]
pub struct BindlessHeapStats {
    pub samplers_allocated: u32,
    pub samplers_capacity: u32,
    pub sampled_images_allocated: u32,
    pub sampled_images_capacity: u32,
    pub storage_images_allocated: u32,
    pub storage_images_capacity: u32,
    pub uniform_buffers_allocated: u32,
    pub uniform_buffers_capacity: u32,
    pub storage_buffers_allocated: u32,
    pub storage_buffers_capacity: u32,
    pub accel_structs_allocated: u32,
    pub accel_structs_capacity: u32,
}

impl BindlessHeapStats {
    /// Total allocated descriptors
    pub fn total_allocated(&self) -> u32 {
        self.samplers_allocated
            + self.sampled_images_allocated
            + self.storage_images_allocated
            + self.uniform_buffers_allocated
            + self.storage_buffers_allocated
            + self.accel_structs_allocated
    }

    /// Total capacity
    pub fn total_capacity(&self) -> u32 {
        self.samplers_capacity
            + self.sampled_images_capacity
            + self.storage_images_capacity
            + self.uniform_buffers_capacity
            + self.storage_buffers_capacity
            + self.accel_structs_capacity
    }

    /// Usage percentage (0.0-1.0)
    pub fn usage(&self) -> f32 {
        if self.total_capacity() == 0 {
            0.0
        } else {
            self.total_allocated() as f32 / self.total_capacity() as f32
        }
    }
}

// ============================================================================
// Descriptor Update
// ============================================================================

/// Descriptor write for bindless resources
#[derive(Clone, Debug)]
pub struct BindlessWrite {
    /// Handle to update
    pub handle: BindlessHandle,
    /// Write data
    pub data: BindlessWriteData,
}

/// Data for a bindless write
#[derive(Clone, Debug)]
pub enum BindlessWriteData {
    /// Sampler
    Sampler {
        sampler: u64, // Sampler handle
    },
    /// Sampled image (with optional sampler for combined image sampler)
    SampledImage {
        image_view: u64,
        image_layout: ImageLayout,
        sampler: Option<u64>,
    },
    /// Storage image
    StorageImage {
        image_view: u64,
        image_layout: ImageLayout,
    },
    /// Uniform buffer
    UniformBuffer {
        buffer: u64,
        offset: u64,
        range: u64,
    },
    /// Storage buffer
    StorageBuffer {
        buffer: u64,
        offset: u64,
        range: u64,
    },
    /// Acceleration structure
    AccelerationStructure { acceleration_structure: u64 },
}

/// Image layout for descriptor writes
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImageLayout {
    #[default]
    Undefined,
    General,
    ColorAttachment,
    DepthStencilAttachment,
    DepthStencilReadOnly,
    ShaderReadOnly,
    TransferSrc,
    TransferDst,
    Present,
}

// ============================================================================
// Bindless Extension Trait
// ============================================================================

/// Extension trait for devices to create bindless heaps
pub trait BindlessDevice {
    /// Create a bindless heap
    fn create_bindless_heap(&self, config: &BindlessHeapConfig) -> Result<BindlessHeap>;

    /// Destroy a bindless heap
    fn destroy_bindless_heap(&self, heap: BindlessHeap);

    /// Update bindless descriptors
    fn update_bindless_descriptors(&self, heap: &BindlessHeap, writes: &[BindlessWrite]);
}

// ============================================================================
// Shader Macros/Helpers
// ============================================================================

/// Helper struct for passing bindless indices to shaders
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct BindlessIndices {
    /// Albedo texture index
    pub albedo: u32,
    /// Normal map index
    pub normal: u32,
    /// Roughness/metallic texture index
    pub roughness_metallic: u32,
    /// Emissive texture index
    pub emissive: u32,
    /// AO texture index
    pub ao: u32,
    /// Padding
    pub _padding: [u32; 3],
}

impl BindlessIndices {
    /// Create from handles
    pub fn from_handles(
        albedo: BindlessHandle,
        normal: BindlessHandle,
        roughness_metallic: BindlessHandle,
        emissive: BindlessHandle,
        ao: BindlessHandle,
    ) -> Self {
        Self {
            albedo: albedo.index(),
            normal: normal.index(),
            roughness_metallic: roughness_metallic.index(),
            emissive: emissive.index(),
            ao: ao.index(),
            _padding: [0; 3],
        }
    }

    /// Check if all indices are valid
    pub fn is_valid(&self) -> bool {
        self.albedo != u32::MAX && self.normal != u32::MAX && self.roughness_metallic != u32::MAX
    }
}

/// Material data using bindless indices
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct BindlessMaterial {
    /// Texture indices
    pub textures: BindlessIndices,
    /// Base color factor
    pub base_color: [f32; 4],
    /// Metallic factor
    pub metallic: f32,
    /// Roughness factor
    pub roughness: f32,
    /// Emissive factor
    pub emissive: [f32; 3],
    /// Alpha cutoff
    pub alpha_cutoff: f32,
    /// Flags
    pub flags: u32,
    /// Padding
    pub _padding: [u32; 3],
}

impl BindlessMaterial {
    /// Material is double-sided
    pub const FLAG_DOUBLE_SIDED: u32 = 1 << 0;
    /// Material uses alpha blending
    pub const FLAG_ALPHA_BLEND: u32 = 1 << 1;
    /// Material uses alpha cutoff
    pub const FLAG_ALPHA_CUTOFF: u32 = 1 << 2;
    /// Material is unlit
    pub const FLAG_UNLIT: u32 = 1 << 3;
}
