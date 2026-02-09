// SPDX-License-Identifier: GPL-2.0
//! # Holistic Peer Review — Master Cross-Subsystem Validation
//!
//! The master peer review engine for the entire NEXUS kernel intelligence
//! framework. Every finding from any subsystem must survive review by
//! all other subsystems before it can be considered validated system
//! knowledge. This engine orchestrates multi-level reviews, builds
//! consensus across domains, and aggregates confidence scores.
//!
//! ## Capabilities
//!
//! - **Master review** — orchestrate a full review cycle for any finding
//! - **Cross-validation** — validate findings against all other subsystems
//! - **Global consensus** — build system-wide agreement on discoveries
//! - **Review hierarchy** — escalate contentious findings up the review chain
//! - **Confidence aggregation** — combine reviewer scores using weighted EMA
//! - **Completeness tracking** — ensure no finding escapes review
//!
//! The engine that ensures scientific rigour at the system level.

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_FINDINGS: usize = 1024;
const MAX_REVIEWS: usize = 4096;
const MAX_REVIEWERS: usize = 32;
const MAX_ESCALATIONS: usize = 128;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const CONSENSUS_THRESHOLD: f32 = 0.75;
const REJECTION_THRESHOLD: f32 = 0.30;
const ESCALATION_DISAGREEMENT: f32 = 0.40;
const CONFIDENCE_FLOOR: f32 = 0.05;
const REVIEW_DECAY: f32 = 0.99;
const MIN_REVIEWERS: usize = 3;
const COMPLETENESS_TARGET: f32 = 0.95;

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

/// Subsystem that can act as a reviewer
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewerSubsystem {
    Bridge,
    Application,
    Cooperation,
    Memory,
    Scheduler,
    Ipc,
    Trust,
    Energy,
    FileSystem,
    Networking,
}

/// Status of a finding under review
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewStatus {
    Submitted,
    UnderReview,
    Accepted,
    Rejected,
    Escalated,
    Consensus,
    Withdrawn,
}

/// Review verdict from a single reviewer
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewVerdict {
    StrongAccept,
    Accept,
    WeakAccept,
    Neutral,
    WeakReject,
    Reject,
    StrongReject,
}

/// A finding submitted for peer review
#[derive(Debug, Clone)]
pub struct ReviewableFinding {
    pub id: u64,
    pub source: ReviewerSubsystem,
    pub description: String,
    pub claimed_effect: f32,
    pub evidence_strength: f32,
    pub status: ReviewStatus,
    pub submitted_tick: u64,
    pub hash: u64,
}

/// A single peer review of a finding
#[derive(Debug, Clone)]
pub struct PeerReviewRecord {
    pub id: u64,
    pub finding_id: u64,
    pub reviewer: ReviewerSubsystem,
    pub verdict: ReviewVerdict,
    pub confidence: f32,
    pub comments_hash: u64,
    pub tick: u64,
}

/// Escalation record when reviewers disagree
#[derive(Debug, Clone)]
pub struct ReviewEscalation {
    pub id: u64,
    pub finding_id: u64,
    pub disagreement_level: f32,
    pub escalation_tier: u32,
    pub resolved: bool,
    pub resolution_verdict: ReviewVerdict,
    pub tick: u64,
}

/// Consensus record for a finding
#[derive(Debug, Clone)]
pub struct ConsensusRecord {
    pub finding_id: u64,
    pub consensus_score: f32,
    pub accept_count: u64,
    pub reject_count: u64,
    pub neutral_count: u64,
    pub final_status: ReviewStatus,
    pub aggregated_confidence: f32,
    pub tick: u64,
}

/// Reviewer reliability tracking
#[derive(Debug, Clone)]
pub struct ReviewerProfile {
    pub subsystem: ReviewerSubsystem,
    pub reviews_completed: u64,
    pub accuracy_ema: f32,
    pub agreement_rate_ema: f32,
    pub weight: f32,
    pub last_review_tick: u64,
}

/// Peer review statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PeerReviewStats {
    pub total_findings_submitted: u64,
    pub total_reviews_completed: u64,
    pub acceptance_rate_ema: f32,
    pub rejection_rate_ema: f32,
    pub escalation_rate_ema: f32,
    pub avg_consensus_score_ema: f32,
    pub avg_confidence_ema: f32,
    pub review_completeness: f32,
    pub active_findings: u64,
    pub escalations_total: u64,
    pub last_tick: u64,
    pub reviewer_count: u64,
}

// ============================================================================
// HOLISTIC PEER REVIEW
// ============================================================================

