// SPDX-License-Identifier: GPL-2.0
//! Holistic DM — device mapper with linear, striped, and snapshot targets

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// DM target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmTargetType {
    Linear,
    Striped,
    Mirror,
    Snapshot,
    SnapshotOrigin,
    Thin,
    ThinPool,
    Cache,
    Crypt,
    Integrity,
    Era,
    Writecache,
    Zero,
    Error,
}

/// DM device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmDevState {
    Active,
    Suspended,
    Error,
    Creating,
    Removing,
}

/// DM target
#[derive(Debug, Clone)]
pub struct DmTarget {
    pub target_type: DmTargetType,
    pub start_sector: u64,
    pub length_sectors: u64,
    pub underlying_dev: u64,
    pub offset: u64,
}

impl DmTarget {
    pub fn new(target_type: DmTargetType, start: u64, length: u64) -> Self {
        Self { target_type, start_sector: start, length_sectors: length, underlying_dev: 0, offset: 0 }
    }

    #[inline(always)]
    pub fn end_sector(&self) -> u64 { self.start_sector + self.length_sectors }
    #[inline(always)]
    pub fn size_bytes(&self) -> u64 { self.length_sectors * 512 }
}

/// DM device
#[derive(Debug, Clone)]
pub struct DmDevice {
    pub dm_id: u64,
    pub name_hash: u64,
    pub state: DmDevState,
    pub targets: Vec<DmTarget>,
    pub open_count: u32,
    pub event_nr: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

impl DmDevice {
    pub fn new(dm_id: u64, name: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            dm_id, name_hash: h, state: DmDevState::Creating, targets: Vec::new(),
            open_count: 0, event_nr: 0, read_bytes: 0, write_bytes: 0,
        }
    }

    #[inline(always)]
    pub fn add_target(&mut self, target: DmTarget) {
        self.targets.push(target);
    }

    #[inline(always)]
    pub fn activate(&mut self) { self.state = DmDevState::Active; }
    #[inline(always)]
    pub fn suspend(&mut self) { self.state = DmDevState::Suspended; }
    #[inline(always)]
    pub fn resume(&mut self) { self.state = DmDevState::Active; }

    #[inline(always)]
    pub fn total_sectors(&self) -> u64 {
        self.targets.iter().map(|t| t.length_sectors).sum()
    }

    #[inline(always)]
    pub fn size_bytes(&self) -> u64 { self.total_sectors() * 512 }

    #[inline(always)]
    pub fn record_io(&mut self, read: bool, bytes: u64) {
        if read { self.read_bytes += bytes; } else { self.write_bytes += bytes; }
    }
}

/// Thin pool status
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DmThinPoolStatus {
    pub pool_id: u64,
    pub total_data_blocks: u64,
    pub used_data_blocks: u64,
    pub total_metadata_blocks: u64,
    pub used_metadata_blocks: u64,
    pub thin_devices: u32,
    pub held_metadata_root: bool,
}

impl DmThinPoolStatus {
    pub fn new(pool_id: u64, data_blocks: u64, meta_blocks: u64) -> Self {
        Self {
            pool_id, total_data_blocks: data_blocks, used_data_blocks: 0,
            total_metadata_blocks: meta_blocks, used_metadata_blocks: 0,
            thin_devices: 0, held_metadata_root: false,
        }
    }

    #[inline(always)]
    pub fn data_usage_pct(&self) -> f64 {
        if self.total_data_blocks == 0 { 0.0 } else { self.used_data_blocks as f64 / self.total_data_blocks as f64 }
    }

    #[inline(always)]
    pub fn metadata_usage_pct(&self) -> f64 {
        if self.total_metadata_blocks == 0 { 0.0 }
        else { self.used_metadata_blocks as f64 / self.total_metadata_blocks as f64 }
    }
}

/// DM holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticDmStats {
    pub total_devices: u64,
    pub total_targets: u64,
    pub thin_pools: u64,
    pub total_bytes: u64,
}

/// Main holistic DM manager
#[derive(Debug)]
pub struct HolisticDm {
    pub devices: BTreeMap<u64, DmDevice>,
    pub thin_pools: BTreeMap<u64, DmThinPoolStatus>,
    pub stats: HolisticDmStats,
}

impl HolisticDm {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            thin_pools: BTreeMap::new(),
            stats: HolisticDmStats { total_devices: 0, total_targets: 0, thin_pools: 0, total_bytes: 0 },
        }
    }

    #[inline]
    pub fn create_device(&mut self, dev: DmDevice) {
        self.stats.total_devices += 1;
        self.stats.total_targets += dev.targets.len() as u64;
        self.devices.insert(dev.dm_id, dev);
    }

    #[inline(always)]
    pub fn register_pool(&mut self, pool: DmThinPoolStatus) {
        self.thin_pools.insert(pool.pool_id, pool);
        self.stats.thin_pools += 1;
    }

    #[inline]
    pub fn activate_device(&mut self, dm_id: u64) -> bool {
        if let Some(dev) = self.devices.get_mut(&dm_id) {
            dev.activate();
            true
        } else { false }
    }
}
