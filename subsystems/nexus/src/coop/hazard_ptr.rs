// SPDX-License-Identifier: GPL-2.0
//! Coop hazard_ptr — hazard pointer-based memory reclamation for lock-free structures.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Hazard pointer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardState {
    /// Slot is free
    Free,
    /// Slot is active — protecting a pointer
    Active,
    /// Slot reserved but no pointer set yet
    Reserved,
}

/// A single hazard pointer slot
#[derive(Debug, Clone)]
pub struct HazardSlot {
    pub slot_id: u64,
    pub thread_id: u64,
    pub state: HazardState,
    pub protected_addr: u64,
    pub acquire_ns: u64,
    pub protect_count: u64,
}

impl HazardSlot {
    pub fn new(slot_id: u64, thread_id: u64) -> Self {
        Self {
            slot_id,
            thread_id,
            state: HazardState::Free,
            protected_addr: 0,
            acquire_ns: 0,
            protect_count: 0,
        }
    }

    pub fn protect(&mut self, addr: u64, now_ns: u64) {
        self.state = HazardState::Active;
        self.protected_addr = addr;
        self.acquire_ns = now_ns;
        self.protect_count += 1;
    }

    pub fn release(&mut self) {
        self.state = HazardState::Free;
        self.protected_addr = 0;
    }

    pub fn is_protecting(&self, addr: u64) -> bool {
        self.state == HazardState::Active && self.protected_addr == addr
    }

    pub fn hold_duration(&self, now_ns: u64) -> u64 {
        if self.state != HazardState::Active { return 0; }
        now_ns.saturating_sub(self.acquire_ns)
    }
}

/// Retired node awaiting reclamation
#[derive(Debug)]
pub struct RetiredNode {
    pub addr: u64,
    pub size_bytes: usize,
    pub retire_epoch: u64,
    pub retire_ns: u64,
    pub owner_thread: u64,
}

impl RetiredNode {
    pub fn new(addr: u64, size: usize, epoch: u64, thread: u64, now_ns: u64) -> Self {
        Self {
            addr,
            size_bytes: size,
            retire_epoch: epoch,
            retire_ns: now_ns,
            owner_thread: thread,
        }
    }
}

/// Per-thread hazard pointer context
#[derive(Debug)]
pub struct ThreadHazardCtx {
    pub thread_id: u64,
    pub slots: Vec<HazardSlot>,
    pub retired_list: Vec<RetiredNode>,
    pub total_protects: u64,
    pub total_retires: u64,
    pub total_reclaims: u64,
    pub reclaimed_bytes: u64,
    pub scan_count: u64,
    max_slots: usize,
}

impl ThreadHazardCtx {
    pub fn new(thread_id: u64, max_slots: usize) -> Self {
        Self {
            thread_id,
            slots: Vec::new(),
            retired_list: Vec::new(),
            total_protects: 0,
            total_retires: 0,
            total_reclaims: 0,
            reclaimed_bytes: 0,
            scan_count: 0,
            max_slots,
        }
    }

    pub fn acquire_slot(&mut self) -> Option<u64> {
        // Find a free slot
        for slot in &mut self.slots {
            if slot.state == HazardState::Free {
                slot.state = HazardState::Reserved;
                return Some(slot.slot_id);
            }
        }
        // Create new slot if under limit
        if self.slots.len() < self.max_slots {
            let id = self.slots.len() as u64;
            let mut slot = HazardSlot::new(id, self.thread_id);
            slot.state = HazardState::Reserved;
            self.slots.push(slot);
            return Some(id);
        }
        None
    }

