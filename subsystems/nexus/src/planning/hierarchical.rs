//! # Hierarchical Task Network (HTN) Planning for NEXUS
//!
//! Hierarchical planning with task decomposition.

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::actions::{Action, ActionId, ActionSpace, WorldState};

// ============================================================================
// TASK TYPES
// ============================================================================

/// Task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u32);

/// Method identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MethodId(pub u32);

/// Task type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    /// Primitive task (directly executable)
    Primitive,
    /// Compound task (must be decomposed)
    Compound,
}

/// A task in the HTN
#[derive(Debug, Clone)]
pub struct Task {
    /// Task ID
    pub id: TaskId,
    /// Task name
    pub name: String,
    /// Task type
    pub task_type: TaskType,
    /// Associated action (for primitive tasks)
    pub action: Option<ActionId>,
    /// Available methods (for compound tasks)
    pub methods: Vec<MethodId>,
}

impl Task {
    /// Create primitive task
    pub fn primitive(id: TaskId, name: String, action: ActionId) -> Self {
        Self {
            id,
            name,
            task_type: TaskType::Primitive,
            action: Some(action),
            methods: Vec::new(),
        }
    }

    /// Create compound task
    pub fn compound(id: TaskId, name: String) -> Self {
        Self {
            id,
            name,
            task_type: TaskType::Compound,
            action: None,
            methods: Vec::new(),
        }
    }

    /// Add method
    pub fn with_method(mut self, method: MethodId) -> Self {
        self.methods.push(method);
        self
    }

    /// Is primitive?
    pub fn is_primitive(&self) -> bool {
        self.task_type == TaskType::Primitive
    }
}

/// A method for decomposing compound tasks
pub struct Method {
    /// Method ID
    pub id: MethodId,
    /// Method name
    pub name: String,
    /// Task this method applies to
    pub task: TaskId,
    /// Precondition (state condition for this method)
    pub precondition: Option<Box<dyn Fn(&WorldState) -> bool + Send + Sync>>,
    /// Subtasks (ordered)
    pub subtasks: Vec<TaskId>,
    /// Cost multiplier
    pub cost_factor: f64,
}

impl Clone for Method {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            task: self.task,
            precondition: None, // Cannot clone closures
            subtasks: self.subtasks.clone(),
            cost_factor: self.cost_factor,
        }
    }
}

impl Method {
    /// Create new method
    pub fn new(id: MethodId, name: String, task: TaskId) -> Self {
        Self {
            id,
            name,
            task,
            precondition: None,
            subtasks: Vec::new(),
            cost_factor: 1.0,
        }
    }

    /// Add subtask
    pub fn with_subtask(mut self, subtask: TaskId) -> Self {
        self.subtasks.push(subtask);
        self
    }

    /// Set subtasks
    pub fn with_subtasks(mut self, subtasks: Vec<TaskId>) -> Self {
        self.subtasks = subtasks;
        self
    }

    /// Check if method is applicable
    pub fn is_applicable(&self, state: &WorldState) -> bool {
        match &self.precondition {
            Some(f) => f(state),
            None => true,
        }
    }
}

impl core::fmt::Debug for Method {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Method")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("task", &self.task)
            .field("subtasks", &self.subtasks)
            .field("cost_factor", &self.cost_factor)
            .finish()
    }
}

// ============================================================================
// TASK NETWORK
// ============================================================================

/// A task network (plan or partial plan)
#[derive(Debug, Clone)]
pub struct TaskNetwork {
    /// Tasks in network order
    pub tasks: Vec<TaskId>,
    /// Decomposition history
    pub decompositions: Vec<(TaskId, MethodId)>,
}

impl TaskNetwork {
    /// Create empty network
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            decompositions: Vec::new(),
        }
    }

    /// Create network with initial task
    pub fn with_task(task: TaskId) -> Self {
        Self {
            tasks: vec![task],
            decompositions: Vec::new(),
        }
    }

    /// Add task
    pub fn add_task(&mut self, task: TaskId) {
        self.tasks.push(task);
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get first task
    pub fn first(&self) -> Option<TaskId> {
        self.tasks.first().copied()
    }

    /// Remove first task
    pub fn pop_first(&mut self) -> Option<TaskId> {
        if self.tasks.is_empty() {
            None
        } else {
            Some(self.tasks.remove(0))
        }
    }

    /// Record decomposition
    pub fn record_decomposition(&mut self, task: TaskId, method: MethodId) {
        self.decompositions.push((task, method));
    }

    /// Replace first task with subtasks
    pub fn decompose(&mut self, subtasks: Vec<TaskId>, task: TaskId, method: MethodId) {
        if !self.tasks.is_empty() {
            self.tasks.remove(0);
            // Insert subtasks at beginning
            for (i, subtask) in subtasks.into_iter().enumerate() {
                self.tasks.insert(i, subtask);
            }
            self.record_decomposition(task, method);
        }
    }
}

