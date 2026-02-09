//! Act Domain — Orchestrator
//!
//! The ActDomain is the main orchestrator for the execution layer.
//! It coordinates validation, rate limiting, transaction management,
//! effector execution, and audit logging.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::audit::{AuditLogger, AuditStats};
use super::effect::{ActionOutcome, Effect};
use super::effector::EffectorRegistry;
use super::limiter::{RateLimiter, RateLimiterStats};
use super::transaction::{TransactionManager, TransactionStats};
use super::validator::{PreValidator, ValidationFailure, ValidatorStats};
use crate::types::*;
// Intent now comes from types::* above

// ============================================================================
// CONFIGURATION
// ============================================================================

/// Configuration for Act domain
#[derive(Debug, Clone)]
pub struct ActConfig {
    /// Enable transactions
    pub enable_transactions: bool,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Maximum actions per tick
    pub max_actions_per_tick: usize,
    /// Audit log size
    pub audit_log_size: usize,
}

impl Default for ActConfig {
    fn default() -> Self {
        Self {
            enable_transactions: true,
            enable_rate_limiting: true,
            max_actions_per_tick: 100,
            audit_log_size: 10000,
        }
    }
}

impl ActConfig {
    /// Create minimal configuration
    #[inline]
    pub fn minimal() -> Self {
        Self {
            enable_transactions: false,
            enable_rate_limiting: false,
            max_actions_per_tick: 10,
            audit_log_size: 1000,
        }
    }

    /// Create safe configuration
    #[inline]
    pub fn safe() -> Self {
        Self {
            enable_transactions: true,
            enable_rate_limiting: true,
            max_actions_per_tick: 50,
            audit_log_size: 50000,
        }
    }
}

// ============================================================================
// ACT DOMAIN
// ============================================================================

/// The Act domain - execution layer
pub struct ActDomain {
    /// Domain ID
    id: DomainId,
    /// Configuration
    config: ActConfig,
    /// Is running
    running: AtomicBool,
    /// Pre-validator
    validator: PreValidator,
    /// Rate limiter
    rate_limiter: RateLimiter,
    /// Transaction manager
    transactions: TransactionManager,
    /// Effector registry
    effectors: EffectorRegistry,
    /// Audit logger
    audit: AuditLogger,
    /// Pending intents
    pending: Vec<Intent>,
    /// Total ticks
    total_ticks: AtomicU64,
    /// Total actions
    total_actions: AtomicU64,
}

impl ActDomain {
    /// Create new Act domain
    pub fn new(config: ActConfig) -> Self {
        Self {
            id: DomainId::generate(),
            config: config.clone(),
            running: AtomicBool::new(false),
            validator: PreValidator::new(),
            rate_limiter: RateLimiter::new(),
            transactions: TransactionManager::new(),
            effectors: EffectorRegistry::with_defaults(),
            audit: AuditLogger::new(config.audit_log_size),
            pending: Vec::new(),
            total_ticks: AtomicU64::new(0),
            total_actions: AtomicU64::new(0),
        }
    }

    /// Get domain ID
    #[inline(always)]
    pub fn id(&self) -> DomainId {
        self.id
    }

    /// Is running?
    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Get configuration
    #[inline(always)]
    pub fn config(&self) -> &ActConfig {
        &self.config
    }

    /// Get validator
    #[inline(always)]
    pub fn validator(&self) -> &PreValidator {
        &self.validator
    }

    /// Get mutable validator
    #[inline(always)]
    pub fn validator_mut(&mut self) -> &mut PreValidator {
        &mut self.validator
    }

    /// Get audit logger
    #[inline(always)]
    pub fn audit(&self) -> &AuditLogger {
        &self.audit
    }

    /// Get effector registry
    #[inline(always)]
    pub fn effectors(&self) -> &EffectorRegistry {
        &self.effectors
    }

    /// Get mutable effector registry
    #[inline(always)]
    pub fn effectors_mut(&mut self) -> &mut EffectorRegistry {
        &mut self.effectors
    }

    /// Start the domain
    #[inline]
    pub fn start(&mut self) -> Result<(), ActError> {
        if self.running.load(Ordering::Acquire) {
            return Err(ActError::AlreadyRunning);
        }
        self.running.store(true, Ordering::Release);
        Ok(())
    }

    /// Stop the domain
    #[inline]
    pub fn stop(&mut self) -> Result<(), ActError> {
        if !self.running.load(Ordering::Acquire) {
            return Err(ActError::NotRunning);
        }
        self.running.store(false, Ordering::Release);
        Ok(())
    }

    /// Submit an intent for execution
    #[inline(always)]
    pub fn submit(&mut self, intent: Intent) {
        self.pending.push(intent);
    }

    /// Submit multiple intents
    #[inline(always)]
    pub fn submit_batch(&mut self, intents: Vec<Intent>) {
        self.pending.extend(intents);
    }

    /// Get pending count
    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Process one tick
    pub fn tick(&mut self, now: Timestamp) -> Vec<Effect> {
        if !self.running.load(Ordering::Acquire) {
            return Vec::new();
        }

        self.total_ticks.fetch_add(1, Ordering::Relaxed);

        let mut effects = Vec::new();
        let intents: Vec<_> = self.pending.drain(..).collect();

        for intent in intents {
            if let Some(effect) = self.execute_intent(&intent, now) {
                effects.push(effect);
                self.total_actions.fetch_add(1, Ordering::Relaxed);

                if effects.len() >= self.config.max_actions_per_tick {
                    break;
                }
            }
        }

        effects
    }