    pub fn protect(&mut self, slot_id: u64, addr: u64, now_ns: u64) -> bool {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.slot_id == slot_id) {
            slot.protect(addr, now_ns);
            self.total_protects += 1;
            true
        } else {
            false
        }
    }

    pub fn release_slot(&mut self, slot_id: u64) {
        if let Some(slot) = self.slots.iter_mut().find(|s| s.slot_id == slot_id) {
            slot.release();
        }
    }

    pub fn retire(&mut self, addr: u64, size: usize, epoch: u64, now_ns: u64) {
        self.retired_list.push(RetiredNode::new(addr, size, epoch, self.thread_id, now_ns));
        self.total_retires += 1;
    }

    pub fn active_protections(&self) -> Vec<u64> {
        self.slots.iter()
            .filter(|s| s.state == HazardState::Active)
            .map(|s| s.protected_addr)
            .collect()
    }

    pub fn retired_count(&self) -> usize {
        self.retired_list.len()
    }

    pub fn retired_bytes(&self) -> u64 {
        self.retired_list.iter().map(|r| r.size_bytes as u64).sum()
    }

    pub fn needs_scan(&self, threshold: usize) -> bool {
        self.retired_list.len() >= threshold
    }
}

/// Hazard pointer domain — manages all threads' hazard pointers
#[derive(Debug)]
pub struct HazardDomain {
    pub id: u64,
    pub max_slots_per_thread: usize,
    pub scan_threshold: usize,
    threads: BTreeMap<u64, ThreadHazardCtx>,
    pub current_epoch: u64,
}

impl HazardDomain {
    pub fn new(id: u64, max_slots: usize) -> Self {
        Self {
            id,
            max_slots_per_thread: max_slots,
            scan_threshold: 64,
            threads: BTreeMap::new(),
            current_epoch: 0,
        }
    }

    pub fn register_thread(&mut self, thread_id: u64) {
        self.threads.insert(thread_id, ThreadHazardCtx::new(thread_id, self.max_slots_per_thread));
    }

    pub fn unregister_thread(&mut self, thread_id: u64) -> Vec<RetiredNode> {
        if let Some(ctx) = self.threads.remove(&thread_id) {
            ctx.retired_list
        } else {
            Vec::new()
        }
    }

    /// Collect all currently protected addresses across all threads
    fn collect_protected(&self) -> Vec<u64> {
        let mut protected = Vec::new();
        for ctx in self.threads.values() {
            protected.extend(ctx.active_protections());
        }
        protected
    }

    /// Scan and reclaim safe-to-free retired nodes for a given thread
    pub fn scan(&mut self, thread_id: u64) -> (u64, u64) {
        let protected = self.collect_protected();
        let mut count = 0u64;
        let mut bytes = 0u64;

        if let Some(ctx) = self.threads.get_mut(&thread_id) {
            ctx.scan_count += 1;
            ctx.retired_list.retain(|node| {
                if protected.contains(&node.addr) {
                    true // still protected, keep
                } else {
                    count += 1;
                    bytes += node.size_bytes as u64;
                    false // safe to reclaim
                }
            });
            ctx.total_reclaims += count;
            ctx.reclaimed_bytes += bytes;
        }
        (count, bytes)
    }

    /// Scan all threads that exceed the threshold
    pub fn scan_all(&mut self) -> (u64, u64) {
        let thread_ids: Vec<u64> = self.threads.keys().copied()
            .filter(|tid| {
                self.threads.get(tid)
                    .map(|ctx| ctx.needs_scan(self.scan_threshold))
                    .unwrap_or(false)
            })
            .collect();

        let mut total_count = 0u64;
        let mut total_bytes = 0u64;
        for tid in thread_ids {
            let (c, b) = self.scan(tid);
            total_count += c;
            total_bytes += b;
        }
        (total_count, total_bytes)
    }

    pub fn total_retired(&self) -> u64 {
        self.threads.values().map(|c| c.retired_count() as u64).sum()
    }

    pub fn total_retired_bytes(&self) -> u64 {
        self.threads.values().map(|c| c.retired_bytes()).sum()
    }

    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }
}

/// Hazard pointer stats
#[derive(Debug, Clone)]
pub struct HazardPtrStats {
    pub total_domains: u64,
    pub total_threads: u64,
    pub total_protects: u64,
    pub total_retires: u64,
    pub total_reclaims: u64,
    pub total_reclaimed_bytes: u64,
    pub pending_retired: u64,
    pub pending_bytes: u64,
}

