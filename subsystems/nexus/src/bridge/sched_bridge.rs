//! # Bridge Scheduler Bridge
//!
//! Integration between syscall bridge and scheduler:
//! - Syscall-aware scheduling hints
//! - Yield-on-block optimization
//! - Priority inheritance for blocked syscalls
//! - Scheduler latency tracking per syscall
//! - Preemption control around critical syscalls

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SCHEDULING TYPES
// ============================================================================

/// Syscall scheduling class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallSchedClass {
    /// Non-blocking, fast
    FastPath,
    /// Potentially blocking (I/O)
    Blocking,
    /// CPU-intensive (e.g., crypto)
    CpuIntensive,
    /// Memory-intensive (mmap, brk)
    MemoryIntensive,
    /// IPC (pipe, socket, signal)
    Ipc,
    /// Administrative
    Admin,
}

/// Scheduling hint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedHint {
    /// No special treatment
    None,
    /// Prefer not to preempt
    NoPreempt,
    /// Yield CPU after completion
    YieldAfter,
    /// Boost priority temporarily
    BoostPriority,
    /// Pin to current CPU
    PinCpu,
    /// Migrate to idle CPU
    MigrateIdle,
}

/// Priority inheritance state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityInheritance {
    /// No inheritance
    None,
    /// Inherited from waiter
    Inherited,
    /// Propagated through chain
    Propagated,
}

// ============================================================================
// SYSCALL CLASSIFICATION
// ============================================================================

/// Syscall classifier
#[derive(Debug)]
#[repr(align(64))]
pub struct SyscallClassifier {
    /// Classification map
    classifications: BTreeMap<u32, SyscallSchedClass>,
    /// Default class
    default_class: SyscallSchedClass,
}

impl SyscallClassifier {
    pub fn new() -> Self {
        Self {
            classifications: BTreeMap::new(),
            default_class: SyscallSchedClass::FastPath,
        }
    }

    /// Register classification
    #[inline(always)]
    pub fn register(&mut self, syscall_nr: u32, class: SyscallSchedClass) {
        self.classifications.insert(syscall_nr, class);
    }

    /// Classify syscall
    #[inline(always)]
    pub fn classify(&self, syscall_nr: u32) -> SyscallSchedClass {
        self.classifications.get(&syscall_nr).copied().unwrap_or(self.default_class)
    }

    /// Get scheduling hint
    #[inline]
    pub fn hint_for(&self, syscall_nr: u32) -> SchedHint {
        match self.classify(syscall_nr) {
            SyscallSchedClass::FastPath => SchedHint::NoPreempt,
            SyscallSchedClass::Blocking => SchedHint::YieldAfter,
            SyscallSchedClass::CpuIntensive => SchedHint::None,
            SyscallSchedClass::MemoryIntensive => SchedHint::PinCpu,
            SyscallSchedClass::Ipc => SchedHint::BoostPriority,
            SyscallSchedClass::Admin => SchedHint::None,
        }
    }
}

// ============================================================================
// BLOCKED SYSCALL TRACKING
// ============================================================================

/// Blocked syscall entry
#[derive(Debug, Clone)]
pub struct BlockedSyscall {
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Class
    pub class: SyscallSchedClass,
    /// Block start (ns)
    pub block_start_ns: u64,
    /// Original priority
    pub original_priority: u8,
    /// Current priority (after inheritance)
    pub current_priority: u8,
    /// Inheritance state
    pub inheritance: PriorityInheritance,
    /// Waiters (PIDs blocked on this)
    pub waiters: Vec<u64>,
}

impl BlockedSyscall {
    pub fn new(pid: u64, tid: u64, syscall_nr: u32, class: SyscallSchedClass, priority: u8, now: u64) -> Self {
        Self {
            pid,
            tid,
            syscall_nr,
            class,
            block_start_ns: now,
            original_priority: priority,
            current_priority: priority,
            inheritance: PriorityInheritance::None,
            waiters: Vec::new(),
        }
    }

    /// Add waiter (priority inheritance)
    #[inline]
    pub fn add_waiter(&mut self, waiter_pid: u64, waiter_priority: u8) {
        self.waiters.push(waiter_pid);
        if waiter_priority > self.current_priority {
            self.current_priority = waiter_priority;
            self.inheritance = PriorityInheritance::Inherited;
        }
    }

