//! Device Subsystem
//!
//! AI-powered device management and driver matching.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  DeviceIntelligence                        │
//! │  ┌────────┬─────────┬──────────┬───────────────────────┐   │
//! │  │ Tree   │ Matcher │  Power   │      Hotplug          │   │
//! │  │ Parser │         │ Manager  │      Handler          │   │
//! │  └────────┴─────────┴──────────┴───────────────────────┘   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - `types` - Core types (DeviceId, DriverId, BusType, DeviceState, PowerState)
//! - `info` - Device and driver information structures
//! - `tree` - Device tree parsing
//! - `matcher` - ML-like driver matching
//! - `power` - Power state management
//! - `hotplug` - Hotplug event handling
//! - `analysis` - Device health analysis
//! - `intelligence` - AI-powered device management

mod analysis;
mod hotplug;
mod info;
mod intelligence;
mod matcher;
mod power;
mod tree;
mod types;

// Core types
pub use types::{BusId, BusType, ClassId, DeviceId, DeviceState, DriverId, PowerState};

// Device and driver info
pub use info::{DeviceInfo, DriverInfo};

// Device tree
pub use tree::{DeviceTreeNode, DeviceTreeParser};

// Driver matching
pub use matcher::{DriverMatcher, MatchScore, MatchType};

// Power management
pub use power::{DevicePowerManager, PowerPolicy, PowerTransition};

// Hotplug handling
pub use hotplug::{HotplugEvent, HotplugHandler, HotplugNotification};

// Analysis
pub use analysis::{
    DeviceAction, DeviceAnalysis, DeviceIssue, DeviceIssueType, DeviceRecommendation,
};

// Intelligence
pub use intelligence::DeviceIntelligence;
