//! # Cooperative Health Checking
//!
//! Health monitoring for cooperative process groups:
//! - Liveness probes (process is alive)
//! - Readiness probes (process can accept work)
//! - Startup probes (process has initialized)
//! - Custom health endpoints
//! - Cascading failure detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// HEALTH CHECK TYPES
// ============================================================================

/// Probe type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeType {
    /// Liveness (is it running?)
    Liveness,
    /// Readiness (can it serve?)
    Readiness,
    /// Startup (has it initialized?)
    Startup,
    /// Custom check
    Custom,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopHealthStatus {
    /// Healthy
    Healthy,
    /// Degraded (partially working)
    Degraded,
    /// Unhealthy
    Unhealthy,
    /// Unknown (probe pending)
    Unknown,
    /// Starting up
    Starting,
    /// Shutting down
    ShuttingDown,
}

/// Probe result
#[derive(Debug, Clone)]
pub struct ProbeResult {
    /// Probe type
    pub probe_type: ProbeType,
    /// Success
    pub success: bool,
    /// Latency (ns)
    pub latency_ns: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Failure reason (if failed)
    pub failure_reason: Option<u8>,
}

// ============================================================================
// HEALTH PROBE CONFIGURATION
// ============================================================================

/// Probe configuration
#[derive(Debug, Clone)]
pub struct ProbeConfig {
    /// Probe type
    pub probe_type: ProbeType,
    /// Interval between probes (ns)
    pub interval_ns: u64,
    /// Timeout (ns)
    pub timeout_ns: u64,
    /// Success threshold (consecutive)
    pub success_threshold: u32,
    /// Failure threshold (consecutive)
    pub failure_threshold: u32,
    /// Initial delay (ns, for startup)
    pub initial_delay_ns: u64,
}

impl ProbeConfig {
    pub fn liveness() -> Self {
        Self {
            probe_type: ProbeType::Liveness,
            interval_ns: 10_000_000_000, // 10s
            timeout_ns: 1_000_000_000,   // 1s
            success_threshold: 1,
            failure_threshold: 3,
            initial_delay_ns: 0,
        }
    }

    pub fn readiness() -> Self {
        Self {
            probe_type: ProbeType::Readiness,
            interval_ns: 5_000_000_000, // 5s
            timeout_ns: 1_000_000_000,  // 1s
            success_threshold: 1,
            failure_threshold: 3,
            initial_delay_ns: 5_000_000_000, // 5s
        }
    }

    pub fn startup() -> Self {
        Self {
            probe_type: ProbeType::Startup,
            interval_ns: 1_000_000_000, // 1s
            timeout_ns: 1_000_000_000,  // 1s
            success_threshold: 1,
            failure_threshold: 30,
            initial_delay_ns: 0,
        }
    }
}

// ============================================================================
// PROCESS HEALTH STATE
// ============================================================================

/// Per-process health state
#[derive(Debug)]
pub struct ProcessHealthState {
    /// Process id
    pub pid: u64,
    /// Overall status
    pub status: CoopHealthStatus,
    /// Liveness
    pub liveness: ProbeState,
    /// Readiness
    pub readiness: ProbeState,
    /// Startup
    pub startup: ProbeState,
    /// Last status change
    pub last_change: u64,
    /// Consecutive healthy checks
    pub consecutive_healthy: u32,
    /// Consecutive unhealthy checks
    pub consecutive_unhealthy: u32,
    /// Total probes
    pub total_probes: u64,
    /// Total failures
    pub total_failures: u64,
}

/// State of a single probe
#[derive(Debug, Clone)]
pub struct ProbeState {
    /// Configured?
    pub configured: bool,
    /// Last result
    pub last_result: Option<ProbeResult>,
    /// Consecutive successes
    pub consecutive_success: u32,
    /// Consecutive failures
    pub consecutive_failure: u32,
    /// Next probe due at
    pub next_probe_at: u64,
    /// Average latency (EMA, ns)
    pub avg_latency_ns: f64,
}

impl ProbeState {
    pub fn new() -> Self {
        Self {
            configured: false,
            last_result: None,
            consecutive_success: 0,
            consecutive_failure: 0,
            next_probe_at: 0,
            avg_latency_ns: 0.0,
        }
    }