    /// Block duration
    #[inline(always)]
    pub fn block_duration(&self, now: u64) -> u64 {
        now.saturating_sub(self.block_start_ns)
    }

    /// Was priority boosted
    #[inline(always)]
    pub fn is_boosted(&self) -> bool {
        self.current_priority > self.original_priority
    }
}

// ============================================================================
// LATENCY TRACKER
// ============================================================================

/// Per-class latency tracker
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ClassLatencyTracker {
    /// Total latency (ns)
    pub total_ns: u64,
    /// Count
    pub count: u64,
    /// Min (ns)
    pub min_ns: u64,
    /// Max (ns)
    pub max_ns: u64,
    /// P50 estimate (ns)
    pub p50_ns: f64,
}

impl ClassLatencyTracker {
    pub fn new() -> Self {
        Self {
            min_ns: u64::MAX,
            ..Default::default()
        }
    }

    /// Record sample
    pub fn record(&mut self, latency_ns: u64) {
        self.count += 1;
        self.total_ns += latency_ns;
        if latency_ns < self.min_ns {
            self.min_ns = latency_ns;
        }
        if latency_ns > self.max_ns {
            self.max_ns = latency_ns;
        }
        // EMA for p50
        self.p50_ns = 0.95 * self.p50_ns + 0.05 * latency_ns as f64;
    }

    /// Average
    #[inline(always)]
    pub fn avg_ns(&self) -> u64 {
        if self.count == 0 { 0 } else { self.total_ns / self.count }
    }
}

// ============================================================================
// PREEMPTION CONTROL
// ============================================================================

/// Preemption region
#[derive(Debug, Clone)]
pub struct PreemptionRegion {
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Start timestamp
    pub start_ns: u64,
    /// Max allowed duration (ns)
    pub max_duration_ns: u64,
    /// Preemption disabled
    pub preempt_disabled: bool,
}

impl PreemptionRegion {
    pub fn new(pid: u64, tid: u64, syscall_nr: u32, now: u64, max_duration_ns: u64) -> Self {
        Self {
            pid,
            tid,
            syscall_nr,
            start_ns: now,
            max_duration_ns,
            preempt_disabled: true,
        }
    }

    /// Check if expired
    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        now.saturating_sub(self.start_ns) > self.max_duration_ns
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Scheduler bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeSchedStats {
    /// Total syscalls classified
    pub classified: u64,
    /// Currently blocked
    pub currently_blocked: usize,
    /// Priority inheritance events
    pub pi_events: u64,
    /// Preemption regions active
    pub preempt_regions: usize,
    /// Hints issued
    pub hints_issued: u64,
}

/// Bridge scheduler integration
#[repr(align(64))]
pub struct BridgeSchedBridge {
    /// Classifier
    classifier: SyscallClassifier,
    /// Blocked syscalls
    blocked: BTreeMap<u64, BlockedSyscall>,
    /// Per-class latency
    latencies: BTreeMap<u8, ClassLatencyTracker>,
    /// Active preemption regions
    regions: Vec<PreemptionRegion>,
    /// Stats
    stats: BridgeSchedStats,
}

impl BridgeSchedBridge {
    pub fn new() -> Self {
        Self {
            classifier: SyscallClassifier::new(),
            blocked: BTreeMap::new(),
            latencies: BTreeMap::new(),
            regions: Vec::new(),
            stats: BridgeSchedStats::default(),
        }
    }

    /// Register syscall classification
    #[inline(always)]
    pub fn register_class(&mut self, syscall_nr: u32, class: SyscallSchedClass) {
        self.classifier.register(syscall_nr, class);
    }

    /// Get scheduling hint for syscall
    #[inline]
    pub fn get_hint(&mut self, syscall_nr: u32) -> SchedHint {
        self.stats.classified += 1;
        self.stats.hints_issued += 1;
        self.classifier.hint_for(syscall_nr)
    }

