//! # Bridge Health Monitor
//!
//! Health monitoring for the bridge/syscall subsystem:
//! - Component health tracking
//! - Heartbeat management
//! - Degraded mode detection
//! - Self-healing triggers
//! - Health scoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// HEALTH TYPES
// ============================================================================

/// Component health state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentHealth {
    /// Healthy
    Healthy,
    /// Degraded but functional
    Degraded,
    /// Failing (partial functionality)
    Failing,
    /// Dead (no functionality)
    Dead,
    /// Unknown (no heartbeat)
    Unknown,
}

/// Bridge component type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BridgeComponent {
    /// Syscall dispatcher
    Dispatcher,
    /// Cache subsystem
    Cache,
    /// Security layer
    Security,
    /// Metrics collector
    Metrics,
    /// Pipeline
    Pipeline,
    /// Router
    Router,
    /// Validator
    Validator,
    /// Throttler
    Throttler,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Component
    pub component: BridgeComponent,
    /// Health state
    pub health: ComponentHealth,
    /// Score (0-100)
    pub score: u32,
    /// Message code
    pub message_code: u32,
    /// Check timestamp
    pub timestamp: u64,
    /// Latency of health check (ns)
    pub check_latency_ns: u64,
}

// ============================================================================
// HEARTBEAT
// ============================================================================

/// Heartbeat tracker
#[derive(Debug)]
pub struct Heartbeat {
    /// Component
    pub component: BridgeComponent,
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
    /// Expected interval (ns)
    pub expected_interval_ns: u64,
    /// Missed beats
    pub missed_beats: u32,
    /// Max tolerable missed
    pub max_missed: u32,
    /// Total beats received
    pub total_beats: u64,
}

impl Heartbeat {
    pub fn new(component: BridgeComponent, expected_interval_ns: u64) -> Self {
        Self {
            component,
            last_heartbeat: 0,
            expected_interval_ns,
            missed_beats: 0,
            max_missed: 3,
            total_beats: 0,
        }
    }

    /// Record heartbeat
    pub fn beat(&mut self, now: u64) {
        self.last_heartbeat = now;
        self.missed_beats = 0;
        self.total_beats += 1;
    }

    /// Check for missed heartbeat
    pub fn check(&mut self, now: u64) -> bool {
        let elapsed = now.saturating_sub(self.last_heartbeat);
        if elapsed > self.expected_interval_ns && self.last_heartbeat > 0 {
            let intervals = elapsed / self.expected_interval_ns;
            self.missed_beats = intervals as u32;
            return self.missed_beats > self.max_missed;
        }
        false
    }

    /// Is alive?
    pub fn is_alive(&self) -> bool {
        self.missed_beats <= self.max_missed
    }
}

// ============================================================================
// COMPONENT STATUS
// ============================================================================

/// Component status
#[derive(Debug)]
pub struct ComponentStatus {
    /// Component
    pub component: BridgeComponent,
    /// Current health
    pub health: ComponentHealth,
    /// Health score (0-100)
    pub score: u32,
    /// Heartbeat
    pub heartbeat: Heartbeat,
    /// Error count (recent)
    pub error_count: u64,
    /// Total errors
    pub total_errors: u64,
    /// Success count (recent)
    pub success_count: u64,
    /// Last check
    pub last_check: u64,
    /// Consecutive failures
    pub consecutive_failures: u32,
}

impl ComponentStatus {
    pub fn new(component: BridgeComponent, heartbeat_interval_ns: u64) -> Self {
        Self {
            component,
            health: ComponentHealth::Unknown,
            score: 100,
            heartbeat: Heartbeat::new(component, heartbeat_interval_ns),
            error_count: 0,
            total_errors: 0,
            success_count: 0,
            last_check: 0,
            consecutive_failures: 0,
        }
    }

    /// Record success
    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.consecutive_failures = 0;
        self.update_score();
    }

    /// Record error
    pub fn record_error(&mut self) {
        self.error_count += 1;
        self.total_errors += 1;
        self.consecutive_failures += 1;
        self.update_score();
    }

    /// Update health score
    fn update_score(&mut self) {
        let total = self.success_count + self.error_count;
        if total == 0 {
            self.score = 100;
            self.health = ComponentHealth::Unknown;
            return;
        }

        let success_rate = self.success_count as f64 / total as f64;
        self.score = (success_rate * 100.0) as u32;

        // Factor in consecutive failures
        if self.consecutive_failures > 10 {
            self.score = self.score.saturating_sub(self.consecutive_failures * 5);
        }

        // Heartbeat factor
        if !self.heartbeat.is_alive() {
            self.score = self.score / 2;
        }

        self.health = if self.score >= 90 {
            ComponentHealth::Healthy
        } else if self.score >= 60 {
            ComponentHealth::Degraded
        } else if self.score >= 20 {
            ComponentHealth::Failing
        } else {
            ComponentHealth::Dead
        };
    }

    /// Error rate
    pub fn error_rate(&self) -> f64 {
        let total = self.success_count + self.error_count;
        if total == 0 {
            return 0.0;
        }
        self.error_count as f64 / total as f64
    }
}

