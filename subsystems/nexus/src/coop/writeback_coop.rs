// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Writeback (cooperative dirty page writeback)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Writeback trigger reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopWritebackReason {
    Periodic,
    MemoryPressure,
    SyncRequest,
    Threshold,
    BackgroundFlush,
    ForcedFlush,
    Umount,
}

/// Writeback entry for a dirty page/inode
#[derive(Debug, Clone)]
pub struct CoopWritebackEntry {
    pub inode: u64,
    pub dirty_pages: u32,
    pub oldest_dirty_time: u64,
    pub priority: u8,
    pub in_progress: bool,
}

/// Stats for writeback cooperation
#[derive(Debug, Clone)]
pub struct CoopWritebackStats {
    pub total_writebacks: u64,
    pub pages_written: u64,
    pub inodes_cleaned: u64,
    pub pressure_events: u64,
    pub avg_writeback_pages: u64,
    pub writeback_errors: u64,
}

/// Manager for writeback cooperative operations
pub struct CoopWritebackManager {
    dirty_inodes: BTreeMap<u64, CoopWritebackEntry>,
    writeback_queue: Vec<u64>,
    stats: CoopWritebackStats,
    dirty_threshold: u32,
    background_threshold: u32,
}

impl CoopWritebackManager {
    pub fn new() -> Self {
        Self {
            dirty_inodes: BTreeMap::new(),
            writeback_queue: Vec::new(),
            stats: CoopWritebackStats {
                total_writebacks: 0,
                pages_written: 0,
                inodes_cleaned: 0,
                pressure_events: 0,
                avg_writeback_pages: 0,
                writeback_errors: 0,
            },
            dirty_threshold: 40,
            background_threshold: 10,
        }
    }

    pub fn mark_dirty(&mut self, inode: u64, pages: u32) {
        if let Some(entry) = self.dirty_inodes.get_mut(&inode) {
            entry.dirty_pages += pages;
        } else {
            let entry = CoopWritebackEntry {
                inode,
                dirty_pages: pages,
                oldest_dirty_time: inode.wrapping_mul(47),
                priority: 5,
                in_progress: false,
            };
            self.dirty_inodes.insert(inode, entry);
        }
    }

    pub fn schedule_writeback(&mut self, reason: CoopWritebackReason) -> usize {
        if matches!(reason, CoopWritebackReason::MemoryPressure) {
            self.stats.pressure_events += 1;
        }
        let candidates: Vec<u64> = self.dirty_inodes.iter()
            .filter(|(_, e)| !e.in_progress)
            .map(|(&ino, _)| ino)
            .collect();
        let count = candidates.len();
        for ino in &candidates {
            if let Some(entry) = self.dirty_inodes.get_mut(ino) {
                entry.in_progress = true;
            }
            self.writeback_queue.push(*ino);
        }
        count
    }

    pub fn complete_writeback(&mut self, inode: u64) -> bool {
        if let Some(entry) = self.dirty_inodes.remove(&inode) {
            self.stats.total_writebacks += 1;
            self.stats.pages_written += entry.dirty_pages as u64;
            self.stats.inodes_cleaned += 1;
            self.writeback_queue.retain(|&i| i != inode);
            true
        } else {
            false
        }
    }

    pub fn dirty_count(&self) -> usize {
        self.dirty_inodes.len()
    }

    pub fn set_thresholds(&mut self, dirty: u32, background: u32) {
        self.dirty_threshold = dirty;
        self.background_threshold = background;
    }

    pub fn stats(&self) -> &CoopWritebackStats {
        &self.stats
    }
}
