//! Free extent list for tracking large contiguous free regions.
//!
//! Uses a sorted list of extents for fast lookup and coalescing.

use crate::core::error::{HfsError, HfsResult};
use core::cmp::Ordering;

// ============================================================================
// Free Extent Entry
// ============================================================================

/// Entry in the free extent list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FreeExtent {
    /// Starting block
    pub start: u64,
    /// Length in blocks
    pub length: u64,
}

impl FreeExtent {
    /// Create new free extent
    #[inline]
    pub const fn new(start: u64, length: u64) -> Self {
        Self { start, length }
    }
    
    /// Get end block (exclusive)
    #[inline]
    pub fn end(&self) -> u64 {
        self.start + self.length
    }
    
    /// Check if this extent contains a block
    #[inline]
    pub fn contains(&self, block: u64) -> bool {
        block >= self.start && block < self.end()
    }
    
    /// Check if this extent can be coalesced with another
    pub fn can_coalesce(&self, other: &Self) -> bool {
        // Check if adjacent or overlapping
        self.end() >= other.start && other.end() >= self.start
    }
    
    /// Coalesce with another extent
    pub fn coalesce(&self, other: &Self) -> Option<Self> {
        if !self.can_coalesce(other) {
            return None;
        }
        
        let new_start = self.start.min(other.start);
        let new_end = self.end().max(other.end());
        
        Some(Self {
            start: new_start,
            length: new_end - new_start,
        })
    }
    
    /// Split extent at position
    pub fn split_at(&self, position: u64) -> Option<(Self, Self)> {
        if position <= self.start || position >= self.end() {
            return None;
        }
        
        let left = Self {
            start: self.start,
            length: position - self.start,
        };
        
        let right = Self {
            start: position,
            length: self.end() - position,
        };
        
        Some((left, right))
    }
    
    /// Allocate from beginning of extent
    pub fn allocate_front(&self, count: u64) -> Option<(Self, Option<Self>)> {
        if count > self.length {
            return None;
        }
        
        let allocated = Self {
            start: self.start,
            length: count,
        };
        
        let remaining = if count < self.length {
            Some(Self {
                start: self.start + count,
                length: self.length - count,
            })
        } else {
            None
        };
        
        Some((allocated, remaining))
    }
    
    /// Allocate from end of extent
    pub fn allocate_back(&self, count: u64) -> Option<(Self, Option<Self>)> {
        if count > self.length {
            return None;
        }
        
        let remaining = if count < self.length {
            Some(Self {
                start: self.start,
                length: self.length - count,
            })
        } else {
            None
        };
        
        let allocated = Self {
            start: self.end() - count,
            length: count,
        };
        
        Some((allocated, remaining))
    }
}

impl PartialOrd for FreeExtent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FreeExtent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}

// ============================================================================
// Free Extent List (fixed-size for no_std)
// ============================================================================

/// Maximum entries in free list
pub const MAX_FREE_EXTENTS: usize = 1024;

/// Fixed-size free extent list.
///
/// Maintains sorted order by start block.
pub struct FreeExtentList {
    /// Extents (sorted by start)
    extents: [FreeExtent; MAX_FREE_EXTENTS],
    /// Number of valid entries
    count: usize,
    /// Total free blocks
    total_free: u64,
    /// Largest extent
    largest: u64,
}

impl FreeExtentList {
    /// Create new empty list
    pub const fn new() -> Self {
        Self {
            extents: [FreeExtent::new(0, 0); MAX_FREE_EXTENTS],
            count: 0,
            total_free: 0,
            largest: 0,
        }
    }
    
    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    
    /// Check if full
    #[inline]
    pub fn is_full(&self) -> bool {
        self.count >= MAX_FREE_EXTENTS
    }
    
