//! Process Capabilities
//!
//! Process capability state management.

use alloc::vec::Vec;

use super::{Capability, CapabilitySet, Pid, Uid};

/// Process capability state
#[derive(Debug, Clone)]
pub struct ProcessCaps {
    /// Process ID
    pub pid: Pid,
    /// User ID
    pub uid: Uid,
    /// Effective set
    pub effective: CapabilitySet,
    /// Permitted set
    pub permitted: CapabilitySet,
    /// Inheritable set
    pub inheritable: CapabilitySet,
    /// Bounding set
    pub bounding: CapabilitySet,
    /// Ambient set
    pub ambient: CapabilitySet,
    /// Is root
    pub is_root: bool,
    /// No new privs flag
    pub no_new_privs: bool,
    /// Seccomp mode
    pub seccomp_mode: u8,
}

impl ProcessCaps {
    /// Create new process caps
    pub fn new(pid: Pid, uid: Uid) -> Self {
        Self {
            pid,
            uid,
            effective: CapabilitySet::new(),
            permitted: CapabilitySet::new(),
            inheritable: CapabilitySet::new(),
            bounding: CapabilitySet::FULL,
            ambient: CapabilitySet::new(),
            is_root: uid == Uid::ROOT,
            no_new_privs: false,
            seccomp_mode: 0,
        }
    }

    /// Create for root process
    pub fn root(pid: Pid) -> Self {
        Self {
            pid,
            uid: Uid::ROOT,
            effective: CapabilitySet::FULL,
            permitted: CapabilitySet::FULL,
            inheritable: CapabilitySet::FULL,
            bounding: CapabilitySet::FULL,
            ambient: CapabilitySet::new(),
            is_root: true,
            no_new_privs: false,
            seccomp_mode: 0,
        }
    }

    /// Check if has capability
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.effective.has(cap)
    }

    /// Can acquire capability
    pub fn can_acquire(&self, cap: Capability) -> bool {
        self.permitted.has(cap) && self.bounding.has(cap)
    }

    /// Is capability bounded
    pub fn is_bounded(&self, cap: Capability) -> bool {
        self.bounding.has(cap)
    }

    /// Drop capability
    pub fn drop_cap(&mut self, cap: Capability) {
        self.effective.clear(cap);
        self.permitted.clear(cap);
    }

    /// Raise capability (if permitted)
    pub fn raise_cap(&mut self, cap: Capability) -> bool {
        if self.permitted.has(cap) {
            self.effective.set(cap);
            true
        } else {
            false
        }
    }

    /// Get risk score
    pub fn risk_score(&self) -> f32 {
        let mut score = 0.0f32;

        for cap in self.effective.iter() {
            score += cap.risk_level().score() as f32;
        }

        // Normalize to 0-100
        (score / (Capability::all().len() as f32 * 10.0) * 100.0).min(100.0)
    }

    /// Is privileged
    pub fn is_privileged(&self) -> bool {
        for cap in self.effective.iter() {
            if cap.is_privileged() {
                return true;
            }
        }
        false
    }

    /// Get privileged caps
    pub fn privileged_caps(&self) -> Vec<Capability> {
        self.effective
            .iter()
            .filter(|c| c.is_privileged())
            .collect()
    }
}
