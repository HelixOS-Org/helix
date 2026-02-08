//! # Holistic IO Scheduler
//!
//! System-wide IO scheduling optimization:
//! - Multi-queue deadline scheduling
//! - Request merging and coalescing
//! - Priority-based IO queuing
//! - Throughput/latency balancing
//! - Per-device bandwidth allocation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// IO TYPES
// ============================================================================

/// IO direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoDirection {
    Read,
    Write,
    Flush,
    Discard,
}

/// IO priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPriorityClass {
    /// Real-time IO
    RealTime,
    /// Best effort (default)
    BestEffort,
    /// Background / idle
    Idle,
}

/// Scheduling algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSchedAlgo {
    /// Deadline-based
    Deadline,
    /// Completely fair queueing
    Cfq,
    /// Budget fair queueing
    Bfq,
    /// Simple FIFO (for NVMe)
    None,
}

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoDeviceType {
    Hdd,
    Ssd,
    Nvme,
    RamDisk,
    Network,
}

// ============================================================================
// IO REQUEST
// ============================================================================

/// IO request
#[derive(Debug, Clone)]
pub struct IoRequest {
    /// Request ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// Direction
    pub direction: IoDirection,
    /// Starting sector
    pub sector: u64,
    /// Number of sectors
    pub count: u32,
    /// Priority
    pub priority: IoPriorityClass,
    /// Submit time (ns)
    pub submit_ns: u64,
    /// Deadline (ns, 0 = no deadline)
    pub deadline_ns: u64,
    /// Merged count
    pub merged: u32,
}

impl IoRequest {
    pub fn new(id: u64, pid: u64, direction: IoDirection, sector: u64, count: u32) -> Self {
        Self {
            id,
            pid,
            direction,
            sector,
            count,
            priority: IoPriorityClass::BestEffort,
            submit_ns: 0,
            deadline_ns: 0,
            merged: 0,
        }
    }

    /// Check if two requests can be merged (adjacent sectors, same direction)
    pub fn can_merge(&self, other: &IoRequest) -> bool {
        if self.direction != other.direction {
            return false;
        }
        let self_end = self.sector + self.count as u64;
        let other_end = other.sector + other.count as u64;
        self_end == other.sector || other_end == self.sector
    }

    /// Merge another request into this one
    pub fn merge_with(&mut self, other: &IoRequest) -> bool {
        if !self.can_merge(other) {
            return false;
        }
        let self_end = self.sector + self.count as u64;
        if self_end == other.sector {
            // Back merge
            self.count += other.count;
        } else {
            // Front merge
            self.sector = other.sector;
            self.count += other.count;
        }
        self.merged += 1;
        // Take earlier deadline
        if other.deadline_ns > 0 {
            if self.deadline_ns == 0 || other.deadline_ns < self.deadline_ns {
                self.deadline_ns = other.deadline_ns;
            }
        }
        true
    }

    /// Size in bytes (assuming 512-byte sectors)
    pub fn size_bytes(&self) -> u64 {
        self.count as u64 * 512
    }
}

// ============================================================================
// DEVICE QUEUE
// ============================================================================

/// Per-device IO queue
#[derive(Debug)]
pub struct DeviceQueue {
    /// Device ID hash (FNV-1a)
    pub device_hash: u64,
    /// Device type
    pub device_type: IoDeviceType,
    /// Scheduling algorithm
    pub algorithm: IoSchedAlgo,
    /// Read queue
    read_queue: Vec<IoRequest>,
    /// Write queue
    write_queue: Vec<IoRequest>,
    /// Max queue depth
    pub max_depth: usize,
    /// Bandwidth limit (bytes/sec, 0 = unlimited)
    pub bandwidth_limit: u64,
    /// Current throughput EMA
    pub throughput_ema: f64,
    /// Average latency EMA (ns)
    pub latency_ema: f64,
    /// Total requests dispatched
    pub dispatched: u64,
    /// Total merges
    pub merges: u64,
}

impl DeviceQueue {
    pub fn new(device_name: &str, device_type: IoDeviceType) -> Self {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in device_name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let algorithm = match device_type {
            IoDeviceType::Hdd => IoSchedAlgo::Bfq,
            IoDeviceType::Ssd => IoSchedAlgo::Deadline,
            IoDeviceType::Nvme => IoSchedAlgo::None,
            IoDeviceType::RamDisk => IoSchedAlgo::None,
            IoDeviceType::Network => IoSchedAlgo::Cfq,
        };
        let max_depth = match device_type {
            IoDeviceType::Hdd => 32,
            IoDeviceType::Ssd => 128,
            IoDeviceType::Nvme => 1024,
            IoDeviceType::RamDisk => 2048,
            IoDeviceType::Network => 64,
        };
        Self {
            device_hash: hash,
            device_type,
            algorithm,
            read_queue: Vec::new(),
            write_queue: Vec::new(),
            max_depth,
            bandwidth_limit: 0,
            throughput_ema: 0.0,
            latency_ema: 0.0,
            dispatched: 0,
            merges: 0,
        }
    }

