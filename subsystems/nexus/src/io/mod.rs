//! # I/O Intelligence Module
//!
//! AI-powered I/O optimization and prediction.
//!
//! ## Key Features
//!
//! - **Request Prediction**: Predict upcoming I/O requests
//! - **Queue Optimization**: Smart I/O queue scheduling
//! - **Prefetch Intelligence**: Intelligent prefetching
//! - **Latency Prediction**: Predict I/O latencies
//! - **Bandwidth Optimization**: Maximize bandwidth utilization
//! - **Error Prediction**: Predict I/O failures
//!
//! # Submodules
//!
//! - `types` - Core type definitions (IoOpType, IoPriority, DeviceType)
//! - `request` - I/O request representation
//! - `pattern` - Access pattern analysis
//! - `latency` - Latency prediction
//! - `scheduler` - I/O scheduling
//! - `prefetch` - Prefetch engine
//! - `intelligence` - Central coordinator

#![allow(dead_code)]

extern crate alloc;

// ============================================================================
// SUBMODULES
// ============================================================================

mod intelligence;
mod latency;
mod pattern;
mod prefetch;
mod request;
mod scheduler;
mod types;

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub use intelligence::{DeviceInfo, IoIntelligence};
pub use latency::LatencyPredictor;
pub use pattern::{IoPattern, IoPatternAnalyzer};
pub use prefetch::{PrefetchConfig, PrefetchEngine, PrefetchStats};
pub use request::IoRequest;
pub use scheduler::{IoScheduler, IoSchedulerStats, SchedulingAlgorithm};
pub use types::{DeviceType, IoOpType, IoPriority};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_request() {
        let request = IoRequest::new(IoOpType::Read, 1, 0, 4096);
        assert!(request.is_read());
        assert!(!request.is_write());
        assert_eq!(request.end_offset(), 4096);
    }

    #[test]
    fn test_pattern_analyzer() {
        let mut analyzer = IoPatternAnalyzer::new(100);

        // Sequential pattern
        for i in 0..50 {
            analyzer.record(i * 4096, 4096, true);
        }

        assert_eq!(analyzer.get_pattern(), IoPattern::SequentialRead);
        assert!(analyzer.confidence() > 0.7);
    }

    #[test]
    fn test_latency_predictor() {
        let mut predictor = LatencyPredictor::new();
        predictor.register_device(1, DeviceType::Ssd);

        let predicted = predictor.predict(1, 4096);
        assert!(predicted > 0);

        // Record some latencies
        for _ in 0..20 {
            predictor.record(1, 4096, 100_000);
        }

        let avg = predictor.average_latency(1).unwrap();
        assert!((avg - 100_000.0).abs() < 1000.0);
    }

    #[test]
    fn test_io_scheduler() {
        let mut scheduler = IoScheduler::new(SchedulingAlgorithm::Fifo);

        // Submit requests
        for i in 0..10 {
            let request = IoRequest::new(IoOpType::Read, 1, i * 4096, 4096);
            scheduler.submit(request);
        }

        assert_eq!(scheduler.queue_depth(1), 10);

        // Dispatch
        let _request = scheduler.dispatch(1).unwrap();
        assert_eq!(scheduler.queue_depth(1), 9);
    }

    #[test]
    fn test_prefetch_engine() {
        let mut engine = PrefetchEngine::new();

        // Sequential reads should trigger prefetch
        for i in 0..20 {
            engine.record_access(1, 1, i * 4096, 4096, true);
        }

        let prefetches = engine.record_access(1, 1, 20 * 4096, 4096, true);
        assert!(!prefetches.is_empty());
    }
}