    /// Record syscall block
    #[inline]
    pub fn record_block(&mut self, pid: u64, tid: u64, syscall_nr: u32, priority: u8, now: u64) {
        let class = self.classifier.classify(syscall_nr);
        let entry = BlockedSyscall::new(pid, tid, syscall_nr, class, priority, now);
        // Key by FNV-1a of (pid, tid)
        let key = Self::block_key(pid, tid);
        self.blocked.insert(key, entry);
        self.stats.currently_blocked = self.blocked.len();
    }

    /// Record unblock
    #[inline]
    pub fn record_unblock(&mut self, pid: u64, tid: u64, now: u64) {
        let key = Self::block_key(pid, tid);
        if let Some(entry) = self.blocked.remove(&key) {
            let latency = entry.block_duration(now);
            let class_key = entry.class as u8;
            self.latencies.entry(class_key)
                .or_insert_with(ClassLatencyTracker::new)
                .record(latency);
        }
        self.stats.currently_blocked = self.blocked.len();
    }

    /// Apply priority inheritance
    #[inline]
    pub fn apply_pi(&mut self, blocked_pid: u64, blocked_tid: u64, waiter_pid: u64, waiter_priority: u8) {
        let key = Self::block_key(blocked_pid, blocked_tid);
        if let Some(entry) = self.blocked.get_mut(&key) {
            entry.add_waiter(waiter_pid, waiter_priority);
            self.stats.pi_events += 1;
        }
    }

    /// Enter preemption-disabled region
    #[inline]
    pub fn enter_no_preempt(&mut self, pid: u64, tid: u64, syscall_nr: u32, now: u64) {
        let max_ns = match self.classifier.classify(syscall_nr) {
            SyscallSchedClass::FastPath => 10_000, // 10us
            SyscallSchedClass::CpuIntensive => 100_000, // 100us
            _ => 50_000, // 50us
        };
        self.regions.push(PreemptionRegion::new(pid, tid, syscall_nr, now, max_ns));
        self.stats.preempt_regions = self.regions.len();
    }

    /// Leave preemption-disabled region
    #[inline(always)]
    pub fn leave_no_preempt(&mut self, pid: u64, tid: u64) {
        self.regions.retain(|r| !(r.pid == pid && r.tid == tid));
        self.stats.preempt_regions = self.regions.len();
    }

    /// Expire preemption regions
    #[inline(always)]
    pub fn expire_regions(&mut self, now: u64) {
        self.regions.retain(|r| !r.is_expired(now));
        self.stats.preempt_regions = self.regions.len();
    }

    fn block_key(pid: u64, tid: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= pid;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= tid;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &BridgeSchedStats {
        &self.stats
    }
}

// ============================================================================
// Merged from sched_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedV2Policy {
    Normal,
    Fifo,
    RoundRobin,
    Batch,
    Idle,
    Deadline,
}

/// Sched v2 attributes
#[derive(Debug)]
pub struct SchedV2Attr {
    pub policy: SchedV2Policy,
    pub priority: i32,
    pub nice: i32,
    pub runtime_ns: u64,
    pub deadline_ns: u64,
    pub period_ns: u64,
    pub flags: u32,
}

impl SchedV2Attr {
    pub fn new(policy: SchedV2Policy) -> Self {
        Self { policy, priority: 0, nice: 0, runtime_ns: 0, deadline_ns: 0, period_ns: 0, flags: 0 }
    }
}

/// Process sched v2 state
#[derive(Debug)]
pub struct ProcessSchedV2 {
    pub pid: u64,
    pub attr: SchedV2Attr,
    pub vruntime: u64,
    pub total_runtime_ns: u64,
    pub nr_switches: u64,
    pub nr_migrations: u64,
    pub wait_time_ns: u64,
    pub last_cpu: u32,
}

impl ProcessSchedV2 {
    pub fn new(pid: u64, policy: SchedV2Policy) -> Self {
        Self { pid, attr: SchedV2Attr::new(policy), vruntime: 0, total_runtime_ns: 0, nr_switches: 0, nr_migrations: 0, wait_time_ns: 0, last_cpu: 0 }
    }

