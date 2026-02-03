//! Bindless Resources Types for Lumina Extended
//!
//! This module provides extended bindless resource infrastructure
//! for descriptor indexing and GPU-driven resource management.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Bindless Handles
// ============================================================================

/// Bindless resource table handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BindlessTableHandle(pub u64);

impl BindlessTableHandle {
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

impl Default for BindlessTableHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Bindless descriptor index
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BindlessIndex(pub u32);

impl BindlessIndex {
    /// Invalid index
    pub const INVALID: Self = Self(u32::MAX);

    /// Creates new index
    #[inline]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Is valid
    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }

    /// Raw value
    #[inline]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

impl Default for BindlessIndex {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Bindless heap handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BindlessHeapHandle(pub u64);

impl BindlessHeapHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for BindlessHeapHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Bindless allocator handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BindlessAllocatorHandle(pub u64);

impl BindlessAllocatorHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for BindlessAllocatorHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Bindless Table Creation
// ============================================================================

/// Bindless table create info
#[derive(Clone, Debug)]
pub struct BindlessTableCreateInfo {
    /// Name
    pub name: String,
    /// Table type
    pub table_type: BindlessTableType,
    /// Capacity
    pub capacity: u32,
    /// Allow resize
    pub allow_resize: bool,
    /// Shader visible
    pub shader_visible: bool,
    /// Update mode
    pub update_mode: BindlessUpdateMode,
}

impl BindlessTableCreateInfo {
    /// Creates new info
    pub fn new(table_type: BindlessTableType, capacity: u32) -> Self {
        Self {
            name: String::new(),
            table_type,
            capacity,
            allow_resize: false,
            shader_visible: true,
            update_mode: BindlessUpdateMode::Immediate,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Allow resize
    pub fn with_resize(mut self) -> Self {
        self.allow_resize = true;
        self
    }

    /// With update mode
    pub fn with_update_mode(mut self, mode: BindlessUpdateMode) -> Self {
        self.update_mode = mode;
        self
    }

    /// CPU only (not shader visible)
    pub fn cpu_only(mut self) -> Self {
        self.shader_visible = false;
        self
    }

    /// Texture table
    pub fn textures(capacity: u32) -> Self {
        Self::new(BindlessTableType::Textures, capacity)
    }

    /// Buffer table
    pub fn buffers(capacity: u32) -> Self {
        Self::new(BindlessTableType::Buffers, capacity)
    }

    /// Sampler table
    pub fn samplers(capacity: u32) -> Self {
        Self::new(BindlessTableType::Samplers, capacity)
    }

    /// Combined table
    pub fn combined(capacity: u32) -> Self {
        Self::new(BindlessTableType::Combined, capacity)
    }

    /// Large texture table preset
    pub fn large_texture_table() -> Self {
        Self::textures(1_000_000).with_resize()
    }

    /// Standard combined table
    pub fn standard_combined() -> Self {
        Self::combined(100_000)
    }
}

impl Default for BindlessTableCreateInfo {
    fn default() -> Self {
        Self::combined(10000)
    }
}

/// Bindless table type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BindlessTableType {
    /// Texture/Image descriptors
    Textures = 0,
    /// Buffer descriptors
    Buffers = 1,
    /// Sampler descriptors
    Samplers = 2,
    /// Combined (all types)
    #[default]
    Combined = 3,
    /// Storage images
    StorageImages = 4,
    /// Acceleration structures
    AccelerationStructures = 5,
}

impl BindlessTableType {
    /// Display name
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Textures => "Textures",
            Self::Buffers => "Buffers",
            Self::Samplers => "Samplers",
            Self::Combined => "Combined",
            Self::StorageImages => "Storage Images",
            Self::AccelerationStructures => "Acceleration Structures",
        }
    }
}

/// Bindless update mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BindlessUpdateMode {
    /// Immediate updates
    #[default]
    Immediate = 0,
    /// Deferred updates (batched)
    Deferred = 1,
    /// Update after barrier
    AfterBarrier = 2,
}

// ============================================================================
// Bindless Resource Operations
// ============================================================================

/// Bindless resource descriptor
#[derive(Clone, Debug)]
pub struct BindlessDescriptor {
    /// Resource type
    pub resource_type: BindlessResourceType,
    /// Resource handle
    pub handle: u64,
    /// View type
    pub view_type: BindlessViewType,
    /// Format override (0 = use resource format)
    pub format: u32,
    /// Base mip level
    pub base_mip: u32,
    /// Mip level count (0 = all)
    pub mip_count: u32,
    /// Base array layer
    pub base_layer: u32,
    /// Layer count (0 = all)
    pub layer_count: u32,
}

