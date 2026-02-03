//! GPU Memory Pools Types for Lumina
//!
//! This module provides GPU memory pool infrastructure
//! for efficient memory allocation and management.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Memory Pool Handles
// ============================================================================

/// Memory pool handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MemoryPoolHandle(pub u64);

impl MemoryPoolHandle {
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

impl Default for MemoryPoolHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Memory block handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MemoryBlockHandle(pub u64);

impl MemoryBlockHandle {
    /// Null handle
    pub const NULL: Self = Self(0);

    /// Is null
    #[inline]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for MemoryBlockHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Staging buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StagingBufferHandle(pub u64);

impl StagingBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StagingBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Ring buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RingBufferHandle(pub u64);

impl RingBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for RingBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Memory Pool Creation
// ============================================================================

/// Memory pool create info
#[derive(Clone, Debug)]
pub struct MemoryPoolCreateInfo {
    /// Name
    pub name: String,
    /// Pool type
    pub pool_type: MemoryPoolType,
    /// Block size
    pub block_size: u64,
    /// Initial block count
    pub initial_blocks: u32,
    /// Max block count (0 = unlimited)
    pub max_blocks: u32,
    /// Memory location
    pub memory_location: MemoryLocation,
    /// Allocation strategy
    pub strategy: AllocationStrategy,
    /// Min allocation size
    pub min_allocation_size: u64,
    /// Allocation alignment
    pub alignment: u64,
}

impl MemoryPoolCreateInfo {
    /// Creates new info
    pub fn new(pool_type: MemoryPoolType) -> Self {
        Self {
            name: String::new(),
            pool_type,
            block_size: 64 * 1024 * 1024,  // 64 MB
            initial_blocks: 1,
            max_blocks: 0,
            memory_location: MemoryLocation::GpuOnly,
            strategy: AllocationStrategy::BestFit,
            min_allocation_size: 256,
            alignment: 256,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With block size
    pub fn with_block_size(mut self, size: u64) -> Self {
        self.block_size = size;
        self
    }

    /// With block count
    pub fn with_blocks(mut self, initial: u32, max: u32) -> Self {
        self.initial_blocks = initial;
        self.max_blocks = max;
        self
    }

    /// With memory location
    pub fn with_memory_location(mut self, location: MemoryLocation) -> Self {
        self.memory_location = location;
        self
    }

    /// With strategy
    pub fn with_strategy(mut self, strategy: AllocationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// With alignment
    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Buffer pool preset
    pub fn buffer_pool() -> Self {
        Self::new(MemoryPoolType::Buffer)
            .with_block_size(64 * 1024 * 1024)
            .with_memory_location(MemoryLocation::GpuOnly)
    }

    /// Texture pool preset
    pub fn texture_pool() -> Self {
        Self::new(MemoryPoolType::Texture)
            .with_block_size(256 * 1024 * 1024)
            .with_memory_location(MemoryLocation::GpuOnly)
            .with_alignment(65536)  // 64KB for textures
    }

    /// Staging pool preset
    pub fn staging_pool() -> Self {
        Self::new(MemoryPoolType::Staging)
            .with_block_size(32 * 1024 * 1024)
            .with_memory_location(MemoryLocation::CpuToGpu)
            .with_strategy(AllocationStrategy::Linear)
    }

    /// Readback pool preset
    pub fn readback_pool() -> Self {
        Self::new(MemoryPoolType::Readback)
            .with_block_size(16 * 1024 * 1024)
            .with_memory_location(MemoryLocation::GpuToCpu)
    }

    /// Uniform buffer pool preset
    pub fn uniform_pool() -> Self {
        Self::new(MemoryPoolType::Uniform)
            .with_block_size(4 * 1024 * 1024)
            .with_memory_location(MemoryLocation::CpuToGpu)
            .with_alignment(256)  // UBO alignment
    }
}

impl Default for MemoryPoolCreateInfo {
    fn default() -> Self {
        Self::buffer_pool()
    }
}

/// Memory pool type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MemoryPoolType {
    /// General buffer pool
    #[default]
    Buffer = 0,
    /// Texture pool
    Texture = 1,
    /// Staging (upload) pool
    Staging = 2,
    /// Readback pool
    Readback = 3,
    /// Uniform buffer pool
    Uniform = 4,
    /// Storage buffer pool
    Storage = 5,
    /// Vertex/Index buffer pool
    Geometry = 6,
    /// Acceleration structure pool
    AccelerationStructure = 7,
}

impl MemoryPoolType {
    /// Display name
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Buffer => "Buffer",
            Self::Texture => "Texture",
            Self::Staging => "Staging",
            Self::Readback => "Readback",
            Self::Uniform => "Uniform",
            Self::Storage => "Storage",
            Self::Geometry => "Geometry",
            Self::AccelerationStructure => "Acceleration Structure",
        }
    }
}

/// Memory location
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MemoryLocation {
    /// GPU only (device local)
    #[default]
    GpuOnly = 0,
    /// CPU to GPU (host visible, write combined)
    CpuToGpu = 1,
    /// GPU to CPU (host visible, cached)
    GpuToCpu = 2,
    /// CPU only (host only)
    CpuOnly = 3,
    /// Prefer GPU (fallback to CPU)
    PreferGpu = 4,
    /// Prefer CPU (fallback to GPU)
    PreferCpu = 5,
}

impl MemoryLocation {
    /// Is host visible
    pub const fn is_host_visible(&self) -> bool {
        matches!(self, Self::CpuToGpu | Self::GpuToCpu | Self::CpuOnly | Self::PreferCpu)
    }

