//! # Holistic Signal Dispatch
//!
//! Signal delivery and dispatch management:
//! - Signal queue per process/thread
//! - Pending and blocked signal masks
//! - Real-time signal priority ordering
//! - Signal handler registration tracking
//! - Coalescing for standard signals
//! - Signal delivery statistics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Signal number range
pub const SIGRTMIN: u32 = 34;
pub const SIGRTMAX: u32 = 64;
pub const MAX_SIGNALS: u32 = 64;

/// Signal class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalClass {
    Standard,
    RealTime,
    Stop,
    Kill,
    Ignore,
}

/// Signal action type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigAction {
    Default,
    Ignore,
    Handler,
    CoreDump,
    Stop,
    Continue,
}

/// Signal disposition
#[derive(Debug, Clone)]
pub struct SignalDisposition {
    pub signum: u32,
    pub action: SigAction,
    pub handler_addr: u64,
    pub flags: u32,
    pub restart: bool,
    pub nodefer: bool,
    pub oneshot: bool,
}

impl SignalDisposition {
    #[inline]
    pub fn default_for(signum: u32) -> Self {
        let action = match signum {
            9 => SigAction::Kill,
            19 => SigAction::Stop,
            17 => SigAction::Ignore,
            1 | 2 | 3 | 6 | 8 | 11 | 13 | 14 | 15 => SigAction::Default,
            _ => SigAction::Default,
        };
        Self { signum, action, handler_addr: 0, flags: 0, restart: false, nodefer: false, oneshot: false }
    }

    #[inline(always)]
    pub fn is_fatal(&self) -> bool {
        matches!(self.action, SigAction::Default | SigAction::CoreDump | SigAction::Kill)
    }
}

/// Signal mask (64-bit)
#[derive(Debug, Clone, Copy)]
pub struct SigMask {
    bits: u64,
}

impl SigMask {
    #[inline(always)]
    pub fn empty() -> Self { Self { bits: 0 } }
    #[inline(always)]
    pub fn full() -> Self { Self { bits: u64::MAX } }

    #[inline(always)]
    pub fn set(&mut self, sig: u32) { if sig > 0 && sig <= MAX_SIGNALS { self.bits |= 1u64 << (sig - 1); } }
    #[inline(always)]
    pub fn clear(&mut self, sig: u32) { if sig > 0 && sig <= MAX_SIGNALS { self.bits &= !(1u64 << (sig - 1)); } }
    #[inline(always)]
    pub fn is_set(&self, sig: u32) -> bool { if sig > 0 && sig <= MAX_SIGNALS { (self.bits >> (sig - 1)) & 1 == 1 } else { false } }
    #[inline(always)]
    pub fn count(&self) -> u32 { self.bits.count_ones() }
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.bits == 0 }

    #[inline(always)]
    pub fn block(&mut self, other: &SigMask) { self.bits |= other.bits; }
    #[inline(always)]
    pub fn unblock(&mut self, other: &SigMask) { self.bits &= !other.bits; }
    #[inline(always)]
    pub fn pending_unblocked(&self, blocked: &SigMask) -> SigMask { SigMask { bits: self.bits & !blocked.bits } }
}

/// Queued signal info
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct QueuedSignal {
    pub signum: u32,
    pub class: SignalClass,
    pub sender_pid: u64,
    pub value: i64,
    pub timestamp: u64,
    pub delivered: bool,
    pub delivery_ts: u64,
}

impl QueuedSignal {
    pub fn new(signum: u32, sender: u64, ts: u64) -> Self {
        let class = if signum == 9 { SignalClass::Kill }
            else if signum == 19 || signum == 20 { SignalClass::Stop }
            else if signum >= SIGRTMIN && signum <= SIGRTMAX { SignalClass::RealTime }
            else { SignalClass::Standard };
        Self { signum, class, sender_pid: sender, value: 0, timestamp: ts, delivered: false, delivery_ts: 0 }
    }

    #[inline(always)]
    pub fn deliver(&mut self, ts: u64) {
        self.delivered = true;
        self.delivery_ts = ts;
    }

    #[inline(always)]
    pub fn latency_ns(&self) -> u64 {
        if self.delivered { self.delivery_ts.saturating_sub(self.timestamp) } else { 0 }
    }
}

/// Per-process signal state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessSignalState {
    pub pid: u64,
    pub blocked: SigMask,
    pub pending: SigMask,
    pub queue: Vec<QueuedSignal>,
    pub dispositions: BTreeMap<u32, SignalDisposition>,
    pub signals_received: u64,
    pub signals_delivered: u64,
    pub signals_ignored: u64,
    pub signals_coalesced: u64,
    pub max_queue_len: usize,
}

impl ProcessSignalState {
    pub fn new(pid: u64) -> Self {
        let mut disps = BTreeMap::new();
        for sig in 1..=MAX_SIGNALS {
            disps.insert(sig, SignalDisposition::default_for(sig));
        }
        Self {
            pid, blocked: SigMask::empty(), pending: SigMask::empty(),
            queue: Vec::new(), dispositions: disps,
            signals_received: 0, signals_delivered: 0, signals_ignored: 0,
            signals_coalesced: 0, max_queue_len: 128,
        }
    }

