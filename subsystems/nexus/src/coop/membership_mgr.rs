//! # Coop Membership Manager
//!
//! Dynamic membership management for cooperative groups:
//! - Join/leave protocol with handshake
//! - Member health tracking with failure detection
//! - View change protocol (membership epochs)
//! - Suspicion-based failure detection (SWIM-like)
//! - Member roles and capabilities
//! - Graceful shutdown with state drain

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Member status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatus {
    Joining,
    Active,
    Suspect,
    Faulty,
    Leaving,
    Left,
    Expelled,
    Recovering,
}

/// Member role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberRole {
    Regular,
    Coordinator,
    Observer,
    Arbiter,
    Seed,
    Learner,
}

/// Member descriptor
#[derive(Debug, Clone)]
pub struct MemberDesc {
    pub id: u64,
    pub status: MemberStatus,
    pub role: MemberRole,
    pub join_epoch: u64,
    pub join_ts: u64,
    pub last_heartbeat: u64,
    pub incarnation: u64,
    pub suspicion_count: u32,
    pub capabilities: u64,
    pub weight: u32,
    pub metadata_hash: u64,
}

impl MemberDesc {
    pub fn new(id: u64, role: MemberRole, ts: u64, epoch: u64) -> Self {
        Self {
            id, status: MemberStatus::Joining, role, join_epoch: epoch,
            join_ts: ts, last_heartbeat: ts, incarnation: 0,
            suspicion_count: 0, capabilities: 0, weight: 100,
            metadata_hash: 0,
        }
    }

    pub fn activate(&mut self) { self.status = MemberStatus::Active; self.suspicion_count = 0; }
    pub fn suspect(&mut self) { self.suspicion_count += 1; if self.suspicion_count >= 3 { self.status = MemberStatus::Suspect; } }
    pub fn mark_faulty(&mut self) { self.status = MemberStatus::Faulty; }
    pub fn begin_leave(&mut self) { self.status = MemberStatus::Leaving; }
    pub fn complete_leave(&mut self) { self.status = MemberStatus::Left; }
    pub fn expel(&mut self) { self.status = MemberStatus::Expelled; }
    pub fn heartbeat(&mut self, ts: u64) { self.last_heartbeat = ts; self.suspicion_count = 0; if self.status == MemberStatus::Suspect { self.status = MemberStatus::Active; } }
    pub fn is_alive(&self) -> bool { matches!(self.status, MemberStatus::Active | MemberStatus::Joining | MemberStatus::Leaving) }
    pub fn refute(&mut self) { self.incarnation += 1; self.status = MemberStatus::Active; self.suspicion_count = 0; }
}

/// View (membership epoch)
#[derive(Debug, Clone)]
pub struct MembershipView {
    pub epoch: u64,
    pub members: Vec<u64>,
    pub coordinator: Option<u64>,
    pub ts: u64,
    pub checksum: u64,
}

impl MembershipView {
    pub fn new(epoch: u64, members: Vec<u64>, coord: Option<u64>, ts: u64) -> Self {
        let mut ck: u64 = 0xcbf29ce484222325;
        ck ^= epoch;
        ck = ck.wrapping_mul(0x100000001b3);
        for &m in &members { ck ^= m; ck = ck.wrapping_mul(0x100000001b3); }
        Self { epoch, members, coordinator: coord, ts, checksum: ck }
    }
}

/// Join request
#[derive(Debug, Clone)]
pub struct JoinRequest {
    pub member_id: u64,
    pub role: MemberRole,
    pub capabilities: u64,
    pub ts: u64,
    pub sponsor: Option<u64>,
}

/// Membership event
#[derive(Debug, Clone)]
pub struct MembershipEvent {
    pub ts: u64,
    pub kind: MembershipEventKind,
    pub member_id: u64,
    pub epoch: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipEventKind {
    Joined,
    Activated,
    Suspected,
    Faulted,
    Left,
    Expelled,
    Recovered,
    ViewChanged,
    CoordinatorChanged,
    Refuted,
}

/// Membership stats
#[derive(Debug, Clone, Default)]
pub struct MembershipStats {
    pub total_members: usize,
    pub active: usize,
    pub suspect: usize,
    pub faulty: usize,
    pub current_epoch: u64,
    pub joins: u64,
    pub leaves: u64,
    pub expulsions: u64,
}

/// Cooperative membership manager
pub struct CoopMembershipMgr {
    members: BTreeMap<u64, MemberDesc>,
    views: Vec<MembershipView>,
    events: Vec<MembershipEvent>,
    stats: MembershipStats,
    current_epoch: u64,
    suspect_timeout_ns: u64,
    faulty_timeout_ns: u64,
}

impl CoopMembershipMgr {
    pub fn new(suspect_timeout: u64, faulty_timeout: u64) -> Self {
        Self {
            members: BTreeMap::new(), views: Vec::new(),
            events: Vec::new(), stats: MembershipStats::default(),
            current_epoch: 0, suspect_timeout_ns: suspect_timeout,
            faulty_timeout_ns: faulty_timeout,
        }
    }

