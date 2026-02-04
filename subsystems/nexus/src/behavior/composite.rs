//! NEXUS Year 2: Composite Behavior Patterns
//!
//! Combines multiple behavior systems (behavior trees, state machines,
//! utility AI, reactive behaviors) into unified architectures.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::reactive::{Response, Stimulus, StimulusBuffer, SubsumptionArchitecture};
use super::state_machine::{StateContext, StateEvent, StateId, StateMachine};
use super::tree::{BehaviorStatus, BehaviorTree, Blackboard};
use super::utility::{ActionId as UtilityActionId, UtilityContext, UtilitySelector};

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for composite behaviors
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CompositeId(pub u64);

impl CompositeId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Result of composite behavior execution
#[derive(Debug, Clone)]
pub struct CompositeResult {
    pub status: CompositeStatus,
    pub active_layers: Vec<String>,
    pub responses: Vec<CompositeResponse>,
    pub state_changes: Vec<StateChange>,
}

impl CompositeResult {
    pub fn new(status: CompositeStatus) -> Self {
        Self {
            status,
            active_layers: Vec::new(),
            responses: Vec::new(),
            state_changes: Vec::new(),
        }
    }

    pub fn with_layer(mut self, layer: impl Into<String>) -> Self {
        self.active_layers.push(layer.into());
        self
    }

    pub fn with_response(mut self, response: CompositeResponse) -> Self {
        self.responses.push(response);
        self
    }
}

/// Status of composite execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeStatus {
    /// Successfully completed
    Success,
    /// Still running
    Running,
    /// Failed
    Failed,
    /// No action taken
    Idle,
}

/// Response from composite behavior
#[derive(Debug, Clone)]
pub struct CompositeResponse {
    pub source: String,
    pub action: String,
    pub priority: u8,
}

/// State change record
#[derive(Debug, Clone)]
pub struct StateChange {
    pub system: String,
    pub from: String,
    pub to: String,
    pub timestamp: u64,
}

// ============================================================================
// Behavior Layer
// ============================================================================

/// Type of behavior layer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorLayerType {
    /// Behavior tree layer
    BehaviorTree,
    /// State machine layer
    StateMachine,
    /// Reactive layer (subsumption)
    Reactive,
    /// Utility AI layer
    UtilityAI,
}

/// A layer in the composite behavior system
pub struct BehaviorLayer {
    pub id: u32,
    pub name: String,
    pub layer_type: BehaviorLayerType,
    pub priority: i32,
    pub is_enabled: bool,
    pub subsumes: Vec<u32>, // Layer IDs this layer can override

    // Layer implementations (only one is active based on type)
    behavior_tree: Option<BehaviorTree>,
    state_machine: Option<StateMachine>,
    reactive: Option<SubsumptionArchitecture>,
    utility: Option<UtilitySelector>,
}

impl BehaviorLayer {
    pub fn new_behavior_tree(id: u32, name: impl Into<String>, tree: BehaviorTree) -> Self {
        Self {
            id,
            name: name.into(),
            layer_type: BehaviorLayerType::BehaviorTree,
            priority: 0,
            is_enabled: true,
            subsumes: Vec::new(),
            behavior_tree: Some(tree),
            state_machine: None,
            reactive: None,
            utility: None,
        }
    }

    pub fn new_state_machine(id: u32, name: impl Into<String>, sm: StateMachine) -> Self {
        Self {
            id,
            name: name.into(),
            layer_type: BehaviorLayerType::StateMachine,
            priority: 0,
            is_enabled: true,
            subsumes: Vec::new(),
            behavior_tree: None,
            state_machine: Some(sm),
            reactive: None,
            utility: None,
        }
    }

    pub fn new_reactive(id: u32, name: impl Into<String>, arch: SubsumptionArchitecture) -> Self {
        Self {
            id,
            name: name.into(),
            layer_type: BehaviorLayerType::Reactive,
            priority: 0,
            is_enabled: true,
            subsumes: Vec::new(),
            behavior_tree: None,
            state_machine: None,
            reactive: Some(arch),
            utility: None,
        }
    }