    /// Get count
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }
    
    /// Get total free blocks
    #[inline]
    pub fn total_free(&self) -> u64 {
        self.total_free
    }
    
    /// Get largest extent size
    #[inline]
    pub fn largest_extent(&self) -> u64 {
        self.largest
    }
    
    /// Get extents slice
    #[inline]
    pub fn extents(&self) -> &[FreeExtent] {
        &self.extents[..self.count]
    }
    
    /// Find insertion point for extent (binary search)
    fn find_insert_pos(&self, start: u64) -> usize {
        let mut left = 0;
        let mut right = self.count;
        
        while left < right {
            let mid = left + (right - left) / 2;
            if self.extents[mid].start < start {
                left = mid + 1;
            } else {
                right = mid;
            }
        }
        
        left
    }
    
    /// Insert extent (maintains sorted order, attempts coalescing)
    pub fn insert(&mut self, extent: FreeExtent) -> HfsResult<()> {
        if extent.length == 0 {
            return Ok(());
        }
        
        let pos = self.find_insert_pos(extent.start);
        
        // Check for coalescing with previous extent
        let mut merged = extent;
        let mut remove_prev = false;
        
        if pos > 0 {
            let prev = &self.extents[pos - 1];
            if prev.end() == merged.start {
                // Coalesce with previous
                merged = FreeExtent::new(prev.start, prev.length + merged.length);
                remove_prev = true;
            }
        }
        
        // Check for coalescing with next extent
        let mut remove_next = false;
        
        if pos < self.count {
            let next = &self.extents[pos];
            if merged.end() == next.start {
                // Coalesce with next
                merged = FreeExtent::new(merged.start, merged.length + next.length);
                remove_next = true;
            }
        }
        
        // Perform removals and insertion
        if remove_prev && remove_next {
            // Replace prev, remove next
            self.extents[pos - 1] = merged;
            // Shift left to remove pos
            for i in pos..self.count - 1 {
                self.extents[i] = self.extents[i + 1];
            }
            self.count -= 1;
        } else if remove_prev {
            // Just replace prev
            self.extents[pos - 1] = merged;
        } else if remove_next {
            // Replace next
            self.extents[pos] = merged;
        } else {
            // Insert new extent
            if self.is_full() {
                return Err(HfsError::NoSpace);
            }
            
            // Shift right
            for i in (pos..self.count).rev() {
                self.extents[i + 1] = self.extents[i];
            }
            self.extents[pos] = merged;
            self.count += 1;
        }
        
        // Update stats
        self.total_free += extent.length;
        self.update_largest();
        
        Ok(())
    }
    
    /// Remove extent covering specific range
    pub fn remove(&mut self, start: u64, length: u64) -> HfsResult<()> {
        if length == 0 {
            return Ok(());
        }
        
        let end = start + length;
        
        // Find extent containing start
        let mut found_idx = None;
        for (i, ext) in self.extents[..self.count].iter().enumerate() {
            if ext.contains(start) {
                found_idx = Some(i);
                break;
            }
        }
        
        let idx = found_idx.ok_or(HfsError::InvalidBlockNumber)?;
        let ext = self.extents[idx];
        
        // Check if range is fully contained
        if end > ext.end() {
            return Err(HfsError::InvalidBlockNumber);
        }
        
        // Handle split cases
        let before = if start > ext.start {
            Some(FreeExtent::new(ext.start, start - ext.start))
        } else {
            None
        };
        
        let after = if end < ext.end() {
            Some(FreeExtent::new(end, ext.end() - end))
        } else {
            None
        };
        
        match (before, after) {
            (Some(b), Some(a)) => {
                // Split in middle - need to insert one, modify one
                if self.is_full() {
                    return Err(HfsError::NoSpace);
                }
                self.extents[idx] = b;
                // Insert 'a' after
                for i in (idx + 1..self.count).rev() {
                    self.extents[i + 1] = self.extents[i];
                }
                self.extents[idx + 1] = a;
                self.count += 1;
            }
            (Some(b), None) => {
                // Trim from end
                self.extents[idx] = b;
            }
            (None, Some(a)) => {
                // Trim from start
                self.extents[idx] = a;
            }
            (None, None) => {
                // Remove entire extent
                for i in idx..self.count - 1 {
                    self.extents[i] = self.extents[i + 1];
                }
                self.count -= 1;
            }
        }
        
        self.total_free -= length;
        self.update_largest();
        
        Ok(())
    }
    
    /// Find first extent with at least 'count' blocks
    pub fn find_first_fit(&self, count: u64) -> Option<usize> {
        for (i, ext) in self.extents[..self.count].iter().enumerate() {
            if ext.length >= count {
                return Some(i);
            }
        }
        None
    }
    
    /// Find best fit (smallest extent >= count)
    pub fn find_best_fit(&self, count: u64) -> Option<usize> {
        let mut best_idx = None;
        let mut best_size = u64::MAX;
        
        for (i, ext) in self.extents[..self.count].iter().enumerate() {
            if ext.length >= count && ext.length < best_size {
                best_idx = Some(i);
                best_size = ext.length;
                
                // Perfect fit
                if best_size == count {
                    break;
                }
            }
        }
        
        best_idx
    }
    
    /// Find extent near goal block
    pub fn find_near(&self, goal: u64, count: u64) -> Option<usize> {
        let mut best_idx = None;
        let mut best_distance = u64::MAX;
        
        for (i, ext) in self.extents[..self.count].iter().enumerate() {
            if ext.length >= count {
                let distance = if goal >= ext.start && goal < ext.end() {
                    0
                } else if goal < ext.start {
                    ext.start - goal
                } else {
                    goal - ext.end()
                };
                
                if distance < best_distance {
                    best_idx = Some(i);
                    best_distance = distance;
                }
            }
        }
        
        best_idx
    }
    
    /// Allocate from specific extent
    pub fn allocate_from(&mut self, idx: usize, count: u64) -> HfsResult<u64> {
        if idx >= self.count {
            return Err(HfsError::InvalidBlockNumber);
        }
        
        let ext = self.extents[idx];
        if ext.length < count {
            return Err(HfsError::NoSpace);
        }
        
        let start = ext.start;
        
        if ext.length == count {
            // Remove entire extent
            for i in idx..self.count - 1 {
                self.extents[i] = self.extents[i + 1];
            }
            self.count -= 1;
        } else {
            // Shrink extent
            self.extents[idx] = FreeExtent::new(
                ext.start + count,
                ext.length - count,
            );
        }
        
        self.total_free -= count;
        self.update_largest();
        
        Ok(start)
    }
    
    /// Update largest extent size
    fn update_largest(&mut self) {
        self.largest = self.extents[..self.count]
            .iter()
            .map(|e| e.length)
            .max()
            .unwrap_or(0);
    }
    
    /// Clear all entries
    pub fn clear(&mut self) {
        self.count = 0;
        self.total_free = 0;
        self.largest = 0;
    }
}

