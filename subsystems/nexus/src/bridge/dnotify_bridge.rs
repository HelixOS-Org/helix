// SPDX-License-Identifier: GPL-2.0
//! Bridge dnotify_bridge — directory notification bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Directory event mask
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnotifyMask(pub u32);

impl DnotifyMask {
    pub const ACCESS: u32 = 1 << 0;
    pub const MODIFY: u32 = 1 << 1;
    pub const CREATE: u32 = 1 << 2;
    pub const DELETE: u32 = 1 << 3;
    pub const RENAME: u32 = 1 << 4;
    pub const ATTRIB: u32 = 1 << 5;

    pub fn new() -> Self { Self(0) }
    #[inline(always)]
    pub fn all() -> Self { Self(0x3F) }
    #[inline(always)]
    pub fn set(&mut self, flag: u32) { self.0 |= flag; }
    #[inline(always)]
    pub fn has(&self, flag: u32) -> bool { self.0 & flag != 0 }
    #[inline(always)]
    pub fn matches(&self, event: u32) -> bool { self.0 & event != 0 }
}

/// Watch entry
#[derive(Debug)]
pub struct DnotifyWatch {
    pub id: u64,
    pub dir_fd: i32,
    pub dir_inode: u64,
    pub mask: DnotifyMask,
    pub owner_pid: u64,
    pub signal: i32,
    pub events_delivered: u64,
    pub created_at: u64,
}

impl DnotifyWatch {
    pub fn new(id: u64, fd: i32, inode: u64, mask: DnotifyMask, pid: u64, now: u64) -> Self {
        Self { id, dir_fd: fd, dir_inode: inode, mask, owner_pid: pid, signal: 29, events_delivered: 0, created_at: now }
    }

    #[inline(always)]
    pub fn deliver(&mut self, event_mask: u32) -> bool {
        if self.mask.matches(event_mask) { self.events_delivered += 1; true } else { false }
    }
}

/// Directory event
#[derive(Debug, Clone)]
pub struct DnotifyEvent {
    pub watch_id: u64,
    pub event_mask: u32,
    pub filename_hash: u64,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DnotifyBridgeStats {
    pub total_watches: u32,
    pub total_events: u64,
    pub active_directories: u32,
    pub avg_events_per_watch: f64,
}

/// Main dnotify bridge
#[repr(align(64))]
pub struct BridgeDnotify {
    watches: BTreeMap<u64, DnotifyWatch>,
    events: Vec<DnotifyEvent>,
    next_id: u64,
    max_events: usize,
}

impl BridgeDnotify {
    pub fn new() -> Self { Self { watches: BTreeMap::new(), events: Vec::new(), next_id: 1, max_events: 4096 } }

    #[inline]
    pub fn add_watch(&mut self, fd: i32, inode: u64, mask: DnotifyMask, pid: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.watches.insert(id, DnotifyWatch::new(id, fd, inode, mask, pid, now));
        id
    }

    #[inline(always)]
    pub fn remove_watch(&mut self, id: u64) { self.watches.remove(&id); }

    pub fn notify(&mut self, inode: u64, event_mask: u32, filename_hash: u64, now: u64) {
        let matching: Vec<u64> = self.watches.iter()
            .filter(|(_, w)| w.dir_inode == inode).map(|(&id, _)| id).collect();
        for id in matching {
            if let Some(w) = self.watches.get_mut(&id) {
                if w.deliver(event_mask) {
                    if self.events.len() >= self.max_events { self.events.drain(..self.max_events / 2); }
                    self.events.push(DnotifyEvent { watch_id: id, event_mask, filename_hash, timestamp: now });
                }
            }
        }
    }

    #[inline]
    pub fn stats(&self) -> DnotifyBridgeStats {
        let dirs: u32 = {
            let mut inodes = Vec::new();
            for w in self.watches.values() { if !inodes.contains(&w.dir_inode) { inodes.push(w.dir_inode); } }
            inodes.len() as u32
        };
        let events: u64 = self.watches.values().map(|w| w.events_delivered).sum();
        let avg = if self.watches.is_empty() { 0.0 } else { events as f64 / self.watches.len() as f64 };
        DnotifyBridgeStats { total_watches: self.watches.len() as u32, total_events: events, active_directories: dirs, avg_events_per_watch: avg }
    }
}
