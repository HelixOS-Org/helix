//! GPU Allocator Types for Lumina
//!
//! This module provides GPU memory allocation infrastructure including
//! memory pools, allocation strategies, and defragmentation.

extern crate alloc;

use alloc::vec::Vec;

// ============================================================================
// Allocator Handles
// ============================================================================

/// GPU allocator handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuAllocatorHandle(pub u64);

impl GpuAllocatorHandle {
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

impl Default for GpuAllocatorHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// GPU allocation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuAllocationHandle(pub u64);

impl GpuAllocationHandle {
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

impl Default for GpuAllocationHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Memory heap handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MemoryHeapHandle(pub u64);

impl MemoryHeapHandle {
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

impl Default for MemoryHeapHandle {
    fn default() -> Self {
        Self::NULL
    }
}

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

// ============================================================================
// Memory Types
// ============================================================================

/// GPU memory type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum GpuMemoryType {
    /// Device local (GPU only)
    #[default]
    DeviceLocal = 0,
    /// Host visible (CPU mappable)
    HostVisible = 1,
    /// Host coherent (no flush needed)
    HostCoherent = 2,
    /// Host cached (cached reads)
    HostCached = 3,
    /// Lazily allocated
    LazilyAllocated = 4,
}

/// Memory property flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MemoryPropertyFlags(pub u32);

impl MemoryPropertyFlags {
    /// None
    pub const NONE: Self = Self(0);
    /// Device local
    pub const DEVICE_LOCAL: Self = Self(1 << 0);
    /// Host visible
    pub const HOST_VISIBLE: Self = Self(1 << 1);
    /// Host coherent
    pub const HOST_COHERENT: Self = Self(1 << 2);
    /// Host cached
    pub const HOST_CACHED: Self = Self(1 << 3);
    /// Lazily allocated
    pub const LAZILY_ALLOCATED: Self = Self(1 << 4);
    /// Protected
    pub const PROTECTED: Self = Self(1 << 5);

    /// Has property
    pub const fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Is device only
    pub fn is_device_only(&self) -> bool {
        self.has(Self::DEVICE_LOCAL) && !self.has(Self::HOST_VISIBLE)
    }

    /// Is mappable
    pub fn is_mappable(&self) -> bool {
        self.has(Self::HOST_VISIBLE)
    }
}

impl Default for MemoryPropertyFlags {
    fn default() -> Self {
        Self::DEVICE_LOCAL
    }
}

impl core::ops::BitOr for MemoryPropertyFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for MemoryPropertyFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

// ============================================================================
// Allocator Settings
// ============================================================================

/// Allocator create info
#[derive(Clone, Debug)]
pub struct AllocatorCreateInfo {
    /// Device memory budget
    pub device_budget: u64,
    /// Host memory budget
    pub host_budget: u64,
    /// Block size (for sub-allocation)
    pub block_size: u64,
    /// Allocation strategy
    pub strategy: AllocationStrategy,
    /// Enable defragmentation
    pub defragmentation: bool,
    /// Small allocation threshold
    pub small_allocation_threshold: u64,
}

impl AllocatorCreateInfo {
    /// Creates info
    pub fn new() -> Self {
        Self {
            device_budget: 4 * 1024 * 1024 * 1024, // 4 GB
            host_budget: 1 * 1024 * 1024 * 1024,   // 1 GB
            block_size: 256 * 1024 * 1024,         // 256 MB blocks
            strategy: AllocationStrategy::BestFit,
            defragmentation: true,
            small_allocation_threshold: 64 * 1024, // 64 KB
        }
    }

    /// With device budget
    pub fn with_device_budget(mut self, bytes: u64) -> Self {
        self.device_budget = bytes;
        self
    }

    /// With block size
    pub fn with_block_size(mut self, bytes: u64) -> Self {
        self.block_size = bytes;
        self
    }

