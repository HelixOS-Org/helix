//! # Advanced Syscall Pattern Recognition
//!
//! Detects complex multi-syscall patterns including:
//! - Sequential file processing patterns
//! - Network server accept loops
//! - Database-style transaction patterns
//! - Producer-consumer patterns
//! - Memory-mapped I/O patterns
//! - Polling/epoll patterns
//! - Fork-exec patterns

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// PATTERN TYPES
// ============================================================================

/// Recognized multi-syscall pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    /// open → read/write → close in sequence
    FileProcessing,
    /// socket → bind → listen → accept loop
    ServerAcceptLoop,
    /// open → mmap → access → munmap
    MemoryMappedIo,
    /// epoll_create → epoll_ctl → epoll_wait loop
    EventDrivenIo,
    /// fork → exec → waitpid
    ForkExec,
    /// read → process → write pipeline
    DataPipeline,
    /// repeated read → parse → write sequences (database)
    TransactionalIo,
    /// connect → send → recv → close
    ClientRequest,
    /// stat → open → read (file scanning)
    DirectoryScan,
    /// mmap → access → msync → munmap
    SharedMemoryIpc,
    /// ioctl sequences (device control)
    DeviceControl,
    /// signal setup → wait → handler patterns
    SignalDriven,
    /// brk/mmap for heap expansion
    HeapGrowth,
    /// timer_create → timer_settime → wait
    TimerDriven,
    /// sendmsg → recvmsg (message passing)
    MessagePassing,
    /// Unknown / no pattern
    Unknown,
}

/// Match confidence
#[derive(Debug, Clone, Copy)]
pub struct PatternMatch {
    /// Pattern type
    pub kind: PatternKind,
    /// Confidence (0.0 - 1.0)
    pub confidence: f64,
    /// Number of syscalls in the pattern instance
    pub syscall_count: usize,
    /// Estimated repetitions per second
    pub repetition_rate: f64,
    /// Pattern length (in distinct syscall types)
    pub pattern_length: usize,
}

// ============================================================================
// PATTERN TEMPLATES
// ============================================================================

/// A pattern template is a sequence of expected syscall types
/// with optional gaps and repetitions
#[derive(Debug, Clone)]
pub struct PatternTemplate {
    /// Pattern kind
    pub kind: PatternKind,
    /// Expected sequence of syscall types
    pub sequence: Vec<PatternElement>,
    /// Minimum confidence threshold
    pub min_confidence: f64,
    /// Allow gaps between elements
    pub allow_gaps: bool,
    /// Maximum gap size
    pub max_gap: usize,
}

/// Element in a pattern template
#[derive(Debug, Clone)]
pub enum PatternElement {
    /// Exact syscall type required
    Exact(SyscallType),
    /// Any of these types
    AnyOf(Vec<SyscallType>),
    /// Repeating element (type, min, max)
    Repeat(SyscallType, usize, usize),
    /// Optional element
    Optional(SyscallType),
    /// Wildcard (any single syscall)
    Wildcard,
    /// Repeating wildcard (min, max)
    WildcardRepeat(usize, usize),
}

impl PatternTemplate {
    pub fn new(kind: PatternKind) -> Self {
        Self {
            kind,
            sequence: Vec::new(),
            min_confidence: 0.7,
            allow_gaps: true,
            max_gap: 3,
        }
    }

    pub fn add(mut self, elem: PatternElement) -> Self {
        self.sequence.push(elem);
        self
    }

    pub fn with_min_confidence(mut self, c: f64) -> Self {
        self.min_confidence = c;
        self
    }

