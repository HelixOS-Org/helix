// SPDX-License-Identifier: GPL-2.0
//! Apps eventpoll_mgr — epoll event polling management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Epoll event flags
#[derive(Debug, Clone, Copy)]
pub struct EpollEvents {
    pub bits: u32,
}

impl EpollEvents {
    pub const IN: u32 = 0x001;
    pub const OUT: u32 = 0x004;
    pub const ERR: u32 = 0x008;
    pub const HUP: u32 = 0x010;
    pub const ET: u32 = 1 << 31;
    pub const ONESHOT: u32 = 1 << 30;
    pub const RDHUP: u32 = 0x2000;
    pub const EXCLUSIVE: u32 = 1 << 28;
    pub const WAKEUP: u32 = 1 << 29;

    pub fn new(bits: u32) -> Self { Self { bits } }
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
    pub fn is_edge_triggered(&self) -> bool { self.has(Self::ET) }
    pub fn is_oneshot(&self) -> bool { self.has(Self::ONESHOT) }
}

/// Epoll operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollOp {
    Add,
    Mod,
    Del,
}

/// Epoll wait result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollWaitResult {
    Ready(u32),
    Timeout,
    Interrupted,
    Error,
}

/// Epoll item (registered fd)
#[derive(Debug, Clone)]
pub struct EpollItem {
    pub fd: i32,
    pub events: EpollEvents,
    pub data: u64,
    pub active: bool,
    pub ready_events: u32,
    pub trigger_count: u64,
    pub last_triggered: u64,
}

impl EpollItem {
    pub fn new(fd: i32, events: EpollEvents, data: u64) -> Self {
        Self { fd, events, data, active: true, ready_events: 0, trigger_count: 0, last_triggered: 0 }
    }

    pub fn fire(&mut self, events: u32, now: u64) -> bool {
        let masked = events & self.events.bits;
        if masked == 0 { return false; }
        self.ready_events |= masked;
        self.trigger_count += 1;
        self.last_triggered = now;
        if self.events.is_oneshot() { self.active = false; }
        true
    }

    pub fn consume(&mut self) -> u32 {
        let r = self.ready_events;
        if self.events.is_edge_triggered() { self.ready_events = 0; }
        r
    }
}

/// Epoll instance
#[derive(Debug)]
pub struct EpollInstance {
    pub id: u64,
    pub items: BTreeMap<i32, EpollItem>,
    pub max_events: u32,
    pub total_waits: u64,
    pub total_ready: u64,
    pub total_timeouts: u64,
    pub created_at: u64,
}

impl EpollInstance {
    pub fn new(id: u64, now: u64) -> Self {
        Self {
            id, items: BTreeMap::new(), max_events: 1024,
            total_waits: 0, total_ready: 0, total_timeouts: 0, created_at: now,
        }
    }

    pub fn add(&mut self, fd: i32, events: EpollEvents, data: u64) -> bool {
        if self.items.contains_key(&fd) { return false; }
        self.items.insert(fd, EpollItem::new(fd, events, data));
        true
    }

    pub fn modify(&mut self, fd: i32, events: EpollEvents) -> bool {
        if let Some(item) = self.items.get_mut(&fd) { item.events = events; item.active = true; true }
        else { false }
    }

    pub fn remove(&mut self, fd: i32) -> bool { self.items.remove(&fd).is_some() }

    pub fn ready_count(&self) -> u32 {
        self.items.values().filter(|i| i.active && i.ready_events != 0).count() as u32
    }

    pub fn wait(&mut self, max: u32) -> Vec<(i32, u32, u64)> {
        self.total_waits += 1;
        let mut results = Vec::new();
        for item in self.items.values_mut() {
            if !item.active || item.ready_events == 0 { continue; }
            let events = item.consume();
            results.push((item.fd, events, item.data));
            if results.len() as u32 >= max { break; }
        }
        self.total_ready += results.len() as u64;
        if results.is_empty() { self.total_timeouts += 1; }
        results
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct EventPollMgrStats {
    pub total_instances: u32,
    pub total_fds_monitored: u32,
    pub total_waits: u64,
    pub total_ready: u64,
    pub total_timeouts: u64,
}

/// Main eventpoll manager
pub struct AppEventPollMgr {
    instances: BTreeMap<u64, EpollInstance>,
    next_id: u64,
}

impl AppEventPollMgr {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.instances.insert(id, EpollInstance::new(id, now));
        id
    }

    pub fn add_fd(&mut self, inst_id: u64, fd: i32, events: EpollEvents, data: u64) -> bool {
        self.instances.get_mut(&inst_id).map(|i| i.add(fd, events, data)).unwrap_or(false)
    }

    pub fn stats(&self) -> EventPollMgrStats {
        let fds: u32 = self.instances.values().map(|i| i.items.len() as u32).sum();
        let waits: u64 = self.instances.values().map(|i| i.total_waits).sum();
        let ready: u64 = self.instances.values().map(|i| i.total_ready).sum();
        let tos: u64 = self.instances.values().map(|i| i.total_timeouts).sum();
        EventPollMgrStats {
            total_instances: self.instances.len() as u32, total_fds_monitored: fds,
            total_waits: waits, total_ready: ready, total_timeouts: tos,
        }
    }
}