    /// With strategy
    pub fn with_strategy(mut self, strategy: AllocationStrategy) -> Self {
        self.strategy = strategy;
        self
    }
}

impl Default for AllocatorCreateInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Allocation strategy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AllocationStrategy {
    /// First fit
    FirstFit = 0,
    /// Best fit (less fragmentation)
    #[default]
    BestFit = 1,
    /// Worst fit
    WorstFit = 2,
    /// Buddy allocator
    Buddy = 3,
    /// Linear (bump allocator)
    Linear = 4,
    /// Ring buffer
    Ring = 5,
}

// ============================================================================
// Allocation
// ============================================================================

/// Allocation create info
#[derive(Clone, Debug)]
pub struct AllocationCreateInfo {
    /// Size in bytes
    pub size: u64,
    /// Alignment
    pub alignment: u64,
    /// Memory type requirements
    pub memory_type: MemoryPropertyFlags,
    /// Usage hint
    pub usage: AllocationUsage,
    /// Dedicated allocation
    pub dedicated: bool,
    /// Name (for debugging)
    pub name: Option<&'static str>,
}

impl AllocationCreateInfo {
    /// Creates info
    pub fn new(size: u64) -> Self {
        Self {
            size,
            alignment: 256,
            memory_type: MemoryPropertyFlags::DEVICE_LOCAL,
            usage: AllocationUsage::GpuOnly,
            dedicated: false,
            name: None,
        }
    }

    /// For buffer
    pub fn for_buffer(size: u64, usage: AllocationUsage) -> Self {
        Self {
            size,
            usage,
            ..Self::new(size)
        }
    }

    /// For texture
    pub fn for_texture(size: u64) -> Self {
        Self {
            alignment: 4096, // Page aligned
            ..Self::new(size)
        }
    }

    /// With alignment
    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Dedicated
    pub fn dedicated(mut self) -> Self {
        self.dedicated = true;
        self
    }

    /// With name
    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }
}

impl Default for AllocationCreateInfo {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Allocation usage hint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AllocationUsage {
    /// GPU only (no CPU access)
    #[default]
    GpuOnly = 0,
    /// CPU to GPU (upload)
    CpuToGpu = 1,
    /// GPU to CPU (readback)
    GpuToCpu = 2,
    /// CPU only (staging)
    CpuOnly = 3,
    /// GPU lazy (framebuffer)
    GpuLazy = 4,
}

impl AllocationUsage {
    /// Required memory properties
    pub fn required_properties(&self) -> MemoryPropertyFlags {
        match self {
            Self::GpuOnly => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::CpuToGpu => MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            Self::GpuToCpu => {
                MemoryPropertyFlags::HOST_VISIBLE
                    | MemoryPropertyFlags::HOST_COHERENT
                    | MemoryPropertyFlags::HOST_CACHED
            }
            Self::CpuOnly => MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            Self::GpuLazy => MemoryPropertyFlags::DEVICE_LOCAL | MemoryPropertyFlags::LAZILY_ALLOCATED,
        }
    }
}

/// Allocation info (result)
#[derive(Clone, Debug)]
pub struct AllocationInfo {
    /// Handle
    pub handle: GpuAllocationHandle,
    /// Offset in heap
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Alignment
    pub alignment: u64,
    /// Memory type index
    pub memory_type: u32,
    /// Heap handle
    pub heap: MemoryHeapHandle,
    /// Pool handle (if sub-allocated)
    pub pool: MemoryPoolHandle,
    /// Mapped pointer (if mappable)
    pub mapped_ptr: u64,
}

impl AllocationInfo {
    /// Creates info
    pub fn new(handle: GpuAllocationHandle, offset: u64, size: u64) -> Self {
        Self {
            handle,
            offset,
            size,
            alignment: 1,
            memory_type: 0,
            heap: MemoryHeapHandle::NULL,
            pool: MemoryPoolHandle::NULL,
            mapped_ptr: 0,
        }
    }

