//! # Bridge Instrumentation Engine
//!
//! Dynamic syscall instrumentation:
//! - Probe points insertion
//! - Tracepoint management
//! - Performance counters
//! - Event filtering
//! - Probe callback registration

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// INSTRUMENTATION TYPES
// ============================================================================

/// Probe type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeType {
    /// Entry probe (before syscall)
    Entry,
    /// Exit probe (after syscall)
    Exit,
    /// Error probe (on error)
    Error,
    /// Latency probe (timing)
    Latency,
    /// Argument probe (inspect args)
    Argument,
    /// Return value probe
    ReturnValue,
}

/// Probe state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeState {
    /// Disabled
    Disabled,
    /// Active
    Active,
    /// Paused
    Paused,
    /// Errored
    Errored,
}

/// Filter operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    /// Equal
    Equal,
    /// Not equal
    NotEqual,
    /// Greater than
    GreaterThan,
    /// Less than
    LessThan,
    /// Bitwise AND non-zero
    BitAnd,
}

/// Event filter
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Field to filter on
    pub field: FilterField,
    /// Operation
    pub op: FilterOp,
    /// Value to compare
    pub value: u64,
}

/// Filter field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterField {
    /// Syscall number
    SyscallNr,
    /// Process ID
    Pid,
    /// Argument N
    Arg(u8),
    /// Return value
    ReturnValue,
    /// Latency (ns)
    Latency,
}

impl EventFilter {
    /// Check if event matches
    pub fn matches(&self, actual: u64) -> bool {
        match self.op {
            FilterOp::Equal => actual == self.value,
            FilterOp::NotEqual => actual != self.value,
            FilterOp::GreaterThan => actual > self.value,
            FilterOp::LessThan => actual < self.value,
            FilterOp::BitAnd => (actual & self.value) != 0,
        }
    }
}

// ============================================================================
// PROBE
// ============================================================================

/// Probe definition
#[derive(Debug, Clone)]
pub struct InstrumentationProbe {
    /// Probe id
    pub id: u64,
    /// Type
    pub probe_type: ProbeType,
    /// Target syscall (None = all)
    pub target_syscall: Option<u32>,
    /// Target process (None = all)
    pub target_pid: Option<u64>,
    /// State
    pub state: ProbeState,
    /// Filters
    pub filters: Vec<EventFilter>,
    /// Hit count
    pub hits: u64,
    /// Last hit time
    pub last_hit: u64,
    /// Sample rate (1 = every, N = every Nth)
    pub sample_rate: u32,
    /// Sample counter
    sample_counter: u32,
}

impl InstrumentationProbe {
    pub fn new(id: u64, probe_type: ProbeType) -> Self {
        Self {
            id,
            probe_type,
            target_syscall: None,
            target_pid: None,
            state: ProbeState::Active,
            filters: Vec::new(),
            hits: 0,
            last_hit: 0,
            sample_rate: 1,
            sample_counter: 0,
        }
    }

    /// Add filter
    pub fn add_filter(&mut self, filter: EventFilter) {
        self.filters.push(filter);
    }

    /// Set target syscall
    pub fn for_syscall(mut self, nr: u32) -> Self {
        self.target_syscall = Some(nr);
        self
    }

    /// Set target pid
    pub fn for_pid(mut self, pid: u64) -> Self {
        self.target_pid = Some(pid);
        self
    }

    /// Check if probe should fire
    pub fn should_fire(&mut self, syscall_nr: u32, pid: u64) -> bool {
        if !matches!(self.state, ProbeState::Active) {
            return false;
        }

        if let Some(target) = self.target_syscall {
            if target != syscall_nr {
                return false;
            }
        }

        if let Some(target_pid) = self.target_pid {
            if target_pid != pid {
                return false;
            }
        }

        // Sampling
        self.sample_counter += 1;
        if self.sample_counter < self.sample_rate {
            return false;
        }
        self.sample_counter = 0;

        true
    }

    /// Record hit
    pub fn record_hit(&mut self, now: u64) {
        self.hits += 1;
        self.last_hit = now;
    }

    /// Enable
    pub fn enable(&mut self) {
        self.state = ProbeState::Active;
    }

    /// Disable
    pub fn disable(&mut self) {
        self.state = ProbeState::Disabled;
    }

    /// Pause
    pub fn pause(&mut self) {
        self.state = ProbeState::Paused;
    }
}

// ============================================================================
// PERF COUNTER
// ============================================================================

/// Performance counter type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PerfCounterType {
    /// Syscall count
    SyscallCount,
    /// Error count
    ErrorCount,
    /// Total latency
    TotalLatency,
    /// Cache hits
    CacheHits,
    /// Cache misses
    CacheMisses,
    /// Retries
    Retries,
    /// Batched calls
    BatchedCalls,
    /// Context switches
    ContextSwitches,
}

/// Performance counter
#[derive(Debug, Clone)]
pub struct PerfCounter {
    /// Type
    pub counter_type: PerfCounterType,
    /// Value
    pub value: u64,
    /// Previous snapshot value
    prev_value: u64,
    /// Snapshot time
    prev_time: u64,
}

impl PerfCounter {
    pub fn new(counter_type: PerfCounterType) -> Self {
        Self {
            counter_type,
            value: 0,
            prev_value: 0,
            prev_time: 0,
        }
    }

    /// Increment
    pub fn increment(&mut self, amount: u64) {
        self.value += amount;
    }

    /// Rate since last snapshot
    pub fn rate(&self, now: u64) -> f64 {
        let dt = now.saturating_sub(self.prev_time);
        if dt == 0 {
            return 0.0;
        }
        let dv = self.value.saturating_sub(self.prev_value);
        dv as f64 / (dt as f64 / 1_000_000_000.0)
    }

