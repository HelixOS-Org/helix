//! NEXUS Year 2: Reactive Behavior System
//!
//! Reactive behavior patterns for immediate stimulus-response behavior.
//! Implements layered reactive architectures similar to subsumption.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for stimuli
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StimulusId(pub u64);

impl StimulusId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResponseId(pub u64);

impl ResponseId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for behaviors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BehaviorId(pub u64);

impl BehaviorId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Priority level for behaviors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BehaviorPriority {
    /// Emergency behaviors (highest)
    Emergency  = 5,
    /// Critical behaviors
    Critical   = 4,
    /// High priority
    High       = 3,
    /// Normal priority
    Normal     = 2,
    /// Low priority
    Low        = 1,
    /// Background (lowest)
    Background = 0,
}

impl BehaviorPriority {
    pub fn as_i32(&self) -> i32 {
        *self as i32
    }
}

/// Stimulus data value
#[derive(Debug, Clone)]
pub enum StimulusValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Vector(Vec<f64>),
    Bytes(Vec<u8>),
}

impl StimulusValue {
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            Self::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }
}

/// A stimulus from the environment
#[derive(Debug, Clone)]
pub struct Stimulus {
    pub id: StimulusId,
    pub name: String,
    pub value: StimulusValue,
    pub intensity: f32,
    pub timestamp: u64,
    pub source: Option<String>,
}