/// Main hazard pointer manager
pub struct CoopHazardPtr {
    domains: BTreeMap<u64, HazardDomain>,
    next_domain_id: u64,
    stats: HazardPtrStats,
}

impl CoopHazardPtr {
    pub fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            next_domain_id: 1,
            stats: HazardPtrStats {
                total_domains: 0,
                total_threads: 0,
                total_protects: 0,
                total_retires: 0,
                total_reclaims: 0,
                total_reclaimed_bytes: 0,
                pending_retired: 0,
                pending_bytes: 0,
            },
        }
    }

    pub fn create_domain(&mut self, max_slots: usize) -> u64 {
        let id = self.next_domain_id;
        self.next_domain_id += 1;
        self.domains.insert(id, HazardDomain::new(id, max_slots));
        self.stats.total_domains += 1;
        id
    }

    pub fn register_thread(&mut self, domain_id: u64, thread_id: u64) {
        if let Some(d) = self.domains.get_mut(&domain_id) {
            d.register_thread(thread_id);
            self.stats.total_threads += 1;
        }
    }

    pub fn scan_domain(&mut self, domain_id: u64) -> (u64, u64) {
        if let Some(d) = self.domains.get_mut(&domain_id) {
            let (c, b) = d.scan_all();
            self.stats.total_reclaims += c;
            self.stats.total_reclaimed_bytes += b;
            (c, b)
        } else {
            (0, 0)
        }
    }

    pub fn scan_all_domains(&mut self) -> (u64, u64) {
        let ids: Vec<u64> = self.domains.keys().copied().collect();
        let mut tc = 0u64;
        let mut tb = 0u64;
        for id in ids {
            let (c, b) = self.scan_domain(id);
            tc += c;
            tb += b;
        }
        (tc, tb)
    }

    pub fn total_pending(&self) -> (u64, u64) {
        let count: u64 = self.domains.values().map(|d| d.total_retired()).sum();
        let bytes: u64 = self.domains.values().map(|d| d.total_retired_bytes()).sum();
        (count, bytes)
    }

    pub fn get_domain(&self, id: u64) -> Option<&HazardDomain> {
        self.domains.get(&id)
    }

    pub fn stats(&self) -> &HazardPtrStats {
        &self.stats
    }
}

// ============================================================================
// Merged from hazard_ptr_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HpState {
    Free,
    Active,
    Protecting,
}

/// Hazard pointer
#[derive(Debug)]
pub struct HazardPointerV2 {
    pub id: u64,
    pub owner_tid: u64,
    pub protected_addr: u64,
    pub state: HpState,
    pub protect_count: u64,
}

impl HazardPointerV2 {
    pub fn new(id: u64, tid: u64) -> Self {
        Self { id, owner_tid: tid, protected_addr: 0, state: HpState::Free, protect_count: 0 }
    }

    pub fn protect(&mut self, addr: u64) { self.protected_addr = addr; self.state = HpState::Protecting; self.protect_count += 1; }
    pub fn release(&mut self) { self.protected_addr = 0; self.state = HpState::Free; }
    pub fn is_protecting(&self, addr: u64) -> bool { self.state == HpState::Protecting && self.protected_addr == addr }
}

/// Retired node
#[derive(Debug)]
pub struct RetiredNodeV2 {
    pub addr: u64,
    pub size: u64,
    pub retired_at: u64,
    pub retire_tid: u64,
}

/// Thread local HP domain
#[derive(Debug)]
pub struct HpThreadV2 {
    pub tid: u64,
    pub hazard_ptrs: Vec<HazardPointerV2>,
    pub retired: Vec<RetiredNodeV2>,
    pub scan_count: u64,
    pub reclaimed_count: u64,
    pub reclaimed_bytes: u64,
}

impl HpThreadV2 {
    pub fn new(tid: u64, hp_count: u32) -> Self {
        let hps = (0..hp_count).map(|i| HazardPointerV2::new(i as u64, tid)).collect();
        Self { tid, hazard_ptrs: hps, retired: Vec::new(), scan_count: 0, reclaimed_count: 0, reclaimed_bytes: 0 }
    }

