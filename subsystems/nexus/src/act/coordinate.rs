//! # Action Coordination
//!
//! Coordinates multiple actions and ensures consistency.
//! Implements distributed coordination patterns.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// COORDINATION TYPES
// ============================================================================

/// Coordinated action
#[derive(Debug, Clone)]
pub struct CoordinatedAction {
    /// Action ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Participants
    pub participants: Vec<u64>,
    /// Phase
    pub phase: ActionPhase,
    /// Votes
    pub votes: BTreeMap<u64, Vote>,
    /// Created
    pub created: Timestamp,
    /// Deadline
    pub deadline: Option<Timestamp>,
}

/// Action phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionPhase {
    Preparing,
    Voting,
    Committing,
    Aborting,
    Completed,
    Failed,
}

/// Vote
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vote {
    Prepare,
    Commit,
    Abort,
    Timeout,
}

/// Participant
#[derive(Debug, Clone)]
pub struct Participant {
    /// Participant ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Status
    pub status: ParticipantStatus,
    /// Resources
    pub resources: Vec<String>,
    /// Last seen
    pub last_seen: Timestamp,
}

/// Participant status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantStatus {
    Active,
    Busy,
    Waiting,
    Offline,
}

/// Coordination result
#[derive(Debug, Clone)]
pub struct CoordinationResult {
    /// Action ID
    pub action_id: u64,
    /// Success
    pub success: bool,
    /// Final phase
    pub final_phase: ActionPhase,
    /// Participant results
    pub participant_results: BTreeMap<u64, ParticipantResult>,
    /// Duration
    pub duration_ms: u64,
}

/// Participant result
#[derive(Debug, Clone)]
pub struct ParticipantResult {
    /// Participant ID
    pub participant_id: u64,
    /// Vote
    pub vote: Vote,
    /// Committed
    pub committed: bool,
    /// Error
    pub error: Option<String>,
}

/// Lock
#[derive(Debug, Clone)]
pub struct Lock {
    /// Lock ID
    pub id: u64,
    /// Resource
    pub resource: String,
    /// Owner
    pub owner: u64,
    /// Acquired
    pub acquired: Timestamp,
    /// Expires
    pub expires: Option<Timestamp>,
}

/// Barrier
#[derive(Debug, Clone)]
pub struct Barrier {
    /// Barrier ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Required count
    pub required: usize,
    /// Current count
    pub arrived: Vec<u64>,
    /// Released
    pub released: bool,
}

// ============================================================================
// COORDINATOR
// ============================================================================

/// Coordinator
pub struct Coordinator {
    /// Actions
    actions: BTreeMap<u64, CoordinatedAction>,
    /// Participants
    participants: BTreeMap<u64, Participant>,
    /// Locks
    locks: BTreeMap<String, Lock>,
    /// Barriers
    barriers: BTreeMap<u64, Barrier>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: CoordinatorConfig,
    /// Statistics
    stats: CoordinatorStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Default timeout (ms)
    pub default_timeout_ms: u64,
    /// Require all votes
    pub require_all_votes: bool,
    /// Lock timeout (ms)
    pub lock_timeout_ms: u64,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 5000,
            require_all_votes: true,
            lock_timeout_ms: 10000,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoordinatorStats {
    /// Actions initiated
    pub actions_initiated: u64,
    /// Actions committed
    pub actions_committed: u64,
    /// Actions aborted
    pub actions_aborted: u64,
    /// Locks acquired
    pub locks_acquired: u64,
}

impl Coordinator {
    /// Create new coordinator
    pub fn new(config: CoordinatorConfig) -> Self {
        Self {
            actions: BTreeMap::new(),
            participants: BTreeMap::new(),
            locks: BTreeMap::new(),
            barriers: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: CoordinatorStats::default(),
        }
    }

    /// Register participant
    pub fn register_participant(&mut self, name: &str, resources: Vec<String>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let participant = Participant {
            id,
            name: name.into(),
            status: ParticipantStatus::Active,
            resources,
            last_seen: Timestamp::now(),
        };

        self.participants.insert(id, participant);

        id
    }

