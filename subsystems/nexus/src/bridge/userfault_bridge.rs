// SPDX-License-Identifier: GPL-2.0
//! Bridge userfaultfd — user-space page fault handling proxy for live migration and post-copy.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Userfaultfd feature flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UffdFeatures(pub u64);

impl UffdFeatures {
    pub const PAGEFAULT_FLAG_WP: Self = Self(1 << 0);
    pub const EVENT_FORK: Self = Self(1 << 1);
    pub const EVENT_REMAP: Self = Self(1 << 2);
    pub const EVENT_REMOVE: Self = Self(1 << 3);
    pub const EVENT_UNMAP: Self = Self(1 << 4);
    pub const MISSING_HUGETLBFS: Self = Self(1 << 5);
    pub const MISSING_SHMEM: Self = Self(1 << 6);
    pub const SIGBUS: Self = Self(1 << 7);
    pub const THREAD_ID: Self = Self(1 << 8);
    pub const MINOR_HUGETLBFS: Self = Self(1 << 9);
    pub const MINOR_SHMEM: Self = Self(1 << 10);
    pub const EXACT_ADDRESS: Self = Self(1 << 11);
    pub const WP_HUGETLBFS_SHMEM: Self = Self(1 << 12);
    pub const WP_UNPOPULATED: Self = Self(1 << 13);
    pub const POISON: Self = Self(1 << 14);
    pub const WP_ASYNC: Self = Self(1 << 15);
    pub const MOVE: Self = Self(1 << 16);

    pub fn has(&self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn combine(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Page fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultType {
    /// Missing page (never populated)
    Missing,
    /// Write-protect fault
    WriteProtect,
    /// Minor fault (page present but needs update)
    Minor,
}

/// Userfaultfd event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UffdEventType {
    Pagefault,
    Fork,
    Remap,
    Remove,
    Unmap,
}

/// A page fault message
#[derive(Debug, Clone)]
pub struct UffdMsg {
    pub event: UffdEventType,
    pub fault_type: FaultType,
    pub address: u64,
    pub flags: u64,
    pub thread_id: u64,
    pub timestamp_ns: u64,
}

impl UffdMsg {
    pub fn pagefault(fault_type: FaultType, address: u64, tid: u64) -> Self {
        Self {
            event: UffdEventType::Pagefault,
            fault_type,
            address: address & !4095, // page-align
            flags: 0,
            thread_id: tid,
            timestamp_ns: 0,
        }
    }
}

/// Registration mode for a memory range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterMode {
    /// Handle missing pages
    Missing,
    /// Handle write-protect faults
    WriteProtect,
    /// Handle minor faults
    Minor,
    /// Combined missing + WP
    MissingWp,
}

/// A registered memory range
#[derive(Debug)]
pub struct RegisteredRange {
    pub start: u64,
    pub end: u64,
    pub mode: RegisterMode,
    pub fault_count: u64,
    pub resolved_count: u64,
    pub pending_faults: u64,
    pub pages_copied: u64,
    pub pages_zeroed: u64,
}

impl RegisteredRange {
    pub fn new(start: u64, end: u64, mode: RegisterMode) -> Self {
        Self {
            start,
            end,
            mode,
            fault_count: 0,
            resolved_count: 0,
            pending_faults: 0,
            pages_copied: 0,
            pages_zeroed: 0,
        }
    }

    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }

    pub fn page_count(&self) -> u64 {
        (self.end - self.start) / 4096
    }

    pub fn resolution_rate(&self) -> f64 {
        if self.fault_count == 0 { return 1.0; }
        self.resolved_count as f64 / self.fault_count as f64
    }
}

/// A userfaultfd instance
#[derive(Debug)]
pub struct UffdInstance {
    pub fd: i32,
    pub pid: u64,
    pub features: UffdFeatures,
    pub ranges: Vec<RegisteredRange>,
    pub msg_queue: Vec<UffdMsg>,
    pub max_queue_size: usize,
    pub non_blocking: bool,
    overflow_count: u64,
}

impl UffdInstance {
    pub fn new(fd: i32, pid: u64, features: UffdFeatures) -> Self {
        Self {
            fd,
            pid,
            features,
            ranges: Vec::new(),
            msg_queue: Vec::new(),
            max_queue_size: 4096,
            non_blocking: false,
            overflow_count: 0,
        }
    }

