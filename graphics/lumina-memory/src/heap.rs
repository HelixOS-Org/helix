//! Memory Heap Management
//!
//! GPU memory heaps represent physical memory regions.

use alloc::string::String;
use alloc::vec::Vec;

use bitflags::bitflags;
use lumina_core::Handle;

// ============================================================================
// Heap Flags
// ============================================================================

bitflags! {
    /// Memory heap flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HeapFlags: u32 {
        /// Device local memory.
        const DEVICE_LOCAL = 1 << 0;
        /// Host visible memory.
        const HOST_VISIBLE = 1 << 1;
        /// Host coherent memory.
        const HOST_COHERENT = 1 << 2;
        /// Host cached memory.
        const HOST_CACHED = 1 << 3;
        /// Lazily allocated.
        const LAZILY_ALLOCATED = 1 << 4;
        /// Protected memory.
        const PROTECTED = 1 << 5;
        /// Multi-instance heap.
        const MULTI_INSTANCE = 1 << 6;
    }
}

impl Default for HeapFlags {
    fn default() -> Self {
        HeapFlags::empty()
    }
}

impl HeapFlags {
    /// Device local.
    pub fn device_local() -> Self {
        HeapFlags::DEVICE_LOCAL
    }

    /// Host visible and coherent.
    pub fn host_visible_coherent() -> Self {
        HeapFlags::HOST_VISIBLE | HeapFlags::HOST_COHERENT
    }

    /// Host visible, coherent and cached.
    pub fn host_visible_cached() -> Self {
        HeapFlags::HOST_VISIBLE | HeapFlags::HOST_COHERENT | HeapFlags::HOST_CACHED
    }
}

// ============================================================================
// Heap Type
// ============================================================================

/// Memory heap type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeapType {
    /// Device local (VRAM).
    DeviceLocal,
    /// Upload heap (CPU -> GPU).
    Upload,
    /// Readback heap (GPU -> CPU).
    Readback,
    /// Custom heap.
    Custom,
}

impl Default for HeapType {
    fn default() -> Self {
        HeapType::DeviceLocal
    }
}

impl HeapType {
    /// Get heap flags for this type.
    pub fn flags(&self) -> HeapFlags {
        match self {
            HeapType::DeviceLocal => HeapFlags::DEVICE_LOCAL,
            HeapType::Upload => HeapFlags::HOST_VISIBLE | HeapFlags::HOST_COHERENT,
            HeapType::Readback => {
                HeapFlags::HOST_VISIBLE | HeapFlags::HOST_COHERENT | HeapFlags::HOST_CACHED
            },
            HeapType::Custom => HeapFlags::empty(),
        }
    }
}

// ============================================================================
// Heap Handle
// ============================================================================

/// Handle to a memory heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeapHandle(Handle<MemoryHeap>);

impl HeapHandle {
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
// Memory Heap
// ============================================================================

/// A memory heap.
pub struct MemoryHeap {
    /// Handle.
    pub handle: HeapHandle,
    /// Heap size.
    pub size: u64,
    /// Used memory.
    pub used: u64,
    /// Heap type.
    pub heap_type: HeapType,
    /// Heap flags.
    pub flags: HeapFlags,
    /// Heap index (backend-specific).
    pub heap_index: u32,
    /// Debug name.
    pub name: Option<String>,
}

impl MemoryHeap {
    /// Create a new memory heap.
    pub fn new(
        handle: HeapHandle,
        size: u64,
        heap_type: HeapType,
        flags: HeapFlags,
        heap_index: u32,
    ) -> Self {
        Self {
            handle,
            size,
            used: 0,
            heap_type,
            flags,
            heap_index,
            name: None,
        }
    }

    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.used)
    }

    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            self.used as f32 / self.size as f32
        }
    }

    /// Check if device local.
    pub fn is_device_local(&self) -> bool {
        self.flags.contains(HeapFlags::DEVICE_LOCAL)
    }

    /// Check if host visible.
    pub fn is_host_visible(&self) -> bool {
        self.flags.contains(HeapFlags::HOST_VISIBLE)
    }

    /// Check if host coherent.
    pub fn is_host_coherent(&self) -> bool {
        self.flags.contains(HeapFlags::HOST_COHERENT)
    }

    /// Check if host cached.
    pub fn is_host_cached(&self) -> bool {
        self.flags.contains(HeapFlags::HOST_CACHED)
    }

    /// Allocate from heap.
    pub fn allocate(&mut self, size: u64) -> bool {
        if self.available() >= size {
            self.used += size;
            true
        } else {
            false
        }
    }

    /// Free from heap.
    pub fn free(&mut self, size: u64) {
        self.used = self.used.saturating_sub(size);
    }
}

// ============================================================================
// Heap Info
// ============================================================================

