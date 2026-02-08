//! # Bridge Ptrace Manager
//!
//! Ptrace syscall interception and tracing bridge:
//! - Tracee state machine (stopped, running, syscall-enter/exit)
//! - Breakpoint / watchpoint management
//! - Register snapshot capture
//! - Single-step tracing
//! - Multi-threaded attach/detach
//! - PTRACE_PEEKDATA / PTRACE_POKEDATA proxying

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Ptrace request types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtraceRequest {
    Attach,
    Detach,
    PeekText,
    PeekData,
    PokeText,
    PokeData,
    GetRegs,
    SetRegs,
    GetFpRegs,
    SetFpRegs,
    SingleStep,
    Continue,
    Syscall,
    Kill,
    SetOptions,
    GetEventMsg,
    Seize,
    Interrupt,
    Listen,
}

/// Tracee state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceeState {
    Running,
    SyscallEnter,
    SyscallExit,
    SignalStop,
    GroupStop,
    EventStop,
    SingleStepping,
    Exiting,
    Detached,
}

/// Register snapshot (x86_64 subset)
#[derive(Debug, Clone, Copy, Default)]
pub struct RegisterSnapshot {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub orig_rax: u64,
}

impl RegisterSnapshot {
    pub fn syscall_nr(&self) -> u64 { self.orig_rax }
    pub fn syscall_return(&self) -> u64 { self.rax }
    pub fn arg0(&self) -> u64 { self.rdi }
    pub fn arg1(&self) -> u64 { self.rsi }
    pub fn arg2(&self) -> u64 { self.rdx }
    pub fn arg3(&self) -> u64 { self.r10 }
    pub fn arg4(&self) -> u64 { self.r8 }
    pub fn arg5(&self) -> u64 { self.r9 }
}

/// Breakpoint entry
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub id: u32,
    pub address: u64,
    pub original_byte: u8,
    pub enabled: bool,
    pub hit_count: u64,
    pub condition: Option<BreakCondition>,
}

/// Breakpoint condition
#[derive(Debug, Clone)]
pub struct BreakCondition {
    pub register: u8,
    pub compare: CompareOp,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessEqual,
    GreaterEqual,
}

/// Watchpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchType {
    Write,
    ReadWrite,
    Execute,
}

/// Watchpoint entry
#[derive(Debug, Clone)]
pub struct Watchpoint {
    pub id: u32,
    pub address: u64,
    pub size: u8,
    pub watch_type: WatchType,
    pub enabled: bool,
    pub hit_count: u64,
}

/// Tracee (traced process/thread)
#[derive(Debug, Clone)]
pub struct Tracee {
    pub pid: u64,
    pub tid: u64,
    pub tracer_pid: u64,
    pub state: TraceeState,
    pub regs: RegisterSnapshot,
    pub breakpoints: Vec<Breakpoint>,
    pub watchpoints: Vec<Watchpoint>,
    pub syscall_trace: bool,
    pub event_mask: u32,
    pub stop_signal: Option<u32>,
    pub attach_ns: u64,
    pub total_stops: u64,
    pub next_bp_id: u32,
    pub next_wp_id: u32,
}

impl Tracee {
    pub fn new(pid: u64, tid: u64, tracer: u64, ts: u64) -> Self {
        Self {
            pid, tid, tracer_pid: tracer,
            state: TraceeState::Running,
            regs: RegisterSnapshot::default(),
            breakpoints: Vec::new(),
            watchpoints: Vec::new(),
            syscall_trace: false,
            event_mask: 0,
            stop_signal: None,
            attach_ns: ts,
            total_stops: 0,
            next_bp_id: 1,
            next_wp_id: 1,
        }
    }

