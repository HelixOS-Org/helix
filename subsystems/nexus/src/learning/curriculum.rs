//! # Curriculum Learning for NEXUS
//!
//! Progressive learning with increasing difficulty.
//!
//! ## Features
//!
//! - Lesson ordering by difficulty
//! - Automatic difficulty estimation
//! - Progress tracking
//! - Mastery-based progression

extern crate alloc;

use alloc::collections::BTreeMap;
use crate::math::F64Ext;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// LESSON TYPES
// ============================================================================

/// Lesson identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LessonId(pub u32);

/// Difficulty level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LessonDifficulty {
    /// Beginner level
    Beginner,
    /// Easy level
    Easy,
    /// Medium level
    Medium,
    /// Hard level
    Hard,
    /// Expert level
    Expert,
}

impl LessonDifficulty {
    /// Convert to numeric value
    pub fn to_value(&self) -> f64 {
        match self {
            LessonDifficulty::Beginner => 0.1,
            LessonDifficulty::Easy => 0.25,
            LessonDifficulty::Medium => 0.5,
            LessonDifficulty::Hard => 0.75,
            LessonDifficulty::Expert => 0.9,
        }
    }

    /// Create from numeric value
    pub fn from_value(value: f64) -> Self {
        if value < 0.2 {
            LessonDifficulty::Beginner
        } else if value < 0.4 {
            LessonDifficulty::Easy
        } else if value < 0.6 {
            LessonDifficulty::Medium
        } else if value < 0.8 {
            LessonDifficulty::Hard
        } else {
            LessonDifficulty::Expert
        }
    }
}

/// Alias for backward compatibility
pub type DifficultyLevel = LessonDifficulty;

/// A lesson in the curriculum
#[derive(Debug, Clone)]
pub struct Lesson {
    /// Lesson ID
    pub id: LessonId,
    /// Lesson name
    pub name: String,
    /// Difficulty level
    pub difficulty: LessonDifficulty,
    /// Estimated difficulty (learned)
    pub estimated_difficulty: f64,
    /// Prerequisites
    pub prerequisites: Vec<LessonId>,
    /// Completion count
    pub completions: u64,
    /// Success count
    pub successes: u64,
    /// Average score
    pub avg_score: f64,
    /// Is mastered
    pub mastered: bool,
}

impl Lesson {
    /// Create new lesson
    pub fn new(id: LessonId, name: String, difficulty: LessonDifficulty) -> Self {
        Self {
            id,
            name,
            difficulty,
            estimated_difficulty: difficulty.to_value(),
            prerequisites: Vec::new(),
            completions: 0,
            successes: 0,
            avg_score: 0.0,
            mastered: false,
        }
    }

    /// Add prerequisite
    pub fn with_prerequisite(mut self, prereq: LessonId) -> Self {
        self.prerequisites.push(prereq);
        self
    }

    /// Record completion
    pub fn record_completion(&mut self, score: f64, success: bool) {
        self.completions += 1;
        if success {
            self.successes += 1;
        }

        // Update average score
        let n = self.completions as f64;
        self.avg_score = self.avg_score * (n - 1.0) / n + score / n;

        // Update estimated difficulty based on performance
        // Low score -> higher difficulty than expected
        // High score -> lower difficulty than expected
        let performance = score.clamp(0.0, 1.0);
        let difficulty_adjustment = 0.1 * (0.5 - performance);
        self.estimated_difficulty =
            (self.estimated_difficulty + difficulty_adjustment).clamp(0.0, 1.0);

        // Check mastery (80% success rate with 5+ attempts)
        if self.completions >= 5 {
            let success_rate = self.successes as f64 / self.completions as f64;
            self.mastered = success_rate >= 0.8 && self.avg_score >= 0.7;
        }
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.completions == 0 {
            0.0
        } else {
            self.successes as f64 / self.completions as f64
        }
    }
}

// ============================================================================
// TASK PROGRESSION
// ============================================================================

