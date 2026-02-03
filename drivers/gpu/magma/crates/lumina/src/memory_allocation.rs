//! Memory Allocation Types for Lumina
//!
//! This module provides memory allocation strategies, memory types,
//! and memory management structures.

use core::ptr::NonNull;

// ============================================================================
// Memory Handle
// ============================================================================

/// Device memory handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MemoryHandle(pub u64);

impl MemoryHandle {
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

impl Default for MemoryHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// Memory Properties
// ============================================================================

/// Memory property flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MemoryPropertyFlags(pub u32);

impl MemoryPropertyFlags {
    /// No properties
    pub const NONE: Self = Self(0);
    /// Device local (GPU memory)
    pub const DEVICE_LOCAL: Self = Self(1 << 0);
    /// Host visible (CPU can map)
    pub const HOST_VISIBLE: Self = Self(1 << 1);
    /// Host coherent (no manual flush)
    pub const HOST_COHERENT: Self = Self(1 << 2);
    /// Host cached (faster CPU reads)
    pub const HOST_CACHED: Self = Self(1 << 3);
    /// Lazily allocated
    pub const LAZILY_ALLOCATED: Self = Self(1 << 4);
    /// Protected
    pub const PROTECTED: Self = Self(1 << 5);
    /// Device coherent AMD
    pub const DEVICE_COHERENT_AMD: Self = Self(1 << 6);
    /// Device uncached AMD
    pub const DEVICE_UNCACHED_AMD: Self = Self(1 << 7);
    /// RDMA capable NV
    pub const RDMA_CAPABLE_NV: Self = Self(1 << 8);

    /// GPU only (optimal for textures, render targets)
    pub const GPU_ONLY: Self = Self(Self::DEVICE_LOCAL.0);

    /// CPU to GPU (staging, dynamic uniforms)
    pub const CPU_TO_GPU: Self = Self(
        Self::HOST_VISIBLE.0 | Self::HOST_COHERENT.0 | Self::DEVICE_LOCAL.0,
    );

    /// GPU to CPU (readback)
    pub const GPU_TO_CPU: Self = Self(
        Self::HOST_VISIBLE.0 | Self::HOST_COHERENT.0 | Self::HOST_CACHED.0,
    );

    /// CPU only (staging buffers)
    pub const CPU_ONLY: Self = Self(Self::HOST_VISIBLE.0 | Self::HOST_COHERENT.0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersection
    #[inline]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Is empty
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Is device local
    #[inline]
    pub const fn is_device_local(&self) -> bool {
        self.contains(Self::DEVICE_LOCAL)
    }

    /// Is host visible
    #[inline]
    pub const fn is_host_visible(&self) -> bool {
        self.contains(Self::HOST_VISIBLE)
    }

    /// Is host coherent
    #[inline]
    pub const fn is_host_coherent(&self) -> bool {
        self.contains(Self::HOST_COHERENT)
    }

    /// Is host cached
    #[inline]
    pub const fn is_host_cached(&self) -> bool {
        self.contains(Self::HOST_CACHED)
    }

    /// Is mappable
    #[inline]
    pub const fn is_mappable(&self) -> bool {
        self.is_host_visible()
    }
}

// ============================================================================
// Memory Heap Properties
// ============================================================================

/// Memory heap flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MemoryHeapFlags(pub u32);

impl MemoryHeapFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Device local heap
    pub const DEVICE_LOCAL: Self = Self(1 << 0);
    /// Multi-instance heap
    pub const MULTI_INSTANCE: Self = Self(1 << 1);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Memory heap
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryHeap {
    /// Size in bytes
    pub size: u64,
    /// Flags
    pub flags: MemoryHeapFlags,
}

impl MemoryHeap {
    /// Creates new heap
    #[inline]
    pub const fn new(size: u64, flags: MemoryHeapFlags) -> Self {
        Self { size, flags }
    }

    /// Size in megabytes
    #[inline]
    pub const fn size_mb(&self) -> u64 {
        self.size / (1024 * 1024)
    }

    /// Size in gigabytes
    #[inline]
    pub const fn size_gb(&self) -> u64 {
        self.size / (1024 * 1024 * 1024)
    }

