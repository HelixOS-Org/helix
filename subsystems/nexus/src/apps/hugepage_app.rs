// SPDX-License-Identifier: MIT
//! # Application Huge Page Manager
//!
//! Per-application huge page usage optimization:
//! - 2MB/1GB page promotion candidates
//! - Access density heatmap for promotion decisions
//! - TLB miss correlation with huge page coverage
//! - Transparent huge page (THP) tracking
//! - Fragmentation-aware promotion scheduling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Huge page size tier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HugePageTier {
    /// 2 MB (x86_64 PMD, AArch64 L2 block)
    Large,
    /// 1 GB (x86_64 PUD, AArch64 L1 block)
    Huge,
}

impl HugePageTier {
    #[inline]
    pub fn size_bytes(&self) -> u64 {
        match self {
            Self::Large => 2 * 1024 * 1024,
            Self::Huge => 1024 * 1024 * 1024,
        }
    }

    #[inline(always)]
    pub fn pages_4k(&self) -> u64 {
        self.size_bytes() / 4096
    }
}

/// Promotion candidate — a contiguous region worth promoting
#[derive(Debug, Clone)]
pub struct PromotionCandidate {
    pub app_id: u64,
    pub base_vpn: u64,
    pub tier: HugePageTier,
    /// Fraction of sub-pages accessed in the last epoch (0.0–1.0)
    pub access_density: f64,
    /// Estimated TLB misses saved per second if promoted
    pub tlb_savings_per_sec: u64,
    /// Score: access_density × tlb_savings (higher = better candidate)
    pub score: f64,
    /// Is the backing physical memory contiguous and aligned?
    pub physically_eligible: bool,
}

/// THP (Transparent Huge Page) state for a region
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThpState {
    /// Not considered for THP
    None,
    /// Eligible, waiting for access density threshold
    Monitoring,
    /// Promotion in progress (compaction/migration)
    Promoting,
    /// Successfully promoted to huge page
    Promoted,
    /// Demoted back to 4K pages (e.g., after partial unmap)
    Demoted,
}

/// Per-region huge page tracking
#[derive(Debug, Clone)]
pub struct HugePageRegion {
    pub base_vpn: u64,
    pub tier: HugePageTier,
    pub state: ThpState,
    pub access_count: u64,
    pub subpage_touches: u64,
    pub promoted_at: Option<u64>,
    pub demoted_at: Option<u64>,
    pub promotion_attempts: u64,
    pub promotion_failures: u64,
}

impl HugePageRegion {
    pub fn new(base_vpn: u64, tier: HugePageTier) -> Self {
        Self {
            base_vpn,
            tier,
            state: ThpState::Monitoring,
            access_count: 0,
            subpage_touches: 0,
            promoted_at: None,
            demoted_at: None,
            promotion_attempts: 0,
            promotion_failures: 0,
        }
    }

    #[inline]
    pub fn record_access(&mut self, subpage_offset: u64) {
        self.access_count += 1;
        if subpage_offset < self.tier.pages_4k() {
            self.subpage_touches += 1;
        }
    }

    #[inline]
    pub fn access_density(&self) -> f64 {
        let total = self.tier.pages_4k();
        if total == 0 {
            return 0.0;
        }
        let touched = self.subpage_touches.min(total);
        touched as f64 / total as f64
    }

    #[inline(always)]
    pub fn mark_promoted(&mut self, now: u64) {
        self.state = ThpState::Promoted;
        self.promoted_at = Some(now);
    }

    #[inline(always)]
    pub fn mark_demoted(&mut self, now: u64) {
        self.state = ThpState::Demoted;
        self.demoted_at = Some(now);
    }

    #[inline(always)]
    pub fn mark_failed(&mut self) {
        self.promotion_attempts += 1;
        self.promotion_failures += 1;
    }
}

