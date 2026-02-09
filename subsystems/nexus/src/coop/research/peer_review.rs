// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Peer Review — Cross-Validation of Cooperation Findings
//!
//! Implements a rigorous peer-review process where cooperation findings are
//! validated by other subsystems. When a new fairness algorithm or trust
//! model is proposed, multiple independent reviewers evaluate the claim.
//! Consensus is built through weighted voting where reviewer quality and
//! domain expertise are tracked over time. This prevents publication of
//! unreliable cooperation optimizations and ensures that only reproducible,
//! genuinely beneficial findings influence the cooperation protocols.
//!
//! The engine that keeps cooperation research honest.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_FINDINGS: usize = 256;
const MAX_REVIEWS_PER_FINDING: usize = 16;
const MAX_REVIEWERS: usize = 64;
const CONSENSUS_THRESHOLD: f32 = 0.66;
const STRONG_CONSENSUS: f32 = 0.85;
const MIN_REVIEWS_REQUIRED: usize = 3;
const REVIEWER_QUALITY_DECAY: f32 = 0.01;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const REVIEW_QUALITY_WEIGHT: f32 = 0.70;
const EXPERTISE_WEIGHT: f32 = 0.30;
const QUALITY_BONUS_AGREE: f32 = 0.05;
const QUALITY_PENALTY_OUTLIER: f32 = 0.03;
const MAX_REVIEW_HISTORY: usize = 512;

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
// PEER REVIEW TYPES
// ============================================================================

/// Domain of the cooperation finding being reviewed
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopFindingDomain {
    FairnessAlgorithm,
    TrustModel,
    ContentionResolution,
    ResourceSharing,
    NegotiationProtocol,
    AuctionMechanism,
    CoalitionStrategy,
}

/// Status of a finding in the review process
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewStatus {
    Submitted,
    UnderReview,
    Accepted,
    Rejected,
    RevisionRequired,
    Withdrawn,
}

/// Review verdict from a single reviewer
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewVerdict {
    Accept,
    WeakAccept,
    Neutral,
    WeakReject,
    Reject,
}

/// A reviewer (subsystem) in the peer network
#[derive(Debug, Clone)]
pub struct Reviewer {
    pub id: u64,
    pub name: String,
    pub quality_score: f32,
    pub reviews_completed: u64,
    pub correct_predictions: u64,
    pub expertise: LinearMap<f32, 64>,
    pub last_review_tick: u64,
}

/// A single review of a cooperation finding
#[derive(Debug, Clone)]
pub struct Review {
    pub id: u64,
    pub reviewer_id: u64,
    pub finding_id: u64,
    pub verdict: ReviewVerdict,
    pub confidence: f32,
    pub justification: String,
    pub tick: u64,
    pub quality_at_time: f32,
}

/// A cooperation finding submitted for peer review
#[derive(Debug, Clone)]
pub struct CoopFinding {
    pub id: u64,
    pub domain: CoopFindingDomain,
    pub claim: String,
    pub evidence_strength: f32,
    pub effect_size: f32,
    pub reviews: Vec<Review>,
    pub status: ReviewStatus,
    pub submitted_tick: u64,
    pub resolved_tick: u64,
    pub consensus_score: f32,
}

/// Consensus result for a finding
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    pub finding_id: u64,
    pub consensus_level: f32,
    pub weighted_score: f32,
    pub accept_count: u32,
    pub reject_count: u32,
    pub neutral_count: u32,
    pub strong_consensus: bool,
}

// ============================================================================
// PEER REVIEW STATS
// ============================================================================

/// Aggregate statistics for the peer review system
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct PeerReviewStats {
    pub total_submissions: u64,
    pub total_reviews: u64,
    pub accepted_findings: u64,
    pub rejected_findings: u64,
    pub avg_consensus_ema: f32,
    pub avg_review_quality_ema: f32,
    pub reviewer_count: u64,
    pub strong_consensus_rate: f32,
    pub pending_reviews: u64,
    pub revision_requests: u64,
}