    pub fn add_breakpoint(&mut self, addr: u64, orig_byte: u8) -> u32 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.breakpoints.push(Breakpoint {
            id, address: addr, original_byte: orig_byte,
            enabled: true, hit_count: 0, condition: None,
        });
        id
    }

    pub fn add_watchpoint(&mut self, addr: u64, size: u8, wtype: WatchType) -> u32 {
        let id = self.next_wp_id;
        self.next_wp_id += 1;
        self.watchpoints.push(Watchpoint {
            id, address: addr, size, watch_type: wtype,
            enabled: true, hit_count: 0,
        });
        id
    }

    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        if let Some(idx) = self.breakpoints.iter().position(|b| b.id == id) {
            self.breakpoints.remove(idx);
            true
        } else { false }
    }

    pub fn stop(&mut self, state: TraceeState, signal: Option<u32>) {
        self.state = state;
        self.stop_signal = signal;
        self.total_stops += 1;
    }

    pub fn resume(&mut self) {
        self.state = TraceeState::Running;
        self.stop_signal = None;
    }
}

/// Ptrace event record
#[derive(Debug, Clone)]
pub struct PtraceEvent {
    pub tracer_pid: u64,
    pub tracee_pid: u64,
    pub request: PtraceRequest,
    pub timestamp_ns: u64,
}

/// Bridge ptrace stats
#[derive(Debug, Clone, Default)]
pub struct BridgePtraceStats {
    pub total_tracees: usize,
    pub total_breakpoints: usize,
    pub total_watchpoints: usize,
    pub total_stops: u64,
    pub total_events: usize,
}

/// Bridge Ptrace Manager
pub struct BridgePtraceBridge {
    tracees: BTreeMap<u64, Tracee>,
    events: Vec<PtraceEvent>,
    max_events: usize,
    stats: BridgePtraceStats,
}

impl BridgePtraceBridge {
    pub fn new(max_events: usize) -> Self {
        Self {
            tracees: BTreeMap::new(),
            events: Vec::new(),
            max_events,
            stats: BridgePtraceStats::default(),
        }
    }

    pub fn attach(&mut self, tracer_pid: u64, tracee_pid: u64, tid: u64, ts: u64) {
        let tracee = Tracee::new(tracee_pid, tid, tracer_pid, ts);
        self.tracees.insert(tid, tracee);
        self.emit_event(tracer_pid, tracee_pid, PtraceRequest::Attach, ts);
    }

    pub fn detach(&mut self, tid: u64, ts: u64) -> bool {
        if let Some(mut tracee) = self.tracees.remove(&tid) {
            tracee.state = TraceeState::Detached;
            self.emit_event(tracee.tracer_pid, tracee.pid, PtraceRequest::Detach, ts);
            true
        } else { false }
    }

    pub fn syscall_enter(&mut self, tid: u64, regs: RegisterSnapshot) {
        if let Some(tracee) = self.tracees.get_mut(&tid) {
            tracee.regs = regs;
            tracee.stop(TraceeState::SyscallEnter, None);
        }
    }

    pub fn syscall_exit(&mut self, tid: u64, regs: RegisterSnapshot) {
        if let Some(tracee) = self.tracees.get_mut(&tid) {
            tracee.regs = regs;
            tracee.stop(TraceeState::SyscallExit, None);
        }
    }

    pub fn continue_tracee(&mut self, tid: u64, ts: u64) {
        if let Some(tracee) = self.tracees.get_mut(&tid) {
            let pid = tracee.pid;
            let tracer = tracee.tracer_pid;
            tracee.resume();
            self.emit_event(tracer, pid, PtraceRequest::Continue, ts);
        }
    }

    pub fn single_step(&mut self, tid: u64, ts: u64) {
        if let Some(tracee) = self.tracees.get_mut(&tid) {
            let pid = tracee.pid;
            let tracer = tracee.tracer_pid;
            tracee.state = TraceeState::SingleStepping;
            self.emit_event(tracer, pid, PtraceRequest::SingleStep, ts);
        }
    }

    pub fn get_regs(&self, tid: u64) -> Option<&RegisterSnapshot> {
        self.tracees.get(&tid).map(|t| &t.regs)
    }

