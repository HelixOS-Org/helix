//! GPU Sorting Types for Lumina
//!
//! This module provides GPU-accelerated sorting algorithms
//! including radix sort, bitonic sort, and merge sort.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Sort Handles
// ============================================================================

/// GPU sorter handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GpuSorterHandle(pub u64);

impl GpuSorterHandle {
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

impl Default for GpuSorterHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sort key buffer handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SortKeyBufferHandle(pub u64);

impl SortKeyBufferHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SortKeyBufferHandle {
    fn default() -> Self {
        Self::NULL
    }
}

/// Sort pipeline handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SortPipelineHandle(pub u64);

impl SortPipelineHandle {
    /// Null handle
    pub const NULL: Self = Self(0);
}

impl Default for SortPipelineHandle {
    fn default() -> Self {
        Self::NULL
    }
}

// ============================================================================
// GPU Sorter Creation
// ============================================================================

/// GPU sorter create info
#[derive(Clone, Debug)]
pub struct GpuSorterCreateInfo {
    /// Name
    pub name: String,
    /// Sort algorithm
    pub algorithm: SortAlgorithm,
    /// Max elements
    pub max_elements: u32,
    /// Key type
    pub key_type: SortKeyType,
    /// Value type (None = key-only)
    pub value_type: Option<SortValueType>,
    /// Sort direction
    pub direction: SortDirection,
    /// Flags
    pub flags: SortFlags,
}

impl GpuSorterCreateInfo {
    /// Creates new info
    pub fn new(algorithm: SortAlgorithm) -> Self {
        Self {
            name: String::new(),
            algorithm,
            max_elements: 1024 * 1024,
            key_type: SortKeyType::Uint32,
            value_type: None,
            direction: SortDirection::Ascending,
            flags: SortFlags::empty(),
        }
    }

    /// With name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// With max elements
    pub fn with_max_elements(mut self, max: u32) -> Self {
        self.max_elements = max;
        self
    }

    /// With key type
    pub fn with_key_type(mut self, key_type: SortKeyType) -> Self {
        self.key_type = key_type;
        self
    }

    /// With value type (key-value sort)
    pub fn with_value_type(mut self, value_type: SortValueType) -> Self {
        self.value_type = Some(value_type);
        self
    }

    /// With direction
    pub fn with_direction(mut self, direction: SortDirection) -> Self {
        self.direction = direction;
        self
    }

    /// With flags
    pub fn with_flags(mut self, flags: SortFlags) -> Self {
        self.flags |= flags;
        self
    }

    /// Radix sort preset
    pub fn radix_sort(max_elements: u32) -> Self {
        Self::new(SortAlgorithm::RadixSort)
            .with_max_elements(max_elements)
            .with_key_type(SortKeyType::Uint32)
    }

    /// Radix sort with values
    pub fn radix_sort_pairs(max_elements: u32) -> Self {
        Self::radix_sort(max_elements).with_value_type(SortValueType::Uint32)
    }

    /// Bitonic sort preset
    pub fn bitonic_sort(max_elements: u32) -> Self {
        Self::new(SortAlgorithm::BitonicSort).with_max_elements(max_elements)
    }

    /// Small list sort (for small arrays)
    pub fn small_sort(max_elements: u32) -> Self {
        Self::new(SortAlgorithm::SmallSort).with_max_elements(max_elements.min(4096))
    }

    /// Depth sort preset (for transparency)
    pub fn depth_sort(max_elements: u32) -> Self {
        Self::new(SortAlgorithm::RadixSort)
            .with_max_elements(max_elements)
            .with_key_type(SortKeyType::Float32)
            .with_value_type(SortValueType::Uint32)  // Index
            .with_direction(SortDirection::Descending) // Back to front
    }

