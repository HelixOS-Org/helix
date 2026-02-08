// SPDX-License-Identifier: GPL-2.0
//! Holistic slab_alloc — slab allocator for kernel object caches.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Slab state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabState {
    Full,
    Partial,
    Empty,
}

/// Cache flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheFlags(pub u32);

impl CacheFlags {
    pub const HWCACHE_ALIGN: u32 = 1 << 0;
    pub const POISON: u32 = 1 << 1;
    pub const REDZONES: u32 = 1 << 2;
    pub const RECLAIM: u32 = 1 << 3;
    pub const PANIC: u32 = 1 << 4;
    pub const ACCOUNT: u32 = 1 << 5;

    pub fn new() -> Self { Self(0) }
    pub fn set(&mut self, f: u32) { self.0 |= f; }
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Individual slab
#[derive(Debug)]
pub struct Slab {
    pub page_addr: u64,
    pub state: SlabState,
    pub objects_total: u32,
    pub objects_used: u32,
    pub freelist_head: u32,
    pub order: u32,
}

impl Slab {
    pub fn new(addr: u64, objects: u32, order: u32) -> Self {
        Self { page_addr: addr, state: SlabState::Empty, objects_total: objects, objects_used: 0, freelist_head: 0, order }
    }

    pub fn allocate(&mut self) -> Option<u64> {
        if self.objects_used >= self.objects_total { return None; }
        let idx = self.objects_used;
        self.objects_used += 1;
        self.state = if self.objects_used == self.objects_total { SlabState::Full } else { SlabState::Partial };
        Some(self.page_addr + (idx as u64 * 64)) // simplified offset
    }

    pub fn free(&mut self) -> bool {
        if self.objects_used == 0 { return false; }
        self.objects_used -= 1;
        self.state = if self.objects_used == 0 { SlabState::Empty } else { SlabState::Partial };
        true
    }

    pub fn utilization(&self) -> f64 {
        if self.objects_total == 0 { return 0.0; }
        self.objects_used as f64 / self.objects_total as f64
    }
}

/// Slab cache (kmem_cache equivalent)
#[derive(Debug)]
pub struct SlabCache {
    pub name_hash: u64,
    pub object_size: u32,
    pub aligned_size: u32,
    pub slab_order: u32,
    pub objects_per_slab: u32,
    pub flags: CacheFlags,
    pub slabs: Vec<Slab>,
    pub total_allocated: u64,
    pub total_freed: u64,
    pub high_watermark: u64,
    pub cpu_caches: BTreeMap<u32, Vec<u64>>,
}

impl SlabCache {
    pub fn new(name_hash: u64, obj_size: u32, flags: CacheFlags) -> Self {
        let aligned = (obj_size + 7) & !7;
        let slab_size = 4096u32 << 0; // order-0
        let per_slab = slab_size / aligned;
        Self {
            name_hash, object_size: obj_size, aligned_size: aligned,
            slab_order: 0, objects_per_slab: per_slab, flags,
            slabs: Vec::new(), total_allocated: 0, total_freed: 0,
            high_watermark: 0, cpu_caches: BTreeMap::new(),
        }
    }

    pub fn alloc(&mut self, slab_addr: u64) -> Option<u64> {
        // Try partial slabs first
        for slab in &mut self.slabs {
            if slab.state != SlabState::Full {
                if let Some(addr) = slab.allocate() {
                    self.total_allocated += 1;
                    if self.total_allocated - self.total_freed > self.high_watermark {
                        self.high_watermark = self.total_allocated - self.total_freed;
                    }
                    return Some(addr);
                }
            }
        }
        // New slab
        let mut slab = Slab::new(slab_addr, self.objects_per_slab, self.slab_order);
        let addr = slab.allocate();
        self.slabs.push(slab);
        self.total_allocated += 1;
        addr
    }

    pub fn free_obj(&mut self) {
        for slab in &mut self.slabs {
            if slab.state != SlabState::Empty {
                slab.free();
                self.total_freed += 1;
                return;
            }
        }
    }

