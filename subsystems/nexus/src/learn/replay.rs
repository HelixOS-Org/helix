//! # Experience Replay
//!
//! Implements experience replay for learning from past experiences.
//! Supports prioritized replay and buffer management.
//!
//! Part of Year 2 COGNITION - Q4: Continuous Learning

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// REPLAY TYPES
// ============================================================================

/// Experience
#[derive(Debug, Clone)]
pub struct Experience {
    /// Experience ID
    pub id: u64,
    /// State before
    pub state: State,
    /// Action taken
    pub action: Action,
    /// Reward received
    pub reward: f64,
    /// Next state
    pub next_state: State,
    /// Terminal
    pub terminal: bool,
    /// Priority
    pub priority: f64,
    /// Created
    pub created: Timestamp,
    /// Times replayed
    pub replays: u64,
}

/// State
#[derive(Debug, Clone)]
pub struct State {
    /// State ID
    pub id: u64,
    /// Features
    pub features: Vec<f64>,
    /// Context
    pub context: BTreeMap<String, String>,
}

/// Action
#[derive(Debug, Clone)]
pub struct Action {
    /// Action ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Parameters
    pub parameters: Vec<f64>,
}

/// Replay batch
#[derive(Debug, Clone)]
pub struct ReplayBatch {
    /// Experiences
    pub experiences: Vec<Experience>,
    /// Weights (for importance sampling)
    pub weights: Vec<f64>,
    /// Indices in buffer
    pub indices: Vec<usize>,
}

/// Sampling strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingStrategy {
    Uniform,
    Prioritized,
    Recent,
    Diverse,
}

/// Segment
#[derive(Debug, Clone)]
pub struct Segment {
    /// Segment ID
    pub id: u64,
    /// Start index
    pub start: usize,
    /// End index
    pub end: usize,
    /// Sum of priorities
    pub priority_sum: f64,
}

/// Replay statistics
#[derive(Debug, Clone)]
pub struct ReplayInfo {
    /// Total experiences
    pub total_experiences: usize,
    /// Average priority
    pub avg_priority: f64,
    /// Average reward
    pub avg_reward: f64,
    /// Terminal ratio
    pub terminal_ratio: f64,
}

// ============================================================================
// REPLAY BUFFER
// ============================================================================

