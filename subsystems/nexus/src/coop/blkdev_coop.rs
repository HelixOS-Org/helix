// SPDX-License-Identifier: GPL-2.0
//! Coop blkdev — cooperative block device sharing with bandwidth allocation

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop blkdev state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopBlkdevState {
    Exclusive,
    Shared,
    ReadOnly,
    Suspended,
    Error,
}

/// Bandwidth allocation
#[derive(Debug, Clone)]
pub struct CoopBwAlloc {
    pub owner_id: u64,
    pub read_bps: u64,
    pub write_bps: u64,
    pub iops_limit: u64,
    pub used_read_bps: u64,
    pub used_write_bps: u64,
    pub used_iops: u64,
}

impl CoopBwAlloc {
    pub fn new(owner_id: u64, read_bps: u64, write_bps: u64) -> Self {
        Self {
            owner_id,
            read_bps,
            write_bps,
            iops_limit: 0,
            used_read_bps: 0,
            used_write_bps: 0,
            used_iops: 0,
        }
    }

    pub fn can_read(&self, bytes: u64) -> bool {
        self.read_bps == 0 || self.used_read_bps + bytes <= self.read_bps
    }
    pub fn can_write(&self, bytes: u64) -> bool {
        self.write_bps == 0 || self.used_write_bps + bytes <= self.write_bps
    }
    pub fn consume_read(&mut self, bytes: u64) {
        self.used_read_bps += bytes;
    }
    pub fn consume_write(&mut self, bytes: u64) {
        self.used_write_bps += bytes;
    }
    pub fn reset(&mut self) {
        self.used_read_bps = 0;
        self.used_write_bps = 0;
        self.used_iops = 0;
    }
}

/// Shared block device
#[derive(Debug, Clone)]
pub struct CoopBlkdevInstance {
    pub dev_id: u64,
    pub state: CoopBlkdevState,
    pub capacity_sectors: u64,
    pub allocs: Vec<CoopBwAlloc>,
    pub total_io: u64,
}

impl CoopBlkdevInstance {
    pub fn new(dev_id: u64, capacity_sectors: u64) -> Self {
        Self {
            dev_id,
            state: CoopBlkdevState::Exclusive,
            capacity_sectors,
            allocs: Vec::new(),
            total_io: 0,
        }
    }

    pub fn share_with(&mut self, owner_id: u64, read_bps: u64, write_bps: u64) {
        self.allocs
            .push(CoopBwAlloc::new(owner_id, read_bps, write_bps));
        self.state = CoopBlkdevState::Shared;
    }
}

/// Coop blkdev stats
#[derive(Debug, Clone)]
pub struct CoopBlkdevStats {
    pub total_devices: u64,
    pub shared_devices: u64,
    pub throttled_ios: u64,
    pub total_bytes: u64,
}

/// Main coop blkdev
#[derive(Debug)]
pub struct CoopBlkdev {
    pub devices: BTreeMap<u64, CoopBlkdevInstance>,
    pub stats: CoopBlkdevStats,
}

impl CoopBlkdev {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            stats: CoopBlkdevStats {
                total_devices: 0,
                shared_devices: 0,
                throttled_ios: 0,
                total_bytes: 0,
            },
        }
    }

    pub fn register(&mut self, dev_id: u64, capacity: u64) {
        self.stats.total_devices += 1;
        self.devices
            .insert(dev_id, CoopBlkdevInstance::new(dev_id, capacity));
    }

    pub fn share_device(&mut self, dev_id: u64, owner: u64, rbps: u64, wbps: u64) {
        if let Some(dev) = self.devices.get_mut(&dev_id) {
            dev.share_with(owner, rbps, wbps);
            self.stats.shared_devices += 1;
        }
    }
}