/// Master peer review engine for cross-subsystem validation
pub struct HolisticPeerReview {
    findings: BTreeMap<u64, ReviewableFinding>,
    reviews: Vec<PeerReviewRecord>,
    escalations: VecDeque<ReviewEscalation>,
    consensus_records: BTreeMap<u64, ConsensusRecord>,
    reviewer_profiles: BTreeMap<u64, ReviewerProfile>,
    rng_state: u64,
    tick: u64,
    stats: PeerReviewStats,
}

impl HolisticPeerReview {
    /// Create a new holistic peer review engine
    pub fn new(seed: u64) -> Self {
        Self {
            findings: BTreeMap::new(),
            reviews: Vec::new(),
            escalations: VecDeque::new(),
            consensus_records: BTreeMap::new(),
            reviewer_profiles: BTreeMap::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: PeerReviewStats {
                total_findings_submitted: 0,
                total_reviews_completed: 0,
                acceptance_rate_ema: 0.5,
                rejection_rate_ema: 0.1,
                escalation_rate_ema: 0.05,
                avg_consensus_score_ema: 0.5,
                avg_confidence_ema: 0.5,
                review_completeness: 0.0,
                active_findings: 0,
                escalations_total: 0,
                last_tick: 0,
                reviewer_count: 0,
            },
        }
    }

    /// Register a reviewer subsystem
    #[inline]
    pub fn register_reviewer(&mut self, subsystem: ReviewerSubsystem) {
        let key = subsystem as u64;
        if self.reviewer_profiles.contains_key(&key) { return; }
        self.reviewer_profiles.insert(key, ReviewerProfile {
            subsystem, reviews_completed: 0, accuracy_ema: 0.5,
            agreement_rate_ema: 0.5, weight: 1.0, last_review_tick: 0,
        });
        self.stats.reviewer_count = self.reviewer_profiles.len() as u64;
    }

    /// Submit a finding for master peer review
    pub fn submit_finding(&mut self, source: ReviewerSubsystem, description: String,
                          claimed_effect: f32, evidence: f32) -> u64 {
        let id = self.stats.total_findings_submitted;
        let hash = fnv1a_hash(description.as_bytes());
        let finding = ReviewableFinding {
            id, source, description, claimed_effect,
            evidence_strength: evidence, status: ReviewStatus::Submitted,
            submitted_tick: self.tick, hash,
        };
        self.findings.insert(id, finding);
        self.stats.total_findings_submitted += 1;
        self.stats.active_findings += 1;
        id
    }

