// SPDX-License-Identifier: GPL-2.0
//! Bridge eventfd_bridge — eventfd interface bridge for event notification.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Eventfd flags
#[derive(Debug, Clone, Copy)]
pub struct EventfdFlags(pub u32);

impl EventfdFlags {
    pub const CLOEXEC: Self = Self(0x01);
    pub const NONBLOCK: Self = Self(0x02);
    pub const SEMAPHORE: Self = Self(0x04);

    #[inline(always)]
    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    #[inline(always)]
    pub fn is_semaphore(&self) -> bool {
        self.contains(Self::SEMAPHORE)
    }
}

/// An eventfd instance
#[derive(Debug)]
pub struct EventfdInstance {
    pub fd: i32,
    pub counter: u64,
    pub flags: EventfdFlags,
    pub owner_pid: u32,
    pub write_count: u64,
    pub read_count: u64,
    pub poll_count: u64,
    pub overflow_count: u64,
    pub waiters: u32,
    pub create_timestamp: u64,
    pub last_write: u64,
    pub last_read: u64,
}

impl EventfdInstance {
    pub fn new(fd: i32, flags: EventfdFlags, owner_pid: u32, now: u64) -> Self {
        Self {
            fd, counter: 0, flags, owner_pid,
            write_count: 0, read_count: 0, poll_count: 0,
            overflow_count: 0, waiters: 0,
            create_timestamp: now, last_write: 0, last_read: 0,
        }
    }

    #[inline]
    pub fn write(&mut self, value: u64, now: u64) -> bool {
        let max = u64::MAX - 1;
        if self.counter > max - value {
            self.overflow_count += 1;
            return false;
        }
        self.counter += value;
        self.write_count += 1;
        self.last_write = now;
        true
    }

    pub fn read(&mut self, now: u64) -> u64 {
        let val = if self.flags.is_semaphore() {
            if self.counter > 0 { self.counter -= 1; 1 } else { 0 }
        } else {
            let v = self.counter;
            self.counter = 0;
            v
        };
        if val > 0 {
            self.read_count += 1;
            self.last_read = now;
        }
        val
    }

    #[inline(always)]
    pub fn is_ready(&self) -> bool {
        self.counter > 0
    }

    #[inline(always)]
    pub fn idle_time(&self, now: u64) -> u64 {
        let last = self.last_write.max(self.last_read);
        now.saturating_sub(last)
    }

    #[inline]
    pub fn activity_ratio(&self) -> f64 {
        let total = self.write_count + self.read_count;
        if total == 0 { return 0.0; }
        self.read_count as f64 / self.write_count.max(1) as f64
    }
}

/// Eventfd operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdOp {
    Create,
    Write,
    Read,
    Poll,
    Close,
}

/// Eventfd event record
#[derive(Debug, Clone)]
pub struct EventfdEvent {
    pub fd: i32,
    pub op: EventfdOp,
    pub value: u64,
    pub pid: u32,
    pub timestamp: u64,
}

/// Eventfd bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventfdBridgeStats {
    pub active_eventfds: u32,
    pub total_created: u64,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_overflows: u64,
    pub semaphore_count: u32,
}

/// Main eventfd bridge
#[repr(align(64))]
pub struct BridgeEventfd {
    instances: BTreeMap<i32, EventfdInstance>,
    events: VecDeque<EventfdEvent>,
    max_events: usize,
    stats: EventfdBridgeStats,
}

