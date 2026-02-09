//! # Bridge Epoll Bridging
//!
//! Epoll/poll/select syscall multiplexing bridge:
//! - Epoll interest list management per FD
//! - Edge-triggered and level-triggered tracking
//! - Ready-list coalescing for batch delivery
//! - Poll timeout optimization
//! - FD interest set tracking across processes
//! - Wakeup source attribution

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Epoll trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollTrigger {
    LevelTriggered,
    EdgeTriggered,
    OneShot,
    ExclusiveWakeup,
}

/// Epoll event mask bits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpollEvents {
    pub readable: bool,
    pub writable: bool,
    pub error: bool,
    pub hangup: bool,
    pub priority: bool,
    pub rdhup: bool,
}

impl EpollEvents {
    #[inline(always)]
    pub fn empty() -> Self {
        Self { readable: false, writable: false, error: false, hangup: false, priority: false, rdhup: false }
    }
    #[inline]
    pub fn read() -> Self {
        let mut e = Self::empty();
        e.readable = true;
        e
    }
    #[inline]
    pub fn read_write() -> Self {
        let mut e = Self::empty();
        e.readable = true;
        e.writable = true;
        e
    }
    #[inline(always)]
    pub fn any_set(&self) -> bool {
        self.readable || self.writable || self.error || self.hangup || self.priority || self.rdhup
    }
    #[inline]
    pub fn matches(&self, interest: &EpollEvents) -> bool {
        (self.readable && interest.readable)
            || (self.writable && interest.writable)
            || (self.error && interest.error)
            || (self.hangup && interest.hangup)
            || (self.priority && interest.priority)
            || (self.rdhup && interest.rdhup)
    }
}

/// Watched file descriptor entry
#[derive(Debug, Clone)]
pub struct EpollEntry {
    pub fd: i32,
    pub interest: EpollEvents,
    pub trigger: EpollTrigger,
    pub user_data: u64,
    pub armed: bool,
    pub last_event_ns: u64,
    pub total_events: u64,
}

impl EpollEntry {
    pub fn new(fd: i32, interest: EpollEvents, trigger: EpollTrigger, data: u64) -> Self {
        Self {
            fd,
            interest,
            trigger,
            user_data: data,
            armed: true,
            last_event_ns: 0,
            total_events: 0,
        }
    }

    #[inline]
    pub fn deliver_event(&mut self, ts: u64) {
        self.total_events += 1;
        self.last_event_ns = ts;
        if self.trigger == EpollTrigger::OneShot {
            self.armed = false;
        }
    }
}

/// Epoll instance
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpollInstance {
    pub epfd: i32,
    pub owner_pid: u64,
    pub entries: BTreeMap<i32, EpollEntry>,
    pub ready_list: Vec<i32>,
    pub wait_count: u64,
    pub timeout_count: u64,
    pub max_events_per_wait: u32,
    pub created_ns: u64,
}

impl EpollInstance {
    pub fn new(epfd: i32, pid: u64, ts: u64) -> Self {
        Self {
            epfd,
            owner_pid: pid,
            entries: BTreeMap::new(),
            ready_list: Vec::new(),
            wait_count: 0,
            timeout_count: 0,
            max_events_per_wait: 64,
            created_ns: ts,
        }
    }

    #[inline(always)]
    pub fn add(&mut self, fd: i32, interest: EpollEvents, trigger: EpollTrigger, data: u64) {
        self.entries.insert(fd, EpollEntry::new(fd, interest, trigger, data));
    }

