//! # Syscall Intent Analysis Engine
//!
//! Analyzes sequences of syscalls to determine the *intent* behind application
//! behavior. Rather than treating each syscall in isolation, this module builds
//! a semantic understanding of what the application is trying to accomplish.
//!
//! ## Intent Categories
//!
//! - **File Operations**: open→read→close sequences, file copying, directory traversal
//! - **Network Operations**: connect→send→recv patterns, server accept loops
//! - **Memory Operations**: mmap→write→munmap patterns, shared memory setup
//! - **Process Operations**: fork→exec→wait patterns, daemon setup
//! - **I/O Multiplexing**: epoll/select patterns, event-driven architectures

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// INTENT TYPES
// ============================================================================

/// High-level intent inferred from syscall patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntentType {
    /// Sequential file reading (e.g., video playback, log processing)
    SequentialFileRead,
    /// Random file access (e.g., database queries)
    RandomFileAccess,
    /// File copying (read from one fd, write to another)
    FileCopy,
    /// Directory traversal (opendir, readdir, stat sequences)
    DirectoryTraversal,
    /// File metadata scan (stat-heavy workload)
    MetadataScan,
    /// Large file write (sequential writes with possible fsync)
    LargeFileWrite,
    /// Log appending (open+append+close or persistent fd writes)
    LogAppend,
    /// Network client connection (connect + send/recv)
    NetworkClient,
    /// Network server accept loop (bind + listen + accept)
    NetworkServer,
    /// Network data streaming (continuous send or recv)
    NetworkStreaming,
    /// DNS resolution (short-lived UDP exchanges)
    DnsResolution,
    /// Memory-mapped file I/O
    MmapFileIo,
    /// Shared memory IPC setup
    SharedMemoryIpc,
    /// Anonymous memory allocation (brk/mmap anonymous)
    MemoryAllocation,
    /// Process spawning (fork/clone + exec)
    ProcessSpawn,
    /// Daemon setup (fork + setsid + redirect stdio)
    DaemonSetup,
    /// Signal handling setup
    SignalSetup,
    /// Event-driven I/O (epoll/select + non-blocking I/O)
    EventDrivenIo,
    /// Timer-based polling
    TimerPolling,
    /// Pipe-based IPC
    PipeIpc,
    /// File locking operations
    FileLocking,
    /// System information gathering
    SystemInfoQuery,
    /// Unknown / not yet classified
    Unknown,
}

impl IntentType {
    /// Whether this intent benefits from prefetching
    #[inline]
    pub fn benefits_from_prefetch(&self) -> bool {
        matches!(
            self,
            Self::SequentialFileRead
                | Self::FileCopy
                | Self::DirectoryTraversal
                | Self::NetworkStreaming
                | Self::LargeFileWrite
        )
    }

    /// Whether this intent benefits from batching
    #[inline]
    pub fn benefits_from_batching(&self) -> bool {
        matches!(
            self,
            Self::MetadataScan | Self::DirectoryTraversal | Self::LogAppend | Self::SystemInfoQuery
        )
    }

    /// Estimated relative latency sensitivity (0.0 = tolerant, 1.0 = very sensitive)
    pub fn latency_sensitivity(&self) -> f64 {
        match self {
            Self::EventDrivenIo => 0.95,
            Self::NetworkServer => 0.9,
            Self::NetworkClient => 0.85,
            Self::NetworkStreaming => 0.8,
            Self::DnsResolution => 0.85,
            Self::PipeIpc => 0.8,
            Self::SharedMemoryIpc => 0.75,
            Self::TimerPolling => 0.7,
            Self::SequentialFileRead => 0.5,
            Self::RandomFileAccess => 0.6,
            Self::FileCopy => 0.3,
            Self::LargeFileWrite => 0.3,
            Self::LogAppend => 0.4,
            Self::DirectoryTraversal => 0.4,
            Self::MetadataScan => 0.3,
            Self::MmapFileIo => 0.5,
            Self::MemoryAllocation => 0.6,
            Self::ProcessSpawn => 0.4,
            Self::DaemonSetup => 0.1,
            Self::SignalSetup => 0.3,
            Self::FileLocking => 0.7,
            Self::SystemInfoQuery => 0.2,
            Self::Unknown => 0.5,
        }
    }

