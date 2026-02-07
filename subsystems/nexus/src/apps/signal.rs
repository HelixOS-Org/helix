//! # Application Signal Behavior Analysis
//!
//! Understanding and optimizing signal handling patterns:
//! - Signal delivery optimization
//! - Signal coalescing
//! - Signal handler profiling
//! - Signal-driven architecture detection
//! - Pending signal management
//! - Signal priority classification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SIGNAL TYPES
// ============================================================================

/// Signal category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignalCategory {
    /// Terminal signals (SIGTERM, SIGKILL, etc.)
    Termination,
    /// Stop/Continue (SIGSTOP, SIGCONT)
    JobControl,
    /// Error signals (SIGSEGV, SIGFPE, etc.)
    Error,
    /// I/O signals (SIGIO, SIGPOLL)
    Io,
    /// Timer signals (SIGALRM, SIGVTALRM)
    Timer,
    /// User-defined (SIGUSR1, SIGUSR2)
    User,
    /// Child signals (SIGCHLD)
    Child,
    /// Real-time signals
    Realtime,
}

/// Signal handling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalHandlerMode {
    /// Default handler
    Default,
    /// Ignored
    Ignored,
    /// Custom handler installed
    Custom,
    /// Blocked (masked)
    Blocked,
}

/// Signal handler info
#[derive(Debug, Clone)]
pub struct SignalHandlerInfo {
    /// Signal number
    pub signal: u32,
    /// Category
    pub category: SignalCategory,
    /// Handler mode
    pub mode: SignalHandlerMode,
    /// Handler address (if custom)
    pub handler_addr: u64,
    /// SA_RESTART flag
    pub sa_restart: bool,
    /// SA_NODEFER flag
    pub sa_nodefer: bool,
    /// SA_SIGINFO flag
    pub sa_siginfo: bool,
    /// Delivery count
    pub delivery_count: u64,
    /// Average handler time (nanoseconds)
    pub avg_handler_time_ns: u64,
    /// Max handler time
    pub max_handler_time_ns: u64,
}

// ============================================================================
// SIGNAL PATTERNS
// ============================================================================

/// Signal-driven architecture pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalArchPattern {
    /// App uses signals minimally
    Minimal,
    /// Signal-driven I/O (SIGIO)
    SignalDrivenIo,
    /// Timer-based (SIGALRM heartbeats)
    TimerBased,
    /// Real-time signal queuing
    RealtimeQueue,
    /// Parent-child coordination (SIGCHLD)
    ParentChild,
    /// Inter-process signaling (SIGUSR)
    InterProcess,
}

/// Signal delivery preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryPreference {
    /// Deliver immediately (interrupt current syscall)
    Immediate,
    /// Deliver at next safe point
    SafePoint,
    /// Coalesce with pending signals
    Coalesced,
    /// Batch deliver (for non-critical)
    Batched,
}

// ============================================================================
// SIGNAL STATISTICS
// ============================================================================

/// Per-signal statistics
#[derive(Debug, Clone, Default)]
pub struct SignalStats {
    /// Total sent
    pub sent: u64,
    /// Total delivered
    pub delivered: u64,
    /// Total blocked (queued)
    pub blocked: u64,
    /// Total ignored
    pub ignored: u64,
    /// Total coalesced
    pub coalesced: u64,
    /// Delivery latency sum (ns)
    pub latency_sum_ns: u64,
    /// Max delivery latency
    pub max_latency_ns: u64,
}

/// Per-process signal profile
#[derive(Debug, Clone)]
pub struct ProcessSignalProfile {
    /// Process ID
    pub pid: u64,
    /// Architecture pattern detected
    pub arch_pattern: SignalArchPattern,
    /// Handlers installed
    pub handlers: Vec<SignalHandlerInfo>,
    /// Signal mask (blocked signals)
    pub blocked_mask: u64,
    /// Per-signal stats
    pub signal_stats: BTreeMap<u32, SignalStats>,
    /// Total signals received
    pub total_received: u64,
    /// Is signal-heavy
    pub signal_heavy: bool,
    /// Recommended delivery mode
    pub delivery_preference: DeliveryPreference,
}

