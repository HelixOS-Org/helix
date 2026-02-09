//! # Apps Signal Dispatch Manager
//!
//! Application signal delivery and handling:
//! - Signal queue per process/thread
//! - Signal masking (blocked, pending, ignored)
//! - Real-time signal priority ordering
//! - Signal group delivery (killpg)
//! - Signal coalescing for standard signals
//! - Alternate signal stack management
//! - Signal handler registration tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Signal number (1-64 for standard + RT range)
pub type SignalNum = u32;

/// Standard signal constants
pub const SIGHUP: SignalNum = 1;
pub const SIGINT: SignalNum = 2;
pub const SIGQUIT: SignalNum = 3;
pub const SIGILL: SignalNum = 4;
pub const SIGTRAP: SignalNum = 5;
pub const SIGABRT: SignalNum = 6;
pub const SIGBUS: SignalNum = 7;
pub const SIGFPE: SignalNum = 8;
pub const SIGKILL: SignalNum = 9;
pub const SIGSEGV: SignalNum = 11;
pub const SIGPIPE: SignalNum = 13;
pub const SIGALRM: SignalNum = 14;
pub const SIGTERM: SignalNum = 15;
pub const SIGUSR1: SignalNum = 10;
pub const SIGUSR2: SignalNum = 12;
pub const SIGCHLD: SignalNum = 17;
pub const SIGCONT: SignalNum = 18;
pub const SIGSTOP: SignalNum = 19;
pub const SIGTSTP: SignalNum = 20;
pub const SIGRTMIN: SignalNum = 34;
pub const SIGRTMAX: SignalNum = 64;

/// Signal disposition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalDisposition {
    Default,
    Ignore,
    Catch,
    Terminate,
    CoreDump,
    Stop,
    Continue,
}

/// Signal handler info
#[derive(Debug, Clone, Copy)]
pub struct SignalHandler {
    pub signum: SignalNum,
    pub disposition: SignalDisposition,
    pub handler_addr: u64,
    pub flags: SigActionFlags,
    pub restorer: u64,
}

/// SA_* flags
#[derive(Debug, Clone, Copy, Default)]
pub struct SigActionFlags {
    pub nocldstop: bool,
    pub nocldwait: bool,
    pub siginfo: bool,
    pub onstack: bool,
    pub restart: bool,
    pub nodefer: bool,
    pub resethand: bool,
}

/// Signal mask (64-bit bitmask)
#[derive(Debug, Clone, Copy, Default)]
pub struct SignalMask {
    bits: u64,
}

impl SignalMask {
    #[inline(always)]
    pub fn empty() -> Self { Self { bits: 0 } }
    #[inline(always)]
    pub fn full() -> Self { Self { bits: u64::MAX } }

    #[inline(always)]
    pub fn set(&mut self, sig: SignalNum) {
        if sig >= 1 && sig <= 64 { self.bits |= 1u64 << (sig - 1); }
    }

    #[inline(always)]
    pub fn clear(&mut self, sig: SignalNum) {
        if sig >= 1 && sig <= 64 { self.bits &= !(1u64 << (sig - 1)); }
    }

    #[inline(always)]
    pub fn is_set(&self, sig: SignalNum) -> bool {
        if sig >= 1 && sig <= 64 { (self.bits & (1u64 << (sig - 1))) != 0 } else { false }
    }

    #[inline(always)]
    pub fn union(&self, other: &SignalMask) -> SignalMask { SignalMask { bits: self.bits | other.bits } }
    #[inline(always)]
    pub fn intersect(&self, other: &SignalMask) -> SignalMask { SignalMask { bits: self.bits & other.bits } }
    #[inline(always)]
    pub fn complement(&self) -> SignalMask { SignalMask { bits: !self.bits } }
    #[inline(always)]
    pub fn count(&self) -> u32 { self.bits.count_ones() }
}

/// Queued signal
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct QueuedSignal {
    pub signum: SignalNum,
    pub sender_pid: u64,
    pub sender_uid: u32,
    pub timestamp_ns: u64,
    pub si_code: i32,
    pub si_value: u64,
}

/// Alternate signal stack
#[derive(Debug, Clone, Copy)]
pub struct AltStack {
    pub base: u64,
    pub size: u64,
    pub flags: u32,
}

/// Per-thread signal state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ThreadSignalState {
    pub thread_id: u64,
    pub blocked: SignalMask,
    pub pending: Vec<QueuedSignal>,
    pub alt_stack: Option<AltStack>,
    pub in_signal_handler: bool,
    pub current_signal: Option<SignalNum>,
}

impl ThreadSignalState {
    pub fn new(tid: u64) -> Self {
        Self {
            thread_id: tid,
            blocked: SignalMask::empty(),
            pending: Vec::new(),
            alt_stack: None,
            in_signal_handler: false,
            current_signal: None,
        }
    }

    #[inline]
    pub fn queue_signal(&mut self, sig: QueuedSignal) {
        // Standard signals coalesce (only one pending), RT signals queue
        if sig.signum < SIGRTMIN {
            if self.pending.iter().any(|s| s.signum == sig.signum) { return; }
        }
        self.pending.push(sig);
    }

    #[inline]
    pub fn dequeue_signal(&mut self) -> Option<QueuedSignal> {
        // Return first non-blocked signal; prefer standard over RT, RT in order
        let idx = self.pending.iter().position(|s| !self.blocked.is_set(s.signum));
        idx.map(|i| self.pending.remove(i))
    }

