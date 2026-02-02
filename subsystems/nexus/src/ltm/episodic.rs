//! Episodic Memory
//!
//! This module provides storage and retrieval of episodes (events).

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{BootId, EpisodeId, TimeRange, Timestamp};

/// Episode type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EpisodeType {
    /// Normal boot
    NormalBoot,
    /// Crash occurred
    Crash,
    /// Recovery performed
    Recovery,
    /// Anomaly detected
    Anomaly,
    /// Performance degradation
    Degradation,
    /// Configuration change
    ConfigChange,
    /// Security event
    SecurityEvent,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Hardware event
    HardwareEvent,
    /// User action
    UserAction,
    /// System update
    SystemUpdate,
    /// Milestone
    Milestone,
}

impl EpisodeType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::NormalBoot => "normal_boot",
            Self::Crash => "crash",
            Self::Recovery => "recovery",
            Self::Anomaly => "anomaly",
            Self::Degradation => "degradation",
            Self::ConfigChange => "config_change",
            Self::SecurityEvent => "security_event",
            Self::ResourceExhaustion => "resource_exhaustion",
            Self::HardwareEvent => "hardware_event",
            Self::UserAction => "user_action",
            Self::SystemUpdate => "system_update",
            Self::Milestone => "milestone",
        }
    }

    /// Importance level (1-10)
    pub fn importance(&self) -> u8 {
        match self {
            Self::NormalBoot => 1,
            Self::UserAction => 2,
            Self::ConfigChange => 3,
            Self::Milestone => 4,
            Self::Degradation => 5,
            Self::Anomaly => 6,
            Self::ResourceExhaustion => 7,
            Self::HardwareEvent => 7,
            Self::SecurityEvent => 8,
            Self::SystemUpdate => 8,
            Self::Recovery => 9,
            Self::Crash => 10,
        }
    }

    /// Should persist
    pub fn should_persist(&self) -> bool {
        self.importance() >= 3
    }
}

/// Episode outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpisodeOutcome {
    /// Successful
    Success,
    /// Failed
    Failure,
    /// Partial success
    Partial,
    /// Ongoing
    Ongoing,
    /// Unknown
    Unknown,
}

impl EpisodeOutcome {
    /// Get outcome name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Partial => "partial",
            Self::Ongoing => "ongoing",
            Self::Unknown => "unknown",
        }
    }

    /// Is positive outcome
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Success)
    }
}

/// Episode event
#[derive(Debug, Clone)]
pub struct EpisodeEvent {
    /// Event timestamp
    pub timestamp: Timestamp,
    /// Event description
    pub description: String,
    /// Event data
    pub data: BTreeMap<String, String>,
}

impl EpisodeEvent {
    /// Create new event
    pub fn new(timestamp: Timestamp, description: String) -> Self {
        Self {
            timestamp,
            description,
            data: BTreeMap::new(),
        }
    }

    /// Add data
    pub fn with_data(mut self, key: &str, value: &str) -> Self {
        self.data.insert(String::from(key), String::from(value));
        self
    }
}

/// Episode
#[derive(Debug)]
pub struct Episode {
    /// Episode ID
    pub id: EpisodeId,
    /// Episode type
    pub episode_type: EpisodeType,
    /// Boot ID when this occurred
    pub boot_id: BootId,
    /// Time range
    pub time_range: TimeRange,
    /// Summary
    pub summary: String,
    /// Events in this episode
    pub events: Vec<EpisodeEvent>,
    /// Outcome
    pub outcome: EpisodeOutcome,
    /// Lessons learned
    pub lessons: Vec<String>,
    /// Related episodes
    pub related: Vec<EpisodeId>,
    /// Tags
    pub tags: Vec<String>,
    /// Importance (1-10)
    pub importance: u8,
    /// Access count (for LRU)
    access_count: AtomicU64,
}

impl Episode {
    /// Create new episode
    pub fn new(
        id: EpisodeId,
        episode_type: EpisodeType,
        boot_id: BootId,
        time_range: TimeRange,
    ) -> Self {
        Self {
            id,
            episode_type,
            boot_id,
            time_range,
            summary: String::new(),
            events: Vec::new(),
            outcome: EpisodeOutcome::Unknown,
            lessons: Vec::new(),
            related: Vec::new(),
            tags: Vec::new(),
            importance: episode_type.importance(),
            access_count: AtomicU64::new(0),
        }
    }

    /// With summary
    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }

    /// Add event
    pub fn add_event(&mut self, event: EpisodeEvent) {
        self.events.push(event);
    }

    /// Add lesson
    pub fn add_lesson(&mut self, lesson: String) {
        self.lessons.push(lesson);
    }

    /// Add tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Set outcome
    pub fn set_outcome(&mut self, outcome: EpisodeOutcome) {
        self.outcome = outcome;
    }

    /// Duration
    pub fn duration_ns(&self) -> u64 {
        self.time_range.duration_ns()
    }

    /// Record access
    pub fn record_access(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Access count
    pub fn access_count(&self) -> u64 {
        self.access_count.load(Ordering::Relaxed)
    }
}

