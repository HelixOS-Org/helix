// SPDX-License-Identifier: GPL-2.0
//! Apps exec shield — executable memory protection, ASLR enforcement, and exploit mitigation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Mitigation feature flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MitigationFlags(pub u64);

impl MitigationFlags {
    pub const ASLR: Self = Self(1 << 0);
    pub const NX_STACK: Self = Self(1 << 1);
    pub const NX_HEAP: Self = Self(1 << 2);
    pub const STACK_CANARY: Self = Self(1 << 3);
    pub const FORTIFY_SOURCE: Self = Self(1 << 4);
    pub const RELRO: Self = Self(1 << 5);
    pub const PIE: Self = Self(1 << 6);
    pub const CFI: Self = Self(1 << 7);
    pub const SHADOW_STACK: Self = Self(1 << 8);
    pub const IBT: Self = Self(1 << 9);
    pub const CET: Self = Self(1 << 10);
    pub const KASLR: Self = Self(1 << 11);

    pub fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn combine(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn count_enabled(&self) -> u32 {
        let mut n = self.0;
        let mut count = 0u32;
        while n != 0 {
            count += 1;
            n &= n - 1;
        }
        count
    }

    pub fn security_score(&self) -> f64 {
        // Each mitigation contributes to overall security score
        let max_mitigations = 12.0;
        self.count_enabled() as f64 / max_mitigations
    }
}

/// Violation type when protection is breached
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    /// Attempt to execute from non-executable region
    NxViolation,
    /// Stack buffer overflow detected
    StackSmash,
    /// Heap corruption detected
    HeapCorrupt,
    /// Return-oriented programming detected
    RopAttempt,
    /// Jump-oriented programming detected
    JopAttempt,
    /// Invalid control flow transfer
    CfiViolation,
    /// Shadow stack mismatch
    ShadowStackMismatch,
    /// Write to read-only region
    WriteToReadonly,
    /// ASLR bypass attempt
    AslrBypass,
}

/// A security violation record
#[derive(Debug, Clone)]
pub struct ViolationRecord {
    pub pid: u64,
    pub tid: u64,
    pub violation: ViolationType,
    pub address: u64,
    pub instruction_ptr: u64,
    pub stack_ptr: u64,
    pub timestamp_ns: u64,
    pub killed: bool,
}

/// Per-process exec shield state
#[derive(Debug)]
pub struct ProcessShieldState {
    pub pid: u64,
    pub binary_name: String,
    pub mitigations: MitigationFlags,
    pub aslr_entropy_bits: u32,
    pub stack_canary_value: u64,
    pub violations: Vec<ViolationRecord>,
    pub max_violations: usize,
    pub kill_on_violation: bool,
    pub code_regions: Vec<(u64, u64)>,
    pub nx_regions: Vec<(u64, u64)>,
    violation_count: u64,
}

impl ProcessShieldState {
    pub fn new(pid: u64, binary_name: String, mitigations: MitigationFlags) -> Self {
        Self {
            pid,
            binary_name,
            mitigations,
            aslr_entropy_bits: 28,
            stack_canary_value: 0,
            violations: Vec::new(),
            max_violations: 64,
            kill_on_violation: true,
            code_regions: Vec::new(),
            nx_regions: Vec::new(),
            violation_count: 0,
        }
    }

    pub fn is_code_addr(&self, addr: u64) -> bool {
        self.code_regions.iter().any(|(s, e)| addr >= *s && addr < *e)
    }

    pub fn is_nx_addr(&self, addr: u64) -> bool {
        self.nx_regions.iter().any(|(s, e)| addr >= *s && addr < *e)
    }

    pub fn check_execute(&self, addr: u64) -> bool {
        if !self.mitigations.has(MitigationFlags::NX_STACK)
            && !self.mitigations.has(MitigationFlags::NX_HEAP)
        {
            return true;
        }
        self.is_code_addr(addr)
    }

    pub fn record_violation(&mut self, violation: ViolationRecord) {
        self.violation_count += 1;
        if self.violations.len() >= self.max_violations {
            self.violations.remove(0);
        }
        self.violations.push(violation);
    }

    pub fn security_score(&self) -> f64 {
        let base = self.mitigations.security_score();
        // Penalize for violations
        let penalty = (self.violation_count as f64 * 0.05).min(0.5);
        (base - penalty).max(0.0)
    }
}

/// Stack canary validation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanaryCheck {
    Valid,
    Corrupted,
    Missing,
}

/// ASLR randomization record
#[derive(Debug, Clone)]
pub struct AslrLayout {
    pub text_base: u64,
    pub stack_base: u64,
    pub mmap_base: u64,
    pub heap_base: u64,
    pub vdso_base: u64,
    pub entropy_bits: u32,
}

