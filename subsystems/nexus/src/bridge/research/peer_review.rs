// SPDX-License-Identifier: GPL-2.0
//! # Bridge Peer Review — Cross-Validation of Research Findings
//!
//! When the bridge discovers an optimization, it must be validated not just
//! internally but against observations from other subsystems (memory, IPC,
//! scheduler). This module implements a peer-review protocol: findings are
//! submitted, reviewers from other subsystems assess them, and consensus is
//! reached before a finding is accepted into the knowledge base.
//!
//! Trust is earned: reviewers build reputation over time, and findings
//! require a quorum of agreement before acceptance.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_FINDINGS: usize = 512;
const MAX_REVIEWERS: usize = 64;
const MAX_REVIEWS_PER_FINDING: usize = 16;
const CONSENSUS_THRESHOLD: f32 = 0.66;
const HIGH_CONFIDENCE_THRESHOLD: f32 = 0.85;
const REVIEWER_TRUST_INIT: f32 = 0.5;
const TRUST_GAIN: f32 = 0.05;
const TRUST_DECAY: f32 = 0.02;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const REPLICATION_REQUIRED: u32 = 2;
const REVIEW_QUALITY_WEIGHT: f32 = 0.7;
const MAX_REVIEW_HISTORY: usize = 2048;

// ============================================================================
// HELPERS
// ============================================================================

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

fn xorshift_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 10000) as f32 / 10000.0
}

// ============================================================================
// TYPES
// ============================================================================

/// A finding submitted for peer review.
#[derive(Clone)]
struct Finding {
    id: u64,
    description: String,
    effect_magnitude: f32,
    submitter: String,
    submit_tick: u64,
    status: FindingStatus,
    reviews: Vec<Review>,
    replication_count: u32,
    consensus_score: f32,
}

/// Status of a finding in the review pipeline.
#[derive(Clone, Copy, PartialEq)]
enum FindingStatus {
    Submitted,
    UnderReview,
    Accepted,
    Rejected,
    NeedsReplication,
}

/// A single review by a subsystem reviewer.
#[derive(Clone)]
struct Review {
    reviewer_id: u64,
    reviewer_subsystem: String,
    confirmed: bool,
    confidence: f32,
    comments: String,
    tick: u64,
    quality_score: f32,
}

/// A registered reviewer (subsystem).
#[derive(Clone)]
struct Reviewer {
    id: u64,
    subsystem: String,
    trust_score: f32,
    reviews_given: u64,
    correct_reviews: u64,
    last_review_tick: u64,
}

/// Public verdict on a finding.
#[derive(Clone)]
pub struct ReviewVerdict {
    pub finding_id: u64,
    pub reviewer_subsystem: String,
    pub confirmed: bool,
    pub confidence: f32,
    pub quality: f32,
}

/// Peer review statistics.
#[derive(Clone)]
pub struct PeerReviewStats {
    pub total_submissions: u64,
    pub accepted_findings: u64,
    pub rejected_findings: u64,
    pub pending_reviews: u64,
    pub avg_consensus_ema: f32,
    pub avg_review_quality_ema: f32,
    pub replication_requests: u64,
    pub active_reviewers: usize,
    pub avg_trust_score: f32,
    pub review_throughput_ema: f32,
}

/// Consensus result for a finding.
#[derive(Clone)]
pub struct ConsensusResult {
    pub finding_id: u64,
    pub consensus_reached: bool,
    pub agreement_ratio: f32,
    pub weighted_confidence: f32,
    pub reviewer_count: usize,
    pub status: String,
}

// ============================================================================
// BRIDGE PEER REVIEW
// ============================================================================

/// Cross-validation engine for bridge research findings.
pub struct BridgePeerReview {
    findings: BTreeMap<u64, Finding>,
    reviewers: BTreeMap<u64, Reviewer>,
    stats: PeerReviewStats,
    review_history: Vec<(u64, u64, bool)>, // (finding_id, reviewer_id, confirmed)
    rng_state: u64,
    tick: u64,
}

impl BridgePeerReview {
    /// Create a new peer review system.
    pub fn new(seed: u64) -> Self {
        Self {
            findings: BTreeMap::new(),
            reviewers: BTreeMap::new(),
            stats: PeerReviewStats {
                total_submissions: 0,
                accepted_findings: 0,
                rejected_findings: 0,
                pending_reviews: 0,
                avg_consensus_ema: 0.0,
                avg_review_quality_ema: 0.5,
                replication_requests: 0,
                active_reviewers: 0,
                avg_trust_score: REVIEWER_TRUST_INIT,
                review_throughput_ema: 0.0,
            },
            review_history: Vec::new(),
            rng_state: seed ^ 0xDE3BBE01E00001,
            tick: 0,
        }
    }

