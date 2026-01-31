//! Kprobe Intelligence Module
//!
//! AI-powered dynamic kernel probing capabilities.
//!
//! ## Architecture
//!
//! - `types` - Core types: KprobeId, KretprobeId, ProbeAddress, SymbolInfo, KprobeState, Architecture
//! - `instruction` - Instruction analysis: InstructionType, InstructionInfo, InstructionAnalyzer
//! - `context` - Execution context: KprobeContext
//! - `definition` - Probe definitions: KprobeDef, KretprobeDef
//! - `tracer` - Function tracing: FunctionCall, FunctionStats, FunctionTracer
//! - `manager` - Kprobe lifecycle: KprobeManager
//! - `analysis` - Analysis results: KprobeAnalysis, KprobeIssue, KprobeRecommendation
//! - `intelligence` - Central coordinator: KprobeIntelligence

pub mod analysis;
pub mod context;
pub mod definition;
pub mod instruction;
pub mod intelligence;
pub mod manager;
pub mod tracer;
pub mod types;

// Re-export types
pub use types::{Architecture, KprobeId, KprobeState, KretprobeId, ProbeAddress, SymbolInfo};

// Re-export instruction
pub use instruction::{InstructionAnalyzer, InstructionInfo, InstructionType};

// Re-export context
pub use context::KprobeContext;

// Re-export definition
pub use definition::{KprobeDef, KretprobeDef};

// Re-export tracer
pub use tracer::{FunctionCall, FunctionStats, FunctionTracer};

// Re-export manager
pub use manager::KprobeManager;

// Re-export analysis
pub use analysis::{
    KprobeAction, KprobeAnalysis, KprobeIssue, KprobeIssueType, KprobeRecommendation,
};

// Re-export intelligence
pub use intelligence::KprobeIntelligence;
