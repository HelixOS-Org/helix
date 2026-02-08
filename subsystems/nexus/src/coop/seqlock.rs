// SPDX-License-Identifier: GPL-2.0
//! Coop seqlock — sequence-lock mechanism for low-overhead read-mostly data.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Seqlock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqlockState {
    /// Idle — no writers active
    Idle,
    /// Write-locked — a writer holds the lock
    Writing,
}

/// Read attempt outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadOutcome {
    /// Successfully read consistent data
    Success,
    /// Reader detected concurrent write — must retry
    Retry,
    /// Too many retries — gave up
    GaveUp,
}

/// A single seqlock instance
#[derive(Debug)]
pub struct Seqlock {
    pub id: u64,
    pub state: SeqlockState,
    pub sequence: u64,
    pub writer_thread: Option<u64>,
    pub read_attempts: u64,
    pub read_successes: u64,
    pub read_retries: u64,
    pub write_count: u64,
    pub total_write_ns: u64,
    pub max_write_ns: u64,
    pub max_reader_retries: u32,
    write_start_ns: u64,
}

impl Seqlock {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            state: SeqlockState::Idle,
            sequence: 0,
            writer_thread: None,
            read_attempts: 0,
            read_successes: 0,
            read_retries: 0,
            write_count: 0,
            total_write_ns: 0,
            max_write_ns: 0,
            max_reader_retries: 0,
            write_start_ns: 0,
        }
    }

    pub fn write_begin(&mut self, thread_id: u64, now_ns: u64) -> bool {
        if self.state == SeqlockState::Writing { return false; }
        self.sequence += 1; // Odd = write in progress
        self.state = SeqlockState::Writing;
        self.writer_thread = Some(thread_id);
        self.write_start_ns = now_ns;
        true
    }

    pub fn write_end(&mut self, now_ns: u64) {
        self.sequence += 1; // Even = no write in progress
        self.state = SeqlockState::Idle;
        self.writer_thread = None;
        let dur = now_ns.saturating_sub(self.write_start_ns);
        self.write_count += 1;
        self.total_write_ns += dur;
        if dur > self.max_write_ns {
            self.max_write_ns = dur;
        }
    }

    pub fn read_begin(&self) -> u64 {
        self.sequence
    }

    pub fn read_validate(&mut self, start_seq: u64) -> ReadOutcome {
        self.read_attempts += 1;
        if start_seq & 1 != 0 {
            // Odd: write was in progress at read_begin
            self.read_retries += 1;
            ReadOutcome::Retry
        } else if self.sequence != start_seq {
            // Sequence changed during read
            self.read_retries += 1;
            ReadOutcome::Retry
        } else {
            self.read_successes += 1;
            ReadOutcome::Success
        }
    }

    pub fn read_with_retry(&mut self, max_retries: u32) -> ReadOutcome {
        for attempt in 0..max_retries {
            let seq = self.read_begin();
            // In real code, the caller would read data between read_begin/read_validate
            let outcome = self.read_validate(seq);
            match outcome {
                ReadOutcome::Success => {
                    if attempt > self.max_reader_retries as u32 {
                        self.max_reader_retries = attempt as u32;
                    }
                    return ReadOutcome::Success;
                }
                ReadOutcome::Retry => continue,
                ReadOutcome::GaveUp => return ReadOutcome::GaveUp,
            }
        }
        ReadOutcome::GaveUp
    }

    pub fn is_writing(&self) -> bool {
        self.state == SeqlockState::Writing
    }

    pub fn current_sequence(&self) -> u64 {
        self.sequence
    }

    pub fn retry_rate(&self) -> f64 {
        if self.read_attempts == 0 { return 0.0; }
        self.read_retries as f64 / self.read_attempts as f64
    }

    pub fn avg_write_ns(&self) -> f64 {
        if self.write_count == 0 { return 0.0; }
        self.total_write_ns as f64 / self.write_count as f64
    }

    pub fn read_write_ratio(&self) -> f64 {
        if self.write_count == 0 { return 0.0; }
        self.read_successes as f64 / self.write_count as f64
    }
}

/// Seqcount — lighter weight version (no spin lock, just sequence counter)
#[derive(Debug)]
pub struct Seqcount {
    pub id: u64,
    pub sequence: u64,
    pub reads: u64,
    pub writes: u64,
    pub retries: u64,
}

impl Seqcount {
    pub fn new(id: u64) -> Self {
        Self { id, sequence: 0, reads: 0, writes: 0, retries: 0 }
    }

    pub fn begin_write(&mut self) {
        self.sequence += 1;
    }

    pub fn end_write(&mut self) {
        self.sequence += 1;
        self.writes += 1;
    }

    pub fn read_begin(&self) -> u64 {
        self.sequence
    }

    pub fn read_check(&mut self, start: u64) -> bool {
        self.reads += 1;
        if start & 1 != 0 || self.sequence != start {
            self.retries += 1;
            false
        } else {
            true
        }
    }
}

