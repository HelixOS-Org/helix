// SPDX-License-Identifier: GPL-2.0
//! Bridge pidfd_bridge — pidfd (process file descriptor) bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Pidfd flags
#[derive(Debug, Clone, Copy)]
pub struct PidfdFlags(pub u32);

impl PidfdFlags {
    pub const NONBLOCK: Self = Self(0x01);
    pub const THREAD: Self = Self(0x02);

    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn is_thread(&self) -> bool {
        self.contains(Self::THREAD)
    }
}

/// Pidfd signal info
#[derive(Debug, Clone, Copy)]
pub struct PidfdSignalInfo {
    pub signal: i32,
    pub sender_pid: u32,
    pub timestamp: u64,
}

/// A pidfd instance
#[derive(Debug)]
pub struct PidfdInstance {
    pub fd: i32,
    pub target_pid: u32,
    pub owner_pid: u32,
    pub flags: PidfdFlags,
    pub created: u64,
    pub last_poll: u64,
    pub poll_count: u64,
    pub signal_count: u64,
    pub wait_count: u64,
    pub target_alive: bool,
    pub target_exit_code: Option<i32>,
}

impl PidfdInstance {
    pub fn new(fd: i32, target: u32, owner: u32, flags: PidfdFlags, now: u64) -> Self {
        Self {
            fd, target_pid: target, owner_pid: owner,
            flags, created: now, last_poll: 0,
            poll_count: 0, signal_count: 0, wait_count: 0,
            target_alive: true, target_exit_code: None,
        }
    }

    pub fn send_signal(&mut self, _signal: i32) -> bool {
        if !self.target_alive { return false; }
        self.signal_count += 1;
        true
    }

    pub fn poll(&mut self, now: u64) -> bool {
        self.poll_count += 1;
        self.last_poll = now;
        !self.target_alive
    }

    pub fn mark_exited(&mut self, exit_code: i32) {
        self.target_alive = false;
        self.target_exit_code = Some(exit_code);
    }

    pub fn waitid(&mut self) -> Option<i32> {
        self.wait_count += 1;
        self.target_exit_code
    }

    pub fn idle_time(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_poll.max(self.created))
    }

    pub fn is_stale(&self, now: u64, threshold: u64) -> bool {
        !self.target_alive && self.idle_time(now) > threshold
    }
}

/// Pidfd operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidfdOp {
    Open,
    SendSignal,
    Poll,
    Wait,
    GetFd,
    Close,
}

/// Pidfd event
#[derive(Debug, Clone)]
pub struct PidfdEvent {
    pub fd: i32,
    pub op: PidfdOp,
    pub target_pid: u32,
    pub owner_pid: u32,
    pub result: bool,
    pub timestamp: u64,
}

/// Per-process pidfd tracking
#[derive(Debug)]
pub struct ProcessPidfdState {
    pub pid: u32,
    pub owned_pidfds: Vec<i32>,
    pub watched_by: Vec<i32>,
    pub max_pidfds: u32,
}

impl ProcessPidfdState {
    pub fn new(pid: u32) -> Self {
        Self {
            pid, owned_pidfds: Vec::new(),
            watched_by: Vec::new(),
            max_pidfds: 256,
        }
    }

    pub fn is_watched(&self) -> bool {
        !self.watched_by.is_empty()
    }

    pub fn watcher_count(&self) -> usize {
        self.watched_by.len()
    }

    pub fn can_create(&self) -> bool {
        self.owned_pidfds.len() < self.max_pidfds as usize
    }
}

/// Pidfd bridge stats
#[derive(Debug, Clone)]
pub struct PidfdBridgeStats {
    pub active_pidfds: u32,
    pub total_created: u64,
    pub total_signals_sent: u64,
    pub total_polls: u64,
    pub total_waits: u64,
    pub stale_pidfds: u32,
}

/// Main pidfd bridge
pub struct BridgePidfd {
    instances: BTreeMap<i32, PidfdInstance>,
    process_states: BTreeMap<u32, ProcessPidfdState>,
    events: Vec<PidfdEvent>,
    max_events: usize,
    stats: PidfdBridgeStats,
}

