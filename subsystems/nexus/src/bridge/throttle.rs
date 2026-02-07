//! # Syscall Throttling Engine
//!
//! Rate limiting and throttling for syscalls:
//! - Per-process rate limits
//! - Per-syscall rate limits
//! - Token bucket algorithm
//! - Sliding window counters
//! - Adaptive throttling based on system load
//! - Priority-aware throttling
//! - Burst allowance management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TOKEN BUCKET
// ============================================================================

/// Token bucket rate limiter
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// Maximum tokens (burst capacity)
    pub capacity: u64,
    /// Current tokens
    pub tokens: u64,
    /// Refill rate (tokens per second)
    pub rate: u64,
    /// Last refill timestamp (milliseconds)
    pub last_refill: u64,
}

impl TokenBucket {
    pub fn new(capacity: u64, rate: u64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            rate,
            last_refill: 0,
        }
    }

    /// Refill tokens based on elapsed time
    pub fn refill(&mut self, current_time_ms: u64) {
        if self.last_refill == 0 {
            self.last_refill = current_time_ms;
            return;
        }

        let elapsed_ms = current_time_ms.saturating_sub(self.last_refill);
        if elapsed_ms == 0 {
            return;
        }

        let new_tokens = self.rate * elapsed_ms / 1000;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = current_time_ms;
    }

    /// Try to consume tokens
    pub fn try_consume(&mut self, tokens: u64, current_time_ms: u64) -> bool {
        self.refill(current_time_ms);
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Available tokens
    pub fn available(&self) -> u64 {
        self.tokens
    }

    /// Time until N tokens available (milliseconds)
    pub fn time_until(&self, tokens: u64) -> u64 {
        if self.tokens >= tokens {
            return 0;
        }
        let needed = tokens - self.tokens;
        if self.rate == 0 {
            return u64::MAX;
        }
        (needed * 1000 + self.rate - 1) / self.rate
    }
}

// ============================================================================
// SLIDING WINDOW
// ============================================================================

/// Sliding window counter
#[derive(Debug, Clone)]
pub struct SlidingWindow {
    /// Window size (milliseconds)
    pub window_ms: u64,
    /// Slots (each represents a sub-window)
    slots: Vec<u64>,
    /// Slot duration (milliseconds)
    slot_ms: u64,
    /// Current slot index
    current_slot: usize,
    /// Last update timestamp
    last_update: u64,
    /// Total in current window
    total: u64,
}

impl SlidingWindow {
    pub fn new(window_ms: u64, num_slots: usize) -> Self {
        let slot_ms = window_ms / num_slots.max(1) as u64;
        Self {
            window_ms,
            slots: alloc::vec![0; num_slots],
            slot_ms: slot_ms.max(1),
            current_slot: 0,
            last_update: 0,
            total: 0,
        }
    }

    /// Advance to current time
    fn advance(&mut self, current_time_ms: u64) {
        if self.last_update == 0 {
            self.last_update = current_time_ms;
            return;
        }

        let elapsed = current_time_ms.saturating_sub(self.last_update);
        let slots_to_advance = (elapsed / self.slot_ms) as usize;

        if slots_to_advance > 0 {
            let num_slots = self.slots.len();
            let clear_count = slots_to_advance.min(num_slots);

            for i in 0..clear_count {
                let idx = (self.current_slot + 1 + i) % num_slots;
                self.total = self.total.saturating_sub(self.slots[idx]);
                self.slots[idx] = 0;
            }

            self.current_slot = (self.current_slot + slots_to_advance) % num_slots;
            self.last_update = current_time_ms;
        }
    }

    /// Record an event
    pub fn record(&mut self, current_time_ms: u64) {
        self.advance(current_time_ms);
        self.slots[self.current_slot] += 1;
        self.total += 1;
    }

    /// Get count in current window
    pub fn count(&mut self, current_time_ms: u64) -> u64 {
        self.advance(current_time_ms);
        self.total
    }

    /// Get rate (events per second)
    pub fn rate(&mut self, current_time_ms: u64) -> f64 {
        let count = self.count(current_time_ms);
        if self.window_ms == 0 {
            return 0.0;
        }
        count as f64 / (self.window_ms as f64 / 1000.0)
    }
}

// ============================================================================
// THROTTLE POLICY
// ============================================================================