    #[inline]
    pub fn modify(&mut self, fd: i32, interest: EpollEvents, trigger: EpollTrigger) -> bool {
        if let Some(entry) = self.entries.get_mut(&fd) {
            entry.interest = interest;
            entry.trigger = trigger;
            entry.armed = true;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn remove(&mut self, fd: i32) -> bool {
        self.entries.remove(&fd).is_some()
    }

    #[inline]
    pub fn signal_ready(&mut self, fd: i32, events: &EpollEvents, ts: u64) {
        if let Some(entry) = self.entries.get_mut(&fd) {
            if entry.armed && events.matches(&entry.interest) {
                entry.deliver_event(ts);
                if !self.ready_list.contains(&fd) {
                    self.ready_list.push(fd);
                }
            }
        }
    }

    pub fn drain_ready(&mut self, max: usize) -> Vec<(i32, u64)> {
        self.wait_count += 1;
        let take = max.min(self.ready_list.len());
        let fds: Vec<i32> = self.ready_list.drain(..take).collect();
        let mut result = Vec::with_capacity(fds.len());
        for fd in fds {
            if let Some(entry) = self.entries.get(&fd) {
                result.push((fd, entry.user_data));
            }
        }
        result
    }

    #[inline(always)]
    pub fn watched_count(&self) -> usize { self.entries.len() }
    #[inline(always)]
    pub fn ready_count(&self) -> usize { self.ready_list.len() }
}

/// Per-process poll tracking
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessPollState {
    pub process_id: u64,
    pub epoll_instances: Vec<i32>,
    pub total_polls: u64,
    pub total_selects: u64,
    pub avg_wait_us: f64,
}

/// Wakeup source for attribution
#[derive(Debug, Clone)]
pub struct WakeupSource {
    pub fd: i32,
    pub source_type: WakeupSourceType,
    pub timestamp_ns: u64,
    pub wakeup_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupSourceType {
    Socket,
    Pipe,
    Timer,
    Signal,
    Eventfd,
    Inotify,
    Timerfd,
    Device,
}

/// Bridge epoll stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeEpollStats {
    pub total_instances: usize,
    pub total_watched_fds: usize,
    pub total_ready_fds: usize,
    pub total_waits: u64,
    pub total_timeouts: u64,
}

/// Bridge Epoll Manager
#[repr(align(64))]
pub struct BridgeEpollBridge {
    instances: BTreeMap<i32, EpollInstance>,
    process_poll: BTreeMap<u64, ProcessPollState>,
    wakeup_sources: BTreeMap<i32, WakeupSource>,
    stats: BridgeEpollStats,
    next_epfd: i32,
}

impl BridgeEpollBridge {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            process_poll: BTreeMap::new(),
            wakeup_sources: BTreeMap::new(),
            stats: BridgeEpollStats::default(),
            next_epfd: 1000,
        }
    }

    pub fn epoll_create(&mut self, pid: u64, ts: u64) -> i32 {
        let epfd = self.next_epfd;
        self.next_epfd += 1;
        self.instances.insert(epfd, EpollInstance::new(epfd, pid, ts));

        let poll_state = self.process_poll.entry(pid).or_insert_with(|| {
            ProcessPollState {
                process_id: pid,
                epoll_instances: Vec::new(),
                total_polls: 0,
                total_selects: 0,
                avg_wait_us: 0.0,
            }
        });
        poll_state.epoll_instances.push(epfd);
        epfd
    }

    #[inline]
    pub fn epoll_ctl_add(&mut self, epfd: i32, fd: i32, interest: EpollEvents, trigger: EpollTrigger, data: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&epfd) {
            inst.add(fd, interest, trigger, data);
            true
        } else { false }
    }

    #[inline]
    pub fn epoll_ctl_mod(&mut self, epfd: i32, fd: i32, interest: EpollEvents, trigger: EpollTrigger) -> bool {
        if let Some(inst) = self.instances.get_mut(&epfd) {
            inst.modify(fd, interest, trigger)
        } else { false }
    }

    #[inline]
    pub fn epoll_ctl_del(&mut self, epfd: i32, fd: i32) -> bool {
        if let Some(inst) = self.instances.get_mut(&epfd) {
            inst.remove(fd)
        } else { false }
    }

    #[inline]
    pub fn signal_fd_ready(&mut self, fd: i32, events: EpollEvents, ts: u64) {
        for inst in self.instances.values_mut() {
            inst.signal_ready(fd, &events, ts);
        }
    }

    #[inline]
    pub fn epoll_wait(&mut self, epfd: i32, max: usize) -> Vec<(i32, u64)> {
        if let Some(inst) = self.instances.get_mut(&epfd) {
            inst.drain_ready(max)
        } else { Vec::new() }
    }

    #[inline]
    pub fn register_wakeup_source(&mut self, fd: i32, stype: WakeupSourceType, ts: u64) {
        self.wakeup_sources.insert(fd, WakeupSource {
            fd,
            source_type: stype,
            timestamp_ns: ts,
            wakeup_count: 0,
        });
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_instances = self.instances.len();
        self.stats.total_watched_fds = self.instances.values().map(|i| i.watched_count()).sum();
        self.stats.total_ready_fds = self.instances.values().map(|i| i.ready_count()).sum();
        self.stats.total_waits = self.instances.values().map(|i| i.wait_count).sum();
        self.stats.total_timeouts = self.instances.values().map(|i| i.timeout_count).sum();
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeEpollStats { &self.stats }
    #[inline(always)]
    pub fn instance(&self, epfd: i32) -> Option<&EpollInstance> { self.instances.get(&epfd) }
}

