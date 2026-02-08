//! # Coop Task Graph
//!
//! Cooperative task dependency graph execution:
//! - DAG-based task dependency modeling
//! - Topological ordering for execution
//! - Parallel execution of independent tasks
//! - Failure propagation and recovery
//! - Critical path analysis
//! - Task result caching and memoization

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphTaskStatus {
    Pending,
    Ready,
    Running,
    Complete,
    Failed,
    Cancelled,
    Skipped,
    Cached,
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GraphTaskPriority {
    Background = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

/// Failure policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailurePolicy {
    CancelDependents,
    SkipDependents,
    RetryThenCancel(u32),
    Ignore,
}

/// A task in the dependency graph
#[derive(Debug, Clone)]
pub struct GraphTask {
    pub id: u64,
    pub name_hash: u64,
    pub status: GraphTaskStatus,
    pub priority: GraphTaskPriority,
    pub deps: Vec<u64>,
    pub dependents: Vec<u64>,
    pub assigned_to: Option<u64>,
    pub result_hash: u64,
    pub cost_ns: u64,
    pub start_ts: u64,
    pub end_ts: u64,
    pub retries: u32,
    pub max_retries: u32,
    pub failure_policy: FailurePolicy,
    pub cached: bool,
}

impl GraphTask {
    pub fn new(id: u64, name_hash: u64, prio: GraphTaskPriority, policy: FailurePolicy) -> Self {
        Self {
            id, name_hash, status: GraphTaskStatus::Pending, priority: prio,
            deps: Vec::new(), dependents: Vec::new(), assigned_to: None,
            result_hash: 0, cost_ns: 0, start_ts: 0, end_ts: 0,
            retries: 0, max_retries: 3, failure_policy: policy, cached: false,
        }
    }

    pub fn add_dep(&mut self, dep: u64) { if !self.deps.contains(&dep) { self.deps.push(dep); } }
    pub fn add_dependent(&mut self, d: u64) { if !self.dependents.contains(&d) { self.dependents.push(d); } }

    pub fn start(&mut self, worker: u64, ts: u64) { self.status = GraphTaskStatus::Running; self.assigned_to = Some(worker); self.start_ts = ts; }
    pub fn complete(&mut self, result: u64, ts: u64) { self.status = GraphTaskStatus::Complete; self.result_hash = result; self.end_ts = ts; self.cost_ns = ts.saturating_sub(self.start_ts); }
    pub fn fail(&mut self, ts: u64) { self.retries += 1; if self.retries >= self.max_retries { self.status = GraphTaskStatus::Failed; } else { self.status = GraphTaskStatus::Pending; } self.end_ts = ts; }
    pub fn cancel(&mut self) { self.status = GraphTaskStatus::Cancelled; }
    pub fn skip(&mut self) { self.status = GraphTaskStatus::Skipped; }
    pub fn use_cache(&mut self, result: u64) { self.status = GraphTaskStatus::Cached; self.result_hash = result; self.cached = true; }

    pub fn is_terminal(&self) -> bool { matches!(self.status, GraphTaskStatus::Complete | GraphTaskStatus::Failed | GraphTaskStatus::Cancelled | GraphTaskStatus::Skipped | GraphTaskStatus::Cached) }
    pub fn is_success(&self) -> bool { matches!(self.status, GraphTaskStatus::Complete | GraphTaskStatus::Cached) }
}

/// Execution level (parallel batch)
#[derive(Debug, Clone)]
pub struct ExecLevel {
    pub level: u32,
    pub tasks: Vec<u64>,
}

/// Critical path segment
#[derive(Debug, Clone)]
pub struct CriticalPathSegment {
    pub task_id: u64,
    pub cost_ns: u64,
    pub cumulative_ns: u64,
}

/// Result cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub input_hash: u64,
    pub result_hash: u64,
    pub ts: u64,
    pub hits: u64,
}

