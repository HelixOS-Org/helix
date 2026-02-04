//! # Kernel Memory Intelligence for NEXUS
//!
//! AI-driven optimization of kernel memory subsystems.
//! Focuses on performance prediction, pattern analysis, and optimization.
//!
//! ## Components
//!
//! - `allocation`: Intelligent allocation advisor and fragmentation analysis
//! - `numa`: NUMA topology analysis and cross-node optimization
//! - `prefetch`: Memory prefetch prediction and optimization
//! - `hotpage`: Hot page detection and prioritization
//! - `cache`: Cache behavior analysis and optimization
//! - `pattern`: Memory access pattern recognition
//! - `buffer`: Buffer management optimization
//! - `intelligence`: Unified memory intelligence coordinator
//!
//! Note: Cognitive memory systems (episodic, semantic, procedural) are in `ltm/`

#![allow(dead_code)]

pub mod allocation;
pub mod buffer;
pub mod cache;
pub mod hotpage;
pub mod intelligence;
pub mod numa;
pub mod pattern;
pub mod prefetch;

// Re-exports
pub use allocation::AllocationIntelligence;
pub use hotpage::HotPageTracker;
pub use intelligence::MemoryIntelligence;
pub use numa::NumaAnalyzer;
pub use pattern::PatternRecognizer;
pub use prefetch::PrefetchPredictor;