/// Task progression tracker
#[derive(Debug, Clone)]
pub struct TaskProgression {
    /// Current level
    pub level: u32,
    /// Current difficulty
    pub current_difficulty: f64,
    /// Target difficulty
    pub target_difficulty: f64,
    /// Difficulty increase rate
    pub increase_rate: f64,
    /// Minimum success rate to progress
    pub min_success_rate: f64,
    /// Recent success count
    recent_successes: u32,
    /// Recent total count
    recent_total: u32,
    /// Window size
    window_size: u32,
}

impl TaskProgression {
    /// Create new progression
    pub fn new() -> Self {
        Self {
            level: 1,
            current_difficulty: 0.1,
            target_difficulty: 1.0,
            increase_rate: 0.05,
            min_success_rate: 0.7,
            recent_successes: 0,
            recent_total: 0,
            window_size: 10,
        }
    }

    /// Record attempt
    pub fn record(&mut self, success: bool) {
        if self.recent_total >= self.window_size {
            // Sliding window approximation
            self.recent_successes = (self.recent_successes as f64 * 0.9) as u32;
            self.recent_total = (self.recent_total as f64 * 0.9) as u32;
        }

        self.recent_total += 1;
        if success {
            self.recent_successes += 1;
        }

        // Check for progression
        self.check_progression();
    }

    /// Check if should progress to next level
    fn check_progression(&mut self) {
        if self.recent_total < 5 {
            return;
        }

        let success_rate = self.recent_successes as f64 / self.recent_total as f64;

        if success_rate >= self.min_success_rate {
            // Progress!
            self.current_difficulty =
                (self.current_difficulty + self.increase_rate).min(self.target_difficulty);
            self.level += 1;
            // Reset counters
            self.recent_successes = 0;
            self.recent_total = 0;
        } else if success_rate < 0.3 && self.current_difficulty > 0.1 {
            // Regress if doing very poorly
            self.current_difficulty = (self.current_difficulty - self.increase_rate * 0.5).max(0.1);
            if self.level > 1 {
                self.level -= 1;
            }
            self.recent_successes = 0;
            self.recent_total = 0;
        }
    }

    /// Get current success rate
    pub fn success_rate(&self) -> f64 {
        if self.recent_total == 0 {
            0.0
        } else {
            self.recent_successes as f64 / self.recent_total as f64
        }
    }

    /// Is at max difficulty?
    pub fn is_complete(&self) -> bool {
        (self.current_difficulty - self.target_difficulty).abs() < 0.001
    }
}

impl Default for TaskProgression {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CURRICULUM LEARNER
// ============================================================================

/// Curriculum learner
pub struct CurriculumLearner {
    /// Available lessons
    lessons: BTreeMap<LessonId, Lesson>,
    /// Lesson order (sorted by difficulty)
    order: Vec<LessonId>,
    /// Current lesson index
    current_index: usize,
    /// Progression tracker
    progression: TaskProgression,
    /// Next lesson ID
    next_id: u32,
    /// Mastery threshold
    mastery_threshold: f64,
}

impl CurriculumLearner {
    /// Create new curriculum learner
    pub fn new() -> Self {
        Self {
            lessons: BTreeMap::new(),
            order: Vec::new(),
            current_index: 0,
            progression: TaskProgression::new(),
            next_id: 0,
            mastery_threshold: 0.8,
        }
    }

    /// Add lesson
    pub fn add_lesson(&mut self, lesson: Lesson) -> LessonId {
        let id = lesson.id;
        self.lessons.insert(id, lesson);
        self.reorder();
        id
    }

    /// Create and add lesson
    pub fn create_lesson(&mut self, name: String, difficulty: LessonDifficulty) -> LessonId {
        let id = LessonId(self.next_id);
        self.next_id += 1;
        let lesson = Lesson::new(id, name, difficulty);
        self.add_lesson(lesson)
    }

    /// Reorder lessons by difficulty
    fn reorder(&mut self) {
        let mut lessons: Vec<(LessonId, f64)> = self
            .lessons
            .iter()
            .map(|(&id, lesson)| (id, lesson.estimated_difficulty))
            .collect();

        lessons.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

        self.order = lessons.into_iter().map(|(id, _)| id).collect();
    }

