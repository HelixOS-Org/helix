//! Explanation Generator
//!
//! This module provides human-readable explanations for causal events.

use alloc::string::String;
use alloc::vec::Vec;

use super::{CausalEventId, ChainId, CausalGraph, CausalChainBuilder};

/// Explanation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationType {
    /// Why did X happen?
    WhyDid,
    /// What caused X?
    WhatCaused,
    /// What would happen if X?
    WhatIf,
    /// How to prevent X?
    HowToPrevent,
    /// What are the consequences of X?
    Consequences,
}

impl ExplanationType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::WhyDid => "why_did",
            Self::WhatCaused => "what_caused",
            Self::WhatIf => "what_if",
            Self::HowToPrevent => "how_to_prevent",
            Self::Consequences => "consequences",
        }
    }
}

/// Explanation
#[derive(Debug, Clone)]
pub struct Explanation {
    /// Explanation type
    pub explanation_type: ExplanationType,
    /// Subject event
    pub subject: CausalEventId,
    /// Summary (one sentence)
    pub summary: String,
    /// Detailed explanation
    pub details: Vec<String>,
    /// Related events
    pub related_events: Vec<CausalEventId>,
    /// Causal chain
    pub chain: Option<ChainId>,
    /// Confidence
    pub confidence: f32,
}

impl Explanation {
    /// Create new explanation
    pub fn new(explanation_type: ExplanationType, subject: CausalEventId) -> Self {
        Self {
            explanation_type,
            subject,
            summary: String::new(),
            details: Vec::new(),
            related_events: Vec::new(),
            chain: None,
            confidence: 0.5,
        }
    }

    /// With summary
    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = summary;
        self
    }

    /// With confidence
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    /// Add detail
    pub fn add_detail(&mut self, detail: String) {
        self.details.push(detail);
    }

    /// Add related event
    pub fn add_related(&mut self, event: CausalEventId) {
        if !self.related_events.contains(&event) {
            self.related_events.push(event);
        }
    }

    /// Set chain
    pub fn set_chain(&mut self, chain_id: ChainId) {
        self.chain = Some(chain_id);
    }
}

/// Explanation generator
pub struct ExplanationGenerator {
    /// Chain builder
    chain_builder: CausalChainBuilder,
}

impl ExplanationGenerator {
    /// Create new generator
    pub fn new() -> Self {
        Self {
            chain_builder: CausalChainBuilder::new(),
        }
    }

    /// Generate "why did X happen" explanation
    pub fn explain_why(
        &self,
        graph: &CausalGraph,
        event_id: CausalEventId,
    ) -> Option<Explanation> {
        let node = graph.get_node_by_event(event_id)?;
        let mut explanation = Explanation::new(ExplanationType::WhyDid, event_id);

        // Find root causes
        let roots = graph.find_root_causes(node.id);

        if roots.is_empty() {
            explanation.summary = alloc::format!(
                "Event '{}' has no identified causes (it may be a root cause itself)",
                node.event.event_type.name()
            );
            return Some(explanation);
        }

        // Build chain from first root cause
        if let Some(root_id) = roots.first() {
            if let Some(chain) = self.chain_builder.build_chain(graph, *root_id, node.id) {
                explanation.set_chain(chain.id);

                // Build summary
                if let Some(root_node) = graph.get_node(*root_id) {
                    explanation.summary = alloc::format!(
                        "'{}' occurred because '{}' triggered a chain of {} events",
                        node.event.event_type.name(),
                        root_node.event.event_type.name(),
                        chain.len()
                    );
                }

                // Add details for each step
                for (i, &step_id) in chain.nodes.iter().enumerate() {
                    if let Some(step_node) = graph.get_node(step_id) {
                        explanation.add_detail(alloc::format!(
                            "Step {}: {} - {}",
                            i + 1,
                            step_node.event.event_type.name(),
                            step_node.event.description
                        ));
                        explanation.add_related(step_node.event.id);
                    }
                }

                explanation.confidence = chain.total_confidence;
            }
        }

        Some(explanation)
    }

    /// Generate "what caused X" explanation
    pub fn explain_causes(
        &self,
        graph: &CausalGraph,
        event_id: CausalEventId,
    ) -> Option<Explanation> {
        let node = graph.get_node_by_event(event_id)?;
        let mut explanation = Explanation::new(ExplanationType::WhatCaused, event_id);

        // Get direct causes
        let direct_causes: Vec<_> = node.causes.iter()
            .filter_map(|&edge_id| graph.get_edge(edge_id))
            .filter_map(|edge| graph.get_node(edge.source))
            .collect();

        if direct_causes.is_empty() {
            explanation.summary = alloc::format!(
                "'{}' has no identified direct causes",
                node.event.event_type.name()
            );
        } else {
            explanation.summary = alloc::format!(
                "'{}' was directly caused by {} event(s)",
                node.event.event_type.name(),
                direct_causes.len()
            );

            for cause in direct_causes {
                explanation.add_detail(alloc::format!(
                    "Direct cause: {} - {}",
                    cause.event.event_type.name(),
                    cause.event.description
                ));
                explanation.add_related(cause.event.id);
            }
        }

        explanation.confidence = 0.85;
        Some(explanation)
    }

    /// Generate "consequences of X" explanation
    pub fn explain_consequences(
        &self,
        graph: &CausalGraph,
        event_id: CausalEventId,
    ) -> Option<Explanation> {
        let node = graph.get_node_by_event(event_id)?;
        let mut explanation = Explanation::new(ExplanationType::Consequences, event_id);

        let effects = graph.find_effects(node.id);

        if effects.is_empty() {
            explanation.summary = alloc::format!(
                "'{}' had no observable consequences",
                node.event.event_type.name()
            );
        } else {
            let error_count = effects.iter()
                .filter_map(|&id| graph.get_node(id))
                .filter(|n| n.event.event_type.is_error())
                .count();

            explanation.summary = alloc::format!(
                "'{}' led to {} consequence(s), including {} error(s)",
                node.event.event_type.name(),
                effects.len(),
                error_count
            );

            for effect_id in effects {
                if let Some(effect_node) = graph.get_node(effect_id) {
                    explanation.add_detail(alloc::format!(
                        "Consequence: {} - {}",
                        effect_node.event.event_type.name(),
                        effect_node.event.description
                    ));
                    explanation.add_related(effect_node.event.id);
                }
            }
        }

        explanation.confidence = 0.8;
        Some(explanation)
    }
}

impl Default for ExplanationGenerator {
    fn default() -> Self {
        Self::new()
    }
}
