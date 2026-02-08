// SPDX-License-Identifier: GPL-2.0
//! Bridge signalfd_bridge — signalfd interface bridge for signal notification via fd.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Signal set (bitmask for 64 signals)
#[derive(Debug, Clone, Copy)]
pub struct SignalSet(pub u64);

impl SignalSet {
    pub fn empty() -> Self { Self(0) }

    pub fn contains(&self, sig: u32) -> bool {
        if sig == 0 || sig > 64 { return false; }
        self.0 & (1u64 << (sig - 1)) != 0
    }

    pub fn add(&mut self, sig: u32) {
        if sig > 0 && sig <= 64 {
            self.0 |= 1u64 << (sig - 1);
        }
    }

    pub fn remove(&mut self, sig: u32) {
        if sig > 0 && sig <= 64 {
            self.0 &= !(1u64 << (sig - 1));
        }
    }

    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

/// Signalfd flags
#[derive(Debug, Clone, Copy)]
pub struct SignalfdFlags(pub u32);

impl SignalfdFlags {
    pub const CLOEXEC: Self = Self(0x01);
    pub const NONBLOCK: Self = Self(0x02);

    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}

/// Pending signal info
#[derive(Debug, Clone)]
pub struct PendingSignal {
    pub signal: u32,
    pub sender_pid: u32,
    pub code: i32,
    pub value: u64,
    pub timestamp: u64,
}

/// A signalfd instance
#[derive(Debug)]
pub struct SignalfdInstance {
    pub fd: i32,
    pub owner_pid: u32,
    pub mask: SignalSet,
    pub flags: SignalfdFlags,
    pub pending: Vec<PendingSignal>,
    pub max_pending: usize,
    pub read_count: u64,
    pub delivered_count: u64,
    pub dropped_count: u64,
    pub created: u64,
    pub last_read: u64,
}

impl SignalfdInstance {
    pub fn new(fd: i32, owner: u32, mask: SignalSet, flags: SignalfdFlags, now: u64) -> Self {
        Self {
            fd, owner_pid: owner, mask, flags,
            pending: Vec::new(), max_pending: 256,
            read_count: 0, delivered_count: 0, dropped_count: 0,
            created: now, last_read: 0,
        }
    }

    pub fn deliver(&mut self, signal: PendingSignal) -> bool {
        if !self.mask.contains(signal.signal) { return false; }
        if self.pending.len() >= self.max_pending {
            self.dropped_count += 1;
            return false;
        }
        self.delivered_count += 1;
        self.pending.push(signal);
        true
    }

    pub fn read_signal(&mut self, now: u64) -> Option<PendingSignal> {
        if self.pending.is_empty() { return None; }
        self.read_count += 1;
        self.last_read = now;
        Some(self.pending.remove(0))
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn is_readable(&self) -> bool {
        !self.pending.is_empty()
    }

    pub fn drop_rate(&self) -> f64 {
        let total = self.delivered_count + self.dropped_count;
        if total == 0 { return 0.0; }
        self.dropped_count as f64 / total as f64
    }

    pub fn idle_time(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_read.max(self.created))
    }
}

/// Signalfd operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalfdOp {
    Create,
    Read,
    SetMask,
    Close,
}

/// Signalfd event
#[derive(Debug, Clone)]
pub struct SignalfdEvent {
    pub fd: i32,
    pub op: SignalfdOp,
    pub pid: u32,
    pub timestamp: u64,
}

/// Signalfd bridge stats
#[derive(Debug, Clone)]
pub struct SignalfdBridgeStats {
    pub active_signalfds: u32,
    pub total_created: u64,
    pub total_signals_delivered: u64,
    pub total_signals_read: u64,
    pub total_dropped: u64,
}

/// Main signalfd bridge
pub struct BridgeSignalfd {
    instances: BTreeMap<i32, SignalfdInstance>,
    events: Vec<SignalfdEvent>,
    max_events: usize,
    stats: SignalfdBridgeStats,
}

