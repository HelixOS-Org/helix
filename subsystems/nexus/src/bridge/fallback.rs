//! # Syscall Fallback Engine
//!
//! Graceful degradation for syscall handling:
//! - Multi-tier fallback strategies
//! - Compatibility shims
//! - Error recovery
//! - Retry with backoff
//! - Emulation layers
//! - Fallback monitoring and alerting

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// FALLBACK STRATEGY
// ============================================================================

/// Fallback strategy type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FallbackStrategy {
    /// Retry same handler
    Retry,
    /// Retry with different parameters
    RetryModified,
    /// Use alternative handler
    AlternativeHandler,
    /// Emulate with software
    Emulate,
    /// Return cached result
    ReturnCached,
    /// Queue for later
    DeferExecution,
    /// Return default/safe value
    ReturnDefault,
    /// Propagate error
    PropagateError,
    /// Panic (last resort)
    Panic,
}

impl FallbackStrategy {
    /// Safety level (higher = safer)
    pub fn safety_level(&self) -> u32 {
        match self {
            Self::Retry => 8,
            Self::RetryModified => 7,
            Self::AlternativeHandler => 6,
            Self::Emulate => 5,
            Self::ReturnCached => 4,
            Self::DeferExecution => 6,
            Self::ReturnDefault => 3,
            Self::PropagateError => 9,
            Self::Panic => 1,
        }
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum retries
    pub max_retries: u32,
    /// Initial backoff (microseconds)
    pub initial_backoff_us: u64,
    /// Backoff multiplier (fixed-point, 100 = 1.0)
    pub backoff_multiplier: u32,
    /// Maximum backoff (microseconds)
    pub max_backoff_us: u64,
    /// Jitter (percent)
    pub jitter_percent: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_us: 100,
            backoff_multiplier: 200, // 2.0x
            max_backoff_us: 10_000,
            jitter_percent: 25,
        }
    }
}

impl RetryConfig {
    /// Compute backoff for attempt N
    pub fn backoff_us(&self, attempt: u32) -> u64 {
        let mut delay = self.initial_backoff_us;
        for _ in 0..attempt {
            delay = delay * self.backoff_multiplier as u64 / 100;
            if delay > self.max_backoff_us {
                delay = self.max_backoff_us;
                break;
            }
        }
        delay
    }
}

// ============================================================================
// FALLBACK ENTRY
// ============================================================================

/// Error category for fallback selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorCategory {
    /// Temporary resource shortage
    ResourceExhausted,
    /// Permission denied
    PermissionDenied,
    /// Invalid argument
    InvalidArgument,
    /// Not found
    NotFound,
    /// Timeout
    Timeout,
    /// Hardware failure
    HardwareFailure,
    /// Internal error
    Internal,
    /// Not supported
    NotSupported,
    /// Busy / contention
    Busy,
    /// Interrupted
    Interrupted,
}

/// Fallback rule: maps error to strategy
#[derive(Debug, Clone)]
pub struct FallbackRule {
    /// Error category to match
    pub error_category: ErrorCategory,
    /// Strategy to apply
    pub strategy: FallbackStrategy,
    /// Retry config (if strategy is Retry/RetryModified)
    pub retry_config: Option<RetryConfig>,
    /// Alternative handler ID (if strategy is AlternativeHandler)
    pub alt_handler: Option<u32>,
    /// Default return value (if strategy is ReturnDefault)
    pub default_value: Option<i64>,
    /// Priority (higher = tried first)
    pub priority: u32,
}

/// Fallback chain for a specific syscall
#[derive(Debug, Clone)]
pub struct SyscallFallbackChain {
    /// Syscall number
    pub syscall_nr: u32,
    /// Rules in priority order
    rules: Vec<FallbackRule>,
    /// Catch-all strategy
    pub catch_all: FallbackStrategy,
}

impl SyscallFallbackChain {
    pub fn new(syscall_nr: u32) -> Self {
        Self {
            syscall_nr,
            rules: Vec::new(),
            catch_all: FallbackStrategy::PropagateError,
        }
    }