    /// Is device local
    pub const fn is_device_local(&self) -> bool {
        matches!(self, Self::GpuOnly | Self::PreferGpu)
    }
}

/// Allocation strategy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AllocationStrategy {
    /// Best fit (minimize fragmentation)
    #[default]
    BestFit = 0,
    /// First fit (faster allocation)
    FirstFit = 1,
    /// Linear/bump allocator (fastest, no free)
    Linear = 2,
    /// Buddy allocator
    Buddy = 3,
    /// TLSF (Two-Level Segregated Fit)
    Tlsf = 4,
    /// Ring buffer
    Ring = 5,
}

// ============================================================================
// Memory Allocation
// ============================================================================

/// Allocation request
#[derive(Clone, Debug)]
pub struct AllocationRequest {
    /// Size in bytes
    pub size: u64,
    /// Alignment
    pub alignment: u64,
    /// Pool (None = auto-select)
    pub pool: Option<MemoryPoolHandle>,
    /// Memory location preference
    pub memory_location: MemoryLocation,
    /// Flags
    pub flags: AllocationFlags,
    /// User data
    pub user_data: u64,
}

impl AllocationRequest {
    /// Creates new request
    pub fn new(size: u64) -> Self {
        Self {
            size,
            alignment: 256,
            pool: None,
            memory_location: MemoryLocation::GpuOnly,
            flags: AllocationFlags::empty(),
            user_data: 0,
        }
    }

    /// With alignment
    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// With pool
    pub fn with_pool(mut self, pool: MemoryPoolHandle) -> Self {
        self.pool = Some(pool);
        self
    }

