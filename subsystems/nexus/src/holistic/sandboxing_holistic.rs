// SPDX-License-Identifier: GPL-2.0
//! Holistic sandboxing — cross-layer sandbox confinement analysis

extern crate alloc;
use alloc::vec::Vec;

/// Sandboxing holistic metric
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxHolisticMetric {
    ConfinementStrength,
    EscapeVector,
    LayerRedundancy,
    AttackSurface,
    NamespaceIsolation,
}

/// Sandbox finding
#[derive(Debug, Clone)]
pub struct SandboxHolisticFinding {
    pub metric: SandboxHolisticMetric,
    pub score: u64,
    pub pid: u32,
    pub sandbox_layers: u32,
    pub escape_vectors: u32,
    pub accessible_syscalls: u32,
}

impl SandboxHolisticFinding {
    pub fn new(metric: SandboxHolisticMetric) -> Self {
        Self { metric, score: 0, pid: 0, sandbox_layers: 0, escape_vectors: 0, accessible_syscalls: 0 }
    }
}

/// Sandboxing holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SandboxHolisticStats {
    pub total_analyses: u64,
    pub weak_sandboxes: u64,
    pub escape_risks: u64,
    pub avg_layers: f64,
}

/// Main holistic sandboxing
#[derive(Debug)]
pub struct HolisticSandboxing {
    pub stats: SandboxHolisticStats,
}

impl HolisticSandboxing {
    pub fn new() -> Self {
        Self { stats: SandboxHolisticStats { total_analyses: 0, weak_sandboxes: 0, escape_risks: 0, avg_layers: 0.0 } }
    }

    #[inline]
    pub fn analyze(&mut self, finding: &SandboxHolisticFinding) {
        self.stats.total_analyses += 1;
        if finding.sandbox_layers < 2 { self.stats.weak_sandboxes += 1; }
        if finding.escape_vectors > 0 { self.stats.escape_risks += finding.escape_vectors as u64; }
        let n = self.stats.total_analyses as f64;
        self.stats.avg_layers = self.stats.avg_layers * ((n - 1.0) / n) + finding.sandbox_layers as f64 / n;
    }
}