    pub fn protect(&mut self, slot: usize, addr: u64) {
        if slot < self.hazard_ptrs.len() { self.hazard_ptrs[slot].protect(addr); }
    }

    pub fn release(&mut self, slot: usize) {
        if slot < self.hazard_ptrs.len() { self.hazard_ptrs[slot].release(); }
    }

    pub fn retire(&mut self, addr: u64, size: u64, now: u64) {
        self.retired.push(RetiredNodeV2 { addr, size, retired_at: now, retire_tid: self.tid });
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct HazardPtrV2Stats {
    pub total_threads: u32,
    pub total_hazard_ptrs: u32,
    pub active_protections: u32,
    pub pending_retired: u32,
    pub total_reclaimed: u64,
    pub total_reclaimed_bytes: u64,
}

/// Main hazard pointer v2 manager
pub struct CoopHazardPtrV2 {
    threads: BTreeMap<u64, HpThreadV2>,
    hp_per_thread: u32,
}

impl CoopHazardPtrV2 {
    pub fn new(hp_per_thread: u32) -> Self { Self { threads: BTreeMap::new(), hp_per_thread } }

    pub fn register(&mut self, tid: u64) { self.threads.insert(tid, HpThreadV2::new(tid, self.hp_per_thread)); }

    pub fn protect(&mut self, tid: u64, slot: usize, addr: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.protect(slot, addr); }
    }

    pub fn retire(&mut self, tid: u64, addr: u64, size: u64, now: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.retire(addr, size, now); }
    }

    pub fn scan(&mut self, tid: u64) -> u64 {
        let protected: Vec<u64> = self.threads.values().flat_map(|t| &t.hazard_ptrs).filter(|hp| hp.state == HpState::Protecting).map(|hp| hp.protected_addr).collect();
        if let Some(thread) = self.threads.get_mut(&tid) {
            thread.scan_count += 1;
            let mut reclaimed = 0u64;
            let mut kept = Vec::new();
            for r in thread.retired.drain(..) {
                if !protected.contains(&r.addr) { thread.reclaimed_count += 1; thread.reclaimed_bytes += r.size; reclaimed += r.size; }
                else { kept.push(r); }
            }
            thread.retired = kept;
            reclaimed
        } else { 0 }
    }

    pub fn stats(&self) -> HazardPtrV2Stats {
        let hps: u32 = self.threads.values().map(|t| t.hazard_ptrs.len() as u32).sum();
        let active: u32 = self.threads.values().flat_map(|t| &t.hazard_ptrs).filter(|hp| hp.state == HpState::Protecting).count() as u32;
        let retired: u32 = self.threads.values().map(|t| t.retired.len() as u32).sum();
        let reclaimed: u64 = self.threads.values().map(|t| t.reclaimed_count).sum();
        let bytes: u64 = self.threads.values().map(|t| t.reclaimed_bytes).sum();
        HazardPtrV2Stats { total_threads: self.threads.len() as u32, total_hazard_ptrs: hps, active_protections: active, pending_retired: retired, total_reclaimed: reclaimed, total_reclaimed_bytes: bytes }
    }
}

// ============================================================================
// Merged from hazard_ptr_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardSlotState {
    Empty,
    Guarding,
    Released,
}

/// Hazard slot
#[derive(Debug)]
pub struct HazardSlot {
    pub slot_id: u64,
    pub owner: u64,
    pub guarded_addr: u64,
    pub state: HazardSlotState,
    pub guard_count: u64,
}

impl HazardSlot {
    pub fn new(slot_id: u64, owner: u64) -> Self {
        Self { slot_id, owner, guarded_addr: 0, state: HazardSlotState::Empty, guard_count: 0 }
    }

    pub fn guard(&mut self, addr: u64) { self.guarded_addr = addr; self.state = HazardSlotState::Guarding; self.guard_count += 1; }
    pub fn clear(&mut self) { self.guarded_addr = 0; self.state = HazardSlotState::Empty; }
}

