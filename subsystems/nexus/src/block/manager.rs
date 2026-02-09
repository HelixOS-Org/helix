//! Block Manager
//!
//! Block device and partition management.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use super::{
    BlockDevice, BlockDeviceId, BlockDeviceState, IoRequest, Partition, WorkloadAnalysis,
    WorkloadType,
};

/// Block device manager
pub struct BlockManager {
    /// Devices
    devices: BTreeMap<BlockDeviceId, BlockDevice>,
    /// Partitions
    partitions: BTreeMap<BlockDeviceId, Partition>,
    /// Device count
    device_count: AtomicU32,
    /// Total I/O requests
    total_requests: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl BlockManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            partitions: BTreeMap::new(),
            device_count: AtomicU32::new(0),
            total_requests: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Register device
    #[inline]
    pub fn register_device(&mut self, mut device: BlockDevice) {
        device.state = BlockDeviceState::Active;
        device.init_request_queue();
        self.device_count.fetch_add(1, Ordering::Relaxed);
        self.devices.insert(device.id, device);
    }

    /// Get device
    #[inline(always)]
    pub fn get_device(&self, id: BlockDeviceId) -> Option<&BlockDevice> {
        self.devices.get(&id)
    }

    /// Get device mutably
    #[inline(always)]
    pub fn get_device_mut(&mut self, id: BlockDeviceId) -> Option<&mut BlockDevice> {
        self.devices.get_mut(&id)
    }

    /// Register partition
    #[inline]
    pub fn register_partition(&mut self, partition: Partition, parent: BlockDeviceId) {
        if let Some(parent_dev) = self.devices.get_mut(&parent) {
            parent_dev.children.push(partition.device_id);
        }
        self.partitions.insert(partition.device_id, partition);
    }

    /// Get partition
    #[inline(always)]
    pub fn get_partition(&self, id: BlockDeviceId) -> Option<&Partition> {
        self.partitions.get(&id)
    }

    /// Get all disks (non-partition devices)
    #[inline]
    pub fn disks(&self) -> Vec<&BlockDevice> {
        self.devices
            .values()
            .filter(|d| d.parent.is_none())
            .collect()
    }

    /// Get all SSDs
    #[inline]
    pub fn ssds(&self) -> Vec<&BlockDevice> {
        self.devices
            .values()
            .filter(|d| d.device_type.is_solid_state())
            .collect()
    }

    /// Get all HDDs
    #[inline]
    pub fn hdds(&self) -> Vec<&BlockDevice> {
        self.devices
            .values()
            .filter(|d| d.device_type.is_rotational())
            .collect()
    }

    /// Submit I/O request
    #[inline]
    pub fn submit_io(&mut self, device_id: BlockDeviceId, request: IoRequest) {
        if let Some(device) = self.devices.get_mut(&device_id) {
            device.submit_io(request);
            self.total_requests.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Complete I/O request
    #[inline]
    pub fn complete_io(
        &mut self,
        device_id: BlockDeviceId,
        request_id: u64,
        end_time: u64,
    ) -> Option<IoRequest> {
        if let Some(device) = self.devices.get_mut(&device_id) {
            device.complete_io(request_id, end_time)
        } else {
            None
        }
    }

    /// Get device count
    #[inline(always)]
    pub fn device_count(&self) -> u32 {
        self.device_count.load(Ordering::Relaxed)
    }

    /// Analyze workload for device
    pub fn analyze_workload(&self, device_id: BlockDeviceId) -> Option<WorkloadAnalysis> {
        let device = self.devices.get(&device_id)?;
        let queue = device.request_queue()?;

        let (read_reqs, read_bytes, _) = queue.read_stats();
        let (write_reqs, write_bytes, _) = queue.write_stats();

        let total_reqs = read_reqs + write_reqs;
        let read_ratio = if total_reqs > 0 {
            read_reqs as f32 / total_reqs as f32
        } else {
            0.5
        };

        let total_bytes = read_bytes + write_bytes;
        let avg_io_size = if total_reqs > 0 {
            total_bytes / total_reqs
        } else {
            4096
        };

        let mut analysis = WorkloadAnalysis {
            workload_type: WorkloadType::Unknown,
            read_ratio,
            sequential_ratio: 0.5,
            avg_io_size,
            iops: total_reqs,
            throughput_mbps: total_bytes as f32 / (1024.0 * 1024.0),
            queue_utilization: queue.depth.utilization(),
        };

        analysis.classify();
        Some(analysis)
    }

    /// Get devices iterator
    #[inline(always)]
    pub fn devices(&self) -> impl Iterator<Item = &BlockDevice> {
        self.devices.values()
    }
}

impl Default for BlockManager {
    fn default() -> Self {
        Self::new()
    }
}
