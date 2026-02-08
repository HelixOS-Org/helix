//! # Holistic TLB Management
//!
//! System-wide TLB optimization and shootdown management:
//! - TLB shootdown batching and coalescing
//! - Per-CPU TLB flush tracking
//! - Huge page promotion/demotion decisions
//! - TLB pressure estimation
//! - PCID/ASID management
//! - Cross-CPU shootdown latency tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// TLB flush reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbFlushReason {
    PageTableChange,
    ProcessSwitch,
    KernelMapping,
    HugePageSplit,
    HugePageCollapse,
    MprotectChange,
    MunmapRange,
    FullFlush,
}

/// Page size for TLB entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbPageSize {
    Page4K,
    Page2M,
    Page1G,
}

/// TLB shootdown request
#[derive(Debug, Clone)]
pub struct ShootdownRequest {
    pub request_id: u64,
    pub initiator_cpu: u32,
    pub target_cpus: Vec<u32>,
    pub start_va: u64,
    pub end_va: u64,
    pub page_size: TlbPageSize,
    pub reason: TlbFlushReason,
    pub submitted_ns: u64,
    pub completed_ns: u64,
    pub pages_flushed: u64,
}

impl ShootdownRequest {
    pub fn new(id: u64, cpu: u32, start: u64, end: u64, reason: TlbFlushReason, now: u64) -> Self {
        Self {
            request_id: id,
            initiator_cpu: cpu,
            target_cpus: Vec::new(),
            start_va: start,
            end_va: end,
            page_size: TlbPageSize::Page4K,
            reason,
            submitted_ns: now,
            completed_ns: 0,
            pages_flushed: 0,
        }
    }

    pub fn range_pages(&self, page_size: u64) -> u64 {
        if page_size == 0 { return 0; }
        (self.end_va - self.start_va) / page_size
    }

    pub fn latency_ns(&self) -> u64 {
        self.completed_ns.saturating_sub(self.submitted_ns)
    }
}

/// PCID/ASID slot
#[derive(Debug, Clone)]
pub struct PcidSlot {
    pub pcid: u16,
    pub process_id: u64,
    pub generation: u64,
    pub last_use_ns: u64,
}

/// Per-CPU TLB state
#[derive(Debug, Clone)]
pub struct CpuTlbState {
    pub cpu_id: u32,
    pub total_flushes: u64,
    pub full_flushes: u64,
    pub range_flushes: u64,
    pub shootdowns_received: u64,
    pub shootdowns_sent: u64,
    pub total_flush_latency_ns: u64,
    pub max_flush_latency_ns: u64,
    pub pcid_slots: Vec<PcidSlot>,
    pub pcid_capacity: u16,
    pub pcid_evictions: u64,
}

impl CpuTlbState {
    pub fn new(cpu_id: u32, pcid_capacity: u16) -> Self {
        Self {
            cpu_id,
            total_flushes: 0,
            full_flushes: 0,
            range_flushes: 0,
            shootdowns_received: 0,
            shootdowns_sent: 0,
            total_flush_latency_ns: 0,
            max_flush_latency_ns: 0,
            pcid_slots: Vec::new(),
            pcid_capacity,
            pcid_evictions: 0,
        }
    }

    pub fn avg_flush_ns(&self) -> u64 {
        if self.total_flushes == 0 { return 0; }
        self.total_flush_latency_ns / self.total_flushes
    }

    pub fn pcid_hit_rate(&self) -> f64 {
        let total = self.total_flushes;
        if total == 0 { return 1.0; }
        let saved = total.saturating_sub(self.full_flushes);
        saved as f64 / total as f64
    }

    pub fn assign_pcid(&mut self, process_id: u64, generation: u64, now_ns: u64) -> u16 {
        // Check if already assigned
        for slot in &mut self.pcid_slots {
            if slot.process_id == process_id {
                slot.last_use_ns = now_ns;
                slot.generation = generation;
                return slot.pcid;
            }
        }

        // Allocate new slot
        if (self.pcid_slots.len() as u16) < self.pcid_capacity {
            let pcid = self.pcid_slots.len() as u16;
            self.pcid_slots.push(PcidSlot {
                pcid,
                process_id,
                generation,
                last_use_ns: now_ns,
            });
            return pcid;
        }

        // Evict LRU
        self.pcid_evictions += 1;
        let lru_idx = self.pcid_slots.iter()
            .enumerate()
            .min_by_key(|(_, s)| s.last_use_ns)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let pcid = self.pcid_slots[lru_idx].pcid;
        self.pcid_slots[lru_idx] = PcidSlot {
            pcid,
            process_id,
            generation,
            last_use_ns: now_ns,
        };
        pcid
    }
}

