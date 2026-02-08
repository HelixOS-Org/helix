//! # Cooperative Checkpoint Protocol
//!
//! Cooperative checkpointing for process state:
//! - Coordinated checkpoints
//! - Incremental checkpointing
//! - Consistent global snapshots
//! - Rollback coordination
//! - Checkpoint scheduling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CHECKPOINT TYPES
// ============================================================================

/// Checkpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointType {
    /// Full checkpoint (all state)
    Full,
    /// Incremental (changes since last)
    Incremental,
    /// Coordinated (multi-process)
    Coordinated,
    /// On-demand
    OnDemand,
}

/// Checkpoint state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointState {
    /// Initiating
    Initiating,
    /// Collecting participant responses
    Collecting,
    /// In progress
    InProgress,
    /// Complete
    Complete,
    /// Failed
    Failed,
    /// Rolled back
    RolledBack,
}

/// Participant state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantState {
    /// Not yet responded
    Pending,
    /// Ready to checkpoint
    Ready,
    /// Checkpointing
    Checkpointing,
    /// Done
    Done,
    /// Refused
    Refused,
}

// ============================================================================
// CHECKPOINT DATA
// ============================================================================

/// Process checkpoint data
#[derive(Debug, Clone)]
pub struct ProcessCheckpoint {
    /// Process id
    pub pid: u64,
    /// Checkpoint size (bytes)
    pub size: u64,
    /// Pages checkpointed
    pub pages: u64,
    /// Dirty pages (for incremental)
    pub dirty_pages: u64,
    /// Register state size
    pub register_size: u64,
    /// Duration to create (ns)
    pub duration_ns: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl ProcessCheckpoint {
    pub fn new(pid: u64, now: u64) -> Self {
        Self {
            pid,
            size: 0,
            pages: 0,
            dirty_pages: 0,
            register_size: 0,
            duration_ns: 0,
            timestamp: now,
        }
    }

    /// Total size
    pub fn total_size(&self) -> u64 {
        self.size + self.register_size
    }
}

// ============================================================================
// COORDINATED CHECKPOINT
// ============================================================================

/// A coordinated checkpoint across processes
#[derive(Debug)]
pub struct CoordinatedCheckpoint {
    /// Checkpoint id
    pub id: u64,
    /// Type
    pub checkpoint_type: CheckpointType,
    /// State
    pub state: CheckpointState,
    /// Participants and their states
    pub participants: BTreeMap<u64, ParticipantState>,
    /// Process checkpoints
    pub data: BTreeMap<u64, ProcessCheckpoint>,
    /// Initiated at
    pub initiated_at: u64,
    /// Completed at
    pub completed_at: Option<u64>,
    /// Timeout (ns)
    pub timeout_ns: u64,
    /// Sequence number
    pub sequence: u64,
}

impl CoordinatedCheckpoint {
    pub fn new(id: u64, checkpoint_type: CheckpointType, sequence: u64, now: u64) -> Self {
        Self {
            id,
            checkpoint_type,
            state: CheckpointState::Initiating,
            participants: BTreeMap::new(),
            data: BTreeMap::new(),
            initiated_at: now,
            completed_at: None,
            timeout_ns: 5_000_000_000, // 5 seconds
            sequence,
        }
    }

    /// Add participant
    pub fn add_participant(&mut self, pid: u64) {
        self.participants.insert(pid, ParticipantState::Pending);
    }

    /// Record participant ready
    pub fn participant_ready(&mut self, pid: u64) -> bool {
        if let Some(state) = self.participants.get_mut(&pid) {
            *state = ParticipantState::Ready;
            // Check if all ready
            if self.all_ready() {
                self.state = CheckpointState::InProgress;
            }
            true
        } else {
            false
        }
    }

    /// Record participant refused
    pub fn participant_refused(&mut self, pid: u64) {
        if let Some(state) = self.participants.get_mut(&pid) {
            *state = ParticipantState::Refused;
        }
        // If any refused, fail the checkpoint
        self.state = CheckpointState::Failed;
    }

    /// Record participant done
    pub fn participant_done(&mut self, pid: u64, data: ProcessCheckpoint, now: u64) -> bool {
        if let Some(state) = self.participants.get_mut(&pid) {
            *state = ParticipantState::Done;
            self.data.insert(pid, data);
            // Check if all done
            if self.all_done() {
                self.state = CheckpointState::Complete;
                self.completed_at = Some(now);
            }
            true
        } else {
            false
        }
    }

    /// All participants ready?
    pub fn all_ready(&self) -> bool {
        !self.participants.is_empty()
            && self
                .participants
                .values()
                .all(|s| *s == ParticipantState::Ready || *s == ParticipantState::Done)
    }

    /// All participants done?
    pub fn all_done(&self) -> bool {
        !self.participants.is_empty()
            && self
                .participants
                .values()
                .all(|s| *s == ParticipantState::Done)
    }

    /// Check timeout
    pub fn check_timeout(&mut self, now: u64) -> bool {
        if now > self.initiated_at + self.timeout_ns
            && self.state != CheckpointState::Complete
            && self.state != CheckpointState::Failed
        {
            self.state = CheckpointState::Failed;
            return true;
        }
        false
    }

    /// Total checkpoint size
    pub fn total_size(&self) -> u64 {
        self.data.values().map(|d| d.total_size()).sum()
    }

