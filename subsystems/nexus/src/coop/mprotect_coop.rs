// SPDX-License-Identifier: MIT
//! # Cooperative Memory Protection
//!
//! Multi-process memory protection coordination:
//! - W^X policy negotiation for shared regions
//! - Guard page sharing between cooperating processes
//! - Sandboxing boundary enforcement
//! - Protection key (PKU) domain management
//! - Permission escalation audit across process groups

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtDomain {
    Kernel,
    Trusted,
    Normal,
    Sandbox,
    Untrusted,
}

impl ProtDomain {
    #[inline]
    pub fn trust_level(&self) -> u32 {
        match self {
            Self::Kernel => 4,
            Self::Trusted => 3,
            Self::Normal => 2,
            Self::Sandbox => 1,
            Self::Untrusted => 0,
        }
    }

    #[inline(always)]
    pub fn can_share_with(&self, other: &Self) -> bool {
        (self.trust_level() as i32 - other.trust_level() as i32).unsigned_abs() <= 1
    }
}

#[derive(Debug, Clone)]
pub struct ProtDomainEntry {
    pub pid: u64,
    pub domain: ProtDomain,
    pub pku_key: Option<u32>,
    pub shared_guard_pages: u64,
    pub wx_violations: u64,
    pub escalation_attempts: u64,
}

#[derive(Debug, Clone)]
pub struct SharedGuardRegion {
    pub region_id: u64,
    pub base_addr: u64,
    pub size: u64,
    pub participants: Vec<u64>,
    pub guard_density: f64, // guards per 4K page
}

#[derive(Debug, Clone)]
pub struct EscalationEvent {
    pub pid: u64,
    pub from_domain: ProtDomain,
    pub requested_domain: ProtDomain,
    pub timestamp: u64,
    pub granted: bool,
    pub reason: u64, // encoded reason
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MprotectCoopStats {
    pub domain_assignments: u64,
    pub shared_guards_created: u64,
    pub wx_violations_total: u64,
    pub escalation_denied: u64,
    pub escalation_granted: u64,
    pub cross_domain_shares: u64,
}

pub struct MprotectCoopManager {
    domains: BTreeMap<u64, ProtDomainEntry>,
    guards: BTreeMap<u64, SharedGuardRegion>,
    escalation_log: Vec<EscalationEvent>,
    next_guard_id: u64,
    stats: MprotectCoopStats,
}

impl MprotectCoopManager {
    pub fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            guards: BTreeMap::new(),
            escalation_log: Vec::new(),
            next_guard_id: 1,
            stats: MprotectCoopStats::default(),
        }
    }

    #[inline]
    pub fn assign_domain(&mut self, pid: u64, domain: ProtDomain, pku_key: Option<u32>) {
        self.domains.insert(pid, ProtDomainEntry {
            pid,
            domain,
            pku_key,
            shared_guard_pages: 0,
            wx_violations: 0,
            escalation_attempts: 0,
        });
        self.stats.domain_assignments += 1;
    }

    /// Check if two processes can share a memory region
    #[inline]
    pub fn can_share(&self, pid_a: u64, pid_b: u64) -> bool {
        let da = match self.domains.get(&pid_a) {
            Some(d) => d,
            None => return false,
        };
        let db = match self.domains.get(&pid_b) {
            Some(d) => d,
            None => return false,
        };
        da.domain.can_share_with(&db.domain)
    }

    /// Create a shared guard page region between cooperating processes
    pub fn create_shared_guard(
        &mut self,
        base: u64,
        size: u64,
        participants: Vec<u64>,
        density: f64,
    ) -> u64 {
        let id = self.next_guard_id;
        self.next_guard_id += 1;
        let guard_pages = (size / 4096) as f64 * density;
        for &pid in &participants {
            if let Some(d) = self.domains.get_mut(&pid) {
                d.shared_guard_pages += guard_pages as u64;
            }
        }
        self.guards.insert(id, SharedGuardRegion {
            region_id: id,
            base_addr: base,
            size,
            participants,
            guard_density: density,
        });
        self.stats.shared_guards_created += 1;
        id
    }

    /// Record a W^X violation
    #[inline]
    pub fn record_wx_violation(&mut self, pid: u64) {
        if let Some(d) = self.domains.get_mut(&pid) {
            d.wx_violations += 1;
        }
        self.stats.wx_violations_total += 1;
    }

    /// Request domain escalation
    pub fn request_escalation(
        &mut self,
        pid: u64,
        target: ProtDomain,
        now: u64,
        reason: u64,
    ) -> bool {
        let current = match self.domains.get_mut(&pid) {
            Some(d) => {
                d.escalation_attempts += 1;
                d.domain
            },
            None => return false,
        };

        let granted =
            target.trust_level() <= current.trust_level() + 1 && target != ProtDomain::Kernel;

        self.escalation_log.push(EscalationEvent {
            pid,
            from_domain: current,
            requested_domain: target,
            timestamp: now,
            granted,
            reason,
        });
        if self.escalation_log.len() > 512 {
            self.escalation_log.drain(..256);
        }

        if granted {
            if let Some(d) = self.domains.get_mut(&pid) {
                d.domain = target;
            }
            self.stats.escalation_granted += 1;
        } else {
            self.stats.escalation_denied += 1;
        }
        granted
    }

    #[inline(always)]
    pub fn domain(&self, pid: u64) -> Option<&ProtDomainEntry> {
        self.domains.get(&pid)
    }
    #[inline(always)]
    pub fn stats(&self) -> &MprotectCoopStats {
        &self.stats
    }
}
