// SPDX-License-Identifier: GPL-2.0
//! # Apps Interface — Advanced Application Communication & Developer Advice
//!
//! The engine can explain to developers WHY their app is slow and WHAT to do.
//! Generates human-readable bottleneck explanations, optimization advice,
//! performance narratives, comprehensive developer reports, and multi-step
//! improvement roadmaps.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 10;
const MAX_ADVICE_ENTRIES: usize = 512;
const MAX_ROADMAP_STEPS: usize = 32;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut s = *state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    *state = s;
    s
}

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Identifies the category of a performance bottleneck.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BottleneckKind {
    CpuBound,
    MemoryPressure,
    IoBound,
    LockContention,
    IpcLatency,
    SchedulingDelay,
}

/// A diagnosed bottleneck for a specific application.
#[derive(Clone, Debug)]
pub struct Bottleneck {
    pub app_id: u64,
    pub kind: BottleneckKind,
    pub severity: u64,
    pub explanation: String,
}

/// A single piece of optimization advice.
#[derive(Clone, Debug)]
pub struct Advice {
    pub advice_id: u64,
    pub app_id: u64,
    pub priority: u64,
    pub title: String,
    pub description: String,
    pub expected_improvement_pct: u64,
}

/// A step in an improvement roadmap.
#[derive(Clone, Debug)]
pub struct RoadmapStep {
    pub step_index: u64,
    pub title: String,
    pub description: String,
    pub effort_score: u64,
    pub impact_score: u64,
}

/// Per-app performance metrics used for diagnosis.
#[derive(Clone, Debug)]
pub struct AppMetrics {
    pub app_id: u64,
    pub name: String,
    pub cpu_util_ema: u64,
    pub mem_util_ema: u64,
    pub io_wait_ema: u64,
    pub lock_wait_ema: u64,
    pub ipc_latency_ema: u64,
    pub sched_delay_ema: u64,
    pub observation_count: u64,
}

/// Statistics for the interface engine.
#[derive(Clone, Debug, Default)]
pub struct InterfaceStats {
    pub bottlenecks_diagnosed: u64,
    pub advice_generated: u64,
    pub reports_produced: u64,
    pub roadmaps_created: u64,
    pub avg_severity_ema: u64,
}

// ---------------------------------------------------------------------------
// AppsInterface
// ---------------------------------------------------------------------------

/// Engine that communicates application performance insights to developers.
pub struct AppsInterface {
    metrics: BTreeMap<u64, AppMetrics>,
    bottlenecks: BTreeMap<u64, Vec<Bottleneck>>,
    advice_log: Vec<Advice>,
    stats: InterfaceStats,
    rng: u64,
}

impl AppsInterface {
    /// Create a new interface engine.
    pub fn new(seed: u64) -> Self {
        Self {
            metrics: BTreeMap::new(),
            bottlenecks: BTreeMap::new(),
            advice_log: Vec::new(),
            stats: InterfaceStats::default(),
            rng: seed | 1,
        }
    }

    // -- observation --------------------------------------------------------

    /// Ingest performance metrics for an app.
    pub fn ingest_metrics(
        &mut self,
        app_id: u64,
        name: &str,
        cpu: u64,
        mem: u64,
        io_wait: u64,
        lock_wait: u64,
        ipc_lat: u64,
        sched_delay: u64,
    ) {
        let m = self.metrics.entry(app_id).or_insert(AppMetrics {
            app_id,
            name: String::from(name),
            cpu_util_ema: cpu,
            mem_util_ema: mem,
            io_wait_ema: io_wait,
            lock_wait_ema: lock_wait,
            ipc_latency_ema: ipc_lat,
            sched_delay_ema: sched_delay,
            observation_count: 0,
        });
        m.cpu_util_ema = ema_update(m.cpu_util_ema, cpu);
        m.mem_util_ema = ema_update(m.mem_util_ema, mem);
        m.io_wait_ema = ema_update(m.io_wait_ema, io_wait);
        m.lock_wait_ema = ema_update(m.lock_wait_ema, lock_wait);
        m.ipc_latency_ema = ema_update(m.ipc_latency_ema, ipc_lat);
        m.sched_delay_ema = ema_update(m.sched_delay_ema, sched_delay);
        m.observation_count += 1;
    }

    // -- public API ---------------------------------------------------------