/// Huge page stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HugePageAppStats {
    pub candidates_evaluated: u64,
    pub promotions_attempted: u64,
    pub promotions_succeeded: u64,
    pub promotions_failed: u64,
    pub demotions: u64,
    pub estimated_tlb_misses_saved: u64,
}

/// Per-application huge page manager
pub struct HugePageAppManager {
    /// app_id → (base_vpn → region)
    app_regions: BTreeMap<u64, BTreeMap<u64, HugePageRegion>>,
    stats: HugePageAppStats,
    /// Minimum access density to trigger promotion (default 0.7)
    promotion_threshold: f64,
    /// Maximum concurrent promotions
    max_concurrent_promotions: u64,
    /// Current in-flight promotions
    active_promotions: u64,
}

impl HugePageAppManager {
    pub fn new() -> Self {
        Self {
            app_regions: BTreeMap::new(),
            stats: HugePageAppStats::default(),
            promotion_threshold: 0.7,
            max_concurrent_promotions: 4,
            active_promotions: 0,
        }
    }

    /// Register a huge-page-eligible region for monitoring
    #[inline]
    pub fn monitor_region(&mut self, app_id: u64, base_vpn: u64, tier: HugePageTier) {
        let regions = self.app_regions.entry(app_id).or_insert_with(BTreeMap::new);
        regions
            .entry(base_vpn)
            .or_insert_with(|| HugePageRegion::new(base_vpn, tier));
    }

    /// Record a page access within a monitored region
    pub fn record_access(&mut self, app_id: u64, vpn: u64) {
        if let Some(regions) = self.app_regions.get_mut(&app_id) {
            // Find the containing region (round down to tier alignment)
            for (_, region) in regions.iter_mut() {
                let size = region.tier.pages_4k();
                if vpn >= region.base_vpn && vpn < region.base_vpn + size {
                    region.record_access(vpn - region.base_vpn);
                    return;
                }
            }
        }
    }

    /// Find the best promotion candidates across all apps
    pub fn find_candidates(&mut self, max: usize) -> Vec<PromotionCandidate> {
        let mut candidates = Vec::new();

        for (&app_id, regions) in self.app_regions.iter() {
            for (_, region) in regions.iter() {
                if region.state != ThpState::Monitoring {
                    continue;
                }
                let density = region.access_density();
                if density < self.promotion_threshold {
                    continue;
                }

                let tlb_savings = (density * region.tier.pages_4k() as f64 * 100.0) as u64;
                let score = density * tlb_savings as f64;

                candidates.push(PromotionCandidate {
                    app_id,
                    base_vpn: region.base_vpn,
                    tier: region.tier,
                    access_density: density,
                    tlb_savings_per_sec: tlb_savings,
                    score,
                    physically_eligible: true, // Kernel memory manager decides this
                });
            }
        }

        candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal));
        candidates.truncate(max);
        self.stats.candidates_evaluated += candidates.len() as u64;
        candidates
    }

    /// Mark a region as successfully promoted
    pub fn promote(&mut self, app_id: u64, base_vpn: u64, now: u64) -> bool {
        if let Some(regions) = self.app_regions.get_mut(&app_id) {
            if let Some(region) = regions.get_mut(&base_vpn) {
                region.mark_promoted(now);
                self.stats.promotions_attempted += 1;
                self.stats.promotions_succeeded += 1;
                self.stats.estimated_tlb_misses_saved += region.tier.pages_4k() * 100;
                return true;
            }
        }
        false
    }

    /// Demote a huge page region back to 4K pages
    #[inline]
    pub fn demote(&mut self, app_id: u64, base_vpn: u64, now: u64) {
        if let Some(regions) = self.app_regions.get_mut(&app_id) {
            if let Some(region) = regions.get_mut(&base_vpn) {
                region.mark_demoted(now);
                self.stats.demotions += 1;
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &HugePageAppStats {
        &self.stats
    }

    #[inline(always)]
    pub fn set_threshold(&mut self, threshold: f64) {
        self.promotion_threshold = threshold.clamp(0.1, 1.0);
    }
}