    /// Duration
    pub fn duration_ns(&self) -> Option<u64> {
        self.completed_at.map(|c| c.saturating_sub(self.initiated_at))
    }

    /// Participant count
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }
}

// ============================================================================
// CHECKPOINT SCHEDULER
// ============================================================================

/// Checkpoint schedule policy
#[derive(Debug, Clone)]
pub struct CheckpointSchedule {
    /// Interval between checkpoints (ns)
    pub interval_ns: u64,
    /// Use incremental?
    pub incremental: bool,
    /// Full checkpoint every N incrementals
    pub full_every: u32,
    /// Max checkpoint size
    pub max_size: u64,
}

impl CheckpointSchedule {
    pub fn default_schedule() -> Self {
        Self {
            interval_ns: 60_000_000_000, // 60 seconds
            incremental: true,
            full_every: 10,
            max_size: 1024 * 1024 * 1024, // 1 GB
        }
    }
}

// ============================================================================
// CHECKPOINT MANAGER
// ============================================================================

/// Checkpoint stats
#[derive(Debug, Clone, Default)]
pub struct CoopCheckpointStats {
    /// Total checkpoints
    pub total: u64,
    /// Successful
    pub successful: u64,
    /// Failed
    pub failed: u64,
    /// Total data checkpointed (bytes)
    pub total_bytes: u64,
    /// Average duration (ns)
    pub avg_duration_ns: u64,
}

/// Cooperative checkpoint manager
pub struct CoopCheckpointManager {
    /// Checkpoints
    checkpoints: BTreeMap<u64, CoordinatedCheckpoint>,
    /// Schedule
    schedule: CheckpointSchedule,
    /// Next id
    next_id: u64,
    /// Sequence counter
    sequence: u64,
    /// Stats
    stats: CoopCheckpointStats,
    /// Duration sum
    duration_sum: u64,
    /// Last checkpoint time
    last_checkpoint: u64,
    /// Incremental counter
    incremental_count: u32,
}

impl CoopCheckpointManager {
    pub fn new() -> Self {
        Self {
            checkpoints: BTreeMap::new(),
            schedule: CheckpointSchedule::default_schedule(),
            next_id: 1,
            sequence: 0,
            stats: CoopCheckpointStats::default(),
            duration_sum: 0,
            last_checkpoint: 0,
            incremental_count: 0,
        }
    }

    /// Set schedule
    pub fn set_schedule(&mut self, schedule: CheckpointSchedule) {
        self.schedule = schedule;
    }

    /// Initiate checkpoint
    pub fn initiate(&mut self, participants: &[u64], now: u64) -> u64 {
        self.sequence += 1;
        let id = self.next_id;
        self.next_id += 1;

        // Decide full vs incremental
        let ctype = if self.schedule.incremental && self.incremental_count < self.schedule.full_every
        {
            self.incremental_count += 1;
            CheckpointType::Incremental
        } else {
            self.incremental_count = 0;
            CheckpointType::Full
        };

        let ctype = if participants.len() > 1 {
            CheckpointType::Coordinated
        } else {
            ctype
        };

        let mut checkpoint = CoordinatedCheckpoint::new(id, ctype, self.sequence, now);
        for &pid in participants {
            checkpoint.add_participant(pid);
        }
        checkpoint.state = CheckpointState::Collecting;
        self.checkpoints.insert(id, checkpoint);
        self.stats.total += 1;
        self.last_checkpoint = now;
        id
    }

    /// Record participant ready
    pub fn ready(&mut self, checkpoint_id: u64, pid: u64) -> bool {
        if let Some(cp) = self.checkpoints.get_mut(&checkpoint_id) {
            cp.participant_ready(pid)
        } else {
            false
        }
    }

    /// Record participant done
    pub fn done(
        &mut self,
        checkpoint_id: u64,
        pid: u64,
        data: ProcessCheckpoint,
        now: u64,
    ) -> bool {
        let result = if let Some(cp) = self.checkpoints.get_mut(&checkpoint_id) {
            cp.participant_done(pid, data, now)
        } else {
            false
        };

        // Update stats if checkpoint complete
        if let Some(cp) = self.checkpoints.get(&checkpoint_id) {
            if cp.state == CheckpointState::Complete {
                self.stats.successful += 1;
                self.stats.total_bytes += cp.total_size();
                if let Some(dur) = cp.duration_ns() {
                    self.duration_sum += dur;
                    self.stats.avg_duration_ns =
                        self.duration_sum / self.stats.successful;
                }
            }
        }
        result
    }

    /// Tick: check timeouts, decide if new checkpoint needed
    pub fn tick(&mut self, now: u64) -> Option<bool> {
        // Check timeouts
        for cp in self.checkpoints.values_mut() {
            if cp.check_timeout(now) {
                self.stats.failed += 1;
            }
        }

        // Should we initiate a new checkpoint?
        if now > self.last_checkpoint + self.schedule.interval_ns {
            return Some(true);
        }
        Some(false)
    }

    /// Get checkpoint
    pub fn checkpoint(&self, id: u64) -> Option<&CoordinatedCheckpoint> {
        self.checkpoints.get(&id)
    }

    /// Stats
    pub fn stats(&self) -> &CoopCheckpointStats {
        &self.stats
    }
}
