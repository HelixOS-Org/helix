//! Memory allocation and resource binding
//!
//! This module provides types for GPU memory management.

use crate::surface::TextureUsageFlags;
use crate::types::BufferHandle;

/// Memory type flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct MemoryPropertyFlags(pub u32);

impl MemoryPropertyFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Device local memory (fast GPU access)
    pub const DEVICE_LOCAL: Self = Self(1 << 0);
    /// Host visible memory (CPU can map)
    pub const HOST_VISIBLE: Self = Self(1 << 1);
    /// Host coherent memory (no flush/invalidate needed)
    pub const HOST_COHERENT: Self = Self(1 << 2);
    /// Host cached memory (faster CPU reads)
    pub const HOST_CACHED: Self = Self(1 << 3);
    /// Lazily allocated memory
    pub const LAZILY_ALLOCATED: Self = Self(1 << 4);
    /// Protected memory
    pub const PROTECTED: Self = Self(1 << 5);

    /// GPU only (device local, not host visible)
    pub const GPU_ONLY: Self = Self(Self::DEVICE_LOCAL.0);
    /// CPU to GPU (upload heap)
    pub const CPU_TO_GPU: Self = Self(Self::HOST_VISIBLE.0 | Self::HOST_COHERENT.0);
    /// GPU to CPU (readback heap)
    pub const GPU_TO_CPU: Self =
        Self(Self::HOST_VISIBLE.0 | Self::HOST_COHERENT.0 | Self::HOST_CACHED.0);

    /// Checks if flag is set
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct MemoryHeapFlags(pub u32);

impl MemoryHeapFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Device local heap
    pub const DEVICE_LOCAL: Self = Self(1 << 0);
    /// Multi-instance heap
    pub const MULTI_INSTANCE: Self = Self(1 << 1);
}

impl core::ops::BitOr for MemoryHeapFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Memory type info
#[derive(Clone, Copy, Debug)]
pub struct MemoryType {
    /// Property flags
    pub property_flags: MemoryPropertyFlags,
    /// Heap index
    pub heap_index: u32,
}

/// Memory heap info
#[derive(Clone, Copy, Debug)]
pub struct MemoryHeap {
    /// Size in bytes
    pub size: u64,
    /// Heap flags
    pub flags: MemoryHeapFlags,
}

/// Physical device memory properties
#[derive(Clone, Debug)]
pub struct MemoryProperties {
    /// Memory types
    pub memory_types: [MemoryType; 32],
    /// Number of valid memory types
    pub memory_type_count: u32,
    /// Memory heaps
    pub memory_heaps: [MemoryHeap; 16],
    /// Number of valid memory heaps
    pub memory_heap_count: u32,
}