// ============================================================================
// SIGNAL COALESCING
// ============================================================================

/// Coalescing rule
#[derive(Debug, Clone)]
pub struct CoalescingRule {
    /// Signal number
    pub signal: u32,
    /// Maximum coalescing window (ms)
    pub window_ms: u64,
    /// Maximum signals to coalesce
    pub max_coalesce: u32,
    /// Enabled
    pub enabled: bool,
}

/// Pending coalesced signal
#[derive(Debug, Clone)]
pub struct CoalescedSignal {
    /// Signal number
    pub signal: u32,
    /// Count of coalesced signals
    pub count: u32,
    /// First signal timestamp
    pub first_at: u64,
    /// Last signal timestamp
    pub last_at: u64,
    /// Sender PIDs
    pub senders: Vec<u64>,
}

/// Signal coalescing engine
pub struct SignalCoalescer {
    /// Rules per signal
    rules: BTreeMap<u32, CoalescingRule>,
    /// Pending signals per process
    pending: BTreeMap<u64, BTreeMap<u32, CoalescedSignal>>,
    /// Total coalesced
    pub total_coalesced: u64,
}

impl SignalCoalescer {
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
            pending: BTreeMap::new(),
            total_coalesced: 0,
        }
    }

    /// Add coalescing rule
    pub fn add_rule(&mut self, rule: CoalescingRule) {
        self.rules.insert(rule.signal, rule);
    }

    /// Submit signal for potential coalescing
    pub fn submit(&mut self, pid: u64, signal: u32, sender: u64, timestamp: u64) -> bool {
        let rule = match self.rules.get(&signal) {
            Some(r) if r.enabled => r.clone(),
            _ => return false,
        };

        let process_pending = self.pending.entry(pid).or_insert_with(BTreeMap::new);

        if let Some(coalesced) = process_pending.get_mut(&signal) {
            // Check window
            if timestamp.saturating_sub(coalesced.first_at) <= rule.window_ms
                && coalesced.count < rule.max_coalesce
            {
                coalesced.count += 1;
                coalesced.last_at = timestamp;
                if !coalesced.senders.contains(&sender) && coalesced.senders.len() < 16 {
                    coalesced.senders.push(sender);
                }
                self.total_coalesced += 1;
                return true;
            }
        }

        // Start new coalesce group
        process_pending.insert(signal, CoalescedSignal {
            signal,
            count: 1,
            first_at: timestamp,
            last_at: timestamp,
            senders: alloc::vec![sender],
        });

        false
    }

    /// Flush ready signals
    pub fn flush(&mut self, pid: u64, current_time: u64) -> Vec<CoalescedSignal> {
        let mut ready = Vec::new();

        if let Some(process_pending) = self.pending.get_mut(&pid) {
            let expired_keys: Vec<u32> = process_pending
                .iter()
                .filter(|(sig, cs)| {
                    let rule = self.rules.get(sig);
                    match rule {
                        Some(r) => current_time.saturating_sub(cs.first_at) > r.window_ms,
                        None => true,
                    }
                })
                .map(|(&sig, _)| sig)
                .collect();

            for key in expired_keys {
                if let Some(cs) = process_pending.remove(&key) {
                    ready.push(cs);
                }
            }
        }

        ready
    }
}

// ============================================================================
// SIGNAL ANALYZER
// ============================================================================

/// Application signal analyzer
pub struct AppSignalAnalyzer {
    /// Per-process profiles
    profiles: BTreeMap<u64, ProcessSignalProfile>,
    /// Coalescer
    coalescer: SignalCoalescer,
    /// Total signals tracked
    pub total_signals: u64,
}

