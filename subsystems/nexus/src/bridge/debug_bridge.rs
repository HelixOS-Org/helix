// SPDX-License-Identifier: GPL-2.0
//! Bridge debug_bridge — debug interface and kernel debugging facility bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// Debug facility type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugFacility {
    Kgdb,
    Kdb,
    DynamicDebug,
    Ftrace,
    Kprobes,
    Uprobes,
    eBpfTrace,
    HwBreakpoint,
}

/// Breakpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointType {
    Software,
    Hardware,
    Watchpoint,
    WatchpointRead,
    WatchpointWrite,
    WatchpointAccess,
}

/// Breakpoint state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpState {
    Active,
    Disabled,
    Pending,
    Error,
}

/// A debug breakpoint
#[derive(Debug, Clone)]
pub struct DebugBreakpoint {
    pub id: u32,
    pub address: u64,
    pub bp_type: BreakpointType,
    pub state: BpState,
    pub hit_count: u64,
    pub condition: Option<String>,
    pub size: u8,
    pub enabled: bool,
}

impl DebugBreakpoint {
    pub fn new(id: u32, address: u64, bp_type: BreakpointType) -> Self {
        Self {
            id, address, bp_type,
            state: BpState::Pending,
            hit_count: 0,
            condition: None,
            size: 1,
            enabled: true,
        }
    }

    #[inline]
    pub fn is_hardware(&self) -> bool {
        matches!(self.bp_type, BreakpointType::Hardware
            | BreakpointType::Watchpoint
            | BreakpointType::WatchpointRead
            | BreakpointType::WatchpointWrite
            | BreakpointType::WatchpointAccess)
    }
}

/// Kprobe entry
#[derive(Debug, Clone)]
pub struct KprobeEntry {
    pub name: String,
    pub address: u64,
    pub symbol: String,
    pub offset: u32,
    pub hit_count: u64,
    pub missed: u64,
    pub enabled: bool,
    pub is_return_probe: bool,
}

impl KprobeEntry {
    #[inline]
    pub fn miss_rate(&self) -> f64 {
        let total = self.hit_count + self.missed;
        if total == 0 { return 0.0; }
        self.missed as f64 / total as f64
    }
}

/// Dynamic debug entry
#[derive(Debug, Clone)]
pub struct DynDbgEntry {
    pub module: String,
    pub file: String,
    pub line: u32,
    pub function: String,
    pub format: String,
    pub enabled: bool,
    pub flags: u32,
}

/// Debug event
#[derive(Debug, Clone)]
pub struct DebugEvent {
    pub facility: DebugFacility,
    pub event_type: DebugEventType,
    pub address: u64,
    pub pid: Option<u32>,
    pub message: String,
    pub timestamp: u64,
}

/// Debug event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugEventType {
    BreakpointHit,
    WatchpointTrigger,
    SingleStep,
    KprobeHit,
    UprobeHit,
    Panic,
    Oops,
    Warning,
    BugOn,
}

/// Ftrace function entry
#[derive(Debug, Clone)]
pub struct FtraceFunction {
    pub address: u64,
    pub symbol: String,
    pub tracer: String,
    pub hit_count: u64,
    pub total_time_ns: u64,
}

impl FtraceFunction {
    #[inline(always)]
    pub fn avg_time_ns(&self) -> u64 {
        if self.hit_count == 0 { return 0; }
        self.total_time_ns / self.hit_count
    }
}

/// Debug bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DebugBridgeStats {
    pub breakpoint_count: u32,
    pub kprobe_count: u32,
    pub uprobe_count: u32,
    pub dyndbg_entries: u32,
    pub total_hits: u64,
    pub total_events: u64,
    pub ftrace_functions: u32,
}

/// Main debug bridge
#[repr(align(64))]
pub struct BridgeDebug {
    breakpoints: BTreeMap<u32, DebugBreakpoint>,
    kprobes: BTreeMap<String, KprobeEntry>,
    dyndbg: Vec<DynDbgEntry>,
    ftrace: BTreeMap<u64, FtraceFunction>,
    events: VecDeque<DebugEvent>,
    max_events: usize,
    next_bp_id: u32,
    stats: DebugBridgeStats,
    hw_bp_max: u32,
    hw_bp_used: u32,
}

impl BridgeDebug {
    pub fn new(hw_bp_max: u32) -> Self {
        Self {
            breakpoints: BTreeMap::new(),
            kprobes: BTreeMap::new(),
            dyndbg: Vec::new(),
            ftrace: BTreeMap::new(),
            events: VecDeque::new(),
            max_events: 4096,
            next_bp_id: 1,
            stats: DebugBridgeStats {
                breakpoint_count: 0, kprobe_count: 0, uprobe_count: 0,
                dyndbg_entries: 0, total_hits: 0, total_events: 0,
                ftrace_functions: 0,
            },
            hw_bp_max,
            hw_bp_used: 0,
        }
    }

    pub fn add_breakpoint(&mut self, address: u64, bp_type: BreakpointType) -> Option<u32> {
        if bp_type != BreakpointType::Software && self.hw_bp_used >= self.hw_bp_max {
            return None;
        }
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        let mut bp = DebugBreakpoint::new(id, address, bp_type);
        bp.state = BpState::Active;
        if bp.is_hardware() { self.hw_bp_used += 1; }
        self.breakpoints.insert(id, bp);
        self.stats.breakpoint_count += 1;
        Some(id)
    }

    #[inline]
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        if let Some(bp) = self.breakpoints.remove(&id) {
            self.stats.breakpoint_count -= 1;
            if bp.is_hardware() && self.hw_bp_used > 0 { self.hw_bp_used -= 1; }
            true
        } else { false }
    }

    #[inline(always)]
    pub fn register_kprobe(&mut self, probe: KprobeEntry) {
        self.stats.kprobe_count += 1;
        self.kprobes.insert(probe.name.clone(), probe);
    }

    #[inline(always)]
    pub fn add_dyndbg(&mut self, entry: DynDbgEntry) {
        self.stats.dyndbg_entries += 1;
        self.dyndbg.push(entry);
    }

    #[inline(always)]
    pub fn add_ftrace_fn(&mut self, func: FtraceFunction) {
        self.stats.ftrace_functions += 1;
        self.ftrace.insert(func.address, func);
    }

    #[inline]
    pub fn record_event(&mut self, event: DebugEvent) {
        self.stats.total_events += 1;
        self.stats.total_hits += 1;
        if self.events.len() >= self.max_events { self.events.pop_front(); }
        self.events.push_back(event);
    }

    #[inline]
    pub fn bp_hit(&mut self, id: u32) {
        if let Some(bp) = self.breakpoints.get_mut(&id) {
            bp.hit_count += 1;
            self.stats.total_hits += 1;
        }
    }

    #[inline]
    pub fn hottest_kprobes(&self, n: usize) -> Vec<(&str, u64)> {
        let mut v: Vec<_> = self.kprobes.iter()
            .map(|(name, p)| (name.as_str(), p.hit_count))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    #[inline]
    pub fn hottest_ftrace(&self, n: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<_> = self.ftrace.iter()
            .map(|(&addr, f)| (addr, f.hit_count))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    #[inline(always)]
    pub fn available_hw_bp(&self) -> u32 {
        self.hw_bp_max.saturating_sub(self.hw_bp_used)
    }

    #[inline(always)]
    pub fn stats(&self) -> &DebugBridgeStats {
        &self.stats
    }
}