    /// Is mapped
    pub fn is_mapped(&self) -> bool {
        self.mapped_ptr != 0
    }
}

impl Default for AllocationInfo {
    fn default() -> Self {
        Self::new(GpuAllocationHandle::NULL, 0, 0)
    }
}

// ============================================================================
// Memory Heap
// ============================================================================

/// Memory heap create info
#[derive(Clone, Debug)]
pub struct MemoryHeapCreateInfo {
    /// Size
    pub size: u64,
    /// Properties
    pub properties: MemoryPropertyFlags,
    /// Is device local
    pub device_local: bool,
}

impl MemoryHeapCreateInfo {
    /// Creates info
    pub fn new(size: u64, properties: MemoryPropertyFlags) -> Self {
        Self {
            size,
            properties,
            device_local: properties.has(MemoryPropertyFlags::DEVICE_LOCAL),
        }
    }

    /// Device heap
    pub fn device(size: u64) -> Self {
        Self::new(size, MemoryPropertyFlags::DEVICE_LOCAL)
    }

    /// Host heap
    pub fn host(size: u64) -> Self {
        Self::new(
            size,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        )
    }
}

impl Default for MemoryHeapCreateInfo {
    fn default() -> Self {
        Self::device(1024 * 1024 * 1024)
    }
}

/// Memory heap info
#[derive(Clone, Debug)]
pub struct MemoryHeapInfo {
    /// Handle
    pub handle: MemoryHeapHandle,
    /// Total size
    pub size: u64,
    /// Used size
    pub used: u64,
    /// Properties
    pub properties: MemoryPropertyFlags,
    /// Allocation count
    pub allocation_count: u32,
}

impl MemoryHeapInfo {
    /// Available size
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.used)
    }

    /// Usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            (self.used as f64 / self.size as f64 * 100.0) as f32
        }
    }
}

impl Default for MemoryHeapInfo {
    fn default() -> Self {
        Self {
            handle: MemoryHeapHandle::NULL,
            size: 0,
            used: 0,
            properties: MemoryPropertyFlags::NONE,
            allocation_count: 0,
        }
    }
}

// ============================================================================
// Memory Pool
// ============================================================================

/// Memory pool create info
#[derive(Clone, Debug)]
pub struct MemoryPoolCreateInfo {
    /// Block size
    pub block_size: u64,
    /// Min block count
    pub min_block_count: u32,
    /// Max block count
    pub max_block_count: u32,
    /// Memory type
    pub memory_type: u32,
    /// Linear allocator
    pub linear: bool,
    /// Name
    pub name: Option<&'static str>,
}

impl MemoryPoolCreateInfo {
    /// Creates info
    pub fn new(block_size: u64) -> Self {
        Self {
            block_size,
            min_block_count: 1,
            max_block_count: 16,
            memory_type: 0,
            linear: false,
            name: None,
        }
    }

    /// Linear pool (for staging)
    pub fn linear(block_size: u64) -> Self {
        Self {
            linear: true,
            ..Self::new(block_size)
        }
    }

    /// With memory type
    pub fn with_memory_type(mut self, memory_type: u32) -> Self {
        self.memory_type = memory_type;
        self
    }
}

impl Default for MemoryPoolCreateInfo {
    fn default() -> Self {
        Self::new(64 * 1024 * 1024)
    }
}

/// Memory pool info
#[derive(Clone, Debug)]
pub struct MemoryPoolInfo {
    /// Handle
    pub handle: MemoryPoolHandle,
    /// Block size
    pub block_size: u64,
    /// Block count
    pub block_count: u32,
    /// Total size
    pub total_size: u64,
    /// Used size
    pub used_size: u64,
    /// Allocation count
    pub allocation_count: u32,
}

impl MemoryPoolInfo {
    /// Available size
    pub fn available(&self) -> u64 {
        self.total_size.saturating_sub(self.used_size)
    }

