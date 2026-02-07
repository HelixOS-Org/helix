//! # Application Statistical Sampling
//!
//! Statistical sampling engine for application profiling:
//! - CPU sampling
//! - Stack trace sampling
//! - Frequency-based profiling
//! - Flame graph data collection
//! - Hot path detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SAMPLING TYPES
// ============================================================================

/// Sample source
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SampleSource {
    /// Timer-based CPU sampling
    CpuTimer,
    /// Hardware performance counter
    HwCounter,
    /// Software event
    SwEvent,
    /// Manual trigger
    Manual,
    /// Page fault
    PageFault,
    /// Context switch
    ContextSwitch,
}

/// Sample data
#[derive(Debug, Clone)]
pub struct Sample {
    /// Process id
    pub pid: u64,
    /// Thread id
    pub tid: u64,
    /// Instruction pointer
    pub ip: u64,
    /// Stack trace (bottom to top)
    pub stack: Vec<u64>,
    /// Timestamp
    pub timestamp: u64,
    /// Source
    pub source: SampleSource,
    /// Weight (for frequency-based)
    pub weight: u32,
}

impl Sample {
    pub fn new(pid: u64, tid: u64, ip: u64, source: SampleSource, now: u64) -> Self {
        Self {
            pid,
            tid,
            ip,
            stack: Vec::new(),
            timestamp: now,
            source,
            weight: 1,
        }
    }

    /// Add stack frame
    pub fn push_frame(&mut self, addr: u64) {
        self.stack.push(addr);
    }

    /// Stack depth
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

// ============================================================================
// ADDRESS HISTOGRAM
// ============================================================================

/// Address hit histogram
#[derive(Debug)]
pub struct AddressHistogram {
    /// Address -> hit count
    hits: BTreeMap<u64, u64>,
    /// Total samples
    total: u64,
}

impl AddressHistogram {
    pub fn new() -> Self {
        Self {
            hits: BTreeMap::new(),
            total: 0,
        }
    }

    /// Record hit
    pub fn record(&mut self, addr: u64, weight: u32) {
        *self.hits.entry(addr).or_insert(0) += weight as u64;
        self.total += weight as u64;
    }

    /// Top N addresses
    pub fn top_n(&self, n: usize) -> Vec<(u64, u64, f64)> {
        let mut entries: Vec<_> = self.hits.iter().map(|(&a, &c)| (a, c)).collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(n);
        entries
            .into_iter()
            .map(|(addr, count)| {
                let pct = if self.total > 0 {
                    count as f64 / self.total as f64 * 100.0
                } else {
                    0.0
                };
                (addr, count, pct)
            })
            .collect()
    }

    /// Unique addresses
    pub fn unique_count(&self) -> usize {
        self.hits.len()
    }

    /// Total samples
    pub fn total(&self) -> u64 {
        self.total
    }
}

// ============================================================================
// CALL GRAPH
// ============================================================================

/// Call graph edge (caller -> callee)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CallEdge {
    /// Caller address
    pub caller: u64,
    /// Callee address
    pub callee: u64,
}

/// Call graph for flame graph construction
#[derive(Debug)]
pub struct CallGraph {
    /// Edge weights
    edges: BTreeMap<u64, u64>,
    /// Node self-weight
    self_weight: BTreeMap<u64, u64>,
    /// Total weight
    total_weight: u64,
}

impl CallGraph {
    pub fn new() -> Self {
        Self {
            edges: BTreeMap::new(),
            self_weight: BTreeMap::new(),
            total_weight: 0,
        }
    }

    fn edge_key(caller: u64, callee: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= caller;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= callee;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Add sample stack trace to call graph
    pub fn add_stack(&mut self, stack: &[u64], weight: u32) {
        if stack.is_empty() {
            return;
        }
        // Self-weight to the top of stack (leaf)
        let leaf = stack[stack.len() - 1];
        *self.self_weight.entry(leaf).or_insert(0) += weight as u64;
        self.total_weight += weight as u64;

        // Edges from bottom to top
        for i in 0..stack.len() - 1 {
            let key = Self::edge_key(stack[i], stack[i + 1]);
            *self.edges.entry(key).or_insert(0) += weight as u64;
        }
    }

    /// Hot functions (by self time)
    pub fn hot_functions(&self, limit: usize) -> Vec<(u64, u64, f64)> {
        let mut funcs: Vec<_> = self.self_weight.iter().map(|(&a, &w)| (a, w)).collect();
        funcs.sort_by(|a, b| b.1.cmp(&a.1));
        funcs.truncate(limit);
        funcs
            .into_iter()
            .map(|(addr, weight)| {
                let pct = if self.total_weight > 0 {
                    weight as f64 / self.total_weight as f64 * 100.0
                } else {
                    0.0
                };
                (addr, weight, pct)
            })
            .collect()
    }

    /// Total weight
    pub fn total_weight(&self) -> u64 {
        self.total_weight
    }
}

// ============================================================================
// SAMPLING CONFIG
// ============================================================================

/// Sampling configuration
#[derive(Debug, Clone)]
pub struct SamplingConfig {
    /// Sample rate (samples per second)
    pub rate: u32,
    /// Max stack depth
    pub max_stack_depth: usize,
    /// Source
    pub source: SampleSource,
    /// Per-process max samples before rotation
    pub max_samples_per_process: usize,
    /// Enabled
    pub enabled: bool,
}

impl SamplingConfig {
    pub fn default_config() -> Self {
        Self {
            rate: 99, // 99 Hz to avoid aliasing with timers
            max_stack_depth: 128,
            source: SampleSource::CpuTimer,
            max_samples_per_process: 16384,
            enabled: false,
        }
    }

