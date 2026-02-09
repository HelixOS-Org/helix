// SPDX-License-Identifier: MIT
//! # Holistic Memory Map Analysis
//!
//! System-wide mmap pattern analysis:
//! - Global address space utilization heatmap
//! - Cross-process mapping deduplication opportunities
//! - System-wide fragmentation index
//! - Hot region identification across all processes
//! - Memory-mapped I/O bottleneck detection

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

const HEATMAP_BUCKETS: usize = 256;

#[derive(Debug, Clone)]
pub struct AddressHeatmap {
    buckets: [u64; HEATMAP_BUCKETS],
    bucket_size: u64,
    base_addr: u64,
    total_samples: u64,
}

impl AddressHeatmap {
    pub fn new(base: u64, range: u64) -> Self {
        Self {
            buckets: [0; HEATMAP_BUCKETS],
            bucket_size: range / HEATMAP_BUCKETS as u64,
            base_addr: base,
            total_samples: 0,
        }
    }

    #[inline]
    pub fn record(&mut self, addr: u64) {
        if addr < self.base_addr { return; }
        let offset = addr - self.base_addr;
        let idx = (offset / self.bucket_size.max(1)) as usize;
        if idx < HEATMAP_BUCKETS {
            self.buckets[idx] += 1;
            self.total_samples += 1;
        }
    }

    #[inline]
    pub fn hottest_regions(&self, top_n: usize) -> Vec<(u64, u64)> {
        let mut indexed: Vec<(usize, u64)> = self.buckets.iter()
            .enumerate().map(|(i, &v)| (i, v)).collect();
        indexed.sort_by(|a, b| b.1.cmp(&a.1));
        indexed.into_iter().take(top_n)
            .map(|(i, count)| (self.base_addr + i as u64 * self.bucket_size, count))
            .collect()
    }

    #[inline]
    pub fn entropy(&self) -> f64 {
        if self.total_samples == 0 { return 0.0; }
        let mut h = 0.0f64;
        for &b in &self.buckets {
            if b > 0 {
                let p = b as f64 / self.total_samples as f64;
                h -= p * libm::log2(p);
            }
        }
        h
    }
}

#[derive(Debug, Clone)]
pub struct DedupCandidate {
    pub file_hash: u64,
    pub pids: Vec<u64>,
    pub size: u64,
    pub savings: u64,
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MmapHolisticStats {
    pub total_mapped_bytes: u64,
    pub total_processes: u64,
    pub dedup_candidates: u64,
    pub potential_savings: u64,
    pub fragmentation_index: f64,
    pub hot_regions_identified: u64,
}

pub struct MmapHolisticManager {
    /// pid → (addr, size, file_hash)
    process_mappings: BTreeMap<u64, Vec<(u64, u64, u64)>>,
    /// file_hash → list of pids
    file_sharing: BTreeMap<u64, Vec<u64>>,
    heatmap: AddressHeatmap,
    stats: MmapHolisticStats,
}

impl MmapHolisticManager {
    pub fn new() -> Self {
        Self {
            process_mappings: BTreeMap::new(),
            file_sharing: BTreeMap::new(),
            heatmap: AddressHeatmap::new(0, 1u64 << 47),
            stats: MmapHolisticStats::default(),
        }
    }

    #[inline]
    pub fn register_mapping(&mut self, pid: u64, addr: u64, size: u64, file_hash: u64) {
        self.process_mappings.entry(pid).or_insert_with(Vec::new)
            .push((addr, size, file_hash));
        self.file_sharing.entry(file_hash).or_insert_with(Vec::new)
            .push(pid);
        self.heatmap.record(addr);
        self.stats.total_mapped_bytes += size;
    }

    /// Find file mappings shared by multiple processes (dedup opportunities)
    pub fn find_dedup_candidates(&self, min_sharers: usize) -> Vec<DedupCandidate> {
        self.file_sharing.iter()
            .filter(|(_, pids)| pids.len() >= min_sharers)
            .map(|(&hash, pids)| {
                let size = self.process_mappings.values()
                    .flat_map(|v| v.iter())
                    .find(|(_, _, h)| *h == hash)
                    .map(|(_, s, _)| *s)
                    .unwrap_or(0);
                let savings = size * (pids.len() as u64 - 1);
                DedupCandidate {
                    file_hash: hash,
                    pids: pids.clone(),
                    size,
                    savings,
                }
            })
            .collect()
    }

    /// Compute system-wide fragmentation: variance of gap sizes
    pub fn compute_fragmentation(&mut self) -> f64 {
        let mut all_addrs: Vec<(u64, u64)> = self.process_mappings.values()
            .flat_map(|v| v.iter().map(|&(a, s, _)| (a, a + s)))
            .collect();
        all_addrs.sort();

        if all_addrs.len() < 2 { return 0.0; }

        let mut gaps = Vec::new();
        for i in 1..all_addrs.len() {
            let gap = all_addrs[i].0.saturating_sub(all_addrs[i - 1].1);
            if gap > 0 { gaps.push(gap); }
        }

        if gaps.is_empty() { return 0.0; }
        let avg = gaps.iter().sum::<u64>() / gaps.len() as u64;
        let variance: f64 = gaps.iter()
            .map(|&g| { let d = g as f64 - avg as f64; d * d })
            .sum::<f64>() / gaps.len() as f64;
        let idx = libm::sqrt(variance) / avg.max(1) as f64;
        self.stats.fragmentation_index = idx;
        idx
    }

    #[inline(always)]
    pub fn hot_regions(&self, n: usize) -> Vec<(u64, u64)> {
        self.heatmap.hottest_regions(n)
    }

    #[inline(always)]
    pub fn address_entropy(&self) -> f64 { self.heatmap.entropy() }
    #[inline(always)]
    pub fn stats(&self) -> &MmapHolisticStats { &self.stats }
}
