//! # Application Watchdog
//!
//! Per-application watchdog timers and health monitoring:
//! - Heartbeat tracking
//! - Hang detection
//! - Deadlock detection
//! - Resource leak monitoring
//! - Automatic recovery actions
//! - Health scoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// WATCHDOG TYPES
// ============================================================================

/// Watchdog status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchdogStatus {
    /// Healthy
    Healthy,
    /// Warning threshold
    Warning,
    /// Critical threshold
    Critical,
    /// Not responding
    Unresponsive,
    /// Recovering
    Recovering,
}

/// Health check type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthCheckType {
    /// Heartbeat (periodic ping)
    Heartbeat,
    /// CPU usage check
    CpuUsage,
    /// Memory usage check
    MemoryUsage,
    /// File descriptor leak
    FdLeak,
    /// Thread count
    ThreadCount,
    /// Deadlock detection
    DeadlockDetect,
    /// Stack overflow proximity
    StackOverflow,
    /// Custom check
    Custom,
}

/// Recovery action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Log and continue
    LogOnly,
    /// Send signal
    SendSignal,
    /// Throttle CPU
    Throttle,
    /// Reduce priority
    ReducePriority,
    /// Force garbage collect
    ForceGc,
    /// Restart process
    Restart,
    /// Kill process
    Kill,
}

// ============================================================================
// HEALTH CHECK
// ============================================================================

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Check type
    pub check_type: HealthCheckType,
    /// Passed
    pub passed: bool,
    /// Value (context-dependent)
    pub value: f64,
    /// Threshold
    pub threshold: f64,
    /// Message
    pub message: String,
    /// Timestamp
    pub timestamp: u64,
}

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Check type
    pub check_type: HealthCheckType,
    /// Warning threshold
    pub warning_threshold: f64,
    /// Critical threshold
    pub critical_threshold: f64,
    /// Check interval (ns)
    pub interval_ns: u64,
    /// Enabled
    pub enabled: bool,
    /// Recovery action on warning
    pub warning_action: RecoveryAction,
    /// Recovery action on critical
    pub critical_action: RecoveryAction,
}

impl HealthCheckConfig {
    #[inline]
    pub fn heartbeat(interval_ns: u64, timeout_ns: u64) -> Self {
        Self {
            check_type: HealthCheckType::Heartbeat,
            warning_threshold: timeout_ns as f64 * 0.8,
            critical_threshold: timeout_ns as f64,
            interval_ns,
            enabled: true,
            warning_action: RecoveryAction::LogOnly,
            critical_action: RecoveryAction::Restart,
        }
    }

    #[inline]
    pub fn cpu_usage(interval_ns: u64, warn_pct: f64, crit_pct: f64) -> Self {
        Self {
            check_type: HealthCheckType::CpuUsage,
            warning_threshold: warn_pct,
            critical_threshold: crit_pct,
            interval_ns,
            enabled: true,
            warning_action: RecoveryAction::LogOnly,
            critical_action: RecoveryAction::Throttle,
        }
    }

    #[inline]
    pub fn memory_usage(interval_ns: u64, warn_bytes: u64, crit_bytes: u64) -> Self {
        Self {
            check_type: HealthCheckType::MemoryUsage,
            warning_threshold: warn_bytes as f64,
            critical_threshold: crit_bytes as f64,
            interval_ns,
            enabled: true,
            warning_action: RecoveryAction::ForceGc,
            critical_action: RecoveryAction::Kill,
        }
    }
}

// ============================================================================
// WATCHDOG INSTANCE
// ============================================================================