impl Default for FreeExtentList {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Size-Indexed Free Lists
// ============================================================================

/// Size classes for quick lookup
pub const SIZE_CLASSES: [u64; 16] = [
    1, 2, 4, 8, 16, 32, 64, 128,
    256, 512, 1024, 2048, 4096, 8192, 16384, 65536,
];

/// Get size class for block count
#[inline]
pub fn size_class(blocks: u64) -> usize {
    for (i, &size) in SIZE_CLASSES.iter().enumerate() {
        if blocks <= size {
            return i;
        }
    }
    SIZE_CLASSES.len() - 1
}

/// Size-indexed free extent tracker.
///
/// Maintains lists of extents indexed by size class for O(1) lookup.
pub struct SizeIndexedFreeList {
    /// Heads of each size class list
    heads: [u32; SIZE_CLASSES.len()],
    /// Free extent count per class
    counts: [u32; SIZE_CLASSES.len()],
    /// Total free blocks
    total_free: u64,
}

impl SizeIndexedFreeList {
    /// Create new
    pub const fn new() -> Self {
        Self {
            heads: [0; SIZE_CLASSES.len()],
            counts: [0; SIZE_CLASSES.len()],
            total_free: 0,
        }
    }
    
    /// Find class with available extents of at least 'count' blocks
    pub fn find_class(&self, count: u64) -> Option<usize> {
        let min_class = size_class(count);
        
        for class in min_class..SIZE_CLASSES.len() {
            if self.counts[class] > 0 {
                return Some(class);
            }
        }
        
        None
    }
    
