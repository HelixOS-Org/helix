//! Sandbox capability types and permission levels

use alloc::string::String;

/// Sandbox capabilities that can be granted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxCapability {
    /// Can generate new code
    CodeGeneration,
    /// Can execute generated code
    CodeExecution,
    /// Can use genetic algorithms
    GeneticOptimization,
    /// Can evolve system parameters
    ParameterEvolution,
    /// Can modify running code
    SelfModification,
    /// Can modify core subsystems
    CoreModification,
}

/// Permission level for sandbox operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PermissionLevel {
    /// No access
    Denied   = 0,
    /// Read-only/simulation mode
    Simulate = 1,
    /// Limited modifications with rollback
    Limited  = 2,
    /// Full access (dangerous!)
    Full     = 3,
}

/// Sandbox operation errors
#[derive(Debug, Clone)]
pub enum SandboxError {
    /// Sandbox is disabled
    Disabled,
    /// Permission denied for operation
    PermissionDenied,
    /// Rate limit exceeded
    RateLimited,
    /// Operation failed
    OperationFailed(String),
    /// Validation failed
    ValidationFailed(String),
    /// Rollback required
    RollbackRequired,
}