    /// Is device local
    #[inline]
    pub const fn is_device_local(&self) -> bool {
        self.flags.contains(MemoryHeapFlags::DEVICE_LOCAL)
    }
}

// ============================================================================
// Memory Type
// ============================================================================

/// Memory type
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryType {
    /// Property flags
    pub property_flags: MemoryPropertyFlags,
    /// Heap index
    pub heap_index: u32,
}

impl MemoryType {
    /// Creates new type
    #[inline]
    pub const fn new(flags: MemoryPropertyFlags, heap_index: u32) -> Self {
        Self {
            property_flags: flags,
            heap_index,
        }
    }
}

// ============================================================================
// Physical Device Memory Properties
// ============================================================================

/// Physical device memory properties
#[derive(Clone, Debug)]
#[repr(C)]
pub struct PhysicalDeviceMemoryProperties {
    /// Memory types
    pub memory_types: [MemoryType; 32],
    /// Memory type count
    pub memory_type_count: u32,
    /// Memory heaps
    pub memory_heaps: [MemoryHeap; 16],
    /// Memory heap count
    pub memory_heap_count: u32,
}

impl PhysicalDeviceMemoryProperties {
    /// Maximum memory types
    pub const MAX_MEMORY_TYPES: usize = 32;
    /// Maximum memory heaps
    pub const MAX_MEMORY_HEAPS: usize = 16;

    /// Creates default properties
    #[inline]
    pub const fn new() -> Self {
        Self {
            memory_types: [MemoryType::new(MemoryPropertyFlags::NONE, 0); 32],
            memory_type_count: 0,
            memory_heaps: [MemoryHeap::new(0, MemoryHeapFlags::NONE); 16],
            memory_heap_count: 0,
        }
    }

    /// Find memory type
    #[inline]
    pub fn find_memory_type(
        &self,
        type_bits: u32,
        required: MemoryPropertyFlags,
    ) -> Option<u32> {
        for i in 0..self.memory_type_count {
            if (type_bits & (1 << i)) != 0
                && self.memory_types[i as usize].property_flags.contains(required)
            {
                return Some(i);
            }
        }
        None
    }

    /// Find memory type with preferred
    #[inline]
    pub fn find_memory_type_preferred(
        &self,
        type_bits: u32,
        required: MemoryPropertyFlags,
        preferred: MemoryPropertyFlags,
    ) -> Option<u32> {
        // Try with preferred first
        let combined = required.union(preferred);
        if let Some(index) = self.find_memory_type(type_bits, combined) {
            return Some(index);
        }
        // Fall back to required only
        self.find_memory_type(type_bits, required)
    }

    /// Total device local memory
    #[inline]
    pub fn total_device_local_memory(&self) -> u64 {
        let mut total = 0;
        for i in 0..self.memory_heap_count {
            if self.memory_heaps[i as usize].is_device_local() {
                total += self.memory_heaps[i as usize].size;
            }
        }
        total
    }

    /// Total host visible memory
    #[inline]
    pub fn total_host_visible_memory(&self) -> u64 {
        let mut total = 0;
        for i in 0..self.memory_type_count {
            if self.memory_types[i as usize].property_flags.is_host_visible() {
                let heap_idx = self.memory_types[i as usize].heap_index;
                total += self.memory_heaps[heap_idx as usize].size;
            }
        }
        total
    }
}

impl Default for PhysicalDeviceMemoryProperties {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Memory Allocate Info
// ============================================================================

/// Memory allocate info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryAllocateInfo {
    /// Allocation size
    pub allocation_size: u64,
    /// Memory type index
    pub memory_type_index: u32,
    /// Flags
    pub flags: MemoryAllocateFlags,
}

impl MemoryAllocateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(size: u64, type_index: u32) -> Self {
        Self {
            allocation_size: size,
            memory_type_index: type_index,
            flags: MemoryAllocateFlags::NONE,
        }
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: MemoryAllocateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With device address
    #[inline]
    pub const fn with_device_address(mut self) -> Self {
        self.flags = self.flags.union(MemoryAllocateFlags::DEVICE_ADDRESS);
        self
    }
}

impl Default for MemoryAllocateInfo {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// Memory allocate flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MemoryAllocateFlags(pub u32);

impl MemoryAllocateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Device mask
    pub const DEVICE_MASK: Self = Self(1 << 0);
    /// Device address
    pub const DEVICE_ADDRESS: Self = Self(1 << 1);
    /// Device address capture replay
    pub const DEVICE_ADDRESS_CAPTURE_REPLAY: Self = Self(1 << 2);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Mapped Memory Range
// ============================================================================

/// Mapped memory range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MappedMemoryRange {
    /// Memory handle
    pub memory: MemoryHandle,
    /// Offset
    pub offset: u64,
    /// Size (WHOLE_SIZE for entire mapping)
    pub size: u64,
}

impl MappedMemoryRange {
    /// Whole size constant
    pub const WHOLE_SIZE: u64 = !0;

