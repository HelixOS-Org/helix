// SPDX-License-Identifier: GPL-2.0
//! Bridge readahead — file readahead advice with adaptive window tuning

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Readahead pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadaheadPattern {
    Sequential,
    Random,
    Strided,
    Interleaved,
    Unknown,
}

/// Readahead state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadaheadState {
    Disabled,
    Initial,
    Active,
    Async,
    Thrashing,
}

/// File readahead context
#[derive(Debug, Clone)]
pub struct ReadaheadContext {
    pub fd: i32,
    pub state: ReadaheadState,
    pub pattern: ReadaheadPattern,
    pub window_pages: u32,
    pub max_window_pages: u32,
    pub start_page: u64,
    pub async_size: u32,
    pub prev_offset: u64,
    pub reads: u64,
    pub hits: u64,
    pub misses: u64,
    pub pages_fetched: u64,
    pub pages_used: u64,
    pub stride: u64,
}

impl ReadaheadContext {
    pub fn new(fd: i32, max_window_pages: u32) -> Self {
        Self {
            fd,
            state: ReadaheadState::Initial,
            pattern: ReadaheadPattern::Unknown,
            window_pages: 4,
            max_window_pages,
            start_page: 0,
            async_size: 0,
            prev_offset: 0,
            reads: 0,
            hits: 0,
            misses: 0,
            pages_fetched: 0,
            pages_used: 0,
            stride: 0,
        }
    }

    pub fn record_read(&mut self, offset: u64, len: u32) {
        self.reads += 1;
        let page = offset / 4096;
        if self.reads > 1 {
            if offset == self.prev_offset + len as u64 || offset == self.prev_offset {
                self.pattern = ReadaheadPattern::Sequential;
                self.grow_window();
            } else if offset > self.prev_offset {
                let diff = offset - self.prev_offset;
                if self.stride > 0 && diff == self.stride {
                    self.pattern = ReadaheadPattern::Strided;
                } else {
                    self.stride = diff;
                    self.pattern = ReadaheadPattern::Random;
                    self.shrink_window();
                }
            } else {
                self.pattern = ReadaheadPattern::Random;
                self.shrink_window();
            }
        }
        if page >= self.start_page && page < self.start_page + self.window_pages as u64 {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
        self.prev_offset = offset;
    }

    pub fn grow_window(&mut self) {
        if self.window_pages < self.max_window_pages {
            self.window_pages = (self.window_pages * 2).min(self.max_window_pages);
            self.state = ReadaheadState::Active;
        }
    }

    pub fn shrink_window(&mut self) {
        self.window_pages = (self.window_pages / 2).max(4);
        if self.window_pages <= 4 {
            self.state = ReadaheadState::Thrashing;
        }
    }

    pub fn trigger_async(&mut self) {
        self.state = ReadaheadState::Async;
        self.async_size = self.window_pages / 4;
        self.pages_fetched += self.window_pages as u64;
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }

    pub fn waste_pct(&self) -> f64 {
        if self.pages_fetched == 0 { 0.0 }
        else {
            let unused = self.pages_fetched.saturating_sub(self.pages_used);
            (unused as f64 / self.pages_fetched as f64) * 100.0
        }
    }
}

/// Readahead bridge stats
#[derive(Debug, Clone)]
pub struct ReadaheadBridgeStats {
    pub total_files: u64,
    pub total_reads: u64,
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_pages_fetched: u64,
}

/// Main bridge readahead
#[derive(Debug)]
pub struct BridgeReadahead {
    pub contexts: BTreeMap<i32, ReadaheadContext>,
    pub stats: ReadaheadBridgeStats,
    pub default_max_window: u32,
}

impl BridgeReadahead {
    pub fn new(default_max_window: u32) -> Self {
        Self {
            contexts: BTreeMap::new(),
            stats: ReadaheadBridgeStats {
                total_files: 0,
                total_reads: 0,
                total_hits: 0,
                total_misses: 0,
                total_pages_fetched: 0,
            },
            default_max_window,
        }
    }

    pub fn get_or_create(&mut self, fd: i32) -> &mut ReadaheadContext {
        if !self.contexts.contains_key(&fd) {
            self.contexts.insert(fd, ReadaheadContext::new(fd, self.default_max_window));
            self.stats.total_files += 1;
        }
        self.contexts.get_mut(&fd).unwrap()
    }

    pub fn record_read(&mut self, fd: i32, offset: u64, len: u32) {
        self.stats.total_reads += 1;
        let ctx = self.get_or_create(fd);
        ctx.record_read(offset, len);
    }

    pub fn advise(&mut self, fd: i32, offset: u64, len: u64) {
        let ctx = self.get_or_create(fd);
        ctx.start_page = offset / 4096;
        ctx.window_pages = (len / 4096) as u32;
        ctx.state = ReadaheadState::Active;
    }

    pub fn overall_hit_rate(&self) -> f64 {
        let total = self.stats.total_hits + self.stats.total_misses;
        if total == 0 { 0.0 } else { self.stats.total_hits as f64 / total as f64 }
    }
}
