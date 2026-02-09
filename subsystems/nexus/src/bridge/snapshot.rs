//! # Bridge Snapshot Engine
//!
//! Syscall state snapshots for debugging and replay:
//! - Point-in-time state capture
//! - Differential snapshots
//! - Snapshot comparison
//! - State restoration
//! - History navigation

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SNAPSHOT TYPES
// ============================================================================

/// Snapshot scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotScope {
    /// Full system state
    System,
    /// Per-process
    Process,
    /// Per-syscall
    Syscall,
    /// Memory state
    Memory,
    /// Fd table
    FdTable,
}

/// Snapshot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotState {
    /// Creating
    Creating,
    /// Complete
    Complete,
    /// Corrupted
    Corrupted,
    /// Archived
    Archived,
}

/// Register state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct RegisterState {
    /// General purpose registers
    pub gpr: [u64; 16],
    /// Instruction pointer
    pub rip: u64,
    /// Stack pointer
    pub rsp: u64,
    /// Flags
    pub rflags: u64,
}

impl RegisterState {
    #[inline]
    pub fn empty() -> Self {
        Self {
            gpr: [0u64; 16],
            rip: 0,
            rsp: 0,
            rflags: 0,
        }
    }

    /// Diff with another state
    pub fn diff(&self, other: &RegisterState) -> Vec<(usize, u64, u64)> {
        let mut diffs = Vec::new();
        for i in 0..16 {
            if self.gpr[i] != other.gpr[i] {
                diffs.push((i, self.gpr[i], other.gpr[i]));
            }
        }
        if self.rip != other.rip {
            diffs.push((16, self.rip, other.rip));
        }
        if self.rsp != other.rsp {
            diffs.push((17, self.rsp, other.rsp));
        }
        if self.rflags != other.rflags {
            diffs.push((18, self.rflags, other.rflags));
        }
        diffs
    }
}

/// Memory region snapshot
#[derive(Debug, Clone)]
pub struct MemoryRegionSnapshot {
    /// Base address
    pub base: u64,
    /// Size
    pub size: usize,
    /// Hash of contents
    pub content_hash: u64,
    /// Permissions
    pub permissions: u32,
}

/// File descriptor snapshot
#[derive(Debug, Clone)]
pub struct FdSnapshot {
    /// Fd number
    pub fd: i32,
    /// File type
    pub fd_type: u8,
    /// Offset
    pub offset: u64,
    /// Flags
    pub flags: u32,
}

// ============================================================================
// PROCESS SNAPSHOT
// ============================================================================

/// Process state snapshot
#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    /// Process id
    pub pid: u64,
    /// Register state
    pub registers: RegisterState,
    /// Memory regions
    pub memory_regions: Vec<MemoryRegionSnapshot>,
    /// File descriptors
    pub fds: Vec<FdSnapshot>,
    /// Pending signals
    pub pending_signals: u64,
    /// Current syscall (if any)
    pub current_syscall: Option<u32>,
}

impl ProcessSnapshot {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            registers: RegisterState::empty(),
            memory_regions: Vec::new(),
            fds: Vec::new(),
            pending_signals: 0,
            current_syscall: None,
        }
    }

    /// Memory size
    #[inline(always)]
    pub fn total_memory(&self) -> usize {
        self.memory_regions.iter().map(|r| r.size).sum()
    }

    /// Fd count
    #[inline(always)]
    pub fn fd_count(&self) -> usize {
        self.fds.len()
    }
}

// ============================================================================
// SYSTEM SNAPSHOT
// ============================================================================

/// Full system snapshot
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeSnapshot {
    /// Snapshot id
    pub id: u64,
    /// Scope
    pub scope: SnapshotScope,
    /// State
    pub state: SnapshotState,
    /// Timestamp
    pub timestamp: u64,
    /// Process snapshots
    pub processes: BTreeMap<u64, ProcessSnapshot>,
    /// Global counters
    pub counters: ArrayMap<u64, 32>,
    /// Parent snapshot (for differential)
    pub parent_id: Option<u64>,
    /// Size estimate (bytes)
    pub size_bytes: u64,
}

impl BridgeSnapshot {
    pub fn new(id: u64, scope: SnapshotScope, now: u64) -> Self {
        Self {
            id,
            scope,
            state: SnapshotState::Creating,
            timestamp: now,
            processes: BTreeMap::new(),
            counters: ArrayMap::new(0),
            parent_id: None,
            size_bytes: 0,
        }
    }

    /// Add process
    #[inline]
    pub fn add_process(&mut self, snap: ProcessSnapshot) {
        let mem = snap.total_memory() as u64;
        self.size_bytes += mem;
        self.processes.insert(snap.pid, snap);
    }