impl BridgeEventfd {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            events: VecDeque::new(),
            max_events: 4096,
            stats: EventfdBridgeStats {
                active_eventfds: 0, total_created: 0,
                total_writes: 0, total_reads: 0,
                total_overflows: 0, semaphore_count: 0,
            },
        }
    }

    #[inline]
    pub fn create(&mut self, fd: i32, flags: EventfdFlags, pid: u32, now: u64) {
        let inst = EventfdInstance::new(fd, flags, pid, now);
        self.stats.total_created += 1;
        self.stats.active_eventfds += 1;
        if flags.is_semaphore() { self.stats.semaphore_count += 1; }
        self.instances.insert(fd, inst);
    }

    #[inline]
    pub fn write_eventfd(&mut self, fd: i32, value: u64, now: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            let ok = inst.write(value, now);
            self.stats.total_writes += 1;
            if !ok { self.stats.total_overflows += 1; }
            ok
        } else { false }
    }

    #[inline]
    pub fn read_eventfd(&mut self, fd: i32, now: u64) -> Option<u64> {
        if let Some(inst) = self.instances.get_mut(&fd) {
            let val = inst.read(now);
            self.stats.total_reads += 1;
            Some(val)
        } else { None }
    }

    #[inline]
    pub fn close(&mut self, fd: i32) -> bool {
        if let Some(inst) = self.instances.remove(&fd) {
            if self.stats.active_eventfds > 0 { self.stats.active_eventfds -= 1; }
            if inst.flags.is_semaphore() && self.stats.semaphore_count > 0 {
                self.stats.semaphore_count -= 1;
            }
            true
        } else { false }
    }

    #[inline(always)]
    pub fn record_event(&mut self, event: EventfdEvent) {
        if self.events.len() >= self.max_events { self.events.pop_front(); }
        self.events.push_back(event);
    }

    #[inline]
    pub fn idle_eventfds(&self, now: u64, threshold: u64) -> Vec<i32> {
        self.instances.iter()
            .filter(|(_, inst)| inst.idle_time(now) > threshold)
            .map(|(&fd, _)| fd)
            .collect()
    }

    #[inline]
    pub fn busiest_eventfds(&self, n: usize) -> Vec<(i32, u64)> {
        let mut v: Vec<_> = self.instances.iter()
            .map(|(&fd, inst)| (fd, inst.write_count + inst.read_count))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    #[inline(always)]
    pub fn get_instance(&self, fd: i32) -> Option<&EventfdInstance> {
        self.instances.get(&fd)
    }

    #[inline(always)]
    pub fn stats(&self) -> &EventfdBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from eventfd_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdV2Flag {
    CloseExec,
    NonBlock,
    Semaphore,
}

/// Eventfd state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdV2State {
    Idle,
    Signaled,
    Waiting,
    Closed,
}

/// An eventfd V2 instance.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventfdV2Instance {
    pub efd_id: u64,
    pub fd: i32,
    pub counter: u64,
    pub flags: Vec<EventfdV2Flag>,
    pub state: EventfdV2State,
    pub is_semaphore: bool,
    pub write_count: u64,
    pub read_count: u64,
    pub overflow_count: u64,
    pub poll_waiters: u32,
    pub owner_pid: u64,
}

impl EventfdV2Instance {
    pub fn new(efd_id: u64, fd: i32, initial: u64) -> Self {
        Self {
            efd_id,
            fd,
            counter: initial,
            flags: Vec::new(),
            state: if initial > 0 {
                EventfdV2State::Signaled
            } else {
                EventfdV2State::Idle
            },
            is_semaphore: false,
            write_count: 0,
            read_count: 0,
            overflow_count: 0,
            poll_waiters: 0,
            owner_pid: 0,
        }
    }

    pub fn write(&mut self, value: u64) -> bool {
        let new_val = self.counter.checked_add(value);
        if let Some(v) = new_val {
            if v > u64::MAX - 1 {
                self.overflow_count += 1;
                return false;
            }
            self.counter = v;
            self.write_count += 1;
            self.state = EventfdV2State::Signaled;
            true
        } else {
            self.overflow_count += 1;
            false
        }
    }

    pub fn read(&mut self) -> u64 {
        if self.is_semaphore {
            if self.counter > 0 {
                self.counter -= 1;
                self.read_count += 1;
                if self.counter == 0 {
                    self.state = EventfdV2State::Idle;
                }
                1
            } else {
                0
            }
        } else {
            let val = self.counter;
            self.counter = 0;
            self.read_count += 1;
            self.state = EventfdV2State::Idle;
            val
        }
    }

