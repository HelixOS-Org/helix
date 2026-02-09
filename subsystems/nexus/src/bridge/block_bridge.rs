//! # Bridge Block Device Bridge
//!
//! Bridges block I/O operations between kernel and devices:
//! - Block device registration and management
//! - Request queue abstraction
//! - Multi-queue (blk-mq) support
//! - I/O scheduler interface
//! - Partition table management
//! - I/O accounting per device

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Block I/O operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioOp {
    Read,
    Write,
    Flush,
    Discard,
    SecureErase,
    WriteZeros,
    ZoneReset,
    ZoneOpen,
    ZoneClose,
    ZoneFinish,
}

/// Block request state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioState {
    Pending,
    Dispatched,
    InFlight,
    Completed,
    Error,
    Requeued,
}

/// I/O scheduler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSched {
    None,
    Mq_Deadline,
    Bfq,
    Kyber,
}

/// Block I/O request
#[derive(Debug, Clone)]
pub struct BlockReq {
    pub id: u64,
    pub device_id: u64,
    pub op: BioOp,
    pub state: BioState,
    pub sector: u64,
    pub nr_sectors: u32,
    pub priority: u8,
    pub hw_queue: u16,
    pub enqueue_ts: u64,
    pub dispatch_ts: u64,
    pub complete_ts: u64,
    pub error: i32,
    pub merged: bool,
}

impl BlockReq {
    pub fn new(id: u64, dev: u64, op: BioOp, sector: u64, count: u32) -> Self {
        Self {
            id, device_id: dev, op, state: BioState::Pending,
            sector, nr_sectors: count, priority: 0, hw_queue: 0,
            enqueue_ts: 0, dispatch_ts: 0, complete_ts: 0,
            error: 0, merged: false,
        }
    }

    #[inline(always)]
    pub fn dispatch(&mut self, ts: u64) { self.state = BioState::Dispatched; self.dispatch_ts = ts; }
    #[inline(always)]
    pub fn start(&mut self) { self.state = BioState::InFlight; }
    #[inline(always)]
    pub fn complete(&mut self, ts: u64) { self.state = BioState::Completed; self.complete_ts = ts; }
    #[inline(always)]
    pub fn fail(&mut self, err: i32, ts: u64) { self.state = BioState::Error; self.error = err; self.complete_ts = ts; }
    #[inline(always)]
    pub fn queue_latency_ns(&self) -> u64 { if self.dispatch_ts > self.enqueue_ts { self.dispatch_ts - self.enqueue_ts } else { 0 } }
    #[inline(always)]
    pub fn service_time_ns(&self) -> u64 { if self.complete_ts > self.dispatch_ts { self.complete_ts - self.dispatch_ts } else { 0 } }
    #[inline(always)]
    pub fn total_latency_ns(&self) -> u64 { if self.complete_ts > self.enqueue_ts { self.complete_ts - self.enqueue_ts } else { 0 } }
    #[inline(always)]
    pub fn bytes(&self) -> u64 { self.nr_sectors as u64 * 512 }
}

/// Block device partition
#[derive(Debug, Clone)]
pub struct Partition {
    pub id: u32,
    pub start_sector: u64,
    pub nr_sectors: u64,
    pub read_ios: u64,
    pub write_ios: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

/// Block device
#[derive(Debug, Clone)]
pub struct BlockDevice {
    pub id: u64,
    pub name: String,
    pub major: u32,
    pub minor: u32,
    pub capacity_sectors: u64,
    pub block_size: u32,
    pub max_sectors: u32,
    pub nr_hw_queues: u16,
    pub queue_depth: u32,
    pub sched: IoSched,
    pub rotational: bool,
    pub partitions: Vec<Partition>,
    pub read_ios: u64,
    pub write_ios: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub io_ticks: u64,
    pub in_flight: u32,
    pub discard_ios: u64,
    pub flush_ios: u64,
}

impl BlockDevice {
    pub fn new(id: u64, name: String, capacity: u64) -> Self {
        Self {
            id, name, major: 0, minor: 0, capacity_sectors: capacity,
            block_size: 512, max_sectors: 256, nr_hw_queues: 1,
            queue_depth: 128, sched: IoSched::None, rotational: false,
            partitions: Vec::new(), read_ios: 0, write_ios: 0,
            read_bytes: 0, write_bytes: 0, io_ticks: 0, in_flight: 0,
            discard_ios: 0, flush_ios: 0,
        }
    }

    #[inline(always)]
    pub fn capacity_bytes(&self) -> u64 { self.capacity_sectors * self.block_size as u64 }

