// SPDX-License-Identifier: GPL-2.0
//! Bridge sched_ext — extensible scheduler (sched_ext/BPF) bridge for pluggable scheduling policies.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Scheduler extension operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedExtOp {
    /// Select a CPU for a waking task
    SelectCpu,
    /// Enqueue a task
    Enqueue,
    /// Dequeue a task
    Dequeue,
    /// Dispatch tasks from BPF scheduler
    Dispatch,
    /// Running callback (task starts running)
    Running,
    /// Stopping callback (task stops running)
    Stopping,
    /// Quiescent callback (task no longer runnable)
    Quiescent,
    /// CPU acquire (CPU becomes available)
    CpuAcquire,
    /// CPU release (CPU becomes unavailable)
    CpuRelease,
    /// Init task
    InitTask,
    /// Exit task
    ExitTask,
}

/// Dispatch flags for sched_ext tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DispatchFlags(pub u64);

impl DispatchFlags {
    pub const SCX_DSQ_LOCAL: Self = Self(1 << 0);
    pub const SCX_DSQ_GLOBAL: Self = Self(1 << 1);
    pub const SCX_DSQ_LOCAL_ON: Self = Self(1 << 2);
    pub const ENQUEUE_WAKEUP: Self = Self(1 << 8);
    pub const ENQUEUE_HEAD: Self = Self(1 << 9);

    #[inline(always)]
    pub fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    #[inline(always)]
    pub fn combine(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Per-task sched_ext state
#[derive(Debug)]
#[repr(align(64))]
pub struct ScxTaskState {
    pub pid: u64,
    pub dsq_id: u64,
    pub weight: u32,
    pub slice_ns: u64,
    pub sticky_cpu: i32,
    pub enqueue_count: u64,
    pub dispatch_count: u64,
    pub total_runtime_ns: u64,
    pub total_wait_ns: u64,
    last_enqueue_ns: u64,
    last_dispatch_ns: u64,
}

impl ScxTaskState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            dsq_id: 0,
            weight: 100,
            slice_ns: 20_000_000, // 20ms default
            sticky_cpu: -1,
            enqueue_count: 0,
            dispatch_count: 0,
            total_runtime_ns: 0,
            total_wait_ns: 0,
            last_enqueue_ns: 0,
            last_dispatch_ns: 0,
        }
    }

    #[inline(always)]
    pub fn record_enqueue(&mut self, timestamp_ns: u64) {
        self.enqueue_count += 1;
        self.last_enqueue_ns = timestamp_ns;
    }

    #[inline]
    pub fn record_dispatch(&mut self, timestamp_ns: u64) {
        self.dispatch_count += 1;
        if self.last_enqueue_ns > 0 {
            self.total_wait_ns += timestamp_ns.saturating_sub(self.last_enqueue_ns);
        }
        self.last_dispatch_ns = timestamp_ns;
    }

    #[inline]
    pub fn record_stop(&mut self, timestamp_ns: u64) {
        if self.last_dispatch_ns > 0 {
            self.total_runtime_ns += timestamp_ns.saturating_sub(self.last_dispatch_ns);
        }
    }

    #[inline(always)]
    pub fn avg_wait_ns(&self) -> u64 {
        if self.dispatch_count == 0 { 0 } else { self.total_wait_ns / self.dispatch_count }
    }

    #[inline(always)]
    pub fn avg_runtime_ns(&self) -> u64 {
        if self.dispatch_count == 0 { 0 } else { self.total_runtime_ns / self.dispatch_count }
    }
}

/// Dispatch queue (DSQ)
#[derive(Debug)]
#[repr(align(64))]
pub struct DispatchQueue {
    pub id: u64,
    pub name: String,
    /// O(1) deque — was Vec (O(n) remove(0)), now VecDeque (O(1) pop_front)
    pub tasks: VecDeque<u64>,
    pub max_tasks: usize,
    pub fifo: bool,
    total_dispatched: u64,
    total_consumed: u64,
    max_depth_seen: usize,
}