    fn emit_event(&mut self, tracer: u64, tracee: u64, req: PtraceRequest, ts: u64) {
        self.events.push(PtraceEvent {
            tracer_pid: tracer,
            tracee_pid: tracee,
            request: req,
            timestamp_ns: ts,
        });
        while self.events.len() > self.max_events { self.events.remove(0); }
    }

    pub fn recompute(&mut self) {
        self.stats.total_tracees = self.tracees.len();
        self.stats.total_breakpoints = self.tracees.values().map(|t| t.breakpoints.len()).sum();
        self.stats.total_watchpoints = self.tracees.values().map(|t| t.watchpoints.len()).sum();
        self.stats.total_stops = self.tracees.values().map(|t| t.total_stops).sum();
        self.stats.total_events = self.events.len();
    }

    pub fn tracee(&self, tid: u64) -> Option<&Tracee> { self.tracees.get(&tid) }
    pub fn stats(&self) -> &BridgePtraceStats { &self.stats }
}

// ============================================================================
// Merged from ptrace_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtraceRequest {
    Attach,
    Detach,
    PeekText,
    PeekData,
    PokeText,
    PokeData,
    GetRegs,
    SetRegs,
    GetFpRegs,
    SetFpRegs,
    SingleStep,
    Continue,
    Syscall,
    Seize,
    Interrupt,
    Listen,
    GetSigInfo,
    SetSigInfo,
    Seccomp,
}

/// Ptrace stop reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtraceStop {
    Signal(u32),
    SyscallEntry,
    SyscallExit,
    Clone,
    Exec,
    Exit,
    SeccompStop,
    GroupStop,
    Interrupt,
}

/// Tracee state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceeState {
    Running,
    Stopped,
    Stepping,
    SyscallTracing,
    Detached,
    Exited,
}

/// Register set snapshot
#[derive(Debug, Clone)]
pub struct RegisterSet {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cs: u64,
    pub ss: u64,
    pub orig_rax: u64,
}

impl RegisterSet {
    pub fn zeroed() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0, rsi: 0, rdi: 0,
            rbp: 0, rsp: 0, r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0, rip: 0, rflags: 0,
            cs: 0, ss: 0, orig_rax: 0,
        }
    }
}

/// Tracee (traced process)
#[derive(Debug)]
pub struct Tracee {
    pub pid: u64,
    pub tracer_pid: u64,
    pub state: TraceeState,
    pub stop_reason: Option<PtraceStop>,
    pub regs: RegisterSet,
    pub syscall_nr: Option<u64>,
    pub options: u32,
    pub total_stops: u64,
    pub total_syscalls: u64,
    pub attached_at: u64,
}

impl Tracee {
    pub fn new(pid: u64, tracer: u64, now: u64) -> Self {
        Self {
            pid, tracer_pid: tracer, state: TraceeState::Stopped,
            stop_reason: None, regs: RegisterSet::zeroed(),
            syscall_nr: None, options: 0, total_stops: 0,
            total_syscalls: 0, attached_at: now,
        }
    }

    pub fn stop(&mut self, reason: PtraceStop) {
        self.state = TraceeState::Stopped;
        self.stop_reason = Some(reason);
        self.total_stops += 1;
        if matches!(reason, PtraceStop::SyscallEntry | PtraceStop::SyscallExit) {
            self.total_syscalls += 1;
        }
    }

    pub fn resume(&mut self, stepping: bool) {
        self.state = if stepping { TraceeState::Stepping } else { TraceeState::Running };
        self.stop_reason = None;
    }

    pub fn detach(&mut self) { self.state = TraceeState::Detached; }
}

/// Ptrace event record
#[derive(Debug, Clone)]
pub struct PtraceEvent {
    pub tracee_pid: u64,
    pub request: PtraceRequest,
    pub result: i64,
    pub timestamp: u64,
}