    /// With memory location
    pub fn with_memory_location(mut self, location: MemoryLocation) -> Self {
        self.memory_location = location;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: AllocationFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// Dedicated allocation (own memory block)
    pub fn dedicated(mut self) -> Self {
        self.flags |= AllocationFlags::DEDICATED;
        self
    }

    /// Mappable allocation
    pub fn mappable(mut self) -> Self {
        self.flags |= AllocationFlags::MAPPABLE;
        self.memory_location = MemoryLocation::CpuToGpu;
        self
    }

    /// Buffer request
    pub fn buffer(size: u64) -> Self {
        Self::new(size)
            .with_alignment(256)
            .with_memory_location(MemoryLocation::GpuOnly)
    }

    /// Texture request
    pub fn texture(size: u64) -> Self {
        Self::new(size)
            .with_alignment(65536)
            .with_memory_location(MemoryLocation::GpuOnly)
    }

    /// Upload buffer request
    pub fn upload(size: u64) -> Self {
        Self::new(size)
            .with_alignment(256)
            .with_memory_location(MemoryLocation::CpuToGpu)
            .with_flags(AllocationFlags::MAPPABLE)
    }

    /// Readback buffer request
    pub fn readback(size: u64) -> Self {
        Self::new(size)
            .with_alignment(256)
            .with_memory_location(MemoryLocation::GpuToCpu)
            .with_flags(AllocationFlags::MAPPABLE)
    }
}

impl Default for AllocationRequest {
    fn default() -> Self {
        Self::new(0)
    }
}

bitflags::bitflags! {
    /// Allocation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct AllocationFlags: u32 {
        /// None
        const NONE = 0;
        /// Dedicated allocation (own memory block)
        const DEDICATED = 1 << 0;
        /// Mappable (host visible)
        const MAPPABLE = 1 << 1;
        /// Persistent mapping
        const PERSISTENT_MAP = 1 << 2;
        /// Prefer dedicated
        const PREFER_DEDICATED = 1 << 3;
        /// Never alias
        const NEVER_ALIAS = 1 << 4;
        /// Can be exported
        const EXPORTABLE = 1 << 5;
    }
}

/// Allocation result
#[derive(Clone, Copy, Debug, Default)]
pub struct Allocation {
    /// Memory block
    pub block: MemoryBlockHandle,
    /// Offset within block
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Mapped pointer (if mapped)
    pub mapped_ptr: u64,
    /// Memory location
    pub memory_location: MemoryLocation,
}

impl Allocation {
    /// Is mapped
    pub fn is_mapped(&self) -> bool {
        self.mapped_ptr != 0
    }

