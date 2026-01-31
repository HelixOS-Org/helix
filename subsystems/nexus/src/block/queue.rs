//! Request Queue
//!
//! I/O request handling and queue management.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{BlockDeviceId, IoRequestType, IoScheduler};

/// Queue depth
#[derive(Debug, Clone, Copy)]
pub struct QueueDepth {
    /// Current queue depth
    pub current: u32,
    /// Maximum queue depth
    pub max: u32,
}

impl QueueDepth {
    /// Create new queue depth
    pub fn new(current: u32, max: u32) -> Self {
        Self { current, max }
    }

    /// Utilization
    pub fn utilization(&self) -> f32 {
        if self.max > 0 {
            self.current as f32 / self.max as f32
        } else {
            0.0
        }
    }
}

/// I/O request
#[derive(Debug, Clone)]
pub struct IoRequest {
    /// Request ID
    pub id: u64,
    /// Device
    pub device: BlockDeviceId,
    /// Request type
    pub req_type: IoRequestType,
    /// Sector offset
    pub sector: u64,
    /// Size in sectors
    pub nr_sectors: u32,
    /// Start time (ns)
    pub start_time: u64,
    /// End time (ns)
    pub end_time: Option<u64>,
    /// Was merged
    pub merged: bool,
    /// Priority
    pub priority: i16,
}

impl IoRequest {
    /// Create new request
    pub fn new(
        id: u64,
        device: BlockDeviceId,
        req_type: IoRequestType,
        sector: u64,
        nr_sectors: u32,
        start_time: u64,
    ) -> Self {
        Self {
            id,
            device,
            req_type,
            sector,
            nr_sectors,
            start_time,
            end_time: None,
            merged: false,
            priority: 0,
        }
    }

    /// Complete request
    pub fn complete(&mut self, end_time: u64) {
        self.end_time = Some(end_time);
    }

    /// Is completed
    pub fn is_completed(&self) -> bool {
        self.end_time.is_some()
    }

    /// Latency (ns)
    pub fn latency(&self) -> Option<u64> {
        self.end_time.map(|e| e.saturating_sub(self.start_time))
    }

    /// Size in bytes
    pub fn size_bytes(&self) -> u64 {
        self.nr_sectors as u64 * 512
    }
}

/// Request queue
pub struct RequestQueue {
    /// Device
    pub device: BlockDeviceId,
    /// Queue depth
    pub depth: QueueDepth,
    /// Scheduler
    pub scheduler: IoScheduler,
    /// Pending requests
    pending: Vec<IoRequest>,
    /// Completed requests (for stats)
    completed: Vec<IoRequest>,
    /// Max completed to keep
    max_completed: usize,
    /// Total read bytes
    read_bytes: AtomicU64,
    /// Total write bytes
    write_bytes: AtomicU64,
    /// Total read requests
    read_requests: AtomicU64,
    /// Total write requests
    write_requests: AtomicU64,
    /// Total read time (ns)
    read_time: AtomicU64,
    /// Total write time (ns)
    write_time: AtomicU64,
    /// Merge count
    merges: AtomicU64,
}

impl RequestQueue {
    /// Create new queue
    pub fn new(device: BlockDeviceId, max_depth: u32) -> Self {
        Self {
            device,
            depth: QueueDepth::new(0, max_depth),
            scheduler: IoScheduler::None,
            pending: Vec::new(),
            completed: Vec::new(),
            max_completed: 1000,
            read_bytes: AtomicU64::new(0),
            write_bytes: AtomicU64::new(0),
            read_requests: AtomicU64::new(0),
            write_requests: AtomicU64::new(0),
            read_time: AtomicU64::new(0),
            write_time: AtomicU64::new(0),
            merges: AtomicU64::new(0),
        }
    }

    /// Submit request
    pub fn submit(&mut self, request: IoRequest) {
        self.depth.current += 1;
        self.pending.push(request);
    }

    /// Complete request
    pub fn complete(&mut self, id: u64, end_time: u64) -> Option<IoRequest> {
        if let Some(pos) = self.pending.iter().position(|r| r.id == id) {
            let mut request = self.pending.remove(pos);
            request.complete(end_time);

            self.depth.current = self.depth.current.saturating_sub(1);

            let latency = request.latency().unwrap_or(0);
            let bytes = request.size_bytes();

            match request.req_type {
                IoRequestType::Read => {
                    self.read_requests.fetch_add(1, Ordering::Relaxed);
                    self.read_bytes.fetch_add(bytes, Ordering::Relaxed);
                    self.read_time.fetch_add(latency, Ordering::Relaxed);
                },
                IoRequestType::Write => {
                    self.write_requests.fetch_add(1, Ordering::Relaxed);
                    self.write_bytes.fetch_add(bytes, Ordering::Relaxed);
                    self.write_time.fetch_add(latency, Ordering::Relaxed);
                },
                _ => {},
            }

            if request.merged {
                self.merges.fetch_add(1, Ordering::Relaxed);
            }

            if self.completed.len() >= self.max_completed {
                self.completed.remove(0);
            }
            self.completed.push(request.clone());

            Some(request)
        } else {
            None
        }
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get read stats
    pub fn read_stats(&self) -> (u64, u64, u64) {
        (
            self.read_requests.load(Ordering::Relaxed),
            self.read_bytes.load(Ordering::Relaxed),
            self.read_time.load(Ordering::Relaxed),
        )
    }

    /// Get write stats
    pub fn write_stats(&self) -> (u64, u64, u64) {
        (
            self.write_requests.load(Ordering::Relaxed),
            self.write_bytes.load(Ordering::Relaxed),
            self.write_time.load(Ordering::Relaxed),
        )
    }

    /// Average read latency (ns)
    pub fn avg_read_latency(&self) -> u64 {
        let reqs = self.read_requests.load(Ordering::Relaxed);
        if reqs > 0 {
            self.read_time.load(Ordering::Relaxed) / reqs
        } else {
            0
        }
    }

    /// Average write latency (ns)
    pub fn avg_write_latency(&self) -> u64 {
        let reqs = self.write_requests.load(Ordering::Relaxed);
        if reqs > 0 {
            self.write_time.load(Ordering::Relaxed) / reqs
        } else {
            0
        }
    }

    /// Merge rate
    pub fn merge_rate(&self) -> f32 {
        let total = self.read_requests.load(Ordering::Relaxed)
            + self.write_requests.load(Ordering::Relaxed);
        let merges = self.merges.load(Ordering::Relaxed);
        if total > 0 {
            merges as f32 / total as f32
        } else {
            0.0
        }
    }
}