impl MemoryProperties {
    /// Finds a suitable memory type
    pub fn find_memory_type(
        &self,
        type_bits: u32,
        required_flags: MemoryPropertyFlags,
    ) -> Option<u32> {
        for i in 0..self.memory_type_count {
            if (type_bits & (1 << i)) != 0 {
                let mem_type = &self.memory_types[i as usize];
                if mem_type.property_flags.contains(required_flags) {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Finds device local memory type
    pub fn find_device_local(&self, type_bits: u32) -> Option<u32> {
        self.find_memory_type(type_bits, MemoryPropertyFlags::DEVICE_LOCAL)
    }

    /// Finds host visible memory type
    pub fn find_host_visible(&self, type_bits: u32) -> Option<u32> {
        // Try coherent first
        self.find_memory_type(type_bits, MemoryPropertyFlags::CPU_TO_GPU)
            .or_else(|| self.find_memory_type(type_bits, MemoryPropertyFlags::HOST_VISIBLE))
    }
}

/// Memory allocation handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct AllocationHandle(pub u64);

impl AllocationHandle {
    /// Null/invalid allocation
    pub const NULL: Self = Self(0);

    /// Creates from raw value
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Returns raw value
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// Checks if valid
    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }
}

/// Memory allocation info
#[derive(Clone, Copy, Debug)]
pub struct AllocationInfo {
    /// Allocation handle
    pub allocation: AllocationHandle,
    /// Offset within memory block
    pub offset: u64,
    /// Size of allocation
    pub size: u64,
    /// Memory type index
    pub memory_type_index: u32,
    /// Mapped pointer (if mapped)
    pub mapped_ptr: Option<*mut u8>,
}

impl AllocationInfo {
    /// Returns mapped slice if available
    ///
    /// # Safety
    ///
    /// The caller must ensure the physical and virtual addresses are valid and properly aligned.
    pub unsafe fn mapped_slice(&self) -> Option<&[u8]> {
        self.mapped_ptr
            .map(|ptr| core::slice::from_raw_parts(ptr, self.size as usize))
    }

    /// Returns mapped mutable slice if available
    ///
    /// # Safety
    ///
    /// The caller must ensure the physical and virtual addresses are valid and properly aligned.
    pub unsafe fn mapped_slice_mut(&mut self) -> Option<&mut [u8]> {
        self.mapped_ptr
            .map(|ptr| core::slice::from_raw_parts_mut(ptr, self.size as usize))
    }
}

/// Memory allocation requirements
#[derive(Clone, Copy, Debug)]
pub struct MemoryRequirements {
    /// Size in bytes
    pub size: u64,
    /// Alignment in bytes
    pub alignment: u64,
    /// Bitmask of suitable memory types
    pub memory_type_bits: u32,
}

/// Allocation create flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct AllocationCreateFlags(pub u32);

impl AllocationCreateFlags {
    /// No flags
    pub const NONE: Self = Self(0);
    /// Dedicated allocation
    pub const DEDICATED: Self = Self(1 << 0);
    /// Mapped persistently
    pub const MAPPED: Self = Self(1 << 1);
    /// Within budget
    pub const WITHIN_BUDGET: Self = Self(1 << 2);
    /// Can alias memory
    pub const CAN_ALIAS: Self = Self(1 << 3);
    /// Host access sequential write
    pub const HOST_SEQUENTIAL_WRITE: Self = Self(1 << 4);
    /// Host access random
    pub const HOST_RANDOM_ACCESS: Self = Self(1 << 5);
    /// Min memory
    pub const MIN_MEMORY: Self = Self(1 << 6);
    /// Min time
    pub const MIN_TIME: Self = Self(1 << 7);
}

impl core::ops::BitOr for AllocationCreateFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Memory usage hint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MemoryUsage {
    /// Unknown / auto
    #[default]
    Unknown,
    /// GPU only
    GpuOnly,
    /// CPU only
    CpuOnly,
    /// CPU to GPU (upload)
    CpuToGpu,
    /// GPU to CPU (readback)
    GpuToCpu,
    /// CPU copy
    CpuCopy,
    /// GPU lazily allocated
    GpuLazilyAllocated,
}

impl MemoryUsage {
    /// Returns required memory property flags
    pub const fn required_flags(self) -> MemoryPropertyFlags {
        match self {
            Self::Unknown => MemoryPropertyFlags::NONE,
            Self::GpuOnly => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::CpuOnly => MemoryPropertyFlags::HOST_VISIBLE,
            Self::CpuToGpu => MemoryPropertyFlags::HOST_VISIBLE,
            Self::GpuToCpu => MemoryPropertyFlags::HOST_VISIBLE,
            Self::CpuCopy => MemoryPropertyFlags::HOST_VISIBLE,
            Self::GpuLazilyAllocated => MemoryPropertyFlags::LAZILY_ALLOCATED,
        }
    }

    /// Returns preferred memory property flags
    pub const fn preferred_flags(self) -> MemoryPropertyFlags {
        match self {
            Self::Unknown => MemoryPropertyFlags::NONE,
            Self::GpuOnly => MemoryPropertyFlags::DEVICE_LOCAL,
            Self::CpuOnly => MemoryPropertyFlags(
                MemoryPropertyFlags::HOST_VISIBLE.0
                    | MemoryPropertyFlags::HOST_COHERENT.0
                    | MemoryPropertyFlags::HOST_CACHED.0,
            ),
            Self::CpuToGpu => MemoryPropertyFlags(
                MemoryPropertyFlags::HOST_VISIBLE.0
                    | MemoryPropertyFlags::HOST_COHERENT.0
                    | MemoryPropertyFlags::DEVICE_LOCAL.0,
            ),
            Self::GpuToCpu => MemoryPropertyFlags(
                MemoryPropertyFlags::HOST_VISIBLE.0
                    | MemoryPropertyFlags::HOST_COHERENT.0
                    | MemoryPropertyFlags::HOST_CACHED.0,
            ),
            Self::CpuCopy => MemoryPropertyFlags(
                MemoryPropertyFlags::HOST_VISIBLE.0
                    | MemoryPropertyFlags::HOST_COHERENT.0
                    | MemoryPropertyFlags::HOST_CACHED.0,
            ),
            Self::GpuLazilyAllocated => MemoryPropertyFlags(
                MemoryPropertyFlags::LAZILY_ALLOCATED.0 | MemoryPropertyFlags::DEVICE_LOCAL.0,
            ),
        }
    }
}

/// Allocation create info
#[derive(Clone, Copy, Debug)]
pub struct AllocationCreateInfo {
    /// Flags
    pub flags: AllocationCreateFlags,
    /// Memory usage hint
    pub usage: MemoryUsage,
    /// Required memory property flags
    pub required_flags: MemoryPropertyFlags,
    /// Preferred memory property flags
    pub preferred_flags: MemoryPropertyFlags,
    /// Memory type bits filter (0 = all)
    pub memory_type_bits: u32,
    /// User data
    pub user_data: u64,
}

impl AllocationCreateInfo {
    /// GPU only allocation
    pub const GPU_ONLY: Self = Self {
        flags: AllocationCreateFlags::NONE,
        usage: MemoryUsage::GpuOnly,
        required_flags: MemoryPropertyFlags::NONE,
        preferred_flags: MemoryPropertyFlags::NONE,
        memory_type_bits: 0,
        user_data: 0,
    };

    /// CPU to GPU (staging/upload)
    pub const CPU_TO_GPU: Self = Self {
        flags: AllocationCreateFlags::MAPPED,
        usage: MemoryUsage::CpuToGpu,
        required_flags: MemoryPropertyFlags::NONE,
        preferred_flags: MemoryPropertyFlags::NONE,
        memory_type_bits: 0,
        user_data: 0,
    };

    /// GPU to CPU (readback)
    pub const GPU_TO_CPU: Self = Self {
        flags: AllocationCreateFlags::MAPPED,
        usage: MemoryUsage::GpuToCpu,
        required_flags: MemoryPropertyFlags::NONE,
        preferred_flags: MemoryPropertyFlags::NONE,
        memory_type_bits: 0,
        user_data: 0,
    };

    /// Sets flags
    pub const fn with_flags(mut self, flags: AllocationCreateFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Makes allocation dedicated
    pub const fn dedicated(mut self) -> Self {
        self.flags = AllocationCreateFlags(self.flags.0 | AllocationCreateFlags::DEDICATED.0);
        self
    }

    /// Makes allocation persistently mapped
    pub const fn mapped(mut self) -> Self {
        self.flags = AllocationCreateFlags(self.flags.0 | AllocationCreateFlags::MAPPED.0);
        self
    }
}

impl Default for AllocationCreateInfo {
    fn default() -> Self {
        Self::GPU_ONLY
    }
}

/// Buffer create info (with memory)
#[derive(Clone, Debug)]
pub struct BufferCreateInfo<'a> {
    /// Size in bytes
    pub size: u64,
    /// Usage flags
    pub usage: BufferUsageFlags,
    /// Sharing mode
    pub sharing_mode: SharingMode,
    /// Queue families (for concurrent sharing)
    pub queue_families: &'a [u32],
    /// Allocation info
    pub allocation: AllocationCreateInfo,
    /// Debug label
    pub label: Option<&'a str>,
}

impl<'a> BufferCreateInfo<'a> {
    /// Creates a vertex buffer
    pub const fn vertex(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::VERTEX,
            sharing_mode: SharingMode::Exclusive,
            queue_families: &[],
            allocation: AllocationCreateInfo::GPU_ONLY,
            label: None,
        }
    }

    /// Creates an index buffer
    pub const fn index(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::INDEX,
            sharing_mode: SharingMode::Exclusive,
            queue_families: &[],
            allocation: AllocationCreateInfo::GPU_ONLY,
            label: None,
        }
    }

    /// Creates a uniform buffer
    pub const fn uniform(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::UNIFORM,
            sharing_mode: SharingMode::Exclusive,
            queue_families: &[],
            allocation: AllocationCreateInfo::CPU_TO_GPU,
            label: None,
        }
    }

