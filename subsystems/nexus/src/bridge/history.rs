//! # Syscall History & Replay Engine
//!
//! Records full syscall execution history for:
//! - Debugging (replay past syscall sequences)
//! - Auditing (compliance & forensics)
//! - Performance analysis (post-hoc bottleneck detection)
//! - Machine learning training data
//! - Regression testing

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// HISTORY RECORDS
// ============================================================================

/// A single syscall execution record
#[derive(Debug, Clone)]
pub struct SyscallRecord {
    /// Unique sequence number
    pub seq: u64,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Syscall type
    pub syscall_type: SyscallType,
    /// Syscall number (raw)
    pub syscall_number: u32,
    /// Arguments (up to 6)
    pub args: [u64; 6],
    /// Return value
    pub return_value: i64,
    /// Entry timestamp (ns)
    pub entry_time: u64,
    /// Exit timestamp (ns)
    pub exit_time: u64,
    /// CPU that handled the syscall
    pub cpu_id: u32,
    /// Whether syscall was intercepted
    pub intercepted: bool,
    /// Whether syscall was transformed
    pub transformed: bool,
    /// Whether result was cached
    pub cache_hit: bool,
    /// Error code (if any)
    pub error: Option<i32>,
    /// Bytes transferred
    pub bytes: u64,
    /// Additional flags
    pub flags: RecordFlags,
}

/// Record flags
#[derive(Debug, Clone, Copy)]
pub struct RecordFlags {
    bits: u32,
}

impl RecordFlags {
    pub const NONE: Self = Self { bits: 0 };
    pub const AUDITED: Self = Self { bits: 1 };
    pub const SECURITY_CHECKED: Self = Self { bits: 2 };
    pub const RATE_LIMITED: Self = Self { bits: 4 };
    pub const BATCHED: Self = Self { bits: 8 };
    pub const ASYNC: Self = Self { bits: 16 };
    pub const PREFETCHED: Self = Self { bits: 32 };
    pub const COALESCED: Self = Self { bits: 64 };
    pub const DEPRECATED: Self = Self { bits: 128 };

    pub fn has(&self, flag: RecordFlags) -> bool {
        (self.bits & flag.bits) != 0
    }

    pub fn set(&mut self, flag: RecordFlags) {
        self.bits |= flag.bits;
    }
}

impl SyscallRecord {
    /// Execution latency
    pub fn latency_ns(&self) -> u64 {
        self.exit_time.saturating_sub(self.entry_time)
    }

    /// Whether the syscall succeeded
    pub fn success(&self) -> bool {
        self.return_value >= 0
    }
}

// ============================================================================
// RING BUFFER STORAGE
// ============================================================================

/// Ring buffer for storing records without heap reallocation
pub struct RecordRingBuffer {
    /// Storage
    records: Vec<SyscallRecord>,
    /// Write position
    write_pos: usize,
    /// Total records written (including overwritten)
    total_written: u64,
    /// Capacity
    capacity: usize,
    /// Whether the buffer has wrapped
    wrapped: bool,
}

impl RecordRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            records: Vec::with_capacity(capacity),
            write_pos: 0,
            total_written: 0,
            capacity,
            wrapped: false,
        }
    }

    /// Push a record
    pub fn push(&mut self, record: SyscallRecord) {
        if self.records.len() < self.capacity {
            self.records.push(record);
        } else {
            self.records[self.write_pos] = record;
            self.wrapped = true;
        }
        self.write_pos = (self.write_pos + 1) % self.capacity;
        self.total_written += 1;
    }

    /// Number of valid records
    pub fn len(&self) -> usize {
        if self.wrapped {
            self.capacity
        } else {
            self.records.len()
        }
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Get record by index (0 = oldest)
    pub fn get(&self, idx: usize) -> Option<&SyscallRecord> {
        if idx >= self.len() {
            return None;
        }
        if self.wrapped {
            let actual = (self.write_pos + idx) % self.capacity;
            self.records.get(actual)
        } else {
            self.records.get(idx)
        }
    }

    /// Iterate over records (oldest first)
    pub fn iter(&self) -> RingBufferIter<'_> {
        RingBufferIter {
            buffer: self,
            pos: 0,
            remaining: self.len(),
        }
    }

    /// Get last N records
    pub fn last_n(&self, n: usize) -> Vec<&SyscallRecord> {
        let count = n.min(self.len());
        let start = self.len().saturating_sub(count);
        (start..self.len()).filter_map(|i| self.get(i)).collect()
    }

    /// Total records written (including overwritten)
    pub fn total_written(&self) -> u64 {
        self.total_written
    }

    /// Records dropped (overwritten)
    pub fn dropped(&self) -> u64 {
        if self.wrapped {
            self.total_written - self.capacity as u64
        } else {
            0
        }
    }

    /// Clear all records
    pub fn clear(&mut self) {
        self.records.clear();
        self.write_pos = 0;
        self.wrapped = false;
    }
}