    /// Begin coordinated action
    pub fn begin_action(&mut self, name: &str, participant_ids: Vec<u64>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let action = CoordinatedAction {
            id,
            name: name.into(),
            participants: participant_ids,
            phase: ActionPhase::Preparing,
            votes: BTreeMap::new(),
            created: now,
            deadline: Some(Timestamp(now.0 + self.config.default_timeout_ms)),
        };

        self.actions.insert(id, action);
        self.stats.actions_initiated += 1;

        id
    }

    /// Prepare phase
    pub fn prepare(&mut self, action_id: u64) -> bool {
        if let Some(action) = self.actions.get_mut(&action_id) {
            if action.phase != ActionPhase::Preparing {
                return false;
            }

            action.phase = ActionPhase::Voting;
            true
        } else {
            false
        }
    }

    /// Cast vote
    pub fn vote(&mut self, action_id: u64, participant_id: u64, vote: Vote) -> bool {
        if let Some(action) = self.actions.get_mut(&action_id) {
            if action.phase != ActionPhase::Voting {
                return false;
            }

            if !action.participants.contains(&participant_id) {
                return false;
            }

            action.votes.insert(participant_id, vote);
            true
        } else {
            false
        }
    }

    /// Check if ready to commit
    pub fn can_commit(&self, action_id: u64) -> bool {
        if let Some(action) = self.actions.get(&action_id) {
            if action.phase != ActionPhase::Voting {
                return false;
            }

            // Check all participants voted
            let all_voted = action
                .participants
                .iter()
                .all(|p| action.votes.contains_key(p));

            if !all_voted && self.config.require_all_votes {
                return false;
            }

            // Check all voted commit
            action
                .votes
                .values()
                .all(|v| *v == Vote::Commit || *v == Vote::Prepare)
        } else {
            false
        }
    }

    /// Commit action
    pub fn commit(&mut self, action_id: u64) -> Option<CoordinationResult> {
        if !self.can_commit(action_id) {
            return self.abort(action_id);
        }

        let action = self.actions.get_mut(&action_id)?;
        let start = action.created.0;

        action.phase = ActionPhase::Committing;

        // Collect results
        let mut participant_results = BTreeMap::new();
        for &p_id in &action.participants {
            let vote = action.votes.get(&p_id).unwrap_or(Vote::Timeout);
            participant_results.insert(p_id, ParticipantResult {
                participant_id: p_id,
                vote,
                committed: true,
                error: None,
            });
        }

        action.phase = ActionPhase::Completed;
        self.stats.actions_committed += 1;

        Some(CoordinationResult {
            action_id,
            success: true,
            final_phase: ActionPhase::Completed,
            participant_results,
            duration_ms: Timestamp::now().0 - start,
        })
    }

    /// Abort action
    pub fn abort(&mut self, action_id: u64) -> Option<CoordinationResult> {
        let action = self.actions.get_mut(&action_id)?;
        let start = action.created.0;

        action.phase = ActionPhase::Aborting;

        let mut participant_results = BTreeMap::new();
        for &p_id in &action.participants {
            let vote = action.votes.get(&p_id).unwrap_or(Vote::Timeout);
            participant_results.insert(p_id, ParticipantResult {
                participant_id: p_id,
                vote,
                committed: false,
                error: Some("Aborted".into()),
            });
        }

        action.phase = ActionPhase::Failed;
        self.stats.actions_aborted += 1;

        Some(CoordinationResult {
            action_id,
            success: false,
            final_phase: ActionPhase::Failed,
            participant_results,
            duration_ms: Timestamp::now().0 - start,
        })
    }

    /// Acquire lock
    pub fn acquire_lock(&mut self, resource: &str, owner: u64) -> Option<u64> {
        // Check if already locked
        if let Some(lock) = self.locks.get(resource) {
            // Check expiry
            if let Some(expires) = lock.expires {
                if Timestamp::now().0 < expires.0 {
                    return None; // Still locked
                }
            } else {
                return None;
            }
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let lock = Lock {
            id,
            resource: resource.into(),
            owner,
            acquired: now,
            expires: Some(Timestamp(now.0 + self.config.lock_timeout_ms)),
        };

        self.locks.insert(resource.into(), lock);
        self.stats.locks_acquired += 1;

        Some(id)
    }

    /// Release lock
    #[inline]
    pub fn release_lock(&mut self, resource: &str, owner: u64) -> bool {
        if let Some(lock) = self.locks.get(resource) {
            if lock.owner == owner {
                self.locks.remove(resource);
                return true;
            }
        }
        false
    }

    /// Create barrier
    pub fn create_barrier(&mut self, name: &str, required: usize) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let barrier = Barrier {
            id,
            name: name.into(),
            required,
            arrived: Vec::new(),
            released: false,
        };

        self.barriers.insert(id, barrier);

        id
    }

