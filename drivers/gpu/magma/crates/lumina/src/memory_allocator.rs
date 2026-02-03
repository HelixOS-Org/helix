//! Memory allocator types
//!
//! This module provides types for GPU memory allocation and management.

use core::num::NonZeroU32;

/// Device memory handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct DeviceMemoryHandle(pub NonZeroU32);

impl DeviceMemoryHandle {
    /// Creates a new handle from raw ID
    pub const fn from_raw(id: u32) -> Option<Self> {
        match NonZeroU32::new(id) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }

    /// Gets the raw ID
    pub const fn raw(&self) -> u32 {
        self.0.get()
    }
}

/// Memory allocation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryAllocateInfo {
    /// Allocation size in bytes
    pub allocation_size: u64,
    /// Memory type index
    pub memory_type_index: u32,
    /// Allocation flags
    pub flags: MemoryAllocateFlags,
}

impl MemoryAllocateInfo {
    /// Creates allocation info
    pub const fn new(size: u64, memory_type: u32) -> Self {
        Self {
            allocation_size: size,
            memory_type_index: memory_type,
            flags: MemoryAllocateFlags::empty(),
        }
    }

    /// With device address
    pub const fn with_device_address(mut self) -> Self {
        self.flags = self.flags.union(MemoryAllocateFlags::DEVICE_ADDRESS);
        self
    }
}

bitflags::bitflags! {
    /// Memory allocation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct MemoryAllocateFlags: u32 {
        /// Device address
        const DEVICE_ADDRESS = 1 << 1;
        /// Device address capture replay
        const DEVICE_ADDRESS_CAPTURE_REPLAY = 1 << 2;
    }
}

impl MemoryAllocateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Memory type
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryType {
    /// Property flags
    pub property_flags: MemoryPropertyFlags,
    /// Heap index
    pub heap_index: u32,
}

impl MemoryType {
    /// Is this device local
    pub const fn is_device_local(self) -> bool {
        self.property_flags
            .contains(MemoryPropertyFlags::DEVICE_LOCAL)
    }

    /// Is this host visible
    pub const fn is_host_visible(self) -> bool {
        self.property_flags
            .contains(MemoryPropertyFlags::HOST_VISIBLE)
    }

    /// Is this host coherent
    pub const fn is_host_coherent(self) -> bool {
        self.property_flags
            .contains(MemoryPropertyFlags::HOST_COHERENT)
    }

    /// Is this host cached
    pub const fn is_host_cached(self) -> bool {
        self.property_flags
            .contains(MemoryPropertyFlags::HOST_CACHED)
    }
}

bitflags::bitflags! {
    /// Memory property flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct MemoryPropertyFlags: u32 {
        /// Device local memory
        const DEVICE_LOCAL = 1 << 0;
        /// Host visible memory
        const HOST_VISIBLE = 1 << 1;
        /// Host coherent memory
        const HOST_COHERENT = 1 << 2;
        /// Host cached memory
        const HOST_CACHED = 1 << 3;
        /// Lazily allocated memory
        const LAZILY_ALLOCATED = 1 << 4;
        /// Protected memory
        const PROTECTED = 1 << 5;
        /// Device coherent (AMD)
        const DEVICE_COHERENT = 1 << 6;
        /// Device uncached (AMD)
        const DEVICE_UNCACHED = 1 << 7;
        /// RDMA capable
        const RDMA_CAPABLE = 1 << 8;
    }
}

impl MemoryPropertyFlags {
    /// Staging memory (host visible + coherent)
    pub const STAGING: Self =
        Self::from_bits_truncate(Self::HOST_VISIBLE.bits() | Self::HOST_COHERENT.bits());

    /// Readback memory (host visible + cached)
    pub const READBACK: Self =
        Self::from_bits_truncate(Self::HOST_VISIBLE.bits() | Self::HOST_CACHED.bits());

    /// Optimal GPU memory
    pub const GPU_OPTIMAL: Self = Self::DEVICE_LOCAL;
}

/// Memory heap
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryHeap {
    /// Heap size in bytes
    pub size: u64,
    /// Heap flags
    pub flags: MemoryHeapFlags,
}

