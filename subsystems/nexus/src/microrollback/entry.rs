//! Rollback entry for history tracking.

use alloc::string::String;
use alloc::vec::Vec;

use crate::core::{ComponentId, NexusTimestamp};

/// An entry in the rollback log
#[derive(Debug, Clone)]
pub struct RollbackEntry {
    /// Rollback ID
    pub id: u64,
    /// Target component
    pub component: ComponentId,
    /// Rollback point used
    pub rollback_point: u64,
    /// Start timestamp
    pub started: NexusTimestamp,
    /// End timestamp
    pub ended: Option<NexusTimestamp>,
    /// Success
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
    /// State before rollback (for undo)
    pub pre_rollback_state: Option<Vec<u8>>,
}
