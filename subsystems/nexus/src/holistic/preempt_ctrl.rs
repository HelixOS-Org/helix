//! # Holistic Preemption Control
//!
//! Kernel preemption control and scheduling latency management:
//! - Preemption disable/enable tracking
//! - Critical section duration monitoring
//! - Preempt count nesting depth
//! - Voluntary vs involuntary preemption stats
//! - Real-time preemption latency budgets
//! - Preemption point injection analysis

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Preemption model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreemptModel {
    /// No preemption (server workloads)
    None,
    /// Voluntary preemption points only
    Voluntary,
    /// Full kernel preemption
    Full,
    /// Real-time preemption (PREEMPT_RT)
    RealTime,
}

/// Preemption disable reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisableReason {
    SpinLock,
    IrqDisable,
    SoftIrqProcess,
    RcuReadLock,
    Migration,
    SchedulerLock,
    BhDisable,
    Explicit,
}

/// Preempt count entry
#[derive(Debug, Clone)]
pub struct PreemptDisableEntry {
    pub reason: DisableReason,
    pub nesting: u32,
    pub start_ts: u64,
    pub site: u64,
}

/// Per-CPU preemption state
#[derive(Debug, Clone)]
pub struct CpuPreemptState {
    pub cpu_id: u32,
    pub preempt_count: u32,
    pub disable_stack: Vec<PreemptDisableEntry>,
    pub voluntary_preempts: u64,
    pub involuntary_preempts: u64,
    pub total_disable_ns: u64,
    pub max_disable_ns: u64,
    pub current_disable_start: Option<u64>,
    pub need_resched: bool,
    pub need_resched_lazy: bool,
}

impl CpuPreemptState {
    pub fn new(cpu: u32) -> Self {
        Self {
            cpu_id: cpu, preempt_count: 0, disable_stack: Vec::new(),
            voluntary_preempts: 0, involuntary_preempts: 0,
            total_disable_ns: 0, max_disable_ns: 0,
            current_disable_start: None, need_resched: false,
            need_resched_lazy: false,
        }
    }

    pub fn disable(&mut self, reason: DisableReason, site: u64, ts: u64) {
        if self.preempt_count == 0 { self.current_disable_start = Some(ts); }
        self.preempt_count += 1;
        self.disable_stack.push(PreemptDisableEntry {
            reason, nesting: self.preempt_count, start_ts: ts, site,
        });
    }

    pub fn enable(&mut self, ts: u64) {
        if self.preempt_count == 0 { return; }
        self.preempt_count -= 1;
        self.disable_stack.pop();
        if self.preempt_count == 0 {
            if let Some(start) = self.current_disable_start.take() {
                let duration = ts.saturating_sub(start);
                self.total_disable_ns += duration;
                if duration > self.max_disable_ns { self.max_disable_ns = duration; }
            }
        }
    }

    pub fn is_preemptible(&self) -> bool { self.preempt_count == 0 }

    pub fn current_nesting(&self) -> u32 { self.preempt_count }

    pub fn record_voluntary(&mut self) { self.voluntary_preempts += 1; }
    pub fn record_involuntary(&mut self) { self.involuntary_preempts += 1; }

    pub fn voluntary_ratio(&self) -> f64 {
        let total = self.voluntary_preempts + self.involuntary_preempts;
        if total == 0 { return 0.0; }
        self.voluntary_preempts as f64 / total as f64
    }
}

/// Critical section tracker
#[derive(Debug, Clone)]
pub struct CriticalSection {
    pub section_id: u64,
    pub cpu_id: u32,
    pub reason: DisableReason,
    pub start_ts: u64,
    pub end_ts: Option<u64>,
    pub duration_ns: u64,
    pub site: u64,
    pub depth: u32,
}

/// Latency budget for real-time tasks
#[derive(Debug, Clone)]
pub struct LatencyBudget {
    pub task_id: u64,
    pub max_preempt_latency_ns: u64,
    pub max_irq_latency_ns: u64,
    pub violations: u64,
    pub worst_observed_ns: u64,
}

impl LatencyBudget {
    pub fn new(task: u64, preempt_ns: u64, irq_ns: u64) -> Self {
        Self { task_id: task, max_preempt_latency_ns: preempt_ns, max_irq_latency_ns: irq_ns, violations: 0, worst_observed_ns: 0 }
    }

    pub fn check(&mut self, observed_ns: u64) -> bool {
        if observed_ns > self.worst_observed_ns { self.worst_observed_ns = observed_ns; }
        if observed_ns > self.max_preempt_latency_ns { self.violations += 1; return false; }
        true
    }
}

/// Preemption hotspot
#[derive(Debug, Clone)]
pub struct PreemptHotspot {
    pub site: u64,
    pub reason: DisableReason,
    pub occurrences: u64,
    pub total_duration_ns: u64,
    pub max_duration_ns: u64,
}