    /// Creates new range
    #[inline]
    pub const fn new(memory: MemoryHandle, offset: u64, size: u64) -> Self {
        Self { memory, offset, size }
    }

    /// Whole mapping
    #[inline]
    pub const fn whole(memory: MemoryHandle) -> Self {
        Self::new(memory, 0, Self::WHOLE_SIZE)
    }
}

impl Default for MappedMemoryRange {
    fn default() -> Self {
        Self::new(MemoryHandle::NULL, 0, 0)
    }
}

// ============================================================================
// Memory Map Flags
// ============================================================================

/// Memory map flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MemoryMapFlags(pub u32);

impl MemoryMapFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Placed (EXT)
    pub const PLACED: Self = Self(1 << 0);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Allocation Strategy
// ============================================================================

/// Allocation strategy
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum AllocationStrategy {
    /// Best fit (minimize fragmentation)
    #[default]
    BestFit = 0,
    /// First fit (faster allocation)
    FirstFit = 1,
    /// Worst fit (large allocations)
    WorstFit = 2,
    /// Linear (bump allocator)
    Linear = 3,
    /// Buddy (power of 2 blocks)
    Buddy = 4,
    /// Pool (fixed size blocks)
    Pool = 5,
    /// TLSF (Two-Level Segregated Fit)
    Tlsf = 6,
}

impl AllocationStrategy {
    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::BestFit => "Best Fit",
            Self::FirstFit => "First Fit",
            Self::WorstFit => "Worst Fit",
            Self::Linear => "Linear",
            Self::Buddy => "Buddy",
            Self::Pool => "Pool",
            Self::Tlsf => "TLSF",
        }
    }

    /// Fragmentation resistance (0-10)
    #[inline]
    pub const fn fragmentation_resistance(&self) -> u32 {
        match self {
            Self::BestFit => 8,
            Self::FirstFit => 5,
            Self::WorstFit => 3,
            Self::Linear => 10,
            Self::Buddy => 7,
            Self::Pool => 10,
            Self::Tlsf => 9,
        }
    }

    /// Allocation speed (0-10)
    #[inline]
    pub const fn allocation_speed(&self) -> u32 {
        match self {
            Self::BestFit => 4,
            Self::FirstFit => 7,
            Self::WorstFit => 3,
            Self::Linear => 10,
            Self::Buddy => 8,
            Self::Pool => 10,
            Self::Tlsf => 9,
        }
    }
}

// ============================================================================
// Memory Usage
// ============================================================================

/// Memory usage hint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum MemoryUsage {
    /// Unknown / general
    #[default]
    Unknown = 0,
    /// GPU only (textures, render targets)
    GpuOnly = 1,
    /// CPU only (staging)
    CpuOnly = 2,
    /// CPU to GPU (dynamic uniforms)
    CpuToGpu = 3,
    /// GPU to CPU (readback)
    GpuToCpu = 4,
    /// CPU copy (staging)
    CpuCopy = 5,
    /// GPU lazily allocated
    GpuLazilyAllocated = 6,
    /// Auto (let allocator decide)
    Auto = 7,
    /// Auto prefer device
    AutoPreferDevice = 8,
    /// Auto prefer host
    AutoPreferHost = 9,
}

