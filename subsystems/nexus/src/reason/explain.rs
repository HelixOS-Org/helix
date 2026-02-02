//! # Explanation Generation
//!
//! Generates human-readable explanations for decisions,
//! predictions, and causal relationships.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// EXPLANATION TYPES
// ============================================================================

/// Explanation
#[derive(Debug, Clone)]
pub struct Explanation {
    /// Explanation ID
    pub id: u64,
    /// Target (what is being explained)
    pub target: ExplanationTarget,
    /// Explanation type
    pub explanation_type: ExplanationType,
    /// Components
    pub components: Vec<ExplanationComponent>,
    /// Summary (one-line)
    pub summary: String,
    /// Full text
    pub full_text: String,
    /// Confidence
    pub confidence: f64,
    /// Audience level
    pub audience: AudienceLevel,
    /// Created
    pub created: Timestamp,
}

/// What is being explained
#[derive(Debug, Clone)]
pub struct ExplanationTarget {
    /// Target type
    pub target_type: TargetType,
    /// Target ID
    pub target_id: u64,
    /// Target description
    pub description: String,
    /// Context
    pub context: BTreeMap<String, String>,
}

/// Target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    Decision,
    Prediction,
    Action,
    Error,
    Behavior,
    Anomaly,
    Recommendation,
    Classification,
}

/// Explanation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplanationType {
    /// Why did this happen?
    Causal,
    /// How did we reach this conclusion?
    Reasoning,
    /// What factors contributed?
    Contrastive,
    /// Why this and not that?
    Counterfactual,
    /// Step by step
    Trace,
    /// Example-based
    Exemplar,
    /// Feature importance
    Attribution,
}

/// Explanation component
#[derive(Debug, Clone)]
pub struct ExplanationComponent {
    /// Component type
    pub component_type: ComponentType,
    /// Content
    pub content: String,
    /// Importance (0-1)
    pub importance: f64,
    /// References
    pub references: Vec<Reference>,
}

/// Component type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    Cause,
    Effect,
    Evidence,
    Assumption,
    Constraint,
    Justification,
    Alternative,
    Uncertainty,
}

/// Reference
#[derive(Debug, Clone)]
pub struct Reference {
    /// Reference type
    pub ref_type: ReferenceType,
    /// ID
    pub id: u64,
    /// Description
    pub description: String,
}

/// Reference type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceType {
    Rule,
    Pattern,
    Example,
    Data,
    Knowledge,
    External,
}

/// Audience level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudienceLevel {
    Technical,
    Professional,
    General,
    Simple,
}

// ============================================================================
// EXPLANATION REQUEST
// ============================================================================

/// Request for explanation
#[derive(Debug, Clone)]
pub struct ExplanationRequest {
    /// Target
    pub target: ExplanationTarget,
    /// Requested type
    pub explanation_type: Option<ExplanationType>,
    /// Audience level
    pub audience: AudienceLevel,
    /// Maximum length
    pub max_length: Option<usize>,
    /// Include components
    pub include_components: bool,
    /// Specific question
    pub question: Option<String>,
}

// ============================================================================
// EXPLANATION GENERATOR
// ============================================================================

/// Explanation generator
pub struct ExplanationGenerator {
    /// Generated explanations
    explanations: BTreeMap<u64, Explanation>,
    /// Templates
    templates: BTreeMap<(ExplanationType, TargetType), ExplanationTemplate>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: GeneratorConfig,
    /// Statistics
    stats: GeneratorStats,
}

/// Explanation template
#[derive(Debug, Clone)]
pub struct ExplanationTemplate {
    /// Template for summary
    pub summary_template: String,
    /// Template for full explanation
    pub full_template: String,
    /// Component templates
    pub component_templates: BTreeMap<ComponentType, String>,
    /// Transition phrases
    pub transitions: Vec<String>,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Default max length
    pub default_max_length: usize,
    /// Include confidence intervals
    pub include_confidence: bool,
    /// Include alternatives
    pub include_alternatives: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            default_max_length: 500,
            include_confidence: true,
            include_alternatives: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct GeneratorStats {
    /// Explanations generated
    pub explanations_generated: u64,
    /// By type
    pub by_type: BTreeMap<ExplanationType, u64>,
    /// Average confidence
    pub avg_confidence: f64,
}

impl ExplanationGenerator {
    /// Create new generator
    pub fn new(config: GeneratorConfig) -> Self {
        let mut generator = Self {
            explanations: BTreeMap::new(),
            templates: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: GeneratorStats::default(),
        };
        generator.init_templates();
        generator
    }