    /// Recommended I/O block size multiplier
    pub fn io_size_multiplier(&self) -> u32 {
        match self {
            Self::SequentialFileRead => 4, // 4x readahead
            Self::FileCopy => 8,           // 8x for throughput
            Self::LargeFileWrite => 4,     // 4x write buffering
            Self::NetworkStreaming => 2,   // 2x socket buffer
            Self::DirectoryTraversal => 2, // 2x directory buffer
            Self::RandomFileAccess => 1,   // No readahead
            Self::LogAppend => 1,          // Direct writes
            _ => 1,
        }
    }
}

// ============================================================================
// INTENT CONFIDENCE
// ============================================================================

/// Confidence level for an intent detection
#[derive(Debug, Clone, Copy)]
pub struct IntentConfidence {
    /// The detected intent
    pub intent: IntentType,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
    /// Number of syscalls that contributed to this detection
    pub evidence_count: u32,
    /// Timestamp of first evidence
    pub first_seen: u64,
    /// Timestamp of latest evidence
    pub last_seen: u64,
}

impl IntentConfidence {
    pub fn new(intent: IntentType, confidence: f64) -> Self {
        Self {
            intent,
            confidence: confidence.clamp(0.0, 1.0),
            evidence_count: 1,
            first_seen: 0,
            last_seen: 0,
        }
    }

    /// Strengthen this detection with additional evidence
    #[inline]
    pub fn strengthen(&mut self, additional_confidence: f64, timestamp: u64) {
        self.evidence_count += 1;
        self.last_seen = timestamp;
        // Asymptotic approach to 1.0
        let remaining = 1.0 - self.confidence;
        self.confidence += remaining * additional_confidence.clamp(0.0, 0.5);
    }

    /// Decay confidence over time
    #[inline(always)]
    pub fn decay(&mut self, elapsed_ms: u64) {
        let decay_factor = 1.0 / (1.0 + elapsed_ms as f64 / 5000.0);
        self.confidence *= decay_factor;
    }

    /// Whether this detection is strong enough to act on
    #[inline(always)]
    pub fn is_actionable(&self) -> bool {
        self.confidence >= 0.6 && self.evidence_count >= 3
    }
}

// ============================================================================
// PATTERN TEMPLATES
// ============================================================================

/// A pattern template that defines a sequence of syscall types that indicate an intent
#[derive(Debug, Clone)]
pub struct IntentPattern {
    /// Name of the pattern
    pub name: &'static str,
    /// The intent this pattern indicates
    pub intent: IntentType,
    /// Required syscall sequence (in order)
    pub sequence: Vec<SyscallType>,
    /// Minimum match ratio to detect (0.0 - 1.0)
    pub min_match_ratio: f64,
    /// Maximum gap allowed between sequence elements
    pub max_gap: usize,
    /// Base confidence when pattern is matched
    pub base_confidence: f64,
}

impl IntentPattern {
    pub fn new(name: &'static str, intent: IntentType, sequence: Vec<SyscallType>) -> Self {
        Self {
            name,
            intent,
            sequence,
            min_match_ratio: 0.7,
            max_gap: 3,
            base_confidence: 0.7,
        }
    }

    /// Try to match this pattern against a syscall window
    pub fn match_against(&self, window: &[SyscallType]) -> Option<f64> {
        if window.is_empty() || self.sequence.is_empty() {
            return None;
        }

        let mut seq_idx = 0;
        let mut matched = 0;
        let mut gap = 0;

        for syscall in window {
            if seq_idx < self.sequence.len() && *syscall == self.sequence[seq_idx] {
                matched += 1;
                seq_idx += 1;
                gap = 0;
            } else {
                gap += 1;
                if gap > self.max_gap {
                    // Reset if gap too large
                    seq_idx = 0;
                    matched = 0;
                    gap = 0;
                }
            }
        }

        let match_ratio = matched as f64 / self.sequence.len() as f64;
        if match_ratio >= self.min_match_ratio {
            Some(self.base_confidence * match_ratio)
        } else {
            None
        }
    }
}