/// Retired object
#[derive(Debug)]
pub struct RetiredObjV3 {
    pub addr: u64,
    pub size_bytes: u64,
    pub retire_tick: u64,
}

/// Hazard domain v3
#[derive(Debug)]
pub struct HazardDomainV3 {
    pub slots: Vec<HazardSlot>,
    pub retired_list: Vec<RetiredObjV3>,
    pub reclaim_threshold: usize,
    pub total_reclaimed: u64,
    pub total_reclaimed_bytes: u64,
    pub scans: u64,
    pub next_slot_id: u64,
}

impl HazardDomainV3 {
    pub fn new(threshold: usize) -> Self {
        Self { slots: Vec::new(), retired_list: Vec::new(), reclaim_threshold: threshold, total_reclaimed: 0, total_reclaimed_bytes: 0, scans: 0, next_slot_id: 1 }
    }

    pub fn allocate_slot(&mut self, owner: u64) -> u64 {
        let id = self.next_slot_id; self.next_slot_id += 1;
        self.slots.push(HazardSlot::new(id, owner));
        id
    }

    pub fn guard(&mut self, slot_id: u64, addr: u64) {
        if let Some(s) = self.slots.iter_mut().find(|s| s.slot_id == slot_id) { s.guard(addr); }
    }

    pub fn clear(&mut self, slot_id: u64) {
        if let Some(s) = self.slots.iter_mut().find(|s| s.slot_id == slot_id) { s.clear(); }
    }

    pub fn retire(&mut self, addr: u64, size: u64, tick: u64) {
        self.retired_list.push(RetiredObjV3 { addr, size_bytes: size, retire_tick: tick });
        if self.retired_list.len() >= self.reclaim_threshold { self.scan(); }
    }

