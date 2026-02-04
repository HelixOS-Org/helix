//! # Sandbox for Dangerous Year 3 "EVOLUTION" Modules
//!
//! This module provides a controlled, sandboxed environment for potentially
//! dangerous self-modification and code generation capabilities.
//!
//! ## CRITICAL SAFETY NOTICE
//!
//! The modules in this sandbox can modify the kernel's own code and behavior
//! at runtime. They MUST be:
//! - Audited before any production use
//! - Protected by multiple safety layers
//! - Rate-limited and monitored
//! - Disabled by default in production builds
//!
//! ## Sandboxed Capabilities
//!
//! - `codegen`: Runtime code generation (JIT, bytecode compilation)
//! - `genetic`: Genetic/evolutionary algorithms for optimization
//! - `selfmod`: Self-modification capabilities
//!
//! ## Safety Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    SANDBOX BOUNDARY                         │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │                 PERMISSION GUARD                       │  │
//! │  │    Checks capabilities before any operation           │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! │                           │                                 │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
//! │  │  codegen/   │ │  genetic/   │ │     selfmod/        │   │
//! │  │  JIT, etc   │ │  Evolution  │ │  Self-modification  │   │
//! │  └─────────────┘ └─────────────┘ └─────────────────────┘   │
//! │                           │                                 │
//! │  ┌───────────────────────────────────────────────────────┐  │
//! │  │               AUDIT TRAIL                              │  │
//! │  │    Logs every modification attempt                    │  │
//! │  └───────────────────────────────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]

extern crate alloc;

// ============================================================================
// SUBMODULES
// ============================================================================

mod audit;
mod guard;
mod types;
mod wrappers;

pub use audit::AuditEvent;
pub use guard::{MAX_OPERATIONS_PER_WINDOW, OPERATION_COUNT, SANDBOX_ENABLED, SandboxGuard};
pub use types::{PermissionLevel, SandboxCapability, SandboxError};
pub use wrappers::{safe_codegen, safe_genetic, safe_selfmod};

// Re-export sandboxed modules with safety wrappers
pub use crate::codegen as codegen_unsafe;
pub use crate::{genetic as genetic_unsafe, selfmod as selfmod_unsafe};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_disabled_by_default() {
        let guard = SandboxGuard::new();
        assert!(!guard.enabled);
        assert!(!guard.check(SandboxCapability::CodeGeneration, PermissionLevel::Limited));
    }

    #[test]
    fn test_sandbox_permission_levels() {
        let mut guard = SandboxGuard::new();
        unsafe {
            guard.enable();
        }

        guard.grant(SandboxCapability::CodeGeneration, PermissionLevel::Simulate);

        // Simulate level granted
        assert!(guard.check(SandboxCapability::CodeGeneration, PermissionLevel::Simulate));
        // Limited level NOT granted (higher than Simulate)
        assert!(!guard.check(SandboxCapability::CodeGeneration, PermissionLevel::Limited));
    }

    #[test]
    fn test_rate_limiting() {
        let mut guard = SandboxGuard::new();
        unsafe {
            guard.enable();
        }
        guard.grant(SandboxCapability::CodeGeneration, PermissionLevel::Full);

        // Exhaust rate limit
        guard.rate_limit_remaining = 1;

        let result1 = guard.execute(
            SandboxCapability::CodeGeneration,
            PermissionLevel::Limited,
            alloc::string::String::from("test1"),
            || Ok(()),
        );
        assert!(result1.is_ok());

        let result2 = guard.execute(
            SandboxCapability::CodeGeneration,
            PermissionLevel::Limited,
            alloc::string::String::from("test2"),
            || Ok(()),
        );
        assert!(matches!(result2, Err(SandboxError::RateLimited)));
    }
}
