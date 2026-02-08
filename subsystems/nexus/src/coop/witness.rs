//! # Cooperative Witness System
//!
//! Third-party witness verification for cooperative agreements:
//! - Witness registration and selection
//! - Agreement attestation
//! - Dispute resolution evidence
//! - Multi-witness consensus
//! - Tamper-proof witness logs

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// WITNESS TYPES
// ============================================================================

/// Witness role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessRole {
    /// Primary witness
    Primary,
    /// Backup witness
    Backup,
    /// Audit witness (read-only)
    Audit,
}

/// Witness status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WitnessStatus {
    /// Available
    Available,
    /// Active (witnessing an agreement)
    Active,
    /// Busy (too many agreements)
    Busy,
    /// Suspended (unreliable)
    Suspended,
}

/// Attestation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationType {
    /// Agreement creation
    Creation,
    /// Fulfillment event
    Fulfillment,
    /// Violation event
    Violation,
    /// Completion
    Completion,
    /// Dispute
    Dispute,
}

// ============================================================================
// WITNESS
// ============================================================================

/// A witness process
#[derive(Debug, Clone)]
pub struct Witness {
    /// Process ID
    pub pid: u64,
    /// Role
    pub role: WitnessRole,
    /// Status
    pub status: WitnessStatus,
    /// Agreements witnessed
    pub agreements_witnessed: u64,
    /// Attestations made
    pub attestations_made: u64,
    /// Trust score (0.0-1.0)
    pub trust_score: f64,
    /// Active agreements
    pub active_agreements: Vec<u64>,
    /// Max concurrent
    pub max_concurrent: usize,
}

impl Witness {
    pub fn new(pid: u64, role: WitnessRole) -> Self {
        Self {
            pid,
            role,
            status: WitnessStatus::Available,
            agreements_witnessed: 0,
            attestations_made: 0,
            trust_score: 1.0,
            active_agreements: Vec::new(),
            max_concurrent: 8,
        }
    }

    /// Can accept new agreement
    pub fn can_accept(&self) -> bool {
        self.status == WitnessStatus::Available
            && self.active_agreements.len() < self.max_concurrent
    }

    /// Assign agreement
    pub fn assign(&mut self, agreement_id: u64) {
        self.active_agreements.push(agreement_id);
        self.agreements_witnessed += 1;
        if self.active_agreements.len() >= self.max_concurrent {
            self.status = WitnessStatus::Busy;
        } else {
            self.status = WitnessStatus::Active;
        }
    }

    /// Complete agreement
    pub fn complete(&mut self, agreement_id: u64) {
        self.active_agreements.retain(|&a| a != agreement_id);
        if self.active_agreements.is_empty() {
            self.status = WitnessStatus::Available;
        } else {
            self.status = WitnessStatus::Active;
        }
    }
}

// ============================================================================
// ATTESTATION
// ============================================================================

/// Witness attestation
#[derive(Debug, Clone)]
pub struct Attestation {
    /// Attestation ID
    pub id: u64,
    /// Agreement being attested
    pub agreement_id: u64,
    /// Witness
    pub witness_pid: u64,
    /// Type
    pub attestation_type: AttestationType,
    /// Timestamp
    pub timestamp: u64,
    /// Evidence hash
    pub evidence_hash: u64,
    /// Verified
    pub verified: bool,
    /// Notes value
    pub detail_code: u32,
}

impl Attestation {
    pub fn new(
        id: u64,
        agreement_id: u64,
        witness_pid: u64,
        attestation_type: AttestationType,
        now: u64,
    ) -> Self {
        Self {
            id,
            agreement_id,
            witness_pid,
            attestation_type,
            timestamp: now,
            evidence_hash: 0,
            verified: false,
            detail_code: 0,
        }
    }

    /// Compute evidence hash
    pub fn compute_hash(&mut self) {
        let mut h: u64 = 0xcbf29ce484222325;
        h ^= self.agreement_id;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.witness_pid;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.attestation_type as u64;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.timestamp;
        h = h.wrapping_mul(0x100000001b3);
        self.evidence_hash = h;
    }

    /// Verify hash integrity
    pub fn verify(&self) -> bool {
        let mut h: u64 = 0xcbf29ce484222325;
        h ^= self.agreement_id;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.witness_pid;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.attestation_type as u64;
        h = h.wrapping_mul(0x100000001b3);
        h ^= self.timestamp;
        h = h.wrapping_mul(0x100000001b3);
        h == self.evidence_hash
    }
}

// ============================================================================
// AGREEMENT RECORD
// ============================================================================

/// Witnessed agreement record
#[derive(Debug, Clone)]
pub struct AgreementRecord {
    /// Agreement ID
    pub id: u64,
    /// Parties
    pub parties: Vec<u64>,
    /// Assigned witnesses
    pub witnesses: Vec<u64>,
    /// Attestations
    pub attestations: Vec<u64>,
    /// Created at
    pub created_at: u64,
    /// Completed at
    pub completed_at: Option<u64>,
    /// Dispute pending
    pub dispute_pending: bool,
}

