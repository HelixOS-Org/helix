//! LUMINA Tools - Revolutionary Development Ecosystem
//!
//! The most advanced GPU development toolkit ever created.
//!
//! # Revolutionary Features
//!
//! - **AI-Assisted Development**: Intelligent shader optimization suggestions
//! - **Live GPU Profiling**: Real-time performance analysis without frame drops
//! - **Hot-Reload Everything**: Shaders, pipelines, assets - all hot-reloadable
//! - **Time-Travel Debugging**: Replay and inspect any frame from history
//! - **Predictive Compilation**: Pre-compile shader variants before needed
//! - **Cross-Platform Analysis**: Compare GPU behavior across vendors
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                     LUMINA TOOLS ECOSYSTEM                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌───────────────────────────────────────────────────────────────────┐ │
//! │  │                      lumina-cli                                    │ │
//! │  │  $ lumina new my-app --template pbr                               │ │
//! │  │  $ lumina build --target vulkan,metal,dx12                        │ │
//! │  │  $ lumina watch --hot-reload                                      │ │
//! │  │  $ lumina profile --ai-suggest                                    │ │
//! │  │  $ lumina optimize --auto                                         │ │
//! │  └───────────────────────────────────────────────────────────────────┘ │
//! │                                                                         │
//! │  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────┐   │
//! │  │   Hot Reload    │ │   AI Optimizer  │ │   Cross-GPU Analyzer    │   │
//! │  │   Engine        │ │                 │ │                         │   │
//! │  │ • Shader swap   │ │ • Pattern detect│ │ • Vendor comparison     │   │
//! │  │ • Pipeline edit │ │ • Auto optimize │ │ • Performance matrix    │   │
//! │  │ • Asset stream  │ │ • Bug predict   │ │ • Compatibility check   │   │
//! │  └─────────────────┘ └─────────────────┘ └─────────────────────────┘   │
//! │                                                                         │
//! │  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────────┐   │
//! │  │  Time-Travel    │ │   Live Metrics  │ │   Predictive Cache      │   │
//! │  │  Debugger       │ │                 │ │                         │   │
//! │  │ • Frame history │ │ • Zero-overhead │ │ • Usage prediction      │   │
//! │  │ • State replay  │ │ • GPU counters  │ │ • Pre-compilation       │   │
//! │  │ • Diff analysis │ │ • Bottleneck AI │ │ • Variant pruning       │   │
//! │  └─────────────────┘ └─────────────────┘ └─────────────────────────┘   │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![no_std]
#![allow(dead_code)]

extern crate alloc;

pub mod cli;
pub mod hot_reload;
pub mod ai_optimizer;
pub mod time_travel;
pub mod live_metrics;
pub mod predictive;
pub mod cross_gpu;
pub mod project;

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Core Types
// ============================================================================

/// Tools result type
pub type ToolsResult<T> = Result<T, ToolsError>;

/// Tools error
#[derive(Debug, Clone)]
pub struct ToolsError {
    /// Error kind
    pub kind: ToolsErrorKind,
    /// Error message
    pub message: String,
    /// Context chain
    pub context: Vec<String>,
}

/// Error kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolsErrorKind {
    /// Configuration error
    Config,
    /// Build error
    Build,
    /// Shader error
    Shader,
    /// Profile error
    Profile,
    /// Hot reload error
    HotReload,
    /// AI error
    Ai,
    /// IO error
    Io,
    /// Network error
    Network,
    /// Internal error
    Internal,
}

impl ToolsError {
    /// Create new error
    pub fn new(kind: ToolsErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: Vec::new(),
        }
    }

    /// Add context
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }
}

// ============================================================================
// Version Info
// ============================================================================

/// LUMINA Tools version
pub const VERSION: &str = "0.1.0";

/// LUMINA Tools build info
#[derive(Debug, Clone)]
pub struct BuildInfo {
    /// Version string
    pub version: &'static str,
    /// Git commit hash
    pub git_hash: Option<&'static str>,
    /// Build timestamp
    pub build_time: Option<&'static str>,
    /// Target triple
    pub target: &'static str,
    /// Features enabled
    pub features: &'static [&'static str],
}

impl BuildInfo {
    /// Get build info
    pub const fn current() -> Self {
        Self {
            version: VERSION,
            git_hash: None,
            build_time: None,
            target: "x86_64-unknown-none",
            features: &[],
        }
    }
}

// ============================================================================
// Prelude
// ============================================================================

/// Prelude for common imports
pub mod prelude {
    pub use crate::{
        BuildInfo, ToolsError, ToolsErrorKind, ToolsResult, VERSION,
    };
}