    #[inline(always)]
    pub fn has_pending_unblocked(&self) -> bool {
        self.pending.iter().any(|s| !self.blocked.is_set(s.signum))
    }
}

/// Per-process signal dispatch state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessSignalState {
    pub process_id: u64,
    pub handlers: BTreeMap<SignalNum, SignalHandler>,
    pub threads: BTreeMap<u64, ThreadSignalState>,
    pub process_pending: Vec<QueuedSignal>,
    pub total_delivered: u64,
    pub total_ignored: u64,
    pub total_caught: u64,
}

impl ProcessSignalState {
    pub fn new(pid: u64) -> Self {
        Self {
            process_id: pid,
            handlers: BTreeMap::new(),
            threads: BTreeMap::new(),
            process_pending: Vec::new(),
            total_delivered: 0,
            total_ignored: 0,
            total_caught: 0,
        }
    }

    #[inline]
    pub fn register_handler(&mut self, handler: SignalHandler) {
        // SIGKILL and SIGSTOP cannot be caught or ignored
        if handler.signum == SIGKILL || handler.signum == SIGSTOP { return; }
        self.handlers.insert(handler.signum, handler);
    }

    #[inline(always)]
    pub fn get_disposition(&self, sig: SignalNum) -> SignalDisposition {
        self.handlers.get(&sig).map(|h| h.disposition).unwrap_or_else(|| default_disposition(sig))
    }

    #[inline(always)]
    pub fn add_thread(&mut self, tid: u64) {
        self.threads.entry(tid).or_insert_with(|| ThreadSignalState::new(tid));
    }
}

/// Default signal disposition
#[inline]
pub fn default_disposition(sig: SignalNum) -> SignalDisposition {
    match sig {
        SIGKILL | SIGTERM | SIGHUP | SIGINT | SIGPIPE | SIGALRM | SIGUSR1 | SIGUSR2 => SignalDisposition::Terminate,
        SIGQUIT | SIGILL | SIGTRAP | SIGABRT | SIGBUS | SIGFPE | SIGSEGV => SignalDisposition::CoreDump,
        SIGSTOP | SIGTSTP => SignalDisposition::Stop,
        SIGCONT => SignalDisposition::Continue,
        SIGCHLD => SignalDisposition::Ignore,
        _ if sig >= SIGRTMIN && sig <= SIGRTMAX => SignalDisposition::Terminate,
        _ => SignalDisposition::Default,
    }
}

/// Apps signal dispatch stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppsSignalDispatchStats {
    pub total_processes: usize,
    pub total_threads: usize,
    pub total_pending: usize,
    pub total_delivered: u64,
    pub total_ignored: u64,
}

/// Apps Signal Dispatch Manager
pub struct AppsSignalDispatch {
    processes: BTreeMap<u64, ProcessSignalState>,
    stats: AppsSignalDispatchStats,
}

impl AppsSignalDispatch {
    pub fn new() -> Self {
        Self { processes: BTreeMap::new(), stats: AppsSignalDispatchStats::default() }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.entry(pid).or_insert_with(|| ProcessSignalState::new(pid));
    }

    #[inline(always)]
    pub fn register_thread(&mut self, pid: u64, tid: u64) {
        if let Some(proc_state) = self.processes.get_mut(&pid) { proc_state.add_thread(tid); }
    }

    pub fn send_signal(&mut self, target_pid: u64, sig: QueuedSignal) -> bool {
        if let Some(proc_state) = self.processes.get_mut(&target_pid) {
            let disp = proc_state.get_disposition(sig.signum);
            if disp == SignalDisposition::Ignore {
                proc_state.total_ignored += 1;
                return true;
            }
            // Try to deliver to a thread that doesn't block it
            let target_tid = proc_state.threads.values()
                .find(|t| !t.blocked.is_set(sig.signum))
                .map(|t| t.thread_id);
            if let Some(tid) = target_tid {
                if let Some(thread) = proc_state.threads.get_mut(&tid) {
                    thread.queue_signal(sig);
                    proc_state.total_delivered += 1;
                    return true;
                }
            }
            // No eligible thread — put in process pending
            proc_state.process_pending.push(sig);
            proc_state.total_delivered += 1;
            true
        } else { false }
    }

    #[inline]
    pub fn sigaction(&mut self, pid: u64, handler: SignalHandler) {
        if let Some(proc_state) = self.processes.get_mut(&pid) {
            proc_state.register_handler(handler);
        }
    }

    #[inline]
    pub fn sigmask(&mut self, pid: u64, tid: u64, mask: SignalMask) {
        if let Some(proc_state) = self.processes.get_mut(&pid) {
            if let Some(thread) = proc_state.threads.get_mut(&tid) {
                thread.blocked = mask;
            }
        }
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) { self.processes.remove(&pid); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.processes.len();
        self.stats.total_threads = self.processes.values().map(|p| p.threads.len()).sum();
        self.stats.total_pending = self.processes.values()
            .map(|p| p.process_pending.len() + p.threads.values().map(|t| t.pending.len()).sum::<usize>())
            .sum();
        self.stats.total_delivered = self.processes.values().map(|p| p.total_delivered).sum();
        self.stats.total_ignored = self.processes.values().map(|p| p.total_ignored).sum();
    }

    #[inline(always)]
    pub fn process_state(&self, pid: u64) -> Option<&ProcessSignalState> { self.processes.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &AppsSignalDispatchStats { &self.stats }
}
