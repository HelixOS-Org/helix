// SPDX-License-Identifier: GPL-2.0
//! Apps fadvise_app — file access pattern advisor.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fadvise advice type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadviseAdvice {
    Normal,
    Sequential,
    Random,
    NoReuse,
    WillNeed,
    DontNeed,
}

/// File access region
#[derive(Debug, Clone)]
pub struct FadviseRegion {
    pub offset: u64,
    pub length: u64,
    pub advice: FadviseAdvice,
    pub timestamp: u64,
}

/// File access tracker
#[derive(Debug)]
pub struct FileAccessTracker {
    pub fd: i32,
    pub pid: u64,
    pub inode: u64,
    pub regions: Vec<FadviseRegion>,
    pub read_bytes: u64,
    pub sequential_reads: u64,
    pub random_reads: u64,
    pub last_offset: u64,
    pub readahead_hits: u64,
    pub readahead_misses: u64,
}

impl FileAccessTracker {
    pub fn new(fd: i32, pid: u64, inode: u64) -> Self {
        Self { fd, pid, inode, regions: Vec::new(), read_bytes: 0, sequential_reads: 0, random_reads: 0, last_offset: 0, readahead_hits: 0, readahead_misses: 0 }
    }

    #[inline]
    pub fn record_read(&mut self, offset: u64, len: u64) {
        if offset == self.last_offset { self.sequential_reads += 1; } else { self.random_reads += 1; }
        self.last_offset = offset + len;
        self.read_bytes += len;
    }

    #[inline]
    pub fn detect_pattern(&self) -> FadviseAdvice {
        let total = self.sequential_reads + self.random_reads;
        if total < 10 { return FadviseAdvice::Normal; }
        let seq_ratio = self.sequential_reads as f64 / total as f64;
        if seq_ratio > 0.8 { FadviseAdvice::Sequential } else if seq_ratio < 0.2 { FadviseAdvice::Random } else { FadviseAdvice::Normal }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FadviseAppStats {
    pub tracked_files: u32,
    pub total_advices: u64,
    pub sequential_files: u32,
    pub random_files: u32,
    pub total_read_bytes: u64,
}

/// Main fadvise app
pub struct AppFadvise {
    trackers: BTreeMap<u64, FileAccessTracker>,
}

impl AppFadvise {
    pub fn new() -> Self { Self { trackers: BTreeMap::new() } }

    #[inline(always)]
    pub fn track(&mut self, fd: i32, pid: u64, inode: u64) {
        let key = ((pid as u64) << 32) | fd as u64;
        self.trackers.insert(key, FileAccessTracker::new(fd, pid, inode));
    }

    #[inline(always)]
    pub fn advise(&mut self, pid: u64, fd: i32, region: FadviseRegion) {
        let key = ((pid as u64) << 32) | fd as u64;
        if let Some(t) = self.trackers.get_mut(&key) { t.regions.push(region); }
    }

    #[inline]
    pub fn stats(&self) -> FadviseAppStats {
        let advices: u64 = self.trackers.values().map(|t| t.regions.len() as u64).sum();
        let seq = self.trackers.values().filter(|t| t.detect_pattern() == FadviseAdvice::Sequential).count() as u32;
        let rand = self.trackers.values().filter(|t| t.detect_pattern() == FadviseAdvice::Random).count() as u32;
        let bytes: u64 = self.trackers.values().map(|t| t.read_bytes).sum();
        FadviseAppStats { tracked_files: self.trackers.len() as u32, total_advices: advices, sequential_files: seq, random_files: rand, total_read_bytes: bytes }
    }
}

// ============================================================================
// Merged from fadvise_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadviseV2Advice {
    Normal,
    Sequential,
    Random,
    NoReuse,
    WillNeed,
    DontNeed,
}

/// Fadvise v2 region
#[derive(Debug)]
pub struct FadviseV2Region {
    pub offset: u64,
    pub length: u64,
    pub advice: FadviseV2Advice,
    pub applied_at: u64,
}

/// File advice tracker v2
#[derive(Debug)]
pub struct FileAdviceV2 {
    pub fd: u64,
    pub regions: Vec<FadviseV2Region>,
    pub total_advised: u64,
    pub total_bytes: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
}

impl FileAdviceV2 {
    pub fn new(fd: u64) -> Self {
        Self { fd, regions: Vec::new(), total_advised: 0, total_bytes: 0, cache_hits: 0, cache_misses: 0 }
    }