    pub fn register_range(&mut self, start: u64, size: u64, mode: RegisterMode) -> bool {
        let end = match start.checked_add(size) {
            Some(e) => e,
            None => return false,
        };
        // Page-align
        let aligned_start = start & !4095;
        let aligned_end = (end + 4095) & !4095;
        // Check overlap
        for range in &self.ranges {
            if range.start < aligned_end && aligned_start < range.end {
                return false;
            }
        }
        self.ranges.push(RegisteredRange::new(aligned_start, aligned_end, mode));
        true
    }

    pub fn unregister_range(&mut self, start: u64) -> bool {
        let aligned = start & !4095;
        if let Some(idx) = self.ranges.iter().position(|r| r.start == aligned) {
            self.ranges.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn push_fault(&mut self, msg: UffdMsg) -> bool {
        // Check if address is in a registered range
        let in_range = self.ranges.iter().any(|r| r.contains(msg.address));
        if !in_range {
            return false;
        }
        if self.msg_queue.len() >= self.max_queue_size {
            self.overflow_count += 1;
            return false;
        }
        // Update range fault count
        for range in &mut self.ranges {
            if range.contains(msg.address) {
                range.fault_count += 1;
                range.pending_faults += 1;
                break;
            }
        }
        self.msg_queue.push(msg);
        true
    }

    pub fn read_msg(&mut self) -> Option<UffdMsg> {
        if self.msg_queue.is_empty() {
            return None;
        }
        Some(self.msg_queue.remove(0))
    }

    pub fn pending_count(&self) -> usize {
        self.msg_queue.len()
    }

    pub fn total_registered_pages(&self) -> u64 {
        self.ranges.iter().map(|r| r.page_count()).sum()
    }
}

/// Copy/zero operation for resolving faults
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolveOp {
    /// Copy data to the faulting page
    Copy,
    /// Zero the faulting page
    ZeroPage,
    /// Continue (minor fault resolution)
    Continue,
    /// Write-protect a range
    WriteProtect,
    /// Write-unprotect a range
    WriteUnprotect,
    /// Poison the page (error injection)
    Poison,
    /// Move pages (UFFDIO_MOVE)
    Move,
}

/// Resolve a page fault
#[derive(Debug)]
pub struct ResolveRequest {
    pub op: ResolveOp,
    pub address: u64,
    pub length: u64,
    pub dont_wake: bool,
}

/// Userfault bridge stats
#[derive(Debug, Clone)]
pub struct UserfaultBridgeStats {
    pub instances_created: u64,
    pub ranges_registered: u64,
    pub faults_received: u64,
    pub faults_resolved: u64,
    pub pages_copied: u64,
    pub pages_zeroed: u64,
    pub pages_poisoned: u64,
    pub overflows: u64,
}

/// Main userfaultfd bridge manager
pub struct BridgeUserfault {
    instances: BTreeMap<i32, UffdInstance>,
    next_fd: i32,
    supported_features: UffdFeatures,
    max_instances: usize,
    stats: UserfaultBridgeStats,
}

impl BridgeUserfault {
    pub fn new() -> Self {
        let features = UffdFeatures::PAGEFAULT_FLAG_WP
            .combine(UffdFeatures::EVENT_FORK)
            .combine(UffdFeatures::EVENT_REMAP)
            .combine(UffdFeatures::EVENT_REMOVE)
            .combine(UffdFeatures::EVENT_UNMAP)
            .combine(UffdFeatures::MISSING_HUGETLBFS)
            .combine(UffdFeatures::MISSING_SHMEM)
            .combine(UffdFeatures::THREAD_ID)
            .combine(UffdFeatures::MINOR_HUGETLBFS)
            .combine(UffdFeatures::MINOR_SHMEM)
            .combine(UffdFeatures::WP_ASYNC)
            .combine(UffdFeatures::MOVE)
            .combine(UffdFeatures::POISON);

        Self {
            instances: BTreeMap::new(),
            next_fd: 300,
            supported_features: features,
            max_instances: 256,
            stats: UserfaultBridgeStats {
                instances_created: 0,
                ranges_registered: 0,
                faults_received: 0,
                faults_resolved: 0,
                pages_copied: 0,
                pages_zeroed: 0,
                pages_poisoned: 0,
                overflows: 0,
            },
        }
    }

    pub fn create_instance(&mut self, pid: u64, requested_features: UffdFeatures) -> Option<i32> {
        if self.instances.len() >= self.max_instances {
            return None;
        }
        // Grant only supported features
        let granted = UffdFeatures(requested_features.0 & self.supported_features.0);
        let fd = self.next_fd;
        self.next_fd += 1;
        self.instances.insert(fd, UffdInstance::new(fd, pid, granted));
        self.stats.instances_created += 1;
        Some(fd)
    }

    pub fn destroy_instance(&mut self, fd: i32) -> bool {
        self.instances.remove(&fd).is_some()
    }

    pub fn register_range(
        &mut self,
        fd: i32,
        start: u64,
        size: u64,
        mode: RegisterMode,
    ) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            if inst.register_range(start, size, mode) {
                self.stats.ranges_registered += 1;
                return true;
            }
        }
        false
    }

