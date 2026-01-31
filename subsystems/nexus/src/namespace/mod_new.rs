//! Namespace Intelligence Module
//!
//! Comprehensive namespace analysis and management.
//!
//! ## Architecture
//!
//! - `types` - Core types: NamespaceId, ProcessId, UserId, GroupId, NamespaceType, NamespaceState
//! - `info` - Namespace info: NamespaceInfo, IdMapping, UserNamespaceInfo, PidNamespaceInfo, NetNamespaceInfo
//! - `isolation` - Isolation analysis: IsolationLevel, IsolationAnalysis, IsolationAnalyzer
//! - `security` - Security enforcement: BoundaryViolation, ViolationType, SecurityEnforcer
//! - `manager` - Namespace lifecycle: NamespaceOptions, NamespaceManager
//! - `analysis` - Analysis results: NamespaceAnalysis, NamespaceIssue, NamespaceRecommendation
//! - `intelligence` - Central coordinator: NamespaceIntelligence

pub mod analysis;
pub mod info;
pub mod intelligence;
pub mod isolation;
pub mod manager;
pub mod security;
pub mod types;

// Re-export types
pub use types::{GroupId, NamespaceId, NamespaceState, NamespaceType, ProcessId, UserId};

// Re-export info
pub use info::{
    IdMapping, NamespaceInfo, NetNamespaceInfo, PidNamespaceInfo, UserNamespaceInfo,
};

// Re-export isolation
pub use isolation::{IsolationAnalysis, IsolationAnalyzer, IsolationLevel};

// Re-export security
pub use security::{BoundaryViolation, SecurityEnforcer, ViolationType};

// Re-export manager
pub use manager::{NamespaceManager, NamespaceOptions};

// Re-export analysis
pub use analysis::{
    NamespaceAction, NamespaceAnalysis, NamespaceIssue, NamespaceIssueType,
    NamespaceRecommendation,
};

// Re-export intelligence
pub use intelligence::NamespaceIntelligence;
