// SPDX-License-Identifier: GPL-2.0
//! Apps epoll_app — epoll I/O event notification application layer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Epoll event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollEventType {
    In,
    Out,
    RdHup,
    Pri,
    Err,
    Hup,
    Et,
    OneShot,
    WakeUp,
    Exclusive,
}

/// Epoll entry
#[derive(Debug)]
pub struct EpollEntry {
    pub fd: u64,
    pub events: u32,
    pub data: u64,
    pub edge_triggered: bool,
    pub oneshot: bool,
    pub registered_at: u64,
    pub triggered_count: u64,
}

impl EpollEntry {
    pub fn new(fd: u64, events: u32, data: u64, et: bool, os: bool, now: u64) -> Self {
        Self { fd, events, data, edge_triggered: et, oneshot: os, registered_at: now, triggered_count: 0 }
    }
}

/// Epoll instance
#[derive(Debug)]
pub struct EpollInstance {
    pub id: u64,
    pub entries: BTreeMap<u64, EpollEntry>,
    pub ready_list: Vec<u64>,
    pub max_events: u32,
    pub total_waits: u64,
    pub total_events: u64,
}

impl EpollInstance {
    pub fn new(id: u64, max: u32) -> Self {
        Self { id, entries: BTreeMap::new(), ready_list: Vec::new(), max_events: max, total_waits: 0, total_events: 0 }
    }

    pub fn add(&mut self, entry: EpollEntry) {
        let fd = entry.fd;
        self.entries.insert(fd, entry);
    }

    pub fn remove(&mut self, fd: u64) { self.entries.remove(&fd); self.ready_list.retain(|&f| f != fd); }

    pub fn signal(&mut self, fd: u64) {
        if self.entries.contains_key(&fd) {
            if let Some(e) = self.entries.get_mut(&fd) { e.triggered_count += 1; }
            if !self.ready_list.contains(&fd) { self.ready_list.push(fd); }
            self.total_events += 1;
        }
    }

    pub fn wait(&mut self) -> Vec<u64> {
        self.total_waits += 1;
        let ready = self.ready_list.clone();
        self.ready_list.clear();
        ready
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct EpollAppStats {
    pub total_instances: u32,
    pub total_fds_monitored: u32,
    pub total_waits: u64,
    pub total_events: u64,
}

/// Main app epoll
pub struct AppEpoll {
    instances: BTreeMap<u64, EpollInstance>,
    next_id: u64,
}

impl AppEpoll {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, max: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.instances.insert(id, EpollInstance::new(id, max));
        id
    }

    pub fn ctl_add(&mut self, epid: u64, entry: EpollEntry) {
        if let Some(inst) = self.instances.get_mut(&epid) { inst.add(entry); }
    }

    pub fn ctl_del(&mut self, epid: u64, fd: u64) {
        if let Some(inst) = self.instances.get_mut(&epid) { inst.remove(fd); }
    }

    pub fn wait(&mut self, epid: u64) -> Vec<u64> {
        if let Some(inst) = self.instances.get_mut(&epid) { inst.wait() }
        else { Vec::new() }
    }

    pub fn stats(&self) -> EpollAppStats {
        let fds: u32 = self.instances.values().map(|i| i.entries.len() as u32).sum();
        let waits: u64 = self.instances.values().map(|i| i.total_waits).sum();
        let evts: u64 = self.instances.values().map(|i| i.total_events).sum();
        EpollAppStats { total_instances: self.instances.len() as u32, total_fds_monitored: fds, total_waits: waits, total_events: evts }
    }
}
