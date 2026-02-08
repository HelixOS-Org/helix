// SPDX-License-Identifier: GPL-2.0
//! Holistic dirty_writeback — dirty page writeback management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Writeback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritebackState {
    Idle,
    Active,
    Congested,
    BackgroundThrottle,
    ForegroundThrottle,
}

/// Dirty page info
#[derive(Debug)]
pub struct DirtyPageInfo {
    pub inode: u64,
    pub offset: u64,
    pub dirtied_at: u64,
    pub age_ms: u64,
}

/// BDI writeback
#[derive(Debug)]
pub struct BdiWriteback {
    pub bdi_id: u64,
    pub state: WritebackState,
    pub dirty_pages: u64,
    pub writeback_pages: u64,
    pub reclaimable_pages: u64,
    pub bandwidth_bps: u64,
    pub dirty_thresh: u64,
    pub bg_thresh: u64,
    pub total_written: u64,
    pub total_writebacks: u64,
}

impl BdiWriteback {
    pub fn new(id: u64) -> Self {
        Self { bdi_id: id, state: WritebackState::Idle, dirty_pages: 0, writeback_pages: 0, reclaimable_pages: 0, bandwidth_bps: 0, dirty_thresh: 40, bg_thresh: 10, total_written: 0, total_writebacks: 0 }
    }

    pub fn mark_dirty(&mut self, count: u64) {
        self.dirty_pages += count;
        self.update_state();
    }

    pub fn writeback(&mut self, count: u64) {
        let wb = count.min(self.dirty_pages);
        self.dirty_pages -= wb;
        self.writeback_pages += wb;
        self.total_writebacks += 1;
    }

    pub fn complete_writeback(&mut self, count: u64) {
        let done = count.min(self.writeback_pages);
        self.writeback_pages -= done;
        self.total_written += done;
        self.update_state();
    }

    fn update_state(&mut self) {
        let ratio = if self.dirty_thresh == 0 { 0 } else { self.dirty_pages * 100 / self.dirty_thresh.max(1) };
        if ratio > 90 { self.state = WritebackState::ForegroundThrottle; }
        else if ratio > 70 { self.state = WritebackState::Congested; }
        else if ratio > self.bg_thresh { self.state = WritebackState::Active; }
        else { self.state = WritebackState::Idle; }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct DirtyWritebackStats {
    pub total_bdis: u32,
    pub total_dirty_pages: u64,
    pub total_writeback_pages: u64,
    pub total_written: u64,
    pub congested_count: u32,
}

/// Main holistic dirty writeback
pub struct HolisticDirtyWriteback {
    bdis: BTreeMap<u64, BdiWriteback>,
    next_id: u64,
}

impl HolisticDirtyWriteback {
    pub fn new() -> Self { Self { bdis: BTreeMap::new(), next_id: 1 } }

    pub fn register_bdi(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.bdis.insert(id, BdiWriteback::new(id));
        id
    }

    pub fn mark_dirty(&mut self, bdi: u64, count: u64) {
        if let Some(b) = self.bdis.get_mut(&bdi) { b.mark_dirty(count); }
    }

    pub fn writeback(&mut self, bdi: u64, count: u64) {
        if let Some(b) = self.bdis.get_mut(&bdi) { b.writeback(count); }
    }

    pub fn complete(&mut self, bdi: u64, count: u64) {
        if let Some(b) = self.bdis.get_mut(&bdi) { b.complete_writeback(count); }
    }

    pub fn stats(&self) -> DirtyWritebackStats {
        let dirty: u64 = self.bdis.values().map(|b| b.dirty_pages).sum();
        let wb: u64 = self.bdis.values().map(|b| b.writeback_pages).sum();
        let written: u64 = self.bdis.values().map(|b| b.total_written).sum();
        let congested = self.bdis.values().filter(|b| matches!(b.state, WritebackState::Congested | WritebackState::ForegroundThrottle)).count() as u32;
        DirtyWritebackStats { total_bdis: self.bdis.len() as u32, total_dirty_pages: dirty, total_writeback_pages: wb, total_written: written, congested_count: congested }
    }
}