    pub fn new_utility(id: u32, name: impl Into<String>, selector: UtilitySelector) -> Self {
        Self {
            id,
            name: name.into(),
            layer_type: BehaviorLayerType::UtilityAI,
            priority: 0,
            is_enabled: true,
            subsumes: Vec::new(),
            behavior_tree: None,
            state_machine: None,
            reactive: None,
            utility: Some(selector),
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_subsumption(mut self, layer_id: u32) -> Self {
        self.subsumes.push(layer_id);
        self
    }

    pub fn enable(&mut self) {
        self.is_enabled = true;
    }

    pub fn disable(&mut self) {
        self.is_enabled = false;
    }

    pub fn behavior_tree(&self) -> Option<&BehaviorTree> {
        self.behavior_tree.as_ref()
    }

    pub fn behavior_tree_mut(&mut self) -> Option<&mut BehaviorTree> {
        self.behavior_tree.as_mut()
    }

    pub fn state_machine(&self) -> Option<&StateMachine> {
        self.state_machine.as_ref()
    }

    pub fn state_machine_mut(&mut self) -> Option<&mut StateMachine> {
        self.state_machine.as_mut()
    }

    pub fn reactive(&self) -> Option<&SubsumptionArchitecture> {
        self.reactive.as_ref()
    }

    pub fn reactive_mut(&mut self) -> Option<&mut SubsumptionArchitecture> {
        self.reactive.as_mut()
    }

    pub fn utility(&self) -> Option<&UtilitySelector> {
        self.utility.as_ref()
    }

    pub fn utility_mut(&mut self) -> Option<&mut UtilitySelector> {
        self.utility.as_mut()
    }
}

// ============================================================================
// Behavior Blend
// ============================================================================

/// Blending strategy for combining multiple behavior outputs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendStrategy {
    /// Winner takes all (highest priority)
    Priority,
    /// All outputs are combined
    Combine,
    /// Weighted average based on priority
    Weighted,
    /// First successful output wins
    FirstSuccess,
}

/// Blends outputs from multiple behavior systems
pub struct BehaviorBlend {
    name: String,
    strategy: BlendStrategy,
    outputs: Vec<BlendOutput>,
}

/// An output from a behavior system to be blended
#[derive(Debug, Clone)]
pub struct BlendOutput {
    pub source: String,
    pub weight: f32,
    pub action: BlendAction,
}

/// Action type for blending
#[derive(Debug, Clone)]
pub enum BlendAction {
    None,
    Command(String),
    Value(f64),
    Vector(Vec<f64>),
}

impl BehaviorBlend {
    pub fn new(name: impl Into<String>, strategy: BlendStrategy) -> Self {
        Self {
            name: name.into(),
            strategy,
            outputs: Vec::new(),
        }
    }

    pub fn add_output(&mut self, output: BlendOutput) {
        self.outputs.push(output);
    }

    pub fn clear_outputs(&mut self) {
        self.outputs.clear();
    }