    /// Creates a storage buffer
    pub const fn storage(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::STORAGE,
            sharing_mode: SharingMode::Exclusive,
            queue_families: &[],
            allocation: AllocationCreateInfo::GPU_ONLY,
            label: None,
        }
    }

    /// Creates a staging buffer
    pub const fn staging(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: SharingMode::Exclusive,
            queue_families: &[],
            allocation: AllocationCreateInfo::CPU_TO_GPU,
            label: None,
        }
    }

    /// Creates a readback buffer
    pub const fn readback(size: u64) -> Self {
        Self {
            size,
            usage: BufferUsageFlags::TRANSFER_DST,
            sharing_mode: SharingMode::Exclusive,
            queue_families: &[],
            allocation: AllocationCreateInfo::GPU_TO_CPU,
            label: None,
        }
    }

    /// Adds transfer destination usage
    pub const fn with_transfer_dst(mut self) -> Self {
        self.usage = BufferUsageFlags(self.usage.0 | BufferUsageFlags::TRANSFER_DST.0);
        self
    }

    /// Sets label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

/// Buffer usage flags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct BufferUsageFlags(pub u32);

impl BufferUsageFlags {
    /// No usage
    pub const NONE: Self = Self(0);
    /// Transfer source
    pub const TRANSFER_SRC: Self = Self(1 << 0);
    /// Transfer destination
    pub const TRANSFER_DST: Self = Self(1 << 1);
    /// Uniform texel buffer
    pub const UNIFORM_TEXEL: Self = Self(1 << 2);
    /// Storage texel buffer
    pub const STORAGE_TEXEL: Self = Self(1 << 3);
    /// Uniform buffer
    pub const UNIFORM: Self = Self(1 << 4);
    /// Storage buffer
    pub const STORAGE: Self = Self(1 << 5);
    /// Index buffer
    pub const INDEX: Self = Self(1 << 6);
    /// Vertex buffer
    pub const VERTEX: Self = Self(1 << 7);
    /// Indirect buffer
    pub const INDIRECT: Self = Self(1 << 8);
    /// Shader device address
    pub const SHADER_DEVICE_ADDRESS: Self = Self(1 << 9);
    /// Acceleration structure build input (ray tracing)
    pub const ACCELERATION_STRUCTURE_BUILD_INPUT: Self = Self(1 << 10);
    /// Acceleration structure storage
    pub const ACCELERATION_STRUCTURE_STORAGE: Self = Self(1 << 11);
    /// Shader binding table
    pub const SHADER_BINDING_TABLE: Self = Self(1 << 12);

