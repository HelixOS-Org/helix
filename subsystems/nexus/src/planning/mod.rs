//! # Planning Module for NEXUS
//!
//! Year 2 "COGNITION" - Hierarchical planning and goal management.
//!
//! ## Components
//!
//! - `goals`: Goal representation and management
//! - `actions`: Action definitions and preconditions
//! - `hierarchical`: HTN (Hierarchical Task Network) planning
//! - `temporal`: Temporal planning with constraints
//! - `reactive`: Reactive planning and replanning

#![allow(dead_code)]

pub mod actions;
pub mod goals;
pub mod hierarchical;
pub mod reactive;
pub mod temporal;

// Re-exports
pub use actions::{Action, ActionEffect, ActionId, ActionPrecondition, ActionSpace as PlanActionSpace};
pub use goals::{Goal, GoalId, GoalManager, GoalPriority, GoalStatus};
pub use hierarchical::{HTNPlanner, Method, MethodId, Task, TaskId, TaskNetwork};
pub use reactive::{ReactivePlanner, Trigger, TriggerCondition, Response};
pub use temporal::{TemporalConstraint, TemporalPlanner, TimePoint, Timeline};
