// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Research Journal — Protocol Experiment Archive
//!
//! Archives all cooperation protocol experiments, discoveries, and validated
//! results. Every protocol test, fairness measurement, and negotiation
//! improvement is recorded with full metadata, provenance tracking, and
//! impact metrics. The journal supports searching by protocol type, time
//! range, and impact score. A discovery timeline provides a chronological
//! view of the cooperation research frontier's evolution.
//!
//! The engine that remembers every cooperation experiment ever run.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ENTRIES: usize = 2048;
const MAX_FINDINGS: usize = 512;
const MAX_TIMELINE_EVENTS: usize = 1024;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const IMPACT_DECAY_RATE: f32 = 0.002;
const HIGH_IMPACT_THRESHOLD: f32 = 0.75;
const CITATION_BOOST: f32 = 0.05;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// JOURNAL TYPES
// ============================================================================

/// Category of a protocol test entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TestCategory {
    NegotiationAlgorithm,
    FairnessMetric,
    TrustModel,
    AuctionMechanism,
    ConflictResolution,
    ResourceSharing,
    BackoffStrategy,
}

/// Status of a journal finding
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingStatus {
    Draft,
    Published,
    Superseded,
    Retracted,
}

/// A protocol test record in the journal
#[derive(Debug, Clone)]
pub struct ProtocolTestEntry {
    pub id: u64,
    pub protocol_name: String,
    pub category: TestCategory,
    pub tick: u64,
    pub control_metric: f32,
    pub treatment_metric: f32,
    pub effect_size: f32,
    pub significant: bool,
    pub fairness_score: f32,
    pub notes: String,
    pub experiment_id: u64,
}

/// A published finding from cooperation research
#[derive(Debug, Clone)]
pub struct Finding {
    pub id: u64,
    pub title: String,
    pub category: TestCategory,
    pub status: FindingStatus,
    pub tick: u64,
    pub impact_score: f32,
    pub confidence: f32,
    pub supporting_tests: Vec<u64>,
    pub citation_count: u32,
    pub summary: String,
}

/// A timeline event marking a discovery
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub tick: u64,
    pub finding_id: u64,
    pub category: TestCategory,
    pub impact: f32,
    pub description: String,
}

// ============================================================================
// JOURNAL STATS
// ============================================================================

/// Aggregate journal statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct JournalStats {
    pub total_entries: u64,
    pub total_findings: u64,
    pub published_findings: u64,
    pub superseded_findings: u64,
    pub retracted_findings: u64,
    pub avg_impact_ema: f32,
    pub avg_effect_size_ema: f32,
    pub high_impact_count: u64,
    pub total_citations: u64,
    pub timeline_events: u64,
}

// ============================================================================
// COOPERATION JOURNAL
// ============================================================================

/// Research journal for cooperation protocol experiments
#[derive(Debug)]
pub struct CoopJournal {
    entries: BTreeMap<u64, ProtocolTestEntry>,
    findings: BTreeMap<u64, Finding>,
    timeline: Vec<TimelineEvent>,
    category_counts: BTreeMap<u64, u32>,
    tick: u64,
    rng_state: u64,
    stats: JournalStats,
}