impl MemoryUsage {
    /// Required memory properties
    #[inline]
    pub const fn required_properties(&self) -> MemoryPropertyFlags {
        match self {
            Self::Unknown => MemoryPropertyFlags::NONE,
            Self::GpuOnly => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::CpuOnly | Self::CpuCopy => MemoryPropertyFlags::HOST_VISIBLE,
            Self::CpuToGpu => MemoryPropertyFlags::HOST_VISIBLE,
            Self::GpuToCpu => MemoryPropertyFlags::HOST_VISIBLE,
            Self::GpuLazilyAllocated => MemoryPropertyFlags::LAZILY_ALLOCATED,
            Self::Auto | Self::AutoPreferDevice | Self::AutoPreferHost => {
                MemoryPropertyFlags::NONE
            }
        }
    }

    /// Preferred memory properties
    #[inline]
    pub const fn preferred_properties(&self) -> MemoryPropertyFlags {
        match self {
            Self::Unknown => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::GpuOnly => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::CpuOnly => MemoryPropertyFlags::HOST_COHERENT,
            Self::CpuCopy => MemoryPropertyFlags::HOST_CACHED,
            Self::CpuToGpu => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::GpuToCpu => MemoryPropertyFlags::HOST_CACHED,
            Self::GpuLazilyAllocated => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::Auto => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::AutoPreferDevice => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::AutoPreferHost => MemoryPropertyFlags::HOST_VISIBLE,
        }
    }

    /// Name
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::GpuOnly => "GPU Only",
            Self::CpuOnly => "CPU Only",
            Self::CpuToGpu => "CPU to GPU",
            Self::GpuToCpu => "GPU to CPU",
            Self::CpuCopy => "CPU Copy",
            Self::GpuLazilyAllocated => "GPU Lazily Allocated",
            Self::Auto => "Auto",
            Self::AutoPreferDevice => "Auto Prefer Device",
            Self::AutoPreferHost => "Auto Prefer Host",
        }
    }
}

// ============================================================================
// Allocation Create Info
// ============================================================================

/// Allocation create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AllocationCreateInfo {
    /// Usage hint
    pub usage: MemoryUsage,
    /// Required flags
    pub required_flags: MemoryPropertyFlags,
    /// Preferred flags
    pub preferred_flags: MemoryPropertyFlags,
    /// Strategy
    pub strategy: AllocationStrategy,
    /// Flags
    pub flags: AllocationCreateFlags,
    /// Priority (0.0 - 1.0)
    pub priority: f32,
}

impl AllocationCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(usage: MemoryUsage) -> Self {
        Self {
            usage,
            required_flags: MemoryPropertyFlags::NONE,
            preferred_flags: MemoryPropertyFlags::NONE,
            strategy: AllocationStrategy::BestFit,
            flags: AllocationCreateFlags::NONE,
            priority: 0.5,
        }
    }

    /// GPU only
    pub const GPU_ONLY: Self = Self::new(MemoryUsage::GpuOnly);

    /// CPU to GPU
    pub const CPU_TO_GPU: Self = Self::new(MemoryUsage::CpuToGpu);

    /// GPU to CPU
    pub const GPU_TO_CPU: Self = Self::new(MemoryUsage::GpuToCpu);

    /// Staging
    pub const STAGING: Self = Self::new(MemoryUsage::CpuOnly);

    /// With required flags
    #[inline]
    pub const fn with_required_flags(mut self, flags: MemoryPropertyFlags) -> Self {
        self.required_flags = flags;
        self
    }

    /// With preferred flags
    #[inline]
    pub const fn with_preferred_flags(mut self, flags: MemoryPropertyFlags) -> Self {
        self.preferred_flags = flags;
        self
    }

    /// With strategy
    #[inline]
    pub const fn with_strategy(mut self, strategy: AllocationStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: AllocationCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With priority
    #[inline]
    pub const fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }

    /// Mapped (persistently)
    #[inline]
    pub const fn mapped(mut self) -> Self {
        self.flags = self.flags.union(AllocationCreateFlags::MAPPED);
        self
    }

    /// Dedicated allocation
    #[inline]
    pub const fn dedicated(mut self) -> Self {
        self.flags = self.flags.union(AllocationCreateFlags::DEDICATED_MEMORY);
        self
    }
}

