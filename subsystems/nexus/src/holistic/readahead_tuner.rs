//! # Holistic Readahead Tuner
//!
//! Adaptive file readahead management:
//! - Per-file sequential detection
//! - Window size tuning based on access patterns
//! - Interleaved stream tracking
//! - Readahead hit/miss ratio monitoring
//! - Throughput-aware window scaling
//! - Memory pressure integration

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Access pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPattern {
    Sequential,
    Strided,
    Random,
    Interleaved,
    Unknown,
}

/// Readahead state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadaheadState {
    Initial,
    Sampling,
    Active,
    Throttled,
    Disabled,
}

/// Per-file readahead context
#[derive(Debug, Clone)]
pub struct FileReadahead {
    pub file_id: u64,
    pub pattern: AccessPattern,
    pub state: ReadaheadState,
    pub window_pages: u32,
    pub max_window_pages: u32,
    pub min_window_pages: u32,
    pub last_offset: u64,
    pub last_stride: i64,
    pub sequential_count: u32,
    pub random_count: u32,
    pub ra_hits: u64,
    pub ra_misses: u64,
    pub pages_read: u64,
    pub pages_prefetched: u64,
    pub last_access_ts: u64,
}

impl FileReadahead {
    pub fn new(file_id: u64) -> Self {
        Self {
            file_id, pattern: AccessPattern::Unknown, state: ReadaheadState::Initial,
            window_pages: 4, max_window_pages: 256, min_window_pages: 2,
            last_offset: 0, last_stride: 0, sequential_count: 0, random_count: 0,
            ra_hits: 0, ra_misses: 0, pages_read: 0, pages_prefetched: 0,
            last_access_ts: 0,
        }
    }

    pub fn record_access(&mut self, offset: u64, ts: u64) {
        let stride = offset as i64 - self.last_offset as i64;
        self.last_access_ts = ts;

        if self.last_offset > 0 {
            let page_stride = stride / 4096;
            if page_stride == 1 || page_stride == self.last_stride / 4096 {
                self.sequential_count += 1;
                self.random_count = 0;
            } else if libm::fabs(page_stride as f64 - self.last_stride as f64 / 4096.0) < 2.0 {
                self.sequential_count += 1;
            } else {
                self.random_count += 1;
                self.sequential_count = 0;
            }
        }

        self.last_stride = stride;
        self.last_offset = offset;
        self.pages_read += 1;
        self.classify_pattern();
    }

    fn classify_pattern(&mut self) {
        if self.sequential_count > 4 {
            self.pattern = AccessPattern::Sequential;
            self.state = ReadaheadState::Active;
        } else if self.random_count > 4 {
            self.pattern = AccessPattern::Random;
            self.state = ReadaheadState::Throttled;
        } else {
            self.pattern = AccessPattern::Unknown;
            self.state = ReadaheadState::Sampling;
        }
    }

    pub fn hit_ratio(&self) -> f64 {
        let total = self.ra_hits + self.ra_misses;
        if total == 0 { 0.0 } else { self.ra_hits as f64 / total as f64 }
    }

    pub fn adjust_window(&mut self) {
        match self.pattern {
            AccessPattern::Sequential => {
                let ratio = self.hit_ratio();
                if ratio > 0.8 {
                    self.window_pages = (self.window_pages * 2).min(self.max_window_pages);
                } else if ratio < 0.3 {
                    self.window_pages = (self.window_pages / 2).max(self.min_window_pages);
                }
            }
            AccessPattern::Random => {
                self.window_pages = self.min_window_pages;
            }
            AccessPattern::Strided => {
                let ratio = self.hit_ratio();
                if ratio > 0.6 {
                    self.window_pages = (self.window_pages + 4).min(self.max_window_pages / 2);
                }
            }
            _ => {}
        }
    }

    pub fn record_hit(&mut self) { self.ra_hits += 1; self.pages_prefetched += 1; }
    pub fn record_miss(&mut self) { self.ra_misses += 1; }
}

/// Interleaved stream detection
#[derive(Debug, Clone)]
pub struct InterleavedStream {
    pub stream_id: u64,
    pub file_ids: Vec<u64>,
    pub access_sequence: Vec<u64>,
    pub max_sequence: usize,
    pub detected_period: u32,
}

impl InterleavedStream {
    pub fn new(stream_id: u64) -> Self {
        Self { stream_id, file_ids: Vec::new(), access_sequence: Vec::new(), max_sequence: 64, detected_period: 0 }
    }

    pub fn record_file_access(&mut self, file_id: u64) {
        self.access_sequence.push(file_id);
        if self.access_sequence.len() > self.max_sequence {
            self.access_sequence.remove(0);
        }
        if !self.file_ids.contains(&file_id) {
            self.file_ids.push(file_id);
        }
        self.detect_period();
    }