impl CoopJournal {
    /// Create a new cooperation research journal
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            findings: BTreeMap::new(),
            timeline: Vec::new(),
            category_counts: BTreeMap::new(),
            tick: 0,
            rng_state: seed | 1,
            stats: JournalStats::default(),
        }
    }

    /// Record a protocol test result in the journal
    pub fn record_protocol_test(
        &mut self,
        protocol_name: String,
        category: TestCategory,
        control_metric: f32,
        treatment_metric: f32,
        effect_size: f32,
        significant: bool,
        fairness_score: f32,
        experiment_id: u64,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(protocol_name.as_bytes())
            ^ fnv1a_hash(&self.tick.to_le_bytes())
            ^ xorshift64(&mut self.rng_state);

        let entry = ProtocolTestEntry {
            id,
            protocol_name,
            category,
            tick: self.tick,
            control_metric,
            treatment_metric,
            effect_size,
            significant,
            fairness_score,
            notes: String::new(),
            experiment_id,
        };

        if self.entries.len() < MAX_ENTRIES {
            self.entries.insert(id, entry);
            self.stats.total_entries += 1;
        } else {
            // Evict oldest entry
            if let Some(&oldest) = self.entries.keys().next() {
                self.entries.remove(&oldest);
                self.entries.insert(id, entry);
            }
        }

        let cat_key = category as u64;
        let count = self.category_counts.entry(cat_key).or_insert(0);
        *count += 1;

        let abs_effect = if effect_size < 0.0 {
            -effect_size
        } else {
            effect_size
        };
        self.stats.avg_effect_size_ema =
            EMA_ALPHA * abs_effect + (1.0 - EMA_ALPHA) * self.stats.avg_effect_size_ema;

        id
    }

    /// Publish a finding from accumulated test results
    pub fn publish_finding(
        &mut self,
        title: String,
        category: TestCategory,
        confidence: f32,
        supporting_test_ids: Vec<u64>,
        summary: String,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(title.as_bytes()) ^ xorshift64(&mut self.rng_state);

        // Compute impact from supporting test effect sizes
        let mut impact_sum: f32 = 0.0;
        let mut impact_count: u32 = 0;
        for &test_id in &supporting_test_ids {
            if let Some(entry) = self.entries.get(&test_id) {
                let abs_eff = if entry.effect_size < 0.0 {
                    -entry.effect_size
                } else {
                    entry.effect_size
                };
                impact_sum += abs_eff * entry.fairness_score;
                impact_count += 1;
            }
        }
        let impact_score = if impact_count > 0 {
            (impact_sum / impact_count as f32).clamp(0.0, 1.0)
        } else {
            confidence * 0.5
        };

        // Check for superseding existing findings on same topic
        let title_hash = fnv1a_hash(title.as_bytes());
        for existing in self.findings.values_mut() {
            if existing.status == FindingStatus::Published {
                let existing_hash = fnv1a_hash(existing.title.as_bytes());
                let xor_dist = (title_hash ^ existing_hash).count_ones();
                if xor_dist < 12 && impact_score > existing.impact_score {
                    existing.status = FindingStatus::Superseded;
                    self.stats.superseded_findings += 1;
                }
            }
        }

        let finding = Finding {
            id,
            title,
            category,
            status: FindingStatus::Published,
            tick: self.tick,
            impact_score,
            confidence,
            supporting_tests: supporting_test_ids,
            citation_count: 0,
            summary,
        };

        if self.findings.len() < MAX_FINDINGS {
            self.findings.insert(id, finding);
        }
        self.stats.total_findings += 1;
        self.stats.published_findings += 1;
        self.stats.avg_impact_ema =
            EMA_ALPHA * impact_score + (1.0 - EMA_ALPHA) * self.stats.avg_impact_ema;
        if impact_score >= HIGH_IMPACT_THRESHOLD {
            self.stats.high_impact_count += 1;
        }

        // Add timeline event
        if self.timeline.len() < MAX_TIMELINE_EVENTS {
            self.timeline.push(TimelineEvent {
                tick: self.tick,
                finding_id: id,
                category,
                impact: impact_score,
                description: String::from("New cooperation finding published"),
            });
            self.stats.timeline_events = self.timeline.len() as u64;
        }

        id
    }

    /// Retrieve the test history for a specific protocol category
    pub fn protocol_history(&self, category: TestCategory) -> Vec<&ProtocolTestEntry> {
        self.entries
            .values()
            .filter(|e| e.category == category)
            .collect()
    }

    /// Compute the impact metric for a finding, including citation boost
    pub fn impact_metric(&mut self, finding_id: u64) -> Option<f32> {
        let finding = self.findings.get_mut(&finding_id)?;
        let age_ticks = if self.tick > finding.tick {
            self.tick - finding.tick
        } else {
            0
        };
        let decay = IMPACT_DECAY_RATE * age_ticks as f32;
        let citation_boost = CITATION_BOOST * finding.citation_count as f32;
        let adjusted_impact = (finding.impact_score - decay + citation_boost).clamp(0.0, 1.0);
        finding.impact_score = adjusted_impact;
        Some(adjusted_impact)
    }

    /// Cite a finding — increases its impact over time
    pub fn cite_finding(&mut self, finding_id: u64) -> bool {
        if let Some(f) = self.findings.get_mut(&finding_id) {
            f.citation_count += 1;
            self.stats.total_citations += 1;
            true
        } else {
            false
        }
    }

    /// Get the full discovery timeline
    pub fn discovery_timeline(&self) -> &[TimelineEvent] {
        &self.timeline
    }

    /// Get timeline events within a tick range
    pub fn discovery_timeline_range(&self, start_tick: u64, end_tick: u64) -> Vec<&TimelineEvent> {
        self.timeline
            .iter()
            .filter(|e| e.tick >= start_tick && e.tick <= end_tick)
            .collect()
    }

    /// Get current journal statistics
    pub fn stats(&self) -> &JournalStats {
        &self.stats
    }
}
