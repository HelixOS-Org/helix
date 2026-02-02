//! # Memory Pool
//!
//! Fixed-size memory pools for common allocation patterns.

use alloc::vec::Vec;

use magma_core::{Error, Result, GpuAddr, ByteSize};

// =============================================================================
// POOL CONFIGURATION
// =============================================================================

/// Memory pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Block size
    pub block_size: ByteSize,
    /// Number of blocks
    pub num_blocks: u32,
    /// Alignment requirement
    pub alignment: u64,
    /// Pool name for debugging
    pub name: &'static str,
}

impl PoolConfig {
    /// Create pool config for uniform buffers
    pub const fn uniform_buffer(count: u32) -> Self {
        Self {
            block_size: ByteSize::from_bytes(256), // Vulkan min uniform buffer alignment
            num_blocks: count,
            alignment: 256,
            name: "uniform_pool",
        }
    }

    /// Create pool config for descriptor sets
    pub const fn descriptor_set(count: u32) -> Self {
        Self {
            block_size: ByteSize::from_bytes(64),
            num_blocks: count,
            alignment: 64,
            name: "descriptor_pool",
        }
    }

    /// Create pool config for command buffers
    pub const fn command_buffer(count: u32) -> Self {
        Self {
            block_size: ByteSize::from_kib(4),
            num_blocks: count,
            alignment: 4096,
            name: "command_pool",
        }
    }
}

// =============================================================================
// POOL BLOCK
// =============================================================================

/// A block in the pool
#[derive(Debug, Clone, Copy)]
struct PoolBlock {
    /// GPU address
    addr: GpuAddr,
    /// Is block in use?
    in_use: bool,
}

// =============================================================================
// MEMORY POOL
// =============================================================================

/// Fixed-size memory pool for fast allocation
#[derive(Debug)]
pub struct MemoryPool {
    /// Configuration
    config: PoolConfig,
    /// Base address
    base: GpuAddr,
    /// Pool blocks
    blocks: Vec<PoolBlock>,
    /// Free block indices (stack for O(1) alloc/free)
    free_stack: Vec<u32>,
    /// Statistics
    stats: PoolStats,
}

/// Pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total allocations
    pub allocs: u64,
    /// Total frees
    pub frees: u64,
    /// High water mark
    pub peak_used: u32,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(base: GpuAddr, config: PoolConfig) -> Self {
        let mut blocks = Vec::with_capacity(config.num_blocks as usize);
        let mut free_stack = Vec::with_capacity(config.num_blocks as usize);

        let aligned_size = (config.block_size.as_bytes() + config.alignment - 1)
            & !(config.alignment - 1);

        for i in 0..config.num_blocks {
            let addr = base + (i as u64 * aligned_size);
            blocks.push(PoolBlock {
                addr,
                in_use: false,
            });
            free_stack.push(i);
        }

        Self {
            config,
            base,
            blocks,
            free_stack,
            stats: PoolStats::default(),
        }
    }

    /// Allocate a block from the pool
    pub fn allocate(&mut self) -> Result<PoolAllocation> {
        let index = self.free_stack.pop().ok_or(Error::OutOfMemory)?;

        self.blocks[index as usize].in_use = true;
        self.stats.allocs += 1;

        let used = self.config.num_blocks - self.free_stack.len() as u32;
        self.stats.peak_used = self.stats.peak_used.max(used);

        Ok(PoolAllocation {
            addr: self.blocks[index as usize].addr,
            size: self.config.block_size,
            index,
        })
    }

    /// Free a block back to the pool
    pub fn free(&mut self, alloc: PoolAllocation) -> Result<()> {
        if alloc.index >= self.config.num_blocks {
            return Err(Error::InvalidParameter);
        }

        if !self.blocks[alloc.index as usize].in_use {
            return Err(Error::InvalidParameter);
        }

        self.blocks[alloc.index as usize].in_use = false;
        self.free_stack.push(alloc.index);
        self.stats.frees += 1;

        Ok(())
    }

    /// Get pool statistics
    pub fn stats(&self) -> &PoolStats {
        &self.stats
    }

    /// Get number of free blocks
    pub fn free_count(&self) -> u32 {
        self.free_stack.len() as u32
    }

    /// Get number of used blocks
    pub fn used_count(&self) -> u32 {
        self.config.num_blocks - self.free_stack.len() as u32
    }

    /// Get total size of pool
    pub fn total_size(&self) -> ByteSize {
        let aligned_size = (self.config.block_size.as_bytes() + self.config.alignment - 1)
            & !(self.config.alignment - 1);
        ByteSize::from_bytes(self.config.num_blocks as u64 * aligned_size)
    }

    /// Get pool name
    pub fn name(&self) -> &'static str {
        self.config.name
    }

    /// Reset pool (free all blocks)
    pub fn reset(&mut self) {
        self.free_stack.clear();
        for (i, block) in self.blocks.iter_mut().enumerate() {
            block.in_use = false;
            self.free_stack.push(i as u32);
        }
    }
}

/// Allocation from a pool
#[derive(Debug, Clone)]
pub struct PoolAllocation {
    /// GPU address
    pub addr: GpuAddr,
    /// Size
    pub size: ByteSize,
    /// Block index (for freeing)
    index: u32,
}

// =============================================================================
// POOL MANAGER
// =============================================================================

/// Manages multiple memory pools
#[derive(Debug)]
pub struct PoolManager {
    pools: alloc::collections::BTreeMap<&'static str, MemoryPool>,
}

impl PoolManager {
    /// Create new pool manager
    pub fn new() -> Self {
        Self {
            pools: alloc::collections::BTreeMap::new(),
        }
    }

    /// Add a pool
    pub fn add_pool(&mut self, pool: MemoryPool) {
        self.pools.insert(pool.name(), pool);
    }

    /// Get pool by name
    pub fn get(&self, name: &str) -> Option<&MemoryPool> {
        self.pools.get(name)
    }

    /// Get mutable pool by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut MemoryPool> {
        self.pools.get_mut(name)
    }

    /// Allocate from named pool
    pub fn allocate(&mut self, pool_name: &str) -> Result<PoolAllocation> {
        self.pools
            .get_mut(pool_name)
            .ok_or(Error::NotFound)?
            .allocate()
    }

    /// Free to named pool
    pub fn free(&mut self, pool_name: &str, alloc: PoolAllocation) -> Result<()> {
        self.pools
            .get_mut(pool_name)
            .ok_or(Error::NotFound)?
            .free(alloc)
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}
