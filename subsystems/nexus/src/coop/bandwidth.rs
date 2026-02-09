//! # Coop Bandwidth
//!
//! Cooperative bandwidth allocation and sharing:
//! - Token bucket with cooperative lending
//! - Bandwidth reservation with preemption
//! - Fair queueing across cooperative processes
//! - Burst allowance negotiation
//! - Bandwidth credit system

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Bandwidth resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BandwidthResource {
    /// Disk IO bandwidth
    DiskIo,
    /// Network bandwidth
    Network,
    /// Memory bandwidth
    MemBandwidth,
    /// PCIe bandwidth
    Pcie,
    /// Inter-CPU bandwidth
    Interconnect,
}

/// Token bucket for bandwidth management
#[derive(Debug, Clone)]
pub struct BandwidthBucket {
    pub resource: BandwidthResource,
    /// Current tokens (bytes)
    pub tokens: u64,
    /// Max burst capacity
    pub capacity: u64,
    /// Refill rate (bytes per second)
    pub rate: u64,
    /// Last refill timestamp
    pub last_refill_ns: u64,
    /// Lent tokens to others
    pub lent: u64,
    /// Borrowed tokens from others
    pub borrowed: u64,
}

impl BandwidthBucket {
    pub fn new(resource: BandwidthResource, rate: u64, capacity: u64) -> Self {
        Self {
            resource,
            tokens: capacity,
            capacity,
            rate,
            last_refill_ns: 0,
            lent: 0,
            borrowed: 0,
        }
    }

    /// Refill tokens based on elapsed time
    #[inline]
    pub fn refill(&mut self, now_ns: u64) {
        if now_ns <= self.last_refill_ns {
            return;
        }
        let elapsed_ns = now_ns - self.last_refill_ns;
        let new_tokens = (self.rate as u128 * elapsed_ns as u128 / 1_000_000_000u128) as u64;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill_ns = now_ns;
    }

    /// Try to consume tokens
    #[inline]
    pub fn try_consume(&mut self, bytes: u64, now_ns: u64) -> bool {
        self.refill(now_ns);
        if self.tokens >= bytes {
            self.tokens -= bytes;
            true
        } else {
            false
        }
    }

    /// Available tokens (including refill projection)
    #[inline]
    pub fn available(&self, now_ns: u64) -> u64 {
        let elapsed_ns = now_ns.saturating_sub(self.last_refill_ns);
        let refill = (self.rate as u128 * elapsed_ns as u128 / 1_000_000_000u128) as u64;
        (self.tokens + refill).min(self.capacity)
    }

    /// Lend tokens to another process
    #[inline]
    pub fn lend(&mut self, amount: u64) -> bool {
        if self.tokens >= amount {
            self.tokens -= amount;
            self.lent += amount;
            true
        } else {
            false
        }
    }

    /// Accept borrowed tokens
    #[inline(always)]
    pub fn borrow_tokens(&mut self, amount: u64) {
        self.tokens += amount;
        self.borrowed += amount;
    }

    /// Utilization ratio
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { 0.0 } else {
            1.0 - (self.tokens as f64 / self.capacity as f64)
        }
    }
}

/// Per-process bandwidth allocation
#[derive(Debug)]
pub struct ProcessBandwidth {
    pub pid: u64,
    pub buckets: BTreeMap<u8, BandwidthBucket>,
    pub total_consumed: u64,
    pub total_denied: u64,
    pub total_lent: u64,
    pub total_borrowed: u64,
    /// Priority for bandwidth allocation
    pub priority: u8,
}

impl ProcessBandwidth {
    pub fn new(pid: u64, priority: u8) -> Self {
        Self {
            pid,
            buckets: BTreeMap::new(),
            total_consumed: 0,
            total_denied: 0,
            total_lent: 0,
            total_borrowed: 0,
            priority,
        }
    }

    #[inline(always)]
    pub fn add_bucket(&mut self, resource: BandwidthResource, rate: u64, capacity: u64) {
        self.buckets.insert(resource as u8, BandwidthBucket::new(resource, rate, capacity));
    }

    pub fn try_consume(&mut self, resource: BandwidthResource, bytes: u64, now_ns: u64) -> bool {
        if let Some(bucket) = self.buckets.get_mut(&(resource as u8)) {
            if bucket.try_consume(bytes, now_ns) {
                self.total_consumed += bytes;
                true
            } else {
                self.total_denied += 1;
                false
            }
        } else {
            false
        }
    }