    /// Record result
    pub fn record(&mut self, result: ProbeResult, interval_ns: u64) {
        if result.success {
            self.consecutive_success += 1;
            self.consecutive_failure = 0;
        } else {
            self.consecutive_failure += 1;
            self.consecutive_success = 0;
        }
        // EMA
        let alpha = 0.3;
        self.avg_latency_ns =
            alpha * result.latency_ns as f64 + (1.0 - alpha) * self.avg_latency_ns;
        self.next_probe_at = result.timestamp + interval_ns;
        self.last_result = Some(result);
    }
}

impl ProcessHealthState {
    pub fn new(pid: u64, now: u64) -> Self {
        Self {
            pid,
            status: CoopHealthStatus::Unknown,
            liveness: ProbeState::new(),
            readiness: ProbeState::new(),
            startup: ProbeState::new(),
            last_change: now,
            consecutive_healthy: 0,
            consecutive_unhealthy: 0,
            total_probes: 0,
            total_failures: 0,
        }
    }

    /// Record probe result
    pub fn record_probe(&mut self, result: ProbeResult, config: &ProbeConfig, now: u64) {
        self.total_probes += 1;
        if !result.success {
            self.total_failures += 1;
        }

        match result.probe_type {
            ProbeType::Liveness => self.liveness.record(result, config.interval_ns),
            ProbeType::Readiness => self.readiness.record(result, config.interval_ns),
            ProbeType::Startup => self.startup.record(result, config.interval_ns),
            ProbeType::Custom => {},
        }

        self.recalculate_status(now);
    }

    fn recalculate_status(&mut self, now: u64) {
        let old_status = self.status;

        // Startup check first
        if self.startup.configured && self.startup.consecutive_success == 0 {
            self.status = CoopHealthStatus::Starting;
        } else if self.liveness.configured && self.liveness.consecutive_failure >= 3 {
            self.status = CoopHealthStatus::Unhealthy;
        } else if self.readiness.configured && self.readiness.consecutive_failure >= 3 {
            self.status = CoopHealthStatus::Degraded;
        } else if self.liveness.configured && self.liveness.consecutive_success >= 1 {
            if self.readiness.configured && self.readiness.consecutive_success >= 1 {
                self.status = CoopHealthStatus::Healthy;
            } else if !self.readiness.configured {
                self.status = CoopHealthStatus::Healthy;
            } else {
                self.status = CoopHealthStatus::Degraded;
            }
        }

        if self.status != old_status {
            self.last_change = now;
            if self.status == CoopHealthStatus::Healthy {
                self.consecutive_healthy += 1;
                self.consecutive_unhealthy = 0;
            } else if self.status == CoopHealthStatus::Unhealthy {
                self.consecutive_unhealthy += 1;
                self.consecutive_healthy = 0;
            }
        }
    }

    /// Failure rate
    pub fn failure_rate(&self) -> f64 {
        if self.total_probes == 0 {
            return 0.0;
        }
        self.total_failures as f64 / self.total_probes as f64
    }
}

// ============================================================================
// CASCADE DETECTOR
// ============================================================================

/// Cascading failure event
#[derive(Debug, Clone)]
pub struct CascadeEvent {
    /// Origin pid
    pub origin: u64,
    /// Affected pids
    pub affected: Vec<u64>,
    /// Timestamp
    pub timestamp: u64,
    /// Depth (propagation hops)
    pub depth: u32,
}

/// Cascade detector
#[derive(Debug)]
pub struct CascadeDetector {
    /// Dependency graph: pid -> depends_on
    dependencies: BTreeMap<u64, Vec<u64>>,
    /// Recent cascades
    cascades: Vec<CascadeEvent>,
    /// Max cascades
    max_cascades: usize,
}

impl CascadeDetector {
    pub fn new() -> Self {
        Self {
            dependencies: BTreeMap::new(),
            cascades: Vec::new(),
            max_cascades: 64,
        }
    }

    /// Add dependency
    pub fn add_dependency(&mut self, pid: u64, depends_on: u64) {
        self.dependencies
            .entry(pid)
            .or_insert_with(Vec::new)
            .push(depends_on);
    }

    /// Remove process
    pub fn remove(&mut self, pid: u64) {
        self.dependencies.remove(&pid);
        for deps in self.dependencies.values_mut() {
            deps.retain(|&d| d != pid);
        }
    }

