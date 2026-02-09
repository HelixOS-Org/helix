//! NEXUS Year 2: Behavior Tree Implementation
//!
//! Full-featured behavior trees for kernel AI decision making.
//! Supports composite nodes, decorators, parallel execution,
//! and blackboard-based context sharing.

#![allow(dead_code)]

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for behavior tree nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

impl NodeId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Status returned by behavior tree nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorStatus {
    /// Node is still running, needs more ticks
    Running,
    /// Node completed successfully
    Success,
    /// Node failed
    Failure,
    /// Node was interrupted/cancelled
    Cancelled,
    /// Node hasn't started yet
    Ready,
}

impl BehaviorStatus {
    #[inline(always)]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Success | Self::Failure | Self::Cancelled)
    }

    #[inline(always)]
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running)
    }
}

/// Blackboard entry value
#[derive(Debug, Clone)]
pub enum BlackboardValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    NodeId(NodeId),
    Data(Vec<u8>),
}

impl BlackboardValue {
    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    #[inline]
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    #[inline]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }
}

/// Blackboard for sharing data between nodes
#[derive(Debug, Clone)]
pub struct Blackboard {
    entries: BTreeMap<String, BlackboardValue>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    #[inline(always)]
    pub fn get(&self, key: &str) -> Option<&BlackboardValue> {
        self.entries.get(key)
    }

    #[inline(always)]
    pub fn set(&mut self, key: String, value: BlackboardValue) {
        self.entries.insert(key, value);
    }

    #[inline(always)]
    pub fn remove(&mut self, key: &str) -> Option<BlackboardValue> {
        self.entries.remove(key)
    }

    #[inline(always)]
    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Context passed to behavior tree nodes during execution
#[repr(align(64))]
pub struct TreeContext<'a> {
    /// Shared blackboard for data exchange
    pub blackboard: &'a mut Blackboard,
    /// Current simulation/game time
    pub time: u64,
    /// Delta time since last tick (microseconds)
    pub delta_time: u64,
    /// Node execution stack for debugging
    pub execution_stack: Vec<NodeId>,
}

impl<'a> TreeContext<'a> {
    pub fn new(blackboard: &'a mut Blackboard, time: u64, delta_time: u64) -> Self {
        Self {
            blackboard,
            time,
            delta_time,
            execution_stack: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn push_node(&mut self, id: NodeId) {
        self.execution_stack.push(id);
    }

    #[inline(always)]
    pub fn pop_node(&mut self) -> Option<NodeId> {
        self.execution_stack.pop()
    }
}

// ============================================================================
// Behavior Node Trait
// ============================================================================

/// Core trait for all behavior tree nodes
pub trait BehaviorNode: Send + Sync {
    /// Get this node's unique identifier
    fn id(&self) -> NodeId;

    /// Get this node's name for debugging
    fn name(&self) -> &str;

    /// Initialize the node before first execution
    fn initialize(&mut self, _ctx: &mut TreeContext) {}

    /// Execute one tick of this node
    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus;

    /// Called when node is interrupted
    fn abort(&mut self, _ctx: &mut TreeContext) {}

    /// Reset node to initial state
    fn reset(&mut self) {}
}

// ============================================================================
// Leaf Nodes
// ============================================================================

/// Action node - performs an action and returns status
pub struct ActionNode<F>
where
    F: FnMut(&mut TreeContext) -> BehaviorStatus + Send + Sync,
{
    id: NodeId,
    name: String,
    action: F,
    status: BehaviorStatus,
}

impl<F> ActionNode<F>
where
    F: FnMut(&mut TreeContext) -> BehaviorStatus + Send + Sync,
{
    pub fn new(id: NodeId, name: impl Into<String>, action: F) -> Self {
        Self {
            id,
            name: name.into(),
            action,
            status: BehaviorStatus::Ready,
        }
    }
}

impl<F> BehaviorNode for ActionNode<F>
where
    F: FnMut(&mut TreeContext) -> BehaviorStatus + Send + Sync,
{
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        self.status = (self.action)(ctx);
        self.status
    }

    fn reset(&mut self) {
        self.status = BehaviorStatus::Ready;
    }
}

/// Condition node - checks a condition
pub struct ConditionNode<F>
where
    F: Fn(&TreeContext) -> bool + Send + Sync,
{
    id: NodeId,
    name: String,
    condition: F,
}

impl<F> ConditionNode<F>
where
    F: Fn(&TreeContext) -> bool + Send + Sync,
{
    pub fn new(id: NodeId, name: impl Into<String>, condition: F) -> Self {
        Self {
            id,
            name: name.into(),
            condition,
        }
    }
}

impl<F> BehaviorNode for ConditionNode<F>
where
    F: Fn(&TreeContext) -> bool + Send + Sync,
{
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        if (self.condition)(ctx) {
            BehaviorStatus::Success
        } else {
            BehaviorStatus::Failure
        }
    }
}

