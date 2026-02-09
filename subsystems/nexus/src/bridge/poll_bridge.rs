//! # Bridge Poll Bridge
//!
//! Poll/select/ppoll syscall bridging:
//! - File descriptor poll event translation
//! - Poll table management
//! - Timeout tracking and conversion
//! - Edge-triggered vs level-triggered handling
//! - Poll wait queue integration
//! - Scalability metrics for large fd sets

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Poll event flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PollEvents {
    pub bits: u32,
}

impl PollEvents {
    pub const POLLIN: u32 = 0x001;
    pub const POLLPRI: u32 = 0x002;
    pub const POLLOUT: u32 = 0x004;
    pub const POLLERR: u32 = 0x008;
    pub const POLLHUP: u32 = 0x010;
    pub const POLLNVAL: u32 = 0x020;
    pub const POLLRDNORM: u32 = 0x040;
    pub const POLLRDBAND: u32 = 0x080;
    pub const POLLWRNORM: u32 = 0x100;
    pub const POLLWRBAND: u32 = 0x200;
    pub const POLLRDHUP: u32 = 0x2000;

    #[inline(always)]
    pub fn empty() -> Self { Self { bits: 0 } }
    pub fn new(bits: u32) -> Self { Self { bits } }
    #[inline(always)]
    pub fn has(&self, flag: u32) -> bool { self.bits & flag != 0 }
    #[inline(always)]
    pub fn set(&mut self, flag: u32) { self.bits |= flag; }
    #[inline(always)]
    pub fn clear(&mut self, flag: u32) { self.bits &= !flag; }
    #[inline(always)]
    pub fn is_readable(&self) -> bool { self.has(Self::POLLIN) || self.has(Self::POLLRDNORM) }
    #[inline(always)]
    pub fn is_writable(&self) -> bool { self.has(Self::POLLOUT) || self.has(Self::POLLWRNORM) }
    #[inline(always)]
    pub fn is_error(&self) -> bool { self.has(Self::POLLERR) || self.has(Self::POLLHUP) || self.has(Self::POLLNVAL) }
}

/// Poll syscall variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollVariant {
    Select,
    Poll,
    Ppoll,
    Epoll,
    Io_uring_poll,
}

/// Poll fd entry
#[derive(Debug, Clone)]
pub struct PollFdEntry {
    pub fd: i32,
    pub requested_events: PollEvents,
    pub returned_events: PollEvents,
    pub wait_queue_registered: bool,
}

impl PollFdEntry {
    pub fn new(fd: i32, events: PollEvents) -> Self {
        Self { fd, requested_events: events, returned_events: PollEvents::empty(), wait_queue_registered: false }
    }

    #[inline(always)]
    pub fn is_ready(&self) -> bool { self.returned_events.bits & self.requested_events.bits != 0 || self.returned_events.is_error() }
}

/// Poll request
#[derive(Debug, Clone)]
pub struct PollRequest {
    pub request_id: u64,
    pub variant: PollVariant,
    pub fds: Vec<PollFdEntry>,
    pub timeout_ms: i64,
    pub start_ts: u64,
    pub end_ts: u64,
    pub ready_count: u32,
    pub timed_out: bool,
    pub interrupted: bool,
}

impl PollRequest {
    pub fn new(id: u64, variant: PollVariant, ts: u64) -> Self {
        Self {
            request_id: id, variant, fds: Vec::new(), timeout_ms: -1,
            start_ts: ts, end_ts: 0, ready_count: 0, timed_out: false,
            interrupted: false,
        }
    }

    #[inline(always)]
    pub fn add_fd(&mut self, fd: i32, events: PollEvents) {
        self.fds.push(PollFdEntry::new(fd, events));
    }

    #[inline(always)]
    pub fn complete(&mut self, ts: u64) {
        self.end_ts = ts;
        self.ready_count = self.fds.iter().filter(|f| f.is_ready()).count() as u32;
    }

    #[inline(always)]
    pub fn wall_time_ns(&self) -> u64 { self.end_ts.saturating_sub(self.start_ts) }
    #[inline(always)]
    pub fn fd_count(&self) -> usize { self.fds.len() }
}

/// Per-fd poll statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FdPollStats {
    pub fd: i32,
    pub poll_count: u64,
    pub ready_count: u64,
    pub avg_wait_ns: u64,
    pub total_wait_ns: u64,
}

impl FdPollStats {
    pub fn new(fd: i32) -> Self {
        Self { fd, poll_count: 0, ready_count: 0, avg_wait_ns: 0, total_wait_ns: 0 }
    }

