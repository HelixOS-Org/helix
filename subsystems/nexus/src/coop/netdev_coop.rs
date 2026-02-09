// SPDX-License-Identifier: GPL-2.0
//! Coop netdev — cooperative network device management with shared queues

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop netdev state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopNetdevState {
    Down,
    Up,
    Shared,
    Migrating,
    Suspended,
}

/// Coop netdev type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopNetdevType {
    Physical,
    Virtual,
    Bridge,
    Bond,
    Vlan,
    Macvlan,
    Veth,
    Tun,
    Tap,
}

/// Shared network queue
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SharedNetQueue {
    pub queue_id: u32,
    pub capacity: u32,
    pub enqueued: u32,
    pub processed: u64,
    pub dropped: u64,
    pub bytes_total: u64,
    pub consumers: Vec<u64>,
}

impl SharedNetQueue {
    pub fn new(queue_id: u32, capacity: u32) -> Self {
        Self { queue_id, capacity, enqueued: 0, processed: 0, dropped: 0, bytes_total: 0, consumers: Vec::new() }
    }

    #[inline]
    pub fn enqueue(&mut self, bytes: u64) -> bool {
        if self.enqueued >= self.capacity {
            self.dropped += 1;
            false
        } else {
            self.enqueued += 1;
            self.bytes_total += bytes;
            true
        }
    }

    #[inline(always)]
    pub fn dequeue(&mut self) -> bool {
        if self.enqueued == 0 { false }
        else { self.enqueued -= 1; self.processed += 1; true }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { 0.0 } else { self.enqueued as f64 / self.capacity as f64 }
    }

    #[inline(always)]
    pub fn drop_rate(&self) -> f64 {
        let total = self.processed + self.dropped;
        if total == 0 { 0.0 } else { self.dropped as f64 / total as f64 }
    }
}

/// Coop netdev instance
#[derive(Debug, Clone)]
pub struct CoopNetdevInstance {
    pub dev_id: u64,
    pub name_hash: u64,
    pub dev_type: CoopNetdevType,
    pub state: CoopNetdevState,
    pub tx_queues: Vec<SharedNetQueue>,
    pub rx_queues: Vec<SharedNetQueue>,
    pub mtu: u32,
    pub shared_ns: Vec<u64>,
}

impl CoopNetdevInstance {
    pub fn new(dev_id: u64, name: &[u8], dev_type: CoopNetdevType) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            dev_id, name_hash: h, dev_type, state: CoopNetdevState::Down,
            tx_queues: Vec::new(), rx_queues: Vec::new(), mtu: 1500, shared_ns: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn bring_up(&mut self) { self.state = CoopNetdevState::Up; }
    #[inline(always)]
    pub fn share_with(&mut self, ns_id: u64) {
        if !self.shared_ns.contains(&ns_id) { self.shared_ns.push(ns_id); }
        self.state = CoopNetdevState::Shared;
    }

    #[inline(always)]
    pub fn total_tx_bytes(&self) -> u64 { self.tx_queues.iter().map(|q| q.bytes_total).sum() }
    #[inline(always)]
    pub fn total_rx_bytes(&self) -> u64 { self.rx_queues.iter().map(|q| q.bytes_total).sum() }
}

/// Coop netdev stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopNetdevStats {
    pub total_devices: u64,
    pub shared_devices: u64,
    pub total_queues: u64,
    pub total_bytes: u64,
}

/// Main coop netdev manager
#[derive(Debug)]
pub struct CoopNetdev {
    pub devices: BTreeMap<u64, CoopNetdevInstance>,
    pub stats: CoopNetdevStats,
}

impl CoopNetdev {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            stats: CoopNetdevStats { total_devices: 0, shared_devices: 0, total_queues: 0, total_bytes: 0 },
        }
    }

    #[inline(always)]
    pub fn register(&mut self, dev: CoopNetdevInstance) {
        self.stats.total_devices += 1;
        self.devices.insert(dev.dev_id, dev);
    }

    #[inline]
    pub fn share_device(&mut self, dev_id: u64, ns_id: u64) -> bool {
        if let Some(dev) = self.devices.get_mut(&dev_id) {
            dev.share_with(ns_id);
            self.stats.shared_devices += 1;
            true
        } else { false }
    }
}
