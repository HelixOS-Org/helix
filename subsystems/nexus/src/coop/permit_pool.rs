// SPDX-License-Identifier: GPL-2.0
//! Coop permit_pool — cooperative permit pool for concurrency control.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Permit type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitType {
    Exclusive,
    Shared,
    ReadOnly,
    WriteOnly,
    Weighted(u32),
}

/// Permit state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermitState {
    Available,
    Acquired,
    Expired,
    Revoked,
}

/// Permit
#[derive(Debug, Clone)]
pub struct Permit {
    pub id: u64,
    pub pool_id: u64,
    pub owner_tid: u64,
    pub permit_type: PermitType,
    pub state: PermitState,
    pub weight: u32,
    pub acquired_at: u64,
    pub expires_at: Option<u64>,
}

impl Permit {
    pub fn new(id: u64, pool_id: u64, owner: u64, ptype: PermitType, now: u64) -> Self {
        let weight = match ptype { PermitType::Weighted(w) => w, PermitType::Exclusive => u32::MAX, _ => 1 };
        Self {
            id, pool_id, owner_tid: owner, permit_type: ptype,
            state: PermitState::Acquired, weight, acquired_at: now, expires_at: None,
        }
    }

    #[inline(always)]
    pub fn is_valid(&self, now: u64) -> bool {
        self.state == PermitState::Acquired && self.expires_at.map(|e| now < e).unwrap_or(true)
    }

    #[inline(always)]
    pub fn revoke(&mut self) { self.state = PermitState::Revoked; }
    #[inline(always)]
    pub fn expire(&mut self) { self.state = PermitState::Expired; }

    #[inline(always)]
    pub fn hold_time(&self, now: u64) -> u64 { now.saturating_sub(self.acquired_at) }
}

/// Pool waiter
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PoolWaiter {
    pub tid: u64,
    pub requested_type: PermitType,
    pub weight_needed: u32,
    pub enqueued_at: u64,
}

/// Permit pool
#[derive(Debug)]
#[repr(align(64))]
pub struct PermitPool {
    pub id: u64,
    pub capacity: u32,
    pub available: u32,
    pub permits: Vec<Permit>,
    pub waiters: Vec<PoolWaiter>,
    pub total_acquired: u64,
    pub total_released: u64,
    pub total_timeouts: u64,
}

impl PermitPool {
    pub fn new(id: u64, capacity: u32) -> Self {
        Self {
            id, capacity, available: capacity, permits: Vec::new(),
            waiters: Vec::new(), total_acquired: 0, total_released: 0,
            total_timeouts: 0,
        }
    }

    #[inline]
    pub fn try_acquire(&mut self, tid: u64, ptype: PermitType, now: u64) -> Option<u64> {
        let weight = match ptype { PermitType::Weighted(w) => w, PermitType::Exclusive => self.capacity, _ => 1 };
        if self.available < weight { return None; }

        self.available -= weight;
        let permit_id = self.total_acquired;
        self.total_acquired += 1;
        let permit = Permit::new(permit_id, self.id, tid, ptype, now);
        self.permits.push(permit);
        Some(permit_id)
    }

    #[inline]
    pub fn release(&mut self, permit_id: u64) -> bool {
        if let Some(pos) = self.permits.iter().position(|p| p.id == permit_id && p.state == PermitState::Acquired) {
            let weight = self.permits[pos].weight;
            self.permits[pos].state = PermitState::Available;
            self.available = (self.available + weight).min(self.capacity);
            self.total_released += 1;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        (self.capacity - self.available) as f64 / self.capacity as f64
    }

    #[inline(always)]
    pub fn enqueue_waiter(&mut self, tid: u64, ptype: PermitType, now: u64) {
        let weight = match ptype { PermitType::Weighted(w) => w, _ => 1 };
        self.waiters.push(PoolWaiter { tid, requested_type: ptype, weight_needed: weight, enqueued_at: now });
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PermitPoolStats {
    pub total_pools: u32,
    pub total_active_permits: u32,
    pub total_waiters: u32,
    pub total_acquired: u64,
    pub total_released: u64,
    pub avg_utilization: f64,
}

/// Main permit pool manager
#[repr(align(64))]
pub struct CoopPermitPool {
    pools: BTreeMap<u64, PermitPool>,
    next_id: u64,
}

impl CoopPermitPool {
    pub fn new() -> Self { Self { pools: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create_pool(&mut self, capacity: u32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.pools.insert(id, PermitPool::new(id, capacity));
        id
    }

    #[inline(always)]
    pub fn acquire(&mut self, pool_id: u64, tid: u64, ptype: PermitType, now: u64) -> Option<u64> {
        self.pools.get_mut(&pool_id)?.try_acquire(tid, ptype, now)
    }

    #[inline(always)]
    pub fn release(&mut self, pool_id: u64, permit_id: u64) -> bool {
        self.pools.get_mut(&pool_id).map(|p| p.release(permit_id)).unwrap_or(false)
    }

    pub fn stats(&self) -> PermitPoolStats {
        let active: u32 = self.pools.values().map(|p| p.permits.iter().filter(|pr| pr.state == PermitState::Acquired).count() as u32).sum();
        let waiters: u32 = self.pools.values().map(|p| p.waiters.len() as u32).sum();
        let acquired: u64 = self.pools.values().map(|p| p.total_acquired).sum();
        let released: u64 = self.pools.values().map(|p| p.total_released).sum();
        let utils: Vec<f64> = self.pools.values().map(|p| p.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        PermitPoolStats {
            total_pools: self.pools.len() as u32, total_active_permits: active,
            total_waiters: waiters, total_acquired: acquired,
            total_released: released, avg_utilization: avg,
        }
    }
}
