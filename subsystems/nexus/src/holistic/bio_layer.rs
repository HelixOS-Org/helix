// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic bio layer — Block I/O request tracking and merging
//!
//! Models the block I/O layer with bio splitting, merging, bounce buffering,
//! integrity metadata, and per-device queue depth tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Bio operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioOp {
    Read,
    Write,
    Flush,
    Discard,
    SecureErase,
    WriteZeroes,
    ZoneReset,
    ZoneOpen,
    ZoneClose,
    ZoneFinish,
}

/// Bio flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioFlag {
    Sync,
    Meta,
    Prio,
    Fua,
    Preflush,
    Nomerge,
    Integrity,
    Bounce,
}

/// Bio state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioState {
    Submitted,
    Merged,
    Split,
    InFlight,
    Completed,
    Error,
}

/// A block I/O request.
#[derive(Debug, Clone)]
pub struct BioRequest {
    pub bio_id: u64,
    pub device_id: u32,
    pub op: BioOp,
    pub sector: u64,
    pub nr_sectors: u32,
    pub state: BioState,
    pub flags: Vec<BioFlag>,
    pub priority: u16,
    pub submit_time: u64,
    pub complete_time: u64,
    pub parent_bio: Option<u64>,
    pub bytes_done: u64,
}

impl BioRequest {
    pub fn new(bio_id: u64, device_id: u32, op: BioOp, sector: u64, nr_sectors: u32) -> Self {
        Self {
            bio_id,
            device_id,
            op,
            sector,
            nr_sectors,
            state: BioState::Submitted,
            flags: Vec::new(),
            priority: 0,
            submit_time: 0,
            complete_time: 0,
            parent_bio: None,
            bytes_done: 0,
        }
    }

    pub fn byte_size(&self) -> u64 {
        self.nr_sectors as u64 * 512
    }

    pub fn latency(&self) -> u64 {
        self.complete_time.saturating_sub(self.submit_time)
    }

    pub fn can_merge_with(&self, other: &BioRequest) -> bool {
        if self.device_id != other.device_id {
            return false;
        }
        if self.op != other.op {
            return false;
        }
        let self_end = self.sector + self.nr_sectors as u64;
        self_end == other.sector || other.sector + other.nr_sectors as u64 == self.sector
    }
}

/// Per-device queue state.
#[derive(Debug, Clone)]
pub struct BioDeviceQueue {
    pub device_id: u32,
    pub queue_depth: u32,
    pub max_queue_depth: u32,
    pub inflight_reads: u32,
    pub inflight_writes: u32,
    pub total_submitted: u64,
    pub total_completed: u64,
    pub total_merged: u64,
    pub total_split: u64,
}

impl BioDeviceQueue {
    pub fn new(device_id: u32, max_depth: u32) -> Self {
        Self {
            device_id,
            queue_depth: 0,
            max_queue_depth: max_depth,
            inflight_reads: 0,
            inflight_writes: 0,
            total_submitted: 0,
            total_completed: 0,
            total_merged: 0,
            total_split: 0,
        }
    }
}

/// Statistics for bio layer.
#[derive(Debug, Clone)]
pub struct BioLayerStats {
    pub total_bios: u64,
    pub total_reads: u64,
    pub total_writes: u64,
    pub total_merged: u64,
    pub total_split: u64,
    pub total_bytes: u64,
    pub avg_latency_ns: u64,
}

/// Main holistic bio layer manager.
pub struct HolisticBioLayer {
    pub bios: BTreeMap<u64, BioRequest>,
    pub device_queues: BTreeMap<u32, BioDeviceQueue>,
    pub next_bio_id: u64,
    pub stats: BioLayerStats,
}

impl HolisticBioLayer {
    pub fn new() -> Self {
        Self {
            bios: BTreeMap::new(),
            device_queues: BTreeMap::new(),
            next_bio_id: 1,
            stats: BioLayerStats {
                total_bios: 0,
                total_reads: 0,
                total_writes: 0,
                total_merged: 0,
                total_split: 0,
                total_bytes: 0,
                avg_latency_ns: 0,
            },
        }
    }

    pub fn submit_bio(&mut self, device_id: u32, op: BioOp, sector: u64, nr_sectors: u32) -> u64 {
        let id = self.next_bio_id;
        self.next_bio_id += 1;
        let bio = BioRequest::new(id, device_id, op, sector, nr_sectors);
        self.stats.total_bios += 1;
        self.stats.total_bytes += bio.byte_size();
        match op {
            BioOp::Read => self.stats.total_reads += 1,
            BioOp::Write => self.stats.total_writes += 1,
            _ => {}
        }
        self.bios.insert(id, bio);
        id
    }

    pub fn register_device(&mut self, device_id: u32, max_depth: u32) {
        let queue = BioDeviceQueue::new(device_id, max_depth);
        self.device_queues.insert(device_id, queue);
    }

    pub fn bio_count(&self) -> usize {
        self.bios.len()
    }
}