/// Wait node - waits for a specified duration
pub struct WaitNode {
    id: NodeId,
    name: String,
    duration_us: u64,
    elapsed: u64,
}

impl WaitNode {
    pub fn new(id: NodeId, name: impl Into<String>, duration_us: u64) -> Self {
        Self {
            id,
            name: name.into(),
            duration_us,
            elapsed: 0,
        }
    }
}

impl BehaviorNode for WaitNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        self.elapsed += ctx.delta_time;
        if self.elapsed >= self.duration_us {
            BehaviorStatus::Success
        } else {
            BehaviorStatus::Running
        }
    }

    fn reset(&mut self) {
        self.elapsed = 0;
    }
}

// ============================================================================
// Composite Nodes
// ============================================================================

/// Sequence node - runs children in order until one fails
pub struct Sequence {
    id: NodeId,
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
    current_child: usize,
}

impl Sequence {
    pub fn new(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            children: Vec::new(),
            current_child: 0,
        }
    }

    #[inline(always)]
    pub fn add_child(&mut self, child: Box<dyn BehaviorNode>) {
        self.children.push(child);
    }

    #[inline(always)]
    pub fn with_child(mut self, child: Box<dyn BehaviorNode>) -> Self {
        self.add_child(child);
        self
    }
}

impl BehaviorNode for Sequence {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn initialize(&mut self, ctx: &mut TreeContext) {
        for child in &mut self.children {
            child.initialize(ctx);
        }
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        while self.current_child < self.children.len() {
            ctx.push_node(self.children[self.current_child].id());
            let status = self.children[self.current_child].tick(ctx);
            ctx.pop_node();

            match status {
                BehaviorStatus::Running => return BehaviorStatus::Running,
                BehaviorStatus::Failure => {
                    self.current_child = 0;
                    return BehaviorStatus::Failure;
                },
                BehaviorStatus::Success => {
                    self.current_child += 1;
                },
                BehaviorStatus::Cancelled => {
                    self.current_child = 0;
                    return BehaviorStatus::Cancelled;
                },
                BehaviorStatus::Ready => {
                    // Child not started, tick it again
                },
            }
        }

        self.current_child = 0;
        BehaviorStatus::Success
    }

    fn abort(&mut self, ctx: &mut TreeContext) {
        if self.current_child < self.children.len() {
            self.children[self.current_child].abort(ctx);
        }
        self.current_child = 0;
    }

    fn reset(&mut self) {
        self.current_child = 0;
        for child in &mut self.children {
            child.reset();
        }
    }
}

/// Selector node - runs children until one succeeds
pub struct Selector {
    id: NodeId,
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
    current_child: usize,
}

impl Selector {
    pub fn new(id: NodeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            children: Vec::new(),
            current_child: 0,
        }
    }

    #[inline(always)]
    pub fn add_child(&mut self, child: Box<dyn BehaviorNode>) {
        self.children.push(child);
    }

    #[inline(always)]
    pub fn with_child(mut self, child: Box<dyn BehaviorNode>) -> Self {
        self.add_child(child);
        self
    }
}

impl BehaviorNode for Selector {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn initialize(&mut self, ctx: &mut TreeContext) {
        for child in &mut self.children {
            child.initialize(ctx);
        }
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        while self.current_child < self.children.len() {
            ctx.push_node(self.children[self.current_child].id());
            let status = self.children[self.current_child].tick(ctx);
            ctx.pop_node();

            match status {
                BehaviorStatus::Running => return BehaviorStatus::Running,
                BehaviorStatus::Success => {
                    self.current_child = 0;
                    return BehaviorStatus::Success;
                },
                BehaviorStatus::Failure => {
                    self.current_child += 1;
                },
                BehaviorStatus::Cancelled => {
                    self.current_child = 0;
                    return BehaviorStatus::Cancelled;
                },
                BehaviorStatus::Ready => {
                    // Child not started, tick it again
                },
            }
        }

        self.current_child = 0;
        BehaviorStatus::Failure
    }

