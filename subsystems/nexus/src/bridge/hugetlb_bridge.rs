// SPDX-License-Identifier: GPL-2.0
//! Bridge hugetlb — huge page allocation, management, and reservation proxy.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Huge page size tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HugePageSize {
    /// 2 MiB huge pages (x86_64)
    Size2M,
    /// 1 GiB huge pages (x86_64)
    Size1G,
    /// 16 KiB (aarch64 4K granule)
    Size16K,
    /// 32 MiB (aarch64 16K granule)
    Size32M,
    /// 512 MiB (aarch64 64K granule)
    Size512M,
}

impl HugePageSize {
    #[inline]
    pub fn bytes(&self) -> u64 {
        match self {
            Self::Size2M => 2 * 1024 * 1024,
            Self::Size1G => 1024 * 1024 * 1024,
            Self::Size16K => 16 * 1024,
            Self::Size32M => 32 * 1024 * 1024,
            Self::Size512M => 512 * 1024 * 1024,
        }
    }

    #[inline(always)]
    pub fn small_pages(&self) -> u64 {
        self.bytes() / 4096
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Size2M => "2M",
            Self::Size1G => "1G",
            Self::Size16K => "16K",
            Self::Size32M => "32M",
            Self::Size512M => "512M",
        }
    }
}

/// Reservation state for a huge page region
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationState {
    /// Reserved but not yet allocated
    Reserved,
    /// Partially fulfilled
    Partial,
    /// Fully allocated
    Fulfilled,
    /// Overcommitted (may fail)
    Overcommitted,
}

/// A huge page pool for a given size
#[derive(Debug)]
#[repr(align(64))]
pub struct HugePagePool {
    pub size: HugePageSize,
    pub total_pages: u64,
    pub free_pages: u64,
    pub reserved_pages: u64,
    pub surplus_pages: u64,
    pub max_surplus: u64,
    alloc_count: u64,
    free_count: u64,
    alloc_failures: u64,
}

impl HugePagePool {
    pub fn new(size: HugePageSize, total: u64) -> Self {
        Self {
            size,
            total_pages: total,
            free_pages: total,
            reserved_pages: 0,
            surplus_pages: 0,
            max_surplus: total / 4,
            alloc_count: 0,
            free_count: 0,
            alloc_failures: 0,
        }
    }

    #[inline(always)]
    pub fn available(&self) -> u64 {
        self.free_pages.saturating_sub(self.reserved_pages)
    }

    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.total_pages == 0 { return 0.0; }
        let used = self.total_pages.saturating_sub(self.free_pages);
        used as f64 / self.total_pages as f64
    }

    #[inline(always)]
    pub fn total_bytes(&self) -> u64 {
        self.total_pages * self.size.bytes()
    }

    #[inline(always)]
    pub fn free_bytes(&self) -> u64 {
        self.free_pages * self.size.bytes()
    }

    #[inline(always)]
    pub fn can_allocate(&self, count: u64) -> bool {
        self.available() >= count || self.surplus_pages + count <= self.max_surplus
    }

    pub fn allocate(&mut self, count: u64) -> u64 {
        let from_free = count.min(self.available());
        let need_surplus = count.saturating_sub(from_free);
        let from_surplus = need_surplus.min(self.max_surplus.saturating_sub(self.surplus_pages));
        let allocated = from_free + from_surplus;

        if allocated == 0 {
            self.alloc_failures += 1;
            return 0;
        }

        self.free_pages = self.free_pages.saturating_sub(from_free);
        self.surplus_pages += from_surplus;
        self.total_pages += from_surplus;
        self.alloc_count += allocated;
        allocated
    }

    #[inline]
    pub fn release(&mut self, count: u64) {
        let release_surplus = count.min(self.surplus_pages);
        let release_normal = count.saturating_sub(release_surplus);

        self.surplus_pages -= release_surplus;
        self.total_pages -= release_surplus;
        self.free_pages += release_normal;
        self.free_count += count;
    }

    #[inline]
    pub fn failure_rate(&self) -> f64 {
        let total = self.alloc_count + self.alloc_failures;
        if total == 0 { return 0.0; }
        self.alloc_failures as f64 / total as f64
    }
}

