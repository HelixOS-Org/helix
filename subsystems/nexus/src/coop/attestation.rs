//! # Coop Attestation
//!
//! Process attestation protocol for cooperative trust verification:
//! - Remote attestation of process integrity
//! - Platform Configuration Register (PCR) simulation
//! - Quote generation and verification
//! - Endorsement key management
//! - Attestation identity delegation
//! - Nonce-challenge protocol

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Attestation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationState {
    /// Not yet attested
    Unattested,
    /// Challenge sent, awaiting response
    Challenged,
    /// Verified successfully
    Verified,
    /// Verification failed
    Failed,
    /// Attestation expired
    Expired,
}

/// PCR register
#[derive(Debug, Clone)]
pub struct PcrRegister {
    pub index: u8,
    pub value: [u8; 32],
    pub extend_count: u64,
    pub last_extend_ns: u64,
}

impl PcrRegister {
    pub fn new(index: u8) -> Self {
        Self {
            index,
            value: [0u8; 32],
            extend_count: 0,
            last_extend_ns: 0,
        }
    }

    /// Extend PCR: PCR_new = FNV-1a(PCR_old || measurement)
    pub fn extend(&mut self, measurement: &[u8], now_ns: u64) {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in self.value.iter().chain(measurement.iter()) {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        // Spread hash into 32 bytes using xorshift
        let mut state = hash;
        for chunk in self.value.chunks_mut(8) {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let bytes = state.to_le_bytes();
            for (i, b) in chunk.iter_mut().enumerate() {
                if i < bytes.len() {
                    *b = bytes[i];
                }
            }
        }
        self.extend_count += 1;
        self.last_extend_ns = now_ns;
    }
}

/// Attestation quote
#[derive(Debug, Clone)]
pub struct AttestationQuote {
    pub quote_id: u64,
    pub pcr_mask: u32,
    pub pcr_values: Vec<[u8; 32]>,
    pub nonce: u64,
    pub timestamp_ns: u64,
    /// FNV-1a signature of quote contents
    pub signature: u64,
}

impl AttestationQuote {
    pub fn compute_signature(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= self.quote_id;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.nonce;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.pcr_mask as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        for pcr_val in &self.pcr_values {
            for &b in pcr_val {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        hash
    }

    #[inline(always)]
    pub fn verify_signature(&self) -> bool {
        self.signature == self.compute_signature()
    }
}

/// Per-process attestation state
#[derive(Debug)]
pub struct ProcessAttestation {
    pub pid: u64,
    pub state: AttestationState,
    pub pcrs: Vec<PcrRegister>,
    pub last_quote: Option<AttestationQuote>,
    pub verification_count: u64,
    pub failure_count: u64,
    pub last_verified_ns: u64,
    pub expiry_ns: u64,
    /// Endorsement key (FNV-1a of process identity)
    pub endorsement_key: u64,
}

impl ProcessAttestation {
    pub fn new(pid: u64, num_pcrs: u8) -> Self {
        let mut pcrs = Vec::new();
        for i in 0..num_pcrs {
            pcrs.push(PcrRegister::new(i));
        }
        // Compute endorsement key from pid
        let mut ek: u64 = 0xcbf29ce484222325;
        ek ^= pid;
        ek = ek.wrapping_mul(0x100000001b3);
        Self {
            pid,
            state: AttestationState::Unattested,
            pcrs,
            last_quote: None,
            verification_count: 0,
            failure_count: 0,
            last_verified_ns: 0,
            expiry_ns: 0,
            endorsement_key: ek,
        }
    }

    #[inline]
    pub fn extend_pcr(&mut self, index: u8, measurement: &[u8], now_ns: u64) {
        if let Some(pcr) = self.pcrs.get_mut(index as usize) {
            pcr.extend(measurement, now_ns);
        }
    }

    pub fn generate_quote(&mut self, nonce: u64, pcr_mask: u32, now_ns: u64) -> AttestationQuote {
        let mut pcr_values = Vec::new();
        for (i, pcr) in self.pcrs.iter().enumerate() {
            if pcr_mask & (1 << i) != 0 {
                pcr_values.push(pcr.value);
            }
        }
        let mut quote = AttestationQuote {
            quote_id: now_ns ^ self.endorsement_key,
            pcr_mask,
            pcr_values,
            nonce,
            timestamp_ns: now_ns,
            signature: 0,
        };
        quote.signature = quote.compute_signature();
        self.last_quote = Some(quote.clone());
        quote
    }

    pub fn verify(&mut self, expected_pcrs: &[[u8; 32]], now_ns: u64) -> bool {
        let matches = self.pcrs.iter().zip(expected_pcrs.iter())
            .all(|(pcr, expected)| pcr.value == *expected);
        if matches {
            self.state = AttestationState::Verified;
            self.verification_count += 1;
            self.last_verified_ns = now_ns;
            self.expiry_ns = now_ns + 60_000_000_000; // 60s expiry
        } else {
            self.state = AttestationState::Failed;
            self.failure_count += 1;
        }
        matches
    }

    #[inline(always)]
    pub fn is_valid(&self, now_ns: u64) -> bool {
        self.state == AttestationState::Verified && now_ns < self.expiry_ns
    }

    #[inline]
    pub fn check_expiry(&mut self, now_ns: u64) {
        if self.state == AttestationState::Verified && now_ns >= self.expiry_ns {
            self.state = AttestationState::Expired;
        }
    }
}

/// Attestation protocol stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopAttestationStats {
    pub tracked_processes: usize,
    pub verified_count: usize,
    pub failed_count: usize,
    pub expired_count: usize,
    pub total_verifications: u64,
    pub total_failures: u64,
}

/// Coop Attestation Protocol
pub struct CoopAttestationProtocol {
    processes: BTreeMap<u64, ProcessAttestation>,
    stats: CoopAttestationStats,
    default_pcr_count: u8,
}

impl CoopAttestationProtocol {
    pub fn new(default_pcr_count: u8) -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: CoopAttestationStats::default(),
            default_pcr_count,
        }
    }

