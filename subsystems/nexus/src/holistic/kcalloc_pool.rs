//! # Holistic Kernel Calloc Pool
//!
//! Kernel-level allocation pool with size-class caching:
//! - Size-class based slab allocation
//! - Per-CPU allocation caches
//! - Emergency reserve pools
//! - Allocation tracking and leak detection
//! - Fragmentation monitoring
//! - Object reuse and recycling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Size class for slab allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeClass {
    Tiny,    // 8-32 bytes
    Small,   // 33-128 bytes
    Medium,  // 129-512 bytes
    Large,   // 513-2048 bytes
    Huge,    // 2049-8192 bytes
    Custom(usize),
}

impl SizeClass {
    pub fn from_size(size: usize) -> Self {
        match size {
            0..=32 => SizeClass::Tiny,
            33..=128 => SizeClass::Small,
            129..=512 => SizeClass::Medium,
            513..=2048 => SizeClass::Large,
            2049..=8192 => SizeClass::Huge,
            _ => SizeClass::Custom(size),
        }
    }

    pub fn slab_size(&self) -> usize {
        match self {
            SizeClass::Tiny => 32,
            SizeClass::Small => 128,
            SizeClass::Medium => 512,
            SizeClass::Large => 2048,
            SizeClass::Huge => 8192,
            SizeClass::Custom(s) => *s,
        }
    }
}

/// Allocation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocState {
    Free,
    Allocated,
    Quarantined,
    Poisoned,
}

/// Object header for tracking
#[derive(Debug, Clone)]
pub struct ObjectHeader {
    pub addr: u64,
    pub size: usize,
    pub state: AllocState,
    pub alloc_ts: u64,
    pub free_ts: Option<u64>,
    pub alloc_cpu: u32,
    pub owner_pid: u32,
    pub alloc_site: u64,
}

/// Per-CPU cache
#[derive(Debug, Clone)]
pub struct CpuCache {
    pub cpu_id: u32,
    pub size_class: SizeClass,
    pub freelist: Vec<u64>,
    pub cache_capacity: usize,
    pub allocs: u64,
    pub frees: u64,
    pub refills: u64,
    pub flushes: u64,
}

impl CpuCache {
    pub fn new(cpu: u32, class: SizeClass, capacity: usize) -> Self {
        Self {
            cpu_id: cpu, size_class: class, freelist: Vec::new(),
            cache_capacity: capacity, allocs: 0, frees: 0,
            refills: 0, flushes: 0,
        }
    }

    pub fn try_alloc(&mut self) -> Option<u64> {
        let addr = self.freelist.pop()?;
        self.allocs += 1;
        Some(addr)
    }

    pub fn try_free(&mut self, addr: u64) -> bool {
        if self.freelist.len() >= self.cache_capacity { return false; }
        self.freelist.push(addr);
        self.frees += 1;
        true
    }

    pub fn fill_ratio(&self) -> f64 {
        if self.cache_capacity == 0 { return 0.0; }
        self.freelist.len() as f64 / self.cache_capacity as f64
    }
}

/// Slab page
#[derive(Debug, Clone)]
pub struct SlabPage {
    pub page_addr: u64,
    pub size_class: SizeClass,
    pub objects_total: u32,
    pub objects_used: u32,
    pub fragmentation: f64,
}

impl SlabPage {
    pub fn new(addr: u64, class: SizeClass) -> Self {
        let obj_size = class.slab_size();
        let total = if obj_size > 0 { 4096 / obj_size } else { 0 };
        Self {
            page_addr: addr, size_class: class,
            objects_total: total as u32, objects_used: 0,
            fragmentation: 0.0,
        }
    }

    pub fn usage_ratio(&self) -> f64 {
        if self.objects_total == 0 { return 0.0; }
        self.objects_used as f64 / self.objects_total as f64
    }

    pub fn is_full(&self) -> bool { self.objects_used >= self.objects_total }
    pub fn is_empty(&self) -> bool { self.objects_used == 0 }
}

/// Emergency reserve
#[derive(Debug, Clone)]
pub struct EmergencyReserve {
    pub reserved_pages: u32,
    pub used_pages: u32,
    pub min_pages: u32,
    pub emergency_allocs: u64,
}

impl EmergencyReserve {
    pub fn new(pages: u32) -> Self {
        Self { reserved_pages: pages, used_pages: 0, min_pages: pages / 4, emergency_allocs: 0 }
    }

    pub fn can_allocate(&self) -> bool { self.used_pages < self.reserved_pages }
    pub fn is_critical(&self) -> bool { self.reserved_pages - self.used_pages <= self.min_pages }
}

/// Leak suspect
#[derive(Debug, Clone)]
pub struct LeakSuspect {
    pub addr: u64,
    pub size: usize,
    pub alloc_ts: u64,
    pub age_ns: u64,
    pub alloc_site: u64,
    pub pid: u32,
}

