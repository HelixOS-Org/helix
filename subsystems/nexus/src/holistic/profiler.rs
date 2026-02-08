//! # Holistic System Profiler
//!
//! Coordinated system-wide profiling:
//! - CPU profiling across all processes
//! - Memory profiling with cross-process analysis
//! - I/O profiling with device correlation
//! - Lock contention profiling
//! - Call graph aggregation
//! - Profile-guided optimization hints

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// PROFILE TYPES
// ============================================================================

/// Profile domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProfileDomain {
    /// CPU time profiling
    Cpu,
    /// Memory allocation profiling
    Memory,
    /// I/O profiling
    Io,
    /// Lock/synchronization profiling
    Lock,
    /// Cache behavior profiling
    Cache,
    /// Branch prediction profiling
    Branch,
    /// Instruction mix profiling
    InstructionMix,
    /// Power consumption profiling
    Power,
}

/// Profile granularity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileGranularity {
    /// System-wide
    System,
    /// Per-CPU
    PerCpu,
    /// Per-process
    PerProcess,
    /// Per-thread
    PerThread,
    /// Per-function
    PerFunction,
}

/// Sample source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleSource {
    /// Timer interrupt
    Timer,
    /// Performance counter overflow
    PerfCounter,
    /// Software event
    SoftwareEvent,
    /// Tracepoint
    Tracepoint,
    /// Manual instrumentation
    Manual,
}

// ============================================================================
// PROFILE SAMPLE
// ============================================================================

/// A profiling sample
#[derive(Debug, Clone)]
pub struct ProfileSample {
    /// Timestamp (ns)
    pub timestamp: u64,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// CPU core
    pub cpu: u32,
    /// Instruction pointer
    pub ip: u64,
    /// Domain
    pub domain: ProfileDomain,
    /// Source
    pub source: SampleSource,
    /// Value (interpretation depends on domain)
    pub value: u64,
    /// Call stack depth (frame IPs)
    pub stack: Vec<u64>,
}

impl ProfileSample {
    pub fn new(
        timestamp: u64,
        pid: u64,
        tid: u64,
        cpu: u32,
        ip: u64,
        domain: ProfileDomain,
        source: SampleSource,
        value: u64,
    ) -> Self {
        Self {
            timestamp,
            pid,
            tid,
            cpu,
            ip,
            domain,
            source,
            value,
            stack: Vec::new(),
        }
    }

    pub fn with_stack(mut self, stack: Vec<u64>) -> Self {
        self.stack = stack;
        self
    }
}

// ============================================================================
// HOTSPOT ANALYSIS
// ============================================================================

/// A hotspot in the system
#[derive(Debug, Clone)]
pub struct Hotspot {
    /// Address
    pub address: u64,
    /// Symbol name (if resolved)
    pub symbol: String,
    /// Domain
    pub domain: ProfileDomain,
    /// Sample count
    pub samples: u64,
    /// Total value
    pub total_value: u64,
    /// Percentage of total
    pub percentage: f64,
    /// Contributing processes
    pub processes: BTreeMap<u64, u64>,
}

impl Hotspot {
    pub fn new(address: u64, domain: ProfileDomain) -> Self {
        Self {
            address,
            symbol: String::new(),
            domain,
            samples: 0,
            total_value: 0,
            percentage: 0.0,
            processes: BTreeMap::new(),
        }
    }

    pub fn with_symbol(mut self, symbol: String) -> Self {
        self.symbol = symbol;
        self
    }

    /// Record sample
    pub fn record(&mut self, pid: u64, value: u64) {
        self.samples += 1;
        self.total_value += value;
        *self.processes.entry(pid).or_insert(0) += 1;
    }

    /// Average value per sample
    pub fn avg_value(&self) -> u64 {
        if self.samples == 0 {
            return 0;
        }
        self.total_value / self.samples
    }

    /// Number of unique processes
    pub fn unique_processes(&self) -> usize {
        self.processes.len()
    }

    /// Is shared hotspot (multiple processes)
    pub fn is_shared(&self) -> bool {
        self.processes.len() > 1
    }
}

// ============================================================================
// LOCK CONTENTION
// ============================================================================

/// Lock contention record
#[derive(Debug, Clone)]
pub struct LockContention {
    /// Lock address
    pub lock_address: u64,
    /// Lock type name
    pub lock_type: String,
    /// Total wait time (ns)
    pub total_wait_ns: u64,
    /// Number of contentions
    pub contentions: u64,
    /// Max wait time (ns)
    pub max_wait_ns: u64,
    /// Waiters by process
    pub waiters: BTreeMap<u64, u64>,
    /// Holder (current)
    pub current_holder: u64,
}

