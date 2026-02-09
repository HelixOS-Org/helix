//! # Syscall Transformation & Rewriting Engine
//!
//! Transforms, rewrites, and optimizes syscalls before they reach the kernel
//! execution path. This includes:
//! - Replacing slow syscalls with faster equivalents
//! - Merging multiple syscalls into single operations
//! - Converting blocking calls to async where beneficial
//! - Adding prefetch hints based on intent analysis

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// TRANSFORMATION TYPES
// ============================================================================

/// Type of transformation applied to a syscall
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformType {
    /// No transformation (passthrough)
    Identity,
    /// Replace with a faster equivalent
    Replace,
    /// Merge with other pending syscalls
    Merge,
    /// Convert blocking to async
    AsyncConvert,
    /// Add prefetch before execution
    Prefetch,
    /// Batch with similar operations
    Batch,
    /// Reorder for better cache locality
    Reorder,
    /// Eliminate redundant syscall
    Eliminate,
    /// Split into smaller operations for better parallelism
    Split,
    /// Coalesce small I/O into large I/O
    Coalesce,
    /// Short-circuit with cached result
    ShortCircuit,
}

/// A transformation rule
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TransformRule {
    /// Rule identifier
    pub id: u32,
    /// Rule name
    pub name: &'static str,
    /// Input syscall type to match
    pub input_type: SyscallType,
    /// Transformation to apply
    pub transform_type: TransformType,
    /// Priority (lower = applied first)
    pub priority: u8,
    /// Whether this rule is enabled
    pub enabled: bool,
    /// Minimum confidence to apply
    pub min_confidence: f64,
    /// Times this rule has been applied
    pub apply_count: u64,
    /// Cumulative latency saved (ns)
    pub latency_saved_ns: u64,
}

impl TransformRule {
    pub fn new(
        id: u32,
        name: &'static str,
        input_type: SyscallType,
        transform_type: TransformType,
    ) -> Self {
        Self {
            id,
            name,
            input_type,
            transform_type,
            priority: 5,
            enabled: true,
            min_confidence: 0.5,
            apply_count: 0,
            latency_saved_ns: 0,
        }
    }

    #[inline(always)]
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Record that this rule was applied
    #[inline(always)]
    pub fn record_application(&mut self, saved_ns: u64) {
        self.apply_count += 1;
        self.latency_saved_ns += saved_ns;
    }

    /// Average latency saved per application
    #[inline]
    pub fn avg_saved_ns(&self) -> u64 {
        if self.apply_count == 0 {
            0
        } else {
            self.latency_saved_ns / self.apply_count
        }
    }
}

// ============================================================================
// TRANSFORMED SYSCALL
// ============================================================================

/// A syscall after transformation
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TransformedSyscall {
    /// Original syscall type
    pub original_type: SyscallType,
    /// Transformed syscall type (may be the same)
    pub new_type: SyscallType,
    /// Transformation applied
    pub transform: TransformType,
    /// Rule that was applied
    pub rule_id: Option<u32>,
    /// Process ID
    pub pid: u64,
    /// File descriptor (if applicable)
    pub fd: Option<i64>,
    /// Buffer size (if applicable)
    pub buffer_size: Option<u64>,
    /// Offset (if applicable)
    pub offset: Option<u64>,
    /// Additional operations to prepend (e.g., prefetch)
    pub prepend_ops: Vec<PrependOp>,
    /// Whether the original call was eliminated
    pub eliminated: bool,
    /// Estimated latency savings (ns)
    pub estimated_savings_ns: u64,
}

/// An operation to prepend before the main syscall
#[derive(Debug, Clone)]
pub struct PrependOp {
    /// Operation type
    pub op_type: PrependOpType,
    /// Target file descriptor
    pub fd: i64,
    /// Offset
    pub offset: u64,
    /// Size
    pub size: u64,
}

/// Types of prepend operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrependOpType {
    /// Prefetch data into page cache
    Prefetch,
    /// Pre-allocate memory
    PreAllocate,
    /// Warm up TLB
    TlbWarmup,
    /// Pre-resolve path
    PathResolve,
    /// Pre-connect socket
    PreConnect,
}

// ============================================================================
// COALESCE ENGINE
// ============================================================================