    /// Take snapshot
    pub fn snapshot(&mut self, now: u64) {
        self.prev_value = self.value;
        self.prev_time = now;
    }

    /// Delta since snapshot
    pub fn delta(&self) -> u64 {
        self.value.saturating_sub(self.prev_value)
    }
}

// ============================================================================
// INSTRUMENTATION ENGINE
// ============================================================================

/// Instrumentation event
#[derive(Debug, Clone)]
pub struct InstrumentationEvent {
    /// Probe id
    pub probe_id: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Process id
    pub pid: u64,
    /// Arguments (up to 6)
    pub args: [u64; 6],
    /// Return value
    pub return_value: i64,
    /// Latency (ns)
    pub latency_ns: u64,
}

/// Instrumentation stats
#[derive(Debug, Clone, Default)]
pub struct InstrumentationStats {
    /// Active probes
    pub active_probes: usize,
    /// Total probes
    pub total_probes: usize,
    /// Total events
    pub total_events: u64,
    /// Events per second
    pub events_per_sec: f64,
    /// Counters tracked
    pub counters: usize,
}

/// Bridge instrumentation engine
pub struct BridgeInstrumentationEngine {
    /// Probes
    probes: BTreeMap<u64, InstrumentationProbe>,
    /// Perf counters
    counters: BTreeMap<u8, PerfCounter>,
    /// Event ring buffer
    events: Vec<InstrumentationEvent>,
    /// Max events
    max_events: usize,
    /// Next probe id
    next_id: u64,
    /// Stats
    stats: InstrumentationStats,
}

impl BridgeInstrumentationEngine {
    pub fn new() -> Self {
        Self {
            probes: BTreeMap::new(),
            counters: BTreeMap::new(),
            events: Vec::new(),
            max_events: 4096,
            next_id: 1,
            stats: InstrumentationStats::default(),
        }
    }

    /// Add probe
    pub fn add_probe(&mut self, probe_type: ProbeType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let probe = InstrumentationProbe::new(id, probe_type);
        self.probes.insert(id, probe);
        self.update_stats();
        id
    }

    /// Configure probe
    pub fn configure_probe<F: FnOnce(&mut InstrumentationProbe)>(&mut self, id: u64, f: F) {
        if let Some(probe) = self.probes.get_mut(&id) {
            f(probe);
        }
    }

    /// Enable probe
    pub fn enable_probe(&mut self, id: u64) {
        if let Some(p) = self.probes.get_mut(&id) {
            p.enable();
            self.update_stats();
        }
    }

    /// Disable probe
    pub fn disable_probe(&mut self, id: u64) {
        if let Some(p) = self.probes.get_mut(&id) {
            p.disable();
            self.update_stats();
        }
    }

    /// Remove probe
    pub fn remove_probe(&mut self, id: u64) {
        self.probes.remove(&id);
        self.update_stats();
    }

    /// Fire probes for syscall entry
    pub fn fire_entry(
        &mut self,
        syscall_nr: u32,
        pid: u64,
        args: [u64; 6],
        now: u64,
    ) {
        let matching: Vec<u64> = self
            .probes
            .iter_mut()
            .filter(|(_, p)| matches!(p.probe_type, ProbeType::Entry | ProbeType::Argument))
            .filter(|(_, p)| p.should_fire(syscall_nr, pid))
            .map(|(&id, p)| {
                p.record_hit(now);
                id
            })
            .collect();

        for probe_id in matching {
            let event = InstrumentationEvent {
                probe_id,
                timestamp: now,
                syscall_nr,
                pid,
                args,
                return_value: 0,
                latency_ns: 0,
            };
            self.push_event(event);
        }
    }

    /// Fire probes for syscall exit
    pub fn fire_exit(
        &mut self,
        syscall_nr: u32,
        pid: u64,
        return_value: i64,
        latency_ns: u64,
        now: u64,
    ) {
        let matching: Vec<u64> = self
            .probes
            .iter_mut()
            .filter(|(_, p)| {
                matches!(
                    p.probe_type,
                    ProbeType::Exit | ProbeType::Latency | ProbeType::ReturnValue
                )
            })
            .filter(|(_, p)| p.should_fire(syscall_nr, pid))
            .map(|(&id, p)| {
                p.record_hit(now);
                id
            })
            .collect();

        for probe_id in matching {
            let event = InstrumentationEvent {
                probe_id,
                timestamp: now,
                syscall_nr,
                pid,
                args: [0; 6],
                return_value,
                latency_ns,
            };
            self.push_event(event);
        }
    }

    fn push_event(&mut self, event: InstrumentationEvent) {
        self.events.push(event);
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
        self.stats.total_events += 1;
    }

    /// Get counter
    pub fn counter(&mut self, ct: PerfCounterType) -> &mut PerfCounter {
        self.counters
            .entry(ct as u8)
            .or_insert_with(|| PerfCounter::new(ct))
    }

    /// Increment counter
    pub fn increment_counter(&mut self, ct: PerfCounterType, amount: u64) {
        self.counters
            .entry(ct as u8)
            .or_insert_with(|| PerfCounter::new(ct))
            .increment(amount);
        self.stats.counters = self.counters.len();
    }

    /// Recent events
    pub fn recent_events(&self, count: usize) -> &[InstrumentationEvent] {
        if self.events.len() > count {
            &self.events[self.events.len() - count..]
        } else {
            &self.events
        }
    }

    fn update_stats(&mut self) {
        self.stats.total_probes = self.probes.len();
        self.stats.active_probes = self
            .probes
            .values()
            .filter(|p| matches!(p.state, ProbeState::Active))
            .count();
    }

    /// Stats
    pub fn stats(&self) -> &InstrumentationStats {
        &self.stats
    }
}
