//! BPF Intelligence Module
//!
//! This module provides AI-powered eBPF program analysis and optimization for the NEXUS subsystem.
//! It includes BPF program verification, map management, JIT compilation optimization,
//! helper function tracking, and intelligent program lifecycle management.
//!
//! # Architecture
//!
//! The BPF module is organized into:
//! - **Types**: Core identifiers and type enums (program types, map types, attach types)
//! - **Instruction**: BPF bytecode instruction representation
//! - **Program**: BPF program state and metadata
//! - **Map**: BPF map structures and flags
//! - **Verifier**: Program verification engine
//! - **JIT**: Just-in-time compilation
//! - **Helpers**: Helper function tracking
//! - **Manager**: Central program and map management
//! - **Intelligence**: AI-powered analysis and optimization
//!
//! # Usage
//!
//! ```rust,ignore
//! use nexus::bpf::{BpfIntelligence, BpfInsn, BpfProgType, BpfMapType};
//!
//! let mut intel = BpfIntelligence::new();
//!
//! // Load a program
//! let insns = vec![
//!     BpfInsn::new(0xB7, 0, 0, 0, 2), // MOV r0, XDP_PASS
//!     BpfInsn::new(0x95, 0, 0, 0, 0), // EXIT
//! ];
//! let prog_id = intel.load_program("xdp_pass".into(), BpfProgType::Xdp, &insns, timestamp)?;
//!
//! // Create a map
//! let map_id = intel.create_map("my_map".into(), BpfMapType::Hash, 4, 8, 1024, timestamp);
//!
//! // Analyze
//! let analysis = intel.analyze_program(prog_id);
//! ```

extern crate alloc;

// Submodules
pub mod helpers;
pub mod instruction;
pub mod intelligence;
pub mod jit;
pub mod manager;
pub mod map;
pub mod program;
pub mod types;
pub mod verifier;

// Re-export all public types
pub use helpers::{BpfHelperId, BpfHelperInfo};
pub use instruction::{BpfInsn, BpfOpcode};
pub use intelligence::{
    BpfAction, BpfAnalysis, BpfIntelligence, BpfIssue, BpfIssueType, BpfRecommendation,
};
pub use jit::{BpfJit, JitResult, JitStats};
pub use manager::BpfManager;
pub use map::{BpfMapFlags, BpfMapInfo};
pub use program::{BpfProgInfo, BpfProgState};
pub use types::{BpfAttachType, BpfMapId, BpfMapType, BpfProgId, BpfProgType, BtfId};
pub use verifier::{BpfVerifier, VerificationResult};