impl Default for TaskNetwork {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// HTN PLANNER
// ============================================================================

/// HTN Planner configuration
#[derive(Debug, Clone)]
pub struct HTNConfig {
    /// Maximum decomposition depth
    pub max_depth: usize,
    /// Maximum plan length
    pub max_plan_length: usize,
    /// Prefer lower cost methods
    pub prefer_low_cost: bool,
}

impl Default for HTNConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            max_plan_length: 1000,
            prefer_low_cost: true,
        }
    }
}

/// HTN Planner
pub struct HTNPlanner {
    /// Configuration
    config: HTNConfig,
    /// Task definitions
    tasks: BTreeMap<TaskId, Task>,
    /// Method definitions
    methods: BTreeMap<MethodId, Method>,
    /// Action space
    actions: ActionSpace,
    /// Next IDs
    next_task_id: u32,
    next_method_id: u32,
}

impl HTNPlanner {
    /// Create new HTN planner
    pub fn new(config: HTNConfig) -> Self {
        Self {
            config,
            tasks: BTreeMap::new(),
            methods: BTreeMap::new(),
            actions: ActionSpace::new(),
            next_task_id: 0,
            next_method_id: 0,
        }
    }

    /// Add task
    pub fn add_task(&mut self, task: Task) -> TaskId {
        let id = task.id;
        self.tasks.insert(id, task);
        id
    }

    /// Create primitive task
    pub fn create_primitive(&mut self, name: String, action: Action) -> TaskId {
        let task_id = TaskId(self.next_task_id);
        self.next_task_id += 1;

        let action_id = self.actions.add(action);
        let task = Task::primitive(task_id, name, action_id);
        self.add_task(task)
    }

    /// Create compound task
    pub fn create_compound(&mut self, name: String) -> TaskId {
        let task_id = TaskId(self.next_task_id);
        self.next_task_id += 1;
        let task = Task::compound(task_id, name);
        self.add_task(task)
    }

    /// Add method
    pub fn add_method(&mut self, method: Method) -> MethodId {
        let id = method.id;
        let task_id = method.task;

        self.methods.insert(id, method);

        // Register method with task
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.methods.push(id);
        }

        id
    }

    /// Create method
    pub fn create_method(&mut self, name: String, task: TaskId, subtasks: Vec<TaskId>) -> MethodId {
        let id = MethodId(self.next_method_id);
        self.next_method_id += 1;
        let method = Method::new(id, name, task).with_subtasks(subtasks);
        self.add_method(method)
    }

    /// Plan from initial state and goal task
    pub fn plan(&self, initial_state: &WorldState, goal_task: TaskId) -> Option<Plan> {
        let mut network = TaskNetwork::with_task(goal_task);
        let mut state = initial_state.clone();
        let mut plan = Plan::new();
        let mut depth = 0;

        while !network.is_empty() {
            if depth > self.config.max_depth {
                return None; // Max depth exceeded
            }
            if plan.actions.len() > self.config.max_plan_length {
                return None; // Plan too long
            }

            let task_id = network.first()?;
            let task = self.tasks.get(&task_id)?;

            if task.is_primitive() {
                // Execute primitive task
                let action_id = task.action?;
                let action = self.actions.get(action_id)?;

                if !action.is_applicable(&state) {
                    return None; // Action not applicable
                }

                // Apply action
                state = action.apply(&state);
                plan.add_action(action_id, action.cost);
                network.pop_first();
            } else {
                // Decompose compound task
                let method = self.find_applicable_method(task, &state)?;
                network.decompose(method.subtasks.clone(), task_id, method.id);
                plan.add_decomposition(task_id, method.id);
                depth += 1;
            }
        }

        Some(plan)
    }

    /// Find applicable method for compound task
    fn find_applicable_method(&self, task: &Task, state: &WorldState) -> Option<&Method> {
        let mut applicable: Vec<&Method> = task
            .methods
            .iter()
            .filter_map(|id| self.methods.get(id))
            .filter(|m| m.is_applicable(state))
            .collect();

        if applicable.is_empty() {
            return None;
        }

        if self.config.prefer_low_cost {
            applicable.sort_by(|a, b| {
                a.cost_factor
                    .partial_cmp(&b.cost_factor)
                    .unwrap_or(core::cmp::Ordering::Equal)
            });
        }

        applicable.into_iter().next()
    }

    /// Get task
    pub fn get_task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(&id)
    }

    /// Get method
    pub fn get_method(&self, id: MethodId) -> Option<&Method> {
        self.methods.get(&id)
    }

    /// Get action
    pub fn get_action(&self, id: ActionId) -> Option<&Action> {
        self.actions.get(id)
    }
}

