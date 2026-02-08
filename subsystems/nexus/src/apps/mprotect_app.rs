// SPDX-License-Identifier: MIT
//! # Application Memory Protection Manager
//!
//! Per-application mprotect tracking and analysis:
//! - Permission change history per VMA
//! - W^X violation detection
//! - Guard page placement optimization
//! - ASLR entropy scoring per application
//! - Stack/heap protection policy enforcement

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Memory protection flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtFlags(pub u32);

impl ProtFlags {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1);
    pub const WRITE: Self = Self(2);
    pub const EXEC: Self = Self(4);
    pub const RW: Self = Self(3);
    pub const RX: Self = Self(5);
    pub const RWX: Self = Self(7);

    pub fn is_writable(self) -> bool { self.0 & 2 != 0 }
    pub fn is_executable(self) -> bool { self.0 & 4 != 0 }
    pub fn is_wx(self) -> bool { self.is_writable() && self.is_executable() }
}

/// A protection change event
#[derive(Debug, Clone)]
pub struct ProtChange {
    pub vma_start: u64,
    pub vma_end: u64,
    pub old_prot: ProtFlags,
    pub new_prot: ProtFlags,
    pub timestamp: u64,
}

/// W^X violation record
#[derive(Debug, Clone)]
pub struct WxViolation {
    pub app_id: u64,
    pub address: u64,
    pub prot: ProtFlags,
    pub timestamp: u64,
    pub caller_site: u64,
}

/// Guard page configuration
#[derive(Debug, Clone)]
pub struct GuardPageConfig {
    /// Pages below stack
    pub stack_guard_pages: u32,
    /// Pages between heap regions
    pub heap_guard_pages: u32,
    /// Random guard page insertion probability (0.0-1.0)
    pub random_guard_probability: f64,
}

impl Default for GuardPageConfig {
    fn default() -> Self {
        Self {
            stack_guard_pages: 1,
            heap_guard_pages: 0,
            random_guard_probability: 0.0,
        }
    }
}

/// ASLR entropy analysis for an application
#[derive(Debug, Clone)]
pub struct AslrEntropy {
    pub stack_entropy_bits: u8,
    pub heap_entropy_bits: u8,
    pub mmap_entropy_bits: u8,
    pub vdso_entropy_bits: u8,
    pub total_score: f64,
}

impl AslrEntropy {
    pub fn compute(stack_samples: &[u64], heap_samples: &[u64], mmap_samples: &[u64]) -> Self {
        let stack_bits = Self::estimate_entropy(stack_samples);
        let heap_bits = Self::estimate_entropy(heap_samples);
        let mmap_bits = Self::estimate_entropy(mmap_samples);
        let total = (stack_bits + heap_bits + mmap_bits) as f64 / 3.0;
        Self {
            stack_entropy_bits: stack_bits,
            heap_entropy_bits: heap_bits,
            mmap_entropy_bits: mmap_bits,
            vdso_entropy_bits: 0,
            total_score: total / 28.0, // normalize to 0-1 (28 bits = ideal)
        }
    }

    fn estimate_entropy(samples: &[u64]) -> u8 {
        if samples.len() < 2 { return 0; }
        let mut xor_acc = 0u64;
        for i in 1..samples.len() {
            xor_acc |= samples[i] ^ samples[i - 1];
        }
        xor_acc.count_ones() as u8
    }
}

/// Per-app protection profile
#[derive(Debug, Clone)]
pub struct AppProtProfile {
    pub app_id: u64,
    pub prot_changes: Vec<ProtChange>,
    pub wx_violations: Vec<WxViolation>,
    pub guard_config: GuardPageConfig,
    pub aslr: Option<AslrEntropy>,
    pub jit_regions: u64,
}

/// Protection manager stats
#[derive(Debug, Clone, Default)]
pub struct MprotectAppStats {
    pub total_prot_changes: u64,
    pub wx_violations_detected: u64,
    pub guard_pages_inserted: u64,
    pub jit_regions_tracked: u64,
}

/// Memory protection application manager
pub struct MprotectAppManager {
    apps: BTreeMap<u64, AppProtProfile>,
    stats: MprotectAppStats,
    /// Max prot change history per app
    max_history: usize,
    /// W^X enforcement mode
    wx_enforce: bool,
}

impl MprotectAppManager {
    pub fn new(wx_enforce: bool) -> Self {
        Self {
            apps: BTreeMap::new(),
            stats: MprotectAppStats::default(),
            max_history: 1024,
            wx_enforce,
        }
    }

    pub fn record_prot_change(
        &mut self,
        app_id: u64,
        vma_start: u64,
        vma_end: u64,
        old_prot: ProtFlags,
        new_prot: ProtFlags,
        now: u64,
    ) -> Result<(), WxViolation> {
        let profile = self.apps.entry(app_id).or_insert_with(|| AppProtProfile {
            app_id,
            prot_changes: Vec::new(),
            wx_violations: Vec::new(),
            guard_config: GuardPageConfig::default(),
            aslr: None,
            jit_regions: 0,
        });

        // W^X check
        if new_prot.is_wx() && self.wx_enforce {
            let violation = WxViolation {
                app_id,
                address: vma_start,
                prot: new_prot,
                timestamp: now,
                caller_site: 0,
            };
            profile.wx_violations.push(violation.clone());
            self.stats.wx_violations_detected += 1;
            return Err(violation);
        }

        // Track JIT regions (write → exec transitions)
        if old_prot.is_writable() && !old_prot.is_executable()
            && new_prot.is_executable() && !new_prot.is_writable()
        {
            profile.jit_regions += 1;
            self.stats.jit_regions_tracked += 1;
        }

        let change = ProtChange { vma_start, vma_end, old_prot, new_prot, timestamp: now };
        profile.prot_changes.push(change);
        if profile.prot_changes.len() > self.max_history {
            profile.prot_changes.remove(0);
        }
        self.stats.total_prot_changes += 1;

        Ok(())
    }

    pub fn set_guard_config(&mut self, app_id: u64, config: GuardPageConfig) {
        if let Some(profile) = self.apps.get_mut(&app_id) {
            profile.guard_config = config;
        }
    }

    pub fn update_aslr_entropy(&mut self, app_id: u64, entropy: AslrEntropy) {
        if let Some(profile) = self.apps.get_mut(&app_id) {
            profile.aslr = Some(entropy);
        }
    }

    pub fn wx_violations(&self, app_id: u64) -> &[WxViolation] {
        self.apps.get(&app_id).map(|p| p.wx_violations.as_slice()).unwrap_or(&[])
    }

    pub fn stats(&self) -> &MprotectAppStats {
        &self.stats
    }
}