impl Clone for Episode {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            episode_type: self.episode_type,
            boot_id: self.boot_id,
            time_range: self.time_range,
            summary: self.summary.clone(),
            events: self.events.clone(),
            outcome: self.outcome,
            lessons: self.lessons.clone(),
            related: self.related.clone(),
            tags: self.tags.clone(),
            importance: self.importance,
            access_count: AtomicU64::new(self.access_count.load(Ordering::Relaxed)),
        }
    }
}

/// Episodic memory store
#[derive(Debug)]
pub struct EpisodicMemory {
    /// Episodes
    episodes: BTreeMap<EpisodeId, Episode>,
    /// Episodes by boot
    by_boot: BTreeMap<BootId, Vec<EpisodeId>>,
    /// Episodes by type
    by_type: BTreeMap<EpisodeType, Vec<EpisodeId>>,
    /// Episode counter
    counter: AtomicU64,
    /// Current boot ID
    current_boot: BootId,
    /// Max episodes to keep
    max_episodes: usize,
}

impl EpisodicMemory {
    /// Create new episodic memory
    pub fn new(current_boot: BootId) -> Self {
        Self {
            episodes: BTreeMap::new(),
            by_boot: BTreeMap::new(),
            by_type: BTreeMap::new(),
            counter: AtomicU64::new(0),
            current_boot,
            max_episodes: 10000,
        }
    }

    /// Create new episode
    pub fn create_episode(
        &mut self,
        episode_type: EpisodeType,
        time_range: TimeRange,
    ) -> EpisodeId {
        let id = EpisodeId(self.counter.fetch_add(1, Ordering::Relaxed));
        let episode = Episode::new(id, episode_type, self.current_boot, time_range);

        // Index by boot
        self.by_boot
            .entry(self.current_boot)
            .or_insert_with(Vec::new)
            .push(id);

        // Index by type
        self.by_type
            .entry(episode_type)
            .or_insert_with(Vec::new)
            .push(id);

        self.episodes.insert(id, episode);

        // Evict if over limit
        self.maybe_evict();

        id
    }

    /// Get episode
    pub fn get(&self, id: EpisodeId) -> Option<&Episode> {
        if let Some(ep) = self.episodes.get(&id) {
            ep.record_access();
            return Some(ep);
        }
        None
    }

    /// Get episode mutably
    pub fn get_mut(&mut self, id: EpisodeId) -> Option<&mut Episode> {
        self.episodes.get_mut(&id)
    }

    /// Find by type
    pub fn find_by_type(&self, episode_type: EpisodeType) -> Vec<&Episode> {
        self.by_type
            .get(&episode_type)
            .map(|ids| ids.iter().filter_map(|id| self.episodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Find by boot
    pub fn find_by_boot(&self, boot_id: BootId) -> Vec<&Episode> {
        self.by_boot
            .get(&boot_id)
            .map(|ids| ids.iter().filter_map(|id| self.episodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Find in time range
    pub fn find_in_range(&self, range: TimeRange) -> Vec<&Episode> {
        self.episodes
            .values()
            .filter(|ep| ep.time_range.overlaps(&range))
            .collect()
    }

    /// Find similar episodes
    pub fn find_similar(&self, episode_type: EpisodeType, tags: &[String]) -> Vec<&Episode> {
        self.episodes
            .values()
            .filter(|ep| {
                ep.episode_type == episode_type || tags.iter().any(|t| ep.tags.contains(t))
            })
            .collect()
    }

    /// Recent episodes
    pub fn recent(&self, limit: usize) -> Vec<&Episode> {
        let mut episodes: Vec<_> = self.episodes.values().collect();
        episodes.sort_by(|a, b| b.time_range.start.cmp(&a.time_range.start));
        episodes.into_iter().take(limit).collect()
    }

    /// Episode count
    pub fn count(&self) -> usize {
        self.episodes.len()
    }

    /// Evict old/unimportant episodes
    fn maybe_evict(&mut self) {
        if self.episodes.len() <= self.max_episodes {
            return;
        }

        // Find episodes to evict (lowest importance + oldest)
        let mut candidates: Vec<_> = self
            .episodes
            .values()
            .map(|ep| (ep.id, ep.importance, ep.access_count(), ep.time_range.start))
            .collect();

        // Sort by importance (asc), then access count (asc), then time (asc)
        candidates.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));

        // Remove 10% of episodes
        let to_remove = self.max_episodes / 10;
        for (id, _, _, _) in candidates.into_iter().take(to_remove) {
            self.episodes.remove(&id);
        }
    }
}
