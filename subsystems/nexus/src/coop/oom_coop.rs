// SPDX-License-Identifier: MIT
//! # Cooperative OOM Protocol
//!
//! Multi-process OOM negotiation:
//! - Cooperative memory shedding before OOM kill
//! - Process group sacrifice negotiation
//! - Graceful degradation protocol
//! - Memory donation between cooperating processes
//! - Kill notification and recovery coordination

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShedStrategy { DropCaches, CompressHeap, ReleaseBuffers, Downgrade, None }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OomPhase { Normal, Warning, Critical, Killing, Recovering }

#[derive(Debug, Clone)]
pub struct MemDonation {
    pub donor_pid: u64,
    pub recipient_pid: u64,
    pub pages: u64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct ShedResponse {
    pub pid: u64,
    pub strategy: ShedStrategy,
    pub pages_freed: u64,
    pub response_time_ns: u64,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct GroupSacrifice {
    pub group_id: u64,
    pub volunteer_pid: Option<u64>,
    pub victim_pid: u64,
    pub pages_to_recover: u64,
    pub negotiation_rounds: u32,
}

#[derive(Debug, Clone, Default)]
pub struct OomCoopStats {
    pub shed_requests_sent: u64,
    pub shed_pages_freed: u64,
    pub donations_made: u64,
    pub donations_pages: u64,
    pub negotiations_completed: u64,
    pub volunteers_found: u64,
    pub forced_kills: u64,
    pub graceful_exits: u64,
}

pub struct OomCoopManager {
    /// group_id → member pids with their shedding capability
    groups: BTreeMap<u64, Vec<(u64, u64)>>, // (pid, max_shed_pages)
    /// Active donations
    donations: Vec<MemDonation>,
    /// Pending shed responses
    shed_responses: BTreeMap<u64, ShedResponse>,
    phase: OomPhase,
    stats: OomCoopStats,
}

impl OomCoopManager {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            donations: Vec::new(),
            shed_responses: BTreeMap::new(),
            phase: OomPhase::Normal,
            stats: OomCoopStats::default(),
        }
    }

    pub fn register_group(&mut self, group_id: u64, members: Vec<(u64, u64)>) {
        self.groups.insert(group_id, members);
    }

    pub fn set_phase(&mut self, phase: OomPhase) { self.phase = phase; }

    /// Request cooperative memory shedding from a group
    pub fn request_shed(&mut self, group_id: u64, needed_pages: u64) -> Vec<u64> {
        let members = match self.groups.get(&group_id) {
            Some(m) => m,
            None => return Vec::new(),
        };

        let mut targets = Vec::new();
        let mut remaining = needed_pages;

        // Ask members with largest shed capability first
        let mut sorted: Vec<_> = members.clone();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        for (pid, max_shed) in sorted {
            if remaining == 0 { break; }
            let shed_amount = max_shed.min(remaining);
            targets.push(pid);
            remaining = remaining.saturating_sub(shed_amount);
            self.stats.shed_requests_sent += 1;
        }
        targets
    }

    /// Process a shed response from a member
    pub fn process_shed_response(&mut self, response: ShedResponse) {
        if response.success {
            self.stats.shed_pages_freed += response.pages_freed;
        }
        self.shed_responses.insert(response.pid, response);
    }

    /// A process donates memory pages to another
    pub fn donate_memory(&mut self, donor: u64, recipient: u64, pages: u64, now: u64) {
        self.donations.push(MemDonation {
            donor_pid: donor, recipient_pid: recipient, pages, timestamp: now,
        });
        self.stats.donations_made += 1;
        self.stats.donations_pages += pages;
    }

    /// Negotiate sacrifice within a group
    pub fn negotiate_sacrifice(
        &mut self, group_id: u64, needed_pages: u64,
    ) -> Option<GroupSacrifice> {
        let members = match self.groups.get(&group_id) {
            Some(m) if !m.is_empty() => m,
            _ => return None,
        };

        // Find volunteer (process with most shed capacity willing to die)
        let mut best_volunteer: Option<(u64, u64)> = None;
        for &(pid, max_shed) in members {
            if max_shed >= needed_pages {
                if best_volunteer.is_none() || max_shed > best_volunteer.unwrap().1 {
                    best_volunteer = Some((pid, max_shed));
                }
            }
        }

        let sacrifice = if let Some((vol_pid, _)) = best_volunteer {
            self.stats.volunteers_found += 1;
            GroupSacrifice {
                group_id,
                volunteer_pid: Some(vol_pid),
                victim_pid: vol_pid,
                pages_to_recover: needed_pages,
                negotiation_rounds: 1,
            }
        } else {
            // No volunteer: pick the least essential (smallest shed = smallest)
            let victim = members.iter().min_by_key(|(_, s)| *s)
                .map(|(p, _)| *p)
                .unwrap_or(0);
            self.stats.forced_kills += 1;
            GroupSacrifice {
                group_id,
                volunteer_pid: None,
                victim_pid: victim,
                pages_to_recover: needed_pages,
                negotiation_rounds: 2,
            }
        };

        self.stats.negotiations_completed += 1;
        Some(sacrifice)
    }

    /// Notify group of a kill for recovery coordination
    pub fn notify_kill(&mut self, group_id: u64, killed_pid: u64) {
        if let Some(members) = self.groups.get_mut(&group_id) {
            members.retain(|&(p, _)| p != killed_pid);
        }
    }

    pub fn phase(&self) -> OomPhase { self.phase }
    pub fn stats(&self) -> &OomCoopStats { &self.stats }
}