/// Iterator over ring buffer
pub struct RingBufferIter<'a> {
    buffer: &'a RecordRingBuffer,
    pos: usize,
    remaining: usize,
}

impl<'a> Iterator for RingBufferIter<'a> {
    type Item = &'a SyscallRecord;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let record = self.buffer.get(self.pos);
        self.pos += 1;
        self.remaining -= 1;
        record
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

// ============================================================================
// QUERY ENGINE
// ============================================================================

/// Query filter for searching history
#[derive(Debug, Clone)]
pub struct HistoryQuery {
    /// Filter by PID
    pub pid: Option<u64>,
    /// Filter by TID
    pub tid: Option<u64>,
    /// Filter by syscall type
    pub syscall_type: Option<SyscallType>,
    /// Filter by time range
    pub time_range: Option<(u64, u64)>,
    /// Filter by success/failure
    pub success: Option<bool>,
    /// Filter by minimum latency
    pub min_latency_ns: Option<u64>,
    /// Filter by maximum latency
    pub max_latency_ns: Option<u64>,
    /// Filter by flags
    pub required_flags: Option<RecordFlags>,
    /// Max results
    pub limit: usize,
}

impl HistoryQuery {
    pub fn new() -> Self {
        Self {
            pid: None,
            tid: None,
            syscall_type: None,
            time_range: None,
            success: None,
            min_latency_ns: None,
            max_latency_ns: None,
            required_flags: None,
            limit: 100,
        }
    }

    pub fn for_pid(mut self, pid: u64) -> Self {
        self.pid = Some(pid);
        self
    }

    pub fn for_type(mut self, t: SyscallType) -> Self {
        self.syscall_type = Some(t);
        self
    }

    pub fn in_range(mut self, start: u64, end: u64) -> Self {
        self.time_range = Some((start, end));
        self
    }

    pub fn failures_only(mut self) -> Self {
        self.success = Some(false);
        self
    }

    pub fn slow_calls(mut self, min_ns: u64) -> Self {
        self.min_latency_ns = Some(min_ns);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Check if a record matches this query
    fn matches(&self, record: &SyscallRecord) -> bool {
        if let Some(pid) = self.pid {
            if record.pid != pid {
                return false;
            }
        }
        if let Some(tid) = self.tid {
            if record.tid != tid {
                return false;
            }
        }
        if let Some(st) = self.syscall_type {
            if record.syscall_type != st {
                return false;
            }
        }
        if let Some((start, end)) = self.time_range {
            if record.entry_time < start || record.entry_time > end {
                return false;
            }
        }
        if let Some(success) = self.success {
            if record.success() != success {
                return false;
            }
        }
        if let Some(min) = self.min_latency_ns {
            if record.latency_ns() < min {
                return false;
            }
        }
        if let Some(max) = self.max_latency_ns {
            if record.latency_ns() > max {
                return false;
            }
        }
        if let Some(flags) = self.required_flags {
            if !record.flags.has(flags) {
                return false;
            }
        }
        true
    }
}

/// Query result
#[derive(Debug)]
pub struct QueryResult {
    /// Matching records
    pub records: Vec<SyscallRecord>,
    /// Total scanned
    pub scanned: u64,
    /// Whether result was truncated
    pub truncated: bool,
}

// ============================================================================
// HISTORY AGGREGATION
// ============================================================================

/// Aggregated statistics from history queries
#[derive(Debug, Clone)]
pub struct HistoryAggregation {
    /// Total records matching
    pub total_records: u64,
    /// Per-type counts
    pub type_counts: BTreeMap<u8, u64>,
    /// Per-process counts
    pub process_counts: BTreeMap<u64, u64>,
    /// Total latency sum
    pub total_latency_ns: u64,
    /// Min latency
    pub min_latency_ns: u64,
    /// Max latency
    pub max_latency_ns: u64,
    /// Error count
    pub error_count: u64,
    /// Bytes transferred
    pub total_bytes: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Transformed count
    pub transformed_count: u64,
}

impl HistoryAggregation {
    fn new() -> Self {
        Self {
            total_records: 0,
            type_counts: BTreeMap::new(),
            process_counts: BTreeMap::new(),
            total_latency_ns: 0,
            min_latency_ns: u64::MAX,
            max_latency_ns: 0,
            error_count: 0,
            total_bytes: 0,
            cache_hits: 0,
            transformed_count: 0,
        }
    }