/// Throttle reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleReason {
    /// Process rate limit exceeded
    ProcessRateLimit,
    /// Syscall rate limit exceeded
    SyscallRateLimit,
    /// System overloaded
    SystemOverload,
    /// Memory pressure
    MemoryPressure,
    /// I/O congestion
    IoCongestion,
    /// Security throttle
    SecurityThrottle,
    /// Fair share enforcement
    FairShareLimit,
    /// Adaptive throttle
    AdaptiveThrottle,
}

/// Throttle decision
#[derive(Debug, Clone)]
pub struct ThrottleDecision {
    /// Whether to throttle
    pub throttled: bool,
    /// Reason
    pub reason: Option<ThrottleReason>,
    /// Delay to impose (microseconds)
    pub delay_us: u64,
    /// Retry after (milliseconds, 0 = immediate)
    pub retry_after_ms: u64,
}

impl ThrottleDecision {
    pub fn allow() -> Self {
        Self {
            throttled: false,
            reason: None,
            delay_us: 0,
            retry_after_ms: 0,
        }
    }

    pub fn throttle(reason: ThrottleReason, delay_us: u64) -> Self {
        Self {
            throttled: true,
            reason: Some(reason),
            delay_us,
            retry_after_ms: delay_us / 1000,
        }
    }
}

// ============================================================================
// THROTTLE CONFIG
// ============================================================================

/// Per-process throttle config
#[derive(Debug, Clone)]
pub struct ProcessThrottleConfig {
    /// Max syscalls per second
    pub max_rate: u64,
    /// Burst allowance
    pub burst_size: u64,
    /// Priority (higher = less likely to be throttled)
    pub priority: u32,
    /// Adaptive throttle enabled
    pub adaptive: bool,
}

impl Default for ProcessThrottleConfig {
    fn default() -> Self {
        Self {
            max_rate: 10_000,
            burst_size: 100,
            priority: 50,
            adaptive: true,
        }
    }
}

/// Per-syscall throttle config
#[derive(Debug, Clone)]
pub struct SyscallThrottleConfig {
    /// Max calls per second (global)
    pub max_global_rate: u64,
    /// Max calls per second per process
    pub max_per_process_rate: u64,
    /// Cost weight (1 = normal, higher = more expensive)
    pub cost_weight: u32,
}

impl Default for SyscallThrottleConfig {
    fn default() -> Self {
        Self {
            max_global_rate: 100_000,
            max_per_process_rate: 5_000,
            cost_weight: 1,
        }
    }
}

// ============================================================================
// THROTTLE STATE
// ============================================================================

/// Per-process throttle state
struct ProcessThrottleState {
    /// Token bucket for rate limiting
    bucket: TokenBucket,
    /// Sliding window for monitoring
    window: SlidingWindow,
    /// Config
    config: ProcessThrottleConfig,
    /// Times throttled
    throttle_count: u64,
    /// Last throttle time
    last_throttled: u64,
}

/// Per-syscall throttle state
struct SyscallThrottleState {
    /// Global sliding window
    global_window: SlidingWindow,
    /// Per-process windows
    process_windows: BTreeMap<u64, SlidingWindow>,
    /// Config
    config: SyscallThrottleConfig,
}

// ============================================================================
// THROTTLE ENGINE
// ============================================================================

/// Throttle statistics
#[derive(Debug, Clone, Default)]
pub struct ThrottleStats {
    /// Total checks
    pub total_checks: u64,
    /// Total throttled
    pub total_throttled: u64,
    /// By reason
    pub by_reason: BTreeMap<u8, u64>,
    /// Total delay imposed (microseconds)
    pub total_delay_us: u64,
}

/// Syscall throttling engine
pub struct ThrottleEngine {
    /// Per-process state
    process_state: BTreeMap<u64, ProcessThrottleState>,
    /// Per-syscall state
    syscall_state: BTreeMap<u32, SyscallThrottleState>,
    /// System load factor (0-100)
    pub system_load: u32,
    /// Memory pressure level (0-100)
    pub memory_pressure: u32,
    /// Statistics
    pub stats: ThrottleStats,
    /// Enabled
    pub enabled: bool,
}

impl ThrottleEngine {
    pub fn new() -> Self {
        Self {
            process_state: BTreeMap::new(),
            syscall_state: BTreeMap::new(),
            system_load: 0,
            memory_pressure: 0,
            stats: ThrottleStats::default(),
            enabled: true,
        }
    }

