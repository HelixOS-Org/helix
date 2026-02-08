// SPDX-License-Identifier: GPL-2.0
//! Holistic page_reclaim — page frame reclamation.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Reclaim scan type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimScanType {
    Anon,
    File,
    AnonAndFile,
}

/// LRU list type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LruListType {
    InactiveAnon,
    ActiveAnon,
    InactiveFile,
    ActiveFile,
    Unevictable,
}

/// Page reclaim zone
#[derive(Debug)]
pub struct ReclaimZone {
    pub zone_id: u32,
    pub inactive_anon: u64,
    pub active_anon: u64,
    pub inactive_file: u64,
    pub active_file: u64,
    pub unevictable: u64,
    pub pages_scanned: u64,
    pub pages_reclaimed: u64,
    pub scan_priority: u32,
}

impl ReclaimZone {
    pub fn new(id: u32) -> Self {
        Self { zone_id: id, inactive_anon: 0, active_anon: 0, inactive_file: 0, active_file: 0, unevictable: 0, pages_scanned: 0, pages_reclaimed: 0, scan_priority: 12 }
    }

    pub fn reclaim(&mut self, scanned: u64, reclaimed: u64) {
        self.pages_scanned += scanned;
        self.pages_reclaimed += reclaimed;
    }

    pub fn total_reclaimable(&self) -> u64 {
        self.inactive_anon + self.inactive_file
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PageReclaimStats {
    pub total_zones: u32,
    pub total_scanned: u64,
    pub total_reclaimed: u64,
    pub total_reclaimable: u64,
}

/// Main holistic page reclaim
pub struct HolisticPageReclaim {
    zones: BTreeMap<u32, ReclaimZone>,
}

impl HolisticPageReclaim {
    pub fn new() -> Self { Self { zones: BTreeMap::new() } }
    pub fn add_zone(&mut self, id: u32) { self.zones.insert(id, ReclaimZone::new(id)); }

    pub fn update_lru(&mut self, zone: u32, lru: LruListType, count: u64) {
        if let Some(z) = self.zones.get_mut(&zone) {
            match lru {
                LruListType::InactiveAnon => z.inactive_anon = count,
                LruListType::ActiveAnon => z.active_anon = count,
                LruListType::InactiveFile => z.inactive_file = count,
                LruListType::ActiveFile => z.active_file = count,
                LruListType::Unevictable => z.unevictable = count,
            }
        }
    }

    pub fn reclaim(&mut self, zone: u32, scanned: u64, reclaimed: u64) {
        if let Some(z) = self.zones.get_mut(&zone) { z.reclaim(scanned, reclaimed); }
    }

    pub fn stats(&self) -> PageReclaimStats {
        let scanned: u64 = self.zones.values().map(|z| z.pages_scanned).sum();
        let reclaimed: u64 = self.zones.values().map(|z| z.pages_reclaimed).sum();
        let reclaimable: u64 = self.zones.values().map(|z| z.total_reclaimable()).sum();
        PageReclaimStats { total_zones: self.zones.len() as u32, total_scanned: scanned, total_reclaimed: reclaimed, total_reclaimable: reclaimable }
    }
}
