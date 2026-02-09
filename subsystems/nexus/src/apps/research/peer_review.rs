// SPDX-License-Identifier: GPL-2.0
//! # Apps Peer Review — Cross-validation of App Research Findings
//!
//! Bridge and coop subsystems review app research findings. Every discovery
//! is submitted for peer review before it influences production classification.
//! Multiple independent reviewers score findings, and a consensus mechanism
//! determines whether the finding meets the evidence bar. Replication requests
//! are issued for findings that are promising but not yet fully validated.
//!
//! The engine that ensures research integrity in app understanding.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_FINDINGS: usize = 512;
const MAX_REVIEWS_PER_FINDING: usize = 8;
const MAX_BOARD_MEMBERS: usize = 16;
const CONSENSUS_THRESHOLD: f32 = 0.70;
const HIGH_CONFIDENCE: f32 = 0.85;
const REPLICATION_THRESHOLD: f32 = 0.50;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const REVIEW_WEIGHT_SENIOR: f32 = 1.5;
const REVIEW_WEIGHT_JUNIOR: f32 = 1.0;
const MAX_REPLICATION_REQUESTS: usize = 256;
const CONFIDENCE_DECAY: f32 = 0.998;
const MIN_REVIEWS_FOR_CONSENSUS: usize = 3;

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

// ============================================================================
// TYPES
// ============================================================================

/// Status of a finding in the peer review pipeline.
#[derive(Clone, Copy, PartialEq)]
pub enum FindingStatus {
    Submitted,
    UnderReview,
    Accepted,
    Rejected,
    ReplicationRequested,
    Withdrawn,
}

/// A research finding submitted for peer review.
#[derive(Clone)]
pub struct ResearchFinding {
    pub finding_id: u64,
    pub title: String,
    pub category: String,
    pub evidence_strength: f32,
    pub effect_size: f32,
    pub sample_size: usize,
    pub status: FindingStatus,
    pub submitted_tick: u64,
    pub reviews: Vec<PeerReviewEntry>,
    pub consensus_score: f32,
}

/// A single review from one reviewer.
#[derive(Clone)]
pub struct PeerReviewEntry {
    pub reviewer_id: u64,
    pub score: f32,
    pub weight: f32,
    pub methodology_ok: bool,
    pub reproducible: bool,
    pub comment_hash: u64,
    pub review_tick: u64,
}

/// A replication request for a finding.
#[derive(Clone)]
pub struct ReplicationRequest {
    pub request_id: u64,
    pub finding_id: u64,
    pub priority: f32,
    pub requested_tick: u64,
    pub status: ReplicationStatus,
    pub attempts: u32,
    pub last_result: f32,
}

/// Status of a replication request.
#[derive(Clone, Copy, PartialEq)]
pub enum ReplicationStatus {
    Pending,
    InProgress,
    Replicated,
    FailedToReplicate,
    Abandoned,
}

/// Review board member profile.
#[derive(Clone)]
pub struct BoardMember {
    pub member_id: u64,
    pub label: String,
    pub expertise_weight: f32,
    pub reviews_completed: u64,
    pub accuracy_ema: f32,
    pub senior: bool,
}

/// Confidence metric for a finding.
#[derive(Clone)]
pub struct PeerConfidence {
    pub finding_id: u64,
    pub weighted_score: f32,
    pub review_count: usize,
    pub methodology_approval: f32,
    pub reproducibility_approval: f32,
    pub overall_confidence: f32,
}

/// Engine-level statistics.
#[derive(Clone)]
pub struct PeerReviewStats {
    pub findings_submitted: u64,
    pub findings_accepted: u64,
    pub findings_rejected: u64,
    pub replications_requested: u64,
    pub ema_consensus: f32,
    pub ema_confidence: f32,
    pub ema_accept_rate: f32,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Cross-validation engine for app research findings.
pub struct AppsPeerReview {
    findings: BTreeMap<u64, ResearchFinding>,
    replication_requests: BTreeMap<u64, ReplicationRequest>,
    board: BTreeMap<u64, BoardMember>,
    stats: PeerReviewStats,
    rng_state: u64,
    tick: u64,
}

impl AppsPeerReview {
    /// Create a new peer review engine.
    pub fn new(seed: u64) -> Self {
        Self {
            findings: BTreeMap::new(),
            replication_requests: BTreeMap::new(),
            board: BTreeMap::new(),
            stats: PeerReviewStats {
                findings_submitted: 0,
                findings_accepted: 0,
                findings_rejected: 0,
                replications_requested: 0,
                ema_consensus: 0.0,
                ema_confidence: 0.0,
                ema_accept_rate: 0.0,
            },
            rng_state: seed ^ 0xe7c1a5d329bf0846,
            tick: 0,
        }
    }