    /// End offset
    pub fn end_offset(&self) -> u64 {
        self.offset + self.size
    }
}

// ============================================================================
// Staging Buffer
// ============================================================================

/// Staging buffer create info
#[derive(Clone, Debug)]
pub struct StagingBufferCreateInfo {
    /// Name
    pub name: String,
    /// Size
    pub size: u64,
    /// Direction
    pub direction: StagingDirection,
    /// Ring buffer (reuse memory)
    pub ring_buffer: bool,
    /// Frames in flight (for ring buffer)
    pub frames_in_flight: u32,
}

impl StagingBufferCreateInfo {
    /// Creates new info
    pub fn new(size: u64, direction: StagingDirection) -> Self {
        Self {
            name: String::new(),
            size,
            direction,
            ring_buffer: false,
            frames_in_flight: 2,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Ring buffer mode
    pub fn ring_buffer(mut self, frames: u32) -> Self {
        self.ring_buffer = true;
        self.frames_in_flight = frames;
        self
    }

    /// Upload staging buffer
    pub fn upload(size: u64) -> Self {
        Self::new(size, StagingDirection::Upload)
    }

    /// Download staging buffer
    pub fn download(size: u64) -> Self {
        Self::new(size, StagingDirection::Download)
    }
}

impl Default for StagingBufferCreateInfo {
    fn default() -> Self {
        Self::upload(64 * 1024 * 1024)
    }
}

/// Staging direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum StagingDirection {
    /// CPU to GPU (upload)
    #[default]
    Upload = 0,
    /// GPU to CPU (download/readback)
    Download = 1,
}

/// Staging allocation
#[derive(Clone, Copy, Debug, Default)]
pub struct StagingAllocation {
    /// Staging buffer
    pub buffer: StagingBufferHandle,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Mapped pointer
    pub mapped_ptr: u64,
    /// Fence value (for ring buffer)
    pub fence_value: u64,
}

// ============================================================================
// Ring Buffer
// ============================================================================

/// Ring buffer create info
#[derive(Clone, Debug)]
pub struct RingBufferCreateInfo {
    /// Name
    pub name: String,
    /// Size
    pub size: u64,
    /// Memory location
    pub memory_location: MemoryLocation,
    /// Alignment
    pub alignment: u64,
    /// Frames in flight
    pub frames_in_flight: u32,
}

impl RingBufferCreateInfo {
    /// Creates new info
    pub fn new(size: u64) -> Self {
        Self {
            name: String::new(),
            size,
            memory_location: MemoryLocation::CpuToGpu,
            alignment: 256,
            frames_in_flight: 2,
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With alignment
    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// With frames in flight
    pub fn with_frames(mut self, frames: u32) -> Self {
        self.frames_in_flight = frames;
        self
    }

    /// Constant buffer ring
    pub fn constant_buffer(size: u64) -> Self {
        Self::new(size)
            .with_alignment(256)
    }

    /// Per-frame data ring
    pub fn per_frame(size: u64, frames: u32) -> Self {
        Self::new(size)
            .with_frames(frames)
    }
}

impl Default for RingBufferCreateInfo {
    fn default() -> Self {
        Self::new(4 * 1024 * 1024)
    }
}

/// Ring buffer allocation
#[derive(Clone, Copy, Debug, Default)]
pub struct RingAllocation {
    /// Ring buffer
    pub buffer: RingBufferHandle,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Frame index
    pub frame_index: u32,
    /// Mapped pointer
    pub mapped_ptr: u64,
}

// ============================================================================
// Memory Heap Info
// ============================================================================

/// Memory heap info
#[derive(Clone, Debug, Default)]
pub struct MemoryHeapInfo {
    /// Heap index
    pub index: u32,
    /// Total size
    pub total_size: u64,
    /// Available size
    pub available_size: u64,
    /// Heap flags
    pub flags: MemoryHeapFlags,
}

impl MemoryHeapInfo {
    /// Used size
    pub fn used_size(&self) -> u64 {
        self.total_size.saturating_sub(self.available_size)
    }

    /// Usage ratio (0.0 - 1.0)
    pub fn usage_ratio(&self) -> f32 {
        if self.total_size == 0 {
            return 0.0;
        }
        self.used_size() as f32 / self.total_size as f32
    }
}

bitflags::bitflags! {
    /// Memory heap flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct MemoryHeapFlags: u32 {
        /// None
        const NONE = 0;
        /// Device local
        const DEVICE_LOCAL = 1 << 0;
        /// Multi-instance
        const MULTI_INSTANCE = 1 << 1;
    }
}

/// Memory type info
#[derive(Clone, Debug, Default)]
pub struct MemoryTypeInfo {
    /// Type index
    pub index: u32,
    /// Heap index
    pub heap_index: u32,
    /// Property flags
    pub property_flags: MemoryPropertyFlags,
}

bitflags::bitflags! {
    /// Memory property flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct MemoryPropertyFlags: u32 {
        /// None
        const NONE = 0;
        /// Device local
        const DEVICE_LOCAL = 1 << 0;
        /// Host visible
        const HOST_VISIBLE = 1 << 1;
        /// Host coherent
        const HOST_COHERENT = 1 << 2;
        /// Host cached
        const HOST_CACHED = 1 << 3;
        /// Lazily allocated
        const LAZILY_ALLOCATED = 1 << 4;
        /// Protected
        const PROTECTED = 1 << 5;
    }
}

// ============================================================================
// Defragmentation
// ============================================================================

/// Defragmentation request
#[derive(Clone, Debug)]
pub struct DefragmentationRequest {
    /// Pool to defragment
    pub pool: MemoryPoolHandle,
    /// Max bytes to move
    pub max_bytes_to_move: u64,
    /// Max allocations to move
    pub max_allocations_to_move: u32,
    /// Flags
    pub flags: DefragmentationFlags,
}

impl DefragmentationRequest {
    /// Creates new request
    pub fn new(pool: MemoryPoolHandle) -> Self {
        Self {
            pool,
            max_bytes_to_move: u64::MAX,
            max_allocations_to_move: u32::MAX,
            flags: DefragmentationFlags::empty(),
        }
    }

