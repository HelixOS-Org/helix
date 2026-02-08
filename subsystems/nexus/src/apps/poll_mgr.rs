// SPDX-License-Identifier: GPL-2.0
//! Apps poll_mgr — poll/select/epoll pattern analysis and optimization per application.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Poll mechanism type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollMechanism {
    Select,
    Poll,
    Epoll,
    EpollEdge,
    IoUring,
    Kqueue,
}

impl PollMechanism {
    pub fn scalability_score(&self) -> u32 {
        match self {
            Self::Select => 1,
            Self::Poll => 2,
            Self::Epoll => 4,
            Self::EpollEdge => 5,
            Self::IoUring => 5,
            Self::Kqueue => 4,
        }
    }
}

/// Event interest flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollEvents(pub u32);

impl PollEvents {
    pub const IN: u32 = 1 << 0;
    pub const OUT: u32 = 1 << 1;
    pub const ERR: u32 = 1 << 2;
    pub const HUP: u32 = 1 << 3;
    pub const PRI: u32 = 1 << 4;
    pub const RDNORM: u32 = 1 << 5;
    pub const WRNORM: u32 = 1 << 6;
    pub const RDHUP: u32 = 1 << 7;
    pub const ET: u32 = 1 << 8;
    pub const ONESHOT: u32 = 1 << 9;

    pub fn has(&self, ev: u32) -> bool {
        self.0 & ev != 0
    }

    pub fn interest_count(&self) -> u32 {
        (self.0 & 0x3FF).count_ones()
    }

    pub fn is_read_only(&self) -> bool {
        self.has(Self::IN) && !self.has(Self::OUT)
    }

    pub fn is_write_only(&self) -> bool {
        !self.has(Self::IN) && self.has(Self::OUT)
    }
}

/// Epoll instance descriptor
#[derive(Debug)]
pub struct EpollInstance {
    pub epfd: u64,
    pub owner_pid: u64,
    pub created_ns: u64,
    fds: Vec<EpollFdEntry>,
    pub wait_count: u64,
    pub event_count: u64,
    pub timeout_count: u64,
    pub max_events_param: u32,
}

/// An fd registered in an epoll instance
#[derive(Debug, Clone)]
pub struct EpollFdEntry {
    pub fd: u64,
    pub events: PollEvents,
    pub fired_count: u64,
    pub last_fired_ns: u64,
    pub stale_waits: u64,
}

impl EpollFdEntry {
    pub fn new(fd: u64, events: PollEvents) -> Self {
        Self {
            fd,
            events,
            fired_count: 0,
            last_fired_ns: 0,
            stale_waits: 0,
        }
    }

    pub fn fire(&mut self, now_ns: u64) {
        self.fired_count += 1;
        self.last_fired_ns = now_ns;
    }

    pub fn activity_ratio(&self, total_waits: u64) -> f64 {
        if total_waits == 0 { return 0.0; }
        self.fired_count as f64 / total_waits as f64
    }
}

impl EpollInstance {
    pub fn new(epfd: u64, pid: u64, created_ns: u64) -> Self {
        Self {
            epfd,
            owner_pid: pid,
            created_ns,
            fds: Vec::new(),
            wait_count: 0,
            event_count: 0,
            timeout_count: 0,
            max_events_param: 64,
        }
    }

    pub fn add_fd(&mut self, fd: u64, events: PollEvents) {
        if !self.fds.iter().any(|e| e.fd == fd) {
            self.fds.push(EpollFdEntry::new(fd, events));
        }
    }

    pub fn modify_fd(&mut self, fd: u64, events: PollEvents) {
        if let Some(entry) = self.fds.iter_mut().find(|e| e.fd == fd) {
            entry.events = events;
        }
    }

    pub fn remove_fd(&mut self, fd: u64) {
        if let Some(pos) = self.fds.iter().position(|e| e.fd == fd) {
            self.fds.swap_remove(pos);
        }
    }

    pub fn fd_count(&self) -> usize {
        self.fds.len()
    }

    pub fn record_wait(&mut self, events_returned: u32) {
        self.wait_count += 1;
        self.event_count += events_returned as u64;
        if events_returned == 0 {
            self.timeout_count += 1;
        }
    }

    pub fn fire_fd(&mut self, fd: u64, now_ns: u64) {
        if let Some(entry) = self.fds.iter_mut().find(|e| e.fd == fd) {
            entry.fire(now_ns);
        }
    }

    pub fn avg_events_per_wait(&self) -> f64 {
        if self.wait_count == 0 { return 0.0; }
        self.event_count as f64 / self.wait_count as f64
    }

    pub fn timeout_rate(&self) -> f64 {
        if self.wait_count == 0 { return 0.0; }
        self.timeout_count as f64 / self.wait_count as f64
    }