    /// Vertex buffer with transfer dst
    pub const VERTEX_UPLOAD: Self = Self(Self::VERTEX.0 | Self::TRANSFER_DST.0);
    /// Index buffer with transfer dst
    pub const INDEX_UPLOAD: Self = Self(Self::INDEX.0 | Self::TRANSFER_DST.0);

    /// Checks if flag is set
    pub const fn contains(self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }
}

impl core::ops::BitOr for BufferUsageFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for BufferUsageFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

/// Sharing mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SharingMode {
    /// Exclusive access
    #[default]
    Exclusive,
    /// Concurrent access across queue families
    Concurrent,
}

/// Texture create info (with memory)
#[derive(Clone, Debug)]
pub struct TextureCreateInfo<'a> {
    /// Texture type
    pub texture_type: TextureType,
    /// Format
    pub format: crate::compute::TextureFormat,
    /// Extent
    pub extent: TextureExtent,
    /// Mip levels
    pub mip_levels: u32,
    /// Array layers
    pub array_layers: u32,
    /// Sample count
    pub samples: SampleCount,
    /// Tiling
    pub tiling: ImageTiling,
    /// Usage
    pub usage: TextureUsageFlags,
    /// Sharing mode
    pub sharing_mode: SharingMode,
    /// Queue families
    pub queue_families: &'a [u32],
    /// Initial layout
    pub initial_layout: crate::command::ImageLayout,
    /// Allocation info
    pub allocation: AllocationCreateInfo,
    /// Debug label
    pub label: Option<&'a str>,
}

