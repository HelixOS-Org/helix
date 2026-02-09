//! # Trust Scoring for Cooperative Processes
//!
//! Multi-dimensional trust model:
//! - Behavioral trust (how well does process follow advisories?)
//! - Resource trust (does process use resources responsibly?)
//! - Communication trust (are hints accurate?)
//! - Temporal trust (how long has process been cooperative?)
//! - Social trust (does process cooperate with others?)
//! - Trust decay and recovery
//! - Trust-based access control

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TRUST DIMENSIONS
// ============================================================================

/// Trust dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustDimension {
    /// Follows kernel advisories
    Behavioral,
    /// Uses resources responsibly
    Resource,
    /// Provides accurate hints/information
    Communication,
    /// Time-based trust accumulation
    Temporal,
    /// Cooperates with other processes
    Social,
    /// Security compliance
    Security,
}

/// Trust level category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrustLevel {
    /// Untrusted (new or misbehaving)
    Untrusted,
    /// Minimal trust
    Minimal,
    /// Moderate trust
    Moderate,
    /// High trust
    High,
    /// Fully trusted
    FullyTrusted,
}

impl TrustLevel {
    pub fn from_score(score: f64) -> Self {
        if score >= 0.9 {
            Self::FullyTrusted
        } else if score >= 0.7 {
            Self::High
        } else if score >= 0.4 {
            Self::Moderate
        } else if score >= 0.15 {
            Self::Minimal
        } else {
            Self::Untrusted
        }
    }
}

// ============================================================================
// TRUST EVIDENCE
// ============================================================================

/// Evidence that affects trust
#[derive(Debug, Clone, Copy)]
pub enum TrustEvidence {
    /// Advisory was followed
    AdvisoryFollowed {
        dimension: TrustDimension,
        weight: f64,
    },
    /// Advisory was ignored
    AdvisoryIgnored {
        dimension: TrustDimension,
        weight: f64,
    },
    /// Hint was accurate
    AccurateHint {
        accuracy: f64,
    },
    /// Hint was inaccurate
    InaccurateHint {
        error_magnitude: f64,
    },
    /// Resource used efficiently
    EfficientResourceUse {
        dimension: TrustDimension,
        efficiency: f64,
    },
    /// Resource wasted
    ResourceWaste {
        dimension: TrustDimension,
        waste_factor: f64,
    },
    /// Cooperative action with another process
    CooperativeAction {
        partner_pid: u64,
    },
    /// Uncooperative action
    UncooperativeAction {
        victim_pid: u64,
    },
    /// Security violation
    SecurityViolation {
        severity: f64,
    },
    /// Time-based trust accumulation
    TimePassed {
        seconds: u64,
    },
}

// ============================================================================
// DIMENSION SCORE
// ============================================================================

/// Score for a single trust dimension
#[derive(Debug, Clone)]
struct DimensionScore {
    /// Current score (0.0 - 1.0)
    score: f64,
    /// Confidence (0.0 - 1.0, based on evidence count)
    confidence: f64,
    /// Evidence count
    evidence_count: u64,
    /// Positive evidence count
    positive_count: u64,
    /// Negative evidence count
    negative_count: u64,
    /// Last update time
    last_update: u64,
    /// Weight of this dimension in overall trust
    weight: f64,
}

impl DimensionScore {
    fn new(initial_score: f64, weight: f64) -> Self {
        Self {
            score: initial_score,
            confidence: 0.0,
            evidence_count: 0,
            positive_count: 0,
            negative_count: 0,
            last_update: 0,
            weight,
        }
    }

    /// Apply positive evidence
    fn positive(&mut self, strength: f64, timestamp: u64) {
        self.evidence_count += 1;
        self.positive_count += 1;
        self.last_update = timestamp;

        // Bayesian-inspired update
        let alpha = 0.1 * strength;
        self.score = self.score * (1.0 - alpha) + 1.0 * alpha;
        if self.score > 1.0 {
            self.score = 1.0;
        }

        self.update_confidence();
    }

    /// Apply negative evidence
    fn negative(&mut self, strength: f64, timestamp: u64) {
        self.evidence_count += 1;
        self.negative_count += 1;
        self.last_update = timestamp;

        let alpha = 0.15 * strength; // Negative evidence has more impact
        self.score = self.score * (1.0 - alpha);
        if self.score < 0.0 {
            self.score = 0.0;
        }

        self.update_confidence();
    }

    /// Update confidence based on evidence count
    fn update_confidence(&mut self) {
        // Confidence grows logarithmically with evidence
        let count = self.evidence_count as f64;
        self.confidence = 1.0 - 1.0 / (1.0 + count * 0.1);
        if self.confidence > 1.0 {
            self.confidence = 1.0;
        }
    }

    /// Apply time-based decay toward neutral (0.5)
    fn decay(&mut self, elapsed_secs: u64) {
        if elapsed_secs == 0 {
            return;
        }
        let decay_rate = 0.001; // Per second
        let factor = 1.0 - decay_rate * elapsed_secs as f64;
        let factor = if factor < 0.0 { 0.0 } else { factor };

        // Decay toward 0.5
        let delta = self.score - 0.5;
        self.score = 0.5 + delta * factor;
    }

