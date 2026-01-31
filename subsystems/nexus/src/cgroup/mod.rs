//! Cgroup Subsystem
//!
//! Comprehensive cgroup management with AI-powered intelligence.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  CgroupIntelligence                        │
//! │  ┌─────────────┬───────────────┬──────────────────────┐    │
//! │  │  Hierarchy  │  Accountant   │     Enforcer         │    │
//! │  │  Manager    │               │                      │    │
//! │  └─────────────┴───────────────┴──────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - `types` - Core type definitions (CgroupId, CgroupVersion, ControllerType)
//! - `limits` - Resource limits (CPU, Memory, I/O, PIDs)
//! - `usage` - Resource usage statistics
//! - `info` - Cgroup information and metadata
//! - `hierarchy` - Hierarchy management and process placement
//! - `accountant` - Resource accounting and rate calculation
//! - `enforcer` - Limits enforcement and OOM handling
//! - `intelligence` - AI-powered analysis and optimization

mod accountant;
mod enforcer;
mod hierarchy;
mod info;
mod intelligence;
mod limits;
mod types;
mod usage;

// Core types
pub use types::{CgroupId, CgroupState, CgroupVersion, ControllerType, ProcessId};

// Resource limits
pub use limits::{CpuLimits, IoLimits, MemoryLimits, PidsLimits};

// Resource usage
pub use usage::{CpuUsage, IoUsage, MemoryPressure, MemoryUsage};

// Cgroup information
pub use info::CgroupInfo;

// Hierarchy management
pub use hierarchy::{HierarchyManager, HierarchyStats};

// Resource accounting
pub use accountant::{ResourceAccountant, ResourceSample};

// Limits enforcement
pub use enforcer::{EnforcementAction, EnforcementResult, LimitsEnforcer};

// Intelligence
pub use intelligence::{
    CgroupAction, CgroupAnalysis, CgroupIntelligence, CgroupIssue, CgroupIssueType,
    CgroupRecommendation,
};
