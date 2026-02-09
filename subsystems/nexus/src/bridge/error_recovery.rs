//! # Bridge Error Recovery
//!
//! Syscall error handling and automatic recovery:
//! - Error pattern detection
//! - Automatic retry with backoff
//! - Error translation and normalization
//! - Recovery strategy selection
//! - Error budget tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Error category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyscallErrorCategory {
    /// Permission denied
    Permission,
    /// Resource not found
    NotFound,
    /// Resource busy/locked
    Busy,
    /// Out of memory/resources
    Resource,
    /// Invalid argument
    Invalid,
    /// Timeout
    Timeout,
    /// I/O error
    Io,
    /// Interrupted
    Interrupted,
    /// Not supported
    NotSupported,
    /// Internal error
    Internal,
}

/// Recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Retry immediately
    RetryImmediate,
    /// Retry with exponential backoff
    RetryBackoff,
    /// Return error to caller
    PropagateError,
    /// Fall back to alternative syscall
    Fallback,
    /// Queue for later retry
    DeferRetry,
    /// Abort operation
    Abort,
}

/// Error entry
#[derive(Debug, Clone)]
pub struct SyscallError {
    /// Syscall number
    pub syscall_nr: u32,
    /// Error code
    pub error_code: i32,
    /// Category
    pub category: SyscallErrorCategory,
    /// PID
    pub pid: u64,
    /// Timestamp (ns)
    pub timestamp_ns: u64,
    /// Recovery attempted
    pub recovery_attempted: bool,
    /// Recovery succeeded
    pub recovery_succeeded: bool,
}

/// Error pattern (recurring error)
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ErrorPattern {
    /// Syscall number
    pub syscall_nr: u32,
    /// Category
    pub category: SyscallErrorCategory,
    /// Occurrence count
    pub count: u64,
    /// First seen (ns)
    pub first_seen_ns: u64,
    /// Last seen (ns)
    pub last_seen_ns: u64,
    /// Rate (errors/sec EMA)
    pub rate_ema: f64,
    /// Best recovery strategy observed
    pub best_strategy: RecoveryStrategy,
    /// Recovery success rate
    pub recovery_success_rate: f64,
    /// Recovery attempts
    recovery_attempts: u64,
    /// Recovery successes
    recovery_successes: u64,
}

impl ErrorPattern {
    pub fn new(syscall_nr: u32, category: SyscallErrorCategory, now_ns: u64) -> Self {
        Self {
            syscall_nr,
            category,
            count: 1,
            first_seen_ns: now_ns,
            last_seen_ns: now_ns,
            rate_ema: 0.0,
            best_strategy: RecoveryStrategy::PropagateError,
            recovery_success_rate: 0.0,
            recovery_attempts: 0,
            recovery_successes: 0,
        }
    }

    /// Record occurrence
    #[inline]
    pub fn record(&mut self, now_ns: u64) {
        self.count += 1;
        let interval = now_ns.saturating_sub(self.last_seen_ns) as f64 / 1_000_000_000.0;
        if interval > 0.0 {
            let instant_rate = 1.0 / interval;
            self.rate_ema = 0.8 * self.rate_ema + 0.2 * instant_rate;
        }
        self.last_seen_ns = now_ns;
    }

    /// Record recovery attempt
    #[inline]
    pub fn record_recovery(&mut self, succeeded: bool) {
        self.recovery_attempts += 1;
        if succeeded {
            self.recovery_successes += 1;
        }
        if self.recovery_attempts > 0 {
            self.recovery_success_rate = self.recovery_successes as f64 / self.recovery_attempts as f64;
        }
    }

    /// Is this a burst?
    #[inline(always)]
    pub fn is_burst(&self) -> bool {
        self.rate_ema > 10.0
    }

    /// Pattern key
    #[inline]
    pub fn key(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= self.syscall_nr as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.category as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }
}

/// Error recovery engine stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeErrorRecoveryStats {
    pub total_errors: u64,
    pub unique_patterns: usize,
    pub recovery_attempts: u64,
    pub recovery_successes: u64,
    pub recovery_rate: f64,
    pub active_bursts: usize,
}

/// Bridge error recovery engine
#[repr(align(64))]
pub struct BridgeErrorRecovery {
    /// Error patterns
    patterns: BTreeMap<u64, ErrorPattern>,
    /// Recent errors (ring buffer)
    recent: Vec<SyscallError>,
    /// Recent position
    recent_pos: usize,
    /// Max recent
    max_recent: usize,
    /// Total errors
    total_errors: u64,
    /// Recovery attempts
    recovery_attempts: u64,
    /// Recovery successes
    recovery_successes: u64,
    /// Stats
    stats: BridgeErrorRecoveryStats,
}

impl BridgeErrorRecovery {
    pub fn new() -> Self {
        let max_recent = 128;
        Self {
            patterns: BTreeMap::new(),
            recent: Vec::new(),
            recent_pos: 0,
            max_recent,
            total_errors: 0,
            recovery_attempts: 0,
            recovery_successes: 0,
            stats: BridgeErrorRecoveryStats::default(),
        }
    }

