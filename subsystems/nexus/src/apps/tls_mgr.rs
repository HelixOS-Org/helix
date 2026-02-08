//! # Apps TLS Manager
//!
//! Thread-local storage management:
//! - TLS area allocation and tracking per thread
//! - TLS variant (exec/dynamic) classification
//! - TLS segment size monitoring
//! - Dtv (dynamic thread vector) tracking
//! - TLS initialization overhead measurement
//! - Thread exit cleanup tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// TLS variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVariant {
    ExecI,
    ExecII,
    Dynamic,
}

/// TLS module entry
#[derive(Debug, Clone)]
pub struct TlsModule {
    pub module_id: u32,
    pub variant: TlsVariant,
    pub base_addr: u64,
    pub size: usize,
    pub align: usize,
    pub init_size: usize,
    pub is_static: bool,
}

impl TlsModule {
    pub fn new(id: u32, variant: TlsVariant, addr: u64, size: usize, align: usize) -> Self {
        Self { module_id: id, variant, base_addr: addr, size, align, init_size: size, is_static: variant != TlsVariant::Dynamic }
    }

    pub fn aligned_size(&self) -> usize {
        if self.align == 0 { return self.size; }
        (self.size + self.align - 1) & !(self.align - 1)
    }
}

/// Per-thread TLS state
#[derive(Debug, Clone)]
pub struct ThreadTlsState {
    pub tid: u64,
    pub tls_base: u64,
    pub dtv_addr: u64,
    pub dtv_generation: u32,
    pub dtv_slots: u32,
    pub static_area_size: usize,
    pub dynamic_allocs: u32,
    pub init_time_ns: u64,
    pub cleanup_pending: bool,
}

impl ThreadTlsState {
    pub fn new(tid: u64, base: u64, ts: u64) -> Self {
        Self {
            tid, tls_base: base, dtv_addr: 0, dtv_generation: 0,
            dtv_slots: 0, static_area_size: 0, dynamic_allocs: 0,
            init_time_ns: 0, cleanup_pending: false,
        }
    }

    pub fn record_dynamic_alloc(&mut self) { self.dynamic_allocs += 1; }
    pub fn update_dtv(&mut self, gen: u32, slots: u32) { self.dtv_generation = gen; self.dtv_slots = slots; }
    pub fn total_overhead(&self) -> usize { self.static_area_size + (self.dtv_slots as usize * 16) }
}

/// TLS image info (per module loaded)
#[derive(Debug, Clone)]
pub struct TlsImage {
    pub module_id: u32,
    pub init_data_size: usize,
    pub bss_size: usize,
    pub alignment: usize,
    pub ref_count: u32,
}

impl TlsImage {
    pub fn new(id: u32, init: usize, bss: usize, align: usize) -> Self {
        Self { module_id: id, init_data_size: init, bss_size: bss, alignment: align, ref_count: 0 }
    }
    pub fn total_size(&self) -> usize { self.init_data_size + self.bss_size }
}

/// TLS manager stats
#[derive(Debug, Clone, Default)]
pub struct TlsMgrStats {
    pub tracked_threads: usize,
    pub total_modules: usize,
    pub static_modules: usize,
    pub dynamic_modules: usize,
    pub total_tls_memory: usize,
    pub avg_per_thread_size: f64,
    pub total_dynamic_allocs: u64,
    pub max_dtv_slots: u32,
}

/// Apps TLS manager
pub struct AppsTlsMgr {
    threads: BTreeMap<u64, ThreadTlsState>,
    modules: BTreeMap<u32, TlsModule>,
    images: BTreeMap<u32, TlsImage>,
    next_module_id: u32,
    stats: TlsMgrStats,
}

impl AppsTlsMgr {
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new(), modules: BTreeMap::new(),
            images: BTreeMap::new(), next_module_id: 1,
            stats: TlsMgrStats::default(),
        }
    }

    pub fn register_module(&mut self, variant: TlsVariant, addr: u64, size: usize, align: usize) -> u32 {
        let id = self.next_module_id;
        self.next_module_id += 1;
        self.modules.insert(id, TlsModule::new(id, variant, addr, size, align));
        id
    }

    pub fn register_thread(&mut self, tid: u64, base: u64, ts: u64) {
        self.threads.insert(tid, ThreadTlsState::new(tid, base, ts));
    }

    pub fn record_init(&mut self, tid: u64, static_size: usize, init_ns: u64) {
        if let Some(t) = self.threads.get_mut(&tid) {
            t.static_area_size = static_size;
            t.init_time_ns = init_ns;
        }
    }

    pub fn update_dtv(&mut self, tid: u64, gen: u32, slots: u32) {
        if let Some(t) = self.threads.get_mut(&tid) { t.update_dtv(gen, slots); }
    }

    pub fn record_dynamic_alloc(&mut self, tid: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.record_dynamic_alloc(); }
    }

    pub fn thread_exit(&mut self, tid: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.cleanup_pending = true; }
    }

    pub fn cleanup_thread(&mut self, tid: u64) { self.threads.remove(&tid); }

    pub fn gc_exited(&mut self) { self.threads.retain(|_, t| !t.cleanup_pending); }

    pub fn recompute(&mut self) {
        self.stats.tracked_threads = self.threads.len();
        self.stats.total_modules = self.modules.len();
        self.stats.static_modules = self.modules.values().filter(|m| m.is_static).count();
        self.stats.dynamic_modules = self.modules.values().filter(|m| !m.is_static).count();
        self.stats.total_tls_memory = self.threads.values().map(|t| t.total_overhead()).sum();
        if self.stats.tracked_threads > 0 {
            self.stats.avg_per_thread_size = self.stats.total_tls_memory as f64 / self.stats.tracked_threads as f64;
        }
        self.stats.total_dynamic_allocs = self.threads.values().map(|t| t.dynamic_allocs as u64).sum();
        self.stats.max_dtv_slots = self.threads.values().map(|t| t.dtv_slots).max().unwrap_or(0);
    }

    pub fn thread(&self, tid: u64) -> Option<&ThreadTlsState> { self.threads.get(&tid) }
    pub fn module(&self, id: u32) -> Option<&TlsModule> { self.modules.get(&id) }
    pub fn stats(&self) -> &TlsMgrStats { &self.stats }
}