// ============================================================================
// Merged from epoll_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpollV2Flags(pub u32);

impl EpollV2Flags {
    pub const IN: u32 = 1 << 0;
    pub const OUT: u32 = 1 << 1;
    pub const ERR: u32 = 1 << 2;
    pub const HUP: u32 = 1 << 3;
    pub const ET: u32 = 1 << 4;
    pub const ONESHOT: u32 = 1 << 5;
    pub const EXCLUSIVE: u32 = 1 << 6;
    pub const RDHUP: u32 = 1 << 7;

    pub fn new() -> Self { Self(0) }
    #[inline(always)]
    pub fn set(&mut self, f: u32) { self.0 |= f; }
    #[inline(always)]
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Epoll v2 operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollV2Op {
    Add,
    Modify,
    Delete,
}

/// Monitored FD
#[derive(Debug)]
pub struct EpollV2Item {
    pub fd: i32,
    pub events: EpollV2Flags,
    pub data: u64,
    pub ready: bool,
    pub fired_count: u64,
    pub edge_triggered: bool,
    pub oneshot: bool,
    pub disabled: bool,
}

impl EpollV2Item {
    pub fn new(fd: i32, events: EpollV2Flags, data: u64) -> Self {
        Self {
            fd, events, data, ready: false, fired_count: 0,
            edge_triggered: events.has(EpollV2Flags::ET),
            oneshot: events.has(EpollV2Flags::ONESHOT),
            disabled: false,
        }
    }

    #[inline]
    pub fn fire(&mut self) {
        self.ready = true;
        self.fired_count += 1;
        if self.oneshot { self.disabled = true; }
    }

    #[inline(always)]
    pub fn consume(&mut self) {
        if self.edge_triggered || self.oneshot { self.ready = false; }
    }
}

/// Epoll v2 instance
#[derive(Debug)]
#[repr(align(64))]
pub struct EpollV2Instance {
    pub id: u64,
    pub items: BTreeMap<i32, EpollV2Item>,
    pub total_waits: u64,
    pub total_events: u64,
    pub max_events: u32,
    pub busy_poll_ns: u64,
}

impl EpollV2Instance {
    pub fn new(id: u64) -> Self {
        Self { id, items: BTreeMap::new(), total_waits: 0, total_events: 0, max_events: 128, busy_poll_ns: 0 }
    }

    #[inline(always)]
    pub fn add(&mut self, fd: i32, events: EpollV2Flags, data: u64) {
        self.items.insert(fd, EpollV2Item::new(fd, events, data));
    }

    #[inline(always)]
    pub fn modify(&mut self, fd: i32, events: EpollV2Flags) {
        if let Some(item) = self.items.get_mut(&fd) { item.events = events; }
    }

    #[inline(always)]
    pub fn remove(&mut self, fd: i32) { self.items.remove(&fd); }

    #[inline(always)]
    pub fn ready_count(&self) -> u32 { self.items.values().filter(|i| i.ready && !i.disabled).count() as u32 }

