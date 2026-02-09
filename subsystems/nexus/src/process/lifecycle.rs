//! Process Lifecycle Manager
//!
//! Manages process lifecycle decisions.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;
use super::{ProcessId, ProcessProfile, ProcessState, ProcessType};

/// Process lifecycle state
#[derive(Debug, Clone)]
struct ProcessLifecycleState {
    /// Current state
    state: ProcessState,
    /// Start time
    start_time: NexusTimestamp,
    /// Last activity
    last_activity: NexusTimestamp,
    /// Importance score
    importance: f64,
}

/// Lifecycle event
#[derive(Debug, Clone)]
pub struct LifecycleEvent {
    /// Event type
    pub event_type: LifecycleEventType,
    /// Process ID
    pub pid: ProcessId,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Types of lifecycle events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEventType {
    /// Process created
    Created,
    /// Process started running
    Running,
    /// Process blocked
    Blocked,
    /// Process resumed
    Resumed,
    /// Process terminated
    Terminated,
    /// Process killed
    Killed,
}

/// Kill recommendation
#[derive(Debug, Clone)]
pub struct KillRecommendation {
    /// Process ID
    pub pid: ProcessId,
    /// Reason
    pub reason: String,
    /// Urgency (0.0 - 1.0)
    pub urgency: f64,
    /// Memory to be freed
    pub memory_freed: u64,
}

/// Manages process lifecycle decisions
pub struct LifecycleManager {
    /// Process states
    states: BTreeMap<ProcessId, ProcessLifecycleState>,
    /// Lifecycle events
    events: VecDeque<LifecycleEvent>,
    /// Max events
    max_events: usize,
    /// Kill recommendations
    kill_recommendations: Vec<KillRecommendation>,
}

impl LifecycleManager {
    /// Create new lifecycle manager
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            events: VecDeque::new(),
            max_events: 10000,
            kill_recommendations: Vec::new(),
        }
    }

    /// Record process creation
    #[inline]
    pub fn process_created(&mut self, pid: ProcessId) {
        let now = NexusTimestamp::now();
        self.states.insert(pid, ProcessLifecycleState {
            state: ProcessState::Ready,
            start_time: now,
            last_activity: now,
            importance: 0.5,
        });

        self.record_event(LifecycleEventType::Created, pid);
    }

    /// Update process state
    pub fn update_state(&mut self, pid: ProcessId, state: ProcessState) {
        if let Some(lifecycle) = self.states.get_mut(&pid) {
            lifecycle.state = state;
            lifecycle.last_activity = NexusTimestamp::now();

            let event = match state {
                ProcessState::Running => LifecycleEventType::Running,
                ProcessState::BlockedIo | ProcessState::BlockedSync | ProcessState::Sleeping => {
                    LifecycleEventType::Blocked
                },
                ProcessState::Ready => LifecycleEventType::Resumed,
                ProcessState::Stopped | ProcessState::Zombie => LifecycleEventType::Terminated,
            };

            self.record_event(event, pid);
        }
    }

    /// Set process importance
    #[inline]
    pub fn set_importance(&mut self, pid: ProcessId, importance: f64) {
        if let Some(lifecycle) = self.states.get_mut(&pid) {
            lifecycle.importance = importance.clamp(0.0, 1.0);
        }
    }

    /// Record lifecycle event
    fn record_event(&mut self, event_type: LifecycleEventType, pid: ProcessId) {
        self.events.push_back(LifecycleEvent {
            event_type,
            pid,
            timestamp: NexusTimestamp::now(),
        });

        if self.events.len() > self.max_events {
            self.events.pop_front();
        }
    }

    /// Find processes to kill for memory
    pub fn find_kill_candidates(
        &mut self,
        memory_needed: u64,
        profiles: &BTreeMap<ProcessId, ProcessProfile>,
    ) -> Vec<KillRecommendation> {
        self.kill_recommendations.clear();

        let mut candidates: Vec<_> = self
            .states
            .iter()
            .filter(|(pid, state)| {
                profiles
                    .get(pid)
                    .map(|p| p.process_type != ProcessType::System)
                    .unwrap_or(true)
                    && state.state != ProcessState::Zombie
            })
            .map(|(&pid, state)| {
                let profile = profiles.get(&pid);
                let memory = profile.map(|p| p.avg_memory as u64).unwrap_or(0);
                let importance = state.importance;

                (pid, memory, importance)
            })
            .collect();

        candidates.sort_by(|a, b| {
            let score_a = a.2 - (a.1 as f64 / 1_000_000_000.0);
            let score_b = b.2 - (b.1 as f64 / 1_000_000_000.0);
            score_a.partial_cmp(&score_b).unwrap_or(core::cmp::Ordering::Equal)
        });

        let mut total_freed = 0u64;
        for (pid, memory, importance) in candidates {
            if total_freed >= memory_needed {
                break;
            }

            self.kill_recommendations.push(KillRecommendation {
                pid,
                reason: String::from("Memory pressure"),
                urgency: 1.0 - importance,
                memory_freed: memory,
            });

            total_freed += memory;
        }

        self.kill_recommendations.clone()
    }

    /// Get process state
    #[inline(always)]
    pub fn get_state(&self, pid: ProcessId) -> Option<ProcessState> {
        self.states.get(&pid).map(|s| s.state)
    }

    /// Get recent events
    #[inline(always)]
    pub fn recent_events(&self, n: usize) -> &[LifecycleEvent] {
        let start = self.events.len().saturating_sub(n);
        &self.events[start..]
    }

    /// Cleanup terminated processes
    #[inline(always)]
    pub fn cleanup(&mut self) {
        self.states.retain(|_, state| state.state != ProcessState::Zombie);
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}
