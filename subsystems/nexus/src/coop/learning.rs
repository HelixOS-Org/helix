//! # Cooperative Learning Engine
//!
//! Machine learning for cooperative scheduling optimization:
//! - Reinforcement learning for scheduling decisions
//! - Feature extraction from process behavior
//! - Q-table for action selection
//! - Reward shaping for cooperative behavior
//! - Online learning with exploration/exploitation
//! - Model persistence and transfer

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

// ============================================================================
// FEATURE TYPES
// ============================================================================

/// Feature for learning
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Feature {
    /// CPU usage bucket (0-10)
    CpuUsage,
    /// Memory pressure bucket (0-10)
    MemoryPressure,
    /// I/O intensity bucket (0-10)
    IoIntensity,
    /// Process age bucket
    ProcessAge,
    /// Cooperation score bucket
    CoopScore,
    /// Priority bucket
    Priority,
    /// Wait time bucket
    WaitTime,
    /// Cache warmth bucket
    CacheWarmth,
    /// NUMA locality bucket
    NumaLocality,
    /// Deadline urgency bucket
    DeadlineUrgency,
}

/// Feature vector (discretized)
#[derive(Debug, Clone)]
pub struct FeatureVector {
    /// Feature values (feature → bucket index)
    pub values: BTreeMap<u8, u8>,
}

impl FeatureVector {
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
        }
    }

    /// Set feature value
    #[inline(always)]
    pub fn set(&mut self, feature: Feature, bucket: u8) {
        self.values.insert(feature as u8, bucket.min(10));
    }

    /// Get feature value
    #[inline(always)]
    pub fn get(&self, feature: Feature) -> u8 {
        self.values.get(&(feature as u8)).copied().unwrap_or(5)
    }

    /// State key for Q-table (compact representation)
    #[inline]
    pub fn state_key(&self) -> u64 {
        let mut key: u64 = 0;
        for (&feat, &val) in &self.values {
            key = key.wrapping_mul(11).wrapping_add(val as u64);
            key = key.wrapping_mul(17).wrapping_add(feat as u64);
        }
        key
    }
}

// ============================================================================
// ACTIONS
// ============================================================================

/// Scheduling action
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchedulingAction {
    /// Keep current scheduling
    KeepCurrent,
    /// Boost priority
    BoostPriority,
    /// Lower priority
    LowerPriority,
    /// Extend timeslice
    ExtendTimeslice,
    /// Shorten timeslice
    ShortenTimeslice,
    /// Migrate to another CPU
    Migrate,
    /// Pin to current CPU
    Pin,
    /// Increase memory allocation
    IncreaseMemory,
    /// Throttle I/O
    ThrottleIo,
    /// Boost I/O
    BoostIo,
}

impl SchedulingAction {
    pub const ALL: &'static [SchedulingAction] = &[
        Self::KeepCurrent,
        Self::BoostPriority,
        Self::LowerPriority,
        Self::ExtendTimeslice,
        Self::ShortenTimeslice,
        Self::Migrate,
        Self::Pin,
        Self::IncreaseMemory,
        Self::ThrottleIo,
        Self::BoostIo,
    ];

    #[inline(always)]
    pub fn count() -> usize {
        Self::ALL.len()
    }
}

// ============================================================================
// Q-TABLE
// ============================================================================

/// Q-value entry
#[derive(Debug, Clone)]
pub struct QEntry {
    /// Q-value (scaled to integer, × 1000)
    pub q_value: i64,
    /// Visit count
    pub visits: u64,
    /// Last updated
    pub last_update: u64,
}

impl QEntry {
    pub fn new() -> Self {
        Self {
            q_value: 0,
            visits: 0,
            last_update: 0,
        }
    }
}

/// Q-table for state-action pairs
pub struct QTable {
    /// State → Action → QEntry
    entries: BTreeMap<u64, BTreeMap<u8, QEntry>>,
    /// Total entries
    pub total_entries: usize,
}

