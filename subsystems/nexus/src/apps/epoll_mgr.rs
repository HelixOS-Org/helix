//! # Apps Epoll Manager
//!
//! Application-level epoll usage profiling:
//! - Epoll instance tracking per process
//! - FD interest set monitoring
//! - Event readiness pattern analysis
//! - Thundering herd detection
//! - Level vs edge trigger profiling
//! - Stale FD detection in epoll sets

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Epoll trigger mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollTriggerMode {
    LevelTriggered,
    EdgeTriggered,
    OneShot,
    Exclusive,
}

/// Epoll event flags
#[derive(Debug, Clone, Copy)]
pub struct EpollEventMask {
    pub bits: u32,
}

impl EpollEventMask {
    pub const EPOLLIN: u32 = 0x001;
    pub const EPOLLOUT: u32 = 0x004;
    pub const EPOLLERR: u32 = 0x008;
    pub const EPOLLHUP: u32 = 0x010;
    pub const EPOLLRDHUP: u32 = 0x2000;
    pub const EPOLLONESHOT: u32 = 0x40000000;
    pub const EPOLLET: u32 = 0x80000000;

    pub fn new(bits: u32) -> Self { Self { bits } }
    #[inline(always)]
    pub fn empty() -> Self { Self { bits: 0 } }
    #[inline(always)]
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
    #[inline(always)]
    pub fn is_read(&self) -> bool { self.has(Self::EPOLLIN) }
    #[inline(always)]
    pub fn is_write(&self) -> bool { self.has(Self::EPOLLOUT) }
    #[inline(always)]
    pub fn is_edge(&self) -> bool { self.has(Self::EPOLLET) }
}

/// FD registered in epoll
#[derive(Debug, Clone)]
pub struct EpollRegisteredFd {
    pub fd: i32,
    pub events: EpollEventMask,
    pub trigger: EpollTriggerMode,
    pub ready_count: u64,
    pub spurious_count: u64,
    pub last_ready_ts: u64,
    pub registered_ts: u64,
    pub stale_threshold_ns: u64,
}

impl EpollRegisteredFd {
    pub fn new(fd: i32, events: EpollEventMask, ts: u64) -> Self {
        let trigger = if events.is_edge() { EpollTriggerMode::EdgeTriggered }
                      else if events.has(EpollEventMask::EPOLLONESHOT) { EpollTriggerMode::OneShot }
                      else { EpollTriggerMode::LevelTriggered };
        Self {
            fd, events, trigger, ready_count: 0, spurious_count: 0,
            last_ready_ts: 0, registered_ts: ts, stale_threshold_ns: 30_000_000_000,
        }
    }

    #[inline(always)]
    pub fn record_ready(&mut self, ts: u64) {
        self.ready_count += 1;
        self.last_ready_ts = ts;
    }

    #[inline(always)]
    pub fn is_stale(&self, now: u64) -> bool {
        if self.last_ready_ts == 0 { return now - self.registered_ts > self.stale_threshold_ns; }
        now - self.last_ready_ts > self.stale_threshold_ns
    }

    #[inline]
    pub fn spurious_ratio(&self) -> f64 {
        let total = self.ready_count + self.spurious_count;
        if total == 0 { return 0.0; }
        self.spurious_count as f64 / total as f64
    }
}

/// Epoll instance
#[derive(Debug, Clone)]
pub struct EpollInstance {
    pub epfd: i32,
    pub pid: u64,
    pub fds: BTreeMap<i32, EpollRegisteredFd>,
    pub wait_count: u64,
    pub timeout_count: u64,
    pub total_events_returned: u64,
    pub max_events_per_wait: u32,
    pub avg_latency_ns: u64,
    pub created_ts: u64,
    pub last_wait_ts: u64,
}

impl EpollInstance {
    pub fn new(epfd: i32, pid: u64, ts: u64) -> Self {
        Self {
            epfd, pid, fds: BTreeMap::new(), wait_count: 0, timeout_count: 0,
            total_events_returned: 0, max_events_per_wait: 0, avg_latency_ns: 0,
            created_ts: ts, last_wait_ts: 0,
        }
    }

    #[inline(always)]
    pub fn add_fd(&mut self, fd: i32, events: EpollEventMask, ts: u64) {
        self.fds.insert(fd, EpollRegisteredFd::new(fd, events, ts));
    }

    #[inline(always)]
    pub fn remove_fd(&mut self, fd: i32) { self.fds.remove(&fd); }

    #[inline]
    pub fn record_wait(&mut self, events_returned: u32, latency_ns: u64, ts: u64) {
        self.wait_count += 1;
        self.last_wait_ts = ts;
        self.total_events_returned += events_returned as u64;
        if events_returned == 0 { self.timeout_count += 1; }
        if events_returned > self.max_events_per_wait { self.max_events_per_wait = events_returned; }
        // Exponential moving average of latency
        if self.avg_latency_ns == 0 { self.avg_latency_ns = latency_ns; }
        else { self.avg_latency_ns = (self.avg_latency_ns * 7 + latency_ns) / 8; }
    }