    /// Get count for size class
    #[inline]
    pub fn class_count(&self, class: usize) -> u32 {
        self.counts.get(class).copied().unwrap_or(0)
    }
    
    /// Total free blocks
    #[inline]
    pub fn total_free(&self) -> u64 {
        self.total_free
    }
}

impl Default for SizeIndexedFreeList {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_free_extent_basic() {
        let ext = FreeExtent::new(100, 50);
        
        assert_eq!(ext.start, 100);
        assert_eq!(ext.length, 50);
        assert_eq!(ext.end(), 150);
        
        assert!(ext.contains(100));
        assert!(ext.contains(149));
        assert!(!ext.contains(150));
    }
    
    #[test]
    fn test_free_extent_coalesce() {
        let a = FreeExtent::new(100, 50);
        let b = FreeExtent::new(150, 50);
        
        assert!(a.can_coalesce(&b));
        
        let merged = a.coalesce(&b).unwrap();
        assert_eq!(merged.start, 100);
        assert_eq!(merged.length, 100);
        
        let c = FreeExtent::new(300, 50);
        assert!(!a.can_coalesce(&c));
    }
    
    #[test]
    fn test_free_extent_split() {
        let ext = FreeExtent::new(100, 100);
        
        let (left, right) = ext.split_at(150).unwrap();
        assert_eq!(left.start, 100);
        assert_eq!(left.length, 50);
        assert_eq!(right.start, 150);
        assert_eq!(right.length, 50);
    }
    
    #[test]
    fn test_free_list_insert() {
        let mut list = FreeExtentList::new();
        
        list.insert(FreeExtent::new(100, 50)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list.total_free(), 50);
        
        list.insert(FreeExtent::new(200, 50)).unwrap();
        assert_eq!(list.len(), 2);
        
        // Insert adjacent - should coalesce
        list.insert(FreeExtent::new(150, 50)).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list.total_free(), 150);
    }
    
    #[test]
    fn test_free_list_remove() {
        let mut list = FreeExtentList::new();
        list.insert(FreeExtent::new(100, 100)).unwrap();
        
        // Remove from middle - splits
        list.remove(130, 20).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list.total_free(), 80);
        
        assert_eq!(list.extents()[0].start, 100);
        assert_eq!(list.extents()[0].length, 30);
        assert_eq!(list.extents()[1].start, 150);
        assert_eq!(list.extents()[1].length, 50);
    }
    
    #[test]
    fn test_free_list_find() {
        let mut list = FreeExtentList::new();
        list.insert(FreeExtent::new(100, 10)).unwrap();
        list.insert(FreeExtent::new(200, 50)).unwrap();
        list.insert(FreeExtent::new(300, 20)).unwrap();
        
        // First fit
        assert_eq!(list.find_first_fit(15), Some(1));
        
        // Best fit
        assert_eq!(list.find_best_fit(15), Some(2)); // 20 is closest to 15
    }
    
    #[test]
    fn test_size_class() {
        assert_eq!(size_class(1), 0);
        assert_eq!(size_class(2), 1);
        assert_eq!(size_class(3), 2);
        assert_eq!(size_class(1024), 10);
        assert_eq!(size_class(100000), 15);
    }
}