    pub fn join(&mut self, req: JoinRequest) -> u64 {
        let mut m = MemberDesc::new(req.member_id, req.role, req.ts, self.current_epoch);
        m.capabilities = req.capabilities;
        self.members.insert(req.member_id, m);
        self.events.push(MembershipEvent { ts: req.ts, kind: MembershipEventKind::Joined, member_id: req.member_id, epoch: self.current_epoch });
        self.stats.joins += 1;
        req.member_id
    }

    pub fn activate(&mut self, id: u64, ts: u64) {
        if let Some(m) = self.members.get_mut(&id) {
            m.activate();
            self.events.push(MembershipEvent { ts, kind: MembershipEventKind::Activated, member_id: id, epoch: self.current_epoch });
        }
    }

    pub fn heartbeat(&mut self, id: u64, ts: u64) {
        if let Some(m) = self.members.get_mut(&id) { m.heartbeat(ts); }
    }

    pub fn leave(&mut self, id: u64, ts: u64) {
        if let Some(m) = self.members.get_mut(&id) {
            m.begin_leave();
            m.complete_leave();
            self.events.push(MembershipEvent { ts, kind: MembershipEventKind::Left, member_id: id, epoch: self.current_epoch });
            self.stats.leaves += 1;
        }
    }

    pub fn expel(&mut self, id: u64, ts: u64) {
        if let Some(m) = self.members.get_mut(&id) {
            m.expel();
            self.events.push(MembershipEvent { ts, kind: MembershipEventKind::Expelled, member_id: id, epoch: self.current_epoch });
            self.stats.expulsions += 1;
        }
    }

    pub fn detect_failures(&mut self, now: u64) -> Vec<u64> {
        let mut faulted = Vec::new();
        let ids: Vec<u64> = self.members.keys().copied().collect();
        for id in ids {
            if let Some(m) = self.members.get_mut(&id) {
                if !m.is_alive() { continue; }
                let elapsed = now.saturating_sub(m.last_heartbeat);
                if elapsed >= self.faulty_timeout_ns && m.status != MemberStatus::Faulty {
                    m.mark_faulty();
                    faulted.push(id);
                    self.events.push(MembershipEvent { ts: now, kind: MembershipEventKind::Faulted, member_id: id, epoch: self.current_epoch });
                } else if elapsed >= self.suspect_timeout_ns && m.status == MemberStatus::Active {
                    m.suspect();
                    if m.status == MemberStatus::Suspect {
                        self.events.push(MembershipEvent { ts: now, kind: MembershipEventKind::Suspected, member_id: id, epoch: self.current_epoch });
                    }
                }
            }
        }
        if !faulted.is_empty() { self.advance_view(now); }
        faulted
    }

    pub fn advance_view(&mut self, ts: u64) {
        self.current_epoch += 1;
        let members: Vec<u64> = self.members.values().filter(|m| m.is_alive()).map(|m| m.id).collect();
        let coord = self.members.values().filter(|m| m.role == MemberRole::Coordinator && m.is_alive()).map(|m| m.id).next();
        let view = MembershipView::new(self.current_epoch, members, coord, ts);
        self.views.push(view);
        self.events.push(MembershipEvent { ts, kind: MembershipEventKind::ViewChanged, member_id: 0, epoch: self.current_epoch });
    }

    pub fn set_coordinator(&mut self, id: u64, ts: u64) {
        for m in self.members.values_mut() { if m.role == MemberRole::Coordinator { m.role = MemberRole::Regular; } }
        if let Some(m) = self.members.get_mut(&id) { m.role = MemberRole::Coordinator; }
        self.events.push(MembershipEvent { ts, kind: MembershipEventKind::CoordinatorChanged, member_id: id, epoch: self.current_epoch });
    }

    pub fn recompute(&mut self) {
        self.stats.total_members = self.members.len();
        self.stats.active = self.members.values().filter(|m| m.status == MemberStatus::Active).count();
        self.stats.suspect = self.members.values().filter(|m| m.status == MemberStatus::Suspect).count();
        self.stats.faulty = self.members.values().filter(|m| m.status == MemberStatus::Faulty).count();
        self.stats.current_epoch = self.current_epoch;
    }

    pub fn member(&self, id: u64) -> Option<&MemberDesc> { self.members.get(&id) }
    pub fn current_view(&self) -> Option<&MembershipView> { self.views.last() }
    pub fn stats(&self) -> &MembershipStats { &self.stats }
    pub fn events(&self) -> &[MembershipEvent] { &self.events }
    pub fn active_members(&self) -> Vec<u64> { self.members.values().filter(|m| m.is_alive()).map(|m| m.id).collect() }
    pub fn epoch(&self) -> u64 { self.current_epoch }
}