impl<'a> TextureCreateInfo<'a> {
    /// Creates a 2D texture
    pub const fn texture_2d(
        width: u32,
        height: u32,
        format: crate::compute::TextureFormat,
    ) -> Self {
        Self {
            texture_type: TextureType::Texture2D,
            format,
            extent: TextureExtent {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            tiling: ImageTiling::Optimal,
            usage: TextureUsageFlags::SAMPLED,
            sharing_mode: SharingMode::Exclusive,
            queue_families: &[],
            initial_layout: crate::command::ImageLayout::Undefined,
            allocation: AllocationCreateInfo::GPU_ONLY,
            label: None,
        }
    }

    /// Creates a render target
    pub const fn render_target(
        width: u32,
        height: u32,
        format: crate::compute::TextureFormat,
    ) -> Self {
        Self {
            texture_type: TextureType::Texture2D,
            format,
            extent: TextureExtent {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            tiling: ImageTiling::Optimal,
            usage: TextureUsageFlags::RENDER_TARGET,
            sharing_mode: SharingMode::Exclusive,
            queue_families: &[],
            initial_layout: crate::command::ImageLayout::Undefined,
            allocation: AllocationCreateInfo::GPU_ONLY,
            label: None,
        }
    }

    /// Creates a depth buffer
    pub const fn depth_buffer(
        width: u32,
        height: u32,
        format: crate::compute::TextureFormat,
    ) -> Self {
        Self {
            texture_type: TextureType::Texture2D,
            format,
            extent: TextureExtent {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: SampleCount::S1,
            tiling: ImageTiling::Optimal,
            usage: TextureUsageFlags::DEPTH_BUFFER,
            sharing_mode: SharingMode::Exclusive,
            queue_families: &[],
            initial_layout: crate::command::ImageLayout::Undefined,
            allocation: AllocationCreateInfo::GPU_ONLY,
            label: None,
        }
    }

    /// Sets mip levels
    pub const fn with_mip_levels(mut self, levels: u32) -> Self {
        self.mip_levels = levels;
        self
    }

    /// Sets array layers
    pub const fn with_array_layers(mut self, layers: u32) -> Self {
        self.array_layers = layers;
        self
    }

    /// Sets sample count
    pub const fn with_samples(mut self, samples: SampleCount) -> Self {
        self.samples = samples;
        self
    }

    /// Sets transfer dst usage
    pub const fn with_transfer_dst(mut self) -> Self {
        self.usage = TextureUsageFlags(self.usage.0 | TextureUsageFlags::TRANSFER_DST.0);
        self
    }

    /// Sets label
    pub const fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
}

/// Texture type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextureType {
    /// 1D texture
    Texture1D,
    /// 2D texture
    #[default]
    Texture2D,
    /// 3D texture
    Texture3D,
}

/// Texture extent
#[derive(Clone, Copy, Debug)]
pub struct TextureExtent {
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Depth
    pub depth: u32,
}

impl TextureExtent {
    /// Creates 2D extent
    pub const fn extent_2d(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            depth: 1,
        }
    }

    /// Creates 3D extent
    pub const fn extent_3d(width: u32, height: u32, depth: u32) -> Self {
        Self {
            width,
            height,
            depth,
        }
    }
}

/// Sample count
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SampleCount {
    /// 1 sample (no MSAA)
    #[default]
    S1,
    /// 2 samples
    S2,
    /// 4 samples
    S4,
    /// 8 samples
    S8,
    /// 16 samples
    S16,
    /// 32 samples
    S32,
    /// 64 samples
    S64,
}

impl SampleCount {
    /// Returns numeric sample count
    pub const fn count(self) -> u32 {
        match self {
            Self::S1 => 1,
            Self::S2 => 2,
            Self::S4 => 4,
            Self::S8 => 8,
            Self::S16 => 16,
            Self::S32 => 32,
            Self::S64 => 64,
        }
    }
}

/// Image tiling
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ImageTiling {
    /// Optimal tiling (GPU-specific)
    #[default]
    Optimal,
    /// Linear tiling (row-major)
    Linear,
}

/// Mapped memory range
#[derive(Clone, Copy, Debug)]
pub struct MappedMemoryRange {
    /// Allocation handle
    pub allocation: AllocationHandle,
    /// Offset
    pub offset: u64,
    /// Size (0 = whole allocation)
    pub size: u64,
}

impl MappedMemoryRange {
    /// Creates range for whole allocation
    pub const fn whole(allocation: AllocationHandle) -> Self {
        Self {
            allocation,
            offset: 0,
            size: 0,
        }
    }