/// Huge page promotion candidate
#[derive(Debug, Clone)]
pub struct HugePageCandidate {
    pub va_start: u64,
    pub process_id: u64,
    pub current_size: TlbPageSize,
    pub target_size: TlbPageSize,
    pub access_count: u64,
    pub benefit_score: f64,
}

/// Shootdown batch
#[derive(Debug, Clone)]
pub struct ShootdownBatch {
    pub requests: Vec<ShootdownRequest>,
    pub coalesced_ranges: Vec<(u64, u64)>,
}

impl ShootdownBatch {
    pub fn new() -> Self {
        Self { requests: Vec::new(), coalesced_ranges: Vec::new() }
    }

    pub fn add(&mut self, req: ShootdownRequest) {
        self.requests.push(req);
    }

    /// Coalesce overlapping ranges
    pub fn coalesce(&mut self) {
        if self.requests.is_empty() { return; }
        let mut ranges: Vec<(u64, u64)> = self.requests.iter()
            .map(|r| (r.start_va, r.end_va))
            .collect();
        ranges.sort_by_key(|r| r.0);

        self.coalesced_ranges.clear();
        let mut cur = ranges[0];
        for &(start, end) in &ranges[1..] {
            if start <= cur.1 {
                if end > cur.1 { cur.1 = end; }
            } else {
                self.coalesced_ranges.push(cur);
                cur = (start, end);
            }
        }
        self.coalesced_ranges.push(cur);
    }

    pub fn total_pages_4k(&self) -> u64 {
        self.coalesced_ranges.iter()
            .map(|(s, e)| (e - s) / 4096)
            .sum()
    }
}

/// Holistic TLB Management stats
#[derive(Debug, Clone, Default)]
pub struct HolisticTlbMgrStats {
    pub total_cpus: usize,
    pub total_flushes: u64,
    pub total_shootdowns: u64,
    pub avg_shootdown_latency_ns: u64,
    pub coalesce_savings_pct: f64,
    pub avg_pcid_hit_rate: f64,
}

/// Holistic TLB Manager
pub struct HolisticTlbMgr {
    cpus: BTreeMap<u32, CpuTlbState>,
    pending_batch: ShootdownBatch,
    next_request_id: u64,
    total_shootdowns: u64,
    total_shootdown_latency_ns: u64,
    stats: HolisticTlbMgrStats,
}

impl HolisticTlbMgr {
    pub fn new() -> Self {
        Self {
            cpus: BTreeMap::new(),
            pending_batch: ShootdownBatch::new(),
            next_request_id: 1,
            total_shootdowns: 0,
            total_shootdown_latency_ns: 0,
            stats: HolisticTlbMgrStats::default(),
        }
    }

    pub fn register_cpu(&mut self, cpu_id: u32, pcid_cap: u16) {
        self.cpus.entry(cpu_id)
            .or_insert_with(|| CpuTlbState::new(cpu_id, pcid_cap));
    }

    /// Queue a shootdown request
    pub fn queue_shootdown(&mut self, cpu: u32, start: u64, end: u64, reason: TlbFlushReason, now: u64) {
        let id = self.next_request_id;
        self.next_request_id += 1;
        let req = ShootdownRequest::new(id, cpu, start, end, reason, now);
        self.pending_batch.add(req);
    }

    /// Flush the batch — coalesce and execute
    pub fn flush_batch(&mut self, now_ns: u64) -> u64 {
        self.pending_batch.coalesce();
        let pages = self.pending_batch.total_pages_4k();

        for req in &self.pending_batch.requests {
            if let Some(cpu) = self.cpus.get_mut(&req.initiator_cpu) {
                cpu.shootdowns_sent += 1;
            }
            self.total_shootdowns += 1;
        }

        self.pending_batch = ShootdownBatch::new();
        let _ = now_ns;
        pages
    }

