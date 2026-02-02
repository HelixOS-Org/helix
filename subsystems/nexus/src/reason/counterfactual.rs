//! Counterfactual Reasoning
//!
//! This module provides "what if" counterfactual analysis.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{CausalEvent, CausalEventId, CausalEventType, CausalGraph, EventSeverity};

/// Counterfactual scenario
#[derive(Debug, Clone)]
pub struct CounterfactualScenario {
    /// Scenario ID
    pub id: u64,
    /// Description
    pub description: String,
    /// Event to remove/modify
    pub modified_event: CausalEventId,
    /// Modification type
    pub modification: CounterfactualModification,
}

/// Counterfactual modification
#[derive(Debug, Clone)]
pub enum CounterfactualModification {
    /// Remove event entirely
    Remove,
    /// Delay event by duration
    Delay(u64),
    /// Change event property
    ChangeProperty { key: String, new_value: String },
    /// Change event type
    ChangeType(CausalEventType),
    /// Reduce severity
    ReduceSeverity(EventSeverity),
}

/// Counterfactual result
#[derive(Debug, Clone)]
pub struct CounterfactualResult {
    /// Scenario
    pub scenario: CounterfactualScenario,
    /// Events that would not have occurred
    pub prevented_events: Vec<CausalEventId>,
    /// Events that would still occur
    pub remaining_events: Vec<CausalEventId>,
    /// New events that might occur
    pub new_events: Vec<CausalEvent>,
    /// Impact assessment
    pub impact: CounterfactualImpact,
}

/// Counterfactual impact
#[derive(Debug, Clone)]
pub struct CounterfactualImpact {
    /// Severity reduction (0.0 - 1.0)
    pub severity_reduction: f32,
    /// Events prevented count
    pub events_prevented: u32,
    /// Errors prevented
    pub errors_prevented: u32,
    /// Estimated time saved (nanoseconds)
    pub time_saved: u64,
    /// Confidence in assessment
    pub confidence: f32,
}

impl CounterfactualImpact {
    /// Create empty impact
    pub fn empty() -> Self {
        Self {
            severity_reduction: 0.0,
            events_prevented: 0,
            errors_prevented: 0,
            time_saved: 0,
            confidence: 0.5,
        }
    }

    /// Is significant impact
    pub fn is_significant(&self) -> bool {
        self.severity_reduction > 0.2 || self.errors_prevented > 0
    }
}

/// Counterfactual engine
pub struct CounterfactualEngine {
    /// Scenario counter
    scenario_counter: AtomicU64,
}

impl CounterfactualEngine {
    /// Create new engine
    pub fn new() -> Self {
        Self {
            scenario_counter: AtomicU64::new(0),
        }
    }

    /// Create scenario
    pub fn create_scenario(
        &self,
        description: String,
        event: CausalEventId,
        modification: CounterfactualModification,
    ) -> CounterfactualScenario {
        CounterfactualScenario {
            id: self.scenario_counter.fetch_add(1, Ordering::Relaxed),
            description,
            modified_event: event,
            modification,
        }
    }

    /// Simulate counterfactual
    pub fn simulate(
        &self,
        graph: &CausalGraph,
        scenario: CounterfactualScenario,
    ) -> CounterfactualResult {
        let mut prevented = Vec::new();
        let mut remaining = Vec::new();

        // Find the node for the modified event
        if let Some(node) = graph.get_node_by_event(scenario.modified_event) {
            // All effects would be prevented (simplified model)
            for effect_id in graph.find_effects(node.id) {
                if let Some(effect_node) = graph.get_node(effect_id) {
                    prevented.push(effect_node.event.id);
                }
            }

            // Nodes not affected remain
            for other_node in graph.nodes.values() {
                if other_node.id != node.id && !prevented.contains(&other_node.event.id) {
                    remaining.push(other_node.event.id);
                }
            }
        }

        // Calculate impact
        let errors_prevented = prevented
            .iter()
            .filter_map(|id| graph.get_node_by_event(*id))
            .filter(|n| n.event.event_type.is_error())
            .count() as u32;

        let severity_reduction = if graph.node_count() > 0 {
            prevented.len() as f32 / graph.node_count() as f32
        } else {
            0.0
        };

        CounterfactualResult {
            scenario,
            prevented_events: prevented,
            remaining_events: remaining,
            new_events: Vec::new(),
            impact: CounterfactualImpact {
                severity_reduction,
                events_prevented: errors_prevented,
                errors_prevented,
                time_saved: 0,
                confidence: 0.7,
            },
        }
    }

    /// Scenario count
    pub fn scenario_count(&self) -> u64 {
        self.scenario_counter.load(Ordering::Relaxed)
    }
}

impl Default for CounterfactualEngine {
    fn default() -> Self {
        Self::new()
    }
}