    /// Classify error code to category
    pub fn classify(error_code: i32) -> SyscallErrorCategory {
        match error_code {
            -1 => SyscallErrorCategory::Permission,      // EPERM
            -2 => SyscallErrorCategory::NotFound,         // ENOENT
            -11 => SyscallErrorCategory::Busy,            // EAGAIN
            -12 => SyscallErrorCategory::Resource,        // ENOMEM
            -13 => SyscallErrorCategory::Permission,      // EACCES
            -14 => SyscallErrorCategory::Invalid,         // EFAULT
            -16 => SyscallErrorCategory::Busy,            // EBUSY
            -22 => SyscallErrorCategory::Invalid,         // EINVAL
            -28 => SyscallErrorCategory::Resource,        // ENOSPC
            -110 => SyscallErrorCategory::Timeout,        // ETIMEDOUT
            -4 => SyscallErrorCategory::Interrupted,      // EINTR
            -38 => SyscallErrorCategory::NotSupported,    // ENOSYS
            _ => SyscallErrorCategory::Internal,
        }
    }

    /// Record error and get recovery strategy
    pub fn record_error(&mut self, syscall_nr: u32, error_code: i32, pid: u64, now_ns: u64) -> RecoveryStrategy {
        let category = Self::classify(error_code);
        self.total_errors += 1;

        // Update pattern
        let pattern_key = {
            let mut hash: u64 = 0xcbf29ce484222325;
            hash ^= syscall_nr as u64;
            hash = hash.wrapping_mul(0x100000001b3);
            hash ^= category as u64;
            hash = hash.wrapping_mul(0x100000001b3);
            hash
        };

        let pattern = self.patterns.entry(pattern_key)
            .or_insert_with(|| ErrorPattern::new(syscall_nr, category, now_ns));
        pattern.record(now_ns);

        // Store in recent
        let entry = SyscallError {
            syscall_nr, error_code, category, pid,
            timestamp_ns: now_ns,
            recovery_attempted: false,
            recovery_succeeded: false,
        };
        if self.recent.len() < self.max_recent {
            self.recent.push(entry);
        } else {
            self.recent[self.recent_pos % self.max_recent] = entry;
        }
        self.recent_pos += 1;

        // Determine strategy
        let strategy = self.select_strategy(category, pattern);
        self.update_stats();
        strategy
    }

    fn select_strategy(&self, category: SyscallErrorCategory, pattern: &ErrorPattern) -> RecoveryStrategy {
        // If pattern has good recovery rate, use its best strategy
        if pattern.recovery_attempts > 5 && pattern.recovery_success_rate > 0.7 {
            return pattern.best_strategy;
        }

        match category {
            SyscallErrorCategory::Interrupted => RecoveryStrategy::RetryImmediate,
            SyscallErrorCategory::Busy => {
                if pattern.is_burst() {
                    RecoveryStrategy::DeferRetry
                } else {
                    RecoveryStrategy::RetryBackoff
                }
            }
            SyscallErrorCategory::Resource => RecoveryStrategy::DeferRetry,
            SyscallErrorCategory::Timeout => RecoveryStrategy::RetryBackoff,
            SyscallErrorCategory::Permission => RecoveryStrategy::PropagateError,
            SyscallErrorCategory::NotFound => RecoveryStrategy::PropagateError,
            SyscallErrorCategory::Invalid => RecoveryStrategy::Abort,
            SyscallErrorCategory::NotSupported => RecoveryStrategy::Fallback,
            SyscallErrorCategory::Io => RecoveryStrategy::RetryBackoff,
            SyscallErrorCategory::Internal => RecoveryStrategy::Abort,
        }
    }

    /// Record recovery result
    pub fn record_recovery(&mut self, syscall_nr: u32, category: SyscallErrorCategory, succeeded: bool) {
        self.recovery_attempts += 1;
        if succeeded {
            self.recovery_successes += 1;
        }
        let pattern_key = {
            let mut hash: u64 = 0xcbf29ce484222325;
            hash ^= syscall_nr as u64;
            hash = hash.wrapping_mul(0x100000001b3);
            hash ^= category as u64;
            hash = hash.wrapping_mul(0x100000001b3);
            hash
        };
        if let Some(pattern) = self.patterns.get_mut(&pattern_key) {
            pattern.record_recovery(succeeded);
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.total_errors = self.total_errors;
        self.stats.unique_patterns = self.patterns.len();
        self.stats.recovery_attempts = self.recovery_attempts;
        self.stats.recovery_successes = self.recovery_successes;
        self.stats.recovery_rate = if self.recovery_attempts > 0 {
            self.recovery_successes as f64 / self.recovery_attempts as f64
        } else {
            0.0
        };
        self.stats.active_bursts = self.patterns.values().filter(|p| p.is_burst()).count();
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeErrorRecoveryStats {
        &self.stats
    }
}