    /// Diagnose and explain the primary bottleneck(s) for an application.
    pub fn explain_bottleneck(&mut self, app_id: u64) -> Vec<Bottleneck> {
        let m = match self.metrics.get(&app_id) {
            Some(m) => m.clone(),
            None => return Vec::new(),
        };

        let mut found: Vec<Bottleneck> = Vec::new();

        if m.cpu_util_ema > 80 {
            found.push(Bottleneck {
                app_id,
                kind: BottleneckKind::CpuBound,
                severity: m.cpu_util_ema,
                explanation: Self::build_cpu_explanation(&m),
            });
        }
        if m.mem_util_ema > 75 {
            found.push(Bottleneck {
                app_id,
                kind: BottleneckKind::MemoryPressure,
                severity: m.mem_util_ema,
                explanation: Self::build_mem_explanation(&m),
            });
        }
        if m.io_wait_ema > 40 {
            found.push(Bottleneck {
                app_id,
                kind: BottleneckKind::IoBound,
                severity: m.io_wait_ema,
                explanation: Self::build_io_explanation(&m),
            });
        }
        if m.lock_wait_ema > 30 {
            found.push(Bottleneck {
                app_id,
                kind: BottleneckKind::LockContention,
                severity: m.lock_wait_ema,
                explanation: Self::build_lock_explanation(&m),
            });
        }
        if m.ipc_latency_ema > 50 {
            found.push(Bottleneck {
                app_id,
                kind: BottleneckKind::IpcLatency,
                severity: m.ipc_latency_ema,
                explanation: Self::build_ipc_explanation(&m),
            });
        }
        if m.sched_delay_ema > 20 {
            found.push(Bottleneck {
                app_id,
                kind: BottleneckKind::SchedulingDelay,
                severity: m.sched_delay_ema,
                explanation: Self::build_sched_explanation(&m),
            });
        }

        found.sort_by(|a, b| b.severity.cmp(&a.severity));

        let severity_avg = if found.is_empty() {
            0
        } else {
            found.iter().map(|b| b.severity).sum::<u64>() / found.len() as u64
        };
        self.stats.avg_severity_ema = ema_update(self.stats.avg_severity_ema, severity_avg);
        self.stats.bottlenecks_diagnosed += found.len() as u64;

        self.bottlenecks.insert(app_id, found.clone());
        found
    }

    /// Generate prioritised optimization advice for an application.
    pub fn optimization_advice(&mut self, app_id: u64) -> Vec<Advice> {
        let bottlenecks = self.bottlenecks.get(&app_id).cloned().unwrap_or_default();
        let mut advice_list: Vec<Advice> = Vec::new();

        for bn in &bottlenecks {
            let (title, desc, improvement) = self.advice_for_bottleneck(bn);
            let advice_id = fnv1a(title.as_bytes()) ^ xorshift64(&mut self.rng);
            advice_list.push(Advice {
                advice_id,
                app_id,
                priority: bn.severity,
                title,
                description: desc,
                expected_improvement_pct: improvement,
            });
        }

        advice_list.sort_by(|a, b| b.priority.cmp(&a.priority));

        for adv in &advice_list {
            if self.advice_log.len() < MAX_ADVICE_ENTRIES {
                self.advice_log.push(adv.clone());
            }
        }
        self.stats.advice_generated += advice_list.len() as u64;
        advice_list
    }