// ============================================================================
// COOPERATION PEER REVIEW
// ============================================================================

/// Cross-subsystem peer review engine for cooperation findings
#[derive(Debug)]
pub struct CoopPeerReview {
    findings: VecDeque<CoopFinding>,
    reviewers: BTreeMap<u64, Reviewer>,
    review_history: VecDeque<Review>,
    domain_consensus: LinearMap<f32, 64>,
    rng_state: u64,
    tick: u64,
    stats: PeerReviewStats,
}

impl CoopPeerReview {
    /// Create a new peer review system with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            findings: VecDeque::new(),
            reviewers: BTreeMap::new(),
            review_history: VecDeque::new(),
            domain_consensus: LinearMap::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: PeerReviewStats::default(),
        }
    }

    /// Register a reviewer (subsystem) in the peer network
    pub fn register_reviewer(&mut self, name: String) -> u64 {
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let reviewer = Reviewer {
            id,
            name,
            quality_score: 0.5,
            reviews_completed: 0,
            correct_predictions: 0,
            expertise: LinearMap::new(),
            last_review_tick: 0,
        };
        self.reviewers.insert(id, reviewer);
        self.stats.reviewer_count = self.reviewers.len() as u64;
        id
    }

    /// Submit a cooperation finding for peer review
    pub fn submit_cooperation_finding(
        &mut self,
        domain: CoopFindingDomain,
        claim: String,
        evidence_strength: f32,
        effect_size: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(claim.as_bytes()) ^ fnv1a_hash(&self.tick.to_le_bytes());
        let finding = CoopFinding {
            id,
            domain,
            claim,
            evidence_strength: evidence_strength.min(1.0).max(0.0),
            effect_size,
            reviews: Vec::new(),
            status: ReviewStatus::Submitted,
            submitted_tick: self.tick,
            resolved_tick: 0,
            consensus_score: 0.0,
        };
        if self.findings.len() >= MAX_FINDINGS {
            self.findings.pop_front();
        }
        self.findings.push_back(finding);
        self.stats.total_submissions += 1;
        self.stats.pending_reviews += 1;
        id
    }

    /// Process a review from a cross-subsystem reviewer
    #[inline]
    pub fn cross_subsystem_review(
        &mut self,
        finding_id: u64,
        reviewer_id: u64,
        verdict: ReviewVerdict,
        confidence: f32,
        justification: String,
    ) -> bool {
        self.tick += 1;
        let reviewer_quality = self
            .reviewers
            .get(&reviewer_id)
            .map(|r| r.quality_score)
            .unwrap_or(0.5);
        let finding = match self.findings.iter_mut().find(|f| f.id == finding_id) {
            Some(f) => f,
            None => return false,
        };
        if finding.reviews.len() >= MAX_REVIEWS_PER_FINDING {
            return false;
        }
        let review_id = fnv1a_hash(&self.tick.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
        let review = Review {
            id: review_id,
            reviewer_id,
            finding_id,
            verdict,
            confidence: confidence.min(1.0).max(0.0),
            justification,
            tick: self.tick,
            quality_at_time: reviewer_quality,
        };
        finding.reviews.push(review.clone());
        if finding.status == ReviewStatus::Submitted {
            finding.status = ReviewStatus::UnderReview;
        }
        if self.review_history.len() >= MAX_REVIEW_HISTORY {
            self.review_history.pop_front();
        }
        self.review_history.push_back(review);
        self.stats.total_reviews += 1;
        // Update reviewer stats
        if let Some(reviewer) = self.reviewers.get_mut(&reviewer_id) {
            reviewer.reviews_completed += 1;
            reviewer.last_review_tick = self.tick;
            let domain_key = finding.domain as u64;
            let exp = reviewer.expertise.entry(domain_key).or_insert(0.0);
            *exp = EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * *exp;
        }
        // Auto-resolve if enough reviews
        if finding.reviews.len() >= MIN_REVIEWS_REQUIRED {
            self.try_resolve_finding(finding_id);
        }
        true
    }

    /// Compute the consensus level for a finding
    pub fn consensus_level(&self, finding_id: u64) -> Option<ConsensusResult> {
        let finding = self.findings.iter().find(|f| f.id == finding_id)?;
        if finding.reviews.is_empty() {
            return None;
        }
        let mut weighted_accept = 0.0f32;
        let mut weighted_reject = 0.0f32;
        let mut total_weight = 0.0f32;
        let mut accept_count = 0u32;
        let mut reject_count = 0u32;
        let mut neutral_count = 0u32;
        for review in &finding.reviews {
            let reviewer_quality = self
                .reviewers
                .get(&review.reviewer_id)
                .map(|r| r.quality_score)
                .unwrap_or(0.5);
            let domain_key = finding.domain as u64;
            let expertise = self
                .reviewers
                .get(&review.reviewer_id)
                .and_then(|r| r.expertise.get(&domain_key).copied())
                .unwrap_or(0.0);
            let weight = REVIEW_QUALITY_WEIGHT * reviewer_quality + EXPERTISE_WEIGHT * expertise;
            let weight = weight * review.confidence;
            match review.verdict {
                ReviewVerdict::Accept => {
                    weighted_accept += weight;
                    accept_count += 1;
                }
                ReviewVerdict::WeakAccept => {
                    weighted_accept += weight * 0.6;
                    accept_count += 1;
                }
                ReviewVerdict::Neutral => {
                    neutral_count += 1;
                }
                ReviewVerdict::WeakReject => {
                    weighted_reject += weight * 0.6;
                    reject_count += 1;
                }
                ReviewVerdict::Reject => {
                    weighted_reject += weight;
                    reject_count += 1;
                }
            }
            total_weight += weight;
        }
        let consensus = if total_weight > 0.0 {
            weighted_accept / total_weight
        } else {
            0.5
        };
        let strong = consensus >= STRONG_CONSENSUS || (1.0 - consensus) >= STRONG_CONSENSUS;
        Some(ConsensusResult {
            finding_id,
            consensus_level: consensus,
            weighted_score: weighted_accept - weighted_reject,
            accept_count,
            reject_count,
            neutral_count,
            strong_consensus: strong,
        })
    }

    /// Validate a fairness-specific finding
    pub fn fairness_validation(
        &mut self,
        finding_id: u64,
        fairness_before: f32,
        fairness_after: f32,
    ) -> bool {
        self.tick += 1;
        let finding = match self.findings.iter().find(|f| f.id == finding_id) {
            Some(f) => f,
            None => return false,
        };
        if finding.domain != CoopFindingDomain::FairnessAlgorithm {
            return false;
        }
        let improvement = fairness_after - fairness_before;
        let consistent = improvement > 0.0 && improvement >= finding.effect_size * 0.5;
        consistent
    }

    /// Get the quality score for a specific review
    pub fn review_quality(&self, review_id: u64) -> f32 {
        for review in &self.review_history {
            if review.id == review_id {
                let reviewer_quality = self
                    .reviewers
                    .get(&review.reviewer_id)
                    .map(|r| r.quality_score)
                    .unwrap_or(0.5);
                let confidence_factor = review.confidence;
                return reviewer_quality * confidence_factor;
            }
        }
        0.0
    }

    /// Get the peer network topology as a list of reviewers and their quality
    #[inline]
    pub fn peer_network(&self) -> Vec<(u64, f32, u64)> {
        self.reviewers
            .values()
            .map(|r| (r.id, r.quality_score, r.reviews_completed))
            .collect()
    }

    /// Get current peer review statistics
    #[inline(always)]
    pub fn stats(&self) -> &PeerReviewStats {
        &self.stats
    }

    /// Number of findings in the system
    #[inline(always)]
    pub fn finding_count(&self) -> usize {
        self.findings.len()
    }

    /// Number of pending findings awaiting review
    #[inline]
    pub fn pending_count(&self) -> usize {
        self.findings
            .iter()
            .filter(|f| matches!(f.status, ReviewStatus::Submitted | ReviewStatus::UnderReview))
            .count()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    #[inline]
    fn try_resolve_finding(&mut self, finding_id: u64) {
        let consensus = match self.consensus_level(finding_id) {
            Some(c) => c,
            None => return,
        };
        let finding = match self.findings.iter_mut().find(|f| f.id == finding_id) {
            Some(f) => f,
            None => return,
        };
        finding.consensus_score = consensus.consensus_level;
        if consensus.consensus_level >= CONSENSUS_THRESHOLD {
            finding.status = ReviewStatus::Accepted;
            finding.resolved_tick = self.tick;
            self.stats.accepted_findings += 1;
            if self.stats.pending_reviews > 0 {
                self.stats.pending_reviews -= 1;
            }
            self.update_reviewer_quality(finding_id, true);
        } else if (1.0 - consensus.consensus_level) >= CONSENSUS_THRESHOLD {
            finding.status = ReviewStatus::Rejected;
            finding.resolved_tick = self.tick;
            self.stats.rejected_findings += 1;
            if self.stats.pending_reviews > 0 {
                self.stats.pending_reviews -= 1;
            }
            self.update_reviewer_quality(finding_id, false);
        } else if finding.reviews.len() >= MAX_REVIEWS_PER_FINDING / 2 {
            finding.status = ReviewStatus::RevisionRequired;
            self.stats.revision_requests += 1;
        }
        let domain_key = finding.domain as u64;
        let prev = self.domain_consensus.get(domain_key).copied().unwrap_or(0.5);
        let new_ema = EMA_ALPHA * consensus.consensus_level + (1.0 - EMA_ALPHA) * prev;
        self.domain_consensus.insert(domain_key, new_ema);
        self.stats.avg_consensus_ema =
            EMA_ALPHA * consensus.consensus_level + (1.0 - EMA_ALPHA) * self.stats.avg_consensus_ema;
        let total_resolved = self.stats.accepted_findings + self.stats.rejected_findings;
        if total_resolved > 0 {
            let strong_count = self
                .findings
                .iter()
                .filter(|f| f.consensus_score >= STRONG_CONSENSUS || f.consensus_score <= (1.0 - STRONG_CONSENSUS))
                .count() as f32;
            self.stats.strong_consensus_rate = strong_count / total_resolved as f32;
        }
    }

    #[inline]
    fn update_reviewer_quality(&mut self, finding_id: u64, accepted: bool) {
        let finding = match self.findings.iter().find(|f| f.id == finding_id) {
            Some(f) => f,
            None => return,
        };
        let reviews = finding.reviews.clone();
        for review in &reviews {
            let agreed = match review.verdict {
                ReviewVerdict::Accept | ReviewVerdict::WeakAccept => accepted,
                ReviewVerdict::Reject | ReviewVerdict::WeakReject => !accepted,
                ReviewVerdict::Neutral => true,
            };
            if let Some(reviewer) = self.reviewers.get_mut(&review.reviewer_id) {
                if agreed {
                    reviewer.quality_score = (reviewer.quality_score + QUALITY_BONUS_AGREE).min(1.0);
                    reviewer.correct_predictions += 1;
                } else {
                    reviewer.quality_score = (reviewer.quality_score - QUALITY_PENALTY_OUTLIER).max(0.0);
                }
                self.stats.avg_review_quality_ema =
                    EMA_ALPHA * reviewer.quality_score
                        + (1.0 - EMA_ALPHA) * self.stats.avg_review_quality_ema;
            }
        }
    }
}
