//! Block Device Domain
//!
//! AI-powered block device analysis and I/O optimization.

extern crate alloc;

pub mod device;
pub mod intelligence;
pub mod manager;
pub mod queue;
pub mod scheduler;
pub mod stats;
pub mod types;
pub mod workload;

// Re-export all types
pub use device::*;
pub use intelligence::*;
pub use manager::*;
pub use queue::*;
pub use scheduler::*;
pub use stats::*;
pub use types::*;
pub use workload::*;
