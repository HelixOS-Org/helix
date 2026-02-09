// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Lseek (file seek operations)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Seek origin type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSeekWhence {
    Set,
    Current,
    End,
    Data,
    Hole,
}

/// Seek operation record
#[derive(Debug, Clone)]
pub struct AppSeekRecord {
    pub fd: u64,
    pub whence: AppSeekWhence,
    pub offset: i64,
    pub resulting_position: u64,
    pub timestamp: u64,
}

/// Statistics for seek operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppSeekStats {
    pub total_seeks: u64,
    pub sequential_seeks: u64,
    pub random_seeks: u64,
    pub seek_errors: u64,
    pub avg_seek_distance: u64,
}

/// File position tracker
#[derive(Debug, Clone)]
pub struct AppFilePosition {
    pub fd: u64,
    pub current_offset: u64,
    pub file_size: u64,
    pub last_seek_whence: AppSeekWhence,
}

/// Manager for application seek operations
pub struct AppLseekManager {
    positions: BTreeMap<u64, AppFilePosition>,
    history: BTreeMap<u64, AppSeekRecord>,
    next_record: u64,
    stats: AppSeekStats,
}

impl AppLseekManager {
    pub fn new() -> Self {
        Self {
            positions: BTreeMap::new(),
            history: BTreeMap::new(),
            next_record: 1,
            stats: AppSeekStats {
                total_seeks: 0,
                sequential_seeks: 0,
                random_seeks: 0,
                seek_errors: 0,
                avg_seek_distance: 0,
            },
        }
    }

    #[inline]
    pub fn register_fd(&mut self, fd: u64, file_size: u64) {
        let pos = AppFilePosition {
            fd,
            current_offset: 0,
            file_size,
            last_seek_whence: AppSeekWhence::Set,
        };
        self.positions.insert(fd, pos);
    }

    pub fn seek(&mut self, fd: u64, whence: AppSeekWhence, offset: i64) -> Option<u64> {
        if let Some(pos) = self.positions.get_mut(&fd) {
            let new_offset = match whence {
                AppSeekWhence::Set => {
                    if offset < 0 { return None; }
                    offset as u64
                }
                AppSeekWhence::Current => {
                    let cur = pos.current_offset as i64;
                    let new = cur.wrapping_add(offset);
                    if new < 0 { return None; }
                    new as u64
                }
                AppSeekWhence::End => {
                    let end = pos.file_size as i64;
                    let new = end.wrapping_add(offset);
                    if new < 0 { return None; }
                    new as u64
                }
                AppSeekWhence::Data | AppSeekWhence::Hole => {
                    if offset < 0 { return None; }
                    offset as u64
                }
            };
            let distance = if new_offset > pos.current_offset {
                new_offset - pos.current_offset
            } else {
                pos.current_offset - new_offset
            };
            if distance <= 4096 {
                self.stats.sequential_seeks += 1;
            } else {
                self.stats.random_seeks += 1;
            }
            pos.current_offset = new_offset;
            pos.last_seek_whence = whence;
            let rec = AppSeekRecord {
                fd,
                whence,
                offset,
                resulting_position: new_offset,
                timestamp: self.next_record.wrapping_mul(41),
            };
            self.history.insert(self.next_record, rec);
            self.next_record += 1;
            self.stats.total_seeks += 1;
            Some(new_offset)
        } else {
            self.stats.seek_errors += 1;
            None
        }
    }

    #[inline(always)]
    pub fn get_position(&self, fd: u64) -> Option<u64> {
        self.positions.get(&fd).map(|p| p.current_offset)
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppSeekStats {
        &self.stats
    }
}