impl AslrLayout {
    pub fn spread(&self) -> u64 {
        let bases = [self.text_base, self.stack_base, self.mmap_base, self.heap_base];
        let min = bases.iter().copied().min().unwrap_or(0);
        let max = bases.iter().copied().max().unwrap_or(0);
        max.saturating_sub(min)
    }
}

/// Exec shield stats
#[derive(Debug, Clone)]
pub struct ExecShieldStats {
    pub processes_protected: u64,
    pub violations_total: u64,
    pub nx_violations: u64,
    pub stack_smashes: u64,
    pub cfi_violations: u64,
    pub processes_killed: u64,
    pub avg_security_score: f64,
}

/// Main apps exec shield manager
pub struct AppExecShield {
    processes: BTreeMap<u64, ProcessShieldState>,
    default_mitigations: MitigationFlags,
    global_kill_on_violation: bool,
    stats: ExecShieldStats,
}

impl AppExecShield {
    pub fn new() -> Self {
        let defaults = MitigationFlags::ASLR
            .combine(MitigationFlags::NX_STACK)
            .combine(MitigationFlags::NX_HEAP)
            .combine(MitigationFlags::STACK_CANARY)
            .combine(MitigationFlags::RELRO)
            .combine(MitigationFlags::PIE);

        Self {
            processes: BTreeMap::new(),
            default_mitigations: defaults,
            global_kill_on_violation: true,
            stats: ExecShieldStats {
                processes_protected: 0,
                violations_total: 0,
                nx_violations: 0,
                stack_smashes: 0,
                cfi_violations: 0,
                processes_killed: 0,
                avg_security_score: 0.0,
            },
        }
    }

    pub fn register_process(
        &mut self,
        pid: u64,
        binary_name: String,
        mitigations: Option<MitigationFlags>,
    ) {
        let mits = mitigations.unwrap_or(self.default_mitigations);
        let mut state = ProcessShieldState::new(pid, binary_name, mits);
        state.kill_on_violation = self.global_kill_on_violation;
        self.processes.insert(pid, state);
        self.stats.processes_protected += 1;
    }

    pub fn unregister_process(&mut self, pid: u64) -> bool {
        self.processes.remove(&pid).is_some()
    }

    pub fn add_code_region(&mut self, pid: u64, start: u64, end: u64) -> bool {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.code_regions.push((start, end));
            true
        } else {
            false
        }
    }

    pub fn add_nx_region(&mut self, pid: u64, start: u64, end: u64) -> bool {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.nx_regions.push((start, end));
            true
        } else {
            false
        }
    }

    pub fn check_execute(&self, pid: u64, addr: u64) -> bool {
        if let Some(proc) = self.processes.get(&pid) {
            proc.check_execute(addr)
        } else {
            true // No protection registered
        }
    }

    pub fn report_violation(
        &mut self,
        pid: u64,
        violation: ViolationType,
        addr: u64,
        ip: u64,
        sp: u64,
        timestamp_ns: u64,
    ) -> bool {
        let kill = if let Some(proc) = self.processes.get_mut(&pid) {
            let record = ViolationRecord {
                pid,
                tid: 0,
                violation,
                address: addr,
                instruction_ptr: ip,
                stack_ptr: sp,
                timestamp_ns,
                killed: proc.kill_on_violation,
            };
            proc.record_violation(record);
            proc.kill_on_violation
        } else {
            return false;
        };

        self.stats.violations_total += 1;
        match violation {
            ViolationType::NxViolation => self.stats.nx_violations += 1,
            ViolationType::StackSmash => self.stats.stack_smashes += 1,
            ViolationType::CfiViolation | ViolationType::RopAttempt | ViolationType::JopAttempt => {
                self.stats.cfi_violations += 1;
            }
            _ => {}
        }

        if kill {
            self.stats.processes_killed += 1;
        }
        kill
    }

    pub fn set_aslr_layout(&mut self, pid: u64, layout: AslrLayout) -> bool {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.aslr_entropy_bits = layout.entropy_bits;
            true
        } else {
            false
        }
    }

    pub fn update_avg_score(&mut self) {
        if self.processes.is_empty() {
            self.stats.avg_security_score = 0.0;
            return;
        }
        let sum: f64 = self.processes.values().map(|p| p.security_score()).sum();
        self.stats.avg_security_score = sum / self.processes.len() as f64;
    }

    pub fn weakest_processes(&self, top_n: usize) -> Vec<(u64, f64)> {
        let mut scores: Vec<(u64, f64)> = self.processes.iter()
            .map(|(pid, p)| (*pid, p.security_score()))
            .collect();
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));
        scores.truncate(top_n);
        scores
    }

    pub fn stats(&self) -> &ExecShieldStats {
        &self.stats
    }
}