    pub fn record_flush(&mut self, cpu_id: u32, full: bool, latency_ns: u64) {
        if let Some(cpu) = self.cpus.get_mut(&cpu_id) {
            cpu.total_flushes += 1;
            if full { cpu.full_flushes += 1; }
            else { cpu.range_flushes += 1; }
            cpu.total_flush_latency_ns += latency_ns;
            if latency_ns > cpu.max_flush_latency_ns { cpu.max_flush_latency_ns = latency_ns; }
        }
        self.total_shootdown_latency_ns += latency_ns;
    }

    pub fn recompute(&mut self) {
        self.stats.total_cpus = self.cpus.len();
        self.stats.total_flushes = self.cpus.values().map(|c| c.total_flushes).sum();
        self.stats.total_shootdowns = self.total_shootdowns;
        self.stats.avg_shootdown_latency_ns = if self.total_shootdowns > 0 {
            self.total_shootdown_latency_ns / self.total_shootdowns
        } else { 0 };
        let sum_pcid: f64 = self.cpus.values().map(|c| c.pcid_hit_rate()).sum();
        self.stats.avg_pcid_hit_rate = if self.cpus.is_empty() { 0.0 }
        else { sum_pcid / self.cpus.len() as f64 };
    }

    pub fn cpu_tlb(&self, id: u32) -> Option<&CpuTlbState> { self.cpus.get(&id) }
    pub fn stats(&self) -> &HolisticTlbMgrStats { &self.stats }
}

// ============================================================================
// Merged from tlb_mgr_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbV2Scope {
    SinglePage,
    PageRange,
    FullFlush,
    AsidFlush,
    GlobalFlush,
    KernelOnly,
    UserOnly,
}

/// TLB entry type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbV2EntryType {
    Instruction,
    Data,
    Unified,
    Large2M,
    Huge1G,
    Global,
}

/// CPU TLB state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbV2CpuState {
    Active,
    LazyMode,
    Offline,
    ShootdownPending,
    Flushing,
}

/// A batched TLB shootdown request.
#[derive(Debug, Clone)]
pub struct TlbV2ShootdownBatch {
    pub batch_id: u64,
    pub scope: TlbV2Scope,
    pub start_addr: u64,
    pub end_addr: u64,
    pub asid: u16,
    pub target_cpus: Vec<u32>,
    pub generation: u64,
    pub page_count: u64,
    pub initiated_by_cpu: u32,
}

impl TlbV2ShootdownBatch {
    pub fn new(batch_id: u64, scope: TlbV2Scope, start_addr: u64, end_addr: u64) -> Self {
        Self {
            batch_id,
            scope,
            start_addr,
            end_addr,
            asid: 0,
            target_cpus: Vec::new(),
            generation: 0,
            page_count: if end_addr > start_addr {
                (end_addr - start_addr) / 4096
            } else {
                0
            },
            initiated_by_cpu: 0,
        }
    }

    pub fn add_target(&mut self, cpu_id: u32) {
        if !self.target_cpus.contains(&cpu_id) {
            self.target_cpus.push(cpu_id);
        }
    }

    pub fn should_full_flush(&self) -> bool {
        self.page_count > 32 || self.scope == TlbV2Scope::FullFlush
    }
}

/// Per-CPU TLB tracking data.
#[derive(Debug, Clone)]
pub struct TlbV2CpuData {
    pub cpu_id: u32,
    pub state: TlbV2CpuState,
    pub current_asid: u16,
    pub generation: u64,
    pub flush_count: u64,
    pub shootdown_received: u64,
    pub lazy_switches: u64,
    pub active_asids: Vec<u16>,
    pub max_asids: u16,
    pub next_asid: u16,
}

impl TlbV2CpuData {
    pub fn new(cpu_id: u32, max_asids: u16) -> Self {
        Self {
            cpu_id,
            state: TlbV2CpuState::Active,
            current_asid: 0,
            generation: 0,
            flush_count: 0,
            shootdown_received: 0,
            lazy_switches: 0,
            active_asids: Vec::new(),
            max_asids,
            next_asid: 1,
        }
    }

