//! GPU Memory Allocator
//!
//! High-level GPU memory allocation interface.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use bitflags::bitflags;
use lumina_core::Handle;

// ============================================================================
// Memory Location
// ============================================================================

/// Memory location preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryLocation {
    /// Unknown/auto.
    Unknown,
    /// GPU only (fastest for GPU).
    GpuOnly,
    /// CPU to GPU (upload).
    CpuToGpu,
    /// GPU to CPU (readback).
    GpuToCpu,
}

impl Default for MemoryLocation {
    fn default() -> Self {
        MemoryLocation::Unknown
    }
}

impl MemoryLocation {
    /// Check if host visible.
    pub fn is_host_visible(&self) -> bool {
        matches!(self, MemoryLocation::CpuToGpu | MemoryLocation::GpuToCpu)
    }

    /// Check if device local.
    pub fn is_device_local(&self) -> bool {
        matches!(self, MemoryLocation::GpuOnly | MemoryLocation::CpuToGpu)
    }
}

// ============================================================================
// Allocation Type
// ============================================================================

/// Allocation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocationType {
    /// Unknown.
    Unknown,
    /// Buffer allocation.
    Buffer,
    /// Image/texture allocation.
    Image,
    /// Linear image allocation.
    ImageLinear,
    /// Optimal image allocation.
    ImageOptimal,
}

impl Default for AllocationType {
    fn default() -> Self {
        AllocationType::Unknown
    }
}

// ============================================================================
// Allocation Flags
// ============================================================================

bitflags! {
    /// Allocation flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AllocationFlags: u32 {
        /// Dedicated allocation.
        const DEDICATED = 1 << 0;
        /// Mapped memory.
        const MAPPED = 1 << 1;
        /// Linear (not tiled).
        const LINEAR = 1 << 2;
        /// Can be aliased.
        const CAN_ALIAS = 1 << 3;
        /// Within budget.
        const WITHIN_BUDGET = 1 << 4;
        /// Host access sequential write.
        const HOST_ACCESS_SEQUENTIAL_WRITE = 1 << 5;
        /// Host access random.
        const HOST_ACCESS_RANDOM = 1 << 6;
    }
}

impl Default for AllocationFlags {
    fn default() -> Self {
        AllocationFlags::empty()
    }
}

// ============================================================================
// Allocation Description
// ============================================================================

/// Description for allocation.
#[derive(Debug, Clone)]
pub struct AllocationDesc {
    /// Size in bytes.
    pub size: u64,
    /// Alignment requirement.
    pub alignment: u64,
    /// Memory location.
    pub location: MemoryLocation,
    /// Allocation type.
    pub allocation_type: AllocationType,
    /// Allocation flags.
    pub flags: AllocationFlags,
    /// Debug name.
    pub name: Option<String>,
}

impl Default for AllocationDesc {
    fn default() -> Self {
        Self {
            size: 0,
            alignment: 1,
            location: MemoryLocation::Unknown,
            allocation_type: AllocationType::Unknown,
            flags: AllocationFlags::empty(),
            name: None,
        }
    }
}

impl AllocationDesc {
    /// Create a new allocation description.
    pub fn new(size: u64) -> Self {
        Self {
            size,
            ..Default::default()
        }
    }

    /// Set alignment.
    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set location.
    pub fn with_location(mut self, location: MemoryLocation) -> Self {
        self.location = location;
        self
    }

    /// Set allocation type.
    pub fn with_type(mut self, allocation_type: AllocationType) -> Self {
        self.allocation_type = allocation_type;
        self
    }

