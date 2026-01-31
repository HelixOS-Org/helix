//! Perf Events and Configuration
//!
//! Perf event configuration and state management.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{CpuId, EventId, EventType, PmuId};

// ============================================================================
// EVENT CONFIGURATION
// ============================================================================

/// Event configuration
#[derive(Debug, Clone)]
pub struct EventConfig {
    /// Event type
    pub event_type: EventType,
    /// Period (samples after N events)
    pub period: u64,
    /// Frequency (samples per second)
    pub frequency: u32,
    /// Exclude user
    pub exclude_user: bool,
    /// Exclude kernel
    pub exclude_kernel: bool,
    /// Exclude hypervisor
    pub exclude_hv: bool,
    /// Exclude idle
    pub exclude_idle: bool,
    /// Inherit to children
    pub inherit: bool,
    /// Pinned (always on)
    pub pinned: bool,
}

impl EventConfig {
    /// Create new config
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            period: 0,
            frequency: 0,
            exclude_user: false,
            exclude_kernel: false,
            exclude_hv: false,
            exclude_idle: false,
            inherit: false,
            pinned: false,
        }
    }

    /// With sampling period
    pub fn with_period(mut self, period: u64) -> Self {
        self.period = period;
        self
    }

    /// With sampling frequency
    pub fn with_frequency(mut self, frequency: u32) -> Self {
        self.frequency = frequency;
        self
    }

    /// Kernel only
    pub fn kernel_only(mut self) -> Self {
        self.exclude_user = true;
        self.exclude_hv = true;
        self
    }

    /// User only
    pub fn user_only(mut self) -> Self {
        self.exclude_kernel = true;
        self.exclude_hv = true;
        self
    }
}

// ============================================================================
// EVENT STATE
// ============================================================================

/// Perf event state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventState {
    /// Off
    Off,
    /// Active
    Active,
    /// Error
    Error,
}

// ============================================================================
// PERF EVENT
// ============================================================================

/// Perf event
#[derive(Debug)]
pub struct PerfEvent {
    /// Event ID
    pub id: EventId,
    /// Configuration
    pub config: EventConfig,
    /// PMU
    pub pmu: PmuId,
    /// CPU
    pub cpu: Option<CpuId>,
    /// PID
    pub pid: Option<i32>,
    /// State
    pub(crate) state: EventState,
    /// Counter value
    count: AtomicU64,
    /// Time enabled (ns)
    time_enabled: AtomicU64,
    /// Time running (ns)
    time_running: AtomicU64,
    /// Sample count
    sample_count: AtomicU64,
}

impl PerfEvent {
    /// Create new event
    pub fn new(id: EventId, config: EventConfig, pmu: PmuId) -> Self {
        Self {
            id,
            config,
            pmu,
            cpu: None,
            pid: None,
            state: EventState::Off,
            count: AtomicU64::new(0),
            time_enabled: AtomicU64::new(0),
            time_running: AtomicU64::new(0),
            sample_count: AtomicU64::new(0),
        }
    }

    /// Get count
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Update count
    pub fn update_count(&self, value: u64) {
        self.count.store(value, Ordering::Relaxed);
    }

    /// Add to count
    pub fn add_count(&self, delta: u64) {
        self.count.fetch_add(delta, Ordering::Relaxed);
    }

    /// Time enabled
    pub fn time_enabled(&self) -> u64 {
        self.time_enabled.load(Ordering::Relaxed)
    }

    /// Time running
    pub fn time_running(&self) -> u64 {
        self.time_running.load(Ordering::Relaxed)
    }

    /// Multiplexing ratio
    pub fn mux_ratio(&self) -> f64 {
        let enabled = self.time_enabled();
        let running = self.time_running();
        if running == 0 {
            return 0.0;
        }
        enabled as f64 / running as f64
    }

    /// Scaled count (accounting for multiplexing)
    pub fn scaled_count(&self) -> u64 {
        let count = self.count();
        let ratio = self.mux_ratio();
        if ratio == 0.0 {
            return count;
        }
        (count as f64 * ratio) as u64
    }

    /// Is running
    pub fn is_running(&self) -> bool {
        self.state == EventState::Active
    }

    /// Start event
    pub fn start(&mut self) {
        self.state = EventState::Active;
    }

    /// Stop event
    pub fn stop(&mut self) {
        self.state = EventState::Off;
    }
}

// ============================================================================
// SAMPLE TYPES
// ============================================================================

/// Sample type flags
#[derive(Debug, Clone, Copy)]
pub struct SampleType(pub u64);

impl SampleType {
    /// IP (instruction pointer)
    pub const IP: u64 = 1 << 0;
    /// TID (thread ID)
    pub const TID: u64 = 1 << 1;
    /// Time
    pub const TIME: u64 = 1 << 2;
    /// Address
    pub const ADDR: u64 = 1 << 3;
    /// Read values
    pub const READ: u64 = 1 << 4;
    /// Callchain
    pub const CALLCHAIN: u64 = 1 << 5;
    /// ID
    pub const ID: u64 = 1 << 6;
    /// CPU
    pub const CPU: u64 = 1 << 7;
    /// Period
    pub const PERIOD: u64 = 1 << 8;
    /// Branch stack
    pub const BRANCH_STACK: u64 = 1 << 11;
    /// Registers (user)
    pub const REGS_USER: u64 = 1 << 12;
    /// Stack user
    pub const STACK_USER: u64 = 1 << 13;
    /// Weight
    pub const WEIGHT: u64 = 1 << 14;
    /// Data source
    pub const DATA_SRC: u64 = 1 << 15;
}

/// Sample
#[derive(Debug, Clone)]
pub struct Sample {
    /// IP
    pub ip: Option<u64>,
    /// PID
    pub pid: Option<i32>,
    /// TID
    pub tid: Option<i32>,
    /// Time
    pub time: Option<u64>,
    /// Address
    pub addr: Option<u64>,
    /// Period
    pub period: Option<u64>,
    /// CPU
    pub cpu: Option<CpuId>,
    /// Callchain
    pub callchain: Vec<u64>,
    /// Weight
    pub weight: Option<u64>,
}

impl Sample {
    /// Create new sample
    pub fn new() -> Self {
        Self {
            ip: None,
            pid: None,
            tid: None,
            time: None,
            addr: None,
            period: None,
            cpu: None,
            callchain: Vec::new(),
            weight: None,
        }
    }
}

impl Default for Sample {
    fn default() -> Self {
        Self::new()
    }
}
