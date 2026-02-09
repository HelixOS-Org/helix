// SPDX-License-Identifier: GPL-2.0
//! Coop credit_flow — credit-based flow control protocol.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Credit type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditType {
    SendCredit,
    ReceiveCredit,
    BufferCredit,
    BandwidthCredit,
}

/// Credit flow state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditFlowState {
    Normal,
    LowCredit,
    Blocked,
    Replenishing,
    Overdrawn,
}

/// Credit grant
#[derive(Debug, Clone)]
pub struct CreditGrant {
    pub id: u64,
    pub credit_type: CreditType,
    pub amount: u64,
    pub remaining: u64,
    pub granted_at: u64,
    pub expires_at: u64,
}

impl CreditGrant {
    pub fn new(id: u64, ctype: CreditType, amount: u64, now: u64, ttl: u64) -> Self {
        Self {
            id, credit_type: ctype, amount, remaining: amount,
            granted_at: now, expires_at: now + ttl,
        }
    }

    #[inline]
    pub fn consume(&mut self, n: u64) -> u64 {
        let consumed = n.min(self.remaining);
        self.remaining -= consumed;
        consumed
    }

    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool { now >= self.expires_at }
    #[inline(always)]
    pub fn is_exhausted(&self) -> bool { self.remaining == 0 }
    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.amount == 0 { return 1.0; }
        (self.amount - self.remaining) as f64 / self.amount as f64
    }
}

/// Credit endpoint (sender/receiver)
#[derive(Debug)]
pub struct CreditEndpoint {
    pub id: u64,
    pub grants: Vec<CreditGrant>,
    pub state: CreditFlowState,
    pub total_credits_received: u64,
    pub total_credits_consumed: u64,
    pub total_blocked_ns: u64,
    pub block_count: u64,
    pub last_grant_at: u64,
}

impl CreditEndpoint {
    pub fn new(id: u64) -> Self {
        Self {
            id, grants: Vec::new(), state: CreditFlowState::Normal,
            total_credits_received: 0, total_credits_consumed: 0,
            total_blocked_ns: 0, block_count: 0, last_grant_at: 0,
        }
    }

    #[inline]
    pub fn grant(&mut self, ctype: CreditType, amount: u64, now: u64, ttl: u64) {
        let grant = CreditGrant::new(self.grants.len() as u64, ctype, amount, now, ttl);
        self.total_credits_received += amount;
        self.last_grant_at = now;
        self.grants.push(grant);
        self.update_state();
    }

    #[inline]
    pub fn consume(&mut self, amount: u64) -> u64 {
        let mut remaining = amount;
        for grant in &mut self.grants {
            if remaining == 0 { break; }
            remaining -= grant.consume(remaining);
        }
        let consumed = amount - remaining;
        self.total_credits_consumed += consumed;
        self.update_state();
        consumed
    }

    #[inline(always)]
    pub fn available(&self) -> u64 {
        self.grants.iter().map(|g| g.remaining).sum()
    }

    #[inline(always)]
    pub fn cleanup_expired(&mut self, now: u64) {
        self.grants.retain(|g| !g.is_expired(now) || !g.is_exhausted());
    }

    fn update_state(&mut self) {
        let avail = self.available();
        self.state = if avail == 0 { CreditFlowState::Blocked }
            else if avail < 10 { CreditFlowState::LowCredit }
            else { CreditFlowState::Normal };
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CreditFlowStats {
    pub total_endpoints: u32,
    pub blocked_endpoints: u32,
    pub total_credits_granted: u64,
    pub total_credits_consumed: u64,
    pub total_blocks: u64,
    pub avg_utilization: f64,
}

/// Main credit flow manager
pub struct CoopCreditFlow {
    endpoints: BTreeMap<u64, CreditEndpoint>,
    next_id: u64,
}

impl CoopCreditFlow {
    pub fn new() -> Self { Self { endpoints: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create_endpoint(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.endpoints.insert(id, CreditEndpoint::new(id));
        id
    }

    #[inline(always)]
    pub fn grant(&mut self, endpoint: u64, ctype: CreditType, amount: u64, now: u64, ttl: u64) {
        if let Some(ep) = self.endpoints.get_mut(&endpoint) { ep.grant(ctype, amount, now, ttl); }
    }

    #[inline(always)]
    pub fn consume(&mut self, endpoint: u64, amount: u64) -> u64 {
        self.endpoints.get_mut(&endpoint).map(|ep| ep.consume(amount)).unwrap_or(0)
    }

    pub fn stats(&self) -> CreditFlowStats {
        let blocked = self.endpoints.values().filter(|e| e.state == CreditFlowState::Blocked).count() as u32;
        let granted: u64 = self.endpoints.values().map(|e| e.total_credits_received).sum();
        let consumed: u64 = self.endpoints.values().map(|e| e.total_credits_consumed).sum();
        let blocks: u64 = self.endpoints.values().map(|e| e.block_count).sum();
        let utils: Vec<f64> = self.endpoints.values().map(|e| {
            if e.total_credits_received == 0 { 0.0 } else { e.total_credits_consumed as f64 / e.total_credits_received as f64 }
        }).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        CreditFlowStats {
            total_endpoints: self.endpoints.len() as u32, blocked_endpoints: blocked,
            total_credits_granted: granted, total_credits_consumed: consumed,
            total_blocks: blocks, avg_utilization: avg,
        }
    }
}