/// Memory heap information.
#[derive(Debug, Clone)]
pub struct HeapInfo {
    /// Heap size.
    pub size: u64,
    /// Used memory.
    pub used: u64,
    /// Heap type.
    pub heap_type: HeapType,
    /// Heap flags.
    pub flags: HeapFlags,
    /// Heap index.
    pub heap_index: u32,
}

impl HeapInfo {
    /// Create from heap.
    pub fn from_heap(heap: &MemoryHeap) -> Self {
        Self {
            size: heap.size,
            used: heap.used,
            heap_type: heap.heap_type,
            flags: heap.flags,
            heap_index: heap.heap_index,
        }
    }

    /// Get available memory.
    pub fn available(&self) -> u64 {
        self.size.saturating_sub(self.used)
    }

    /// Get utilization ratio.
    pub fn utilization(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            self.used as f32 / self.size as f32
        }
    }
}

// ============================================================================
// Memory Type
// ============================================================================

/// Memory type.
#[derive(Debug, Clone)]
pub struct MemoryType {
    /// Memory type index.
    pub index: u32,
    /// Heap index.
    pub heap_index: u32,
    /// Property flags.
    pub flags: HeapFlags,
}

impl MemoryType {
    /// Create a new memory type.
    pub fn new(index: u32, heap_index: u32, flags: HeapFlags) -> Self {
        Self {
            index,
            heap_index,
            flags,
        }
    }

    /// Check if device local.
    pub fn is_device_local(&self) -> bool {
        self.flags.contains(HeapFlags::DEVICE_LOCAL)
    }

    /// Check if host visible.
    pub fn is_host_visible(&self) -> bool {
        self.flags.contains(HeapFlags::HOST_VISIBLE)
    }

    /// Check if suitable for upload.
    pub fn is_upload_suitable(&self) -> bool {
        self.flags.contains(HeapFlags::HOST_VISIBLE)
            && self.flags.contains(HeapFlags::HOST_COHERENT)
    }

    /// Check if suitable for readback.
    pub fn is_readback_suitable(&self) -> bool {
        self.flags.contains(HeapFlags::HOST_VISIBLE) && self.flags.contains(HeapFlags::HOST_CACHED)
    }
}

// ============================================================================
// Memory Properties
// ============================================================================

/// GPU memory properties.
#[derive(Debug, Clone)]
pub struct MemoryProperties {
    /// Memory heaps.
    pub heaps: Vec<HeapInfo>,
    /// Memory types.
    pub types: Vec<MemoryType>,
}

impl Default for MemoryProperties {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryProperties {
    /// Create default memory properties.
    pub fn new() -> Self {
        Self {
            heaps: vec![
                HeapInfo {
                    size: 4 * 1024 * 1024 * 1024, // 4GB VRAM
                    used: 0,
                    heap_type: HeapType::DeviceLocal,
                    flags: HeapFlags::DEVICE_LOCAL,
                    heap_index: 0,
                },
                HeapInfo {
                    size: 16 * 1024 * 1024 * 1024, // 16GB system
                    used: 0,
                    heap_type: HeapType::Upload,
                    flags: HeapFlags::HOST_VISIBLE | HeapFlags::HOST_COHERENT,
                    heap_index: 1,
                },
            ],
            types: vec![
                MemoryType::new(0, 0, HeapFlags::DEVICE_LOCAL),
                MemoryType::new(1, 1, HeapFlags::HOST_VISIBLE | HeapFlags::HOST_COHERENT),
                MemoryType::new(
                    2,
                    1,
                    HeapFlags::HOST_VISIBLE | HeapFlags::HOST_COHERENT | HeapFlags::HOST_CACHED,
                ),
            ],
        }
    }

    /// Find memory type.
    pub fn find_memory_type(
        &self,
        type_bits: u32,
        required_flags: HeapFlags,
        preferred_flags: HeapFlags,
    ) -> Option<u32> {
        // First try with preferred flags
        let preferred = self.types.iter().find(|t| {
            (type_bits & (1 << t.index)) != 0
                && t.flags.contains(required_flags)
                && t.flags.contains(preferred_flags)
        });

        if let Some(t) = preferred {
            return Some(t.index);
        }

        // Fall back to required only
        self.types
            .iter()
            .find(|t| (type_bits & (1 << t.index)) != 0 && t.flags.contains(required_flags))
            .map(|t| t.index)
    }

    /// Find device local memory type.
    pub fn find_device_local(&self, type_bits: u32) -> Option<u32> {
        self.find_memory_type(type_bits, HeapFlags::DEVICE_LOCAL, HeapFlags::empty())
    }

    /// Find upload memory type.
    pub fn find_upload(&self, type_bits: u32) -> Option<u32> {
        self.find_memory_type(
            type_bits,
            HeapFlags::HOST_VISIBLE | HeapFlags::HOST_COHERENT,
            HeapFlags::empty(),
        )
    }