    fn abort(&mut self, ctx: &mut TreeContext) {
        if self.current_child < self.children.len() {
            self.children[self.current_child].abort(ctx);
        }
        self.current_child = 0;
    }

    fn reset(&mut self) {
        self.current_child = 0;
        for child in &mut self.children {
            child.reset();
        }
    }
}

/// Parallel execution policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelPolicy {
    /// Succeed if all children succeed
    RequireAll,
    /// Succeed if any child succeeds
    RequireOne,
    /// Succeed if N children succeed
    RequireN(usize),
}

/// Parallel node - runs all children simultaneously
pub struct Parallel {
    id: NodeId,
    name: String,
    children: Vec<Box<dyn BehaviorNode>>,
    policy: ParallelPolicy,
    child_statuses: Vec<BehaviorStatus>,
}

impl Parallel {
    pub fn new(id: NodeId, name: impl Into<String>, policy: ParallelPolicy) -> Self {
        Self {
            id,
            name: name.into(),
            children: Vec::new(),
            policy,
            child_statuses: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn add_child(&mut self, child: Box<dyn BehaviorNode>) {
        self.children.push(child);
        self.child_statuses.push(BehaviorStatus::Ready);
    }
}

impl BehaviorNode for Parallel {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn initialize(&mut self, ctx: &mut TreeContext) {
        for child in &mut self.children {
            child.initialize(ctx);
        }
        self.child_statuses = self
            .children
            .iter()
            .map(|_| BehaviorStatus::Ready)
            .collect();
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut running_count = 0;

        for (i, child) in self.children.iter_mut().enumerate() {
            if !self.child_statuses[i].is_terminal() {
                ctx.push_node(child.id());
                self.child_statuses[i] = child.tick(ctx);
                ctx.pop_node();
            }

            match self.child_statuses[i] {
                BehaviorStatus::Success => success_count += 1,
                BehaviorStatus::Failure => failure_count += 1,
                BehaviorStatus::Running | BehaviorStatus::Ready => running_count += 1,
                BehaviorStatus::Cancelled => {},
            }
        }

        match self.policy {
            ParallelPolicy::RequireAll => {
                if failure_count > 0 {
                    BehaviorStatus::Failure
                } else if running_count > 0 {
                    BehaviorStatus::Running
                } else {
                    BehaviorStatus::Success
                }
            },
            ParallelPolicy::RequireOne => {
                if success_count > 0 {
                    BehaviorStatus::Success
                } else if running_count > 0 {
                    BehaviorStatus::Running
                } else {
                    BehaviorStatus::Failure
                }
            },
            ParallelPolicy::RequireN(n) => {
                if success_count >= n {
                    BehaviorStatus::Success
                } else if success_count + running_count < n {
                    BehaviorStatus::Failure
                } else {
                    BehaviorStatus::Running
                }
            },
        }
    }

    fn abort(&mut self, ctx: &mut TreeContext) {
        for (i, child) in self.children.iter_mut().enumerate() {
            if !self.child_statuses[i].is_terminal() {
                child.abort(ctx);
            }
        }
    }

    fn reset(&mut self) {
        self.child_statuses = self
            .children
            .iter()
            .map(|_| BehaviorStatus::Ready)
            .collect();
        for child in &mut self.children {
            child.reset();
        }
    }
}

// ============================================================================
// Decorator Nodes
// ============================================================================

/// Decorator type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoratorType {
    /// Inverts child result
    Inverter,
    /// Always succeeds
    Succeeder,
    /// Repeats child N times
    Repeat(usize),
    /// Repeats until child fails
    RepeatUntilFail,
    /// Repeats until child succeeds
    RepeatUntilSuccess,
    /// Times out after duration
    Timeout(u64),
    /// Limits execution to once
    Once,
}

/// Generic decorator wrapper
pub struct Decorator {
    id: NodeId,
    name: String,
    child: Box<dyn BehaviorNode>,
    decorator_type: DecoratorType,
    repeat_count: usize,
    has_executed: bool,
    elapsed: u64,
}

impl Decorator {
    pub fn new(
        id: NodeId,
        name: impl Into<String>,
        child: Box<dyn BehaviorNode>,
        decorator_type: DecoratorType,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            child,
            decorator_type,
            repeat_count: 0,
            has_executed: false,
            elapsed: 0,
        }
    }

