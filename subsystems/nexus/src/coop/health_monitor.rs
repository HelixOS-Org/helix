//! # Cooperative Health Monitor
//!
//! Distributed health monitoring for cooperative subsystems:
//! - Heartbeat-based liveness detection
//! - Health score aggregation
//! - Failure domain tracking
//! - Cascading failure prevention
//! - Recovery orchestration
//! - Health dependency graph

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
    Failed,
}

/// Failure domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureDomain {
    Cpu,
    Memory,
    Io,
    Network,
    Software,
    Hardware,
}

/// Recovery action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryActionCoop {
    None,
    Restart,
    Failover,
    Isolate,
    Degrade,
    Escalate,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    pub component_id: u64,
    pub status: HealthStatus,
    pub score: f64, // 0.0 = dead, 1.0 = perfect
    pub timestamp: u64,
    pub message_hash: u64,
    pub latency_ns: u64,
}

/// Heartbeat record
#[derive(Debug, Clone)]
pub struct Heartbeat {
    pub component_id: u64,
    pub sequence: u64,
    pub timestamp: u64,
    pub load: f64,
    pub healthy: bool,
}

/// Monitored component
#[derive(Debug, Clone)]
pub struct MonitoredComponent {
    pub component_id: u64,
    pub domain: FailureDomain,
    pub status: HealthStatus,
    pub health_score: f64,
    pub last_heartbeat: u64,
    pub heartbeat_interval_ns: u64,
    pub missed_heartbeats: u32,
    pub max_missed: u32,
    pub checks: Vec<HealthCheck>,
    pub dependencies: Vec<u64>,
    pub total_failures: u64,
    pub last_failure_ts: u64,
    pub recovery_action: RecoveryActionCoop,
}

impl MonitoredComponent {
    pub fn new(component_id: u64, domain: FailureDomain) -> Self {
        Self {
            component_id,
            domain,
            status: HealthStatus::Unknown,
            health_score: 1.0,
            last_heartbeat: 0,
            heartbeat_interval_ns: 1_000_000_000, // 1s
            missed_heartbeats: 0,
            max_missed: 3,
            checks: Vec::new(),
            dependencies: Vec::new(),
            total_failures: 0,
            last_failure_ts: 0,
            recovery_action: RecoveryActionCoop::None,
        }
    }

    pub fn record_heartbeat(&mut self, hb: Heartbeat) {
        self.last_heartbeat = hb.timestamp;
        self.missed_heartbeats = 0;
        if hb.healthy {
            self.health_score = 0.9 * self.health_score + 0.1 * 1.0;
        } else {
            self.health_score = 0.9 * self.health_score + 0.1 * 0.3;
        }
        self.update_status();
    }

    pub fn check_timeout(&mut self, now: u64) {
        if self.last_heartbeat == 0 { return; }
        let elapsed = now.saturating_sub(self.last_heartbeat);
        if elapsed > self.heartbeat_interval_ns {
            let missed = (elapsed / self.heartbeat_interval_ns) as u32;
            self.missed_heartbeats = missed;
            if missed >= self.max_missed {
                self.health_score *= 0.5;
                self.status = HealthStatus::Failed;
                self.total_failures += 1;
                self.last_failure_ts = now;
            }
        }
        self.update_status();
    }

    pub fn add_check(&mut self, check: HealthCheck) {
        self.health_score = 0.7 * self.health_score + 0.3 * check.score;
        self.checks.push(check);
        if self.checks.len() > 32 { self.checks.remove(0); }
        self.update_status();
    }

    fn update_status(&mut self) {
        self.status = if self.health_score > 0.8 { HealthStatus::Healthy }
        else if self.health_score > 0.5 { HealthStatus::Degraded }
        else if self.health_score > 0.1 { HealthStatus::Unhealthy }
        else { HealthStatus::Failed };

        self.recovery_action = match self.status {
            HealthStatus::Healthy => RecoveryActionCoop::None,
            HealthStatus::Degraded => RecoveryActionCoop::Degrade,
            HealthStatus::Unhealthy => RecoveryActionCoop::Restart,
            HealthStatus::Failed => RecoveryActionCoop::Failover,
            HealthStatus::Unknown => RecoveryActionCoop::None,
        };
    }

    pub fn is_alive(&self) -> bool {
        !matches!(self.status, HealthStatus::Failed | HealthStatus::Unknown)
    }
}

