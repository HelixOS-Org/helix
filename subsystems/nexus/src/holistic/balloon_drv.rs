// SPDX-License-Identifier: GPL-2.0
//! Holistic balloon_drv — memory balloon driver management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Balloon action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalloonAction {
    Inflate,
    Deflate,
    Freeze,
    Thaw,
}

/// Balloon state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalloonState {
    Idle,
    Inflating,
    Deflating,
    Frozen,
    Error,
}

/// Balloon page range
#[derive(Debug)]
pub struct BalloonPageRange {
    pub start_pfn: u64,
    pub count: u32,
    pub inflated_at: u64,
}

/// Balloon instance
#[derive(Debug)]
pub struct BalloonInstance {
    pub id: u64,
    pub state: BalloonState,
    pub target_pages: u64,
    pub current_pages: u64,
    pub max_pages: u64,
    pub ranges: Vec<BalloonPageRange>,
    pub inflate_count: u64,
    pub deflate_count: u64,
    pub total_inflated: u64,
    pub total_deflated: u64,
}

impl BalloonInstance {
    pub fn new(id: u64, max: u64) -> Self {
        Self { id, state: BalloonState::Idle, target_pages: 0, current_pages: 0, max_pages: max, ranges: Vec::new(), inflate_count: 0, deflate_count: 0, total_inflated: 0, total_deflated: 0 }
    }

    pub fn inflate(&mut self, pfn: u64, count: u32, now: u64) {
        self.ranges.push(BalloonPageRange { start_pfn: pfn, count, inflated_at: now });
        self.current_pages += count as u64;
        self.inflate_count += 1;
        self.total_inflated += count as u64;
        self.state = BalloonState::Inflating;
    }

    pub fn deflate(&mut self, count: u32) -> u32 {
        let mut released = 0u32;
        while released < count && !self.ranges.is_empty() {
            let range = self.ranges.last_mut().unwrap();
            if range.count <= count - released {
                released += range.count;
                self.ranges.pop();
            } else {
                range.count -= count - released;
                released = count;
            }
        }
        self.current_pages = self.current_pages.saturating_sub(released as u64);
        self.deflate_count += 1;
        self.total_deflated += released as u64;
        self.state = BalloonState::Deflating;
        released
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct BalloonDrvStats {
    pub total_instances: u32,
    pub total_inflated_pages: u64,
    pub total_deflated_pages: u64,
    pub current_balloon_pages: u64,
}

/// Main holistic balloon driver
pub struct HolisticBalloonDrv {
    instances: BTreeMap<u64, BalloonInstance>,
    next_id: u64,
}

impl HolisticBalloonDrv {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, max: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.instances.insert(id, BalloonInstance::new(id, max));
        id
    }

    pub fn inflate(&mut self, id: u64, pfn: u64, count: u32, now: u64) {
        if let Some(b) = self.instances.get_mut(&id) { b.inflate(pfn, count, now); }
    }

    pub fn deflate(&mut self, id: u64, count: u32) -> u32 {
        if let Some(b) = self.instances.get_mut(&id) { b.deflate(count) } else { 0 }
    }

    pub fn stats(&self) -> BalloonDrvStats {
        let inflated: u64 = self.instances.values().map(|b| b.total_inflated).sum();
        let deflated: u64 = self.instances.values().map(|b| b.total_deflated).sum();
        let current: u64 = self.instances.values().map(|b| b.current_pages).sum();
        BalloonDrvStats { total_instances: self.instances.len() as u32, total_inflated_pages: inflated, total_deflated_pages: deflated, current_balloon_pages: current }
    }
}