    /// Request bandwidth lending
    #[inline]
    pub fn can_lend(&self, resource: BandwidthResource, amount: u64) -> bool {
        self.buckets.get(&(resource as u8))
            .map(|b| b.tokens >= amount && b.utilization() < 0.5)
            .unwrap_or(false)
    }

    #[inline]
    pub fn lend(&mut self, resource: BandwidthResource, amount: u64) -> bool {
        if let Some(bucket) = self.buckets.get_mut(&(resource as u8)) {
            if bucket.lend(amount) {
                self.total_lent += amount;
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn receive_borrow(&mut self, resource: BandwidthResource, amount: u64) {
        if let Some(bucket) = self.buckets.get_mut(&(resource as u8)) {
            bucket.borrow_tokens(amount);
            self.total_borrowed += amount;
        }
    }
}

/// Bandwidth transfer record
#[derive(Debug, Clone)]
pub struct BandwidthTransfer {
    pub from_pid: u64,
    pub to_pid: u64,
    pub resource: BandwidthResource,
    pub amount: u64,
    pub timestamp_ns: u64,
}

/// Bandwidth allocator stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopBandwidthStats {
    pub tracked_processes: usize,
    pub total_consumed: u64,
    pub total_denied: u64,
    pub total_transfers: u64,
    pub avg_utilization: f64,
}

/// Coop Bandwidth Allocator
pub struct CoopBandwidthAllocator {
    processes: BTreeMap<u64, ProcessBandwidth>,
    transfers: Vec<BandwidthTransfer>,
    stats: CoopBandwidthStats,
}

impl CoopBandwidthAllocator {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            transfers: Vec::new(),
            stats: CoopBandwidthStats::default(),
        }
    }

    #[inline(always)]
    pub fn register(&mut self, pid: u64, priority: u8) {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessBandwidth::new(pid, priority));
    }

    #[inline]
    pub fn add_bucket(&mut self, pid: u64, resource: BandwidthResource, rate: u64, cap: u64) {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.add_bucket(resource, rate, cap);
        }
    }

    #[inline]
    pub fn try_consume(&mut self, pid: u64, resource: BandwidthResource, bytes: u64, now_ns: u64) -> bool {
        if let Some(proc) = self.processes.get_mut(&pid) {
            if proc.try_consume(resource, bytes, now_ns) {
                return true;
            }
        }
        // Try cooperative lending
        self.try_borrow(pid, resource, bytes, now_ns)
    }

    fn try_borrow(&mut self, borrower_pid: u64, resource: BandwidthResource, amount: u64, now_ns: u64) -> bool {
        // Find a lender with spare capacity
        let lender_pid = {
            let mut candidates: Vec<(u64, u64)> = self.processes.iter()
                .filter(|(&pid, proc)| pid != borrower_pid && proc.can_lend(resource, amount))
                .map(|(&pid, proc)| {
                    let available = proc.buckets.get(&(resource as u8))
                        .map(|b| b.tokens)
                        .unwrap_or(0);
                    (pid, available)
                })
                .collect();
            candidates.sort_by(|a, b| b.1.cmp(&a.1));
            candidates.first().map(|&(pid, _)| pid)
        };

        if let Some(lpid) = lender_pid {
            if let Some(lender) = self.processes.get_mut(&lpid) {
                if lender.lend(resource, amount) {
                    if let Some(borrower) = self.processes.get_mut(&borrower_pid) {
                        borrower.receive_borrow(resource, amount);
                        borrower.total_consumed += amount;
                    }
                    self.transfers.push(BandwidthTransfer {
                        from_pid: lpid,
                        to_pid: borrower_pid,
                        resource,
                        amount,
                        timestamp_ns: now_ns,
                    });
                    self.update_stats();
                    return true;
                }
            }
        }
        false
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_consumed = self.processes.values().map(|p| p.total_consumed).sum();
        self.stats.total_denied = self.processes.values().map(|p| p.total_denied).sum();
        self.stats.total_transfers = self.transfers.len() as u64;
        if !self.processes.is_empty() {
            let total_util: f64 = self.processes.values()
                .flat_map(|p| p.buckets.values())
                .map(|b| b.utilization())
                .sum();
            let count: usize = self.processes.values()
                .map(|p| p.buckets.len())
                .sum();
            if count > 0 {
                self.stats.avg_utilization = total_util / count as f64;
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopBandwidthStats {
        &self.stats
    }
}