    pub fn unregister_range(&mut self, fd: i32, start: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            inst.unregister_range(start)
        } else {
            false
        }
    }

    pub fn inject_fault(&mut self, fd: i32, msg: UffdMsg) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            if inst.push_fault(msg) {
                self.stats.faults_received += 1;
                true
            } else {
                self.stats.overflows += 1;
                false
            }
        } else {
            false
        }
    }

    pub fn read_fault(&mut self, fd: i32) -> Option<UffdMsg> {
        if let Some(inst) = self.instances.get_mut(&fd) {
            inst.read_msg()
        } else {
            None
        }
    }

    pub fn resolve(&mut self, fd: i32, req: ResolveRequest) -> bool {
        let inst = match self.instances.get_mut(&fd) {
            Some(i) => i,
            None => return false,
        };
        let page_aligned = req.address & !4095;
        let pages = (req.length + 4095) / 4096;

        // Find matching range and update stats
        for range in &mut inst.ranges {
            if range.contains(page_aligned) {
                match req.op {
                    ResolveOp::Copy => {
                        range.pages_copied += pages;
                        self.stats.pages_copied += pages;
                    }
                    ResolveOp::ZeroPage => {
                        range.pages_zeroed += pages;
                        self.stats.pages_zeroed += pages;
                    }
                    ResolveOp::Poison => {
                        self.stats.pages_poisoned += pages;
                    }
                    _ => {}
                }
                range.resolved_count += 1;
                range.pending_faults = range.pending_faults.saturating_sub(1);
                self.stats.faults_resolved += 1;
                return true;
            }
        }
        false
    }

    pub fn instance_info(&self, fd: i32) -> Option<(u64, usize, usize, u64)> {
        self.instances.get(&fd).map(|inst| {
            (inst.pid, inst.ranges.len(), inst.pending_count(), inst.total_registered_pages())
        })
    }

    pub fn stats(&self) -> &UserfaultBridgeStats {
        &self.stats
    }
}

// ============================================================================
// Merged from userfault_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UffdV2Feature {
    Pagefault,
    Fork,
    Remap,
    Unmap,
    MissingHugeTlb,
    MissingShmem,
    MinorHugeTlb,
    MinorShmem,
    ExactAddress,
    WriteProtect,
    Poison,
}

/// Page fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UffdV2FaultType {
    Missing,
    WriteProtect,
    Minor,
    Poison,
}

/// A page fault event
#[derive(Debug, Clone)]
pub struct UffdV2Event {
    pub fault_type: UffdV2FaultType,
    pub address: u64,
    pub flags: u32,
    pub thread_id: u64,
    pub timestamp_ns: u64,
}

/// Userfaultfd registration
#[derive(Debug, Clone)]
pub struct UffdV2Registration {
    pub start_addr: u64,
    pub length: u64,
    pub mode: u32,
    pub missing: bool,
    pub write_protect: bool,
    pub minor: bool,
}