    #[inline]
    pub fn wait(&mut self) -> Vec<(i32, u64)> {
        self.total_waits += 1;
        let ready: Vec<(i32, u64)> = self.items.values().filter(|i| i.ready && !i.disabled).map(|i| (i.fd, i.data)).collect();
        self.total_events += ready.len() as u64;
        for (fd, _) in &ready {
            if let Some(item) = self.items.get_mut(fd) { item.consume(); }
        }
        ready
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpollV2BridgeStats {
    pub total_instances: u32,
    pub total_fds: u32,
    pub total_waits: u64,
    pub total_events: u64,
    pub avg_events_per_wait: f64,
}

/// Main epoll v2 bridge
#[repr(align(64))]
pub struct BridgeEpollV2 {
    instances: BTreeMap<u64, EpollV2Instance>,
    next_id: u64,
}

impl BridgeEpollV2 {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.instances.insert(id, EpollV2Instance::new(id));
        id
    }

    #[inline]
    pub fn ctl(&mut self, epfd: u64, op: EpollV2Op, fd: i32, events: EpollV2Flags, data: u64) {
        if let Some(inst) = self.instances.get_mut(&epfd) {
            match op {
                EpollV2Op::Add => inst.add(fd, events, data),
                EpollV2Op::Modify => inst.modify(fd, events),
                EpollV2Op::Delete => inst.remove(fd),
            }
        }
    }

    #[inline]
    pub fn stats(&self) -> EpollV2BridgeStats {
        let fds: u32 = self.instances.values().map(|i| i.items.len() as u32).sum();
        let waits: u64 = self.instances.values().map(|i| i.total_waits).sum();
        let events: u64 = self.instances.values().map(|i| i.total_events).sum();
        let avg = if waits == 0 { 0.0 } else { events as f64 / waits as f64 };
        EpollV2BridgeStats { total_instances: self.instances.len() as u32, total_fds: fds, total_waits: waits, total_events: events, avg_events_per_wait: avg }
    }
}

// ============================================================================
// Merged from epoll_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollV3Trigger {
    LevelTriggered,
    EdgeTriggered,
    OneShot,
    ExclusiveWake,
    MultiShot,
}

/// Epoll event flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollV3Event {
    In,
    Out,
    Pri,
    Err,
    Hup,
    RdHup,
    RdNorm,
    WrNorm,
    RdBand,
    WrBand,
    Msg,
    Et,
}

/// A watched file descriptor entry
#[derive(Debug, Clone)]
pub struct EpollV3Interest {
    pub fd: u64,
    pub events: u32,
    pub trigger: EpollV3Trigger,
    pub user_data: u64,
    pub active: bool,
    pub ready_count: u64,
    pub last_ready_tick: u64,
}

/// An epoll instance managing multiple interests
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpollV3Instance {
    pub id: u64,
    pub interests: BTreeMap<u64, EpollV3Interest>,
    pub max_events: usize,
    pub timeout_ns: u64,
    pub nested_depth: u32,
    pub created_tick: u64,
    pub total_waits: u64,
    pub total_ready: u64,
}

impl EpollV3Instance {
    pub fn new(id: u64, max_events: usize, tick: u64) -> Self {
        Self {
            id,
            interests: BTreeMap::new(),
            max_events,
            timeout_ns: 0,
            nested_depth: 0,
            created_tick: tick,
            total_waits: 0,
            total_ready: 0,
        }
    }

    pub fn add_interest(&mut self, fd: u64, events: u32, trigger: EpollV3Trigger, data: u64, tick: u64) {
        let interest = EpollV3Interest {
            fd,
            events,
            trigger,
            user_data: data,
            active: true,
            ready_count: 0,
            last_ready_tick: tick,
        };
        self.interests.insert(fd, interest);
    }

    #[inline(always)]
    pub fn remove_interest(&mut self, fd: u64) -> bool {
        self.interests.remove(&fd).is_some()
    }

    pub fn mark_ready(&mut self, fd: u64, tick: u64) -> bool {
        if let Some(interest) = self.interests.get_mut(&fd) {
            interest.ready_count += 1;
            interest.last_ready_tick = tick;
            self.total_ready += 1;
            if interest.trigger == EpollV3Trigger::OneShot {
                interest.active = false;
            }
            true
        } else {
            false
        }
    }

    pub fn collect_ready(&self) -> Vec<u64> {
        let mut ready = Vec::new();
        for (fd, interest) in &self.interests {
            if interest.active && interest.ready_count > 0 {
                ready.push(*fd);
                if ready.len() >= self.max_events {
                    break;
                }
            }
        }
        ready
    }
}