// ============================================================================
// INTENT ANALYZER
// ============================================================================

/// Sliding window size for syscall analysis
const WINDOW_SIZE: usize = 64;
const MAX_ACTIVE_INTENTS: usize = 8;

/// Per-process intent state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessIntentState {
    /// Process ID
    pub pid: u64,
    /// Recent syscall window
    window: VecDeque<SyscallType>,
    /// Active detected intents
    active_intents: Vec<IntentConfidence>,
    /// Syscall type frequency counters
    type_counts: BTreeMap<u8, u64>,
    /// Total syscalls observed
    total_syscalls: u64,
    /// Last analysis timestamp
    last_analysis: u64,
}

impl ProcessIntentState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            window: Vec::with_capacity(WINDOW_SIZE),
            active_intents: Vec::new(),
            type_counts: BTreeMap::new(),
            total_syscalls: 0,
            last_analysis: 0,
        }
    }

    /// Record a syscall
    #[inline]
    pub fn record(&mut self, syscall_type: SyscallType) {
        if self.window.len() >= WINDOW_SIZE {
            self.window.pop_front();
        }
        self.window.push_back(syscall_type);
        self.total_syscalls += 1;

        let key = syscall_type as u8;
        *self.type_counts.entry(key).or_insert(0) += 1;
    }

    /// Get the current syscall window
    #[inline(always)]
    pub fn window(&self) -> &[SyscallType] {
        &self.window
    }

    /// Get the dominant intent (highest confidence)
    #[inline]
    pub fn dominant_intent(&self) -> Option<&IntentConfidence> {
        self.active_intents.iter().max_by(|a, b| {
            a.confidence
                .partial_cmp(&b.confidence)
                .unwrap_or(core::cmp::Ordering::Equal)
        })
    }

    /// Get all actionable intents
    #[inline]
    pub fn actionable_intents(&self) -> Vec<&IntentConfidence> {
        self.active_intents
            .iter()
            .filter(|i| i.is_actionable())
            .collect()
    }

    /// Update or add an intent detection
    pub fn update_intent(&mut self, intent: IntentType, confidence: f64, timestamp: u64) {
        if let Some(existing) = self.active_intents.iter_mut().find(|i| i.intent == intent) {
            existing.strengthen(confidence, timestamp);
        } else {
            if self.active_intents.len() >= MAX_ACTIVE_INTENTS {
                // Remove weakest
                if let Some(min_idx) = self
                    .active_intents
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.confidence
                            .partial_cmp(&b.confidence)
                            .unwrap_or(core::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                {
                    self.active_intents.remove(min_idx);
                }
            }
            let mut ic = IntentConfidence::new(intent, confidence);
            ic.first_seen = timestamp;
            ic.last_seen = timestamp;
            self.active_intents.push(ic);
        }
    }

    /// Decay all intents
    #[inline]
    pub fn decay_all(&mut self, elapsed_ms: u64) {
        for intent in &mut self.active_intents {
            intent.decay(elapsed_ms);
        }
        // Remove very weak intents
        self.active_intents.retain(|i| i.confidence > 0.05);
    }

    /// Frequency-based intent detection (fast heuristic)
    pub fn detect_frequency_intents(&self) -> Vec<(IntentType, f64)> {
        if self.total_syscalls < 10 {
            return Vec::new();
        }

        let mut results = Vec::new();
        let total = self.total_syscalls as f64;

        // Count reads vs writes
        let reads = *self
            .type_counts
            .get(&(SyscallType::Read as u8))
            .unwrap_or(&0) as f64;
        let writes = *self
            .type_counts
            .get(&(SyscallType::Write as u8))
            .unwrap_or(&0) as f64;
        let opens = *self
            .type_counts
            .get(&(SyscallType::Open as u8))
            .unwrap_or(&0) as f64;
        let stats = *self
            .type_counts
            .get(&(SyscallType::Stat as u8))
            .unwrap_or(&0) as f64;
        let connects = *self
            .type_counts
            .get(&(SyscallType::Connect as u8))
            .unwrap_or(&0) as f64;
        let accepts = *self
            .type_counts
            .get(&(SyscallType::Accept as u8))
            .unwrap_or(&0) as f64;
        let mmaps = *self
            .type_counts
            .get(&(SyscallType::Mmap as u8))
            .unwrap_or(&0) as f64;
        let polls = *self
            .type_counts
            .get(&(SyscallType::Poll as u8))
            .unwrap_or(&0) as f64;
        let forks = *self
            .type_counts
            .get(&(SyscallType::Fork as u8))
            .unwrap_or(&0) as f64;
        let sends = *self
            .type_counts
            .get(&(SyscallType::Send as u8))
            .unwrap_or(&0) as f64;
        let recvs = *self
            .type_counts
            .get(&(SyscallType::Recv as u8))
            .unwrap_or(&0) as f64;

        // Sequential read pattern: high read ratio, low random
        if reads / total > 0.4 && opens / total < 0.1 {
            results.push((IntentType::SequentialFileRead, reads / total));
        }

        // File copy: balanced reads and writes
        if reads / total > 0.2 && writes / total > 0.2 {
            let balance = 1.0 - (reads - writes).abs() / (reads + writes).max(1.0);
            if balance > 0.5 {
                results.push((IntentType::FileCopy, balance * 0.8));
            }
        }

        // Directory traversal: high stat + open ratio
        if stats / total > 0.3 && opens / total > 0.15 {
            results.push((IntentType::DirectoryTraversal, (stats + opens) / total));
        }

        // Metadata scan: very high stat ratio
        if stats / total > 0.5 {
            results.push((IntentType::MetadataScan, stats / total));
        }

        // Network server: accept-heavy
        if accepts / total > 0.05 {
            results.push((IntentType::NetworkServer, accepts / total * 5.0));
        }

        // Network client: connect-heavy
        if connects / total > 0.05 && accepts == 0.0 {
            results.push((IntentType::NetworkClient, connects / total * 5.0));
        }

        // Network streaming: high send/recv ratio
        if (sends + recvs) / total > 0.5 {
            results.push((IntentType::NetworkStreaming, (sends + recvs) / total));
        }

        // Event-driven I/O: high poll ratio
        if polls / total > 0.1 {
            results.push((IntentType::EventDrivenIo, polls / total * 3.0));
        }

        // Memory-mapped I/O: high mmap ratio
        if mmaps / total > 0.05 {
            results.push((IntentType::MmapFileIo, mmaps / total * 5.0));
        }

        // Process spawning: any forks
        if forks > 0.0 {
            results.push((IntentType::ProcessSpawn, (forks / total * 10.0).min(0.9)));
        }

        // Log appending: writes without reads
        if writes / total > 0.3 && reads / total < 0.05 {
            results.push((IntentType::LogAppend, writes / total));
        }

        // Clamp all confidences
        for (_, conf) in &mut results {
            *conf = conf.clamp(0.0, 1.0);
        }

        results
    }
}

