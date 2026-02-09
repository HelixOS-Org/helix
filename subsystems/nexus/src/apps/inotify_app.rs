// SPDX-License-Identifier: GPL-2.0
//! Apps inotify_app — inotify file event monitoring application layer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Inotify event mask
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InotifyAppMask {
    Access,
    Modify,
    Attrib,
    CloseWrite,
    CloseNoWrite,
    Open,
    MovedFrom,
    MovedTo,
    Create,
    Delete,
    DeleteSelf,
    MoveSelf,
    IsDir,
    OneShot,
}

/// Watch descriptor
#[derive(Debug)]
pub struct InotifyWatch {
    pub wd: u64,
    pub path_hash: u64,
    pub mask: u32,
    pub added_at: u64,
    pub event_count: u64,
}

impl InotifyWatch {
    pub fn new(wd: u64, path_hash: u64, mask: u32, now: u64) -> Self {
        Self { wd, path_hash, mask, added_at: now, event_count: 0 }
    }
}

/// Inotify event
#[derive(Debug)]
pub struct InotifyAppEvent {
    pub wd: u64,
    pub mask: u32,
    pub cookie: u32,
    pub name_hash: u64,
    pub timestamp: u64,
}

/// Inotify instance
#[derive(Debug)]
pub struct InotifyAppInstance {
    pub fd: u64,
    pub watches: BTreeMap<u64, InotifyWatch>,
    pub event_queue: Vec<InotifyAppEvent>,
    pub max_queued: u32,
    pub overflow_count: u64,
    pub total_events: u64,
}

impl InotifyAppInstance {
    pub fn new(fd: u64, max_q: u32) -> Self {
        Self { fd, watches: BTreeMap::new(), event_queue: Vec::new(), max_queued: max_q, overflow_count: 0, total_events: 0 }
    }

    #[inline(always)]
    pub fn add_watch(&mut self, wd: u64, path_hash: u64, mask: u32, now: u64) {
        self.watches.insert(wd, InotifyWatch::new(wd, path_hash, mask, now));
    }

    #[inline(always)]
    pub fn remove_watch(&mut self, wd: u64) { self.watches.remove(&wd); }

    #[inline]
    pub fn queue_event(&mut self, evt: InotifyAppEvent) {
        if self.event_queue.len() >= self.max_queued as usize { self.overflow_count += 1; return; }
        if let Some(w) = self.watches.get_mut(&evt.wd) { w.event_count += 1; }
        self.total_events += 1;
        self.event_queue.push(evt);
    }

    #[inline(always)]
    pub fn read_events(&mut self) -> Vec<InotifyAppEvent> {
        let evts = core::mem::take(&mut self.event_queue);
        evts
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct InotifyAppStats {
    pub total_instances: u32,
    pub total_watches: u32,
    pub total_events: u64,
    pub total_overflows: u64,
}

/// Main app inotify
pub struct AppInotify {
    instances: BTreeMap<u64, InotifyAppInstance>,
    next_fd: u64,
}

impl AppInotify {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_fd: 1 } }

    #[inline]
    pub fn init(&mut self, max_queued: u32) -> u64 {
        let fd = self.next_fd; self.next_fd += 1;
        self.instances.insert(fd, InotifyAppInstance::new(fd, max_queued));
        fd
    }

    #[inline(always)]
    pub fn add_watch(&mut self, fd: u64, wd: u64, path_hash: u64, mask: u32, now: u64) {
        if let Some(inst) = self.instances.get_mut(&fd) { inst.add_watch(wd, path_hash, mask, now); }
    }

    #[inline(always)]
    pub fn remove_watch(&mut self, fd: u64, wd: u64) {
        if let Some(inst) = self.instances.get_mut(&fd) { inst.remove_watch(wd); }
    }

    #[inline(always)]
    pub fn close(&mut self, fd: u64) { self.instances.remove(&fd); }

    #[inline]
    pub fn stats(&self) -> InotifyAppStats {
        let watches: u32 = self.instances.values().map(|i| i.watches.len() as u32).sum();
        let events: u64 = self.instances.values().map(|i| i.total_events).sum();
        let overflows: u64 = self.instances.values().map(|i| i.overflow_count).sum();
        InotifyAppStats { total_instances: self.instances.len() as u32, total_watches: watches, total_events: events, total_overflows: overflows }
    }
}
