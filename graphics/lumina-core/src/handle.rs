//! Handle management for GPU resources
//!
//! This module provides the handle allocation and validation system.

use alloc::vec::Vec;

/// A generational handle allocator
///
/// This allocator provides type-safe handles with generation counting
/// to detect use-after-free bugs.
pub struct HandleAllocator<T> {
    /// Generation counter for each slot
    generations: Vec<u32>,
    /// Free list of available slots
    free_list: Vec<u32>,
    /// Phantom marker
    _marker: core::marker::PhantomData<T>,
}

impl<T> HandleAllocator<T> {
    /// Creates a new handle allocator with initial capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            generations: Vec::with_capacity(capacity),
            free_list: Vec::new(),
            _marker: core::marker::PhantomData,
        }
    }

    /// Allocates a new handle
    pub fn allocate(&mut self) -> crate::types::Handle<T> {
        if let Some(id) = self.free_list.pop() {
            // Reuse a freed slot
            let generation = self.generations[id as usize];
            crate::types::Handle::new(id, generation)
        } else {
            // Allocate a new slot
            let id = self.generations.len() as u32;
            self.generations.push(0);
            crate::types::Handle::new(id, 0)
        }
    }

    /// Frees a handle
    ///
    /// Returns true if the handle was valid and is now freed.
    pub fn free(&mut self, handle: crate::types::Handle<T>) -> bool {
        let id = handle.id() as usize;

        if id >= self.generations.len() {
            return false;
        }

        if self.generations[id] != handle.generation() {
            return false;
        }

        // Increment generation to invalidate existing handles
        self.generations[id] = self.generations[id].wrapping_add(1);
        self.free_list.push(id as u32);

        true
    }

    /// Checks if a handle is valid
    pub fn is_valid(&self, handle: crate::types::Handle<T>) -> bool {
        let id = handle.id() as usize;

        if id >= self.generations.len() {
            return false;
        }

        self.generations[id] == handle.generation()
    }

    /// Returns the number of active handles
    pub fn active_count(&self) -> usize {
        self.generations.len() - self.free_list.len()
    }
}

impl<T> Default for HandleAllocator<T> {
    fn default() -> Self {
        Self::new(64)
    }
}