    #[inline]
    pub fn advise(&mut self, offset: u64, len: u64, advice: FadviseV2Advice, now: u64) {
        self.regions.push(FadviseV2Region { offset, length: len, advice, applied_at: now });
        self.total_advised += 1;
        self.total_bytes += len;
    }

    #[inline]
    pub fn record_access(&mut self, offset: u64, hit: bool) {
        if hit { self.cache_hits += 1; }
        else { self.cache_misses += 1; }
        let _ = offset;
    }

    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 { return 0.0; }
        self.cache_hits as f64 / total as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FadviseV2AppStats {
    pub tracked_files: u32,
    pub total_advise_calls: u64,
    pub total_bytes_advised: u64,
    pub avg_hit_rate: f64,
}

/// Main app fadvise v2
pub struct AppFadviseV2 {
    files: BTreeMap<u64, FileAdviceV2>,
}

impl AppFadviseV2 {
    pub fn new() -> Self { Self { files: BTreeMap::new() } }

    #[inline(always)]
    pub fn track(&mut self, fd: u64) { self.files.insert(fd, FileAdviceV2::new(fd)); }

    #[inline(always)]
    pub fn advise(&mut self, fd: u64, offset: u64, len: u64, advice: FadviseV2Advice, now: u64) {
        if let Some(f) = self.files.get_mut(&fd) { f.advise(offset, len, advice, now); }
    }

    #[inline(always)]
    pub fn record_access(&mut self, fd: u64, offset: u64, hit: bool) {
        if let Some(f) = self.files.get_mut(&fd) { f.record_access(offset, hit); }
    }

    #[inline(always)]
    pub fn untrack(&mut self, fd: u64) { self.files.remove(&fd); }

    #[inline]
    pub fn stats(&self) -> FadviseV2AppStats {
        let calls: u64 = self.files.values().map(|f| f.total_advised).sum();
        let bytes: u64 = self.files.values().map(|f| f.total_bytes).sum();
        let rates: f64 = if self.files.is_empty() { 0.0 }
            else { self.files.values().map(|f| f.hit_rate()).sum::<f64>() / self.files.len() as f64 };
        FadviseV2AppStats { tracked_files: self.files.len() as u32, total_advise_calls: calls, total_bytes_advised: bytes, avg_hit_rate: rates }
    }
}

// ============================================================================
// Merged from fadvise_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadviseV3Advice {
    Normal,
    Sequential,
    Random,
    WillNeed,
    DontNeed,
    NoReuse,
}

/// Fadvise operation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadviseV3Result {
    Success,
    BadFd,
    InvalidArg,
    NoSpace,
    IoError,
}

/// A fadvise operation record.
#[derive(Debug, Clone)]
pub struct FadviseV3Record {
    pub record_id: u64,
    pub pid: u64,
    pub fd: i32,
    pub offset: u64,
    pub length: u64,
    pub advice: FadviseV3Advice,
    pub result: FadviseV3Result,
    pub timestamp: u64,
}

impl FadviseV3Record {
    pub fn new(record_id: u64, pid: u64, fd: i32, advice: FadviseV3Advice) -> Self {
        Self {
            record_id,
            pid,
            fd,
            offset: 0,
            length: 0,
            advice,
            result: FadviseV3Result::Success,
            timestamp: 0,
        }
    }
}

