//! Memory security monitoring and protection.

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use super::types::ThreatSeverity;
use crate::core::NexusTimestamp;

// ============================================================================
// MEMORY SECURITY MONITOR
// ============================================================================

/// Memory security monitoring
pub struct MemorySecurityMonitor {
    /// Protected memory regions
    protected_regions: Vec<ProtectedRegion>,
    /// Stack canary values
    stack_canaries: LinearMap<u64, 64>, // thread_id -> canary
    /// Heap metadata checksums
    heap_checksums: LinearMap<u32, 64>, // block_addr -> checksum
    /// Violations detected
    violations: VecDeque<MemoryViolation>,
    /// Max violations to retain
    max_violations: usize,
}

/// Protected memory region
#[derive(Debug, Clone)]
pub struct ProtectedRegion {
    /// Start address
    pub start: u64,
    /// End address
    pub end: u64,
    /// Protection flags
    pub flags: MemoryProtectionFlags,
    /// Description
    pub description: String,
}

/// Memory protection flags
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryProtectionFlags {
    /// Read allowed
    pub read: bool,
    /// Write allowed
    pub write: bool,
    /// Execute allowed
    pub execute: bool,
    /// Is kernel memory
    pub kernel: bool,
}

/// Memory violation record
#[derive(Debug, Clone)]
pub struct MemoryViolation {
    /// Violation type
    pub violation_type: MemoryViolationType,
    /// Address involved
    pub address: u64,
    /// Process/task ID
    pub source_id: u64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Severity
    pub severity: ThreatSeverity,
}

/// Type of memory violation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryViolationType {
    /// Write to read-only
    WriteViolation,
    /// Execute non-executable
    ExecuteViolation,
    /// Access kernel memory from user
    KernelAccessViolation,
    /// Stack buffer overflow
    StackOverflow,
    /// Heap corruption
    HeapCorruption,
    /// Use after free
    UseAfterFree,
    /// Double free
    DoubleFree,
    /// Null pointer dereference
    NullPointer,
}

impl MemorySecurityMonitor {
    /// Create new memory security monitor
    pub fn new() -> Self {
        Self {
            protected_regions: Vec::new(),
            stack_canaries: LinearMap::new(),
            heap_checksums: LinearMap::new(),
            violations: VecDeque::new(),
            max_violations: 1000,
        }
    }

    /// Add protected region
    #[inline(always)]
    pub fn add_protected_region(&mut self, region: ProtectedRegion) {
        self.protected_regions.push(region);
    }

    /// Set stack canary
    #[inline(always)]
    pub fn set_stack_canary(&mut self, thread_id: u64, canary: u64) {
        self.stack_canaries.insert(thread_id, canary);
    }

    /// Check stack canary
    #[inline]
    pub fn check_stack_canary(&self, thread_id: u64, current: u64) -> bool {
        self.stack_canaries
            .get(&thread_id)
            .map(|&expected| expected == current)
            .unwrap_or(true) // No canary set = pass
    }

    /// Set heap checksum
    #[inline(always)]
    pub fn set_heap_checksum(&mut self, block_addr: u64, checksum: u32) {
        self.heap_checksums.insert(block_addr, checksum);
    }

    /// Verify heap integrity
    #[inline]
    pub fn verify_heap(&self, block_addr: u64, current_checksum: u32) -> bool {
        self.heap_checksums
            .get(&block_addr)
            .map(|&expected| expected == current_checksum)
            .unwrap_or(true)
    }

    /// Check memory access
    pub fn check_access(
        &self,
        address: u64,
        is_write: bool,
        is_execute: bool,
        source_id: u64,
        is_kernel: bool,
    ) -> Option<MemoryViolation> {
        for region in &self.protected_regions {
            if address >= region.start && address < region.end {
                // Check permissions
                if is_write && !region.flags.write {
                    return Some(MemoryViolation {
                        violation_type: MemoryViolationType::WriteViolation,
                        address,
                        source_id,
                        timestamp: NexusTimestamp::now(),
                        severity: ThreatSeverity::High,
                    });
                }

                if is_execute && !region.flags.execute {
                    return Some(MemoryViolation {
                        violation_type: MemoryViolationType::ExecuteViolation,
                        address,
                        source_id,
                        timestamp: NexusTimestamp::now(),
                        severity: ThreatSeverity::Critical,
                    });
                }

                if region.flags.kernel && !is_kernel {
                    return Some(MemoryViolation {
                        violation_type: MemoryViolationType::KernelAccessViolation,
                        address,
                        source_id,
                        timestamp: NexusTimestamp::now(),
                        severity: ThreatSeverity::Critical,
                    });
                }
            }
        }

        None
    }

    /// Record violation
    #[inline]
    pub fn record_violation(&mut self, violation: MemoryViolation) {
        self.violations.push_back(violation);
        if self.violations.len() > self.max_violations {
            self.violations.pop_front();
        }
    }

    /// Get recent violations
    #[inline(always)]
    pub fn recent_violations(&self, count: usize) -> &[MemoryViolation] {
        let start = self.violations.len().saturating_sub(count);
        &self.violations[start..]
    }

    /// Get violation count for source
    #[inline]
    pub fn violation_count(&self, source_id: u64) -> usize {
        self.violations
            .iter()
            .filter(|v| v.source_id == source_id)
            .count()
    }
}

impl Default for MemorySecurityMonitor {
    fn default() -> Self {
        Self::new()
    }
}