impl AppSignalAnalyzer {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            coalescer: SignalCoalescer::new(),
            total_signals: 0,
        }
    }

    /// Register process
    pub fn register_process(&mut self, pid: u64) {
        self.profiles.insert(pid, ProcessSignalProfile {
            pid,
            arch_pattern: SignalArchPattern::Minimal,
            handlers: Vec::new(),
            blocked_mask: 0,
            signal_stats: BTreeMap::new(),
            total_received: 0,
            signal_heavy: false,
            delivery_preference: DeliveryPreference::Immediate,
        });
    }

    /// Record signal handler installation
    pub fn record_handler(&mut self, pid: u64, info: SignalHandlerInfo) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            // Update or add handler
            if let Some(existing) = profile
                .handlers
                .iter_mut()
                .find(|h| h.signal == info.signal)
            {
                *existing = info;
            } else {
                profile.handlers.push(info);
            }
        }
    }

    /// Record signal delivery
    pub fn record_delivery(
        &mut self,
        pid: u64,
        signal: u32,
        latency_ns: u64,
        handler_time_ns: u64,
    ) {
        self.total_signals += 1;

        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.total_received += 1;

            let stats = profile.signal_stats.entry(signal).or_default();
            stats.delivered += 1;
            stats.latency_sum_ns += latency_ns;
            if latency_ns > stats.max_latency_ns {
                stats.max_latency_ns = latency_ns;
            }

            // Update handler stats
            if let Some(handler) = profile.handlers.iter_mut().find(|h| h.signal == signal) {
                handler.delivery_count += 1;
                handler.avg_handler_time_ns =
                    (handler.avg_handler_time_ns * 7 + handler_time_ns) / 8;
                if handler_time_ns > handler.max_handler_time_ns {
                    handler.max_handler_time_ns = handler_time_ns;
                }
            }
        }
    }

    /// Detect architecture pattern
    pub fn detect_pattern(&mut self, pid: u64) -> Option<SignalArchPattern> {
        let profile = self.profiles.get_mut(&pid)?;

        let has_sigio = profile
            .handlers
            .iter()
            .any(|h| h.category == SignalCategory::Io);
        let has_sigalrm = profile
            .handlers
            .iter()
            .any(|h| h.category == SignalCategory::Timer);
        let has_sigchld = profile
            .handlers
            .iter()
            .any(|h| h.category == SignalCategory::Child);
        let has_sigusr = profile
            .handlers
            .iter()
            .any(|h| h.category == SignalCategory::User);
        let has_rt = profile
            .handlers
            .iter()
            .any(|h| h.category == SignalCategory::Realtime);

        let pattern = if has_rt {
            SignalArchPattern::RealtimeQueue
        } else if has_sigio {
            SignalArchPattern::SignalDrivenIo
        } else if has_sigalrm
            && profile
                .signal_stats
                .values()
                .filter(|s| s.delivered > 10)
                .count()
                > 0
        {
            SignalArchPattern::TimerBased
        } else if has_sigchld {
            SignalArchPattern::ParentChild
        } else if has_sigusr {
            SignalArchPattern::InterProcess
        } else {
            SignalArchPattern::Minimal
        };

        profile.arch_pattern = pattern;
        profile.signal_heavy = profile.total_received > 1000;

        profile.delivery_preference = match pattern {
            SignalArchPattern::RealtimeQueue => DeliveryPreference::Immediate,
            SignalArchPattern::SignalDrivenIo => DeliveryPreference::Immediate,
            SignalArchPattern::TimerBased => DeliveryPreference::SafePoint,
            SignalArchPattern::ParentChild => DeliveryPreference::Coalesced,
            SignalArchPattern::InterProcess => DeliveryPreference::SafePoint,
            SignalArchPattern::Minimal => DeliveryPreference::Batched,
        };

        Some(pattern)
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessSignalProfile> {
        self.profiles.get(&pid)
    }

    /// Unregister process
    pub fn unregister_process(&mut self, pid: u64) {
        self.profiles.remove(&pid);
    }
}
