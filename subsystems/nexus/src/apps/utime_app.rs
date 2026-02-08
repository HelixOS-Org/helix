// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps utime — File timestamp modification tracking
//!
//! Tracks utimensat/futimens/utimes/utime calls with nanosecond precision,
//! UTIME_NOW / UTIME_OMIT handling, and timestamp anomaly detection.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Utime call variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtimeVariant {
    Utime,
    Utimes,
    Futimens,
    Utimensat,
}

/// Timestamp special value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtimeSpecial {
    Explicit(u64, u32), // sec, nsec
    Now,
    Omit,
}

/// Utime result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtimeResult {
    Success,
    Permission,
    NotFound,
    ReadOnly,
    InvalidArg,
    BadFd,
}

/// A utime operation record.
#[derive(Debug, Clone)]
pub struct UtimeRecord {
    pub record_id: u64,
    pub pid: u64,
    pub variant: UtimeVariant,
    pub path: Option<String>,
    pub fd: Option<i32>,
    pub atime: UtimeSpecial,
    pub mtime: UtimeSpecial,
    pub result: UtimeResult,
    pub timestamp: u64,
}

impl UtimeRecord {
    pub fn new(record_id: u64, pid: u64, variant: UtimeVariant) -> Self {
        Self {
            record_id,
            pid,
            variant,
            path: None,
            fd: None,
            atime: UtimeSpecial::Now,
            mtime: UtimeSpecial::Now,
            result: UtimeResult::Success,
            timestamp: 0,
        }
    }
}

/// Per-file timestamp change tracking.
#[derive(Debug, Clone)]
pub struct FileTimestampState {
    pub inode: u64,
    pub last_atime_sec: u64,
    pub last_mtime_sec: u64,
    pub change_count: u64,
    pub backdated_count: u64,
    pub touch_count: u64,
}

impl FileTimestampState {
    pub fn new(inode: u64) -> Self {
        Self {
            inode,
            last_atime_sec: 0,
            last_mtime_sec: 0,
            change_count: 0,
            backdated_count: 0,
            touch_count: 0,
        }
    }

    pub fn update(&mut self, atime: &UtimeSpecial, mtime: &UtimeSpecial, current_time: u64) {
        self.change_count += 1;
        if let UtimeSpecial::Explicit(sec, _) = mtime {
            if *sec < self.last_mtime_sec {
                self.backdated_count += 1;
            }
            self.last_mtime_sec = *sec;
        }
        if let UtimeSpecial::Now = atime {
            if let UtimeSpecial::Now = mtime {
                self.touch_count += 1;
            }
        }
        if let UtimeSpecial::Explicit(sec, _) = atime {
            self.last_atime_sec = *sec;
        }
        if let UtimeSpecial::Now = atime {
            self.last_atime_sec = current_time;
        }
        if let UtimeSpecial::Now = mtime {
            self.last_mtime_sec = current_time;
        }
    }
}

/// Statistics for utime app.
#[derive(Debug, Clone)]
pub struct UtimeAppStats {
    pub total_calls: u64,
    pub utimensat_calls: u64,
    pub futimens_calls: u64,
    pub touch_operations: u64,
    pub backdated_operations: u64,
    pub failures: u64,
    pub utime_now_count: u64,
    pub utime_omit_count: u64,
}

/// Main apps utime manager.
pub struct AppUtime {
    pub files: BTreeMap<u64, FileTimestampState>,
    pub recent_records: Vec<UtimeRecord>,
    pub next_record_id: u64,
    pub stats: UtimeAppStats,
}

impl AppUtime {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            recent_records: Vec::new(),
            next_record_id: 1,
            stats: UtimeAppStats {
                total_calls: 0,
                utimensat_calls: 0,
                futimens_calls: 0,
                touch_operations: 0,
                backdated_operations: 0,
                failures: 0,
                utime_now_count: 0,
                utime_omit_count: 0,
            },
        }
    }

    pub fn record_utime(
        &mut self,
        pid: u64,
        variant: UtimeVariant,
        inode: u64,
        atime: UtimeSpecial,
        mtime: UtimeSpecial,
        current_time: u64,
    ) -> u64 {
        let id = self.next_record_id;
        self.next_record_id += 1;
        let state = self.files.entry(inode).or_insert_with(|| FileTimestampState::new(inode));
        state.update(&atime, &mtime, current_time);
        let mut rec = UtimeRecord::new(id, pid, variant);
        rec.atime = atime;
        rec.mtime = mtime;
        self.stats.total_calls += 1;
        match variant {
            UtimeVariant::Utimensat => self.stats.utimensat_calls += 1,
            UtimeVariant::Futimens => self.stats.futimens_calls += 1,
            _ => {}
        }
        if matches!(atime, UtimeSpecial::Now) { self.stats.utime_now_count += 1; }
        if matches!(mtime, UtimeSpecial::Now) { self.stats.utime_now_count += 1; }
        if matches!(atime, UtimeSpecial::Omit) { self.stats.utime_omit_count += 1; }
        if matches!(mtime, UtimeSpecial::Omit) { self.stats.utime_omit_count += 1; }
        self.recent_records.push(rec);
        id
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}
