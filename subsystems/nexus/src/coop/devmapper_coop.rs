// SPDX-License-Identifier: GPL-2.0
//! Coop device mapper — cooperative DM target management with shared thin pools

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop DM target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopDmTarget {
    Linear,
    Striped,
    Mirror,
    Snapshot,
    ThinPool,
    ThinVolume,
    Cache,
    Crypt,
}

/// Coop DM state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopDmState {
    Active,
    Suspended,
    Shared,
    Error,
}

/// Shared thin pool
#[derive(Debug, Clone)]
pub struct CoopThinPool {
    pub pool_id: u64,
    pub total_data_blocks: u64,
    pub used_data_blocks: u64,
    pub total_metadata_blocks: u64,
    pub used_metadata_blocks: u64,
    pub subscribers: u32,
}

impl CoopThinPool {
    pub fn new(pool_id: u64, data_blocks: u64, meta_blocks: u64) -> Self {
        Self {
            pool_id,
            total_data_blocks: data_blocks,
            used_data_blocks: 0,
            total_metadata_blocks: meta_blocks,
            used_metadata_blocks: 0,
            subscribers: 1,
        }
    }

    pub fn allocate(&mut self, blocks: u64) -> bool {
        if self.used_data_blocks + blocks > self.total_data_blocks {
            return false;
        }
        self.used_data_blocks += blocks;
        true
    }

    pub fn free(&mut self, blocks: u64) {
        self.used_data_blocks = self.used_data_blocks.saturating_sub(blocks);
    }
    pub fn subscribe(&mut self) {
        self.subscribers += 1;
    }
    pub fn data_usage_pct(&self) -> f64 {
        if self.total_data_blocks == 0 {
            0.0
        } else {
            self.used_data_blocks as f64 / self.total_data_blocks as f64
        }
    }
}

/// Coop DM device
#[derive(Debug, Clone)]
pub struct CoopDmDevice {
    pub name_hash: u64,
    pub target: CoopDmTarget,
    pub state: CoopDmState,
    pub size_sectors: u64,
    pub shared_count: u32,
    pub io_count: u64,
}

impl CoopDmDevice {
    pub fn new(name: &[u8], target: CoopDmTarget, size: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            name_hash: h,
            target,
            state: CoopDmState::Active,
            size_sectors: size,
            shared_count: 1,
            io_count: 0,
        }
    }

    pub fn share(&mut self) {
        self.shared_count += 1;
        self.state = CoopDmState::Shared;
    }
    pub fn record_io(&mut self) {
        self.io_count += 1;
    }
}

/// Coop DM stats
#[derive(Debug, Clone)]
pub struct CoopDmStats {
    pub total_devices: u64,
    pub thin_pools: u64,
    pub shared_devices: u64,
    pub total_io: u64,
}

/// Main coop device mapper
#[derive(Debug)]
pub struct CoopDevMapper {
    pub devices: BTreeMap<u64, CoopDmDevice>,
    pub pools: BTreeMap<u64, CoopThinPool>,
    pub stats: CoopDmStats,
}

impl CoopDevMapper {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            pools: BTreeMap::new(),
            stats: CoopDmStats {
                total_devices: 0,
                thin_pools: 0,
                shared_devices: 0,
                total_io: 0,
            },
        }
    }

    pub fn create_device(&mut self, id: u64, name: &[u8], target: CoopDmTarget, size: u64) {
        self.stats.total_devices += 1;
        self.devices
            .insert(id, CoopDmDevice::new(name, target, size));
    }

    pub fn create_pool(&mut self, id: u64, data_blocks: u64, meta_blocks: u64) {
        self.stats.thin_pools += 1;
        self.pools
            .insert(id, CoopThinPool::new(id, data_blocks, meta_blocks));
    }
}
