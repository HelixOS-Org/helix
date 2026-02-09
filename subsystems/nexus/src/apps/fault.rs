//! # Application Fault Analysis
//!
//! Application fault and crash analysis:
//! - Page fault tracking
//! - Segfault analysis
//! - OOM kill tracking
//! - Crash pattern detection
//! - Fault correlation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// FAULT TYPES
// ============================================================================

/// Fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultType {
    /// Minor page fault
    MinorPageFault,
    /// Major page fault (disk I/O)
    MajorPageFault,
    /// Segmentation fault
    Segfault,
    /// Bus error
    BusError,
    /// Stack overflow
    StackOverflow,
    /// Illegal instruction
    IllegalInstruction,
    /// Division by zero
    DivisionByZero,
    /// Out of memory
    OutOfMemory,
    /// Floating point exception
    FloatingPoint,
}

/// Fault severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultSeverity {
    /// Info (minor page faults)
    Info,
    /// Warning (high fault rate)
    Warning,
    /// Error (recoverable faults)
    Error,
    /// Fatal (process killed)
    Fatal,
}

impl FaultType {
    /// Default severity
    pub fn severity(&self) -> FaultSeverity {
        match self {
            Self::MinorPageFault => FaultSeverity::Info,
            Self::MajorPageFault => FaultSeverity::Warning,
            Self::OutOfMemory => FaultSeverity::Fatal,
            Self::Segfault | Self::BusError | Self::StackOverflow => FaultSeverity::Fatal,
            Self::IllegalInstruction | Self::DivisionByZero | Self::FloatingPoint => {
                FaultSeverity::Error
            }
        }
    }

    /// Is fatal?
    pub fn is_fatal(&self) -> bool {
        matches!(self.severity(), FaultSeverity::Fatal)
    }
}

// ============================================================================
// FAULT EVENT
// ============================================================================

/// Fault event
#[derive(Debug, Clone)]
pub struct FaultEvent {
    /// Process id
    pub pid: u64,
    /// Thread id
    pub tid: u64,
    /// Fault type
    pub fault_type: FaultType,
    /// Faulting address
    pub address: u64,
    /// Instruction pointer
    pub ip: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Resolved? (e.g., page fault served)
    pub resolved: bool,
    /// Resolution latency (ns)
    pub resolution_ns: u64,
}

// ============================================================================
// FAULT PATTERN
// ============================================================================

/// Fault pattern (recurring issue)
#[derive(Debug, Clone)]
pub struct FaultPattern {
    /// Pattern id
    pub id: u64,
    /// Fault type
    pub fault_type: FaultType,
    /// Common address range
    pub addr_range: (u64, u64),
    /// Occurrences
    pub occurrences: u64,
    /// Affected processes
    pub affected_pids: Vec<u64>,
    /// First seen
    pub first_seen: u64,
    /// Last seen
    pub last_seen: u64,
}

impl FaultPattern {
    /// Duration
    pub fn duration_ns(&self) -> u64 {
        self.last_seen.saturating_sub(self.first_seen)
    }

    /// Rate (per second)
    pub fn rate(&self) -> f64 {
        let dur = self.duration_ns();
        if dur == 0 {
            return self.occurrences as f64;
        }
        self.occurrences as f64 / (dur as f64 / 1_000_000_000.0)
    }
}

// ============================================================================
// PROCESS FAULT PROFILE
// ============================================================================

/// Per-process fault profile
#[derive(Debug, Clone)]
pub struct ProcessFaultProfile {
    /// Process id
    pub pid: u64,
    /// Fault counts by type
    counts: BTreeMap<u8, u64>,
    /// Recent faults
    recent: VecDeque<FaultEvent>,
    /// Max recent
    max_recent: usize,
    /// Total faults
    pub total_faults: u64,
    /// Fatal faults
    pub fatal_faults: u64,
    /// Page fault rate (per second, EMA)
    pub page_fault_rate: f64,
    /// Last update
    last_update: u64,
}

