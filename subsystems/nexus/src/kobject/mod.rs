//! Kobject Intelligence Module
//!
//! Comprehensive kernel object analysis and management.
//!
//! ## Architecture
//!
//! - `types` - Core types: KobjectId, KsetId, KtypeId, states, info structs
//! - `refcount` - Reference count tracking and leak detection
//! - `sysfs` - Sysfs directory and attribute management
//! - `uevent` - Uevent handling and netlink messaging
//! - `lifecycle` - Object lifecycle tracking
//! - `analysis` - Analysis results and recommendations
//! - `intelligence` - Central coordinator

pub mod analysis;
pub mod intelligence;
pub mod lifecycle;
pub mod refcount;
pub mod sysfs;
pub mod types;
pub mod uevent;

// Re-export types
pub use types::{
    KobjectId, KobjectInfo, KobjectState, KsetId, KsetInfo, KtypeId, KtypeInfo, UeventAction,
};

// Re-export refcount
pub use refcount::{RefCountAnalyzer, RefLeak, RefOpType, RefOperation};

// Re-export sysfs
pub use sysfs::{SysfsAttribute, SysfsDirEntry, SysfsManager};

// Re-export uevent
pub use uevent::{Uevent, UeventHandler};

// Re-export lifecycle
pub use lifecycle::{LifecycleEvent, LifecycleEventType, LifecycleTracker};

// Re-export analysis
pub use analysis::{
    KobjectAction, KobjectAnalysis, KobjectIssue, KobjectIssueType, KobjectRecommendation,
};

// Re-export intelligence
pub use intelligence::KobjectIntelligence;
