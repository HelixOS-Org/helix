//! # Coop Flow Control
//!
//! Distributed flow control for cooperative scheduling:
//! - Credit-based flow control between nodes
//! - Token bucket rate limiting per channel
//! - Congestion window management
//! - ECN-like backpressure signaling
//! - Fair bandwidth allocation
//! - Priority-based preemption

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Flow state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowState {
    Idle,
    Active,
    Throttled,
    Congested,
    Blocked,
}

/// Congestion signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionSignal {
    None,
    EarlyWarning,
    Moderate,
    Severe,
    Critical,
}

/// Credit counter for flow control
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CreditCounter {
    pub credits: i64,
    pub max_credits: i64,
    pub min_credits: i64,
    pub refill_rate: u64,
    pub last_refill_ts: u64,
    pub consumed_total: u64,
    pub refilled_total: u64,
}

impl CreditCounter {
    pub fn new(max: i64, refill_rate: u64) -> Self {
        Self { credits: max, max_credits: max, min_credits: 0, refill_rate, last_refill_ts: 0, consumed_total: 0, refilled_total: 0 }
    }

    #[inline(always)]
    pub fn consume(&mut self, amount: i64) -> bool {
        if self.credits >= amount { self.credits -= amount; self.consumed_total += amount as u64; true } else { false }
    }

    #[inline]
    pub fn refill(&mut self, now: u64) {
        let elapsed_ns = now.saturating_sub(self.last_refill_ts);
        let refill = (self.refill_rate as f64 * (elapsed_ns as f64 / 1_000_000_000.0)) as i64;
        if refill > 0 {
            self.credits = (self.credits + refill).min(self.max_credits);
            self.refilled_total += refill as u64;
            self.last_refill_ts = now;
        }
    }

    #[inline(always)]
    pub fn available_pct(&self) -> f64 {
        if self.max_credits == 0 { return 0.0; }
        (self.credits as f64 / self.max_credits as f64) * 100.0
    }
}

/// Congestion window
#[derive(Debug, Clone)]
pub struct CongestionWindow {
    pub cwnd: u32,
    pub ssthresh: u32,
    pub min_cwnd: u32,
    pub max_cwnd: u32,
    pub in_slow_start: bool,
    pub bytes_in_flight: u64,
    pub acked_bytes: u64,
    pub lost_bytes: u64,
    pub rtt_ns: u64,
    pub min_rtt_ns: u64,
    pub srtt_ns: u64,
}

impl CongestionWindow {
    pub fn new(initial: u32, max: u32) -> Self {
        Self {
            cwnd: initial, ssthresh: max / 2, min_cwnd: 1, max_cwnd: max,
            in_slow_start: true, bytes_in_flight: 0, acked_bytes: 0,
            lost_bytes: 0, rtt_ns: 0, min_rtt_ns: u64::MAX, srtt_ns: 0,
        }
    }

    #[inline]
    pub fn on_ack(&mut self, acked: u32) {
        self.acked_bytes += acked as u64;
        if self.in_slow_start {
            self.cwnd = (self.cwnd + acked).min(self.max_cwnd);
            if self.cwnd >= self.ssthresh { self.in_slow_start = false; }
        } else {
            // AIMD: additive increase
            let inc = ((acked as u64 * acked as u64) / self.cwnd as u64) as u32;
            self.cwnd = (self.cwnd + inc.max(1)).min(self.max_cwnd);
        }
    }

    #[inline]
    pub fn on_loss(&mut self) {
        self.ssthresh = (self.cwnd / 2).max(self.min_cwnd);
        self.cwnd = self.ssthresh;
        self.in_slow_start = false;
        self.lost_bytes += 1;
    }

    #[inline]
    pub fn update_rtt(&mut self, rtt_ns: u64) {
        self.rtt_ns = rtt_ns;
        if rtt_ns < self.min_rtt_ns { self.min_rtt_ns = rtt_ns; }
        // Smoothed RTT: SRTT = 7/8 * SRTT + 1/8 * RTT
        if self.srtt_ns == 0 { self.srtt_ns = rtt_ns; }
        else { self.srtt_ns = (self.srtt_ns * 7 + rtt_ns) / 8; }
    }

    #[inline(always)]
    pub fn can_send(&self, bytes: u64) -> bool {
        self.bytes_in_flight + bytes <= self.cwnd as u64
    }
}

/// Flow channel between two nodes
#[derive(Debug, Clone)]
pub struct FlowChannel {
    pub id: u64,
    pub src_node: u64,
    pub dst_node: u64,
    pub state: FlowState,
    pub credits: CreditCounter,
    pub cwnd: CongestionWindow,
    pub congestion: CongestionSignal,
    pub priority: u8,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub msgs_sent: u64,
    pub msgs_recv: u64,
    pub created_ts: u64,
    pub last_activity_ts: u64,
}

