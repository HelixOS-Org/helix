//! CLI Module
//!
//! Command-line interface for LUMINA development tools.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// CLI command
#[derive(Debug, Clone)]
pub enum Command {
    /// Create new project
    New(NewOptions),
    /// Build project
    Build(BuildOptions),
    /// Check/validate shaders
    Check(CheckOptions),
    /// Disassemble shader
    Disasm(DisasmOptions),
    /// Profile GPU
    Profile(ProfileOptions),
    /// Run application
    Run(RunOptions),
    /// Watch for changes
    Watch(WatchOptions),
    /// Optimize shaders
    Optimize(OptimizeOptions),
    /// Analyze cross-GPU
    Analyze(AnalyzeOptions),
    /// Print help
    Help,
    /// Print version
    Version,
}

/// New project options
#[derive(Debug, Clone, Default)]
pub struct NewOptions {
    /// Project name
    pub name: String,
    /// Template
    pub template: ProjectTemplate,
    /// Path
    pub path: Option<String>,
}

/// Project template
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProjectTemplate {
    #[default]
    Minimal,
    Triangle,
    Compute,
    RayTracing,
    Ui,
    Full3D,
}

/// Build options
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    /// Release mode
    pub release: bool,
    /// Target directory
    pub target_dir: Option<String>,
    /// Features
    pub features: Vec<String>,
    /// Verbose
    pub verbose: bool,
}

/// Check options
#[derive(Debug, Clone, Default)]
pub struct CheckOptions {
    /// Files to check
    pub files: Vec<String>,
    /// Strict mode
    pub strict: bool,
    /// Fix issues automatically
    pub fix: bool,
}

/// Disasm options
#[derive(Debug, Clone, Default)]
pub struct DisasmOptions {
    /// File
    pub file: String,
    /// Line
    pub line: Option<u32>,
    /// Format
    pub format: DisasmFormat,
}

/// Disasm format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisasmFormat {
    #[default]
    SpirV,
    SpirVText,
    Gcn,
    Ptx,
    Dxil,
}

/// Profile options
#[derive(Debug, Clone, Default)]
pub struct ProfileOptions {
    /// Profile GPU
    pub gpu: bool,
    /// Profile CPU
    pub cpu: bool,
    /// Frame count
    pub frames: u32,
    /// Output file
    pub output: Option<String>,
    /// AI suggestions
    pub ai_suggest: bool,
}

/// Run options
#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    /// Release mode
    pub release: bool,
    /// Hot reload
    pub hot_reload: bool,
    /// Arguments
    pub args: Vec<String>,
}

/// Watch options
#[derive(Debug, Clone, Default)]
pub struct WatchOptions {
    /// Paths to watch
    pub paths: Vec<String>,
    /// Command on change
    pub command: WatchCommand,
    /// Hot reload
    pub hot_reload: bool,
}

/// Watch command
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WatchCommand {
    #[default]
    Build,
    Run,
    Check,
}

/// Optimize options
#[derive(Debug, Clone, Default)]
pub struct OptimizeOptions {
    /// Files to optimize
    pub files: Vec<String>,
    /// Optimization level
    pub level: OptLevel,
    /// Auto-apply safe optimizations
    pub auto: bool,
    /// Target architecture
    pub target: Option<String>,
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptLevel {
    /// Size optimization
    Size,
    #[default]
    /// Speed optimization
    Speed,
    /// Aggressive optimization
    Aggressive,
}

/// Analyze options
#[derive(Debug, Clone, Default)]
pub struct AnalyzeOptions {
    /// Files to analyze
    pub files: Vec<String>,
    /// Target vendors
    pub vendors: Vec<String>,
    /// Generate performance matrix
    pub perf_matrix: bool,
}
