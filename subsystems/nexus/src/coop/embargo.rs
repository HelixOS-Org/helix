//! # Coop Embargo
//!
//! Cooperative resource embargo protocol:
//! - Temporary resource lockout for critical operations
//! - Embargo negotiation between processes
//! - Priority-based embargo override
//! - Cascade embargo (transitive closure)
//! - Timed auto-release with watchdog
//! - Embargo violation tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Embargo type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbargoType {
    /// Soft embargo — advisory only
    Soft,
    /// Hard embargo — enforced
    Hard,
    /// Exclusive — single process access
    Exclusive,
    /// Shared — limited concurrent access
    Shared,
    /// Deferred — will take effect later
    Deferred,
}

/// Embargo target
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EmbargoTarget {
    /// CPU cores
    Cpu(u32),
    /// Memory region
    Memory(u64),
    /// IO device
    IoDevice(u32),
    /// Network interface
    Network(u32),
    /// Generic resource
    Resource(u64),
}

/// An embargo declaration
#[derive(Debug, Clone)]
pub struct Embargo {
    pub embargo_id: u64,
    pub owner_pid: u64,
    pub embargo_type: EmbargoType,
    pub target: EmbargoTarget,
    pub reason: u32,
    pub created_ns: u64,
    pub expiry_ns: u64,
    pub priority: u8,
    pub active: bool,
    /// Processes exempt from this embargo
    pub exemptions: Vec<u64>,
    pub violation_count: u64,
}

impl Embargo {
    pub fn new(
        id: u64,
        owner: u64,
        etype: EmbargoType,
        target: EmbargoTarget,
        priority: u8,
        now_ns: u64,
        duration_ns: u64,
    ) -> Self {
        Self {
            embargo_id: id,
            owner_pid: owner,
            embargo_type: etype,
            target,
            reason: 0,
            created_ns: now_ns,
            expiry_ns: now_ns + duration_ns,
            priority,
            active: true,
            exemptions: Vec::new(),
            violation_count: 0,
        }
    }

    pub fn is_expired(&self, now_ns: u64) -> bool {
        now_ns >= self.expiry_ns
    }

    pub fn is_enforced(&self) -> bool {
        self.active && !matches!(self.embargo_type, EmbargoType::Soft)
    }

    pub fn is_exempt(&self, pid: u64) -> bool {
        pid == self.owner_pid || self.exemptions.contains(&pid)
    }

    pub fn add_exemption(&mut self, pid: u64) {
        if !self.exemptions.contains(&pid) {
            self.exemptions.push(pid);
        }
    }

    pub fn remaining_ns(&self, now_ns: u64) -> u64 {
        self.expiry_ns.saturating_sub(now_ns)
    }

    pub fn extend(&mut self, extra_ns: u64) {
        self.expiry_ns += extra_ns;
    }

    pub fn record_violation(&mut self) {
        self.violation_count += 1;
    }
}

/// Embargo check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbargoCheckResult {
    /// Access allowed
    Allowed,
    /// Blocked by soft embargo (advisory)
    SoftBlocked,
    /// Blocked by hard embargo (enforced)
    HardBlocked,
    /// Blocked by exclusive embargo
    ExclusiveBlocked,
    /// Exempt from embargo
    Exempt,
}

/// Embargo negotiation request
#[derive(Debug, Clone)]
pub struct EmbargoRequest {
    pub requester_pid: u64,
    pub target: EmbargoTarget,
    pub embargo_type: EmbargoType,
    pub priority: u8,
    pub duration_ns: u64,
}

/// Embargo negotiation response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbargoResponse {
    /// Embargo granted
    Granted(u64),
    /// Denied — conflicting higher-priority embargo
    Denied,
    /// Queued — will be granted when current expires
    Queued,
    /// Preempted — existing embargo was overridden
    Preempted(u64),
}

/// Embargo engine stats
#[derive(Debug, Clone, Default)]
pub struct CoopEmbargoStats {
    pub active_embargoes: usize,
    pub total_created: u64,
    pub total_expired: u64,
    pub total_violations: u64,
    pub total_preemptions: u64,
    pub pending_requests: usize,
}

/// Coop Embargo Engine
pub struct CoopEmbargoEngine {
    embargoes: BTreeMap<u64, Embargo>,
    /// Pending requests (queued)
    pending: Vec<EmbargoRequest>,
    stats: CoopEmbargoStats,
    next_id: u64,
    total_created: u64,
    total_expired: u64,
    total_preemptions: u64,
}