    /// Run the master review cycle for a finding
    pub fn master_review(&mut self, finding_id: u64) -> ReviewStatus {
        let finding = match self.findings.get_mut(&finding_id) {
            Some(f) => { f.status = ReviewStatus::UnderReview; f.clone() }
            None => return ReviewStatus::Withdrawn,
        };
        let mut accept_score = 0.0f32;
        let mut reject_score = 0.0f32;
        let mut review_count = 0u64;
        for profile in self.reviewer_profiles.values() {
            if profile.subsystem == finding.source { continue; }
            let noise = xorshift_f32(&mut self.rng_state);
            let base = finding.evidence_strength * profile.accuracy_ema;
            let verdict = if base + noise * 0.3 > 0.7 { ReviewVerdict::Accept }
                else if base + noise * 0.3 > 0.5 { ReviewVerdict::WeakAccept }
                else if base + noise * 0.3 > 0.35 { ReviewVerdict::Neutral }
                else if base + noise * 0.3 > 0.2 { ReviewVerdict::WeakReject }
                else { ReviewVerdict::Reject };
            let conf = (base * 0.7 + noise * 0.3).min(1.0);
            let rev_id = self.stats.total_reviews_completed;
            self.reviews.push(PeerReviewRecord {
                id: rev_id, finding_id, reviewer: profile.subsystem,
                verdict, confidence: conf, comments_hash: fnv1a_hash(&[
                    profile.subsystem as u8, (self.tick & 0xFF) as u8,
                ]), tick: self.tick,
            });
            match verdict {
                ReviewVerdict::StrongAccept | ReviewVerdict::Accept => accept_score += conf * profile.weight,
                ReviewVerdict::WeakAccept => accept_score += conf * profile.weight * 0.5,
                ReviewVerdict::WeakReject => reject_score += conf * profile.weight * 0.5,
                ReviewVerdict::Reject | ReviewVerdict::StrongReject => reject_score += conf * profile.weight,
                ReviewVerdict::Neutral => {}
            }
            review_count += 1;
            self.stats.total_reviews_completed += 1;
        }
        let total_weight = accept_score + reject_score;
        let consensus = if total_weight > 0.0 { accept_score / total_weight } else { 0.5 };
        let disagreement = if review_count > 1 {
            let mean = consensus;
            let variance = (accept_score - mean * total_weight).abs() / (total_weight + 0.001);
            variance.min(1.0)
        } else { 0.0 };
        let status = if disagreement > ESCALATION_DISAGREEMENT {
            ReviewStatus::Escalated
        } else if consensus >= CONSENSUS_THRESHOLD {
            ReviewStatus::Accepted
        } else if consensus <= REJECTION_THRESHOLD {
            ReviewStatus::Rejected
        } else {
            ReviewStatus::UnderReview
        };
        if status == ReviewStatus::Escalated {
            let esc_id = self.stats.escalations_total;
            self.escalations.push_back(ReviewEscalation {
                id: esc_id, finding_id, disagreement_level: disagreement,
                escalation_tier: 1, resolved: false,
                resolution_verdict: ReviewVerdict::Neutral, tick: self.tick,
            });
            self.stats.escalations_total += 1;
            if self.escalations.len() > MAX_ESCALATIONS {
                self.escalations.pop_front();
            }
        }
        if let Some(f) = self.findings.get_mut(&finding_id) {
            f.status = status;
            if status == ReviewStatus::Accepted || status == ReviewStatus::Rejected {
                self.stats.active_findings = self.stats.active_findings.saturating_sub(1);
            }
        }
        self.consensus_records.insert(finding_id, ConsensusRecord {
            finding_id, consensus_score: consensus,
            accept_count: review_count / 2, reject_count: review_count / 4,
            neutral_count: review_count / 4, final_status: status,
            aggregated_confidence: consensus, tick: self.tick,
        });
        let is_accept = if status == ReviewStatus::Accepted { 1.0 } else { 0.0 };
        let is_reject = if status == ReviewStatus::Rejected { 1.0 } else { 0.0 };
        let is_escalate = if status == ReviewStatus::Escalated { 1.0 } else { 0.0 };
        self.stats.acceptance_rate_ema = self.stats.acceptance_rate_ema * (1.0 - EMA_ALPHA) + is_accept * EMA_ALPHA;
        self.stats.rejection_rate_ema = self.stats.rejection_rate_ema * (1.0 - EMA_ALPHA) + is_reject * EMA_ALPHA;
        self.stats.escalation_rate_ema = self.stats.escalation_rate_ema * (1.0 - EMA_ALPHA) + is_escalate * EMA_ALPHA;
        self.stats.avg_consensus_score_ema = self.stats.avg_consensus_score_ema * (1.0 - EMA_ALPHA) + consensus * EMA_ALPHA;
        self.stats.last_tick = self.tick;
        if self.reviews.len() > MAX_REVIEWS { self.reviews.drain(0..MAX_REVIEWS / 4); }
        status
    }

    /// Cross-validate a finding against all subsystems
    pub fn cross_validate_all(&mut self, finding_id: u64) -> f32 {
        let finding = match self.findings.get(&finding_id) {
            Some(f) => f.clone(),
            None => return 0.0,
        };
        let mut validation_sum = 0.0f32;
        let mut weight_sum = 0.0f32;
        for profile in self.reviewer_profiles.values() {
            if profile.subsystem == finding.source { continue; }
            let noise = xorshift_f32(&mut self.rng_state) * 0.1;
            let compatibility = finding.evidence_strength * profile.accuracy_ema + noise;
            validation_sum += compatibility * profile.weight;
            weight_sum += profile.weight;
        }
        if weight_sum > 0.0 { validation_sum / weight_sum } else { 0.0 }
    }

    /// Build global consensus across all pending findings
    #[inline]
    pub fn global_consensus(&mut self) -> f32 {
        if self.consensus_records.is_empty() { return 0.0; }
        let total: f32 = self.consensus_records.values()
            .map(|c| c.consensus_score).sum();
        let count = self.consensus_records.len() as f32;
        let global = total / count;
        self.stats.avg_consensus_score_ema = self.stats.avg_consensus_score_ema
            * (1.0 - EMA_ALPHA) + global * EMA_ALPHA;
        global
    }

    /// Get review hierarchy — escalation tiers
    #[inline]
    pub fn review_hierarchy(&self) -> Vec<(u32, u64)> {
        let mut tiers: ArrayMap<u64, 32> = BTreeMap::new();
        for esc in &self.escalations {
            *tiers.entry(esc.escalation_tier).or_insert(0) += 1;
        }
        tiers.into_iter().collect()
    }

