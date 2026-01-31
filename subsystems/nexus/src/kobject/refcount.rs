//! Reference Count Analyzer
//!
//! Tracking and analyzing reference count operations.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::KobjectId;

/// Reference operation
#[derive(Debug, Clone)]
pub struct RefOperation {
    /// Kobject ID
    pub kobject: KobjectId,
    /// Operation type
    pub op_type: RefOpType,
    /// New refcount
    pub new_count: u32,
    /// Caller location
    pub caller: String,
    /// Timestamp
    pub timestamp: u64,
}

/// Reference operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefOpType {
    /// kobject_get
    Get,
    /// kobject_put
    Put,
    /// Initial reference
    Init,
    /// Final release
    Release,
}

/// Reference leak info
#[derive(Debug, Clone)]
pub struct RefLeak {
    /// Kobject ID
    pub kobject: KobjectId,
    /// Object name
    pub name: String,
    /// Expected refcount
    pub expected: u32,
    /// Actual refcount
    pub actual: u32,
    /// Unbalanced gets
    pub unbalanced_gets: Vec<String>,
    /// Detection timestamp
    pub detected_at: u64,
}

/// Reference counting analyzer
pub struct RefCountAnalyzer {
    /// Reference operations
    operations: Vec<RefOperation>,
    /// Maximum operations to track
    max_operations: usize,
    /// Per-kobject reference history
    ref_history: BTreeMap<KobjectId, Vec<RefOperation>>,
    /// Detected leaks
    leaks: Vec<RefLeak>,
    /// Total get operations
    total_gets: AtomicU64,
    /// Total put operations
    total_puts: AtomicU64,
    /// Underflow detections
    underflows: AtomicU64,
}

impl RefCountAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            operations: Vec::with_capacity(10000),
            max_operations: 10000,
            ref_history: BTreeMap::new(),
            leaks: Vec::new(),
            total_gets: AtomicU64::new(0),
            total_puts: AtomicU64::new(0),
            underflows: AtomicU64::new(0),
        }
    }

    /// Record get operation
    pub fn record_get(
        &mut self,
        kobject: KobjectId,
        new_count: u32,
        caller: String,
        timestamp: u64,
    ) {
        self.total_gets.fetch_add(1, Ordering::Relaxed);
        self.record_op(kobject, RefOpType::Get, new_count, caller, timestamp);
    }

    /// Record put operation
    pub fn record_put(
        &mut self,
        kobject: KobjectId,
        new_count: u32,
        caller: String,
        timestamp: u64,
    ) {
        self.total_puts.fetch_add(1, Ordering::Relaxed);

        if new_count == u32::MAX {
            self.underflows.fetch_add(1, Ordering::Relaxed);
        }

        self.record_op(kobject, RefOpType::Put, new_count, caller, timestamp);
    }

    /// Record operation
    fn record_op(
        &mut self,
        kobject: KobjectId,
        op_type: RefOpType,
        new_count: u32,
        caller: String,
        timestamp: u64,
    ) {
        let op = RefOperation {
            kobject,
            op_type,
            new_count,
            caller,
            timestamp,
        };

        if self.operations.len() >= self.max_operations {
            self.operations.remove(0);
        }
        self.operations.push(op.clone());

        self.ref_history.entry(kobject).or_default().push(op);
    }

    /// Analyze kobject for leaks
    pub fn analyze_kobject(
        &mut self,
        kobject: KobjectId,
        name: &str,
        current_count: u32,
        expected_count: u32,
        timestamp: u64,
    ) {
        if current_count != expected_count {
            let history = self.ref_history.get(&kobject);
            let unbalanced_gets: Vec<String> = history
                .map(|ops| {
                    ops.iter()
                        .filter(|op| op.op_type == RefOpType::Get)
                        .map(|op| op.caller.clone())
                        .collect()
                })
                .unwrap_or_default();

            let leak = RefLeak {
                kobject,
                name: String::from(name),
                expected: expected_count,
                actual: current_count,
                unbalanced_gets,
                detected_at: timestamp,
            };
            self.leaks.push(leak);
        }
    }

    /// Get detected leaks
    pub fn get_leaks(&self) -> &[RefLeak] {
        &self.leaks
    }

    /// Get refcount history for kobject
    pub fn get_history(&self, kobject: KobjectId) -> Option<&[RefOperation]> {
        self.ref_history.get(&kobject).map(|v| v.as_slice())
    }

    /// Get total gets
    pub fn total_gets(&self) -> u64 {
        self.total_gets.load(Ordering::Relaxed)
    }

    /// Get total puts
    pub fn total_puts(&self) -> u64 {
        self.total_puts.load(Ordering::Relaxed)
    }

    /// Get underflow count
    pub fn underflow_count(&self) -> u64 {
        self.underflows.load(Ordering::Relaxed)
    }

    /// Clear history for kobject
    pub fn clear_history(&mut self, kobject: KobjectId) {
        self.ref_history.remove(&kobject);
    }
}

impl Default for RefCountAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