    fn add_record(&mut self, record: &SyscallRecord) {
        self.total_records += 1;
        *self.type_counts.entry(record.syscall_type as u8).or_insert(0) += 1;
        *self.process_counts.entry(record.pid).or_insert(0) += 1;
        let lat = record.latency_ns();
        self.total_latency_ns += lat;
        if lat < self.min_latency_ns {
            self.min_latency_ns = lat;
        }
        if lat > self.max_latency_ns {
            self.max_latency_ns = lat;
        }
        if !record.success() {
            self.error_count += 1;
        }
        self.total_bytes += record.bytes;
        if record.cache_hit {
            self.cache_hits += 1;
        }
        if record.transformed {
            self.transformed_count += 1;
        }
    }

    /// Average latency
    pub fn avg_latency_ns(&self) -> u64 {
        if self.total_records == 0 {
            0
        } else {
            self.total_latency_ns / self.total_records
        }
    }

    /// Error rate
    pub fn error_rate(&self) -> f64 {
        if self.total_records == 0 {
            0.0
        } else {
            self.error_count as f64 / self.total_records as f64
        }
    }

    /// Cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        if self.total_records == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_records as f64
        }
    }
}

// ============================================================================
// HISTORY MANAGER
// ============================================================================

/// Central history management
pub struct HistoryManager {
    /// Global history ring buffer
    global_history: RecordRingBuffer,
    /// Per-process history (smaller buffers)
    process_histories: BTreeMap<u64, RecordRingBuffer>,
    /// Per-process buffer size
    per_process_size: usize,
    /// Max processes with history
    max_processes: usize,
    /// Sequence counter
    sequence: u64,
    /// Recording enabled
    pub recording: bool,
    /// Audit mode (never drop records)
    pub audit_mode: bool,
}

impl HistoryManager {
    pub fn new(global_size: usize, per_process_size: usize, max_processes: usize) -> Self {
        Self {
            global_history: RecordRingBuffer::new(global_size),
            process_histories: BTreeMap::new(),
            per_process_size,
            max_processes,
            sequence: 0,
            recording: true,
            audit_mode: false,
        }
    }

    /// Record a syscall
    pub fn record(&mut self, mut record: SyscallRecord) {
        if !self.recording {
            return;
        }

        self.sequence += 1;
        record.seq = self.sequence;

        let pid = record.pid;

        // Store in global history
        self.global_history.push(record.clone());

        // Store in per-process history
        if self.process_histories.len() < self.max_processes || self.process_histories.contains_key(&pid) {
            let size = self.per_process_size;
            self.process_histories
                .entry(pid)
                .or_insert_with(|| RecordRingBuffer::new(size))
                .push(record);
        }
    }

    /// Query global history
    pub fn query(&self, query: &HistoryQuery) -> QueryResult {
        let mut results = Vec::new();
        let mut scanned = 0u64;

        for record in self.global_history.iter() {
            scanned += 1;
            if query.matches(record) {
                results.push(record.clone());
                if results.len() >= query.limit {
                    return QueryResult {
                        records: results,
                        scanned,
                        truncated: true,
                    };
                }
            }
        }

        QueryResult {
            records: results,
            scanned,
            truncated: false,
        }
    }

    /// Query process-specific history
    pub fn query_process(&self, pid: u64, query: &HistoryQuery) -> QueryResult {
        let mut results = Vec::new();
        let mut scanned = 0u64;

        if let Some(history) = self.process_histories.get(&pid) {
            for record in history.iter() {
                scanned += 1;
                if query.matches(record) {
                    results.push(record.clone());
                    if results.len() >= query.limit {
                        return QueryResult {
                            records: results,
                            scanned,
                            truncated: true,
                        };
                    }
                }
            }
        }

        QueryResult {
            records: results,
            scanned,
            truncated: false,
        }
    }

    /// Aggregate statistics
    pub fn aggregate(&self, query: &HistoryQuery) -> HistoryAggregation {
        let mut agg = HistoryAggregation::new();

        for record in self.global_history.iter() {
            if query.matches(record) {
                agg.add_record(record);
            }
        }

        agg
    }

    /// Get last N records for a process
    pub fn recent_for_process(&self, pid: u64, n: usize) -> Vec<&SyscallRecord> {
        self.process_histories
            .get(&pid)
            .map(|h| h.last_n(n))
            .unwrap_or_default()
    }

    /// Remove process history
    pub fn remove_process(&mut self, pid: u64) {
        self.process_histories.remove(&pid);
    }

    /// Global record count
    pub fn global_count(&self) -> usize {
        self.global_history.len()
    }

    /// Total records ever written
    pub fn total_records(&self) -> u64 {
        self.global_history.total_written()
    }

    /// Records dropped
    pub fn dropped_records(&self) -> u64 {
        self.global_history.dropped()
    }

    /// Number of tracked processes
    pub fn tracked_processes(&self) -> usize {
        self.process_histories.len()
    }
}
