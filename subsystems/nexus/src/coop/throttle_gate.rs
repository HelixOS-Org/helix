// SPDX-License-Identifier: GPL-2.0
//! Coop throttle_gate — rate-limiting gate for cooperative resource access.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Throttle algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleAlgorithm {
    TokenBucket,
    SlidingWindow,
    FixedWindow,
    LeakyBucket,
    Adaptive,
}

/// Throttle gate state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateState {
    Open,
    Throttled,
    Closed,
    Bursting,
}

/// Token bucket state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TokenBucketState {
    pub tokens: f64,
    pub max_tokens: f64,
    pub refill_rate: f64,
    pub last_refill: u64,
}

impl TokenBucketState {
    pub fn new(max_tokens: f64, refill_rate: f64, now: u64) -> Self {
        Self { tokens: max_tokens, max_tokens, refill_rate, last_refill: now }
    }

    #[inline]
    pub fn refill(&mut self, now: u64) {
        let elapsed_ns = now.saturating_sub(self.last_refill);
        let elapsed_sec = elapsed_ns as f64 / 1_000_000_000.0;
        self.tokens += elapsed_sec * self.refill_rate;
        if self.tokens > self.max_tokens { self.tokens = self.max_tokens; }
        self.last_refill = now;
    }

    #[inline]
    pub fn try_consume(&mut self, n: f64, now: u64) -> bool {
        self.refill(now);
        if self.tokens >= n {
            self.tokens -= n;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn available(&self) -> f64 { self.tokens }
    #[inline(always)]
    pub fn utilization(&self) -> f64 { 1.0 - (self.tokens / self.max_tokens) }
}

/// Sliding window state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SlidingWindowState {
    pub window_ns: u64,
    pub max_requests: u64,
    pub timestamps: Vec<u64>,
}

impl SlidingWindowState {
    pub fn new(window_ns: u64, max_requests: u64) -> Self {
        Self { window_ns, max_requests, timestamps: Vec::new() }
    }

    #[inline]
    pub fn try_admit(&mut self, now: u64) -> bool {
        let cutoff = now.saturating_sub(self.window_ns);
        self.timestamps.retain(|&t| t >= cutoff);
        if self.timestamps.len() as u64 >= self.max_requests { return false; }
        self.timestamps.push(now);
        true
    }

    #[inline]
    pub fn current_rate(&self, now: u64) -> f64 {
        let cutoff = now.saturating_sub(self.window_ns);
        let count = self.timestamps.iter().filter(|&&t| t >= cutoff).count();
        let window_sec = self.window_ns as f64 / 1_000_000_000.0;
        if window_sec == 0.0 { return 0.0; }
        count as f64 / window_sec
    }
}

/// Throttle gate instance
#[derive(Debug)]
pub struct ThrottleGateInstance {
    pub id: u64,
    pub name_hash: u64,
    pub algorithm: ThrottleAlgorithm,
    pub state: GateState,
    pub token_bucket: Option<TokenBucketState>,
    pub sliding_window: Option<SlidingWindowState>,
    pub total_admitted: u64,
    pub total_throttled: u64,
    pub total_rejected: u64,
    pub burst_count: u64,
    pub created_at: u64,
    pub last_throttle: u64,
}

impl ThrottleGateInstance {
    #[inline]
    pub fn token_bucket(id: u64, max_tokens: f64, refill_rate: f64, now: u64) -> Self {
        Self {
            id, name_hash: id, algorithm: ThrottleAlgorithm::TokenBucket,
            state: GateState::Open,
            token_bucket: Some(TokenBucketState::new(max_tokens, refill_rate, now)),
            sliding_window: None,
            total_admitted: 0, total_throttled: 0, total_rejected: 0,
            burst_count: 0, created_at: now, last_throttle: 0,
        }
    }

    #[inline]
    pub fn sliding_window(id: u64, window_ns: u64, max_requests: u64, now: u64) -> Self {
        Self {
            id, name_hash: id, algorithm: ThrottleAlgorithm::SlidingWindow,
            state: GateState::Open,
            token_bucket: None,
            sliding_window: Some(SlidingWindowState::new(window_ns, max_requests)),
            total_admitted: 0, total_throttled: 0, total_rejected: 0,
            burst_count: 0, created_at: now, last_throttle: 0,
        }
    }