    #[inline]
    pub fn context_switch(&mut self, runtime_ns: u64, cpu: u32) {
        self.total_runtime_ns += runtime_ns;
        self.nr_switches += 1;
        if self.last_cpu != cpu && self.nr_switches > 1 { self.nr_migrations += 1; }
        self.last_cpu = cpu;
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SchedV2BridgeStats {
    pub total_tasks: u32,
    pub total_switches: u64,
    pub total_migrations: u64,
    pub avg_runtime_ns: u64,
}

/// Main bridge sched v2
#[repr(align(64))]
pub struct BridgeSchedV2 {
    tasks: BTreeMap<u64, ProcessSchedV2>,
}

impl BridgeSchedV2 {
    pub fn new() -> Self { Self { tasks: BTreeMap::new() } }

    #[inline(always)]
    pub fn add_task(&mut self, pid: u64, policy: SchedV2Policy) {
        self.tasks.insert(pid, ProcessSchedV2::new(pid, policy));
    }

    #[inline(always)]
    pub fn context_switch(&mut self, pid: u64, runtime: u64, cpu: u32) {
        if let Some(t) = self.tasks.get_mut(&pid) { t.context_switch(runtime, cpu); }
    }

    #[inline(always)]
    pub fn set_nice(&mut self, pid: u64, nice: i32) {
        if let Some(t) = self.tasks.get_mut(&pid) { t.attr.nice = nice; }
    }

    #[inline(always)]
    pub fn remove_task(&mut self, pid: u64) { self.tasks.remove(&pid); }

    #[inline]
    pub fn stats(&self) -> SchedV2BridgeStats {
        let switches: u64 = self.tasks.values().map(|t| t.nr_switches).sum();
        let migrations: u64 = self.tasks.values().map(|t| t.nr_migrations).sum();
        let runtime: u64 = self.tasks.values().map(|t| t.total_runtime_ns).sum();
        let avg = if switches == 0 { 0 } else { runtime / switches };
        SchedV2BridgeStats { total_tasks: self.tasks.len() as u32, total_switches: switches, total_migrations: migrations, avg_runtime_ns: avg }
    }
}

// ============================================================================
// Merged from sched_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedV3Policy {
    Normal,
    Batch,
    Idle,
    Fifo,
    RoundRobin,
    Deadline,
    Ext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedV3Flag {
    ResetOnFork,
    RecoverFromFork,
    UtilClamp,
    LatNice,
    EevdfPlacement,
}

#[derive(Debug, Clone)]
pub struct SchedV3DeadlineParams {
    pub runtime_ns: u64,
    pub deadline_ns: u64,
    pub period_ns: u64,
    pub flags: u32,
}

impl SchedV3DeadlineParams {
    #[inline(always)]
    pub fn utilization_pct(&self) -> u64 {
        if self.period_ns == 0 { 0 } else { (self.runtime_ns * 100) / self.period_ns }
    }

    #[inline(always)]
    pub fn is_feasible(&self) -> bool {
        self.runtime_ns <= self.deadline_ns && self.deadline_ns <= self.period_ns
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SchedV3EevdfState {
    pub vruntime: u64,
    pub vdeadline: u64,
    pub slice_ns: u64,
    pub lag: i64,
    pub eligible: bool,
}

impl SchedV3EevdfState {
    pub fn new() -> Self {
        Self { vruntime: 0, vdeadline: 0, slice_ns: 4_000_000, lag: 0, eligible: true }
    }

    #[inline]
    pub fn update_vruntime(&mut self, delta_ns: u64, weight: u32) {
        let weighted = if weight > 0 { (delta_ns * 1024) / weight as u64 } else { delta_ns };
        self.vruntime = self.vruntime.wrapping_add(weighted);
        self.vdeadline = self.vruntime.wrapping_add(self.slice_ns);
    }

    #[inline(always)]
    pub fn update_lag(&mut self, avg_vruntime: u64) {
        self.lag = avg_vruntime as i64 - self.vruntime as i64;
        self.eligible = self.lag >= 0;
    }
}

#[derive(Debug, Clone)]
pub struct SchedV3ExtOps {
    pub ops_name_hash: u64,
    pub dispatch_count: u64,
    pub select_cpu_count: u64,
    pub enqueue_count: u64,
    pub dequeue_count: u64,
    pub running_count: u64,
    pub stopping_count: u64,
}

impl SchedV3ExtOps {
    pub fn new(name: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in name { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            ops_name_hash: h,
            dispatch_count: 0,
            select_cpu_count: 0,
            enqueue_count: 0,
            dequeue_count: 0,
            running_count: 0,
            stopping_count: 0,
        }
    }

    #[inline(always)]
    pub fn total_ops(&self) -> u64 {
        self.dispatch_count + self.select_cpu_count + self.enqueue_count
            + self.dequeue_count + self.running_count + self.stopping_count
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SchedV3TaskState {
    pub pid: u64,
    pub policy: SchedV3Policy,
    pub priority: i32,
    pub nice: i32,
    pub weight: u32,
    pub util_clamp_min: u32,
    pub util_clamp_max: u32,
    pub lat_nice: i32,
    pub eevdf: SchedV3EevdfState,
    pub dl_params: Option<SchedV3DeadlineParams>,
    pub cpu_affinity_mask: u64,
    pub total_runtime_ns: u64,
    pub nr_switches: u64,
    pub nr_wakeups: u64,
}

impl SchedV3TaskState {
    pub fn new(pid: u64, policy: SchedV3Policy) -> Self {
        let weight = match policy {
            SchedV3Policy::Normal => 1024,
            SchedV3Policy::Batch => 512,
            SchedV3Policy::Idle => 3,
            _ => 1024,
        };
        Self {
            pid, policy, priority: 0, nice: 0, weight,
            util_clamp_min: 0, util_clamp_max: 1024, lat_nice: 0,
            eevdf: SchedV3EevdfState::new(), dl_params: None,
            cpu_affinity_mask: u64::MAX, total_runtime_ns: 0,
            nr_switches: 0, nr_wakeups: 0,
        }
    }

    #[inline]
    pub fn context_switch(&mut self, runtime_ns: u64) {
        self.total_runtime_ns += runtime_ns;
        self.nr_switches += 1;
        self.eevdf.update_vruntime(runtime_ns, self.weight);
    }

    #[inline(always)]
    pub fn wakeup(&mut self) { self.nr_wakeups += 1; }

    #[inline(always)]
    pub fn avg_timeslice_ns(&self) -> u64 {
        if self.nr_switches == 0 { 0 } else { self.total_runtime_ns / self.nr_switches }
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SchedV3BridgeStats {
    pub total_tasks: u64,
    pub total_switches: u64,
    pub dl_tasks: u64,
    pub rt_tasks: u64,
    pub ext_tasks: u64,
    pub total_migrations: u64,
    pub total_wakeups: u64,
}

#[repr(align(64))]
pub struct BridgeSchedV3 {
    tasks: BTreeMap<u64, SchedV3TaskState>,
    ext_ops: Option<SchedV3ExtOps>,
    stats: SchedV3BridgeStats,
}

impl BridgeSchedV3 {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            ext_ops: None,
            stats: SchedV3BridgeStats {
                total_tasks: 0, total_switches: 0, dl_tasks: 0,
                rt_tasks: 0, ext_tasks: 0, total_migrations: 0,
                total_wakeups: 0,
            },
        }
    }

    #[inline]
    pub fn add_task(&mut self, pid: u64, policy: SchedV3Policy) {
        let task = SchedV3TaskState::new(pid, policy);
        self.tasks.insert(pid, task);
        self.stats.total_tasks += 1;
        match policy {
            SchedV3Policy::Deadline => self.stats.dl_tasks += 1,
            SchedV3Policy::Fifo | SchedV3Policy::RoundRobin => self.stats.rt_tasks += 1,
            SchedV3Policy::Ext => self.stats.ext_tasks += 1,
            _ => {}
        }
    }

    #[inline(always)]
    pub fn load_ext_ops(&mut self, name: &[u8]) {
        self.ext_ops = Some(SchedV3ExtOps::new(name));
    }

    #[inline]
    pub fn record_switch(&mut self, pid: u64, runtime_ns: u64) {
        if let Some(t) = self.tasks.get_mut(&pid) {
            t.context_switch(runtime_ns);
            self.stats.total_switches += 1;
        }
    }

    #[inline]
    pub fn record_wakeup(&mut self, pid: u64) {
        if let Some(t) = self.tasks.get_mut(&pid) {
            t.wakeup();
            self.stats.total_wakeups += 1;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &SchedV3BridgeStats {
        &self.stats
    }
}