/// Per-process watchdog
#[derive(Debug, Clone)]
pub struct ProcessWatchdog {
    /// Process ID
    pub pid: u64,
    /// Status
    pub status: WatchdogStatus,
    /// Health checks
    pub checks: BTreeMap<u8, HealthCheckConfig>,
    /// Last heartbeat
    pub last_heartbeat: u64,
    /// Heartbeat count
    pub heartbeat_count: u64,
    /// Failed checks
    pub failed_checks: u64,
    /// Recovery attempts
    pub recovery_attempts: u64,
    /// Health score (0.0-1.0)
    pub health_score: f64,
    /// Recent results
    pub recent_results: VecDeque<HealthCheckResult>,
    /// Max results
    max_results: usize,
    /// Created at
    pub created_at: u64,
}

impl ProcessWatchdog {
    pub fn new(pid: u64, now: u64) -> Self {
        Self {
            pid,
            status: WatchdogStatus::Healthy,
            checks: BTreeMap::new(),
            last_heartbeat: now,
            heartbeat_count: 0,
            failed_checks: 0,
            recovery_attempts: 0,
            health_score: 1.0,
            recent_results: VecDeque::new(),
            max_results: 32,
            created_at: now,
        }
    }

    /// Add health check
    #[inline(always)]
    pub fn add_check(&mut self, config: HealthCheckConfig) {
        self.checks.insert(config.check_type as u8, config);
    }

    /// Record heartbeat
    #[inline(always)]
    pub fn heartbeat(&mut self, now: u64) {
        self.last_heartbeat = now;
        self.heartbeat_count += 1;
    }

    /// Check heartbeat timeout
    #[inline]
    pub fn check_heartbeat(&self, now: u64) -> Option<u64> {
        let elapsed = now.saturating_sub(self.last_heartbeat);
        if let Some(config) = self.checks.get(&(HealthCheckType::Heartbeat as u8)) {
            if elapsed > config.critical_threshold as u64 {
                return Some(elapsed);
            }
        }
        None
    }

    /// Record check result
    #[inline]
    pub fn record_result(&mut self, result: HealthCheckResult) {
        if !result.passed {
            self.failed_checks += 1;
        }
        self.recent_results.push_back(result);
        if self.recent_results.len() > self.max_results {
            self.recent_results.pop_front();
        }
        self.recalculate_health();
    }

    /// Recalculate health score
    fn recalculate_health(&mut self) {
        if self.recent_results.is_empty() {
            self.health_score = 1.0;
            self.status = WatchdogStatus::Healthy;
            return;
        }

        let recent_count = self.recent_results.len().min(10);
        let recent = &self.recent_results[self.recent_results.len() - recent_count..];
        let passed = recent.iter().filter(|r| r.passed).count();
        self.health_score = passed as f64 / recent_count as f64;

        self.status = if self.health_score >= 0.9 {
            WatchdogStatus::Healthy
        } else if self.health_score >= 0.6 {
            WatchdogStatus::Warning
        } else if self.health_score >= 0.3 {
            WatchdogStatus::Critical
        } else {
            WatchdogStatus::Unresponsive
        };
    }