    /// Build a performance narrative — a prose summary of app behaviour.
    pub fn performance_narrative(&self, app_id: u64) -> String {
        let m = match self.metrics.get(&app_id) {
            Some(m) => m,
            None => return String::from("No data available for this application."),
        };

        let mut parts: Vec<String> = Vec::new();
        parts.push(alloc::format!(
            "Application '{}' (id={}) has been observed {} times.",
            m.name, m.app_id, m.observation_count
        ));

        if m.cpu_util_ema > 70 {
            parts.push(alloc::format!(
                "CPU utilisation is high at ~{}%, indicating compute-bound phases.",
                m.cpu_util_ema
            ));
        } else {
            parts.push(alloc::format!(
                "CPU utilisation is moderate at ~{}%.",
                m.cpu_util_ema
            ));
        }

        if m.mem_util_ema > 60 {
            parts.push(alloc::format!(
                "Memory pressure sits at ~{}%, watch for allocation stalls.",
                m.mem_util_ema
            ));
        }

        if m.io_wait_ema > 30 {
            parts.push(alloc::format!(
                "IO wait time averages ~{}%, suggesting storage or network bottlenecks.",
                m.io_wait_ema
            ));
        }

        if m.lock_wait_ema > 20 {
            parts.push(alloc::format!(
                "Lock contention at ~{}% — consider reducing critical section size.",
                m.lock_wait_ema
            ));
        }

        let mut narrative = String::new();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                narrative.push(' ');
            }
            narrative.push_str(part);
        }
        narrative
    }

    /// Produce a comprehensive developer report for an app.
    pub fn developer_report(&mut self, app_id: u64) -> String {
        self.stats.reports_produced += 1;
        let narrative = self.performance_narrative(app_id);
        let advice = self.optimization_advice(app_id);

        let mut report = String::from("=== Developer Performance Report ===\n\n");
        report.push_str(&narrative);
        report.push_str("\n\n--- Recommendations ---\n");

        for (i, adv) in advice.iter().enumerate() {
            report.push_str(&alloc::format!(
                "\n{}. [P{}] {}\n   {}\n   Expected improvement: ~{}%\n",
                i + 1,
                adv.priority,
                adv.title,
                adv.description,
                adv.expected_improvement_pct,
            ));
        }

        if advice.is_empty() {
            report.push_str("\nNo actionable recommendations at this time.\n");
        }

        report.push_str("\n=== End of Report ===\n");
        report
    }

    /// Create a multi-step improvement roadmap for an application.
    pub fn improvement_roadmap(&mut self, app_id: u64) -> Vec<RoadmapStep> {
        self.stats.roadmaps_created += 1;
        let advice = self.optimization_advice(app_id);
        let mut steps: Vec<RoadmapStep> = Vec::new();

        for (i, adv) in advice.iter().enumerate() {
            if steps.len() >= MAX_ROADMAP_STEPS {
                break;
            }
            let effort = self.estimate_effort(&adv.title);
            steps.push(RoadmapStep {
                step_index: i as u64,
                title: adv.title.clone(),
                description: adv.description.clone(),
                effort_score: effort,
                impact_score: adv.expected_improvement_pct,
            });
        }

        // Sort by impact/effort ratio (highest first).
        steps.sort_by(|a, b| {
            let ratio_a = a.impact_score * 100 / a.effort_score.max(1);
            let ratio_b = b.impact_score * 100 / b.effort_score.max(1);
            ratio_b.cmp(&ratio_a)
        });

        // Re-index after sorting.
        for (i, step) in steps.iter_mut().enumerate() {
            step.step_index = i as u64;
        }
        steps
    }

    /// Return current statistics.
    pub fn stats(&self) -> &InterfaceStats {
        &self.stats
    }

    // -- internal -----------------------------------------------------------

    fn build_cpu_explanation(m: &AppMetrics) -> String {
        alloc::format!(
            "'{}' is CPU-bound at ~{}% utilisation. \
             The process spends most cycles in compute rather than waiting.",
            m.name, m.cpu_util_ema
        )
    }

    fn build_mem_explanation(m: &AppMetrics) -> String {
        alloc::format!(
            "'{}' has high memory pressure at ~{}%. \
             Allocation latency may increase if usage continues to grow.",
            m.name, m.mem_util_ema
        )
    }

    fn build_io_explanation(m: &AppMetrics) -> String {
        alloc::format!(
            "'{}' is IO-bound with ~{}% wait time. \
             Consider batching IO or using async operations.",
            m.name, m.io_wait_ema
        )
    }

    fn build_lock_explanation(m: &AppMetrics) -> String {
        alloc::format!(
            "'{}' experiences lock contention at ~{}%. \
             Reducing critical section granularity would help.",
            m.name, m.lock_wait_ema
        )
    }

    fn build_ipc_explanation(m: &AppMetrics) -> String {
        alloc::format!(
            "'{}' suffers IPC latency at ~{}%. \
             Consider shared-memory or batched message passing.",
            m.name, m.ipc_latency_ema
        )
    }

    fn build_sched_explanation(m: &AppMetrics) -> String {
        alloc::format!(
            "'{}' has scheduling delays of ~{}%. \
             Raising priority or reducing competing load would help.",
            m.name, m.sched_delay_ema
        )
    }

    fn advice_for_bottleneck(&mut self, bn: &Bottleneck) -> (String, String, u64) {
        match bn.kind {
            BottleneckKind::CpuBound => (
                String::from("Reduce CPU hotspots"),
                String::from("Profile the application to find tight loops or redundant computation. Consider algorithmic improvements or caching."),
                15 + xorshift64(&mut self.rng) % 10,
            ),
            BottleneckKind::MemoryPressure => (
                String::from("Optimise memory allocation"),
                String::from("Audit allocation patterns. Use arenas or pools for short-lived objects. Reduce peak working set."),
                10 + xorshift64(&mut self.rng) % 12,
            ),
            BottleneckKind::IoBound => (
                String::from("Batch or async IO"),
                String::from("Aggregate small IO operations into larger batches. Use asynchronous IO where possible to overlap with computation."),
                20 + xorshift64(&mut self.rng) % 10,
            ),
            BottleneckKind::LockContention => (
                String::from("Reduce lock granularity"),
                String::from("Break coarse-grained locks into finer-grained ones. Consider lock-free data structures for hot paths."),
                12 + xorshift64(&mut self.rng) % 8,
            ),
            BottleneckKind::IpcLatency => (
                String::from("Improve IPC strategy"),
                String::from("Migrate to shared-memory IPC for high-throughput channels. Batch small messages into fewer, larger transfers."),
                18 + xorshift64(&mut self.rng) % 7,
            ),
            BottleneckKind::SchedulingDelay => (
                String::from("Adjust scheduling priority"),
                String::from("Increase the process priority or reduce the runnable set on the target CPU. Consider CPU affinity pinning."),
                8 + xorshift64(&mut self.rng) % 10,
            ),
        }
    }

    fn estimate_effort(&mut self, title: &str) -> u64 {
        let h = fnv1a(title.as_bytes());
        let base = (h % 50) + 10;
        let jitter = xorshift64(&mut self.rng) % 10;
        base + jitter
    }
}