    /// Get built-in patterns
    pub fn builtin_patterns() -> Vec<PatternTemplate> {
        let mut patterns = Vec::new();

        // File processing: open → read* → close
        patterns.push(
            PatternTemplate::new(PatternKind::FileProcessing)
                .add(PatternElement::Exact(SyscallType::Open))
                .add(PatternElement::Repeat(SyscallType::Read, 1, 100))
                .add(PatternElement::Exact(SyscallType::Close))
                .with_min_confidence(0.8),
        );

        // Server accept loop: accept → read → write → close
        patterns.push(
            PatternTemplate::new(PatternKind::ServerAcceptLoop)
                .add(PatternElement::Exact(SyscallType::Accept))
                .add(PatternElement::Repeat(SyscallType::Read, 1, 10))
                .add(PatternElement::Repeat(SyscallType::Write, 1, 10))
                .add(PatternElement::Optional(SyscallType::Close))
                .with_min_confidence(0.7),
        );

        // Memory-mapped I/O: open → mmap → read/write → munmap
        patterns.push(
            PatternTemplate::new(PatternKind::MemoryMappedIo)
                .add(PatternElement::Exact(SyscallType::Open))
                .add(PatternElement::Exact(SyscallType::Mmap))
                .add(PatternElement::WildcardRepeat(0, 20))
                .add(PatternElement::Exact(SyscallType::Munmap))
                .with_min_confidence(0.75),
        );

        // Fork-exec: fork → exec → waitpid
        patterns.push(
            PatternTemplate::new(PatternKind::ForkExec)
                .add(PatternElement::AnyOf(alloc::vec![
                    SyscallType::Fork,
                    SyscallType::Clone
                ]))
                .add(PatternElement::WildcardRepeat(0, 5))
                .add(PatternElement::Exact(SyscallType::Exec))
                .with_min_confidence(0.8),
        );

        // Client request: connect → write → read → close
        patterns.push(
            PatternTemplate::new(PatternKind::ClientRequest)
                .add(PatternElement::Exact(SyscallType::Connect))
                .add(PatternElement::Repeat(SyscallType::Write, 1, 5))
                .add(PatternElement::Repeat(SyscallType::Read, 1, 10))
                .add(PatternElement::Exact(SyscallType::Close))
                .with_min_confidence(0.75),
        );

        patterns
    }
}

// ============================================================================
// PATTERN MATCHER
// ============================================================================

/// State machine for pattern matching
#[derive(Debug)]
struct MatchState {
    /// Template being matched
    template_idx: usize,
    /// Current position in the template sequence
    position: usize,
    /// Matched syscall count
    matched: usize,
    /// Gap count
    gaps: usize,
    /// Repeat count for current element
    repeat_count: usize,
    /// Start index in the history
    start_idx: usize,
}

/// Pattern matcher engine
pub struct PatternMatcher {
    /// Pattern templates
    templates: Vec<PatternTemplate>,
    /// Per-process syscall history
    histories: BTreeMap<u64, SyscallHistory>,
    /// Max history length
    max_history: usize,
    /// Detected patterns per process
    detected: BTreeMap<u64, Vec<PatternMatch>>,
    /// Total patterns detected
    pub total_detections: u64,
}

/// Syscall history for a process
#[derive(Debug, Clone)]
struct SyscallHistory {
    /// Recent syscall types
    types: Vec<SyscallType>,
    /// Timestamps
    timestamps: Vec<u64>,
}

impl SyscallHistory {
    fn new() -> Self {
        Self {
            types: Vec::new(),
            timestamps: Vec::new(),
        }
    }

    fn push(&mut self, syscall_type: SyscallType, timestamp: u64, max: usize) {
        if self.types.len() >= max {
            self.types.remove(0);
            self.timestamps.remove(0);
        }
        self.types.push(syscall_type);
        self.timestamps.push(timestamp);
    }
}

impl PatternMatcher {
    pub fn new() -> Self {
        Self {
            templates: PatternTemplate::builtin_patterns(),
            histories: BTreeMap::new(),
            max_history: 256,
            detected: BTreeMap::new(),
            total_detections: 0,
        }
    }

    /// Add a custom pattern template
    pub fn add_template(&mut self, template: PatternTemplate) {
        self.templates.push(template);
    }