    // ── Primary API ────────────────────────────────────────────────────

    /// Submit a research finding for peer review.
    pub fn submit_finding(
        &mut self,
        title: &str,
        category: &str,
        evidence_strength: f32,
        effect_size: f32,
        sample_size: usize,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(title.as_bytes()) ^ self.tick;
        self.stats.findings_submitted += 1;

        let finding = ResearchFinding {
            finding_id: id,
            title: String::from(title),
            category: String::from(category),
            evidence_strength: evidence_strength.min(1.0).max(0.0),
            effect_size,
            sample_size,
            status: FindingStatus::Submitted,
            submitted_tick: self.tick,
            reviews: Vec::new(),
            consensus_score: 0.0,
        };

        if self.findings.len() >= MAX_FINDINGS {
            // Evict oldest rejected finding
            let mut evict_id = None;
            let mut oldest_tick = u64::MAX;
            for (fid, f) in self.findings.iter() {
                if f.status == FindingStatus::Rejected && f.submitted_tick < oldest_tick {
                    oldest_tick = f.submitted_tick;
                    evict_id = Some(*fid);
                }
            }
            if let Some(eid) = evict_id {
                self.findings.remove(&eid);
            } else {
                // Evict absolute oldest
                if let Some(oldest) = self.findings.keys().next().cloned() {
                    self.findings.remove(&oldest);
                }
            }
        }

        self.findings.insert(id, finding);
        id
    }

    /// Perform cross-validation of a finding by a specific reviewer.
    pub fn cross_validate(&mut self, finding_id: u64, reviewer_id: u64, score: f32, method_ok: bool, reproducible: bool) -> bool {
        let weight = if let Some(member) = self.board.get_mut(&reviewer_id) {
            member.reviews_completed += 1;
            if member.senior { REVIEW_WEIGHT_SENIOR } else { REVIEW_WEIGHT_JUNIOR }
        } else {
            REVIEW_WEIGHT_JUNIOR
        };

        if let Some(finding) = self.findings.get_mut(&finding_id) {
            if finding.reviews.len() >= MAX_REVIEWS_PER_FINDING {
                return false;
            }

            let entry = PeerReviewEntry {
                reviewer_id,
                score: score.min(1.0).max(0.0),
                weight,
                methodology_ok: method_ok,
                reproducible,
                comment_hash: fnv1a_hash(&reviewer_id.to_le_bytes()),
                review_tick: self.tick,
            };
            finding.reviews.push(entry);

            if finding.status == FindingStatus::Submitted {
                finding.status = FindingStatus::UnderReview;
            }

            // Recompute consensus if enough reviews
            if finding.reviews.len() >= MIN_REVIEWS_FOR_CONSENSUS {
                let consensus = self.compute_consensus(&finding.reviews);
                finding.consensus_score = consensus;

                if consensus >= HIGH_CONFIDENCE {
                    finding.status = FindingStatus::Accepted;
                    self.stats.findings_accepted += 1;
                } else if consensus < REPLICATION_THRESHOLD && finding.reviews.len() >= MAX_REVIEWS_PER_FINDING {
                    finding.status = FindingStatus::Rejected;
                    self.stats.findings_rejected += 1;
                } else if consensus >= REPLICATION_THRESHOLD && consensus < CONSENSUS_THRESHOLD {
                    finding.status = FindingStatus::ReplicationRequested;
                }

                let accept_rate = self.stats.findings_accepted as f32
                    / self.stats.findings_submitted.max(1) as f32;
                self.stats.ema_accept_rate =
                    EMA_ALPHA * accept_rate + (1.0 - EMA_ALPHA) * self.stats.ema_accept_rate;
                self.stats.ema_consensus =
                    EMA_ALPHA * consensus + (1.0 - EMA_ALPHA) * self.stats.ema_consensus;
            }
            true
        } else {
            false
        }
    }

    /// Get the consensus score for a finding.
    pub fn consensus_score(&self, finding_id: u64) -> Option<f32> {
        self.findings.get(&finding_id).map(|f| f.consensus_score)
    }

    /// Issue a replication request for a finding.
    pub fn replication_request(&mut self, finding_id: u64, priority: f32) -> Option<u64> {
        if !self.findings.contains_key(&finding_id) {
            return None;
        }
        self.tick += 1;
        self.stats.replications_requested += 1;

        let req_id = fnv1a_hash(&finding_id.to_le_bytes()) ^ self.tick;
        let request = ReplicationRequest {
            request_id: req_id,
            finding_id,
            priority: priority.min(1.0).max(0.0),
            requested_tick: self.tick,
            status: ReplicationStatus::Pending,
            attempts: 0,
            last_result: 0.0,
        };

        if self.replication_requests.len() >= MAX_REPLICATION_REQUESTS {
            // Evict lowest-priority request
            let mut min_id = 0u64;
            let mut min_pri = f32::MAX;
            for (rid, r) in self.replication_requests.iter() {
                if r.priority < min_pri {
                    min_pri = r.priority;
                    min_id = *rid;
                }
            }
            self.replication_requests.remove(&min_id);
        }
        self.replication_requests.insert(req_id, request);
        Some(req_id)
    }