impl LockContention {
    pub fn new(lock_address: u64, lock_type: String) -> Self {
        Self {
            lock_address,
            lock_type,
            total_wait_ns: 0,
            contentions: 0,
            max_wait_ns: 0,
            waiters: BTreeMap::new(),
            current_holder: 0,
        }
    }

    /// Record contention
    pub fn record_contention(&mut self, pid: u64, wait_ns: u64) {
        self.contentions += 1;
        self.total_wait_ns += wait_ns;
        if wait_ns > self.max_wait_ns {
            self.max_wait_ns = wait_ns;
        }
        *self.waiters.entry(pid).or_insert(0) += 1;
    }

    /// Average wait
    pub fn avg_wait_ns(&self) -> u64 {
        if self.contentions == 0 {
            return 0;
        }
        self.total_wait_ns / self.contentions
    }

    /// Number of unique waiters
    pub fn unique_waiters(&self) -> usize {
        self.waiters.len()
    }
}

// ============================================================================
// PROFILE SESSION
// ============================================================================

/// Profile session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSessionState {
    /// Collecting samples
    Active,
    /// Paused
    Paused,
    /// Analysis complete
    Complete,
}

/// A profiling session
#[derive(Debug, Clone)]
pub struct ProfileSession {
    /// Session ID
    pub id: u64,
    /// State
    pub state: ProfileSessionState,
    /// Domains being profiled
    pub domains: Vec<ProfileDomain>,
    /// Granularity
    pub granularity: ProfileGranularity,
    /// Start time
    pub start_time: u64,
    /// End time (0 = ongoing)
    pub end_time: u64,
    /// Sample count
    pub sample_count: u64,
    /// Overhead percentage
    pub overhead_pct: f64,
}

impl ProfileSession {
    pub fn new(
        id: u64,
        domains: Vec<ProfileDomain>,
        granularity: ProfileGranularity,
        start_time: u64,
    ) -> Self {
        Self {
            id,
            state: ProfileSessionState::Active,
            domains,
            granularity,
            start_time,
            end_time: 0,
            sample_count: 0,
            overhead_pct: 0.0,
        }
    }

    pub fn pause(&mut self) {
        if self.state == ProfileSessionState::Active {
            self.state = ProfileSessionState::Paused;
        }
    }

    pub fn resume(&mut self) {
        if self.state == ProfileSessionState::Paused {
            self.state = ProfileSessionState::Active;
        }
    }

    pub fn complete(&mut self, end_time: u64) {
        self.state = ProfileSessionState::Complete;
        self.end_time = end_time;
    }

    pub fn duration_ns(&self) -> u64 {
        if self.end_time > 0 {
            self.end_time.saturating_sub(self.start_time)
        } else {
            0
        }
    }
}

// ============================================================================
// OPTIMIZATION HINT
// ============================================================================

/// PGO hint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationHintType {
    /// Hot function (should be optimized)
    HotFunction,
    /// Cold function (can be optimized for size)
    ColdFunction,
    /// Should be inlined
    InlineCandidate,
    /// Should use prefetch
    PrefetchCandidate,
    /// Lock should be split
    LockSplit,
    /// Should use lock-free structure
    LockFree,
    /// Cache line alignment needed
    CacheLineAlign,
    /// NUMA-local allocation needed
    NumaLocal,
}

/// Optimization hint
#[derive(Debug, Clone)]
pub struct OptimizationHint {
    /// Hint type
    pub hint_type: OptimizationHintType,
    /// Target address
    pub address: u64,
    /// Symbol
    pub symbol: String,
    /// Confidence (0.0-1.0)
    pub confidence: f64,
    /// Expected improvement (0.0-1.0)
    pub expected_improvement: f64,
    /// Description
    pub description: String,
}

impl OptimizationHint {
    pub fn new(
        hint_type: OptimizationHintType,
        address: u64,
        confidence: f64,
        expected_improvement: f64,
    ) -> Self {
        Self {
            hint_type,
            address,
            symbol: String::new(),
            confidence,
            expected_improvement,
            description: String::new(),
        }
    }
}

// ============================================================================
// PROFILER MANAGER
// ============================================================================

