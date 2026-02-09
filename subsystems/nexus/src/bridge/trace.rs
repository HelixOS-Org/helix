//! # Syscall Tracing Engine
//!
//! Low-overhead syscall tracing and profiling:
//! - Per-process trace sessions
//! - Selective tracing (by syscall, PID, pattern)
//! - Ring buffer trace storage
//! - Trace aggregation and analysis
//! - Latency distribution tracking
//! - Call graph construction
//! - Trace export

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TRACE EVENTS
// ============================================================================

/// Trace event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEventType {
    /// Syscall entry
    SyscallEntry,
    /// Syscall exit
    SyscallExit,
    /// Syscall error
    SyscallError,
    /// Throttled
    Throttled,
    /// Coalesced
    Coalesced,
    /// Cached result used
    CacheHit,
    /// Route decision
    RouteDecision,
    /// Validation failure
    ValidationFailure,
    /// Context switch during syscall
    ContextSwitch,
    /// Syscall blocked
    Blocked,
    /// Syscall resumed
    Resumed,
}

/// Single trace event
#[derive(Debug, Clone)]
pub struct TraceEvent {
    /// Event type
    pub event_type: TraceEventType,
    /// Timestamp (nanoseconds)
    pub timestamp_ns: u64,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// CPU ID
    pub cpu_id: u32,
    /// Arguments (entry) or return value (exit)
    pub data: [u64; 4],
    /// Duration (for exit events, nanoseconds)
    pub duration_ns: u64,
}

impl TraceEvent {
    pub fn entry(pid: u64, tid: u64, syscall_nr: u32, cpu: u32, ts: u64, args: [u64; 4]) -> Self {
        Self {
            event_type: TraceEventType::SyscallEntry,
            timestamp_ns: ts,
            pid,
            tid,
            syscall_nr,
            cpu_id: cpu,
            data: args,
            duration_ns: 0,
        }
    }

    pub fn exit(
        pid: u64,
        tid: u64,
        syscall_nr: u32,
        cpu: u32,
        ts: u64,
        ret: i64,
        dur: u64,
    ) -> Self {
        Self {
            event_type: TraceEventType::SyscallExit,
            timestamp_ns: ts,
            pid,
            tid,
            syscall_nr,
            cpu_id: cpu,
            data: [ret as u64, dur, 0, 0],
            duration_ns: dur,
        }
    }
}

// ============================================================================
// TRACE FILTER
// ============================================================================

/// Trace filter
#[derive(Debug, Clone)]
pub struct TraceFilter {
    /// PIDs to trace (empty = all)
    pub pids: Vec<u64>,
    /// Syscalls to trace (empty = all)
    pub syscalls: Vec<u32>,
    /// Event types to trace (empty = all)
    pub event_types: Vec<TraceEventType>,
    /// Minimum duration to capture (nanoseconds, 0 = all)
    pub min_duration_ns: u64,
    /// Sample rate (1 = every call, N = 1 in N)
    pub sample_rate: u32,
}

impl Default for TraceFilter {
    fn default() -> Self {
        Self {
            pids: Vec::new(),
            syscalls: Vec::new(),
            event_types: Vec::new(),
            min_duration_ns: 0,
            sample_rate: 1,
        }
    }
}

impl TraceFilter {
    /// Check if event passes filter
    pub fn matches(&self, event: &TraceEvent) -> bool {
        if !self.pids.is_empty() && !self.pids.contains(&event.pid) {
            return false;
        }
        if !self.syscalls.is_empty() && !self.syscalls.contains(&event.syscall_nr) {
            return false;
        }
        if !self.event_types.is_empty() && !self.event_types.contains(&event.event_type) {
            return false;
        }
        if self.min_duration_ns > 0 && event.duration_ns < self.min_duration_ns {
            return false;
        }
        true
    }
}

// ============================================================================
// RING BUFFER
// ============================================================================

/// Ring buffer for trace events
#[repr(align(64))]
pub struct TraceRingBuffer {
    /// Events
    events: Vec<TraceEvent>,
    /// Write position
    write_pos: usize,
    /// Capacity
    capacity: usize,
    /// Total written (including overwrites)
    pub total_written: u64,
    /// Overflows (events lost due to wrapping)
    pub overflows: u64,
    /// Is full (at least one wrap)
    wrapped: bool,
}

