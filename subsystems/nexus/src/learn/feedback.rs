//! Feedback loop for learning from outcomes
//!
//! This module provides feedback-based learning capabilities.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::types::{ExperienceId, Timestamp};

/// Feedback type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackType {
    /// Positive outcome
    Positive,
    /// Negative outcome
    Negative,
    /// Neutral outcome
    Neutral,
    /// Timeout (no response)
    Timeout,
    /// Explicit correction
    Correction,
    /// Performance metric
    Performance,
}

impl FeedbackType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
            Self::Neutral => "neutral",
            Self::Timeout => "timeout",
            Self::Correction => "correction",
            Self::Performance => "performance",
        }
    }

    /// Reward value (-1.0 to 1.0)
    pub fn reward(&self) -> f32 {
        match self {
            Self::Positive => 1.0,
            Self::Negative => -1.0,
            Self::Neutral => 0.0,
            Self::Timeout => -0.5,
            Self::Correction => -0.3,
            Self::Performance => 0.0, // Depends on metric
        }
    }
}

/// Feedback entry
#[derive(Debug, Clone)]
pub struct FeedbackEntry {
    /// Entry ID
    pub id: ExperienceId,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Action taken
    pub action: String,
    /// Context when action was taken
    pub context: BTreeMap<String, String>,
    /// Feedback type
    pub feedback_type: FeedbackType,
    /// Reward value (-1.0 to 1.0)
    pub reward: f32,
    /// Additional data
    pub data: BTreeMap<String, String>,
}

impl FeedbackEntry {
    /// Create new feedback entry
    pub fn new(
        id: ExperienceId,
        timestamp: Timestamp,
        action: String,
        feedback_type: FeedbackType,
    ) -> Self {
        Self {
            id,
            timestamp,
            action,
            context: BTreeMap::new(),
            feedback_type,
            reward: feedback_type.reward(),
            data: BTreeMap::new(),
        }
    }

    /// With context
    pub fn with_context(mut self, key: &str, value: &str) -> Self {
        self.context.insert(String::from(key), String::from(value));
        self
    }

    /// With reward override
    pub fn with_reward(mut self, reward: f32) -> Self {
        self.reward = reward.clamp(-1.0, 1.0);
        self
    }
}

/// Action statistics
#[derive(Debug, Clone)]
pub struct ActionStats {
    /// Action name
    pub action: String,
    /// Total occurrences
    pub count: u64,
    /// Positive count
    pub positive_count: u64,
    /// Negative count
    pub negative_count: u64,
    /// Total reward
    pub total_reward: f32,
    /// Average reward
    pub avg_reward: f32,
    /// Last used
    pub last_used: Timestamp,
}

impl ActionStats {
    /// Create new stats
    pub fn new(action: String) -> Self {
        Self {
            action,
            count: 0,
            positive_count: 0,
            negative_count: 0,
            total_reward: 0.0,
            avg_reward: 0.0,
            last_used: Timestamp::new(0),
        }
    }

    /// Record feedback
    pub fn record(&mut self, feedback: &FeedbackEntry) {
        self.count += 1;
        self.total_reward += feedback.reward;
        self.avg_reward = self.total_reward / self.count as f32;
        self.last_used = feedback.timestamp;

        match feedback.feedback_type {
            FeedbackType::Positive => self.positive_count += 1,
            FeedbackType::Negative => self.negative_count += 1,
            _ => {}
        }
    }

    /// Success rate
    pub fn success_rate(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        self.positive_count as f32 / self.count as f32
    }
}

/// Feedback loop manager
pub struct FeedbackLoop {
    /// Feedback history
    history: Vec<FeedbackEntry>,
    /// Action statistics
    action_stats: BTreeMap<String, ActionStats>,
    /// Entry counter
    counter: AtomicU64,
    /// Max history size
    max_history: usize,
    /// Learning rate
    learning_rate: f32,
}

impl FeedbackLoop {
    /// Create new feedback loop
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            action_stats: BTreeMap::new(),
            counter: AtomicU64::new(0),
            max_history: 100000,
            learning_rate: 0.1,
        }
    }

    /// Set learning rate
    pub fn set_learning_rate(&mut self, rate: f32) {
        self.learning_rate = rate.clamp(0.001, 1.0);
    }

    /// Record feedback
    pub fn record(
        &mut self,
        action: &str,
        feedback_type: FeedbackType,
        timestamp: u64,
    ) -> ExperienceId {
        let id = ExperienceId(self.counter.fetch_add(1, Ordering::Relaxed));
        let ts = Timestamp::new(timestamp);
        let entry = FeedbackEntry::new(id, ts, String::from(action), feedback_type);

        // Update action stats
        self.action_stats
            .entry(String::from(action))
            .or_insert_with(|| ActionStats::new(String::from(action)))
            .record(&entry);

        // Add to history
        self.history.push(entry);

        // Evict old entries
        if self.history.len() > self.max_history {
            self.history.drain(0..self.max_history / 10);
        }

        id
    }

    /// Record with full entry
    pub fn record_full(&mut self, mut entry: FeedbackEntry) -> ExperienceId {
        let id = ExperienceId(self.counter.fetch_add(1, Ordering::Relaxed));
        entry.id = id;

        // Update action stats
        self.action_stats
            .entry(entry.action.clone())
            .or_insert_with(|| ActionStats::new(entry.action.clone()))
            .record(&entry);

        self.history.push(entry);

        if self.history.len() > self.max_history {
            self.history.drain(0..self.max_history / 10);
        }

        id
    }

    /// Get action stats
    pub fn get_stats(&self, action: &str) -> Option<&ActionStats> {
        self.action_stats.get(action)
    }

    /// Get best action for context
    pub fn best_action(&self, candidates: &[String]) -> Option<&String> {
        candidates.iter().max_by(|a, b| {
            let stats_a = self.action_stats.get(*a);
            let stats_b = self.action_stats.get(*b);

            let score_a = stats_a.map(|s| s.avg_reward).unwrap_or(-2.0);
            let score_b = stats_b.map(|s| s.avg_reward).unwrap_or(-2.0);

            score_a
                .partial_cmp(&score_b)
                .unwrap_or(core::cmp::Ordering::Equal)
        })
    }

    /// History length
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Action count
    pub fn action_count(&self) -> usize {
        self.action_stats.len()
    }

    /// Recent feedback
    pub fn recent(&self, limit: usize) -> &[FeedbackEntry] {
        let start = self.history.len().saturating_sub(limit);
        &self.history[start..]
    }
}

impl Default for FeedbackLoop {
    fn default() -> Self {
        Self::new()
    }
}
