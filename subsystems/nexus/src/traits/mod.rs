//! NEXUS Traits Module
//!
//! Core trait definitions for all cognitive domains.
//!
//! # Module Organization
//!
//! ```text
//! traits/
//! ├── component.rs     - Base NexusComponent trait
//! ├── sensor.rs        - SENSE domain traits
//! ├── analyzer.rs      - UNDERSTAND domain traits
//! ├── reasoner.rs      - REASON domain traits
//! ├── decider.rs       - DECIDE domain traits
//! ├── effector.rs      - ACT domain traits
//! ├── memory.rs        - MEMORY domain traits
//! ├── introspection.rs - REFLECT domain traits
//! └── lifecycle.rs     - Lifecycle management traits
//! ```

#![allow(dead_code)]

// ============================================================================
// SUBMODULES
// ============================================================================

pub mod component;
pub mod sensor;
pub mod analyzer;
pub mod reasoner;
pub mod decider;
pub mod effector;
pub mod memory;
pub mod introspection;
pub mod lifecycle;

// ============================================================================
// RE-EXPORTS: Component
// ============================================================================

pub use component::{ComponentStatus, NexusComponent};

// ============================================================================
// RE-EXPORTS: Sensor (SENSE domain)
// ============================================================================

pub use sensor::{
    EventStream, Sensor, SensorMetadata, SensorType, SignalAggregator,
};

// ============================================================================
// RE-EXPORTS: Analyzer (UNDERSTAND domain)
// ============================================================================

pub use analyzer::{
    Analyzer, AnalyzerStats, AnomalyDetectorTrait, Classifier, FeatureExtractor,
    PatternDetector,
};

// ============================================================================
// RE-EXPORTS: Reasoner (REASON domain)
// ============================================================================

pub use reasoner::{
    CausalLink, CausalLinkType, CausalReasoner, Forecast, HypothesisGenerator,
    HypothesisResult, Reasoner, Seasonality, TemporalReasoner, TrendDirection,
    TrendInfo,
};

// ============================================================================
// RE-EXPORTS: Decider (DECIDE domain)
// ============================================================================

pub use decider::{
    Conflict, ConflictResolver, ConflictType, Decider, OptionGenerator, PolicyEngine,
    PolicyViolation, Resolution, ResolutionStrategy, ValidationResult,
};

// ============================================================================
// RE-EXPORTS: Effector (ACT domain)
// ============================================================================

pub use effector::{
    AuditEntry, AuditEntryType, AuditFilter, AuditLogger, Effector, RateLimit,
    TransactionManager, TransactionState,
};

// ============================================================================
// RE-EXPORTS: Memory (MEMORY domain)
// ============================================================================

pub use memory::{
    ConceptRelation, ConsolidationResult, EpisodicMemory, ExecutionFeedback,
    GcResult, MemoryManager, MemoryStats, MemoryStore, ProceduralMemory,
    RelationType, SemanticMemory, WorkingMemory,
};

// ============================================================================
// RE-EXPORTS: Introspection (REFLECT domain)
// ============================================================================

pub use introspection::{
    BiasReport, BiasType, CalibrationResult, CognitiveAssessment, DiagnosisFinding,
    DiagnosisReport, FindingType, HealthStatus, Introspectable, IntrospectionReport,
    MetaCognitiveMonitor,
};

// ============================================================================
// RE-EXPORTS: Lifecycle
// ============================================================================

pub use lifecycle::{
    Configurable, GracefulShutdown, Observable, Observer, Pausable, Resettable,
    Startable, Tickable,
};
