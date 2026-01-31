//! Conflict Resolver — Handles conflicting options
//!
//! The conflict resolver detects when two options are incompatible
//! (e.g., enable vs disable the same target) and resolves the conflict
//! using configured strategies.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, Ordering};

use super::options::{OptionId, ActionType, ActionTarget, ActionCost};
use super::ranker::RankedOption;

// ============================================================================
// CONFLICT
// ============================================================================

/// A conflict between options
#[derive(Debug, Clone)]
pub struct Conflict {
    /// First option
    pub option_a: OptionId,
    /// Second option
    pub option_b: OptionId,
    /// Conflict type
    pub conflict_type: ConflictType,
    /// Description
    pub description: String,
}

impl Conflict {
    /// Create new conflict
    pub fn new(
        option_a: OptionId,
        option_b: OptionId,
        conflict_type: ConflictType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            option_a,
            option_b,
            conflict_type,
            description: description.into(),
        }
    }

    /// Is this a critical conflict?
    pub fn is_critical(&self) -> bool {
        matches!(self.conflict_type, ConflictType::IncompatibleActions)
    }
}

/// Conflict type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictType {
    /// Actions are incompatible
    IncompatibleActions,
    /// Resource contention
    ResourceContention,
    /// Timing conflict
    Timing,
    /// Dependency conflict
    Dependency,
}

impl ConflictType {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::IncompatibleActions => "Incompatible Actions",
            Self::ResourceContention => "Resource Contention",
            Self::Timing => "Timing Conflict",
            Self::Dependency => "Dependency Conflict",
        }
    }
}

// ============================================================================
// RESOLUTION
// ============================================================================

/// Conflict resolution
#[derive(Debug, Clone)]
pub struct Resolution {
    /// The conflict
    pub conflict: Conflict,
    /// Winning option
    pub winner: Option<OptionId>,
    /// Losing option
    pub loser: Option<OptionId>,
    /// Strategy used
    pub strategy: ResolutionStrategy,
}

impl Resolution {
    /// Create new resolution
    pub fn new(conflict: Conflict, strategy: ResolutionStrategy) -> Self {
        Self {
            conflict,
            winner: None,
            loser: None,
            strategy,
        }
    }

    /// Set winner and loser
    pub fn with_outcome(mut self, winner: OptionId, loser: OptionId) -> Self {
        self.winner = Some(winner);
        self.loser = Some(loser);
        self
    }

    /// Was conflict resolved?
    pub fn is_resolved(&self) -> bool {
        self.winner.is_some()
    }
}

/// Resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStrategy {
    /// Higher score wins
    HigherScoreWins,
    /// Safety wins
    SafetyWins,
    /// Both deferred
    BothDeferred,
    /// Both cancelled
    BothCancelled,
    /// Merged
    Merged,
}

impl ResolutionStrategy {
    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::HigherScoreWins => "Higher Score Wins",
            Self::SafetyWins => "Safety Wins",
            Self::BothDeferred => "Both Deferred",
            Self::BothCancelled => "Both Cancelled",
            Self::Merged => "Merged",
        }
    }
}

// ============================================================================
// CONFLICT RESOLVER
// ============================================================================

/// Conflict resolver - handles conflicting options
pub struct ConflictResolver {
    /// Default strategy
    default_strategy: ResolutionStrategy,
    /// Conflicts detected
    conflicts_detected: AtomicU64,
    /// Conflicts resolved
    conflicts_resolved: AtomicU64,
}

impl ConflictResolver {
    /// Create new resolver
    pub fn new() -> Self {
        Self {
            default_strategy: ResolutionStrategy::HigherScoreWins,
            conflicts_detected: AtomicU64::new(0),
            conflicts_resolved: AtomicU64::new(0),
        }
    }

    /// Create with specific strategy
    pub fn with_strategy(strategy: ResolutionStrategy) -> Self {
        Self {
            default_strategy: strategy,
            conflicts_detected: AtomicU64::new(0),
            conflicts_resolved: AtomicU64::new(0),
        }
    }

    /// Set default strategy
    pub fn set_strategy(&mut self, strategy: ResolutionStrategy) {
        self.default_strategy = strategy;
    }