    /// Distance sort (front to back)
    pub fn distance_sort(max_elements: u32) -> Self {
        Self::new(SortAlgorithm::RadixSort)
            .with_max_elements(max_elements)
            .with_key_type(SortKeyType::Float32)
            .with_value_type(SortValueType::Uint32)
            .with_direction(SortDirection::Ascending) // Front to back
    }
}

impl Default for GpuSorterCreateInfo {
    fn default() -> Self {
        Self::radix_sort(1024 * 1024)
    }
}

/// Sort algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SortAlgorithm {
    /// Radix sort (best for large arrays)
    #[default]
    RadixSort        = 0,
    /// Bitonic sort (power of 2 sizes)
    BitonicSort      = 1,
    /// Merge sort
    MergeSort        = 2,
    /// Small sort (optimized for small arrays)
    SmallSort        = 3,
    /// Odd-even merge sort
    OddEvenMergeSort = 4,
    /// Counting sort (for limited range keys)
    CountingSort     = 5,
}

impl SortAlgorithm {
    /// Display name
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::RadixSort => "Radix Sort",
            Self::BitonicSort => "Bitonic Sort",
            Self::MergeSort => "Merge Sort",
            Self::SmallSort => "Small Sort",
            Self::OddEvenMergeSort => "Odd-Even Merge Sort",
            Self::CountingSort => "Counting Sort",
        }
    }

    /// Is stable sort
    pub const fn is_stable(&self) -> bool {
        matches!(self, Self::RadixSort | Self::MergeSort | Self::CountingSort)
    }

    /// Requires power of 2 size
    pub const fn requires_power_of_2(&self) -> bool {
        matches!(self, Self::BitonicSort | Self::OddEvenMergeSort)
    }
}

/// Sort key type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SortKeyType {
    /// 32-bit unsigned int
    #[default]
    Uint32  = 0,
    /// 32-bit signed int
    Int32   = 1,
    /// 32-bit float
    Float32 = 2,
    /// 64-bit unsigned int
    Uint64  = 3,
    /// 64-bit signed int
    Int64   = 4,
    /// 64-bit float
    Float64 = 5,
    /// 16-bit unsigned int
    Uint16  = 6,
    /// 16-bit float (half)
    Float16 = 7,
}

impl SortKeyType {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Uint16 | Self::Float16 => 2,
            Self::Uint32 | Self::Int32 | Self::Float32 => 4,
            Self::Uint64 | Self::Int64 | Self::Float64 => 8,
        }
    }

    /// Is floating point
    pub const fn is_float(&self) -> bool {
        matches!(self, Self::Float16 | Self::Float32 | Self::Float64)
    }

    /// Is signed
    pub const fn is_signed(&self) -> bool {
        matches!(
            self,
            Self::Int32 | Self::Int64 | Self::Float16 | Self::Float32 | Self::Float64
        )
    }
}

/// Sort value type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SortValueType {
    /// 32-bit unsigned int (index)
    #[default]
    Uint32    = 0,
    /// 64-bit unsigned int
    Uint64    = 1,
    /// 32-bit float
    Float32   = 2,
    /// 4x32-bit (uvec4)
    Uint32x4  = 3,
    /// 4x32-bit float (vec4)
    Float32x4 = 4,
    /// Custom size
    Custom { size: u32 } = 5,
}

impl SortValueType {
    /// Size in bytes
    pub const fn size(&self) -> u32 {
        match self {
            Self::Uint32 | Self::Float32 => 4,
            Self::Uint64 => 8,
            Self::Uint32x4 | Self::Float32x4 => 16,
            Self::Custom { size } => *size,
        }
    }
}

/// Sort direction
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum SortDirection {
    /// Ascending order (smallest first)
    #[default]
    Ascending  = 0,
    /// Descending order (largest first)
    Descending = 1,
}

bitflags::bitflags! {
    /// Sort flags
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    #[repr(transparent)]
    pub struct SortFlags: u32 {
        /// None
        const NONE = 0;
        /// Stable sort (preserve order of equal elements)
        const STABLE = 1 << 0;
        /// In-place sort (modify input buffer)
        const IN_PLACE = 1 << 1;
        /// Partial sort (only sort first N elements)
        const PARTIAL = 1 << 2;
        /// Async sort
        const ASYNC = 1 << 3;
        /// Multi-buffer (sort multiple buffers with same keys)
        const MULTI_BUFFER = 1 << 4;
    }
}

