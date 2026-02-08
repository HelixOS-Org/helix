// SPDX-License-Identifier: MIT
//! # Application Swap Policy Engine
//!
//! Per-application swap strategy:
//! - Swap-in/swap-out rate tracking per app
//! - Working set vs RSS divergence detection
//! - zswap compression ratio estimation
//! - Swap priority scoring (which app to swap first)
//! - Thrash detection: excessive swap-in/out cycles

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapState { Active, SwapCandidate, PartialSwap, FullySwapped }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrashLevel { None, Mild, Moderate, Severe }

#[derive(Debug, Clone)]
pub struct AppSwapProfile {
    pub app_id: u64,
    pub rss_pages: u64,
    pub working_set_pages: u64,
    pub swap_in_count: u64,
    pub swap_out_count: u64,
    pub swap_in_rate: f64,   // per second
    pub swap_out_rate: f64,
    pub compression_ratio: f64,
    pub state: SwapState,
    pub priority_score: f64,
    pub last_updated: u64,
}

impl AppSwapProfile {
    pub fn divergence(&self) -> f64 {
        if self.rss_pages == 0 { return 0.0; }
        1.0 - (self.working_set_pages as f64 / self.rss_pages as f64).min(1.0)
    }

    pub fn thrash_level(&self) -> ThrashLevel {
        let churn = self.swap_in_rate + self.swap_out_rate;
        if churn > 500.0 { ThrashLevel::Severe }
        else if churn > 100.0 { ThrashLevel::Moderate }
        else if churn > 20.0 { ThrashLevel::Mild }
        else { ThrashLevel::None }
    }

    pub fn swappable_pages(&self) -> u64 {
        self.rss_pages.saturating_sub(self.working_set_pages)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SwapAppStats {
    pub total_swap_ins: u64,
    pub total_swap_outs: u64,
    pub total_compressed_bytes: u64,
    pub total_original_bytes: u64,
    pub thrash_events: u64,
    pub proactive_swaps: u64,
}

pub struct SwapAppManager {
    profiles: BTreeMap<u64, AppSwapProfile>,
    /// Ring buffer of recent swap events: (timestamp, app_id, is_in, page_count)
    event_ring: Vec<(u64, u64, bool, u64)>,
    ring_head: usize,
    ring_capacity: usize,
    stats: SwapAppStats,
}

impl SwapAppManager {
    pub fn new() -> Self {
        let cap = 2048;
        Self {
            profiles: BTreeMap::new(),
            event_ring: Vec::with_capacity(cap),
            ring_head: 0,
            ring_capacity: cap,
            stats: SwapAppStats::default(),
        }
    }

    pub fn update_profile(
        &mut self, app_id: u64, rss_pages: u64,
        working_set: u64, now: u64,
    ) {
        let profile = self.profiles.entry(app_id).or_insert(AppSwapProfile {
            app_id,
            rss_pages,
            working_set_pages: working_set,
            swap_in_count: 0, swap_out_count: 0,
            swap_in_rate: 0.0, swap_out_rate: 0.0,
            compression_ratio: 1.0,
            state: SwapState::Active,
            priority_score: 0.0,
            last_updated: now,
        });
        profile.rss_pages = rss_pages;
        profile.working_set_pages = working_set;

        // Compute priority: higher = more eligible for swap-out
        let divergence = profile.divergence();
        let age_factor = (now.saturating_sub(profile.last_updated)) as f64 / 1_000_000.0;
        profile.priority_score = divergence * 0.6 + age_factor.min(1.0) * 0.4;

        // Update state
        profile.state = if profile.swappable_pages() > profile.rss_pages / 2 {
            SwapState::SwapCandidate
        } else {
            SwapState::Active
        };
        profile.last_updated = now;
    }

    pub fn record_swap_in(&mut self, app_id: u64, page_count: u64, now: u64) {
        if let Some(p) = self.profiles.get_mut(&app_id) {
            p.swap_in_count += page_count;
            p.swap_in_rate = p.swap_in_rate * 0.9 + page_count as f64 * 0.1;
            if p.thrash_level() == ThrashLevel::Severe {
                self.stats.thrash_events += 1;
            }
        }
        self.push_event(now, app_id, true, page_count);
        self.stats.total_swap_ins += page_count;
    }

    pub fn record_swap_out(
        &mut self, app_id: u64, page_count: u64,
        original_bytes: u64, compressed_bytes: u64, now: u64,
    ) {
        if let Some(p) = self.profiles.get_mut(&app_id) {
            p.swap_out_count += page_count;
            p.swap_out_rate = p.swap_out_rate * 0.9 + page_count as f64 * 0.1;
            if compressed_bytes > 0 {
                p.compression_ratio = original_bytes as f64 / compressed_bytes as f64;
            }
        }
        self.push_event(now, app_id, false, page_count);
        self.stats.total_swap_outs += page_count;
        self.stats.total_original_bytes += original_bytes;
        self.stats.total_compressed_bytes += compressed_bytes;
    }

    /// Select top N apps to swap out based on priority
    pub fn select_swap_candidates(&self, n: usize) -> Vec<u64> {
        let mut candidates: Vec<_> = self.profiles.iter()
            .filter(|(_, p)| p.state == SwapState::SwapCandidate || p.state == SwapState::Active)
            .filter(|(_, p)| p.thrash_level() == ThrashLevel::None || p.thrash_level() == ThrashLevel::Mild)
            .map(|(id, p)| (*id, p.priority_score))
            .collect();
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        candidates.into_iter().take(n).map(|(id, _)| id).collect()
    }

    fn push_event(&mut self, ts: u64, app_id: u64, is_in: bool, pages: u64) {
        if self.event_ring.len() < self.ring_capacity {
            self.event_ring.push((ts, app_id, is_in, pages));
        } else {
            self.event_ring[self.ring_head] = (ts, app_id, is_in, pages);
        }
        self.ring_head = (self.ring_head + 1) % self.ring_capacity;
    }

    pub fn profile(&self, app_id: u64) -> Option<&AppSwapProfile> { self.profiles.get(&app_id) }
    pub fn stats(&self) -> &SwapAppStats { &self.stats }
    pub fn global_compression_ratio(&self) -> f64 {
        if self.stats.total_compressed_bytes == 0 { return 1.0; }
        self.stats.total_original_bytes as f64 / self.stats.total_compressed_bytes as f64
    }
}