    fn init_templates(&mut self) {
        // Causal explanation for decision
        self.templates.insert(
            (ExplanationType::Causal, TargetType::Decision),
            ExplanationTemplate {
                summary_template: "The decision was made because {main_cause}.".into(),
                full_template: "The decision to {action} was made because:\n\n{causes}\n\nThis was influenced by {factors}.".into(),
                component_templates: BTreeMap::new(),
                transitions: vec![
                    "As a result,".into(),
                    "Therefore,".into(),
                    "This led to".into(),
                ],
            },
        );

        // Contrastive explanation
        self.templates.insert(
            (ExplanationType::Contrastive, TargetType::Classification),
            ExplanationTemplate {
                summary_template: "This is {result} rather than {alternative} because {reason}.".into(),
                full_template: "The classification as {result} instead of {alternative} is based on:\n\n{differences}".into(),
                component_templates: BTreeMap::new(),
                transitions: vec![
                    "In contrast,".into(),
                    "Unlike".into(),
                    "Whereas".into(),
                ],
            },
        );
    }

    /// Generate explanation
    pub fn generate(&mut self, request: ExplanationRequest) -> Explanation {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let explanation_type = request
            .explanation_type
            .unwrap_or_else(|| self.select_best_type(&request.target));

        let components = self.build_components(&request, explanation_type);
        let (summary, full_text) = self.build_text(&request, &components, explanation_type);
        let confidence = self.calculate_confidence(&components);

        let explanation = Explanation {
            id,
            target: request.target,
            explanation_type,
            components,
            summary,
            full_text,
            confidence,
            audience: request.audience,
            created: Timestamp::now(),
        };

        // Update statistics
        self.stats.explanations_generated += 1;
        *self.stats.by_type.entry(explanation_type).or_insert(0) += 1;
        let n = self.stats.explanations_generated as f64;
        self.stats.avg_confidence = (self.stats.avg_confidence * (n - 1.0) + confidence) / n;

        self.explanations.insert(id, explanation.clone());
        explanation
    }

    fn select_best_type(&self, target: &ExplanationTarget) -> ExplanationType {
        match target.target_type {
            TargetType::Decision => ExplanationType::Causal,
            TargetType::Prediction => ExplanationType::Attribution,
            TargetType::Action => ExplanationType::Trace,
            TargetType::Error => ExplanationType::Causal,
            TargetType::Behavior => ExplanationType::Reasoning,
            TargetType::Anomaly => ExplanationType::Contrastive,
            TargetType::Recommendation => ExplanationType::Reasoning,
            TargetType::Classification => ExplanationType::Contrastive,
        }
    }

    fn build_components(
        &self,
        request: &ExplanationRequest,
        explanation_type: ExplanationType,
    ) -> Vec<ExplanationComponent> {
        let mut components = Vec::new();

        match explanation_type {
            ExplanationType::Causal => {
                components.push(ExplanationComponent {
                    component_type: ComponentType::Cause,
                    content: format!("Primary cause for {}", request.target.description),
                    importance: 1.0,
                    references: Vec::new(),
                });
            },
            ExplanationType::Contrastive => {
                components.push(ExplanationComponent {
                    component_type: ComponentType::Evidence,
                    content: "Key distinguishing features".into(),
                    importance: 0.9,
                    references: Vec::new(),
                });
                components.push(ExplanationComponent {
                    component_type: ComponentType::Alternative,
                    content: "Alternative outcomes considered".into(),
                    importance: 0.7,
                    references: Vec::new(),
                });
            },
            ExplanationType::Reasoning => {
                components.push(ExplanationComponent {
                    component_type: ComponentType::Justification,
                    content: "The reasoning process".into(),
                    importance: 0.9,
                    references: Vec::new(),
                });
            },
            ExplanationType::Attribution => {
                components.push(ExplanationComponent {
                    component_type: ComponentType::Evidence,
                    content: "Feature contributions".into(),
                    importance: 0.95,
                    references: Vec::new(),
                });
            },
            _ => {
                components.push(ExplanationComponent {
                    component_type: ComponentType::Justification,
                    content: "General explanation".into(),
                    importance: 0.8,
                    references: Vec::new(),
                });
            },
        }

        if self.config.include_alternatives {
            components.push(ExplanationComponent {
                component_type: ComponentType::Alternative,
                content: "Other possibilities were considered".into(),
                importance: 0.5,
                references: Vec::new(),
            });
        }

        components
    }

    fn build_text(
        &self,
        request: &ExplanationRequest,
        components: &[ExplanationComponent],
        explanation_type: ExplanationType,
    ) -> (String, String) {
        let max_len = request.max_length.unwrap_or(self.config.default_max_length);

        // Build summary
        let summary = match explanation_type {
            ExplanationType::Causal => {
                format!(
                    "The {} occurred due to {}.",
                    request.target.description,
                    components
                        .first()
                        .map(|c| c.content.as_str())
                        .unwrap_or("multiple factors")
                )
            },
            ExplanationType::Contrastive => {
                format!(
                    "The result was {} because of distinguishing characteristics.",
                    request.target.description
                )
            },
            ExplanationType::Reasoning => {
                format!(
                    "The reasoning for {} follows from the evidence.",
                    request.target.description
                )
            },
            _ => {
                format!("Explanation for: {}", request.target.description)
            },
        };

        // Build full text
        let mut full_text = String::new();
        full_text.push_str(&format!(
            "## Explanation: {}\n\n",
            request.target.description
        ));

        if request.include_components {
            for component in components {
                full_text.push_str(&format!(
                    "**{}**: {}\n\n",
                    format!("{:?}", component.component_type),
                    component.content
                ));
            }
        }

        // Adapt for audience
        let full_text = self.adapt_for_audience(&full_text, request.audience);

        // Truncate if needed
        let full_text = if full_text.len() > max_len {
            let mut truncated = full_text[..max_len].to_string();
            truncated.push_str("...");
            truncated
        } else {
            full_text
        };

        (summary, full_text)
    }