// ============================================================================
// SELF-HEALING
// ============================================================================

/// Self-healing action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealingAction {
    /// Restart component
    Restart,
    /// Reset state
    ResetState,
    /// Failover to backup
    Failover,
    /// Reduce load
    ReduceLoad,
    /// Escalate to operator
    Escalate,
}

/// Healing trigger
#[derive(Debug, Clone)]
pub struct HealingTrigger {
    /// Component
    pub component: BridgeComponent,
    /// Action
    pub action: HealingAction,
    /// Triggered at
    pub timestamp: u64,
    /// Reason score
    pub score_at_trigger: u32,
}

// ============================================================================
// HEALTH ENGINE
// ============================================================================

/// Health stats
#[derive(Debug, Clone, Default)]
pub struct BridgeHealthStats {
    /// Components monitored
    pub components_monitored: usize,
    /// Healthy components
    pub healthy_count: usize,
    /// Degraded components
    pub degraded_count: usize,
    /// Overall score
    pub overall_score: u32,
    /// Healing triggers
    pub healing_triggers: u64,
}

/// Bridge health monitor
pub struct BridgeHealthMonitor {
    /// Component statuses
    components: BTreeMap<u8, ComponentStatus>,
    /// Healing triggers
    triggers: Vec<HealingTrigger>,
    /// Auto-heal enabled
    pub auto_heal: bool,
    /// Score threshold for healing
    pub heal_threshold: u32,
    /// Stats
    stats: BridgeHealthStats,
}

impl BridgeHealthMonitor {
    pub fn new() -> Self {
        Self {
            components: BTreeMap::new(),
            triggers: Vec::new(),
            auto_heal: true,
            heal_threshold: 30,
            stats: BridgeHealthStats::default(),
        }
    }

    /// Register component
    pub fn register(&mut self, component: BridgeComponent, heartbeat_interval_ns: u64) {
        let key = component as u8;
        self.components.insert(key, ComponentStatus::new(component, heartbeat_interval_ns));
        self.update_stats();
    }

    /// Record heartbeat
    pub fn heartbeat(&mut self, component: BridgeComponent, now: u64) {
        let key = component as u8;
        if let Some(status) = self.components.get_mut(&key) {
            status.heartbeat.beat(now);
        }
    }

    /// Record success
    pub fn record_success(&mut self, component: BridgeComponent) {
        let key = component as u8;
        if let Some(status) = self.components.get_mut(&key) {
            status.record_success();
        }
        self.update_stats();
    }

    /// Record error
    pub fn record_error(&mut self, component: BridgeComponent, now: u64) {
        let key = component as u8;
        if let Some(status) = self.components.get_mut(&key) {
            status.record_error();
            if self.auto_heal && status.score < self.heal_threshold {
                let trigger = HealingTrigger {
                    component,
                    action: HealingAction::Restart,
                    timestamp: now,
                    score_at_trigger: status.score,
                };
                self.triggers.push(trigger);
                self.stats.healing_triggers += 1;
            }
        }
        self.update_stats();
    }

    /// Check all heartbeats
    pub fn check_heartbeats(&mut self, now: u64) -> Vec<BridgeComponent> {
        let mut dead = Vec::new();
        for status in self.components.values_mut() {
            if status.heartbeat.check(now) {
                dead.push(status.component);
                status.health = ComponentHealth::Dead;
                status.score = 0;
            }
        }
        self.update_stats();
        dead
    }

    /// Overall health
    pub fn overall_health(&self) -> ComponentHealth {
        if self.components.is_empty() {
            return ComponentHealth::Unknown;
        }
        let worst = self.components.values().map(|c| c.health).max();
        worst.unwrap_or(ComponentHealth::Unknown)
    }

    fn update_stats(&mut self) {
        self.stats.components_monitored = self.components.len();
        self.stats.healthy_count = self.components.values()
            .filter(|c| c.health == ComponentHealth::Healthy).count();
        self.stats.degraded_count = self.components.values()
            .filter(|c| c.health == ComponentHealth::Degraded).count();
        if self.components.is_empty() {
            self.stats.overall_score = 0;
        } else {
            let sum: u32 = self.components.values().map(|c| c.score).sum();
            self.stats.overall_score = sum / self.components.len() as u32;
        }
    }

    /// Stats
    pub fn stats(&self) -> &BridgeHealthStats {
        &self.stats
    }
}