bitflags::bitflags! {
    /// Memory heap flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct MemoryHeapFlags: u32 {
        /// Device local heap
        const DEVICE_LOCAL = 1 << 0;
        /// Multi-instance heap
        const MULTI_INSTANCE = 1 << 1;
    }
}

impl MemoryHeapFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Physical device memory properties
#[derive(Clone, Debug)]
pub struct PhysicalDeviceMemoryProperties {
    /// Memory types
    pub memory_types: alloc::vec::Vec<MemoryType>,
    /// Memory heaps
    pub memory_heaps: alloc::vec::Vec<MemoryHeap>,
}

use alloc::vec::Vec;

impl PhysicalDeviceMemoryProperties {
    /// Finds a memory type matching requirements
    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: MemoryPropertyFlags,
    ) -> Option<u32> {
        for (i, mem_type) in self.memory_types.iter().enumerate() {
            if (type_filter & (1 << i)) != 0 && mem_type.property_flags.contains(properties) {
                return Some(i as u32);
            }
        }
        None
    }

    /// Finds device local memory
    pub fn find_device_local(&self, type_filter: u32) -> Option<u32> {
        self.find_memory_type(type_filter, MemoryPropertyFlags::DEVICE_LOCAL)
    }

    /// Finds host visible memory
    pub fn find_host_visible(&self, type_filter: u32) -> Option<u32> {
        self.find_memory_type(type_filter, MemoryPropertyFlags::HOST_VISIBLE)
    }

    /// Finds staging memory
    pub fn find_staging(&self, type_filter: u32) -> Option<u32> {
        self.find_memory_type(type_filter, MemoryPropertyFlags::STAGING)
    }

    /// Total device local memory
    pub fn total_device_local_memory(&self) -> u64 {
        self.memory_heaps
            .iter()
            .filter(|h| h.flags.contains(MemoryHeapFlags::DEVICE_LOCAL))
            .map(|h| h.size)
            .sum()
    }
}

/// Memory requirements
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryRequirements {
    /// Size in bytes
    pub size: u64,
    /// Alignment in bytes
    pub alignment: u64,
    /// Memory type bits (bitmask of valid memory types)
    pub memory_type_bits: u32,
}

impl MemoryRequirements {
    /// Calculates aligned offset
    pub const fn aligned_offset(&self, offset: u64) -> u64 {
        (offset + self.alignment - 1) & !(self.alignment - 1)
    }

    /// Aligned size
    pub const fn aligned_size(&self) -> u64 {
        (self.size + self.alignment - 1) & !(self.alignment - 1)
    }
}

/// Memory requirements 2 (extended)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryRequirements2 {
    /// Basic memory requirements
    pub memory_requirements: MemoryRequirements,
    /// Prefers dedicated allocation
    pub prefers_dedicated_allocation: bool,
    /// Requires dedicated allocation
    pub requires_dedicated_allocation: bool,
}

/// Mapped memory range
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MappedMemoryRange {
    /// Memory
    pub memory: DeviceMemoryHandle,
    /// Offset
    pub offset: u64,
    /// Size (WHOLE_SIZE for whole memory)
    pub size: u64,
}

/// Whole size constant
pub const WHOLE_SIZE: u64 = !0u64;

impl MappedMemoryRange {
    /// Whole memory range
    pub const fn whole(memory: DeviceMemoryHandle) -> Self {
        Self {
            memory,
            offset: 0,
            size: WHOLE_SIZE,
        }
    }

    /// Specific range
    pub const fn range(memory: DeviceMemoryHandle, offset: u64, size: u64) -> Self {
        Self {
            memory,
            offset,
            size,
        }
    }
}

/// Memory allocator strategy
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum AllocationStrategy {
    /// Best fit allocation
    #[default]
    BestFit  = 0,
    /// First fit allocation
    FirstFit = 1,
    /// Worst fit allocation (for fragmentation testing)
    WorstFit = 2,
}