    /// With limits
    pub fn with_limits(mut self, max_bytes: u64, max_allocations: u32) -> Self {
        self.max_bytes_to_move = max_bytes;
        self.max_allocations_to_move = max_allocations;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: DefragmentationFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// Incremental defragmentation
    pub fn incremental(pool: MemoryPoolHandle) -> Self {
        Self::new(pool)
            .with_limits(8 * 1024 * 1024, 10)
            .with_flags(DefragmentationFlags::INCREMENTAL)
    }

    /// Full defragmentation
    pub fn full(pool: MemoryPoolHandle) -> Self {
        Self::new(pool)
            .with_flags(DefragmentationFlags::FULL)
    }
}

impl Default for DefragmentationRequest {
    fn default() -> Self {
        Self::new(MemoryPoolHandle::NULL)
    }
}

bitflags::bitflags! {
    /// Defragmentation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct DefragmentationFlags: u32 {
        /// None
        const NONE = 0;
        /// Incremental (spread across frames)
        const INCREMENTAL = 1 << 0;
        /// Full defragmentation
        const FULL = 1 << 1;
        /// GPU-assisted
        const GPU_ASSISTED = 1 << 2;
    }
}

/// Defragmentation result
#[derive(Clone, Debug, Default)]
pub struct DefragmentationResult {
    /// Bytes freed
    pub bytes_freed: u64,
    /// Bytes moved
    pub bytes_moved: u64,
    /// Allocations moved
    pub allocations_moved: u32,
    /// Blocks freed
    pub blocks_freed: u32,
    /// Complete
    pub complete: bool,
}

// ============================================================================
// Statistics
// ============================================================================

/// Memory pool statistics
#[derive(Clone, Debug, Default)]
pub struct MemoryPoolStats {
    /// Pool handle
    pub pool: MemoryPoolHandle,
    /// Block count
    pub block_count: u32,
    /// Total size
    pub total_size: u64,
    /// Used size
    pub used_size: u64,
    /// Allocation count
    pub allocation_count: u32,
    /// Free region count
    pub free_region_count: u32,
    /// Largest free region
    pub largest_free_region: u64,
    /// Fragmentation (0.0 - 1.0)
    pub fragmentation: f32,
}

impl MemoryPoolStats {
    /// Available size
    pub fn available_size(&self) -> u64 {
        self.total_size.saturating_sub(self.used_size)
    }

    /// Usage ratio (0.0 - 1.0)
    pub fn usage_ratio(&self) -> f32 {
        if self.total_size == 0 {
            return 0.0;
        }
        self.used_size as f32 / self.total_size as f32
    }

    /// Average allocation size
    pub fn avg_allocation_size(&self) -> u64 {
        if self.allocation_count == 0 {
            return 0;
        }
        self.used_size / self.allocation_count as u64
    }
}

/// Global memory statistics
#[derive(Clone, Debug, Default)]
pub struct GlobalMemoryStats {
    /// Total allocated
    pub total_allocated: u64,
    /// Total used
    pub total_used: u64,
    /// Total pools
    pub pool_count: u32,
    /// Total blocks
    pub block_count: u32,
    /// Total allocations
    pub allocation_count: u32,
    /// Device local used
    pub device_local_used: u64,
    /// Host visible used
    pub host_visible_used: u64,
    /// Peak usage
    pub peak_usage: u64,
    /// Allocations this frame
    pub frame_allocations: u32,
    /// Frees this frame
    pub frame_frees: u32,
}

impl GlobalMemoryStats {
    /// Total available
    pub fn total_available(&self) -> u64 {
        self.total_allocated.saturating_sub(self.total_used)
    }

    /// Overall usage ratio
    pub fn usage_ratio(&self) -> f32 {
        if self.total_allocated == 0 {
            return 0.0;
        }
        self.total_used as f32 / self.total_allocated as f32
    }
}
