//! Memory types and allocation utilities
//!
//! This module provides types for GPU memory management.

/// Memory property flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MemoryPropertyFlags(pub u32);

impl MemoryPropertyFlags {
    /// No special properties
    pub const NONE: Self = Self(0);
    /// Device local (GPU-only, fastest)
    pub const DEVICE_LOCAL: Self = Self(1 << 0);
    /// Host visible (CPU can access)
    pub const HOST_VISIBLE: Self = Self(1 << 1);
    /// Host coherent (no flush needed)
    pub const HOST_COHERENT: Self = Self(1 << 2);
    /// Host cached (faster CPU reads)
    pub const HOST_CACHED: Self = Self(1 << 3);
    /// Lazily allocated
    pub const LAZILY_ALLOCATED: Self = Self(1 << 4);
    /// Protected memory
    pub const PROTECTED: Self = Self(1 << 5);
    /// Device coherent (AMD)
    pub const DEVICE_COHERENT: Self = Self(1 << 6);
    /// Device uncached (AMD)
    pub const DEVICE_UNCACHED: Self = Self(1 << 7);
    /// RDMA capable
    pub const RDMA_CAPABLE: Self = Self(1 << 8);

    /// Device local only (optimal for GPU)
    pub const DEVICE_ONLY: Self = Self::DEVICE_LOCAL;

    /// Host visible and coherent (for streaming)
    pub const HOST_UPLOAD: Self = Self(Self::HOST_VISIBLE.0 | Self::HOST_COHERENT.0);

    /// Host visible, coherent, and cached (for readback)
    pub const HOST_READBACK: Self = Self(
        Self::HOST_VISIBLE.0 | Self::HOST_COHERENT.0 | Self::HOST_CACHED.0
    );

    /// Checks if contains flag
    pub const fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Is device local
    pub const fn is_device_local(&self) -> bool {
        self.contains(Self::DEVICE_LOCAL)
    }

    /// Is host visible
    pub const fn is_host_visible(&self) -> bool {
        self.contains(Self::HOST_VISIBLE)
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

/// Memory heap flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct MemoryHeapFlags(pub u32);

impl MemoryHeapFlags {
    /// No special flags
    pub const NONE: Self = Self(0);
    /// Device local heap
    pub const DEVICE_LOCAL: Self = Self(1 << 0);
    /// Multi-instance heap
    pub const MULTI_INSTANCE: Self = Self(1 << 1);
}

/// Memory type info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryType {
    /// Property flags
    pub property_flags: MemoryPropertyFlags,
    /// Heap index
    pub heap_index: u32,
}

/// Memory heap info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryHeap {
    /// Heap size in bytes
    pub size: u64,
    /// Heap flags
    pub flags: MemoryHeapFlags,
}

/// Memory allocation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryAllocationInfo {
    /// Size in bytes
    pub size: u64,
    /// Alignment requirement
    pub alignment: u64,
    /// Memory type index
    pub memory_type_index: u32,
}

impl MemoryAllocationInfo {
    /// Creates new allocation info
    pub const fn new(size: u64, alignment: u64, memory_type: u32) -> Self {
        Self {
            size,
            alignment,
            memory_type_index: memory_type,
        }
    }

    /// Aligned size
    pub const fn aligned_size(&self) -> u64 {
        let mask = self.alignment - 1;
        (self.size + mask) & !mask
    }
}

/// Memory allocation flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct AllocationFlags(pub u32);

impl AllocationFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Dedicated allocation
    pub const DEDICATED: Self = Self(1 << 0);
    /// Can alias
    pub const CAN_ALIAS: Self = Self(1 << 1);
    /// Mapped persistently
    pub const MAPPED: Self = Self(1 << 2);
    /// Create device address
    pub const DEVICE_ADDRESS: Self = Self(1 << 3);
    /// Linear (for export)
    pub const LINEAR: Self = Self(1 << 4);
}

impl core::ops::BitOr for AllocationFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Memory usage hint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum MemoryUsage {
    /// Unknown/general
    #[default]
    Unknown = 0,
    /// GPU only (fastest)
    GpuOnly = 1,
    /// CPU to GPU upload
    CpuToGpu = 2,
    /// GPU to CPU readback
    GpuToCpu = 3,
    /// CPU only
    CpuOnly = 4,
    /// CPU copy (staging)
    CpuCopy = 5,
    /// GPU lazily allocated
    GpuLazily = 6,
}

impl MemoryUsage {
    /// Preferred memory property flags
    pub const fn preferred_flags(&self) -> MemoryPropertyFlags {
        match self {
            Self::Unknown => MemoryPropertyFlags::NONE,
            Self::GpuOnly => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::CpuToGpu => MemoryPropertyFlags::HOST_UPLOAD,
            Self::GpuToCpu => MemoryPropertyFlags::HOST_READBACK,
            Self::CpuOnly => MemoryPropertyFlags::HOST_UPLOAD,
            Self::CpuCopy => MemoryPropertyFlags::HOST_UPLOAD,
            Self::GpuLazily => MemoryPropertyFlags(
                MemoryPropertyFlags::DEVICE_LOCAL.0 | MemoryPropertyFlags::LAZILY_ALLOCATED.0
            ),
        }
    }
}

/// Memory block (allocation result)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryBlock {
    /// Device memory handle
    pub memory: u64,
    /// Offset within the allocation
    pub offset: u64,
    /// Size of the block
    pub size: u64,
    /// Memory type index
    pub memory_type: u32,
    /// Is mapped
    pub is_mapped: bool,
    /// Mapped pointer (if mapped)
    pub mapped_ptr: *mut u8,
}

