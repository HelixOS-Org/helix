//! NEXUS Year 2: Behavior Module
//!
//! Advanced behavior modeling for kernel AI including behavior trees,
//! state machines, reactive behaviors, and utility-based AI.
//!
//! # Submodules
//!
//! - `tree`: Behavior tree implementation with decorators
//! - `state_machine`: Hierarchical finite state machines
//! - `reactive`: Reactive behavior patterns
//! - `utility`: Utility-based AI decision making
//! - `composite`: Composite behavior patterns

extern crate alloc;

pub mod composite;
pub mod reactive;
pub mod state_machine;
pub mod tree;
pub mod utility;

// Re-export key types
pub use composite::{BehaviorBlend, BehaviorComposite, BehaviorLayer, CompositeExecutor};
pub use reactive::{
    BehaviorPriority, ReactiveBehavior, ReactiveLayer, Response, ResponseId, Stimulus, StimulusId,
};
pub use state_machine::{
    HierarchicalStateMachine, State, StateContext, StateEvent, StateId, StateMachine, Transition,
    TransitionCondition,
};
pub use tree::{
    BehaviorNode, BehaviorStatus, BehaviorTree, Decorator, Parallel, Selector, Sequence,
    TreeContext, TreeExecutor,
};
pub use utility::{
    Consideration, ConsiderationId, ReasonerAI, UtilityAction, UtilityCurve, UtilitySelector,
};