    pub fn add_rule(&mut self, rule: FallbackRule) {
        self.rules.push(rule);
        self.rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Find best fallback for error
    pub fn find_fallback(&self, error: ErrorCategory) -> &FallbackRule {
        self.rules
            .iter()
            .find(|r| r.error_category == error)
            .unwrap_or_else(|| {
                // Return catch-all as a static rule
                &FallbackRule {
                    error_category: error,
                    strategy: self.catch_all,
                    retry_config: None,
                    alt_handler: None,
                    default_value: None,
                    priority: 0,
                }
            })
    }
}

// ============================================================================
// FALLBACK STATE
// ============================================================================

/// Retry state for in-progress fallback
#[derive(Debug, Clone)]
pub struct RetryState {
    /// Syscall number
    pub syscall_nr: u32,
    /// Process ID
    pub pid: u64,
    /// Current attempt
    pub attempt: u32,
    /// Max attempts
    pub max_attempts: u32,
    /// Strategy being used
    pub strategy: FallbackStrategy,
    /// Start timestamp
    pub started_at: u64,
    /// Last attempt timestamp
    pub last_attempt_at: u64,
    /// Next retry timestamp
    pub next_retry_at: u64,
    /// Error category
    pub error: ErrorCategory,
}

/// Fallback result
#[derive(Debug, Clone)]
pub struct FallbackResult {
    /// Was fallback successful
    pub success: bool,
    /// Strategy used
    pub strategy: FallbackStrategy,
    /// Number of attempts
    pub attempts: u32,
    /// Total time spent (microseconds)
    pub total_time_us: u64,
    /// Return value (if applicable)
    pub return_value: Option<i64>,
}

// ============================================================================
// EMULATION LAYER
// ============================================================================

/// Emulation entry
#[derive(Debug, Clone)]
pub struct EmulationEntry {
    /// Syscall number being emulated
    pub syscall_nr: u32,
    /// Emulation handler ID
    pub emulation_handler: u32,
    /// Performance overhead (percent, 100 = same speed)
    pub overhead_percent: u32,
    /// Fidelity (0-100, how closely it matches native)
    pub fidelity: u32,
    /// Known limitations
    pub limitation_flags: u64,
}

/// Emulation registry
pub struct EmulationRegistry {
    /// Available emulations
    entries: BTreeMap<u32, EmulationEntry>,
    /// Usage stats
    pub total_emulations: u64,
}

impl EmulationRegistry {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            total_emulations: 0,
        }
    }

    pub fn register(&mut self, entry: EmulationEntry) {
        self.entries.insert(entry.syscall_nr, entry);
    }

    pub fn lookup(&mut self, syscall_nr: u32) -> Option<&EmulationEntry> {
        let entry = self.entries.get(&syscall_nr);
        if entry.is_some() {
            self.total_emulations += 1;
        }
        entry
    }

    pub fn has_emulation(&self, syscall_nr: u32) -> bool {
        self.entries.contains_key(&syscall_nr)
    }
}

// ============================================================================
// FALLBACK MONITORING
// ============================================================================

/// Fallback statistics per syscall
#[derive(Debug, Clone, Default)]
pub struct FallbackStats {
    /// Total fallback invocations
    pub total_invocations: u64,
    /// Successful recoveries
    pub successful: u64,
    /// Failed recoveries
    pub failed: u64,
    /// By strategy
    pub by_strategy: BTreeMap<u8, u64>,
    /// By error category
    pub by_error: BTreeMap<u8, u64>,
    /// Average recovery time (us)
    pub avg_recovery_time_us: u64,
}