    pub fn blend(&self) -> Option<BlendAction> {
        if self.outputs.is_empty() {
            return None;
        }

        match self.strategy {
            BlendStrategy::Priority => {
                // Return output with highest weight
                self.outputs
                    .iter()
                    .max_by(|a, b| {
                        a.weight
                            .partial_cmp(&b.weight)
                            .unwrap_or(core::cmp::Ordering::Equal)
                    })
                    .map(|o| o.action.clone())
            },
            BlendStrategy::FirstSuccess => {
                // Return first non-None output
                self.outputs
                    .iter()
                    .find(|o| !matches!(o.action, BlendAction::None))
                    .map(|o| o.action.clone())
            },
            BlendStrategy::Weighted => {
                // Weighted average for vector/value outputs
                let total_weight: f32 = self.outputs.iter().map(|o| o.weight).sum();
                if total_weight <= 0.0 {
                    return None;
                }

                // Check if all outputs are values
                let all_values: Vec<(f32, f64)> = self
                    .outputs
                    .iter()
                    .filter_map(|o| {
                        if let BlendAction::Value(v) = o.action {
                            Some((o.weight, v))
                        } else {
                            None
                        }
                    })
                    .collect();

                if all_values.len() == self.outputs.len() {
                    let weighted_sum: f64 = all_values.iter().map(|(w, v)| *w as f64 * v).sum();
                    return Some(BlendAction::Value(weighted_sum / total_weight as f64));
                }

                // Fallback to priority
                self.outputs
                    .iter()
                    .max_by(|a, b| {
                        a.weight
                            .partial_cmp(&b.weight)
                            .unwrap_or(core::cmp::Ordering::Equal)
                    })
                    .map(|o| o.action.clone())
            },
            BlendStrategy::Combine => {
                // Combine all commands into a vector
                let commands: Vec<String> = self
                    .outputs
                    .iter()
                    .filter_map(|o| {
                        if let BlendAction::Command(cmd) = &o.action {
                            Some(cmd.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                if commands.is_empty() {
                    None
                } else if commands.len() == 1 {
                    Some(BlendAction::Command(commands.into_iter().next().unwrap()))
                } else {
                    // Join commands
                    let combined = commands.join(";");
                    Some(BlendAction::Command(combined))
                }
            },
        }
    }
}

// ============================================================================
// Behavior Composite
// ============================================================================

/// Composite behavior system combining multiple behavior paradigms
pub struct BehaviorComposite {
    id: CompositeId,
    name: String,
    layers: Vec<BehaviorLayer>,
    suppressed_layers: Vec<u32>,

    // Shared context
    blackboard: Blackboard,
    stimulus_buffer: StimulusBuffer,
    current_time: u64,
    delta_time: u64,
}

impl BehaviorComposite {
    pub fn new(id: CompositeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            layers: Vec::new(),
            suppressed_layers: Vec::new(),
            blackboard: Blackboard::new(),
            stimulus_buffer: StimulusBuffer::new(10000, 100),
            current_time: 0,
            delta_time: 0,
        }
    }

    pub fn add_layer(&mut self, layer: BehaviorLayer) {
        self.layers.push(layer);
        // Sort by priority (highest first)
        self.layers.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn get_layer(&self, id: u32) -> Option<&BehaviorLayer> {
        self.layers.iter().find(|l| l.id == id)
    }

    pub fn get_layer_mut(&mut self, id: u32) -> Option<&mut BehaviorLayer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    pub fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }

    pub fn blackboard_mut(&mut self) -> &mut Blackboard {
        &mut self.blackboard
    }

    pub fn add_stimulus(&mut self, stimulus: Stimulus) {
        self.stimulus_buffer.add(stimulus);
    }

    /// Update the composite system
    pub fn update(&mut self, time: u64, delta_time: u64) -> CompositeResult {
        self.current_time = time;
        self.delta_time = delta_time;
        self.suppressed_layers.clear();

        // Update stimulus buffer
        self.stimulus_buffer.update(time);

        let mut result = CompositeResult::new(CompositeStatus::Idle);

        // Process layers in priority order
        for layer in &mut self.layers {
            if !layer.is_enabled {
                continue;
            }

            if self.suppressed_layers.contains(&layer.id) {
                continue;
            }

            let layer_result = self.process_layer(layer);

            if layer_result.status != CompositeStatus::Idle {
                result.active_layers.push(layer.name.clone());
                result.responses.extend(layer_result.responses);
                result.state_changes.extend(layer_result.state_changes);

                // Apply subsumption
                for subsumed_id in &layer.subsumes {
                    if !self.suppressed_layers.contains(subsumed_id) {
                        self.suppressed_layers.push(*subsumed_id);
                    }
                }

                // Update result status
                if result.status == CompositeStatus::Idle {
                    result.status = layer_result.status;
                }
            }
        }

        result
    }

    fn process_layer(&mut self, layer: &mut BehaviorLayer) -> CompositeResult {
        match layer.layer_type {
            BehaviorLayerType::BehaviorTree => self.process_behavior_tree_layer(layer),
            BehaviorLayerType::StateMachine => self.process_state_machine_layer(layer),
            BehaviorLayerType::Reactive => self.process_reactive_layer(layer),
            BehaviorLayerType::UtilityAI => self.process_utility_layer(layer),
        }
    }

    fn process_behavior_tree_layer(&mut self, layer: &mut BehaviorLayer) -> CompositeResult {
        if let Some(tree) = layer.behavior_tree_mut() {
            let status = tree.tick(self.current_time, self.delta_time);

            let composite_status = match status {
                BehaviorStatus::Success => CompositeStatus::Success,
                BehaviorStatus::Running => CompositeStatus::Running,
                BehaviorStatus::Failure | BehaviorStatus::Cancelled => CompositeStatus::Failed,
                BehaviorStatus::Ready => CompositeStatus::Idle,
            };

            CompositeResult::new(composite_status).with_layer(&layer.name)
        } else {
            CompositeResult::new(CompositeStatus::Idle)
        }
    }

    fn process_state_machine_layer(&mut self, layer: &mut BehaviorLayer) -> CompositeResult {
        let layer_name = layer.name.clone();
        if let Some(sm) = layer.state_machine_mut() {
            let old_state = sm.current_state();

            let mut ctx = StateContext::new(self.current_time, self.delta_time);
            sm.update(&mut ctx);

            let new_state = sm.current_state();

            let mut result = CompositeResult::new(CompositeStatus::Running).with_layer(&layer_name);

            if old_state != new_state {
                result.state_changes.push(StateChange {
                    system: layer_name.clone(),
                    from: alloc::format!("{:?}", old_state),
                    to: alloc::format!("{:?}", new_state),
                    timestamp: self.current_time,
                });
            }

            if sm.is_finished() {
                result.status = CompositeStatus::Success;
            }

            result
        } else {
            CompositeResult::new(CompositeStatus::Idle)
        }
    }

    fn process_reactive_layer(&mut self, layer: &mut BehaviorLayer) -> CompositeResult {
        let layer_name = layer.name.clone();
        let layer_priority = layer.priority;
        if let Some(reactive) = layer.reactive_mut() {
            let stimuli = self.stimulus_buffer.get_stimuli();
            let responses = reactive.process(stimuli, self.current_time);

            let mut result = CompositeResult::new(if responses.is_empty() {
                CompositeStatus::Idle
            } else {
                CompositeStatus::Success
            })
            .with_layer(&layer_name);

            for response in responses {
                result.responses.push(CompositeResponse {
                    source: layer_name.clone(),
                    action: response.name.clone(),
                    priority: response.priority as u8,
                });
            }

            result
        } else {
            CompositeResult::new(CompositeStatus::Idle)
        }
    }

    fn process_utility_layer(&mut self, layer: &mut BehaviorLayer) -> CompositeResult {
        let layer_name = layer.name.clone();
        let layer_priority = layer.priority;
        if let Some(utility) = layer.utility_mut() {
            let ctx = self.create_utility_context();

            if let Some(action_id) = utility.select_and_execute(&ctx) {
                let action_name = utility
                    .get_action(action_id)
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| alloc::format!("Action_{:?}", action_id));

                CompositeResult::new(CompositeStatus::Success)
                    .with_layer(&layer_name)
                    .with_response(CompositeResponse {
                        source: layer_name.clone(),
                        action: action_name,
                        priority: layer_priority as u8,
                    })
            } else {
                CompositeResult::new(CompositeStatus::Idle)
            }
        } else {
            CompositeResult::new(CompositeStatus::Idle)
        }
    }

    fn create_utility_context(&self) -> UtilityContext {
        let mut ctx = UtilityContext::new().with_time(self.current_time);

        // Transfer relevant blackboard values to utility context
        // This would be customized based on your specific needs

        ctx
    }

    /// Process an event through all state machines
    pub fn process_event(&mut self, event: &StateEvent) {
        for layer in &mut self.layers {
            if let Some(sm) = layer.state_machine_mut() {
                let mut ctx = StateContext::new(self.current_time, self.delta_time);
                sm.process_event(event, &mut ctx);
            }
        }
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> CompositeId {
        self.id
    }
}

// ============================================================================
// Composite Executor
// ============================================================================

/// Executor for managing multiple composite behaviors
pub struct CompositeExecutor {
    composites: BTreeMap<CompositeId, BehaviorComposite>,
    active_composite: Option<CompositeId>,
    execution_history: Vec<ExecutionRecord>,
    max_history: usize,
}

/// Record of composite execution
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    pub timestamp: u64,
    pub composite_id: CompositeId,
    pub status: CompositeStatus,
    pub active_layers: usize,
}

impl CompositeExecutor {
    pub fn new() -> Self {
        Self {
            composites: BTreeMap::new(),
            active_composite: None,
            execution_history: Vec::new(),
            max_history: 100,
        }
    }

    pub fn register(&mut self, composite: BehaviorComposite) {
        let id = composite.id();
        if self.active_composite.is_none() {
            self.active_composite = Some(id);
        }
        self.composites.insert(id, composite);
    }

    pub fn set_active(&mut self, id: CompositeId) -> bool {
        if self.composites.contains_key(&id) {
            self.active_composite = Some(id);
            true
        } else {
            false
        }
    }

    pub fn get(&self, id: CompositeId) -> Option<&BehaviorComposite> {
        self.composites.get(&id)
    }

    pub fn get_mut(&mut self, id: CompositeId) -> Option<&mut BehaviorComposite> {
        self.composites.get_mut(&id)
    }

    /// Update the active composite
    pub fn update(&mut self, time: u64, delta_time: u64) -> Option<CompositeResult> {
        let id = self.active_composite?;
        let composite = self.composites.get_mut(&id)?;

        let result = composite.update(time, delta_time);

        // Record execution
        self.execution_history.push(ExecutionRecord {
            timestamp: time,
            composite_id: id,
            status: result.status,
            active_layers: result.active_layers.len(),
        });

        if self.execution_history.len() > self.max_history {
            self.execution_history.remove(0);
        }

        Some(result)
    }

    /// Update all composites
    pub fn update_all(
        &mut self,
        time: u64,
        delta_time: u64,
    ) -> Vec<(CompositeId, CompositeResult)> {
        let ids: Vec<CompositeId> = self.composites.keys().copied().collect();
        let mut results = Vec::new();

        for id in ids {
            if let Some(composite) = self.composites.get_mut(&id) {
                let result = composite.update(time, delta_time);
                results.push((id, result));
            }
        }

        results
    }

    pub fn composite_count(&self) -> usize {
        self.composites.len()
    }

    pub fn execution_history(&self) -> &[ExecutionRecord] {
        &self.execution_history
    }
}

impl Default for CompositeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Kernel Composite Behavior
// ============================================================================

/// Create a kernel composite behavior system
pub fn create_kernel_composite() -> BehaviorComposite {
    use super::reactive::create_kernel_reactive_system;
    use super::state_machine::create_kernel_load_fsm;
    use super::utility::create_kernel_memory_selector;

    let mut composite = BehaviorComposite::new(CompositeId::new(1), "KernelBehavior");

    // Layer 1: Reactive emergency responses (highest priority)
    let reactive_layer =
        BehaviorLayer::new_reactive(1, "EmergencyReactive", create_kernel_reactive_system())
            .with_priority(100)
            .with_subsumption(2)
            .with_subsumption(3);

    // Layer 2: State machine for load management
    let mut load_sm = create_kernel_load_fsm();
    let mut ctx = StateContext::new(0, 0);
    load_sm.initialize(&mut ctx);

    let sm_layer = BehaviorLayer::new_state_machine(2, "LoadManagement", load_sm).with_priority(50);

    // Layer 3: Utility AI for memory optimization
    let utility_layer =
        BehaviorLayer::new_utility(3, "MemoryOptimization", create_kernel_memory_selector())
            .with_priority(10);

    composite.add_layer(reactive_layer);
    composite.add_layer(sm_layer);
    composite.add_layer(utility_layer);

    composite
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composite_id() {
        let id = CompositeId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_composite_result() {
        let result = CompositeResult::new(CompositeStatus::Success).with_layer("test");

        assert_eq!(result.status, CompositeStatus::Success);
        assert_eq!(result.active_layers.len(), 1);
    }

    #[test]
    fn test_blend_strategy() {
        let blend = BehaviorBlend::new("test", BlendStrategy::Priority);
        assert!(blend.blend().is_none());
    }

    #[test]
    fn test_composite_executor() {
        let executor = CompositeExecutor::new();
        assert_eq!(executor.composite_count(), 0);
    }
}