/// Bridge stats
#[derive(Debug, Clone)]
pub struct PtraceV2BridgeStats {
    pub total_tracees: u32,
    pub active_tracees: u32,
    pub total_events: u64,
    pub total_syscalls_traced: u64,
    pub total_stops: u64,
}

/// Main ptrace v2 bridge
pub struct BridgePtraceV2 {
    tracees: BTreeMap<u64, Tracee>,
    events: Vec<PtraceEvent>,
    max_events: usize,
}

impl BridgePtraceV2 {
    pub fn new() -> Self {
        Self { tracees: BTreeMap::new(), events: Vec::new(), max_events: 4096 }
    }

    pub fn attach(&mut self, pid: u64, tracer: u64, now: u64) -> bool {
        if self.tracees.contains_key(&pid) { return false; }
        self.tracees.insert(pid, Tracee::new(pid, tracer, now));
        self.record_event(pid, PtraceRequest::Attach, 0, now);
        true
    }

    pub fn detach(&mut self, pid: u64, now: u64) -> bool {
        if let Some(t) = self.tracees.get_mut(&pid) {
            t.detach();
            self.record_event(pid, PtraceRequest::Detach, 0, now);
            true
        } else { false }
    }

    pub fn stop_tracee(&mut self, pid: u64, reason: PtraceStop) {
        if let Some(t) = self.tracees.get_mut(&pid) { t.stop(reason); }
    }

    pub fn continue_tracee(&mut self, pid: u64, stepping: bool, now: u64) {
        if let Some(t) = self.tracees.get_mut(&pid) {
            t.resume(stepping);
            let req = if stepping { PtraceRequest::SingleStep } else { PtraceRequest::Continue };
            self.record_event(pid, req, 0, now);
        }
    }

    pub fn get_regs(&self, pid: u64) -> Option<&RegisterSet> {
        self.tracees.get(&pid).map(|t| &t.regs)
    }

    fn record_event(&mut self, pid: u64, req: PtraceRequest, result: i64, now: u64) {
        if self.events.len() >= self.max_events { self.events.drain(..self.max_events / 4); }
        self.events.push(PtraceEvent { tracee_pid: pid, request: req, result, timestamp: now });
    }

    pub fn stats(&self) -> PtraceV2BridgeStats {
        let active = self.tracees.values().filter(|t| t.state != TraceeState::Detached && t.state != TraceeState::Exited).count() as u32;
        let syscalls: u64 = self.tracees.values().map(|t| t.total_syscalls).sum();
        let stops: u64 = self.tracees.values().map(|t| t.total_stops).sum();
        PtraceV2BridgeStats {
            total_tracees: self.tracees.len() as u32, active_tracees: active,
            total_events: self.events.len() as u64, total_syscalls_traced: syscalls,
            total_stops: stops,
        }
    }
}

// ============================================================================
// Merged from ptrace_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtraceV3Request {
    Attach,
    Detach,
    PeekText,
    PeekData,
    PokeText,
    PokeData,
    GetRegs,
    SetRegs,
    GetFpRegs,
    SetFpRegs,
    Cont,
    SingleStep,
    Syscall,
    Kill,
    Seize,
    Interrupt,
    Listen,
    GetSigInfo,
    SetSigInfo,
}

/// Ptrace v3 event
#[derive(Debug)]
pub struct PtraceV3Event {
    pub seq: u64,
    pub request: PtraceV3Request,
    pub tracer_pid: u64,
    pub tracee_pid: u64,
    pub addr: u64,
    pub data: u64,
    pub success: bool,
    pub timestamp: u64,
}

/// Ptrace v3 session
#[derive(Debug)]
pub struct PtraceV3Session {
    pub tracer: u64,
    pub tracee: u64,
    pub attached: bool,
    pub events: Vec<PtraceV3Event>,
    pub breakpoints: Vec<u64>,
    pub watchpoints: Vec<(u64, u64)>,
    pub single_stepping: bool,
    pub syscall_tracing: bool,
}