/// Per-process huge page reservation
#[derive(Debug)]
pub struct ProcessReservation {
    pub pid: u64,
    pub size: HugePageSize,
    pub reserved: u64,
    pub allocated: u64,
    pub state: ReservationState,
    pub addr_start: u64,
}

impl ProcessReservation {
    pub fn new(pid: u64, size: HugePageSize, count: u64, addr: u64) -> Self {
        Self {
            pid,
            size,
            reserved: count,
            allocated: 0,
            state: ReservationState::Reserved,
            addr_start: addr,
        }
    }

    pub fn fulfill_one(&mut self) -> bool {
        if self.allocated >= self.reserved {
            return false;
        }
        self.allocated += 1;
        self.state = if self.allocated >= self.reserved {
            ReservationState::Fulfilled
        } else {
            ReservationState::Partial
        };
        true
    }

    #[inline(always)]
    pub fn fulfillment_ratio(&self) -> f64 {
        if self.reserved == 0 { return 1.0; }
        self.allocated as f64 / self.reserved as f64
    }
}

/// Huge page NUMA node binding
#[derive(Debug, Clone)]
pub struct NumaHugeBinding {
    pub node_id: u32,
    pub size: HugePageSize,
    pub count: u64,
    pub strict: bool,
}

/// Hugetlb cgroup limit
#[derive(Debug, Clone)]
pub struct HugetlbCgroupLimit {
    pub cgroup_id: u64,
    pub size: HugePageSize,
    pub limit_pages: u64,
    pub current_pages: u64,
}

impl HugetlbCgroupLimit {
    #[inline(always)]
    pub fn remaining(&self) -> u64 {
        self.limit_pages.saturating_sub(self.current_pages)
    }

    #[inline(always)]
    pub fn usage_ratio(&self) -> f64 {
        if self.limit_pages == 0 { return 0.0; }
        self.current_pages as f64 / self.limit_pages as f64
    }
}

/// Hugetlb bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HugetlbStats {
    pub total_huge_bytes: u64,
    pub free_huge_bytes: u64,
    pub reservations_active: u64,
    pub alloc_successes: u64,
    pub alloc_failures: u64,
    pub cgroup_denials: u64,
    pub numa_misses: u64,
}

/// Main hugetlb bridge manager
#[repr(align(64))]
pub struct BridgeHugetlb {
    pools: BTreeMap<u8, HugePagePool>,
    reservations: Vec<ProcessReservation>,
    cgroup_limits: BTreeMap<u64, Vec<HugetlbCgroupLimit>>,
    numa_bindings: Vec<NumaHugeBinding>,
    stats: HugetlbStats,
}

impl BridgeHugetlb {
    pub fn new() -> Self {
        Self {
            pools: BTreeMap::new(),
            reservations: Vec::new(),
            cgroup_limits: BTreeMap::new(),
            numa_bindings: Vec::new(),
            stats: HugetlbStats {
                total_huge_bytes: 0,
                free_huge_bytes: 0,
                reservations_active: 0,
                alloc_successes: 0,
                alloc_failures: 0,
                cgroup_denials: 0,
                numa_misses: 0,
            },
        }
    }

    fn size_key(size: HugePageSize) -> u8 {
        match size {
            HugePageSize::Size2M => 0,
            HugePageSize::Size1G => 1,
            HugePageSize::Size16K => 2,
            HugePageSize::Size32M => 3,
            HugePageSize::Size512M => 4,
        }
    }