/// Profiler stats
#[derive(Debug, Clone, Default)]
pub struct HolisticProfilerStats {
    /// Active sessions
    pub active_sessions: usize,
    /// Total samples collected
    pub total_samples: u64,
    /// Hotspots identified
    pub hotspot_count: usize,
    /// Lock contentions tracked
    pub lock_contentions: usize,
    /// Optimization hints
    pub optimization_hints: usize,
    /// Average overhead
    pub avg_overhead_pct: f64,
}

/// System-wide profiler
pub struct HolisticProfiler {
    /// Sessions
    sessions: BTreeMap<u64, ProfileSession>,
    /// Hotspots per domain
    hotspots: BTreeMap<u8, BTreeMap<u64, Hotspot>>,
    /// Lock contentions
    lock_contentions: BTreeMap<u64, LockContention>,
    /// Optimization hints
    hints: Vec<OptimizationHint>,
    /// Sample buffer
    sample_buffer: Vec<ProfileSample>,
    /// Max buffer size
    max_buffer: usize,
    /// Next session ID
    next_session_id: u64,
    /// Stats
    stats: HolisticProfilerStats,
}

impl HolisticProfiler {
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            hotspots: BTreeMap::new(),
            lock_contentions: BTreeMap::new(),
            hints: Vec::new(),
            sample_buffer: Vec::new(),
            max_buffer: 8192,
            next_session_id: 1,
            stats: HolisticProfilerStats::default(),
        }
    }

    /// Start profiling session
    pub fn start_session(
        &mut self,
        domains: Vec<ProfileDomain>,
        granularity: ProfileGranularity,
        now: u64,
    ) -> u64 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        self.sessions
            .insert(id, ProfileSession::new(id, domains, granularity, now));
        self.stats.active_sessions = self
            .sessions
            .values()
            .filter(|s| s.state == ProfileSessionState::Active)
            .count();
        id
    }

    /// Stop session
    pub fn stop_session(&mut self, session_id: u64, now: u64) {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.complete(now);
        }
        self.stats.active_sessions = self
            .sessions
            .values()
            .filter(|s| s.state == ProfileSessionState::Active)
            .count();
    }

    /// Ingest sample
    pub fn ingest_sample(&mut self, sample: ProfileSample) {
        // Update hotspot
        let domain_key = sample.domain as u8;
        let hotspot = self
            .hotspots
            .entry(domain_key)
            .or_insert_with(BTreeMap::new)
            .entry(sample.ip)
            .or_insert_with(|| Hotspot::new(sample.ip, sample.domain));
        hotspot.record(sample.pid, sample.value);

        // Buffer sample
        self.sample_buffer.push(sample);
        if self.sample_buffer.len() > self.max_buffer {
            self.sample_buffer.remove(0);
        }

        self.stats.total_samples += 1;
    }

    /// Record lock contention
    pub fn record_lock_contention(
        &mut self,
        lock_address: u64,
        lock_type: String,
        pid: u64,
        wait_ns: u64,
    ) {
        let contention = self
            .lock_contentions
            .entry(lock_address)
            .or_insert_with(|| LockContention::new(lock_address, lock_type));
        contention.record_contention(pid, wait_ns);
        self.stats.lock_contentions = self.lock_contentions.len();
    }

    /// Add optimization hint
    pub fn add_hint(&mut self, hint: OptimizationHint) {
        self.hints.push(hint);
        self.stats.optimization_hints = self.hints.len();
    }

    /// Get top hotspots for a domain
    pub fn top_hotspots(&self, domain: ProfileDomain, count: usize) -> Vec<&Hotspot> {
        let domain_key = domain as u8;
        let Some(hotspots) = self.hotspots.get(&domain_key) else {
            return Vec::new();
        };

        let mut sorted: Vec<&Hotspot> = hotspots.values().collect();
        sorted.sort_by(|a, b| b.samples.cmp(&a.samples));
        sorted.truncate(count);
        sorted
    }

    /// Get worst lock contentions
    pub fn worst_contentions(&self, count: usize) -> Vec<&LockContention> {
        let mut sorted: Vec<&LockContention> = self.lock_contentions.values().collect();
        sorted.sort_by(|a, b| b.total_wait_ns.cmp(&a.total_wait_ns));
        sorted.truncate(count);
        sorted
    }

    /// Get hints
    pub fn hints(&self) -> &[OptimizationHint] {
        &self.hints
    }

    /// Get stats
    pub fn stats(&self) -> &HolisticProfilerStats {
        &self.stats
    }
}