/// Pending I/O operation for coalescing
#[derive(Debug, Clone)]
pub struct PendingIo {
    /// Process ID
    pub pid: u64,
    /// File descriptor
    pub fd: i64,
    /// Is this a read or write?
    pub is_read: bool,
    /// Starting offset
    pub offset: u64,
    /// Size in bytes
    pub size: u64,
    /// Submitted timestamp
    pub submitted_at: u64,
}

/// Coalescing result
#[derive(Debug, Clone)]
pub struct CoalescedIo {
    /// File descriptor
    pub fd: i64,
    /// Is this a read or write?
    pub is_read: bool,
    /// Coalesced starting offset (min of all)
    pub offset: u64,
    /// Coalesced total size
    pub total_size: u64,
    /// Number of original operations merged
    pub merged_count: u32,
    /// Original individual PIDs
    pub pids: Vec<u64>,
}

/// I/O coalescing engine
#[repr(align(64))]
pub struct CoalesceEngine {
    /// Pending I/O operations grouped by (fd, is_read)
    pending: BTreeMap<(i64, bool), Vec<PendingIo>>,
    /// Coalescing threshold (minimum operations to coalesce)
    threshold: usize,
    /// Maximum gap between offsets to coalesce (bytes)
    max_gap: u64,
    /// Maximum coalesced size (bytes)
    max_size: u64,
    /// Statistics
    total_coalesced: u64,
    total_operations_saved: u64,
}

impl CoalesceEngine {
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            threshold: 3,
            max_gap: 64 * 1024,        // 64KB gap
            max_size: 4 * 1024 * 1024, // 4MB max coalesced
            total_coalesced: 0,
            total_operations_saved: 0,
        }
    }

    /// Submit a pending I/O operation
    #[inline(always)]
    pub fn submit(&mut self, io: PendingIo) {
        let key = (io.fd, io.is_read);
        self.pending.entry(key).or_insert_with(Vec::new).push(io);
    }

    /// Try to coalesce pending operations for a given fd
    pub fn try_coalesce(&mut self, fd: i64, is_read: bool) -> Option<CoalescedIo> {
        let key = (fd, is_read);
        let ops = self.pending.get_mut(&key)?;

        if ops.len() < self.threshold {
            return None;
        }

        // Sort by offset
        ops.sort_by_key(|o| o.offset);

        // Try to merge contiguous or close operations
        let mut merged_offset = ops[0].offset;
        let mut merged_end = ops[0].offset + ops[0].size;
        let mut pids = Vec::new();
        let mut mergeable = vec![true; ops.len()];

        for (i, op) in ops.iter().enumerate() {
            let op_end = op.offset + op.size;
            if op.offset <= merged_end + self.max_gap {
                // Can merge
                if op_end > merged_end {
                    merged_end = op_end;
                }
                if op.offset < merged_offset {
                    merged_offset = op.offset;
                }
                pids.push(op.pid);
            } else {
                mergeable[i] = false;
            }
        }

        let total_size = merged_end - merged_offset;
        if total_size > self.max_size {
            return None;
        }

        let merged_count = mergeable.iter().filter(|&&m| m).count() as u32;
        if merged_count < self.threshold as u32 {
            return None;
        }

        // Remove merged operations
        let mut i = 0;
        ops.retain(|_| {
            let keep = !mergeable.get(i).copied().unwrap_or(false);
            i += 1;
            keep
        });

        self.total_coalesced += 1;
        self.total_operations_saved += (merged_count - 1) as u64;

        Some(CoalescedIo {
            fd,
            is_read,
            offset: merged_offset,
            total_size,
            merged_count,
            pids,
        })
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> (u64, u64) {
        (self.total_coalesced, self.total_operations_saved)
    }
}

// ============================================================================
// TRANSFORM ENGINE
// ============================================================================

/// The main transformation engine
#[repr(align(64))]
pub struct TransformEngine {
    /// Transformation rules
    rules: Vec<TransformRule>,
    /// Coalescing engine
    coalesce: CoalesceEngine,
    /// Total transformations applied
    total_transforms: u64,
    /// Total eliminations
    total_eliminations: u64,
    /// Total estimated savings (ns)
    total_savings_ns: u64,
    /// Per-syscall-type transform counters
    per_type_counts: BTreeMap<u8, u64>,
}

