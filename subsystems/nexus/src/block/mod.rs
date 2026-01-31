//! Block Device Domain
//!
//! AI-powered block device analysis and I/O optimization.

extern crate alloc;

pub mod types;
pub mod scheduler;
pub mod queue;
pub mod stats;
pub mod device;
pub mod workload;
pub mod manager;
pub mod intelligence;

// Re-export all types
pub use types::*;
pub use scheduler::*;
pub use queue::*;
pub use stats::*;
pub use device::*;
pub use workload::*;
pub use manager::*;
pub use intelligence::*;
