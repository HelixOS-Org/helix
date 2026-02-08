// SPDX-License-Identifier: GPL-2.0
//! Bridge inotify — file system event notification proxy for kernel↔userspace.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Inotify event mask flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InotifyMask(pub u32);

impl InotifyMask {
    pub const IN_ACCESS: Self = Self(0x0000_0001);
    pub const IN_MODIFY: Self = Self(0x0000_0002);
    pub const IN_ATTRIB: Self = Self(0x0000_0004);
    pub const IN_CLOSE_WRITE: Self = Self(0x0000_0008);
    pub const IN_CLOSE_NOWRITE: Self = Self(0x0000_0010);
    pub const IN_OPEN: Self = Self(0x0000_0020);
    pub const IN_MOVED_FROM: Self = Self(0x0000_0040);
    pub const IN_MOVED_TO: Self = Self(0x0000_0080);
    pub const IN_CREATE: Self = Self(0x0000_0100);
    pub const IN_DELETE: Self = Self(0x0000_0200);
    pub const IN_DELETE_SELF: Self = Self(0x0000_0400);
    pub const IN_MOVE_SELF: Self = Self(0x0000_0800);
    pub const IN_ALL_EVENTS: Self = Self(0x0000_0FFF);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn matches(&self, event_mask: Self) -> bool {
        (self.0 & event_mask.0) != 0
    }

    pub fn combine(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// A watch descriptor for a single path
#[derive(Debug)]
pub struct WatchDescriptor {
    pub wd: i32,
    pub path: String,
    pub mask: InotifyMask,
    pub recursive: bool,
    pub event_count: u64,
    pub last_event_ns: u64,
}

impl WatchDescriptor {
    pub fn new(wd: i32, path: String, mask: InotifyMask) -> Self {
        Self {
            wd,
            path,
            mask,
            recursive: false,
            event_count: 0,
            last_event_ns: 0,
        }
    }

    pub fn should_report(&self, event_mask: InotifyMask) -> bool {
        self.mask.matches(event_mask)
    }
}

/// An inotify event
#[derive(Debug, Clone)]
pub struct InotifyEvent {
    pub wd: i32,
    pub mask: InotifyMask,
    pub cookie: u32,
    pub name: String,
    pub timestamp_ns: u64,
}

impl InotifyEvent {
    pub fn is_move_pair(&self, other: &Self) -> bool {
        self.cookie != 0 && self.cookie == other.cookie
    }

    pub fn event_size(&self) -> usize {
        // struct inotify_event header (16) + name length aligned to 4
        16 + ((self.name.len() + 4) & !3)
    }
}

/// Per-process inotify instance
#[derive(Debug)]
pub struct InotifyInstance {
    pub fd: i32,
    pub pid: u64,
    watches: BTreeMap<i32, WatchDescriptor>,
    event_queue: Vec<InotifyEvent>,
    next_wd: i32,
    max_events: usize,
    max_watches: usize,
    overflow_count: u64,
}

impl InotifyInstance {
    pub fn new(fd: i32, pid: u64) -> Self {
        Self {
            fd,
            pid,
            watches: BTreeMap::new(),
            event_queue: Vec::new(),
            next_wd: 1,
            max_events: 16384,
            max_watches: 8192,
            overflow_count: 0,
        }
    }

    pub fn add_watch(&mut self, path: String, mask: InotifyMask) -> Option<i32> {
        if self.watches.len() >= self.max_watches {
            return None;
        }
        // Check if already watching this path
        for (wd, watch) in &self.watches {
            if watch.path == path {
                return Some(*wd);
            }
        }
        let wd = self.next_wd;
        self.next_wd += 1;
        self.watches.insert(wd, WatchDescriptor::new(wd, path, mask));
        Some(wd)
    }

    pub fn remove_watch(&mut self, wd: i32) -> bool {
        self.watches.remove(&wd).is_some()
    }

    pub fn push_event(&mut self, event: InotifyEvent) -> bool {
        if self.event_queue.len() >= self.max_events {
            self.overflow_count += 1;
            return false;
        }
        // Update watch stats
        if let Some(watch) = self.watches.get_mut(&event.wd) {
            watch.event_count += 1;
            watch.last_event_ns = event.timestamp_ns;
        }
        self.event_queue.push(event);
        true
    }

    pub fn read_events(&mut self, max: usize) -> Vec<InotifyEvent> {
        let count = max.min(self.event_queue.len());
        self.event_queue.drain(..count).collect()
    }

    pub fn pending_events(&self) -> usize {
        self.event_queue.len()
    }

    pub fn pending_bytes(&self) -> usize {
        self.event_queue.iter().map(|e| e.event_size()).sum()
    }

    pub fn watch_count(&self) -> usize {
        self.watches.len()
    }

    pub fn watches_for_path(&self, path: &str) -> Vec<i32> {
        self.watches
            .iter()
            .filter(|(_, w)| {
                w.path == path || (w.recursive && path.starts_with(w.path.as_str()))
            })
            .map(|(wd, _)| *wd)
            .collect()
    }
}

/// Coalescing configuration to reduce event storms
#[derive(Debug, Clone)]
pub struct CoalesceConfig {
    pub enabled: bool,
    pub window_ns: u64,
    pub max_coalesced: u32,
}

impl CoalesceConfig {
    pub fn default_config() -> Self {
        Self {
            enabled: true,
            window_ns: 50_000_000, // 50ms
            max_coalesced: 16,
        }
    }
}

/// Inotify bridge stats
#[derive(Debug, Clone)]
pub struct InotifyBridgeStats {
    pub instances_created: u64,
    pub watches_active: u64,
    pub events_generated: u64,
    pub events_delivered: u64,
    pub events_coalesced: u64,
    pub overflows: u64,
}

/// Main inotify bridge manager
pub struct BridgeInotify {
    instances: BTreeMap<i32, InotifyInstance>,
    next_fd: i32,
    coalesce: CoalesceConfig,
    path_watch_count: BTreeMap<String, u32>,
    max_instances_per_process: u32,
    stats: InotifyBridgeStats,
}

impl BridgeInotify {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            next_fd: 100,
            coalesce: CoalesceConfig::default_config(),
            path_watch_count: BTreeMap::new(),
            max_instances_per_process: 128,
            stats: InotifyBridgeStats {
                instances_created: 0,
                watches_active: 0,
                events_generated: 0,
                events_delivered: 0,
                events_coalesced: 0,
                overflows: 0,
            },
        }
    }