/// Statistics for epoll V3 bridge
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpollV3BridgeStats {
    pub instances_created: u64,
    pub interests_added: u64,
    pub interests_removed: u64,
    pub waits_performed: u64,
    pub events_delivered: u64,
    pub timeouts: u64,
    pub nested_epoll_count: u64,
}

/// Main epoll V3 bridge manager
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeEpollV3 {
    instances: BTreeMap<u64, EpollV3Instance>,
    next_id: u64,
    stats: EpollV3BridgeStats,
}

impl BridgeEpollV3 {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            next_id: 1,
            stats: EpollV3BridgeStats {
                instances_created: 0,
                interests_added: 0,
                interests_removed: 0,
                waits_performed: 0,
                events_delivered: 0,
                timeouts: 0,
                nested_epoll_count: 0,
            },
        }
    }

    #[inline]
    pub fn create_instance(&mut self, max_events: usize, tick: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let inst = EpollV3Instance::new(id, max_events, tick);
        self.instances.insert(id, inst);
        self.stats.instances_created += 1;
        id
    }

    #[inline]
    pub fn add_interest(&mut self, inst_id: u64, fd: u64, events: u32, trigger: EpollV3Trigger, data: u64, tick: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&inst_id) {
            inst.add_interest(fd, events, trigger, data, tick);
            self.stats.interests_added += 1;
            true
        } else {
            false
        }
    }

    pub fn wait_events(&mut self, inst_id: u64, tick: u64) -> Vec<u64> {
        if let Some(inst) = self.instances.get_mut(&inst_id) {
            inst.total_waits += 1;
            self.stats.waits_performed += 1;
            let ready = inst.collect_ready();
            self.stats.events_delivered += ready.len() as u64;
            if ready.is_empty() {
                self.stats.timeouts += 1;
            }
            ready
        } else {
            Vec::new()
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &EpollV3BridgeStats {
        &self.stats
    }

    #[inline(always)]
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}

// ============================================================================
// Merged from epoll_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollV4Event {
    In,
    Out,
    RdHup,
    Pri,
    Err,
    Hup,
    Et,
    Oneshot,
    WakeUp,
    Exclusive,
}

/// Epoll v4 mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollV4Mode {
    LevelTriggered,
    EdgeTriggered,
    Oneshot,
    BusyPoll,
}

/// Epoll interest entry
#[derive(Debug, Clone)]
pub struct EpollV4Interest {
    pub fd: i32,
    pub events: u32,
    pub mode: EpollV4Mode,
    pub user_data: u64,
    pub active: bool,
    pub ready: bool,
    pub trigger_count: u64,
    pub last_trigger_ns: u64,
}

impl EpollV4Interest {
    pub fn new(fd: i32, events: u32, mode: EpollV4Mode) -> Self {
        Self {
            fd,
            events,
            mode,
            user_data: fd as u64,
            active: true,
            ready: false,
            trigger_count: 0,
            last_trigger_ns: 0,
        }
    }

    #[inline]
    pub fn trigger(&mut self, ts_ns: u64) {
        self.ready = true;
        self.trigger_count += 1;
        self.last_trigger_ns = ts_ns;
        if self.mode == EpollV4Mode::Oneshot {
            self.active = false;
        }
    }

    #[inline]
    pub fn consume(&mut self) {
        if self.mode == EpollV4Mode::EdgeTriggered {
            self.ready = false;
        }
    }
}

/// Epoll instance
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpollV4Instance {
    pub epfd: u32,
    pub interests: BTreeMap<i32, EpollV4Interest>,
    pub max_events: u32,
    pub busy_poll_us: u32,
    pub total_waits: u64,
    pub total_events: u64,
    pub empty_waits: u64,
    pub max_batch_seen: u32,
}

impl EpollV4Instance {
    pub fn new(epfd: u32, max_events: u32) -> Self {
        Self {
            epfd,
            interests: BTreeMap::new(),
            max_events,
            busy_poll_us: 0,
            total_waits: 0,
            total_events: 0,
            empty_waits: 0,
            max_batch_seen: 0,
        }
    }