impl TraceRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            write_pos: 0,
            capacity,
            total_written: 0,
            overflows: 0,
            wrapped: false,
        }
    }

    /// Write event to buffer
    pub fn write(&mut self, event: TraceEvent) {
        self.total_written += 1;

        if self.events.len() < self.capacity {
            self.events.push(event);
            self.write_pos = self.events.len();
        } else {
            let pos = self.write_pos % self.capacity;
            self.events[pos] = event;
            self.write_pos = pos + 1;
            if !self.wrapped {
                self.wrapped = true;
            }
            self.overflows += 1;
        }
    }

    /// Read all events in order
    pub fn read_all(&self) -> Vec<&TraceEvent> {
        if !self.wrapped {
            self.events.iter().collect()
        } else {
            let pos = self.write_pos % self.capacity;
            let mut result = Vec::with_capacity(self.events.len());
            for i in 0..self.events.len() {
                let idx = (pos + i) % self.events.len();
                result.push(&self.events[idx]);
            }
            result
        }
    }

    /// Read recent N events
    #[inline]
    pub fn read_recent(&self, n: usize) -> Vec<&TraceEvent> {
        let all = self.read_all();
        let start = all.len().saturating_sub(n);
        all[start..].to_vec()
    }

    /// Clear buffer
    #[inline]
    pub fn clear(&mut self) {
        self.events.clear();
        self.write_pos = 0;
        self.wrapped = false;
    }

    /// Event count
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

// ============================================================================
// LATENCY HISTOGRAM
// ============================================================================

/// Latency distribution tracker
#[derive(Debug, Clone)]
pub struct LatencyHistogram {
    /// Buckets (upper bound in ns -> count)
    buckets: Vec<(u64, u64)>,
    /// Total count
    pub count: u64,
    /// Sum of all values
    pub sum_ns: u64,
    /// Min
    pub min_ns: u64,
    /// Max
    pub max_ns: u64,
}

impl LatencyHistogram {
    /// Create with default buckets
    pub fn new() -> Self {
        let bucket_bounds = [
            100,         // 100ns
            500,         // 500ns
            1_000,       // 1us
            5_000,       // 5us
            10_000,      // 10us
            50_000,      // 50us
            100_000,     // 100us
            500_000,     // 500us
            1_000_000,   // 1ms
            5_000_000,   // 5ms
            10_000_000,  // 10ms
            50_000_000,  // 50ms
            100_000_000, // 100ms
            u64::MAX,    // overflow
        ];

        Self {
            buckets: bucket_bounds.iter().map(|&b| (b, 0)).collect(),
            count: 0,
            sum_ns: 0,
            min_ns: u64::MAX,
            max_ns: 0,
        }
    }

    /// Record a latency
    pub fn record(&mut self, latency_ns: u64) {
        self.count += 1;
        self.sum_ns += latency_ns;
        if latency_ns < self.min_ns {
            self.min_ns = latency_ns;
        }
        if latency_ns > self.max_ns {
            self.max_ns = latency_ns;
        }

        for bucket in &mut self.buckets {
            if latency_ns <= bucket.0 {
                bucket.1 += 1;
                break;
            }
        }
    }

    /// Average latency
    #[inline]
    pub fn avg_ns(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum_ns as f64 / self.count as f64
    }

    /// Approximate percentile
    #[inline]
    pub fn percentile(&self, p: f64) -> u64 {
        let target = (self.count as f64 * p / 100.0) as u64;
        let mut cumulative = 0u64;
        for &(bound, count) in &self.buckets {
            cumulative += count;
            if cumulative >= target {
                return bound;
            }
        }
        self.max_ns
    }
}

// ============================================================================
// TRACE AGGREGATION
// ============================================================================

/// Per-syscall trace summary
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SyscallTraceSummary {
    /// Syscall number
    pub syscall_nr: u32,
    /// Call count
    pub count: u64,
    /// Error count
    pub errors: u64,
    /// Latency histogram
    pub latency: LatencyHistogram,
    /// By process
    pub by_pid: LinearMap<u64, 64>,
}

impl SyscallTraceSummary {
    pub fn new(syscall_nr: u32) -> Self {
        Self {
            syscall_nr,
            count: 0,
            errors: 0,
            latency: LatencyHistogram::new(),
            by_pid: LinearMap::new(),
        }
    }
}

// ============================================================================
// TRACE SESSION
// ============================================================================

/// Trace session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Created but not started
    Created,
    /// Actively tracing
    Active,
    /// Paused
    Paused,
    /// Stopped
    Stopped,
}

/// A trace session
#[repr(align(64))]
pub struct BridgeTraceSession {
    /// Session ID
    pub id: u32,
    /// State
    pub state: SessionState,
    /// Filter
    pub filter: TraceFilter,
    /// Ring buffer
    buffer: TraceRingBuffer,
    /// Start time
    pub started_at: u64,
    /// Per-syscall summaries
    summaries: BTreeMap<u32, SyscallTraceSummary>,
    /// Sample counter (for rate limiting)
    sample_counter: u64,
}