    pub fn create_instance(&mut self, pid: u64) -> Option<i32> {
        let per_process = self.instances.values().filter(|i| i.pid == pid).count() as u32;
        if per_process >= self.max_instances_per_process {
            return None;
        }
        let fd = self.next_fd;
        self.next_fd += 1;
        self.instances.insert(fd, InotifyInstance::new(fd, pid));
        self.stats.instances_created += 1;
        Some(fd)
    }

    pub fn destroy_instance(&mut self, fd: i32) -> bool {
        if let Some(inst) = self.instances.remove(&fd) {
            for (_, watch) in &inst.watches {
                if let Some(count) = self.path_watch_count.get_mut(&watch.path) {
                    *count = count.saturating_sub(1);
                }
            }
            self.stats.watches_active = self.stats.watches_active.saturating_sub(inst.watch_count() as u64);
            true
        } else {
            false
        }
    }

    pub fn add_watch(&mut self, fd: i32, path: String, mask: InotifyMask) -> Option<i32> {
        let inst = self.instances.get_mut(&fd)?;
        let wd = inst.add_watch(path.clone(), mask)?;
        *self.path_watch_count.entry(path).or_insert(0) += 1;
        self.stats.watches_active += 1;
        Some(wd)
    }

    pub fn remove_watch(&mut self, fd: i32, wd: i32) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            if inst.remove_watch(wd) {
                self.stats.watches_active = self.stats.watches_active.saturating_sub(1);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn emit_event(
        &mut self,
        path: &str,
        mask: InotifyMask,
        name: String,
        cookie: u32,
        timestamp_ns: u64,
    ) -> u32 {
        self.stats.events_generated += 1;
        let mut delivered = 0u32;

        // Find all instances watching this path
        let fds: Vec<i32> = self.instances.keys().copied().collect();
        for fd in fds {
            let inst = self.instances.get_mut(&fd).unwrap();
            let matching_wds = inst.watches_for_path(path);
            for wd in matching_wds {
                let event = InotifyEvent {
                    wd,
                    mask,
                    cookie,
                    name: name.clone(),
                    timestamp_ns,
                };
                if inst.push_event(event) {
                    delivered += 1;
                } else {
                    self.stats.overflows += 1;
                }
            }
        }

        self.stats.events_delivered += delivered as u64;
        delivered
    }

    pub fn read_events(&mut self, fd: i32, max: usize) -> Vec<InotifyEvent> {
        if let Some(inst) = self.instances.get_mut(&fd) {
            inst.read_events(max)
        } else {
            Vec::new()
        }
    }

    pub fn set_coalesce_config(&mut self, config: CoalesceConfig) {
        self.coalesce = config;
    }

    pub fn most_watched_paths(&self, top_n: usize) -> Vec<(String, u32)> {
        let mut paths: Vec<(String, u32)> = self.path_watch_count
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        paths.sort_by(|a, b| b.1.cmp(&a.1));
        paths.truncate(top_n);
        paths
    }

    pub fn stats(&self) -> &InotifyBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from inotify_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InotifyV2Mask {
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
    Overflow,
}

/// Inotify v2 watch
#[derive(Debug)]
pub struct InotifyV2Watch {
    pub wd: i32,
    pub path_hash: u64,
    pub mask: u32,
    pub events_received: u64,
    pub recursive: bool,
}

impl InotifyV2Watch {
    pub fn new(wd: i32, path_hash: u64, mask: u32) -> Self {
        Self { wd, path_hash, mask, events_received: 0, recursive: false }
    }
}

/// Inotify v2 event
#[derive(Debug, Clone)]
pub struct InotifyV2Event {
    pub wd: i32,
    pub mask: u32,
    pub cookie: u32,
    pub name_hash: u64,
    pub timestamp: u64,
}

/// Inotify v2 instance
#[derive(Debug)]
pub struct InotifyV2Instance {
    pub fd: u64,
    pub watches: Vec<InotifyV2Watch>,
    pub event_queue: Vec<InotifyV2Event>,
    pub max_queued: u32,
    pub overflow_count: u64,
}

impl InotifyV2Instance {
    pub fn new(fd: u64, max_queued: u32) -> Self {
        Self { fd, watches: Vec::new(), event_queue: Vec::new(), max_queued, overflow_count: 0 }
    }

    pub fn add_watch(&mut self, path_hash: u64, mask: u32) -> i32 {
        let wd = self.watches.len() as i32 + 1;
        self.watches.push(InotifyV2Watch::new(wd, path_hash, mask));
        wd
    }

    pub fn queue_event(&mut self, wd: i32, mask: u32, cookie: u32, name_hash: u64, now: u64) {
        if self.event_queue.len() as u32 >= self.max_queued { self.overflow_count += 1; return; }
        self.event_queue.push(InotifyV2Event { wd, mask, cookie, name_hash, timestamp: now });
        if let Some(w) = self.watches.iter_mut().find(|w| w.wd == wd) { w.events_received += 1; }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct InotifyV2BridgeStats {
    pub total_instances: u32,
    pub total_watches: u32,
    pub total_events: u64,
    pub total_overflow: u64,
}

/// Main inotify v2 bridge
pub struct BridgeInotifyV2 {
    instances: BTreeMap<u64, InotifyV2Instance>,
    next_fd: u64,
}

impl BridgeInotifyV2 {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_fd: 1 } }

    pub fn create(&mut self, max_queued: u32) -> u64 {
        let fd = self.next_fd; self.next_fd += 1;
        self.instances.insert(fd, InotifyV2Instance::new(fd, max_queued));
        fd
    }

    pub fn stats(&self) -> InotifyV2BridgeStats {
        let watches: u32 = self.instances.values().map(|i| i.watches.len() as u32).sum();
        let events: u64 = self.instances.values().flat_map(|i| i.watches.iter()).map(|w| w.events_received).sum();
        let overflow: u64 = self.instances.values().map(|i| i.overflow_count).sum();
        InotifyV2BridgeStats { total_instances: self.instances.len() as u32, total_watches: watches, total_events: events, total_overflow: overflow }
    }
}

// ============================================================================
// Merged from inotify_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InotifyV3Mask {
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
    Unmount,
    QOverflow,
    Ignored,
    IsDir,
    OneShot,
    DontFollow,
    ExclUnlink,
    MaskAdd,
    MaskCreate,
}

/// An inotify event
#[derive(Debug, Clone)]
pub struct InotifyV3Event {
    pub wd: i64,
    pub mask: u32,
    pub cookie: u32,
    pub name: String,
    pub timestamp_ns: u64,
    pub is_dir: bool,
}

/// A watch descriptor tracking a path
#[derive(Debug, Clone)]
pub struct InotifyV3Watch {
    pub wd: i64,
    pub path: String,
    pub mask: u32,
    pub recursive: bool,
    pub event_count: u64,
    pub children: Vec<i64>,
}

/// An inotify instance with event queue
#[derive(Debug, Clone)]
pub struct InotifyV3Instance {
    pub fd: u64,
    pub watches: BTreeMap<i64, InotifyV3Watch>,
    pub event_queue: Vec<InotifyV3Event>,
    pub max_queue_size: usize,
    pub next_wd: i64,
    pub overflow_count: u64,
    pub total_events: u64,
}

impl InotifyV3Instance {
    pub fn new(fd: u64, max_queue: usize) -> Self {
        Self {
            fd,
            watches: BTreeMap::new(),
            event_queue: Vec::new(),
            max_queue_size: max_queue,
            next_wd: 1,
            overflow_count: 0,
            total_events: 0,
        }
    }