    pub fn allocate_asid(&mut self) -> u16 {
        if self.next_asid >= self.max_asids {
            // Wrap around — need a full flush
            self.next_asid = 1;
            self.active_asids.clear();
            self.flush_count += 1;
            self.generation += 1;
        }
        let asid = self.next_asid;
        self.next_asid += 1;
        self.active_asids.push(asid);
        self.current_asid = asid;
        asid
    }

    pub fn enter_lazy_mode(&mut self) {
        if self.state == TlbV2CpuState::Active {
            self.state = TlbV2CpuState::LazyMode;
            self.lazy_switches += 1;
        }
    }

    pub fn exit_lazy_mode(&mut self) {
        if self.state == TlbV2CpuState::LazyMode {
            self.state = TlbV2CpuState::Active;
        }
    }

    pub fn receive_shootdown(&mut self) {
        self.shootdown_received += 1;
        if self.state == TlbV2CpuState::LazyMode {
            self.state = TlbV2CpuState::ShootdownPending;
        } else {
            self.flush_count += 1;
        }
    }
}

/// Statistics for the TLB manager V2.
#[derive(Debug, Clone)]
pub struct TlbMgrV2Stats {
    pub total_shootdowns: u64,
    pub batched_shootdowns: u64,
    pub single_page_flushes: u64,
    pub full_flushes: u64,
    pub asid_allocations: u64,
    pub asid_wraps: u64,
    pub lazy_mode_entries: u64,
    pub lazy_avoided_flushes: u64,
    pub ipis_sent: u64,
    pub ipis_avoided: u64,
}

/// Main holistic TLB manager V2.
pub struct HolisticTlbMgrV2 {
    pub cpu_data: BTreeMap<u32, TlbV2CpuData>,
    pub pending_batches: Vec<TlbV2ShootdownBatch>,
    pub global_generation: AtomicU64,
    pub next_batch_id: u64,
    pub stats: TlbMgrV2Stats,
}

impl HolisticTlbMgrV2 {
    pub fn new() -> Self {
        Self {
            cpu_data: BTreeMap::new(),
            pending_batches: Vec::new(),
            global_generation: AtomicU64::new(0),
            next_batch_id: 1,
            stats: TlbMgrV2Stats {
                total_shootdowns: 0,
                batched_shootdowns: 0,
                single_page_flushes: 0,
                full_flushes: 0,
                asid_allocations: 0,
                asid_wraps: 0,
                lazy_mode_entries: 0,
                lazy_avoided_flushes: 0,
                ipis_sent: 0,
                ipis_avoided: 0,
            },
        }
    }

    pub fn register_cpu(&mut self, cpu_id: u32, max_asids: u16) {
        let data = TlbV2CpuData::new(cpu_id, max_asids);
        self.cpu_data.insert(cpu_id, data);
    }

    pub fn create_shootdown(
        &mut self,
        scope: TlbV2Scope,
        start_addr: u64,
        end_addr: u64,
    ) -> u64 {
        let id = self.next_batch_id;
        self.next_batch_id += 1;
        let gen = self.global_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let mut batch = TlbV2ShootdownBatch::new(id, scope, start_addr, end_addr);
        batch.generation = gen;
        self.pending_batches.push(batch);
        self.stats.total_shootdowns += 1;
        id
    }

    pub fn execute_pending_shootdowns(&mut self) -> u64 {
        let batches = core::mem::take(&mut self.pending_batches);
        let mut ipis_sent = 0u64;
        for batch in &batches {
            for &cpu_id in &batch.target_cpus {
                if let Some(data) = self.cpu_data.get_mut(&cpu_id) {
                    if data.state == TlbV2CpuState::LazyMode {
                        self.stats.lazy_avoided_flushes += 1;
                        self.stats.ipis_avoided += 1;
                    } else {
                        data.receive_shootdown();
                        ipis_sent += 1;
                    }
                }
            }
            if batch.should_full_flush() {
                self.stats.full_flushes += 1;
            } else {
                self.stats.single_page_flushes += batch.page_count;
            }
        }
        self.stats.batched_shootdowns += batches.len() as u64;
        self.stats.ipis_sent += ipis_sent;
        ipis_sent
    }

    pub fn cpu_count(&self) -> usize {
        self.cpu_data.len()
    }

    pub fn pending_count(&self) -> usize {
        self.pending_batches.len()
    }
}