// ============================================================================
// Sort Operations
// ============================================================================

/// Sort request
#[derive(Clone, Debug)]
pub struct SortRequest {
    /// Sorter
    pub sorter: GpuSorterHandle,
    /// Key buffer
    pub key_buffer: u64, // Buffer handle
    /// Key offset
    pub key_offset: u64,
    /// Value buffer (optional)
    pub value_buffer: Option<u64>,
    /// Value offset
    pub value_offset: u64,
    /// Element count
    pub element_count: u32,
    /// Direction override
    pub direction: Option<SortDirection>,
    /// First N elements (for partial sort)
    pub first_n: Option<u32>,
}

impl SortRequest {
    /// Creates new request
    pub fn new(sorter: GpuSorterHandle, key_buffer: u64, count: u32) -> Self {
        Self {
            sorter,
            key_buffer,
            key_offset: 0,
            value_buffer: None,
            value_offset: 0,
            element_count: count,
            direction: None,
            first_n: None,
        }
    }

    /// With key offset
    pub fn with_key_offset(mut self, offset: u64) -> Self {
        self.key_offset = offset;
        self
    }

    /// With value buffer
    pub fn with_values(mut self, buffer: u64, offset: u64) -> Self {
        self.value_buffer = Some(buffer);
        self.value_offset = offset;
        self
    }

    /// With direction
    pub fn with_direction(mut self, direction: SortDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Partial sort (first N elements)
    pub fn first_n(mut self, n: u32) -> Self {
        self.first_n = Some(n);
        self
    }
}

/// Radix sort configuration
#[derive(Clone, Copy, Debug)]
pub struct RadixSortConfig {
    /// Bits per pass
    pub bits_per_pass: u32,
    /// Workgroup size
    pub workgroup_size: u32,
    /// Items per thread
    pub items_per_thread: u32,
    /// Use local memory
    pub use_local_memory: bool,
    /// First bit to sort
    pub first_bit: u32,
    /// Last bit to sort
    pub last_bit: u32,
}

impl RadixSortConfig {
    /// Default configuration
    pub const DEFAULT: Self = Self {
        bits_per_pass: 4,
        workgroup_size: 256,
        items_per_thread: 4,
        use_local_memory: true,
        first_bit: 0,
        last_bit: 32,
    };

    /// High performance config
    pub const HIGH_PERF: Self = Self {
        bits_per_pass: 8,
        workgroup_size: 512,
        items_per_thread: 8,
        use_local_memory: true,
        first_bit: 0,
        last_bit: 32,
    };

    /// For 64-bit keys
    pub const FOR_64BIT: Self = Self {
        bits_per_pass: 8,
        workgroup_size: 256,
        items_per_thread: 4,
        use_local_memory: true,
        first_bit: 0,
        last_bit: 64,
    };

    /// Number of passes needed
    pub fn pass_count(&self) -> u32 {
        let bits = self.last_bit - self.first_bit;
        (bits + self.bits_per_pass - 1) / self.bits_per_pass
    }