impl Stimulus {
    pub fn new(id: StimulusId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            value: StimulusValue::None,
            intensity: 1.0,
            timestamp: 0,
            source: None,
        }
    }

    pub fn with_value(mut self, value: StimulusValue) -> Self {
        self.value = value;
        self
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

/// Response to a stimulus
#[derive(Debug, Clone)]
pub struct Response {
    pub id: ResponseId,
    pub name: String,
    pub action: ResponseAction,
    pub intensity: f32,
    pub priority: BehaviorPriority,
}

impl Response {
    pub fn new(id: ResponseId, name: impl Into<String>, action: ResponseAction) -> Self {
        Self {
            id,
            name: name.into(),
            action,
            intensity: 1.0,
            priority: BehaviorPriority::Normal,
        }
    }

    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn with_priority(mut self, priority: BehaviorPriority) -> Self {
        self.priority = priority;
        self
    }
}

/// Type of response action
#[derive(Debug, Clone)]
pub enum ResponseAction {
    /// No action
    NoOp,
    /// Emit a command
    Command(String),
    /// Set a variable
    SetVariable(String, StimulusValue),
    /// Trigger another behavior
    TriggerBehavior(BehaviorId),
    /// Inhibit a behavior
    InhibitBehavior(BehaviorId),
    /// Composite actions
    Sequence(Vec<ResponseAction>),
    /// Custom action ID
    Custom(u64),
}

// ============================================================================
// Reactive Behavior
// ============================================================================

/// Condition for behavior activation
pub enum ActivationCondition {
    /// Always active
    Always,
    /// Active when stimulus is present
    StimulusPresent(StimulusId),
    /// Active when stimulus exceeds threshold
    StimulusAbove(StimulusId, f32),
    /// Active when stimulus is below threshold
    StimulusBelow(StimulusId, f32),
    /// Multiple conditions must be true
    All(Vec<ActivationCondition>),
    /// Any condition must be true
    Any(Vec<ActivationCondition>),
    /// Negation
    Not(Box<ActivationCondition>),
    /// Custom condition
    Custom(Box<dyn Fn(&[Stimulus]) -> bool + Send + Sync>),
}

impl ActivationCondition {
    pub fn evaluate(&self, stimuli: &[Stimulus]) -> bool {
        match self {
            Self::Always => true,
            Self::StimulusPresent(id) => stimuli.iter().any(|s| s.id == *id),
            Self::StimulusAbove(id, threshold) => stimuli
                .iter()
                .find(|s| s.id == *id)
                .map(|s| s.intensity > *threshold)
                .unwrap_or(false),
            Self::StimulusBelow(id, threshold) => stimuli
                .iter()
                .find(|s| s.id == *id)
                .map(|s| s.intensity < *threshold)
                .unwrap_or(false),
            Self::All(conditions) => conditions.iter().all(|c| c.evaluate(stimuli)),
            Self::Any(conditions) => conditions.iter().any(|c| c.evaluate(stimuli)),
            Self::Not(condition) => !condition.evaluate(stimuli),
            Self::Custom(f) => f(stimuli),
        }
    }
}

/// A reactive behavior mapping stimuli to responses
pub struct ReactiveBehavior {
    pub id: BehaviorId,
    pub name: String,
    pub priority: BehaviorPriority,
    pub condition: ActivationCondition,
    pub response_generator: Box<dyn Fn(&[Stimulus]) -> Vec<Response> + Send + Sync>,
    pub is_enabled: bool,
    pub inhibited_by: Vec<BehaviorId>,
    pub last_activation: u64,
    pub activation_count: u64,
}

impl ReactiveBehavior {
    pub fn new<F>(
        id: BehaviorId,
        name: impl Into<String>,
        priority: BehaviorPriority,
        condition: ActivationCondition,
        response_generator: F,
    ) -> Self
    where
        F: Fn(&[Stimulus]) -> Vec<Response> + Send + Sync + 'static,
    {
        Self {
            id,
            name: name.into(),
            priority,
            condition,
            response_generator: Box::new(response_generator),
            is_enabled: true,
            inhibited_by: Vec::new(),
            last_activation: 0,
            activation_count: 0,
        }
    }

    pub fn with_inhibition(mut self, inhibitor: BehaviorId) -> Self {
        self.inhibited_by.push(inhibitor);
        self
    }

    pub fn enable(&mut self) {
        self.is_enabled = true;
    }

    pub fn disable(&mut self) {
        self.is_enabled = false;
    }

    pub fn can_activate(&self, stimuli: &[Stimulus], active_behaviors: &[BehaviorId]) -> bool {
        if !self.is_enabled {
            return false;
        }

        // Check if inhibited
        for inhibitor in &self.inhibited_by {
            if active_behaviors.contains(inhibitor) {
                return false;
            }
        }

        self.condition.evaluate(stimuli)
    }

    pub fn generate_responses(&self, stimuli: &[Stimulus]) -> Vec<Response> {
        (self.response_generator)(stimuli)
    }
}

// ============================================================================
// Reactive Layer (Subsumption Architecture)
// ============================================================================

/// A layer in subsumption architecture
pub struct ReactiveLayer {
    pub id: u32,
    pub name: String,
    pub priority: BehaviorPriority,
    pub behaviors: Vec<ReactiveBehavior>,
    pub is_enabled: bool,
    pub suppresses: Vec<u32>, // Layer IDs this layer suppresses
}

impl ReactiveLayer {
    pub fn new(id: u32, name: impl Into<String>, priority: BehaviorPriority) -> Self {
        Self {
            id,
            name: name.into(),
            priority,
            behaviors: Vec::new(),
            is_enabled: true,
            suppresses: Vec::new(),
        }
    }

    pub fn add_behavior(&mut self, behavior: ReactiveBehavior) {
        self.behaviors.push(behavior);
    }

    pub fn with_behavior(mut self, behavior: ReactiveBehavior) -> Self {
        self.add_behavior(behavior);
        self
    }

    pub fn suppresses_layer(mut self, layer_id: u32) -> Self {
        self.suppresses.push(layer_id);
        self
    }

    pub fn enable(&mut self) {
        self.is_enabled = true;
    }

    pub fn disable(&mut self) {
        self.is_enabled = false;
    }
}

/// Subsumption-style reactive architecture
pub struct SubsumptionArchitecture {
    layers: Vec<ReactiveLayer>,
    suppressed_layers: Vec<u32>,
    active_behaviors: Vec<BehaviorId>,
    current_responses: Vec<Response>,
}

impl SubsumptionArchitecture {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
            suppressed_layers: Vec::new(),
            active_behaviors: Vec::new(),
            current_responses: Vec::new(),
        }
    }

    pub fn add_layer(&mut self, layer: ReactiveLayer) {
        // Keep layers sorted by priority (highest first)
        self.layers.push(layer);
        self.layers.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn process(&mut self, stimuli: &[Stimulus], timestamp: u64) -> &[Response] {
        self.active_behaviors.clear();
        self.current_responses.clear();
        self.suppressed_layers.clear();

        // Process layers from highest to lowest priority
        for layer in &mut self.layers {
            if !layer.is_enabled {
                continue;
            }

            if self.suppressed_layers.contains(&layer.id) {
                continue;
            }

            let mut layer_activated = false;

            for behavior in &mut layer.behaviors {
                if behavior.can_activate(stimuli, &self.active_behaviors) {
                    let responses = behavior.generate_responses(stimuli);

                    if !responses.is_empty() {
                        behavior.last_activation = timestamp;
                        behavior.activation_count += 1;
                        self.active_behaviors.push(behavior.id);
                        self.current_responses.extend(responses);
                        layer_activated = true;
                    }
                }
            }

            // If this layer activated, suppress lower layers
            if layer_activated {
                self.suppressed_layers.extend_from_slice(&layer.suppresses);
            }
        }

        &self.current_responses
    }

    pub fn active_behaviors(&self) -> &[BehaviorId] {
        &self.active_behaviors
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn get_layer(&self, id: u32) -> Option<&ReactiveLayer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn get_layer_mut(&mut self, id: u32) -> Option<&mut ReactiveLayer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }
}

