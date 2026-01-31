//! Process Domain
//!
//! AI-powered process behavior analysis and optimization.

extern crate alloc;

pub mod types;
pub mod metrics;
pub mod profile;
pub mod behavior;
pub mod anomaly;
pub mod resource;
pub mod lifecycle;
pub mod intelligence;

// Re-export all types
pub use types::*;
pub use metrics::*;
pub use profile::*;
pub use behavior::*;
pub use anomaly::*;
pub use resource::*;
pub use lifecycle::*;
pub use intelligence::*;