impl BindlessDescriptor {
    /// Creates texture descriptor
    pub fn texture(handle: u64) -> Self {
        Self {
            resource_type: BindlessResourceType::Texture,
            handle,
            view_type: BindlessViewType::SampledImage,
            format: 0,
            base_mip: 0,
            mip_count: 0,
            base_layer: 0,
            layer_count: 0,
        }
    }

    /// Creates buffer descriptor
    pub fn buffer(handle: u64) -> Self {
        Self {
            resource_type: BindlessResourceType::Buffer,
            handle,
            view_type: BindlessViewType::UniformBuffer,
            format: 0,
            base_mip: 0,
            mip_count: 0,
            base_layer: 0,
            layer_count: 0,
        }
    }

    /// Creates storage buffer descriptor
    pub fn storage_buffer(handle: u64) -> Self {
        Self {
            resource_type: BindlessResourceType::Buffer,
            handle,
            view_type: BindlessViewType::StorageBuffer,
            format: 0,
            base_mip: 0,
            mip_count: 0,
            base_layer: 0,
            layer_count: 0,
        }
    }

    /// Creates storage image descriptor
    pub fn storage_image(handle: u64) -> Self {
        Self {
            resource_type: BindlessResourceType::Texture,
            handle,
            view_type: BindlessViewType::StorageImage,
            format: 0,
            base_mip: 0,
            mip_count: 1,
            base_layer: 0,
            layer_count: 0,
        }
    }

    /// Creates sampler descriptor
    pub fn sampler(handle: u64) -> Self {
        Self {
            resource_type: BindlessResourceType::Sampler,
            handle,
            view_type: BindlessViewType::Sampler,
            format: 0,
            base_mip: 0,
            mip_count: 0,
            base_layer: 0,
            layer_count: 0,
        }
    }

    /// Creates acceleration structure descriptor
    pub fn acceleration_structure(handle: u64) -> Self {
        Self {
            resource_type: BindlessResourceType::AccelerationStructure,
            handle,
            view_type: BindlessViewType::AccelerationStructure,
            format: 0,
            base_mip: 0,
            mip_count: 0,
            base_layer: 0,
            layer_count: 0,
        }
    }

    /// With format
    pub fn with_format(mut self, format: u32) -> Self {
        self.format = format;
        self
    }

    /// With mip levels
    pub fn with_mips(mut self, base: u32, count: u32) -> Self {
        self.base_mip = base;
        self.mip_count = count;
        self
    }

    /// With array layers
    pub fn with_layers(mut self, base: u32, count: u32) -> Self {
        self.base_layer = base;
        self.layer_count = count;
        self
    }
}

impl Default for BindlessDescriptor {
    fn default() -> Self {
        Self::texture(0)
    }
}

/// Bindless resource type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BindlessResourceType {
    /// Texture
    #[default]
    Texture = 0,
    /// Buffer
    Buffer = 1,
    /// Sampler
    Sampler = 2,
    /// Acceleration structure
    AccelerationStructure = 3,
}

/// Bindless view type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BindlessViewType {
    /// Sampled image
    #[default]
    SampledImage = 0,
    /// Storage image
    StorageImage = 1,
    /// Uniform buffer
    UniformBuffer = 2,
    /// Storage buffer
    StorageBuffer = 3,
    /// Uniform texel buffer
    UniformTexelBuffer = 4,
    /// Storage texel buffer
    StorageTexelBuffer = 5,
    /// Sampler
    Sampler = 6,
    /// Combined image sampler
    CombinedImageSampler = 7,
    /// Acceleration structure
    AccelerationStructure = 8,
}

// ============================================================================
// Bindless Allocation
// ============================================================================

/// Bindless allocation request
#[derive(Clone, Debug)]
pub struct BindlessAllocationRequest {
    /// Descriptor
    pub descriptor: BindlessDescriptor,
    /// Preferred index (INVALID = auto)
    pub preferred_index: BindlessIndex,
    /// Flags
    pub flags: BindlessAllocationFlags,
}

