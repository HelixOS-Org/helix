// SPDX-License-Identifier: GPL-2.0
//! Holistic readahead — readahead pattern detection and efficiency analysis

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Readahead pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadaheadPattern {
    Sequential,
    Interleaved,
    Random,
    Strided,
    Unknown,
}

/// Readahead state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadaheadState {
    Idle,
    Async,
    Sync,
    Throttled,
}

/// Per-file readahead window
#[derive(Debug, Clone)]
pub struct HolisticRaWindow {
    pub inode: u64,
    pub start: u64,
    pub size: u64,
    pub async_size: u64,
    pub pattern: ReadaheadPattern,
    pub state: ReadaheadState,
    pub pages_issued: u64,
    pub pages_used: u64,
    pub wasted_pages: u64,
}

impl HolisticRaWindow {
    pub fn new(inode: u64) -> Self {
        Self {
            inode,
            start: 0,
            size: 32,
            async_size: 16,
            pattern: ReadaheadPattern::Unknown,
            state: ReadaheadState::Idle,
            pages_issued: 0,
            pages_used: 0,
            wasted_pages: 0,
        }
    }

    pub fn issue(&mut self, pages: u64) {
        self.pages_issued += pages;
        self.state = ReadaheadState::Async;
    }

    pub fn use_page(&mut self) {
        self.pages_used += 1;
    }
    pub fn waste(&mut self, pages: u64) {
        self.wasted_pages += pages;
    }

    pub fn efficiency(&self) -> f64 {
        if self.pages_issued == 0 {
            0.0
        } else {
            self.pages_used as f64 / self.pages_issued as f64
        }
    }

    pub fn adjust_window(&mut self) {
        let eff = self.efficiency();
        if eff > 0.8 {
            self.size = (self.size * 2).min(256);
        } else if eff < 0.3 {
            self.size = (self.size / 2).max(4);
        }
    }
}

/// Holistic readahead stats
#[derive(Debug, Clone)]
pub struct HolisticReadaheadStats {
    pub total_issues: u64,
    pub total_pages_issued: u64,
    pub total_pages_used: u64,
    pub total_wasted: u64,
    pub sequential_detected: u64,
    pub random_detected: u64,
}

/// Main holistic readahead
#[derive(Debug)]
pub struct HolisticReadahead {
    pub windows: BTreeMap<u64, HolisticRaWindow>,
    pub stats: HolisticReadaheadStats,
}

impl HolisticReadahead {
    pub fn new() -> Self {
        Self {
            windows: BTreeMap::new(),
            stats: HolisticReadaheadStats {
                total_issues: 0,
                total_pages_issued: 0,
                total_pages_used: 0,
                total_wasted: 0,
                sequential_detected: 0,
                random_detected: 0,
            },
        }
    }

    pub fn issue_readahead(&mut self, inode: u64, pages: u64) {
        self.stats.total_issues += 1;
        self.stats.total_pages_issued += pages;
        let w = self
            .windows
            .entry(inode)
            .or_insert_with(|| HolisticRaWindow::new(inode));
        w.issue(pages);
    }

    pub fn record_use(&mut self, inode: u64) {
        self.stats.total_pages_used += 1;
        if let Some(w) = self.windows.get_mut(&inode) {
            w.use_page();
        }
    }

    pub fn overall_efficiency(&self) -> f64 {
        if self.stats.total_pages_issued == 0 {
            0.0
        } else {
            self.stats.total_pages_used as f64 / self.stats.total_pages_issued as f64
        }
    }
}