impl Default for HTNPlanner {
    fn default() -> Self {
        Self::new(HTNConfig::default())
    }
}

// ============================================================================
// PLAN
// ============================================================================

/// A complete plan
#[derive(Debug, Clone)]
pub struct Plan {
    /// Actions in execution order
    pub actions: Vec<ActionId>,
    /// Action costs
    pub costs: Vec<f64>,
    /// Decomposition trace
    pub decompositions: Vec<(TaskId, MethodId)>,
    /// Total cost
    pub total_cost: f64,
}

impl Plan {
    /// Create empty plan
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            costs: Vec::new(),
            decompositions: Vec::new(),
            total_cost: 0.0,
        }
    }

    /// Add action
    pub fn add_action(&mut self, action: ActionId, cost: f64) {
        self.actions.push(action);
        self.costs.push(cost);
        self.total_cost += cost;
    }

    /// Add decomposition record
    pub fn add_decomposition(&mut self, task: TaskId, method: MethodId) {
        self.decompositions.push((task, method));
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Plan length
    pub fn len(&self) -> usize {
        self.actions.len()
    }
}

impl Default for Plan {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::actions::{ActionEffect, ActionPrecondition};
    use super::*;

    #[test]
    fn test_task_creation() {
        let primitive = Task::primitive(TaskId(0), String::from("walk"), ActionId(0));
        assert!(primitive.is_primitive());

        let compound = Task::compound(TaskId(1), String::from("travel"));
        assert!(!compound.is_primitive());
    }

    #[test]
    fn test_task_network() {
        let mut network = TaskNetwork::new();
        network.add_task(TaskId(0));
        network.add_task(TaskId(1));

        assert_eq!(network.first(), Some(TaskId(0)));

        let first = network.pop_first();
        assert_eq!(first, Some(TaskId(0)));
        assert_eq!(network.first(), Some(TaskId(1)));
    }

    #[test]
    fn test_htn_planner_simple() {
        let mut planner = HTNPlanner::default();

        // Create actions
        let action = Action::new(ActionId(0), String::from("open_door"))
            .with_precondition(ActionPrecondition::is_true("has_key"))
            .with_effect(ActionEffect::set_true("door_open"));

        // Create primitive task
        let open_door = planner.create_primitive(String::from("open_door_task"), action);

        // Create initial state
        let mut state = WorldState::new();
        state.set_bool("has_key", true);
        state.set_bool("door_open", false);

        // Plan
        let plan = planner.plan(&state, open_door);
        assert!(plan.is_some());
        assert_eq!(plan.unwrap().len(), 1);
    }

    #[test]
    fn test_htn_decomposition() {
        let mut planner = HTNPlanner::default();

        // Create primitive actions
        let step1 = Action::new(ActionId(0), String::from("step1"))
            .with_effect(ActionEffect::set_true("step1_done"));

        let step2 = Action::new(ActionId(1), String::from("step2"))
            .with_precondition(ActionPrecondition::is_true("step1_done"))
            .with_effect(ActionEffect::set_true("step2_done"));

        let task1 = planner.create_primitive(String::from("do_step1"), step1);
        let task2 = planner.create_primitive(String::from("do_step2"), step2);

        // Create compound task
        let compound = planner.create_compound(String::from("do_both"));
        planner.create_method(String::from("both_method"), compound, vec![task1, task2]);

        // Plan
        let state = WorldState::new();
        let plan = planner.plan(&state, compound);

        assert!(plan.is_some());
        let plan = plan.unwrap();
        assert_eq!(plan.len(), 2); // Both primitive actions
    }
}
