// SPDX-License-Identifier: GPL-2.0
//! Coop QoS — cooperative quality of service with shared bandwidth allocation

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// QoS class type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopQosClass {
    BestEffort,
    Interactive,
    Streaming,
    Bulk,
    RealTime,
    Control,
}

/// QoS scheduling discipline
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopQosSched {
    Fifo,
    Sfq,
    Htb,
    Fq,
    FqCodel,
    Tbf,
}

/// Bandwidth allocation
#[derive(Debug, Clone)]
pub struct BwAllocation {
    pub class: CoopQosClass,
    pub rate_bps: u64,
    pub ceil_bps: u64,
    pub burst_bytes: u32,
    pub used_bps: u64,
    pub priority: u8,
}

impl BwAllocation {
    pub fn new(class: CoopQosClass, rate_bps: u64, ceil_bps: u64) -> Self {
        Self { class, rate_bps, ceil_bps, burst_bytes: 4096, used_bps: 0, priority: 4 }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.rate_bps == 0 { 0.0 } else { self.used_bps as f64 / self.rate_bps as f64 }
    }

    #[inline(always)]
    pub fn can_borrow(&self) -> bool {
        self.used_bps < self.ceil_bps
    }

    #[inline(always)]
    pub fn consume(&mut self, bytes: u64) {
        self.used_bps += bytes * 8;
    }

    #[inline(always)]
    pub fn reset_interval(&mut self) {
        self.used_bps = 0;
    }
}

/// Shared QoS policy
#[derive(Debug, Clone)]
pub struct SharedQosPolicy {
    pub policy_id: u64,
    pub allocations: Vec<BwAllocation>,
    pub total_bandwidth_bps: u64,
    pub subscribers: Vec<u64>,
}

impl SharedQosPolicy {
    pub fn new(policy_id: u64, total_bw: u64) -> Self {
        Self { policy_id, allocations: Vec::new(), total_bandwidth_bps: total_bw, subscribers: Vec::new() }
    }

    #[inline(always)]
    pub fn add_class(&mut self, alloc: BwAllocation) {
        self.allocations.push(alloc);
    }

    #[inline(always)]
    pub fn allocated_bps(&self) -> u64 {
        self.allocations.iter().map(|a| a.rate_bps).sum()
    }

    #[inline(always)]
    pub fn remaining_bps(&self) -> u64 {
        self.total_bandwidth_bps.saturating_sub(self.allocated_bps())
    }

    #[inline(always)]
    pub fn subscribe(&mut self, ns_id: u64) {
        if !self.subscribers.contains(&ns_id) { self.subscribers.push(ns_id); }
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
pub struct CoopTokenBucket {
    pub tokens: u64,
    pub max_tokens: u64,
    pub rate: u64,
    pub last_refill_ns: u64,
}

impl CoopTokenBucket {
    pub fn new(max_tokens: u64, rate: u64) -> Self {
        Self { tokens: max_tokens, max_tokens, rate, last_refill_ns: 0 }
    }

    #[inline]
    pub fn refill(&mut self, now_ns: u64) {
        let elapsed = now_ns.saturating_sub(self.last_refill_ns);
        let new_tokens = (self.rate * elapsed) / 1_000_000_000;
        self.tokens = (self.tokens + new_tokens).min(self.max_tokens);
        self.last_refill_ns = now_ns;
    }

    #[inline(always)]
    pub fn consume(&mut self, tokens: u64) -> bool {
        if self.tokens >= tokens { self.tokens -= tokens; true } else { false }
    }
}

/// Coop QoS stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopQosStats {
    pub total_policies: u64,
    pub total_classes: u64,
    pub total_bytes_shaped: u64,
    pub total_drops: u64,
}

/// Main coop QoS manager
#[derive(Debug)]
pub struct CoopQos {
    pub policies: BTreeMap<u64, SharedQosPolicy>,
    pub buckets: BTreeMap<u64, CoopTokenBucket>,
    pub stats: CoopQosStats,
}

impl CoopQos {
    pub fn new() -> Self {
        Self {
            policies: BTreeMap::new(),
            buckets: BTreeMap::new(),
            stats: CoopQosStats { total_policies: 0, total_classes: 0, total_bytes_shaped: 0, total_drops: 0 },
        }
    }

    #[inline(always)]
    pub fn create_policy(&mut self, policy_id: u64, total_bw: u64) {
        self.policies.insert(policy_id, SharedQosPolicy::new(policy_id, total_bw));
        self.stats.total_policies += 1;
    }

    #[inline]
    pub fn add_class(&mut self, policy_id: u64, alloc: BwAllocation) -> bool {
        if let Some(policy) = self.policies.get_mut(&policy_id) {
            policy.add_class(alloc);
            self.stats.total_classes += 1;
            true
        } else { false }
    }
}