    pub fn stale_fds(&self) -> Vec<u64> {
        self.fds.iter()
            .filter(|e| e.fired_count == 0 && self.wait_count > 10)
            .map(|e| e.fd)
            .collect()
    }

    pub fn hottest_fds(&self, top: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = self.fds.iter().map(|e| (e.fd, e.fired_count)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }
}

/// Per-app poll behavior profile
#[derive(Debug)]
pub struct AppPollProfile {
    pub pid: u64,
    pub preferred_mechanism: PollMechanism,
    pub total_poll_calls: u64,
    pub total_fds_watched: u64,
    pub busy_wait_detected: u64,
    pub thundering_herd_risk: u32,
    pub epoll_instances: Vec<u64>,
}

impl AppPollProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            preferred_mechanism: PollMechanism::Epoll,
            total_poll_calls: 0,
            total_fds_watched: 0,
            busy_wait_detected: 0,
            thundering_herd_risk: 0,
            epoll_instances: Vec::new(),
        }
    }

    pub fn record_poll_call(&mut self, fd_count: u32) {
        self.total_poll_calls += 1;
        self.total_fds_watched += fd_count as u64;
    }

    pub fn avg_fds_per_call(&self) -> f64 {
        if self.total_poll_calls == 0 { return 0.0; }
        self.total_fds_watched as f64 / self.total_poll_calls as f64
    }

    pub fn should_upgrade_mechanism(&self) -> bool {
        self.avg_fds_per_call() > 100.0 && self.preferred_mechanism.scalability_score() < 4
    }
}

/// Poll manager stats
#[derive(Debug, Clone)]
pub struct PollMgrStats {
    pub total_epoll_instances: u64,
    pub total_poll_calls: u64,
    pub total_events_delivered: u64,
    pub total_timeouts: u64,
    pub busy_wait_detections: u64,
    pub upgrade_suggestions: u64,
}

/// Main poll manager
pub struct AppPollMgr {
    epoll_instances: BTreeMap<u64, EpollInstance>,
    app_profiles: BTreeMap<u64, AppPollProfile>,
    next_epfd: u64,
    stats: PollMgrStats,
}

impl AppPollMgr {
    pub fn new() -> Self {
        Self {
            epoll_instances: BTreeMap::new(),
            app_profiles: BTreeMap::new(),
            next_epfd: 1,
            stats: PollMgrStats {
                total_epoll_instances: 0,
                total_poll_calls: 0,
                total_events_delivered: 0,
                total_timeouts: 0,
                busy_wait_detections: 0,
                upgrade_suggestions: 0,
            },
        }
    }

    pub fn register_app(&mut self, pid: u64) {
        self.app_profiles.insert(pid, AppPollProfile::new(pid));
    }

    pub fn create_epoll(&mut self, pid: u64, timestamp_ns: u64) -> u64 {
        let epfd = self.next_epfd;
        self.next_epfd += 1;
        self.epoll_instances.insert(epfd, EpollInstance::new(epfd, pid, timestamp_ns));
        if let Some(prof) = self.app_profiles.get_mut(&pid) {
            prof.epoll_instances.push(epfd);
        }
        self.stats.total_epoll_instances += 1;
        epfd
    }

    pub fn epoll_ctl_add(&mut self, epfd: u64, fd: u64, events: PollEvents) {
        if let Some(inst) = self.epoll_instances.get_mut(&epfd) {
            inst.add_fd(fd, events);
        }
    }

    pub fn epoll_ctl_mod(&mut self, epfd: u64, fd: u64, events: PollEvents) {
        if let Some(inst) = self.epoll_instances.get_mut(&epfd) {
            inst.modify_fd(fd, events);
        }
    }

    pub fn epoll_ctl_del(&mut self, epfd: u64, fd: u64) {
        if let Some(inst) = self.epoll_instances.get_mut(&epfd) {
            inst.remove_fd(fd);
        }
    }

    pub fn record_wait(&mut self, epfd: u64, events_returned: u32) {
        self.stats.total_poll_calls += 1;
        self.stats.total_events_delivered += events_returned as u64;
        if events_returned == 0 {
            self.stats.total_timeouts += 1;
        }
        if let Some(inst) = self.epoll_instances.get_mut(&epfd) {
            inst.record_wait(events_returned);
            if let Some(prof) = self.app_profiles.get_mut(&inst.owner_pid) {
                prof.record_poll_call(inst.fd_count() as u32);
            }
        }
    }

    pub fn record_poll_syscall(&mut self, pid: u64, mechanism: PollMechanism, fd_count: u32) {
        self.stats.total_poll_calls += 1;
        if let Some(prof) = self.app_profiles.get_mut(&pid) {
            prof.preferred_mechanism = mechanism;
            prof.record_poll_call(fd_count);
            if prof.should_upgrade_mechanism() {
                self.stats.upgrade_suggestions += 1;
            }
        }
    }