impl BridgePidfd {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            process_states: BTreeMap::new(),
            events: Vec::new(),
            max_events: 2048,
            stats: PidfdBridgeStats {
                active_pidfds: 0, total_created: 0,
                total_signals_sent: 0, total_polls: 0,
                total_waits: 0, stale_pidfds: 0,
            },
        }
    }

    pub fn open(&mut self, fd: i32, target: u32, owner: u32, flags: PidfdFlags, now: u64) {
        let inst = PidfdInstance::new(fd, target, owner, flags, now);
        self.stats.total_created += 1;
        self.stats.active_pidfds += 1;

        let owner_state = self.process_states.entry(owner)
            .or_insert_with(|| ProcessPidfdState::new(owner));
        owner_state.owned_pidfds.push(fd);

        let target_state = self.process_states.entry(target)
            .or_insert_with(|| ProcessPidfdState::new(target));
        target_state.watched_by.push(fd);

        self.instances.insert(fd, inst);
    }

    pub fn send_signal(&mut self, fd: i32, signal: i32) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            let ok = inst.send_signal(signal);
            if ok { self.stats.total_signals_sent += 1; }
            ok
        } else { false }
    }

    pub fn poll(&mut self, fd: i32, now: u64) -> Option<bool> {
        self.stats.total_polls += 1;
        self.instances.get_mut(&fd).map(|inst| inst.poll(now))
    }

    pub fn waitid(&mut self, fd: i32) -> Option<i32> {
        self.stats.total_waits += 1;
        self.instances.get_mut(&fd).and_then(|inst| inst.waitid())
    }

    pub fn process_exited(&mut self, pid: u32, exit_code: i32) {
        for inst in self.instances.values_mut() {
            if inst.target_pid == pid {
                inst.mark_exited(exit_code);
            }
        }
    }

    pub fn close(&mut self, fd: i32) -> bool {
        if let Some(inst) = self.instances.remove(&fd) {
            if self.stats.active_pidfds > 0 { self.stats.active_pidfds -= 1; }
            if let Some(state) = self.process_states.get_mut(&inst.owner_pid) {
                state.owned_pidfds.retain(|&f| f != fd);
            }
            if let Some(state) = self.process_states.get_mut(&inst.target_pid) {
                state.watched_by.retain(|&f| f != fd);
            }
            true
        } else { false }
    }

    pub fn record_event(&mut self, event: PidfdEvent) {
        if self.events.len() >= self.max_events { self.events.remove(0); }
        self.events.push(event);
    }

    pub fn stale_pidfds(&self, now: u64, threshold: u64) -> Vec<i32> {
        self.instances.iter()
            .filter(|(_, inst)| inst.is_stale(now, threshold))
            .map(|(&fd, _)| fd)
            .collect()
    }

    pub fn most_watched_processes(&self, n: usize) -> Vec<(u32, usize)> {
        let mut v: Vec<_> = self.process_states.iter()
            .filter(|(_, s)| s.is_watched())
            .map(|(&pid, s)| (pid, s.watcher_count()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    pub fn stats(&self) -> &PidfdBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from pidfd_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidfdV2Flag {
    NonBlock,
    CloseOnExec,
}

/// Pidfd state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidfdV2State {
    Open,
    Signaled,
    Waited,
    Closed,
}

/// Pidfd entry
#[derive(Debug)]
pub struct PidfdV2Entry {
    pub fd: u64,
    pub target_pid: u64,
    pub owner_pid: u64,
    pub state: PidfdV2State,
    pub flags: u32,
    pub signals_sent: u64,
    pub created_at: u64,
}

impl PidfdV2Entry {
    pub fn new(fd: u64, target: u64, owner: u64, flags: u32, now: u64) -> Self {
        Self { fd, target_pid: target, owner_pid: owner, state: PidfdV2State::Open, flags, signals_sent: 0, created_at: now }
    }
}

/// Pidfd send signal record
#[derive(Debug)]
pub struct PidfdSignalV2 {
    pub fd: u64,
    pub signal: u32,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct PidfdV2BridgeStats {
    pub total_pidfds: u32,
    pub open_pidfds: u32,
    pub total_signals_sent: u64,
}

/// Main bridge pidfd v2
pub struct BridgePidfdV2 {
    entries: BTreeMap<u64, PidfdV2Entry>,
}

impl BridgePidfdV2 {
    pub fn new() -> Self { Self { entries: BTreeMap::new() } }

    pub fn open(&mut self, fd: u64, target: u64, owner: u64, flags: u32, now: u64) {
        self.entries.insert(fd, PidfdV2Entry::new(fd, target, owner, flags, now));
    }

    pub fn send_signal(&mut self, fd: u64, signal: u32) -> bool {
        if let Some(e) = self.entries.get_mut(&fd) {
            if e.state == PidfdV2State::Open { e.signals_sent += 1; e.state = PidfdV2State::Signaled; return true; }
        }
        false
    }

    pub fn close(&mut self, fd: u64) {
        if let Some(e) = self.entries.get_mut(&fd) { e.state = PidfdV2State::Closed; }
    }

    pub fn stats(&self) -> PidfdV2BridgeStats {
        let open = self.entries.values().filter(|e| e.state == PidfdV2State::Open || e.state == PidfdV2State::Signaled).count() as u32;
        let sigs: u64 = self.entries.values().map(|e| e.signals_sent).sum();
        PidfdV2BridgeStats { total_pidfds: self.entries.len() as u32, open_pidfds: open, total_signals_sent: sigs }
    }
}

// ============================================================================
// Merged from pidfd_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidfdV3Flag {
    NonBlock,
    CloseExec,
    Thread,
}

/// Process state as seen through pidfd.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidfdV3ProcState {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Dead,
    TraceStopped,
    Unknown,
}

/// Wait result via pidfd.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PidfdV3WaitResult {
    Exited(i32),
    Signaled(i32),
    Stopped(i32),
    Continued,
    StillRunning,
}

