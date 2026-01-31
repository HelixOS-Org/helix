//! BPF Verifier
//!
//! BPF program verification.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{BpfInsn, BpfProgType};

/// Verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Success
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Error instruction (if failed)
    pub error_insn: Option<u32>,
    /// Instructions analyzed
    pub insns_analyzed: u32,
    /// States analyzed
    pub states_analyzed: u32,
    /// Peak states
    pub peak_states: u32,
    /// Verification time (ns)
    pub verification_time_ns: u64,
    /// Log output
    pub log: Vec<String>,
}

impl VerificationResult {
    /// Create success result
    pub fn success(insns_analyzed: u32, states_analyzed: u32, time_ns: u64) -> Self {
        Self {
            success: true,
            error: None,
            error_insn: None,
            insns_analyzed,
            states_analyzed,
            peak_states: states_analyzed,
            verification_time_ns: time_ns,
            log: Vec::new(),
        }
    }

    /// Create failure result
    pub fn failure(error: String, error_insn: u32) -> Self {
        Self {
            success: false,
            error: Some(error),
            error_insn: Some(error_insn),
            insns_analyzed: 0,
            states_analyzed: 0,
            peak_states: 0,
            verification_time_ns: 0,
            log: Vec::new(),
        }
    }
}

/// BPF verifier
pub struct BpfVerifier {
    /// Max instructions
    max_insns: u32,
    /// Max states
    max_states: u32,
    /// Verification log level
    log_level: u32,
    /// Total verifications
    total_verifications: AtomicU64,
    /// Failed verifications
    failed_verifications: AtomicU64,
    /// Total verification time
    total_verification_time: AtomicU64,
}

impl BpfVerifier {
    /// Create new verifier
    pub fn new() -> Self {
        Self {
            max_insns: 1_000_000,
            max_states: 64 * 1024,
            log_level: 1,
            total_verifications: AtomicU64::new(0),
            failed_verifications: AtomicU64::new(0),
            total_verification_time: AtomicU64::new(0),
        }
    }

    /// Verify program
    pub fn verify(&self, insns: &[BpfInsn], _prog_type: BpfProgType) -> VerificationResult {
        self.total_verifications.fetch_add(1, Ordering::Relaxed);

        // Basic verification checks
        if insns.is_empty() {
            self.failed_verifications.fetch_add(1, Ordering::Relaxed);
            return VerificationResult::failure(String::from("Empty program"), 0);
        }

        if insns.len() > self.max_insns as usize {
            self.failed_verifications.fetch_add(1, Ordering::Relaxed);
            return VerificationResult::failure(String::from("Program too large"), 0);
        }

        // Check for exit instruction
        let has_exit = insns.iter().any(|i| i.is_exit());
        if !has_exit {
            self.failed_verifications.fetch_add(1, Ordering::Relaxed);
            return VerificationResult::failure(
                String::from("No exit instruction"),
                insns.len() as u32 - 1,
            );
        }

        // Simulate verification
        let insns_analyzed = insns.len() as u32;
        let states_analyzed = insns_analyzed * 2;
        let time_ns = 1000 * insns_analyzed as u64;

        self.total_verification_time
            .fetch_add(time_ns, Ordering::Relaxed);

        VerificationResult::success(insns_analyzed, states_analyzed, time_ns)
    }

    /// Get stats
    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.total_verifications.load(Ordering::Relaxed),
            self.failed_verifications.load(Ordering::Relaxed),
            self.total_verification_time.load(Ordering::Relaxed),
        )
    }

    /// Get max instructions
    pub fn max_insns(&self) -> u32 {
        self.max_insns
    }

    /// Set max instructions
    pub fn set_max_insns(&mut self, max: u32) {
        self.max_insns = max;
    }

    /// Get log level
    pub fn log_level(&self) -> u32 {
        self.log_level
    }

    /// Set log level
    pub fn set_log_level(&mut self, level: u32) {
        self.log_level = level;
    }
}

impl Default for BpfVerifier {
    fn default() -> Self {
        Self::new()
    }
}
