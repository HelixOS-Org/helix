//! # Cooperation Reward/Penalty System
//!
//! Incentivizes cooperative behavior through:
//! - Cooperation score tracking per process
//! - Reward tokens for good behavior
//! - Penalty tokens for violations
//! - Priority boost/reduction based on score
//! - Resource allocation influence
//! - Reputation persistence across sessions
//! - Gamification-inspired tiered levels

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// COOPERATION SCORE
// ============================================================================

/// Cooperation level/tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopLevel {
    /// Uncooperative — actively harmful
    Hostile,
    /// Non-cooperative — ignores advisories
    NonCooperative,
    /// Basic — minimal cooperation
    Basic,
    /// Cooperative — follows most advisories
    Cooperative,
    /// Highly cooperative — actively helps
    HighlyCooperative,
    /// Exemplary — model citizen
    Exemplary,
    /// Symbiotic — deep integration
    Symbiotic,
}

impl CoopLevel {
    /// Score threshold for each level
    pub fn threshold(&self) -> i64 {
        match self {
            Self::Hostile => i64::MIN,
            Self::NonCooperative => -100,
            Self::Basic => 0,
            Self::Cooperative => 100,
            Self::HighlyCooperative => 500,
            Self::Exemplary => 1000,
            Self::Symbiotic => 5000,
        }
    }

    /// Determine level from score
    pub fn from_score(score: i64) -> Self {
        if score >= 5000 {
            Self::Symbiotic
        } else if score >= 1000 {
            Self::Exemplary
        } else if score >= 500 {
            Self::HighlyCooperative
        } else if score >= 100 {
            Self::Cooperative
        } else if score >= 0 {
            Self::Basic
        } else if score >= -100 {
            Self::NonCooperative
        } else {
            Self::Hostile
        }
    }

    /// Priority boost multiplier (basis points, 10000 = 1.0x)
    pub fn priority_boost(&self) -> u32 {
        match self {
            Self::Hostile => 5000,       // 0.5x
            Self::NonCooperative => 8000, // 0.8x
            Self::Basic => 10000,         // 1.0x
            Self::Cooperative => 11000,   // 1.1x
            Self::HighlyCooperative => 12000, // 1.2x
            Self::Exemplary => 14000,     // 1.4x
            Self::Symbiotic => 16000,     // 1.6x
        }
    }

    /// Resource allocation multiplier (basis points)
    pub fn resource_multiplier(&self) -> u32 {
        match self {
            Self::Hostile => 7000,
            Self::NonCooperative => 9000,
            Self::Basic => 10000,
            Self::Cooperative => 10500,
            Self::HighlyCooperative => 11500,
            Self::Exemplary => 13000,
            Self::Symbiotic => 15000,
        }
    }
}

// ============================================================================
// REWARD/PENALTY EVENTS
// ============================================================================

/// Reason for reward
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardReason {
    /// Responded to memory pressure advisory
    MemoryPressureResponse,
    /// Yielded CPU voluntarily
    VoluntaryCpuYield,
    /// Reduced I/O during saturation
    IoReduction,
    /// Shared cache-hot data
    CacheSharing,
    /// Batch-friendly syscall patterns
    BatchFriendly,
    /// Provided accurate hints
    AccurateHints,
    /// Fast advisory acknowledgment
    FastAck,
    /// Cooperative scheduling
    CoopScheduling,
    /// Resource release when not needed
    ResourceRelease,
    /// Consistent good behavior
    ConsistencyBonus,
    /// Helping other processes
    Altruistic,
    /// Clean shutdown
    CleanShutdown,
}

impl RewardReason {
    /// Base reward points
    pub fn base_points(&self) -> i64 {
        match self {
            Self::MemoryPressureResponse => 20,
            Self::VoluntaryCpuYield => 15,
            Self::IoReduction => 10,
            Self::CacheSharing => 25,
            Self::BatchFriendly => 5,
            Self::AccurateHints => 10,
            Self::FastAck => 5,
            Self::CoopScheduling => 30,
            Self::ResourceRelease => 15,
            Self::ConsistencyBonus => 50,
            Self::Altruistic => 40,
            Self::CleanShutdown => 10,
        }
    }
}

