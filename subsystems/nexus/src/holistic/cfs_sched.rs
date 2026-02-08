// SPDX-License-Identifier: GPL-2.0
//! Holistic cfs_sched — Completely Fair Scheduler implementation.

extern crate alloc;

use alloc::collections::BTreeMap;

/// CFS task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfsState {
    Running,
    Runnable,
    Sleeping,
    Blocked,
}

/// CFS task entity
#[derive(Debug)]
pub struct CfsEntity {
    pub pid: u64,
    pub state: CfsState,
    pub nice: i8,
    pub weight: u32,
    pub vruntime: u64,
    pub exec_start: u64,
    pub sum_exec_runtime: u64,
    pub nr_switches: u64,
    pub prev_sum_exec: u64,
    pub last_update: u64,
    pub load_avg: u64,
}

impl CfsEntity {
    pub fn new(pid: u64, nice: i8) -> Self {
        let weight = Self::nice_to_weight(nice);
        Self { pid, state: CfsState::Runnable, nice, weight, vruntime: 0, exec_start: 0, sum_exec_runtime: 0, nr_switches: 0, prev_sum_exec: 0, last_update: 0, load_avg: 0 }
    }

    fn nice_to_weight(nice: i8) -> u32 {
        let base = 1024u32;
        if nice == 0 { return base; }
        if nice > 0 { base / (1 + nice as u32) } else { base * (1 + (-nice) as u32) }
    }

    pub fn update_vruntime(&mut self, delta_exec: u64, min_granularity: u64) {
        let ideal_runtime = if self.weight == 0 { delta_exec } else { delta_exec * 1024 / self.weight as u64 };
        self.vruntime += ideal_runtime.max(min_granularity);
        self.sum_exec_runtime += delta_exec;
    }

    pub fn slice_ns(&self, total_weight: u32, period_ns: u64) -> u64 {
        if total_weight == 0 { return period_ns; }
        (period_ns * self.weight as u64) / total_weight as u64
    }
}

/// CFS run queue
#[derive(Debug)]
pub struct CfsRunQueue {
    pub cpu: u32,
    pub entities: BTreeMap<u64, CfsEntity>,
    pub min_vruntime: u64,
    pub nr_running: u32,
    pub total_weight: u32,
    pub clock: u64,
    pub period_ns: u64,
    pub min_granularity_ns: u64,
    pub nr_switches: u64,
}

impl CfsRunQueue {
    pub fn new(cpu: u32) -> Self {
        Self { cpu, entities: BTreeMap::new(), min_vruntime: 0, nr_running: 0, total_weight: 0, clock: 0, period_ns: 6_000_000, min_granularity_ns: 750_000, nr_switches: 0 }
    }

    pub fn enqueue(&mut self, entity: CfsEntity) {
        self.total_weight += entity.weight;
        self.nr_running += 1;
        self.entities.insert(entity.pid, entity);
    }

    pub fn dequeue(&mut self, pid: u64) {
        if let Some(e) = self.entities.remove(&pid) {
            self.total_weight -= e.weight;
            self.nr_running -= 1;
        }
    }

    pub fn pick_next(&self) -> Option<u64> {
        self.entities.values().filter(|e| e.state == CfsState::Runnable).min_by_key(|e| e.vruntime).map(|e| e.pid)
    }

    pub fn tick(&mut self, now: u64) {
        self.clock = now;
        self.min_vruntime = self.entities.values().map(|e| e.vruntime).min().unwrap_or(0);
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct CfsSchedStats {
    pub total_cpus: u32,
    pub total_tasks: u32,
    pub total_switches: u64,
    pub avg_load: f64,
}

/// Main CFS scheduler
pub struct HolisticCfsSched {
    run_queues: BTreeMap<u32, CfsRunQueue>,
}

impl HolisticCfsSched {
    pub fn new() -> Self { Self { run_queues: BTreeMap::new() } }
    pub fn add_cpu(&mut self, cpu: u32) { self.run_queues.insert(cpu, CfsRunQueue::new(cpu)); }

    pub fn enqueue(&mut self, cpu: u32, pid: u64, nice: i8) {
        if let Some(rq) = self.run_queues.get_mut(&cpu) { rq.enqueue(CfsEntity::new(pid, nice)); }
    }

    pub fn stats(&self) -> CfsSchedStats {
        let tasks: u32 = self.run_queues.values().map(|rq| rq.nr_running).sum();
        let switches: u64 = self.run_queues.values().map(|rq| rq.nr_switches).sum();
        let loads: Vec<f64> = self.run_queues.values().map(|rq| rq.nr_running as f64).collect();
        let avg = if loads.is_empty() { 0.0 } else { loads.iter().sum::<f64>() / loads.len() as f64 };
        CfsSchedStats { total_cpus: self.run_queues.len() as u32, total_tasks: tasks, total_switches: switches, avg_load: avg }
    }
}