impl Default for AllocationCreateInfo {
    fn default() -> Self {
        Self::new(MemoryUsage::Auto)
    }
}

/// Allocation create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct AllocationCreateFlags(pub u32);

impl AllocationCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Dedicated memory
    pub const DEDICATED_MEMORY: Self = Self(1 << 0);
    /// Never allocate
    pub const NEVER_ALLOCATE: Self = Self(1 << 1);
    /// Mapped (persistently)
    pub const MAPPED: Self = Self(1 << 2);
    /// Can alias
    pub const CAN_ALIAS: Self = Self(1 << 3);
    /// Host access sequential write
    pub const HOST_ACCESS_SEQUENTIAL_WRITE: Self = Self(1 << 4);
    /// Host access random
    pub const HOST_ACCESS_RANDOM: Self = Self(1 << 5);
    /// Host access allow transfer instead
    pub const HOST_ACCESS_ALLOW_TRANSFER_INSTEAD: Self = Self(1 << 6);
    /// Strategy min memory
    pub const STRATEGY_MIN_MEMORY: Self = Self(1 << 16);
    /// Strategy min time
    pub const STRATEGY_MIN_TIME: Self = Self(1 << 17);
    /// Strategy min fragmentation
    pub const STRATEGY_MIN_FRAGMENTATION: Self = Self(1 << 18);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Allocation
// ============================================================================

/// Allocation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AllocationInfo {
    /// Memory type index
    pub memory_type_index: u32,
    /// Device memory handle
    pub device_memory: MemoryHandle,
    /// Offset within memory
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Mapped pointer (if mapped)
    pub mapped_data: Option<NonNull<u8>>,
    /// Name (for debugging)
    pub name: u64, // Pointer to name string
}

impl AllocationInfo {
    /// Creates new info
    #[inline]
    pub const fn new(
        memory: MemoryHandle,
        offset: u64,
        size: u64,
        type_index: u32,
    ) -> Self {
        Self {
            memory_type_index: type_index,
            device_memory: memory,
            offset,
            size,
            mapped_data: None,
            name: 0,
        }
    }

    /// Is mapped
    #[inline]
    pub const fn is_mapped(&self) -> bool {
        self.mapped_data.is_some()
    }
}

impl Default for AllocationInfo {
    fn default() -> Self {
        Self::new(MemoryHandle::NULL, 0, 0, 0)
    }
}

// ============================================================================
// Memory Budget
// ============================================================================

/// Memory budget
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryBudget {
    /// Heap budget (estimated available)
    pub heap_budget: [u64; 16],
    /// Heap usage (current usage)
    pub heap_usage: [u64; 16],
}

impl MemoryBudget {
    /// Available memory for heap
    #[inline]
    pub const fn available(&self, heap_index: usize) -> u64 {
        if heap_index >= 16 {
            return 0;
        }
        self.heap_budget[heap_index].saturating_sub(self.heap_usage[heap_index])
    }

    /// Usage ratio for heap (0.0 - 1.0)
    #[inline]
    pub fn usage_ratio(&self, heap_index: usize) -> f32 {
        if heap_index >= 16 || self.heap_budget[heap_index] == 0 {
            return 0.0;
        }
        self.heap_usage[heap_index] as f32 / self.heap_budget[heap_index] as f32
    }

    /// Is heap under pressure
    #[inline]
    pub fn is_under_pressure(&self, heap_index: usize) -> bool {
        self.usage_ratio(heap_index) > 0.9
    }
}

// ============================================================================
// Defragmentation
// ============================================================================

/// Defragmentation flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DefragmentationFlags(pub u32);

