//! Causal Reasoning Intelligence
//!
//! This module provides the main causal reasoning interface.

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    CausalEvent, CausalEventId, CausalEventType, CausalGraph, CausalRelationType, CqlEngine,
    CqlQuery, CqlResult,
};

/// Causal reasoning analysis
#[derive(Debug, Clone)]
pub struct CausalReasoningAnalysis {
    /// Total events
    pub total_events: u64,
    /// Total relationships
    pub total_relationships: u64,
    /// Root causes count
    pub root_causes: u64,
    /// Terminal effects count
    pub terminal_effects: u64,
    /// Average chain length
    pub avg_chain_length: f32,
    /// Average confidence
    pub avg_confidence: f32,
    /// Queries executed
    pub queries_executed: u64,
}

/// Causal reasoning intelligence
pub struct CausalReasoningIntelligence {
    /// Causal graph
    graph: CausalGraph,
    /// CQL engine
    cql: CqlEngine,
    /// Event counter
    event_counter: AtomicU64,
}

impl CausalReasoningIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            graph: CausalGraph::new(),
            cql: CqlEngine::new(),
            event_counter: AtomicU64::new(0),
        }
    }

    /// Record event
    pub fn record_event(&mut self, event_type: CausalEventType, timestamp: u64) -> CausalEventId {
        let id = CausalEventId(self.event_counter.fetch_add(1, Ordering::Relaxed));
        let event = CausalEvent::new(id, event_type, timestamp);
        self.graph.add_event(event);
        id
    }

    /// Record event with description
    pub fn record_event_with_description(
        &mut self,
        event_type: CausalEventType,
        timestamp: u64,
        description: String,
    ) -> CausalEventId {
        let id = CausalEventId(self.event_counter.fetch_add(1, Ordering::Relaxed));
        let event = CausalEvent::new(id, event_type, timestamp).with_description(description);
        self.graph.add_event(event);
        id
    }

    /// Add causality
    pub fn add_causality(
        &mut self,
        cause: CausalEventId,
        effect: CausalEventId,
        relation: CausalRelationType,
        confidence: f32,
    ) -> bool {
        if let (Some(cause_node), Some(effect_node)) = (
            self.graph.get_node_by_event(cause),
            self.graph.get_node_by_event(effect),
        ) {
            let cause_id = cause_node.id;
            let effect_id = effect_node.id;
            self.graph
                .add_relationship(cause_id, effect_id, relation, confidence);
            return true;
        }
        false
    }

    /// Query why
    pub fn query_why(&self, event: CausalEventId) -> CqlResult {
        self.cql.execute(&self.graph, CqlQuery::WhyQuery { event })
    }

    /// Query root causes
    pub fn query_root_causes(&self, event: CausalEventId) -> CqlResult {
        self.cql
            .execute(&self.graph, CqlQuery::RootCausesQuery { event })
    }

    /// Query effects
    pub fn query_effects(&self, event: CausalEventId) -> CqlResult {
        self.cql
            .execute(&self.graph, CqlQuery::EffectsQuery { event })
    }

    /// Query counterfactual
    pub fn query_counterfactual(&self, event: CausalEventId) -> CqlResult {
        self.cql
            .execute(&self.graph, CqlQuery::CounterfactualQuery {
                event,
                modification: super::CounterfactualModification::Remove,
            })
    }

    /// Execute CQL query
    pub fn query(&self, query: CqlQuery) -> CqlResult {
        self.cql.execute(&self.graph, query)
    }

    /// Calculate depths
    pub fn calculate_depths(&mut self) {
        self.graph.calculate_depths();
    }

    /// Get analysis
    pub fn analyze(&self) -> CausalReasoningAnalysis {
        let mut total_confidence = 0.0f32;
        for edge in self.graph.edges_iter() {
            total_confidence += edge.confidence;
        }

        let avg_confidence = if self.graph.edge_count() > 0 {
            total_confidence / self.graph.edge_count() as f32
        } else {
            0.0
        };

        CausalReasoningAnalysis {
            total_events: self.graph.node_count() as u64,
            total_relationships: self.graph.edge_count() as u64,
            root_causes: self.graph.root_causes().len() as u64,
            terminal_effects: self.graph.terminal_effects().len() as u64,
            avg_chain_length: 0.0, // Would need to calculate
            avg_confidence,
            queries_executed: self.cql.query_count(),
        }
    }

    /// Get graph reference
    pub fn graph(&self) -> &CausalGraph {
        &self.graph
    }

    /// Get graph mutably
    pub fn graph_mut(&mut self) -> &mut CausalGraph {
        &mut self.graph
    }

    /// Get CQL engine
    pub fn cql(&self) -> &CqlEngine {
        &self.cql
    }

    /// Event count
    pub fn event_count(&self) -> u64 {
        self.event_counter.load(Ordering::Relaxed)
    }
}

impl Default for CausalReasoningIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