    fn detect_period(&mut self) {
        let len = self.access_sequence.len();
        if len < 4 { return; }
        for period in 2..=(len / 2).min(8) {
            let mut matches = 0u32;
            let mut total = 0u32;
            for i in period..len {
                total += 1;
                if self.access_sequence[i] == self.access_sequence[i - period] { matches += 1; }
            }
            if total > 0 && matches as f64 / total as f64 > 0.7 {
                self.detected_period = period as u32;
                return;
            }
        }
        self.detected_period = 0;
    }

    pub fn is_periodic(&self) -> bool { self.detected_period > 0 }
}

/// Readahead tuner stats
#[derive(Debug, Clone, Default)]
pub struct ReadaheadStats {
    pub files_tracked: usize,
    pub sequential_files: usize,
    pub random_files: usize,
    pub avg_window_pages: f64,
    pub overall_hit_ratio: f64,
    pub total_pages_prefetched: u64,
    pub total_ra_hits: u64,
    pub total_ra_misses: u64,
    pub interleaved_streams: usize,
    pub memory_pressure_throttled: u64,
}

/// Holistic readahead tuner
pub struct HolisticReadaheadTuner {
    files: BTreeMap<u64, FileReadahead>,
    streams: BTreeMap<u64, InterleavedStream>,
    stats: ReadaheadStats,
    pressure_level: f64,
    pressure_throttle_threshold: f64,
    next_stream_id: u64,
}

impl HolisticReadaheadTuner {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(), streams: BTreeMap::new(),
            stats: ReadaheadStats::default(), pressure_level: 0.0,
            pressure_throttle_threshold: 0.8, next_stream_id: 1,
        }
    }

    pub fn track_file(&mut self, file_id: u64) {
        self.files.entry(file_id).or_insert_with(|| FileReadahead::new(file_id));
    }

    pub fn record_access(&mut self, file_id: u64, offset: u64, ts: u64) {
        self.track_file(file_id);
        if let Some(f) = self.files.get_mut(&file_id) {
            f.record_access(offset, ts);
        }
    }

    pub fn record_hit(&mut self, file_id: u64) {
        if let Some(f) = self.files.get_mut(&file_id) { f.record_hit(); }
    }

    pub fn record_miss(&mut self, file_id: u64) {
        if let Some(f) = self.files.get_mut(&file_id) { f.record_miss(); }
    }

    pub fn set_pressure(&mut self, pressure: f64) {
        self.pressure_level = pressure;
    }

    pub fn get_window(&self, file_id: u64) -> u32 {
        if self.pressure_level > self.pressure_throttle_threshold {
            return 2;
        }
        self.files.get(&file_id).map(|f| f.window_pages).unwrap_or(4)
    }

    pub fn tune_all(&mut self) {
        let throttled = self.pressure_level > self.pressure_throttle_threshold;
        let addrs: Vec<u64> = self.files.keys().copied().collect();
        for fid in addrs {
            if let Some(f) = self.files.get_mut(&fid) {
                if throttled {
                    f.window_pages = f.min_window_pages;
                    f.state = ReadaheadState::Throttled;
                } else {
                    f.adjust_window();
                }
            }
        }
    }

    pub fn create_stream(&mut self) -> u64 {
        let id = self.next_stream_id;
        self.next_stream_id += 1;
        self.streams.insert(id, InterleavedStream::new(id));
        id
    }

    pub fn record_stream_access(&mut self, stream_id: u64, file_id: u64) {
        if let Some(s) = self.streams.get_mut(&stream_id) {
            s.record_file_access(file_id);
        }
    }

    pub fn recompute(&mut self) {
        self.stats.files_tracked = self.files.len();
        self.stats.sequential_files = self.files.values().filter(|f| f.pattern == AccessPattern::Sequential).count();
        self.stats.random_files = self.files.values().filter(|f| f.pattern == AccessPattern::Random).count();
        let windows: Vec<f64> = self.files.values().map(|f| f.window_pages as f64).collect();
        self.stats.avg_window_pages = if windows.is_empty() { 0.0 } else { windows.iter().sum::<f64>() / windows.len() as f64 };
        self.stats.total_ra_hits = self.files.values().map(|f| f.ra_hits).sum();
        self.stats.total_ra_misses = self.files.values().map(|f| f.ra_misses).sum();
        let total_ra = self.stats.total_ra_hits + self.stats.total_ra_misses;
        self.stats.overall_hit_ratio = if total_ra == 0 { 0.0 } else { self.stats.total_ra_hits as f64 / total_ra as f64 };
        self.stats.total_pages_prefetched = self.files.values().map(|f| f.pages_prefetched).sum();
        self.stats.interleaved_streams = self.streams.values().filter(|s| s.is_periodic()).count();
    }

    pub fn file(&self, id: u64) -> Option<&FileReadahead> { self.files.get(&id) }
    pub fn stats(&self) -> &ReadaheadStats { &self.stats }
}
