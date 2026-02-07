//! # Bridge Checkpoint Manager
//!
//! Syscall-level checkpoint/restore for deterministic replay:
//! - Checkpoint creation at syscall boundaries
//! - Incremental state capture
//! - Restore point management
//! - Process state serialization
//! - Checkpoint verification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CHECKPOINT TYPES
// ============================================================================

/// Checkpoint state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointState {
    /// Being created
    Creating,
    /// Valid and usable
    Valid,
    /// Being restored from
    Restoring,
    /// Invalidated (state changed)
    Invalidated,
    /// Expired
    Expired,
}

/// State component type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateComponent {
    /// Register state
    Registers,
    /// Memory mappings
    MemoryMappings,
    /// Open file descriptors
    FileDescriptors,
    /// Signal handlers
    SignalHandlers,
    /// IPC state
    IpcState,
    /// Timer state
    TimerState,
    /// Credential state
    Credentials,
}

/// Checkpoint trigger
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointTrigger {
    /// Manual request
    Manual,
    /// Periodic timer
    Periodic,
    /// Before dangerous syscall
    PreSyscall,
    /// After transaction
    PostTransaction,
    /// Memory threshold
    MemoryThreshold,
}

// ============================================================================
// STATE CAPTURE
// ============================================================================

/// Captured state fragment
#[derive(Debug, Clone)]
pub struct StateFragment {
    /// Component type
    pub component: StateComponent,
    /// Data hash (for verification)
    pub data_hash: u64,
    /// Size in bytes
    pub size_bytes: usize,
    /// Is incremental (delta from previous)
    pub incremental: bool,
    /// Base checkpoint ID (for incremental)
    pub base_id: Option<u64>,
}

impl StateFragment {
    pub fn new(component: StateComponent, data_hash: u64, size_bytes: usize) -> Self {
        Self {
            component,
            data_hash,
            size_bytes,
            incremental: false,
            base_id: None,
        }
    }

    /// Create incremental fragment
    pub fn incremental(
        component: StateComponent,
        data_hash: u64,
        size_bytes: usize,
        base_id: u64,
    ) -> Self {
        Self {
            component,
            data_hash,
            size_bytes,
            incremental: true,
            base_id: Some(base_id),
        }
    }
}

/// Checkpoint
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Unique ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// State
    pub state: CheckpointState,
    /// Trigger
    pub trigger: CheckpointTrigger,
    /// Creation timestamp
    pub created_ns: u64,
    /// State fragments
    pub fragments: Vec<StateFragment>,
    /// Total size (bytes)
    pub total_size: usize,
    /// Verification hash (FNV-1a of all fragment hashes)
    pub verification_hash: u64,
    /// Syscall number at checkpoint (if applicable)
    pub syscall_nr: Option<u32>,
    /// Description
    pub description: String,
}

impl Checkpoint {
    pub fn new(id: u64, pid: u64, trigger: CheckpointTrigger, now: u64) -> Self {
        Self {
            id,
            pid,
            state: CheckpointState::Creating,
            trigger,
            created_ns: now,
            fragments: Vec::new(),
            total_size: 0,
            verification_hash: 0xcbf29ce484222325,
            syscall_nr: None,
            description: String::new(),
        }
    }

    /// Add fragment
    pub fn add_fragment(&mut self, fragment: StateFragment) {
        self.total_size += fragment.size_bytes;
        // Update verification hash
        self.verification_hash ^= fragment.data_hash;
        self.verification_hash = self.verification_hash.wrapping_mul(0x100000001b3);
        self.fragments.push(fragment);
    }

    /// Finalize
    pub fn finalize(&mut self) {
        self.state = CheckpointState::Valid;
    }

    /// Invalidate
    pub fn invalidate(&mut self) {
        self.state = CheckpointState::Invalidated;
    }

    /// Verify integrity
    pub fn verify(&self) -> bool {
        let mut hash: u64 = 0xcbf29ce484222325;
        for f in &self.fragments {
            hash ^= f.data_hash;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash == self.verification_hash
    }

    /// Fragment count by component
    pub fn fragment_count(&self, component: StateComponent) -> usize {
        self.fragments
            .iter()
            .filter(|f| f.component == component)
            .count()
    }
}

// ============================================================================
// RESTORE PLAN
// ============================================================================

/// Restore plan
#[derive(Debug, Clone)]
pub struct RestorePlan {
    /// Checkpoint chain (from base to target)
    pub checkpoint_chain: Vec<u64>,
    /// Total data to restore
    pub total_bytes: usize,
    /// Estimated restore time (ns)
    pub estimated_time_ns: u64,
    /// Components to restore
    pub components: Vec<StateComponent>,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Checkpoint stats
#[derive(Debug, Clone, Default)]
pub struct BridgeCheckpointStats {
    /// Total checkpoints created
    pub checkpoints_created: u64,
    /// Valid checkpoints
    pub valid_checkpoints: usize,
    /// Restores performed
    pub restores: u64,
    /// Total checkpoint storage (bytes)
    pub total_storage: usize,
    /// Processes tracked
    pub tracked_processes: usize,
}

/// Process checkpoint history
#[derive(Debug)]
struct ProcessCheckpoints {
    /// Ordered checkpoints
    checkpoints: Vec<Checkpoint>,
    /// Max checkpoints per process
    max_checkpoints: usize,
}

impl ProcessCheckpoints {
    fn new(max: usize) -> Self {
        Self {
            checkpoints: Vec::new(),
            max_checkpoints: max,
        }
    }

