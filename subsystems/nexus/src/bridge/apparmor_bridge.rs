// SPDX-License-Identifier: GPL-2.0
//! Bridge AppArmor — AppArmor profile management bridge

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// AppArmor profile mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppArmorMode {
    Enforce,
    Complain,
    Kill,
    Unconfined,
}

/// AppArmor operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppArmorOp {
    LoadProfile,
    ReplaceProfile,
    RemoveProfile,
    ChangeProfile,
    ChangeHat,
    FileAccess,
    NetworkAccess,
    Capability,
    Signal,
    Ptrace,
}

/// AppArmor result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppArmorResult {
    Allowed,
    Denied,
    Audited,
    ProfileLoaded,
    ProfileRemoved,
    Error,
}

/// AppArmor bridge record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppArmorRecord {
    pub op: AppArmorOp,
    pub result: AppArmorResult,
    pub profile_hash: u64,
    pub mode: AppArmorMode,
    pub pid: u32,
    pub latency_ns: u64,
}

impl AppArmorRecord {
    pub fn new(op: AppArmorOp, profile: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in profile {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            op,
            result: AppArmorResult::Allowed,
            profile_hash: h,
            mode: AppArmorMode::Enforce,
            pid: 0,
            latency_ns: 0,
        }
    }
}

/// AppArmor bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppArmorBridgeStats {
    pub total_ops: u64,
    pub profiles_loaded: u64,
    pub access_checks: u64,
    pub denials: u64,
    pub errors: u64,
}

/// Main bridge AppArmor
#[derive(Debug)]
pub struct BridgeAppArmor {
    pub stats: AppArmorBridgeStats,
}

impl BridgeAppArmor {
    pub fn new() -> Self {
        Self {
            stats: AppArmorBridgeStats {
                total_ops: 0,
                profiles_loaded: 0,
                access_checks: 0,
                denials: 0,
                errors: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &AppArmorRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            AppArmorOp::LoadProfile | AppArmorOp::ReplaceProfile => self.stats.profiles_loaded += 1,
            AppArmorOp::FileAccess | AppArmorOp::NetworkAccess | AppArmorOp::Capability => {
                self.stats.access_checks += 1
            },
            _ => {},
        }
        if rec.result == AppArmorResult::Denied {
            self.stats.denials += 1;
        }
        if rec.result == AppArmorResult::Error {
            self.stats.errors += 1;
        }
    }
}
