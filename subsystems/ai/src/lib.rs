//! # Helix AI - Kernel-Integrated Artificial Intelligence
//!
//! Revolutionary AI subsystem for autonomous operating system management.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         HELIX AI SUBSYSTEM                               │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐       │
//! │  │   Intent Engine  │  │  Neural Engine   │  │ Security Oracle  │       │
//! │  │                  │  │                  │  │                  │       │
//! │  │ • NLP Processing │  │ • Inference      │  │ • Threat Detect  │       │
//! │  │ • Goal Inference │  │ • Pattern Match  │  │ • Anomaly Score  │       │
//! │  │ • Context Track  │  │ • Decision Trees │  │ • Attack Predict │       │
//! │  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────────┘       │
//! │           │                     │                     │                  │
//! │           └─────────────────────┼─────────────────────┘                  │
//! │                                 ▼                                        │
//! │  ┌──────────────────────────────────────────────────────────────────┐   │
//! │  │                    CORTEX (Central AI Core)                       │   │
//! │  │                                                                   │   │
//! │  │  • Decision Fusion     • Priority Arbitration   • Action Queue   │   │
//! │  │  • State Management    • Policy Enforcement     • Rollback Mgmt  │   │
//! │  └───────────────────────────────┬──────────────────────────────────┘   │
//! │                                  │                                       │
//! │           ┌──────────────────────┼──────────────────────┐               │
//! │           ▼                      ▼                      ▼               │
//! │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐       │
//! │  │  Self-Optimizer  │  │   Self-Healer    │  │ Resource Oracle  │       │
//! │  │                  │  │                  │  │                  │       │
//! │  │ • Perf Analysis  │  │ • Bug Detection  │  │ • CPU Scheduling │       │
//! │  │ • Auto-Tuning    │  │ • Hot Patching   │  │ • Memory Predict │       │
//! │  │ • Workload Pred  │  │ • State Restore  │  │ • GPU/NPU Offload│       │
//! │  └──────────────────┘  └──────────────────┘  └──────────────────┘       │
//! │                                                                          │
//! │  ┌──────────────────────────────────────────────────────────────────┐   │
//! │  │                    MEMORY (Learning Substrate)                    │   │
//! │  │                                                                   │   │
//! │  │  • Usage Patterns      • Anomaly Database    • Decision History  │   │
//! │  │  • Performance Metrics • Threat Signatures   • Optimization Log  │   │
//! │  └──────────────────────────────────────────────────────────────────┘   │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Core Capabilities
//!
//! 1. **Intent Understanding** - Infers user goals from system interactions
//! 2. **Self-Optimization** - Autonomous kernel performance tuning
//! 3. **Self-Healing** - Automatic bug detection and repair
//! 4. **Predictive Security** - Proactive threat detection and mitigation
//! 5. **Resource Orchestration** - Intelligent CPU/GPU/NPU scheduling
//! 6. **Continuous Learning** - Adapts to usage patterns over time
//!
//! ## Safety Guarantees
//!
//! - All AI decisions are bounded by system invariants
//! - Critical operations require consensus from multiple AI components
//! - Full rollback capability for any AI-initiated change
//! - Rate limiting on autonomous actions
//! - Human override always available

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]

extern crate alloc;

// =============================================================================
// Math Utilities (libm wrappers for no_std)
// =============================================================================

/// Math functions for no_std environment using libm
pub mod math {
    //! Mathematical functions available in no_std through libm

    /// Exponential function for f32
    #[inline]
    pub fn exp_f32(x: f32) -> f32 {
        libm::expf(x)
    }

    /// Exponential function for f64
    #[inline]
    pub fn exp_f64(x: f64) -> f64 {
        libm::exp(x)
    }

    /// Square root for f32
    #[inline]
    pub fn sqrt_f32(x: f32) -> f32 {
        libm::sqrtf(x)
    }

    /// Square root for f64
    #[inline]
    pub fn sqrt_f64(x: f64) -> f64 {
        libm::sqrt(x)
    }

    /// Natural logarithm of (1 + x) for f32
    #[inline]
    pub fn ln_1p_f32(x: f32) -> f32 {
        libm::log1pf(x)
    }

    /// Natural logarithm of (1 + x) for f64
    #[inline]
    pub fn ln_1p_f64(x: f64) -> f64 {
        libm::log1p(x)
    }

    /// Power function (x^n) for f32
    #[inline]
    pub fn powi_f32(x: f32, n: i32) -> f32 {
        libm::powf(x, n as f32)
    }

    /// Power function (x^n) for f64
    #[inline]
    pub fn powi_f64(x: f64, n: i32) -> f64 {
        libm::pow(x, n as f64)
    }

    /// Natural logarithm for f32
    #[inline]
    pub fn ln_f32(x: f32) -> f32 {
        libm::logf(x)
    }

    /// Natural logarithm for f64
    #[inline]
    pub fn ln_f64(x: f64) -> f64 {
        libm::log(x)
    }

    /// Absolute value for f32
    #[inline]
    pub fn abs_f32(x: f32) -> f32 {
        libm::fabsf(x)
    }