impl FlowChannel {
    pub fn new(id: u64, src: u64, dst: u64, max_credits: i64, ts: u64) -> Self {
        Self {
            id, src_node: src, dst_node: dst, state: FlowState::Idle,
            credits: CreditCounter::new(max_credits, max_credits as u64),
            cwnd: CongestionWindow::new(10, 1024), congestion: CongestionSignal::None,
            priority: 4, bytes_sent: 0, bytes_recv: 0, msgs_sent: 0, msgs_recv: 0,
            created_ts: ts, last_activity_ts: ts,
        }
    }

    #[inline]
    pub fn send(&mut self, bytes: u64, ts: u64) -> bool {
        if !self.credits.consume(bytes as i64) { self.state = FlowState::Throttled; return false; }
        if !self.cwnd.can_send(bytes) { self.state = FlowState::Congested; return false; }
        self.bytes_sent += bytes;
        self.msgs_sent += 1;
        self.cwnd.bytes_in_flight += bytes;
        self.last_activity_ts = ts;
        self.state = FlowState::Active;
        true
    }

    #[inline]
    pub fn ack(&mut self, bytes: u32, rtt_ns: u64) {
        self.cwnd.on_ack(bytes);
        self.cwnd.update_rtt(rtt_ns);
        self.cwnd.bytes_in_flight = self.cwnd.bytes_in_flight.saturating_sub(bytes as u64);
    }

    #[inline(always)]
    pub fn loss(&mut self) { self.cwnd.on_loss(); }

    #[inline]
    pub fn recv(&mut self, bytes: u64, ts: u64) {
        self.bytes_recv += bytes;
        self.msgs_recv += 1;
        self.last_activity_ts = ts;
    }

    #[inline(always)]
    pub fn tick(&mut self, now: u64) { self.credits.refill(now); }

    #[inline(always)]
    pub fn throughput_bps(&self, now: u64) -> f64 {
        let elapsed = now.saturating_sub(self.created_ts);
        if elapsed == 0 { 0.0 } else { self.bytes_sent as f64 / (elapsed as f64 / 1_000_000_000.0) }
    }
}

/// Flow control stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FlowControlStats {
    pub total_channels: usize,
    pub active_channels: usize,
    pub throttled_channels: usize,
    pub congested_channels: usize,
    pub total_bytes_sent: u64,
    pub total_bytes_recv: u64,
    pub total_messages: u64,
    pub avg_credit_pct: f64,
    pub avg_cwnd: f64,
}

/// Coop flow control manager
pub struct CoopFlowControl {
    channels: BTreeMap<u64, FlowChannel>,
    stats: FlowControlStats,
    next_id: u64,
}

impl CoopFlowControl {
    pub fn new() -> Self {
        Self { channels: BTreeMap::new(), stats: FlowControlStats::default(), next_id: 1 }
    }

    #[inline]
    pub fn create_channel(&mut self, src: u64, dst: u64, max_credits: i64, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.channels.insert(id, FlowChannel::new(id, src, dst, max_credits, ts));
        id
    }

    #[inline(always)]
    pub fn send(&mut self, ch_id: u64, bytes: u64, ts: u64) -> bool {
        if let Some(ch) = self.channels.get_mut(&ch_id) { ch.send(bytes, ts) } else { false }
    }

    #[inline(always)]
    pub fn ack(&mut self, ch_id: u64, bytes: u32, rtt_ns: u64) {
        if let Some(ch) = self.channels.get_mut(&ch_id) { ch.ack(bytes, rtt_ns); }
    }

    #[inline(always)]
    pub fn recv(&mut self, ch_id: u64, bytes: u64, ts: u64) {
        if let Some(ch) = self.channels.get_mut(&ch_id) { ch.recv(bytes, ts); }
    }

    #[inline(always)]
    pub fn tick(&mut self, now: u64) {
        for ch in self.channels.values_mut() { ch.tick(now); }
    }

    pub fn recompute(&mut self) {
        self.stats.total_channels = self.channels.len();
        self.stats.active_channels = self.channels.values().filter(|c| c.state == FlowState::Active).count();
        self.stats.throttled_channels = self.channels.values().filter(|c| c.state == FlowState::Throttled).count();
        self.stats.congested_channels = self.channels.values().filter(|c| c.state == FlowState::Congested).count();
        self.stats.total_bytes_sent = self.channels.values().map(|c| c.bytes_sent).sum();
        self.stats.total_bytes_recv = self.channels.values().map(|c| c.bytes_recv).sum();
        self.stats.total_messages = self.channels.values().map(|c| c.msgs_sent + c.msgs_recv).sum();
        let n = self.channels.len();
        if n > 0 {
            self.stats.avg_credit_pct = self.channels.values().map(|c| c.credits.available_pct()).sum::<f64>() / n as f64;
            self.stats.avg_cwnd = self.channels.values().map(|c| c.cwnd.cwnd as f64).sum::<f64>() / n as f64;
        }
    }

    #[inline(always)]
    pub fn channel(&self, id: u64) -> Option<&FlowChannel> { self.channels.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &FlowControlStats { &self.stats }
}
