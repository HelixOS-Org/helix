//! # Bridge Flow Control
//!
//! Flow control for syscall traffic management:
//! - Credit-based flow control
//! - Sliding window admission control
//! - Per-process flow state tracking
//! - Congestion avoidance
//! - Fair queuing across processes

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// FLOW TYPES
// ============================================================================

/// Flow state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowState {
    /// Open — accepting requests
    Open,
    /// Throttled — reduced rate
    Throttled,
    /// Paused — no new requests
    Paused,
    /// Congested — back-pressure active
    Congested,
    /// Closed — terminating
    Closed,
}

/// Congestion signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionSignal {
    /// No congestion
    None,
    /// Early congestion (ECN-like)
    EarlyWarning,
    /// Queue building up
    QueueGrowing,
    /// Queue full
    QueueFull,
    /// Latency spike
    LatencySpike,
}

/// Flow priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FlowPriority {
    /// Background
    Background = 0,
    /// Best effort
    BestEffort = 1,
    /// Normal
    Normal     = 2,
    /// High priority
    High       = 3,
    /// Critical
    Critical   = 4,
}

// ============================================================================
// CREDIT-BASED CONTROL
// ============================================================================

/// Credit bucket for a process
#[derive(Debug, Clone)]
pub struct CreditBucket {
    /// Current credits
    pub credits: u64,
    /// Max credits
    pub max_credits: u64,
    /// Refill rate (credits per second)
    pub refill_rate: u64,
    /// Last refill timestamp
    pub last_refill_ns: u64,
    /// Credits consumed total
    pub total_consumed: u64,
    /// Credits denied
    pub total_denied: u64,
}

impl CreditBucket {
    pub fn new(max_credits: u64, refill_rate: u64, now: u64) -> Self {
        Self {
            credits: max_credits,
            max_credits,
            refill_rate,
            last_refill_ns: now,
            total_consumed: 0,
            total_denied: 0,
        }
    }

    /// Refill credits based on elapsed time
    #[inline]
    pub fn refill(&mut self, now: u64) {
        let elapsed_ns = now.saturating_sub(self.last_refill_ns);
        let elapsed_secs = elapsed_ns as f64 / 1_000_000_000.0;
        let new_credits = (elapsed_secs * self.refill_rate as f64) as u64;
        if new_credits > 0 {
            self.credits = (self.credits + new_credits).min(self.max_credits);
            self.last_refill_ns = now;
        }
    }

    /// Try to consume credits
    #[inline]
    pub fn try_consume(&mut self, amount: u64, now: u64) -> bool {
        self.refill(now);
        if self.credits >= amount {
            self.credits -= amount;
            self.total_consumed += amount;
            true
        } else {
            self.total_denied += amount;
            false
        }
    }

    /// Fill ratio (0.0-1.0)
    #[inline]
    pub fn fill_ratio(&self) -> f64 {
        if self.max_credits == 0 {
            return 0.0;
        }
        self.credits as f64 / self.max_credits as f64
    }
}

// ============================================================================
// SLIDING WINDOW
// ============================================================================

/// Sliding window for admission control
#[derive(Debug, Clone)]
pub struct AdmissionWindow {
    /// Window size (max concurrent)
    pub window_size: u32,
    /// Current in-flight
    pub in_flight: u32,
    /// Total admitted
    pub total_admitted: u64,
    /// Total rejected
    pub total_rejected: u64,
    /// Slow start threshold
    pub ssthresh: u32,
    /// In slow start phase
    pub slow_start: bool,
}

impl AdmissionWindow {
    pub fn new(initial_window: u32) -> Self {
        Self {
            window_size: initial_window,
            in_flight: 0,
            total_admitted: 0,
            total_rejected: 0,
            ssthresh: initial_window * 2,
            slow_start: true,
        }
    }

    /// Try to admit
    #[inline]
    pub fn try_admit(&mut self) -> bool {
        if self.in_flight < self.window_size {
            self.in_flight += 1;
            self.total_admitted += 1;
            true
        } else {
            self.total_rejected += 1;
            false
        }
    }

    /// Release (completion)
    #[inline]
    pub fn release(&mut self) {
        if self.in_flight > 0 {
            self.in_flight -= 1;
        }
        // Additive increase
        if self.slow_start && self.window_size < self.ssthresh {
            self.window_size += 1; // exponential growth during slow start
        }
    }

    /// Signal congestion (multiplicative decrease)
    #[inline]
    pub fn on_congestion(&mut self) {
        self.ssthresh = (self.window_size / 2).max(1);
        self.window_size = self.ssthresh;
        self.slow_start = false;
    }

    /// Utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.window_size == 0 {
            return 0.0;
        }
        self.in_flight as f64 / self.window_size as f64
    }
}

// ============================================================================
// PER-PROCESS FLOW
// ============================================================================

/// Per-process flow state
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessFlow {
    /// Process ID
    pub pid: u64,
    /// Priority
    pub priority: FlowPriority,
    /// State
    pub state: FlowState,
    /// Credit bucket
    pub credits: CreditBucket,
    /// Admission window
    pub window: AdmissionWindow,
    /// Queued requests
    pub queued: u32,
    /// Max queue depth
    pub max_queue: u32,
    /// Latency EMA (ns)
    pub latency_ema_ns: f64,
    /// Congestion signals received
    pub congestion_signals: u64,
}

impl ProcessFlow {
    pub fn new(pid: u64, priority: FlowPriority, now: u64) -> Self {
        let (max_credits, refill, window) = match priority {
            FlowPriority::Background => (50, 10, 4),
            FlowPriority::BestEffort => (100, 20, 8),
            FlowPriority::Normal => (200, 50, 16),
            FlowPriority::High => (500, 100, 32),
            FlowPriority::Critical => (1000, 200, 64),
        };
        Self {
            pid,
            priority,
            state: FlowState::Open,
            credits: CreditBucket::new(max_credits, refill, now),
            window: AdmissionWindow::new(window),
            queued: 0,
            max_queue: window * 4,
            latency_ema_ns: 0.0,
            congestion_signals: 0,
        }
    }