/// A userfaultfd instance
#[derive(Debug, Clone)]
pub struct UffdV2Instance {
    pub fd: u64,
    pub features: u32,
    pub registrations: Vec<UffdV2Registration>,
    pub pending_faults: Vec<UffdV2Event>,
    pub resolved_faults: u64,
    pub copy_ops: u64,
    pub zero_ops: u64,
    pub wp_ops: u64,
    pub poison_ops: u64,
}

impl UffdV2Instance {
    pub fn new(fd: u64, features: u32) -> Self {
        Self {
            fd,
            features,
            registrations: Vec::new(),
            pending_faults: Vec::new(),
            resolved_faults: 0,
            copy_ops: 0,
            zero_ops: 0,
            wp_ops: 0,
            poison_ops: 0,
        }
    }

    pub fn register(&mut self, start: u64, length: u64, mode: u32) {
        self.registrations.push(UffdV2Registration {
            start_addr: start,
            length,
            mode,
            missing: (mode & 1) != 0,
            write_protect: (mode & 2) != 0,
            minor: (mode & 4) != 0,
        });
    }

    pub fn report_fault(&mut self, fault_type: UffdV2FaultType, addr: u64, tid: u64, tick: u64) {
        self.pending_faults.push(UffdV2Event {
            fault_type,
            address: addr,
            flags: 0,
            thread_id: tid,
            timestamp_ns: tick,
        });
    }

    pub fn resolve_copy(&mut self, _dst: u64, _len: u64) -> bool {
        if let Some(_fault) = self.pending_faults.pop() {
            self.resolved_faults += 1;
            self.copy_ops += 1;
            true
        } else {
            false
        }
    }

    pub fn resolve_zero(&mut self, _addr: u64, _len: u64) -> bool {
        if let Some(_fault) = self.pending_faults.pop() {
            self.resolved_faults += 1;
            self.zero_ops += 1;
            true
        } else {
            false
        }
    }

    pub fn write_protect(&mut self, addr: u64, len: u64, protect: bool) {
        self.wp_ops += 1;
    }

    pub fn pending_count(&self) -> usize {
        self.pending_faults.len()
    }
}

/// Statistics for userfaultfd V2 bridge
#[derive(Debug, Clone)]
pub struct UffdV2BridgeStats {
    pub instances_created: u64,
    pub registrations: u64,
    pub faults_reported: u64,
    pub faults_resolved: u64,
    pub copy_operations: u64,
    pub zero_operations: u64,
    pub wp_operations: u64,
    pub poison_operations: u64,
}

/// Main userfaultfd V2 bridge manager
#[derive(Debug)]
pub struct BridgeUserfaultV2 {
    instances: BTreeMap<u64, UffdV2Instance>,
    next_fd: u64,
    stats: UffdV2BridgeStats,
}

impl BridgeUserfaultV2 {
    pub fn new() -> Self {
        Self {
            instances: BTreeMap::new(),
            next_fd: 1,
            stats: UffdV2BridgeStats {
                instances_created: 0,
                registrations: 0,
                faults_reported: 0,
                faults_resolved: 0,
                copy_operations: 0,
                zero_operations: 0,
                wp_operations: 0,
                poison_operations: 0,
            },
        }
    }

    pub fn create(&mut self, features: u32) -> u64 {
        let fd = self.next_fd;
        self.next_fd += 1;
        self.instances.insert(fd, UffdV2Instance::new(fd, features));
        self.stats.instances_created += 1;
        fd
    }

    pub fn register(&mut self, fd: u64, start: u64, length: u64, mode: u32) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            inst.register(start, length, mode);
            self.stats.registrations += 1;
            true
        } else {
            false
        }
    }

    pub fn report_fault(&mut self, fd: u64, fault_type: UffdV2FaultType, addr: u64, tid: u64, tick: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            inst.report_fault(fault_type, addr, tid, tick);
            self.stats.faults_reported += 1;
            true
        } else {
            false
        }
    }

    pub fn resolve_copy(&mut self, fd: u64, dst: u64, len: u64) -> bool {
        if let Some(inst) = self.instances.get_mut(&fd) {
            if inst.resolve_copy(dst, len) {
                self.stats.faults_resolved += 1;
                self.stats.copy_operations += 1;
                return true;
            }
        }
        false
    }

    pub fn stats(&self) -> &UffdV2BridgeStats {
        &self.stats
    }
}