/// Reason for penalty
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenaltyReason {
    /// Ignored memory pressure advisory
    IgnoredMemoryAdvisory,
    /// CPU hogging
    CpuHogging,
    /// Excessive syscalls
    ExcessiveSyscalls,
    /// Lock contention causing
    LockContention,
    /// Resource leak
    ResourceLeak,
    /// SLA violation
    SlaViolation,
    /// Ignored critical advisory
    IgnoredCriticalAdvisory,
    /// Uncooperative scheduling
    UncooperativeScheduling,
    /// Excessive memory allocation
    ExcessiveMemory,
    /// I/O flooding
    IoFlooding,
    /// Contract breach
    ContractBreach,
    /// Hostile behavior detected
    HostileBehavior,
}

impl PenaltyReason {
    /// Base penalty points (negative)
    pub fn base_points(&self) -> i64 {
        match self {
            Self::IgnoredMemoryAdvisory => -10,
            Self::CpuHogging => -15,
            Self::ExcessiveSyscalls => -5,
            Self::LockContention => -10,
            Self::ResourceLeak => -20,
            Self::SlaViolation => -15,
            Self::IgnoredCriticalAdvisory => -30,
            Self::UncooperativeScheduling => -10,
            Self::ExcessiveMemory => -10,
            Self::IoFlooding => -15,
            Self::ContractBreach => -40,
            Self::HostileBehavior => -100,
        }
    }
}

// ============================================================================
// SCORE EVENT LOG
// ============================================================================

/// A recorded reward/penalty event
#[derive(Debug, Clone)]
pub struct ScoreEvent {
    /// Event ID
    pub id: u64,
    /// PID
    pub pid: u64,
    /// Points awarded (positive) or deducted (negative)
    pub points: i64,
    /// Whether this was a reward
    pub is_reward: bool,
    /// Reward reason (if reward)
    pub reward_reason: Option<RewardReason>,
    /// Penalty reason (if penalty)
    pub penalty_reason: Option<PenaltyReason>,
    /// Timestamp
    pub timestamp: u64,
    /// Multiplier applied
    pub multiplier: u32,
}

// ============================================================================
// PER-PROCESS STATE
// ============================================================================

/// Per-process cooperation score state
struct ProcessScore {
    /// PID
    pid: u64,
    /// Current score
    score: i64,
    /// All-time high score
    peak_score: i64,
    /// All-time low score
    trough_score: i64,
    /// Current level
    level: CoopLevel,
    /// Total rewards received
    total_rewards: u64,
    /// Total penalties received
    total_penalties: u64,
    /// Reward streak (consecutive rewards without penalty)
    reward_streak: u32,
    /// Longest reward streak
    max_streak: u32,
    /// Recent events (ring buffer)
    recent_events: Vec<ScoreEvent>,
    /// Max recent events
    max_recent: usize,
    /// Last reward time
    last_reward: u64,
    /// Last penalty time
    last_penalty: u64,
    /// Score at last decay
    last_decay_time: u64,
}

impl ProcessScore {
    fn new(pid: u64, timestamp: u64) -> Self {
        Self {
            pid,
            score: 0,
            peak_score: 0,
            trough_score: 0,
            level: CoopLevel::Basic,
            total_rewards: 0,
            total_penalties: 0,
            reward_streak: 0,
            max_streak: 0,
            recent_events: Vec::new(),
            max_recent: 50,
            last_reward: timestamp,
            last_penalty: 0,
            last_decay_time: timestamp,
        }
    }

