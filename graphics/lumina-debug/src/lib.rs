//! LUMINA Debug Utilities
//!
//! Debug, validation, and profiling utilities for LUMINA graphics.
//!
//! # Features
//!
//! - **Validation**: GPU API validation and error checking
//! - **Profiling**: GPU timing and performance analysis
//! - **Capture**: GPU frame capture and debugging
//! - **Markers**: Debug markers and regions
//! - **Statistics**: Resource and performance statistics
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    lumina-debug                         │
//! │                                                         │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
//! │  │ Validation  │ │  Profiling  │ │   Capture   │       │
//! │  │   Layer     │ │   System    │ │   System    │       │
//! │  └─────────────┘ └─────────────┘ └─────────────┘       │
//! │                                                         │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
//! │  │   Markers   │ │ Statistics  │ │   Logger    │       │
//! │  └─────────────┘ └─────────────┘ └─────────────┘       │
//! └─────────────────────────────────────────────────────────┘
//! ```

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

// ============================================================================
// Modules
// ============================================================================

pub mod capture;
pub mod logger;
pub mod markers;
pub mod profiling;
pub mod statistics;
pub mod validation;

// ============================================================================
// Re-exports
// ============================================================================

pub use capture::{
    CaptureFlags, CaptureFrame, CaptureManager, CaptureSettings, CaptureState, FrameCapture,
};
pub use logger::{DebugLogger, LogBuffer, LogEntry, LogFilter, LogLevel};
pub use markers::{
    DebugColor, DebugMarker, DebugRegion, MarkerManager, MarkerStack, ObjectLabel, ObjectLabelType,
};
pub use profiling::{
    GpuProfiler, GpuTimer, GpuTimestamp, PipelineStatistics, ProfileScope, ProfilerStatistics,
    TimerQuery, TimingResult,
};
pub use statistics::{
    DrawStatistics, FrameStatistics, MemoryStatistics, ResourceStatistics, StatisticsCollector,
    StatisticsSnapshot,
};
pub use validation::{
    ValidationCallback, ValidationError, ValidationErrorKind, ValidationFilter, ValidationLayer,
    ValidationSeverity, ValidationStatistics,
};

// ============================================================================
// Prelude
// ============================================================================

/// Commonly used debug types.
pub mod prelude {
    pub use super::{
        CaptureFlags,
        CaptureState,
        DebugColor,
        // Logger
        DebugLogger,
        // Markers
        DebugMarker,
        DebugRegion,
        DrawStatistics,
        // Capture
        FrameCapture,
        FrameStatistics,
        // Profiling
        GpuProfiler,
        GpuTimer,
        LogLevel,
        MemoryStatistics,
        ObjectLabel,
        ProfileScope,
        // Statistics
        ResourceStatistics,
        TimingResult,
        // Validation
        ValidationError,
        ValidationErrorKind,
        ValidationLayer,
        ValidationSeverity,
    };
}

// ============================================================================
// Debug Context
// ============================================================================

/// Debug context combining all debug features.
pub struct DebugContext {
    /// Validation layer.
    pub validation: ValidationLayer,
    /// GPU profiler.
    pub profiler: GpuProfiler,
    /// Capture manager.
    pub capture: CaptureManager,
    /// Marker manager.
    pub markers: MarkerManager,
    /// Statistics collector.
    pub statistics: StatisticsCollector,
    /// Debug logger.
    pub logger: DebugLogger,
    /// Is enabled.
    pub enabled: bool,
}

impl DebugContext {
    /// Create a new debug context.
    pub fn new() -> Self {
        Self {
            validation: ValidationLayer::new(),
            profiler: GpuProfiler::new(),
            capture: CaptureManager::new(),
            markers: MarkerManager::new(),
            statistics: StatisticsCollector::new(),
            logger: DebugLogger::new(),
            enabled: true,
        }
    }

    /// Create a disabled debug context.
    pub fn disabled() -> Self {
        let mut ctx = Self::new();
        ctx.enabled = false;
        ctx
    }

    /// Enable debug features.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable debug features.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Begin a frame.
    pub fn begin_frame(&mut self, frame_index: u64) {
        if !self.enabled {
            return;
        }

        self.profiler.begin_frame(frame_index);
        self.statistics.begin_frame(frame_index);
    }

    /// End a frame.
    pub fn end_frame(&mut self) {
        if !self.enabled {
            return;
        }

        self.profiler.end_frame();
        self.statistics.end_frame();
    }
}

impl Default for DebugContext {
    fn default() -> Self {
        Self::new()
    }
}
