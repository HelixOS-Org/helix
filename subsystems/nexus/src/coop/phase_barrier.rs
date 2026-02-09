// SPDX-License-Identifier: GPL-2.0
//! Coop phase_barrier — multi-phase synchronization barrier.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Phase state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseState {
    Registration,
    Active,
    Advancing,
    Terminated,
}

/// Phase participant
#[derive(Debug)]
pub struct PhaseParticipant {
    pub id: u64,
    pub current_phase: u64,
    pub arrived: bool,
    pub phases_completed: u64,
    pub total_wait_ns: u64,
}

impl PhaseParticipant {
    pub fn new(id: u64) -> Self {
        Self { id, current_phase: 0, arrived: false, phases_completed: 0, total_wait_ns: 0 }
    }

    #[inline(always)]
    pub fn arrive(&mut self) { self.arrived = true; }

    #[inline]
    pub fn advance(&mut self, wait_ns: u64) {
        self.current_phase += 1;
        self.arrived = false;
        self.phases_completed += 1;
        self.total_wait_ns += wait_ns;
    }
}

/// Phase barrier
#[derive(Debug)]
pub struct PhaseBarrier {
    pub current_phase: u64,
    pub state: PhaseState,
    pub participants: BTreeMap<u64, PhaseParticipant>,
    pub phase_durations: Vec<u64>,
    pub phase_start_time: u64,
}

impl PhaseBarrier {
    pub fn new() -> Self {
        Self { current_phase: 0, state: PhaseState::Registration, participants: BTreeMap::new(), phase_durations: Vec::new(), phase_start_time: 0 }
    }

    #[inline(always)]
    pub fn register(&mut self, id: u64) { self.participants.insert(id, PhaseParticipant::new(id)); }

    #[inline(always)]
    pub fn start(&mut self, now: u64) { self.state = PhaseState::Active; self.phase_start_time = now; }

    #[inline(always)]
    pub fn arrive(&mut self, id: u64) -> bool {
        if let Some(p) = self.participants.get_mut(&id) { p.arrive(); }
        self.participants.values().all(|p| p.arrived)
    }

    #[inline]
    pub fn advance(&mut self, now: u64) {
        let duration = now.saturating_sub(self.phase_start_time);
        self.phase_durations.push(duration);
        for p in self.participants.values_mut() { p.advance(duration); }
        self.current_phase += 1;
        self.phase_start_time = now;
    }

    #[inline(always)]
    pub fn deregister(&mut self, id: u64) { self.participants.remove(&id); }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PhaseBarrierStats {
    pub current_phase: u64,
    pub participant_count: u32,
    pub arrived_count: u32,
    pub avg_phase_duration_ns: u64,
}

/// Main coop phase barrier
pub struct CoopPhaseBarrier {
    barriers: Vec<PhaseBarrier>,
}

impl CoopPhaseBarrier {
    pub fn new() -> Self { Self { barriers: Vec::new() } }

    #[inline(always)]
    pub fn create(&mut self) -> usize { let idx = self.barriers.len(); self.barriers.push(PhaseBarrier::new()); idx }

    #[inline(always)]
    pub fn register(&mut self, idx: usize, id: u64) {
        if let Some(b) = self.barriers.get_mut(idx) { b.register(id); }
    }

    #[inline]
    pub fn stats(&self) -> Vec<PhaseBarrierStats> {
        self.barriers.iter().map(|b| {
            let arrived = b.participants.values().filter(|p| p.arrived).count() as u32;
            let avg = if b.phase_durations.is_empty() { 0 } else { b.phase_durations.iter().sum::<u64>() / b.phase_durations.len() as u64 };
            PhaseBarrierStats { current_phase: b.current_phase, participant_count: b.participants.len() as u32, arrived_count: arrived, avg_phase_duration_ns: avg }
        }).collect()
    }
}

// ============================================================================
// Merged from phase_barrier_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseV2State {
    Arriving,
    Advancing,
    Terminated,
}