    pub fn send_signal(&mut self, signum: u32, sender: u64, ts: u64) -> bool {
        if signum == 0 || signum > MAX_SIGNALS { return false; }
        self.signals_received += 1;

        // Check if ignored
        if let Some(disp) = self.dispositions.get(&signum) {
            if disp.action == SigAction::Ignore && signum != 9 && signum != 19 {
                self.signals_ignored += 1;
                return false;
            }
        }

        // Standard signals coalesce
        let is_rt = signum >= SIGRTMIN && signum <= SIGRTMAX;
        if !is_rt && self.pending.is_set(signum) {
            self.signals_coalesced += 1;
            return true;
        }

        if self.queue.len() >= self.max_queue_len && is_rt {
            return false; // RT queue full
        }

        self.pending.set(signum);
        self.queue.push(QueuedSignal::new(signum, sender, ts));
        true
    }

    pub fn dequeue_signal(&mut self, ts: u64) -> Option<QueuedSignal> {
        let deliverable = self.pending.pending_unblocked(&self.blocked);
        if deliverable.is_empty() { return None; }

        // Priority: SIGKILL, SIGSTOP, standard, RT (in order)
        let priority_order = [9u32, 19];
        for &sig in &priority_order {
            if deliverable.is_set(sig) {
                return self.deliver_signal(sig, ts);
            }
        }

        // Standard signals first
        for sig in 1..SIGRTMIN {
            if deliverable.is_set(sig) {
                return self.deliver_signal(sig, ts);
            }
        }

        // RT signals in order
        for sig in SIGRTMIN..=SIGRTMAX {
            if deliverable.is_set(sig) {
                return self.deliver_signal(sig, ts);
            }
        }

        None
    }

    fn deliver_signal(&mut self, signum: u32, ts: u64) -> Option<QueuedSignal> {
        if let Some(pos) = self.queue.iter().position(|s| s.signum == signum && !s.delivered) {
            self.queue[pos].deliver(ts);
            self.signals_delivered += 1;
            let sig = self.queue.remove(pos);

            // Clear pending for standard signals (RT may have more queued)
            if signum < SIGRTMIN || !self.queue.iter().any(|s| s.signum == signum && !s.delivered) {
                self.pending.clear(signum);
            }

            Some(sig)
        } else { None }
    }

    #[inline]
    pub fn set_handler(&mut self, signum: u32, action: SigAction, handler: u64) {
        if signum == 9 || signum == 19 { return; } // Can't change SIGKILL/SIGSTOP
        if let Some(disp) = self.dispositions.get_mut(&signum) {
            disp.action = action;
            disp.handler_addr = handler;
        }
    }
}

/// Signal dispatch stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SignalDispatchStats {
    pub processes_tracked: usize,
    pub total_signals_received: u64,
    pub total_signals_delivered: u64,
    pub total_signals_ignored: u64,
    pub total_signals_coalesced: u64,
    pub total_pending: u64,
    pub avg_delivery_latency_ns: u64,
    pub rt_signals_queued: u64,
}

/// Holistic signal dispatch manager
pub struct HolisticSignalDispatch {
    processes: BTreeMap<u64, ProcessSignalState>,
    stats: SignalDispatchStats,
}

impl HolisticSignalDispatch {
    pub fn new() -> Self {
        Self { processes: BTreeMap::new(), stats: SignalDispatchStats::default() }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.insert(pid, ProcessSignalState::new(pid));
    }

    #[inline]
    pub fn send_signal(&mut self, target_pid: u64, signum: u32, sender: u64, ts: u64) -> bool {
        if let Some(p) = self.processes.get_mut(&target_pid) {
            p.send_signal(signum, sender, ts)
        } else { false }
    }

    #[inline]
    pub fn dequeue(&mut self, pid: u64, ts: u64) -> Option<QueuedSignal> {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.dequeue_signal(ts)
        } else { None }
    }

    #[inline(always)]
    pub fn set_blocked(&mut self, pid: u64, mask: SigMask) {
        if let Some(p) = self.processes.get_mut(&pid) { p.blocked = mask; }
    }

    #[inline(always)]
    pub fn set_handler(&mut self, pid: u64, signum: u32, action: SigAction, handler: u64) {
        if let Some(p) = self.processes.get_mut(&pid) { p.set_handler(signum, action, handler); }
    }

    #[inline]
    pub fn broadcast_signal(&mut self, signum: u32, sender: u64, ts: u64) {
        let pids: Vec<u64> = self.processes.keys().copied().collect();
        for pid in pids {
            if let Some(p) = self.processes.get_mut(&pid) {
                p.send_signal(signum, sender, ts);
            }
        }
    }

    pub fn recompute(&mut self) {
        self.stats.processes_tracked = self.processes.len();
        self.stats.total_signals_received = self.processes.values().map(|p| p.signals_received).sum();
        self.stats.total_signals_delivered = self.processes.values().map(|p| p.signals_delivered).sum();
        self.stats.total_signals_ignored = self.processes.values().map(|p| p.signals_ignored).sum();
        self.stats.total_signals_coalesced = self.processes.values().map(|p| p.signals_coalesced).sum();
        self.stats.total_pending = self.processes.values().map(|p| p.pending.count() as u64).sum();
        self.stats.rt_signals_queued = self.processes.values().map(|p| {
            p.queue.iter().filter(|s| s.class == SignalClass::RealTime && !s.delivered).count() as u64
        }).sum();
        let all_lats: Vec<u64> = self.processes.values().flat_map(|p| {
            p.queue.iter().filter(|s| s.delivered).map(|s| s.latency_ns())
        }).collect();
        self.stats.avg_delivery_latency_ns = if all_lats.is_empty() { 0 } else { all_lats.iter().sum::<u64>() / all_lats.len() as u64 };
    }

    #[inline(always)]
    pub fn process(&self, pid: u64) -> Option<&ProcessSignalState> { self.processes.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &SignalDispatchStats { &self.stats }
}