    pub fn detect_busy_wait(&mut self, pid: u64, calls_per_second: u64) {
        if calls_per_second > 10_000 {
            self.stats.busy_wait_detections += 1;
            if let Some(prof) = self.app_profiles.get_mut(&pid) {
                prof.busy_wait_detected += 1;
            }
        }
    }

    pub fn worst_timeout_rates(&self, top: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<(u64, f64)> = self.epoll_instances.iter()
            .filter(|(_, inst)| inst.wait_count > 5)
            .map(|(&epfd, inst)| (epfd, inst.timeout_rate()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(top);
        v
    }

    pub fn get_epoll(&self, epfd: u64) -> Option<&EpollInstance> {
        self.epoll_instances.get(&epfd)
    }

    pub fn get_profile(&self, pid: u64) -> Option<&AppPollProfile> {
        self.app_profiles.get(&pid)
    }

    pub fn stats(&self) -> &PollMgrStats {
        &self.stats
    }
}

// ============================================================================
// Merged from poll_v2_mgr
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollV2Event {
    In,
    Out,
    Pri,
    Err,
    Hup,
    Nval,
    RdNorm,
    RdBand,
    WrNorm,
    WrBand,
}

/// A poll fd entry
#[derive(Debug, Clone)]
pub struct PollV2Fd {
    pub fd: u64,
    pub events: u16,
    pub revents: u16,
    pub poll_count: u64,
    pub ready_count: u64,
}

/// A poll call instance
#[derive(Debug, Clone)]
pub struct PollV2Call {
    pub id: u64,
    pub fds: Vec<PollV2Fd>,
    pub timeout_ms: i64,
    pub started_tick: u64,
    pub is_ppoll: bool,
    pub signal_mask: Option<u64>,
    pub ready_count: u32,
    pub completed: bool,
}

impl PollV2Call {
    pub fn new(id: u64, timeout_ms: i64, is_ppoll: bool, tick: u64) -> Self {
        Self {
            id,
            fds: Vec::new(),
            timeout_ms,
            started_tick: tick,
            is_ppoll,
            signal_mask: None,
            ready_count: 0,
            completed: false,
        }
    }

    pub fn add_fd(&mut self, fd: u64, events: u16) {
        self.fds.push(PollV2Fd {
            fd, events, revents: 0,
            poll_count: 0, ready_count: 0,
        });
    }

    pub fn mark_ready(&mut self, fd: u64, revents: u16) -> bool {
        for entry in self.fds.iter_mut() {
            if entry.fd == fd {
                entry.revents = revents;
                entry.ready_count += 1;
                self.ready_count += 1;
                return true;
            }
        }
        false
    }
}

/// Statistics for poll V2 manager
#[derive(Debug, Clone)]
pub struct PollV2MgrStats {
    pub total_poll_calls: u64,
    pub total_ppoll_calls: u64,
    pub total_fds_polled: u64,
    pub total_ready: u64,
    pub timeouts: u64,
    pub max_fds_in_call: u32,
    pub signal_interrupts: u64,
}

/// Main poll V2 manager
#[derive(Debug)]
pub struct AppPollV2Mgr {
    active_calls: BTreeMap<u64, PollV2Call>,
    next_id: u64,
    stats: PollV2MgrStats,
}

impl AppPollV2Mgr {
    pub fn new() -> Self {
        Self {
            active_calls: BTreeMap::new(),
            next_id: 1,
            stats: PollV2MgrStats {
                total_poll_calls: 0, total_ppoll_calls: 0,
                total_fds_polled: 0, total_ready: 0,
                timeouts: 0, max_fds_in_call: 0,
                signal_interrupts: 0,
            },
        }
    }

    pub fn begin_poll(&mut self, timeout_ms: i64, is_ppoll: bool, tick: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.active_calls.insert(id, PollV2Call::new(id, timeout_ms, is_ppoll, tick));
        if is_ppoll { self.stats.total_ppoll_calls += 1; }
        else { self.stats.total_poll_calls += 1; }
        id
    }

    pub fn add_fd(&mut self, call_id: u64, fd: u64, events: u16) -> bool {
        if let Some(call) = self.active_calls.get_mut(&call_id) {
            call.add_fd(fd, events);
            self.stats.total_fds_polled += 1;
            let n = call.fds.len() as u32;
            if n > self.stats.max_fds_in_call {
                self.stats.max_fds_in_call = n;
            }
            true
        } else { false }
    }

    pub fn complete_poll(&mut self, call_id: u64) -> Option<u32> {
        if let Some(call) = self.active_calls.get_mut(&call_id) {
            call.completed = true;
            let ready = call.ready_count;
            self.stats.total_ready += ready as u64;
            if ready == 0 { self.stats.timeouts += 1; }
            Some(ready)
        } else { None }
    }

    pub fn stats(&self) -> &PollV2MgrStats {
        &self.stats
    }
}