    /// Histogram size
    pub fn histogram_size(&self) -> u32 {
        1 << self.bits_per_pass
    }
}

impl Default for RadixSortConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Bitonic sort configuration
#[derive(Clone, Copy, Debug)]
pub struct BitonicSortConfig {
    /// Workgroup size
    pub workgroup_size: u32,
    /// Use local memory
    pub use_local_memory: bool,
    /// Max local sort size
    pub max_local_sort_size: u32,
}

impl BitonicSortConfig {
    /// Default configuration
    pub const DEFAULT: Self = Self {
        workgroup_size: 256,
        use_local_memory: true,
        max_local_sort_size: 2048,
    };
}

impl Default for BitonicSortConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ============================================================================
// Sort GPU Data
// ============================================================================

/// Sort params for GPU
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuSortParams {
    /// Element count
    pub element_count: u32,
    /// Pass index
    pub pass_index: u32,
    /// Bits per pass
    pub bits_per_pass: u32,
    /// Bit offset
    pub bit_offset: u32,
    /// Direction (0 = ascending, 1 = descending)
    pub direction: u32,
    /// Histogram offset
    pub histogram_offset: u32,
    /// Key stride
    pub key_stride: u32,
    /// Value stride
    pub value_stride: u32,
}

impl GpuSortParams {
    /// Creates params for radix sort pass
    pub fn radix_pass(count: u32, pass: u32, config: &RadixSortConfig, ascending: bool) -> Self {
        Self {
            element_count: count,
            pass_index: pass,
            bits_per_pass: config.bits_per_pass,
            bit_offset: config.first_bit + pass * config.bits_per_pass,
            direction: if ascending { 0 } else { 1 },
            histogram_offset: 0,
            key_stride: 0,
            value_stride: 0,
        }
    }
}

/// Bitonic sort params for GPU
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct GpuBitonicParams {
    /// Element count
    pub element_count: u32,
    /// Stage (log2 of subsequence length)
    pub stage: u32,
    /// Pass within stage
    pub pass: u32,
    /// Direction (0 = ascending, 1 = descending)
    pub direction: u32,
}

impl GpuBitonicParams {
    /// Creates params for bitonic sort pass
    pub fn new(count: u32, stage: u32, pass: u32, ascending: bool) -> Self {
        Self {
            element_count: count,
            stage,
            pass,
            direction: if ascending { 0 } else { 1 },
        }
    }

    /// Sequence length
    pub fn sequence_length(&self) -> u32 {
        1 << (self.stage + 1)
    }

    /// Compare distance
    pub fn compare_distance(&self) -> u32 {
        1 << (self.stage - self.pass)
    }
}

// ============================================================================
// Indirect Sort
// ============================================================================

/// Indirect sort arguments
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct IndirectSortArgs {
    /// Element count
    pub element_count: u32,
    /// Key buffer offset
    pub key_offset: u32,
    /// Value buffer offset
    pub value_offset: u32,
    /// Flags
    pub flags: u32,
}

/// Sort dispatch info
#[derive(Clone, Debug)]
pub struct SortDispatchInfo {
    /// Workgroup count X
    pub workgroup_count_x: u32,
    /// Workgroup count Y
    pub workgroup_count_y: u32,
    /// Workgroup count Z
    pub workgroup_count_z: u32,
    /// Pass count
    pub pass_count: u32,
    /// Scratch memory size
    pub scratch_size: u64,
}

impl SortDispatchInfo {
    /// Compute dispatch info for radix sort
    pub fn compute_radix(count: u32, config: &RadixSortConfig) -> Self {
        let items_per_workgroup = config.workgroup_size * config.items_per_thread;
        let workgroups = (count + items_per_workgroup - 1) / items_per_workgroup;
        let pass_count = config.pass_count();

        // Histogram + temp buffer
        let histogram_size = config.histogram_size() as u64 * workgroups as u64 * 4;
        let temp_buffer_size = count as u64 * 8; // Keys + values

        Self {
            workgroup_count_x: workgroups,
            workgroup_count_y: 1,
            workgroup_count_z: 1,
            pass_count,
            scratch_size: histogram_size + temp_buffer_size,
        }
    }