    /// Register process with throttle config
    pub fn register_process(&mut self, pid: u64, config: ProcessThrottleConfig) {
        let state = ProcessThrottleState {
            bucket: TokenBucket::new(config.burst_size, config.max_rate),
            window: SlidingWindow::new(1000, 10), // 1s window, 10 slots
            config,
            throttle_count: 0,
            last_throttled: 0,
        };
        self.process_state.insert(pid, state);
    }

    /// Register syscall throttle config
    pub fn register_syscall(&mut self, syscall_nr: u32, config: SyscallThrottleConfig) {
        let state = SyscallThrottleState {
            global_window: SlidingWindow::new(1000, 10),
            process_windows: BTreeMap::new(),
            config,
        };
        self.syscall_state.insert(syscall_nr, state);
    }

    /// Check if syscall should be throttled
    pub fn check(&mut self, pid: u64, syscall_nr: u32, current_time_ms: u64) -> ThrottleDecision {
        if !self.enabled {
            return ThrottleDecision::allow();
        }

        self.stats.total_checks += 1;

        // Check process rate limit
        if let Some(state) = self.process_state.get_mut(&pid) {
            if !state.bucket.try_consume(1, current_time_ms) {
                state.throttle_count += 1;
                state.last_throttled = current_time_ms;
                let delay = state.bucket.time_until(1) * 1000; // Convert to us
                self.record_throttle(ThrottleReason::ProcessRateLimit, delay);
                return ThrottleDecision::throttle(ThrottleReason::ProcessRateLimit, delay);
            }
            state.window.record(current_time_ms);
        }

        // Check per-syscall global rate
        if let Some(state) = self.syscall_state.get_mut(&syscall_nr) {
            state.global_window.record(current_time_ms);
            let global_rate = state.global_window.rate(current_time_ms);

            if global_rate > state.config.max_global_rate as f64 {
                let delay = 1000u64; // 1ms delay
                self.record_throttle(ThrottleReason::SyscallRateLimit, delay);
                return ThrottleDecision::throttle(ThrottleReason::SyscallRateLimit, delay);
            }

            // Check per-process rate for this syscall
            let process_window = state
                .process_windows
                .entry(pid)
                .or_insert_with(|| SlidingWindow::new(1000, 10));
            process_window.record(current_time_ms);
            let proc_rate = process_window.rate(current_time_ms);

            if proc_rate > state.config.max_per_process_rate as f64 {
                let delay = 2000u64; // 2ms delay
                self.record_throttle(ThrottleReason::SyscallRateLimit, delay);
                return ThrottleDecision::throttle(ThrottleReason::SyscallRateLimit, delay);
            }
        }

        // Adaptive throttling based on system state
        if self.system_load > 90 {
            // Under heavy load, throttle low-priority processes
            let priority = self
                .process_state
                .get(&pid)
                .map(|s| s.config.priority)
                .unwrap_or(50);

            if priority < 30 {
                let delay = (self.system_load as u64 - 80) * 500;
                self.record_throttle(ThrottleReason::SystemOverload, delay);
                return ThrottleDecision::throttle(ThrottleReason::SystemOverload, delay);
            }
        }

        if self.memory_pressure > 80 {
            let priority = self
                .process_state
                .get(&pid)
                .map(|s| s.config.priority)
                .unwrap_or(50);

            if priority < 50 {
                let delay = (self.memory_pressure as u64 - 70) * 1000;
                self.record_throttle(ThrottleReason::MemoryPressure, delay);
                return ThrottleDecision::throttle(ThrottleReason::MemoryPressure, delay);
            }
        }

        ThrottleDecision::allow()
    }

    /// Record throttle
    fn record_throttle(&mut self, reason: ThrottleReason, delay_us: u64) {
        self.stats.total_throttled += 1;
        self.stats.total_delay_us += delay_us;
        *self.stats.by_reason.entry(reason as u8).or_insert(0) += 1;
    }

    /// Unregister process
    pub fn unregister_process(&mut self, pid: u64) {
        self.process_state.remove(&pid);
        for state in self.syscall_state.values_mut() {
            state.process_windows.remove(&pid);
        }
    }

    /// Update system state
    pub fn update_system_state(&mut self, cpu_load: u32, mem_pressure: u32) {
        self.system_load = cpu_load;
        self.memory_pressure = mem_pressure;
    }

    /// Throttle rate (percent)
    pub fn throttle_rate(&self) -> f64 {
        if self.stats.total_checks == 0 {
            return 0.0;
        }
        self.stats.total_throttled as f64 / self.stats.total_checks as f64 * 100.0
    }
}