/// A pidfd instance.
#[derive(Debug, Clone)]
pub struct PidfdV3Instance {
    pub pidfd_id: u64,
    pub fd: i32,
    pub target_pid: u64,
    pub flags: Vec<PidfdV3Flag>,
    pub proc_state: PidfdV3ProcState,
    pub signal_sent_count: u64,
    pub wait_count: u64,
    pub getfd_count: u64,
    pub poll_ready: bool,
    pub owner_pid: u64,
    pub ns_level: u32,
}

impl PidfdV3Instance {
    pub fn new(pidfd_id: u64, fd: i32, target_pid: u64) -> Self {
        Self {
            pidfd_id,
            fd,
            target_pid,
            flags: Vec::new(),
            proc_state: PidfdV3ProcState::Running,
            signal_sent_count: 0,
            wait_count: 0,
            getfd_count: 0,
            poll_ready: false,
            owner_pid: 0,
            ns_level: 0,
        }
    }

    pub fn send_signal(&mut self, _sig: i32) -> bool {
        if self.proc_state == PidfdV3ProcState::Dead {
            return false;
        }
        self.signal_sent_count += 1;
        true
    }

    pub fn wait(&mut self) -> PidfdV3WaitResult {
        self.wait_count += 1;
        match self.proc_state {
            PidfdV3ProcState::Zombie => PidfdV3WaitResult::Exited(0),
            PidfdV3ProcState::Dead => PidfdV3WaitResult::Exited(-1),
            PidfdV3ProcState::Stopped => PidfdV3WaitResult::Stopped(19),
            _ => PidfdV3WaitResult::StillRunning,
        }
    }

    pub fn getfd(&mut self, _target_fd: i32) -> Option<i32> {
        if self.proc_state == PidfdV3ProcState::Dead {
            return None;
        }
        self.getfd_count += 1;
        Some(self.getfd_count as i32 + 100)
    }

    pub fn is_alive(&self) -> bool {
        !matches!(
            self.proc_state,
            PidfdV3ProcState::Dead | PidfdV3ProcState::Zombie
        )
    }
}

/// Statistics for pidfd V3 bridge.
#[derive(Debug, Clone)]
pub struct PidfdV3BridgeStats {
    pub total_pidfds: u64,
    pub total_signals_sent: u64,
    pub total_waits: u64,
    pub total_getfds: u64,
    pub active_pidfds: u64,
    pub zombie_detected: u64,
}

/// Main bridge pidfd V3 manager.
pub struct BridgePidfdV3 {
    pub instances: BTreeMap<u64, PidfdV3Instance>,
    pub pid_map: BTreeMap<u64, u64>, // target_pid → pidfd_id
    pub next_id: u64,
    pub stats: PidfdV3BridgeStats,
}

impl BridgePidfdV3 {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            pid_map: BTreeMap::new(),
            next_id: 1,
            stats: PidfdV3BridgeStats {
                total_pidfds: 0,
                total_signals_sent: 0,
                total_waits: 0,
                total_getfds: 0,
                active_pidfds: 0,
                zombie_detected: 0,
            },
        }
    }

    pub fn open_pidfd(&mut self, fd: i32, target_pid: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let inst = PidfdV3Instance::new(id, fd, target_pid);
        self.pid_map.insert(target_pid, id);
        self.instances.insert(id, inst);
        self.stats.total_pidfds += 1;
        self.stats.active_pidfds += 1;
        id
    }

    pub fn send_signal(&mut self, pidfd_id: u64, sig: i32) -> bool {
        if let Some(inst) = self.instances.get_mut(&pidfd_id) {
            let ok = inst.send_signal(sig);
            if ok {
                self.stats.total_signals_sent += 1;
            }
            ok
        } else {
            false
        }
    }

    pub fn wait_pidfd(&mut self, pidfd_id: u64) -> Option<PidfdV3WaitResult> {
        if let Some(inst) = self.instances.get_mut(&pidfd_id) {
            let result = inst.wait();
            self.stats.total_waits += 1;
            Some(result)
        } else {
            None
        }
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}