    /// Get current lesson
    pub fn current_lesson(&self) -> Option<&Lesson> {
        self.order
            .get(self.current_index)
            .and_then(|id| self.lessons.get(id))
    }

    /// Check if prerequisites are met for lesson
    pub fn prerequisites_met(&self, lesson_id: LessonId) -> bool {
        let lesson = match self.lessons.get(&lesson_id) {
            Some(l) => l,
            None => return false,
        };

        lesson.prerequisites.iter().all(|prereq_id| {
            self.lessons
                .get(prereq_id)
                .map(|l| l.mastered)
                .unwrap_or(true) // If prereq doesn't exist, consider it met
        })
    }

    /// Get next available lesson
    pub fn next_lesson(&self) -> Option<&Lesson> {
        // Find first non-mastered lesson with met prerequisites
        for &id in &self.order {
            let lesson = match self.lessons.get(&id) {
                Some(l) => l,
                None => continue,
            };

            if !lesson.mastered && self.prerequisites_met(id) {
                return Some(lesson);
            }
        }

        // All mastered or prerequisites not met
        None
    }

    /// Record completion of current lesson
    pub fn record_completion(&mut self, score: f64) {
        let success = score >= self.mastery_threshold;

        // Update current lesson
        if let Some(&id) = self.order.get(self.current_index) {
            if let Some(lesson) = self.lessons.get_mut(&id) {
                lesson.record_completion(score, success);
            }
        }

        // Update progression
        self.progression.record(success);

        // Move to next lesson if mastered
        if success && self.current_index < self.order.len() - 1 {
            // Check if current is mastered
            let mastered = self
                .order
                .get(self.current_index)
                .and_then(|id| self.lessons.get(id))
                .map(|l| l.mastered)
                .unwrap_or(false);

            if mastered {
                self.current_index += 1;
            }
        }

        // Reorder based on updated difficulty estimates
        self.reorder();
    }

    /// Get lesson by ID
    pub fn get_lesson(&self, id: LessonId) -> Option<&Lesson> {
        self.lessons.get(&id)
    }

    /// Get lesson count
    pub fn lesson_count(&self) -> usize {
        self.lessons.len()
    }

    /// Get mastered count
    pub fn mastered_count(&self) -> usize {
        self.lessons.values().filter(|l| l.mastered).count()
    }

    /// Get progress percentage
    pub fn progress(&self) -> f64 {
        if self.lessons.is_empty() {
            return 0.0;
        }
        self.mastered_count() as f64 / self.lessons.len() as f64
    }

    /// Get current difficulty level
    pub fn current_difficulty(&self) -> LessonDifficulty {
        LessonDifficulty::from_value(self.progression.current_difficulty)
    }

    /// Get curriculum status
    pub fn status(&self) -> CurriculumStatus {
        CurriculumStatus {
            total_lessons: self.lessons.len(),
            mastered: self.mastered_count(),
            current_level: self.progression.level,
            current_difficulty: self.progression.current_difficulty,
            overall_progress: self.progress(),
        }
    }
}

impl Default for CurriculumLearner {
    fn default() -> Self {
        Self::new()
    }
}

/// Curriculum status
#[derive(Debug, Clone)]
pub struct CurriculumStatus {
    /// Total lessons
    pub total_lessons: usize,
    /// Mastered lessons
    pub mastered: usize,
    /// Current level
    pub current_level: u32,
    /// Current difficulty
    pub current_difficulty: f64,
    /// Overall progress (0-1)
    pub overall_progress: f64,
}

// ============================================================================
// DIFFICULTY ESTIMATOR
// ============================================================================

/// Automatic difficulty estimation
pub struct DifficultyEstimator {
    /// Feature weights for difficulty prediction
    weights: Vec<f64>,
    /// Bias
    bias: f64,
    /// Learning rate
    learning_rate: f64,
    /// Samples seen
    samples: u64,
}

impl DifficultyEstimator {
    /// Create new estimator
    pub fn new(feature_dim: usize) -> Self {
        Self {
            weights: vec![0.0; feature_dim],
            bias: 0.5, // Start with medium difficulty
            learning_rate: 0.1,
            samples: 0,
        }
    }