impl DefragmentationFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Algorithm fast
    pub const ALGORITHM_FAST: Self = Self(1 << 0);
    /// Algorithm balanced
    pub const ALGORITHM_BALANCED: Self = Self(1 << 1);
    /// Algorithm full
    pub const ALGORITHM_FULL: Self = Self(1 << 2);
    /// Algorithm extensive
    pub const ALGORITHM_EXTENSIVE: Self = Self(1 << 3);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Defragmentation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DefragmentationInfo {
    /// Flags
    pub flags: DefragmentationFlags,
    /// Pool (0 = all pools)
    pub pool: u64,
    /// Max bytes per pass
    pub max_bytes_per_pass: u64,
    /// Max allocations per pass
    pub max_allocations_per_pass: u32,
}

impl DefragmentationInfo {
    /// Creates new info
    #[inline]
    pub const fn new() -> Self {
        Self {
            flags: DefragmentationFlags::ALGORITHM_BALANCED,
            pool: 0,
            max_bytes_per_pass: 256 * 1024 * 1024, // 256 MB
            max_allocations_per_pass: 1000,
        }
    }

    /// Fast defragmentation
    pub const FAST: Self = Self {
        flags: DefragmentationFlags::ALGORITHM_FAST,
        pool: 0,
        max_bytes_per_pass: 64 * 1024 * 1024,
        max_allocations_per_pass: 100,
    };

    /// Full defragmentation
    pub const FULL: Self = Self {
        flags: DefragmentationFlags::ALGORITHM_FULL,
        pool: 0,
        max_bytes_per_pass: 0, // Unlimited
        max_allocations_per_pass: 0, // Unlimited
    };

    /// With flags
    #[inline]
    pub const fn with_flags(mut self, flags: DefragmentationFlags) -> Self {
        self.flags = flags;
        self
    }

    /// With pool
    #[inline]
    pub const fn with_pool(mut self, pool: u64) -> Self {
        self.pool = pool;
        self
    }
}

impl Default for DefragmentationInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Defragmentation statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct DefragmentationStats {
    /// Bytes moved
    pub bytes_moved: u64,
    /// Bytes freed
    pub bytes_freed: u64,
    /// Allocations moved
    pub allocations_moved: u32,
    /// Device memory blocks freed
    pub device_memory_blocks_freed: u32,
}

// ============================================================================
// Pool Create Info
// ============================================================================

/// Memory pool create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryPoolCreateInfo {
    /// Memory type index
    pub memory_type_index: u32,
    /// Flags
    pub flags: MemoryPoolCreateFlags,
    /// Block size
    pub block_size: u64,
    /// Min block count
    pub min_block_count: u32,
    /// Max block count
    pub max_block_count: u32,
    /// Priority
    pub priority: f32,
    /// Min allocation alignment
    pub min_allocation_alignment: u64,
}

impl MemoryPoolCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(memory_type_index: u32) -> Self {
        Self {
            memory_type_index,
            flags: MemoryPoolCreateFlags::NONE,
            block_size: 64 * 1024 * 1024, // 64 MB
            min_block_count: 0,
            max_block_count: 0, // Unlimited
            priority: 0.5,
            min_allocation_alignment: 0,
        }
    }

    /// With block size
    #[inline]
    pub const fn with_block_size(mut self, size: u64) -> Self {
        self.block_size = size;
        self
    }

    /// With block count limits
    #[inline]
    pub const fn with_block_count(mut self, min: u32, max: u32) -> Self {
        self.min_block_count = min;
        self.max_block_count = max;
        self
    }

    /// Linear allocation
    #[inline]
    pub const fn linear(mut self) -> Self {
        self.flags = self.flags.union(MemoryPoolCreateFlags::LINEAR_ALGORITHM);
        self
    }

    /// Buddy allocation
    #[inline]
    pub const fn buddy(mut self) -> Self {
        self.flags = self.flags.union(MemoryPoolCreateFlags::BUDDY_ALGORITHM);
        self
    }
}