    pub fn active_objects(&self) -> u64 { self.total_allocated - self.total_freed }
    pub fn waste_bytes(&self) -> u64 {
        (self.aligned_size - self.object_size) as u64 * self.active_objects()
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SlabAllocStats {
    pub total_caches: u32,
    pub total_slabs: u32,
    pub total_objects: u64,
    pub active_objects: u64,
    pub total_bytes: u64,
    pub waste_bytes: u64,
    pub avg_utilization: f64,
}

/// Main slab allocator
pub struct HolisticSlabAlloc {
    caches: BTreeMap<u64, SlabCache>,
}

impl HolisticSlabAlloc {
    pub fn new() -> Self { Self { caches: BTreeMap::new() } }

    pub fn create_cache(&mut self, name_hash: u64, obj_size: u32, flags: CacheFlags) {
        self.caches.insert(name_hash, SlabCache::new(name_hash, obj_size, flags));
    }

    pub fn alloc(&mut self, cache: u64, slab_addr: u64) -> Option<u64> {
        self.caches.get_mut(&cache)?.alloc(slab_addr)
    }

    pub fn free(&mut self, cache: u64) {
        if let Some(c) = self.caches.get_mut(&cache) { c.free_obj(); }
    }

    pub fn stats(&self) -> SlabAllocStats {
        let slabs: u32 = self.caches.values().map(|c| c.slabs.len() as u32).sum();
        let total_alloc: u64 = self.caches.values().map(|c| c.total_allocated).sum();
        let active: u64 = self.caches.values().map(|c| c.active_objects()).sum();
        let waste: u64 = self.caches.values().map(|c| c.waste_bytes()).sum();
        let utils: Vec<f64> = self.caches.values().flat_map(|c| c.slabs.iter()).map(|s| s.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        SlabAllocStats {
            total_caches: self.caches.len() as u32, total_slabs: slabs,
            total_objects: total_alloc, active_objects: active,
            total_bytes: active * 64, waste_bytes: waste, avg_utilization: avg,
        }
    }
}

// ============================================================================
// Merged from slab_alloc_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabV2State {
    Empty,
    Partial,
    Full,
}

/// Slab v2 object
#[derive(Debug)]
pub struct SlabV2Object {
    pub offset: u32,
    pub allocated: bool,
    pub owner_hash: u64,
}

/// Slab v2 page
#[derive(Debug)]
pub struct SlabV2Page {
    pub page_addr: u64,
    pub objects: u32,
    pub in_use: u32,
    pub state: SlabV2State,
    pub freelist_head: u32,
    pub frozen: bool,
}

impl SlabV2Page {
    pub fn new(addr: u64, objs: u32) -> Self {
        Self { page_addr: addr, objects: objs, in_use: 0, state: SlabV2State::Empty, freelist_head: 0, frozen: false }
    }

    pub fn alloc(&mut self) -> bool {
        if self.in_use >= self.objects { return false; }
        self.in_use += 1;
        self.update_state();
        true
    }

    pub fn free(&mut self) -> bool {
        if self.in_use == 0 { return false; }
        self.in_use -= 1;
        self.update_state();
        true
    }

    fn update_state(&mut self) {
        if self.in_use == 0 { self.state = SlabV2State::Empty; }
        else if self.in_use >= self.objects { self.state = SlabV2State::Full; }
        else { self.state = SlabV2State::Partial; }
    }
}

/// Slab v2 cache
#[derive(Debug)]
pub struct SlabV2Cache {
    pub name_hash: u64,
    pub obj_size: u32,
    pub align: u32,
    pub objs_per_slab: u32,
    pub slabs: Vec<SlabV2Page>,
    pub total_allocs: u64,
    pub total_frees: u64,
    pub alloc_failures: u64,
    pub ctor_hash: u64,
}

impl SlabV2Cache {
    pub fn new(name: u64, obj_size: u32, align: u32, per_slab: u32) -> Self {
        Self { name_hash: name, obj_size, align, objs_per_slab: per_slab, slabs: Vec::new(), total_allocs: 0, total_frees: 0, alloc_failures: 0, ctor_hash: 0 }
    }

    pub fn alloc(&mut self) -> bool {
        for slab in &mut self.slabs {
            if slab.alloc() { self.total_allocs += 1; return true; }
        }
        self.alloc_failures += 1;
        false
    }

    pub fn free_obj(&mut self) -> bool {
        for slab in &mut self.slabs {
            if slab.state != SlabV2State::Empty && slab.free() {
                self.total_frees += 1;
                return true;
            }
        }
        false
    }

    pub fn add_slab(&mut self, addr: u64) {
        self.slabs.push(SlabV2Page::new(addr, self.objs_per_slab));
    }

    pub fn utilization(&self) -> f64 {
        let total_objs: u32 = self.slabs.iter().map(|s| s.objects).sum();
        let used: u32 = self.slabs.iter().map(|s| s.in_use).sum();
        if total_objs == 0 { 0.0 } else { used as f64 / total_objs as f64 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SlabAllocV2Stats {
    pub total_caches: u32,
    pub total_slabs: u32,
    pub total_allocs: u64,
    pub total_frees: u64,
    pub avg_utilization: f64,
}

/// Main holistic slab allocator v2
pub struct HolisticSlabAllocV2 {
    caches: BTreeMap<u64, SlabV2Cache>,
}

impl HolisticSlabAllocV2 {
    pub fn new() -> Self { Self { caches: BTreeMap::new() } }

    pub fn create_cache(&mut self, name: u64, size: u32, align: u32, per_slab: u32) {
        self.caches.insert(name, SlabV2Cache::new(name, size, align, per_slab));
    }

    pub fn alloc(&mut self, cache_name: u64) -> bool {
        if let Some(c) = self.caches.get_mut(&cache_name) { c.alloc() }
        else { false }
    }

    pub fn free_obj(&mut self, cache_name: u64) -> bool {
        if let Some(c) = self.caches.get_mut(&cache_name) { c.free_obj() }
        else { false }
    }

    pub fn destroy_cache(&mut self, name: u64) { self.caches.remove(&name); }

    pub fn stats(&self) -> SlabAllocV2Stats {
        let slabs: u32 = self.caches.values().map(|c| c.slabs.len() as u32).sum();
        let allocs: u64 = self.caches.values().map(|c| c.total_allocs).sum();
        let frees: u64 = self.caches.values().map(|c| c.total_frees).sum();
        let avg = if self.caches.is_empty() { 0.0 }
            else { self.caches.values().map(|c| c.utilization()).sum::<f64>() / self.caches.len() as f64 };
        SlabAllocV2Stats { total_caches: self.caches.len() as u32, total_slabs: slabs, total_allocs: allocs, total_frees: frees, avg_utilization: avg }
    }
}

// ============================================================================
// Merged from slab_alloc_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlabV3SizeClass {
    Tiny8,
    Small16,
    Small32,
    Medium64,
    Medium128,
    Large256,
    Large512,
    Huge1024,
    Huge2048,
    Huge4096,
}

/// Magazine state in the depot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagazineState {
    Empty,
    Partial,
    Full,
    Draining,
}

/// Individual magazine holding cached objects.
#[derive(Debug, Clone)]
pub struct SlabV3Magazine {
    pub id: u64,
    pub size_class: SlabV3SizeClass,
    pub capacity: usize,
    pub current_count: usize,
    pub state: MagazineState,
    pub object_addrs: Vec<u64>,
    pub alloc_count: u64,
    pub free_count: u64,
    pub reload_count: u64,
}

impl SlabV3Magazine {
    pub fn new(id: u64, size_class: SlabV3SizeClass, capacity: usize) -> Self {
        Self {
            id,
            size_class,
            capacity,
            current_count: 0,
            state: MagazineState::Empty,
            object_addrs: Vec::new(),
            alloc_count: 0,
            free_count: 0,
            reload_count: 0,
        }
    }

    pub fn try_alloc(&mut self) -> Option<u64> {
        if let Some(addr) = self.object_addrs.pop() {
            self.current_count = self.current_count.saturating_sub(1);
            self.alloc_count += 1;
            self.update_state();
            Some(addr)
        } else {
            None
        }
    }

    pub fn try_free(&mut self, addr: u64) -> bool {
        if self.current_count >= self.capacity {
            return false;
        }
        self.object_addrs.push(addr);
        self.current_count += 1;
        self.free_count += 1;
        self.update_state();
        true
    }

    fn update_state(&mut self) {
        self.state = if self.current_count == 0 {
            MagazineState::Empty
        } else if self.current_count >= self.capacity {
            MagazineState::Full
        } else {
            MagazineState::Partial
        };
    }
}

/// Per-CPU depot holding loaded and spare magazines.
#[derive(Debug, Clone)]
pub struct SlabV3Depot {
    pub cpu_id: u32,
    pub loaded: Option<SlabV3Magazine>,
    pub spare: Option<SlabV3Magazine>,
    pub swap_count: u64,
    pub depot_miss_count: u64,
}

impl SlabV3Depot {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            loaded: None,
            spare: None,
            swap_count: 0,
            depot_miss_count: 0,
        }
    }

    pub fn try_alloc(&mut self) -> Option<u64> {
        if let Some(ref mut mag) = self.loaded {
            if let Some(addr) = mag.try_alloc() {
                return Some(addr);
            }
        }
        // Swap loaded and spare
        core::mem::swap(&mut self.loaded, &mut self.spare);
        self.swap_count += 1;
        if let Some(ref mut mag) = self.loaded {
            if let Some(addr) = mag.try_alloc() {
                return Some(addr);
            }
        }
        self.depot_miss_count += 1;
        None
    }
}

/// A slab cache descriptor for a given size class.
#[derive(Debug, Clone)]
pub struct SlabV3Cache {
    pub name: String,
    pub size_class: SlabV3SizeClass,
    pub object_size: usize,
    pub slab_order: u32,
    pub objects_per_slab: usize,
    pub total_slabs: u64,
    pub total_objects_allocated: u64,
    pub total_objects_freed: u64,
    pub active_objects: u64,
    pub depot_full_magazines: u64,
    pub depot_empty_magazines: u64,
}

impl SlabV3Cache {
    pub fn new(name: String, size_class: SlabV3SizeClass, object_size: usize) -> Self {
        let objects_per_slab = if object_size > 0 { 4096 / object_size } else { 1 };
        Self {
            name,
            size_class,
            object_size,
            slab_order: 0,
            objects_per_slab,
            total_slabs: 0,
            total_objects_allocated: 0,
            total_objects_freed: 0,
            active_objects: 0,
            depot_full_magazines: 0,
            depot_empty_magazines: 0,
        }
    }