    pub fn add_watch(&mut self, path: String, mask: u32, recursive: bool) -> i64 {
        let wd = self.next_wd;
        self.next_wd += 1;
        let watch = InotifyV3Watch {
            wd,
            path,
            mask,
            recursive,
            event_count: 0,
            children: Vec::new(),
        };
        self.watches.insert(wd, watch);
        wd
    }

    pub fn remove_watch(&mut self, wd: i64) -> bool {
        self.watches.remove(&wd).is_some()
    }

    pub fn push_event(&mut self, wd: i64, mask: u32, cookie: u32, name: String, tick: u64, is_dir: bool) -> bool {
        if self.event_queue.len() >= self.max_queue_size {
            self.overflow_count += 1;
            return false;
        }
        if let Some(watch) = self.watches.get_mut(&wd) {
            watch.event_count += 1;
        }
        let event = InotifyV3Event { wd, mask, cookie, name, timestamp_ns: tick, is_dir };
        self.event_queue.push(event);
        self.total_events += 1;
        true
    }

    pub fn read_events(&mut self, max: usize) -> Vec<InotifyV3Event> {
        let count = max.min(self.event_queue.len());
        self.event_queue.drain(..count).collect()
    }

    pub fn pending_events(&self) -> usize {
        self.event_queue.len()
    }
}

/// Statistics for inotify V3 bridge
#[derive(Debug, Clone)]
pub struct InotifyV3BridgeStats {
    pub instances_created: u64,
    pub watches_added: u64,
    pub watches_removed: u64,
    pub events_generated: u64,
    pub events_read: u64,
    pub queue_overflows: u64,
    pub recursive_watches: u64,
}

/// Main inotify V3 bridge manager
#[derive(Debug)]
pub struct BridgeInotifyV3 {
    instances: BTreeMap<u64, InotifyV3Instance>,
    next_fd: u64,
    stats: InotifyV3BridgeStats,
}

impl BridgeInotifyV3 {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            next_fd: 1,
            stats: InotifyV3BridgeStats {
                instances_created: 0,
                watches_added: 0,
                watches_removed: 0,
                events_generated: 0,
                events_read: 0,
                queue_overflows: 0,
                recursive_watches: 0,
            },
        }
    }

    pub fn create_instance(&mut self, max_queue: usize) -> u64 {
        let fd = self.next_fd;
        self.next_fd += 1;
        self.instances.insert(fd, InotifyV3Instance::new(fd, max_queue));
        self.stats.instances_created += 1;
        fd
    }

    pub fn add_watch(&mut self, fd: u64, path: String, mask: u32, recursive: bool) -> Option<i64> {
        if let Some(inst) = self.instances.get_mut(&fd) {
            let wd = inst.add_watch(path, mask, recursive);
            self.stats.watches_added += 1;
            if recursive {
                self.stats.recursive_watches += 1;
            }
            Some(wd)
        } else {
            None
        }
    }

    pub fn read_events(&mut self, fd: u64, max: usize) -> Vec<InotifyV3Event> {
        if let Some(inst) = self.instances.get_mut(&fd) {
            let events = inst.read_events(max);
            self.stats.events_read += events.len() as u64;
            events
        } else {
            Vec::new()
        }
    }

    pub fn stats(&self) -> &InotifyV3BridgeStats {
        &self.stats
    }
}
