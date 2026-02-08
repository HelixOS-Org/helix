// SPDX-License-Identifier: GPL-2.0
//! Coop prio_arbiter — priority arbitration for resource contention.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Arbitration policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitrationPolicy {
    HighestPriority,
    RoundRobin,
    FairShare,
    WeightedFair,
    Lottery,
    EarliestDeadline,
}

/// Contender for a resource
#[derive(Debug, Clone)]
pub struct Contender {
    pub id: u32,
    pub priority: u32,
    pub weight: u32,
    pub deadline: u64,
    pub request_time: u64,
    pub wait_time: u64,
    pub granted_count: u64,
    pub denied_count: u64,
    pub tickets: u32,
}

impl Contender {
    pub fn new(id: u32, priority: u32, weight: u32, now: u64) -> Self {
        Self {
            id, priority, weight, deadline: 0, request_time: now,
            wait_time: 0, granted_count: 0, denied_count: 0, tickets: weight,
        }
    }

    pub fn grant_ratio(&self) -> f64 {
        let total = self.granted_count + self.denied_count;
        if total == 0 { return 0.0; }
        self.granted_count as f64 / total as f64
    }

    pub fn avg_wait(&self) -> u64 {
        let total = self.granted_count + self.denied_count;
        if total == 0 { 0 } else { self.wait_time / total }
    }
}

/// Arbitration request
#[derive(Debug, Clone)]
pub struct ArbitrationRequest {
    pub resource_id: u64,
    pub contender_id: u32,
    pub priority: u32,
    pub weight: u32,
    pub deadline: u64,
    pub timestamp: u64,
}

/// Arbitration result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArbitrationResult {
    Granted,
    Denied,
    Queued,
    Preempted,
}

/// Resource arbitration context
#[derive(Debug)]
pub struct ResourceArbContext {
    pub resource_id: u64,
    pub policy: ArbitrationPolicy,
    pub contenders: Vec<Contender>,
    pub current_holder: Option<u32>,
    pub round_robin_idx: usize,
    pub total_arbitrations: u64,
    pub total_preemptions: u64,
    pub rng_state: u64,
}

impl ResourceArbContext {
    pub fn new(resource_id: u64, policy: ArbitrationPolicy) -> Self {
        Self {
            resource_id, policy, contenders: Vec::new(),
            current_holder: None, round_robin_idx: 0,
            total_arbitrations: 0, total_preemptions: 0,
            rng_state: resource_id ^ 0xdeadbeef,
        }
    }

    fn xorshift64(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng_state = x;
        x
    }

    pub fn add_contender(&mut self, contender: Contender) {
        if !self.contenders.iter().any(|c| c.id == contender.id) {
            self.contenders.push(contender);
        }
    }

    pub fn remove_contender(&mut self, id: u32) -> bool {
        let before = self.contenders.len();
        self.contenders.retain(|c| c.id != id);
        if self.current_holder == Some(id) { self.current_holder = None; }
        self.contenders.len() < before
    }

    pub fn arbitrate(&mut self, now: u64) -> Option<u32> {
        if self.contenders.is_empty() { return None; }
        self.total_arbitrations += 1;

        let winner_id = match self.policy {
            ArbitrationPolicy::HighestPriority => {
                self.contenders.iter().max_by_key(|c| c.priority).map(|c| c.id)
            }
            ArbitrationPolicy::RoundRobin => {
                if self.round_robin_idx >= self.contenders.len() { self.round_robin_idx = 0; }
                let id = self.contenders[self.round_robin_idx].id;
                self.round_robin_idx += 1;
                Some(id)
            }
            ArbitrationPolicy::FairShare => {
                self.contenders.iter().min_by_key(|c| c.granted_count).map(|c| c.id)
            }
            ArbitrationPolicy::WeightedFair => {
                let total_weight: u64 = self.contenders.iter().map(|c| c.weight as u64).sum();
                if total_weight == 0 { return None; }
                let mut target = self.xorshift64() % total_weight;
                let mut winner = self.contenders[0].id;
                for c in &self.contenders {
                    if target < c.weight as u64 { winner = c.id; break; }
                    target -= c.weight as u64;
                }
                Some(winner)
            }
            ArbitrationPolicy::Lottery => {
                let total_tickets: u64 = self.contenders.iter().map(|c| c.tickets as u64).sum();
                if total_tickets == 0 { return None; }
                let ticket = self.xorshift64() % total_tickets;
                let mut acc = 0u64;
                let mut winner = self.contenders[0].id;
                for c in &self.contenders {
                    acc += c.tickets as u64;
                    if ticket < acc { winner = c.id; break; }
                }
                Some(winner)
            }
            ArbitrationPolicy::EarliestDeadline => {
                self.contenders.iter()
                    .filter(|c| c.deadline > 0)
                    .min_by_key(|c| c.deadline)
                    .map(|c| c.id)
                    .or_else(|| self.contenders.first().map(|c| c.id))
            }
        };

        if let Some(wid) = winner_id {
            if let Some(old) = self.current_holder {
                if old != wid { self.total_preemptions += 1; }
            }
            self.current_holder = Some(wid);
            for c in &mut self.contenders {
                let wait = now.saturating_sub(c.request_time);
                if c.id == wid {
                    c.granted_count += 1;
                    c.wait_time += wait;
                } else {
                    c.denied_count += 1;
                    c.wait_time += wait;
                }
                c.request_time = now;
            }
        }
        winner_id
    }
}

/// Prio arbiter stats
#[derive(Debug, Clone)]
pub struct PrioArbiterStats {
    pub tracked_resources: u32,
    pub total_contenders: u64,
    pub total_arbitrations: u64,
    pub total_preemptions: u64,
}

/// Main priority arbiter
pub struct CoopPrioArbiter {
    resources: BTreeMap<u64, ResourceArbContext>,
    total_arbitrations: u64,
    total_preemptions: u64,
}

impl CoopPrioArbiter {
    pub fn new() -> Self {
        Self { resources: BTreeMap::new(), total_arbitrations: 0, total_preemptions: 0 }
    }

    pub fn register_resource(&mut self, resource_id: u64, policy: ArbitrationPolicy) {
        self.resources.insert(resource_id, ResourceArbContext::new(resource_id, policy));
    }

    pub fn add_contender(&mut self, resource_id: u64, contender: Contender) -> bool {
        if let Some(ctx) = self.resources.get_mut(&resource_id) {
            ctx.add_contender(contender);
            true
        } else { false }
    }

    pub fn arbitrate(&mut self, resource_id: u64, now: u64) -> Option<u32> {
        let ctx = self.resources.get_mut(&resource_id)?;
        let result = ctx.arbitrate(now);
        self.total_arbitrations += 1;
        if ctx.total_preemptions > 0 { self.total_preemptions = ctx.total_preemptions; }
        result
    }

    pub fn release(&mut self, resource_id: u64) {
        if let Some(ctx) = self.resources.get_mut(&resource_id) {
            ctx.current_holder = None;
        }
    }

    pub fn stats(&self) -> PrioArbiterStats {
        let total_cont: u64 = self.resources.values().map(|r| r.contenders.len() as u64).sum();
        PrioArbiterStats {
            tracked_resources: self.resources.len() as u32,
            total_contenders: total_cont,
            total_arbitrations: self.total_arbitrations,
            total_preemptions: self.total_preemptions,
        }
    }
}