impl Default for SubsumptionArchitecture {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Stimulus Buffer
// ============================================================================

/// Buffer for managing incoming stimuli
pub struct StimulusBuffer {
    stimuli: Vec<Stimulus>,
    max_age: u64,
    max_size: usize,
}

impl StimulusBuffer {
    pub fn new(max_age: u64, max_size: usize) -> Self {
        Self {
            stimuli: Vec::new(),
            max_age,
            max_size,
        }
    }

    pub fn add(&mut self, stimulus: Stimulus) {
        self.stimuli.push(stimulus);

        // Enforce size limit
        while self.stimuli.len() > self.max_size {
            self.stimuli.remove(0);
        }
    }

    pub fn update(&mut self, current_time: u64) {
        // Remove old stimuli
        self.stimuli
            .retain(|s| current_time.saturating_sub(s.timestamp) < self.max_age);
    }

    pub fn get_stimuli(&self) -> &[Stimulus] {
        &self.stimuli
    }

    pub fn get_by_id(&self, id: StimulusId) -> Option<&Stimulus> {
        self.stimuli.iter().find(|s| s.id == id)
    }

    pub fn get_by_name(&self, name: &str) -> Vec<&Stimulus> {
        self.stimuli.iter().filter(|s| s.name == name).collect()
    }

    pub fn clear(&mut self) {
        self.stimuli.clear();
    }

    pub fn len(&self) -> usize {
        self.stimuli.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stimuli.is_empty()
    }
}

// ============================================================================
// Response Executor
// ============================================================================

/// Executor for response actions
pub struct ResponseExecutor {
    pending_commands: Vec<String>,
    variables: BTreeMap<String, StimulusValue>,
    triggered_behaviors: Vec<BehaviorId>,
    inhibited_behaviors: Vec<BehaviorId>,
    custom_handlers: BTreeMap<u64, Box<dyn Fn() + Send + Sync>>,
}

impl ResponseExecutor {
    pub fn new() -> Self {
        Self {
            pending_commands: Vec::new(),
            variables: BTreeMap::new(),
            triggered_behaviors: Vec::new(),
            inhibited_behaviors: Vec::new(),
            custom_handlers: BTreeMap::new(),
        }
    }

    pub fn register_custom_handler<F>(&mut self, id: u64, handler: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.custom_handlers.insert(id, Box::new(handler));
    }

    pub fn execute(&mut self, response: &Response) {
        self.execute_action(&response.action);
    }

    pub fn execute_all(&mut self, responses: &[Response]) {
        for response in responses {
            self.execute(response);
        }
    }

