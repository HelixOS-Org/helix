// SPDX-License-Identifier: GPL-2.0
//! Apps eventfd_app — event file descriptor management.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Eventfd flags
#[derive(Debug, Clone, Copy)]
pub struct EventfdFlags(pub u32);

impl EventfdFlags {
    pub const SEMAPHORE: u32 = 1;
    pub const NONBLOCK: u32 = 2;
    pub const CLOEXEC: u32 = 4;
    pub fn new() -> Self { Self(0) }
    #[inline(always)]
    pub fn set(&mut self, f: u32) { self.0 |= f; }
    #[inline(always)]
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
    #[inline(always)]
    pub fn is_semaphore(&self) -> bool { self.has(Self::SEMAPHORE) }
}

/// Eventfd instance
#[derive(Debug)]
pub struct EventfdInstance {
    pub id: u64,
    pub counter: u64,
    pub flags: EventfdFlags,
    pub owner_pid: u64,
    pub write_count: u64,
    pub read_count: u64,
    pub waiters: u32,
    pub created_at: u64,
}

impl EventfdInstance {
    pub fn new(id: u64, initval: u64, flags: EventfdFlags, pid: u64, now: u64) -> Self {
        Self { id, counter: initval, flags, owner_pid: pid, write_count: 0, read_count: 0, waiters: 0, created_at: now }
    }

    #[inline(always)]
    pub fn write(&mut self, val: u64) -> bool {
        if self.counter.checked_add(val).map_or(true, |v| v == u64::MAX) { return false; }
        self.counter += val; self.write_count += 1; true
    }

    #[inline]
    pub fn read(&mut self) -> u64 {
        self.read_count += 1;
        if self.flags.is_semaphore() {
            if self.counter > 0 { self.counter -= 1; 1 } else { 0 }
        } else {
            let val = self.counter; self.counter = 0; val
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventfdAppStats {
    pub total_instances: u32,
    pub semaphore_instances: u32,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_waiters: u32,
}

/// Main eventfd app
pub struct AppEventfd {
    instances: BTreeMap<u64, EventfdInstance>,
    next_id: u64,
}

impl AppEventfd {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, initval: u64, flags: EventfdFlags, pid: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.instances.insert(id, EventfdInstance::new(id, initval, flags, pid, now));
        id
    }

    #[inline(always)]
    pub fn close(&mut self, id: u64) { self.instances.remove(&id); }
    #[inline(always)]
    pub fn write(&mut self, id: u64, val: u64) -> bool { self.instances.get_mut(&id).map_or(false, |e| e.write(val)) }
    #[inline(always)]
    pub fn read(&mut self, id: u64) -> Option<u64> { Some(self.instances.get_mut(&id)?.read()) }

    #[inline]
    pub fn stats(&self) -> EventfdAppStats {
        let sems = self.instances.values().filter(|e| e.flags.is_semaphore()).count() as u32;
        let writes: u64 = self.instances.values().map(|e| e.write_count).sum();
        let reads: u64 = self.instances.values().map(|e| e.read_count).sum();
        let waiters: u32 = self.instances.values().map(|e| e.waiters).sum();
        EventfdAppStats { total_instances: self.instances.len() as u32, semaphore_instances: sems, total_writes: writes, total_reads: reads, total_waiters: waiters }
    }
}

// ============================================================================
// Merged from eventfd_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdV2Flag {
    Semaphore,
    NonBlock,
    CloseOnExec,
}

/// Eventfd v2 state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdV2State {
    Idle,
    Signaled,
    Blocked,
    Closed,
}

/// Eventfd v2 instance
#[derive(Debug)]
pub struct EventfdV2Instance {
    pub id: u64,
    pub counter: u64,
    pub flags: u32,
    pub semaphore: bool,
    pub state: EventfdV2State,
    pub write_count: u64,
    pub read_count: u64,
    pub created_at: u64,
}

impl EventfdV2Instance {
    pub fn new(id: u64, initial: u64, semaphore: bool, now: u64) -> Self {
        Self { id, counter: initial, flags: 0, semaphore, state: EventfdV2State::Idle, write_count: 0, read_count: 0, created_at: now }
    }

    #[inline]
    pub fn write(&mut self, val: u64) -> bool {
        if self.counter.checked_add(val).map_or(true, |v| v == u64::MAX) { return false; }
        self.counter += val;
        self.write_count += 1;
        self.state = EventfdV2State::Signaled;
        true
    }

    pub fn read(&mut self) -> u64 {
        self.read_count += 1;
        if self.semaphore {
            if self.counter > 0 { self.counter -= 1; 1 }
            else { self.state = EventfdV2State::Blocked; 0 }
        } else {
            let v = self.counter;
            self.counter = 0;
            if v == 0 { self.state = EventfdV2State::Blocked; }
            else { self.state = EventfdV2State::Idle; }
            v
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventfdV2AppStats {
    pub total_instances: u32,
    pub total_writes: u64,
    pub total_reads: u64,
    pub signaled_count: u32,
}

/// Main app eventfd v2
pub struct AppEventfdV2 {
    instances: BTreeMap<u64, EventfdV2Instance>,
    next_id: u64,
}

impl AppEventfdV2 {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, initial: u64, semaphore: bool, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.instances.insert(id, EventfdV2Instance::new(id, initial, semaphore, now));
        id
    }

    #[inline(always)]
    pub fn write(&mut self, id: u64, val: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&id) { inst.write(val) }
        else { false }
    }

    #[inline(always)]
    pub fn read(&mut self, id: u64) -> u64 {
        if let Some(inst) = self.instances.get_mut(&id) { inst.read() }
        else { 0 }
    }

    #[inline(always)]
    pub fn close(&mut self, id: u64) {
        if let Some(inst) = self.instances.get_mut(&id) { inst.state = EventfdV2State::Closed; }
    }

    #[inline]
    pub fn stats(&self) -> EventfdV2AppStats {
        let writes: u64 = self.instances.values().map(|i| i.write_count).sum();
        let reads: u64 = self.instances.values().map(|i| i.read_count).sum();
        let signaled = self.instances.values().filter(|i| i.state == EventfdV2State::Signaled).count() as u32;
        EventfdV2AppStats { total_instances: self.instances.len() as u32, total_writes: writes, total_reads: reads, signaled_count: signaled }
    }
}

// ============================================================================
// Merged from eventfd_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdV3AppOp { Create, Read, Write }

/// Eventfd v3 app flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdV3AppFlag { None, Nonblock, Cloexec, Semaphore }

/// Eventfd v3 app record
#[derive(Debug, Clone)]
pub struct EventfdV3AppRecord {
    pub op: EventfdV3AppOp,
    pub flags: u32,
    pub value: u64,
    pub fd: i32,
    pub pid: u32,
}

impl EventfdV3AppRecord {
    pub fn new(op: EventfdV3AppOp) -> Self { Self { op, flags: 0, value: 0, fd: -1, pid: 0 } }
}

/// Eventfd v3 app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventfdV3AppStats { pub total_ops: u64, pub created: u64, pub reads: u64, pub writes: u64 }

/// Main app eventfd v3
#[derive(Debug)]
pub struct AppEventfdV3 { pub stats: EventfdV3AppStats }

impl AppEventfdV3 {
    pub fn new() -> Self { Self { stats: EventfdV3AppStats { total_ops: 0, created: 0, reads: 0, writes: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &EventfdV3AppRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            EventfdV3AppOp::Create => self.stats.created += 1,
            EventfdV3AppOp::Read => self.stats.reads += 1,
            EventfdV3AppOp::Write => self.stats.writes += 1,
        }
    }
}
