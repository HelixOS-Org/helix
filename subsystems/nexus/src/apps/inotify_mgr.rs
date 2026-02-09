// SPDX-License-Identifier: GPL-2.0
//! Apps inotify_mgr — inotify filesystem watch manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Inotify event mask
#[derive(Debug, Clone, Copy)]
pub struct InotifyMask {
    pub bits: u32,
}

impl InotifyMask {
    pub const ACCESS: u32 = 0x00000001;
    pub const MODIFY: u32 = 0x00000002;
    pub const ATTRIB: u32 = 0x00000004;
    pub const CLOSE_WRITE: u32 = 0x00000008;
    pub const CLOSE_NOWRITE: u32 = 0x00000010;
    pub const OPEN: u32 = 0x00000020;
    pub const MOVED_FROM: u32 = 0x00000040;
    pub const MOVED_TO: u32 = 0x00000080;
    pub const CREATE: u32 = 0x00000100;
    pub const DELETE: u32 = 0x00000200;
    pub const DELETE_SELF: u32 = 0x00000400;
    pub const MOVE_SELF: u32 = 0x00000800;
    pub const ISDIR: u32 = 0x40000000;
    pub const ONESHOT: u32 = 0x80000000;

    pub fn new(bits: u32) -> Self { Self { bits } }
    #[inline(always)]
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
    #[inline(always)]
    pub fn is_oneshot(&self) -> bool { self.has(Self::ONESHOT) }
}

/// Watch descriptor
#[derive(Debug, Clone)]
pub struct InotifyWatch {
    pub wd: i32,
    pub inode: u64,
    pub mask: InotifyMask,
    pub active: bool,
    pub events_delivered: u64,
    pub created_at: u64,
}

impl InotifyWatch {
    pub fn new(wd: i32, inode: u64, mask: InotifyMask, now: u64) -> Self {
        Self { wd, inode, mask, active: true, events_delivered: 0, created_at: now }
    }

    #[inline(always)]
    pub fn matches(&self, event_mask: u32) -> bool {
        self.active && (self.mask.bits & event_mask) != 0
    }

    #[inline(always)]
    pub fn deliver(&mut self) {
        self.events_delivered += 1;
        if self.mask.is_oneshot() { self.active = false; }
    }
}

/// Inotify event
#[derive(Debug, Clone)]
pub struct InotifyEvent {
    pub wd: i32,
    pub mask: u32,
    pub cookie: u32,
    pub name_hash: u64,
    pub timestamp: u64,
}

/// Inotify instance (fd)
#[derive(Debug)]
pub struct InotifyInstance {
    pub id: u64,
    pub pid: u64,
    pub watches: BTreeMap<i32, InotifyWatch>,
    pub event_queue: Vec<InotifyEvent>,
    pub queue_max: u32,
    pub next_wd: i32,
    pub overflow_count: u64,
    pub created_at: u64,
}

impl InotifyInstance {
    pub fn new(id: u64, pid: u64, now: u64) -> Self {
        Self {
            id, pid, watches: BTreeMap::new(), event_queue: Vec::new(),
            queue_max: 16384, next_wd: 1, overflow_count: 0, created_at: now,
        }
    }

    #[inline]
    pub fn add_watch(&mut self, inode: u64, mask: InotifyMask, now: u64) -> i32 {
        let wd = self.next_wd;
        self.next_wd += 1;
        self.watches.insert(wd, InotifyWatch::new(wd, inode, mask, now));
        wd
    }

    #[inline(always)]
    pub fn remove_watch(&mut self, wd: i32) -> bool { self.watches.remove(&wd).is_some() }

    pub fn deliver_event(&mut self, inode: u64, mask: u32, cookie: u32, name_hash: u64, now: u64) -> u32 {
        let mut delivered = 0u32;
        let wds: Vec<i32> = self.watches.iter()
            .filter(|(_, w)| w.inode == inode && w.matches(mask))
            .map(|(&wd, _)| wd).collect();
        for wd in wds {
            if self.event_queue.len() as u32 >= self.queue_max {
                self.overflow_count += 1;
                break;
            }
            self.event_queue.push(InotifyEvent { wd, mask, cookie, name_hash, timestamp: now });
            if let Some(w) = self.watches.get_mut(&wd) { w.deliver(); }
            delivered += 1;
        }
        delivered
    }

    #[inline(always)]
    pub fn read_events(&mut self, max: usize) -> Vec<InotifyEvent> {
        let n = max.min(self.event_queue.len());
        self.event_queue.drain(..n).collect()
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct InotifyMgrStats {
    pub total_instances: u32,
    pub total_watches: u32,
    pub total_events_queued: u64,
    pub total_overflows: u64,
}

/// Main inotify manager
pub struct AppInotifyMgr {
    instances: BTreeMap<u64, InotifyInstance>,
    next_id: u64,
}

impl AppInotifyMgr {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, pid: u64, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.instances.insert(id, InotifyInstance::new(id, pid, now));
        id
    }

    #[inline(always)]
    pub fn add_watch(&mut self, inst_id: u64, inode: u64, mask: u32, now: u64) -> Option<i32> {
        self.instances.get_mut(&inst_id).map(|i| i.add_watch(inode, InotifyMask::new(mask), now))
    }

    #[inline]
    pub fn stats(&self) -> InotifyMgrStats {
        let watches: u32 = self.instances.values().map(|i| i.watches.len() as u32).sum();
        let queued: u64 = self.instances.values().map(|i| i.event_queue.len() as u64).sum();
        let overflows: u64 = self.instances.values().map(|i| i.overflow_count).sum();
        InotifyMgrStats {
            total_instances: self.instances.len() as u32, total_watches: watches,
            total_events_queued: queued, total_overflows: overflows,
        }
    }
}