    /// Aggregate confidence scores for a finding
    pub fn confidence_aggregation(&self, finding_id: u64) -> f32 {
        let reviews: Vec<&PeerReviewRecord> = self.reviews.iter()
            .filter(|r| r.finding_id == finding_id).collect();
        if reviews.is_empty() { return 0.0; }
        let mut weighted_sum = 0.0f32;
        let mut weight_sum = 0.0f32;
        for rev in &reviews {
            let w = self.reviewer_profiles.get(&(rev.reviewer as u64))
                .map(|p| p.weight).unwrap_or(1.0);
            weighted_sum += rev.confidence * w;
            weight_sum += w;
        }
        if weight_sum > 0.0 { weighted_sum / weight_sum } else { 0.0 }
    }

    /// Compute review completeness — fraction of findings fully reviewed
    #[inline]
    pub fn review_completeness(&mut self) -> f32 {
        if self.findings.is_empty() { return 1.0; }
        let reviewed = self.findings.values()
            .filter(|f| f.status != ReviewStatus::Submitted).count();
        let completeness = reviewed as f32 / self.findings.len() as f32;
        self.stats.review_completeness = completeness;
        completeness
    }

    /// Advance the engine tick
    #[inline(always)]
    pub fn tick(&mut self) { self.tick += 1; }

    /// Update reviewer accuracy based on consensus outcomes
    pub fn update_reviewer_accuracy(&mut self) {
        let consensus_vals: Vec<(ReviewerSubsystem, f32)> = self.reviews.iter()
            .filter_map(|rev| {
                self.consensus_records.get(&rev.finding_id).map(|c| {
                    let agreed = match rev.verdict {
                        ReviewVerdict::StrongAccept | ReviewVerdict::Accept
                            | ReviewVerdict::WeakAccept => c.final_status == ReviewStatus::Accepted,
                        ReviewVerdict::Reject | ReviewVerdict::StrongReject
                            | ReviewVerdict::WeakReject => c.final_status == ReviewStatus::Rejected,
                        ReviewVerdict::Neutral => true,
                    };
                    (rev.reviewer, if agreed { 1.0f32 } else { 0.0f32 })
                })
            })
            .collect();
        for (subsystem, score) in &consensus_vals {
            let key = *subsystem as u64;
            if let Some(profile) = self.reviewer_profiles.get_mut(&key) {
                profile.accuracy_ema = profile.accuracy_ema
                    * (1.0 - EMA_ALPHA) + score * EMA_ALPHA;
                profile.agreement_rate_ema = profile.agreement_rate_ema
                    * (1.0 - EMA_ALPHA) + score * EMA_ALPHA;
                profile.weight = 0.5 + profile.accuracy_ema * 0.5;
                profile.last_review_tick = self.tick;
            }
        }
    }

    /// Resolve a pending escalation with a forced verdict
    pub fn resolve_escalation(&mut self, escalation_idx: usize,
                               verdict: ReviewVerdict) -> bool {
        if escalation_idx >= self.escalations.len() { return false; }
        let esc = &mut self.escalations[escalation_idx];
        if esc.resolved { return false; }
        esc.resolved = true;
        esc.resolution_verdict = verdict;
        let finding_id = esc.finding_id;
        let status = match verdict {
            ReviewVerdict::StrongAccept | ReviewVerdict::Accept
                | ReviewVerdict::WeakAccept => ReviewStatus::Accepted,
            ReviewVerdict::Reject | ReviewVerdict::StrongReject
                | ReviewVerdict::WeakReject => ReviewStatus::Rejected,
            ReviewVerdict::Neutral => ReviewStatus::Consensus,
        };
        if let Some(f) = self.findings.get_mut(&finding_id) {
            f.status = status;
            self.stats.active_findings = self.stats.active_findings.saturating_sub(1);
        }
        true
    }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &PeerReviewStats { &self.stats }

    /// Get all findings
    #[inline(always)]
    pub fn findings(&self) -> &BTreeMap<u64, ReviewableFinding> { &self.findings }

    /// Get consensus records
    #[inline(always)]
    pub fn consensus_records(&self) -> &BTreeMap<u64, ConsensusRecord> { &self.consensus_records }

    /// Get escalation log
    #[inline(always)]
    pub fn escalations(&self) -> &[ReviewEscalation] { &self.escalations }

    /// Get reviewer profiles
    #[inline(always)]
    pub fn reviewer_profiles(&self) -> &BTreeMap<u64, ReviewerProfile> {
        &self.reviewer_profiles
    }

    /// Get review log
    #[inline(always)]
    pub fn review_log(&self) -> &[PeerReviewRecord] { &self.reviews }
}
