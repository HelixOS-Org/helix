// SPDX-License-Identifier: GPL-2.0
//! Holistic irq_thread — threaded IRQ handler management.

extern crate alloc;

use alloc::collections::BTreeMap;

/// IRQ thread state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqThreadState {
    Idle,
    Running,
    Waiting,
    Disabled,
}

/// IRQ thread action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqActionType {
    HardIrq,
    ThreadedIrq,
    Tasklet,
    WorkQueue,
}

/// IRQ thread
#[derive(Debug)]
pub struct IrqThread {
    pub irq_num: u32,
    pub cpu: u32,
    pub state: IrqThreadState,
    pub action_type: IrqActionType,
    pub handler_hash: u64,
    pub thread_fn_hash: u64,
    pub total_handled: u64,
    pub total_wake: u64,
    pub total_ns: u64,
    pub max_latency_ns: u64,
    pub affinity_mask: u64,
}

impl IrqThread {
    pub fn new(irq: u32, cpu: u32, action: IrqActionType) -> Self {
        Self { irq_num: irq, cpu, state: IrqThreadState::Idle, action_type: action, handler_hash: 0, thread_fn_hash: 0, total_handled: 0, total_wake: 0, total_ns: 0, max_latency_ns: 0, affinity_mask: 1u64 << cpu }
    }

    #[inline]
    pub fn handle(&mut self, latency_ns: u64) {
        self.total_handled += 1;
        self.total_ns += latency_ns;
        if latency_ns > self.max_latency_ns { self.max_latency_ns = latency_ns; }
        self.state = IrqThreadState::Running;
    }

    #[inline(always)]
    pub fn wake(&mut self) {
        self.total_wake += 1;
        self.state = IrqThreadState::Waiting;
    }

    #[inline(always)]
    pub fn complete(&mut self) { self.state = IrqThreadState::Idle; }

    #[inline(always)]
    pub fn avg_latency_ns(&self) -> u64 {
        if self.total_handled == 0 { 0 } else { self.total_ns / self.total_handled }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IrqThreadStats {
    pub total_threads: u32,
    pub active: u32,
    pub total_handled: u64,
    pub avg_latency_ns: u64,
    pub max_latency_ns: u64,
}

/// Main holistic IRQ thread manager
pub struct HolisticIrqThread {
    threads: BTreeMap<u32, IrqThread>,
}

impl HolisticIrqThread {
    pub fn new() -> Self { Self { threads: BTreeMap::new() } }

    #[inline(always)]
    pub fn register(&mut self, irq: u32, cpu: u32, action: IrqActionType) {
        self.threads.insert(irq, IrqThread::new(irq, cpu, action));
    }

    #[inline(always)]
    pub fn handle(&mut self, irq: u32, latency_ns: u64) {
        if let Some(t) = self.threads.get_mut(&irq) { t.handle(latency_ns); }
    }

    #[inline(always)]
    pub fn complete(&mut self, irq: u32) {
        if let Some(t) = self.threads.get_mut(&irq) { t.complete(); }
    }

    #[inline(always)]
    pub fn unregister(&mut self, irq: u32) { self.threads.remove(&irq); }

    #[inline]
    pub fn stats(&self) -> IrqThreadStats {
        let active = self.threads.values().filter(|t| t.state == IrqThreadState::Running).count() as u32;
        let handled: u64 = self.threads.values().map(|t| t.total_handled).sum();
        let ns: u64 = self.threads.values().map(|t| t.total_ns).sum();
        let max: u64 = self.threads.values().map(|t| t.max_latency_ns).max().unwrap_or(0);
        let avg = if handled == 0 { 0 } else { ns / handled };
        IrqThreadStats { total_threads: self.threads.len() as u32, active, total_handled: handled, avg_latency_ns: avg, max_latency_ns: max }
    }
}