    /// Execute a single intent
    fn execute_intent(&mut self, intent: &Intent, now: Timestamp) -> Option<Effect> {
        let action_type = intent.action_type;
        let target = &intent.target;

        // Validate
        let validation = self.validator.validate(intent);
        if !validation.valid {
            return Some(Effect::rejected(
                intent.id,
                action_type,
                target.clone(),
                "Validation failed",
                now,
            ));
        }

        // Rate limit check
        if self.config.enable_rate_limiting {
            let rate_result = self.rate_limiter.check(action_type, target, now);
            if !rate_result.allowed {
                return Some(Effect::rejected(
                    intent.id,
                    action_type,
                    target.clone(),
                    "Rate limited",
                    now,
                ));
            }
        }

        // Find effector
        let effector_id = self.effectors.find(action_type, target)?;

        // Begin transaction if enabled
        let tx_id = if self.config.enable_transactions {
            Some(self.transactions.begin(intent.id, now))
        } else {
            None
        };

        // Execute
        let start_time = Timestamp::now();
        let effector_name;
        let result = {
            let effector = self.effectors.get_mut(effector_id)?;
            effector_name = String::from(effector.name());
            effector.execute(action_type, target, &intent.parameters, tx_id)
        };
        let end_time = Timestamp::now();

        // Record rate limit
        self.rate_limiter.record(action_type, target, now);

        // Handle transaction
        let rolled_back = if let Some(tx) = tx_id {
            if result.success {
                for change in &result.changes {
                    self.transactions.record_change(tx, change.clone());
                }
                let _ = self.transactions.commit(tx);
                false
            } else {
                let _ = self.transactions.rollback(tx);
                true
            }
        } else {
            false
        };

        // Create effect
        let effect = Effect {
            id: EffectId::generate(),
            intent_id: intent.id,
            action_type,
            target: target.clone(),
            outcome: if result.success {
                ActionOutcome::Success {
                    summary: format!("{:?} executed successfully", action_type),
                }
            } else {
                ActionOutcome::Failed {
                    error_code: ErrorCode::ExecutionFailed,
                    message: result.error.unwrap_or_default(),
                }
            },
            started_at: start_time,
            ended_at: end_time,
            duration: result.duration,
            transactional: tx_id.is_some(),
            rolled_back,
            changes: result.changes,
        };

        // Audit
        self.audit.log_effect(&effect, &effector_name);

        Some(effect)
    }

    /// Execute intent immediately
    #[inline(always)]
    pub fn execute(&mut self, intent: &Intent, now: Timestamp) -> Option<Effect> {
        self.execute_intent(intent, now)
    }

    /// Get domain statistics
    pub fn stats(&self) -> ActStats {
        ActStats {
            domain_id: self.id,
            is_running: self.running.load(Ordering::Relaxed),
            total_ticks: self.total_ticks.load(Ordering::Relaxed),
            total_actions: self.total_actions.load(Ordering::Relaxed),
            pending_intents: self.pending.len(),
            validator: self.validator.stats(),
            rate_limiter: self.rate_limiter.stats(),
            transactions: self.transactions.stats(),
            audit: self.audit.stats(),
            effectors_count: self.effectors.count(),
        }
    }
}

impl Default for ActDomain {
    fn default() -> Self {
        Self::new(ActConfig::default())
    }
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Act domain statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ActStats {
    /// Domain ID
    pub domain_id: DomainId,
    /// Is running
    pub is_running: bool,
    /// Total ticks
    pub total_ticks: u64,
    /// Total actions executed
    pub total_actions: u64,
    /// Pending intents
    pub pending_intents: usize,
    /// Validator stats
    pub validator: ValidatorStats,
    /// Rate limiter stats
    pub rate_limiter: RateLimiterStats,
    /// Transaction stats
    pub transactions: TransactionStats,
    /// Audit stats
    pub audit: AuditStats,
    /// Effector count
    pub effectors_count: usize,
}

// ============================================================================
// ERRORS
// ============================================================================

/// Act domain errors
#[derive(Debug)]
pub enum ActError {
    /// Domain already running
    AlreadyRunning,
    /// Domain not running
    NotRunning,
    /// Validation failed
    ValidationFailed(Vec<ValidationFailure>),
    /// Rate limited
    RateLimited(Duration),
    /// Effector not found
    EffectorNotFound,
    /// Execution failed
    ExecutionFailed(String),
    /// Other error
    Other(String),
}

impl ActError {
    /// Get error message
    #[inline]
    pub fn message(&self) -> &str {
        match self {
            Self::AlreadyRunning => "Domain already running",
            Self::NotRunning => "Domain not running",
            Self::ValidationFailed(_) => "Validation failed",
            Self::RateLimited(_) => "Rate limited",
            Self::EffectorNotFound => "Effector not found",
            Self::ExecutionFailed(msg) => msg,
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

    #[test]
    fn test_act_domain() {
        let config = ActConfig::default();
        let domain = ActDomain::new(config);

        assert!(!domain.is_running());
        assert!(domain.stats().effectors_count > 0);
    }

    #[test]
    fn test_act_domain_lifecycle() {
        let mut domain = ActDomain::default();

        assert!(domain.start().is_ok());
        assert!(domain.is_running());
        assert!(domain.start().is_err()); // Already running

        assert!(domain.stop().is_ok());
        assert!(!domain.is_running());
    }

    #[test]
    fn test_config_variants() {
        let minimal = ActConfig::minimal();
        assert!(!minimal.enable_transactions);

        let safe = ActConfig::safe();
        assert!(safe.enable_transactions);
    }
}