    /// Record a syscall and check for patterns
    pub fn record(
        &mut self,
        pid: u64,
        syscall_type: SyscallType,
        timestamp: u64,
    ) -> Vec<PatternMatch> {
        let max = self.max_history;
        let history = self
            .histories
            .entry(pid)
            .or_insert_with(SyscallHistory::new);
        history.push(syscall_type, timestamp, max);

        // Try to match all templates against recent history
        let mut matches = Vec::new();
        for template in &self.templates {
            if let Some(m) = self.try_match(template, &history.types, &history.timestamps) {
                matches.push(m);
            }
        }

        if !matches.is_empty() {
            self.total_detections += matches.len() as u64;
            self.detected.insert(pid, matches.clone());
        }

        matches
    }

    /// Try to match a template against a syscall history
    fn try_match(
        &self,
        template: &PatternTemplate,
        types: &[SyscallType],
        _timestamps: &[u64],
    ) -> Option<PatternMatch> {
        if types.len() < template.sequence.len() {
            return None;
        }

        // Try matching from the end of history backwards
        let window_size = types.len().min(64);
        let start = types.len().saturating_sub(window_size);
        let window = &types[start..];

        let mut best_match: Option<(usize, usize, usize)> = None; // (matched, total, gaps)

        // Sliding window matching
        for start_pos in 0..window.len() {
            let (matched, total, gaps) = self.match_from(template, window, start_pos);
            if total > 0 {
                let better = match best_match {
                    None => true,
                    Some((bm, bt, _)) => {
                        let old_conf = bm as f64 / bt as f64;
                        let new_conf = matched as f64 / total as f64;
                        new_conf > old_conf
                    },
                };
                if better {
                    best_match = Some((matched, total, gaps));
                }
            }
        }

        if let Some((matched, total, _gaps)) = best_match {
            let confidence = matched as f64 / total as f64;
            if confidence >= template.min_confidence {
                return Some(PatternMatch {
                    kind: template.kind,
                    confidence,
                    syscall_count: matched,
                    repetition_rate: 0.0,
                    pattern_length: template.sequence.len(),
                });
            }
        }

        None
    }

    /// Match template starting from a position in the window
    fn match_from(
        &self,
        template: &PatternTemplate,
        window: &[SyscallType],
        start: usize,
    ) -> (usize, usize, usize) {
        let mut pos = start;
        let mut matched = 0usize;
        let mut total_required = 0usize;
        let mut gaps = 0usize;

        for elem in &template.sequence {
            if pos >= window.len() {
                total_required += 1;
                continue;
            }

            match elem {
                PatternElement::Exact(expected) => {
                    total_required += 1;
                    if window[pos] == *expected {
                        matched += 1;
                        pos += 1;
                    } else if template.allow_gaps {
                        // Try skipping up to max_gap
                        let mut found = false;
                        for skip in 1..=template.max_gap {
                            if pos + skip < window.len() && window[pos + skip] == *expected {
                                matched += 1;
                                gaps += skip;
                                pos += skip + 1;
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            pos += 1;
                        }
                    } else {
                        pos += 1;
                    }
                },
                PatternElement::AnyOf(options) => {
                    total_required += 1;
                    if options.contains(&window[pos]) {
                        matched += 1;
                    }
                    pos += 1;
                },
                PatternElement::Repeat(expected, min, max) => {
                    total_required += *min;
                    let mut count = 0;
                    while pos < window.len() && window[pos] == *expected && count < *max {
                        count += 1;
                        pos += 1;
                    }
                    if count >= *min {
                        matched += count.min(*min);
                    }
                },
                PatternElement::Optional(expected) => {
                    if pos < window.len() && window[pos] == *expected {
                        matched += 1;
                        total_required += 1;
                        pos += 1;
                    }
                },
                PatternElement::Wildcard => {
                    total_required += 1;
                    matched += 1;
                    pos += 1;
                },
                PatternElement::WildcardRepeat(min, max) => {
                    total_required += *min;
                    let advance = (*max).min(window.len() - pos);
                    matched += advance.min(*min);
                    pos += advance;
                },
            }
        }

        (matched, total_required, gaps)
    }

    /// Get most recently detected pattern for a process
    pub fn get_pattern(&self, pid: u64) -> Option<&[PatternMatch]> {
        self.detected.get(&pid).map(|v| v.as_slice())
    }

    /// Get dominant pattern for a process
    pub fn dominant_pattern(&self, pid: u64) -> Option<PatternKind> {
        self.detected.get(&pid).and_then(|matches| {
            matches
                .iter()
                .max_by(|a, b| {
                    a.confidence
                        .partial_cmp(&b.confidence)
                        .unwrap_or(core::cmp::Ordering::Equal)
                })
                .map(|m| m.kind)
        })
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.histories.remove(&pid);
        self.detected.remove(&pid);
    }

    /// Number of tracked processes
    pub fn tracked_processes(&self) -> usize {
        self.histories.len()
    }
}

// ============================================================================
// PATTERN FREQUENCY ANALYZER
// ============================================================================

/// N-gram frequency analyzer for pattern discovery
pub struct NgramAnalyzer {
    /// Bigrams (2-gram)
    bigrams: BTreeMap<(u8, u8), u64>,
    /// Trigrams (3-gram)
    trigrams: BTreeMap<(u8, u8, u8), u64>,
    /// Quadgrams (4-gram)
    quadgrams: BTreeMap<(u8, u8, u8, u8), u64>,
    /// Total samples
    pub total_samples: u64,
}

impl NgramAnalyzer {
    pub fn new() -> Self {
        Self {
            bigrams: BTreeMap::new(),
            trigrams: BTreeMap::new(),
            quadgrams: BTreeMap::new(),
            total_samples: 0,
        }
    }