impl BindlessAllocationRequest {
    /// Creates new request
    pub fn new(descriptor: BindlessDescriptor) -> Self {
        Self {
            descriptor,
            preferred_index: BindlessIndex::INVALID,
            flags: BindlessAllocationFlags::empty(),
        }
    }

    /// With preferred index
    pub fn with_preferred_index(mut self, index: u32) -> Self {
        self.preferred_index = BindlessIndex::new(index);
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: BindlessAllocationFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// Persistent allocation
    pub fn persistent(mut self) -> Self {
        self.flags |= BindlessAllocationFlags::PERSISTENT;
        self
    }
}

impl Default for BindlessAllocationRequest {
    fn default() -> Self {
        Self::new(BindlessDescriptor::default())
    }
}

bitflags::bitflags! {
    /// Bindless allocation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct BindlessAllocationFlags: u32 {
        /// None
        const NONE = 0;
        /// Persistent (never freed automatically)
        const PERSISTENT = 1 << 0;
        /// Allow aliasing
        const ALIASED = 1 << 1;
        /// High priority (prefer fast slots)
        const HIGH_PRIORITY = 1 << 2;
    }
}

/// Bindless allocation result
#[derive(Clone, Copy, Debug, Default)]
pub struct BindlessAllocation {
    /// Allocated index
    pub index: BindlessIndex,
    /// Table handle
    pub table: BindlessTableHandle,
    /// Version (for validation)
    pub version: u32,
}

impl BindlessAllocation {
    /// Is valid
    pub fn is_valid(&self) -> bool {
        self.index.is_valid() && !self.table.is_null()
    }

    /// Creates GPU reference data
    pub fn gpu_data(&self) -> BindlessGpuReference {
        BindlessGpuReference {
            index: self.index.raw(),
            version: self.version,
        }
    }
}

/// GPU-side bindless reference
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct BindlessGpuReference {
    /// Descriptor index
    pub index: u32,
    /// Version for validation
    pub version: u32,
}

impl BindlessGpuReference {
    /// Invalid reference
    pub const INVALID: Self = Self {
        index: u32::MAX,
        version: 0,
    };

    /// Creates new reference
    pub const fn new(index: u32) -> Self {
        Self { index, version: 0 }
    }

    /// With version
    pub const fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Is valid
    pub const fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

// ============================================================================
// Bindless Heap
// ============================================================================

/// Bindless heap create info
#[derive(Clone, Debug)]
pub struct BindlessHeapCreateInfo {
    /// Name
    pub name: String,
    /// Descriptor count
    pub descriptor_count: u32,
    /// Heap type
    pub heap_type: BindlessHeapType,
    /// Shader visible
    pub shader_visible: bool,
    /// Node mask (for multi-GPU)
    pub node_mask: u32,
}

impl BindlessHeapCreateInfo {
    /// Creates new info
    pub fn new(descriptor_count: u32) -> Self {
        Self {
            name: String::new(),
            descriptor_count,
            heap_type: BindlessHeapType::CbvSrvUav,
            shader_visible: true,
            node_mask: 1,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With heap type
    pub fn with_type(mut self, heap_type: BindlessHeapType) -> Self {
        self.heap_type = heap_type;
        self
    }

    /// With node mask
    pub fn with_node_mask(mut self, mask: u32) -> Self {
        self.node_mask = mask;
        self
    }

    /// CBV/SRV/UAV heap
    pub fn cbv_srv_uav(count: u32) -> Self {
        Self::new(count).with_type(BindlessHeapType::CbvSrvUav)
    }

    /// Sampler heap
    pub fn sampler(count: u32) -> Self {
        Self::new(count).with_type(BindlessHeapType::Sampler)
    }
}

impl Default for BindlessHeapCreateInfo {
    fn default() -> Self {
        Self::cbv_srv_uav(10000)
    }
}

/// Bindless heap type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BindlessHeapType {
    /// CBV/SRV/UAV heap
    #[default]
    CbvSrvUav = 0,
    /// Sampler heap
    Sampler = 1,
    /// RTV heap
    Rtv = 2,
    /// DSV heap
    Dsv = 3,
}

impl BindlessHeapType {
    /// Max descriptors
    pub const fn max_descriptors(&self) -> u32 {
        match self {
            Self::CbvSrvUav => 1_000_000,
            Self::Sampler => 2048,
            Self::Rtv => 1024,
            Self::Dsv => 1024,
        }
    }
}

// ============================================================================
// Bindless Allocator
// ============================================================================

/// Bindless allocator create info
#[derive(Clone, Debug)]
pub struct BindlessAllocatorCreateInfo {
    /// Name
    pub name: String,
    /// Heap
    pub heap: BindlessHeapHandle,
    /// Start offset in heap
    pub offset: u32,
    /// Count
    pub count: u32,
    /// Strategy
    pub strategy: BindlessAllocatorStrategy,
}

impl BindlessAllocatorCreateInfo {
    /// Creates new info
    pub fn new(heap: BindlessHeapHandle, offset: u32, count: u32) -> Self {
        Self {
            name: String::new(),
            heap,
            offset,
            count,
            strategy: BindlessAllocatorStrategy::FirstFit,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With strategy
    pub fn with_strategy(mut self, strategy: BindlessAllocatorStrategy) -> Self {
        self.strategy = strategy;
        self
    }
}

impl Default for BindlessAllocatorCreateInfo {
    fn default() -> Self {
        Self::new(BindlessHeapHandle::NULL, 0, 0)
    }
}

/// Bindless allocator strategy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BindlessAllocatorStrategy {
    /// First fit
    #[default]
    FirstFit = 0,
    /// Best fit
    BestFit = 1,
    /// Linear/bump allocator
    Linear = 2,
    /// Ring buffer
    Ring = 3,
    /// Free list
    FreeList = 4,
}

// ============================================================================
// Batch Operations
// ============================================================================

/// Bindless batch update
#[derive(Clone, Debug, Default)]
pub struct BindlessBatchUpdate {
    /// Table
    pub table: BindlessTableHandle,
    /// Operations
    pub operations: Vec<BindlessBatchOperation>,
    /// Defer barrier
    pub defer_barrier: bool,
}

impl BindlessBatchUpdate {
    /// Creates new batch
    pub fn new(table: BindlessTableHandle) -> Self {
        Self {
            table,
            operations: Vec::new(),
            defer_barrier: false,
        }
    }