    #[inline(always)]
    pub fn is_readable(&self) -> bool {
        self.counter > 0
    }

    #[inline(always)]
    pub fn is_writable(&self) -> bool {
        self.counter < u64::MAX - 1
    }
}

/// Statistics for eventfd V2 bridge.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventfdV2BridgeStats {
    pub total_eventfds: u64,
    pub total_writes: u64,
    pub total_reads: u64,
    pub semaphore_count: u64,
    pub overflow_events: u64,
    pub poll_wakeups: u64,
    pub cross_process_signals: u64,
}

/// Main bridge eventfd V2 manager.
#[repr(align(64))]
pub struct BridgeEventfdV2 {
    pub instances: BTreeMap<u64, EventfdV2Instance>,
    pub fd_map: BTreeMap<i32, u64>,
    pub next_id: u64,
    pub stats: EventfdV2BridgeStats,
}

impl BridgeEventfdV2 {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            fd_map: BTreeMap::new(),
            next_id: 1,
            stats: EventfdV2BridgeStats {
                total_eventfds: 0,
                total_writes: 0,
                total_reads: 0,
                semaphore_count: 0,
                overflow_events: 0,
                poll_wakeups: 0,
                cross_process_signals: 0,
            },
        }
    }

    pub fn create(&mut self, fd: i32, initial: u64, semaphore: bool) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut inst = EventfdV2Instance::new(id, fd, initial);
        inst.is_semaphore = semaphore;
        if semaphore {
            inst.flags.push(EventfdV2Flag::Semaphore);
            self.stats.semaphore_count += 1;
        }
        self.fd_map.insert(fd, id);
        self.instances.insert(id, inst);
        self.stats.total_eventfds += 1;
        id
    }

    #[inline]
    pub fn write(&mut self, efd_id: u64, value: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&efd_id) {
            let ok = inst.write(value);
            if ok {
                self.stats.total_writes += 1;
            }
            ok
        } else {
            false
        }
    }

    #[inline]
    pub fn read(&mut self, efd_id: u64) -> Option<u64> {
        if let Some(inst) = self.instances.get_mut(&efd_id) {
            let val = inst.read();
            self.stats.total_reads += 1;
            Some(val)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}

// ============================================================================
// Merged from eventfd_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdV3Op {
    Create,
    Read,
    Write,
    Poll,
    Close,
}

/// Eventfd v3 flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventfdV3Flag {
    None,
    Nonblock,
    Cloexec,
    Semaphore,
}

/// Eventfd v3 record
#[derive(Debug, Clone)]
pub struct EventfdV3Record {
    pub op: EventfdV3Op,
    pub flag: EventfdV3Flag,
    pub fd: i32,
    pub counter: u64,
    pub pid: u32,
}

impl EventfdV3Record {
    pub fn new(op: EventfdV3Op) -> Self {
        Self { op, flag: EventfdV3Flag::None, fd: -1, counter: 0, pid: 0 }
    }
}

/// Eventfd v3 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EventfdV3BridgeStats {
    pub total_ops: u64,
    pub fds_created: u64,
    pub reads: u64,
    pub writes: u64,
    pub semaphore_mode: u64,
}

/// Main bridge eventfd v3
#[derive(Debug)]
pub struct BridgeEventfdV3 {
    pub stats: EventfdV3BridgeStats,
}

impl BridgeEventfdV3 {
    pub fn new() -> Self {
        Self { stats: EventfdV3BridgeStats { total_ops: 0, fds_created: 0, reads: 0, writes: 0, semaphore_mode: 0 } }
    }

    pub fn record(&mut self, rec: &EventfdV3Record) {
        self.stats.total_ops += 1;
        match rec.op {
            EventfdV3Op::Create => {
                self.stats.fds_created += 1;
                if rec.flag == EventfdV3Flag::Semaphore { self.stats.semaphore_mode += 1; }
            }
            EventfdV3Op::Read => self.stats.reads += 1,
            EventfdV3Op::Write => self.stats.writes += 1,
            _ => {}
        }
    }
}