/// Cascading failure detector
#[derive(Debug, Clone)]
pub struct CascadeDetector {
    pub failure_window_ns: u64,
    pub failure_threshold: u32,
    pub recent_failures: Vec<(u64, u64)>, // (component_id, timestamp)
}

impl CascadeDetector {
    pub fn new(window_ns: u64, threshold: u32) -> Self {
        Self {
            failure_window_ns: window_ns,
            failure_threshold: threshold,
            recent_failures: Vec::new(),
        }
    }

    pub fn record_failure(&mut self, component_id: u64, ts: u64) {
        self.recent_failures.push((component_id, ts));
        self.recent_failures.retain(|&(_, t)| ts.saturating_sub(t) <= self.failure_window_ns);
    }

    pub fn is_cascading(&self) -> bool {
        self.recent_failures.len() as u32 >= self.failure_threshold
    }

    pub fn affected_components(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self.recent_failures.iter().map(|&(id, _)| id).collect();
        ids.sort();
        ids.dedup();
        ids
    }
}

/// Coop health monitor stats
#[derive(Debug, Clone, Default)]
pub struct CoopHealthMonitorStats {
    pub total_components: usize,
    pub healthy_count: usize,
    pub degraded_count: usize,
    pub failed_count: usize,
    pub total_failures: u64,
    pub is_cascading: bool,
}

/// Cooperative Health Monitor
pub struct CoopHealthMonitor {
    components: BTreeMap<u64, MonitoredComponent>,
    cascade: CascadeDetector,
    stats: CoopHealthMonitorStats,
}

impl CoopHealthMonitor {
    pub fn new() -> Self {
        Self {
            components: BTreeMap::new(),
            cascade: CascadeDetector::new(5_000_000_000, 3),
            stats: CoopHealthMonitorStats::default(),
        }
    }

    pub fn register(&mut self, component_id: u64, domain: FailureDomain) {
        self.components.entry(component_id)
            .or_insert_with(|| MonitoredComponent::new(component_id, domain));
        self.recompute();
    }

    pub fn add_dependency(&mut self, component_id: u64, depends_on: u64) {
        if let Some(comp) = self.components.get_mut(&component_id) {
            if !comp.dependencies.contains(&depends_on) {
                comp.dependencies.push(depends_on);
            }
        }
    }

    pub fn heartbeat(&mut self, hb: Heartbeat) {
        let cid = hb.component_id;
        if let Some(comp) = self.components.get_mut(&cid) {
            comp.record_heartbeat(hb);
        }
        self.recompute();
    }

    pub fn check(&mut self, check: HealthCheck) {
        let cid = check.component_id;
        if let Some(comp) = self.components.get_mut(&cid) {
            comp.add_check(check);
        }
        self.recompute();
    }

    pub fn tick(&mut self, now: u64) {
        let ids: Vec<u64> = self.components.keys().copied().collect();
        for cid in ids {
            if let Some(comp) = self.components.get_mut(&cid) {
                let was_alive = comp.is_alive();
                comp.check_timeout(now);
                if was_alive && !comp.is_alive() {
                    self.cascade.record_failure(cid, now);
                }
            }
        }
        self.recompute();
    }

    /// Get components that would be affected if a specific component fails
    pub fn impact_analysis(&self, component_id: u64) -> Vec<u64> {
        let mut affected = Vec::new();
        for comp in self.components.values() {
            if comp.dependencies.contains(&component_id) {
                affected.push(comp.component_id);
            }
        }
        affected
    }

    fn recompute(&mut self) {
        self.stats.total_components = self.components.len();
        self.stats.healthy_count = self.components.values()
            .filter(|c| c.status == HealthStatus::Healthy).count();
        self.stats.degraded_count = self.components.values()
            .filter(|c| c.status == HealthStatus::Degraded).count();
        self.stats.failed_count = self.components.values()
            .filter(|c| c.status == HealthStatus::Failed).count();
        self.stats.total_failures = self.components.values().map(|c| c.total_failures).sum();
        self.stats.is_cascading = self.cascade.is_cascading();
    }

    pub fn component(&self, id: u64) -> Option<&MonitoredComponent> {
        self.components.get(&id)
    }

    pub fn stats(&self) -> &CoopHealthMonitorStats {
        &self.stats
    }
}
