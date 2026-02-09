// SPDX-License-Identifier: GPL-2.0
//! Bridge kqueue — BSD-style event notification interface

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Kqueue filter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KqueueFilter {
    EvfiltRead,
    EvfiltWrite,
    EvfiltVnode,
    EvfiltProc,
    EvfiltSignal,
    EvfiltTimer,
    EvfiltUser,
    EvfiltFs,
    EvfiltSock,
    EvfiltMachPort,
}

/// Kqueue event flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KqueueFlag {
    EvAdd,
    EvDelete,
    EvEnable,
    EvDisable,
    EvOneshot,
    EvClear,
    EvReceipt,
    EvDispatch,
    EvEof,
    EvError,
}

/// Kqueue vnode event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VnodeEvent {
    Delete,
    Write,
    Extend,
    Attrib,
    Link,
    Rename,
    Revoke,
    Funlock,
}

/// Kevent entry
#[derive(Debug, Clone)]
pub struct Kevent {
    pub ident: u64,
    pub filter: KqueueFilter,
    pub flags: u32,
    pub fflags: u32,
    pub data: i64,
    pub udata: u64,
    pub active: bool,
    pub triggered: bool,
    pub trigger_count: u64,
}

impl Kevent {
    pub fn new(ident: u64, filter: KqueueFilter, flags: u32) -> Self {
        Self {
            ident,
            filter,
            flags,
            fflags: 0,
            data: 0,
            udata: 0,
            active: true,
            triggered: false,
            trigger_count: 0,
        }
    }

    #[inline]
    pub fn trigger(&mut self, data: i64) {
        self.triggered = true;
        self.data = data;
        self.trigger_count += 1;
    }

    #[inline]
    pub fn consume(&mut self) {
        self.triggered = false;
        if self.flags & 0x10 != 0 {
            self.active = false;
        }
    }
}

/// Kqueue instance
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KqueueInstance {
    pub kq_fd: u32,
    pub events: BTreeMap<u64, Kevent>,
    pub total_kevents: u64,
    pub total_returns: u64,
    pub empty_returns: u64,
    pub max_returned: u32,
}

impl KqueueInstance {
    pub fn new(kq_fd: u32) -> Self {
        Self {
            kq_fd,
            events: BTreeMap::new(),
            total_kevents: 0,
            total_returns: 0,
            empty_returns: 0,
            max_returned: 0,
        }
    }

    #[inline(always)]
    pub fn register(&mut self, kevent: Kevent) {
        self.events.insert(kevent.ident, kevent);
    }

    #[inline(always)]
    pub fn unregister(&mut self, ident: u64) -> bool {
        self.events.remove(&ident).is_some()
    }

    pub fn kevent_poll(&mut self, max_events: u32) -> u32 {
        self.total_kevents += 1;
        let ready: Vec<u64> = self.events.iter()
            .filter(|(_, e)| e.active && e.triggered)
            .map(|(&id, _)| id)
            .take(max_events as usize)
            .collect();
        let count = ready.len() as u32;
        for id in &ready {
            if let Some(ev) = self.events.get_mut(id) {
                ev.consume();
            }
        }
        if count == 0 {
            self.empty_returns += 1;
        }
        self.total_returns += count as u64;
        if count > self.max_returned {
            self.max_returned = count;
        }
        count
    }

    #[inline(always)]
    pub fn avg_events(&self) -> f64 {
        if self.total_kevents == 0 { 0.0 } else { self.total_returns as f64 / self.total_kevents as f64 }
    }
}

/// Kqueue bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KqueueBridgeStats {
    pub total_instances: u64,
    pub total_events_registered: u64,
    pub total_polls: u64,
}

/// Main bridge kqueue
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeKqueue {
    pub instances: BTreeMap<u32, KqueueInstance>,
    pub stats: KqueueBridgeStats,
    pub next_kq_fd: u32,
}

impl BridgeKqueue {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            stats: KqueueBridgeStats {
                total_instances: 0,
                total_events_registered: 0,
                total_polls: 0,
            },
            next_kq_fd: 1,
        }
    }

    #[inline]
    pub fn create(&mut self) -> u32 {
        let id = self.next_kq_fd;
        self.next_kq_fd += 1;
        self.instances.insert(id, KqueueInstance::new(id));
        self.stats.total_instances += 1;
        id
    }

    #[inline]
    pub fn register_event(&mut self, kq_fd: u32, kevent: Kevent) -> bool {
        if let Some(inst) = self.instances.get_mut(&kq_fd) {
            inst.register(kevent);
            self.stats.total_events_registered += 1;
            true
        } else {
            false
        }
    }
}