impl TransformEngine {
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
            coalesce: CoalesceEngine::new(),
            total_transforms: 0,
            total_eliminations: 0,
            total_savings_ns: 0,
            per_type_counts: BTreeMap::new(),
        }
    }

    /// Transform a syscall
    pub fn transform(
        &mut self,
        syscall_type: SyscallType,
        pid: u64,
        fd: Option<i64>,
        offset: Option<u64>,
        size: Option<u64>,
    ) -> TransformedSyscall {
        let mut result = TransformedSyscall {
            original_type: syscall_type,
            new_type: syscall_type,
            transform: TransformType::Identity,
            rule_id: None,
            pid,
            fd,
            buffer_size: size,
            offset,
            prepend_ops: Vec::new(),
            eliminated: false,
            estimated_savings_ns: 0,
        };

        // Find matching rules sorted by priority
        let mut applicable: Vec<usize> = self
            .rules
            .iter()
            .enumerate()
            .filter(|(_, r)| r.enabled && r.input_type == syscall_type)
            .map(|(i, _)| i)
            .collect();
        applicable.sort_by_key(|&i| self.rules[i].priority);

        // Apply first matching rule
        if let Some(&rule_idx) = applicable.first() {
            let rule = &self.rules[rule_idx];
            result.transform = rule.transform_type;
            result.rule_id = Some(rule.id);

            match rule.transform_type {
                TransformType::Prefetch => {
                    // Add prefetch for sequential reads
                    if let (Some(fd_val), Some(off), Some(sz)) = (fd, offset, size) {
                        result.prepend_ops.push(PrependOp {
                            op_type: PrependOpType::Prefetch,
                            fd: fd_val,
                            offset: off + sz,
                            size: sz * 4, // Prefetch 4x ahead
                        });
                        result.estimated_savings_ns = 500_000; // ~500µs
                    }
                },
                TransformType::Coalesce => {
                    // Submit to coalesce engine
                    if let (Some(fd_val), Some(off), Some(sz)) = (fd, offset, size) {
                        let is_read = matches!(syscall_type, SyscallType::Read);
                        self.coalesce.submit(PendingIo {
                            pid,
                            fd: fd_val,
                            is_read,
                            offset: off,
                            size: sz,
                            submitted_at: 0,
                        });
                    }
                },
                TransformType::AsyncConvert => {
                    result.estimated_savings_ns = 200_000; // ~200µs
                },
                TransformType::Eliminate => {
                    result.eliminated = true;
                    result.estimated_savings_ns = 1_000_000; // ~1ms
                    self.total_eliminations += 1;
                },
                _ => {},
            }

            self.rules[rule_idx].apply_count += 1;
            self.total_transforms += 1;
            self.total_savings_ns += result.estimated_savings_ns;
        }

        *self.per_type_counts.entry(syscall_type as u8).or_insert(0) += 1;

        result
    }

    /// Add a custom rule
    #[inline(always)]
    pub fn add_rule(&mut self, rule: TransformRule) {
        self.rules.push(rule);
        self.rules.sort_by_key(|r| r.priority);
    }

    /// Get statistics
    #[inline]
    pub fn stats(&self) -> (u64, u64, u64) {
        (
            self.total_transforms,
            self.total_eliminations,
            self.total_savings_ns,
        )
    }

    /// Get the coalesce engine
    #[inline(always)]
    pub fn coalesce_engine(&mut self) -> &mut CoalesceEngine {
        &mut self.coalesce
    }

    fn default_rules() -> Vec<TransformRule> {
        vec![
            TransformRule::new(
                1,
                "prefetch_seq_read",
                SyscallType::Read,
                TransformType::Prefetch,
            )
            .with_priority(2),
            TransformRule::new(
                2,
                "coalesce_writes",
                SyscallType::Write,
                TransformType::Coalesce,
            )
            .with_priority(3),
            TransformRule::new(
                3,
                "async_connect",
                SyscallType::Connect,
                TransformType::AsyncConvert,
            )
            .with_priority(4),
            TransformRule::new(
                4,
                "prefetch_mmap",
                SyscallType::Mmap,
                TransformType::Prefetch,
            )
            .with_priority(3),
        ]
    }
}