    pub fn scan(&mut self) -> u64 {
        self.scans += 1;
        let guarded: Vec<u64> = self.slots.iter().filter(|s| s.state == HazardSlotState::Guarding).map(|s| s.guarded_addr).collect();
        let mut freed = 0u64;
        self.retired_list.retain(|r| {
            if !guarded.contains(&r.addr) { freed += r.size_bytes; self.total_reclaimed += 1; self.total_reclaimed_bytes += r.size_bytes; false }
            else { true }
        });
        freed
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct HazardPtrV3Stats {
    pub total_slots: u32,
    pub guarding_slots: u32,
    pub pending_retired: u32,
    pub total_reclaimed_bytes: u64,
    pub total_scans: u64,
}

/// Main coop hazard ptr v3
pub struct CoopHazardPtrV3 {
    domain: HazardDomainV3,
}

impl CoopHazardPtrV3 {
    pub fn new(threshold: usize) -> Self { Self { domain: HazardDomainV3::new(threshold) } }
    pub fn allocate(&mut self, owner: u64) -> u64 { self.domain.allocate_slot(owner) }
    pub fn guard(&mut self, slot: u64, addr: u64) { self.domain.guard(slot, addr); }
    pub fn clear(&mut self, slot: u64) { self.domain.clear(slot); }
    pub fn retire(&mut self, addr: u64, size: u64, tick: u64) { self.domain.retire(addr, size, tick); }

    pub fn stats(&self) -> HazardPtrV3Stats {
        let guarding = self.domain.slots.iter().filter(|s| s.state == HazardSlotState::Guarding).count() as u32;
        HazardPtrV3Stats { total_slots: self.domain.slots.len() as u32, guarding_slots: guarding, pending_retired: self.domain.retired_list.len() as u32, total_reclaimed_bytes: self.domain.total_reclaimed_bytes, total_scans: self.domain.scans }
    }
}

// ============================================================================
// Merged from hazard_ptr_v4
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardV4State {
    Free,
    Active,
    Retired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardV4Domain {
    Default,
    Custom(u32),
}

#[derive(Debug, Clone)]
pub struct HazardV4Pointer {
    pub slot_id: u32,
    pub thread_id: u32,
    pub protected_addr: AtomicU64,
    pub state: HazardV4State,
    pub acquisitions: u64,
    pub releases: u64,
}

impl HazardV4Pointer {
    pub fn new(slot_id: u32, thread_id: u32) -> Self {
        Self {
            slot_id, thread_id,
            protected_addr: AtomicU64::new(0),
            state: HazardV4State::Free,
            acquisitions: 0, releases: 0,
        }
    }

    pub fn protect(&mut self, addr: u64) {
        self.protected_addr.store(addr, Ordering::Release);
        self.state = HazardV4State::Active;
        self.acquisitions += 1;
    }

    pub fn release(&mut self) {
        self.protected_addr.store(0, Ordering::Release);
        self.state = HazardV4State::Free;
        self.releases += 1;
    }

    pub fn is_protecting(&self, addr: u64) -> bool {
        self.protected_addr.load(Ordering::Acquire) == addr
    }
}

#[derive(Debug, Clone)]
pub struct HazardV4RetiredNode {
    pub addr: u64,
    pub retire_epoch: u64,
    pub size_bytes: u32,
}

#[derive(Debug, Clone)]
pub struct HazardV4ThreadState {
    pub thread_id: u32,
    pub slots: Vec<HazardV4Pointer>,
    pub retired_list: Vec<HazardV4RetiredNode>,
    pub reclaimed_count: u64,
    pub reclaimed_bytes: u64,
    pub scan_count: u64,
}

impl HazardV4ThreadState {
    pub fn new(thread_id: u32, num_slots: u32) -> Self {
        let slots = (0..num_slots).map(|i| HazardV4Pointer::new(i, thread_id)).collect();
        Self {
            thread_id, slots, retired_list: Vec::new(),
            reclaimed_count: 0, reclaimed_bytes: 0, scan_count: 0,
        }
    }

    pub fn retire(&mut self, addr: u64, size: u32, epoch: u64) {
        self.retired_list.push(HazardV4RetiredNode { addr, retire_epoch: epoch, size_bytes: size });
    }

    pub fn scan_and_reclaim(&mut self, protected_addrs: &[u64]) -> u64 {
        self.scan_count += 1;
        let before = self.retired_list.len();
        self.retired_list.retain(|node| {
            protected_addrs.contains(&node.addr)
        });
        let reclaimed = (before - self.retired_list.len()) as u64;
        self.reclaimed_count += reclaimed;
        reclaimed
    }

    pub fn retired_count(&self) -> usize { self.retired_list.len() }
}

#[derive(Debug, Clone)]
pub struct HazardV4Stats {
    pub total_threads: u32,
    pub total_slots: u32,
    pub total_retired: u64,
    pub total_reclaimed: u64,
    pub total_scans: u64,
    pub pending_reclamation: u64,
}

pub struct CoopHazardPtrV4 {
    threads: BTreeMap<u32, HazardV4ThreadState>,
    domain: HazardV4Domain,
    slots_per_thread: u32,
    epoch: AtomicU64,
    stats: HazardV4Stats,
}

impl CoopHazardPtrV4 {
    pub fn new(slots_per_thread: u32, domain: HazardV4Domain) -> Self {
        Self {
            threads: BTreeMap::new(),
            domain, slots_per_thread,
            epoch: AtomicU64::new(0),
            stats: HazardV4Stats {
                total_threads: 0, total_slots: 0,
                total_retired: 0, total_reclaimed: 0,
                total_scans: 0, pending_reclamation: 0,
            },
        }
    }

    pub fn register_thread(&mut self, id: u32) {
        self.threads.insert(id, HazardV4ThreadState::new(id, self.slots_per_thread));
        self.stats.total_threads += 1;
        self.stats.total_slots += self.slots_per_thread;
    }

    pub fn retire(&mut self, thread_id: u32, addr: u64, size: u32) {
        let epoch = self.epoch.load(Ordering::Relaxed);
        if let Some(t) = self.threads.get_mut(&thread_id) {
            t.retire(addr, size, epoch);
            self.stats.total_retired += 1;
            self.stats.pending_reclamation += 1;
        }
    }

    pub fn stats(&self) -> &HazardV4Stats { &self.stats }
}
