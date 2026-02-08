// SPDX-License-Identifier: GPL-2.0
//! Holistic balloon_driver — memory balloon driver for dynamic memory management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Balloon state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalloonState {
    Deflated,
    Inflating,
    Inflated,
    Deflating,
    Frozen,
}

/// Balloon page type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalloonPageType {
    Normal,
    Huge2M,
    Huge1G,
    Compressible,
}

/// Inflation target source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InflationSource {
    Host,
    Oom,
    Policy,
    Manual,
}

/// Balloon page
#[derive(Debug, Clone)]
pub struct BalloonPage {
    pub pfn: u64,
    pub page_type: BalloonPageType,
    pub order: u32,
    pub inflated_at: u64,
}

impl BalloonPage {
    pub fn new(pfn: u64, ptype: BalloonPageType, order: u32, now: u64) -> Self {
        Self { pfn, page_type: ptype, order, inflated_at: now }
    }

    pub fn size_bytes(&self) -> u64 { 4096u64 << self.order }
}

/// Balloon instance
#[derive(Debug)]
pub struct BalloonInstance {
    pub id: u64,
    pub state: BalloonState,
    pub target_pages: u64,
    pub current_pages: u64,
    pub max_pages: u64,
    pub pages: Vec<BalloonPage>,
    pub total_inflated: u64,
    pub total_deflated: u64,
    pub oom_deflations: u64,
    pub last_adjust_at: u64,
}

impl BalloonInstance {
    pub fn new(id: u64, max_pages: u64) -> Self {
        Self {
            id, state: BalloonState::Deflated, target_pages: 0,
            current_pages: 0, max_pages, pages: Vec::new(),
            total_inflated: 0, total_deflated: 0, oom_deflations: 0,
            last_adjust_at: 0,
        }
    }

    pub fn inflate(&mut self, pfn: u64, ptype: BalloonPageType, order: u32, now: u64) -> bool {
        if self.current_pages >= self.max_pages { return false; }
        let count = 1u64 << order;
        self.pages.push(BalloonPage::new(pfn, ptype, order, now));
        self.current_pages += count;
        self.total_inflated += count;
        self.state = BalloonState::Inflating;
        self.last_adjust_at = now;
        true
    }

    pub fn deflate(&mut self, count: u64, now: u64) -> u64 {
        let mut released = 0u64;
        while released < count && !self.pages.is_empty() {
            let page = self.pages.pop().unwrap();
            let n = 1u64 << page.order;
            self.current_pages = self.current_pages.saturating_sub(n);
            released += n;
        }
        self.total_deflated += released;
        self.state = if self.current_pages == 0 { BalloonState::Deflated } else { BalloonState::Deflating };
        self.last_adjust_at = now;
        released
    }

    pub fn utilization(&self) -> f64 {
        if self.max_pages == 0 { return 0.0; }
        self.current_pages as f64 / self.max_pages as f64
    }

    pub fn total_bytes(&self) -> u64 { self.current_pages * 4096 }
}

/// Stats
#[derive(Debug, Clone)]
pub struct BalloonDriverStats {
    pub total_balloons: u32,
    pub total_inflated_pages: u64,
    pub total_deflated_pages: u64,
    pub current_balloon_bytes: u64,
    pub oom_deflations: u64,
    pub avg_utilization: f64,
}

/// Main balloon driver
pub struct HolisticBalloonDriver {
    balloons: BTreeMap<u64, BalloonInstance>,
    next_id: u64,
}

impl HolisticBalloonDriver {
    pub fn new() -> Self { Self { balloons: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, max_pages: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.balloons.insert(id, BalloonInstance::new(id, max_pages));
        id
    }

    pub fn inflate(&mut self, id: u64, pfn: u64, ptype: BalloonPageType, order: u32, now: u64) -> bool {
        self.balloons.get_mut(&id).map(|b| b.inflate(pfn, ptype, order, now)).unwrap_or(false)
    }

    pub fn deflate(&mut self, id: u64, count: u64, now: u64) -> u64 {
        self.balloons.get_mut(&id).map(|b| b.deflate(count, now)).unwrap_or(0)
    }

    pub fn stats(&self) -> BalloonDriverStats {
        let inflated: u64 = self.balloons.values().map(|b| b.total_inflated).sum();
        let deflated: u64 = self.balloons.values().map(|b| b.total_deflated).sum();
        let bytes: u64 = self.balloons.values().map(|b| b.total_bytes()).sum();
        let oom: u64 = self.balloons.values().map(|b| b.oom_deflations).sum();
        let utils: Vec<f64> = self.balloons.values().map(|b| b.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        BalloonDriverStats {
            total_balloons: self.balloons.len() as u32,
            total_inflated_pages: inflated, total_deflated_pages: deflated,
            current_balloon_bytes: bytes, oom_deflations: oom, avg_utilization: avg,
        }
    }
}
