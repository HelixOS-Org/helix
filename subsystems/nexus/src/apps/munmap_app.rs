// SPDX-License-Identifier: MIT
//! # Application Munmap Tracker
//!
//! Per-application memory unmap analytics:
//! - Unmap pattern recognition (bulk, gradual, random)
//! - Leak detection via unfreed region aging
//! - Address space recycling efficiency
//! - Unmap latency profiling

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmapPattern { Bulk, Gradual, Random, ExitCleanup }

#[derive(Debug, Clone)]
pub struct UnmapEvent {
    pub start: u64,
    pub size: u64,
    pub timestamp: u64,
    pub latency_ns: u64,
}

#[derive(Debug, Clone)]
pub struct LeakCandidate {
    pub region_start: u64,
    pub region_size: u64,
    pub mapped_since: u64,
    pub last_access: u64,
    pub age_ticks: u64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Default)]
pub struct MunmapAppStats {
    pub total_unmaps: u64,
    pub total_bytes_unmapped: u64,
    pub avg_latency_ns: u64,
    pub leak_candidates_found: u64,
    pub recycled_addresses: u64,
}

pub struct MunmapAppManager {
    /// app_id → recent unmap events
    app_events: BTreeMap<u64, Vec<UnmapEvent>>,
    /// app_id → suspected leaks
    leak_candidates: BTreeMap<u64, Vec<LeakCandidate>>,
    /// app_id → freed address ranges available for reuse
    free_ranges: BTreeMap<u64, Vec<(u64, u64)>>,
    stats: MunmapAppStats,
    max_events: usize,
    leak_age_threshold: u64,
}

impl MunmapAppManager {
    pub fn new(leak_age_threshold: u64) -> Self {
        Self {
            app_events: BTreeMap::new(),
            leak_candidates: BTreeMap::new(),
            free_ranges: BTreeMap::new(),
            stats: MunmapAppStats::default(),
            max_events: 512,
            leak_age_threshold,
        }
    }

    pub fn record_unmap(&mut self, app_id: u64, start: u64, size: u64, latency_ns: u64, now: u64) {
        let event = UnmapEvent { start, size, timestamp: now, latency_ns };
        let events = self.app_events.entry(app_id).or_insert_with(Vec::new);
        events.push(event);
        if events.len() > self.max_events { events.remove(0); }

        // Add to free ranges for potential recycling
        let ranges = self.free_ranges.entry(app_id).or_insert_with(Vec::new);
        ranges.push((start, size));

        self.stats.total_unmaps += 1;
        self.stats.total_bytes_unmapped += size;
        // EMA for latency
        self.stats.avg_latency_ns = self.stats.avg_latency_ns
            - (self.stats.avg_latency_ns / 8)
            + (latency_ns / 8);
    }

    /// Classify the unmap pattern for an app based on recent events
    pub fn classify_pattern(&self, app_id: u64) -> UnmapPattern {
        let events = match self.app_events.get(&app_id) {
            Some(e) if e.len() >= 3 => e,
            _ => return UnmapPattern::Random,
        };

        let recent = &events[events.len().saturating_sub(20)..];
        if recent.len() < 3 { return UnmapPattern::Random; }

        // Check for bulk: many unmaps in short time window
        let time_span = recent.last().unwrap().timestamp - recent.first().unwrap().timestamp;
        let rate = recent.len() as f64 / (time_span.max(1) as f64);
        if rate > 10.0 && recent.len() > 10 {
            return UnmapPattern::Bulk;
        }

        // Check for gradual: steady rate over time
        let sizes: Vec<u64> = recent.iter().map(|e| e.size).collect();
        let avg_size = sizes.iter().sum::<u64>() / sizes.len() as u64;
        let variance: f64 = sizes.iter().map(|&s| {
            let diff = s as f64 - avg_size as f64;
            diff * diff
        }).sum::<f64>() / sizes.len() as f64;

        if variance < (avg_size as f64 * 0.5).powi(2) {
            return UnmapPattern::Gradual;
        }

        UnmapPattern::Random
    }

    /// Scan for potential memory leaks: regions mapped for too long without access
    pub fn scan_leaks(
        &mut self,
        app_id: u64,
        live_regions: &[(u64, u64, u64)], // (start, size, last_access)
        now: u64,
    ) {
        let mut candidates = Vec::new();
        for &(start, size, last_access) in live_regions {
            let age = now.saturating_sub(last_access);
            if age > self.leak_age_threshold {
                let confidence = (age as f64 / (self.leak_age_threshold as f64 * 10.0)).min(1.0);
                candidates.push(LeakCandidate {
                    region_start: start,
                    region_size: size,
                    mapped_since: 0,
                    last_access,
                    age_ticks: age,
                    confidence,
                });
            }
        }
        self.stats.leak_candidates_found += candidates.len() as u64;
        self.leak_candidates.insert(app_id, candidates);
    }

    /// Find a recycled address range for a new allocation
    pub fn recycle_address(&mut self, app_id: u64, needed_size: u64) -> Option<u64> {
        let ranges = self.free_ranges.get_mut(&app_id)?;
        let idx = ranges.iter().position(|&(_, size)| size >= needed_size)?;
        let (addr, _) = ranges.remove(idx);
        self.stats.recycled_addresses += 1;
        Some(addr)
    }

    pub fn leak_candidates(&self, app_id: u64) -> &[LeakCandidate] {
        self.leak_candidates.get(&app_id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn stats(&self) -> &MunmapAppStats { &self.stats }
}