impl QTable {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            total_entries: 0,
        }
    }

    /// Get Q-value
    #[inline]
    pub fn get(&self, state: u64, action: SchedulingAction) -> i64 {
        self.entries
            .get(&state)
            .and_then(|actions| actions.get(&(action as u8)))
            .map(|e| e.q_value)
            .unwrap_or(0)
    }

    /// Update Q-value
    #[inline]
    pub fn update(&mut self, state: u64, action: SchedulingAction, new_value: i64, now: u64) {
        let actions = self.entries.entry(state).or_insert_with(BTreeMap::new);
        let entry = actions.entry(action as u8).or_insert_with(QEntry::new);
        entry.q_value = new_value;
        entry.visits += 1;
        entry.last_update = now;
        self.total_entries = self.entries.values().map(|a| a.len()).sum();
    }

    /// Best action for state
    pub fn best_action(&self, state: u64) -> SchedulingAction {
        if let Some(actions) = self.entries.get(&state) {
            actions
                .iter()
                .max_by_key(|(_, e)| e.q_value)
                .map(|(&a, _)| {
                    SchedulingAction::ALL
                        .iter()
                        .find(|&&sa| sa as u8 == a)
                        .copied()
                        .unwrap_or(SchedulingAction::KeepCurrent)
                })
                .unwrap_or(SchedulingAction::KeepCurrent)
        } else {
            SchedulingAction::KeepCurrent
        }
    }

    /// Visit count for state
    #[inline]
    pub fn visits(&self, state: u64) -> u64 {
        self.entries
            .get(&state)
            .map(|actions| actions.values().map(|e| e.visits).sum())
            .unwrap_or(0)
    }

    /// Prune old entries
    #[inline]
    pub fn prune(&mut self, max_age: u64, now: u64) {
        for actions in self.entries.values_mut() {
            actions.retain(|_, e| now.saturating_sub(e.last_update) < max_age);
        }
        self.entries.retain(|_, actions| !actions.is_empty());
        self.total_entries = self.entries.values().map(|a| a.len()).sum();
    }
}

// ============================================================================
// REWARD
// ============================================================================

/// Reward signal
#[derive(Debug, Clone)]
pub struct RewardSignal {
    /// Process ID
    pub pid: u64,
    /// State before action
    pub state: u64,
    /// Action taken
    pub action: SchedulingAction,
    /// Reward value (×1000)
    pub reward: i64,
    /// Next state
    pub next_state: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Reward component
#[derive(Debug, Clone, Copy)]
pub enum RewardComponent {
    /// Throughput improvement
    Throughput(i64),
    /// Latency reduction
    Latency(i64),
    /// Fairness improvement
    Fairness(i64),
    /// Energy savings
    Energy(i64),
    /// Cooperation bonus
    Cooperation(i64),
}

/// Compute composite reward
pub fn compute_reward(components: &[RewardComponent]) -> i64 {
    components
        .iter()
        .map(|c| match c {
            RewardComponent::Throughput(v) => *v * 3,
            RewardComponent::Latency(v) => *v * 4,
            RewardComponent::Fairness(v) => *v * 2,
            RewardComponent::Energy(v) => *v * 1,
            RewardComponent::Cooperation(v) => *v * 2,
        })
        .sum()
}

// ============================================================================
// LEARNING ENGINE
// ============================================================================

/// Learning configuration
#[derive(Debug, Clone)]
pub struct LearningConfig {
    /// Learning rate (×1000)
    pub alpha: u32,
    /// Discount factor (×1000)
    pub gamma: u32,
    /// Exploration rate (×1000)
    pub epsilon: u32,
    /// Epsilon decay per episode (×1000)
    pub epsilon_decay: u32,
    /// Minimum epsilon (×1000)
    pub min_epsilon: u32,
    /// Max Q-table entries before pruning
    pub max_entries: usize,
    /// Prune age (ms)
    pub prune_age_ms: u64,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            alpha: 100,         // 0.1
            gamma: 950,         // 0.95
            epsilon: 200,       // 0.2
            epsilon_decay: 999, // 0.999
            min_epsilon: 10,    // 0.01
            max_entries: 10000,
            prune_age_ms: 600_000,
        }
    }
}

/// Learning stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct LearningStats {
    /// Total episodes
    pub episodes: u64,
    /// Total rewards
    pub total_reward: i64,
    /// Average reward (×1000)
    pub avg_reward: i64,
    /// Explorations
    pub explorations: u64,
    /// Exploitations
    pub exploitations: u64,
    /// Q-table size
    pub q_table_size: usize,
    /// Current epsilon (×1000)
    pub current_epsilon: u32,
}

