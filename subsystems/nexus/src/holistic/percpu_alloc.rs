// SPDX-License-Identifier: GPL-2.0
//! Holistic percpu_alloc — per-CPU memory allocator.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Per-CPU chunk state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PercpuChunkState {
    Free,
    Partial,
    Full,
}

/// Per-CPU allocation
#[derive(Debug)]
pub struct PercpuAlloc {
    pub offset: u32,
    pub size: u32,
    pub owner_hash: u64,
    pub allocated_at: u64,
}

/// Per-CPU chunk
#[derive(Debug)]
pub struct PercpuChunk {
    pub id: u64,
    pub base_addr: u64,
    pub total_size: u32,
    pub used_size: u32,
    pub nr_allocs: u32,
    pub state: PercpuChunkState,
    pub allocs: Vec<PercpuAlloc>,
    pub nr_cpus: u32,
}

impl PercpuChunk {
    pub fn new(id: u64, base: u64, size: u32, cpus: u32) -> Self {
        Self { id, base_addr: base, total_size: size, used_size: 0, nr_allocs: 0, state: PercpuChunkState::Free, allocs: Vec::new(), nr_cpus: cpus }
    }

    pub fn allocate(&mut self, size: u32, owner: u64, now: u64) -> Option<u32> {
        if self.used_size + size > self.total_size { return None; }
        let offset = self.used_size;
        self.allocs.push(PercpuAlloc { offset, size, owner_hash: owner, allocated_at: now });
        self.used_size += size;
        self.nr_allocs += 1;
        self.update_state();
        Some(offset)
    }

    pub fn free(&mut self, offset: u32) {
        if let Some(pos) = self.allocs.iter().position(|a| a.offset == offset) {
            let size = self.allocs[pos].size;
            self.allocs.remove(pos);
            self.used_size = self.used_size.saturating_sub(size);
            self.nr_allocs = self.nr_allocs.saturating_sub(1);
            self.update_state();
        }
    }

    fn update_state(&mut self) {
        if self.nr_allocs == 0 { self.state = PercpuChunkState::Free; }
        else if self.used_size >= self.total_size * 9 / 10 { self.state = PercpuChunkState::Full; }
        else { self.state = PercpuChunkState::Partial; }
    }

    pub fn utilization(&self) -> f64 {
        if self.total_size == 0 { 0.0 } else { self.used_size as f64 / self.total_size as f64 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PercpuAllocStats {
    pub total_chunks: u32,
    pub total_allocs: u32,
    pub total_used_bytes: u64,
    pub total_capacity_bytes: u64,
    pub avg_utilization: f64,
}

/// Main holistic per-CPU allocator
pub struct HolisticPercpuAlloc {
    chunks: BTreeMap<u64, PercpuChunk>,
    next_id: u64,
    nr_cpus: u32,
}

impl HolisticPercpuAlloc {
    pub fn new(cpus: u32) -> Self { Self { chunks: BTreeMap::new(), next_id: 1, nr_cpus: cpus } }

    pub fn create_chunk(&mut self, base: u64, size: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.chunks.insert(id, PercpuChunk::new(id, base, size, self.nr_cpus));
        id
    }

    pub fn allocate(&mut self, chunk_id: u64, size: u32, owner: u64, now: u64) -> Option<u32> {
        if let Some(c) = self.chunks.get_mut(&chunk_id) { c.allocate(size, owner, now) }
        else { None }
    }

    pub fn free(&mut self, chunk_id: u64, offset: u32) {
        if let Some(c) = self.chunks.get_mut(&chunk_id) { c.free(offset); }
    }

    pub fn destroy_chunk(&mut self, id: u64) { self.chunks.remove(&id); }

    pub fn stats(&self) -> PercpuAllocStats {
        let allocs: u32 = self.chunks.values().map(|c| c.nr_allocs).sum();
        let used: u64 = self.chunks.values().map(|c| c.used_size as u64).sum();
        let cap: u64 = self.chunks.values().map(|c| c.total_size as u64).sum();
        let avg = if self.chunks.is_empty() { 0.0 }
            else { self.chunks.values().map(|c| c.utilization()).sum::<f64>() / self.chunks.len() as f64 };
        PercpuAllocStats { total_chunks: self.chunks.len() as u32, total_allocs: allocs, total_used_bytes: used, total_capacity_bytes: cap, avg_utilization: avg }
    }
}

// ============================================================================
// Merged from percpu_alloc_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PercpuV2ChunkState {
    Free,
    Allocated,
    Partial,
    Immutable,
    Depopulated,
}

/// Allocation strategy for per-CPU areas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PercpuV2Strategy {
    FirstFit,
    BestFit,
    NearestNuma,
    Compact,
}

/// A per-CPU memory chunk.
#[derive(Debug, Clone)]
pub struct PercpuV2Chunk {
    pub chunk_id: u64,
    pub base_addr: u64,
    pub size: usize,
    pub state: PercpuV2ChunkState,
    pub cpu_id: u32,
    pub numa_node: u32,
    pub group_id: u64,
    pub populated_pages: u32,
    pub total_pages: u32,
    pub free_bytes: usize,
    pub alloc_count: u64,
}

impl PercpuV2Chunk {
    pub fn new(chunk_id: u64, base_addr: u64, size: usize, cpu_id: u32) -> Self {
        let total_pages = (size / 4096) as u32;
        Self {
            chunk_id,
            base_addr,
            size,
            state: PercpuV2ChunkState::Free,
            cpu_id,
            numa_node: 0,
            group_id: 0,
            populated_pages: 0,
            total_pages,
            free_bytes: size,
            alloc_count: 0,
        }
    }

