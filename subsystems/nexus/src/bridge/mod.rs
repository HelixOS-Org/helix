//! # Intelligent Syscall Bridge — Year 4 SYMBIOSIS (Q1 2029)
//!
//! Revolutionary kernel-userland bridge that transforms syscalls from dumb
//! request-response into an intelligent, predictive, and cooperative channel.
//!
//! ## Key Innovations
//!
//! - **Syscall Prediction**: Anticipate what the app needs before it asks
//! - **Automatic Batching**: Merge similar syscalls for throughput gains
//! - **Context-Aware Optimization**: Adapt execution path to app type
//! - **Async Intelligent I/O**: Non-blocking syscalls with smart scheduling
//! - **Application Profiling**: Continuous learning from app behavior
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                     INTELLIGENT SYSCALL BRIDGE                       │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │                                                                      │
//! │   Userland ──▶ Interceptor ──▶ Predictor ──▶ Batcher ──▶ Executor  │
//! │                     │              │             │            │      │
//! │                     ▼              ▼             ▼            ▼      │
//! │                  Profile       Prefetch       Merge       Optimize  │
//! │                                                                      │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Submodules
//!
//! - `syscall`: Core intelligent syscall interceptor and router
//! - `predict`: Syscall sequence prediction engine
//! - `batch`: Automatic syscall batching and merging
//! - `async_io`: Async intelligent I/O with smart scheduling
//! - `profile`: Application profiling and behavior learning

#![allow(dead_code)]

extern crate alloc;

pub mod async_io;
pub mod batch;
pub mod predict;
pub mod profile;
pub mod syscall;

// Re-export core types
pub use async_io::{AsyncCompletion, AsyncIoEngine, AsyncIoRequest, AsyncPriority, AsyncStatus};
pub use batch::{BatchDecision, BatchEntry, BatchGroup, BatchOptimizer, BatchStats};
pub use predict::{
    PredictedSyscall, SyscallConfidence, SyscallPattern, SyscallPredictor, SyscallSequence,
};
pub use profile::{AppBehavior, AppClass, AppProfile, AppProfiler, ResourceUsagePattern};
pub use syscall::{
    OptimizationHint, SyscallContext, SyscallId, SyscallInterceptor, SyscallMetrics, SyscallResult,
    SyscallRouter, SyscallType,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_type_classification() {
        assert!(SyscallType::Read.is_io());
        assert!(SyscallType::Write.is_io());
        assert!(SyscallType::Mmap.is_memory());
        assert!(SyscallType::Fork.is_process());
        assert!(SyscallType::Socket.is_network());
    }

    #[test]
    fn test_syscall_prediction_basic() {
        let mut predictor = SyscallPredictor::new(64, 3);

        // Feed a pattern: read -> read -> read
        predictor.observe(SyscallType::Read);
        predictor.observe(SyscallType::Read);
        predictor.observe(SyscallType::Read);

        let prediction = predictor.predict_next();
        assert!(prediction.is_some());
        let pred = prediction.unwrap();
        assert_eq!(pred.syscall_type, SyscallType::Read);
        assert!(pred.confidence.value() > 0.5);
    }

    #[test]
    fn test_batch_optimizer() {
        let mut optimizer = BatchOptimizer::new(10, 1000);

        let e1 = BatchEntry::new(SyscallId(1), SyscallType::Read, 4096);
        let e2 = BatchEntry::new(SyscallId(2), SyscallType::Read, 4096);
        let e3 = BatchEntry::new(SyscallId(3), SyscallType::Read, 4096);

        optimizer.submit(e1);
        optimizer.submit(e2);
        optimizer.submit(e3);

        let groups = optimizer.flush();
        // Three reads should be batchable
        assert!(!groups.is_empty());
    }

    #[test]
    fn test_app_profiler() {
        let mut profiler = AppProfiler::new(100);

        // Simulate a sequential reader
        for _ in 0..50 {
            profiler.record_syscall(SyscallType::Read, 100);
        }

        let profile = profiler.build_profile();
        assert_eq!(profile.dominant_class, AppClass::IoIntensive);
    }

    #[test]
    fn test_async_io_engine() {
        let mut engine = AsyncIoEngine::new(256);

        let req = AsyncIoRequest::new(SyscallId(1), SyscallType::Read, 8192, AsyncPriority::Normal);
        let ticket = engine.submit(req);

        assert_eq!(engine.status(ticket), AsyncStatus::Queued);
        assert_eq!(engine.pending_count(), 1);
    }
}