impl AgreementRecord {
    pub fn new(id: u64, parties: Vec<u64>, now: u64) -> Self {
        Self {
            id,
            parties,
            witnesses: Vec::new(),
            attestations: Vec::new(),
            created_at: now,
            completed_at: None,
            dispute_pending: false,
        }
    }

    /// Witness count
    pub fn witness_count(&self) -> usize {
        self.witnesses.len()
    }

    /// Attestation count
    pub fn attestation_count(&self) -> usize {
        self.attestations.len()
    }
}

// ============================================================================
// WITNESS MANAGER
// ============================================================================

/// Witness manager stats
#[derive(Debug, Clone, Default)]
pub struct CoopWitnessStats {
    /// Registered witnesses
    pub witness_count: usize,
    /// Available witnesses
    pub available_count: usize,
    /// Active agreements
    pub active_agreements: usize,
    /// Total attestations
    pub total_attestations: u64,
    /// Disputed agreements
    pub disputes: u64,
}

/// Cooperative witness manager
pub struct CoopWitnessManager {
    /// Witnesses
    witnesses: BTreeMap<u64, Witness>,
    /// Agreement records
    agreements: BTreeMap<u64, AgreementRecord>,
    /// Attestations
    attestations: BTreeMap<u64, Attestation>,
    /// Next IDs
    next_agreement_id: u64,
    next_attestation_id: u64,
    /// Stats
    stats: CoopWitnessStats,
}

impl CoopWitnessManager {
    pub fn new() -> Self {
        Self {
            witnesses: BTreeMap::new(),
            agreements: BTreeMap::new(),
            attestations: BTreeMap::new(),
            next_agreement_id: 1,
            next_attestation_id: 1,
            stats: CoopWitnessStats::default(),
        }
    }

    /// Register witness
    pub fn register_witness(&mut self, pid: u64, role: WitnessRole) {
        self.witnesses.insert(pid, Witness::new(pid, role));
        self.update_counts();
    }

    /// Create agreement with auto-assigned witnesses
    pub fn create_agreement(&mut self, parties: Vec<u64>, now: u64) -> u64 {
        let id = self.next_agreement_id;
        self.next_agreement_id += 1;
        let mut record = AgreementRecord::new(id, parties, now);

        // Assign available witnesses
        let available: Vec<u64> = self
            .witnesses
            .values()
            .filter(|w| w.can_accept())
            .map(|w| w.pid)
            .collect();

        let to_assign = available.len().min(3); // up to 3 witnesses
        for &witness_pid in available.iter().take(to_assign) {
            if let Some(witness) = self.witnesses.get_mut(&witness_pid) {
                witness.assign(id);
                record.witnesses.push(witness_pid);
            }
        }

        self.agreements.insert(id, record);
        self.update_counts();
        id
    }

    /// Create attestation
    pub fn attest(
        &mut self,
        witness_pid: u64,
        agreement_id: u64,
        attestation_type: AttestationType,
        now: u64,
    ) -> Option<u64> {
        // Verify witness is assigned
        let is_assigned = self
            .agreements
            .get(&agreement_id)
            .map(|a| a.witnesses.contains(&witness_pid))
            .unwrap_or(false);

        if !is_assigned {
            return None;
        }

        let id = self.next_attestation_id;
        self.next_attestation_id += 1;

        let mut att = Attestation::new(id, agreement_id, witness_pid, attestation_type, now);
        att.compute_hash();
        att.verified = true;
        self.attestations.insert(id, att);

        if let Some(agreement) = self.agreements.get_mut(&agreement_id) {
            agreement.attestations.push(id);
        }

        if let Some(witness) = self.witnesses.get_mut(&witness_pid) {
            witness.attestations_made += 1;
        }

        self.stats.total_attestations += 1;
        Some(id)
    }

    /// Complete agreement
    pub fn complete_agreement(&mut self, agreement_id: u64, now: u64) {
        if let Some(agreement) = self.agreements.get_mut(&agreement_id) {
            agreement.completed_at = Some(now);
            for &witness_pid in &agreement.witnesses.clone() {
                if let Some(witness) = self.witnesses.get_mut(&witness_pid) {
                    witness.complete(agreement_id);
                }
            }
        }
        self.update_counts();
    }

    /// File dispute
    pub fn file_dispute(&mut self, agreement_id: u64) {
        if let Some(agreement) = self.agreements.get_mut(&agreement_id) {
            agreement.dispute_pending = true;
            self.stats.disputes += 1;
        }
    }

    fn update_counts(&mut self) {
        self.stats.witness_count = self.witnesses.len();
        self.stats.available_count = self
            .witnesses
            .values()
            .filter(|w| w.can_accept())
            .count();
        self.stats.active_agreements = self
            .agreements
            .values()
            .filter(|a| a.completed_at.is_none())
            .count();
    }

    /// Get witness
    pub fn witness(&self, pid: u64) -> Option<&Witness> {
        self.witnesses.get(&pid)
    }

    /// Get agreement
    pub fn agreement(&self, id: u64) -> Option<&AgreementRecord> {
        self.agreements.get(&id)
    }

    /// Stats
    pub fn stats(&self) -> &CoopWitnessStats {
        &self.stats
    }
}