    /// Register a reviewer subsystem.
    pub fn register_reviewer(&mut self, subsystem: &str) {
        if self.reviewers.len() >= MAX_REVIEWERS {
            return;
        }
        let id = fnv1a_hash(subsystem.as_bytes());
        self.reviewers.insert(
            id,
            Reviewer {
                id,
                subsystem: String::from(subsystem),
                trust_score: REVIEWER_TRUST_INIT,
                reviews_given: 0,
                correct_reviews: 0,
                last_review_tick: 0,
            },
        );
        self.stats.active_reviewers = self.reviewers.len();
    }

    /// Submit a finding for peer review.
    pub fn submit_for_review(
        &mut self,
        description: &str,
        effect_magnitude: f32,
        submitter: &str,
    ) -> u64 {
        self.tick += 1;
        self.stats.total_submissions += 1;

        let id = fnv1a_hash(description.as_bytes()) ^ self.tick;
        if self.findings.len() >= MAX_FINDINGS {
            // Evict oldest submitted finding
            let oldest = self
                .findings
                .values()
                .filter(|f| f.status == FindingStatus::Submitted)
                .min_by_key(|f| f.submit_tick)
                .map(|f| f.id);
            if let Some(oid) = oldest {
                self.findings.remove(&oid);
            }
        }

        self.findings.insert(
            id,
            Finding {
                id,
                description: String::from(description),
                effect_magnitude,
                submitter: String::from(submitter),
                submit_tick: self.tick,
                status: FindingStatus::Submitted,
                reviews: Vec::new(),
                replication_count: 0,
                consensus_score: 0.0,
            },
        );
        self.stats.pending_reviews += 1;
        id
    }

    /// Submit a review for a finding.
    pub fn review_finding(
        &mut self,
        finding_id: u64,
        reviewer_subsystem: &str,
        confirmed: bool,
        confidence: f32,
        comments: &str,
    ) -> ReviewVerdict {
        self.tick += 1;
        let reviewer_id = fnv1a_hash(reviewer_subsystem.as_bytes());
        let clamped_conf = confidence.max(0.0).min(1.0);

        // Compute review quality based on reviewer trust and confidence calibration
        let reviewer_trust = self
            .reviewers
            .get(&reviewer_id)
            .map(|r| r.trust_score)
            .unwrap_or(0.3);
        let quality = reviewer_trust * REVIEW_QUALITY_WEIGHT
            + clamped_conf * (1.0 - REVIEW_QUALITY_WEIGHT);

        let review = Review {
            reviewer_id,
            reviewer_subsystem: String::from(reviewer_subsystem),
            confirmed,
            confidence: clamped_conf,
            comments: String::from(comments),
            tick: self.tick,
            quality_score: quality,
        };

        if let Some(finding) = self.findings.get_mut(&finding_id) {
            if finding.reviews.len() < MAX_REVIEWS_PER_FINDING {
                finding.reviews.push(review);
                finding.status = FindingStatus::UnderReview;
            }
            // Recompute consensus
            finding.consensus_score = self.compute_consensus_score(&finding.reviews);
        }

        // Update reviewer stats
        if let Some(reviewer) = self.reviewers.get_mut(&reviewer_id) {
            reviewer.reviews_given += 1;
            reviewer.last_review_tick = self.tick;
        }

        // Record history
        if self.review_history.len() < MAX_REVIEW_HISTORY {
            self.review_history
                .push((finding_id, reviewer_id, confirmed));
        }

        self.stats.avg_review_quality_ema =
            self.stats.avg_review_quality_ema * (1.0 - EMA_ALPHA) + quality * EMA_ALPHA;
        self.stats.review_throughput_ema =
            self.stats.review_throughput_ema * (1.0 - EMA_ALPHA) + 1.0 * EMA_ALPHA;

        ReviewVerdict {
            finding_id,
            reviewer_subsystem: String::from(reviewer_subsystem),
            confirmed,
            confidence: clamped_conf,
            quality,
        }
    }

    /// Check if consensus has been reached for a finding.
    pub fn consensus_check(&mut self, finding_id: u64) -> ConsensusResult {
        self.tick += 1;

        let (consensus_score, review_count, weighted_conf, status) =
            match self.findings.get(&finding_id) {
                Some(f) => {
                    let cs = f.consensus_score;
                    let rc = f.reviews.len();
                    let wc = self.weighted_confidence(&f.reviews);
                    let st = if cs >= CONSENSUS_THRESHOLD && rc >= 2 {
                        String::from("accepted")
                    } else if rc >= 3 && cs < (1.0 - CONSENSUS_THRESHOLD) {
                        String::from("rejected")
                    } else {
                        String::from("under_review")
                    };
                    (cs, rc, wc, st)
                }
                None => (0.0, 0, 0.0, String::from("not_found")),
            };

        let reached = consensus_score >= CONSENSUS_THRESHOLD && review_count >= 2;

        // Update finding status
        if let Some(finding) = self.findings.get_mut(&finding_id) {
            if reached {
                finding.status = FindingStatus::Accepted;
                self.stats.accepted_findings += 1;
                if self.stats.pending_reviews > 0 {
                    self.stats.pending_reviews -= 1;
                }
                // Reward agreeing reviewers
                self.update_reviewer_trust(finding_id, true);
            } else if review_count >= 3 && consensus_score < (1.0 - CONSENSUS_THRESHOLD) {
                finding.status = FindingStatus::Rejected;
                self.stats.rejected_findings += 1;
                if self.stats.pending_reviews > 0 {
                    self.stats.pending_reviews -= 1;
                }
                self.update_reviewer_trust(finding_id, false);
            }
        }

        self.stats.avg_consensus_ema =
            self.stats.avg_consensus_ema * (1.0 - EMA_ALPHA) + consensus_score * EMA_ALPHA;

        ConsensusResult {
            finding_id,
            consensus_reached: reached,
            agreement_ratio: consensus_score,
            weighted_confidence: weighted_conf,
            reviewer_count: review_count,
            status,
        }
    }