    /// Detect conflicts between options
    pub fn detect_conflicts(&self, options: &[RankedOption]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        for i in 0..options.len() {
            for j in (i + 1)..options.len() {
                if let Some(conflict) = self.check_conflict(&options[i], &options[j]) {
                    conflicts.push(conflict);
                    self.conflicts_detected.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        conflicts
    }

    /// Check if two options conflict
    fn check_conflict(&self, a: &RankedOption, b: &RankedOption) -> Option<Conflict> {
        // Same target with incompatible actions
        if self.targets_overlap(&a.option.target, &b.option.target) {
            if self.actions_conflict(a.option.action_type, b.option.action_type) {
                return Some(Conflict {
                    option_a: a.option.id,
                    option_b: b.option.id,
                    conflict_type: ConflictType::IncompatibleActions,
                    description: format!(
                        "{:?} conflicts with {:?}",
                        a.option.action_type, b.option.action_type
                    ),
                });
            }
        }

        // Resource contention
        if self.resources_contend(&a.option.cost, &b.option.cost) {
            return Some(Conflict {
                option_a: a.option.id,
                option_b: b.option.id,
                conflict_type: ConflictType::ResourceContention,
                description: String::from("Resource contention"),
            });
        }

        None
    }

    /// Check if targets overlap
    fn targets_overlap(&self, a: &ActionTarget, b: &ActionTarget) -> bool {
        match (a, b) {
            (ActionTarget::System, _) | (_, ActionTarget::System) => true,
            (ActionTarget::Cpu(x), ActionTarget::Cpu(y)) => x == y,
            (ActionTarget::Process(x), ActionTarget::Process(y)) => x == y,
            _ => false,
        }
    }

    /// Check if actions conflict
    fn actions_conflict(&self, a: ActionType, b: ActionType) -> bool {
        match (a, b) {
            (ActionType::Enable, ActionType::Disable)
            | (ActionType::Disable, ActionType::Enable) => true,
            (ActionType::Kill, ActionType::Restart)
            | (ActionType::Restart, ActionType::Kill) => true,
            (ActionType::Allocate, ActionType::Deallocate)
            | (ActionType::Deallocate, ActionType::Allocate) => true,
            _ => false,
        }
    }

    /// Check resource contention
    fn resources_contend(&self, a: &ActionCost, b: &ActionCost) -> bool {
        (a.cpu as u16 + b.cpu as u16) > 100 || (a.io as u16 + b.io as u16) > 100
    }

    /// Resolve conflicts
    pub fn resolve(&self, conflicts: &[Conflict], options: &mut [RankedOption]) -> Vec<Resolution> {
        let mut resolutions = Vec::new();

        for conflict in conflicts {
            let resolution = self.resolve_conflict(conflict, options);
            resolutions.push(resolution);
            self.conflicts_resolved.fetch_add(1, Ordering::Relaxed);
        }

        resolutions
    }

    /// Resolve a single conflict
    fn resolve_conflict(&self, conflict: &Conflict, options: &[RankedOption]) -> Resolution {
        // Find the options
        let option_a = options.iter().find(|o| o.option.id == conflict.option_a);
        let option_b = options.iter().find(|o| o.option.id == conflict.option_b);

        match (option_a, option_b) {
            (Some(a), Some(b)) => {
                match self.default_strategy {
                    ResolutionStrategy::HigherScoreWins => {
                        if a.score >= b.score {
                            Resolution {
                                conflict: conflict.clone(),
                                winner: Some(conflict.option_a),
                                loser: Some(conflict.option_b),
                                strategy: ResolutionStrategy::HigherScoreWins,
                            }
                        } else {
                            Resolution {
                                conflict: conflict.clone(),
                                winner: Some(conflict.option_b),
                                loser: Some(conflict.option_a),
                                strategy: ResolutionStrategy::HigherScoreWins,
                            }
                        }
                    }
                    ResolutionStrategy::SafetyWins => {
                        // Prefer safe actions
                        let a_safe = a.option.action_type.is_safe();
                        let b_safe = b.option.action_type.is_safe();
                        if a_safe && !b_safe {
                            Resolution {
                                conflict: conflict.clone(),
                                winner: Some(conflict.option_a),
                                loser: Some(conflict.option_b),
                                strategy: ResolutionStrategy::SafetyWins,
                            }
                        } else if b_safe && !a_safe {
                            Resolution {
                                conflict: conflict.clone(),
                                winner: Some(conflict.option_b),
                                loser: Some(conflict.option_a),
                                strategy: ResolutionStrategy::SafetyWins,
                            }
                        } else {
                            // Both same safety, fall back to score
                            self.resolve_by_score(conflict, a.score, b.score)
                        }
                    }
                    ResolutionStrategy::BothDeferred => {
                        Resolution {
                            conflict: conflict.clone(),
                            winner: None,
                            loser: None,
                            strategy: ResolutionStrategy::BothDeferred,
                        }
                    }
                    ResolutionStrategy::BothCancelled => {
                        Resolution {
                            conflict: conflict.clone(),
                            winner: None,
                            loser: None,
                            strategy: ResolutionStrategy::BothCancelled,
                        }
                    }
                    ResolutionStrategy::Merged => {
                        // Merging is complex, default to score
                        self.resolve_by_score(conflict, a.score, b.score)
                    }
                }
            }
            _ => Resolution {
                conflict: conflict.clone(),
                winner: None,
                loser: None,
                strategy: ResolutionStrategy::BothDeferred,
            },
        }
    }

    /// Resolve by score
    fn resolve_by_score(&self, conflict: &Conflict, score_a: f32, score_b: f32) -> Resolution {
        if score_a >= score_b {
            Resolution {
                conflict: conflict.clone(),
                winner: Some(conflict.option_a),
                loser: Some(conflict.option_b),
                strategy: ResolutionStrategy::HigherScoreWins,
            }
        } else {
            Resolution {
                conflict: conflict.clone(),
                winner: Some(conflict.option_b),
                loser: Some(conflict.option_a),
                strategy: ResolutionStrategy::HigherScoreWins,
            }
        }
    }

    /// Get statistics
    pub fn stats(&self) -> ConflictStats {
        ConflictStats {
            conflicts_detected: self.conflicts_detected.load(Ordering::Relaxed),
            conflicts_resolved: self.conflicts_resolved.load(Ordering::Relaxed),
        }
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Conflict statistics
#[derive(Debug, Clone)]
pub struct ConflictStats {
    /// Total conflicts detected
    pub conflicts_detected: u64,
    /// Total conflicts resolved
    pub conflicts_resolved: u64,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::options::{Option, ActionParameters, ExpectedOutcome, OptionSource};
    use crate::types::Duration;

    fn make_option(id: u64, action_type: ActionType) -> RankedOption {
        RankedOption {
            option: Option {
                id: OptionId::new(id),
                action_type,
                description: String::from("Test"),
                target: ActionTarget::System,
                parameters: ActionParameters::new(),
                expected_outcome: ExpectedOutcome {
                    description: String::new(),
                    success_probability: 1.0,
                    time_to_effect: Duration::ZERO,
                    side_effects: Vec::new(),
                },
                reversible: true,
                cost: ActionCost::default(),
                source: OptionSource::Default,
            },
            score: 0.5,
            policy_result: None,
        }
    }

    #[test]
    fn test_conflict_detection() {
        let resolver = ConflictResolver::new();

        let option_a = make_option(1, ActionType::Enable);
        let option_b = make_option(2, ActionType::Disable);

        let conflicts = resolver.detect_conflicts(&[option_a, option_b]);
        assert!(!conflicts.is_empty());
        assert_eq!(conflicts[0].conflict_type, ConflictType::IncompatibleActions);
    }

    #[test]
    fn test_no_conflict() {
        let resolver = ConflictResolver::new();

        let option_a = make_option(1, ActionType::Log);
        let option_b = make_option(2, ActionType::Alert);

        let conflicts = resolver.detect_conflicts(&[option_a, option_b]);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_conflict_resolution() {
        let resolver = ConflictResolver::new();

        let mut option_a = make_option(1, ActionType::Enable);
        option_a.score = 0.8;
        let mut option_b = make_option(2, ActionType::Disable);
        option_b.score = 0.5;

        let conflicts = resolver.detect_conflicts(&[option_a.clone(), option_b.clone()]);
        let resolutions = resolver.resolve(&conflicts, &mut [option_a, option_b]);

        assert!(!resolutions.is_empty());
        assert_eq!(resolutions[0].winner, Some(OptionId::new(1)));
    }
}