/// Per-file readahead state.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FileReadaheadState {
    pub inode: u64,
    pub current_window: u64,
    pub max_window: u64,
    pub min_window: u64,
    pub sequential_score: f64,
    pub random_score: f64,
    pub last_offset: u64,
    pub last_length: u64,
    pub sequential_hits: u64,
    pub random_hits: u64,
    pub willneed_prefetches: u64,
    pub dontneed_evictions: u64,
    pub readahead_pages: u64,
}

impl FileReadaheadState {
    pub fn new(inode: u64) -> Self {
        Self {
            inode,
            current_window: 32,
            max_window: 256,
            min_window: 4,
            sequential_score: 0.5,
            random_score: 0.5,
            last_offset: 0,
            last_length: 0,
            sequential_hits: 0,
            random_hits: 0,
            willneed_prefetches: 0,
            dontneed_evictions: 0,
            readahead_pages: 0,
        }
    }

    pub fn record_access(&mut self, offset: u64, length: u64) {
        let expected = self.last_offset + self.last_length;
        if offset == expected || (offset > self.last_offset && offset <= expected + 4096) {
            self.sequential_hits += 1;
            // Grow readahead window
            if self.current_window < self.max_window {
                self.current_window = core::cmp::min(self.current_window * 2, self.max_window);
            }
        } else {
            self.random_hits += 1;
            // Shrink readahead window
            if self.current_window > self.min_window {
                self.current_window = core::cmp::max(self.current_window / 2, self.min_window);
            }
        }
        let total = (self.sequential_hits + self.random_hits) as f64;
        if total > 0.0 {
            self.sequential_score = self.sequential_hits as f64 / total;
            self.random_score = self.random_hits as f64 / total;
        }
        self.last_offset = offset;
        self.last_length = length;
    }

    #[inline]
    pub fn apply_willneed(&mut self, length: u64) {
        let pages = (length + 4095) / 4096;
        self.willneed_prefetches += 1;
        self.readahead_pages += pages;
    }

    #[inline]
    pub fn apply_dontneed(&mut self, length: u64) {
        let pages = (length + 4095) / 4096;
        self.dontneed_evictions += 1;
        self.readahead_pages = self.readahead_pages.saturating_sub(pages);
    }
}

/// Statistics for fadvise V3 app.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FadviseV3AppStats {
    pub total_calls: u64,
    pub sequential_advices: u64,
    pub random_advices: u64,
    pub willneed_calls: u64,
    pub dontneed_calls: u64,
    pub noreuse_calls: u64,
    pub total_prefetch_pages: u64,
    pub total_evicted_pages: u64,
}

/// Main apps fadvise V3 manager.
pub struct AppFadviseV3 {
    pub files: BTreeMap<u64, FileReadaheadState>,
    pub recent_records: Vec<FadviseV3Record>,
    pub next_record_id: u64,
    pub stats: FadviseV3AppStats,
}

impl AppFadviseV3 {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            recent_records: Vec::new(),
            next_record_id: 1,
            stats: FadviseV3AppStats {
                total_calls: 0,
                sequential_advices: 0,
                random_advices: 0,
                willneed_calls: 0,
                dontneed_calls: 0,
                noreuse_calls: 0,
                total_prefetch_pages: 0,
                total_evicted_pages: 0,
            },
        }
    }

    pub fn record_fadvise(
        &mut self,
        pid: u64,
        fd: i32,
        inode: u64,
        offset: u64,
        length: u64,
        advice: FadviseV3Advice,
    ) -> u64 {
        let id = self.next_record_id;
        self.next_record_id += 1;
        let state = self.files.entry(inode).or_insert_with(|| FileReadaheadState::new(inode));
        match advice {
            FadviseV3Advice::Sequential => {
                self.stats.sequential_advices += 1;
                state.current_window = state.max_window;
            }
            FadviseV3Advice::Random => {
                self.stats.random_advices += 1;
                state.current_window = state.min_window;
            }
            FadviseV3Advice::WillNeed => {
                self.stats.willneed_calls += 1;
                let pages = (length + 4095) / 4096;
                state.apply_willneed(length);
                self.stats.total_prefetch_pages += pages;
            }
            FadviseV3Advice::DontNeed => {
                self.stats.dontneed_calls += 1;
                let pages = (length + 4095) / 4096;
                state.apply_dontneed(length);
                self.stats.total_evicted_pages += pages;
            }
            FadviseV3Advice::NoReuse => {
                self.stats.noreuse_calls += 1;
            }
            FadviseV3Advice::Normal => {}
        }
        let mut rec = FadviseV3Record::new(id, pid, fd, advice);
        rec.offset = offset;
        rec.length = length;
        self.stats.total_calls += 1;
        self.recent_records.push(rec);
        id
    }

    #[inline(always)]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