/// Graph execution stats
#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    pub total_tasks: usize,
    pub complete: usize,
    pub failed: usize,
    pub running: usize,
    pub pending: usize,
    pub cached_hits: u64,
    pub critical_path_ns: u64,
    pub parallelism: f64,
    pub total_cost_ns: u64,
}

/// Cooperative task graph
pub struct CoopTaskGraph {
    tasks: BTreeMap<u64, GraphTask>,
    cache: BTreeMap<u64, CacheEntry>,
    topo_order: Vec<u64>,
    stats: GraphStats,
    next_id: u64,
}

impl CoopTaskGraph {
    pub fn new() -> Self {
        Self { tasks: BTreeMap::new(), cache: BTreeMap::new(), topo_order: Vec::new(), stats: GraphStats::default(), next_id: 1 }
    }

    pub fn add_task(&mut self, name_hash: u64, prio: GraphTaskPriority, policy: FailurePolicy) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.tasks.insert(id, GraphTask::new(id, name_hash, prio, policy));
        id
    }

    pub fn add_dep(&mut self, task: u64, dep: u64) {
        if let Some(t) = self.tasks.get_mut(&task) { t.add_dep(dep); }
        if let Some(d) = self.tasks.get_mut(&dep) { d.add_dependent(task); }
    }

    pub fn topological_sort(&mut self) -> bool {
        let mut in_deg: BTreeMap<u64, usize> = BTreeMap::new();
        for (&id, t) in &self.tasks { in_deg.insert(id, t.deps.len()); }
        let mut queue: Vec<u64> = in_deg.iter().filter(|(_, &d)| d == 0).map(|(&id, _)| id).collect();
        queue.sort_by(|a, b| {
            let pa = self.tasks.get(a).map(|t| t.priority).unwrap_or(GraphTaskPriority::Normal);
            let pb = self.tasks.get(b).map(|t| t.priority).unwrap_or(GraphTaskPriority::Normal);
            pb.cmp(&pa)
        });
        let mut order = Vec::new();
        while let Some(id) = queue.pop() {
            order.push(id);
            let dependents = self.tasks.get(&id).map(|t| t.dependents.clone()).unwrap_or_default();
            for dep in dependents {
                if let Some(d) = in_deg.get_mut(&dep) {
                    *d = d.saturating_sub(1);
                    if *d == 0 { queue.push(dep); }
                }
            }
        }
        let valid = order.len() == self.tasks.len();
        self.topo_order = order;
        valid
    }

    pub fn ready_tasks(&self) -> Vec<u64> {
        self.tasks.values()
            .filter(|t| t.status == GraphTaskStatus::Pending)
            .filter(|t| t.deps.iter().all(|d| self.tasks.get(d).map(|dt| dt.is_success()).unwrap_or(false)))
            .map(|t| t.id)
            .collect()
    }

    pub fn start_task(&mut self, id: u64, worker: u64, ts: u64) {
        // Check cache first
        if let Some(t) = self.tasks.get(&id) {
            if let Some(entry) = self.cache.get_mut(&t.name_hash) {
                entry.hits += 1;
                let result = entry.result_hash;
                if let Some(t) = self.tasks.get_mut(&id) { t.use_cache(result); }
                self.stats.cached_hits += 1;
                return;
            }
        }
        if let Some(t) = self.tasks.get_mut(&id) { t.start(worker, ts); }
    }

    pub fn complete_task(&mut self, id: u64, result: u64, ts: u64) {
        if let Some(t) = self.tasks.get_mut(&id) {
            let nh = t.name_hash;
            t.complete(result, ts);
            self.cache.insert(nh, CacheEntry { input_hash: nh, result_hash: result, ts, hits: 0 });
        }
    }

    pub fn fail_task(&mut self, id: u64, ts: u64) {
        let policy = self.tasks.get(&id).map(|t| t.failure_policy);
        if let Some(t) = self.tasks.get_mut(&id) { t.fail(ts); }
        if let Some(t) = self.tasks.get(&id) {
            if t.status == GraphTaskStatus::Failed {
                let dependents = t.dependents.clone();
                match policy {
                    Some(FailurePolicy::CancelDependents) | Some(FailurePolicy::RetryThenCancel(_)) => {
                        for d in dependents { self.cancel_subtree(d); }
                    }
                    Some(FailurePolicy::SkipDependents) => {
                        for d in dependents { if let Some(dt) = self.tasks.get_mut(&d) { dt.skip(); } }
                    }
                    _ => {}
                }
            }
        }
    }

    fn cancel_subtree(&mut self, id: u64) {
        if let Some(t) = self.tasks.get_mut(&id) {
            if !t.is_terminal() { t.cancel(); }
            let deps = t.dependents.clone();
            for d in deps { self.cancel_subtree(d); }
        }
    }

    pub fn critical_path(&self) -> Vec<CriticalPathSegment> {
        let mut longest: BTreeMap<u64, u64> = BTreeMap::new();
        let mut prev: BTreeMap<u64, Option<u64>> = BTreeMap::new();
        for &id in &self.topo_order {
            let cost = self.tasks.get(&id).map(|t| t.cost_ns).unwrap_or(0);
            let max_dep = self.tasks.get(&id).map(|t| {
                t.deps.iter().map(|d| longest.get(d).copied().unwrap_or(0)).max().unwrap_or(0)
            }).unwrap_or(0);
            let max_dep_id = self.tasks.get(&id).and_then(|t| {
                t.deps.iter().max_by_key(|d| longest.get(d).copied().unwrap_or(0)).copied()
            });
            longest.insert(id, max_dep + cost);
            prev.insert(id, max_dep_id);
        }
        let end = longest.iter().max_by_key(|(_, &v)| v).map(|(&k, _)| k);
        let mut path = Vec::new();
        let mut cur = end;
        while let Some(id) = cur {
            let cost = self.tasks.get(&id).map(|t| t.cost_ns).unwrap_or(0);
            let cum = longest.get(&id).copied().unwrap_or(0);
            path.push(CriticalPathSegment { task_id: id, cost_ns: cost, cumulative_ns: cum });
            cur = prev.get(&id).copied().flatten();
        }
        path.reverse();
        path
    }

    pub fn execution_levels(&self) -> Vec<ExecLevel> {
        let mut levels: BTreeMap<u32, Vec<u64>> = BTreeMap::new();
        let mut depth: BTreeMap<u64, u32> = BTreeMap::new();
        for &id in &self.topo_order {
            let d = self.tasks.get(&id).map(|t| {
                t.deps.iter().map(|dep| depth.get(dep).copied().unwrap_or(0) + 1).max().unwrap_or(0)
            }).unwrap_or(0);
            depth.insert(id, d);
            levels.entry(d).or_insert_with(Vec::new).push(id);
        }
        levels.into_iter().map(|(level, tasks)| ExecLevel { level, tasks }).collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_tasks = self.tasks.len();
        self.stats.complete = self.tasks.values().filter(|t| t.is_success()).count();
        self.stats.failed = self.tasks.values().filter(|t| t.status == GraphTaskStatus::Failed).count();
        self.stats.running = self.tasks.values().filter(|t| t.status == GraphTaskStatus::Running).count();
        self.stats.pending = self.tasks.values().filter(|t| t.status == GraphTaskStatus::Pending).count();
        self.stats.total_cost_ns = self.tasks.values().map(|t| t.cost_ns).sum();
        let cp = self.critical_path();
        self.stats.critical_path_ns = cp.last().map(|s| s.cumulative_ns).unwrap_or(0);
        if self.stats.critical_path_ns > 0 {
            self.stats.parallelism = self.stats.total_cost_ns as f64 / self.stats.critical_path_ns as f64;
        }
    }

    pub fn task(&self, id: u64) -> Option<&GraphTask> { self.tasks.get(&id) }
    pub fn stats(&self) -> &GraphStats { &self.stats }
    pub fn topo_order(&self) -> &[u64] { &self.topo_order }
}
