//! # Intent Recognition Engine
//!
//! The Intent Engine analyzes user behavior, system patterns, and contextual
//! signals to understand user intent and anticipate future actions.
//!
//! ## Capabilities
//!
//! - **Behavioral Analysis**: Learn from user interaction patterns
//! - **Goal Inference**: Deduce user objectives from sequences of actions
//! - **Anticipation**: Predict what the user will do next
//! - **Context Awareness**: Understand the current user context
//! - **Natural Language**: Process text-based intents (if enabled)
//!
//! ## Architecture
//!
//! ```text
//!     User Actions ─────►┌─────────────────────────────────────┐
//!                        │         Intent Engine               │
//!     System Events ────►│                                     │
//!                        │  ┌─────────────────────────────┐    │
//!     Context Data ─────►│  │    Action Sequence Buffer   │    │
//!                        │  └──────────────┬──────────────┘    │
//!                        │                 │                    │
//!                        │                 ▼                    │
//!                        │  ┌─────────────────────────────┐    │
//!                        │  │    Sequence Analyzer        │    │
//!                        │  │    - Pattern Matching       │    │
//!                        │  │    - N-gram Analysis        │    │
//!                        │  │    - Goal Detection         │    │
//!                        │  └──────────────┬──────────────┘    │
//!                        │                 │                    │
//!                        │                 ▼                    │
//!                        │  ┌─────────────────────────────┐    │
//!                        │  │    Context Integrator       │    │
//!                        │  │    - Time of Day            │    │
//!                        │  │    - Session State          │    │
//!                        │  │    - Resource State         │    │
//!                        │  └──────────────┬──────────────┘    │
//!                        │                 │                    │
//!                        │                 ▼                    │
//!                        │  ┌─────────────────────────────┐    │
//!                        │  │    Intent Classifier        │──────────► Intent
//!                        │  └─────────────────────────────┘    │
//!                        │                                     │
//!                        └─────────────────────────────────────┘
//! ```

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};

use spin::{Mutex, RwLock};

use crate::core::{
    AiAction, AiEvent, Confidence, DecisionContext, ResourceType, UserActionType, UserContext,
    WorkloadCategory,
};

// =============================================================================
// Intent Engine
// =============================================================================

/// The Intent Recognition Engine
pub struct IntentEngine {
    /// Whether the engine is enabled
    enabled: bool,

    /// Action sequence buffer for pattern detection
    action_buffer: Mutex<ActionBuffer>,

    /// Learned action sequences
    known_sequences: RwLock<Vec<ActionSequence>>,

    /// Current user context
    current_context: RwLock<UserContext>,

    /// Goal detector
    goal_detector: GoalDetector,

    /// Statistics
    stats: IntentStats,
}

/// Statistics for intent engine
struct IntentStats {
    actions_processed: AtomicU64,
    intents_recognized: AtomicU64,
    predictions_made: AtomicU64,
    predictions_correct: AtomicU64,
}

impl Default for IntentStats {
    fn default() -> Self {
        Self {
            actions_processed: AtomicU64::new(0),
            intents_recognized: AtomicU64::new(0),
            predictions_made: AtomicU64::new(0),
            predictions_correct: AtomicU64::new(0),
        }
    }
}

/// Buffer for recent user actions
struct ActionBuffer {
    actions: VecDeque<TimestampedAction>,
    max_size: usize,
}