    /// Get current review board composition and stats.
    pub fn review_board(&self) -> Vec<BoardMember> {
        self.board.values().cloned().collect()
    }

    /// Compute peer confidence for a specific finding.
    pub fn peer_confidence(&self, finding_id: u64) -> Option<PeerConfidence> {
        let finding = self.findings.get(&finding_id)?;
        if finding.reviews.is_empty() {
            return Some(PeerConfidence {
                finding_id,
                weighted_score: 0.0,
                review_count: 0,
                methodology_approval: 0.0,
                reproducibility_approval: 0.0,
                overall_confidence: 0.0,
            });
        }

        let mut weight_sum = 0.0f32;
        let mut score_sum = 0.0f32;
        let mut method_yes = 0u32;
        let mut repro_yes = 0u32;

        for review in &finding.reviews {
            score_sum += review.score * review.weight;
            weight_sum += review.weight;
            if review.methodology_ok {
                method_yes += 1;
            }
            if review.reproducible {
                repro_yes += 1;
            }
        }

        let n = finding.reviews.len() as f32;
        let weighted = if weight_sum > 0.0 { score_sum / weight_sum } else { 0.0 };
        let method_rate = method_yes as f32 / n;
        let repro_rate = repro_yes as f32 / n;
        let overall = weighted * 0.5 + method_rate * 0.25 + repro_rate * 0.25;

        Some(PeerConfidence {
            finding_id,
            weighted_score: weighted,
            review_count: finding.reviews.len(),
            methodology_approval: method_rate,
            reproducibility_approval: repro_rate,
            overall_confidence: overall,
        })
    }

    /// Add a member to the review board.
    pub fn add_board_member(&mut self, label: &str, senior: bool) -> u64 {
        let id = fnv1a_hash(label.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let weight = if senior { REVIEW_WEIGHT_SENIOR } else { REVIEW_WEIGHT_JUNIOR };

        let member = BoardMember {
            member_id: id,
            label: String::from(label),
            expertise_weight: weight,
            reviews_completed: 0,
            accuracy_ema: 0.5,
            senior,
        };

        if self.board.len() >= MAX_BOARD_MEMBERS {
            // Evict least-active member
            let mut min_id = 0u64;
            let mut min_reviews = u64::MAX;
            for (mid, m) in self.board.iter() {
                if m.reviews_completed < min_reviews {
                    min_reviews = m.reviews_completed;
                    min_id = *mid;
                }
            }
            self.board.remove(&min_id);
        }
        self.board.insert(id, member);
        id
    }

    /// Update a replication request with new results.
    pub fn update_replication(&mut self, request_id: u64, result_score: f32, replicated: bool) {
        if let Some(req) = self.replication_requests.get_mut(&request_id) {
            req.attempts += 1;
            req.last_result = result_score;
            req.status = if replicated {
                ReplicationStatus::Replicated
            } else if req.attempts >= 3 {
                ReplicationStatus::FailedToReplicate
            } else {
                ReplicationStatus::InProgress
            };

            // Update finding status based on replication result
            if replicated {
                if let Some(finding) = self.findings.get_mut(&req.finding_id) {
                    if finding.status == FindingStatus::ReplicationRequested {
                        finding.status = FindingStatus::Accepted;
                        self.stats.findings_accepted += 1;
                    }
                }
            }
        }
    }

    /// Return engine stats.
    pub fn stats(&self) -> &PeerReviewStats {
        &self.stats
    }

    // ── Internal Helpers ───────────────────────────────────────────────

    fn compute_consensus(&self, reviews: &[PeerReviewEntry]) -> f32 {
        if reviews.is_empty() {
            return 0.0;
        }
        let mut weight_sum = 0.0f32;
        let mut score_sum = 0.0f32;

        for review in reviews {
            let w = review.weight;
            let method_bonus = if review.methodology_ok { 0.1 } else { -0.1 };
            let repro_bonus = if review.reproducible { 0.1 } else { -0.1 };
            let adjusted = (review.score + method_bonus + repro_bonus).min(1.0).max(0.0);
            score_sum += adjusted * w;
            weight_sum += w;
        }

        if weight_sum > 0.0 { score_sum / weight_sum } else { 0.0 }
    }
}
