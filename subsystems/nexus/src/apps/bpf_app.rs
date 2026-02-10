// SPDX-License-Identifier: GPL-2.0
//! Apps bpf_app — BPF program management application layer.

extern crate alloc;

use alloc::collections::BTreeMap;

/// BPF program type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfProgType {
    Unspec,
    SocketFilter,
    Kprobe,
    SchedCls,
    SchedAct,
    Tracepoint,
    Xdp,
    PerfEvent,
    CgroupSkb,
    CgroupSock,
    LwtIn,
    LwtOut,
    SockOps,
    SkMsg,
    RawTracepoint,
}

/// BPF map type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpfMapType {
    Hash,
    Array,
    ProgArray,
    PerfEventArray,
    PerCpuHash,
    PerCpuArray,
    StackTrace,
    LpmTrie,
    RingBuf,
    Queue,
    Stack,
}

/// BPF program
#[derive(Debug)]
pub struct BpfProgram {
    pub id: u64,
    pub prog_type: BpfProgType,
    pub insn_count: u32,
    pub verified: bool,
    pub attached: bool,
    pub run_count: u64,
    pub run_time_ns: u64,
}

/// BPF map
#[derive(Debug)]
pub struct BpfMap {
    pub id: u64,
    pub map_type: BpfMapType,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
    pub cur_entries: u32,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BpfAppStats {
    pub total_progs: u32,
    pub total_maps: u32,
    pub attached_progs: u32,
    pub total_runs: u64,
}

/// Main app BPF
pub struct AppBpf {
    progs: BTreeMap<u64, BpfProgram>,
    maps: BTreeMap<u64, BpfMap>,
    next_id: u64,
}

impl AppBpf {
    pub fn new() -> Self { Self { progs: BTreeMap::new(), maps: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn load_prog(&mut self, ptype: BpfProgType, insns: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.progs.insert(id, BpfProgram { id, prog_type: ptype, insn_count: insns, verified: true, attached: false, run_count: 0, run_time_ns: 0 });
        id
    }

    #[inline]
    pub fn create_map(&mut self, mtype: BpfMapType, key: u32, val: u32, max: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.maps.insert(id, BpfMap { id, map_type: mtype, key_size: key, value_size: val, max_entries: max, cur_entries: 0 });
        id
    }

    #[inline(always)]
    pub fn attach(&mut self, prog_id: u64) { if let Some(p) = self.progs.get_mut(&prog_id) { p.attached = true; } }
    #[inline(always)]
    pub fn detach(&mut self, prog_id: u64) { if let Some(p) = self.progs.get_mut(&prog_id) { p.attached = false; } }

    #[inline(always)]
    pub fn record_run(&mut self, prog_id: u64, ns: u64) {
        if let Some(p) = self.progs.get_mut(&prog_id) { p.run_count += 1; p.run_time_ns += ns; }
    }

    #[inline]
    pub fn stats(&self) -> BpfAppStats {
        let attached = self.progs.values().filter(|p| p.attached).count() as u32;
        let runs: u64 = self.progs.values().map(|p| p.run_count).sum();
        BpfAppStats { total_progs: self.progs.len() as u32, total_maps: self.maps.len() as u32, attached_progs: attached, total_runs: runs }
    }
}