    /// High frequency config
    pub fn high_frequency() -> Self {
        Self {
            rate: 999,
            max_stack_depth: 64,
            source: SampleSource::CpuTimer,
            max_samples_per_process: 65536,
            enabled: false,
        }
    }

    /// Interval in nanoseconds
    pub fn interval_ns(&self) -> u64 {
        if self.rate == 0 {
            return u64::MAX;
        }
        1_000_000_000 / self.rate as u64
    }
}

// ============================================================================
// PROCESS SAMPLING PROFILE
// ============================================================================

/// Per-process sampling profile
#[derive(Debug)]
pub struct ProcessSamplingProfile {
    /// Process id
    pub pid: u64,
    /// IP histogram
    pub ip_histogram: AddressHistogram,
    /// Call graph
    pub call_graph: CallGraph,
    /// Sample count
    pub sample_count: u64,
    /// First sample time
    pub first_sample: u64,
    /// Last sample time
    pub last_sample: u64,
    /// Unique threads seen
    pub threads: Vec<u64>,
}

impl ProcessSamplingProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            ip_histogram: AddressHistogram::new(),
            call_graph: CallGraph::new(),
            sample_count: 0,
            first_sample: 0,
            last_sample: 0,
            threads: Vec::new(),
        }
    }

    /// Ingest sample
    pub fn ingest(&mut self, sample: &Sample) {
        self.ip_histogram.record(sample.ip, sample.weight);
        if !sample.stack.is_empty() {
            self.call_graph.add_stack(&sample.stack, sample.weight);
        }
        self.sample_count += 1;
        if self.first_sample == 0 {
            self.first_sample = sample.timestamp;
        }
        self.last_sample = sample.timestamp;
        if !self.threads.contains(&sample.tid) {
            self.threads.push(sample.tid);
        }
    }

    /// Sampling duration
    pub fn duration_ns(&self) -> u64 {
        self.last_sample.saturating_sub(self.first_sample)
    }

    /// Sample rate (actual)
    pub fn actual_rate(&self) -> f64 {
        let dur = self.duration_ns();
        if dur == 0 {
            return 0.0;
        }
        self.sample_count as f64 / (dur as f64 / 1_000_000_000.0)
    }

    /// Top hot IPs
    pub fn hot_ips(&self, limit: usize) -> Vec<(u64, u64, f64)> {
        self.ip_histogram.top_n(limit)
    }

    /// Hot functions
    pub fn hot_functions(&self, limit: usize) -> Vec<(u64, u64, f64)> {
        self.call_graph.hot_functions(limit)
    }
}

// ============================================================================
// SAMPLING ENGINE
// ============================================================================

/// Sampling stats
#[derive(Debug, Clone, Default)]
pub struct AppSamplingStats {
    /// Profiles active
    pub active_profiles: usize,
    /// Total samples collected
    pub total_samples: u64,
    /// Samples per second
    pub current_rate: f64,
}

/// Application sampling engine
pub struct AppSamplingEngine {
    /// Config
    config: SamplingConfig,
    /// Profiles
    profiles: BTreeMap<u64, ProcessSamplingProfile>,
    /// Stats
    stats: AppSamplingStats,
    /// Last tick
    last_tick: u64,
    /// Samples this second
    samples_this_sec: u64,
}

impl AppSamplingEngine {
    pub fn new() -> Self {
        Self {
            config: SamplingConfig::default_config(),
            profiles: BTreeMap::new(),
            stats: AppSamplingStats::default(),
            last_tick: 0,
            samples_this_sec: 0,
        }
    }

    /// Configure
    pub fn configure(&mut self, config: SamplingConfig) {
        self.config = config;
    }

    /// Enable sampling
    pub fn enable(&mut self) {
        self.config.enabled = true;
    }

    /// Disable sampling
    pub fn disable(&mut self) {
        self.config.enabled = false;
    }

    /// Ingest a sample
    pub fn ingest(&mut self, sample: Sample) {
        if !self.config.enabled {
            return;
        }

        let pid = sample.pid;
        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(|| ProcessSamplingProfile::new(pid));
        profile.ingest(&sample);

        self.stats.total_samples += 1;
        self.samples_this_sec += 1;

        // Update rate every second
        let sec_boundary = sample.timestamp / 1_000_000_000;
        let last_sec = self.last_tick / 1_000_000_000;
        if sec_boundary > last_sec && self.last_tick > 0 {
            self.stats.current_rate = self.samples_this_sec as f64;
            self.samples_this_sec = 0;
        }
        self.last_tick = sample.timestamp;
        self.stats.active_profiles = self.profiles.len();
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessSamplingProfile> {
        self.profiles.get(&pid)
    }

    /// Hot across all processes
    pub fn global_hot_ips(&self, limit: usize) -> Vec<(u64, u64, f64)> {
        let mut combined = AddressHistogram::new();
        for profile in self.profiles.values() {
            // Merge histograms - we use top N from each
            for &(addr, count, _) in &profile.hot_ips(limit * 2) {
                combined.record(addr, count as u32);
            }
        }
        combined.top_n(limit)
    }

    /// Stats
    pub fn stats(&self) -> &AppSamplingStats {
        &self.stats
    }

    /// Config
    pub fn config(&self) -> &SamplingConfig {
        &self.config
    }
}