    /// Add operation
    pub fn add(mut self, operation: BindlessBatchOperation) -> Self {
        self.operations.push(operation);
        self
    }

    /// Write descriptor
    pub fn write(mut self, index: u32, descriptor: BindlessDescriptor) -> Self {
        self.operations.push(BindlessBatchOperation::Write {
            index: BindlessIndex::new(index),
            descriptor,
        });
        self
    }

    /// Free index
    pub fn free(mut self, index: u32) -> Self {
        self.operations.push(BindlessBatchOperation::Free {
            index: BindlessIndex::new(index),
        });
        self
    }

    /// Copy from another index
    pub fn copy(mut self, dst: u32, src: u32) -> Self {
        self.operations.push(BindlessBatchOperation::Copy {
            dst: BindlessIndex::new(dst),
            src: BindlessIndex::new(src),
        });
        self
    }

    /// Defer barrier
    pub fn with_deferred_barrier(mut self) -> Self {
        self.defer_barrier = true;
        self
    }

    /// Operation count
    pub fn count(&self) -> usize {
        self.operations.len()
    }
}

/// Bindless batch operation
#[derive(Clone, Debug)]
pub enum BindlessBatchOperation {
    /// Write descriptor
    Write {
        index: BindlessIndex,
        descriptor: BindlessDescriptor,
    },
    /// Free index
    Free {
        index: BindlessIndex,
    },
    /// Copy descriptor
    Copy {
        dst: BindlessIndex,
        src: BindlessIndex,
    },
}

// ============================================================================
// Statistics
// ============================================================================

/// Bindless statistics
#[derive(Clone, Debug, Default)]
pub struct BindlessStats {
    /// Total allocations
    pub total_allocations: u64,
    /// Total frees
    pub total_frees: u64,
    /// Current used count
    pub used_count: u32,
    /// Capacity
    pub capacity: u32,
    /// Fragmentation (0.0 - 1.0)
    pub fragmentation: f32,
    /// Peak usage
    pub peak_usage: u32,
    /// Updates per frame
    pub updates_per_frame: u32,
    /// Batch updates
    pub batch_updates: u64,
}

impl BindlessStats {
    /// Usage ratio (0.0 - 1.0)
    pub fn usage_ratio(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.used_count as f32 / self.capacity as f32
    }

    /// Available slots
    pub fn available(&self) -> u32 {
        self.capacity.saturating_sub(self.used_count)
    }
}
