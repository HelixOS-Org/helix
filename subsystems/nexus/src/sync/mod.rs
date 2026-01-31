//! Synchronization Intelligence Module
//!
//! This module provides intelligent synchronization analysis including:
//! - Lock types and state management
//! - Contention analysis and hotspot detection
//! - Deadlock detection and prevention
//! - Wait time prediction using linear regression
//! - Lock order optimization using topological sort
//! - Spinlock behavior analysis
//! - Read-write lock usage optimization

mod contention;
mod deadlock;
mod intelligence;
mod lock;
mod order;
mod predictor;
mod rwlock;
mod spinlock;
mod types;

pub use contention::{ContentionAnalyzer, ContentionEvent, ContentionStats};
pub use deadlock::{DeadlockDetector, DeadlockInfo, NearMiss};
pub use intelligence::SyncIntelligence;
pub use lock::LockInfo;
pub use order::{LockOrderOptimizer, OrderViolation};
pub use predictor::{WaitTimeModel, WaitTimePredictor};
pub use rwlock::{RwLockOptimizer, RwLockStats, RwPattern, RwRecommendation, RwRecommendationType};
pub use spinlock::{LongSpin, SpinEvent, SpinStats, SpinlockAnalyzer};
pub use types::{AcquireMode, LockId, LockState, LockType, ThreadId};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_types() {
        assert!(LockType::Mutex.is_blocking());
        assert!(!LockType::Mutex.is_spinning());
        assert!(LockType::Spinlock.is_spinning());
        assert!(!LockType::Spinlock.is_blocking());
    }

    #[test]
    fn test_lock_info() {
        let mut lock = LockInfo::new(1, "test", LockType::Mutex);
        assert!(lock.acquire(100, AcquireMode::Exclusive));
        assert_eq!(lock.state, LockState::HeldExclusive);
        assert!(lock.release(100));
        assert_eq!(lock.state, LockState::Free);
    }

    #[test]
    fn test_sync_intelligence() {
        let intel = SyncIntelligence::new();
        assert_eq!(intel.total_ops(), 0);
    }
}