    /// Submit request (attempt merge first)
    pub fn submit(&mut self, req: IoRequest) -> bool {
        let queue = match req.direction {
            IoDirection::Read => &mut self.read_queue,
            IoDirection::Write | IoDirection::Flush | IoDirection::Discard => &mut self.write_queue,
        };
        if queue.len() >= self.max_depth {
            return false;
        }
        // Try to merge with existing
        for existing in queue.iter_mut() {
            if existing.can_merge(&req) {
                existing.merge_with(&req);
                self.merges += 1;
                return true;
            }
        }
        queue.push(req);
        true
    }

    /// Dispatch next request (deadline-based)
    pub fn dispatch(&mut self, now_ns: u64) -> Option<IoRequest> {
        // Check for expired deadlines first
        let read_deadline = self.find_earliest_deadline(&self.read_queue, now_ns);
        let write_deadline = self.find_earliest_deadline(&self.write_queue, now_ns);

        let from_read = match (read_deadline, write_deadline) {
            (Some(r), Some(w)) => r <= w,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => {
                // No deadlines, prefer reads for latency
                !self.read_queue.is_empty()
            }
        };

        let queue = if from_read {
            &mut self.read_queue
        } else {
            &mut self.write_queue
        };

        if queue.is_empty() {
            // Try the other queue
            let other = if from_read {
                &mut self.write_queue
            } else {
                &mut self.read_queue
            };
            if other.is_empty() {
                return None;
            }
            let req = other.remove(0);
            self.dispatched += 1;
            return Some(req);
        }

        let req = queue.remove(0);
        self.dispatched += 1;
        Some(req)
    }

    fn find_earliest_deadline(&self, queue: &[IoRequest], _now_ns: u64) -> Option<u64> {
        queue.iter()
            .filter(|r| r.deadline_ns > 0)
            .map(|r| r.deadline_ns)
            .min()
    }

    /// Record completion
    pub fn record_completion(&mut self, latency_ns: u64, bytes: u64) {
        self.latency_ema = 0.9 * self.latency_ema + 0.1 * latency_ns as f64;
        let elapsed_sec = latency_ns as f64 / 1_000_000_000.0;
        if elapsed_sec > 0.0 {
            let throughput = bytes as f64 / elapsed_sec;
            self.throughput_ema = 0.9 * self.throughput_ema + 0.1 * throughput;
        }
    }

    /// Queue depth
    pub fn queue_depth(&self) -> usize {
        self.read_queue.len() + self.write_queue.len()
    }

    /// Utilization ratio
    pub fn utilization(&self) -> f64 {
        if self.max_depth == 0 {
            return 0.0;
        }
        self.queue_depth() as f64 / self.max_depth as f64
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// IO scheduler stats
#[derive(Debug, Clone, Default)]
pub struct HolisticIoSchedulerStats {
    /// Device count
    pub device_count: usize,
    /// Total dispatched
    pub total_dispatched: u64,
    /// Total merges
    pub total_merges: u64,
    /// Merge ratio
    pub merge_ratio: f64,
    /// Max device utilization
    pub max_utilization: f64,
}

/// System-wide IO scheduler
pub struct HolisticIoScheduler {
    /// Device queues
    devices: BTreeMap<u64, DeviceQueue>,
    /// Next request ID
    next_id: u64,
    /// Stats
    stats: HolisticIoSchedulerStats,
}

impl HolisticIoScheduler {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            next_id: 1,
            stats: HolisticIoSchedulerStats::default(),
        }
    }

    /// Register device
    pub fn register_device(&mut self, name: &str, dev_type: IoDeviceType) -> u64 {
        let queue = DeviceQueue::new(name, dev_type);
        let hash = queue.device_hash;
        self.devices.insert(hash, queue);
        self.update_stats();
        hash
    }

    /// Submit IO request to device
    pub fn submit(&mut self, device_hash: u64, pid: u64, dir: IoDirection, sector: u64, count: u32) -> Option<u64> {
        let id = self.next_id;
        self.next_id += 1;
        let req = IoRequest::new(id, pid, dir, sector, count);
        if let Some(queue) = self.devices.get_mut(&device_hash) {
            if queue.submit(req) {
                self.update_stats();
                return Some(id);
            }
        }
        None
    }

    /// Dispatch next request from device
    pub fn dispatch(&mut self, device_hash: u64, now_ns: u64) -> Option<IoRequest> {
        if let Some(queue) = self.devices.get_mut(&device_hash) {
            let result = queue.dispatch(now_ns);
            self.update_stats();
            result
        } else {
            None
        }
    }

    fn update_stats(&mut self) {
        self.stats.device_count = self.devices.len();
        self.stats.total_dispatched = self.devices.values().map(|d| d.dispatched).sum();
        self.stats.total_merges = self.devices.values().map(|d| d.merges).sum();
        let total_requests = self.stats.total_dispatched + self.stats.total_merges;
        self.stats.merge_ratio = if total_requests > 0 {
            self.stats.total_merges as f64 / total_requests as f64
        } else {
            0.0
        };
        self.stats.max_utilization = self.devices.values()
            .map(|d| d.utilization())
            .fold(0.0_f64, f64::max);
    }

    /// Stats
    pub fn stats(&self) -> &HolisticIoSchedulerStats {
        &self.stats
    }
}
