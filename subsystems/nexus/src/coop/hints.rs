//! # Hint Bus — Bidirectional Communication Channel
//!
//! Application → Kernel hints and Kernel → Application advisories.
//! The HintBus is the central message broker for cooperative communication.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// PRESSURE LEVELS (used by kernel advisories)
// ============================================================================

/// System pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PressureLevel {
    /// No pressure
    None,
    /// Low pressure — informational
    Low,
    /// Medium pressure — consider reducing usage
    Medium,
    /// High pressure — should reduce usage
    High,
    /// Critical pressure — must reduce usage immediately
    Critical,
}

impl PressureLevel {
    /// Numeric severity (0-100)
    #[inline]
    pub fn severity(&self) -> u8 {
        match self {
            Self::None => 0,
            Self::Low => 25,
            Self::Medium => 50,
            Self::High => 75,
            Self::Critical => 100,
        }
    }
}

// ============================================================================
// APPLICATION HINTS (App → Kernel)
// ============================================================================

/// Type of hint an application can provide
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppHintType {
    /// App expects a burst of CPU usage
    CpuBurstExpected,
    /// App is entering idle/sleep mode
    GoingIdle,
    /// App will allocate large memory soon
    LargeAllocationExpected,
    /// App is about to do sequential I/O
    SequentialIoExpected,
    /// App is about to do random I/O
    RandomIoExpected,
    /// App is latency-sensitive for the next period
    LatencySensitive,
    /// App will perform heavy computation
    ComputeIntensive,
    /// App can tolerate degraded resources
    CanDegrade,
    /// App is about to fork/spawn children
    WillFork,
    /// App is about to exit
    WillExit,
    /// App has a real-time deadline
    RealtimeDeadline,
    /// App prefers energy-saving mode
    PreferLowPower,
    /// App expects network burst
    NetworkBurstExpected,
}

/// A hint from application to kernel
#[derive(Debug, Clone)]
pub struct AppHint {
    /// Hint type
    pub hint_type: AppHintType,
    /// Source process ID
    pub pid: u64,
    /// Session ID
    pub session_id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Duration the hint applies for (in ms), 0 = indefinite
    pub duration_ms: u64,
    /// Magnitude/intensity (0.0 - 1.0)
    pub magnitude: f64,
    /// Priority (higher = more important)
    pub priority: u8,
}

impl AppHint {
    pub fn new(hint_type: AppHintType, pid: u64, session_id: u64) -> Self {
        Self {
            hint_type,
            pid,
            session_id,
            timestamp: 0,
            duration_ms: 0,
            magnitude: 0.5,
            priority: 5,
        }
    }

    #[inline(always)]
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    #[inline(always)]
    pub fn with_magnitude(mut self, mag: f64) -> Self {
        self.magnitude = mag.clamp(0.0, 1.0);
        self
    }

    #[inline(always)]
    pub fn with_priority(mut self, prio: u8) -> Self {
        self.priority = prio;
        self
    }

    /// Whether this hint has expired
    #[inline]
    pub fn is_expired(&self, current_time: u64) -> bool {
        if self.duration_ms == 0 {
            return false; // indefinite
        }
        current_time.saturating_sub(self.timestamp) > self.duration_ms
    }
}

// ============================================================================
// KERNEL ADVISORIES (Kernel → App)
// ============================================================================

/// Type of advisory the kernel sends to applications
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelAdvisoryType {
    /// Memory pressure advisory
    MemoryPressure,
    /// CPU contention advisory
    CpuContention,
    /// I/O congestion advisory
    IoCongestion,
    /// Power state change advisory
    PowerStateChange,
    /// Thermal throttling advisory
    ThermalThrottle,
    /// Network congestion advisory
    NetworkCongestion,
    /// Scheduling priority change advisory
    PriorityChange,
    /// Resource quota warning
    QuotaWarning,
    /// System entering low-power mode
    LowPowerMode,
    /// System waking from sleep
    WakeFromSleep,
}

/// A kernel advisory sent to an application
#[derive(Debug, Clone)]
pub struct KernelAdvisory {
    /// Advisory type
    pub advisory_type: KernelAdvisoryType,
    /// Target process ID
    pub target_pid: u64,
    /// Session ID
    pub session_id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Pressure level
    pub pressure: PressureLevel,
    /// Recommended action magnitude (0.0 - 1.0)
    pub recommended_reduction: f64,
    /// Deadline to respond (ms), 0 = no deadline
    pub response_deadline_ms: u64,
}