    /// Try to submit a request
    pub fn try_submit(&mut self, cost: u64, now: u64) -> bool {
        if self.state == FlowState::Closed || self.state == FlowState::Paused {
            return false;
        }
        if !self.credits.try_consume(cost, now) {
            return false;
        }
        if !self.window.try_admit() {
            return false;
        }
        true
    }

    /// Complete a request
    #[inline]
    pub fn complete(&mut self, latency_ns: u64) {
        self.window.release();
        // EMA update
        self.latency_ema_ns = 0.9 * self.latency_ema_ns + 0.1 * latency_ns as f64;
    }

    /// Handle congestion
    pub fn on_congestion(&mut self, signal: CongestionSignal) {
        self.congestion_signals += 1;
        match signal {
            CongestionSignal::None => {},
            CongestionSignal::EarlyWarning => {
                // Reduce slightly
                self.credits.max_credits = (self.credits.max_credits * 9 / 10).max(10);
            },
            CongestionSignal::QueueGrowing => {
                self.state = FlowState::Throttled;
                self.window.on_congestion();
            },
            CongestionSignal::QueueFull | CongestionSignal::LatencySpike => {
                self.state = FlowState::Congested;
                self.window.on_congestion();
            },
        }
    }

    /// Recover from congestion
    #[inline]
    pub fn recover(&mut self) {
        if self.state == FlowState::Throttled || self.state == FlowState::Congested {
            self.state = FlowState::Open;
        }
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Flow control stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeFlowStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Total submissions
    pub total_submissions: u64,
    /// Total rejections
    pub total_rejections: u64,
    /// Congestion events
    pub congestion_events: u64,
    /// Processes throttled
    pub throttled_count: usize,
    /// Global avg latency (ns)
    pub avg_latency_ns: f64,
}

/// Bridge flow controller
#[repr(align(64))]
pub struct BridgeFlowController {
    /// Per-process flows
    flows: BTreeMap<u64, ProcessFlow>,
    /// Global congestion state
    global_congestion: CongestionSignal,
    /// Global queue depth
    global_queue: u64,
    /// Global queue limit
    global_queue_limit: u64,
    /// Stats
    stats: BridgeFlowStats,
}

impl BridgeFlowController {
    pub fn new(global_queue_limit: u64) -> Self {
        Self {
            flows: BTreeMap::new(),
            global_congestion: CongestionSignal::None,
            global_queue: 0,
            global_queue_limit,
            stats: BridgeFlowStats::default(),
        }
    }

    /// Register process
    #[inline(always)]
    pub fn register(&mut self, pid: u64, priority: FlowPriority, now: u64) {
        self.flows.insert(pid, ProcessFlow::new(pid, priority, now));
        self.update_stats();
    }

    /// Submit request
    pub fn submit(&mut self, pid: u64, cost: u64, now: u64) -> bool {
        self.stats.total_submissions += 1;

        // Check global congestion
        if self.global_queue >= self.global_queue_limit {
            self.global_congestion = CongestionSignal::QueueFull;
            self.stats.total_rejections += 1;
            return false;
        }

        let result = if let Some(flow) = self.flows.get_mut(&pid) {
            flow.try_submit(cost, now)
        } else {
            false
        };

        if result {
            self.global_queue += 1;
            self.detect_congestion();
        } else {
            self.stats.total_rejections += 1;
        }

        result
    }

    /// Complete request
    #[inline]
    pub fn complete(&mut self, pid: u64, latency_ns: u64) {
        if let Some(flow) = self.flows.get_mut(&pid) {
            flow.complete(latency_ns);
        }
        self.global_queue = self.global_queue.saturating_sub(1);
        self.detect_congestion();
        self.update_stats();
    }

    /// Detect congestion
    fn detect_congestion(&mut self) {
        let ratio = self.global_queue as f64 / self.global_queue_limit as f64;
        let signal = if ratio >= 1.0 {
            CongestionSignal::QueueFull
        } else if ratio >= 0.8 {
            CongestionSignal::QueueGrowing
        } else if ratio >= 0.5 {
            CongestionSignal::EarlyWarning
        } else {
            CongestionSignal::None
        };

        if signal != self.global_congestion {
            self.global_congestion = signal;
            if signal != CongestionSignal::None {
                self.stats.congestion_events += 1;
                // Propagate to processes
                let pids: Vec<u64> = self.flows.keys().copied().collect();
                for pid in pids {
                    if let Some(flow) = self.flows.get_mut(&pid) {
                        flow.on_congestion(signal);
                    }
                }
            } else {
                let pids: Vec<u64> = self.flows.keys().copied().collect();
                for pid in pids {
                    if let Some(flow) = self.flows.get_mut(&pid) {
                        flow.recover();
                    }
                }
            }
        }
    }

    /// Unregister process
    #[inline(always)]
    pub fn unregister(&mut self, pid: u64) {
        self.flows.remove(&pid);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.flows.len();
        self.stats.throttled_count = self
            .flows
            .values()
            .filter(|f| f.state == FlowState::Throttled || f.state == FlowState::Congested)
            .count();
        let total_latency: f64 = self.flows.values().map(|f| f.latency_ema_ns).sum();
        self.stats.avg_latency_ns = if self.flows.is_empty() {
            0.0
        } else {
            total_latency / self.flows.len() as f64
        };
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &BridgeFlowStats {
        &self.stats
    }
}
