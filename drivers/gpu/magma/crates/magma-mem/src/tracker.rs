//! # Allocation Tracker
//!
//! Track buffer objects and their lifetime for GPU timeline safety.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use magma_core::{Error, Result, GpuAddr, ByteSize, Handle};

use crate::heap::HeapAllocation;

// =============================================================================
// BUFFER OBJECT
// =============================================================================

/// Buffer object ID
pub type BufferId = Handle<BufferObject>;

/// A tracked buffer object
#[derive(Debug)]
pub struct BufferObject {
    /// Unique ID
    id: BufferId,
    /// GPU address
    addr: GpuAddr,
    /// Size
    size: ByteSize,
    /// Reference count
    refcount: u32,
    /// Pending GPU uses (fence values)
    pending_uses: Vec<u64>,
    /// Debug name
    name: Option<alloc::string::String>,
}

impl BufferObject {
    /// Get buffer ID
    pub fn id(&self) -> BufferId {
        self.id
    }

    /// Get GPU address
    pub fn addr(&self) -> GpuAddr {
        self.addr
    }

    /// Get size
    pub fn size(&self) -> ByteSize {
        self.size
    }

    /// Get reference count
    pub fn refcount(&self) -> u32 {
        self.refcount
    }

    /// Check if buffer has pending GPU uses
    pub fn has_pending_uses(&self) -> bool {
        !self.pending_uses.is_empty()
    }

    /// Get debug name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

// =============================================================================
// ALLOCATION TRACKER
// =============================================================================

/// Tracks buffer object allocations and their lifecycle
#[derive(Debug)]
pub struct AllocationTracker {
    /// All tracked buffers
    buffers: BTreeMap<BufferId, BufferObject>,
    /// Next buffer ID
    next_id: u64,
    /// Pending frees (waiting for GPU)
    pending_frees: Vec<PendingFree>,
    /// Current GPU fence value
    current_fence: u64,
    /// Statistics
    stats: TrackerStats,
}

/// A buffer pending free (waiting for GPU timeline)
#[derive(Debug)]
struct PendingFree {
    buffer: BufferObject,
    fence_value: u64,
}

/// Tracker statistics
#[derive(Debug, Clone, Default)]
pub struct TrackerStats {
    /// Total buffers created
    pub total_created: u64,
    /// Total buffers destroyed
    pub total_destroyed: u64,
    /// Current active buffers
    pub active_buffers: u64,
    /// Current pending frees
    pub pending_frees: u64,
    /// Total memory in use
    pub memory_in_use: u64,
}

impl AllocationTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            buffers: BTreeMap::new(),
            next_id: 1,
            pending_frees: Vec::new(),
            current_fence: 0,
            stats: TrackerStats::default(),
        }
    }

    /// Register a new buffer
    pub fn register(
        &mut self,
        alloc: HeapAllocation,
        name: Option<&str>,
    ) -> BufferId {
        let id = BufferId::new(self.next_id);
        self.next_id += 1;

        let buffer = BufferObject {
            id,
            addr: alloc.addr,
            size: alloc.size,
            refcount: 1,
            pending_uses: Vec::new(),
            name: name.map(|s| alloc::string::String::from(s)),
        };

        self.stats.total_created += 1;
        self.stats.active_buffers += 1;
        self.stats.memory_in_use += alloc.size.as_bytes();

        self.buffers.insert(id, buffer);

        id
    }

    /// Get buffer by ID
    pub fn get(&self, id: BufferId) -> Option<&BufferObject> {
        self.buffers.get(&id)
    }

    /// Increment reference count
    pub fn add_ref(&mut self, id: BufferId) -> Result<()> {
        let buffer = self.buffers.get_mut(&id).ok_or(Error::NotFound)?;
        buffer.refcount += 1;
        Ok(())
    }

    /// Decrement reference count
    pub fn release(&mut self, id: BufferId) -> Result<Option<HeapAllocation>> {
        let buffer = self.buffers.get_mut(&id).ok_or(Error::NotFound)?;
        buffer.refcount -= 1;

        if buffer.refcount == 0 {
            let buffer = self.buffers.remove(&id).unwrap();

            if buffer.has_pending_uses() {
                // Defer free until GPU is done
                let max_fence = *buffer.pending_uses.iter().max().unwrap();
                self.pending_frees.push(PendingFree {
                    buffer,
                    fence_value: max_fence,
                });
                self.stats.pending_frees += 1;
                Ok(None)
            } else {
                // Can free immediately
                self.stats.total_destroyed += 1;
                self.stats.active_buffers -= 1;
                self.stats.memory_in_use -= buffer.size.as_bytes();

                Ok(Some(HeapAllocation {
                    addr: buffer.addr,
                    size: buffer.size,
                    heap_type: crate::heap::HeapType::DeviceLocal,
                }))
            }
        } else {
            Ok(None)
        }
    }

    /// Mark buffer as used at a fence value
    pub fn mark_used(&mut self, id: BufferId, fence_value: u64) -> Result<()> {
        let buffer = self.buffers.get_mut(&id).ok_or(Error::NotFound)?;
        buffer.pending_uses.push(fence_value);
        Ok(())
    }

    /// Update current fence value and process pending frees
    pub fn update_fence(&mut self, fence_value: u64) -> Vec<HeapAllocation> {
        self.current_fence = fence_value;

        let mut freed = Vec::new();
        let mut i = 0;

        while i < self.pending_frees.len() {
            if self.pending_frees[i].fence_value <= fence_value {
                let pending = self.pending_frees.swap_remove(i);
                self.stats.total_destroyed += 1;
                self.stats.active_buffers -= 1;
                self.stats.memory_in_use -= pending.buffer.size.as_bytes();
                self.stats.pending_frees -= 1;

                freed.push(HeapAllocation {
                    addr: pending.buffer.addr,
                    size: pending.buffer.size,
                    heap_type: crate::heap::HeapType::DeviceLocal,
                });
            } else {
                i += 1;
            }
        }

        // Also update pending uses in active buffers
        for buffer in self.buffers.values_mut() {
            buffer.pending_uses.retain(|&v| v > fence_value);
        }

        freed
    }

    /// Get statistics
    pub fn stats(&self) -> &TrackerStats {
        &self.stats
    }

    /// Get number of active buffers
    pub fn active_count(&self) -> usize {
        self.buffers.len()
    }

    /// Iterate over all buffers
    pub fn iter(&self) -> impl Iterator<Item = &BufferObject> {
        self.buffers.values()
    }
}

impl Default for AllocationTracker {
    fn default() -> Self {
        Self::new()
    }
}
