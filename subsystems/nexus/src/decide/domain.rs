//! Decide Domain — Orchestrator
//!
//! The DecideDomain is the main orchestrator for the decision layer.
//! It coordinates option generation, policy evaluation, ranking,
//! conflict resolution, and intent production.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::types::*;
use super::conclusion::Conclusion;
use super::generator::OptionGenerator;
use super::policy::{PolicyEngine, PolicyStats};
use super::ranker::{PriorityRanker, RankingContext, RankingWeights, RankedOption};
use super::conflict::{ConflictResolver, ConflictStats};
use super::intent::Intent;

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Configuration for Decide domain
#[derive(Debug, Clone)]
pub struct DecideConfig {
    /// Maximum options to generate per conclusion
    pub max_options: usize,
    /// Maximum intents per tick
    pub max_intents_per_tick: usize,
    /// Default TTL for intents
    pub intent_ttl: Duration,
    /// Enable safety policies
    pub enable_safety: bool,
    /// Ranking weights
    pub ranking_weights: RankingWeights,
}

impl Default for DecideConfig {
    fn default() -> Self {
        Self {
            max_options: 10,
            max_intents_per_tick: 100,
            intent_ttl: Duration::from_secs(30),
            enable_safety: true,
            ranking_weights: RankingWeights::default(),
        }
    }
}

impl DecideConfig {
    /// Create minimal configuration
    pub fn minimal() -> Self {
        Self {
            max_options: 5,
            max_intents_per_tick: 10,
            intent_ttl: Duration::from_secs(10),
            enable_safety: true,
            ranking_weights: RankingWeights::default(),
        }
    }

    /// Create aggressive configuration
    pub fn aggressive() -> Self {
        Self {
            max_options: 20,
            max_intents_per_tick: 500,
            intent_ttl: Duration::from_secs(60),
            enable_safety: true,
            ranking_weights: RankingWeights::performance_first(),
        }
    }

    /// Create safe configuration
    pub fn safe() -> Self {
        Self {
            max_options: 10,
            max_intents_per_tick: 50,
            intent_ttl: Duration::from_secs(30),
            enable_safety: true,
            ranking_weights: RankingWeights::safety_first(),
        }
    }
}

// ============================================================================
// DECIDE DOMAIN
// ============================================================================

/// The Decide domain - decision layer
pub struct DecideDomain {
    /// Domain ID
    id: DomainId,
    /// Configuration
    config: DecideConfig,
    /// Is running
    running: AtomicBool,
    /// Option generator
    generator: OptionGenerator,
    /// Policy engine
    policy_engine: PolicyEngine,
    /// Priority ranker
    ranker: PriorityRanker,
    /// Conflict resolver
    resolver: ConflictResolver,
    /// Pending conclusions
    pending: Vec<Conclusion>,
    /// Total ticks
    total_ticks: AtomicU64,
    /// Total decisions made
    total_decisions: AtomicU64,
}

impl DecideDomain {
    /// Create new Decide domain
    pub fn new(config: DecideConfig) -> Self {
        Self {
            id: DomainId::generate(),
            config: config.clone(),
            running: AtomicBool::new(false),
            generator: OptionGenerator::new(),
            policy_engine: PolicyEngine::new(),
            ranker: PriorityRanker::new(config.ranking_weights),
            resolver: ConflictResolver::new(),
            pending: Vec::new(),
            total_ticks: AtomicU64::new(0),
            total_decisions: AtomicU64::new(0),
        }
    }

    /// Get domain ID
    pub fn id(&self) -> DomainId {
        self.id
    }

    /// Is running?
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Get configuration
    pub fn config(&self) -> &DecideConfig {
        &self.config
    }

    /// Get policy engine
    pub fn policy_engine(&self) -> &PolicyEngine {
        &self.policy_engine
    }

    /// Get mutable policy engine
    pub fn policy_engine_mut(&mut self) -> &mut PolicyEngine {
        &mut self.policy_engine
    }

    /// Get option generator
    pub fn generator(&self) -> &OptionGenerator {
        &self.generator
    }

    /// Get mutable option generator
    pub fn generator_mut(&mut self) -> &mut OptionGenerator {
        &mut self.generator
    }

    /// Start the domain
    pub fn start(&mut self) -> Result<(), DecideError> {
        if self.running.load(Ordering::Acquire) {
            return Err(DecideError::AlreadyRunning);
        }
        self.running.store(true, Ordering::Release);
        Ok(())
    }

    /// Stop the domain
    pub fn stop(&mut self) -> Result<(), DecideError> {
        if !self.running.load(Ordering::Acquire) {
            return Err(DecideError::NotRunning);
        }
        self.running.store(false, Ordering::Release);
        Ok(())
    }

    /// Submit a conclusion for decision
    pub fn submit(&mut self, conclusion: Conclusion) {
        self.pending.push(conclusion);
    }

    /// Submit multiple conclusions
    pub fn submit_batch(&mut self, conclusions: Vec<Conclusion>) {
        self.pending.extend(conclusions);
    }

    /// Get pending count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear pending
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    /// Process one tick
    pub fn tick(&mut self, now: Timestamp) -> Vec<Intent> {
        if !self.running.load(Ordering::Acquire) {
            return Vec::new();
        }

        self.total_ticks.fetch_add(1, Ordering::Relaxed);

        let mut intents = Vec::new();
        let conclusions: Vec<_> = self.pending.drain(..).collect();

        for conclusion in conclusions {
            if let Some(intent) = self.process_conclusion(&conclusion, now) {
                intents.push(intent);
                self.total_decisions.fetch_add(1, Ordering::Relaxed);

                if intents.len() >= self.config.max_intents_per_tick {
                    break;
                }
            }
        }

        intents
    }

