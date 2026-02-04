//! Sandbox audit event types

use alloc::string::String;

use super::{PermissionLevel, SandboxCapability};

/// Sandbox audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Timestamp
    pub timestamp: u64,
    /// Capability used
    pub capability: SandboxCapability,
    /// Permission level at time of operation
    pub permission: PermissionLevel,
    /// Description of operation
    pub description: String,
    /// Success/failure
    pub success: bool,
    /// Rollback available
    pub rollback_available: bool,
}
