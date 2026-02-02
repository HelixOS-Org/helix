//! # GPU Address Space Management
//!
//! Virtual address space allocation and page table management.

use alloc::vec::Vec;

use magma_core::{ByteSize, Error, GpuAddr, Result};

// =============================================================================
// VIRTUAL ADDRESS RANGE
// =============================================================================

/// A range in GPU virtual address space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VaRange {
    /// Start address
    pub start: GpuAddr,
    /// End address (exclusive)
    pub end: GpuAddr,
}

impl VaRange {
    /// Create a new VA range
    pub const fn new(start: GpuAddr, size: ByteSize) -> Self {
        Self {
            start,
            end: GpuAddr(start.0 + size.as_bytes()),
        }
    }

    /// Get range size
    pub fn size(&self) -> ByteSize {
        ByteSize::from_bytes(self.end.0 - self.start.0)
    }

    /// Check if address is in range
    pub fn contains(&self, addr: GpuAddr) -> bool {
        addr >= self.start && addr < self.end
    }

    /// Check if ranges overlap
    pub fn overlaps(&self, other: &VaRange) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Split range at offset
    pub fn split_at(&self, offset: u64) -> Option<(VaRange, VaRange)> {
        let split = self.start + offset;
        if split > self.start && split < self.end {
            Some((
                VaRange {
                    start: self.start,
                    end: split,
                },
                VaRange {
                    start: split,
                    end: self.end,
                },
            ))
        } else {
            None
        }
    }
}

// =============================================================================
// ADDRESS SPACE REGIONS
// =============================================================================

/// Pre-defined address space regions
pub mod regions {
    //! Standard GPU virtual address regions

    use super::*;

    /// User space mappings (0 - 128TB)
    pub const USER: VaRange = VaRange {
        start: GpuAddr(0),
        end: GpuAddr(128 * 1024 * 1024 * 1024 * 1024),
    };

    /// Kernel mappings (128TB - 256TB)
    pub const KERNEL: VaRange = VaRange {
        start: GpuAddr(128 * 1024 * 1024 * 1024 * 1024),
        end: GpuAddr(256 * 1024 * 1024 * 1024 * 1024),
    };

    /// Reserved for hardware (256TB+)
    pub const RESERVED: VaRange = VaRange {
        start: GpuAddr(256 * 1024 * 1024 * 1024 * 1024),
        end: GpuAddr(u64::MAX),
    };
}

// =============================================================================
// VA BLOCK
// =============================================================================

/// A virtual address block
#[derive(Debug, Clone)]
struct VaBlock {
    range: VaRange,
    free: bool,
}

// =============================================================================
// ADDRESS SPACE
// =============================================================================

/// GPU virtual address space
#[derive(Debug)]
pub struct AddressSpace {
    /// Address space ID
    asid: u64,
    /// Managed range
    range: VaRange,
    /// VA blocks
    blocks: Vec<VaBlock>,
    /// Statistics
    stats: AddressSpaceStats,
}

/// Address space statistics
#[derive(Debug, Clone, Default)]
pub struct AddressSpaceStats {
    /// Number of allocations
    pub allocs: u64,
    /// Number of frees
    pub frees: u64,
    /// Current allocated bytes
    pub allocated: u64,
    /// Number of mappings
    pub mappings: u64,
}

impl AddressSpace {
    /// Create a new address space
    pub fn new(asid: u64, range: VaRange) -> Self {
        let initial_block = VaBlock { range, free: true };

        Self {
            asid,
            range,
            blocks: alloc::vec![initial_block],
            stats: AddressSpaceStats::default(),
        }
    }

    /// Get address space ID
    pub fn asid(&self) -> u64 {
        self.asid
    }

    /// Get managed range
    pub fn range(&self) -> VaRange {
        self.range
    }

