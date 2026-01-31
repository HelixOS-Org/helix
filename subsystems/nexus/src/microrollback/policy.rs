//! Rollback policy configuration.

/// Policy for rollback decisions
#[derive(Debug, Clone)]
pub struct RollbackPolicy {
    /// Maximum age of rollback points (cycles)
    pub max_age: u64,
    /// Maximum rollback points per component
    pub max_points: usize,
    /// Auto-cleanup enabled
    pub auto_cleanup: bool,
    /// Verify state after rollback
    pub verify_after: bool,
    /// Allow unsafe rollbacks
    pub allow_unsafe: bool,
    /// Cascade rollback on dependency failure
    pub cascade_on_failure: bool,
}

impl Default for RollbackPolicy {
    fn default() -> Self {
        Self {
            max_age: 60 * 1_000_000_000, // 60 seconds
            max_points: 10,
            auto_cleanup: true,
            verify_after: true,
            allow_unsafe: false,
            cascade_on_failure: true,
        }
    }
}
