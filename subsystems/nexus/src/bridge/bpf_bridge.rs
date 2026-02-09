// SPDX-License-Identifier: GPL-2.0
//! Bridge BPF program loading, verification, and execution proxy.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// BPF program type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfProgType {
    /// Socket filter
    SocketFilter,
    /// Kprobe/kretprobe
    Kprobe,
    /// Tracepoint
    Tracepoint,
    /// XDP (eXpress Data Path)
    Xdp,
    /// Traffic control
    SchedCls,
    /// Cgroup socket operations
    CgroupSock,
    /// LSM hook
    Lsm,
    /// Struct ops
    StructOps,
    /// Perf event
    PerfEvent,
    /// Raw tracepoint
    RawTracepoint,
}

/// BPF map type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfMapType {
    Hash,
    Array,
    PerCpuHash,
    PerCpuArray,
    LruHash,
    RingBuf,
    StackTrace,
    LpmTrie,
    BloomFilter,
    Queue,
}

/// Verification result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyResult {
    /// Program is safe
    Safe,
    /// Potentially unsafe, bounded
    Bounded { max_insns: u32 },
    /// Rejected
    Rejected,
}

/// BPF instruction (simplified)
#[derive(Debug, Clone, Copy)]
pub struct BpfInsn {
    pub opcode: u8,
    pub dst_reg: u8,
    pub src_reg: u8,
    pub offset: i16,
    pub imm: i32,
}

impl BpfInsn {
    #[inline(always)]
    pub fn is_call(&self) -> bool {
        self.opcode == 0x85
    }

    #[inline(always)]
    pub fn is_exit(&self) -> bool {
        self.opcode == 0x95
    }

    #[inline(always)]
    pub fn is_jump(&self) -> bool {
        (self.opcode & 0x07) == 0x05
    }

    #[inline(always)]
    pub fn is_load(&self) -> bool {
        (self.opcode & 0x07) == 0x01 || (self.opcode & 0x07) == 0x00
    }

    #[inline(always)]
    pub fn is_store(&self) -> bool {
        (self.opcode & 0x07) == 0x03 || (self.opcode & 0x07) == 0x02
    }
}

/// A loaded BPF program
#[derive(Debug)]
pub struct BpfProgram {
    pub id: u64,
    pub name: String,
    pub prog_type: BpfProgType,
    pub insns: Vec<BpfInsn>,
    pub verify_result: VerifyResult,
    pub attached: bool,
    pub attach_point: String,
    pub run_count: u64,
    pub run_time_ns: u64,
    pub last_error: Option<i32>,
    created_ns: u64,
}

impl BpfProgram {
    pub fn new(id: u64, name: String, prog_type: BpfProgType, insns: Vec<BpfInsn>) -> Self {
        Self {
            id,
            name,
            prog_type,
            insns,
            verify_result: VerifyResult::Rejected,
            attached: false,
            attach_point: String::new(),
            run_count: 0,
            run_time_ns: 0,
            last_error: None,
            created_ns: 0,
        }
    }

    #[inline(always)]
    pub fn instruction_count(&self) -> usize {
        self.insns.len()
    }

    #[inline(always)]
    pub fn avg_run_ns(&self) -> u64 {
        if self.run_count == 0 { 0 } else { self.run_time_ns / self.run_count }
    }

    #[inline]
    pub fn complexity_score(&self) -> u32 {
        let mut score = self.insns.len() as u32;
        for insn in &self.insns {
            if insn.is_call() { score += 5; }
            if insn.is_jump() { score += 2; }
        }
        score
    }
}

/// A BPF map instance
#[derive(Debug)]
pub struct BpfMap {
    pub id: u64,
    pub name: String,
    pub map_type: BpfMapType,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
    pub current_entries: u32,
    lookup_count: u64,
    update_count: u64,
    delete_count: u64,
}

impl BpfMap {
    pub fn new(id: u64, name: String, map_type: BpfMapType, key_size: u32, value_size: u32, max_entries: u32) -> Self {
        Self {
            id,
            name,
            map_type,
            key_size,
            value_size,
            max_entries,
            current_entries: 0,
            lookup_count: 0,
            update_count: 0,
            delete_count: 0,
        }
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.current_entries >= self.max_entries
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.max_entries == 0 { return 0.0; }
        self.current_entries as f64 / self.max_entries as f64
    }

    #[inline(always)]
    pub fn memory_bytes(&self) -> u64 {
        (self.key_size as u64 + self.value_size as u64) * self.max_entries as u64
    }

    #[inline(always)]
    pub fn record_lookup(&mut self) {
        self.lookup_count = self.lookup_count.saturating_add(1);
    }

    #[inline(always)]
    pub fn record_update(&mut self) {
        self.update_count = self.update_count.saturating_add(1);
    }

    #[inline(always)]
    pub fn record_delete(&mut self) {
        self.delete_count = self.delete_count.saturating_add(1);
    }
}