    fn execute_action(&mut self, action: &ResponseAction) {
        match action {
            ResponseAction::NoOp => {},
            ResponseAction::Command(cmd) => {
                self.pending_commands.push(cmd.clone());
            },
            ResponseAction::SetVariable(name, value) => {
                self.variables.insert(name.clone(), value.clone());
            },
            ResponseAction::TriggerBehavior(id) => {
                if !self.triggered_behaviors.contains(id) {
                    self.triggered_behaviors.push(*id);
                }
            },
            ResponseAction::InhibitBehavior(id) => {
                if !self.inhibited_behaviors.contains(id) {
                    self.inhibited_behaviors.push(*id);
                }
            },
            ResponseAction::Sequence(actions) => {
                for action in actions {
                    self.execute_action(action);
                }
            },
            ResponseAction::Custom(id) => {
                if let Some(handler) = self.custom_handlers.get(id) {
                    handler();
                }
            },
        }
    }

    pub fn take_commands(&mut self) -> Vec<String> {
        core::mem::take(&mut self.pending_commands)
    }

    pub fn get_variable(&self, name: &str) -> Option<&StimulusValue> {
        self.variables.get(name)
    }

    pub fn take_triggered_behaviors(&mut self) -> Vec<BehaviorId> {
        core::mem::take(&mut self.triggered_behaviors)
    }

    pub fn take_inhibited_behaviors(&mut self) -> Vec<BehaviorId> {
        core::mem::take(&mut self.inhibited_behaviors)
    }