    /// Detect cascade from failed pid
    pub fn detect_cascade(&mut self, failed_pid: u64, now: u64) -> Option<CascadeEvent> {
        // Find all processes depending on failed one
        let affected: Vec<u64> = self
            .dependencies
            .iter()
            .filter(|(_, deps)| deps.contains(&failed_pid))
            .map(|(&pid, _)| pid)
            .collect();

        if affected.is_empty() {
            return None;
        }

        let event = CascadeEvent {
            origin: failed_pid,
            affected: affected.clone(),
            timestamp: now,
            depth: 1,
        };

        if self.cascades.len() >= self.max_cascades {
            self.cascades.remove(0);
        }
        self.cascades.push(event.clone());
        Some(event)
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Health check stats
#[derive(Debug, Clone, Default)]
pub struct CoopHealthCheckStats {
    /// Monitored processes
    pub monitored_count: usize,
    /// Healthy count
    pub healthy_count: usize,
    /// Degraded count
    pub degraded_count: usize,
    /// Unhealthy count
    pub unhealthy_count: usize,
    /// Cascade events
    pub cascade_count: usize,
}

/// Cooperative health check manager
pub struct CoopHealthCheckManager {
    /// Per-process states
    states: BTreeMap<u64, ProcessHealthState>,
    /// Probe configs: probe_type -> config
    configs: BTreeMap<u8, ProbeConfig>,
    /// Cascade detector
    pub cascade: CascadeDetector,
    /// Stats
    stats: CoopHealthCheckStats,
}

impl CoopHealthCheckManager {
    pub fn new() -> Self {
        let mut configs = BTreeMap::new();
        configs.insert(ProbeType::Liveness as u8, ProbeConfig::liveness());
        configs.insert(ProbeType::Readiness as u8, ProbeConfig::readiness());
        configs.insert(ProbeType::Startup as u8, ProbeConfig::startup());

        Self {
            states: BTreeMap::new(),
            configs,
            cascade: CascadeDetector::new(),
            stats: CoopHealthCheckStats::default(),
        }
    }

    /// Register process
    pub fn register(&mut self, pid: u64, now: u64) {
        let mut state = ProcessHealthState::new(pid, now);
        state.liveness.configured = true;
        state.readiness.configured = true;
        state.startup.configured = true;
        self.states.insert(pid, state);
        self.update_stats();
    }

    /// Record probe result
    pub fn record_probe(&mut self, pid: u64, result: ProbeResult, now: u64) {
        let config_key = result.probe_type as u8;
        let config = self.configs.get(&config_key).cloned();
        if let (Some(state), Some(cfg)) = (self.states.get_mut(&pid), config) {
            let was_healthy = state.status == CoopHealthStatus::Healthy;
            state.record_probe(result, &cfg, now);
            // Check for cascade if went unhealthy
            if was_healthy && state.status == CoopHealthStatus::Unhealthy {
                self.cascade.detect_cascade(pid, now);
            }
        }
        self.update_stats();
    }

    /// Remove process
    pub fn remove(&mut self, pid: u64) {
        self.states.remove(&pid);
        self.cascade.remove(pid);
        self.update_stats();
    }

    /// Get process health
    pub fn health(&self, pid: u64) -> Option<&ProcessHealthState> {
        self.states.get(&pid)
    }

    /// Get unhealthy processes
    pub fn unhealthy(&self) -> Vec<u64> {
        self.states
            .values()
            .filter(|s| s.status == CoopHealthStatus::Unhealthy)
            .map(|s| s.pid)
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.monitored_count = self.states.len();
        self.stats.healthy_count = self
            .states
            .values()
            .filter(|s| s.status == CoopHealthStatus::Healthy)
            .count();
        self.stats.degraded_count = self
            .states
            .values()
            .filter(|s| s.status == CoopHealthStatus::Degraded)
            .count();
        self.stats.unhealthy_count = self
            .states
            .values()
            .filter(|s| s.status == CoopHealthStatus::Unhealthy)
            .count();
        self.stats.cascade_count = self.cascade.cascades.len();
    }

    /// Stats
    pub fn stats(&self) -> &CoopHealthCheckStats {
        &self.stats
    }
}
