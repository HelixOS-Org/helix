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

#![no_std]

extern crate alloc;

pub mod tree;
pub mod state_machine;
pub mod reactive;
pub mod utility;
pub mod composite;

// Re-export key types
pub use tree::{
    BehaviorTree, BehaviorNode, BehaviorStatus,
    Selector, Sequence, Parallel, Decorator,
    TreeExecutor, TreeContext,
};

pub use state_machine::{
    State, StateId, Transition, TransitionCondition,
    StateMachine, HierarchicalStateMachine,
    StateContext, StateEvent,
};

pub use reactive::{
    Stimulus, StimulusId, Response, ResponseId,
    ReactiveBehavior, ReactiveLayer, BehaviorPriority,
};

pub use utility::{
    Consideration, ConsiderationId, UtilityCurve,
    UtilityAction, UtilitySelector, ReasonerAI,
};

pub use composite::{
    BehaviorComposite, BehaviorBlend, BehaviorLayer,
    CompositeExecutor,
};