impl ActionBuffer {
    fn new(max_size: usize) -> Self {
        Self {
            actions: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    fn push(&mut self, action: TimestampedAction) {
        if self.actions.len() >= self.max_size {
            self.actions.pop_front();
        }
        self.actions.push_back(action);
    }

    fn recent(&self, count: usize) -> Vec<&TimestampedAction> {
        self.actions.iter().rev().take(count).collect()
    }

    fn clear(&mut self) {
        self.actions.clear();
    }
}

/// An action with timestamp
#[derive(Debug, Clone)]
pub struct TimestampedAction {
    /// Type of user action
    pub action_type: UserActionType,
    /// When the action occurred
    pub timestamp: u64,
    /// Details about the action
    pub details: ActionDetails,
}

/// Details about a user action
#[derive(Debug, Clone, Default)]
pub struct ActionDetails {
    /// Process that performed the action
    pub process_id: Option<u64>,
    /// File or resource involved
    pub resource: Option<String>,
    /// Additional metadata
    pub metadata: Vec<(String, String)>,
}

/// A learned action sequence
#[derive(Debug, Clone)]
pub struct ActionSequence {
    /// Unique identifier
    pub id: u64,
    /// Sequence of action types
    pub actions: Vec<UserActionType>,
    /// Likely next action
    pub predicted_next: Option<UserActionType>,
    /// Associated intent class
    pub intent: IntentClass,
    /// How often this sequence has been observed
    pub frequency: u32,
    /// Confidence in this pattern
    pub confidence: Confidence,
}

// =============================================================================
// Intent Classification
// =============================================================================

/// Recognized intent
#[derive(Debug, Clone)]
pub struct Intent {
    /// Intent class
    pub class: IntentClass,
    /// Specific goal if identified
    pub goal: Option<UserGoal>,
    /// Confidence in this classification
    pub confidence: Confidence,
    /// Context that led to this intent
    pub context: IntentContext,
    /// Predicted next actions
    pub predicted_actions: Vec<PredictedAction>,
    /// Suggested AI actions to assist user
    pub suggestions: Vec<IntentSuggestion>,
}

/// Classification of user intent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntentClass {
    /// User is starting the system
    SystemStartup,
    /// User is shutting down
    SystemShutdown,
    /// User is doing development work
    Development,
    /// User is browsing/reading
    ContentConsumption,
    /// User is creating content
    ContentCreation,
    /// User is communicating (email, chat)
    Communication,
    /// User is doing computation-heavy work
    Computation,
    /// User is gaming
    Gaming,
    /// User is administering the system
    SystemAdministration,
    /// User is installing/updating software
    SoftwareManagement,
    /// User is managing files
    FileManagement,
    /// User is doing multimedia work
    Multimedia,
    /// User is idle but system is active
    BackgroundActivity,
    /// User is away from system
    Idle,
    /// Intent is unclear
    Unknown,
}

impl Default for IntentClass {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Specific user goal
#[derive(Debug, Clone)]
pub enum UserGoal {
    /// Compile a project
    CompileProject {
        /// Path to the project
        project_path: String,
    },
    /// Install a package
    InstallPackage {
        /// Name of the package
        package_name: String,
    },
    /// Update system
    UpdateSystem,
    /// Launch an application
    LaunchApplication {
        /// Name of the application
        app_name: String,
    },
    /// Open a file
    OpenFile {
        /// Path to the file
        file_path: String,
    },
    /// Complete a file transfer
    FileTransfer {
        /// Source path
        source: String,
        /// Destination path
        destination: String,
    },
    /// Search for something
    Search {
        /// Search query
        query: String,
    },
    /// Configure settings
    ConfigureSettings {
        /// Settings category
        category: String,
    },
    /// Custom goal
    Custom {
        /// Goal description
        description: String,
    },
}

/// Context for intent detection
#[derive(Debug, Clone, Default)]
pub struct IntentContext {
    /// Recent action sequence
    pub recent_actions: Vec<UserActionType>,
    /// Active processes
    pub active_processes: Vec<String>,
    /// Time of day
    pub time_of_day: u8,
    /// Session duration
    pub session_duration_min: u32,
    /// Current workload
    pub workload: WorkloadCategory,
}

/// Predicted future action
#[derive(Debug, Clone)]
pub struct PredictedAction {
    /// Predicted action type
    pub action: UserActionType,
    /// Expected time until action (seconds)
    pub expected_delay_s: u32,
    /// Confidence in prediction
    pub confidence: Confidence,
}

/// Suggestion for assisting user
#[derive(Debug, Clone)]
pub struct IntentSuggestion {
    /// Description of suggestion
    pub description: String,
    /// AI action to perform
    pub action: AiAction,
    /// Benefit to user
    pub benefit: String,
    /// Confidence that this helps
    pub confidence: Confidence,
}

// =============================================================================
// Goal Detection
// =============================================================================

/// Detects user goals from action sequences
struct GoalDetector {
    /// Known goal patterns
    patterns: Vec<GoalPattern>,
}

/// Pattern that indicates a goal
struct GoalPattern {
    /// Actions that lead to this goal
    trigger_sequence: Vec<UserActionType>,
    /// The inferred goal
    goal: IntentClass,
    /// Minimum sequence length to match
    min_length: usize,
    /// Confidence factor
    confidence_factor: f32,
}

impl Default for GoalDetector {
    fn default() -> Self {
        Self {
            patterns: Self::default_patterns(),
        }
    }
}

impl GoalDetector {
    fn new() -> Self {
        Self::default()
    }

