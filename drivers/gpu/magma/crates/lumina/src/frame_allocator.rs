//! Frame Allocator Types for Lumina
//!
//! This module provides per-frame transient allocation
//! infrastructure for efficient temporary resource management.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Allocator Handles
// ============================================================================

/// Frame allocator handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FrameAllocatorHandle(pub u64);

impl FrameAllocatorHandle {
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

impl Default for FrameAllocatorHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Frame allocation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct FrameAllocationHandle(pub u64);

impl FrameAllocationHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for FrameAllocationHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Staging allocation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct StagingAllocationHandle(pub u64);

impl StagingAllocationHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for StagingAllocationHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Scratch buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ScratchBufferHandle(pub u64);

impl ScratchBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for ScratchBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Frame Allocator Creation
// ============================================================================

/// Frame allocator create info
#[derive(Clone, Debug)]
pub struct FrameAllocatorCreateInfo {
    /// Name
    pub name: String,
    /// Frame count (for buffering)
    pub frame_count: u32,
    /// Per-frame budget (bytes)
    pub per_frame_budget: u64,
    /// Staging buffer size
    pub staging_size: u64,
    /// Scratch buffer size
    pub scratch_size: u64,
    /// Allocation strategy
    pub strategy: AllocationStrategy,
    /// Features
    pub features: FrameAllocatorFeatures,
}

impl FrameAllocatorCreateInfo {
    /// Creates new info
    pub fn new(per_frame_budget: u64) -> Self {
        Self {
            name: String::new(),
            frame_count: 3,  // Triple buffering
            per_frame_budget,
            staging_size: 64 * 1024 * 1024,  // 64MB
            scratch_size: 16 * 1024 * 1024,  // 16MB
            strategy: AllocationStrategy::Linear,
            features: FrameAllocatorFeatures::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With frame count
    pub fn with_frame_count(mut self, count: u32) -> Self {
        self.frame_count = count;
        self
    }

    /// With staging size
    pub fn with_staging(mut self, size: u64) -> Self {
        self.staging_size = size;
        self
    }

    /// With scratch size
    pub fn with_scratch(mut self, size: u64) -> Self {
        self.scratch_size = size;
        self
    }

    /// With strategy
    pub fn with_strategy(mut self, strategy: AllocationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// With features
    pub fn with_features(mut self, features: FrameAllocatorFeatures) -> Self {
        self.features |= features;
        self
    }

    /// Standard preset (256MB per frame)
    pub fn standard() -> Self {
        Self::new(256 * 1024 * 1024)
    }

    /// Small preset (64MB per frame)
    pub fn small() -> Self {
        Self::new(64 * 1024 * 1024)
            .with_staging(16 * 1024 * 1024)
            .with_scratch(4 * 1024 * 1024)
    }

    /// Large preset (1GB per frame)
    pub fn large() -> Self {
        Self::new(1024 * 1024 * 1024)
            .with_staging(256 * 1024 * 1024)
            .with_scratch(64 * 1024 * 1024)
    }

    /// Debug preset (with validation)
    pub fn debug() -> Self {
        Self::standard()
            .with_features(FrameAllocatorFeatures::VALIDATION | FrameAllocatorFeatures::STATISTICS)
    }
}

impl Default for FrameAllocatorCreateInfo {
    fn default() -> Self {
        Self::standard()
    }
}

/// Allocation strategy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AllocationStrategy {
    /// Linear (bump allocator)
    #[default]
    Linear = 0,
    /// Ring buffer
    Ring = 1,
    /// Pool (fixed size blocks)
    Pool = 2,
    /// TLSF
    Tlsf = 3,
}

bitflags::bitflags! {
    /// Frame allocator features
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct FrameAllocatorFeatures: u32 {
        /// None
        const NONE = 0;
        /// Validation
        const VALIDATION = 1 << 0;
        /// Statistics
        const STATISTICS = 1 << 1;
        /// Auto resize
        const AUTO_RESIZE = 1 << 2;
        /// Thread safe
        const THREAD_SAFE = 1 << 3;
        /// GPU visible
        const GPU_VISIBLE = 1 << 4;
        /// Host coherent
        const HOST_COHERENT = 1 << 5;
    }
}

// ============================================================================
// Allocation Requests
// ============================================================================

/// Frame allocation request
#[derive(Clone, Debug)]
pub struct FrameAllocationRequest {
    /// Size (bytes)
    pub size: u64,
    /// Alignment
    pub alignment: u32,
    /// Usage
    pub usage: AllocationUsage,
    /// Flags
    pub flags: AllocationFlags,
    /// Debug name
    pub debug_name: String,
}

impl FrameAllocationRequest {
    /// Creates new request
    pub fn new(size: u64) -> Self {
        Self {
            size,
            alignment: 16,
            usage: AllocationUsage::General,
            flags: AllocationFlags::empty(),
            debug_name: String::new(),
        }
    }

    /// With alignment
    pub fn with_alignment(mut self, alignment: u32) -> Self {
        self.alignment = alignment;
        self
    }

    /// With usage
    pub fn with_usage(mut self, usage: AllocationUsage) -> Self {
        self.usage = usage;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: AllocationFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// With debug name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.debug_name = name.into();
        self
    }

    /// Uniform buffer allocation
    pub fn uniform(size: u64) -> Self {
        Self::new(size)
            .with_alignment(256)  // Typical UBO alignment
            .with_usage(AllocationUsage::UniformBuffer)
    }

    /// Storage buffer allocation
    pub fn storage(size: u64) -> Self {
        Self::new(size)
            .with_alignment(16)
            .with_usage(AllocationUsage::StorageBuffer)
    }

    /// Vertex buffer allocation
    pub fn vertex(size: u64) -> Self {
        Self::new(size)
            .with_usage(AllocationUsage::VertexBuffer)
    }

    /// Index buffer allocation
    pub fn index(size: u64) -> Self {
        Self::new(size)
            .with_alignment(4)
            .with_usage(AllocationUsage::IndexBuffer)
    }

    /// Staging allocation
    pub fn staging(size: u64) -> Self {
        Self::new(size)
            .with_usage(AllocationUsage::Staging)
            .with_flags(AllocationFlags::HOST_VISIBLE | AllocationFlags::HOST_COHERENT)
    }
}

impl Default for FrameAllocationRequest {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Allocation usage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AllocationUsage {
    /// General purpose
    #[default]
    General = 0,
    /// Uniform buffer
    UniformBuffer = 1,
    /// Storage buffer
    StorageBuffer = 2,
    /// Vertex buffer
    VertexBuffer = 3,
    /// Index buffer
    IndexBuffer = 4,
    /// Staging
    Staging = 5,
    /// Scratch (temporary)
    Scratch = 6,
    /// Indirect arguments
    Indirect = 7,
}

bitflags::bitflags! {
    /// Allocation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct AllocationFlags: u32 {
        /// None
        const NONE = 0;
        /// Host visible
        const HOST_VISIBLE = 1 << 0;
        /// Host coherent
        const HOST_COHERENT = 1 << 1;
        /// Device local
        const DEVICE_LOCAL = 1 << 2;
        /// Persistently mapped
        const PERSISTENT = 1 << 3;
        /// Zero initialized
        const ZERO_INIT = 1 << 4;
        /// Allow grow
        const ALLOW_GROW = 1 << 5;
    }
}

// ============================================================================
// Frame Allocation Result
// ============================================================================

/// Frame allocation
#[derive(Clone, Debug, Default)]
pub struct FrameAllocation {
    /// Handle
    pub handle: FrameAllocationHandle,
    /// Buffer handle
    pub buffer: u64,
    /// Offset in buffer
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Mapped pointer (if applicable)
    pub mapped_ptr: u64,
    /// Frame index
    pub frame_index: u32,
}

impl FrameAllocation {
    /// Is valid
    pub fn is_valid(&self) -> bool {
        !self.handle.0 == 0 && self.size > 0
    }

    /// End offset
    pub fn end_offset(&self) -> u64 {
        self.offset + self.size
    }

    /// GPU address (if applicable)
    pub fn gpu_address(&self, buffer_base: u64) -> u64 {
        buffer_base + self.offset
    }
}

/// Staging allocation
#[derive(Clone, Debug, Default)]
pub struct StagingAllocation {
    /// Handle
    pub handle: StagingAllocationHandle,
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Mapped pointer
    pub mapped_ptr: u64,
}

impl StagingAllocation {
    /// Is valid
    pub fn is_valid(&self) -> bool {
        self.size > 0 && self.mapped_ptr != 0
    }
}

/// Scratch allocation
#[derive(Clone, Debug, Default)]
pub struct ScratchAllocation {
    /// Handle
    pub handle: ScratchBufferHandle,
    /// Buffer
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
    /// GPU address
    pub gpu_address: u64,
}

// ============================================================================
// Ring Buffer
// ============================================================================

/// Ring buffer create info
#[derive(Clone, Debug)]
pub struct RingBufferCreateInfo {
    /// Name
    pub name: String,
    /// Size (bytes)
    pub size: u64,
    /// Usage
    pub usage: RingBufferUsage,
    /// Frame count
    pub frame_count: u32,
    /// Flags
    pub flags: RingBufferFlags,
}

impl RingBufferCreateInfo {
    /// Creates new info
    pub fn new(size: u64) -> Self {
        Self {
            name: String::new(),
            size,
            usage: RingBufferUsage::Uniform,
            frame_count: 3,
            flags: RingBufferFlags::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With usage
    pub fn with_usage(mut self, usage: RingBufferUsage) -> Self {
        self.usage = usage;
        self
    }

    /// With frame count
    pub fn with_frame_count(mut self, count: u32) -> Self {
        self.frame_count = count;
        self
    }

    /// Uniform preset
    pub fn uniform(size: u64) -> Self {
        Self::new(size)
            .with_usage(RingBufferUsage::Uniform)
    }

    /// Staging preset
    pub fn staging(size: u64) -> Self {
        Self::new(size)
            .with_usage(RingBufferUsage::Staging)
    }
}

impl Default for RingBufferCreateInfo {
    fn default() -> Self {
        Self::new(16 * 1024 * 1024)  // 16MB
    }
}

/// Ring buffer usage
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum RingBufferUsage {
    /// Uniform data
    #[default]
    Uniform = 0,
    /// Staging uploads
    Staging = 1,
    /// Readback
    Readback = 2,
    /// Scratch
    Scratch = 3,
}

bitflags::bitflags! {
    /// Ring buffer flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct RingBufferFlags: u32 {
        /// None
        const NONE = 0;
        /// Persistent mapping
        const PERSISTENT = 1 << 0;
        /// Coherent
        const COHERENT = 1 << 1;
        /// Allow wrap
        const ALLOW_WRAP = 1 << 2;
    }
}

// ============================================================================
// GPU Data Structures
// ============================================================================

/// GPU allocation info (for bindless)
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuAllocationInfo {
    /// Buffer index
    pub buffer_index: u32,
    /// Offset
    pub offset: u32,
    /// Size
    pub size: u32,
    /// Flags
    pub flags: u32,
}

/// GPU buffer descriptor
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuBufferDescriptor {
    /// GPU address
    pub address: u64,
    /// Size
    pub size: u64,
}

/// GPU frame data
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuFrameData {
    /// Frame index
    pub frame_index: u32,
    /// Uniform buffer offset
    pub uniform_offset: u32,
    /// Storage buffer offset
    pub storage_offset: u32,
    /// Scratch buffer offset
    pub scratch_offset: u32,
}

// ============================================================================
// Statistics
// ============================================================================

/// Frame allocator statistics
#[derive(Clone, Debug, Default)]
pub struct FrameAllocatorStats {
    /// Current frame index
    pub frame_index: u32,
    /// Allocations this frame
    pub allocations_this_frame: u32,
    /// Bytes allocated this frame
    pub bytes_this_frame: u64,
    /// Total allocations
    pub total_allocations: u64,
    /// Total bytes allocated
    pub total_bytes: u64,
    /// Peak usage per frame
    pub peak_per_frame: u64,
    /// Budget per frame
    pub budget_per_frame: u64,
    /// Staging used
    pub staging_used: u64,
    /// Staging capacity
    pub staging_capacity: u64,
    /// Scratch used
    pub scratch_used: u64,
    /// Scratch capacity
    pub scratch_capacity: u64,
}

impl FrameAllocatorStats {
    /// Usage ratio for current frame
    pub fn frame_usage_ratio(&self) -> f32 {
        if self.budget_per_frame == 0 {
            return 0.0;
        }
        self.bytes_this_frame as f32 / self.budget_per_frame as f32
    }

    /// Peak usage ratio
    pub fn peak_usage_ratio(&self) -> f32 {
        if self.budget_per_frame == 0 {
            return 0.0;
        }
        self.peak_per_frame as f32 / self.budget_per_frame as f32
    }

    /// Staging usage ratio
    pub fn staging_usage_ratio(&self) -> f32 {
        if self.staging_capacity == 0 {
            return 0.0;
        }
        self.staging_used as f32 / self.staging_capacity as f32
    }

    /// Scratch usage ratio
    pub fn scratch_usage_ratio(&self) -> f32 {
        if self.scratch_capacity == 0 {
            return 0.0;
        }
        self.scratch_used as f32 / self.scratch_capacity as f32
    }

    /// Is over budget
    pub fn is_over_budget(&self) -> bool {
        self.bytes_this_frame > self.budget_per_frame
    }
}

/// Per-usage statistics
#[derive(Clone, Debug, Default)]
pub struct UsageStats {
    /// Usage type
    pub usage: AllocationUsage,
    /// Allocation count
    pub allocation_count: u32,
    /// Total bytes
    pub total_bytes: u64,
    /// Average size
    pub avg_size: u64,
    /// Max size
    pub max_size: u64,
}
