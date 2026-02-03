//! LUMINA Sync - GPU Synchronization Primitives
//!
//! This crate provides comprehensive GPU synchronization primitives for LUMINA,
//! including fences, semaphores, timeline semaphores, barriers, and events.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Synchronization                       │
//! ├─────────────────┬──────────────────┬───────────────────┤
//! │      Fences     │    Semaphores    │     Barriers      │
//! │  (CPU ↔ GPU)   │    (GPU ↔ GPU)   │   (In-Command)    │
//! ├─────────────────┼──────────────────┼───────────────────┤
//! │     Events      │    Timeline      │     Queries       │
//! │  (Fine-grain)   │   (Monotonic)    │   (Timestamps)    │
//! └─────────────────┴──────────────────┴───────────────────┘
//! ```
//!
//! # Features
//!
//! - **Fences**: CPU-GPU synchronization for frame pacing
//! - **Semaphores**: GPU-GPU queue synchronization
//! - **Timeline Semaphores**: Monotonic counter-based sync
//! - **Barriers**: In-command buffer synchronization
//! - **Events**: Fine-grained GPU signaling
//! - **Queries**: Timestamp and occlusion queries
//! - **Workload**: GPU workload scheduling and load balancing

#![no_std]
#![allow(dead_code)]

extern crate alloc;

pub mod barrier;
pub mod event;
pub mod fence;
pub mod query;
pub mod semaphore;
pub mod timeline;
pub mod wait;
pub mod workload;

/// Prelude for common imports
pub mod prelude {
    pub use crate::barrier::{
        AccessFlags, Barrier, BarrierBatch, BufferBarrier, ImageBarrier, ImageLayout,
        MemoryBarrier, PipelineStageFlags,
    };
    pub use crate::event::{Event, EventHandle, EventManager, EventState};
    pub use crate::fence::{Fence, FenceHandle, FenceManager, FenceState};
    pub use crate::query::{
        OcclusionQuery, PipelineStatisticsQuery, QueryPool, QueryType, TimestampQuery,
    };
    pub use crate::semaphore::{Semaphore, SemaphoreHandle, SemaphoreManager};
    pub use crate::timeline::{TimelineManager, TimelineSemaphore, TimelineSemaphoreHandle};
    pub use crate::wait::{WaitAll, WaitAny, WaitResult, WaitTimeout};
}

pub use prelude::*;