    /// Default goal patterns based on common workflows
    fn default_patterns() -> Vec<GoalPattern> {
        vec![
            // Development patterns
            GoalPattern {
                trigger_sequence: vec![
                    UserActionType::FileOperation,
                    UserActionType::FileOperation,
                    UserActionType::ProcessLaunch,
                ],
                goal: IntentClass::Development,
                min_length: 2,
                confidence_factor: 0.8,
            },
            // System administration
            GoalPattern {
                trigger_sequence: vec![
                    UserActionType::SystemSetting,
                    UserActionType::SystemSetting,
                ],
                goal: IntentClass::SystemAdministration,
                min_length: 2,
                confidence_factor: 0.7,
            },
            // File management
            GoalPattern {
                trigger_sequence: vec![
                    UserActionType::FileOperation,
                    UserActionType::FileOperation,
                    UserActionType::FileOperation,
                ],
                goal: IntentClass::FileManagement,
                min_length: 3,
                confidence_factor: 0.75,
            },
            // Communication
            GoalPattern {
                trigger_sequence: vec![
                    UserActionType::NetworkAccess,
                    UserActionType::ProcessLaunch,
                ],
                goal: IntentClass::Communication,
                min_length: 2,
                confidence_factor: 0.6,
            },
        ]
    }

    /// Detect goal from action sequence
    fn detect(&self, actions: &[UserActionType]) -> Option<(IntentClass, Confidence)> {
        if actions.is_empty() {
            return None;
        }

        let mut best_match: Option<(IntentClass, f32)> = None;

        for pattern in &self.patterns {
            if actions.len() < pattern.min_length {
                continue;
            }

            let match_score = self.pattern_match_score(actions, &pattern.trigger_sequence);
            let adjusted_score = match_score * pattern.confidence_factor;

            if let Some((_, current_best)) = best_match {
                if adjusted_score > current_best {
                    best_match = Some((pattern.goal, adjusted_score));
                }
            } else if adjusted_score > 0.3 {
                best_match = Some((pattern.goal, adjusted_score));
            }
        }

        best_match.map(|(goal, score)| (goal, Confidence::new(score)))
    }

    /// Calculate pattern match score (0.0 to 1.0)
    fn pattern_match_score(&self, actions: &[UserActionType], pattern: &[UserActionType]) -> f32 {
        if pattern.is_empty() || actions.is_empty() {
            return 0.0;
        }

        let mut matches = 0;
        let pattern_len = pattern.len();
        let actions_len = actions.len();

        // Look for pattern in recent actions
        for window in actions.windows(core::cmp::min(pattern_len, actions_len)) {
            let window_matches: usize = window
                .iter()
                .zip(pattern.iter())
                .filter(|(a, p)| a == p)
                .count();
            matches = matches.max(window_matches);
        }

        matches as f32 / pattern_len as f32
    }
}

// =============================================================================
// Intent Engine Implementation
// =============================================================================

impl IntentEngine {
    /// Maximum action buffer size
    const MAX_BUFFER_SIZE: usize = 1000;

    /// Create a new Intent Engine
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            action_buffer: Mutex::new(ActionBuffer::new(Self::MAX_BUFFER_SIZE)),
            known_sequences: RwLock::new(Vec::new()),
            current_context: RwLock::new(UserContext::default()),
            goal_detector: GoalDetector::new(),
            stats: IntentStats::default(),
        }
    }