    /// Set flags.
    pub fn with_flags(mut self, flags: AllocationFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set as dedicated.
    pub fn dedicated(mut self) -> Self {
        self.flags |= AllocationFlags::DEDICATED;
        self
    }

    /// Set as mapped.
    pub fn mapped(mut self) -> Self {
        self.flags |= AllocationFlags::MAPPED;
        self
    }

    /// Create for buffer.
    pub fn buffer(size: u64, location: MemoryLocation) -> Self {
        Self {
            size,
            allocation_type: AllocationType::Buffer,
            location,
            ..Default::default()
        }
    }

    /// Create for image.
    pub fn image(size: u64, location: MemoryLocation) -> Self {
        Self {
            size,
            allocation_type: AllocationType::Image,
            location,
            ..Default::default()
        }
    }

    /// Create for upload buffer.
    pub fn upload_buffer(size: u64) -> Self {
        Self::buffer(size, MemoryLocation::CpuToGpu)
            .with_flags(AllocationFlags::MAPPED | AllocationFlags::HOST_ACCESS_SEQUENTIAL_WRITE)
    }

    /// Create for readback buffer.
    pub fn readback_buffer(size: u64) -> Self {
        Self::buffer(size, MemoryLocation::GpuToCpu)
            .with_flags(AllocationFlags::MAPPED | AllocationFlags::HOST_ACCESS_RANDOM)
    }
}

// ============================================================================
// Allocation Handle
// ============================================================================

/// Handle to an allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AllocationHandle(Handle<Allocation>);

impl AllocationHandle {
    /// Create a new handle.
    pub fn new(index: u32, generation: u32) -> Self {
        Self(Handle::from_raw_parts(index, generation))
    }

    /// Get the index.
    pub fn index(&self) -> u32 {
        self.0.index()
    }

    /// Get the generation.
    pub fn generation(&self) -> u32 {
        self.0.generation()
    }

    /// Invalid handle.
    pub const INVALID: Self = Self(Handle::INVALID);
}

// ============================================================================
// Allocation Info
// ============================================================================

/// Allocation information.
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    /// Offset in memory.
    pub offset: u64,
    /// Size in bytes.
    pub size: u64,
    /// Memory type index.
    pub memory_type_index: u32,
    /// Is dedicated allocation.
    pub is_dedicated: bool,
    /// Mapped pointer (if mapped).
    pub mapped_ptr: Option<*mut u8>,
    /// Debug name.
    pub name: Option<String>,
}

impl AllocationInfo {
    /// Check if mapped.
    pub fn is_mapped(&self) -> bool {
        self.mapped_ptr.is_some()
    }

    /// Get end offset.
    pub fn end_offset(&self) -> u64 {
        self.offset + self.size
    }
}

// ============================================================================
// Allocation
// ============================================================================

/// A GPU memory allocation.
pub struct Allocation {
    /// Handle.
    pub handle: AllocationHandle,
    /// Allocation info.
    pub info: AllocationInfo,
    /// Allocation type.
    pub allocation_type: AllocationType,
    /// Memory location.
    pub location: MemoryLocation,
    /// Frame created.
    pub created_frame: u64,
    /// Last used frame.
    pub last_used_frame: AtomicU64,
}

impl Allocation {
    /// Create a new allocation.
    pub fn new(
        handle: AllocationHandle,
        info: AllocationInfo,
        allocation_type: AllocationType,
        location: MemoryLocation,
        created_frame: u64,
    ) -> Self {
        Self {
            handle,
            info,
            allocation_type,
            location,
            created_frame,
            last_used_frame: AtomicU64::new(created_frame),
        }
    }

    /// Mark as used.
    pub fn mark_used(&self, frame: u64) {
        self.last_used_frame.store(frame, Ordering::Relaxed);
    }

    /// Get size.
    pub fn size(&self) -> u64 {
        self.info.size
    }

    /// Get offset.
    pub fn offset(&self) -> u64 {
        self.info.offset
    }

    /// Check if mappable.
    pub fn is_mappable(&self) -> bool {
        self.location.is_host_visible()
    }

    /// Get mapped pointer.
    pub fn mapped_ptr(&self) -> Option<*mut u8> {
        self.info.mapped_ptr
    }
}

// ============================================================================
// Allocator Statistics
// ============================================================================