    #[inline]
    pub fn init_pool(&mut self, size: HugePageSize, count: u64) {
        let pool = HugePagePool::new(size, count);
        self.stats.total_huge_bytes += pool.total_bytes();
        self.stats.free_huge_bytes += pool.free_bytes();
        self.pools.insert(Self::size_key(size), pool);
    }

    pub fn allocate(&mut self, size: HugePageSize, count: u64, cgroup_id: Option<u64>) -> u64 {
        // Check cgroup limits
        if let Some(cg_id) = cgroup_id {
            if let Some(limits) = self.cgroup_limits.get(&cg_id) {
                for limit in limits {
                    if limit.size == size && limit.current_pages + count > limit.limit_pages {
                        self.stats.cgroup_denials += 1;
                        return 0;
                    }
                }
            }
        }

        let key = Self::size_key(size);
        if let Some(pool) = self.pools.get_mut(&key) {
            let allocated = pool.allocate(count);
            if allocated > 0 {
                self.stats.alloc_successes += allocated;
                self.stats.free_huge_bytes = self.stats.free_huge_bytes.saturating_sub(
                    allocated * size.bytes(),
                );
                // Update cgroup
                if let Some(cg_id) = cgroup_id {
                    if let Some(limits) = self.cgroup_limits.get_mut(&cg_id) {
                        for limit in limits.iter_mut() {
                            if limit.size == size {
                                limit.current_pages += allocated;
                            }
                        }
                    }
                }
            } else {
                self.stats.alloc_failures += count;
            }
            allocated
        } else {
            self.stats.alloc_failures += count;
            0
        }
    }

    pub fn release(&mut self, size: HugePageSize, count: u64, cgroup_id: Option<u64>) {
        let key = Self::size_key(size);
        if let Some(pool) = self.pools.get_mut(&key) {
            pool.release(count);
            self.stats.free_huge_bytes += count * size.bytes();
            if let Some(cg_id) = cgroup_id {
                if let Some(limits) = self.cgroup_limits.get_mut(&cg_id) {
                    for limit in limits.iter_mut() {
                        if limit.size == size {
                            limit.current_pages = limit.current_pages.saturating_sub(count);
                        }
                    }
                }
            }
        }
    }

    pub fn reserve(
        &mut self,
        pid: u64,
        size: HugePageSize,
        count: u64,
        addr: u64,
    ) -> bool {
        let key = Self::size_key(size);
        if let Some(pool) = self.pools.get_mut(&key) {
            if pool.available() < count {
                return false;
            }
            pool.reserved_pages += count;
            self.reservations.push(ProcessReservation::new(pid, size, count, addr));
            self.stats.reservations_active += 1;
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn fulfill_reservation(&mut self, pid: u64, size: HugePageSize) -> bool {
        for res in &mut self.reservations {
            if res.pid == pid && res.size == size && res.state != ReservationState::Fulfilled {
                return res.fulfill_one();
            }
        }
        false
    }

    pub fn set_cgroup_limit(
        &mut self,
        cgroup_id: u64,
        size: HugePageSize,
        limit: u64,
    ) {
        let limits = self.cgroup_limits.entry(cgroup_id).or_insert_with(Vec::new);
        for existing in limits.iter_mut() {
            if existing.size == size {
                existing.limit_pages = limit;
                return;
            }
        }
        limits.push(HugetlbCgroupLimit {
            cgroup_id,
            size,
            limit_pages: limit,
            current_pages: 0,
        });
    }

    #[inline]
    pub fn pool_info(&self, size: HugePageSize) -> Option<(u64, u64, f64)> {
        self.pools.get(&Self::size_key(size)).map(|p| {
            (p.total_pages, p.free_pages, p.utilization())
        })
    }

    #[inline(always)]
    pub fn total_reserved_bytes(&self) -> u64 {
        self.reservations.iter().map(|r| r.reserved * r.size.bytes()).sum()
    }

    #[inline(always)]
    pub fn stats(&self) -> &HugetlbStats {
        &self.stats
    }
}
