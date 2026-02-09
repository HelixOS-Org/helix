// SPDX-License-Identifier: GPL-2.0
//! Holistic dma_pool — DMA buffer pool allocator.

extern crate alloc;

use alloc::vec::Vec;

/// DMA buffer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaBufferState {
    Free,
    Allocated,
    Mapped,
    InTransfer,
}

/// DMA buffer
#[derive(Debug)]
#[repr(align(64))]
pub struct DmaBuffer {
    pub id: u64,
    pub phys_addr: u64,
    pub size: usize,
    pub state: DmaBufferState,
    pub coherent: bool,
    pub owner_device: u64,
    pub transfer_count: u64,
}

impl DmaBuffer {
    pub fn new(id: u64, phys_addr: u64, size: usize) -> Self {
        Self { id, phys_addr, size, state: DmaBufferState::Free, coherent: false, owner_device: 0, transfer_count: 0 }
    }
}

/// DMA pool
#[derive(Debug)]
#[repr(align(64))]
pub struct DmaPool {
    pub id: u64,
    pub buffers: Vec<DmaBuffer>,
    pub total_size: usize,
    pub free_size: usize,
    pub next_phys: u64,
    pub allocations: u64,
    pub frees: u64,
}

impl DmaPool {
    pub fn new(id: u64, base_phys: u64, total_size: usize) -> Self {
        Self { id, buffers: Vec::new(), total_size, free_size: total_size, next_phys: base_phys, allocations: 0, frees: 0 }
    }

    pub fn allocate(&mut self, size: usize) -> Option<u64> {
        if size > self.free_size { return None; }
        let buf_id = self.buffers.len() as u64 + 1;
        let phys = self.next_phys;
        self.next_phys += size as u64;
        let mut buf = DmaBuffer::new(buf_id, phys, size);
        buf.state = DmaBufferState::Allocated;
        self.buffers.push(buf);
        self.free_size -= size;
        self.allocations += 1;
        Some(buf_id)
    }

    #[inline]
    pub fn free(&mut self, buf_id: u64) -> bool {
        if let Some(buf) = self.buffers.iter_mut().find(|b| b.id == buf_id) {
            self.free_size += buf.size;
            buf.state = DmaBufferState::Free;
            self.frees += 1;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 { if self.total_size == 0 { 0.0 } else { 1.0 - self.free_size as f64 / self.total_size as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DmaPoolStats {
    pub total_pools: u32,
    pub total_buffers: u32,
    pub total_allocated: usize,
    pub total_free: usize,
    pub total_allocs: u64,
    pub avg_utilization: f64,
}

/// Main DMA pool manager
#[repr(align(64))]
pub struct HolisticDmaPool {
    pools: Vec<DmaPool>,
    next_id: u64,
}

impl HolisticDmaPool {
    pub fn new() -> Self { Self { pools: Vec::new(), next_id: 1 } }

    #[inline]
    pub fn create_pool(&mut self, base: u64, size: usize) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.pools.push(DmaPool::new(id, base, size));
        id
    }

    #[inline]
    pub fn stats(&self) -> DmaPoolStats {
        let buffers: u32 = self.pools.iter().map(|p| p.buffers.len() as u32).sum();
        let allocated: usize = self.pools.iter().map(|p| p.total_size - p.free_size).sum();
        let free: usize = self.pools.iter().map(|p| p.free_size).sum();
        let allocs: u64 = self.pools.iter().map(|p| p.allocations).sum();
        let utils: Vec<f64> = self.pools.iter().map(|p| p.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        DmaPoolStats { total_pools: self.pools.len() as u32, total_buffers: buffers, total_allocated: allocated, total_free: free, total_allocs: allocs, avg_utilization: avg }
    }
}

// ============================================================================
// Merged from dma_pool_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaV2AllocState {
    Free,
    Allocated,
    Mapped,
    Coherent,
}

/// DMA pool v2 entry
#[derive(Debug)]
pub struct DmaV2Entry {
    pub phys_addr: u64,
    pub virt_addr: u64,
    pub size: u32,
    pub state: DmaV2AllocState,
    pub device_id: u32,
    pub direction: u8,
}

/// DMA pool v2 instance
#[derive(Debug)]
#[repr(align(64))]
pub struct DmaPoolV2 {
    pub name_hash: u64,
    pub block_size: u32,
    pub boundary: u32,
    pub entries: Vec<DmaV2Entry>,
    pub alloc_count: u64,
    pub free_count: u64,
}

impl DmaPoolV2 {
    pub fn new(name: u64, block_size: u32, boundary: u32) -> Self {
        Self { name_hash: name, block_size, boundary, entries: Vec::new(), alloc_count: 0, free_count: 0 }
    }

    #[inline]
    pub fn alloc(&mut self, phys: u64, virt: u64, dev: u32) -> usize {
        let idx = self.entries.len();
        self.entries.push(DmaV2Entry { phys_addr: phys, virt_addr: virt, size: self.block_size, state: DmaV2AllocState::Allocated, device_id: dev, direction: 0 });
        self.alloc_count += 1;
        idx
    }

    #[inline(always)]
    pub fn free(&mut self, idx: usize) {
        if idx < self.entries.len() { self.entries[idx].state = DmaV2AllocState::Free; self.free_count += 1; }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DmaPoolV2Stats {
    pub total_pools: u32,
    pub total_allocs: u64,
    pub total_frees: u64,
    pub active_entries: u32,
}

/// Main holistic DMA pool v2
#[repr(align(64))]
pub struct HolisticDmaPoolV2 {
    pools: BTreeMap<u64, DmaPoolV2>,
}

impl HolisticDmaPoolV2 {
    pub fn new() -> Self { Self { pools: BTreeMap::new() } }

    #[inline(always)]
    pub fn create_pool(&mut self, name: u64, block_size: u32, boundary: u32) {
        self.pools.insert(name, DmaPoolV2::new(name, block_size, boundary));
    }

    #[inline(always)]
    pub fn alloc(&mut self, pool: u64, phys: u64, virt: u64, dev: u32) -> Option<usize> {
        if let Some(p) = self.pools.get_mut(&pool) { Some(p.alloc(phys, virt, dev)) } else { None }
    }

    #[inline(always)]
    pub fn free(&mut self, pool: u64, idx: usize) {
        if let Some(p) = self.pools.get_mut(&pool) { p.free(idx); }
    }

    #[inline(always)]
    pub fn destroy_pool(&mut self, name: u64) { self.pools.remove(&name); }

    #[inline]
    pub fn stats(&self) -> DmaPoolV2Stats {
        let allocs: u64 = self.pools.values().map(|p| p.alloc_count).sum();
        let frees: u64 = self.pools.values().map(|p| p.free_count).sum();
        let active: u32 = self.pools.values().map(|p| p.entries.iter().filter(|e| e.state == DmaV2AllocState::Allocated).count() as u32).sum();
        DmaPoolV2Stats { total_pools: self.pools.len() as u32, total_allocs: allocs, total_frees: frees, active_entries: active }
    }
}