    #[inline(always)]
    pub fn inverter(id: NodeId, name: impl Into<String>, child: Box<dyn BehaviorNode>) -> Self {
        Self::new(id, name, child, DecoratorType::Inverter)
    }

    #[inline(always)]
    pub fn succeeder(id: NodeId, name: impl Into<String>, child: Box<dyn BehaviorNode>) -> Self {
        Self::new(id, name, child, DecoratorType::Succeeder)
    }

    #[inline(always)]
    pub fn repeat(
        id: NodeId,
        name: impl Into<String>,
        child: Box<dyn BehaviorNode>,
        times: usize,
    ) -> Self {
        Self::new(id, name, child, DecoratorType::Repeat(times))
    }

    #[inline(always)]
    pub fn timeout(
        id: NodeId,
        name: impl Into<String>,
        child: Box<dyn BehaviorNode>,
        timeout_us: u64,
    ) -> Self {
        Self::new(id, name, child, DecoratorType::Timeout(timeout_us))
    }
}

impl BehaviorNode for Decorator {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn initialize(&mut self, ctx: &mut TreeContext) {
        self.child.initialize(ctx);
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        match self.decorator_type {
            DecoratorType::Inverter => match self.child.tick(ctx) {
                BehaviorStatus::Success => BehaviorStatus::Failure,
                BehaviorStatus::Failure => BehaviorStatus::Success,
                other => other,
            },
            DecoratorType::Succeeder => {
                let status = self.child.tick(ctx);
                if status.is_terminal() {
                    BehaviorStatus::Success
                } else {
                    status
                }
            },
            DecoratorType::Repeat(n) => loop {
                let status = self.child.tick(ctx);
                match status {
                    BehaviorStatus::Success => {
                        self.repeat_count += 1;
                        if self.repeat_count >= n {
                            self.repeat_count = 0;
                            return BehaviorStatus::Success;
                        }
                        self.child.reset();
                    },
                    BehaviorStatus::Failure => {
                        self.repeat_count = 0;
                        return BehaviorStatus::Failure;
                    },
                    other => return other,
                }
            },
            DecoratorType::RepeatUntilFail => loop {
                let status = self.child.tick(ctx);
                match status {
                    BehaviorStatus::Failure => {
                        return BehaviorStatus::Success;
                    },
                    BehaviorStatus::Success => {
                        self.child.reset();
                    },
                    other => return other,
                }
            },
            DecoratorType::RepeatUntilSuccess => loop {
                let status = self.child.tick(ctx);
                match status {
                    BehaviorStatus::Success => {
                        return BehaviorStatus::Success;
                    },
                    BehaviorStatus::Failure => {
                        self.child.reset();
                    },
                    other => return other,
                }
            },
            DecoratorType::Timeout(timeout_us) => {
                self.elapsed += ctx.delta_time;
                if self.elapsed >= timeout_us {
                    self.child.abort(ctx);
                    return BehaviorStatus::Failure;
                }
                self.child.tick(ctx)
            },
            DecoratorType::Once => {
                if self.has_executed {
                    BehaviorStatus::Failure
                } else {
                    let status = self.child.tick(ctx);
                    if status.is_terminal() {
                        self.has_executed = true;
                    }
                    status
                }
            },
        }
    }

    fn abort(&mut self, ctx: &mut TreeContext) {
        self.child.abort(ctx);
    }

    fn reset(&mut self) {
        self.repeat_count = 0;
        self.has_executed = false;
        self.elapsed = 0;
        self.child.reset();
    }
}

// ============================================================================
// Behavior Tree
// ============================================================================

/// Complete behavior tree structure
pub struct BehaviorTree {
    name: String,
    root: Box<dyn BehaviorNode>,
    blackboard: Blackboard,
    status: BehaviorStatus,
}

impl BehaviorTree {
    pub fn new(name: impl Into<String>, root: Box<dyn BehaviorNode>) -> Self {
        Self {
            name: name.into(),
            root,
            blackboard: Blackboard::new(),
            status: BehaviorStatus::Ready,
        }
    }

    #[inline(always)]
    pub fn with_blackboard(mut self, blackboard: Blackboard) -> Self {
        self.blackboard = blackboard;
        self
    }

    #[inline(always)]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline(always)]
    pub fn status(&self) -> BehaviorStatus {
        self.status
    }