impl DispatchQueue {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            tasks: VecDeque::new(),
            max_tasks: 4096,
            fifo: true,
            total_dispatched: 0,
            total_consumed: 0,
            max_depth_seen: 0,
        }
    }

    /// Enqueue a task. O(1) with VecDeque.
    pub fn enqueue(&mut self, pid: u64, head: bool) -> bool {
        if self.tasks.len() >= self.max_tasks {
            return false;
        }
        if head {
            self.tasks.push_front(pid); // O(1) — was insert(0) O(n)
        } else {
            self.tasks.push_back(pid); // O(1)
        }
        self.total_dispatched += 1;
        if self.tasks.len() > self.max_depth_seen {
            self.max_depth_seen = self.tasks.len();
        }
        true
    }

    /// Consume next task. O(1) — was O(n) with Vec::remove(0).
    #[inline]
    pub fn consume(&mut self) -> Option<u64> {
        if self.tasks.is_empty() {
            return None;
        }
        self.total_consumed += 1;
        self.tasks.pop_front() // O(1) — was remove(0) O(4096)!
    }

    #[inline(always)]
    pub fn depth(&self) -> usize {
        self.tasks.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    #[inline(always)]
    pub fn throughput_ratio(&self) -> f64 {
        if self.total_dispatched == 0 { return 0.0; }
        self.total_consumed as f64 / self.total_dispatched as f64
    }
}

/// Per-CPU sched_ext state
#[derive(Debug)]
#[repr(align(64))]
pub struct CpuScxState {
    pub cpu_id: u32,
    pub local_dsq_id: u64,
    pub current_task: Option<u64>,
    pub idle: bool,
    pub dispatch_count: u64,
    pub preempt_count: u64,
}

impl CpuScxState {
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            local_dsq_id: cpu_id as u64,
            current_task: None,
            idle: true,
            dispatch_count: 0,
            preempt_count: 0,
        }
    }
}

/// Loaded sched_ext scheduler info
#[derive(Debug)]
#[repr(align(64))]
pub struct ScxSchedulerInfo {
    pub name: String,
    pub bpf_prog_id: u64,
    pub enabled: bool,
    pub switch_all: bool,
    pub nr_tasks: u64,
    pub load_timestamp_ns: u64,
    pub ops_registered: Vec<SchedExtOp>,
}

/// Sched_ext bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SchedExtBridgeStats {
    pub tasks_managed: u64,
    pub total_enqueues: u64,
    pub total_dispatches: u64,
    pub dsq_count: u64,
    pub cpu_migrations: u64,
    pub scheduler_switches: u64,
    pub op_errors: u64,
}

/// Main sched_ext bridge manager
#[repr(align(64))]
pub struct BridgeSchedExt {
    tasks: BTreeMap<u64, ScxTaskState>,
    dsqs: BTreeMap<u64, DispatchQueue>,
    cpu_states: BTreeMap<u32, CpuScxState>,
    scheduler: Option<ScxSchedulerInfo>,
    global_dsq_id: u64,
    stats: SchedExtBridgeStats,
}

impl BridgeSchedExt {
    pub fn new() -> Self {
        let mut dsqs = BTreeMap::new();
        // Create global DSQ
        dsqs.insert(u64::MAX, DispatchQueue::new(u64::MAX, String::from("global")));
        Self {
            tasks: BTreeMap::new(),
            dsqs,
            cpu_states: BTreeMap::new(),
            scheduler: None,
            global_dsq_id: u64::MAX,
            stats: SchedExtBridgeStats {
                tasks_managed: 0,
                total_enqueues: 0,
                total_dispatches: 0,
                dsq_count: 1,
                cpu_migrations: 0,
                scheduler_switches: 0,
                op_errors: 0,
            },
        }
    }

    pub fn load_scheduler(
        &mut self,
        name: String,
        bpf_prog_id: u64,
        switch_all: bool,
        ops: Vec<SchedExtOp>,
    ) -> bool {
        if self.scheduler.is_some() {
            return false;
        }
        self.scheduler = Some(ScxSchedulerInfo {
            name,
            bpf_prog_id,
            enabled: true,
            switch_all,
            nr_tasks: 0,
            load_timestamp_ns: 0,
            ops_registered: ops,
        });
        self.stats.scheduler_switches += 1;
        true
    }

