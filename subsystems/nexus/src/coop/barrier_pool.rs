// SPDX-License-Identifier: GPL-2.0
//! Coop barrier_pool — barrier synchronization pool.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Barrier type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierType {
    Cyclic,
    OneShot,
    Phased,
    Tree,
}

/// Barrier state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierState {
    Open,
    Waiting,
    Tripped,
    Broken,
    Reset,
}

/// Barrier participant
#[derive(Debug)]
pub struct BarrierParticipant {
    pub tid: u64,
    pub arrived: bool,
    pub arrival_time: u64,
    pub phase: u32,
    pub trip_count: u64,
}

impl BarrierParticipant {
    pub fn new(tid: u64) -> Self { Self { tid, arrived: false, arrival_time: 0, phase: 0, trip_count: 0 } }
    #[inline(always)]
    pub fn arrive(&mut self, now: u64) { self.arrived = true; self.arrival_time = now; }
    #[inline(always)]
    pub fn reset(&mut self) { self.arrived = false; self.phase += 1; self.trip_count += 1; }
}

/// Barrier instance
#[derive(Debug)]
pub struct BarrierInstance {
    pub id: u64,
    pub barrier_type: BarrierType,
    pub state: BarrierState,
    pub parties: u32,
    pub participants: Vec<BarrierParticipant>,
    pub generation: u64,
    pub trip_count: u64,
    pub total_wait_ns: u64,
}

impl BarrierInstance {
    pub fn new(id: u64, btype: BarrierType, parties: u32) -> Self {
        Self { id, barrier_type: btype, state: BarrierState::Open, parties, participants: Vec::new(), generation: 0, trip_count: 0, total_wait_ns: 0 }
    }

    #[inline(always)]
    pub fn register(&mut self, tid: u64) { if self.participants.len() < self.parties as usize { self.participants.push(BarrierParticipant::new(tid)); } }

    #[inline]
    pub fn arrive(&mut self, tid: u64, now: u64) -> bool {
        if let Some(p) = self.participants.iter_mut().find(|p| p.tid == tid) { p.arrive(now); }
        let arrived = self.participants.iter().filter(|p| p.arrived).count();
        if arrived >= self.parties as usize {
            self.trip(now);
            true
        } else { self.state = BarrierState::Waiting; false }
    }

    fn trip(&mut self, now: u64) {
        self.state = BarrierState::Tripped;
        self.trip_count += 1;
        self.generation += 1;
        let first_arrival = self.participants.iter().filter(|p| p.arrived).map(|p| p.arrival_time).min().unwrap_or(now);
        self.total_wait_ns += now.saturating_sub(first_arrival);
        if self.barrier_type == BarrierType::Cyclic {
            for p in &mut self.participants { p.reset(); }
            self.state = BarrierState::Open;
        }
    }

    #[inline(always)]
    pub fn avg_trip_wait(&self) -> u64 { if self.trip_count == 0 { 0 } else { self.total_wait_ns / self.trip_count } }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BarrierPoolStats {
    pub total_barriers: u32,
    pub total_trips: u64,
    pub active_barriers: u32,
    pub total_participants: u32,
    pub avg_wait_ns: u64,
}

/// Main barrier pool
#[repr(align(64))]
pub struct CoopBarrierPool {
    barriers: BTreeMap<u64, BarrierInstance>,
    next_id: u64,
}

impl CoopBarrierPool {
    pub fn new() -> Self { Self { barriers: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, btype: BarrierType, parties: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.barriers.insert(id, BarrierInstance::new(id, btype, parties));
        id
    }

    #[inline(always)]
    pub fn arrive(&mut self, barrier: u64, tid: u64, now: u64) -> bool {
        self.barriers.get_mut(&barrier).map(|b| b.arrive(tid, now)).unwrap_or(false)
    }

    #[inline]
    pub fn stats(&self) -> BarrierPoolStats {
        let trips: u64 = self.barriers.values().map(|b| b.trip_count).sum();
        let active = self.barriers.values().filter(|b| b.state == BarrierState::Waiting).count() as u32;
        let parts: u32 = self.barriers.values().map(|b| b.participants.len() as u32).sum();
        let waits: Vec<u64> = self.barriers.values().filter(|b| b.trip_count > 0).map(|b| b.avg_trip_wait()).collect();
        let avg = if waits.is_empty() { 0 } else { waits.iter().sum::<u64>() / waits.len() as u64 };
        BarrierPoolStats { total_barriers: self.barriers.len() as u32, total_trips: trips, active_barriers: active, total_participants: parts, avg_wait_ns: avg }
    }
}