    /// Feed a sequence of syscall types
    pub fn feed(&mut self, types: &[SyscallType]) {
        self.total_samples += 1;

        for window in types.windows(2) {
            *self
                .bigrams
                .entry((window[0] as u8, window[1] as u8))
                .or_insert(0) += 1;
        }
        for window in types.windows(3) {
            *self
                .trigrams
                .entry((window[0] as u8, window[1] as u8, window[2] as u8))
                .or_insert(0) += 1;
        }
        for window in types.windows(4) {
            *self
                .quadgrams
                .entry((
                    window[0] as u8,
                    window[1] as u8,
                    window[2] as u8,
                    window[3] as u8,
                ))
                .or_insert(0) += 1;
        }
    }

    /// Top N bigrams
    pub fn top_bigrams(&self, n: usize) -> Vec<((u8, u8), u64)> {
        let mut items: Vec<_> = self.bigrams.iter().map(|(&k, &v)| (k, v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.truncate(n);
        items
    }

    /// Top N trigrams
    pub fn top_trigrams(&self, n: usize) -> Vec<((u8, u8, u8), u64)> {
        let mut items: Vec<_> = self.trigrams.iter().map(|(&k, &v)| (k, v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.truncate(n);
        items
    }

    /// Top N quadgrams
    pub fn top_quadgrams(&self, n: usize) -> Vec<((u8, u8, u8, u8), u64)> {
        let mut items: Vec<_> = self.quadgrams.iter().map(|(&k, &v)| (k, v)).collect();
        items.sort_by(|a, b| b.1.cmp(&a.1));
        items.truncate(n);
        items
    }

    /// Bigram probability P(b | a)
    pub fn bigram_probability(&self, a: SyscallType, b: SyscallType) -> f64 {
        let count = self.bigrams.get(&(a as u8, b as u8)).copied().unwrap_or(0);
        let total_a: u64 = self
            .bigrams
            .iter()
            .filter(|(&(first, _), _)| first == a as u8)
            .map(|(_, &v)| v)
            .sum();
        if total_a == 0 {
            0.0
        } else {
            count as f64 / total_a as f64
        }
    }

    /// Reset all counters
    pub fn reset(&mut self) {
        self.bigrams.clear();
        self.trigrams.clear();
        self.quadgrams.clear();
        self.total_samples = 0;
    }
}