    /// Process a single conclusion
    fn process_conclusion(&self, conclusion: &Conclusion, now: Timestamp) -> Option<Intent> {
        // Generate options
        let options = self.generator.generate(conclusion);
        if options.is_empty() {
            return None;
        }

        // Create ranking context
        let context = RankingContext {
            severity: conclusion.severity,
            confidence: conclusion.confidence,
            time_pressure: conclusion.severity >= Severity::Error,
            resources_available: true,
        };

        // Evaluate policies and rank
        let mut ranked: Vec<_> = options
            .into_iter()
            .map(|option| {
                let policy_result = self.policy_engine.evaluate(&option, conclusion.confidence);
                RankedOption {
                    option,
                    score: 0.0,
                    policy_result: Some(policy_result),
                }
            })
            .filter(|ro| {
                ro.policy_result
                    .as_ref()
                    .map(|pr| pr.allowed)
                    .unwrap_or(true)
            })
            .collect();

        // Rank options
        self.ranker.rank(&mut ranked, &context);

        // Detect and resolve conflicts
        let conflicts = self.resolver.detect_conflicts(&ranked);
        let _resolutions = self.resolver.resolve(&conflicts, &mut ranked);

        // Select best option
        let best = ranked.first()?;

        Some(Intent {
            id: IntentId::generate(),
            selected_option: best.option.clone(),
            score: best.score,
            confidence: conclusion.confidence,
            requires_confirmation: best
                .policy_result
                .as_ref()
                .map(|pr| pr.requires_confirmation)
                .unwrap_or(false),
            rate_limited: best
                .policy_result
                .as_ref()
                .map(|pr| pr.rate_limited)
                .unwrap_or(false),
            justification: format!(
                "Selected {:?} for {} (score: {:.2})",
                best.option.action_type, conclusion.summary, best.score
            ),
            timestamp: now,
            expires_at: Timestamp::new(now.as_nanos() + self.config.intent_ttl.as_nanos()),
            source_conclusion: Some(conclusion.id),
        })
    }

    /// Decide on a single conclusion immediately
    pub fn decide(&self, conclusion: &Conclusion, now: Timestamp) -> Option<Intent> {
        self.process_conclusion(conclusion, now)
    }

    /// Get domain statistics
    pub fn stats(&self) -> DecideStats {
        DecideStats {
            domain_id: self.id,
            is_running: self.running.load(Ordering::Relaxed),
            total_ticks: self.total_ticks.load(Ordering::Relaxed),
            total_decisions: self.total_decisions.load(Ordering::Relaxed),
            pending_conclusions: self.pending.len(),
            options_generated: self.generator.stats(),
            policy: self.policy_engine.stats(),
            conflict: self.resolver.stats(),
        }
    }
}

impl Default for DecideDomain {
    fn default() -> Self {
        Self::new(DecideConfig::default())
    }
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Decide domain statistics
#[derive(Debug, Clone)]
pub struct DecideStats {
    /// Domain ID
    pub domain_id: DomainId,
    /// Is running
    pub is_running: bool,
    /// Total ticks
    pub total_ticks: u64,
    /// Total decisions
    pub total_decisions: u64,
    /// Pending conclusions
    pub pending_conclusions: usize,
    /// Options generated
    pub options_generated: u64,
    /// Policy stats
    pub policy: PolicyStats,
    /// Conflict stats
    pub conflict: ConflictStats,
}

// ============================================================================
// ERRORS
// ============================================================================

/// Decide domain errors
#[derive(Debug)]
pub enum DecideError {
    /// Domain already running
    AlreadyRunning,
    /// Domain not running
    NotRunning,
    /// Invalid configuration
    InvalidConfig(String),
    /// Policy error
    PolicyError(String),
    /// Other error
    Other(String),
}

impl DecideError {
    /// Get error message
    pub fn message(&self) -> &str {
        match self {
            Self::AlreadyRunning => "Domain already running",
            Self::NotRunning => "Domain not running",
            Self::InvalidConfig(msg) => msg,
            Self::PolicyError(msg) => msg,
            Self::Other(msg) => msg,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::conclusion::ConclusionType;

    fn make_test_conclusion() -> Conclusion {
        Conclusion {
            id: ConclusionId::generate(),
            conclusion_type: ConclusionType::Diagnosis,
            severity: Severity::Error,
            confidence: Confidence::new(0.85),
            summary: String::from("Test conclusion"),
            explanation: String::from("This is a test"),
            evidence: Vec::new(),
            suggested_actions: Vec::new(),
            timestamp: Timestamp::now(),
            ttl: Duration::from_secs(60),
        }
    }

    #[test]
    fn test_decide_domain_creation() {
        let config = DecideConfig::default();
        let domain = DecideDomain::new(config);

        assert!(!domain.is_running());
        assert_eq!(domain.stats().total_decisions, 0);
    }

    #[test]
    fn test_decide_domain_lifecycle() {
        let mut domain = DecideDomain::default();

        assert!(domain.start().is_ok());
        assert!(domain.is_running());
        assert!(domain.start().is_err()); // Already running

        assert!(domain.stop().is_ok());
        assert!(!domain.is_running());
        assert!(domain.stop().is_err()); // Not running
    }

    #[test]
    fn test_submit_and_process() {
        let mut domain = DecideDomain::default();
        domain.start().unwrap();

        domain.submit(make_test_conclusion());
        assert_eq!(domain.pending_count(), 1);

        let intents = domain.tick(Timestamp::now());
        assert!(!intents.is_empty());
        assert_eq!(domain.pending_count(), 0);
    }

    #[test]
    fn test_decide_immediate() {
        let domain = DecideDomain::default();
        let conclusion = make_test_conclusion();

        let intent = domain.decide(&conclusion, Timestamp::now());
        assert!(intent.is_some());
    }
}