    /// Arrive at barrier
    pub fn arrive_at_barrier(&mut self, barrier_id: u64, participant_id: u64) -> bool {
        if let Some(barrier) = self.barriers.get_mut(&barrier_id) {
            if barrier.released {
                return true; // Already released
            }

            if !barrier.arrived.contains(&participant_id) {
                barrier.arrived.push(participant_id);
            }

            if barrier.arrived.len() >= barrier.required {
                barrier.released = true;
                return true;
            }

            false
        } else {
            false
        }
    }

    /// Check if barrier released
    #[inline(always)]
    pub fn is_barrier_released(&self, barrier_id: u64) -> bool {
        self.barriers.get(&barrier_id).map_or(false, |b| b.released)
    }

    /// Get action
    #[inline(always)]
    pub fn get_action(&self, id: u64) -> Option<&CoordinatedAction> {
        self.actions.get(&id)
    }

    /// Get participant
    #[inline(always)]
    pub fn get_participant(&self, id: u64) -> Option<&Participant> {
        self.participants.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &CoordinatorStats {
        &self.stats
    }
}

impl Default for Coordinator {
    fn default() -> Self {
        Self::new(CoordinatorConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_participant() {
        let mut coord = Coordinator::default();

        let id = coord.register_participant("worker1", vec!["cpu".into()]);
        assert!(coord.get_participant(id).is_some());
    }

    #[test]
    fn test_coordinated_action() {
        let mut coord = Coordinator::default();

        let p1 = coord.register_participant("p1", vec![]);
        let p2 = coord.register_participant("p2", vec![]);

        let action = coord.begin_action("test", vec![p1, p2]);

        coord.prepare(action);
        coord.vote(action, p1, Vote::Commit);
        coord.vote(action, p2, Vote::Commit);

        assert!(coord.can_commit(action));

        let result = coord.commit(action);
        assert!(result.is_some());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_abort_on_no_vote() {
        let mut coord = Coordinator::default();

        let p1 = coord.register_participant("p1", vec![]);
        let p2 = coord.register_participant("p2", vec![]);

        let action = coord.begin_action("test", vec![p1, p2]);

        coord.prepare(action);
        coord.vote(action, p1, Vote::Commit);
        coord.vote(action, p2, Vote::Abort);

        assert!(!coord.can_commit(action));

        let result = coord.abort(action);
        assert!(result.is_some());
        assert!(!result.unwrap().success);
    }

    #[test]
    fn test_lock() {
        let mut coord = Coordinator::default();

        let owner1 = coord.register_participant("owner1", vec![]);
        let owner2 = coord.register_participant("owner2", vec![]);

        // First acquire succeeds
        let lock = coord.acquire_lock("resource", owner1);
        assert!(lock.is_some());

        // Second acquire fails
        let lock2 = coord.acquire_lock("resource", owner2);
        assert!(lock2.is_none());

        // Release
        assert!(coord.release_lock("resource", owner1));

        // Now can acquire
        let lock3 = coord.acquire_lock("resource", owner2);
        assert!(lock3.is_some());
    }

    #[test]
    fn test_barrier() {
        let mut coord = Coordinator::default();

        let p1 = coord.register_participant("p1", vec![]);
        let p2 = coord.register_participant("p2", vec![]);
        let p3 = coord.register_participant("p3", vec![]);

        let barrier = coord.create_barrier("sync", 3);

        assert!(!coord.arrive_at_barrier(barrier, p1));
        assert!(!coord.arrive_at_barrier(barrier, p2));
        assert!(coord.arrive_at_barrier(barrier, p3)); // Released

        assert!(coord.is_barrier_released(barrier));
    }
}