    pub fn try_pass(&mut self, cost: f64, now: u64) -> bool {
        if self.state == GateState::Closed { self.total_rejected += 1; return false; }

        let admitted = match self.algorithm {
            ThrottleAlgorithm::TokenBucket => {
                self.token_bucket.as_mut().map(|tb| tb.try_consume(cost, now)).unwrap_or(false)
            }
            ThrottleAlgorithm::SlidingWindow | ThrottleAlgorithm::FixedWindow => {
                self.sliding_window.as_mut().map(|sw| sw.try_admit(now)).unwrap_or(false)
            }
            _ => true,
        };

        if admitted {
            self.total_admitted += 1;
            self.state = GateState::Open;
            true
        } else {
            self.total_throttled += 1;
            self.state = GateState::Throttled;
            self.last_throttle = now;
            false
        }
    }

    #[inline]
    pub fn throttle_ratio(&self) -> f64 {
        let total = self.total_admitted + self.total_throttled;
        if total == 0 { return 0.0; }
        self.total_throttled as f64 / total as f64
    }

    #[inline(always)]
    pub fn close(&mut self) { self.state = GateState::Closed; }
    #[inline(always)]
    pub fn open(&mut self) { self.state = GateState::Open; }
}

/// Throttle gate stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ThrottleGateStats {
    pub total_gates: u32,
    pub total_admitted: u64,
    pub total_throttled: u64,
    pub total_rejected: u64,
    pub open_gates: u32,
    pub throttled_gates: u32,
}

/// Main throttle gate manager
pub struct CoopThrottleGate {
    gates: BTreeMap<u64, ThrottleGateInstance>,
    next_id: u64,
}

impl CoopThrottleGate {
    pub fn new() -> Self {
        Self { gates: BTreeMap::new(), next_id: 1 }
    }

    #[inline]
    pub fn create_token_bucket(&mut self, max_tokens: f64, refill_rate: f64, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.gates.insert(id, ThrottleGateInstance::token_bucket(id, max_tokens, refill_rate, now));
        id
    }

    #[inline]
    pub fn create_sliding_window(&mut self, window_ns: u64, max_requests: u64, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.gates.insert(id, ThrottleGateInstance::sliding_window(id, window_ns, max_requests, now));
        id
    }

    #[inline(always)]
    pub fn try_pass(&mut self, gate_id: u64, cost: f64, now: u64) -> bool {
        self.gates.get_mut(&gate_id).map(|g| g.try_pass(cost, now)).unwrap_or(false)
    }

    #[inline(always)]
    pub fn close_gate(&mut self, gate_id: u64) {
        if let Some(g) = self.gates.get_mut(&gate_id) { g.close(); }
    }

    #[inline(always)]
    pub fn open_gate(&mut self, gate_id: u64) {
        if let Some(g) = self.gates.get_mut(&gate_id) { g.open(); }
    }

    #[inline(always)]
    pub fn destroy_gate(&mut self, gate_id: u64) -> bool {
        self.gates.remove(&gate_id).is_some()
    }

    #[inline]
    pub fn most_throttled(&self, n: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<_> = self.gates.iter()
            .filter(|(_, g)| g.total_throttled > 0)
            .map(|(&id, g)| (id, g.throttle_ratio()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(n);
        v
    }

    #[inline]
    pub fn stats(&self) -> ThrottleGateStats {
        let open = self.gates.values().filter(|g| g.state == GateState::Open).count() as u32;
        let throttled = self.gates.values().filter(|g| g.state == GateState::Throttled).count() as u32;
        ThrottleGateStats {
            total_gates: self.gates.len() as u32,
            total_admitted: self.gates.values().map(|g| g.total_admitted).sum(),
            total_throttled: self.gates.values().map(|g| g.total_throttled).sum(),
            total_rejected: self.gates.values().map(|g| g.total_rejected).sum(),
            open_gates: open, throttled_gates: throttled,
        }
    }
}