    #[inline]
    pub fn record_complete(&mut self, req: &BlockReq) {
        match req.op {
            BioOp::Read => { self.read_ios += 1; self.read_bytes += req.bytes(); }
            BioOp::Write => { self.write_ios += 1; self.write_bytes += req.bytes(); }
            BioOp::Discard | BioOp::SecureErase => { self.discard_ios += 1; }
            BioOp::Flush => { self.flush_ios += 1; }
            _ => {}
        }
    }

    #[inline(always)]
    pub fn iops(&self) -> u64 { self.read_ios + self.write_ios }
    #[inline(always)]
    pub fn throughput_bytes(&self) -> u64 { self.read_bytes + self.write_bytes }
}

/// Block bridge stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BlockBridgeStats {
    pub total_devices: usize,
    pub total_ios: u64,
    pub total_bytes: u64,
    pub total_in_flight: u32,
    pub avg_latency_ns: u64,
    pub max_latency_ns: u64,
    pub total_errors: u64,
    pub total_merges: u64,
}

/// Bridge block device manager
#[repr(align(64))]
pub struct BridgeBlockBridge {
    devices: BTreeMap<u64, BlockDevice>,
    requests: BTreeMap<u64, BlockReq>,
    stats: BlockBridgeStats,
    next_dev: u64,
    next_req: u64,
}

impl BridgeBlockBridge {
    pub fn new() -> Self {
        Self { devices: BTreeMap::new(), requests: BTreeMap::new(), stats: BlockBridgeStats::default(), next_dev: 1, next_req: 1 }
    }

    #[inline]
    pub fn register_device(&mut self, name: String, capacity: u64) -> u64 {
        let id = self.next_dev; self.next_dev += 1;
        self.devices.insert(id, BlockDevice::new(id, name, capacity));
        id
    }

    #[inline]
    pub fn submit_bio(&mut self, dev: u64, op: BioOp, sector: u64, count: u32, ts: u64) -> u64 {
        let id = self.next_req; self.next_req += 1;
        let mut req = BlockReq::new(id, dev, op, sector, count);
        req.enqueue_ts = ts;
        self.requests.insert(id, req);
        if let Some(d) = self.devices.get_mut(&dev) { d.in_flight += 1; }
        id
    }

    #[inline(always)]
    pub fn dispatch_req(&mut self, id: u64, ts: u64) {
        if let Some(r) = self.requests.get_mut(&id) { r.dispatch(ts); }
    }

    #[inline]
    pub fn complete_req(&mut self, id: u64, ts: u64) {
        let dev_id = self.requests.get(&id).map(|r| r.device_id);
        if let Some(r) = self.requests.get_mut(&id) { r.complete(ts); }
        if let Some(did) = dev_id {
            if let Some(d) = self.devices.get_mut(&did) {
                d.in_flight = d.in_flight.saturating_sub(1);
                if let Some(r) = self.requests.get(&id) { d.record_complete(r); }
            }
        }
    }

    #[inline]
    pub fn error_req(&mut self, id: u64, err: i32, ts: u64) {
        if let Some(r) = self.requests.get_mut(&id) {
            let dev = r.device_id;
            r.fail(err, ts);
            if let Some(d) = self.devices.get_mut(&dev) { d.in_flight = d.in_flight.saturating_sub(1); }
        }
    }

    #[inline(always)]
    pub fn set_sched(&mut self, dev: u64, sched: IoSched) { if let Some(d) = self.devices.get_mut(&dev) { d.sched = sched; } }
    #[inline(always)]
    pub fn set_queue_depth(&mut self, dev: u64, depth: u32) { if let Some(d) = self.devices.get_mut(&dev) { d.queue_depth = depth; } }

    pub fn recompute(&mut self) {
        self.stats.total_devices = self.devices.len();
        self.stats.total_ios = self.devices.values().map(|d| d.iops()).sum();
        self.stats.total_bytes = self.devices.values().map(|d| d.throughput_bytes()).sum();
        self.stats.total_in_flight = self.devices.values().map(|d| d.in_flight).sum();
        let done: Vec<&BlockReq> = self.requests.values().filter(|r| r.state == BioState::Completed).collect();
        if !done.is_empty() {
            let total: u64 = done.iter().map(|r| r.total_latency_ns()).sum();
            self.stats.avg_latency_ns = total / done.len() as u64;
            self.stats.max_latency_ns = done.iter().map(|r| r.total_latency_ns()).max().unwrap_or(0);
        }
        self.stats.total_errors = self.requests.values().filter(|r| r.state == BioState::Error).count() as u64;
        self.stats.total_merges = self.requests.values().filter(|r| r.merged).count() as u64;
    }

    #[inline(always)]
    pub fn device(&self, id: u64) -> Option<&BlockDevice> { self.devices.get(&id) }
    #[inline(always)]
    pub fn request(&self, id: u64) -> Option<&BlockReq> { self.requests.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &BlockBridgeStats { &self.stats }
}