impl ProcessFaultProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            counts: BTreeMap::new(),
            recent: VecDeque::new(),
            max_recent: 64,
            total_faults: 0,
            fatal_faults: 0,
            page_fault_rate: 0.0,
            last_update: 0,
        }
    }

    /// Record fault
    pub fn record(&mut self, event: &FaultEvent) {
        *self.counts.entry(event.fault_type as u8).or_insert(0) += 1;
        self.total_faults += 1;
        if event.fault_type.is_fatal() {
            self.fatal_faults += 1;
        }

        // Page fault rate EMA
        if matches!(
            event.fault_type,
            FaultType::MinorPageFault | FaultType::MajorPageFault
        ) {
            let dt = event.timestamp.saturating_sub(self.last_update);
            if dt > 0 {
                let instant_rate = 1_000_000_000.0 / dt as f64;
                self.page_fault_rate = 0.1 * instant_rate + 0.9 * self.page_fault_rate;
            }
        }
        self.last_update = event.timestamp;

        self.recent.push_back(event.clone());
        if self.recent.len() > self.max_recent {
            self.recent.pop_front();
        }
    }

    /// Count for type
    pub fn count_for(&self, fault_type: FaultType) -> u64 {
        self.counts.get(&(fault_type as u8)).copied().unwrap_or(0)
    }

    /// Major/minor ratio
    pub fn major_minor_ratio(&self) -> f64 {
        let major = self.count_for(FaultType::MajorPageFault);
        let minor = self.count_for(FaultType::MinorPageFault);
        let total = major + minor;
        if total == 0 {
            return 0.0;
        }
        major as f64 / total as f64
    }
}

// ============================================================================
// FAULT ANALYZER
// ============================================================================

/// Fault analysis stats
#[derive(Debug, Clone, Default)]
pub struct AppFaultStats {
    /// Processes tracked
    pub processes: usize,
    /// Total faults
    pub total_faults: u64,
    /// Fatal faults
    pub fatal_faults: u64,
    /// Patterns detected
    pub patterns: usize,
    /// Average page fault rate
    pub avg_page_fault_rate: f64,
}

/// Application fault analyzer
pub struct AppFaultAnalyzer {
    /// Per-process profiles
    profiles: BTreeMap<u64, ProcessFaultProfile>,
    /// Detected patterns
    patterns: Vec<FaultPattern>,
    /// Next pattern id
    next_pattern_id: u64,
    /// Stats
    stats: AppFaultStats,
}

impl AppFaultAnalyzer {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            patterns: Vec::new(),
            next_pattern_id: 1,
            stats: AppFaultStats::default(),
        }
    }

    /// Record fault
    pub fn record(&mut self, event: FaultEvent) {
        let pid = event.pid;
        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(|| ProcessFaultProfile::new(pid));
        profile.record(&event);

        self.stats.total_faults += 1;
        if event.fault_type.is_fatal() {
            self.stats.fatal_faults += 1;
        }
        self.stats.processes = self.profiles.len();

        // Update patterns
        self.update_patterns(&event);
    }

    fn update_patterns(&mut self, event: &FaultEvent) {
        // Look for matching pattern
        for pattern in &mut self.patterns {
            if pattern.fault_type == event.fault_type
                && event.address >= pattern.addr_range.0
                && event.address <= pattern.addr_range.1
            {
                pattern.occurrences += 1;
                pattern.last_seen = event.timestamp;
                if !pattern.affected_pids.contains(&event.pid) {
                    pattern.affected_pids.push(event.pid);
                }
                return;
            }
        }

        // New pattern (for non-page-faults)
        if !matches!(
            event.fault_type,
            FaultType::MinorPageFault | FaultType::MajorPageFault
        ) {
            let id = self.next_pattern_id;
            self.next_pattern_id += 1;
            self.patterns.push(FaultPattern {
                id,
                fault_type: event.fault_type,
                addr_range: (
                    event.address.saturating_sub(4096),
                    event.address.saturating_add(4096),
                ),
                occurrences: 1,
                affected_pids: alloc::vec![event.pid],
                first_seen: event.timestamp,
                last_seen: event.timestamp,
            });
            self.stats.patterns = self.patterns.len();
        }
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessFaultProfile> {
        self.profiles.get(&pid)
    }

    /// High fault rate processes
    pub fn high_fault_rate(&self, threshold: f64) -> Vec<(u64, f64)> {
        self.profiles
            .values()
            .filter(|p| p.page_fault_rate > threshold)
            .map(|p| (p.pid, p.page_fault_rate))
            .collect()
    }

    /// Stats
    pub fn stats(&self) -> &AppFaultStats {
        &self.stats
    }
}
