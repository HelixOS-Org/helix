//! Seccomp Subsystem
//!
//! AI-powered syscall filtering and security analysis.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  SeccompIntelligence                       │
//! │  ┌─────────────┬───────────────┬──────────────────────┐    │
//! │  │  Manager    │   Profiler    │  AttackSurfaceAnalyzer│    │
//! │  └─────────────┴───────────────┴──────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - `types` - Core types (FilterId, ProfileId, Pid, SeccompMode, FilterAction, Architecture)
//! - `syscall` - Syscall definitions (SyscallNum, SyscallCategory, RiskLevel, SyscallInfo)
//! - `bpf` - BPF instruction definitions
//! - `filter` - Seccomp filter implementation
//! - `profile` - Syscall usage profiles
//! - `profiler` - Active syscall profiling
//! - `attack_surface` - Attack surface analysis
//! - `manager` - Filter management
//! - `intelligence` - AI-powered security analysis

mod attack_surface;
mod bpf;
mod filter;
mod intelligence;
mod manager;
mod profile;
mod profiler;
mod syscall;
mod types;

// Core types
pub use types::{Architecture, FilterAction, FilterId, Pid, ProfileId, SeccompMode};

// Syscall definitions
pub use syscall::{RiskLevel, SyscallCategory, SyscallInfo, SyscallNum};

// BPF instructions
pub use bpf::BpfInsn;

// Filter
pub use filter::SeccompFilter;

// Profiling
pub use profile::{SyscallProfile, SyscallStats};
pub use profiler::SyscallProfiler;

// Attack surface
pub use attack_surface::{AttackSurfaceAnalysis, AttackSurfaceAnalyzer};

// Manager
pub use manager::SeccompManager;

// Intelligence
pub use intelligence::{
    SeccompAction, SeccompAnalysis, SeccompIntelligence, SeccompIssue, SeccompIssueType,
    SeccompRecommendation,
};