    pub fn utilization_percent(&self) -> f64 {
        let capacity = self.total_slabs * self.objects_per_slab as u64;
        if capacity == 0 {
            return 0.0;
        }
        (self.active_objects as f64 / capacity as f64) * 100.0
    }
}

/// Statistics for the V3 slab allocator.
#[derive(Debug, Clone)]
pub struct SlabV3Stats {
    pub total_caches: u64,
    pub total_depots: u64,
    pub magazine_alloc_hits: u64,
    pub magazine_alloc_misses: u64,
    pub depot_swaps: u64,
    pub backend_alloc_count: u64,
    pub backend_free_count: u64,
    pub reap_cycles: u64,
    pub total_memory_bytes: u64,
}

/// Main holistic slab allocator V3 manager.
pub struct HolisticSlabAllocV3 {
    pub caches: BTreeMap<u64, SlabV3Cache>,
    pub depots: BTreeMap<u32, SlabV3Depot>,
    pub next_cache_id: u64,
    pub next_magazine_id: AtomicU64,
    pub stats: SlabV3Stats,
}

impl HolisticSlabAllocV3 {
    pub fn new() -> Self {
        Self {
            caches: BTreeMap::new(),
            depots: BTreeMap::new(),
            next_cache_id: 1,
            next_magazine_id: AtomicU64::new(1),
            stats: SlabV3Stats {
                total_caches: 0,
                total_depots: 0,
                magazine_alloc_hits: 0,
                magazine_alloc_misses: 0,
                depot_swaps: 0,
                backend_alloc_count: 0,
                backend_free_count: 0,
                reap_cycles: 0,
                total_memory_bytes: 0,
            },
        }
    }