/// A participant in the phase barrier
#[derive(Debug, Clone)]
pub struct PhaseV2Participant {
    pub id: u64,
    pub current_phase: u64,
    pub arrived: bool,
    pub phases_completed: u64,
    pub deregistered: bool,
}

/// A phase barrier V2 instance
#[derive(Debug, Clone)]
pub struct PhaseBarrierV2Instance {
    pub id: u64,
    pub current_phase: u64,
    pub state: PhaseV2State,
    pub participants: Vec<PhaseV2Participant>,
    pub arrived_count: u32,
    pub total_advances: u64,
    pub on_advance_action: u32,
}

impl PhaseBarrierV2Instance {
    pub fn new(id: u64) -> Self {
        Self {
            id, current_phase: 0,
            state: PhaseV2State::Arriving,
            participants: Vec::new(),
            arrived_count: 0, total_advances: 0,
            on_advance_action: 0,
        }
    }

    #[inline]
    pub fn register(&mut self, pid: u64) -> u64 {
        let idx = self.participants.len() as u64;
        self.participants.push(PhaseV2Participant {
            id: pid, current_phase: self.current_phase,
            arrived: false, phases_completed: 0,
            deregistered: false,
        });
        idx
    }

    pub fn arrive(&mut self, pid: u64) -> bool {
        for p in self.participants.iter_mut() {
            if p.id == pid && !p.arrived && !p.deregistered {
                p.arrived = true;
                self.arrived_count += 1;
                break;
            }
        }
        let active = self.participants.iter().filter(|p| !p.deregistered).count() as u32;
        if self.arrived_count >= active && active > 0 {
            self.advance();
            return true;
        }
        false
    }

    fn advance(&mut self) {
        self.current_phase += 1;
        self.total_advances += 1;
        self.arrived_count = 0;
        for p in self.participants.iter_mut() {
            if !p.deregistered {
                p.arrived = false;
                p.current_phase = self.current_phase;
                p.phases_completed += 1;
            }
        }
        self.state = PhaseV2State::Arriving;
    }

    #[inline]
    pub fn deregister(&mut self, pid: u64) {
        for p in self.participants.iter_mut() {
            if p.id == pid {
                p.deregistered = true;
                if p.arrived { self.arrived_count -= 1; }
                break;
            }
        }
    }
}

/// Statistics for phase barrier V2
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PhaseBarrierV2Stats {
    pub barriers_created: u64,
    pub total_phases: u64,
    pub total_arrivals: u64,
    pub registrations: u64,
    pub deregistrations: u64,
}

/// Main phase barrier V2 coop manager
#[derive(Debug)]
pub struct CoopPhaseBarrierV2 {
    barriers: BTreeMap<u64, PhaseBarrierV2Instance>,
    next_id: u64,
    stats: PhaseBarrierV2Stats,
}

impl CoopPhaseBarrierV2 {
    pub fn new() -> Self {
        Self {
            barriers: BTreeMap::new(),
            next_id: 1,
            stats: PhaseBarrierV2Stats {
                barriers_created: 0, total_phases: 0,
                total_arrivals: 0, registrations: 0,
                deregistrations: 0,
            },
        }
    }

    #[inline]
    pub fn create(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.barriers.insert(id, PhaseBarrierV2Instance::new(id));
        self.stats.barriers_created += 1;
        id
    }

    #[inline]
    pub fn register(&mut self, barrier_id: u64, pid: u64) -> bool {
        if let Some(b) = self.barriers.get_mut(&barrier_id) {
            b.register(pid);
            self.stats.registrations += 1;
            true
        } else { false }
    }

    #[inline]
    pub fn arrive(&mut self, barrier_id: u64, pid: u64) -> bool {
        if let Some(b) = self.barriers.get_mut(&barrier_id) {
            self.stats.total_arrivals += 1;
            if b.arrive(pid) {
                self.stats.total_phases += 1;
                return true;
            }
        }
        false
    }

    #[inline(always)]
    pub fn stats(&self) -> &PhaseBarrierV2Stats {
        &self.stats
    }
}
