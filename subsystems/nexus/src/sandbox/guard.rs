//! Permission guard for sandbox operations

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{AuditEvent, PermissionLevel, SandboxCapability, SandboxError};

/// Global sandbox enable flag - MUST be explicitly enabled
pub static SANDBOX_ENABLED: AtomicBool = AtomicBool::new(false);

/// Global operation counter for rate limiting
pub static OPERATION_COUNT: AtomicU64 = AtomicU64::new(0);

/// Maximum operations per time window
pub const MAX_OPERATIONS_PER_WINDOW: u64 = 100;

/// Permission guard for sandbox operations
pub struct SandboxGuard {
    /// Granted capabilities
    pub(crate) capabilities: Vec<(SandboxCapability, PermissionLevel)>,
    /// Audit log
    pub(crate) audit_log: Vec<AuditEvent>,
    /// Is sandbox enabled
    pub enabled: bool,
    /// Rate limit remaining
    pub rate_limit_remaining: u64,
}

impl SandboxGuard {
    /// Create a new sandbox guard (disabled by default)
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
            audit_log: Vec::with_capacity(1000),
            enabled: false,
            rate_limit_remaining: MAX_OPERATIONS_PER_WINDOW,
        }
    }

    /// Enable the sandbox (requires explicit call)
    ///
    /// # Safety
    /// Enabling the sandbox allows dangerous operations.
    pub unsafe fn enable(&mut self) {
        self.enabled = true;
        SANDBOX_ENABLED.store(true, Ordering::SeqCst);
    }

    /// Disable the sandbox
    pub fn disable(&mut self) {
        self.enabled = false;
        SANDBOX_ENABLED.store(false, Ordering::SeqCst);
    }

    /// Grant a capability with a permission level
    pub fn grant(&mut self, capability: SandboxCapability, level: PermissionLevel) {
        // Remove existing grant for this capability
        self.capabilities.retain(|(c, _)| *c != capability);
        self.capabilities.push((capability, level));
    }

    /// Revoke a capability
    pub fn revoke(&mut self, capability: SandboxCapability) {
        self.capabilities.retain(|(c, _)| *c != capability);
    }

    /// Check if operation is permitted
    pub fn check(&self, capability: SandboxCapability, required_level: PermissionLevel) -> bool {
        if !self.enabled {
            return false;
        }

        self.capabilities
            .iter()
            .find(|(c, _)| *c == capability)
            .map(|(_, level)| *level >= required_level)
            .unwrap_or(false)
    }

    /// Execute an operation with permission check
    pub fn execute<F, R>(
        &mut self,
        capability: SandboxCapability,
        required_level: PermissionLevel,
        description: String,
        operation: F,
    ) -> Result<R, SandboxError>
    where
        F: FnOnce() -> Result<R, SandboxError>,
    {
        // Check enabled
        if !self.enabled {
            return Err(SandboxError::Disabled);
        }

        // Check rate limit
        if self.rate_limit_remaining == 0 {
            return Err(SandboxError::RateLimited);
        }

        // Check permission
        if !self.check(capability, required_level) {
            self.audit(capability, required_level, description, false, false);
            return Err(SandboxError::PermissionDenied);
        }

        // Decrement rate limit
        self.rate_limit_remaining -= 1;
        OPERATION_COUNT.fetch_add(1, Ordering::SeqCst);

        // Execute operation
        let result = operation();
        let success = result.is_ok();

        // Audit
        self.audit(capability, required_level, description, success, true);

        result
    }

    /// Record an audit event
    fn audit(
        &mut self,
        capability: SandboxCapability,
        permission: PermissionLevel,
        description: String,
        success: bool,
        rollback_available: bool,
    ) {
        let event = AuditEvent {
            timestamp: self.get_timestamp(),
            capability,
            permission,
            description,
            success,
            rollback_available,
        };

        if self.audit_log.len() >= 10000 {
            self.audit_log.drain(0..5000);
        }
        self.audit_log.push(event);
    }

    /// Get recent audit events
    pub fn recent_events(&self, count: usize) -> &[AuditEvent] {
        let start = self.audit_log.len().saturating_sub(count);
        &self.audit_log[start..]
    }

    /// Reset rate limit (call periodically)
    pub fn reset_rate_limit(&mut self) {
        self.rate_limit_remaining = MAX_OPERATIONS_PER_WINDOW;
    }

    fn get_timestamp(&self) -> u64 {
        OPERATION_COUNT.load(Ordering::Relaxed)
    }
}

impl Default for SandboxGuard {
    fn default() -> Self {
        Self::new()
    }
}