    #[inline]
    pub fn unload_scheduler(&mut self) -> bool {
        if self.scheduler.take().is_some() {
            self.stats.scheduler_switches += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn init_cpu(&mut self, cpu_id: u32) {
        let cpu = CpuScxState::new(cpu_id);
        let local_dsq = DispatchQueue::new(cpu_id as u64, alloc::format!("cpu{}", cpu_id));
        self.dsqs.insert(cpu_id as u64, local_dsq);
        self.cpu_states.insert(cpu_id, cpu);
        self.stats.dsq_count += 1;
    }

    #[inline]
    pub fn init_task(&mut self, pid: u64) {
        self.tasks.insert(pid, ScxTaskState::new(pid));
        self.stats.tasks_managed += 1;
        if let Some(ref mut sched) = self.scheduler {
            sched.nr_tasks += 1;
        }
    }

    pub fn exit_task(&mut self, pid: u64) -> bool {
        if self.tasks.remove(&pid).is_some() {
            if let Some(ref mut sched) = self.scheduler {
                sched.nr_tasks = sched.nr_tasks.saturating_sub(1);
            }
            // Remove from all DSQs
            for dsq in self.dsqs.values_mut() {
                dsq.tasks.retain(|&t| t != pid);
            }
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn create_dsq(&mut self, id: u64, name: String) -> bool {
        if self.dsqs.contains_key(&id) {
            return false;
        }
        self.dsqs.insert(id, DispatchQueue::new(id, name));
        self.stats.dsq_count += 1;
        true
    }

    pub fn scx_enqueue(&mut self, pid: u64, dsq_id: u64, flags: DispatchFlags, now_ns: u64) -> bool {
        if let Some(task) = self.tasks.get_mut(&pid) {
            task.dsq_id = dsq_id;
            task.record_enqueue(now_ns);
        } else {
            return false;
        }
        let head = flags.has(DispatchFlags::ENQUEUE_HEAD);
        if let Some(dsq) = self.dsqs.get_mut(&dsq_id) {
            if dsq.enqueue(pid, head) {
                self.stats.total_enqueues += 1;
                return true;
            }
        }
        self.stats.op_errors += 1;
        false
    }

    pub fn scx_dispatch(&mut self, cpu_id: u32, now_ns: u64) -> Option<u64> {
        let local_dsq_id = cpu_id as u64;
        // Try local DSQ first
        if let Some(dsq) = self.dsqs.get_mut(&local_dsq_id) {
            if let Some(pid) = dsq.consume() {
                if let Some(task) = self.tasks.get_mut(&pid) {
                    task.record_dispatch(now_ns);
                }
                if let Some(cpu) = self.cpu_states.get_mut(&cpu_id) {
                    cpu.current_task = Some(pid);
                    cpu.idle = false;
                    cpu.dispatch_count += 1;
                }
                self.stats.total_dispatches += 1;
                return Some(pid);
            }
        }
        // Fall back to global DSQ
        if let Some(global) = self.dsqs.get_mut(&self.global_dsq_id) {
            if let Some(pid) = global.consume() {
                if let Some(task) = self.tasks.get_mut(&pid) {
                    task.record_dispatch(now_ns);
                }
                if let Some(cpu) = self.cpu_states.get_mut(&cpu_id) {
                    cpu.current_task = Some(pid);
                    cpu.idle = false;
                    cpu.dispatch_count += 1;
                }
                self.stats.total_dispatches += 1;
                return Some(pid);
            }
        }
        // CPU is idle
        if let Some(cpu) = self.cpu_states.get_mut(&cpu_id) {
            cpu.idle = true;
            cpu.current_task = None;
        }
        None
    }

    #[inline]
    pub fn scx_stopping(&mut self, cpu_id: u32, now_ns: u64) {
        if let Some(cpu) = self.cpu_states.get_mut(&cpu_id) {
            if let Some(pid) = cpu.current_task.take() {
                if let Some(task) = self.tasks.get_mut(&pid) {
                    task.record_stop(now_ns);
                }
            }
        }
    }

    #[inline]
    pub fn set_task_weight(&mut self, pid: u64, weight: u32) -> bool {
        if let Some(task) = self.tasks.get_mut(&pid) {
            task.weight = weight;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn set_task_slice(&mut self, pid: u64, slice_ns: u64) -> bool {
        if let Some(task) = self.tasks.get_mut(&pid) {
            task.slice_ns = slice_ns;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn dsq_depths(&self) -> Vec<(u64, usize)> {
        self.dsqs.iter().map(|(id, dsq)| (*id, dsq.depth())).collect()
    }

    #[inline(always)]
    pub fn idle_cpus(&self) -> Vec<u32> {
        self.cpu_states.iter().filter(|(_, c)| c.idle).map(|(id, _)| *id).collect()
    }

    #[inline(always)]
    pub fn stats(&self) -> &SchedExtBridgeStats {
        &self.stats
    }
}