    fn adapt_for_audience(&self, text: &str, audience: AudienceLevel) -> String {
        match audience {
            AudienceLevel::Simple => {
                // Simplify language
                text.replace("occurred", "happened")
                    .replace("multiple factors", "several things")
                    .replace("distinguishing characteristics", "key differences")
            },
            AudienceLevel::General => {
                // Moderate simplification
                text.replace("multiple factors", "several reasons")
            },
            _ => text.to_string(),
        }
    }

    fn calculate_confidence(&self, components: &[ExplanationComponent]) -> f64 {
        if components.is_empty() {
            return 0.0;
        }

        let total_importance: f64 = components.iter().map(|c| c.importance).sum();
        total_importance / components.len() as f64
    }

    /// Get explanation
    pub fn get_explanation(&self, id: u64) -> Option<&Explanation> {
        self.explanations.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &GeneratorStats {
        &self.stats
    }
}

impl Default for ExplanationGenerator {
    fn default() -> Self {
        Self::new(GeneratorConfig::default())
    }
}

// ============================================================================
// EXPLANATION BUILDER
// ============================================================================

/// Explanation builder
pub struct ExplanationBuilder {
    target: Option<ExplanationTarget>,
    explanation_type: Option<ExplanationType>,
    components: Vec<ExplanationComponent>,
    audience: AudienceLevel,
    max_length: Option<usize>,
}

impl ExplanationBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            target: None,
            explanation_type: None,
            components: Vec::new(),
            audience: AudienceLevel::General,
            max_length: None,
        }
    }

    /// Set target
    pub fn target(mut self, target_type: TargetType, description: &str) -> Self {
        self.target = Some(ExplanationTarget {
            target_type,
            target_id: 0,
            description: description.into(),
            context: BTreeMap::new(),
        });
        self
    }

    /// Set explanation type
    pub fn explanation_type(mut self, exp_type: ExplanationType) -> Self {
        self.explanation_type = Some(exp_type);
        self
    }

    /// Add cause
    pub fn add_cause(mut self, cause: &str, importance: f64) -> Self {
        self.components.push(ExplanationComponent {
            component_type: ComponentType::Cause,
            content: cause.into(),
            importance,
            references: Vec::new(),
        });
        self
    }

    /// Add evidence
    pub fn add_evidence(mut self, evidence: &str, importance: f64) -> Self {
        self.components.push(ExplanationComponent {
            component_type: ComponentType::Evidence,
            content: evidence.into(),
            importance,
            references: Vec::new(),
        });
        self
    }

    /// Set audience
    pub fn audience(mut self, audience: AudienceLevel) -> Self {
        self.audience = audience;
        self
    }

    /// Build request
    pub fn build(self) -> Option<ExplanationRequest> {
        let target = self.target?;

        Some(ExplanationRequest {
            target,
            explanation_type: self.explanation_type,
            audience: self.audience,
            max_length: self.max_length,
            include_components: true,
            question: None,
        })
    }
}

impl Default for ExplanationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explanation_generation() {
        let mut generator = ExplanationGenerator::default();

        let request = ExplanationBuilder::new()
            .target(TargetType::Decision, "choosing option A")
            .explanation_type(ExplanationType::Causal)
            .add_cause("Primary factor X", 0.9)
            .build()
            .unwrap();

        let explanation = generator.generate(request);

        assert!(!explanation.summary.is_empty());
        assert!(!explanation.full_text.is_empty());
    }

    #[test]
    fn test_audience_adaptation() {
        let mut generator = ExplanationGenerator::default();

        let simple_request = ExplanationBuilder::new()
            .target(TargetType::Decision, "the result")
            .audience(AudienceLevel::Simple)
            .build()
            .unwrap();

        let explanation = generator.generate(simple_request);
        assert!(!explanation.full_text.is_empty());
    }

    #[test]
    fn test_statistics() {
        let mut generator = ExplanationGenerator::default();

        for _ in 0..5 {
            let request = ExplanationBuilder::new()
                .target(TargetType::Action, "test")
                .build()
                .unwrap();
            generator.generate(request);
        }

        assert_eq!(generator.stats().explanations_generated, 5);
    }
}