    pub fn try_alloc(&mut self, requested: usize) -> Option<u64> {
        if self.free_bytes < requested {
            return None;
        }
        let offset = self.size - self.free_bytes;
        let addr = self.base_addr + offset as u64;
        self.free_bytes -= requested;
        self.alloc_count += 1;
        if self.free_bytes == 0 {
            self.state = PercpuV2ChunkState::Allocated;
        } else {
            self.state = PercpuV2ChunkState::Partial;
        }
        Some(addr)
    }

    pub fn populate_page(&mut self) -> bool {
        if self.populated_pages < self.total_pages {
            self.populated_pages += 1;
            true
        } else {
            false
        }
    }

    pub fn depopulate_page(&mut self) -> bool {
        if self.populated_pages > 0 {
            self.populated_pages -= 1;
            if self.populated_pages == 0 {
                self.state = PercpuV2ChunkState::Depopulated;
            }
            true
        } else {
            false
        }
    }

    pub fn utilization_percent(&self) -> f64 {
        if self.size == 0 {
            return 0.0;
        }
        let used = self.size - self.free_bytes;
        (used as f64 / self.size as f64) * 100.0
    }
}

/// A group of chunks for the same NUMA region.
#[derive(Debug, Clone)]
pub struct PercpuV2Group {
    pub group_id: u64,
    pub numa_node: u32,
    pub chunk_ids: Vec<u64>,
    pub total_size: u64,
    pub free_size: u64,
}

impl PercpuV2Group {
    pub fn new(group_id: u64, numa_node: u32) -> Self {
        Self {
            group_id,
            numa_node,
            chunk_ids: Vec::new(),
            total_size: 0,
            free_size: 0,
        }
    }

    pub fn add_chunk(&mut self, chunk_id: u64, size: u64) {
        self.chunk_ids.push(chunk_id);
        self.total_size += size;
        self.free_size += size;
    }
}

/// Statistics for the per-CPU allocator V2.
#[derive(Debug, Clone)]
pub struct PercpuAllocV2Stats {
    pub total_chunks: u64,
    pub total_groups: u64,
    pub alloc_success: u64,
    pub alloc_failures: u64,
    pub pages_populated: u64,
    pub pages_depopulated: u64,
    pub total_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub fragmentation_percent: f64,
}

/// Main holistic per-CPU allocator V2 manager.
pub struct HolisticPercpuAllocV2 {
    pub chunks: BTreeMap<u64, PercpuV2Chunk>,
    pub groups: BTreeMap<u64, PercpuV2Group>,
    pub strategy: PercpuV2Strategy,
    pub next_chunk_id: u64,
    pub next_group_id: u64,
    pub stats: PercpuAllocV2Stats,
}

impl HolisticPercpuAllocV2 {
    pub fn new() -> Self {
        Self {
            chunks: BTreeMap::new(),
            groups: BTreeMap::new(),
            strategy: PercpuV2Strategy::FirstFit,
            next_chunk_id: 1,
            next_group_id: 1,
            stats: PercpuAllocV2Stats {
                total_chunks: 0,
                total_groups: 0,
                alloc_success: 0,
                alloc_failures: 0,
                pages_populated: 0,
                pages_depopulated: 0,
                total_memory_bytes: 0,
                free_memory_bytes: 0,
                fragmentation_percent: 0.0,
            },
        }
    }

    pub fn create_chunk(&mut self, base_addr: u64, size: usize, cpu_id: u32) -> u64 {
        let id = self.next_chunk_id;
        self.next_chunk_id += 1;
        let chunk = PercpuV2Chunk::new(id, base_addr, size, cpu_id);
        self.chunks.insert(id, chunk);
        self.stats.total_chunks += 1;
        self.stats.total_memory_bytes += size as u64;
        self.stats.free_memory_bytes += size as u64;
        id
    }

    pub fn create_group(&mut self, numa_node: u32) -> u64 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        let group = PercpuV2Group::new(id, numa_node);
        self.groups.insert(id, group);
        self.stats.total_groups += 1;
        id
    }

    pub fn alloc_percpu(&mut self, size: usize) -> Option<u64> {
        for chunk in self.chunks.values_mut() {
            if let Some(addr) = chunk.try_alloc(size) {
                self.stats.alloc_success += 1;
                self.stats.free_memory_bytes = self.stats.free_memory_bytes.saturating_sub(size as u64);
                return Some(addr);
            }
        }
        self.stats.alloc_failures += 1;
        None
    }

    pub fn compute_fragmentation(&mut self) {
        let total = self.stats.total_memory_bytes as f64;
        if total == 0.0 {
            self.stats.fragmentation_percent = 0.0;
            return;
        }
        let partial_count = self
            .chunks
            .values()
            .filter(|c| c.state == PercpuV2ChunkState::Partial)
            .count() as f64;
        let total_count = self.chunks.len() as f64;
        if total_count > 0.0 {
            self.stats.fragmentation_percent = (partial_count / total_count) * 100.0;
        }
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
}