/// Kcalloc pool stats
#[derive(Debug, Clone, Default)]
pub struct KcallocPoolStats {
    pub total_allocated: u64,
    pub total_freed: u64,
    pub active_objects: u64,
    pub active_bytes: u64,
    pub slab_pages: usize,
    pub cpu_caches: usize,
    pub cache_hit_rate: f64,
    pub avg_fragmentation: f64,
    pub leak_suspects: usize,
    pub emergency_allocs: u64,
}

/// Holistic kernel calloc pool
pub struct HolisticKcallocPool {
    slabs: BTreeMap<u64, SlabPage>,
    objects: BTreeMap<u64, ObjectHeader>,
    cpu_caches: BTreeMap<(u32, usize), CpuCache>,
    reserve: EmergencyReserve,
    cache_hits: u64,
    cache_misses: u64,
    leak_threshold_ns: u64,
    stats: KcallocPoolStats,
}

impl HolisticKcallocPool {
    pub fn new(reserve_pages: u32) -> Self {
        Self {
            slabs: BTreeMap::new(), objects: BTreeMap::new(),
            cpu_caches: BTreeMap::new(), reserve: EmergencyReserve::new(reserve_pages),
            cache_hits: 0, cache_misses: 0,
            leak_threshold_ns: 300_000_000_000, // 5 minutes
            stats: KcallocPoolStats::default(),
        }
    }

    pub fn init_cpu_cache(&mut self, cpu: u32, class: SizeClass, capacity: usize) {
        let key = (cpu, class.slab_size());
        self.cpu_caches.insert(key, CpuCache::new(cpu, class, capacity));
    }

    pub fn allocate(&mut self, size: usize, cpu: u32, pid: u32, site: u64, ts: u64) -> Option<u64> {
        let class = SizeClass::from_size(size);
        let key = (cpu, class.slab_size());

        // Try CPU cache first
        if let Some(cache) = self.cpu_caches.get_mut(&key) {
            if let Some(addr) = cache.try_alloc() {
                self.cache_hits += 1;
                self.objects.insert(addr, ObjectHeader {
                    addr, size, state: AllocState::Allocated, alloc_ts: ts,
                    free_ts: None, alloc_cpu: cpu, owner_pid: pid, alloc_site: site,
                });
                return Some(addr);
            }
        }

        self.cache_misses += 1;
        // Allocate from slab (simulated: use next available address)
        let addr = (self.objects.len() as u64 + 1) * 0x1000 + size as u64;
        self.objects.insert(addr, ObjectHeader {
            addr, size, state: AllocState::Allocated, alloc_ts: ts,
            free_ts: None, alloc_cpu: cpu, owner_pid: pid, alloc_site: site,
        });
        Some(addr)
    }

    pub fn free(&mut self, addr: u64, cpu: u32, ts: u64) {
        if let Some(obj) = self.objects.get_mut(&addr) {
            obj.state = AllocState::Free;
            obj.free_ts = Some(ts);
            let class = SizeClass::from_size(obj.size);
            let key = (cpu, class.slab_size());
            if let Some(cache) = self.cpu_caches.get_mut(&key) {
                cache.try_free(addr);
            }
        }
    }

    pub fn detect_leaks(&self, now: u64) -> Vec<LeakSuspect> {
        self.objects.values()
            .filter(|o| o.state == AllocState::Allocated && now.saturating_sub(o.alloc_ts) > self.leak_threshold_ns)
            .map(|o| LeakSuspect {
                addr: o.addr, size: o.size, alloc_ts: o.alloc_ts,
                age_ns: now.saturating_sub(o.alloc_ts), alloc_site: o.alloc_site, pid: o.owner_pid,
            })
            .collect()
    }

    pub fn recompute(&mut self) {
        let active: Vec<&ObjectHeader> = self.objects.values().filter(|o| o.state == AllocState::Allocated).collect();
        self.stats.active_objects = active.len() as u64;
        self.stats.active_bytes = active.iter().map(|o| o.size as u64).sum();
        self.stats.total_allocated = self.objects.len() as u64;
        self.stats.total_freed = self.objects.values().filter(|o| o.state == AllocState::Free).count() as u64;
        self.stats.slab_pages = self.slabs.len();
        self.stats.cpu_caches = self.cpu_caches.len();
        let total_cache_ops = self.cache_hits + self.cache_misses;
        self.stats.cache_hit_rate = if total_cache_ops > 0 { self.cache_hits as f64 / total_cache_ops as f64 } else { 0.0 };
        if !self.slabs.is_empty() {
            self.stats.avg_fragmentation = self.slabs.values().map(|s| s.fragmentation).sum::<f64>() / self.slabs.len() as f64;
        }
        self.stats.emergency_allocs = self.reserve.emergency_allocs;
    }

    pub fn stats(&self) -> &KcallocPoolStats { &self.stats }
}