impl BridgeSignalfd {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            events: Vec::new(),
            max_events: 2048,
            stats: SignalfdBridgeStats {
                active_signalfds: 0, total_created: 0,
                total_signals_delivered: 0, total_signals_read: 0,
                total_dropped: 0,
            },
        }
    }

    pub fn create(&mut self, fd: i32, owner: u32, mask: SignalSet, flags: SignalfdFlags, now: u64) {
        let inst = SignalfdInstance::new(fd, owner, mask, flags, now);
        self.stats.total_created += 1;
        self.stats.active_signalfds += 1;
        self.instances.insert(fd, inst);
    }

    pub fn deliver_signal(&mut self, target_pid: u32, signal: PendingSignal) -> u32 {
        let mut delivered = 0u32;
        let sig = signal.signal;
        for inst in self.instances.values_mut() {
            if inst.owner_pid == target_pid && inst.mask.contains(sig) {
                let s = PendingSignal {
                    signal: signal.signal, sender_pid: signal.sender_pid,
                    code: signal.code, value: signal.value,
                    timestamp: signal.timestamp,
                };
                if inst.deliver(s) {
                    delivered += 1;
                    self.stats.total_signals_delivered += 1;
                } else {
                    self.stats.total_dropped += 1;
                }
            }
        }
        delivered
    }

    pub fn read_signal(&mut self, fd: i32, now: u64) -> Option<PendingSignal> {
        let sig = self.instances.get_mut(&fd)?.read_signal(now);
        if sig.is_some() { self.stats.total_signals_read += 1; }
        sig
    }

    pub fn set_mask(&mut self, fd: i32, mask: SignalSet) {
        if let Some(inst) = self.instances.get_mut(&fd) {
            inst.mask = mask;
        }
    }

    pub fn close(&mut self, fd: i32) -> bool {
        if self.instances.remove(&fd).is_some() {
            if self.stats.active_signalfds > 0 { self.stats.active_signalfds -= 1; }
            true
        } else { false }
    }

    pub fn record_event(&mut self, event: SignalfdEvent) {
        if self.events.len() >= self.max_events { self.events.remove(0); }
        self.events.push(event);
    }

    pub fn instances_with_pending(&self) -> Vec<(i32, usize)> {
        self.instances.iter()
            .filter(|(_, inst)| inst.is_readable())
            .map(|(&fd, inst)| (fd, inst.pending_count()))
            .collect()
    }

    pub fn highest_drop_rates(&self, n: usize) -> Vec<(i32, f64)> {
        let mut v: Vec<_> = self.instances.iter()
            .filter(|(_, inst)| inst.dropped_count > 0)
            .map(|(&fd, inst)| (fd, inst.drop_rate()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(n);
        v
    }

    pub fn stats(&self) -> &SignalfdBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from signalfd_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignalfdV2Signal {
    SigHup,
    SigInt,
    SigQuit,
    SigIll,
    SigTrap,
    SigAbrt,
    SigBus,
    SigFpe,
    SigKill,
    SigUsr1,
    SigSegv,
    SigUsr2,
    SigPipe,
    SigAlrm,
    SigTerm,
    SigChld,
    SigCont,
    SigStop,
    SigTstp,
    SigRtMin(u32),
}

/// Signal info structure.
#[derive(Debug, Clone)]
pub struct SignalfdV2Info {
    pub signal: SignalfdV2Signal,
    pub sender_pid: u64,
    pub sender_uid: u32,
    pub timestamp: u64,
    pub code: i32,
    pub value: i64,
    pub coalesced_count: u32,
}

impl SignalfdV2Info {
    pub fn new(signal: SignalfdV2Signal, sender_pid: u64) -> Self {
        Self {
            signal,
            sender_pid,
            sender_uid: 0,
            timestamp: 0,
            code: 0,
            value: 0,
            coalesced_count: 1,
        }
    }

    pub fn coalesce(&mut self) {
        self.coalesced_count += 1;
    }
}

/// A signalfd instance.
#[derive(Debug, Clone)]
pub struct SignalfdV2Instance {
    pub sfd_id: u64,
    pub fd: i32,
    pub mask: Vec<SignalfdV2Signal>,
    pub pending: Vec<SignalfdV2Info>,
    pub max_queue_size: usize,
    pub coalesce_enabled: bool,
    pub read_count: u64,
    pub signal_count: u64,
    pub coalesced_count: u64,
    pub overflow_count: u64,
    pub owner_pid: u64,
}

impl SignalfdV2Instance {
    pub fn new(sfd_id: u64, fd: i32) -> Self {
        Self {
            sfd_id,
            fd,
            mask: Vec::new(),
            pending: Vec::new(),
            max_queue_size: 256,
            coalesce_enabled: true,
            read_count: 0,
            signal_count: 0,
            coalesced_count: 0,
            overflow_count: 0,
            owner_pid: 0,
        }
    }

    pub fn set_mask(&mut self, signals: Vec<SignalfdV2Signal>) {
        self.mask = signals;
    }

    pub fn deliver_signal(&mut self, info: SignalfdV2Info) -> bool {
        if !self.mask.contains(&info.signal) {
            return false;
        }
        if self.coalesce_enabled {
            for existing in &mut self.pending {
                if existing.signal == info.signal {
                    existing.coalesce();
                    self.coalesced_count += 1;
                    self.signal_count += 1;
                    return true;
                }
            }
        }
        if self.pending.len() >= self.max_queue_size {
            self.overflow_count += 1;
            return false;
        }
        self.pending.push(info);
        self.signal_count += 1;
        true
    }

    pub fn read_signal(&mut self) -> Option<SignalfdV2Info> {
        if self.pending.is_empty() {
            return None;
        }
        self.read_count += 1;
        Some(self.pending.remove(0))
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn is_readable(&self) -> bool {
        !self.pending.is_empty()
    }
}

/// Statistics for signalfd V2 bridge.
#[derive(Debug, Clone)]
pub struct SignalfdV2BridgeStats {
    pub total_instances: u64,
    pub total_signals_delivered: u64,
    pub total_signals_read: u64,
    pub total_coalesced: u64,
    pub total_overflows: u64,
    pub rt_signals_queued: u64,
}

/// Main bridge signalfd V2 manager.
pub struct BridgeSignalfdV2 {
    pub instances: BTreeMap<u64, SignalfdV2Instance>,
    pub fd_map: BTreeMap<i32, u64>,
    pub next_id: u64,
    pub stats: SignalfdV2BridgeStats,
}

impl BridgeSignalfdV2 {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            fd_map: BTreeMap::new(),
            next_id: 1,
            stats: SignalfdV2BridgeStats {
                total_instances: 0,
                total_signals_delivered: 0,
                total_signals_read: 0,
                total_coalesced: 0,
                total_overflows: 0,
                rt_signals_queued: 0,
            },
        }
    }

    pub fn create(&mut self, fd: i32, mask: Vec<SignalfdV2Signal>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut inst = SignalfdV2Instance::new(id, fd);
        inst.set_mask(mask);
        self.fd_map.insert(fd, id);
        self.instances.insert(id, inst);
        self.stats.total_instances += 1;
        id
    }

    pub fn deliver(&mut self, sfd_id: u64, info: SignalfdV2Info) -> bool {
        let is_rt = matches!(info.signal, SignalfdV2Signal::SigRtMin(_));
        if let Some(inst) = self.instances.get_mut(&sfd_id) {
            let ok = inst.deliver_signal(info);
            if ok {
                self.stats.total_signals_delivered += 1;
                if is_rt {
                    self.stats.rt_signals_queued += 1;
                }
            }
            ok
        } else {
            false
        }
    }

    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }
}