impl PtraceV3Session {
    pub fn new(tracer: u64, tracee: u64) -> Self {
        Self { tracer, tracee, attached: true, events: Vec::new(), breakpoints: Vec::new(), watchpoints: Vec::new(), single_stepping: false, syscall_tracing: false }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PtraceV3BridgeStats {
    pub total_sessions: u32,
    pub active_sessions: u32,
    pub total_events: u64,
    pub total_breakpoints: u32,
    pub total_watchpoints: u32,
}

/// Main ptrace v3 bridge
pub struct BridgePtraceV3 {
    sessions: BTreeMap<u64, PtraceV3Session>,
    next_id: u64,
}

impl BridgePtraceV3 {
    pub fn new() -> Self { Self { sessions: BTreeMap::new(), next_id: 1 } }

    pub fn attach(&mut self, tracer: u64, tracee: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.sessions.insert(id, PtraceV3Session::new(tracer, tracee));
        id
    }

    pub fn detach(&mut self, id: u64) {
        if let Some(s) = self.sessions.get_mut(&id) { s.attached = false; }
    }

    pub fn add_watchpoint(&mut self, id: u64, addr: u64, size: u64) {
        if let Some(s) = self.sessions.get_mut(&id) { s.watchpoints.push((addr, size)); }
    }

    pub fn stats(&self) -> PtraceV3BridgeStats {
        let active = self.sessions.values().filter(|s| s.attached).count() as u32;
        let events: u64 = self.sessions.values().map(|s| s.events.len() as u64).sum();
        let bps: u32 = self.sessions.values().map(|s| s.breakpoints.len() as u32).sum();
        let wps: u32 = self.sessions.values().map(|s| s.watchpoints.len() as u32).sum();
        PtraceV3BridgeStats { total_sessions: self.sessions.len() as u32, active_sessions: active, total_events: events, total_breakpoints: bps, total_watchpoints: wps }
    }
}

// ============================================================================
// Merged from ptrace_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtraceV4Request {
    TraceMe,
    PeekText,
    PeekData,
    PeekUser,
    PokeText,
    PokeData,
    PokeUser,
    GetRegs,
    SetRegs,
    GetFpRegs,
    SetFpRegs,
    Attach,
    Detach,
    Syscall,
    SingleStep,
    Cont,
    Kill,
    Seize,
    Interrupt,
    Listen,
    GetSigInfo,
    SetSigInfo,
    GetRegSet,
    SetRegSet,
    SeccompGetFilter,
}

/// Ptrace stop reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtraceV4Stop {
    SignalDelivery,
    SyscallEntry,
    SyscallExit,
    Clone,
    Exec,
    Exit,
    Seccomp,
    Vfork,
    VforkDone,
    GroupStop,
}

/// Register set for a traced process
#[derive(Debug, Clone)]
pub struct PtraceV4Regs {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub rip: u64,
    pub rflags: u64,
    pub orig_rax: u64,
}

/// A traced process
#[derive(Debug, Clone)]
pub struct PtraceV4Tracee {
    pub pid: u64,
    pub tracer_pid: u64,
    pub stop_reason: Option<PtraceV4Stop>,
    pub regs: PtraceV4Regs,
    pub stopped: bool,
    pub seized: bool,
    pub syscall_trace: bool,
    pub single_step: bool,
    pub trace_count: u64,
    pub syscall_count: u64,
    pub signal_count: u64,
}

impl PtraceV4Tracee {
    pub fn new(pid: u64, tracer: u64) -> Self {
        Self {
            pid,
            tracer_pid: tracer,
            stop_reason: None,
            regs: PtraceV4Regs {
                rax: 0, rbx: 0, rcx: 0, rdx: 0,
                rsi: 0, rdi: 0, rbp: 0, rsp: 0,
                rip: 0, rflags: 0, orig_rax: 0,
            },
            stopped: false,
            seized: false,
            syscall_trace: false,
            single_step: false,
            trace_count: 0,
            syscall_count: 0,
            signal_count: 0,
        }
    }