/// Fallback alert
#[derive(Debug, Clone)]
pub struct FallbackAlert {
    /// Alert type
    pub alert_type: FallbackAlertType,
    /// Syscall number
    pub syscall_nr: u32,
    /// Timestamp
    pub timestamp: u64,
    /// Details
    pub detail_value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackAlertType {
    /// Too many fallbacks for a syscall
    ExcessiveFallbacks,
    /// Fallback success rate too low
    LowSuccessRate,
    /// Fallback taking too long
    SlowRecovery,
    /// All fallbacks exhausted
    AllFallbacksExhausted,
}

// ============================================================================
// FALLBACK ENGINE
// ============================================================================

/// Main fallback engine
pub struct FallbackEngine {
    /// Per-syscall fallback chains
    chains: BTreeMap<u32, SyscallFallbackChain>,
    /// Emulation registry
    emulation: EmulationRegistry,
    /// Active retry states
    active_retries: BTreeMap<(u64, u32), RetryState>,
    /// Stats per syscall
    stats: BTreeMap<u32, FallbackStats>,
    /// Alerts
    alerts: Vec<FallbackAlert>,
    /// Max alerts
    max_alerts: usize,
    /// Global fallback invocations
    pub total_fallbacks: u64,
    /// Global success
    pub total_success: u64,
}

impl FallbackEngine {
    pub fn new() -> Self {
        Self {
            chains: BTreeMap::new(),
            emulation: EmulationRegistry::new(),
            active_retries: BTreeMap::new(),
            stats: BTreeMap::new(),
            alerts: Vec::new(),
            max_alerts: 200,
            total_fallbacks: 0,
            total_success: 0,
        }
    }

    /// Register fallback chain
    pub fn register_chain(&mut self, chain: SyscallFallbackChain) {
        self.chains.insert(chain.syscall_nr, chain);
    }

    /// Register emulation
    pub fn register_emulation(&mut self, entry: EmulationEntry) {
        self.emulation.register(entry);
    }

    /// Handle error with fallback
    pub fn handle_error(
        &mut self,
        syscall_nr: u32,
        pid: u64,
        error: ErrorCategory,
        timestamp: u64,
    ) -> FallbackStrategy {
        self.total_fallbacks += 1;

        let stats = self.stats.entry(syscall_nr).or_default();
        stats.total_invocations += 1;
        *stats.by_error.entry(error as u8).or_insert(0) += 1;

        // Check if we have an active retry
        let key = (pid, syscall_nr);
        if let Some(retry) = self.active_retries.get_mut(&key) {
            retry.attempt += 1;
            if retry.attempt >= retry.max_attempts {
                self.active_retries.remove(&key);
                stats.failed += 1;
                return FallbackStrategy::PropagateError;
            }
            retry.last_attempt_at = timestamp;
            return retry.strategy;
        }

        // Look up chain
        let strategy = if let Some(chain) = self.chains.get(&syscall_nr) {
            let rule = chain.find_fallback(error);

            if let Some(ref retry_config) = rule.retry_config {
                // Start retry state
                self.active_retries.insert(key, RetryState {
                    syscall_nr,
                    pid,
                    attempt: 1,
                    max_attempts: retry_config.max_retries,
                    strategy: rule.strategy,
                    started_at: timestamp,
                    last_attempt_at: timestamp,
                    next_retry_at: timestamp + retry_config.initial_backoff_us,
                    error,
                });
            }

            rule.strategy
        } else {
            FallbackStrategy::PropagateError
        };

        *stats.by_strategy.entry(strategy as u8).or_insert(0) += 1;

        strategy
    }

    /// Record fallback success
    pub fn record_success(&mut self, syscall_nr: u32, pid: u64) {
        self.total_success += 1;
        if let Some(stats) = self.stats.get_mut(&syscall_nr) {
            stats.successful += 1;
        }
        self.active_retries.remove(&(pid, syscall_nr));
    }

    /// Record fallback failure
    pub fn record_failure(&mut self, syscall_nr: u32, pid: u64) {
        if let Some(stats) = self.stats.get_mut(&syscall_nr) {
            stats.failed += 1;
        }
        self.active_retries.remove(&(pid, syscall_nr));
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_fallbacks == 0 {
            return 1.0;
        }
        self.total_success as f64 / self.total_fallbacks as f64
    }

    /// Get stats for syscall
    pub fn stats(&self, syscall_nr: u32) -> Option<&FallbackStats> {
        self.stats.get(&syscall_nr)
    }

    /// Add alert
    pub fn add_alert(&mut self, alert: FallbackAlert) {
        self.alerts.push(alert);
        if self.alerts.len() > self.max_alerts {
            self.alerts.remove(0);
        }
    }

    /// Active retry count
    pub fn active_retries(&self) -> usize {
        self.active_retries.len()
    }
}