// ============================================================================
// INTENT ANALYZER (global)
// ============================================================================

/// The global intent analyzer manages per-process intent detection
#[repr(align(64))]
pub struct IntentAnalyzer {
    /// Per-process state
    processes: BTreeMap<u64, ProcessIntentState>,
    /// Pattern library
    patterns: Vec<IntentPattern>,
    /// Total analyses performed
    analyses: u64,
    /// Total intents detected
    detections: u64,
}

impl IntentAnalyzer {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            patterns: Self::default_patterns(),
            analyses: 0,
            detections: 0,
        }
    }

    /// Get or create process state
    #[inline]
    pub fn process_state(&mut self, pid: u64) -> &mut ProcessIntentState {
        self.processes
            .entry(pid)
            .or_insert_with(|| ProcessIntentState::new(pid))
    }

    /// Record a syscall and analyze
    pub fn record_and_analyze(
        &mut self,
        pid: u64,
        syscall_type: SyscallType,
        timestamp: u64,
    ) -> Option<IntentType> {
        let state = self
            .processes
            .entry(pid)
            .or_insert_with(|| ProcessIntentState::new(pid));
        state.record(syscall_type);

        // Run analysis every 10 syscalls
        if state.total_syscalls % 10 != 0 {
            return state.dominant_intent().map(|i| i.intent);
        }

        self.analyses += 1;

        // Frequency-based detection
        let freq_intents = state.detect_frequency_intents();
        for (intent, confidence) in freq_intents {
            state.update_intent(intent, confidence, timestamp);
            self.detections += 1;
        }

        // Pattern-based detection
        let window: VecDeque<SyscallType> = state.window().to_vec();
        let patterns = &self.patterns;
        let mut pattern_matches = Vec::new();
        for pattern in patterns {
            if let Some(confidence) = pattern.match_against(&window) {
                pattern_matches.push((pattern.intent, confidence));
            }
        }

        let state = self.processes.get_mut(&pid).unwrap();
        for (intent, confidence) in pattern_matches {
            state.update_intent(intent, confidence, timestamp);
            self.detections += 1;
        }

        state.dominant_intent().map(|i| i.intent)
    }

    /// Get the current intent for a process
    #[inline]
    pub fn get_intent(&self, pid: u64) -> Option<IntentType> {
        self.processes
            .get(&pid)
            .and_then(|s| s.dominant_intent())
            .map(|i| i.intent)
    }

    /// Get all active intents for a process
    #[inline]
    pub fn get_all_intents(&self, pid: u64) -> Vec<(IntentType, f64)> {
        self.processes
            .get(&pid)
            .map(|s| {
                s.active_intents
                    .iter()
                    .map(|i| (i.intent, i.confidence))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Remove a process
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.processes.remove(&pid);
    }

    /// Decay all process intents
    #[inline]
    pub fn decay_all(&mut self, elapsed_ms: u64) {
        for state in self.processes.values_mut() {
            state.decay_all(elapsed_ms);
        }
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> (usize, u64, u64) {
        (self.processes.len(), self.analyses, self.detections)
    }

    fn default_patterns() -> Vec<IntentPattern> {
        vec![
            IntentPattern::new("sequential_read", IntentType::SequentialFileRead, vec![
                SyscallType::Open,
                SyscallType::Read,
                SyscallType::Read,
                SyscallType::Read,
            ]),
            IntentPattern::new("file_copy", IntentType::FileCopy, vec![
                SyscallType::Open,
                SyscallType::Read,
                SyscallType::Write,
                SyscallType::Read,
                SyscallType::Write,
            ]),
            IntentPattern::new("dir_traversal", IntentType::DirectoryTraversal, vec![
                SyscallType::Open,
                SyscallType::Stat,
                SyscallType::Stat,
                SyscallType::Stat,
            ]),
            IntentPattern::new("network_server", IntentType::NetworkServer, vec![
                SyscallType::Accept,
                SyscallType::Read,
                SyscallType::Write,
                SyscallType::Accept,
            ]),
            IntentPattern::new("process_spawn", IntentType::ProcessSpawn, vec![
                SyscallType::Fork,
                SyscallType::Exec,
            ]),
            IntentPattern::new("event_loop", IntentType::EventDrivenIo, vec![
                SyscallType::Poll,
                SyscallType::Read,
                SyscallType::Write,
                SyscallType::Poll,
            ]),
        ]
    }
}