    pub fn create_cache(
        &mut self,
        name: String,
        size_class: SlabV3SizeClass,
        object_size: usize,
    ) -> u64 {
        let id = self.next_cache_id;
        self.next_cache_id += 1;
        let cache = SlabV3Cache::new(name, size_class, object_size);
        self.caches.insert(id, cache);
        self.stats.total_caches += 1;
        id
    }

    pub fn register_depot(&mut self, cpu_id: u32) {
        if !self.depots.contains_key(&cpu_id) {
            self.depots.insert(cpu_id, SlabV3Depot::new(cpu_id));
            self.stats.total_depots += 1;
        }
    }

    pub fn alloc_from_depot(&mut self, cpu_id: u32) -> Option<u64> {
        if let Some(depot) = self.depots.get_mut(&cpu_id) {
            if let Some(addr) = depot.try_alloc() {
                self.stats.magazine_alloc_hits += 1;
                return Some(addr);
            }
            self.stats.depot_swaps += depot.swap_count;
        }
        self.stats.magazine_alloc_misses += 1;
        // Fallback to backend
        self.stats.backend_alloc_count += 1;
        let addr = self.stats.backend_alloc_count * 4096;
        self.stats.total_memory_bytes += 4096;
        Some(addr)
    }

    pub fn reap_empty_magazines(&mut self) -> u64 {
        let mut reaped = 0u64;
        for depot in self.depots.values_mut() {
            if let Some(ref mag) = depot.spare {
                if mag.state == MagazineState::Empty {
                    depot.spare = None;
                    reaped += 1;
                }
            }
        }
        self.stats.reap_cycles += 1;
        reaped
    }

    pub fn cache_count(&self) -> usize {
        self.caches.len()
    }

    pub fn depot_count(&self) -> usize {
        self.depots.len()
    }
}