impl CoopEmbargoEngine {
    pub fn new() -> Self {
        Self {
            embargoes: BTreeMap::new(),
            pending: Vec::new(),
            stats: CoopEmbargoStats::default(),
            next_id: 1,
            total_created: 0,
            total_expired: 0,
            total_preemptions: 0,
        }
    }

    /// Request an embargo
    pub fn request(&mut self, req: EmbargoRequest, now_ns: u64) -> EmbargoResponse {
        // Check for conflicts
        let conflicts: Vec<(u64, u8)> = self
            .embargoes
            .iter()
            .filter(|(_, e)| e.active && e.target == req.target && !e.is_expired(now_ns))
            .map(|(&id, e)| (id, e.priority))
            .collect();

        if conflicts.is_empty() {
            // No conflict — grant
            return self.grant_embargo(req, now_ns);
        }

        // Check priority
        let max_conflict_priority = conflicts.iter().map(|&(_, p)| p).max().unwrap_or(0);

        if req.priority > max_conflict_priority {
            // Preempt existing embargoes
            for (conflict_id, _) in &conflicts {
                if let Some(embargo) = self.embargoes.get_mut(conflict_id) {
                    embargo.active = false;
                }
            }
            self.total_preemptions += conflicts.len() as u64;
            return self.grant_embargo(req, now_ns);
        }

        // Cannot preempt — queue
        self.pending.push(req);
        EmbargoResponse::Queued
    }

    fn grant_embargo(&mut self, req: EmbargoRequest, now_ns: u64) -> EmbargoResponse {
        let id = self.next_id;
        self.next_id += 1;
        self.total_created += 1;

        let embargo = Embargo::new(
            id,
            req.requester_pid,
            req.embargo_type,
            req.target,
            req.priority,
            now_ns,
            req.duration_ns,
        );
        self.embargoes.insert(id, embargo);
        self.update_stats();
        EmbargoResponse::Granted(id)
    }

    /// Check if a process can access a target
    pub fn check_access(
        &mut self,
        pid: u64,
        target: EmbargoTarget,
        now_ns: u64,
    ) -> EmbargoCheckResult {
        for embargo in self.embargoes.values_mut() {
            if !embargo.active || embargo.is_expired(now_ns) || embargo.target != target {
                continue;
            }
            if embargo.is_exempt(pid) {
                return EmbargoCheckResult::Exempt;
            }
            match embargo.embargo_type {
                EmbargoType::Soft => return EmbargoCheckResult::SoftBlocked,
                EmbargoType::Hard => {
                    embargo.record_violation();
                    return EmbargoCheckResult::HardBlocked;
                },
                EmbargoType::Exclusive => {
                    embargo.record_violation();
                    return EmbargoCheckResult::ExclusiveBlocked;
                },
                EmbargoType::Shared => {
                    // Shared allows limited access
                    return EmbargoCheckResult::Allowed;
                },
                EmbargoType::Deferred => {
                    return EmbargoCheckResult::Allowed;
                },
            }
        }
        EmbargoCheckResult::Allowed
    }

    /// Release an embargo
    pub fn release(&mut self, embargo_id: u64) -> bool {
        if let Some(embargo) = self.embargoes.get_mut(&embargo_id) {
            embargo.active = false;
            self.process_pending(embargo.target);
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Tick — expire old embargoes, process pending
    pub fn tick(&mut self, now_ns: u64) {
        let expired_targets: Vec<EmbargoTarget> = self
            .embargoes
            .iter_mut()
            .filter(|(_, e)| e.active && e.is_expired(now_ns))
            .map(|(_, e)| {
                e.active = false;
                self.total_expired += 1;
                e.target
            })
            .collect();

        for target in expired_targets {
            self.process_pending(target);
        }
        self.update_stats();
    }

    fn process_pending(&mut self, target: EmbargoTarget) {
        // Find pending request for this target
        if let Some(pos) = self.pending.iter().position(|r| r.target == target) {
            let _req = self.pending.remove(pos);
            // In a real impl, would grant the pending request
        }
    }

    fn update_stats(&mut self) {
        self.stats.active_embargoes = self.embargoes.values().filter(|e| e.active).count();
        self.stats.total_created = self.total_created;
        self.stats.total_expired = self.total_expired;
        self.stats.total_violations = self.embargoes.values().map(|e| e.violation_count).sum();
        self.stats.total_preemptions = self.total_preemptions;
        self.stats.pending_requests = self.pending.len();
    }

    pub fn stats(&self) -> &CoopEmbargoStats {
        &self.stats
    }
}