/// Allocator statistics.
#[derive(Debug, Clone, Default)]
pub struct AllocatorStatistics {
    /// Total allocations.
    pub total_allocations: u64,
    /// Total deallocations.
    pub total_deallocations: u64,
    /// Active allocations.
    pub active_allocations: u32,
    /// Total memory allocated.
    pub total_allocated: u64,
    /// Total memory in use.
    pub used_memory: u64,
    /// Peak memory used.
    pub peak_memory: u64,
    /// Dedicated allocation count.
    pub dedicated_allocations: u32,
    /// Block count.
    pub block_count: u32,
    /// Fragmentation ratio (0-1).
    pub fragmentation: f32,
}

impl AllocatorStatistics {
    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.total_allocated == 0 {
            0.0
        } else {
            self.used_memory as f32 / self.total_allocated as f32
        }
    }

    /// Get waste ratio.
    pub fn waste_ratio(&self) -> f32 {
        1.0 - self.utilization()
    }
}

// ============================================================================
// Memory Type Statistics
// ============================================================================

/// Per-memory-type statistics.
#[derive(Debug, Clone, Default)]
pub struct MemoryTypeStatistics {
    /// Memory type index.
    pub memory_type_index: u32,
    /// Allocation count.
    pub allocation_count: u32,
    /// Block count.
    pub block_count: u32,
    /// Total allocated.
    pub total_allocated: u64,
    /// Used memory.
    pub used_memory: u64,
}

// ============================================================================
// GPU Allocator
// ============================================================================

/// GPU memory allocator.
pub struct GpuAllocator {
    /// Allocations.
    allocations: Vec<Option<Allocation>>,
    /// Free indices.
    free_indices: Vec<u32>,
    /// Generations.
    generations: Vec<u32>,
    /// Statistics.
    stats: AllocatorStatistics,
    /// Current frame.
    current_frame: AtomicU64,
    /// Device memory limit.
    device_memory_limit: u64,
    /// Host memory limit.
    host_memory_limit: u64,
}

impl GpuAllocator {
    /// Create a new allocator.
    pub fn new(device_memory_limit: u64, host_memory_limit: u64) -> Self {
        Self {
            allocations: Vec::new(),
            free_indices: Vec::new(),
            generations: Vec::new(),
            stats: AllocatorStatistics::default(),
            current_frame: AtomicU64::new(0),
            device_memory_limit,
            host_memory_limit,
        }
    }

    /// Allocate memory.
    pub fn allocate(&mut self, desc: &AllocationDesc) -> Option<AllocationHandle> {
        // Check memory limits
        let limit = if desc.location == MemoryLocation::GpuOnly {
            self.device_memory_limit
        } else {
            self.host_memory_limit
        };

        if self.stats.used_memory + desc.size > limit {
            return None;
        }

        let index = if let Some(index) = self.free_indices.pop() {
            index
        } else {
            let index = self.allocations.len() as u32;
            self.allocations.push(None);
            self.generations.push(0);
            index
        };

        let generation = self.generations[index as usize];
        let handle = AllocationHandle::new(index, generation);

        let info = AllocationInfo {
            offset: self.stats.used_memory, // Simplified
            size: desc.size,
            memory_type_index: 0,
            is_dedicated: desc.flags.contains(AllocationFlags::DEDICATED),
            mapped_ptr: if desc.flags.contains(AllocationFlags::MAPPED) {
                Some(core::ptr::null_mut()) // Placeholder
            } else {
                None
            },
            name: desc.name.clone(),
        };

        let frame = self.current_frame.load(Ordering::Relaxed);
        let allocation = Allocation::new(handle, info, desc.allocation_type, desc.location, frame);

        self.allocations[index as usize] = Some(allocation);

        // Update statistics
        self.stats.total_allocations += 1;
        self.stats.active_allocations += 1;
        self.stats.used_memory += desc.size;
        self.stats.total_allocated += desc.size;
        self.stats.peak_memory = self.stats.peak_memory.max(self.stats.used_memory);
        if desc.flags.contains(AllocationFlags::DEDICATED) {
            self.stats.dedicated_allocations += 1;
        }

        Some(handle)
    }