    /// Fragmentation (0-1)
    pub fn fragmentation(&self) -> f32 {
        // Simplified fragmentation metric
        if self.allocation_count <= 1 || self.used_size == 0 {
            0.0
        } else {
            let avg_alloc_size = self.used_size as f64 / self.allocation_count as f64;
            let theoretical_allocs = self.total_size as f64 / avg_alloc_size;
            let max_possible = (theoretical_allocs / self.allocation_count as f64).min(1.0);
            (1.0 - max_possible) as f32
        }
    }
}

impl Default for MemoryPoolInfo {
    fn default() -> Self {
        Self {
            handle: MemoryPoolHandle::NULL,
            block_size: 0,
            block_count: 0,
            total_size: 0,
            used_size: 0,
            allocation_count: 0,
        }
    }
}

// ============================================================================
// Free List
// ============================================================================

/// Free block
#[derive(Clone, Copy, Debug)]
pub struct FreeBlock {
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
}

impl FreeBlock {
    /// Creates block
    pub fn new(offset: u64, size: u64) -> Self {
        Self { offset, size }
    }

    /// Can fit allocation
    pub fn can_fit(&self, size: u64, alignment: u64) -> bool {
        let aligned_offset = (self.offset + alignment - 1) & !(alignment - 1);
        let padding = aligned_offset - self.offset;
        self.size >= size + padding
    }

    /// Aligned offset
    pub fn aligned_offset(&self, alignment: u64) -> u64 {
        (self.offset + alignment - 1) & !(alignment - 1)
    }
}

/// Free list
#[derive(Clone, Debug)]
pub struct FreeList {
    /// Free blocks
    pub blocks: Vec<FreeBlock>,
    /// Total free size
    pub total_free: u64,
    /// Strategy
    pub strategy: AllocationStrategy,
}

impl FreeList {
    /// Creates free list
    pub fn new(size: u64, strategy: AllocationStrategy) -> Self {
        Self {
            blocks: alloc::vec![FreeBlock::new(0, size)],
            total_free: size,
            strategy,
        }
    }

    /// Find block for allocation
    pub fn find_block(&self, size: u64, alignment: u64) -> Option<usize> {
        match self.strategy {
            AllocationStrategy::FirstFit => {
                self.blocks
                    .iter()
                    .position(|b| b.can_fit(size, alignment))
            }
            AllocationStrategy::BestFit => {
                let mut best: Option<(usize, u64)> = None;
                for (i, block) in self.blocks.iter().enumerate() {
                    if block.can_fit(size, alignment) {
                        let waste = block.size - size;
                        if best.is_none() || waste < best.unwrap().1 {
                            best = Some((i, waste));
                        }
                    }
                }
                best.map(|(i, _)| i)
            }
            AllocationStrategy::WorstFit => {
                let mut best: Option<(usize, u64)> = None;
                for (i, block) in self.blocks.iter().enumerate() {
                    if block.can_fit(size, alignment) {
                        if best.is_none() || block.size > best.unwrap().1 {
                            best = Some((i, block.size));
                        }
                    }
                }
                best.map(|(i, _)| i)
            }
            _ => self.blocks.iter().position(|b| b.can_fit(size, alignment)),
        }
    }

    /// Allocate from block
    pub fn allocate(&mut self, index: usize, size: u64, alignment: u64) -> Option<u64> {
        let block = self.blocks.get(index)?;
        let aligned_offset = block.aligned_offset(alignment);
        let padding = aligned_offset - block.offset;
        let total_size = size + padding;

        if block.size < total_size {
            return None;
        }

        let block = self.blocks.remove(index);

        // Add front padding as free block
        if padding > 0 {
            self.blocks.push(FreeBlock::new(block.offset, padding));
        }

        // Add remaining as free block
        let remaining = block.size - total_size;
        if remaining > 0 {
            self.blocks.push(FreeBlock::new(aligned_offset + size, remaining));
        }

        self.total_free -= total_size;
        Some(aligned_offset)
    }