    pub fn stop(&mut self, reason: PtraceV4Stop) {
        self.stopped = true;
        self.stop_reason = Some(reason);
        self.trace_count += 1;
        match reason {
            PtraceV4Stop::SyscallEntry | PtraceV4Stop::SyscallExit => self.syscall_count += 1,
            PtraceV4Stop::SignalDelivery => self.signal_count += 1,
            _ => {}
        }
    }

    pub fn resume(&mut self) {
        self.stopped = false;
        self.stop_reason = None;
    }
}

/// Statistics for ptrace V4 bridge
#[derive(Debug, Clone)]
pub struct PtraceV4BridgeStats {
    pub attach_count: u64,
    pub detach_count: u64,
    pub syscall_stops: u64,
    pub signal_stops: u64,
    pub peek_ops: u64,
    pub poke_ops: u64,
    pub reg_reads: u64,
    pub reg_writes: u64,
    pub seize_count: u64,
}

/// Main ptrace V4 bridge manager
#[derive(Debug)]
pub struct BridgePtraceV4 {
    tracees: BTreeMap<u64, PtraceV4Tracee>,
    stats: PtraceV4BridgeStats,
}

impl BridgePtraceV4 {
    pub fn new() -> Self {
        Self {
            tracees: BTreeMap::new(),
            stats: PtraceV4BridgeStats {
                attach_count: 0,
                detach_count: 0,
                syscall_stops: 0,
                signal_stops: 0,
                peek_ops: 0,
                poke_ops: 0,
                reg_reads: 0,
                reg_writes: 0,
                seize_count: 0,
            },
        }
    }

    pub fn attach(&mut self, pid: u64, tracer: u64) -> bool {
        if self.tracees.contains_key(&pid) {
            return false;
        }
        self.tracees.insert(pid, PtraceV4Tracee::new(pid, tracer));
        self.stats.attach_count += 1;
        true
    }

    pub fn seize(&mut self, pid: u64, tracer: u64) -> bool {
        if self.tracees.contains_key(&pid) {
            return false;
        }
        let mut tracee = PtraceV4Tracee::new(pid, tracer);
        tracee.seized = true;
        self.tracees.insert(pid, tracee);
        self.stats.seize_count += 1;
        true
    }

    pub fn detach(&mut self, pid: u64) -> bool {
        if self.tracees.remove(&pid).is_some() {
            self.stats.detach_count += 1;
            true
        } else {
            false
        }
    }

    pub fn get_regs(&mut self, pid: u64) -> Option<PtraceV4Regs> {
        if let Some(tracee) = self.tracees.get(&pid) {
            self.stats.reg_reads += 1;
            Some(tracee.regs.clone())
        } else {
            None
        }
    }

    pub fn set_regs(&mut self, pid: u64, regs: PtraceV4Regs) -> bool {
        if let Some(tracee) = self.tracees.get_mut(&pid) {
            tracee.regs = regs;
            self.stats.reg_writes += 1;
            true
        } else {
            false
        }
    }

    pub fn cont(&mut self, pid: u64) -> bool {
        if let Some(tracee) = self.tracees.get_mut(&pid) {
            tracee.resume();
            true
        } else {
            false
        }
    }