    /// Check if engine is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a user action
    pub fn record_action(&self, action: TimestampedAction) {
        if !self.enabled {
            return;
        }

        self.action_buffer.lock().push(action);
        self.stats.actions_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Update user context
    pub fn update_context(&self, context: UserContext) {
        *self.current_context.write() = context;
    }

    /// Analyze an event and possibly recommend an action
    pub fn analyze(
        &self,
        event: &AiEvent,
        _context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        if !self.enabled {
            return Ok(None);
        }

        match event {
            AiEvent::UserAction {
                action_type,
                context: user_ctx,
            } => self.handle_user_action(*action_type, user_ctx),
            AiEvent::UserPattern { pattern_id } => self.handle_pattern(*pattern_id),
            AiEvent::ProcessSpawn { pid, name } => self.handle_process_spawn(*pid, name),
            _ => Ok(None),
        }
    }

    /// Handle a user action event
    fn handle_user_action(
        &self,
        action_type: UserActionType,
        user_ctx: &UserContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        // Update context
        *self.current_context.write() = user_ctx.clone();

        // Record action
        self.record_action(TimestampedAction {
            action_type,
            timestamp: 0, // Would be real timestamp
            details: ActionDetails::default(),
        });

        // Detect intent
        if let Some(intent) = self.detect_intent() {
            self.stats
                .intents_recognized
                .fetch_add(1, Ordering::Relaxed);

            // Generate proactive assistance
            if let Some(suggestion) = intent.suggestions.first() {
                if suggestion.confidence.meets_threshold(0.7) {
                    return Ok(Some((
                        suggestion.action.clone(),
                        suggestion.confidence,
                        suggestion.description.clone(),
                    )));
                }
            }
        }

        Ok(None)
    }

    /// Handle pattern detection event
    fn handle_pattern(
        &self,
        pattern_id: u64,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        // Look up known sequence
        let sequences = self.known_sequences.read();
        if let Some(seq) = sequences.iter().find(|s| s.id == pattern_id) {
            if seq.confidence.meets_threshold(0.8) {
                if let Some(predicted) = &seq.predicted_next {
                    // Proactively prepare for next action
                    return Ok(Some((
                        self.prepare_for_action(predicted),
                        seq.confidence,
                        format!("Preparing for predicted action: {:?}", predicted),
                    )));
                }
            }
        }
        Ok(None)
    }

    /// Handle process spawn event
    fn handle_process_spawn(
        &self,
        _pid: u64,
        name: &str,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        // Infer intent from process name
        let intent = self.infer_intent_from_process(name);

        if intent != IntentClass::Unknown {
            // Adjust resources based on intent
            let action = match intent {
                IntentClass::Development => AiAction::TuneScheduler {
                    granularity_ns: 5_000_000, // 5ms for dev responsiveness
                    preemption: true,
                },
                IntentClass::Gaming => AiAction::TuneScheduler {
                    granularity_ns: 1_000_000, // 1ms for low latency gaming
                    preemption: true,
                },
                IntentClass::Computation => AiAction::PreallocateResources {
                    resource: ResourceType::Cpu,
                    amount: 4, // Reserve CPU cores
                },
                IntentClass::Multimedia => AiAction::PreallocateResources {
                    resource: ResourceType::Memory,
                    amount: 512 * 1024 * 1024, // Reserve memory
                },
                _ => return Ok(None),
            };

            return Ok(Some((
                action,
                Confidence::new(0.7),
                format!(
                    "Optimizing for {} workflow (process: {})",
                    self.intent_name(intent),
                    name
                ),
            )));
        }

        Ok(None)
    }

    /// Detect current user intent
    pub fn detect_intent(&self) -> Option<Intent> {
        let buffer = self.action_buffer.lock();
        let recent = buffer.recent(10);

        if recent.is_empty() {
            return None;
        }

        let action_types: Vec<UserActionType> = recent.iter().map(|a| a.action_type).collect();

        // Detect goal
        let (intent_class, confidence) = self
            .goal_detector
            .detect(&action_types)
            .unwrap_or((IntentClass::Unknown, Confidence::MIN));

        if intent_class == IntentClass::Unknown {
            return None;
        }

        let context = self.current_context.read().clone();

        // Predict next actions
        let predicted_actions = self.predict_next_actions(&action_types);

        // Generate suggestions
        let suggestions = self.generate_suggestions(intent_class, &context);

        Some(Intent {
            class: intent_class,
            goal: None, // Could be enhanced with specific goal detection
            confidence,
            context: IntentContext {
                recent_actions: action_types,
                active_processes: Vec::new(),
                time_of_day: context.hour_of_day,
                session_duration_min: context.session_duration_min,
                workload: context.workload_category,
            },
            predicted_actions,
            suggestions,
        })
    }

    /// Predict next actions based on history
    fn predict_next_actions(&self, recent: &[UserActionType]) -> Vec<PredictedAction> {
        let sequences = self.known_sequences.read();
        let mut predictions = Vec::new();

        for seq in sequences.iter() {
            // Check if current actions match start of known sequence
            if recent.len() >= seq.actions.len() {
                continue;
            }

            let match_len = recent.len();
            let matches = recent
                .iter()
                .zip(seq.actions.iter())
                .filter(|(a, b)| a == b)
                .count();

            if matches == match_len && match_len > 0 {
                // Predict next action in sequence
                if let Some(next) = seq.actions.get(match_len) {
                    predictions.push(PredictedAction {
                        action: *next,
                        expected_delay_s: 5,
                        confidence: Confidence::new(
                            seq.confidence.value() * (matches as f32 / match_len as f32),
                        ),
                    });
                }
            }
        }

        predictions.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        predictions.truncate(3);
        predictions
    }

    /// Generate suggestions based on intent
    fn generate_suggestions(
        &self,
        intent: IntentClass,
        _context: &UserContext,
    ) -> Vec<IntentSuggestion> {
        let mut suggestions = Vec::new();

        match intent {
            IntentClass::Development => {
                suggestions.push(IntentSuggestion {
                    description: "Optimize for development workload".to_string(),
                    action: AiAction::TuneScheduler {
                        granularity_ns: 1_000_000, // 1ms for responsiveness
                        preemption: true,
                    },
                    benefit: "Faster IDE responsiveness".to_string(),
                    confidence: Confidence::new(0.8),
                });

                suggestions.push(IntentSuggestion {
                    description: "Pre-cache build dependencies".to_string(),
                    action: AiAction::PreallocateResources {
                        resource: ResourceType::Memory,
                        amount: 256 * 1024 * 1024,
                    },
                    benefit: "Faster compilation times".to_string(),
                    confidence: Confidence::new(0.7),
                });
            },

            IntentClass::Gaming => {
                suggestions.push(IntentSuggestion {
                    description: "Enable gaming performance mode".to_string(),
                    action: AiAction::SetPowerProfile {
                        profile: crate::core::PowerProfile::Performance,
                    },
                    benefit: "Maximum FPS and responsiveness".to_string(),
                    confidence: Confidence::new(0.9),
                });
            },

            IntentClass::Multimedia => {
                suggestions.push(IntentSuggestion {
                    description: "Optimize for media processing".to_string(),
                    action: AiAction::PreallocateResources {
                        resource: ResourceType::Memory,
                        amount: 1024 * 1024 * 1024,
                    },
                    benefit: "Smoother video editing".to_string(),
                    confidence: Confidence::new(0.75),
                });
            },

            IntentClass::SystemAdministration => {
                suggestions.push(IntentSuggestion {
                    description: "Prepare system analysis tools".to_string(),
                    action: AiAction::PreallocateResources {
                        resource: ResourceType::Cpu,
                        amount: 2,
                    },
                    benefit: "Faster system diagnostics".to_string(),
                    confidence: Confidence::new(0.65),
                });
            },

            _ => {},
        }

        suggestions
    }

    /// Prepare for a predicted action
    fn prepare_for_action(&self, action: &UserActionType) -> AiAction {
        match action {
            UserActionType::FileOperation => AiAction::PreallocateResources {
                resource: ResourceType::Disk,
                amount: 64 * 1024 * 1024,
            },
            UserActionType::ProcessLaunch => AiAction::PreallocateResources {
                resource: ResourceType::Memory,
                amount: 128 * 1024 * 1024,
            },
            UserActionType::NetworkAccess => AiAction::PreallocateResources {
                resource: ResourceType::Network,
                amount: 1,
            },
            _ => AiAction::NoOp,
        }
    }

    /// Infer intent from process name
    fn infer_intent_from_process(&self, name: &str) -> IntentClass {
        let name_lower = name.to_lowercase();

        // Development tools
        if name_lower.contains("code")
            || name_lower.contains("vim")
            || name_lower.contains("emacs")
            || name_lower.contains("cargo")
            || name_lower.contains("rustc")
            || name_lower.contains("gcc")
            || name_lower.contains("clang")
            || name_lower.contains("node")
            || name_lower.contains("python")
            || name_lower.contains("idea")
            || name_lower.contains("studio")
        {
            return IntentClass::Development;
        }

        // Gaming
        if name_lower.contains("game")
            || name_lower.contains("steam")
            || name_lower.contains("wine")
            || name_lower.contains("proton")
        {
            return IntentClass::Gaming;
        }

        // Multimedia
        if name_lower.contains("vlc")
            || name_lower.contains("mpv")
            || name_lower.contains("ffmpeg")
            || name_lower.contains("obs")
            || name_lower.contains("kdenlive")
            || name_lower.contains("audacity")
        {
            return IntentClass::Multimedia;
        }

        // Communication
        if name_lower.contains("discord")
            || name_lower.contains("slack")
            || name_lower.contains("teams")
            || name_lower.contains("zoom")
            || name_lower.contains("thunderbird")
        {
            return IntentClass::Communication;
        }

        // Computation
        if name_lower.contains("blender")
            || name_lower.contains("matlab")
            || name_lower.contains("julia")
            || name_lower.contains("octave")
        {
            return IntentClass::Computation;
        }

        IntentClass::Unknown
    }

    /// Get human-readable intent name
    fn intent_name(&self, intent: IntentClass) -> &'static str {
        match intent {
            IntentClass::SystemStartup => "system startup",
            IntentClass::SystemShutdown => "system shutdown",
            IntentClass::Development => "development",
            IntentClass::ContentConsumption => "content consumption",
            IntentClass::ContentCreation => "content creation",
            IntentClass::Communication => "communication",
            IntentClass::Computation => "computation",
            IntentClass::Gaming => "gaming",
            IntentClass::SystemAdministration => "system administration",
            IntentClass::SoftwareManagement => "software management",
            IntentClass::FileManagement => "file management",
            IntentClass::Multimedia => "multimedia",
            IntentClass::BackgroundActivity => "background activity",
            IntentClass::Idle => "idle",
            IntentClass::Unknown => "unknown",
        }
    }