    pub fn clear(&mut self) {
        self.pending_commands.clear();
        self.triggered_behaviors.clear();
        self.inhibited_behaviors.clear();
    }
}

impl Default for ResponseExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Kernel Reactive Behaviors
// ============================================================================

/// Predefined kernel stimuli
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelStimulus {
    MemoryPressure,
    CpuOverload,
    IoBlocked,
    Deadlock,
    StackOverflow,
    InterruptStorm,
    PageFault,
    Watchdog,
}

impl KernelStimulus {
    pub fn as_stimulus_id(&self) -> StimulusId {
        StimulusId(*self as u64 + 1)
    }
}

/// Create a kernel emergency response behavior
pub fn create_emergency_behavior() -> ReactiveBehavior {
    ReactiveBehavior::new(
        BehaviorId::new(1),
        "EmergencyResponse",
        BehaviorPriority::Emergency,
        ActivationCondition::Any(vec![
            ActivationCondition::StimulusPresent(KernelStimulus::Deadlock.as_stimulus_id()),
            ActivationCondition::StimulusPresent(KernelStimulus::StackOverflow.as_stimulus_id()),
        ]),
        |stimuli| {
            let mut responses = Vec::new();

            for stimulus in stimuli {
                if stimulus.id == KernelStimulus::Deadlock.as_stimulus_id() {
                    responses.push(
                        Response::new(
                            ResponseId::new(1),
                            "BreakDeadlock",
                            ResponseAction::Command("kernel.break_deadlock".into()),
                        )
                        .with_priority(BehaviorPriority::Emergency),
                    );
                }

                if stimulus.id == KernelStimulus::StackOverflow.as_stimulus_id() {
                    responses.push(
                        Response::new(
                            ResponseId::new(2),
                            "HandleStackOverflow",
                            ResponseAction::Command("kernel.handle_stack_overflow".into()),
                        )
                        .with_priority(BehaviorPriority::Emergency),
                    );
                }
            }

            responses
        },
    )
}

/// Create a kernel memory management behavior
pub fn create_memory_behavior() -> ReactiveBehavior {
    ReactiveBehavior::new(
        BehaviorId::new(2),
        "MemoryManagement",
        BehaviorPriority::High,
        ActivationCondition::StimulusAbove(KernelStimulus::MemoryPressure.as_stimulus_id(), 0.7),
        |stimuli| {
            let mut responses = Vec::new();

            for stimulus in stimuli {
                if stimulus.id == KernelStimulus::MemoryPressure.as_stimulus_id() {
                    if stimulus.intensity > 0.9 {
                        responses.push(
                            Response::new(
                                ResponseId::new(10),
                                "AggressiveReclaim",
                                ResponseAction::Sequence(vec![
                                    ResponseAction::Command("kernel.reclaim_memory".into()),
                                    ResponseAction::Command("kernel.flush_caches".into()),
                                    ResponseAction::Command("kernel.oom_kill_candidates".into()),
                                ]),
                            )
                            .with_priority(BehaviorPriority::Critical),
                        );
                    } else {
                        responses.push(
                            Response::new(
                                ResponseId::new(11),
                                "ModerateReclaim",
                                ResponseAction::Command("kernel.reclaim_memory".into()),
                            )
                            .with_priority(BehaviorPriority::High),
                        );
                    }
                }
            }

            responses
        },
    )
}

/// Create a kernel CPU management behavior
pub fn create_cpu_behavior() -> ReactiveBehavior {
    ReactiveBehavior::new(
        BehaviorId::new(3),
        "CpuManagement",
        BehaviorPriority::Normal,
        ActivationCondition::StimulusAbove(KernelStimulus::CpuOverload.as_stimulus_id(), 0.8),
        |stimuli| {
            let mut responses = Vec::new();

            for stimulus in stimuli {
                if stimulus.id == KernelStimulus::CpuOverload.as_stimulus_id() {
                    responses.push(
                        Response::new(
                            ResponseId::new(20),
                            "ThrottleCpu",
                            ResponseAction::Sequence(vec![
                                ResponseAction::Command("kernel.throttle_background".into()),
                                ResponseAction::Command("kernel.migrate_tasks".into()),
                            ]),
                        )
                        .with_priority(BehaviorPriority::Normal),
                    );
                }
            }

            responses
        },
    )
}

/// Create a complete kernel reactive system
pub fn create_kernel_reactive_system() -> SubsumptionArchitecture {
    let mut arch = SubsumptionArchitecture::new();

    // Emergency layer (highest priority)
    let emergency_layer = ReactiveLayer::new(0, "Emergency", BehaviorPriority::Emergency)
        .with_behavior(create_emergency_behavior())
        .suppresses_layer(1)
        .suppresses_layer(2);

    // Resource management layer
    let resource_layer = ReactiveLayer::new(1, "ResourceManagement", BehaviorPriority::High)
        .with_behavior(create_memory_behavior())
        .with_behavior(create_cpu_behavior())
        .suppresses_layer(2);

    // Background optimization layer
    let background_layer = ReactiveLayer::new(2, "Background", BehaviorPriority::Background);

    arch.add_layer(emergency_layer);
    arch.add_layer(resource_layer);
    arch.add_layer(background_layer);

    arch
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stimulus_creation() {
        let stimulus = Stimulus::new(StimulusId::new(1), "test")
            .with_intensity(0.5)
            .with_timestamp(1000);

        assert_eq!(stimulus.id, StimulusId::new(1));
        assert_eq!(stimulus.name, "test");
        assert_eq!(stimulus.intensity, 0.5);
    }

    #[test]
    fn test_response_creation() {
        let response = Response::new(ResponseId::new(1), "test", ResponseAction::NoOp);

        assert_eq!(response.id, ResponseId::new(1));
        assert_eq!(response.priority, BehaviorPriority::Normal);
    }

    #[test]
    fn test_activation_condition_always() {
        let condition = ActivationCondition::Always;
        assert!(condition.evaluate(&[]));
    }

    #[test]
    fn test_stimulus_buffer() {
        let mut buffer = StimulusBuffer::new(1000, 10);
        buffer.add(Stimulus::new(StimulusId::new(1), "test").with_timestamp(100));

        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }

    #[test]
    fn test_response_executor() {
        let mut executor = ResponseExecutor::new();
        executor.execute(&Response::new(
            ResponseId::new(1),
            "test",
            ResponseAction::Command("test_cmd".into()),
        ));

        let commands = executor.take_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0], "test_cmd");
    }
}
