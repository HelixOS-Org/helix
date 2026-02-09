// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic block device — Block device registration and geometry
//!
//! Models block device registration with partition table parsing,
//! I/O scheduling queue association, and device geometry tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Block device type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDevType {
    Disk,
    Partition,
    Loop,
    Raid,
    Dm,
    NVMe,
    Virtio,
    Scsi,
}

/// I/O scheduler type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDevScheduler {
    None,
    Mq,
    Bfq,
    Kyber,
}

/// Block device state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockDevState {
    Initializing,
    Running,
    Suspended,
    Removed,
    Error,
}

/// Block device geometry.
#[derive(Debug, Clone)]
pub struct BlockDevGeometry {
    pub sector_size: u32,
    pub total_sectors: u64,
    pub max_segments: u32,
    pub max_segment_size: u32,
    pub max_hw_sectors: u32,
    pub optimal_io_size: u32,
    pub alignment_offset: u32,
    pub rotational: bool,
}

impl BlockDevGeometry {
    pub fn new(sector_size: u32, total_sectors: u64) -> Self {
        Self {
            sector_size,
            total_sectors,
            max_segments: 128,
            max_segment_size: 65536,
            max_hw_sectors: 2048,
            optimal_io_size: 0,
            alignment_offset: 0,
            rotational: false,
        }
    }

    #[inline(always)]
    pub fn capacity_bytes(&self) -> u64 {
        self.total_sectors * self.sector_size as u64
    }
}

/// A partition entry.
#[derive(Debug, Clone)]
pub struct BlockDevPartition {
    pub part_num: u32,
    pub start_sector: u64,
    pub nr_sectors: u64,
    pub part_type: u8,
    pub name: Option<String>,
}

impl BlockDevPartition {
    pub fn new(part_num: u32, start: u64, sectors: u64) -> Self {
        Self {
            part_num,
            start_sector: start,
            nr_sectors: sectors,
            part_type: 0x83, // Linux
            name: None,
        }
    }
}

/// A registered block device.
#[derive(Debug, Clone)]
pub struct BlockDevEntry {
    pub dev_id: u32,
    pub major: u32,
    pub minor: u32,
    pub dev_type: BlockDevType,
    pub state: BlockDevState,
    pub scheduler: BlockDevScheduler,
    pub geometry: BlockDevGeometry,
    pub partitions: Vec<BlockDevPartition>,
    pub name: String,
    pub queue_depth: u32,
    pub inflight_ios: u32,
}

impl BlockDevEntry {
    pub fn new(dev_id: u32, name: String, dev_type: BlockDevType, geometry: BlockDevGeometry) -> Self {
        Self {
            dev_id,
            major: 0,
            minor: 0,
            dev_type,
            state: BlockDevState::Initializing,
            scheduler: BlockDevScheduler::Mq,
            geometry,
            partitions: Vec::new(),
            name,
            queue_depth: 128,
            inflight_ios: 0,
        }
    }
}

/// Statistics for block devices.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BlockDevStats {
    pub total_devices: u64,
    pub total_partitions: u64,
    pub total_capacity_bytes: u64,
    pub rotational_count: u64,
    pub nvme_count: u64,
}

/// Main holistic block device manager.
pub struct HolisticBlockDev {
    pub devices: BTreeMap<u32, BlockDevEntry>,
    pub next_dev_id: u32,
    pub stats: BlockDevStats,
}

impl HolisticBlockDev {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            next_dev_id: 1,
            stats: BlockDevStats {
                total_devices: 0,
                total_partitions: 0,
                total_capacity_bytes: 0,
                rotational_count: 0,
                nvme_count: 0,
            },
        }
    }

    pub fn register_device(
        &mut self,
        name: String,
        dev_type: BlockDevType,
        sector_size: u32,
        total_sectors: u64,
    ) -> u32 {
        let id = self.next_dev_id;
        self.next_dev_id += 1;
        let geom = BlockDevGeometry::new(sector_size, total_sectors);
        let cap = geom.capacity_bytes();
        if geom.rotational {
            self.stats.rotational_count += 1;
        }
        if dev_type == BlockDevType::NVMe {
            self.stats.nvme_count += 1;
        }
        let entry = BlockDevEntry::new(id, name, dev_type, geom);
        self.devices.insert(id, entry);
        self.stats.total_devices += 1;
        self.stats.total_capacity_bytes += cap;
        id
    }

    #[inline(always)]
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }
}