/// Experience replay buffer
pub struct ReplayBuffer {
    /// Buffer
    buffer: Vec<Experience>,
    /// Capacity
    capacity: usize,
    /// Position
    position: usize,
    /// Priority sum tree
    priority_sum: f64,
    /// Max priority
    max_priority: f64,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ReplayConfig,
    /// Statistics
    stats: ReplayStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    /// Sampling strategy
    pub strategy: SamplingStrategy,
    /// Priority exponent (alpha)
    pub alpha: f64,
    /// Importance sampling exponent (beta)
    pub beta: f64,
    /// Initial priority
    pub initial_priority: f64,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            strategy: SamplingStrategy::Prioritized,
            alpha: 0.6,
            beta: 0.4,
            initial_priority: 1.0,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ReplayStats {
    /// Experiences added
    pub experiences_added: u64,
    /// Batches sampled
    pub batches_sampled: u64,
    /// Total replays
    pub total_replays: u64,
}

impl ReplayBuffer {
    /// Create new buffer
    pub fn new(capacity: usize, config: ReplayConfig) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            position: 0,
            priority_sum: 0.0,
            max_priority: config.initial_priority,
            next_id: AtomicU64::new(1),
            config,
            stats: ReplayStats::default(),
        }
    }

    /// Add experience
    pub fn add(
        &mut self,
        state: State,
        action: Action,
        reward: f64,
        next_state: State,
        terminal: bool,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let priority = self.max_priority;

        let experience = Experience {
            id,
            state,
            action,
            reward,
            next_state,
            terminal,
            priority,
            created: Timestamp::now(),
            replays: 0,
        };

        // Update priority sum
        if self.buffer.len() >= self.capacity {
            // Remove old priority
            let old = &self.buffer[self.position];
            self.priority_sum -= old.priority.powf(self.config.alpha);
        }

        self.priority_sum += priority.powf(self.config.alpha);

        // Add to buffer
        if self.buffer.len() < self.capacity {
            self.buffer.push(experience);
        } else {
            self.buffer[self.position] = experience;
        }

        self.position = (self.position + 1) % self.capacity;
        self.stats.experiences_added += 1;

        id
    }

    /// Sample batch
    pub fn sample(&mut self, batch_size: usize) -> ReplayBatch {
        self.stats.batches_sampled += 1;

        let len = self.buffer.len();
        if len == 0 {
            return ReplayBatch {
                experiences: Vec::new(),
                weights: Vec::new(),
                indices: Vec::new(),
            };
        }

        let indices = match self.config.strategy {
            SamplingStrategy::Uniform => self.sample_uniform(batch_size),
            SamplingStrategy::Prioritized => self.sample_prioritized(batch_size),
            SamplingStrategy::Recent => self.sample_recent(batch_size),
            SamplingStrategy::Diverse => self.sample_diverse(batch_size),
        };

        let mut experiences = Vec::with_capacity(indices.len());
        let mut weights = Vec::with_capacity(indices.len());

        for &idx in &indices {
            if let Some(exp) = self.buffer.get_mut(idx) {
                exp.replays += 1;
                self.stats.total_replays += 1;

                // Importance sampling weight
                let prob = exp.priority.powf(self.config.alpha) / self.priority_sum.max(0.0001);
                let weight = (len as f64 * prob).powf(-self.config.beta);
                weights.push(weight);

                experiences.push(exp.clone());
            }
        }

        // Normalize weights
        let max_weight = weights.iter().fold(0.0f64, |a, &b| a.max(b));
        if max_weight > 0.0 {
            for w in &mut weights {
                *w /= max_weight;
            }
        }

        ReplayBatch {
            experiences,
            weights,
            indices,
        }
    }

    fn sample_uniform(&self, batch_size: usize) -> Vec<usize> {
        let len = self.buffer.len();
        let actual_size = batch_size.min(len);

        // Simple deterministic sampling for no_std
        (0..actual_size)
            .map(|i| (i * 7919) % len) // Prime number for pseudo-random
            .collect()
    }

    fn sample_prioritized(&self, batch_size: usize) -> Vec<usize> {
        let len = self.buffer.len();
        let actual_size = batch_size.min(len);

        // Segment-based sampling
        let segment_size = self.priority_sum / actual_size as f64;
        let mut indices = Vec::with_capacity(actual_size);

        for i in 0..actual_size {
            let target = segment_size * (i as f64 + 0.5);
            let idx = self.find_by_priority(target);
            indices.push(idx);
        }

        indices
    }

    fn find_by_priority(&self, target: f64) -> usize {
        let mut cumsum = 0.0;

        for (idx, exp) in self.buffer.iter().enumerate() {
            cumsum += exp.priority.powf(self.config.alpha);
            if cumsum >= target {
                return idx;
            }
        }

        self.buffer.len().saturating_sub(1)
    }

    fn sample_recent(&self, batch_size: usize) -> Vec<usize> {
        let len = self.buffer.len();
        let actual_size = batch_size.min(len);

        // Sample from most recent experiences
        let start = if len > actual_size {
            len - actual_size
        } else {
            0
        };

        (start..len).collect()
    }

    fn sample_diverse(&self, batch_size: usize) -> Vec<usize> {
        let len = self.buffer.len();
        let actual_size = batch_size.min(len);

        // Evenly spaced samples
        let step = len / actual_size.max(1);

        (0..actual_size)
            .map(|i| (i * step) % len)
            .collect()
    }

    /// Update priorities
    pub fn update_priorities(&mut self, indices: &[usize], priorities: &[f64]) {
        for (&idx, &priority) in indices.iter().zip(priorities.iter()) {
            if let Some(exp) = self.buffer.get_mut(idx) {
                // Update sum
                self.priority_sum -= exp.priority.powf(self.config.alpha);

                exp.priority = priority.max(0.0001);

                self.priority_sum += exp.priority.powf(self.config.alpha);

                // Update max
                if priority > self.max_priority {
                    self.max_priority = priority;
                }
            }
        }
    }

    /// Get info
    pub fn info(&self) -> ReplayInfo {
        if self.buffer.is_empty() {
            return ReplayInfo {
                total_experiences: 0,
                avg_priority: 0.0,
                avg_reward: 0.0,
                terminal_ratio: 0.0,
            };
        }

        let total = self.buffer.len();
        let sum_priority: f64 = self.buffer.iter().map(|e| e.priority).sum();
        let sum_reward: f64 = self.buffer.iter().map(|e| e.reward).sum();
        let terminal_count = self.buffer.iter().filter(|e| e.terminal).count();

        ReplayInfo {
            total_experiences: total,
            avg_priority: sum_priority / total as f64,
            avg_reward: sum_reward / total as f64,
            terminal_ratio: terminal_count as f64 / total as f64,
        }
    }

    /// Get experience
    pub fn get(&self, index: usize) -> Option<&Experience> {
        self.buffer.get(index)
    }

    /// Current size
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Is full
    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.capacity
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.position = 0;
        self.priority_sum = 0.0;
    }

    /// Get statistics
    pub fn stats(&self) -> &ReplayStats {
        &self.stats
    }
}