    /// Learn a new action sequence
    pub fn learn_sequence(&self, actions: Vec<UserActionType>, intent: IntentClass) {
        let mut sequences = self.known_sequences.write();

        // Check if sequence already exists
        for seq in sequences.iter_mut() {
            if seq.actions == actions {
                seq.frequency += 1;
                // Increase confidence with repeated observations
                let new_conf = (seq.confidence.value() + 0.05).min(0.95);
                seq.confidence = Confidence::new(new_conf);
                return;
            }
        }

        // Add new sequence
        static SEQUENCE_COUNTER: AtomicU64 = AtomicU64::new(1);
        let id = SEQUENCE_COUNTER.fetch_add(1, Ordering::Relaxed);

        let predicted_next = if !actions.is_empty() {
            Some(actions[actions.len() - 1])
        } else {
            None
        };

        sequences.push(ActionSequence {
            id,
            actions,
            predicted_next,
            intent,
            frequency: 1,
            confidence: Confidence::new(0.5),
        });
    }

    /// Get prediction accuracy
    pub fn prediction_accuracy(&self) -> f32 {
        let made = self.stats.predictions_made.load(Ordering::Relaxed);
        let correct = self.stats.predictions_correct.load(Ordering::Relaxed);

        if made == 0 {
            return 0.0;
        }

        correct as f32 / made as f32
    }

