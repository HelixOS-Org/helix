// SPDX-License-Identifier: GPL-2.0
//! Apps kthread_mgr — kernel thread lifecycle manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Kernel thread type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KthreadType {
    Worker,
    Daemon,
    Softirq,
    Kcompactd,
    Ksoftirqd,
    Migration,
    RcuGp,
    RcuCb,
    Watchdog,
    Custom,
}

/// Kthread state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KthreadState {
    Created,
    Running,
    Sleeping,
    Parked,
    Stopped,
    Exited,
}

/// Kthread flags
#[derive(Debug, Clone, Copy)]
pub struct KthreadFlags {
    pub bits: u32,
}

impl KthreadFlags {
    pub const SHOULD_STOP: u32 = 1;
    pub const SHOULD_PARK: u32 = 2;
    pub const PERCPU: u32 = 4;
    pub const FREEZABLE: u32 = 8;
    pub const NICE: u32 = 16;
    pub const NOFREEZE: u32 = 32;

    pub fn new(bits: u32) -> Self { Self { bits } }
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
    pub fn should_stop(&self) -> bool { self.has(Self::SHOULD_STOP) }
    pub fn should_park(&self) -> bool { self.has(Self::SHOULD_PARK) }
}

/// Kernel thread descriptor
#[derive(Debug)]
pub struct KthreadInfo {
    pub tid: u64,
    pub name: String,
    pub ktype: KthreadType,
    pub state: KthreadState,
    pub flags: KthreadFlags,
    pub cpu_affinity: Option<u32>,
    pub priority: i32,
    pub nice: i32,
    pub wakeups: u64,
    pub runtime_ns: u64,
    pub sleeptime_ns: u64,
    pub created_at: u64,
    pub last_run: u64,
}

impl KthreadInfo {
    pub fn new(tid: u64, name: String, ktype: KthreadType, now: u64) -> Self {
        Self {
            tid, name, ktype, state: KthreadState::Created,
            flags: KthreadFlags::new(0), cpu_affinity: None,
            priority: 0, nice: 0, wakeups: 0,
            runtime_ns: 0, sleeptime_ns: 0,
            created_at: now, last_run: 0,
        }
    }

    pub fn run(&mut self, now: u64) {
        self.state = KthreadState::Running;
        self.wakeups += 1;
        self.last_run = now;
    }

    pub fn sleep(&mut self, duration: u64) {
        self.state = KthreadState::Sleeping;
        self.sleeptime_ns += duration;
    }

    pub fn park(&mut self) { self.state = KthreadState::Parked; }
    pub fn stop(&mut self) { self.state = KthreadState::Stopped; }
    pub fn exit(&mut self) { self.state = KthreadState::Exited; }

    pub fn utilization(&self, now: u64) -> f64 {
        let total = now.saturating_sub(self.created_at);
        if total == 0 { return 0.0; }
        self.runtime_ns as f64 / total as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct KthreadMgrStats {
    pub total_kthreads: u32,
    pub running: u32,
    pub sleeping: u32,
    pub parked: u32,
    pub exited: u32,
    pub total_wakeups: u64,
    pub total_runtime_ns: u64,
}

/// Main kthread manager
pub struct AppKthreadMgr {
    threads: BTreeMap<u64, KthreadInfo>,
    next_tid: u64,
}

impl AppKthreadMgr {
    pub fn new() -> Self { Self { threads: BTreeMap::new(), next_tid: 1 } }

    pub fn create(&mut self, name: String, ktype: KthreadType, now: u64) -> u64 {
        let tid = self.next_tid;
        self.next_tid += 1;
        self.threads.insert(tid, KthreadInfo::new(tid, name, ktype, now));
        tid
    }

    pub fn wake(&mut self, tid: u64, now: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.run(now); }
    }

    pub fn park(&mut self, tid: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.park(); }
    }

    pub fn stop(&mut self, tid: u64) {
        if let Some(t) = self.threads.get_mut(&tid) { t.stop(); }
    }

    pub fn stats(&self) -> KthreadMgrStats {
        let running = self.threads.values().filter(|t| t.state == KthreadState::Running).count() as u32;
        let sleeping = self.threads.values().filter(|t| t.state == KthreadState::Sleeping).count() as u32;
        let parked = self.threads.values().filter(|t| t.state == KthreadState::Parked).count() as u32;
        let exited = self.threads.values().filter(|t| t.state == KthreadState::Exited).count() as u32;
        let wakeups: u64 = self.threads.values().map(|t| t.wakeups).sum();
        let runtime: u64 = self.threads.values().map(|t| t.runtime_ns).sum();
        KthreadMgrStats {
            total_kthreads: self.threads.len() as u32, running, sleeping, parked,
            exited, total_wakeups: wakeups, total_runtime_ns: runtime,
        }
    }
}
