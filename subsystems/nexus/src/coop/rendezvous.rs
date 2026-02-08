// SPDX-License-Identifier: GPL-2.0
//! Coop rendezvous — rendezvous point for thread synchronization.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Rendezvous state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendezvousState {
    Waiting,
    Ready,
    Exchanging,
    Complete,
    TimedOut,
    Cancelled,
}

/// Exchange mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeMode {
    Pair,
    Group,
    Broadcast,
    Pipeline,
}

/// Participant
#[derive(Debug, Clone)]
pub struct RendezvousParticipant {
    pub tid: u64,
    pub arrived_at: u64,
    pub data_hash: u64,
    pub ready: bool,
    pub received_hash: Option<u64>,
}

impl RendezvousParticipant {
    pub fn new(tid: u64, data_hash: u64, now: u64) -> Self {
        Self { tid, arrived_at: now, data_hash, ready: true, received_hash: None }
    }

    pub fn wait_time(&self, now: u64) -> u64 { now.saturating_sub(self.arrived_at) }
}

/// Rendezvous point
#[derive(Debug)]
pub struct RendezvousPoint {
    pub id: u64,
    pub state: RendezvousState,
    pub mode: ExchangeMode,
    pub required_count: u32,
    pub participants: Vec<RendezvousParticipant>,
    pub created_at: u64,
    pub completed_at: u64,
    pub total_exchanges: u64,
    pub timeout_ns: u64,
}

impl RendezvousPoint {
    pub fn new(id: u64, mode: ExchangeMode, required: u32, now: u64) -> Self {
        Self {
            id, state: RendezvousState::Waiting, mode, required_count: required,
            participants: Vec::new(), created_at: now, completed_at: 0,
            total_exchanges: 0, timeout_ns: 5_000_000_000,
        }
    }

    pub fn arrive(&mut self, tid: u64, data_hash: u64, now: u64) -> bool {
        if self.state != RendezvousState::Waiting { return false; }
        self.participants.push(RendezvousParticipant::new(tid, data_hash, now));
        if self.participants.len() as u32 >= self.required_count {
            self.state = RendezvousState::Ready;
        }
        true
    }

    pub fn exchange(&mut self, now: u64) -> bool {
        if self.state != RendezvousState::Ready { return false; }
        self.state = RendezvousState::Exchanging;

        match self.mode {
            ExchangeMode::Pair if self.participants.len() == 2 => {
                let h0 = self.participants[0].data_hash;
                let h1 = self.participants[1].data_hash;
                self.participants[0].received_hash = Some(h1);
                self.participants[1].received_hash = Some(h0);
            }
            ExchangeMode::Broadcast => {
                let first_hash = self.participants.first().map(|p| p.data_hash).unwrap_or(0);
                for p in &mut self.participants { p.received_hash = Some(first_hash); }
            }
            _ => {
                // Group: each gets the hash of the next participant (ring)
                let n = self.participants.len();
                let hashes: Vec<u64> = self.participants.iter().map(|p| p.data_hash).collect();
                for i in 0..n {
                    self.participants[i].received_hash = Some(hashes[(i + 1) % n]);
                }
            }
        }

        self.state = RendezvousState::Complete;
        self.completed_at = now;
        self.total_exchanges += 1;
        true
    }

    pub fn reset(&mut self) {
        self.participants.clear();
        self.state = RendezvousState::Waiting;
    }

