// SPDX-License-Identifier: GPL-2.0
//! Holistic hotplug_mgr — CPU and memory hotplug coordination.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Hotplug resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugResource {
    Cpu,
    Memory,
    PcieDevice,
    NumaNode,
    DimmSlot,
}

/// Hotplug action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugAction {
    Add,
    Remove,
    Online,
    Offline,
    Replace,
}

/// Hotplug state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugState {
    Idle,
    Preparing,
    Notifying,
    Migrating,
    Completing,
    RollingBack,
    Done,
    Failed,
}

/// Hotplug notifier priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotifierPriority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

/// Hotplug notifier callback registration
#[derive(Debug, Clone)]
pub struct HotplugNotifier {
    pub id: u64,
    pub resource: HotplugResource,
    pub priority: NotifierPriority,
    pub registered_at: u64,
    pub invocations: u64,
    pub failures: u64,
}

impl HotplugNotifier {
    pub fn new(id: u64, resource: HotplugResource, priority: NotifierPriority, now: u64) -> Self {
        Self { id, resource, priority, registered_at: now, invocations: 0, failures: 0 }
    }

    #[inline(always)]
    pub fn invoke(&mut self) { self.invocations += 1; }
    #[inline(always)]
    pub fn fail(&mut self) { self.failures += 1; }

    #[inline(always)]
    pub fn failure_rate(&self) -> f64 {
        if self.invocations == 0 { return 0.0; }
        self.failures as f64 / self.invocations as f64
    }
}

/// Hotplug operation record
#[derive(Debug, Clone)]
pub struct HotplugOperation {
    pub id: u64,
    pub resource: HotplugResource,
    pub action: HotplugAction,
    pub resource_id: u64,
    pub state: HotplugState,
    pub notifiers_pending: u32,
    pub notifiers_complete: u32,
    pub notifiers_failed: u32,
    pub started_at: u64,
    pub completed_at: u64,
    pub rollback_reason: Option<u32>,
}

impl HotplugOperation {
    pub fn new(id: u64, resource: HotplugResource, action: HotplugAction, res_id: u64, now: u64) -> Self {
        Self {
            id, resource, action, resource_id: res_id,
            state: HotplugState::Preparing,
            notifiers_pending: 0, notifiers_complete: 0, notifiers_failed: 0,
            started_at: now, completed_at: 0, rollback_reason: None,
        }
    }

    #[inline]
    pub fn advance(&mut self) {
        self.state = match self.state {
            HotplugState::Preparing => HotplugState::Notifying,
            HotplugState::Notifying => HotplugState::Migrating,
            HotplugState::Migrating => HotplugState::Completing,
            HotplugState::Completing => HotplugState::Done,
            other => other,
        };
    }

    #[inline(always)]
    pub fn rollback(&mut self, reason: u32) {
        self.state = HotplugState::RollingBack;
        self.rollback_reason = Some(reason);
    }

    #[inline(always)]
    pub fn complete(&mut self, now: u64) {
        self.state = HotplugState::Done;
        self.completed_at = now;
    }

    #[inline(always)]
    pub fn fail(&mut self, now: u64) {
        self.state = HotplugState::Failed;
        self.completed_at = now;
    }

    #[inline(always)]
    pub fn latency_ns(&self) -> u64 {
        if self.completed_at > 0 { self.completed_at.saturating_sub(self.started_at) } else { 0 }
    }
}

/// CPU online/offline tracking
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CpuHotplugState {
    pub cpu_id: u32,
    pub online: bool,
    pub transitions: u64,
    pub last_transition: u64,
    pub tasks_migrated: u64,
    pub irqs_migrated: u64,
}

impl CpuHotplugState {
    pub fn new(cpu_id: u32, online: bool) -> Self {
        Self { cpu_id, online, transitions: 0, last_transition: 0, tasks_migrated: 0, irqs_migrated: 0 }
    }

    #[inline]
    pub fn transition(&mut self, online: bool, now: u64) {
        self.online = online;
        self.transitions += 1;
        self.last_transition = now;
    }
}

/// Hotplug manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HotplugMgrStats {
    pub total_operations: u64,
    pub successful_ops: u64,
    pub failed_ops: u64,
    pub rollback_ops: u64,
    pub active_notifiers: u32,
    pub online_cpus: u32,
    pub avg_latency_ns: u64,
}

/// Main hotplug manager
pub struct HolisticHotplugMgr {
    notifiers: BTreeMap<u64, HotplugNotifier>,
    operations: Vec<HotplugOperation>,
    cpu_states: BTreeMap<u32, CpuHotplugState>,
    next_notifier_id: u64,
    next_op_id: u64,
    max_operations: usize,
}

impl HolisticHotplugMgr {
    pub fn new() -> Self {
        Self {
            notifiers: BTreeMap::new(), operations: Vec::new(),
            cpu_states: BTreeMap::new(), next_notifier_id: 1,
            next_op_id: 1, max_operations: 4096,
        }
    }

    #[inline]
    pub fn register_notifier(&mut self, resource: HotplugResource, priority: NotifierPriority, now: u64) -> u64 {
        let id = self.next_notifier_id;
        self.next_notifier_id += 1;
        self.notifiers.insert(id, HotplugNotifier::new(id, resource, priority, now));
        id
    }

    #[inline(always)]
    pub fn unregister_notifier(&mut self, id: u64) -> bool { self.notifiers.remove(&id).is_some() }

    #[inline]
    pub fn begin_operation(&mut self, resource: HotplugResource, action: HotplugAction, res_id: u64, now: u64) -> u64 {
        let id = self.next_op_id;
        self.next_op_id += 1;
        if self.operations.len() >= self.max_operations { self.operations.drain(..self.max_operations / 4); }
        self.operations.push(HotplugOperation::new(id, resource, action, res_id, now));
        id
    }

    #[inline(always)]
    pub fn track_cpu(&mut self, cpu_id: u32, online: bool) {
        self.cpu_states.entry(cpu_id).or_insert_with(|| CpuHotplugState::new(cpu_id, online));
    }

    #[inline(always)]
    pub fn cpu_online(&mut self, cpu_id: u32, now: u64) {
        if let Some(state) = self.cpu_states.get_mut(&cpu_id) { state.transition(true, now); }
    }

    #[inline(always)]
    pub fn cpu_offline(&mut self, cpu_id: u32, now: u64) {
        if let Some(state) = self.cpu_states.get_mut(&cpu_id) { state.transition(false, now); }
    }

    pub fn stats(&self) -> HotplugMgrStats {
        let successful = self.operations.iter().filter(|o| o.state == HotplugState::Done).count() as u64;
        let failed = self.operations.iter().filter(|o| o.state == HotplugState::Failed).count() as u64;
        let rollback = self.operations.iter().filter(|o| o.state == HotplugState::RollingBack).count() as u64;
        let online = self.cpu_states.values().filter(|c| c.online).count() as u32;
        let latencies: Vec<u64> = self.operations.iter().filter(|o| o.completed_at > 0).map(|o| o.latency_ns()).collect();
        let avg_lat = if latencies.is_empty() { 0 } else { latencies.iter().sum::<u64>() / latencies.len() as u64 };
        HotplugMgrStats {
            total_operations: self.operations.len() as u64,
            successful_ops: successful, failed_ops: failed,
            rollback_ops: rollback, active_notifiers: self.notifiers.len() as u32,
            online_cpus: online, avg_latency_ns: avg_lat,
        }
    }
}