    #[inline(always)]
    pub fn fd_count(&self) -> usize { self.fds.len() }
    #[inline(always)]
    pub fn timeout_ratio(&self) -> f64 {
        if self.wait_count == 0 { return 0.0; }
        self.timeout_count as f64 / self.wait_count as f64
    }
    #[inline(always)]
    pub fn avg_events_per_wait(&self) -> f64 {
        if self.wait_count == 0 { return 0.0; }
        self.total_events_returned as f64 / self.wait_count as f64
    }
    #[inline(always)]
    pub fn stale_fds(&self, now: u64) -> Vec<i32> {
        self.fds.iter().filter(|(_, f)| f.is_stale(now)).map(|(&fd, _)| fd).collect()
    }
}

/// Thundering herd detection
#[derive(Debug, Clone)]
pub struct ThunderingHerdDetector {
    pub recent_concurrent_wakes: VecDeque<(u64, u32)>,
    pub max_history: usize,
    pub threshold: u32,
    pub detected_count: u64,
}

impl ThunderingHerdDetector {
    pub fn new(threshold: u32) -> Self {
        Self { recent_concurrent_wakes: VecDeque::new(), max_history: 64, threshold, detected_count: 0 }
    }

    #[inline]
    pub fn record_wake(&mut self, ts: u64, woken_count: u32) {
        self.recent_concurrent_wakes.push_back((ts, woken_count));
        if self.recent_concurrent_wakes.len() > self.max_history { self.recent_concurrent_wakes.pop_front(); }
        if woken_count >= self.threshold { self.detected_count += 1; }
    }

    #[inline]
    pub fn is_thundering(&self) -> bool {
        if self.recent_concurrent_wakes.len() < 3 { return false; }
        let recent = &self.recent_concurrent_wakes[self.recent_concurrent_wakes.len()-3..];
        recent.iter().all(|(_, c)| *c >= self.threshold)
    }
}

/// Epoll manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct EpollMgrStats {
    pub total_instances: usize,
    pub total_monitored_fds: usize,
    pub total_waits: u64,
    pub total_timeouts: u64,
    pub stale_fd_count: usize,
    pub thundering_herd_events: u64,
    pub avg_fds_per_instance: f64,
}

/// Application epoll manager
pub struct AppsEpollMgr {
    instances: BTreeMap<(u64, i32), EpollInstance>,
    herd_detector: ThunderingHerdDetector,
    stats: EpollMgrStats,
}

impl AppsEpollMgr {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            herd_detector: ThunderingHerdDetector::new(4),
            stats: EpollMgrStats::default(),
        }
    }

    #[inline(always)]
    pub fn create(&mut self, pid: u64, epfd: i32, ts: u64) {
        self.instances.insert((pid, epfd), EpollInstance::new(epfd, pid, ts));
    }

    #[inline(always)]
    pub fn ctl_add(&mut self, pid: u64, epfd: i32, fd: i32, events: EpollEventMask, ts: u64) {
        if let Some(inst) = self.instances.get_mut(&(pid, epfd)) { inst.add_fd(fd, events, ts); }
    }

    #[inline(always)]
    pub fn ctl_del(&mut self, pid: u64, epfd: i32, fd: i32) {
        if let Some(inst) = self.instances.get_mut(&(pid, epfd)) { inst.remove_fd(fd); }
    }

    #[inline(always)]
    pub fn record_wait(&mut self, pid: u64, epfd: i32, events: u32, latency: u64, ts: u64) {
        if let Some(inst) = self.instances.get_mut(&(pid, epfd)) { inst.record_wait(events, latency, ts); }
    }

    #[inline(always)]
    pub fn destroy(&mut self, pid: u64, epfd: i32) { self.instances.remove(&(pid, epfd)); }

    #[inline]
    pub fn recompute(&mut self, now: u64) {
        self.stats.total_instances = self.instances.len();
        self.stats.total_monitored_fds = self.instances.values().map(|i| i.fd_count()).sum();
        self.stats.total_waits = self.instances.values().map(|i| i.wait_count).sum();
        self.stats.total_timeouts = self.instances.values().map(|i| i.timeout_count).sum();
        self.stats.stale_fd_count = self.instances.values().map(|i| i.stale_fds(now).len()).sum();
        self.stats.thundering_herd_events = self.herd_detector.detected_count;
        if self.stats.total_instances > 0 {
            self.stats.avg_fds_per_instance = self.stats.total_monitored_fds as f64 / self.stats.total_instances as f64;
        }
    }

    #[inline(always)]
    pub fn instance(&self, pid: u64, epfd: i32) -> Option<&EpollInstance> { self.instances.get(&(pid, epfd)) }
    #[inline(always)]
    pub fn stats(&self) -> &EpollMgrStats { &self.stats }
}
