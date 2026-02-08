// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Page Cache (holistic page cache analysis)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page cache holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticPageCacheMetric {
    HitRate,
    MissRate,
    EvictionRate,
    DirtyPageRatio,
    ReadaheadEfficiency,
    MemoryPressure,
    ThrashingIndex,
}

/// Page cache analysis sample
#[derive(Debug, Clone)]
pub struct HolisticPageCacheSample {
    pub metric: HolisticPageCacheMetric,
    pub value: u64,
    pub timestamp: u64,
}

/// Page cache health assessment
#[derive(Debug, Clone)]
pub struct HolisticPageCacheHealth {
    pub hit_rate_score: u64,
    pub eviction_pressure: u64,
    pub readahead_score: u64,
    pub thrashing_score: u64,
    pub overall: u64,
}

/// Stats for page cache analysis
#[derive(Debug, Clone)]
pub struct HolisticPageCacheAnalysisStats {
    pub samples: u64,
    pub analyses: u64,
    pub thrashing_alerts: u64,
    pub pressure_alerts: u64,
    pub resize_recommendations: u64,
}

/// Manager for page cache holistic analysis
pub struct HolisticPageCacheAnalyzer {
    samples: Vec<HolisticPageCacheSample>,
    health: HolisticPageCacheHealth,
    stats: HolisticPageCacheAnalysisStats,
    min_hit_rate: u64,
}

impl HolisticPageCacheAnalyzer {
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            health: HolisticPageCacheHealth {
                hit_rate_score: 100,
                eviction_pressure: 0,
                readahead_score: 50,
                thrashing_score: 0,
                overall: 100,
            },
            stats: HolisticPageCacheAnalysisStats {
                samples: 0,
                analyses: 0,
                thrashing_alerts: 0,
                pressure_alerts: 0,
                resize_recommendations: 0,
            },
            min_hit_rate: 80,
        }
    }

    pub fn record(&mut self, metric: HolisticPageCacheMetric, value: u64) {
        let sample = HolisticPageCacheSample {
            metric,
            value,
            timestamp: self.samples.len() as u64,
        };
        self.samples.push(sample);
        self.stats.samples += 1;
    }

    pub fn analyze(&mut self) -> &HolisticPageCacheHealth {
        self.stats.analyses += 1;
        let hits: Vec<&HolisticPageCacheSample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticPageCacheMetric::HitRate))
            .collect();
        if !hits.is_empty() {
            let avg: u64 = hits.iter().map(|s| s.value).sum::<u64>() / hits.len() as u64;
            self.health.hit_rate_score = avg;
            if avg < self.min_hit_rate {
                self.stats.resize_recommendations += 1;
            }
        }
        let thrashing: Vec<&HolisticPageCacheSample> = self.samples.iter()
            .filter(|s| matches!(s.metric, HolisticPageCacheMetric::ThrashingIndex))
            .collect();
        if !thrashing.is_empty() {
            let avg: u64 = thrashing.iter().map(|s| s.value).sum::<u64>() / thrashing.len() as u64;
            self.health.thrashing_score = avg.min(100);
            if avg > 30 {
                self.stats.thrashing_alerts += 1;
            }
        }
        self.health.overall = (self.health.hit_rate_score + (100 - self.health.thrashing_score)) / 2;
        &self.health
    }

    pub fn set_min_hit_rate(&mut self, rate: u64) {
        self.min_hit_rate = rate;
    }

    pub fn stats(&self) -> &HolisticPageCacheAnalysisStats {
        &self.stats
    }
}