/// Memory pool configuration
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryPoolConfig {
    /// Block size
    pub block_size: u64,
    /// Maximum blocks
    pub max_blocks: u32,
    /// Memory type index
    pub memory_type_index: u32,
    /// Allocation strategy
    pub strategy: AllocationStrategy,
    /// Minimum allocation size
    pub min_allocation_size: u64,
}

impl MemoryPoolConfig {
    /// Default block size (256 MB)
    pub const DEFAULT_BLOCK_SIZE: u64 = 256 * 1024 * 1024;

    /// Creates a config for device local memory
    pub const fn device_local(memory_type: u32) -> Self {
        Self {
            block_size: Self::DEFAULT_BLOCK_SIZE,
            max_blocks: 16,
            memory_type_index: memory_type,
            strategy: AllocationStrategy::BestFit,
            min_allocation_size: 256,
        }
    }

    /// Creates a config for staging memory
    pub const fn staging(memory_type: u32) -> Self {
        Self {
            block_size: 64 * 1024 * 1024, // 64 MB
            max_blocks: 8,
            memory_type_index: memory_type,
            strategy: AllocationStrategy::FirstFit,
            min_allocation_size: 64,
        }
    }

    /// With custom block size
    pub const fn with_block_size(mut self, size: u64) -> Self {
        self.block_size = size;
        self
    }

    /// With maximum blocks
    pub const fn with_max_blocks(mut self, count: u32) -> Self {
        self.max_blocks = count;
        self
    }
}

/// Allocation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct AllocationHandle {
    /// Pool index
    pub pool: u32,
    /// Block index within pool
    pub block: u32,
    /// Offset within block
    pub offset: u64,
    /// Size of allocation
    pub size: u64,
}

impl AllocationHandle {
    /// Creates a null handle
    pub const NULL: Self = Self {
        pool: 0,
        block: 0,
        offset: 0,
        size: 0,
    };

    /// Is this null
    pub const fn is_null(&self) -> bool {
        self.size == 0
    }
}

/// Allocation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AllocationInfo {
    /// Device memory
    pub device_memory: DeviceMemoryHandle,
    /// Offset in device memory
    pub offset: u64,
    /// Size
    pub size: u64,
    /// Mapped pointer (if mapped)
    pub mapped_ptr: *mut u8,
}

impl AllocationInfo {
    /// Is this mapped
    pub fn is_mapped(&self) -> bool {
        !self.mapped_ptr.is_null()
    }

    /// Gets mapped slice (unsafe)
    ///
    /// # Safety
    /// Caller must ensure the allocation is mapped and size is valid
    pub unsafe fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.mapped_ptr, self.size as usize) }
    }

    /// Gets mutable mapped slice (unsafe)
    ///
    /// # Safety
    /// Caller must ensure exclusive access and valid mapping
    pub unsafe fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.mapped_ptr, self.size as usize) }
    }
}

/// Virtual allocation flags
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct VirtualAllocationFlags: u32 {
        /// Upper address
        const UPPER_ADDRESS = 1 << 0;
        /// Strategy min memory
        const STRATEGY_MIN_MEMORY = 1 << 16;
        /// Strategy min time
        const STRATEGY_MIN_TIME = 1 << 17;
        /// Strategy min offset
        const STRATEGY_MIN_OFFSET = 1 << 18;
    }
}

impl VirtualAllocationFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Virtual block creation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VirtualBlockCreateInfo {
    /// Block size
    pub size: u64,
    /// Flags
    pub flags: VirtualBlockFlags,
}

bitflags::bitflags! {
    /// Virtual block flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct VirtualBlockFlags: u32 {
        /// Linear algorithm
        const LINEAR_ALGORITHM = 1 << 0;
    }
}

impl VirtualBlockFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Budget info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryBudget {
    /// Heap index
    pub heap_index: u32,
    /// Budget in bytes (available to application)
    pub budget: u64,
    /// Current usage in bytes
    pub usage: u64,
}

impl MemoryBudget {
    /// Available memory
    pub fn available(&self) -> u64 {
        self.budget.saturating_sub(self.usage)
    }

    /// Usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.budget == 0 {
            0.0
        } else {
            (self.usage as f64 / self.budget as f64 * 100.0) as f32
        }
    }
}