    #[inline(always)]
    pub fn ready_ratio(&self) -> f64 {
        if self.poll_count == 0 { 0.0 } else { self.ready_count as f64 / self.poll_count as f64 }
    }
}

/// Poll bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct PollBridgeStats {
    pub total_requests: u64,
    pub select_calls: u64,
    pub poll_calls: u64,
    pub ppoll_calls: u64,
    pub avg_fd_count: f64,
    pub max_fd_count: usize,
    pub avg_wait_ns: u64,
    pub timeout_ratio: f64,
    pub total_fds_tracked: usize,
}

/// Bridge poll manager
#[repr(align(64))]
pub struct BridgePollBridge {
    requests: VecDeque<PollRequest>,
    fd_stats: BTreeMap<i32, FdPollStats>,
    max_requests: usize,
    next_id: u64,
    stats: PollBridgeStats,
}

impl BridgePollBridge {
    pub fn new() -> Self {
        Self { requests: VecDeque::new(), fd_stats: BTreeMap::new(), max_requests: 1024, next_id: 1, stats: PollBridgeStats::default() }
    }

    #[inline]
    pub fn begin_poll(&mut self, variant: PollVariant, ts: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.requests.push_back(PollRequest::new(id, variant, ts));
        id
    }

    #[inline]
    pub fn add_fd(&mut self, req_id: u64, fd: i32, events: PollEvents) {
        if let Some(r) = self.requests.iter_mut().find(|r| r.request_id == req_id) {
            r.add_fd(fd, events);
        }
    }

    pub fn complete_poll(&mut self, req_id: u64, ts: u64) {
        if let Some(r) = self.requests.iter_mut().find(|r| r.request_id == req_id) {
            r.complete(ts);
            let wait = r.wall_time_ns();
            for fde in &r.fds {
                let fs = self.fd_stats.entry(fde.fd).or_insert_with(|| FdPollStats::new(fde.fd));
                fs.poll_count += 1;
                fs.total_wait_ns += wait;
                if fde.is_ready() { fs.ready_count += 1; }
                fs.avg_wait_ns = if fs.poll_count == 0 { 0 } else { fs.total_wait_ns / fs.poll_count };
            }
        }
        if self.requests.len() > self.max_requests { self.requests.pop_front(); }
    }

    pub fn recompute(&mut self) {
        self.stats.total_requests = self.requests.len() as u64;
        self.stats.select_calls = self.requests.iter().filter(|r| r.variant == PollVariant::Select).count() as u64;
        self.stats.poll_calls = self.requests.iter().filter(|r| r.variant == PollVariant::Poll).count() as u64;
        self.stats.ppoll_calls = self.requests.iter().filter(|r| r.variant == PollVariant::Ppoll).count() as u64;
        let fd_counts: Vec<f64> = self.requests.iter().map(|r| r.fd_count() as f64).collect();
        self.stats.avg_fd_count = if fd_counts.is_empty() { 0.0 } else { fd_counts.iter().sum::<f64>() / fd_counts.len() as f64 };
        self.stats.max_fd_count = self.requests.iter().map(|r| r.fd_count()).max().unwrap_or(0);
        let waits: Vec<u64> = self.requests.iter().filter(|r| r.end_ts > 0).map(|r| r.wall_time_ns()).collect();
        self.stats.avg_wait_ns = if waits.is_empty() { 0 } else { waits.iter().sum::<u64>() / waits.len() as u64 };
        let total = self.requests.len() as f64;
        self.stats.timeout_ratio = if total <= 0.0 { 0.0 } else { self.requests.iter().filter(|r| r.timed_out).count() as f64 / total };
        self.stats.total_fds_tracked = self.fd_stats.len();
    }

    #[inline(always)]
    pub fn fd_stats(&self, fd: i32) -> Option<&FdPollStats> { self.fd_stats.get(&fd) }
    #[inline(always)]
    pub fn stats(&self) -> &PollBridgeStats { &self.stats }
}

// ============================================================================
// Merged from poll_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollV2Event {
    PollIn,
    PollOut,
    PollPri,
    PollErr,
    PollHup,
    PollNval,
    PollRdNorm,
    PollWrNorm,
    PollRdBand,
    PollWrBand,
}

/// Poll v2 fd entry
#[derive(Debug, Clone)]
pub struct PollV2FdEntry {
    pub fd: i32,
    pub requested_events: u32,
    pub returned_events: u32,
    pub poll_count: u64,
    pub ready_count: u64,
    pub last_ready_ns: u64,
}

impl PollV2FdEntry {
    pub fn new(fd: i32, events: u32) -> Self {
        Self {
            fd,
            requested_events: events,
            returned_events: 0,
            poll_count: 0,
            ready_count: 0,
            last_ready_ns: 0,
        }
    }