// ============================================================================
// MULTI-BUFFER REPLAY
// ============================================================================

/// Multi-buffer replay for different experience types
pub struct MultiBufferReplay {
    /// Buffers by category
    buffers: BTreeMap<String, ReplayBuffer>,
    /// Default capacity
    default_capacity: usize,
    /// Configuration
    config: ReplayConfig,
}

impl MultiBufferReplay {
    /// Create new multi-buffer
    pub fn new(default_capacity: usize, config: ReplayConfig) -> Self {
        Self {
            buffers: BTreeMap::new(),
            default_capacity,
            config,
        }
    }

    /// Get or create buffer
    pub fn buffer(&mut self, category: &str) -> &mut ReplayBuffer {
        if !self.buffers.contains_key(category) {
            let buffer = ReplayBuffer::new(self.default_capacity, self.config.clone());
            self.buffers.insert(category.into(), buffer);
        }
        self.buffers.get_mut(category).unwrap()
    }

    /// Sample from all buffers
    pub fn sample_all(&mut self, batch_size_per_buffer: usize) -> Vec<ReplayBatch> {
        let categories: Vec<String> = self.buffers.keys().cloned().collect();

        categories.iter()
            .filter_map(|cat| {
                self.buffers.get_mut(cat).map(|b| b.sample(batch_size_per_buffer))
            })
            .collect()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(id: u64) -> State {
        State {
            id,
            features: vec![id as f64, id as f64 * 2.0],
            context: BTreeMap::new(),
        }
    }

    fn make_action(id: u64) -> Action {
        Action {
            id,
            name: format!("action_{}", id),
            parameters: vec![1.0],
        }
    }

    #[test]
    fn test_add_experience() {
        let mut buffer = ReplayBuffer::new(100, ReplayConfig::default());

        let id = buffer.add(
            make_state(1),
            make_action(1),
            1.0,
            make_state(2),
            false,
        );

        assert!(buffer.get(0).is_some());
        assert_eq!(buffer.len(), 1);
    }

    #[test]
    fn test_sample_batch() {
        let mut buffer = ReplayBuffer::new(100, ReplayConfig::default());

        for i in 0..10 {
            buffer.add(
                make_state(i),
                make_action(i),
                i as f64 * 0.1,
                make_state(i + 1),
                i == 9,
            );
        }

        let batch = buffer.sample(5);
        assert_eq!(batch.experiences.len(), 5);
        assert_eq!(batch.weights.len(), 5);
    }

    #[test]
    fn test_circular_buffer() {
        let mut buffer = ReplayBuffer::new(5, ReplayConfig::default());

        for i in 0..10 {
            buffer.add(
                make_state(i),
                make_action(i),
                1.0,
                make_state(i + 1),
                false,
            );
        }

        assert_eq!(buffer.len(), 5);
        assert!(buffer.is_full());
    }

    #[test]
    fn test_update_priorities() {
        let mut buffer = ReplayBuffer::new(100, ReplayConfig::default());

        for i in 0..5 {
            buffer.add(
                make_state(i),
                make_action(i),
                1.0,
                make_state(i + 1),
                false,
            );
        }

        buffer.update_priorities(&[0, 1, 2], &[10.0, 20.0, 30.0]);

        let exp = buffer.get(2).unwrap();
        assert!((exp.priority - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_info() {
        let mut buffer = ReplayBuffer::new(100, ReplayConfig::default());

        for i in 0..10 {
            buffer.add(
                make_state(i),
                make_action(i),
                i as f64,
                make_state(i + 1),
                i == 9,
            );
        }

        let info = buffer.info();
        assert_eq!(info.total_experiences, 10);
        assert!(info.avg_reward > 0.0);
        assert!(info.terminal_ratio > 0.0);
    }

    #[test]
    fn test_multi_buffer() {
        let config = ReplayConfig::default();
        let mut multi = MultiBufferReplay::new(100, config);

        multi.buffer("success").add(
            make_state(1),
            make_action(1),
            1.0,
            make_state(2),
            false,
        );

        multi.buffer("failure").add(
            make_state(3),
            make_action(3),
            -1.0,
            make_state(4),
            true,
        );

        let batches = multi.sample_all(1);
        assert_eq!(batches.len(), 2);
    }
}
