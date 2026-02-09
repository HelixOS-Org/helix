// SPDX-License-Identifier: GPL-2.0
//! Holistic chown — ownership change tracking with privilege escalation detection

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Ownership change type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChownChangeType {
    UidOnly,
    GidOnly,
    Both,
    NoChange,
}

/// Privilege direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivilegeDirection {
    Escalation,
    DeEscalation,
    Lateral,
    None,
}

/// Chown change record
#[derive(Debug, Clone)]
pub struct ChownChangeRecord {
    pub inode: u64,
    pub old_uid: u32,
    pub new_uid: u32,
    pub old_gid: u32,
    pub new_gid: u32,
    pub change_type: ChownChangeType,
    pub priv_dir: PrivilegeDirection,
}

impl ChownChangeRecord {
    pub fn new(inode: u64, old_uid: u32, new_uid: u32, old_gid: u32, new_gid: u32) -> Self {
        let ct = if old_uid != new_uid && old_gid != new_gid {
            ChownChangeType::Both
        } else if old_uid != new_uid {
            ChownChangeType::UidOnly
        } else if old_gid != new_gid {
            ChownChangeType::GidOnly
        } else {
            ChownChangeType::NoChange
        };
        let pd = if new_uid == 0 && old_uid != 0 {
            PrivilegeDirection::Escalation
        } else if old_uid == 0 && new_uid != 0 {
            PrivilegeDirection::DeEscalation
        } else if old_uid != new_uid {
            PrivilegeDirection::Lateral
        } else {
            PrivilegeDirection::None
        };
        Self {
            inode,
            old_uid,
            new_uid,
            old_gid,
            new_gid,
            change_type: ct,
            priv_dir: pd,
        }
    }
}

/// Holistic chown stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticChownStats {
    pub total_changes: u64,
    pub escalations: u64,
    pub de_escalations: u64,
    pub uid_changes: u64,
    pub gid_changes: u64,
}

/// Main holistic chown
#[derive(Debug)]
pub struct HolisticChown {
    pub stats: HolisticChownStats,
}

impl HolisticChown {
    pub fn new() -> Self {
        Self {
            stats: HolisticChownStats {
                total_changes: 0,
                escalations: 0,
                de_escalations: 0,
                uid_changes: 0,
                gid_changes: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &ChownChangeRecord) {
        self.stats.total_changes += 1;
        match rec.priv_dir {
            PrivilegeDirection::Escalation => self.stats.escalations += 1,
            PrivilegeDirection::DeEscalation => self.stats.de_escalations += 1,
            _ => {},
        }
        if rec.old_uid != rec.new_uid {
            self.stats.uid_changes += 1;
        }
        if rec.old_gid != rec.new_gid {
            self.stats.gid_changes += 1;
        }
    }
}
