// SPDX-License-Identifier: MIT
//! # Holistic Memory Remap Optimization
//!
//! System-wide mremap analysis:
//! - Global address space compaction opportunities
//! - Cross-process remap conflict prevention
//! - System-wide ASLR re-randomization scheduler
//! - Remap latency hotspot detection
//! - Address space entropy monitoring

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct RemapHotspot {
    pub addr_range: (u64, u64),
    pub remap_count: u64,
    pub avg_latency_ns: u64,
    pub processes_affected: u32,
    pub last_seen: u64,
}

impl RemapHotspot {
    pub fn is_active(&self, now: u64, timeout: u64) -> bool {
        now.saturating_sub(self.last_seen) < timeout
    }
    pub fn severity(&self) -> f64 {
        (self.remap_count as f64 * self.avg_latency_ns as f64) / 1_000_000.0
    }
}

#[derive(Debug, Clone)]
pub struct CompactionCandidate {
    pub pid: u64,
    pub fragmentation: f64,
    pub gap_bytes: u64,
    pub remap_cost: u64,
    pub benefit: f64, // expected TLB improvement
}

#[derive(Debug, Clone)]
pub struct AslrSchedule {
    pub pid: u64,
    pub last_randomized: u64,
    pub period_ns: u64,
    pub entropy_bits: u32,
}

#[derive(Debug, Clone, Default)]
pub struct MremapHolisticStats {
    pub total_remaps: u64,
    pub total_compactions: u64,
    pub hotspots_detected: u64,
    pub aslr_refreshes: u64,
    pub address_entropy: f64,
    pub avg_remap_latency: u64,
}

pub struct MremapHolisticManager {
    hotspots: BTreeMap<u64, RemapHotspot>,
    compaction_candidates: Vec<CompactionCandidate>,
    aslr_schedules: BTreeMap<u64, AslrSchedule>,
    /// per-process remap counter
    remap_counts: BTreeMap<u64, u64>,
    stats: MremapHolisticStats,
}

impl MremapHolisticManager {
    pub fn new() -> Self {
        Self {
            hotspots: BTreeMap::new(),
            compaction_candidates: Vec::new(),
            aslr_schedules: BTreeMap::new(),
            remap_counts: BTreeMap::new(),
            stats: MremapHolisticStats::default(),
        }
    }

    /// Record a remap event system-wide
    pub fn record_remap(&mut self, pid: u64, addr: u64, size: u64, latency: u64, now: u64) {
        *self.remap_counts.entry(pid).or_insert(0) += 1;
        self.stats.total_remaps += 1;

        // Update latency EMA
        self.stats.avg_remap_latency = self.stats.avg_remap_latency
            - (self.stats.avg_remap_latency / 16)
            + (latency / 16);

        // Check if this is a hotspot (frequently remapped region)
        let key = addr / (2 * 1024 * 1024); // 2MB granularity
        let hotspot = self.hotspots.entry(key).or_insert(RemapHotspot {
            addr_range: (addr, addr + size),
            remap_count: 0,
            avg_latency_ns: latency,
            processes_affected: 0,
            last_seen: now,
        });
        hotspot.remap_count += 1;
        hotspot.avg_latency_ns = (hotspot.avg_latency_ns * 7 + latency) / 8;
        hotspot.last_seen = now;
    }

    /// Find hotspots above threshold
    pub fn active_hotspots(&self, min_count: u64, now: u64) -> Vec<&RemapHotspot> {
        self.hotspots.values()
            .filter(|h| h.remap_count >= min_count && h.is_active(now, 60_000_000_000))
            .collect()
    }

    /// Analyze all processes for compaction opportunities
    pub fn analyze_compaction(
        &mut self,
        process_fragmentation: &[(u64, f64, u64)], // (pid, frag_ratio, gap_bytes)
    ) {
        self.compaction_candidates.clear();
        for &(pid, frag, gaps) in process_fragmentation {
            if frag > 0.3 {
                let cost = gaps / 4096 * 100; // 100ns per page copy
                let benefit = frag * 0.8; // expected improvement
                self.compaction_candidates.push(CompactionCandidate {
                    pid, fragmentation: frag, gap_bytes: gaps,
                    remap_cost: cost, benefit,
                });
            }
        }
        self.compaction_candidates.sort_by(|a, b|
            b.benefit.partial_cmp(&a.benefit).unwrap_or(core::cmp::Ordering::Equal));
        self.stats.total_compactions += self.compaction_candidates.len() as u64;
    }

    /// Get top compaction candidates
    pub fn top_compaction_candidates(&self, n: usize) -> &[CompactionCandidate] {
        &self.compaction_candidates[..n.min(self.compaction_candidates.len())]
    }

    /// Schedule ASLR re-randomization
    pub fn schedule_aslr(&mut self, pid: u64, period_ns: u64, entropy: u32, now: u64) {
        self.aslr_schedules.insert(pid, AslrSchedule {
            pid, last_randomized: now, period_ns, entropy_bits: entropy,
        });
    }

    /// Check which processes are due for ASLR refresh
    pub fn due_for_aslr_refresh(&self, now: u64) -> Vec<u64> {
        self.aslr_schedules.iter()
            .filter(|(_, s)| now.saturating_sub(s.last_randomized) >= s.period_ns)
            .map(|(&pid, _)| pid)
            .collect()
    }

    /// Compute system-wide address space entropy
    pub fn compute_entropy(&mut self, bases: &[u64]) -> f64 {
        if bases.is_empty() { return 0.0; }
        // XOR-fold to estimate randomness
        let mut xor_fold = 0u64;
        for &b in bases { xor_fold ^= b; }
        let bits_set = xor_fold.count_ones();
        let entropy = bits_set as f64 / 64.0;
        self.stats.address_entropy = entropy;
        entropy
    }

    pub fn stats(&self) -> &MremapHolisticStats { &self.stats }
}