impl BridgeTraceSession {
    pub fn new(id: u32, filter: TraceFilter, buffer_size: usize) -> Self {
        Self {
            id,
            state: SessionState::Created,
            filter,
            buffer: TraceRingBuffer::new(buffer_size),
            started_at: 0,
            summaries: BTreeMap::new(),
            sample_counter: 0,
        }
    }

    /// Start session
    #[inline(always)]
    pub fn start(&mut self, timestamp: u64) {
        self.state = SessionState::Active;
        self.started_at = timestamp;
    }

    /// Pause session
    #[inline(always)]
    pub fn pause(&mut self) {
        self.state = SessionState::Paused;
    }

    /// Resume session
    #[inline]
    pub fn resume(&mut self) {
        if self.state == SessionState::Paused {
            self.state = SessionState::Active;
        }
    }

    /// Stop session
    #[inline(always)]
    pub fn stop(&mut self) {
        self.state = SessionState::Stopped;
    }

    /// Record event
    pub fn record(&mut self, event: TraceEvent) -> bool {
        if self.state != SessionState::Active {
            return false;
        }

        // Sample rate
        self.sample_counter += 1;
        if self.filter.sample_rate > 1 && self.sample_counter % self.filter.sample_rate as u64 != 0
        {
            return false;
        }

        if !self.filter.matches(&event) {
            return false;
        }

        // Update summary
        let summary = self
            .summaries
            .entry(event.syscall_nr)
            .or_insert_with(|| SyscallTraceSummary::new(event.syscall_nr));

        if event.event_type == TraceEventType::SyscallExit {
            summary.count += 1;
            summary.latency.record(event.duration_ns);
            *summary.by_pid.entry(event.pid).or_insert(0) += 1;
        } else if event.event_type == TraceEventType::SyscallError {
            summary.errors += 1;
        }

        self.buffer.write(event);
        true
    }

    /// Get summary for syscall
    #[inline(always)]
    pub fn summary(&self, syscall_nr: u32) -> Option<&SyscallTraceSummary> {
        self.summaries.get(&syscall_nr)
    }

    /// Get all summaries
    #[inline(always)]
    pub fn all_summaries(&self) -> &BTreeMap<u32, SyscallTraceSummary> {
        &self.summaries
    }

    /// Event count
    #[inline(always)]
    pub fn event_count(&self) -> usize {
        self.buffer.len()
    }

    /// Recent events
    #[inline(always)]
    pub fn recent_events(&self, n: usize) -> Vec<&TraceEvent> {
        self.buffer.read_recent(n)
    }
}

// ============================================================================
// TRACE MANAGER
// ============================================================================

/// Syscall trace manager
#[repr(align(64))]
pub struct BridgeTraceManager {
    /// Active sessions
    sessions: BTreeMap<u32, BridgeTraceSession>,
    /// Next session ID
    next_session_id: u32,
    /// Global stats
    pub total_events: u64,
    /// Global drop count
    pub total_drops: u64,
}

impl BridgeTraceManager {
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            next_session_id: 1,
            total_events: 0,
            total_drops: 0,
        }
    }

    /// Create trace session
    #[inline]
    pub fn create_session(&mut self, filter: TraceFilter, buffer_size: usize) -> u32 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        self.sessions
            .insert(id, BridgeTraceSession::new(id, filter, buffer_size));
        id
    }

    /// Start session
    #[inline]
    pub fn start_session(&mut self, session_id: u32, timestamp: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.start(timestamp);
            true
        } else {
            false
        }
    }

    /// Stop session
    #[inline]
    pub fn stop_session(&mut self, session_id: u32) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.stop();
            true
        } else {
            false
        }
    }

    /// Destroy session
    #[inline(always)]
    pub fn destroy_session(&mut self, session_id: u32) -> bool {
        self.sessions.remove(&session_id).is_some()
    }

    /// Dispatch event to all active sessions
    #[inline]
    pub fn dispatch(&mut self, event: TraceEvent) {
        self.total_events += 1;
        for session in self.sessions.values_mut() {
            session.record(event.clone());
        }
    }

    /// Get session
    #[inline(always)]
    pub fn session(&self, session_id: u32) -> Option<&BridgeTraceSession> {
        self.sessions.get(&session_id)
    }

    /// Active session count
    #[inline]
    pub fn active_sessions(&self) -> usize {
        self.sessions
            .values()
            .filter(|s| s.state == SessionState::Active)
            .count()
    }
}