    /// Verify a prediction (for learning feedback)
    pub fn verify_prediction(&self, predicted: UserActionType, actual: UserActionType) {
        self.stats.predictions_made.fetch_add(1, Ordering::Relaxed);
        if predicted == actual {
            self.stats
                .predictions_correct
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Clear action buffer
    pub fn clear_buffer(&self) {
        self.action_buffer.lock().clear();
    }

    /// Get statistics
    pub fn statistics(&self) -> IntentEngineStatistics {
        IntentEngineStatistics {
            enabled: self.enabled,
            actions_processed: self.stats.actions_processed.load(Ordering::Relaxed),
            intents_recognized: self.stats.intents_recognized.load(Ordering::Relaxed),
            predictions_made: self.stats.predictions_made.load(Ordering::Relaxed),
            predictions_correct: self.stats.predictions_correct.load(Ordering::Relaxed),
            known_sequences: self.known_sequences.read().len(),
            prediction_accuracy: self.prediction_accuracy(),
        }
    }
}

/// Public statistics structure
#[derive(Debug, Clone)]
pub struct IntentEngineStatistics {
    /// Whether the engine is enabled
    pub enabled: bool,
    /// Total actions processed
    pub actions_processed: u64,
    /// Intents successfully recognized
    pub intents_recognized: u64,
    /// Total predictions made
    pub predictions_made: u64,
    /// Correct predictions
    pub predictions_correct: u64,
    /// Number of known action sequences
    pub known_sequences: usize,
    /// Overall prediction accuracy (0.0 - 1.0)
    pub prediction_accuracy: f32,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_engine_creation() {
        let engine = IntentEngine::new(true);
        assert!(engine.is_enabled());

        let disabled = IntentEngine::new(false);
        assert!(!disabled.is_enabled());
    }

    #[test]
    fn test_action_buffer() {
        let mut buffer = ActionBuffer::new(5);

        for i in 0..10 {
            buffer.push(TimestampedAction {
                action_type: UserActionType::FileOperation,
                timestamp: i,
                details: ActionDetails::default(),
            });
        }

        assert_eq!(buffer.actions.len(), 5);
        assert_eq!(buffer.recent(3).len(), 3);
    }

    #[test]
    fn test_goal_detection() {
        let detector = GoalDetector::new();

        let actions = vec![
            UserActionType::FileOperation,
            UserActionType::FileOperation,
            UserActionType::ProcessLaunch,
        ];

        let result = detector.detect(&actions);
        assert!(result.is_some());

        let (intent, confidence) = result.unwrap();
        assert_eq!(intent, IntentClass::Development);
        assert!(confidence.value() > 0.5);
    }

    #[test]
    fn test_process_intent_inference() {
        let engine = IntentEngine::new(true);

        assert_eq!(
            engine.infer_intent_from_process("visual-studio-code"),
            IntentClass::Development
        );
        assert_eq!(
            engine.infer_intent_from_process("steam"),
            IntentClass::Gaming
        );
        assert_eq!(
            engine.infer_intent_from_process("vlc"),
            IntentClass::Multimedia
        );
        assert_eq!(
            engine.infer_intent_from_process("unknown-app"),
            IntentClass::Unknown
        );
    }

    #[test]
    fn test_sequence_learning() {
        let engine = IntentEngine::new(true);

        let actions = vec![UserActionType::FileOperation, UserActionType::ProcessLaunch];

        engine.learn_sequence(actions.clone(), IntentClass::Development);
        assert_eq!(engine.known_sequences.read().len(), 1);

        // Learning same sequence increases frequency
        engine.learn_sequence(actions, IntentClass::Development);
        assert_eq!(engine.known_sequences.read().len(), 1);
        assert_eq!(engine.known_sequences.read()[0].frequency, 2);
    }
}