    /// Get required action based on status
    pub fn required_action(&self) -> RecoveryAction {
        match self.status {
            WatchdogStatus::Healthy => RecoveryAction::LogOnly,
            WatchdogStatus::Warning => {
                // Find worst check
                self.recent_results
                    .last()
                    .and_then(|r| {
                        if !r.passed {
                            self.checks
                                .get(&(r.check_type as u8))
                                .map(|c| c.warning_action)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(RecoveryAction::LogOnly)
            }
            WatchdogStatus::Critical | WatchdogStatus::Unresponsive => {
                self.recent_results
                    .last()
                    .and_then(|r| {
                        if !r.passed {
                            self.checks
                                .get(&(r.check_type as u8))
                                .map(|c| c.critical_action)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(RecoveryAction::Restart)
            }
            WatchdogStatus::Recovering => RecoveryAction::LogOnly,
        }
    }

    /// Uptime (ns)
    #[inline(always)]
    pub fn uptime(&self, now: u64) -> u64 {
        now.saturating_sub(self.created_at)
    }
}

// ============================================================================
// WATCHDOG MANAGER
// ============================================================================

/// Watchdog manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppWatchdogStats {
    /// Tracked processes
    pub process_count: usize,
    /// Healthy count
    pub healthy_count: usize,
    /// Warning count
    pub warning_count: usize,
    /// Critical count
    pub critical_count: usize,
    /// Total heartbeats
    pub total_heartbeats: u64,
    /// Total failed checks
    pub total_failed_checks: u64,
    /// Total recovery attempts
    pub total_recovery_attempts: u64,
}

/// Application watchdog manager
pub struct AppWatchdogManager {
    /// Per-process watchdogs
    watchdogs: BTreeMap<u64, ProcessWatchdog>,
    /// Stats
    stats: AppWatchdogStats,
}

impl AppWatchdogManager {
    pub fn new() -> Self {
        Self {
            watchdogs: BTreeMap::new(),
            stats: AppWatchdogStats::default(),
        }
    }

    /// Register process
    #[inline(always)]
    pub fn register(&mut self, pid: u64, now: u64) {
        self.watchdogs.insert(pid, ProcessWatchdog::new(pid, now));
        self.stats.process_count = self.watchdogs.len();
    }

    /// Add health check
    #[inline]
    pub fn add_check(&mut self, pid: u64, config: HealthCheckConfig) {
        if let Some(wd) = self.watchdogs.get_mut(&pid) {
            wd.add_check(config);
        }
    }

    /// Record heartbeat
    #[inline]
    pub fn heartbeat(&mut self, pid: u64, now: u64) {
        if let Some(wd) = self.watchdogs.get_mut(&pid) {
            wd.heartbeat(now);
            self.stats.total_heartbeats += 1;
        }
    }

    /// Record check result
    #[inline]
    pub fn record_result(&mut self, pid: u64, result: HealthCheckResult) {
        if let Some(wd) = self.watchdogs.get_mut(&pid) {
            if !result.passed {
                self.stats.total_failed_checks += 1;
            }
            wd.record_result(result);
        }
    }

    /// Check all watchdogs for timeouts
    pub fn check_all(&mut self, now: u64) -> Vec<(u64, RecoveryAction)> {
        let mut actions = Vec::new();

        for wd in self.watchdogs.values_mut() {
            if let Some(elapsed) = wd.check_heartbeat(now) {
                let action = if elapsed > 10_000_000_000 {
                    RecoveryAction::Kill
                } else if elapsed > 5_000_000_000 {
                    RecoveryAction::Restart
                } else {
                    RecoveryAction::SendSignal
                };
                wd.status = WatchdogStatus::Unresponsive;
                actions.push((wd.pid, action));
            }
        }

        self.update_counts();
        actions
    }

    fn update_counts(&mut self) {
        self.stats.healthy_count = self
            .watchdogs
            .values()
            .filter(|w| w.status == WatchdogStatus::Healthy)
            .count();
        self.stats.warning_count = self
            .watchdogs
            .values()
            .filter(|w| w.status == WatchdogStatus::Warning)
            .count();
        self.stats.critical_count = self
            .watchdogs
            .values()
            .filter(|w| matches!(w.status, WatchdogStatus::Critical | WatchdogStatus::Unresponsive))
            .count();
    }

    /// Unregister
    #[inline(always)]
    pub fn unregister(&mut self, pid: u64) {
        self.watchdogs.remove(&pid);
        self.stats.process_count = self.watchdogs.len();
    }

    /// Get watchdog
    #[inline(always)]
    pub fn watchdog(&self, pid: u64) -> Option<&ProcessWatchdog> {
        self.watchdogs.get(&pid)
    }

    /// List unhealthy
    #[inline]
    pub fn unhealthy(&self) -> Vec<u64> {
        self.watchdogs
            .iter()
            .filter(|(_, w)| w.status != WatchdogStatus::Healthy)
            .map(|(&pid, _)| pid)
            .collect()
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppWatchdogStats {
        &self.stats
    }
}
