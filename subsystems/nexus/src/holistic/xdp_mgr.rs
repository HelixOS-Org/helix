// SPDX-License-Identifier: GPL-2.0
//! Holistic XDP manager — eXpress Data Path program loading, execution, and map management

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// XDP action verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdpAction {
    Aborted,
    Drop,
    Pass,
    Tx,
    Redirect,
}

/// XDP attach mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdpAttachMode {
    Generic,
    Native,
    Offloaded,
    Multi,
}

/// XDP map type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdpMapType {
    Hash,
    Array,
    ProgArray,
    PerfEventArray,
    DevMap,
    CpuMap,
    XskMap,
    SockMap,
    StackTrace,
    LpmTrie,
    RingBuf,
}

/// XDP program state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XdpProgState {
    Loaded,
    Verified,
    JitCompiled,
    Attached,
    Detached,
    Error,
}

/// XDP map instance
#[derive(Debug, Clone)]
pub struct XdpMap {
    pub map_id: u32,
    pub map_type: XdpMapType,
    pub key_size: u32,
    pub value_size: u32,
    pub max_entries: u32,
    pub current_entries: u32,
    pub lookups: u64,
    pub updates: u64,
    pub deletes: u64,
    pub lookup_misses: u64,
}

impl XdpMap {
    pub fn new(map_id: u32, map_type: XdpMapType, key_size: u32, value_size: u32, max_entries: u32) -> Self {
        Self {
            map_id,
            map_type,
            key_size,
            value_size,
            max_entries,
            current_entries: 0,
            lookups: 0,
            updates: 0,
            deletes: 0,
            lookup_misses: 0,
        }
    }

    pub fn lookup(&mut self, found: bool) {
        self.lookups += 1;
        if !found {
            self.lookup_misses += 1;
        }
    }

    pub fn update(&mut self) -> bool {
        if self.current_entries >= self.max_entries {
            return false;
        }
        self.current_entries += 1;
        self.updates += 1;
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.current_entries == 0 {
            return false;
        }
        self.current_entries -= 1;
        self.deletes += 1;
        true
    }

    pub fn utilization_pct(&self) -> f64 {
        if self.max_entries == 0 {
            return 0.0;
        }
        (self.current_entries as f64 / self.max_entries as f64) * 100.0
    }

    pub fn miss_rate(&self) -> f64 {
        if self.lookups == 0 {
            return 0.0;
        }
        self.lookup_misses as f64 / self.lookups as f64
    }

    pub fn memory_bytes(&self) -> u64 {
        (self.key_size as u64 + self.value_size as u64) * self.max_entries as u64
    }
}

/// XDP program
#[derive(Debug, Clone)]
pub struct XdpProgram {
    pub prog_id: u32,
    pub state: XdpProgState,
    pub attach_mode: XdpAttachMode,
    pub interface_id: u32,
    pub insn_count: u32,
    pub jit_size: u32,
    pub maps: Vec<u32>,
    pub packets_processed: u64,
    pub bytes_processed: u64,
    pub actions: [u64; 5],
    pub run_time_ns: u64,
    pub run_count: u64,
}

impl XdpProgram {
    pub fn new(prog_id: u32, insn_count: u32) -> Self {
        Self {
            prog_id,
            state: XdpProgState::Loaded,
            attach_mode: XdpAttachMode::Native,
            interface_id: 0,
            insn_count,
            jit_size: insn_count * 8,
            maps: Vec::new(),
            packets_processed: 0,
            bytes_processed: 0,
            actions: [0; 5],
            run_time_ns: 0,
            run_count: 0,
        }
    }

    pub fn verify(&mut self) -> bool {
        if self.insn_count > 0 && self.insn_count <= 1_000_000 {
            self.state = XdpProgState::Verified;
            true
        } else {
            self.state = XdpProgState::Error;
            false
        }
    }

    pub fn jit_compile(&mut self) {
        if self.state == XdpProgState::Verified {
            self.state = XdpProgState::JitCompiled;
        }
    }

    pub fn attach(&mut self, interface_id: u32, mode: XdpAttachMode) {
        self.interface_id = interface_id;
        self.attach_mode = mode;
        self.state = XdpProgState::Attached;
    }

    pub fn record_execution(&mut self, action: XdpAction, pkt_bytes: u64, duration_ns: u64) {
        self.packets_processed += 1;
        self.bytes_processed += pkt_bytes;
        self.run_time_ns += duration_ns;
        self.run_count += 1;
        self.actions[action as usize] += 1;
    }

    pub fn avg_latency_ns(&self) -> u64 {
        if self.run_count == 0 { 0 } else { self.run_time_ns / self.run_count }
    }

    pub fn drop_rate(&self) -> f64 {
        if self.packets_processed == 0 {
            return 0.0;
        }
        self.actions[XdpAction::Drop as usize] as f64 / self.packets_processed as f64
    }

    pub fn redirect_rate(&self) -> f64 {
        if self.packets_processed == 0 {
            return 0.0;
        }
        self.actions[XdpAction::Redirect as usize] as f64 / self.packets_processed as f64
    }

    pub fn add_map(&mut self, map_id: u32) {
        self.maps.push(map_id);
    }
}

/// XDP manager stats
#[derive(Debug, Clone)]
pub struct XdpMgrStats {
    pub total_programs: u64,
    pub attached_programs: u64,
    pub total_maps: u64,
    pub total_packets: u64,
    pub total_drops: u64,
    pub total_redirects: u64,
}

/// Main holistic XDP manager
#[derive(Debug)]
pub struct HolisticXdpMgr {
    pub programs: BTreeMap<u32, XdpProgram>,
    pub maps: BTreeMap<u32, XdpMap>,
    pub stats: XdpMgrStats,
    pub next_prog_id: u32,
    pub next_map_id: u32,
}

impl HolisticXdpMgr {
    pub fn new() -> Self {
        Self {
            programs: BTreeMap::new(),
            maps: BTreeMap::new(),
            stats: XdpMgrStats {
                total_programs: 0,
                attached_programs: 0,
                total_maps: 0,
                total_packets: 0,
                total_drops: 0,
                total_redirects: 0,
            },
            next_prog_id: 1,
            next_map_id: 1,
        }
    }

    pub fn load_program(&mut self, insn_count: u32) -> u32 {
        let id = self.next_prog_id;
        self.next_prog_id += 1;
        let mut prog = XdpProgram::new(id, insn_count);
        prog.verify();
        prog.jit_compile();
        self.programs.insert(id, prog);
        self.stats.total_programs += 1;
        id
    }

    pub fn attach_program(&mut self, prog_id: u32, interface_id: u32, mode: XdpAttachMode) -> bool {
        if let Some(prog) = self.programs.get_mut(&prog_id) {
            prog.attach(interface_id, mode);
            self.stats.attached_programs += 1;
            true
        } else {
            false
        }
    }

    pub fn create_map(&mut self, map_type: XdpMapType, key_size: u32, value_size: u32, max_entries: u32) -> u32 {
        let id = self.next_map_id;
        self.next_map_id += 1;
        self.maps.insert(id, XdpMap::new(id, map_type, key_size, value_size, max_entries));
        self.stats.total_maps += 1;
        id
    }

    pub fn total_map_memory(&self) -> u64 {
        self.maps.values().map(|m| m.memory_bytes()).sum()
    }
}
