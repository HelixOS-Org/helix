//! Block Device
//!
//! Block device representation and partition management.

use alloc::string::String;
use alloc::vec::Vec;

use super::{
    BlockDeviceId, BlockDeviceState, BlockDeviceType, DiskStats, IoRequest, IoScheduler,
    RequestQueue,
};

/// Block device
#[derive(Debug)]
pub struct BlockDevice {
    /// Device ID
    pub id: BlockDeviceId,
    /// Device name (e.g., sda, nvme0n1)
    pub name: String,
    /// Device type
    pub device_type: BlockDeviceType,
    /// State
    pub state: BlockDeviceState,
    /// Size in sectors
    pub size_sectors: u64,
    /// Sector size (bytes)
    pub sector_size: u32,
    /// Physical block size
    pub physical_block_size: u32,
    /// Is read-only
    pub read_only: bool,
    /// Is removable
    pub removable: bool,
    /// Is rotational
    pub rotational: bool,
    /// I/O scheduler
    pub scheduler: IoScheduler,
    /// Queue depth
    pub queue_depth: u32,
    /// Statistics
    pub stats: DiskStats,
    /// Model/product name
    pub model: Option<String>,
    /// Vendor
    pub vendor: Option<String>,
    /// Serial number
    pub serial: Option<String>,
    /// Firmware version
    pub firmware: Option<String>,
    /// Supports trim
    pub supports_trim: bool,
    /// Supports write cache
    pub write_cache: bool,
    /// Parent device (for partitions)
    pub parent: Option<BlockDeviceId>,
    /// Children (partitions)
    pub children: Vec<BlockDeviceId>,
    /// Request queue
    request_queue: Option<RequestQueue>,
}

impl BlockDevice {
    /// Create new device
    pub fn new(id: BlockDeviceId, name: String, device_type: BlockDeviceType) -> Self {
        Self {
            id,
            name,
            device_type,
            state: BlockDeviceState::Initializing,
            size_sectors: 0,
            sector_size: 512,
            physical_block_size: 512,
            read_only: false,
            removable: false,
            rotational: device_type.is_rotational(),
            scheduler: IoScheduler::None,
            queue_depth: 64,
            stats: DiskStats::new(),
            model: None,
            vendor: None,
            serial: None,
            firmware: None,
            supports_trim: device_type.supports_trim(),
            write_cache: true,
            parent: None,
            children: Vec::new(),
            request_queue: None,
        }
    }

    /// Size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size_sectors * self.sector_size as u64
    }

    /// Size in GiB
    pub fn size_gib(&self) -> f32 {
        self.size_bytes() as f32 / (1024.0 * 1024.0 * 1024.0)
    }

    /// Is partition
    pub fn is_partition(&self) -> bool {
        self.parent.is_some()
    }

    /// Has partitions
    pub fn has_partitions(&self) -> bool {
        !self.children.is_empty()
    }

    /// Initialize request queue
    pub fn init_request_queue(&mut self) {
        self.request_queue = Some(RequestQueue::new(self.id, self.queue_depth));
    }

    /// Get request queue
    pub fn request_queue(&self) -> Option<&RequestQueue> {
        self.request_queue.as_ref()
    }

    /// Get request queue mutably
    pub fn request_queue_mut(&mut self) -> Option<&mut RequestQueue> {
        self.request_queue.as_mut()
    }

    /// Submit I/O request
    pub fn submit_io(&mut self, request: IoRequest) {
        if let Some(queue) = self.request_queue.as_mut() {
            queue.submit(request);
        }
    }

    /// Complete I/O request
    pub fn complete_io(&mut self, id: u64, end_time: u64) -> Option<IoRequest> {
        if let Some(queue) = self.request_queue.as_mut() {
            queue.complete(id, end_time)
        } else {
            None
        }
    }
}

/// Partition type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionType {
    /// GPT
    Gpt,
    /// MBR
    Mbr,
    /// Unknown
    Unknown,
}

impl PartitionType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Gpt => "gpt",
            Self::Mbr => "mbr",
            Self::Unknown => "unknown",
        }
    }
}

/// Partition
#[derive(Debug, Clone)]
pub struct Partition {
    /// Device ID
    pub device_id: BlockDeviceId,
    /// Partition number
    pub number: u32,
    /// Start sector
    pub start_sector: u64,
    /// Size in sectors
    pub size_sectors: u64,
    /// Partition type (GPT/MBR)
    pub part_type: PartitionType,
    /// GPT type GUID
    pub type_guid: Option<[u8; 16]>,
    /// GPT unique GUID
    pub unique_guid: Option<[u8; 16]>,
    /// Label
    pub label: Option<String>,
    /// Is bootable
    pub bootable: bool,
}

impl Partition {
    /// Create new partition
    pub fn new(
        device_id: BlockDeviceId,
        number: u32,
        start_sector: u64,
        size_sectors: u64,
    ) -> Self {
        Self {
            device_id,
            number,
            start_sector,
            size_sectors,
            part_type: PartitionType::Unknown,
            type_guid: None,
            unique_guid: None,
            label: None,
            bootable: false,
        }
    }

    /// End sector
    pub fn end_sector(&self) -> u64 {
        self.start_sector + self.size_sectors
    }

    /// Size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.size_sectors * 512
    }
}