    pub fn check_timeout(&mut self, now: u64) -> bool {
        if self.state == RendezvousState::Waiting && now.saturating_sub(self.created_at) > self.timeout_ns {
            self.state = RendezvousState::TimedOut;
            true
        } else { false }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct RendezvousStats {
    pub total_points: u32,
    pub waiting: u32,
    pub completed: u32,
    pub timed_out: u32,
    pub total_exchanges: u64,
    pub total_participants: u32,
}

/// Main rendezvous manager
pub struct CoopRendezvous {
    points: BTreeMap<u64, RendezvousPoint>,
    next_id: u64,
}

impl CoopRendezvous {
    pub fn new() -> Self { Self { points: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, mode: ExchangeMode, required: u32, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.points.insert(id, RendezvousPoint::new(id, mode, required, now));
        id
    }

    pub fn arrive(&mut self, point_id: u64, tid: u64, data_hash: u64, now: u64) -> bool {
        self.points.get_mut(&point_id).map(|p| p.arrive(tid, data_hash, now)).unwrap_or(false)
    }

    pub fn try_exchange(&mut self, point_id: u64, now: u64) -> bool {
        self.points.get_mut(&point_id).map(|p| p.exchange(now)).unwrap_or(false)
    }

    pub fn stats(&self) -> RendezvousStats {
        let waiting = self.points.values().filter(|p| p.state == RendezvousState::Waiting).count() as u32;
        let completed = self.points.values().filter(|p| p.state == RendezvousState::Complete).count() as u32;
        let timed_out = self.points.values().filter(|p| p.state == RendezvousState::TimedOut).count() as u32;
        let exchanges: u64 = self.points.values().map(|p| p.total_exchanges).sum();
        let participants: u32 = self.points.values().map(|p| p.participants.len() as u32).sum();
        RendezvousStats {
            total_points: self.points.len() as u32, waiting, completed,
            timed_out, total_exchanges: exchanges, total_participants: participants,
        }
    }
}

// ============================================================================
// Merged from rendezvous_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendezvousStateV2 {
    WaitingSender,
    WaitingReceiver,
    Matched,
    TimedOut,
    Cancelled,
}

/// Rendezvous endpoint
#[derive(Debug)]
pub struct RendezvousEndpointV2 {
    pub id: u64,
    pub is_sender: bool,
    pub value_hash: u64,
    pub timestamp: u64,
    pub state: RendezvousStateV2,
    pub matched_with: Option<u64>,
    pub wait_ns: u64,
}

impl RendezvousEndpointV2 {
    pub fn new_sender(id: u64, value_hash: u64, now: u64) -> Self {
        Self { id, is_sender: true, value_hash, timestamp: now, state: RendezvousStateV2::WaitingSender, matched_with: None, wait_ns: 0 }
    }

    pub fn new_receiver(id: u64, now: u64) -> Self {
        Self { id, is_sender: false, value_hash: 0, timestamp: now, state: RendezvousStateV2::WaitingReceiver, matched_with: None, wait_ns: 0 }
    }
}

/// Rendezvous channel v2
#[derive(Debug)]
pub struct RendezvousChannelV2 {
    pub waiting_senders: Vec<RendezvousEndpointV2>,
    pub waiting_receivers: Vec<RendezvousEndpointV2>,
    pub total_matches: u64,
    pub total_timeouts: u64,
    pub total_wait_ns: u64,
}

impl RendezvousChannelV2 {
    pub fn new() -> Self {
        Self { waiting_senders: Vec::new(), waiting_receivers: Vec::new(), total_matches: 0, total_timeouts: 0, total_wait_ns: 0 }
    }

    pub fn send(&mut self, id: u64, value_hash: u64, now: u64) -> Option<u64> {
        if let Some(recv) = self.waiting_receivers.first_mut() {
            let wait = now.saturating_sub(recv.timestamp);
            recv.state = RendezvousStateV2::Matched;
            recv.matched_with = Some(id);
            recv.value_hash = value_hash;
            recv.wait_ns = wait;
            self.total_matches += 1;
            self.total_wait_ns += wait;
            let recv_id = recv.id;
            self.waiting_receivers.remove(0);
            Some(recv_id)
        } else {
            self.waiting_senders.push(RendezvousEndpointV2::new_sender(id, value_hash, now));
            None
        }
    }

    pub fn recv(&mut self, id: u64, now: u64) -> Option<u64> {
        if let Some(sender) = self.waiting_senders.first_mut() {
            let wait = now.saturating_sub(sender.timestamp);
            sender.state = RendezvousStateV2::Matched;
            sender.matched_with = Some(id);
            sender.wait_ns = wait;
            self.total_matches += 1;
            self.total_wait_ns += wait;
            let val = sender.value_hash;
            self.waiting_senders.remove(0);
            Some(val)
        } else {
            self.waiting_receivers.push(RendezvousEndpointV2::new_receiver(id, now));
            None
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct RendezvousV2Stats {
    pub waiting_senders: u32,
    pub waiting_receivers: u32,
    pub total_matches: u64,
    pub total_timeouts: u64,
    pub avg_wait_ns: u64,
}

/// Main coop rendezvous v2
pub struct CoopRendezvousV2 {
    channels: BTreeMap<u64, RendezvousChannelV2>,
    next_chan: u64,
}

impl CoopRendezvousV2 {
    pub fn new() -> Self { Self { channels: BTreeMap::new(), next_chan: 1 } }

    pub fn create_channel(&mut self) -> u64 {
        let id = self.next_chan; self.next_chan += 1;
        self.channels.insert(id, RendezvousChannelV2::new());
        id
    }

    pub fn send(&mut self, chan: u64, id: u64, val: u64, now: u64) -> Option<u64> {
        self.channels.get_mut(&chan).and_then(|c| c.send(id, val, now))
    }

    pub fn recv(&mut self, chan: u64, id: u64, now: u64) -> Option<u64> {
        self.channels.get_mut(&chan).and_then(|c| c.recv(id, now))
    }

    pub fn stats(&self) -> Vec<RendezvousV2Stats> {
        self.channels.values().map(|c| {
            let avg = if c.total_matches == 0 { 0 } else { c.total_wait_ns / c.total_matches };
            RendezvousV2Stats { waiting_senders: c.waiting_senders.len() as u32, waiting_receivers: c.waiting_receivers.len() as u32, total_matches: c.total_matches, total_timeouts: c.total_timeouts, avg_wait_ns: avg }
        }).collect()
    }
}

// ============================================================================
// Merged from rendezvous_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendezvousV3State {
    Waiting,
    Matched,
    TimedOut,
    Cancelled,
}

/// Rendezvous v3 participant
#[derive(Debug)]
pub struct RendezvousV3Participant {
    pub tid: u64,
    pub data_hash: u64,
    pub arrived_at: u64,
    pub matched_at: u64,
    pub state: RendezvousV3State,
}

impl RendezvousV3Participant {
    pub fn new(tid: u64, data: u64, now: u64) -> Self {
        Self { tid, data_hash: data, arrived_at: now, matched_at: 0, state: RendezvousV3State::Waiting }
    }
}

/// Rendezvous v3 point
#[derive(Debug)]
pub struct RendezvousV3Point {
    pub id: u64,
    pub required: u32,
    pub arrived: Vec<RendezvousV3Participant>,
    pub total_matches: u64,
    pub total_timeouts: u64,
}

impl RendezvousV3Point {
    pub fn new(id: u64, required: u32) -> Self {
        Self { id, required, arrived: Vec::new(), total_matches: 0, total_timeouts: 0 }
    }

    pub fn arrive(&mut self, tid: u64, data: u64, now: u64) -> bool {
        self.arrived.push(RendezvousV3Participant::new(tid, data, now));
        if self.arrived.len() as u32 >= self.required {
            for p in &mut self.arrived { p.state = RendezvousV3State::Matched; p.matched_at = now; }
            self.total_matches += 1;
            true
        } else { false }
    }

    pub fn drain(&mut self) -> Vec<RendezvousV3Participant> {
        let mut v = Vec::new();
        core::mem::swap(&mut self.arrived, &mut v);
        v
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct RendezvousV3Stats {
    pub total_points: u32,
    pub total_matches: u64,
    pub total_timeouts: u64,
    pub waiting_threads: u32,
}

/// Main coop rendezvous v3
pub struct CoopRendezvousV3 {
    points: BTreeMap<u64, RendezvousV3Point>,
    next_id: u64,
}

impl CoopRendezvousV3 {
    pub fn new() -> Self { Self { points: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, required: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.points.insert(id, RendezvousV3Point::new(id, required));
        id
    }

    pub fn arrive(&mut self, point: u64, tid: u64, data: u64, now: u64) -> bool {
        if let Some(p) = self.points.get_mut(&point) { p.arrive(tid, data, now) } else { false }
    }

    pub fn destroy(&mut self, id: u64) { self.points.remove(&id); }

    pub fn stats(&self) -> RendezvousV3Stats {
        let matches: u64 = self.points.values().map(|p| p.total_matches).sum();
        let timeouts: u64 = self.points.values().map(|p| p.total_timeouts).sum();
        let waiting: u32 = self.points.values().map(|p| p.arrived.iter().filter(|a| a.state == RendezvousV3State::Waiting).count() as u32).sum();
        RendezvousV3Stats { total_points: self.points.len() as u32, total_matches: matches, total_timeouts: timeouts, waiting_threads: waiting }
    }
}