/// Seqlock stats
#[derive(Debug, Clone)]
pub struct SeqlockStats {
    pub total_seqlocks: u64,
    pub total_seqcounts: u64,
    pub total_reads: u64,
    pub total_writes: u64,
    pub total_retries: u64,
    pub avg_retry_rate: f64,
}

/// Main seqlock manager
pub struct CoopSeqlock {
    seqlocks: BTreeMap<u64, Seqlock>,
    seqcounts: BTreeMap<u64, Seqcount>,
    next_id: u64,
    stats: SeqlockStats,
}

impl CoopSeqlock {
    pub fn new() -> Self {
        Self {
            seqlocks: BTreeMap::new(),
            seqcounts: BTreeMap::new(),
            next_id: 1,
            stats: SeqlockStats {
                total_seqlocks: 0,
                total_seqcounts: 0,
                total_reads: 0,
                total_writes: 0,
                total_retries: 0,
                avg_retry_rate: 0.0,
            },
        }
    }

    pub fn create_seqlock(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.seqlocks.insert(id, Seqlock::new(id));
        self.stats.total_seqlocks += 1;
        id
    }

    pub fn create_seqcount(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.seqcounts.insert(id, Seqcount::new(id));
        self.stats.total_seqcounts += 1;
        id
    }

    pub fn write_begin(&mut self, id: u64, thread_id: u64, now_ns: u64) -> bool {
        if let Some(sl) = self.seqlocks.get_mut(&id) {
            let ok = sl.write_begin(thread_id, now_ns);
            if ok { self.stats.total_writes += 1; }
            ok
        } else if let Some(sc) = self.seqcounts.get_mut(&id) {
            sc.begin_write();
            self.stats.total_writes += 1;
            true
        } else {
            false
        }
    }

    pub fn write_end(&mut self, id: u64, now_ns: u64) {
        if let Some(sl) = self.seqlocks.get_mut(&id) {
            sl.write_end(now_ns);
        } else if let Some(sc) = self.seqcounts.get_mut(&id) {
            sc.end_write();
        }
    }

    pub fn read_begin(&self, id: u64) -> Option<u64> {
        if let Some(sl) = self.seqlocks.get(&id) {
            Some(sl.read_begin())
        } else if let Some(sc) = self.seqcounts.get(&id) {
            Some(sc.read_begin())
        } else {
            None
        }
    }

    pub fn read_validate(&mut self, id: u64, start_seq: u64) -> bool {
        if let Some(sl) = self.seqlocks.get_mut(&id) {
            let outcome = sl.read_validate(start_seq);
            self.stats.total_reads += 1;
            matches!(outcome, ReadOutcome::Success)
        } else if let Some(sc) = self.seqcounts.get_mut(&id) {
            let ok = sc.read_check(start_seq);
            self.stats.total_reads += 1;
            if !ok { self.stats.total_retries += 1; }
            ok
        } else {
            false
        }
    }

    pub fn most_contended(&self, top: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<(u64, f64)> = self.seqlocks.iter()
            .map(|(&id, sl)| (id, sl.retry_rate()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(top);
        v
    }

    pub fn get_seqlock(&self, id: u64) -> Option<&Seqlock> {
        self.seqlocks.get(&id)
    }

    pub fn stats(&self) -> &SeqlockStats {
        &self.stats
    }
}

// ============================================================================
// Merged from seqlock_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqlockV2State {
    Idle,
    Writing,
    ReadActive,
}

/// Seqlock v2
#[derive(Debug)]
pub struct SeqlockV2 {
    pub id: u64,
    pub sequence: u64,
    pub state: SeqlockV2State,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_retries: u64,
    pub max_retries: u32,
    pub data_hash: u64,
}

impl SeqlockV2 {
    pub fn new(id: u64) -> Self {
        Self { id, sequence: 0, state: SeqlockV2State::Idle, total_writes: 0, total_reads: 0, total_retries: 0, max_retries: 0, data_hash: 0 }
    }

    pub fn write_begin(&mut self) -> u64 {
        self.sequence += 1;
        self.state = SeqlockV2State::Writing;
        self.sequence
    }

    pub fn write_end(&mut self, data_hash: u64) {
        self.data_hash = data_hash;
        self.sequence += 1;
        self.total_writes += 1;
        self.state = SeqlockV2State::Idle;
    }

    pub fn read_begin(&self) -> u64 { self.sequence }

    pub fn read_validate(&mut self, start_seq: u64, retries: u32) -> bool {
        self.total_reads += 1;
        self.total_retries += retries as u64;
        if retries > self.max_retries { self.max_retries = retries; }
        self.sequence == start_seq && (start_seq & 1) == 0
    }