    /// Allocate virtual address range
    pub fn allocate(&mut self, size: ByteSize, alignment: u64) -> Result<VaRange> {
        let size = size.as_bytes();
        let alignment = alignment.max(4096); // Minimum page alignment

        // Find first free block that fits
        for i in 0..self.blocks.len() {
            if !self.blocks[i].free {
                continue;
            }

            let block_start = self.blocks[i].range.start.0;
            let block_end = self.blocks[i].range.end.0;

            // Align start
            let aligned_start = (block_start + alignment - 1) & !(alignment - 1);
            let aligned_end = aligned_start + size;

            if aligned_end <= block_end {
                // Found a fit
                let alloc_range = VaRange {
                    start: GpuAddr(aligned_start),
                    end: GpuAddr(aligned_end),
                };

                // Split block
                self.split_block(i, alloc_range)?;

                self.stats.allocs += 1;
                self.stats.allocated += size;

                return Ok(alloc_range);
            }
        }

        Err(Error::OutOfMemory)
    }

    /// Split a block around an allocation
    fn split_block(&mut self, index: usize, alloc: VaRange) -> Result<()> {
        let block = self.blocks.remove(index);

        // Left fragment (before allocation)
        if alloc.start > block.range.start {
            self.blocks.insert(index, VaBlock {
                range: VaRange {
                    start: block.range.start,
                    end: alloc.start,
                },
                free: true,
            });
        }

        // Allocated block
        let alloc_idx = if alloc.start > block.range.start {
            index + 1
        } else {
            index
        };
        self.blocks.insert(alloc_idx, VaBlock {
            range: alloc,
            free: false,
        });

        // Right fragment (after allocation)
        if alloc.end < block.range.end {
            self.blocks.insert(alloc_idx + 1, VaBlock {
                range: VaRange {
                    start: alloc.end,
                    end: block.range.end,
                },
                free: true,
            });
        }

        Ok(())
    }

    /// Free a virtual address range
    pub fn free(&mut self, range: VaRange) -> Result<()> {
        // Find block
        let index = self
            .blocks
            .iter()
            .position(|b| b.range == range && !b.free)
            .ok_or(Error::NotFound)?;

        self.blocks[index].free = true;
        self.stats.frees += 1;
        self.stats.allocated -= range.size().as_bytes();

        // Try to merge with neighbors
        self.merge_free_blocks();

        Ok(())
    }

    /// Merge adjacent free blocks
    fn merge_free_blocks(&mut self) {
        let mut i = 0;
        while i + 1 < self.blocks.len() {
            if self.blocks[i].free && self.blocks[i + 1].free {
                // Merge
                self.blocks[i].range.end = self.blocks[i + 1].range.end;
                self.blocks.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &AddressSpaceStats {
        &self.stats
    }

    /// Get free space
    pub fn free_space(&self) -> ByteSize {
        ByteSize::from_bytes(
            self.blocks
                .iter()
                .filter(|b| b.free)
                .map(|b| b.range.size().as_bytes())
                .sum(),
        )
    }

    /// Get largest contiguous free region
    pub fn largest_free(&self) -> ByteSize {
        ByteSize::from_bytes(
            self.blocks
                .iter()
                .filter(|b| b.free)
                .map(|b| b.range.size().as_bytes())
                .max()
                .unwrap_or(0),
        )
    }
}

// =============================================================================
// ADDRESS SPACE MANAGER
// =============================================================================

/// Manages multiple address spaces
#[derive(Debug)]
pub struct AddressSpaceManager {
    /// Address spaces
    spaces: alloc::collections::BTreeMap<u64, AddressSpace>,
    /// Next ASID
    next_asid: u64,
}

impl AddressSpaceManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            spaces: alloc::collections::BTreeMap::new(),
            next_asid: 1,
        }
    }

    /// Create a new address space
    pub fn create(&mut self, range: VaRange) -> u64 {
        let asid = self.next_asid;
        self.next_asid += 1;

        let space = AddressSpace::new(asid, range);
        self.spaces.insert(asid, space);

        asid
    }

    /// Destroy an address space
    pub fn destroy(&mut self, asid: u64) -> Result<()> {
        self.spaces.remove(&asid).map(|_| ()).ok_or(Error::NotFound)
    }

    /// Get address space
    pub fn get(&self, asid: u64) -> Option<&AddressSpace> {
        self.spaces.get(&asid)
    }

    /// Get mutable address space
    pub fn get_mut(&mut self, asid: u64) -> Option<&mut AddressSpace> {
        self.spaces.get_mut(&asid)
    }
}

impl Default for AddressSpaceManager {
    fn default() -> Self {
        Self::new()
    }
}
