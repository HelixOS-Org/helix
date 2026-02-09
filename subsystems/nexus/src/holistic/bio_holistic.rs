// SPDX-License-Identifier: GPL-2.0
//! Holistic BIO — block I/O request lifecycle management

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// BIO operation
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
}

/// BIO state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioState {
    Queued,
    Submitted,
    InFlight,
    Completing,
    Completed,
    Error,
    Retrying,
}

/// BIO flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioFlag {
    Sync,
    Meta,
    Prio,
    NoMerge,
    Fua,
    Preflush,
    Integrity,
    Throttled,
}

/// BIO request
#[derive(Debug, Clone)]
pub struct BioRequest {
    pub bio_id: u64,
    pub op: BioOp,
    pub state: BioState,
    pub sector: u64,
    pub nr_sectors: u32,
    pub flags: u32,
    pub device_id: u64,
    pub submit_ns: u64,
    pub complete_ns: u64,
    pub retries: u32,
    pub merged_count: u32,
}

impl BioRequest {
    pub fn new(bio_id: u64, op: BioOp, sector: u64, nr_sectors: u32) -> Self {
        Self {
            bio_id, op, state: BioState::Queued, sector, nr_sectors, flags: 0,
            device_id: 0, submit_ns: 0, complete_ns: 0, retries: 0, merged_count: 0,
        }
    }

    #[inline(always)]
    pub fn submit(&mut self, ts_ns: u64) { self.state = BioState::Submitted; self.submit_ns = ts_ns; }
    #[inline(always)]
    pub fn complete(&mut self, ts_ns: u64) { self.state = BioState::Completed; self.complete_ns = ts_ns; }
    #[inline(always)]
    pub fn error(&mut self) { self.state = BioState::Error; }
    #[inline(always)]
    pub fn retry(&mut self) { self.retries += 1; self.state = BioState::Retrying; }
    #[inline(always)]
    pub fn merge(&mut self) { self.merged_count += 1; }

    #[inline(always)]
    pub fn latency_ns(&self) -> u64 { self.complete_ns.saturating_sub(self.submit_ns) }
    #[inline(always)]
    pub fn bytes(&self) -> u64 { self.nr_sectors as u64 * 512 }

    #[inline(always)]
    pub fn is_write(&self) -> bool { matches!(self.op, BioOp::Write | BioOp::Flush | BioOp::WriteZeroes) }
}

/// BIO holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticBioStats {
    pub total_bios: u64,
    pub reads: u64,
    pub writes: u64,
    pub merged: u64,
    pub errors: u64,
    pub total_bytes: u64,
    pub total_latency_ns: u64,
}

/// Main holistic BIO manager
#[derive(Debug)]
pub struct HolisticBio {
    pub in_flight: BTreeMap<u64, BioRequest>,
    pub stats: HolisticBioStats,
}

impl HolisticBio {
    pub fn new() -> Self {
        Self {
            in_flight: BTreeMap::new(),
            stats: HolisticBioStats { total_bios: 0, reads: 0, writes: 0, merged: 0, errors: 0, total_bytes: 0, total_latency_ns: 0 },
        }
    }

    #[inline]
    pub fn submit(&mut self, mut bio: BioRequest, ts_ns: u64) {
        self.stats.total_bios += 1;
        if bio.is_write() { self.stats.writes += 1; } else { self.stats.reads += 1; }
        self.stats.total_bytes += bio.bytes();
        bio.submit(ts_ns);
        self.in_flight.insert(bio.bio_id, bio);
    }

    #[inline]
    pub fn complete(&mut self, bio_id: u64, ts_ns: u64) -> Option<u64> {
        if let Some(bio) = self.in_flight.get_mut(&bio_id) {
            bio.complete(ts_ns);
            let latency = bio.latency_ns();
            self.stats.total_latency_ns += latency;
            self.in_flight.remove(&bio_id);
            Some(latency)
        } else { None }
    }

    #[inline(always)]
    pub fn avg_latency_ns(&self) -> u64 {
        if self.stats.total_bios == 0 { 0 } else { self.stats.total_latency_ns / self.stats.total_bios }
    }
}