    /// Apply reward
    fn reward(&mut self, reason: RewardReason, multiplier: u32, event: ScoreEvent) {
        let points = (reason.base_points() * multiplier as i64) / 10000;
        self.score += points;
        self.total_rewards += 1;
        self.reward_streak += 1;
        self.last_reward = event.timestamp;

        if self.reward_streak > self.max_streak {
            self.max_streak = self.reward_streak;
        }

        if self.score > self.peak_score {
            self.peak_score = self.score;
        }

        self.level = CoopLevel::from_score(self.score);
        self.push_event(event);
    }

    /// Apply penalty
    fn penalize(&mut self, reason: PenaltyReason, multiplier: u32, event: ScoreEvent) {
        let points = (reason.base_points() * multiplier as i64) / 10000;
        self.score += points; // points is already negative
        self.total_penalties += 1;
        self.reward_streak = 0;
        self.last_penalty = event.timestamp;

        if self.score < self.trough_score {
            self.trough_score = self.score;
        }

        self.level = CoopLevel::from_score(self.score);
        self.push_event(event);
    }

    /// Decay score toward zero
    fn decay(&mut self, current_time: u64, decay_rate_per_sec: i64) {
        let elapsed = current_time.saturating_sub(self.last_decay_time) / 1000;
        if elapsed == 0 {
            return;
        }
        self.last_decay_time = current_time;

        let decay = (decay_rate_per_sec * elapsed as i64).min(self.score.abs());
        if self.score > 0 {
            self.score -= decay;
        } else if self.score < 0 {
            self.score += decay;
        }
        self.level = CoopLevel::from_score(self.score);
    }

    fn push_event(&mut self, event: ScoreEvent) {
        if self.recent_events.len() >= self.max_recent {
            self.recent_events.remove(0);
        }
        self.recent_events.push(event);
    }
}

// ============================================================================
// REWARD ENGINE
// ============================================================================

/// Score snapshot for external use
#[derive(Debug, Clone)]
pub struct ScoreSnapshot {
    /// PID
    pub pid: u64,
    /// Current score
    pub score: i64,
    /// Current level
    pub level: CoopLevel,
    /// Priority boost (basis points)
    pub priority_boost: u32,
    /// Resource multiplier (basis points)
    pub resource_multiplier: u32,
    /// Reward streak
    pub reward_streak: u32,
    /// Total rewards
    pub total_rewards: u64,
    /// Total penalties
    pub total_penalties: u64,
}

/// Reward system configuration
#[derive(Debug, Clone)]
pub struct RewardConfig {
    /// Score decay rate (points per second toward zero)
    pub decay_rate: i64,
    /// Streak bonus multiplier (percent, applied on top)
    pub streak_bonus_percent: u32,
    /// Streak threshold for bonus
    pub streak_threshold: u32,
    /// Max score cap
    pub max_score: i64,
    /// Min score floor
    pub min_score: i64,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            decay_rate: 1,
            streak_bonus_percent: 50,
            streak_threshold: 10,
            max_score: 100_000,
            min_score: -10_000,
        }
    }
}

/// Global reward engine
pub struct RewardEngine {
    /// Per-process scores
    scores: BTreeMap<u64, ProcessScore>,
    /// Configuration
    config: RewardConfig,
    /// Next event ID
    next_event_id: u64,
    /// Total rewards issued
    pub total_rewards: u64,
    /// Total penalties issued
    pub total_penalties: u64,
    /// Total points awarded
    pub total_points_awarded: i64,
    /// Total points deducted
    pub total_points_deducted: i64,
}

impl RewardEngine {
    pub fn new(config: RewardConfig) -> Self {
        Self {
            scores: BTreeMap::new(),
            config,
            next_event_id: 1,
            total_rewards: 0,
            total_penalties: 0,
            total_points_awarded: 0,
            total_points_deducted: 0,
        }
    }

    /// Register a process
    pub fn register(&mut self, pid: u64, timestamp: u64) {
        self.scores.entry(pid).or_insert_with(|| ProcessScore::new(pid, timestamp));
    }

    /// Unregister a process
    pub fn unregister(&mut self, pid: u64) {
        self.scores.remove(&pid);
    }