    #[inline(always)]
    pub fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }

    #[inline(always)]
    pub fn blackboard_mut(&mut self) -> &mut Blackboard {
        &mut self.blackboard
    }

    /// Initialize the tree
    #[inline(always)]
    pub fn initialize(&mut self) {
        let mut ctx = TreeContext::new(&mut self.blackboard, 0, 0);
        self.root.initialize(&mut ctx);
    }

    /// Tick the tree once
    #[inline]
    pub fn tick(&mut self, time: u64, delta_time: u64) -> BehaviorStatus {
        let mut ctx = TreeContext::new(&mut self.blackboard, time, delta_time);
        self.status = self.root.tick(&mut ctx);
        self.status
    }

    /// Abort the tree
    #[inline]
    pub fn abort(&mut self) {
        let mut ctx = TreeContext::new(&mut self.blackboard, 0, 0);
        self.root.abort(&mut ctx);
        self.status = BehaviorStatus::Cancelled;
    }

    /// Reset the tree
    #[inline(always)]
    pub fn reset(&mut self) {
        self.root.reset();
        self.status = BehaviorStatus::Ready;
    }
}

// ============================================================================
// Tree Executor
// ============================================================================

/// Executor for managing multiple behavior trees
pub struct TreeExecutor {
    trees: BTreeMap<String, BehaviorTree>,
    active_tree: Option<String>,
    last_tick_time: u64,
}

impl TreeExecutor {
    pub fn new() -> Self {
        Self {
            trees: BTreeMap::new(),
            active_tree: None,
            last_tick_time: 0,
        }
    }

    #[inline]
    pub fn register_tree(&mut self, tree: BehaviorTree) {
        let name = tree.name().to_string();
        self.trees.insert(name.clone(), tree);
        if self.active_tree.is_none() {
            self.active_tree = Some(name);
        }
    }

    #[inline]
    pub fn set_active(&mut self, name: &str) -> bool {
        if self.trees.contains_key(name) {
            self.active_tree = Some(name.to_string());
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn tick(&mut self, time: u64) -> Option<BehaviorStatus> {
        let delta_time = time.saturating_sub(self.last_tick_time);
        self.last_tick_time = time;

        if let Some(ref name) = self.active_tree {
            if let Some(tree) = self.trees.get_mut(name) {
                return Some(tree.tick(time, delta_time));
            }
        }
        None
    }

    #[inline(always)]
    pub fn get_tree(&self, name: &str) -> Option<&BehaviorTree> {
        self.trees.get(name)
    }

    #[inline(always)]
    pub fn get_tree_mut(&mut self, name: &str) -> Option<&mut BehaviorTree> {
        self.trees.get_mut(name)
    }

    #[inline(always)]
    pub fn tree_count(&self) -> usize {
        self.trees.len()
    }
}

impl Default for TreeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Kernel-Specific Nodes
// ============================================================================

/// Memory pressure check node
pub struct CheckMemoryPressure {
    id: NodeId,
    threshold: f32,
}

impl CheckMemoryPressure {
    pub fn new(id: NodeId, threshold: f32) -> Self {
        Self { id, threshold }
    }
}

impl BehaviorNode for CheckMemoryPressure {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        "CheckMemoryPressure"
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        if let Some(BlackboardValue::Float(pressure)) = ctx.blackboard.get("memory_pressure") {
            if *pressure > self.threshold as f64 {
                BehaviorStatus::Success
            } else {
                BehaviorStatus::Failure
            }
        } else {
            BehaviorStatus::Failure
        }
    }
}

/// CPU load check node
pub struct CheckCpuLoad {
    id: NodeId,
    threshold: f32,
}

impl CheckCpuLoad {
    pub fn new(id: NodeId, threshold: f32) -> Self {
        Self { id, threshold }
    }
}

impl BehaviorNode for CheckCpuLoad {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        "CheckCpuLoad"
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        if let Some(BlackboardValue::Float(load)) = ctx.blackboard.get("cpu_load") {
            if *load > self.threshold as f64 {
                BehaviorStatus::Success
            } else {
                BehaviorStatus::Failure
            }
        } else {
            BehaviorStatus::Failure
        }
    }
}

/// Kernel action node
pub struct KernelActionNode {
    id: NodeId,
    name: String,
    action_type: KernelAction,
}

/// Types of kernel actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelAction {
    ReclaimMemory,
    TriggerGC,
    MigrateProcesses,
    ThrottleCpu,
    EnablePowerSave,
    DisablePowerSave,
    ExpandSwap,
    CompactMemory,
}