    pub fn stats(&self) -> &PtraceV4BridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from ptrace_v5_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtraceV5Request {
    PeekText,
    PeekData,
    PokeText,
    PokeData,
    GetRegs,
    SetRegs,
    GetFpRegs,
    SetFpRegs,
    Attach,
    Detach,
    Syscall,
    SingleStep,
    GetSigInfo,
    SetSigInfo,
    Seize,
    Interrupt,
    Listen,
    GetSigMask,
    SetSigMask,
    GetRegSet,
    SetRegSet,
    SeccompGetFilter,
    SeccompGetMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtraceV5StopKind {
    Signal(u32),
    SyscallEntry,
    SyscallExit,
    Clone,
    Exec,
    Exit,
    Seccomp,
    Vfork,
    VforkDone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtraceV5HwType {
    Breakpoint,
    Watchpoint,
    SingleStep,
    BranchTrace,
    PerfCounter,
}

#[derive(Debug, Clone)]
pub struct PtraceV5HwBreakpoint {
    pub bp_type: PtraceV5HwType,
    pub address: u64,
    pub length: u32,
    pub enabled: bool,
    pub hit_count: u64,
}

impl PtraceV5HwBreakpoint {
    pub fn new(bp_type: PtraceV5HwType, address: u64, length: u32) -> Self {
        Self { bp_type, address, length, enabled: true, hit_count: 0 }
    }

    pub fn hit(&mut self) { self.hit_count += 1; }
    pub fn contains_addr(&self, addr: u64) -> bool {
        addr >= self.address && addr < self.address + self.length as u64
    }
}

#[derive(Debug, Clone)]
pub struct PtraceV5TraceeState {
    pub pid: u64,
    pub tracer_pid: u64,
    pub seized: bool,
    pub stop_kind: Option<PtraceV5StopKind>,
    pub hw_breakpoints: Vec<PtraceV5HwBreakpoint>,
    pub syscall_trace: bool,
    pub total_stops: u64,
    pub total_requests: u64,
}

impl PtraceV5TraceeState {
    pub fn new(pid: u64, tracer: u64) -> Self {
        Self {
            pid,
            tracer_pid: tracer,
            seized: false,
            stop_kind: None,
            hw_breakpoints: Vec::new(),
            syscall_trace: false,
            total_stops: 0,
            total_requests: 0,
        }
    }

    pub fn add_hw_breakpoint(&mut self, bp: PtraceV5HwBreakpoint) -> bool {
        if self.hw_breakpoints.len() >= 4 { return false; }
        self.hw_breakpoints.push(bp);
        true
    }

    pub fn check_breakpoint(&mut self, addr: u64) -> bool {
        for bp in &mut self.hw_breakpoints {
            if bp.enabled && bp.contains_addr(addr) {
                bp.hit();
                return true;
            }
        }
        false
    }

    pub fn stop(&mut self, kind: PtraceV5StopKind) {
        self.stop_kind = Some(kind);
        self.total_stops += 1;
    }
}

#[derive(Debug, Clone)]
pub struct PtraceV5BridgeStats {
    pub total_tracees: u64,
    pub total_requests: u64,
    pub total_stops: u64,
    pub hw_breakpoints_set: u64,
    pub hw_breakpoint_hits: u64,
}

pub struct BridgePtraceV5 {
    tracees: BTreeMap<u64, PtraceV5TraceeState>,
    stats: PtraceV5BridgeStats,
}

impl BridgePtraceV5 {
    pub fn new() -> Self {
        Self {
            tracees: BTreeMap::new(),
            stats: PtraceV5BridgeStats {
                total_tracees: 0,
                total_requests: 0,
                total_stops: 0,
                hw_breakpoints_set: 0,
                hw_breakpoint_hits: 0,
            },
        }
    }

    pub fn attach(&mut self, pid: u64, tracer: u64) {
        let tracee = PtraceV5TraceeState::new(pid, tracer);
        self.tracees.insert(pid, tracee);
        self.stats.total_tracees += 1;
    }

    pub fn seize(&mut self, pid: u64, tracer: u64) {
        let mut tracee = PtraceV5TraceeState::new(pid, tracer);
        tracee.seized = true;
        self.tracees.insert(pid, tracee);
        self.stats.total_tracees += 1;
    }

    pub fn detach(&mut self, pid: u64) {
        self.tracees.remove(&pid);
        if self.stats.total_tracees > 0 {
            self.stats.total_tracees -= 1;
        }
    }

    pub fn stats(&self) -> &PtraceV5BridgeStats {
        &self.stats
    }
}
