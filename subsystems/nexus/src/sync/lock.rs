//! Lock Information
//!
//! Lock metadata and state management.

use alloc::string::String;
use alloc::vec::Vec;

use super::{AcquireMode, LockId, LockState, LockType, ThreadId};
use crate::core::NexusTimestamp;

/// Lock information
#[derive(Debug, Clone)]
pub struct LockInfo {
    /// Lock ID
    pub id: LockId,
    /// Lock name
    pub name: String,
    /// Lock type
    pub lock_type: LockType,
    /// Current state
    pub state: LockState,
    /// Current holder
    pub holder: Option<ThreadId>,
    /// Waiters
    pub waiters: Vec<ThreadId>,
    /// Created timestamp
    pub created_at: NexusTimestamp,
    /// Total acquisitions
    pub acquisitions: u64,
    /// Total contentions
    pub contentions: u64,
}

impl LockInfo {
    /// Create new lock info
    pub fn new(id: LockId, name: &str, lock_type: LockType) -> Self {
        Self {
            id,
            name: String::from(name),
            lock_type,
            state: LockState::Free,
            holder: None,
            waiters: Vec::new(),
            created_at: NexusTimestamp::now(),
            acquisitions: 0,
            contentions: 0,
        }
    }

    /// Contention ratio
    #[inline]
    pub fn contention_ratio(&self) -> f64 {
        if self.acquisitions == 0 {
            0.0
        } else {
            self.contentions as f64 / self.acquisitions as f64
        }
    }

    /// Is highly contended?
    #[inline(always)]
    pub fn is_highly_contended(&self) -> bool {
        self.contention_ratio() > 0.3
    }

    /// Add waiter
    #[inline]
    pub fn add_waiter(&mut self, thread: ThreadId) {
        if !self.waiters.contains(&thread) {
            self.waiters.push(thread);
        }
    }

    /// Remove waiter
    #[inline(always)]
    pub fn remove_waiter(&mut self, thread: ThreadId) {
        self.waiters.retain(|&t| t != thread);
    }

    /// Acquire lock
    pub fn acquire(&mut self, thread: ThreadId, mode: AcquireMode) -> bool {
        let contended = !self.waiters.is_empty() || self.holder.is_some();

        if contended {
            self.contentions += 1;
        }

        match (self.state, mode) {
            (LockState::Free, AcquireMode::Exclusive | AcquireMode::Try) => {
                self.state = LockState::HeldExclusive;
                self.holder = Some(thread);
                self.acquisitions += 1;
                true
            },
            (LockState::Free, AcquireMode::Shared) => {
                self.state = LockState::HeldShared(1);
                self.holder = Some(thread);
                self.acquisitions += 1;
                true
            },
            (LockState::HeldShared(n), AcquireMode::Shared) => {
                self.state = LockState::HeldShared(n + 1);
                self.acquisitions += 1;
                true
            },
            _ => false,
        }
    }

    /// Release lock
    pub fn release(&mut self, thread: ThreadId) -> bool {
        match self.state {
            LockState::HeldExclusive if self.holder == Some(thread) => {
                self.state = LockState::Free;
                self.holder = None;
                true
            },
            LockState::HeldShared(n) if n > 1 => {
                self.state = LockState::HeldShared(n - 1);
                true
            },
            LockState::HeldShared(1) => {
                self.state = LockState::Free;
                self.holder = None;
                true
            },
            _ => false,
        }
    }
}
