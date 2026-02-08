// SPDX-License-Identifier: GPL-2.0
//! Coop BIO — cooperative block I/O with request merging and sharing

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop BIO type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopBioType {
    Read,
    Write,
    Flush,
    Discard,
    WriteZeroes,
}

/// Coop BIO state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopBioState {
    Pending,
    Merged,
    Submitted,
    Completed,
    Failed,
}

/// Shared BIO request
#[derive(Debug, Clone)]
pub struct CoopBioRequest {
    pub id: u64,
    pub bio_type: CoopBioType,
    pub state: CoopBioState,
    pub sector: u64,
    pub nr_sectors: u32,
    pub device_id: u64,
    pub shared_count: u32,
    pub merge_count: u32,
}

impl CoopBioRequest {
    pub fn new(id: u64, bio_type: CoopBioType, sector: u64, nr_sectors: u32) -> Self {
        Self {
            id,
            bio_type,
            state: CoopBioState::Pending,
            sector,
            nr_sectors,
            device_id: 0,
            shared_count: 1,
            merge_count: 0,
        }
    }

    pub fn merge(&mut self, other: &CoopBioRequest) -> bool {
        if self.bio_type != other.bio_type || self.device_id != other.device_id {
            return false;
        }
        if self.sector + self.nr_sectors as u64 == other.sector {
            self.nr_sectors += other.nr_sectors;
            self.merge_count += 1;
            return true;
        }
        false
    }

    pub fn share(&mut self) {
        self.shared_count += 1;
    }
    pub fn complete(&mut self) {
        self.state = CoopBioState::Completed;
    }
    pub fn bytes(&self) -> u64 {
        self.nr_sectors as u64 * 512
    }
}

/// Coop BIO stats
#[derive(Debug, Clone)]
pub struct CoopBioStats {
    pub total_requests: u64,
    pub merges: u64,
    pub shares: u64,
    pub completions: u64,
    pub failures: u64,
    pub total_bytes: u64,
}

/// Main coop BIO
#[derive(Debug)]
pub struct CoopBio {
    pub stats: CoopBioStats,
}

impl CoopBio {
    pub fn new() -> Self {
        Self {
            stats: CoopBioStats {
                total_requests: 0,
                merges: 0,
                shares: 0,
                completions: 0,
                failures: 0,
                total_bytes: 0,
            },
        }
    }

    pub fn submit(&mut self, req: &CoopBioRequest) {
        self.stats.total_requests += 1;
        self.stats.total_bytes += req.bytes();
        self.stats.merges += req.merge_count as u64;
        if req.shared_count > 1 {
            self.stats.shares += 1;
        }
    }

    pub fn complete(&mut self, success: bool) {
        if success {
            self.stats.completions += 1;
        } else {
            self.stats.failures += 1;
        }
    }

    pub fn merge_rate(&self) -> f64 {
        if self.stats.total_requests == 0 {
            0.0
        } else {
            self.stats.merges as f64 / self.stats.total_requests as f64
        }
    }
}