/// Cooperative learning engine
pub struct CoopLearningEngine {
    /// Q-table
    q_table: QTable,
    /// Configuration
    config: LearningConfig,
    /// Stats
    stats: LearningStats,
    /// Reward history
    rewards: VecDeque<i64>,
    /// Max reward history
    max_rewards: usize,
    /// Simple RNG state
    rng_state: u64,
}

impl CoopLearningEngine {
    pub fn new(config: LearningConfig) -> Self {
        Self {
            q_table: QTable::new(),
            config,
            stats: LearningStats::default(),
            rewards: VecDeque::new(),
            max_rewards: 1000,
            rng_state: 42,
        }
    }

    /// Simple pseudo-RNG
    fn next_random(&mut self) -> u64 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.rng_state >> 33
    }

    /// Select action (epsilon-greedy)
    pub fn select_action(&mut self, features: &FeatureVector) -> SchedulingAction {
        let state = features.state_key();
        let rand = self.next_random() % 1000;

        if rand < self.config.epsilon as u64 {
            // Explore
            self.stats.explorations += 1;
            let idx = (self.next_random() as usize) % SchedulingAction::count();
            SchedulingAction::ALL[idx]
        } else {
            // Exploit
            self.stats.exploitations += 1;
            self.q_table.best_action(state)
        }
    }

    /// Process reward and update Q-table
    pub fn learn(&mut self, signal: RewardSignal, now: u64) {
        let current_q = self.q_table.get(signal.state, signal.action);

        // Best Q-value for next state
        let best_next = SchedulingAction::ALL
            .iter()
            .map(|&a| self.q_table.get(signal.next_state, a))
            .max()
            .unwrap_or(0);

        // Q-learning update: Q(s,a) = Q(s,a) + α[r + γ·max Q(s',a') - Q(s,a)]
        let alpha = self.config.alpha as i64;
        let gamma = self.config.gamma as i64;

        let td_target = signal.reward * 1000 + gamma * best_next / 1000;
        let td_error = td_target - current_q;
        let new_q = current_q + alpha * td_error / 1000;

        self.q_table.update(signal.state, signal.action, new_q, now);

        // Update stats
        self.stats.episodes += 1;
        self.stats.total_reward += signal.reward;
        self.rewards.push_back(signal.reward);
        if self.rewards.len() > self.max_rewards {
            self.rewards.pop_front();
        }

        if !self.rewards.is_empty() {
            self.stats.avg_reward = self.rewards.iter().sum::<i64>() / self.rewards.len() as i64;
        }

        // Decay epsilon
        self.config.epsilon =
            ((self.config.epsilon as u64 * self.config.epsilon_decay as u64) / 1000) as u32;
        if self.config.epsilon < self.config.min_epsilon {
            self.config.epsilon = self.config.min_epsilon;
        }
        self.stats.current_epsilon = self.config.epsilon;
        self.stats.q_table_size = self.q_table.total_entries;
    }

    /// Prune old entries
    #[inline(always)]
    pub fn prune(&mut self, now: u64) {
        self.q_table.prune(self.config.prune_age_ms, now);
        self.stats.q_table_size = self.q_table.total_entries;
    }

    /// Extract features from process metrics
    #[inline]
    pub fn extract_features(
        cpu_pct: u32,
        memory_pct: u32,
        io_pct: u32,
        coop_score: u32,
        priority: u32,
        wait_ms: u64,
    ) -> FeatureVector {
        let mut fv = FeatureVector::new();
        fv.set(Feature::CpuUsage, (cpu_pct / 10).min(10) as u8);
        fv.set(Feature::MemoryPressure, (memory_pct / 10).min(10) as u8);
        fv.set(Feature::IoIntensity, (io_pct / 10).min(10) as u8);
        fv.set(Feature::CoopScore, (coop_score / 10).min(10) as u8);
        fv.set(Feature::Priority, (priority / 10).min(10) as u8);
        fv.set(Feature::WaitTime, (wait_ms / 10).min(10) as u8);
        fv
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &LearningStats {
        &self.stats
    }

    /// Q-table size
    #[inline(always)]
    pub fn q_table_size(&self) -> usize {
        self.q_table.total_entries
    }
}