impl PreemptHotspot {
    pub fn avg_duration_ns(&self) -> f64 {
        if self.occurrences == 0 { return 0.0; }
        self.total_duration_ns as f64 / self.occurrences as f64
    }
}

/// Preempt control stats
#[derive(Debug, Clone, Default)]
pub struct PreemptCtrlStats {
    pub total_cpus: usize,
    pub preempt_model: u8,
    pub total_voluntary: u64,
    pub total_involuntary: u64,
    pub global_max_disable_ns: u64,
    pub avg_disable_ns: f64,
    pub active_budgets: usize,
    pub budget_violations: u64,
    pub hotspot_count: usize,
    pub currently_disabled_cpus: usize,
}

/// Holistic preemption control manager
pub struct HolisticPreemptCtrl {
    cpus: BTreeMap<u32, CpuPreemptState>,
    sections: Vec<CriticalSection>,
    budgets: BTreeMap<u64, LatencyBudget>,
    hotspots: BTreeMap<u64, PreemptHotspot>,
    model: PreemptModel,
    next_section_id: u64,
    max_sections: usize,
    stats: PreemptCtrlStats,
}

impl HolisticPreemptCtrl {
    pub fn new(model: PreemptModel) -> Self {
        Self {
            cpus: BTreeMap::new(), sections: Vec::new(),
            budgets: BTreeMap::new(), hotspots: BTreeMap::new(),
            model, next_section_id: 1, max_sections: 10_000,
            stats: PreemptCtrlStats::default(),
        }
    }

    pub fn init_cpu(&mut self, cpu: u32) { self.cpus.insert(cpu, CpuPreemptState::new(cpu)); }

    pub fn preempt_disable(&mut self, cpu: u32, reason: DisableReason, site: u64, ts: u64) {
        if let Some(state) = self.cpus.get_mut(&cpu) {
            state.disable(reason, site, ts);
            let sid = self.next_section_id; self.next_section_id += 1;
            self.sections.push(CriticalSection {
                section_id: sid, cpu_id: cpu, reason, start_ts: ts,
                end_ts: None, duration_ns: 0, site, depth: state.preempt_count,
            });
            if self.sections.len() > self.max_sections { self.sections.remove(0); }
        }
    }

    pub fn preempt_enable(&mut self, cpu: u32, ts: u64) {
        if let Some(state) = self.cpus.get_mut(&cpu) {
            let site = state.disable_stack.last().map(|e| e.site).unwrap_or(0);
            let reason = state.disable_stack.last().map(|e| e.reason).unwrap_or(DisableReason::Explicit);
            let start = state.current_disable_start.unwrap_or(ts);
            let duration = ts.saturating_sub(start);
            state.enable(ts);

            // Update hotspot
            let hs = self.hotspots.entry(site).or_insert_with(|| PreemptHotspot {
                site, reason, occurrences: 0, total_duration_ns: 0, max_duration_ns: 0,
            });
            hs.occurrences += 1;
            hs.total_duration_ns += duration;
            if duration > hs.max_duration_ns { hs.max_duration_ns = duration; }

            // Check budgets
            for budget in self.budgets.values_mut() { budget.check(duration); }
        }
    }

    pub fn set_need_resched(&mut self, cpu: u32) {
        if let Some(state) = self.cpus.get_mut(&cpu) { state.need_resched = true; }
    }

    pub fn add_budget(&mut self, budget: LatencyBudget) {
        self.budgets.insert(budget.task_id, budget);
    }

    pub fn top_hotspots(&self, n: usize) -> Vec<&PreemptHotspot> {
        let mut sorted: Vec<&PreemptHotspot> = self.hotspots.values().collect();
        sorted.sort_by(|a, b| b.max_duration_ns.cmp(&a.max_duration_ns));
        sorted.truncate(n);
        sorted
    }

    pub fn recompute(&mut self) {
        self.stats.total_cpus = self.cpus.len();
        self.stats.preempt_model = self.model as u8;
        self.stats.total_voluntary = self.cpus.values().map(|c| c.voluntary_preempts).sum();
        self.stats.total_involuntary = self.cpus.values().map(|c| c.involuntary_preempts).sum();
        self.stats.global_max_disable_ns = self.cpus.values().map(|c| c.max_disable_ns).max().unwrap_or(0);
        if !self.cpus.is_empty() {
            let total_avg: f64 = self.cpus.values().map(|c| {
                let total = c.voluntary_preempts + c.involuntary_preempts;
                if total == 0 { 0.0 } else { c.total_disable_ns as f64 / total as f64 }
            }).sum();
            self.stats.avg_disable_ns = total_avg / self.cpus.len() as f64;
        }
        self.stats.active_budgets = self.budgets.len();
        self.stats.budget_violations = self.budgets.values().map(|b| b.violations).sum();
        self.stats.hotspot_count = self.hotspots.len();
        self.stats.currently_disabled_cpus = self.cpus.values().filter(|c| !c.is_preemptible()).count();
    }

    pub fn stats(&self) -> &PreemptCtrlStats { &self.stats }
}