    #[inline(always)]
    pub fn register(&mut self, pid: u64) {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessAttestation::new(pid, self.default_pcr_count));
    }

    #[inline]
    pub fn extend_pcr(&mut self, pid: u64, index: u8, measurement: &[u8], now_ns: u64) {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.extend_pcr(index, measurement, now_ns);
        }
    }

    #[inline(always)]
    pub fn generate_quote(&mut self, pid: u64, nonce: u64, pcr_mask: u32, now_ns: u64) -> Option<AttestationQuote> {
        self.processes.get_mut(&pid).map(|p| p.generate_quote(nonce, pcr_mask, now_ns))
    }

    #[inline]
    pub fn verify(&mut self, pid: u64, expected: &[[u8; 32]], now_ns: u64) -> bool {
        if let Some(proc) = self.processes.get_mut(&pid) {
            let result = proc.verify(expected, now_ns);
            self.update_stats();
            result
        } else {
            false
        }
    }

    #[inline]
    pub fn tick(&mut self, now_ns: u64) {
        for proc in self.processes.values_mut() {
            proc.check_expiry(now_ns);
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.verified_count = self.processes.values()
            .filter(|p| p.state == AttestationState::Verified).count();
        self.stats.failed_count = self.processes.values()
            .filter(|p| p.state == AttestationState::Failed).count();
        self.stats.expired_count = self.processes.values()
            .filter(|p| p.state == AttestationState::Expired).count();
        self.stats.total_verifications = self.processes.values()
            .map(|p| p.verification_count).sum();
        self.stats.total_failures = self.processes.values()
            .map(|p| p.failure_count).sum();
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopAttestationStats {
        &self.stats
    }
}