    /// Compute dispatch info for bitonic sort
    pub fn compute_bitonic(count: u32, config: &BitonicSortConfig) -> Self {
        // Round up to power of 2
        let padded_count = count.next_power_of_two();
        let workgroups =
            (padded_count + config.workgroup_size * 2 - 1) / (config.workgroup_size * 2);

        // Number of stages = log2(padded_count)
        let stages = (padded_count as f32).log2().ceil() as u32;
        let total_passes = stages * (stages + 1) / 2;

        Self {
            workgroup_count_x: workgroups,
            workgroup_count_y: 1,
            workgroup_count_z: 1,
            pass_count: total_passes,
            scratch_size: 0, // In-place
        }
    }
}

// ============================================================================
// Multi-Buffer Sort
// ============================================================================

/// Multi-buffer sort request
#[derive(Clone, Debug)]
pub struct MultiBufferSortRequest {
    /// Key buffer
    pub key_buffer: u64,
    /// Key offset
    pub key_offset: u64,
    /// Element count
    pub element_count: u32,
    /// Value buffers to reorder
    pub value_buffers: Vec<SortValueBuffer>,
    /// Direction
    pub direction: SortDirection,
}

impl MultiBufferSortRequest {
    /// Creates new request
    pub fn new(key_buffer: u64, count: u32) -> Self {
        Self {
            key_buffer,
            key_offset: 0,
            element_count: count,
            value_buffers: Vec::new(),
            direction: SortDirection::Ascending,
        }
    }

    /// Add value buffer
    pub fn add_value_buffer(mut self, buffer: u64, offset: u64, stride: u32) -> Self {
        self.value_buffers.push(SortValueBuffer {
            buffer,
            offset,
            stride,
        });
        self
    }

    /// With direction
    pub fn with_direction(mut self, direction: SortDirection) -> Self {
        self.direction = direction;
        self
    }
}

/// Value buffer for multi-buffer sort
#[derive(Clone, Copy, Debug)]
pub struct SortValueBuffer {
    /// Buffer handle
    pub buffer: u64,
    /// Offset
    pub offset: u64,
    /// Stride
    pub stride: u32,
}

// ============================================================================
// Segmented Sort
// ============================================================================

/// Segmented sort request
#[derive(Clone, Debug)]
pub struct SegmentedSortRequest {
    /// Sorter
    pub sorter: GpuSorterHandle,
    /// Key buffer
    pub key_buffer: u64,
    /// Segment offsets buffer
    pub segment_offsets_buffer: u64,
    /// Segment count
    pub segment_count: u32,
    /// Total elements
    pub total_elements: u32,
    /// Max segment size
    pub max_segment_size: u32,
}

impl SegmentedSortRequest {
    /// Creates new request
    pub fn new(sorter: GpuSorterHandle, keys: u64, offsets: u64) -> Self {
        Self {
            sorter,
            key_buffer: keys,
            segment_offsets_buffer: offsets,
            segment_count: 0,
            total_elements: 0,
            max_segment_size: 0,
        }
    }

    /// With segment info
    pub fn with_segments(mut self, count: u32, total_elements: u32, max_size: u32) -> Self {
        self.segment_count = count;
        self.total_elements = total_elements;
        self.max_segment_size = max_size;
        self
    }
}

// ============================================================================
// Statistics
// ============================================================================

/// Sort statistics
#[derive(Clone, Debug, Default)]
pub struct SortStats {
    /// Elements sorted
    pub elements_sorted: u64,
    /// Sort operations
    pub sort_operations: u32,
    /// Total passes
    pub total_passes: u32,
    /// Total dispatch calls
    pub dispatch_calls: u32,
    /// Scratch memory used
    pub scratch_memory_used: u64,
    /// Average sort time (nanoseconds)
    pub avg_sort_time_ns: u64,
    /// Peak sort time
    pub peak_sort_time_ns: u64,
}

impl SortStats {
    /// Elements per operation
    pub fn elements_per_operation(&self) -> f32 {
        if self.sort_operations == 0 {
            return 0.0;
        }
        self.elements_sorted as f32 / self.sort_operations as f32
    }

    /// Passes per operation
    pub fn passes_per_operation(&self) -> f32 {
        if self.sort_operations == 0 {
            return 0.0;
        }
        self.total_passes as f32 / self.sort_operations as f32
    }
}