impl Default for MemoryBlock {
    fn default() -> Self {
        Self {
            memory: 0,
            offset: 0,
            size: 0,
            memory_type: 0,
            is_mapped: false,
            mapped_ptr: core::ptr::null_mut(),
        }
    }
}

impl MemoryBlock {
    /// Null block
    pub const NULL: Self = Self {
        memory: 0,
        offset: 0,
        size: 0,
        memory_type: 0,
        is_mapped: false,
        mapped_ptr: core::ptr::null_mut(),
    };

    /// End offset
    pub const fn end_offset(&self) -> u64 {
        self.offset + self.size
    }

    /// Checks if valid
    pub const fn is_valid(&self) -> bool {
        self.memory != 0 && self.size > 0
    }
}

/// Memory budget info
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryBudget {
    /// Total heap budget
    pub heap_budget: [u64; 16],
    /// Current heap usage
    pub heap_usage: [u64; 16],
    /// Heap count
    pub heap_count: u32,
}

impl MemoryBudget {
    /// Total budget
    pub fn total_budget(&self) -> u64 {
        self.heap_budget[..self.heap_count as usize].iter().sum()
    }

    /// Total usage
    pub fn total_usage(&self) -> u64 {
        self.heap_usage[..self.heap_count as usize].iter().sum()
    }

    /// Available memory
    pub fn available(&self) -> u64 {
        self.total_budget().saturating_sub(self.total_usage())
    }

    /// Usage percentage
    pub fn usage_percent(&self) -> f32 {
        let budget = self.total_budget();
        if budget > 0 {
            (self.total_usage() as f64 / budget as f64 * 100.0) as f32
        } else {
            0.0
        }
    }
}

/// Defragmentation flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct DefragmentationFlags(pub u32);

impl DefragmentationFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Incremental defragmentation
    pub const INCREMENTAL: Self = Self(1 << 0);
    /// GPU defragmentation (with copies)
    pub const GPU_COPY: Self = Self(1 << 1);
    /// Extensive defragmentation
    pub const EXTENSIVE: Self = Self(1 << 2);
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
    /// Blocks freed
    pub blocks_freed: u32,
}

/// Virtual allocation info
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VirtualAllocationInfo {
    /// Allocation size
    pub size: u64,
    /// Offset in virtual block
    pub offset: u64,
}

/// Pool create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct PoolFlags(pub u32);

impl PoolFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Linear algorithm
    pub const LINEAR: Self = Self(1 << 0);
    /// Buddy algorithm
    pub const BUDDY: Self = Self(1 << 1);
    /// TLSF algorithm
    pub const TLSF: Self = Self(1 << 2);
    /// Create mapped
    pub const MAPPED: Self = Self(1 << 3);
}

/// Pool statistics
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PoolStats {
    /// Used bytes
    pub used_bytes: u64,
    /// Unused (free) bytes
    pub unused_bytes: u64,
    /// Allocation count
    pub allocation_count: u32,
    /// Block count
    pub block_count: u32,
    /// Unused range count
    pub unused_range_count: u32,
}

impl PoolStats {
    /// Total size
    pub const fn total_size(&self) -> u64 {
        self.used_bytes + self.unused_bytes
    }

    /// Fragmentation ratio (0.0 to 1.0)
    pub fn fragmentation(&self) -> f32 {
        if self.unused_bytes > 0 && self.unused_range_count > 1 {
            1.0 - (1.0 / self.unused_range_count as f32)
        } else {
            0.0
        }
    }
}

/// Memory range
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct MemoryRange {
    /// Start offset
    pub offset: u64,
    /// Size
    pub size: u64,
}

impl MemoryRange {
    /// Creates new range
    pub const fn new(offset: u64, size: u64) -> Self {
        Self { offset, size }
    }

    /// End offset
    pub const fn end(&self) -> u64 {
        self.offset + self.size
    }

    /// Checks overlap
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.offset < other.end() && other.offset < self.end()
    }

    /// Checks if contains offset
    pub const fn contains(&self, offset: u64) -> bool {
        offset >= self.offset && offset < self.end()
    }

    /// Intersection with another range
    pub const fn intersection(&self, other: &Self) -> Option<Self> {
        let start = if self.offset > other.offset {
            self.offset
        } else {
            other.offset
        };
        let end = if self.end() < other.end() {
            self.end()
        } else {
            other.end()
        };

        if start < end {
            Some(Self::new(start, end - start))
        } else {
            None
        }
    }
}

/// Memory requirements
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct MemoryRequirements {
    /// Size in bytes
    pub size: u64,
    /// Alignment
    pub alignment: u64,
    /// Supported memory type bits
    pub memory_type_bits: u32,
}

impl MemoryRequirements {
    /// Aligned size
    pub const fn aligned_size(&self) -> u64 {
        let mask = self.alignment - 1;
        (self.size + mask) & !mask
    }

    /// Checks if memory type is supported
    pub const fn supports_memory_type(&self, type_index: u32) -> bool {
        (self.memory_type_bits & (1 << type_index)) != 0
    }
}

/// External memory handle type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ExternalMemoryHandleType {
    /// Opaque file descriptor
    OpaqueFd = 1,
    /// Opaque Win32 handle
    OpaqueWin32 = 2,
    /// D3D11 texture
    D3D11Texture = 4,
    /// D3D12 heap
    D3D12Heap = 8,
    /// D3D12 resource
    D3D12Resource = 16,
    /// DMA buffer
    DmaBuf = 32,
    /// Android hardware buffer
    AndroidHardwareBuffer = 64,
    /// Host allocation
    HostAllocation = 128,
    /// Host mapped pointer
    HostMappedPointer = 256,
}
