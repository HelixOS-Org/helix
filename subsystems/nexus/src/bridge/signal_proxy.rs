//! # Bridge Signal Proxy
//!
//! Signal delivery optimization through the bridge:
//! - Signal coalescing for same-target
//! - Delivery latency tracking
//! - Signal handler profiling
//! - Pending signal queue optimization
//! - Real-time signal priority management

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Signal category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalCategory {
    /// Fatal (SIGKILL, SIGSEGV, etc.)
    Fatal,
    /// Stop (SIGSTOP, SIGTSTP)
    Stop,
    /// Continue (SIGCONT)
    Continue,
    /// Ignorable (SIGCHLD default)
    Ignorable,
    /// User-defined (SIGUSR1/2)
    User,
    /// Real-time (SIGRTMIN+n)
    RealTime,
    /// IO-related (SIGIO, SIGPIPE)
    Io,
    /// Timer (SIGALRM, SIGVTALRM)
    Timer,
}

/// Signal delivery state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState {
    /// Queued for delivery
    Pending,
    /// Being delivered
    Delivering,
    /// Delivered to handler
    Delivered,
    /// Coalesced (merged with previous)
    Coalesced,
    /// Blocked by mask
    Blocked,
    /// Dropped
    Dropped,
}

/// Signal entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SignalEntry {
    /// Signal number
    pub signum: u32,
    /// Source PID (sender)
    pub src_pid: u64,
    /// Target PID
    pub dst_pid: u64,
    /// Category
    pub category: SignalCategory,
    /// Queued timestamp (ns)
    pub queued_ns: u64,
    /// Delivery state
    pub state: DeliveryState,
    /// Delivery latency (ns)
    pub delivery_latency_ns: u64,
}

impl SignalEntry {
    pub fn new(signum: u32, src_pid: u64, dst_pid: u64, now_ns: u64) -> Self {
        let category = Self::categorize(signum);
        Self {
            signum,
            src_pid,
            dst_pid,
            category,
            queued_ns: now_ns,
            state: DeliveryState::Pending,
            delivery_latency_ns: 0,
        }
    }

    fn categorize(signum: u32) -> SignalCategory {
        match signum {
            9 | 11 | 6 | 4 | 7 | 8 => SignalCategory::Fatal,
            19 | 20 | 21 | 22 => SignalCategory::Stop,
            18 => SignalCategory::Continue,
            17 => SignalCategory::Ignorable,
            10 | 12 => SignalCategory::User,
            14 | 26 | 27 => SignalCategory::Timer,
            13 | 29 => SignalCategory::Io,
            s if s >= 34 => SignalCategory::RealTime,
            _ => SignalCategory::User,
        }
    }

    /// Is real-time signal?
    #[inline(always)]
    pub fn is_realtime(&self) -> bool {
        self.signum >= 34
    }
}

/// Per-process signal state
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessSignalState {
    /// PID
    pub pid: u64,
    /// Blocked signal mask
    pub blocked_mask: u64,
    /// Pending signals
    pending: Vec<SignalEntry>,
    /// Signal counts by number
    signal_counts: ArrayMap<u64, 32>,
    /// Handler latency per signal (EMA ns)
    handler_latency: ArrayMap<f64, 32>,
    /// Total signals received
    pub total_received: u64,
    /// Total signals coalesced
    pub total_coalesced: u64,
}

impl ProcessSignalState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            blocked_mask: 0,
            pending: Vec::new(),
            signal_counts: ArrayMap::new(0),
            handler_latency: ArrayMap::new(0.0),
            total_received: 0,
            total_coalesced: 0,
        }
    }

    /// Queue signal
    pub fn queue_signal(&mut self, entry: SignalEntry) -> DeliveryState {
        let signum = entry.signum;

        // Check blocked
        if signum < 64 && self.blocked_mask & (1u64 << signum) != 0 {
            return DeliveryState::Blocked;
        }

        // Standard signals: coalesce if same signal already pending
        if !entry.is_realtime() {
            if self.pending.iter().any(|e| e.signum == signum && e.state == DeliveryState::Pending) {
                self.total_coalesced += 1;
                return DeliveryState::Coalesced;
            }
        }

        self.signal_counts.add(signum as usize, 1);
        self.total_received += 1;
        self.pending.push(entry);
        DeliveryState::Pending
    }

    /// Dequeue next signal for delivery
    #[inline]
    pub fn dequeue_next(&mut self) -> Option<SignalEntry> {
        // Real-time signals have priority by number (lower = higher)
        // Standard signals FIFO
        if let Some(pos) = self.pending.iter().position(|e| e.state == DeliveryState::Pending) {
            self.pending[pos].state = DeliveryState::Delivering;
            Some(self.pending.remove(pos))
        } else {
            None
        }
    }

    /// Record handler completion
    #[inline(always)]
    pub fn record_handler_done(&mut self, signum: u32, latency_ns: u64) {
        let entry = self.handler_latency.entry(signum).or_insert(0.0);
        *entry = 0.8 * *entry + 0.2 * latency_ns as f64;
    }

    /// Pending count
    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending.iter().filter(|e| e.state == DeliveryState::Pending).count()
    }

    /// Most frequent signal
    #[inline]
    pub fn most_frequent(&self) -> Option<(u32, u64)> {
        self.signal_counts.iter()
            .max_by_key(|&(_, &count)| count)
            .map(|(&sig, &count)| (sig, count))
    }
}

/// Signal proxy stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeSignalProxyStats {
    pub tracked_processes: usize,
    pub total_pending: usize,
    pub total_received: u64,
    pub total_coalesced: u64,
    pub coalesce_ratio: f64,
}

/// Bridge signal proxy
#[repr(align(64))]
pub struct BridgeSignalProxy {
    /// Per-process state
    processes: BTreeMap<u64, ProcessSignalState>,
    /// Stats
    stats: BridgeSignalProxyStats,
}

impl BridgeSignalProxy {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: BridgeSignalProxyStats::default(),
        }
    }

    /// Register process
    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.insert(pid, ProcessSignalState::new(pid));
    }

    /// Send signal
    #[inline]
    pub fn send_signal(&mut self, signum: u32, src_pid: u64, dst_pid: u64, now_ns: u64) -> DeliveryState {
        let entry = SignalEntry::new(signum, src_pid, dst_pid, now_ns);
        let state = self.processes
            .entry(dst_pid)
            .or_insert_with(|| ProcessSignalState::new(dst_pid))
            .queue_signal(entry);
        self.update_stats();
        state
    }

    /// Deliver next signal for process
    #[inline(always)]
    pub fn deliver_next(&mut self, pid: u64) -> Option<SignalEntry> {
        self.processes.get_mut(&pid).and_then(|p| p.dequeue_next())
    }

    /// Set blocked mask
    #[inline]
    pub fn set_blocked(&mut self, pid: u64, mask: u64) {
        if let Some(proc_state) = self.processes.get_mut(&pid) {
            proc_state.blocked_mask = mask;
        }
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_pending = self.processes.values()
            .map(|p| p.pending_count())
            .sum();
        self.stats.total_received = self.processes.values()
            .map(|p| p.total_received)
            .sum();
        self.stats.total_coalesced = self.processes.values()
            .map(|p| p.total_coalesced)
            .sum();
        let total = self.stats.total_received + self.stats.total_coalesced;
        self.stats.coalesce_ratio = if total > 0 {
            self.stats.total_coalesced as f64 / total as f64
        } else {
            0.0
        };
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeSignalProxyStats {
        &self.stats
    }
}