    /// Free memory.
    pub fn free(&mut self, handle: AllocationHandle) {
        let index = handle.index() as usize;
        if index >= self.allocations.len() {
            return;
        }
        if self.generations[index] != handle.generation() {
            return;
        }

        if let Some(allocation) = self.allocations[index].take() {
            self.stats.total_deallocations += 1;
            self.stats.active_allocations = self.stats.active_allocations.saturating_sub(1);
            self.stats.used_memory = self.stats.used_memory.saturating_sub(allocation.size());
            if allocation.info.is_dedicated {
                self.stats.dedicated_allocations =
                    self.stats.dedicated_allocations.saturating_sub(1);
            }
        }

        self.generations[index] = self.generations[index].wrapping_add(1);
        self.free_indices.push(index as u32);
    }

    /// Get allocation.
    pub fn get(&self, handle: AllocationHandle) -> Option<&Allocation> {
        let index = handle.index() as usize;
        if index >= self.allocations.len() {
            return None;
        }
        if self.generations[index] != handle.generation() {
            return None;
        }
        self.allocations[index].as_ref()
    }

    /// Get allocation info.
    pub fn get_info(&self, handle: AllocationHandle) -> Option<&AllocationInfo> {
        self.get(handle).map(|a| &a.info)
    }

    /// Get statistics.
    pub fn statistics(&self) -> &AllocatorStatistics {
        &self.stats
    }

    /// Advance frame.
    pub fn advance_frame(&mut self) {
        self.current_frame.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current frame.
    pub fn current_frame(&self) -> u64 {
        self.current_frame.load(Ordering::Relaxed)
    }

    /// Find unused allocations.
    pub fn find_unused(&self, frames_threshold: u64) -> Vec<AllocationHandle> {
        let current = self.current_frame();
        self.allocations
            .iter()
            .filter_map(|a| a.as_ref())
            .filter(|a| {
                current.saturating_sub(a.last_used_frame.load(Ordering::Relaxed)) > frames_threshold
            })
            .map(|a| a.handle)
            .collect()
    }

    /// Get memory budget.
    pub fn budget(&self, location: MemoryLocation) -> MemoryBudget {
        let limit = if location == MemoryLocation::GpuOnly {
            self.device_memory_limit
        } else {
            self.host_memory_limit
        };

        MemoryBudget {
            budget: limit,
            usage: self.stats.used_memory,
        }
    }
}

impl Default for GpuAllocator {
    fn default() -> Self {
        Self::new(
            4 * 1024 * 1024 * 1024, // 4GB device
            2 * 1024 * 1024 * 1024, // 2GB host
        )
    }
}

// ============================================================================
// Memory Budget
// ============================================================================

/// Memory budget information.
#[derive(Debug, Clone, Copy)]
pub struct MemoryBudget {
    /// Budget limit.
    pub budget: u64,
    /// Current usage.
    pub usage: u64,
}

impl MemoryBudget {
    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.budget.saturating_sub(self.usage)
    }

    /// Get usage ratio.
    pub fn usage_ratio(&self) -> f32 {
        if self.budget == 0 {
            0.0
        } else {
            self.usage as f32 / self.budget as f32
        }
    }

    /// Check if under budget.
    pub fn is_under_budget(&self) -> bool {
        self.usage <= self.budget
    }
}

// ============================================================================
// Defragmentation
// ============================================================================

/// Defragmentation result.
#[derive(Debug, Clone, Default)]
pub struct DefragmentationResult {
    /// Allocations moved.
    pub allocations_moved: u32,
    /// Bytes moved.
    pub bytes_moved: u64,
    /// Bytes freed.
    pub bytes_freed: u64,
    /// Blocks freed.
    pub blocks_freed: u32,
}

/// Defragmentation settings.
#[derive(Debug, Clone)]
pub struct DefragmentationSettings {
    /// Maximum allocations to move.
    pub max_allocations_to_move: u32,
    /// Maximum bytes to move.
    pub max_bytes_to_move: u64,
}

impl Default for DefragmentationSettings {
    fn default() -> Self {
        Self {
            max_allocations_to_move: 100,
            max_bytes_to_move: 64 * 1024 * 1024, // 64MB
        }
    }
}