    /// Find readback memory type.
    pub fn find_readback(&self, type_bits: u32) -> Option<u32> {
        self.find_memory_type(
            type_bits,
            HeapFlags::HOST_VISIBLE | HeapFlags::HOST_COHERENT,
            HeapFlags::HOST_CACHED,
        )
    }

    /// Get total VRAM.
    pub fn total_vram(&self) -> u64 {
        self.heaps
            .iter()
            .filter(|h| h.flags.contains(HeapFlags::DEVICE_LOCAL))
            .map(|h| h.size)
            .sum()
    }

    /// Get total system memory.
    pub fn total_system_memory(&self) -> u64 {
        self.heaps
            .iter()
            .filter(|h| !h.flags.contains(HeapFlags::DEVICE_LOCAL))
            .map(|h| h.size)
            .sum()
    }
}

// ============================================================================
// Heap Manager
// ============================================================================

/// Memory heap manager.
pub struct HeapManager {
    /// Heaps.
    heaps: Vec<MemoryHeap>,
    /// Memory properties.
    properties: MemoryProperties,
}

impl HeapManager {
    /// Create a new heap manager.
    pub fn new(properties: MemoryProperties) -> Self {
        let heaps = properties
            .heaps
            .iter()
            .enumerate()
            .map(|(i, info)| {
                MemoryHeap::new(
                    HeapHandle::new(i as u32, 0),
                    info.size,
                    info.heap_type,
                    info.flags,
                    info.heap_index,
                )
            })
            .collect();

        Self { heaps, properties }
    }

    /// Get heap.
    pub fn get(&self, index: u32) -> Option<&MemoryHeap> {
        self.heaps.get(index as usize)
    }

    /// Get heap (mutable).
    pub fn get_mut(&mut self, index: u32) -> Option<&mut MemoryHeap> {
        self.heaps.get_mut(index as usize)
    }

    /// Get memory properties.
    pub fn properties(&self) -> &MemoryProperties {
        &self.properties
    }

    /// Get heap count.
    pub fn heap_count(&self) -> usize {
        self.heaps.len()
    }

    /// Get device local heap.
    pub fn device_local_heap(&self) -> Option<&MemoryHeap> {
        self.heaps.iter().find(|h| h.is_device_local())
    }

    /// Get upload heap.
    pub fn upload_heap(&self) -> Option<&MemoryHeap> {
        self.heaps
            .iter()
            .find(|h| h.is_host_visible() && h.is_host_coherent() && !h.is_device_local())
    }

    /// Get total memory.
    pub fn total_memory(&self) -> u64 {
        self.heaps.iter().map(|h| h.size).sum()
    }

    /// Get used memory.
    pub fn used_memory(&self) -> u64 {
        self.heaps.iter().map(|h| h.used).sum()
    }

    /// Get available memory.
    pub fn available_memory(&self) -> u64 {
        self.heaps.iter().map(|h| h.available()).sum()
    }

    /// Allocate from heap.
    pub fn allocate(&mut self, heap_index: u32, size: u64) -> bool {
        if let Some(heap) = self.heaps.get_mut(heap_index as usize) {
            heap.allocate(size)
        } else {
            false
        }
    }

    /// Free from heap.
    pub fn free(&mut self, heap_index: u32, size: u64) {
        if let Some(heap) = self.heaps.get_mut(heap_index as usize) {
            heap.free(size);
        }
    }
}

impl Default for HeapManager {
    fn default() -> Self {
        Self::new(MemoryProperties::default())
    }
}

// ============================================================================
// Heap Budget
// ============================================================================

/// Memory budget per heap.
#[derive(Debug, Clone, Copy)]
pub struct HeapBudget {
    /// Heap index.
    pub heap_index: u32,
    /// Total budget.
    pub budget: u64,
    /// Current usage.
    pub usage: u64,
}

impl HeapBudget {
    /// Get available budget.
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

    /// Check if over budget.
    pub fn is_over_budget(&self) -> bool {
        self.usage > self.budget
    }
}

/// Budget query.
#[derive(Debug, Clone)]
pub struct BudgetQuery {
    /// Budgets per heap.
    pub budgets: Vec<HeapBudget>,
}

impl BudgetQuery {
    /// Get total budget.
    pub fn total_budget(&self) -> u64 {
        self.budgets.iter().map(|b| b.budget).sum()
    }

    /// Get total usage.
    pub fn total_usage(&self) -> u64 {
        self.budgets.iter().map(|b| b.usage).sum()
    }

    /// Get total available.
    pub fn total_available(&self) -> u64 {
        self.budgets.iter().map(|b| b.available()).sum()
    }

    /// Check if any heap is over budget.
    pub fn is_over_budget(&self) -> bool {
        self.budgets.iter().any(|b| b.is_over_budget())
    }
}