// ============================================================================
// Merged from fadvise_v4_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadviseV4Advice {
    Normal,
    Sequential,
    Random,
    WillNeed,
    DontNeed,
    NoReuse,
}

/// Fadvise v4 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadviseV4Result {
    Success,
    BadFd,
    Invalid,
    NoMem,
    Error,
}

/// Fadvise v4 record
#[derive(Debug, Clone)]
pub struct FadviseV4Record {
    pub fd: i32,
    pub advice: FadviseV4Advice,
    pub result: FadviseV4Result,
    pub offset: u64,
    pub length: u64,
}

impl FadviseV4Record {
    pub fn new(fd: i32, advice: FadviseV4Advice) -> Self {
        Self { fd, advice, result: FadviseV4Result::Success, offset: 0, length: 0 }
    }

    #[inline(always)]
    pub fn is_readahead_hint(&self) -> bool {
        matches!(self.advice, FadviseV4Advice::Sequential | FadviseV4Advice::WillNeed)
    }

    #[inline(always)]
    pub fn is_drop_hint(&self) -> bool {
        matches!(self.advice, FadviseV4Advice::DontNeed | FadviseV4Advice::NoReuse)
    }
}

/// Readahead window tracker
#[derive(Debug, Clone)]
pub struct ReadaheadWindow {
    pub fd: i32,
    pub window_start: u64,
    pub window_size: u64,
    pub hits: u64,
    pub misses: u64,
}

impl ReadaheadWindow {
    pub fn new(fd: i32, start: u64, size: u64) -> Self {
        Self { fd, window_start: start, window_size: size, hits: 0, misses: 0 }
    }

    #[inline]
    pub fn check(&mut self, offset: u64) -> bool {
        if offset >= self.window_start && offset < self.window_start + self.window_size {
            self.hits += 1;
            true
        } else {
            self.misses += 1;
            false
        }
    }

    #[inline(always)]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}

/// Fadvise v4 app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FadviseV4AppStats {
    pub total_calls: u64,
    pub sequential_hints: u64,
    pub random_hints: u64,
    pub drop_hints: u64,
    pub errors: u64,
}

/// Main app fadvise v4
#[derive(Debug)]
pub struct AppFadviseV4 {
    pub stats: FadviseV4AppStats,
    pub windows: BTreeMap<i32, ReadaheadWindow>,
}

impl AppFadviseV4 {
    pub fn new() -> Self {
        Self { stats: FadviseV4AppStats { total_calls: 0, sequential_hints: 0, random_hints: 0, drop_hints: 0, errors: 0 }, windows: BTreeMap::new() }
    }

    pub fn record(&mut self, rec: &FadviseV4Record) {
        self.stats.total_calls += 1;
        if rec.result != FadviseV4Result::Success { self.stats.errors += 1; return; }
        match rec.advice {
            FadviseV4Advice::Sequential => {
                self.stats.sequential_hints += 1;
                self.windows.insert(rec.fd, ReadaheadWindow::new(rec.fd, rec.offset, rec.length.max(131072)));
            }
            FadviseV4Advice::Random => self.stats.random_hints += 1,
            FadviseV4Advice::DontNeed | FadviseV4Advice::NoReuse => self.stats.drop_hints += 1,
            _ => {}
        }
    }
}
