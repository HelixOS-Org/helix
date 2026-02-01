//! NEXUS Act Domain — Execution Layer
//!
//! The fifth cognitive domain. ACT executes decisions in a controlled manner.
//! It receives intents from DECIDE and translates them into actual kernel actions,
//! with safety validation, transaction support, and full audit logging.
//!
//! # Philosophy
//!
//! "Agir de manière contrôlée" — Act in a controlled manner
//!
//! ACT is the only domain that modifies kernel state. It must:
//! - Validate all actions before execution
//! - Execute with transaction semantics (rollback on failure)
//! - Rate limit to prevent oscillation
//! - Audit every action for traceability
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                            ACT DOMAIN                                    │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    FROM DECIDE DOMAIN                        │       │
//! │  │  (Intents: chosen actions with justification)                │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    PRE-VALIDATOR                             │       │
//! │  │  • Check preconditions, permissions, parameters              │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    RATE LIMITER                              │       │
//! │  │  • Prevent oscillation, enforce cooldowns                    │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                   TRANSACTION MANAGER                        │       │
//! │  │  • Begin/commit/rollback with state capture                  │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                      EFFECTORS                               │       │
//! │  │  Process | Memory | Module | NoOp | ...                      │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    AUDIT LOGGER                              │       │
//! │  │  • Record all actions for traceability                       │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    OUTPUT EFFECTS                            │       │
//! │  │  → Effect → Effect → ... (To MEMORY, REFLECT)               │       │
//! │  └──────────────────────────────────────────────────────────────┘       │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Structure
//!
//! - [`effect`] - Effect output types, changes, outcomes
//! - [`validator`] - Pre-execution validation
//! - [`limiter`] - Rate limiting and cooldowns
//! - [`transaction`] - Transaction management and rollback
//! - [`effector`] - Effector trait and registry
//! - [`effectors`] - Concrete effector implementations
//! - [`audit`] - Audit logging
//! - [`domain`] - Main orchestrator

#![allow(dead_code)]

// Submodules
pub mod audit;
pub mod domain;
pub mod effect;
pub mod effector;
pub mod effectors;
pub mod limiter;
pub mod transaction;
pub mod validator;

// Re-exports - Effect
// Re-exports - Audit
pub use audit::{AuditEntry, AuditLogger, AuditOutcome, AuditStats};
// Re-exports - Domain
pub use domain::{ActConfig, ActDomain, ActError, ActStats};
pub use effect::{ActionOutcome, Change, ChangeType, ChangeValue, Effect};
// Re-exports - Effector
pub use effector::{Effector, EffectorRegistry, EffectorResult};
// Re-exports - Effectors
pub use effectors::{MemoryEffector, ModuleEffector, NoOpEffector, ProcessEffector};
// Re-exports - Limiter
pub use limiter::{
    RateLimit, RateLimitReason, RateLimitResult, RateLimiter, RateLimiterStats, target_to_string,
};
// Re-exports - Transaction
pub use transaction::{
    CapturedValue, RollbackState, Transaction, TransactionError, TransactionId, TransactionManager,
    TransactionStats, TransactionStatus,
};
// Re-exports - Validator
pub use validator::{
    PreValidator, ValidationCheck, ValidationFailure, ValidationResult, ValidationRule,
    ValidatorStats,
};

pub use crate::types::EffectorId;
// Re-export from types (for backwards compatibility)
pub use crate::types::{ChangeId, EffectId};