    pub fn avg_retries(&self) -> f64 {
        if self.total_reads == 0 { 0.0 }
        else { self.total_retries as f64 / self.total_reads as f64 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SeqlockV2Stats {
    pub total_locks: u32,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_retries: u64,
    pub avg_retries: f64,
}

/// Main coop seqlock v2 manager
pub struct CoopSeqlockV2 {
    locks: BTreeMap<u64, SeqlockV2>,
    next_id: u64,
}

impl CoopSeqlockV2 {
    pub fn new() -> Self { Self { locks: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.locks.insert(id, SeqlockV2::new(id));
        id
    }

    pub fn write_begin(&mut self, id: u64) -> u64 {
        if let Some(l) = self.locks.get_mut(&id) { l.write_begin() }
        else { 0 }
    }

    pub fn write_end(&mut self, id: u64, data_hash: u64) {
        if let Some(l) = self.locks.get_mut(&id) { l.write_end(data_hash); }
    }

    pub fn destroy(&mut self, id: u64) { self.locks.remove(&id); }

    pub fn stats(&self) -> SeqlockV2Stats {
        let writes: u64 = self.locks.values().map(|l| l.total_writes).sum();
        let reads: u64 = self.locks.values().map(|l| l.total_reads).sum();
        let retries: u64 = self.locks.values().map(|l| l.total_retries).sum();
        let avg = if reads == 0 { 0.0 } else { retries as f64 / reads as f64 };
        SeqlockV2Stats { total_locks: self.locks.len() as u32, total_writes: writes, total_reads: reads, total_retries: retries, avg_retries: avg }
    }
}

// ============================================================================
// Merged from seqlock_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqlockV3Variant {
    Standard,
    Latch,
    Raw,
}

/// Seqlock state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqlockV3State {
    Idle,
    Writing,
    ReadActive,
}

/// A seqlock V3 instance.
#[derive(Debug)]
pub struct SeqlockV3Instance {
    pub lock_id: u64,
    pub sequence: AtomicU64,
    pub variant: SeqlockV3Variant,
    pub state: SeqlockV3State,
    pub writer_pid: Option<u64>,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_retries: u64,
    pub max_retries_single: u64,
    pub numa_node: Option<u32>,
}

impl SeqlockV3Instance {
    pub fn new(lock_id: u64, variant: SeqlockV3Variant) -> Self {
        Self {
            lock_id,
            sequence: AtomicU64::new(0),
            variant,
            state: SeqlockV3State::Idle,
            writer_pid: None,
            total_writes: 0,
            total_reads: 0,
            total_retries: 0,
            max_retries_single: 0,
            numa_node: None,
        }
    }

    pub fn write_begin(&mut self, pid: u64) -> u64 {
        let seq = self.sequence.fetch_add(1, Ordering::AcqRel);
        self.state = SeqlockV3State::Writing;
        self.writer_pid = Some(pid);
        seq
    }

    pub fn write_end(&mut self) {
        self.sequence.fetch_add(1, Ordering::AcqRel);
        self.state = SeqlockV3State::Idle;
        self.writer_pid = None;
        self.total_writes += 1;
    }

    pub fn read_begin(&self) -> u64 {
        self.sequence.load(Ordering::Acquire)
    }

    pub fn read_validate(&mut self, start_seq: u64) -> bool {
        let current = self.sequence.load(Ordering::Acquire);
        let valid = current == start_seq && (start_seq & 1) == 0;
        if valid {
            self.total_reads += 1;
        } else {
            self.total_retries += 1;
        }
        valid
    }

    pub fn retry_rate(&self) -> f64 {
        let total = self.total_reads + self.total_retries;
        if total == 0 {
            return 0.0;
        }
        self.total_retries as f64 / total as f64
    }
}

/// Per-reader retry tracking.
#[derive(Debug, Clone)]
pub struct SeqlockV3ReaderStats {
    pub pid: u64,
    pub total_reads: u64,
    pub total_retries: u64,
    pub max_retries: u32,
}

impl SeqlockV3ReaderStats {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            total_reads: 0,
            total_retries: 0,
            max_retries: 0,
        }
    }
}

/// Statistics for seqlock V3.
#[derive(Debug, Clone)]
pub struct SeqlockV3Stats {
    pub total_seqlocks: u64,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_retries: u64,
    pub latch_count: u64,
}

/// Main coop seqlock V3 manager.
pub struct CoopSeqlockV3 {
    pub seqlocks: BTreeMap<u64, SeqlockV3Instance>,
    pub reader_stats: BTreeMap<u64, SeqlockV3ReaderStats>,
    pub next_lock_id: u64,
    pub stats: SeqlockV3Stats,
}

impl CoopSeqlockV3 {
    pub fn new() -> Self {
        Self {
            seqlocks: BTreeMap::new(),
            reader_stats: BTreeMap::new(),
            next_lock_id: 1,
            stats: SeqlockV3Stats {
                total_seqlocks: 0,
                total_writes: 0,
                total_reads: 0,
                total_retries: 0,
                latch_count: 0,
            },
        }
    }

    pub fn create_seqlock(&mut self, variant: SeqlockV3Variant) -> u64 {
        let id = self.next_lock_id;
        self.next_lock_id += 1;
        let sl = SeqlockV3Instance::new(id, variant);
        self.seqlocks.insert(id, sl);
        self.stats.total_seqlocks += 1;
        if variant == SeqlockV3Variant::Latch {
            self.stats.latch_count += 1;
        }
        id
    }

    pub fn seqlock_count(&self) -> usize {
        self.seqlocks.len()
    }
}