    /// Request replication for a finding that needs more evidence.
    pub fn replication_request(&mut self, finding_id: u64) -> bool {
        self.tick += 1;
        self.stats.replication_requests += 1;

        if let Some(finding) = self.findings.get_mut(&finding_id) {
            finding.status = FindingStatus::NeedsReplication;
            finding.replication_count += 1;
            true
        } else {
            false
        }
    }

    /// Compute review quality for a specific reviewer.
    pub fn review_quality(&self, reviewer_subsystem: &str) -> f32 {
        let rid = fnv1a_hash(reviewer_subsystem.as_bytes());
        match self.reviewers.get(&rid) {
            Some(r) => {
                let accuracy = if r.reviews_given > 0 {
                    r.correct_reviews as f32 / r.reviews_given as f32
                } else {
                    0.5
                };
                let experience = (r.reviews_given as f32 / 50.0).min(1.0);
                accuracy * 0.6 + r.trust_score * 0.25 + experience * 0.15
            }
            None => 0.0,
        }
    }

    /// Compute overall peer agreement metric.
    pub fn peer_agreement(&self) -> f32 {
        if self.findings.is_empty() {
            return 0.0;
        }
        let total: f32 = self.findings.values().map(|f| f.consensus_score).sum();
        total / self.findings.len() as f32
    }

    /// Get stats.
    pub fn stats(&self) -> &PeerReviewStats {
        &self.stats
    }

    /// Number of findings.
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    /// Number of accepted findings.
    pub fn accepted_count(&self) -> usize {
        self.findings
            .values()
            .filter(|f| f.status == FindingStatus::Accepted)
            .count()
    }

    /// Findings pending review.
    pub fn pending_count(&self) -> usize {
        self.findings
            .values()
            .filter(|f| {
                f.status == FindingStatus::Submitted || f.status == FindingStatus::UnderReview
            })
            .count()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn compute_consensus_score(&self, reviews: &[Review]) -> f32 {
        if reviews.is_empty() {
            return 0.0;
        }
        let mut weighted_agree: f32 = 0.0;
        let mut total_weight: f32 = 0.0;
        for r in reviews {
            let weight = r.confidence * r.quality_score;
            if r.confirmed {
                weighted_agree += weight;
            }
            total_weight += weight;
        }
        if total_weight > 1e-9 {
            weighted_agree / total_weight
        } else {
            0.0
        }
    }

    fn weighted_confidence(&self, reviews: &[Review]) -> f32 {
        if reviews.is_empty() {
            return 0.0;
        }
        let mut sum: f32 = 0.0;
        let mut weight_sum: f32 = 0.0;
        for r in reviews {
            let w = r.quality_score;
            sum += r.confidence * w;
            weight_sum += w;
        }
        if weight_sum > 1e-9 { sum / weight_sum } else { 0.0 }
    }

    fn update_reviewer_trust(&mut self, finding_id: u64, finding_accepted: bool) {
        let reviews: Vec<(u64, bool)> = match self.findings.get(&finding_id) {
            Some(f) => f.reviews.iter().map(|r| (r.reviewer_id, r.confirmed)).collect(),
            None => return,
        };
        for (rid, confirmed) in reviews {
            if let Some(reviewer) = self.reviewers.get_mut(&rid) {
                let was_correct = confirmed == finding_accepted;
                if was_correct {
                    reviewer.correct_reviews += 1;
                    reviewer.trust_score =
                        (reviewer.trust_score + TRUST_GAIN).min(1.0);
                } else {
                    reviewer.trust_score =
                        (reviewer.trust_score - TRUST_DECAY).max(0.1);
                }
            }
        }
        // Update average trust
        if !self.reviewers.is_empty() {
            let total_trust: f32 = self.reviewers.values().map(|r| r.trust_score).sum();
            self.stats.avg_trust_score = total_trust / self.reviewers.len() as f32;
        }
    }
}