impl KernelAdvisory {
    pub fn new(
        advisory_type: KernelAdvisoryType,
        target_pid: u64,
        session_id: u64,
        pressure: PressureLevel,
    ) -> Self {
        Self {
            advisory_type,
            target_pid,
            session_id,
            timestamp: 0,
            pressure,
            recommended_reduction: pressure.severity() as f64 / 100.0,
            response_deadline_ms: match pressure {
                PressureLevel::None | PressureLevel::Low => 0,
                PressureLevel::Medium => 5000,
                PressureLevel::High => 1000,
                PressureLevel::Critical => 100,
            },
        }
    }
}

// ============================================================================
// HINT BUS — Central message broker
// ============================================================================

const MAX_PENDING_HINTS: usize = 256;
const MAX_PENDING_ADVISORIES: usize = 256;

/// The central hint bus for cooperative communication
pub struct HintBus {
    /// Pending app hints (app → kernel)
    hints: VecDeque<AppHint>,
    /// Pending kernel advisories (kernel → app)
    advisories: VecDeque<KernelAdvisory>,
    /// Total hints received
    total_hints: u64,
    /// Total advisories sent
    total_advisories: u64,
    /// Hints dropped due to overflow
    hints_dropped: u64,
    /// Advisories dropped due to overflow
    advisories_dropped: u64,
}

impl HintBus {
    pub fn new() -> Self {
        Self {
            hints: VecDeque::new(),
            advisories: VecDeque::new(),
            total_hints: 0,
            total_advisories: 0,
            hints_dropped: 0,
            advisories_dropped: 0,
        }
    }

    /// Submit a hint from an application
    pub fn submit_hint(&mut self, hint: AppHint) -> bool {
        self.total_hints += 1;
        if self.hints.len() >= MAX_PENDING_HINTS {
            // Drop lowest priority hint
            if let Some(min_idx) = self
                .hints
                .iter()
                .enumerate()
                .min_by_key(|(_, h)| h.priority)
                .map(|(i, _)| i)
            {
                if self.hints[min_idx].priority < hint.priority {
                    self.hints.remove(min_idx);
                    self.hints_dropped += 1;
                } else {
                    self.hints_dropped += 1;
                    return false;
                }
            }
        }
        self.hints.push_back(hint);
        true
    }

    /// Submit a kernel advisory
    #[inline]
    pub fn submit_advisory(&mut self, advisory: KernelAdvisory) -> bool {
        self.total_advisories += 1;
        if self.advisories.len() >= MAX_PENDING_ADVISORIES {
            // Drop oldest advisory
            self.advisories.pop_front();
            self.advisories_dropped += 1;
        }
        self.advisories.push_back(advisory);
        true
    }

    /// Drain all pending hints
    #[inline(always)]
    pub fn drain_hints(&mut self) -> Vec<AppHint> {
        self.hints.drain(..).collect()
    }

    /// Drain advisories for a specific PID
    pub fn drain_advisories_for(&mut self, pid: u64) -> Vec<KernelAdvisory> {
        let mut result = Vec::new();
        let mut remaining = VecDeque::new();
        for adv in self.advisories.drain(..) {
            if adv.target_pid == pid {
                result.push(adv);
            } else {
                remaining.push_back(adv);
            }
        }
        self.advisories = remaining;
        result
    }

    /// Get pending hint count
    #[inline(always)]
    pub fn pending_hints(&self) -> usize {
        self.hints.len()
    }

    /// Get pending advisory count
    #[inline(always)]
    pub fn pending_advisories(&self) -> usize {
        self.advisories.len()
    }

    /// Purge expired hints
    #[inline]
    pub fn purge_expired(&mut self, current_time: u64) -> usize {
        let before = self.hints.len();
        self.hints.retain(|h| !h.is_expired(current_time));
        before - self.hints.len()
    }

    /// Get bus statistics
    #[inline]
    pub fn stats(&self) -> (u64, u64, u64, u64) {
        (
            self.total_hints,
            self.total_advisories,
            self.hints_dropped,
            self.advisories_dropped,
        )
    }
}