/// Verification configuration
#[derive(Debug, Clone)]
pub struct VerifyConfig {
    pub max_insns: u32,
    pub max_stack_depth: u32,
    pub allow_kfunc: bool,
    pub allow_bpf_to_bpf: bool,
    pub log_level: u32,
}

impl VerifyConfig {
    #[inline]
    pub fn default_config() -> Self {
        Self {
            max_insns: 1_000_000,
            max_stack_depth: 512,
            allow_kfunc: true,
            allow_bpf_to_bpf: true,
            log_level: 1,
        }
    }

    #[inline]
    pub fn strict() -> Self {
        Self {
            max_insns: 4096,
            max_stack_depth: 256,
            allow_kfunc: false,
            allow_bpf_to_bpf: false,
            log_level: 2,
        }
    }
}

/// BPF bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BpfBridgeStats {
    pub programs_loaded: u64,
    pub programs_rejected: u64,
    pub maps_created: u64,
    pub total_runs: u64,
    pub total_run_ns: u64,
    pub attach_count: u64,
    pub detach_count: u64,
}

/// Main BPF bridge manager
#[repr(align(64))]
pub struct BridgeBpf {
    programs: BTreeMap<u64, BpfProgram>,
    maps: BTreeMap<u64, BpfMap>,
    next_prog_id: u64,
    next_map_id: u64,
    verify_config: VerifyConfig,
    stats: BpfBridgeStats,
}

impl BridgeBpf {
    pub fn new() -> Self {
        Self {
            programs: BTreeMap::new(),
            maps: BTreeMap::new(),
            next_prog_id: 1,
            next_map_id: 1,
            verify_config: VerifyConfig::default_config(),
            stats: BpfBridgeStats {
                programs_loaded: 0,
                programs_rejected: 0,
                maps_created: 0,
                total_runs: 0,
                total_run_ns: 0,
                attach_count: 0,
                detach_count: 0,
            },
        }
    }

    fn verify_program(&self, prog: &BpfProgram) -> VerifyResult {
        if prog.insns.is_empty() {
            return VerifyResult::Rejected;
        }
        if prog.insns.len() as u32 > self.verify_config.max_insns {
            return VerifyResult::Rejected;
        }
        // Check last instruction is exit
        if let Some(last) = prog.insns.last() {
            if !last.is_exit() {
                return VerifyResult::Rejected;
            }
        }
        // Check for unrestricted calls
        if !self.verify_config.allow_kfunc {
            for insn in &prog.insns {
                if insn.is_call() && insn.imm < 0 {
                    return VerifyResult::Rejected;
                }
            }
        }
        let call_count = prog.insns.iter().filter(|i| i.is_call()).count() as u32;
        if !self.verify_config.allow_bpf_to_bpf && call_count > 0 {
            // Only helper calls allowed, check for bpf-to-bpf (src_reg == 1)
            let bpf2bpf = prog.insns.iter().any(|i| i.is_call() && i.src_reg == 1);
            if bpf2bpf {
                return VerifyResult::Rejected;
            }
        }
        VerifyResult::Bounded {
            max_insns: prog.insns.len() as u32,
        }
    }

    pub fn load_program(
        &mut self,
        name: String,
        prog_type: BpfProgType,
        insns: Vec<BpfInsn>,
    ) -> Option<u64> {
        let id = self.next_prog_id;
        self.next_prog_id += 1;
        let mut prog = BpfProgram::new(id, name, prog_type, insns);
        let result = self.verify_program(&prog);
        prog.verify_result = result;
        match result {
            VerifyResult::Rejected => {
                self.stats.programs_rejected += 1;
                None
            }
            _ => {
                self.programs.insert(id, prog);
                self.stats.programs_loaded += 1;
                Some(id)
            }
        }
    }

    #[inline]
    pub fn unload_program(&mut self, prog_id: u64) -> bool {
        if let Some(prog) = self.programs.remove(&prog_id) {
            if prog.attached {
                self.stats.detach_count += 1;
            }
            true
        } else {
            false
        }
    }

    pub fn attach_program(&mut self, prog_id: u64, attach_point: String) -> bool {
        if let Some(prog) = self.programs.get_mut(&prog_id) {
            if prog.attached {
                return false;
            }
            match prog.verify_result {
                VerifyResult::Rejected => return false,
                _ => {}
            }
            prog.attached = true;
            prog.attach_point = attach_point;
            self.stats.attach_count += 1;
            true
        } else {
            false
        }
    }

