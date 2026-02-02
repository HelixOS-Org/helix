//! Causal Query Language (CQL)
//!
//! This module provides a query language for causal reasoning.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    CausalChain, CausalChainBuilder, CausalEventId, CausalEventType, CausalGraph, CausalNodeId,
    CounterfactualEngine, CounterfactualModification, CounterfactualResult, Explanation,
    ExplanationGenerator,
};

/// CQL query type
#[derive(Debug, Clone)]
pub enum CqlQuery {
    /// Find why event happened
    WhyQuery { event: CausalEventId },
    /// Find root causes
    RootCausesQuery { event: CausalEventId },
    /// Find effects
    EffectsQuery { event: CausalEventId },
    /// Counterfactual query
    CounterfactualQuery {
        event: CausalEventId,
        modification: CounterfactualModification,
    },
    /// Pattern query
    PatternQuery {
        event_type: CausalEventType,
        time_range: Option<(u64, u64)>,
    },
    /// Chain query
    ChainQuery {
        from: CausalEventId,
        to: CausalEventId,
    },
}

/// CQL result
#[derive(Debug)]
pub enum CqlResult {
    /// Explanation result
    Explanation(Explanation),
    /// Node list result
    NodeList(Vec<CausalNodeId>),
    /// Chain result
    Chain(CausalChain),
    /// Counterfactual result
    Counterfactual(CounterfactualResult),
    /// Error
    Error(String),
}

impl CqlResult {
    /// Is error result
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    /// Get error message
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

/// CQL query engine
pub struct CqlEngine {
    /// Explanation generator
    explainer: ExplanationGenerator,
    /// Counterfactual engine
    counterfactual: CounterfactualEngine,
    /// Chain builder
    chain_builder: CausalChainBuilder,
    /// Query counter
    query_counter: AtomicU64,
}

impl CqlEngine {
    /// Create new engine
    pub fn new() -> Self {
        Self {
            explainer: ExplanationGenerator::new(),
            counterfactual: CounterfactualEngine::new(),
            chain_builder: CausalChainBuilder::new(),
            query_counter: AtomicU64::new(0),
        }
    }

    /// Execute query
    pub fn execute(&self, graph: &CausalGraph, query: CqlQuery) -> CqlResult {
        self.query_counter.fetch_add(1, Ordering::Relaxed);

        match query {
            CqlQuery::WhyQuery { event } => match self.explainer.explain_why(graph, event) {
                Some(explanation) => CqlResult::Explanation(explanation),
                None => CqlResult::Error(String::from("Event not found")),
            },
            CqlQuery::RootCausesQuery { event } => {
                if let Some(node) = graph.get_node_by_event(event) {
                    let roots = graph.find_root_causes(node.id);
                    CqlResult::NodeList(roots)
                } else {
                    CqlResult::Error(String::from("Event not found"))
                }
            },
            CqlQuery::EffectsQuery { event } => {
                if let Some(node) = graph.get_node_by_event(event) {
                    let effects = graph.find_effects(node.id);
                    CqlResult::NodeList(effects)
                } else {
                    CqlResult::Error(String::from("Event not found"))
                }
            },
            CqlQuery::CounterfactualQuery {
                event,
                modification,
            } => {
                let scenario = self.counterfactual.create_scenario(
                    String::from("What if this event didn't happen?"),
                    event,
                    modification,
                );
                let result = self.counterfactual.simulate(graph, scenario);
                CqlResult::Counterfactual(result)
            },
            CqlQuery::PatternQuery {
                event_type,
                time_range,
            } => {
                let mut matching = Vec::new();
                for node in graph.nodes() {
                    if node.event.event_type == event_type {
                        if let Some((start, end)) = time_range {
                            if node.event.timestamp >= start && node.event.timestamp <= end {
                                matching.push(node.id);
                            }
                        } else {
                            matching.push(node.id);
                        }
                    }
                }
                CqlResult::NodeList(matching)
            },
            CqlQuery::ChainQuery { from, to } => {
                if let (Some(from_node), Some(to_node)) =
                    (graph.get_node_by_event(from), graph.get_node_by_event(to))
                {
                    match self
                        .chain_builder
                        .build_chain(graph, from_node.id, to_node.id)
                    {
                        Some(chain) => CqlResult::Chain(chain),
                        None => CqlResult::Error(String::from("No path found")),
                    }
                } else {
                    CqlResult::Error(String::from("Event not found"))
                }
            },
        }
    }

    /// Query count
    pub fn query_count(&self) -> u64 {
        self.query_counter.load(Ordering::Relaxed)
    }
}

impl Default for CqlEngine {
    fn default() -> Self {
        Self::new()
    }
}
