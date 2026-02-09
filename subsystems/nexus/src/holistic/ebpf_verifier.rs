// SPDX-License-Identifier: GPL-2.0
//! Holistic ebpf_verifier — eBPF program verification engine.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Verification result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyResult {
    Safe,
    Unsafe,
    OutOfBounds,
    InvalidAccess,
    InfiniteLoop,
    StackOverflow,
    TypeMismatch,
    UnreachableInsn,
}

/// Register state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegState {
    Uninitialized,
    Scalar,
    PtrToMap,
    PtrToStack,
    PtrToCtx,
    PtrToPacket,
    PtrToPacketEnd,
    Null,
}

/// Verifier instruction
#[derive(Debug)]
pub struct VerifierInsn {
    pub offset: u32,
    pub opcode: u8,
    pub dst_reg: u8,
    pub src_reg: u8,
    pub imm: i32,
    pub visited: bool,
}

/// Program verification
#[derive(Debug)]
pub struct ProgramVerification {
    pub prog_id: u64,
    pub insn_count: u32,
    pub visited_insns: u32,
    pub max_stack_depth: u32,
    pub helper_calls: u32,
    pub map_accesses: u32,
    pub branches: u32,
    pub result: VerifyResult,
    pub complexity: u64,
    pub verify_time_ns: u64,
}

impl ProgramVerification {
    pub fn new(id: u64, insns: u32) -> Self {
        Self { prog_id: id, insn_count: insns, visited_insns: 0, max_stack_depth: 0, helper_calls: 0, map_accesses: 0, branches: 0, result: VerifyResult::Safe, complexity: 0, verify_time_ns: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EbpfVerifierStats {
    pub total_verified: u64,
    pub safe_count: u64,
    pub rejected_count: u64,
    pub avg_complexity: f64,
    pub avg_insn_count: f64,
}

/// Main holistic eBPF verifier
pub struct HolisticEbpfVerifier {
    verifications: BTreeMap<u64, ProgramVerification>,
    total_verified: u64,
    total_safe: u64,
    total_rejected: u64,
    complexity_sum: u64,
    insn_sum: u64,
}

impl HolisticEbpfVerifier {
    pub fn new() -> Self {
        Self { verifications: BTreeMap::new(), total_verified: 0, total_safe: 0, total_rejected: 0, complexity_sum: 0, insn_sum: 0 }
    }

    #[inline]
    pub fn verify(&mut self, prog_id: u64, insn_count: u32) -> u64 {
        let mut v = ProgramVerification::new(prog_id, insn_count);
        v.complexity = insn_count as u64 * 2;
        self.total_verified += 1;
        self.complexity_sum += v.complexity;
        self.insn_sum += insn_count as u64;
        if insn_count > 100000 { v.result = VerifyResult::InfiniteLoop; self.total_rejected += 1; }
        else { v.result = VerifyResult::Safe; self.total_safe += 1; }
        self.verifications.insert(prog_id, v);
        prog_id
    }

    #[inline(always)]
    pub fn reject(&mut self, prog_id: u64, reason: VerifyResult) {
        if let Some(v) = self.verifications.get_mut(&prog_id) { v.result = reason; }
    }

    #[inline]
    pub fn stats(&self) -> EbpfVerifierStats {
        let avg_c = if self.total_verified == 0 { 0.0 } else { self.complexity_sum as f64 / self.total_verified as f64 };
        let avg_i = if self.total_verified == 0 { 0.0 } else { self.insn_sum as f64 / self.total_verified as f64 };
        EbpfVerifierStats { total_verified: self.total_verified, safe_count: self.total_safe, rejected_count: self.total_rejected, avg_complexity: avg_c, avg_insn_count: avg_i }
    }
}