impl Default for MemoryPoolCreateInfo {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Memory pool create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MemoryPoolCreateFlags(pub u32);

impl MemoryPoolCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Ignore buffer image granularity
    pub const IGNORE_BUFFER_IMAGE_GRANULARITY: Self = Self(1 << 1);
    /// Linear algorithm
    pub const LINEAR_ALGORITHM: Self = Self(1 << 2);
    /// Buddy algorithm
    pub const BUDDY_ALGORITHM: Self = Self(1 << 3);
    /// TLSF algorithm
    pub const TLSF_ALGORITHM: Self = Self(1 << 4);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// ============================================================================
// Virtual Allocation
// ============================================================================

/// Virtual allocation create info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VirtualAllocationCreateInfo {
    /// Size
    pub size: u64,
    /// Alignment
    pub alignment: u64,
    /// Flags
    pub flags: VirtualAllocationCreateFlags,
    /// User data
    pub user_data: u64,
}

impl VirtualAllocationCreateInfo {
    /// Creates new info
    #[inline]
    pub const fn new(size: u64) -> Self {
        Self {
            size,
            alignment: 1,
            flags: VirtualAllocationCreateFlags::NONE,
            user_data: 0,
        }
    }

    /// With alignment
    #[inline]
    pub const fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// With user data
    #[inline]
    pub const fn with_user_data(mut self, data: u64) -> Self {
        self.user_data = data;
        self
    }
}

impl Default for VirtualAllocationCreateInfo {
    fn default() -> Self {
        Self::new(0)
    }
}

/// Virtual allocation create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct VirtualAllocationCreateFlags(pub u32);

impl VirtualAllocationCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Upper address
    pub const UPPER_ADDRESS: Self = Self(1 << 0);
    /// Strategy min memory
    pub const STRATEGY_MIN_MEMORY: Self = Self(1 << 16);
    /// Strategy min time
    pub const STRATEGY_MIN_TIME: Self = Self(1 << 17);
    /// Strategy min fragmentation
    pub const STRATEGY_MIN_FRAGMENTATION: Self = Self(1 << 18);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Virtual allocation info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct VirtualAllocationInfo {
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
    /// User data
    pub user_data: u64,
}

// ============================================================================
// External Memory
// ============================================================================

/// External memory handle type flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalMemoryHandleTypeFlags(pub u32);

impl ExternalMemoryHandleTypeFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Opaque FD
    pub const OPAQUE_FD: Self = Self(1 << 0);
    /// Opaque Win32
    pub const OPAQUE_WIN32: Self = Self(1 << 1);
    /// Opaque Win32 KMT
    pub const OPAQUE_WIN32_KMT: Self = Self(1 << 2);
    /// D3D11 texture
    pub const D3D11_TEXTURE: Self = Self(1 << 3);
    /// D3D11 texture KMT
    pub const D3D11_TEXTURE_KMT: Self = Self(1 << 4);
    /// D3D12 heap
    pub const D3D12_HEAP: Self = Self(1 << 5);
    /// D3D12 resource
    pub const D3D12_RESOURCE: Self = Self(1 << 6);
    /// DMA buf
    pub const DMA_BUF: Self = Self(1 << 9);
    /// Host allocation
    pub const HOST_ALLOCATION: Self = Self(1 << 7);
    /// Host mapped foreign memory
    pub const HOST_MAPPED_FOREIGN_MEMORY: Self = Self(1 << 8);
    /// Zircon VMO
    pub const ZIRCON_VMO: Self = Self(1 << 11);
    /// RDMA address NV
    pub const RDMA_ADDRESS_NV: Self = Self(1 << 12);
    /// Screen buffer QNX
    pub const SCREEN_BUFFER_QNX: Self = Self(1 << 14);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// External memory properties
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct ExternalMemoryProperties {
    /// External memory features
    pub external_memory_features: ExternalMemoryFeatureFlags,
    /// Export from imported handle types
    pub export_from_imported_handle_types: ExternalMemoryHandleTypeFlags,
    /// Compatible handle types
    pub compatible_handle_types: ExternalMemoryHandleTypeFlags,
}

/// External memory feature flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExternalMemoryFeatureFlags(pub u32);

impl ExternalMemoryFeatureFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Dedicated only
    pub const DEDICATED_ONLY: Self = Self(1 << 0);
    /// Exportable
    pub const EXPORTABLE: Self = Self(1 << 1);
    /// Importable
    pub const IMPORTABLE: Self = Self(1 << 2);

    /// Contains
    #[inline]
    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Union
    #[inline]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
