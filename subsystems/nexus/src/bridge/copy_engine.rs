//! # Bridge Copy Engine
//!
//! Optimized data copy operations between user/kernel space:
//! - Zero-copy path detection
//! - Page-pinning for DMA transfers
//! - Vectored copy (iovec) handling
//! - Copy-on-write optimization
//! - Cache-line aligned copies
//! - Copy bandwidth tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Copy direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyDirection {
    UserToKernel,
    KernelToUser,
    KernelToKernel,
    UserToUser,
}

/// Copy method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMethod {
    /// Standard memcpy
    Standard,
    /// Page remapping (zero-copy)
    PageRemap,
    /// Copy-on-write (share until write)
    CopyOnWrite,
    /// DMA-assisted copy
    Dma,
    /// SIMD-accelerated
    Simd,
}

/// Iovec entry
#[derive(Debug, Clone, Copy)]
pub struct IoVec {
    pub base_addr: u64,
    pub len: u64,
}

/// Copy request
#[derive(Debug, Clone)]
pub struct CopyRequest {
    pub request_id: u64,
    pub direction: CopyDirection,
    pub src_addr: u64,
    pub dst_addr: u64,
    pub length: u64,
    pub method: CopyMethod,
    pub is_vectored: bool,
    pub iov: Vec<IoVec>,
    pub timestamp: u64,
}

impl CopyRequest {
    pub fn simple(id: u64, dir: CopyDirection, src: u64, dst: u64, len: u64) -> Self {
        Self {
            request_id: id,
            direction: dir,
            src_addr: src,
            dst_addr: dst,
            length: len,
            method: CopyMethod::Standard,
            is_vectored: false,
            iov: Vec::new(),
            timestamp: 0,
        }
    }

    pub fn vectored(id: u64, dir: CopyDirection, iov: Vec<IoVec>) -> Self {
        let total: u64 = iov.iter().map(|v| v.len).sum();
        Self {
            request_id: id,
            direction: dir,
            src_addr: 0,
            dst_addr: 0,
            length: total,
            method: CopyMethod::Standard,
            is_vectored: true,
            iov,
            timestamp: 0,
        }
    }

    /// Is this a large copy that benefits from zero-copy?
    pub fn is_large(&self) -> bool {
        self.length >= 4096
    }

    /// Is page-aligned?
    pub fn is_page_aligned(&self) -> bool {
        (self.src_addr & 0xFFF) == 0 && (self.dst_addr & 0xFFF) == 0
    }
}

/// Copy completion
#[derive(Debug, Clone)]
pub struct CopyCompletion {
    pub request_id: u64,
    pub bytes_copied: u64,
    pub method_used: CopyMethod,
    pub duration_ns: u64,
    pub fault_count: u32,
}

impl CopyCompletion {
    pub fn bandwidth_mbps(&self) -> f64 {
        if self.duration_ns == 0 { return 0.0; }
        (self.bytes_copied as f64 / (1024.0 * 1024.0)) / (self.duration_ns as f64 / 1_000_000_000.0)
    }
}

/// Pinned page tracking
#[derive(Debug, Clone)]
pub struct PinnedPages {
    pub start_pfn: u64,
    pub count: u32,
    pub owner_pid: u64,
    pub pin_count: u32,
    pub for_dma: bool,
}

/// Copy engine stats
#[derive(Debug, Clone, Default)]
pub struct BridgeCopyEngineStats {
    pub total_copies: u64,
    pub total_bytes: u64,
    pub zero_copy_count: u64,
    pub zero_copy_bytes: u64,
    pub cow_count: u64,
    pub page_faults: u64,
    pub pinned_pages: usize,
    pub avg_bandwidth_mbps: f64,
}

/// Bridge Copy Engine
pub struct BridgeCopyEngine {
    completions: Vec<CopyCompletion>,
    pinned: BTreeMap<u64, PinnedPages>,
    max_completions: usize,
    total_bandwidth_sum: f64,
    total_bandwidth_count: u64,
    zero_copy_threshold: u64,
    stats: BridgeCopyEngineStats,
}

impl BridgeCopyEngine {
    pub fn new() -> Self {
        Self {
            completions: Vec::new(),
            pinned: BTreeMap::new(),
            max_completions: 256,
            total_bandwidth_sum: 0.0,
            total_bandwidth_count: 0,
            zero_copy_threshold: 4096,
            stats: BridgeCopyEngineStats::default(),
        }
    }

    /// Choose optimal copy method
    pub fn select_method(&self, request: &CopyRequest) -> CopyMethod {
        if request.length >= self.zero_copy_threshold && request.is_page_aligned() {
            return CopyMethod::PageRemap;
        }
        if request.length >= 64 * 1024 {
            return CopyMethod::Simd;
        }
        CopyMethod::Standard
    }

    /// Record a completed copy
    pub fn record_completion(&mut self, completion: CopyCompletion) {
        let bw = completion.bandwidth_mbps();
        self.total_bandwidth_sum += bw;
        self.total_bandwidth_count += 1;

        let is_zero_copy = matches!(completion.method_used, CopyMethod::PageRemap | CopyMethod::CopyOnWrite);
        self.stats.total_copies += 1;
        self.stats.total_bytes += completion.bytes_copied;
        if is_zero_copy {
            self.stats.zero_copy_count += 1;
            self.stats.zero_copy_bytes += completion.bytes_copied;
        }
        self.stats.page_faults += completion.fault_count as u64;
        self.stats.avg_bandwidth_mbps = self.total_bandwidth_sum / self.total_bandwidth_count as f64;

        if self.completions.len() >= self.max_completions {
            self.completions.remove(0);
        }
        self.completions.push(completion);
    }

    /// Pin pages for DMA
    pub fn pin_pages(&mut self, start_pfn: u64, count: u32, pid: u64) {
        let entry = self.pinned.entry(start_pfn).or_insert(PinnedPages {
            start_pfn,
            count,
            owner_pid: pid,
            pin_count: 0,
            for_dma: true,
        });
        entry.pin_count += 1;
        self.stats.pinned_pages = self.pinned.len();
    }

    /// Unpin pages
    pub fn unpin_pages(&mut self, start_pfn: u64) {
        if let Some(entry) = self.pinned.get_mut(&start_pfn) {
            entry.pin_count = entry.pin_count.saturating_sub(1);
            if entry.pin_count == 0 {
                self.pinned.remove(&start_pfn);
            }
        }
        self.stats.pinned_pages = self.pinned.len();
    }

    /// Zero-copy ratio
    pub fn zero_copy_ratio(&self) -> f64 {
        if self.stats.total_bytes == 0 { return 0.0; }
        self.stats.zero_copy_bytes as f64 / self.stats.total_bytes as f64
    }

    pub fn stats(&self) -> &BridgeCopyEngineStats {
        &self.stats
    }
}