    /// Creates range with offset and size
    pub const fn range(allocation: AllocationHandle, offset: u64, size: u64) -> Self {
        Self {
            allocation,
            offset,
            size,
        }
    }
}

/// Budget info for a memory heap
#[derive(Clone, Copy, Debug, Default)]
pub struct MemoryBudget {
    /// Estimated bytes currently used
    pub usage: u64,
    /// Estimated bytes available (budget)
    pub budget: u64,
    /// Total bytes allocated (may exceed budget)
    pub allocated: u64,
    /// Number of allocations
    pub allocation_count: u32,
    /// Number of blocks
    pub block_count: u32,
}

impl MemoryBudget {
    /// Returns remaining budget
    pub const fn remaining(&self) -> u64 {
        if self.budget > self.usage {
            self.budget - self.usage
        } else {
            0
        }
    }

    /// Returns usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.budget > 0 {
            (self.usage as f32 / self.budget as f32) * 100.0
        } else {
            0.0
        }
    }
}

/// Statistics for an allocator
#[derive(Clone, Debug, Default)]
pub struct AllocatorStats {
    /// Total bytes allocated
    pub total_allocated: u64,
    /// Total bytes used
    pub total_used: u64,
    /// Number of allocations
    pub allocation_count: u32,
    /// Per-heap budgets
    pub heap_budgets: [MemoryBudget; 16],
    /// Number of heaps
    pub heap_count: u32,
}

impl AllocatorStats {
    /// Returns fragmentation ratio (0 = no fragmentation)
    pub fn fragmentation(&self) -> f32 {
        if self.total_allocated > 0 {
            1.0 - (self.total_used as f32 / self.total_allocated as f32)
        } else {
            0.0
        }
    }
}