    pub fn detach_program(&mut self, prog_id: u64) -> bool {
        if let Some(prog) = self.programs.get_mut(&prog_id) {
            if !prog.attached {
                return false;
            }
            prog.attached = false;
            prog.attach_point = String::new();
            self.stats.detach_count += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn create_map(
        &mut self,
        name: String,
        map_type: BpfMapType,
        key_size: u32,
        value_size: u32,
        max_entries: u32,
    ) -> u64 {
        let id = self.next_map_id;
        self.next_map_id += 1;
        self.maps.insert(id, BpfMap::new(id, name, map_type, key_size, value_size, max_entries));
        self.stats.maps_created += 1;
        id
    }

    #[inline(always)]
    pub fn delete_map(&mut self, map_id: u64) -> bool {
        self.maps.remove(&map_id).is_some()
    }

    #[inline]
    pub fn record_run(&mut self, prog_id: u64, duration_ns: u64, error: Option<i32>) {
        if let Some(prog) = self.programs.get_mut(&prog_id) {
            prog.run_count = prog.run_count.saturating_add(1);
            prog.run_time_ns = prog.run_time_ns.saturating_add(duration_ns);
            prog.last_error = error;
            self.stats.total_runs += 1;
            self.stats.total_run_ns = self.stats.total_run_ns.saturating_add(duration_ns);
        }
    }

    #[inline]
    pub fn map_lookup(&mut self, map_id: u64) -> bool {
        if let Some(map) = self.maps.get_mut(&map_id) {
            map.record_lookup();
            true
        } else {
            false
        }
    }

    pub fn map_update(&mut self, map_id: u64) -> bool {
        if let Some(map) = self.maps.get_mut(&map_id) {
            if map.is_full() && map.current_entries >= map.max_entries {
                return false;
            }
            map.record_update();
            if map.current_entries < map.max_entries {
                map.current_entries += 1;
            }
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn set_verify_config(&mut self, config: VerifyConfig) {
        self.verify_config = config;
    }

    #[inline]
    pub fn program_info(&self, prog_id: u64) -> Option<(u64, &str, BpfProgType, bool)> {
        self.programs.get(&prog_id).map(|p| {
            (p.run_count, p.attach_point.as_str(), p.prog_type, p.attached)
        })
    }

    #[inline(always)]
    pub fn total_bpf_memory(&self) -> u64 {
        self.maps.values().map(|m| m.memory_bytes()).sum()
    }

    #[inline(always)]
    pub fn stats(&self) -> &BpfBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from bpf_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfV2ProgType {
    SocketFilter,
    Kprobe,
    TracePoint,
    Xdp,
    PerfEvent,
    CgroupSkb,
    LwtTunnel,
    SchedCls,
    SchedAct,
    SkMsg,
    FlowDissector,
    Lsm,
    StructOps,
}

/// BPF map type v2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfV2MapType {
    Hash,
    Array,
    ProgArray,
    PerfEventArray,
    PerCpuHash,
    PerCpuArray,
    LruHash,
    RingBuf,
    BloomFilter,
}

/// BPF program v2
#[derive(Debug)]
pub struct BpfV2Program {
    pub id: u64,
    pub prog_type: BpfV2ProgType,
    pub insn_count: u32,
    pub verified: bool,
    pub jitted: bool,
    pub attach_count: u32,
    pub run_count: u64,
    pub run_time_ns: u64,
}

impl BpfV2Program {
    pub fn new(id: u64, pt: BpfV2ProgType, insns: u32) -> Self {
        Self { id, prog_type: pt, insn_count: insns, verified: false, jitted: false, attach_count: 0, run_count: 0, run_time_ns: 0 }
    }
    #[inline(always)]
    pub fn avg_run_ns(&self) -> u64 { if self.run_count == 0 { 0 } else { self.run_time_ns / self.run_count } }
}

/// BPF map v2
#[derive(Debug)]
pub struct BpfV2Map {
    pub id: u64,
    pub map_type: BpfV2MapType,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
    pub used_entries: u32,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BpfV2BridgeStats {
    pub total_programs: u32,
    pub total_maps: u32,
    pub verified: u32,
    pub jitted: u32,
    pub total_runs: u64,
}

/// Main BPF v2 bridge
#[repr(align(64))]
pub struct BridgeBpfV2 {
    programs: BTreeMap<u64, BpfV2Program>,
    maps: Vec<BpfV2Map>,
    next_id: u64,
}

impl BridgeBpfV2 {
    pub fn new() -> Self { Self { programs: BTreeMap::new(), maps: Vec::new(), next_id: 1 } }

    #[inline]
    pub fn load_program(&mut self, pt: BpfV2ProgType, insns: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.programs.insert(id, BpfV2Program::new(id, pt, insns));
        id
    }

    #[inline(always)]
    pub fn verify(&mut self, id: u64) -> bool {
        if let Some(p) = self.programs.get_mut(&id) { p.verified = true; true } else { false }
    }

    #[inline]
    pub fn stats(&self) -> BpfV2BridgeStats {
        let verified = self.programs.values().filter(|p| p.verified).count() as u32;
        let jitted = self.programs.values().filter(|p| p.jitted).count() as u32;
        let runs: u64 = self.programs.values().map(|p| p.run_count).sum();
        BpfV2BridgeStats { total_programs: self.programs.len() as u32, total_maps: self.maps.len() as u32, verified, jitted, total_runs: runs }
    }
}
