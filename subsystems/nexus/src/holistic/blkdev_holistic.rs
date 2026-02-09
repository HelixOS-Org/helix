// SPDX-License-Identifier: GPL-2.0
//! Holistic blkdev — block device management with queue depth and partitions

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Block device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlkdevType {
    Hdd,
    Ssd,
    Nvme,
    Ram,
    Loop,
    Virtio,
    Scsi,
    MmcBlock,
    DmDevice,
}

/// Block device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlkdevState {
    Offline,
    Online,
    ReadOnly,
    Suspended,
    Error,
}

/// Partition entry
#[derive(Debug, Clone)]
pub struct BlkPartition {
    pub part_num: u32,
    pub start_sector: u64,
    pub nr_sectors: u64,
    pub fs_type_hash: u64,
    pub flags: u32,
}

impl BlkPartition {
    pub fn new(part_num: u32, start: u64, nr_sectors: u64) -> Self {
        Self { part_num, start_sector: start, nr_sectors, fs_type_hash: 0, flags: 0 }
    }

    #[inline(always)]
    pub fn size_bytes(&self) -> u64 { self.nr_sectors * 512 }
    #[inline(always)]
    pub fn end_sector(&self) -> u64 { self.start_sector + self.nr_sectors }
}

/// Block device instance
#[derive(Debug, Clone)]
pub struct BlkdevInstance {
    pub dev_id: u64,
    pub name_hash: u64,
    pub dev_type: BlkdevType,
    pub state: BlkdevState,
    pub sector_size: u32,
    pub total_sectors: u64,
    pub queue_depth: u32,
    pub max_hw_sectors: u32,
    pub partitions: Vec<BlkPartition>,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub io_ticks_ms: u64,
}

impl BlkdevInstance {
    pub fn new(dev_id: u64, name: &[u8], dev_type: BlkdevType, total_sectors: u64) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            dev_id, name_hash: h, dev_type, state: BlkdevState::Online,
            sector_size: 512, total_sectors, queue_depth: 32, max_hw_sectors: 2048,
            partitions: Vec::new(), read_bytes: 0, write_bytes: 0, io_ticks_ms: 0,
        }
    }

    #[inline(always)]
    pub fn capacity_bytes(&self) -> u64 { self.total_sectors * self.sector_size as u64 }
    #[inline(always)]
    pub fn add_partition(&mut self, part: BlkPartition) { self.partitions.push(part); }
    #[inline(always)]
    pub fn is_rotational(&self) -> bool { self.dev_type == BlkdevType::Hdd }

    #[inline(always)]
    pub fn record_io(&mut self, read: bool, bytes: u64) {
        if read { self.read_bytes += bytes; } else { self.write_bytes += bytes; }
    }

    #[inline(always)]
    pub fn throughput_bytes(&self) -> u64 { self.read_bytes + self.write_bytes }
}

/// Blkdev holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticBlkdevStats {
    pub total_devices: u64,
    pub total_capacity_bytes: u64,
    pub total_read_bytes: u64,
    pub total_write_bytes: u64,
}

/// Main holistic blkdev manager
#[derive(Debug)]
pub struct HolisticBlkdev {
    pub devices: BTreeMap<u64, BlkdevInstance>,
    pub stats: HolisticBlkdevStats,
}

impl HolisticBlkdev {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            stats: HolisticBlkdevStats { total_devices: 0, total_capacity_bytes: 0, total_read_bytes: 0, total_write_bytes: 0 },
        }
    }

    #[inline]
    pub fn register(&mut self, dev: BlkdevInstance) {
        self.stats.total_devices += 1;
        self.stats.total_capacity_bytes += dev.capacity_bytes();
        self.devices.insert(dev.dev_id, dev);
    }

    #[inline]
    pub fn record_io(&mut self, dev_id: u64, read: bool, bytes: u64) {
        if let Some(dev) = self.devices.get_mut(&dev_id) {
            dev.record_io(read, bytes);
            if read { self.stats.total_read_bytes += bytes; }
            else { self.stats.total_write_bytes += bytes; }
        }
    }
}