// ============================================================================
// Merged from signalfd_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalfdV3Op {
    Create,
    Read,
    UpdateMask,
    Close,
}

/// Signalfd v3 record
#[derive(Debug, Clone)]
pub struct SignalfdV3Record {
    pub op: SignalfdV3Op,
    pub fd: i32,
    pub mask_bits: u64,
    pub signals_read: u32,
    pub pid: u32,
}

impl SignalfdV3Record {
    pub fn new(op: SignalfdV3Op) -> Self {
        Self { op, fd: -1, mask_bits: 0, signals_read: 0, pid: 0 }
    }
}

/// Signalfd v3 bridge stats
#[derive(Debug, Clone)]
pub struct SignalfdV3BridgeStats {
    pub total_ops: u64,
    pub fds_created: u64,
    pub signals_read: u64,
    pub mask_updates: u64,
}

/// Main bridge signalfd v3
#[derive(Debug)]
pub struct BridgeSignalfdV3 {
    pub stats: SignalfdV3BridgeStats,
}

impl BridgeSignalfdV3 {
    pub fn new() -> Self {
        Self { stats: SignalfdV3BridgeStats { total_ops: 0, fds_created: 0, signals_read: 0, mask_updates: 0 } }
    }

    pub fn record(&mut self, rec: &SignalfdV3Record) {
        self.stats.total_ops += 1;
        match rec.op {
            SignalfdV3Op::Create => self.stats.fds_created += 1,
            SignalfdV3Op::Read => self.stats.signals_read += rec.signals_read as u64,
            SignalfdV3Op::UpdateMask => self.stats.mask_updates += 1,
            _ => {}
        }
    }
}