    /// Complete
    #[inline(always)]
    pub fn complete(&mut self) {
        self.state = SnapshotState::Complete;
    }

    /// Process count
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}

/// Snapshot diff
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    /// Base snapshot id
    pub base_id: u64,
    /// Target snapshot id
    pub target_id: u64,
    /// New processes
    pub new_processes: Vec<u64>,
    /// Removed processes
    pub removed_processes: Vec<u64>,
    /// Modified processes
    pub modified_processes: Vec<u64>,
    /// Counter changes
    pub counter_changes: Vec<(u32, i64)>,
}

// ============================================================================
// SNAPSHOT MANAGER
// ============================================================================

/// Snapshot stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeSnapshotStats {
    /// Total snapshots
    pub total_snapshots: u64,
    /// Active snapshots
    pub active: usize,
    /// Total size
    pub total_size_bytes: u64,
    /// Diffs computed
    pub diffs_computed: u64,
}

/// Bridge snapshot manager
#[repr(align(64))]
pub struct BridgeSnapshotManager {
    /// Snapshots
    snapshots: BTreeMap<u64, BridgeSnapshot>,
    /// Max snapshots to retain
    max_snapshots: usize,
    /// Next id
    next_id: u64,
    /// Stats
    stats: BridgeSnapshotStats,
}

impl BridgeSnapshotManager {
    pub fn new() -> Self {
        Self {
            snapshots: BTreeMap::new(),
            max_snapshots: 64,
            next_id: 1,
            stats: BridgeSnapshotStats::default(),
        }
    }

    /// Create new snapshot
    pub fn create(&mut self, scope: SnapshotScope, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let snap = BridgeSnapshot::new(id, scope, now);
        self.snapshots.insert(id, snap);
        self.stats.total_snapshots += 1;
        self.stats.active = self.snapshots.len();

        // Evict old if needed
        while self.snapshots.len() > self.max_snapshots {
            if let Some((&oldest, _)) = self.snapshots.iter().next() {
                self.snapshots.remove(&oldest);
            }
        }

        id
    }

    /// Add process to snapshot
    #[inline]
    pub fn add_process(&mut self, snap_id: u64, process: ProcessSnapshot) {
        if let Some(snap) = self.snapshots.get_mut(&snap_id) {
            snap.add_process(process);
        }
    }

    /// Complete snapshot
    #[inline]
    pub fn complete(&mut self, snap_id: u64) {
        if let Some(snap) = self.snapshots.get_mut(&snap_id) {
            snap.complete();
            self.stats.total_size_bytes += snap.size_bytes;
        }
    }

    /// Get snapshot
    #[inline(always)]
    pub fn get(&self, snap_id: u64) -> Option<&BridgeSnapshot> {
        self.snapshots.get(&snap_id)
    }

    /// Compute diff between two snapshots
    pub fn diff(&mut self, base_id: u64, target_id: u64) -> Option<SnapshotDiff> {
        let base = self.snapshots.get(&base_id)?;
        let target = self.snapshots.get(&target_id)?;

        let base_pids: Vec<u64> = base.processes.keys().copied().collect();
        let target_pids: Vec<u64> = target.processes.keys().copied().collect();

        let new_processes: Vec<u64> = target_pids
            .iter()
            .filter(|p| !base_pids.contains(p))
            .copied()
            .collect();
        let removed_processes: Vec<u64> = base_pids
            .iter()
            .filter(|p| !target_pids.contains(p))
            .copied()
            .collect();

        let mut modified_processes = Vec::new();
        for &pid in &target_pids {
            if base.processes.contains_key(&pid) {
                // Check if memory changed
                let b = &base.processes[&pid];
                let t = &target.processes[&pid];
                if b.registers.rip != t.registers.rip
                    || b.memory_regions.len() != t.memory_regions.len()
                    || b.fds.len() != t.fds.len()
                {
                    modified_processes.push(pid);
                }
            }
        }

        let mut counter_changes = Vec::new();
        for (&key, &target_val) in &target.counters {
            let base_val = base.counters.get(&key).copied().unwrap_or(0);
            if target_val != base_val {
                counter_changes.push((key, target_val as i64 - base_val as i64));
            }
        }

        self.stats.diffs_computed += 1;

        Some(SnapshotDiff {
            base_id,
            target_id,
            new_processes,
            removed_processes,
            modified_processes,
            counter_changes,
        })
    }

    /// List snapshots
    #[inline]
    pub fn list(&self) -> Vec<(u64, SnapshotScope, SnapshotState, u64)> {
        self.snapshots
            .values()
            .map(|s| (s.id, s.scope, s.state, s.timestamp))
            .collect()
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &BridgeSnapshotStats {
        &self.stats
    }
}