    #[inline]
    pub fn check(&mut self, available: u32, ts_ns: u64) -> bool {
        self.poll_count += 1;
        self.returned_events = self.requested_events & available;
        if self.returned_events != 0 {
            self.ready_count += 1;
            self.last_ready_ns = ts_ns;
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn readiness_rate(&self) -> f64 {
        if self.poll_count == 0 { 0.0 } else { self.ready_count as f64 / self.poll_count as f64 }
    }
}

/// Poll v2 call record
#[derive(Debug, Clone)]
pub struct PollV2Call {
    pub call_id: u64,
    pub fds: Vec<PollV2FdEntry>,
    pub timeout_ms: i32,
    pub ready_count: u32,
    pub timed_out: bool,
    pub signaled: bool,
    pub duration_ns: u64,
}

impl PollV2Call {
    pub fn new(call_id: u64, timeout_ms: i32) -> Self {
        Self {
            call_id,
            fds: Vec::new(),
            timeout_ms,
            ready_count: 0,
            timed_out: false,
            signaled: false,
            duration_ns: 0,
        }
    }

    #[inline(always)]
    pub fn add_fd(&mut self, fd: i32, events: u32) {
        self.fds.push(PollV2FdEntry::new(fd, events));
    }

    #[inline(always)]
    pub fn fd_count(&self) -> usize {
        self.fds.len()
    }
}

/// Poll v2 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PollV2BridgeStats {
    pub total_calls: u64,
    pub total_fds_polled: u64,
    pub total_ready: u64,
    pub timeouts: u64,
    pub avg_fds_per_call: f64,
}

/// Main bridge poll v2
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgePollV2 {
    pub stats: PollV2BridgeStats,
    pub fd_history: BTreeMap<i32, u64>,
    pub next_call_id: u64,
}

impl BridgePollV2 {
    pub fn new() -> Self {
        Self {
            stats: PollV2BridgeStats {
                total_calls: 0,
                total_fds_polled: 0,
                total_ready: 0,
                timeouts: 0,
                avg_fds_per_call: 0.0,
            },
            fd_history: BTreeMap::new(),
            next_call_id: 1,
        }
    }

    pub fn record_call(&mut self, call: &PollV2Call) {
        self.stats.total_calls += 1;
        self.stats.total_fds_polled += call.fds.len() as u64;
        self.stats.total_ready += call.ready_count as u64;
        if call.timed_out {
            self.stats.timeouts += 1;
        }
        for entry in &call.fds {
            *self.fd_history.entry(entry.fd).or_insert(0) += 1;
        }
        if self.stats.total_calls > 0 {
            self.stats.avg_fds_per_call = self.stats.total_fds_polled as f64 / self.stats.total_calls as f64;
        }
    }

    #[inline(always)]
    pub fn most_polled_fd(&self) -> Option<(i32, u64)> {
        self.fd_history.iter().max_by_key(|(_, &v)| v).map(|(&k, &v)| (k, v))
    }

    #[inline(always)]
    pub fn timeout_rate(&self) -> f64 {
        if self.stats.total_calls == 0 { 0.0 } else { self.stats.timeouts as f64 / self.stats.total_calls as f64 }
    }
}

// ============================================================================
// Merged from poll_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PollV3Event { PollIn, PollOut, PollErr, PollHup, PollNval }

/// Poll v3 record
#[derive(Debug, Clone)]
pub struct PollV3Record {
    pub event: PollV3Event,
    pub fd: i32,
    pub nfds: u32,
    pub timeout_ms: i32,
    pub ready: u32,
}

impl PollV3Record {
    pub fn new(event: PollV3Event, fd: i32) -> Self { Self { event, fd, nfds: 0, timeout_ms: -1, ready: 0 } }
}

/// Poll v3 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PollV3BridgeStats { pub total_polls: u64, pub ready_events: u64, pub errors: u64, pub timeouts: u64 }

/// Main bridge poll v3
#[derive(Debug)]
pub struct BridgePollV3 { pub stats: PollV3BridgeStats }

impl BridgePollV3 {
    pub fn new() -> Self { Self { stats: PollV3BridgeStats { total_polls: 0, ready_events: 0, errors: 0, timeouts: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &PollV3Record) {
        self.stats.total_polls += 1;
        match rec.event {
            PollV3Event::PollIn | PollV3Event::PollOut => self.stats.ready_events += 1,
            PollV3Event::PollErr | PollV3Event::PollNval => self.stats.errors += 1,
            PollV3Event::PollHup => self.stats.timeouts += 1,
        }
    }
}
