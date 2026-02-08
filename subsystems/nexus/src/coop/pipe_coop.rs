// SPDX-License-Identifier: GPL-2.0
//! Coop pipe — cooperative pipe buffer sharing

extern crate alloc;

/// Pipe coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipeCoopEvent { BufferShare, SpliceForward, TeeClone, CapacityPool }

/// Pipe coop record
#[derive(Debug, Clone)]
pub struct PipeCoopRecord {
    pub event: PipeCoopEvent,
    pub bytes: u64,
    pub source_fd: i32,
    pub target_fd: i32,
    pub participants: u32,
}

impl PipeCoopRecord {
    pub fn new(event: PipeCoopEvent) -> Self { Self { event, bytes: 0, source_fd: -1, target_fd: -1, participants: 0 } }
}

/// Pipe coop stats
#[derive(Debug, Clone)]
pub struct PipeCoopStats { pub total_events: u64, pub shares: u64, pub splices: u64, pub bytes_saved: u64 }

/// Main coop pipe
#[derive(Debug)]
pub struct CoopPipe { pub stats: PipeCoopStats }

impl CoopPipe {
    pub fn new() -> Self { Self { stats: PipeCoopStats { total_events: 0, shares: 0, splices: 0, bytes_saved: 0 } } }
    pub fn record(&mut self, rec: &PipeCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            PipeCoopEvent::BufferShare | PipeCoopEvent::CapacityPool => self.stats.shares += 1,
            PipeCoopEvent::SpliceForward | PipeCoopEvent::TeeClone => self.stats.splices += 1,
        }
        self.stats.bytes_saved += rec.bytes;
    }
}