    /// Estimate difficulty from features
    pub fn estimate(&self, features: &[f64]) -> f64 {
        let score: f64 = self
            .weights
            .iter()
            .zip(features.iter())
            .map(|(w, f)| w * f)
            .sum::<f64>()
            + self.bias;

        // Sigmoid to bound output
        1.0 / (1.0 + (-score).exp())
    }

    /// Update from performance
    pub fn update(&mut self, features: &[f64], actual_difficulty: f64) {
        let predicted = self.estimate(features);
        let error = predicted - actual_difficulty;

        // Gradient descent
        for (w, f) in self.weights.iter_mut().zip(features.iter()) {
            *w -= self.learning_rate * error * f * predicted * (1.0 - predicted);
        }
        self.bias -= self.learning_rate * error * predicted * (1.0 - predicted);

        self.samples += 1;

        // Decay learning rate
        if self.samples % 100 == 0 {
            self.learning_rate *= 0.95;
        }
    }

    /// Infer difficulty from performance score
    /// Low score = high difficulty, high score = low difficulty
    pub fn infer_from_performance(performance: f64) -> f64 {
        (1.0 - performance).clamp(0.0, 1.0)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lesson_difficulty() {
        assert!(LessonDifficulty::Beginner.to_value() < LessonDifficulty::Expert.to_value());

        let diff = LessonDifficulty::from_value(0.3);
        assert_eq!(diff, LessonDifficulty::Easy);
    }

    #[test]
    fn test_lesson_completion() {
        let mut lesson = Lesson::new(LessonId(0), String::from("test"), LessonDifficulty::Easy);

        // Record good performance
        for _ in 0..10 {
            lesson.record_completion(0.9, true);
        }

        assert!(lesson.mastered);
        assert!(lesson.avg_score > 0.8);
    }

    #[test]
    fn test_task_progression() {
        let mut progression = TaskProgression::new();

        // Record successes
        for _ in 0..10 {
            progression.record(true);
        }

        // Should have progressed
        assert!(progression.level > 1);
        assert!(progression.current_difficulty > 0.1);
    }

    #[test]
    fn test_curriculum_learner() {
        let mut curriculum = CurriculumLearner::new();

        curriculum.create_lesson(String::from("basics"), LessonDifficulty::Beginner);
        curriculum.create_lesson(String::from("intermediate"), LessonDifficulty::Medium);
        curriculum.create_lesson(String::from("advanced"), LessonDifficulty::Expert);

        assert_eq!(curriculum.lesson_count(), 3);

        let current = curriculum.current_lesson();
        assert!(current.is_some());
        // Should be beginner lesson (lowest difficulty)
        assert_eq!(current.unwrap().difficulty, LessonDifficulty::Beginner);
    }

    #[test]
    fn test_curriculum_progression() {
        let mut curriculum = CurriculumLearner::new();

        let id1 = curriculum.create_lesson(String::from("lesson1"), LessonDifficulty::Beginner);
        let _id2 = curriculum.create_lesson(String::from("lesson2"), LessonDifficulty::Easy);

        // Master first lesson
        for _ in 0..10 {
            curriculum.record_completion(0.95);
        }

        // Check progress
        assert!(
            curriculum.mastered_count() > 0
                || curriculum.get_lesson(id1).unwrap().success_rate() > 0.5
        );
    }

    #[test]
    fn test_difficulty_estimator() {
        let mut estimator = DifficultyEstimator::new(3);

        // Train on examples
        for _ in 0..100 {
            // Easy task (low features, low difficulty)
            estimator.update(&[0.1, 0.1, 0.1], 0.2);
            // Hard task (high features, high difficulty)
            estimator.update(&[0.9, 0.9, 0.9], 0.8);
        }

        // Should predict accordingly
        let easy_pred = estimator.estimate(&[0.1, 0.1, 0.1]);
        let hard_pred = estimator.estimate(&[0.9, 0.9, 0.9]);

        assert!(hard_pred > easy_pred);
    }
}