    /// Issue a reward
    pub fn reward(&mut self, pid: u64, reason: RewardReason, timestamp: u64) -> i64 {
        let event_id = self.next_event_id;
        self.next_event_id += 1;

        // Calculate multiplier
        let streak = self
            .scores
            .get(&pid)
            .map_or(0, |s| s.reward_streak);

        let multiplier = if streak >= self.config.streak_threshold {
            10000 + self.config.streak_bonus_percent * 100
        } else {
            10000
        };

        let points = (reason.base_points() * multiplier as i64) / 10000;

        let event = ScoreEvent {
            id: event_id,
            pid,
            points,
            is_reward: true,
            reward_reason: Some(reason),
            penalty_reason: None,
            timestamp,
            multiplier,
        };

        if let Some(ps) = self.scores.get_mut(&pid) {
            ps.reward(reason, multiplier, event);
            // Clamp
            if ps.score > self.config.max_score {
                ps.score = self.config.max_score;
            }
        }

        self.total_rewards += 1;
        self.total_points_awarded += points;
        points
    }

    /// Issue a penalty
    pub fn penalize(&mut self, pid: u64, reason: PenaltyReason, timestamp: u64) -> i64 {
        let event_id = self.next_event_id;
        self.next_event_id += 1;

        let multiplier = 10000u32;
        let points = (reason.base_points() * multiplier as i64) / 10000;

        let event = ScoreEvent {
            id: event_id,
            pid,
            points,
            is_reward: false,
            reward_reason: None,
            penalty_reason: Some(reason),
            timestamp,
            multiplier,
        };

        if let Some(ps) = self.scores.get_mut(&pid) {
            ps.penalize(reason, multiplier, event);
            // Clamp
            if ps.score < self.config.min_score {
                ps.score = self.config.min_score;
            }
        }

        self.total_penalties += 1;
        self.total_points_deducted += points.abs();
        points
    }

    /// Get score snapshot
    pub fn get_score(&self, pid: u64) -> Option<ScoreSnapshot> {
        let ps = self.scores.get(&pid)?;
        Some(ScoreSnapshot {
            pid,
            score: ps.score,
            level: ps.level,
            priority_boost: ps.level.priority_boost(),
            resource_multiplier: ps.level.resource_multiplier(),
            reward_streak: ps.reward_streak,
            total_rewards: ps.total_rewards,
            total_penalties: ps.total_penalties,
        })
    }

    /// Get cooperation level for a PID
    pub fn get_level(&self, pid: u64) -> CoopLevel {
        self.scores
            .get(&pid)
            .map_or(CoopLevel::Basic, |s| s.level)
    }

    /// Decay all scores
    pub fn decay_all(&mut self, current_time: u64) {
        for ps in self.scores.values_mut() {
            ps.decay(current_time, self.config.decay_rate);
        }
    }

    /// Get top cooperators
    pub fn top_cooperators(&self, count: usize) -> Vec<ScoreSnapshot> {
        let mut snapshots: Vec<ScoreSnapshot> = self
            .scores
            .values()
            .map(|ps| ScoreSnapshot {
                pid: ps.pid,
                score: ps.score,
                level: ps.level,
                priority_boost: ps.level.priority_boost(),
                resource_multiplier: ps.level.resource_multiplier(),
                reward_streak: ps.reward_streak,
                total_rewards: ps.total_rewards,
                total_penalties: ps.total_penalties,
            })
            .collect();

        snapshots.sort_by(|a, b| b.score.cmp(&a.score));
        snapshots.truncate(count);
        snapshots
    }

    /// Registered process count
    pub fn process_count(&self) -> usize {
        self.scores.len()
    }

    /// Average cooperation score
    pub fn average_score(&self) -> f64 {
        if self.scores.is_empty() {
            return 0.0;
        }
        let sum: i64 = self.scores.values().map(|s| s.score).sum();
        sum as f64 / self.scores.len() as f64
    }
}