    #[inline]
    pub fn ctl_add(&mut self, fd: i32, events: u32, mode: EpollV4Mode) -> bool {
        if self.interests.contains_key(&fd) {
            return false;
        }
        self.interests.insert(fd, EpollV4Interest::new(fd, events, mode));
        true
    }

    #[inline]
    pub fn ctl_mod(&mut self, fd: i32, events: u32) -> bool {
        if let Some(interest) = self.interests.get_mut(&fd) {
            interest.events = events;
            interest.active = true;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn ctl_del(&mut self, fd: i32) -> bool {
        self.interests.remove(&fd).is_some()
    }

    pub fn wait(&mut self, ts_ns: u64) -> u32 {
        self.total_waits += 1;
        let ready: Vec<i32> = self.interests.iter()
            .filter(|(_, i)| i.active && i.ready)
            .map(|(&fd, _)| fd)
            .take(self.max_events as usize)
            .collect();
        let count = ready.len() as u32;
        for fd in &ready {
            if let Some(interest) = self.interests.get_mut(fd) {
                interest.consume();
            }
        }
        if count == 0 {
            self.empty_waits += 1;
        } else {
            self.total_events += count as u64;
            if count > self.max_batch_seen {
                self.max_batch_seen = count;
            }
        }
        count
    }

    #[inline(always)]
    pub fn avg_events_per_wait(&self) -> f64 {
        if self.total_waits == 0 { 0.0 } else { self.total_events as f64 / self.total_waits as f64 }
    }

    #[inline(always)]
    pub fn empty_rate(&self) -> f64 {
        if self.total_waits == 0 { 0.0 } else { self.empty_waits as f64 / self.total_waits as f64 }
    }
}

/// Epoll v4 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpollV4BridgeStats {
    pub total_instances: u64,
    pub total_fds_monitored: u64,
    pub total_events_delivered: u64,
    pub total_waits: u64,
}

/// Main bridge epoll v4
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeEpollV4 {
    pub instances: BTreeMap<u32, EpollV4Instance>,
    pub stats: EpollV4BridgeStats,
    pub next_epfd: u32,
}

impl BridgeEpollV4 {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            stats: EpollV4BridgeStats {
                total_instances: 0,
                total_fds_monitored: 0,
                total_events_delivered: 0,
                total_waits: 0,
            },
            next_epfd: 1,
        }
    }

    #[inline]
    pub fn create(&mut self, max_events: u32) -> u32 {
        let id = self.next_epfd;
        self.next_epfd += 1;
        self.instances.insert(id, EpollV4Instance::new(id, max_events));
        self.stats.total_instances += 1;
        id
    }

    pub fn add_fd(&mut self, epfd: u32, fd: i32, events: u32, mode: EpollV4Mode) -> bool {
        if let Some(inst) = self.instances.get_mut(&epfd) {
            if inst.ctl_add(fd, events, mode) {
                self.stats.total_fds_monitored += 1;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

// ============================================================================
// Merged from epoll_v5_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollV5Op { Add, Mod, Del, Wait, WaitTimeout }

/// Epoll v5 record
#[derive(Debug, Clone)]
pub struct EpollV5Record {
    pub op: EpollV5Op,
    pub epfd: i32,
    pub target_fd: i32,
    pub events_mask: u32,
    pub ready_count: u32,
}

impl EpollV5Record {
    pub fn new(op: EpollV5Op, epfd: i32) -> Self { Self { op, epfd, target_fd: -1, events_mask: 0, ready_count: 0 } }
}

/// Epoll v5 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpollV5BridgeStats { pub total_ops: u64, pub adds: u64, pub waits: u64, pub timeouts: u64 }

/// Main bridge epoll v5
#[derive(Debug)]
pub struct BridgeEpollV5 { pub stats: EpollV5BridgeStats }

impl BridgeEpollV5 {
    pub fn new() -> Self { Self { stats: EpollV5BridgeStats { total_ops: 0, adds: 0, waits: 0, timeouts: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &EpollV5Record) {
        self.stats.total_ops += 1;
        match rec.op {
            EpollV5Op::Add => self.stats.adds += 1,
            EpollV5Op::Wait => self.stats.waits += 1,
            EpollV5Op::WaitTimeout => self.stats.timeouts += 1,
            _ => {}
        }
    }
}