    /// Weighted contribution to overall trust
    fn weighted_score(&self) -> f64 {
        self.score * self.weight * self.confidence
    }

    /// Weighted maximum (for normalization)
    fn weighted_max(&self) -> f64 {
        self.weight * self.confidence
    }
}

// ============================================================================
// PROCESS TRUST PROFILE
// ============================================================================

/// Complete trust profile for a process
struct ProcessTrust {
    /// PID
    pid: u64,
    /// Dimension scores
    dimensions: BTreeMap<u8, DimensionScore>,
    /// Overall trust score (cached)
    overall: f64,
    /// Overall trust level
    level: TrustLevel,
    /// Creation time
    created_at: u64,
    /// Last decay time
    last_decay: u64,
    /// Interactions with other processes (pid → interaction count)
    interactions: LinearMap<u32, 64>,
}

impl ProcessTrust {
    fn new(pid: u64, timestamp: u64) -> Self {
        let mut dimensions = BTreeMap::new();

        // Initialize dimensions with default weights
        dimensions.insert(
            TrustDimension::Behavioral as u8,
            DimensionScore::new(0.5, 0.25),
        );
        dimensions.insert(
            TrustDimension::Resource as u8,
            DimensionScore::new(0.5, 0.20),
        );
        dimensions.insert(
            TrustDimension::Communication as u8,
            DimensionScore::new(0.5, 0.20),
        );
        dimensions.insert(
            TrustDimension::Temporal as u8,
            DimensionScore::new(0.0, 0.10),
        );
        dimensions.insert(
            TrustDimension::Social as u8,
            DimensionScore::new(0.5, 0.15),
        );
        dimensions.insert(
            TrustDimension::Security as u8,
            DimensionScore::new(0.5, 0.10),
        );

        Self {
            pid,
            dimensions,
            overall: 0.25,
            level: TrustLevel::Minimal,
            created_at: timestamp,
            last_decay: timestamp,
            interactions: LinearMap::new(),
        }
    }

    /// Apply trust evidence
    fn apply_evidence(&mut self, evidence: TrustEvidence, timestamp: u64) {
        match evidence {
            TrustEvidence::AdvisoryFollowed { dimension, weight } => {
                if let Some(dim) = self.dimensions.get_mut(&(dimension as u8)) {
                    dim.positive(weight, timestamp);
                }
            }
            TrustEvidence::AdvisoryIgnored { dimension, weight } => {
                if let Some(dim) = self.dimensions.get_mut(&(dimension as u8)) {
                    dim.negative(weight, timestamp);
                }
            }
            TrustEvidence::AccurateHint { accuracy } => {
                if let Some(dim) =
                    self.dimensions.get_mut(&(TrustDimension::Communication as u8))
                {
                    dim.positive(accuracy, timestamp);
                }
            }
            TrustEvidence::InaccurateHint { error_magnitude } => {
                if let Some(dim) =
                    self.dimensions.get_mut(&(TrustDimension::Communication as u8))
                {
                    dim.negative(error_magnitude, timestamp);
                }
            }
            TrustEvidence::EfficientResourceUse {
                dimension,
                efficiency,
            } => {
                if let Some(dim) = self.dimensions.get_mut(&(dimension as u8)) {
                    dim.positive(efficiency, timestamp);
                }
            }
            TrustEvidence::ResourceWaste {
                dimension,
                waste_factor,
            } => {
                if let Some(dim) = self.dimensions.get_mut(&(dimension as u8)) {
                    dim.negative(waste_factor, timestamp);
                }
            }
            TrustEvidence::CooperativeAction { partner_pid } => {
                if let Some(dim) = self.dimensions.get_mut(&(TrustDimension::Social as u8)) {
                    dim.positive(0.5, timestamp);
                }
                self.interactions.add(partner_pid, 1);
            }
            TrustEvidence::UncooperativeAction { victim_pid: _ } => {
                if let Some(dim) = self.dimensions.get_mut(&(TrustDimension::Social as u8)) {
                    dim.negative(0.7, timestamp);
                }
            }
            TrustEvidence::SecurityViolation { severity } => {
                if let Some(dim) =
                    self.dimensions.get_mut(&(TrustDimension::Security as u8))
                {
                    dim.negative(severity, timestamp);
                }
            }
            TrustEvidence::TimePassed { seconds } => {
                if let Some(dim) =
                    self.dimensions.get_mut(&(TrustDimension::Temporal as u8))
                {
                    // Time trust grows slowly
                    let growth = (seconds as f64 * 0.0001).min(0.1);
                    dim.positive(growth, timestamp);
                }
            }
        }

        self.recalculate_overall();
    }

    /// Recalculate overall trust score
    fn recalculate_overall(&mut self) {
        let weighted_sum: f64 = self.dimensions.values().map(|d| d.weighted_score()).sum();
        let weight_total: f64 = self.dimensions.values().map(|d| d.weighted_max()).sum();

        self.overall = if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            0.0
        };