    /// Free allocation
    pub fn free(&mut self, offset: u64, size: u64) {
        self.blocks.push(FreeBlock::new(offset, size));
        self.total_free += size;
        self.coalesce();
    }

    /// Coalesce adjacent blocks
    pub fn coalesce(&mut self) {
        if self.blocks.len() < 2 {
            return;
        }

        self.blocks.sort_by_key(|b| b.offset);

        let mut i = 0;
        while i + 1 < self.blocks.len() {
            if self.blocks[i].offset + self.blocks[i].size == self.blocks[i + 1].offset {
                self.blocks[i].size += self.blocks[i + 1].size;
                self.blocks.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    /// Fragmentation (0-1)
    pub fn fragmentation(&self) -> f32 {
        if self.blocks.is_empty() || self.total_free == 0 {
            return 0.0;
        }
        let largest = self.blocks.iter().map(|b| b.size).max().unwrap_or(0);
        1.0 - (largest as f64 / self.total_free as f64) as f32
    }
}

impl Default for FreeList {
    fn default() -> Self {
        Self::new(0, AllocationStrategy::BestFit)
    }
}

// ============================================================================
// Defragmentation
// ============================================================================

/// Defragmentation settings
#[derive(Clone, Debug)]
pub struct DefragmentationSettings {
    /// Max moves per frame
    pub max_moves: u32,
    /// Max bytes per frame
    pub max_bytes: u64,
    /// Priority threshold
    pub priority_threshold: f32,
    /// Algorithm
    pub algorithm: DefragAlgorithm,
}

impl DefragmentationSettings {
    /// Creates settings
    pub fn new() -> Self {
        Self {
            max_moves: 16,
            max_bytes: 64 * 1024 * 1024, // 64 MB
            priority_threshold: 0.5,
            algorithm: DefragAlgorithm::Full,
        }
    }

    /// Incremental
    pub fn incremental() -> Self {
        Self {
            max_moves: 4,
            max_bytes: 16 * 1024 * 1024,
            algorithm: DefragAlgorithm::Incremental,
            ..Self::new()
        }
    }
}

impl Default for DefragmentationSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// Defragmentation algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum DefragAlgorithm {
    /// Full defragmentation
    #[default]
    Full = 0,
    /// Incremental (spread over frames)
    Incremental = 1,
    /// Fast (only large blocks)
    Fast = 2,
}

/// Defragmentation move
#[derive(Clone, Copy, Debug)]
pub struct DefragMove {
    /// Source allocation
    pub src: GpuAllocationHandle,
    /// Source offset
    pub src_offset: u64,
    /// Destination offset
    pub dst_offset: u64,
    /// Size
    pub size: u64,
}

/// Defragmentation result
#[derive(Clone, Debug, Default)]
pub struct DefragmentationResult {
    /// Moves performed
    pub moves: Vec<DefragMove>,
    /// Bytes moved
    pub bytes_moved: u64,
    /// Freed blocks
    pub blocks_freed: u32,
    /// Space reclaimed
    pub space_reclaimed: u64,
}

// ============================================================================
// Statistics
// ============================================================================

/// Allocator statistics
#[derive(Clone, Debug, Default)]
pub struct AllocatorStats {
    /// Total allocations
    pub allocation_count: u32,
    /// Total allocated bytes
    pub allocated_bytes: u64,
    /// Total unused bytes
    pub unused_bytes: u64,
    /// Block count
    pub block_count: u32,
    /// Peak usage
    pub peak_usage: u64,
    /// Fragmentation (0-1)
    pub fragmentation: f32,
    /// Dedicated allocations
    pub dedicated_count: u32,
    /// Sub-allocations
    pub suballocation_count: u32,
}

impl AllocatorStats {
    /// Usage percentage
    pub fn usage_percent(&self) -> f32 {
        let total = self.allocated_bytes + self.unused_bytes;
        if total == 0 {
            0.0
        } else {
            (self.allocated_bytes as f64 / total as f64 * 100.0) as f32
        }
    }
}
