//! Curriculum learning for progressive complexity
//!
//! This module provides curriculum-based learning capabilities.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DifficultyLevel {
    /// Beginner
    Beginner = 0,
    /// Easy
    Easy     = 1,
    /// Medium
    Medium   = 2,
    /// Hard
    Hard     = 3,
    /// Expert
    Expert   = 4,
    /// Master
    Master   = 5,
}

impl DifficultyLevel {
    /// Get level name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Beginner => "beginner",
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
            Self::Expert => "expert",
            Self::Master => "master",
        }
    }

    /// Next level
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Beginner => Some(Self::Easy),
            Self::Easy => Some(Self::Medium),
            Self::Medium => Some(Self::Hard),
            Self::Hard => Some(Self::Expert),
            Self::Expert => Some(Self::Master),
            Self::Master => None,
        }
    }

    /// Previous level
    pub fn previous(&self) -> Option<Self> {
        match self {
            Self::Beginner => None,
            Self::Easy => Some(Self::Beginner),
            Self::Medium => Some(Self::Easy),
            Self::Hard => Some(Self::Medium),
            Self::Expert => Some(Self::Hard),
            Self::Master => Some(Self::Expert),
        }
    }
}

/// Task completion criteria
#[derive(Debug, Clone)]
pub struct TaskCriteria {
    /// Required success rate
    pub success_rate: f32,
    /// Minimum attempts
    pub min_attempts: u64,
    /// Consecutive successes required
    pub consecutive_successes: u64,
}

/// Learning task
#[derive(Debug, Clone)]
pub struct LearningTask {
    /// Task name
    pub name: String,
    /// Difficulty
    pub difficulty: DifficultyLevel,
    /// Description
    pub description: String,
    /// Prerequisites
    pub prerequisites: Vec<String>,
    /// Completion criteria
    pub criteria: TaskCriteria,
    /// Current progress (0-1)
    pub progress: f32,
    /// Is completed
    pub completed: bool,
}

impl LearningTask {
    /// Create new task
    pub fn new(name: String, difficulty: DifficultyLevel) -> Self {
        Self {
            name,
            difficulty,
            description: String::new(),
            prerequisites: Vec::new(),
            criteria: TaskCriteria {
                success_rate: 0.8,
                min_attempts: 10,
                consecutive_successes: 3,
            },
            progress: 0.0,
            completed: false,
        }
    }

    /// Check if prerequisites met
    pub fn prerequisites_met(&self, completed_tasks: &[String]) -> bool {
        self.prerequisites
            .iter()
            .all(|p| completed_tasks.contains(p))
    }
}

/// Curriculum stage
#[derive(Debug, Clone)]
pub struct CurriculumStage {
    /// Stage name
    pub name: String,
    /// Difficulty level
    pub level: DifficultyLevel,
    /// Tasks in this stage
    pub tasks: Vec<LearningTask>,
    /// Is completed
    pub completed: bool,
}

impl CurriculumStage {
    /// Create new stage
    pub fn new(name: String, level: DifficultyLevel) -> Self {
        Self {
            name,
            level,
            tasks: Vec::new(),
            completed: false,
        }
    }

    /// Add task
    pub fn add_task(&mut self, task: LearningTask) {
        self.tasks.push(task);
    }

    /// Check completion
    pub fn check_completion(&mut self) -> bool {
        self.completed = self.tasks.iter().all(|t| t.completed);
        self.completed
    }

    /// Progress (0-1)
    pub fn progress(&self) -> f32 {
        if self.tasks.is_empty() {
            return 0.0;
        }
        let completed = self.tasks.iter().filter(|t| t.completed).count();
        completed as f32 / self.tasks.len() as f32
    }
}

/// Curriculum learner
pub struct CurriculumLearner {
    /// Stages
    stages: Vec<CurriculumStage>,
    /// Current stage index
    current_stage: usize,
    /// Current task index within stage
    current_task: usize,
    /// Completed tasks
    completed_tasks: Vec<String>,
    /// Total successes
    total_successes: u64,
    /// Total failures
    total_failures: u64,
    /// Consecutive successes
    consecutive_successes: u64,
}

impl CurriculumLearner {
    /// Create new curriculum learner
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            current_stage: 0,
            current_task: 0,
            completed_tasks: Vec::new(),
            total_successes: 0,
            total_failures: 0,
            consecutive_successes: 0,
        }
    }

    /// Add stage
    pub fn add_stage(&mut self, stage: CurriculumStage) {
        self.stages.push(stage);
    }

    /// Current difficulty
    pub fn current_difficulty(&self) -> DifficultyLevel {
        self.stages
            .get(self.current_stage)
            .map(|s| s.level)
            .unwrap_or(DifficultyLevel::Beginner)
    }

    /// Current task
    pub fn current_task(&self) -> Option<&LearningTask> {
        self.stages
            .get(self.current_stage)
            .and_then(|s| s.tasks.get(self.current_task))
    }

    /// Record outcome
    pub fn record_outcome(&mut self, success: bool) {
        if success {
            self.total_successes += 1;
            self.consecutive_successes += 1;
        } else {
            self.total_failures += 1;
            self.consecutive_successes = 0;
        }

        // Check task completion
        if let Some(stage) = self.stages.get_mut(self.current_stage) {
            if let Some(task) = stage.tasks.get_mut(self.current_task) {
                let attempts = self.total_successes + self.total_failures;
                let rate = self.total_successes as f32 / attempts as f32;

                task.progress = rate;

                if rate >= task.criteria.success_rate
                    && attempts >= task.criteria.min_attempts
                    && self.consecutive_successes >= task.criteria.consecutive_successes
                {
                    task.completed = true;
                    self.completed_tasks.push(task.name.clone());
                    self.advance();
                }
            }
        }
    }

    /// Advance to next task/stage
    fn advance(&mut self) {
        if let Some(stage) = self.stages.get_mut(self.current_stage) {
            stage.check_completion();

            // Try next task in current stage
            if self.current_task + 1 < stage.tasks.len() {
                self.current_task += 1;
                self.reset_tracking();
                return;
            }

            // Try next stage
            if stage.completed && self.current_stage + 1 < self.stages.len() {
                self.current_stage += 1;
                self.current_task = 0;
                self.reset_tracking();
            }
        }
    }

    /// Reset tracking for new task
    fn reset_tracking(&mut self) {
        self.total_successes = 0;
        self.total_failures = 0;
        self.consecutive_successes = 0;
    }

    /// Overall progress (0-1)
    pub fn overall_progress(&self) -> f32 {
        if self.stages.is_empty() {
            return 0.0;
        }
        let total_tasks: usize = self.stages.iter().map(|s| s.tasks.len()).sum();
        if total_tasks == 0 {
            return 0.0;
        }
        self.completed_tasks.len() as f32 / total_tasks as f32
    }

    /// Stage count
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }
}

impl Default for CurriculumLearner {
    fn default() -> Self {
        Self::new()
    }
}