impl KernelActionNode {
    pub fn new(id: NodeId, name: impl Into<String>, action_type: KernelAction) -> Self {
        Self {
            id,
            name: name.into(),
            action_type,
        }
    }
}

impl BehaviorNode for KernelActionNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn tick(&mut self, ctx: &mut TreeContext) -> BehaviorStatus {
        // Log the action to blackboard
        ctx.blackboard.set(
            alloc::format!("last_action_{}", ctx.time),
            BlackboardValue::Int(self.action_type as i64),
        );

        // In real kernel, this would trigger actual actions
        match self.action_type {
            KernelAction::ReclaimMemory => {
                ctx.blackboard
                    .set("reclaim_requested".into(), BlackboardValue::Bool(true));
            },
            KernelAction::TriggerGC => {
                ctx.blackboard
                    .set("gc_requested".into(), BlackboardValue::Bool(true));
            },
            KernelAction::MigrateProcesses => {
                ctx.blackboard
                    .set("migration_requested".into(), BlackboardValue::Bool(true));
            },
            KernelAction::ThrottleCpu => {
                ctx.blackboard
                    .set("throttle_requested".into(), BlackboardValue::Bool(true));
            },
            KernelAction::EnablePowerSave => {
                ctx.blackboard
                    .set("power_save".into(), BlackboardValue::Bool(true));
            },
            KernelAction::DisablePowerSave => {
                ctx.blackboard
                    .set("power_save".into(), BlackboardValue::Bool(false));
            },
            KernelAction::ExpandSwap => {
                ctx.blackboard
                    .set("expand_swap".into(), BlackboardValue::Bool(true));
            },
            KernelAction::CompactMemory => {
                ctx.blackboard
                    .set("compact_memory".into(), BlackboardValue::Bool(true));
            },
        }

        BehaviorStatus::Success
    }
}

// ============================================================================
// Builder Pattern
// ============================================================================

/// Builder for creating behavior trees
pub struct BehaviorTreeBuilder {
    name: String,
    next_id: u64,
    blackboard: Blackboard,
}

impl BehaviorTreeBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            next_id: 1,
            blackboard: Blackboard::new(),
        }
    }

    fn next_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        id
    }

    #[inline(always)]
    pub fn with_blackboard_entry(mut self, key: String, value: BlackboardValue) -> Self {
        self.blackboard.set(key, value);
        self
    }

    #[inline(always)]
    pub fn sequence(&mut self, name: impl Into<String>) -> Sequence {
        Sequence::new(self.next_id(), name)
    }

    #[inline(always)]
    pub fn selector(&mut self, name: impl Into<String>) -> Selector {
        Selector::new(self.next_id(), name)
    }

    #[inline(always)]
    pub fn parallel(&mut self, name: impl Into<String>, policy: ParallelPolicy) -> Parallel {
        Parallel::new(self.next_id(), name, policy)
    }

    #[inline(always)]
    pub fn wait(&mut self, name: impl Into<String>, duration_us: u64) -> WaitNode {
        WaitNode::new(self.next_id(), name, duration_us)
    }

    #[inline(always)]
    pub fn build(self, root: Box<dyn BehaviorNode>) -> BehaviorTree {
        BehaviorTree::new(self.name, root).with_blackboard(self.blackboard)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_behavior_status() {
        assert!(BehaviorStatus::Success.is_terminal());
        assert!(BehaviorStatus::Failure.is_terminal());
        assert!(!BehaviorStatus::Running.is_terminal());
        assert!(BehaviorStatus::Running.is_running());
    }

    #[test]
    fn test_blackboard() {
        let mut bb = Blackboard::new();
        bb.set("test".into(), BlackboardValue::Int(42));

        assert!(bb.contains("test"));
        assert_eq!(bb.get("test").unwrap().as_int(), Some(42));

        bb.remove("test");
        assert!(!bb.contains("test"));
    }

    #[test]
    fn test_sequence() {
        let mut seq = Sequence::new(NodeId(1), "test_seq");
        assert_eq!(seq.name(), "test_seq");
        assert_eq!(seq.id(), NodeId(1));
    }

    #[test]
    fn test_selector() {
        let sel = Selector::new(NodeId(1), "test_sel");
        assert_eq!(sel.name(), "test_sel");
    }

    #[test]
    fn test_parallel_policy() {
        assert_eq!(ParallelPolicy::RequireAll, ParallelPolicy::RequireAll);
        assert_ne!(ParallelPolicy::RequireAll, ParallelPolicy::RequireOne);
    }
}