    fn add(&mut self, cp: Checkpoint) {
        if self.checkpoints.len() >= self.max_checkpoints {
            // Remove oldest
            self.checkpoints.remove(0);
        }
        self.checkpoints.push(cp);
    }

    fn latest(&self) -> Option<&Checkpoint> {
        self.checkpoints.last()
    }

    fn get(&self, id: u64) -> Option<&Checkpoint> {
        self.checkpoints.iter().find(|c| c.id == id)
    }

    fn get_mut(&mut self, id: u64) -> Option<&mut Checkpoint> {
        self.checkpoints.iter_mut().find(|c| c.id == id)
    }

    fn valid_count(&self) -> usize {
        self.checkpoints
            .iter()
            .filter(|c| c.state == CheckpointState::Valid)
            .count()
    }

    fn total_size(&self) -> usize {
        self.checkpoints.iter().map(|c| c.total_size).sum()
    }
}

/// Bridge checkpoint manager
pub struct BridgeCheckpointManager {
    /// Per-process checkpoints
    processes: BTreeMap<u64, ProcessCheckpoints>,
    /// Next checkpoint ID
    next_id: u64,
    /// Max checkpoints per process
    max_per_process: usize,
    /// Stats
    stats: BridgeCheckpointStats,
}

impl BridgeCheckpointManager {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            next_id: 1,
            max_per_process: 32,
            stats: BridgeCheckpointStats::default(),
        }
    }

    /// Begin creating a checkpoint
    pub fn begin_checkpoint(&mut self, pid: u64, trigger: CheckpointTrigger, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let cp = Checkpoint::new(id, pid, trigger, now);
        let history = self
            .processes
            .entry(pid)
            .or_insert_with(|| ProcessCheckpoints::new(self.max_per_process));
        history.add(cp);
        self.stats.checkpoints_created += 1;
        self.update_stats();
        id
    }

    /// Add state fragment to checkpoint
    pub fn add_fragment(&mut self, pid: u64, checkpoint_id: u64, fragment: StateFragment) -> bool {
        if let Some(history) = self.processes.get_mut(&pid) {
            if let Some(cp) = history.get_mut(checkpoint_id) {
                if cp.state == CheckpointState::Creating {
                    cp.add_fragment(fragment);
                    return true;
                }
            }
        }
        false
    }

    /// Finalize checkpoint
    pub fn finalize_checkpoint(&mut self, pid: u64, checkpoint_id: u64) -> bool {
        if let Some(history) = self.processes.get_mut(&pid) {
            if let Some(cp) = history.get_mut(checkpoint_id) {
                if cp.state == CheckpointState::Creating {
                    cp.finalize();
                    self.update_stats();
                    return true;
                }
            }
        }
        false
    }

    /// Plan restore
    pub fn plan_restore(&self, pid: u64, checkpoint_id: u64) -> Option<RestorePlan> {
        let history = self.processes.get(&pid)?;
        let cp = history.get(checkpoint_id)?;
        if cp.state != CheckpointState::Valid {
            return None;
        }

        // Build chain for incremental restore
        let mut chain = Vec::new();
        let mut current_id = checkpoint_id;
        loop {
            chain.push(current_id);
            let current = history.get(current_id)?;
            // Check if any fragment is incremental
            let base = current.fragments.iter().find_map(|f| f.base_id);
            match base {
                Some(base_id) => current_id = base_id,
                None => break,
            }
        }
        chain.reverse();

        let total_bytes: usize = chain
            .iter()
            .filter_map(|id| history.get(*id))
            .map(|c| c.total_size)
            .sum();

        let components = alloc::vec![
            StateComponent::Registers,
            StateComponent::MemoryMappings,
            StateComponent::FileDescriptors,
            StateComponent::SignalHandlers,
            StateComponent::IpcState,
        ];

        Some(RestorePlan {
            checkpoint_chain: chain,
            total_bytes,
            estimated_time_ns: total_bytes as u64 * 10, // ~10ns per byte
            components,
        })
    }

    /// Verify checkpoint integrity
    pub fn verify(&self, pid: u64, checkpoint_id: u64) -> bool {
        self.processes
            .get(&pid)
            .and_then(|h| h.get(checkpoint_id))
            .map(|c| c.verify())
            .unwrap_or(false)
    }

    /// Invalidate all checkpoints for process (e.g., after state mutation)
    pub fn invalidate_all(&mut self, pid: u64) {
        if let Some(history) = self.processes.get_mut(&pid) {
            for cp in &mut history.checkpoints {
                if cp.state == CheckpointState::Valid {
                    cp.invalidate();
                }
            }
        }
        self.update_stats();
    }

    /// Cleanup old checkpoints
    pub fn cleanup(&mut self, max_age_ns: u64, now: u64) {
        for history in self.processes.values_mut() {
            history.checkpoints.retain(|cp| {
                now.saturating_sub(cp.created_ns) < max_age_ns || cp.state == CheckpointState::Valid
            });
        }
        self.processes.retain(|_, h| !h.checkpoints.is_empty());
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.valid_checkpoints = self.processes.values().map(|h| h.valid_count()).sum();
        self.stats.total_storage = self.processes.values().map(|h| h.total_size()).sum();
    }

    /// Stats
    pub fn stats(&self) -> &BridgeCheckpointStats {
        &self.stats
    }
}