    /// Absolute value for f64
    #[inline]
    pub fn abs_f64(x: f64) -> f64 {
        libm::fabs(x)
    }

    /// Hyperbolic tangent for f32
    #[inline]
    pub fn tanh_f32(x: f32) -> f32 {
        libm::tanhf(x)
    }

    /// Maximum of two f32 values
    #[inline]
    pub fn max_f32(a: f32, b: f32) -> f32 {
        libm::fmaxf(a, b)
    }

    /// Minimum of two f32 values
    #[inline]
    pub fn min_f32(a: f32, b: f32) -> f32 {
        libm::fminf(a, b)
    }

    /// Floor for f32
    #[inline]
    pub fn floor_f32(x: f32) -> f32 {
        libm::floorf(x)
    }

    /// Ceiling for f32
    #[inline]
    pub fn ceil_f32(x: f32) -> f32 {
        libm::ceilf(x)
    }
}

// =============================================================================
// Module Declarations
// =============================================================================

/// Core AI types and traits
pub mod core;

/// Central AI processing cortex
pub mod cortex;

/// Intent recognition and goal inference
pub mod intent;

/// Neural network inference engine
pub mod neural;

/// Self-optimization subsystem
pub mod optimizer;

/// Self-healing and bug repair
pub mod healer;

/// Predictive security oracle
pub mod security;

/// Resource orchestration (CPU/GPU/NPU)
pub mod resources;

/// Continuous learning and adaptation
pub mod learning;

/// AI memory and pattern storage
pub mod memory;

/// Metrics and telemetry
pub mod metrics;

/// Safety constraints and invariants
pub mod safety;

/// Test suite
#[cfg(test)]
mod tests;

// =============================================================================
// Public API Re-exports
// =============================================================================

pub use core::{
    AiAction, AiConfig, AiDecision, AiError, AiEvent, AiPriority, AiState, Confidence,
    DecisionContext, DecisionId, PowerProfile, ResourceType, SafetyLevel,
};

pub use cortex::Cortex;
pub use healer::{BugSignature, Healer, HealingAction, HotPatch};
pub use intent::{Intent, IntentClass, IntentEngine, UserGoal};
pub use learning::{Experience, LearningEngine, Pattern, PatternType};
pub use memory::{AiMemory, MemoryEntry, MemoryId, MemoryUsage};
pub use metrics::{MetricDefinition, MetricId, MetricsCollector, MetricsSummary, TimeSeries};
pub use neural::{NeuralEngine, NeuralModel, Tensor, TensorShape};
pub use optimizer::{OptimizationHint, Optimizer, PerformanceProfile, WorkloadAnalysis};
pub use resources::{
    ComputeDevice, DeviceType, ResourceAllocation, ResourceOracle, WorkloadProfile,
};
pub use safety::{Invariant, RiskAssessment, SafetyCheckResult, SafetyChecker, SafetyConstraint};
pub use security::{SecurityOracle, Threat, ThreatLevel, ThreatPrediction, ThreatType};
// =============================================================================
// Global AI Instance
// =============================================================================
use spin::Once;

/// Type alias for AI results
pub type AiResult<T> = Result<T, AiError>;

/// Global Helix AI instance
static HELIX_AI: Once<Cortex> = Once::new();

/// Initialize the Helix AI subsystem
///
/// This must be called early in kernel initialization, after memory
/// management is available but before user-space processes start.
///
/// # Panics
///
/// Panics if called more than once.
pub fn init(config: AiConfig) -> AiResult<()> {
    log::info!("[HELIX-AI] Initializing kernel AI subsystem...");

    let cortex = Cortex::new(config);

    HELIX_AI.call_once(|| cortex);

    log::info!("[HELIX-AI] AI subsystem initialized successfully");
    Ok(())
}

/// Get a reference to the global AI cortex
///
/// # Panics
///
/// Panics if the AI subsystem has not been initialized.
pub fn cortex() -> &'static Cortex {
    HELIX_AI
        .get()
        .expect("Helix AI not initialized. Call helix_ai::init() first.")
}

/// Check if the AI subsystem is initialized
pub fn is_initialized() -> bool {
    HELIX_AI.get().is_some()
}

// =============================================================================
// Convenience Macros
// =============================================================================

/// Log an AI decision for auditing
#[macro_export]
macro_rules! ai_log {
    ($level:expr, $($arg:tt)*) => {
        if cfg!(feature = "ai-tracing") {
            log::log!($level, "[HELIX-AI] {}", format_args!($($arg)*));
        }
    };
}

/// Assert an AI safety invariant
#[macro_export]
macro_rules! ai_invariant {
    ($cond:expr, $msg:expr) => {
        if !$cond {
            $crate::safety::violation($msg);
        }
    };
}

/// Measure AI operation timing
#[macro_export]
macro_rules! ai_measure {
    ($name:expr, $block:block) => {{
        let start = $crate::metrics::timestamp();
        let result = $block;
        let elapsed = $crate::metrics::timestamp() - start;
        $crate::metrics::record($name, elapsed);
        result
    }};
}