        self.level = TrustLevel::from_score(self.overall);
    }

    /// Apply time-based decay
    fn decay(&mut self, current_time: u64) {
        let elapsed = current_time.saturating_sub(self.last_decay) / 1000;
        if elapsed == 0 {
            return;
        }
        self.last_decay = current_time;

        for dim in self.dimensions.values_mut() {
            dim.decay(elapsed);
        }

        self.recalculate_overall();
    }
}

// ============================================================================
// TRUST MANAGER
// ============================================================================

/// Trust query result
#[derive(Debug, Clone)]
pub struct TrustSnapshot {
    /// PID
    pub pid: u64,
    /// Overall trust (0.0 - 1.0)
    pub overall: f64,
    /// Trust level
    pub level: TrustLevel,
    /// Per-dimension scores
    pub dimensions: Vec<(TrustDimension, f64, f64)>, // (dim, score, confidence)
    /// Process age (seconds)
    pub age_secs: u64,
}

/// Global trust manager
pub struct TrustManager {
    /// Per-process trust profiles
    profiles: BTreeMap<u64, ProcessTrust>,
    /// Trust requirements for actions
    requirements: BTreeMap<u8, f64>,
    /// Total evidence applied
    pub total_evidence: u64,
    /// Processes at each level
    pub level_counts: [u32; 5],
}

impl TrustManager {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            requirements: BTreeMap::new(),
            total_evidence: 0,
            level_counts: [0; 5],
        }
    }

    /// Register a process
    #[inline(always)]
    pub fn register(&mut self, pid: u64, timestamp: u64) {
        self.profiles.entry(pid).or_insert_with(|| ProcessTrust::new(pid, timestamp));
        self.update_level_counts();
    }

    /// Unregister a process
    #[inline(always)]
    pub fn unregister(&mut self, pid: u64) {
        self.profiles.remove(&pid);
        self.update_level_counts();
    }

    /// Submit trust evidence
    #[inline]
    pub fn submit_evidence(
        &mut self,
        pid: u64,
        evidence: TrustEvidence,
        timestamp: u64,
    ) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.apply_evidence(evidence, timestamp);
            self.total_evidence += 1;
        }
        self.update_level_counts();
    }

    /// Get trust snapshot
    pub fn get_trust(&self, pid: u64) -> Option<TrustSnapshot> {
        let profile = self.profiles.get(&pid)?;

        let dimensions: Vec<(TrustDimension, f64, f64)> = vec![
            (TrustDimension::Behavioral, 0.0, 0.0),
            (TrustDimension::Resource, 0.0, 0.0),
            (TrustDimension::Communication, 0.0, 0.0),
            (TrustDimension::Temporal, 0.0, 0.0),
            (TrustDimension::Social, 0.0, 0.0),
            (TrustDimension::Security, 0.0, 0.0),
        ]
        .into_iter()
        .map(|(dim, _, _)| {
            let key = dim as u8;
            if let Some(ds) = profile.dimensions.get(&key) {
                (dim, ds.score, ds.confidence)
            } else {
                (dim, 0.0, 0.0)
            }
        })
        .collect();

        Some(TrustSnapshot {
            pid,
            overall: profile.overall,
            level: profile.level,
            dimensions,
            age_secs: 0, // Would need current_time
        })
    }

    /// Get overall trust score
    #[inline]
    pub fn trust_score(&self, pid: u64) -> f64 {
        self.profiles
            .get(&pid)
            .map_or(0.0, |p| p.overall)
    }

    /// Get trust level
    #[inline]
    pub fn trust_level(&self, pid: u64) -> TrustLevel {
        self.profiles
            .get(&pid)
            .map_or(TrustLevel::Untrusted, |p| p.level)
    }

    /// Check if process meets trust requirement
    #[inline(always)]
    pub fn meets_requirement(&self, pid: u64, required_level: TrustLevel) -> bool {
        self.trust_level(pid) >= required_level
    }

    /// Decay all profiles
    #[inline]
    pub fn decay_all(&mut self, current_time: u64) {
        for profile in self.profiles.values_mut() {
            profile.decay(current_time);
        }
        self.update_level_counts();
    }

    /// Update level distribution
    fn update_level_counts(&mut self) {
        self.level_counts = [0; 5];
        for profile in self.profiles.values() {
            let idx = match profile.level {
                TrustLevel::Untrusted => 0,
                TrustLevel::Minimal => 1,
                TrustLevel::Moderate => 2,
                TrustLevel::High => 3,
                TrustLevel::FullyTrusted => 4,
            };
            self.level_counts[idx] += 1;
        }
    }

    /// Average trust score
    #[inline]
    pub fn average_trust(&self) -> f64 {
        if self.profiles.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.profiles.values().map(|p| p.overall).sum();
        sum / self.profiles.len() as f64
    }

    /// Process count
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.profiles.len()
    }
}