/// Defragmentation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct DefragmentationInfo {
    /// Maximum bytes to move
    pub max_bytes_to_move: u64,
    /// Maximum allocations to move
    pub max_allocations_to_move: u32,
    /// Flags
    pub flags: DefragmentationFlags,
}

impl DefragmentationInfo {
    /// Default defragmentation
    pub const fn default() -> Self {
        Self {
            max_bytes_to_move: u64::MAX,
            max_allocations_to_move: u32::MAX,
            flags: DefragmentationFlags::empty(),
        }
    }

    /// Limited defragmentation
    pub const fn limited(max_bytes: u64, max_moves: u32) -> Self {
        Self {
            max_bytes_to_move: max_bytes,
            max_allocations_to_move: max_moves,
            flags: DefragmentationFlags::empty(),
        }
    }
}

bitflags::bitflags! {
    /// Defragmentation flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct DefragmentationFlags: u32 {
        /// Incremental defragmentation
        const INCREMENTAL = 1 << 0;
    }
}

impl DefragmentationFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Defragmentation stats
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

/// Memory priority
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MemoryPriority {
    /// Low priority (can be evicted)
    Low     = 0,
    /// Normal priority
    #[default]
    Normal  = 1,
    /// High priority (avoid eviction)
    High    = 2,
    /// Maximum priority (never evict)
    Maximum = 3,
}

impl MemoryPriority {
    /// Converts to float (0.0 - 1.0)
    pub fn as_float(self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Normal => 0.5,
            Self::High => 0.75,
            Self::Maximum => 1.0,
        }
    }
}

/// External memory handle type
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct ExternalMemoryHandleType: u32 {
        /// Opaque file descriptor
        const OPAQUE_FD = 1 << 0;
        /// Opaque Windows handle
        const OPAQUE_WIN32 = 1 << 1;
        /// Opaque Windows handle KMT
        const OPAQUE_WIN32_KMT = 1 << 2;
        /// D3D11 texture
        const D3D11_TEXTURE = 1 << 3;
        /// D3D11 texture KMT
        const D3D11_TEXTURE_KMT = 1 << 4;
        /// D3D12 heap
        const D3D12_HEAP = 1 << 5;
        /// D3D12 resource
        const D3D12_RESOURCE = 1 << 6;
        /// DMA buffer
        const DMA_BUF = 1 << 9;
        /// Android hardware buffer
        const ANDROID_HARDWARE_BUFFER = 1 << 10;
        /// Host allocation
        const HOST_ALLOCATION = 1 << 7;
        /// Host mapped foreign memory
        const HOST_MAPPED_FOREIGN_MEMORY = 1 << 8;
    }
}

impl ExternalMemoryHandleType {
    /// No external memory
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }
}

/// Import memory info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ImportMemoryInfo {
    /// Handle type
    pub handle_type: ExternalMemoryHandleType,
    /// File descriptor (for OPAQUE_FD/DMA_BUF)
    pub fd: i32,
    /// Windows handle (for Win32 types)
    pub handle: *mut core::ffi::c_void,
}

impl ImportMemoryInfo {
    /// From file descriptor
    pub const fn from_fd(handle_type: ExternalMemoryHandleType, fd: i32) -> Self {
        Self {
            handle_type,
            fd,
            handle: core::ptr::null_mut(),
        }
    }

    /// From DMA buffer
    pub const fn from_dma_buf(fd: i32) -> Self {
        Self::from_fd(ExternalMemoryHandleType::DMA_BUF, fd)
    }
}

/// Export memory info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ExportMemoryInfo {
    /// Handle types to export
    pub handle_types: ExternalMemoryHandleType,
}

impl ExportMemoryInfo {
    /// Export as DMA buffer
    pub const DMA_BUF: Self = Self {
        handle_types: ExternalMemoryHandleType::DMA_BUF,
    };

    /// Export as opaque FD
    pub const OPAQUE_FD: Self = Self {
        handle_types: ExternalMemoryHandleType::OPAQUE_FD,
    };
}
