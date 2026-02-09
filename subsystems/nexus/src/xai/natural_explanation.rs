//! # Natural Language Explanation Generator for NEXUS
//!
//! Year 2 "COGNITION" - Self-explanation engine that generates
//! human-readable explanations for AI decisions in natural language.
//!
//! ## Features
//!
//! - Template-based explanation generation
//! - Multi-level detail (summary, detailed, technical)
//! - Contrastive explanations
//! - Causal narratives
//! - Confidence-aware language

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Detail level for explanations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DetailLevel {
    /// Brief summary (1-2 sentences)
    Summary,
    /// Standard explanation (paragraph)
    Standard,
    /// Detailed technical explanation
    Detailed,
    /// Full trace with all reasoning steps
    Full,
}

/// Target audience for the explanation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Audience {
    /// Non-technical end user
    User,
    /// System administrator
    Admin,
    /// Developer
    Developer,
    /// Security auditor
    Auditor,
    /// Automated system
    Machine,
}

/// Explanation context
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ExplanationContext {
    /// Decision being explained
    pub decision_type: DecisionType,
    /// Detail level requested
    pub detail: DetailLevel,
    /// Target audience
    pub audience: Audience,
    /// Include confidence information
    pub include_confidence: bool,
    /// Include alternatives considered
    pub include_alternatives: bool,
    /// Include causal chain
    pub include_causality: bool,
    /// Maximum length (characters)
    pub max_length: Option<usize>,
}

impl Default for ExplanationContext {
    fn default() -> Self {
        Self {
            decision_type: DecisionType::Generic,
            detail: DetailLevel::Standard,
            audience: Audience::User,
            include_confidence: true,
            include_alternatives: false,
            include_causality: false,
            max_length: None,
        }
    }
}

/// Type of decision being explained
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionType {
    /// Process scheduling decision
    Scheduling,
    /// Memory allocation decision
    MemoryAllocation,
    /// Security/access control decision
    Security,
    /// Anomaly detection
    AnomalyDetection,
    /// Resource throttling
    ResourceThrottling,
    /// Process termination
    ProcessTermination,
    /// Device driver selection
    DeviceSelection,
    /// Power management
    PowerManagement,
    /// Network routing
    NetworkRouting,
    /// Generic decision
    Generic,
}

// ============================================================================
// EXPLANATION COMPONENTS
// ============================================================================

/// A factor that influenced the decision
#[derive(Debug, Clone)]
pub struct Factor {
    /// Factor name
    pub name: String,
    /// Factor value
    pub value: f64,
    /// Contribution to decision (positive = supports, negative = opposes)
    pub contribution: f64,
    /// Human-readable description
    pub description: String,
    /// Is this a primary factor?
    pub primary: bool,
}

impl Factor {
    /// Create a new factor
    pub fn new(name: String, value: f64, contribution: f64) -> Self {
        Self {
            name,
            value,
            contribution,
            description: String::new(),
            primary: contribution.abs() > 0.3,
        }
    }

    /// Set description
    #[inline(always)]
    pub fn with_description(mut self, desc: String) -> Self {
        self.description = desc;
        self
    }
}

/// Alternative action that was considered
#[derive(Debug, Clone)]
pub struct Alternative {
    /// Action name
    pub name: String,
    /// Why it wasn't chosen
    pub rejection_reason: String,
    /// Score compared to chosen action
    pub relative_score: f64,
}

/// Causal step in the reasoning chain
#[derive(Debug, Clone)]
pub struct CausalStep {
    /// Step description
    pub description: String,
    /// Confidence in this step
    pub confidence: f64,
    /// Evidence for this step
    pub evidence: Vec<String>,
}

/// Complete decision data for explanation
#[derive(Debug, Clone)]
pub struct DecisionData {
    /// Action taken
    pub action: String,
    /// Overall confidence
    pub confidence: f64,
    /// Factors that influenced the decision
    pub factors: Vec<Factor>,
    /// Alternatives considered
    pub alternatives: Vec<Alternative>,
    /// Causal reasoning steps
    pub causal_steps: Vec<CausalStep>,
    /// Additional context
    pub metadata: BTreeMap<String, String>,
}

impl DecisionData {
    /// Create new decision data
    pub fn new(action: String, confidence: f64) -> Self {
        Self {
            action,
            confidence,
            factors: Vec::new(),
            alternatives: Vec::new(),
            causal_steps: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Add a factor
    #[inline(always)]
    pub fn add_factor(&mut self, factor: Factor) {
        self.factors.push(factor);
    }

    /// Add an alternative
    #[inline(always)]
    pub fn add_alternative(&mut self, alt: Alternative) {
        self.alternatives.push(alt);
    }

    /// Add causal step
    #[inline(always)]
    pub fn add_causal_step(&mut self, step: CausalStep) {
        self.causal_steps.push(step);
    }

    /// Get primary factors
    #[inline(always)]
    pub fn primary_factors(&self) -> Vec<&Factor> {
        self.factors.iter().filter(|f| f.primary).collect()
    }

    /// Get top N factors by contribution
    #[inline]
    pub fn top_factors(&self, n: usize) -> Vec<&Factor> {
        let mut sorted: Vec<&Factor> = self.factors.iter().collect();
        sorted.sort_by(|a, b| {
            b.contribution
                .abs()
                .partial_cmp(&a.contribution.abs())
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        sorted.into_iter().take(n).collect()
    }
}

// ============================================================================
// EXPLANATION TEMPLATES
// ============================================================================

/// Template for generating explanations
#[derive(Debug, Clone)]
pub struct ExplanationTemplate {
    /// Template name
    pub name: String,
    /// Template patterns for different detail levels
    patterns: BTreeMap<DetailLevel, String>,
    /// Phrase templates
    phrases: PhraseTemplates,
}

/// Common phrase templates
#[derive(Debug, Clone, Default)]
struct PhraseTemplates {
    /// High confidence phrases
    high_confidence: Vec<String>,
    /// Medium confidence phrases
    medium_confidence: Vec<String>,
    /// Low confidence phrases
    low_confidence: Vec<String>,
    /// Causal connectors
    causal_connectors: Vec<String>,
    /// Contrast connectors
    contrast_connectors: Vec<String>,
}

impl ExplanationTemplate {
    /// Create a new template
    pub fn new(name: String) -> Self {
        let mut patterns = BTreeMap::new();
        patterns.insert(
            DetailLevel::Summary,
            String::from("The system decided to {action} because {primary_reason}."),
        );
        patterns.insert(
            DetailLevel::Standard,
            String::from(
                "The system decided to {action}. {reasoning} {confidence_statement}",
            ),
        );
        patterns.insert(
            DetailLevel::Detailed,
            String::from(
                "Decision: {action}\n\nReasoning:\n{detailed_reasoning}\n\n{alternatives_section}{confidence_statement}",
            ),
        );

        Self {
            name,
            patterns,
            phrases: PhraseTemplates::default_phrases(),
        }
    }

    /// Get pattern for detail level
    #[inline(always)]
    pub fn get_pattern(&self, level: DetailLevel) -> Option<&String> {
        self.patterns.get(&level)
    }

    /// Get confidence phrase
    pub fn confidence_phrase(&self, confidence: f64) -> &str {
        if confidence >= 0.9 {
            self.phrases
                .high_confidence
                .first()
                .map(|s| s.as_str())
                .unwrap_or("with high confidence")
        } else if confidence >= 0.7 {
            self.phrases
                .medium_confidence
                .first()
                .map(|s| s.as_str())
                .unwrap_or("with moderate confidence")
        } else {
            self.phrases
                .low_confidence
                .first()
                .map(|s| s.as_str())
                .unwrap_or("with some uncertainty")
        }
    }

    /// Get causal connector
    #[inline]
    pub fn causal_connector(&self, index: usize) -> &str {
        self.phrases
            .causal_connectors
            .get(index % self.phrases.causal_connectors.len().max(1))
            .map(|s| s.as_str())
            .unwrap_or("because")
    }
}

impl PhraseTemplates {
    fn default_phrases() -> Self {
        Self {
            high_confidence: vec![
                String::from("with high confidence"),
                String::from("with certainty"),
                String::from("confidently"),
            ],
            medium_confidence: vec![
                String::from("with moderate confidence"),
                String::from("reasonably confident"),
                String::from("with some confidence"),
            ],
            low_confidence: vec![
                String::from("with some uncertainty"),
                String::from("tentatively"),
                String::from("with low confidence"),
            ],
            causal_connectors: vec![
                String::from("because"),
                String::from("since"),
                String::from("as"),
                String::from("due to"),
                String::from("given that"),
            ],
            contrast_connectors: vec![
                String::from("however"),
                String::from("on the other hand"),
                String::from("in contrast"),
                String::from("alternatively"),
            ],
        }
    }
}

// ============================================================================
// EXPLANATION GENERATOR
// ============================================================================

/// Natural language explanation generator
pub struct NaturalExplanationGenerator {
    /// Templates for different decision types
    templates: BTreeMap<DecisionType, ExplanationTemplate>,
    /// Default template
    default_template: ExplanationTemplate,
}

impl NaturalExplanationGenerator {
    /// Create a new generator
    pub fn new() -> Self {
        let mut templates = BTreeMap::new();

        // Scheduling template
        let mut sched_template = ExplanationTemplate::new(String::from("scheduling"));
        sched_template.patterns.insert(
            DetailLevel::Summary,
            String::from("Process {process} was {action} because {primary_reason}."),
        );
        sched_template.patterns.insert(
            DetailLevel::Standard,
            String::from(
                "The scheduler {action} process {process}. {reasoning} This decision was made {confidence_phrase}.",
            ),
        );
        templates.insert(DecisionType::Scheduling, sched_template);

        // Security template
        let mut sec_template = ExplanationTemplate::new(String::from("security"));
        sec_template.patterns.insert(
            DetailLevel::Summary,
            String::from("Access was {action} because {primary_reason}."),
        );
        sec_template.patterns.insert(
            DetailLevel::Standard,
            String::from(
                "Security decision: {action}. {reasoning} {confidence_statement}",
            ),
        );
        templates.insert(DecisionType::Security, sec_template);

        // Anomaly template
        let mut anom_template = ExplanationTemplate::new(String::from("anomaly"));
        anom_template.patterns.insert(
            DetailLevel::Summary,
            String::from("Anomaly detected: {action}. Primary indicator: {primary_reason}."),
        );
        templates.insert(DecisionType::AnomalyDetection, anom_template);

        Self {
            templates,
            default_template: ExplanationTemplate::new(String::from("default")),
        }
    }

    /// Generate explanation
    pub fn generate(
        &self,
        data: &DecisionData,
        context: &ExplanationContext,
    ) -> String {
        let template = self
            .templates
            .get(&context.decision_type)
            .unwrap_or(&self.default_template);

        match context.detail {
            DetailLevel::Summary => self.generate_summary(data, template, context),
            DetailLevel::Standard => self.generate_standard(data, template, context),
            DetailLevel::Detailed => self.generate_detailed(data, template, context),
            DetailLevel::Full => self.generate_full(data, template, context),
        }
    }

    /// Generate summary explanation
    fn generate_summary(
        &self,
        data: &DecisionData,
        template: &ExplanationTemplate,
        _context: &ExplanationContext,
    ) -> String {
        let primary_factors = data.primary_factors();
        let primary_reason = if !primary_factors.is_empty() {
            self.format_factor(primary_factors[0])
        } else {
            String::from("of system requirements")
        };

        format!("The system decided to {} because {}.", data.action, primary_reason)
    }

    /// Generate standard explanation
    fn generate_standard(
        &self,
        data: &DecisionData,
        template: &ExplanationTemplate,
        context: &ExplanationContext,
    ) -> String {
        let mut parts = Vec::new();

        // Action statement
        parts.push(format!("The system decided to {}.", data.action));

        // Primary reasoning
        let top_factors = data.top_factors(3);
        if !top_factors.is_empty() {
            let reasons: Vec<String> = top_factors
                .iter()
                .map(|f| self.format_factor(f))
                .collect();

            parts.push(self.join_reasons(&reasons));
        }

        // Confidence
        if context.include_confidence {
            let conf_phrase = template.confidence_phrase(data.confidence);
            parts.push(format!("This decision was made {}.", conf_phrase));
        }

        parts.join(" ")
    }

    /// Generate detailed explanation
    fn generate_detailed(
        &self,
        data: &DecisionData,
        template: &ExplanationTemplate,
        context: &ExplanationContext,
    ) -> String {
        let mut sections = Vec::new();

        // Header
        sections.push(format!("Decision: {}\n", data.action));

        // Reasoning section
        sections.push(String::from("Reasoning:"));
        for factor in data.factors.iter() {
            let direction = if factor.contribution > 0.0 {
                "supports"
            } else {
                "opposes"
            };
            sections.push(format!(
                "  • {} ({} decision, weight: {:.1}%)",
                self.format_factor(factor),
                direction,
                factor.contribution.abs() * 100.0
            ));
        }

        // Alternatives section
        if context.include_alternatives && !data.alternatives.is_empty() {
            sections.push(String::from("\nAlternatives considered:"));
            for alt in &data.alternatives {
                sections.push(format!(
                    "  • {} - rejected because {}",
                    alt.name, alt.rejection_reason
                ));
            }
        }

        // Causal chain
        if context.include_causality && !data.causal_steps.is_empty() {
            sections.push(String::from("\nCausal chain:"));
            for (i, step) in data.causal_steps.iter().enumerate() {
                sections.push(format!(
                    "  {}. {} (confidence: {:.0}%)",
                    i + 1,
                    step.description,
                    step.confidence * 100.0
                ));
            }
        }

        // Confidence
        if context.include_confidence {
            sections.push(format!(
                "\nConfidence: {:.1}% ({})",
                data.confidence * 100.0,
                template.confidence_phrase(data.confidence)
            ));
        }

        sections.join("\n")
    }

    /// Generate full explanation
    fn generate_full(
        &self,
        data: &DecisionData,
        template: &ExplanationTemplate,
        context: &ExplanationContext,
    ) -> String {
        let mut sections = Vec::new();

        // Header with metadata
        sections.push(format!("═══ Decision Report ═══\n"));
        sections.push(format!("Action: {}", data.action));
        sections.push(format!(
            "Confidence: {:.2}%",
            data.confidence * 100.0
        ));

        // Metadata
        if !data.metadata.is_empty() {
            sections.push(String::from("\nContext:"));
            for (key, value) in &data.metadata {
                sections.push(format!("  {}: {}", key, value));
            }
        }

        // All factors with full details
        sections.push(String::from("\n─── Factor Analysis ───"));
        for factor in &data.factors {
            sections.push(format!("\n{}", factor.name));
            sections.push(format!("  Value: {:.4}", factor.value));
            sections.push(format!("  Contribution: {:+.4}", factor.contribution));
            if !factor.description.is_empty() {
                sections.push(format!("  Description: {}", factor.description));
            }
            sections.push(format!(
                "  Classification: {}",
                if factor.primary { "PRIMARY" } else { "secondary" }
            ));
        }

        // Alternatives
        if !data.alternatives.is_empty() {
            sections.push(String::from("\n─── Alternatives ───"));
            for alt in &data.alternatives {
                sections.push(format!("\n{}", alt.name));
                sections.push(format!("  Relative score: {:.4}", alt.relative_score));
                sections.push(format!("  Rejection: {}", alt.rejection_reason));
            }
        }

        // Full causal chain
        if !data.causal_steps.is_empty() {
            sections.push(String::from("\n─── Causal Reasoning ───"));
            for (i, step) in data.causal_steps.iter().enumerate() {
                sections.push(format!(
                    "\nStep {}: {} [conf: {:.1}%]",
                    i + 1,
                    step.description,
                    step.confidence * 100.0
                ));
                for evidence in &step.evidence {
                    sections.push(format!("  Evidence: {}", evidence));
                }
            }
        }

        sections.push(String::from("\n══════════════════════"));

        sections.join("\n")
    }

    /// Format a factor as natural language
    fn format_factor(&self, factor: &Factor) -> String {
        if !factor.description.is_empty() {
            factor.description.clone()
        } else {
            format!("{} was {:.2}", factor.name, factor.value)
        }
    }

    /// Join reasons with appropriate connectors
    fn join_reasons(&self, reasons: &[String]) -> String {
        match reasons.len() {
            0 => String::new(),
            1 => format!("This is because {}.", reasons[0]),
            2 => format!("This is because {} and {}.", reasons[0], reasons[1]),
            _ => {
                let last = reasons.last().unwrap();
                let init: Vec<&str> = reasons[..reasons.len() - 1].iter().map(|s| s.as_str()).collect();
                format!("This is because {}, and {}.", init.join(", "), last)
            }
        }
    }

    /// Generate contrastive explanation
    pub fn generate_contrastive(
        &self,
        data: &DecisionData,
        alternative: &str,
        context: &ExplanationContext,
    ) -> String {
        let top_factors = data.top_factors(2);

        if top_factors.is_empty() {
            return format!(
                "The system chose {} instead of {} based on overall assessment.",
                data.action, alternative
            );
        }

        let reasons: Vec<String> = top_factors
            .iter()
            .map(|f| self.format_factor(f))
            .collect();

        format!(
            "The system chose {} instead of {}. The key difference is that {}.",
            data.action,
            alternative,
            reasons.join(" and ")
        )
    }

    /// Summarize decision for logging
    pub fn summarize_for_log(&self, data: &DecisionData) -> String {
        let primary = data
            .primary_factors()
            .first()
            .map(|f| f.name.as_str())
            .unwrap_or("unknown");

        format!(
            "[{}] action={} confidence={:.2} primary_factor={}",
            match data.confidence {
                c if c >= 0.9 => "HIGH",
                c if c >= 0.7 => "MED",
                _ => "LOW",
            },
            data.action,
            data.confidence,
            primary
        )
    }
}

impl Default for NaturalExplanationGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// KERNEL-SPECIFIC EXPLAINER
// ============================================================================

/// Kernel-specific explanation generator
pub struct KernelExplainer {
    /// Natural language generator
    generator: NaturalExplanationGenerator,
}

impl KernelExplainer {
    /// Create new kernel explainer
    pub fn new() -> Self {
        Self {
            generator: NaturalExplanationGenerator::new(),
        }
    }

    /// Explain scheduling decision
    pub fn explain_scheduling(
        &self,
        process_name: &str,
        action: &str,
        priority: i32,
        cpu_usage: f64,
        wait_time: u64,
    ) -> String {
        let mut data = DecisionData::new(format!("{} {}", action, process_name), 0.85);

        data.add_factor(
            Factor::new(String::from("priority"), priority as f64, 0.4)
                .with_description(format!("process priority is {}", priority)),
        );

        data.add_factor(
            Factor::new(String::from("cpu_usage"), cpu_usage, -0.2)
                .with_description(format!("CPU usage is {:.1}%", cpu_usage * 100.0)),
        );

        data.add_factor(
            Factor::new(String::from("wait_time"), wait_time as f64, 0.3)
                .with_description(format!("process waited {}ms", wait_time)),
        );

        let context = ExplanationContext {
            decision_type: DecisionType::Scheduling,
            detail: DetailLevel::Standard,
            ..Default::default()
        };

        self.generator.generate(&data, &context)
    }

    /// Explain security decision
    pub fn explain_security_decision(
        &self,
        action: &str,
        resource: &str,
        risk_score: f64,
        user_id: u32,
    ) -> String {
        let confidence = if risk_score > 0.8 { 0.95 } else { 0.75 };
        let mut data = DecisionData::new(action.to_string(), confidence);

        data.add_factor(
            Factor::new(String::from("risk_score"), risk_score, if risk_score > 0.5 { 0.6 } else { -0.3 })
                .with_description(format!(
                    "risk score for accessing {} is {:.0}%",
                    resource,
                    risk_score * 100.0
                )),
        );

        data.add_factor(
            Factor::new(String::from("user_privileges"), user_id as f64, 0.3)
                .with_description(format!("user {} has appropriate privileges", user_id)),
        );

        let context = ExplanationContext {
            decision_type: DecisionType::Security,
            detail: DetailLevel::Standard,
            include_confidence: true,
            ..Default::default()
        };

        self.generator.generate(&data, &context)
    }

    /// Explain anomaly detection
    pub fn explain_anomaly(
        &self,
        anomaly_type: &str,
        severity: f64,
        indicators: &[(String, f64)],
    ) -> String {
        let mut data = DecisionData::new(
            format!("flagged {} anomaly", anomaly_type),
            severity,
        );

        for (indicator, value) in indicators {
            data.add_factor(
                Factor::new(indicator.clone(), *value, value.abs())
                    .with_description(format!("{} deviation: {:.2}σ", indicator, value)),
            );
        }

        let context = ExplanationContext {
            decision_type: DecisionType::AnomalyDetection,
            detail: DetailLevel::Detailed,
            include_confidence: true,
            ..Default::default()
        };

        self.generator.generate(&data, &context)
    }

    /// Explain memory allocation decision
    pub fn explain_memory_allocation(
        &self,
        action: &str,
        requested_size: usize,
        available_memory: usize,
        fragmentation: f64,
    ) -> String {
        let mut data = DecisionData::new(action.to_string(), 0.9);

        data.add_factor(
            Factor::new(String::from("requested_size"), requested_size as f64, 0.3)
                .with_description(format!("{} bytes requested", requested_size)),
        );

        data.add_factor(
            Factor::new(String::from("available"), available_memory as f64, 0.4)
                .with_description(format!(
                    "{} bytes available ({}%)",
                    available_memory,
                    (available_memory as f64 / (requested_size as f64 + available_memory as f64)) * 100.0
                )),
        );

        if fragmentation > 0.3 {
            data.add_factor(
                Factor::new(String::from("fragmentation"), fragmentation, -0.2)
                    .with_description(format!(
                        "memory fragmentation at {:.0}%",
                        fragmentation * 100.0
                    )),
            );
        }

        let context = ExplanationContext {
            decision_type: DecisionType::MemoryAllocation,
            detail: DetailLevel::Standard,
            ..Default::default()
        };

        self.generator.generate(&data, &context)
    }
}

impl Default for KernelExplainer {
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
    fn test_factor_creation() {
        let factor = Factor::new(String::from("cpu_usage"), 0.75, 0.4)
            .with_description(String::from("CPU is at 75%"));

        assert_eq!(factor.name, "cpu_usage");
        assert!(factor.primary); // 0.4 > 0.3
    }

    #[test]
    fn test_decision_data() {
        let mut data = DecisionData::new(String::from("schedule process"), 0.85);

        data.add_factor(Factor::new(String::from("priority"), 5.0, 0.5));
        data.add_factor(Factor::new(String::from("wait_time"), 100.0, 0.3));
        data.add_factor(Factor::new(String::from("cpu"), 0.2, -0.1));

        let primary = data.primary_factors();
        assert_eq!(primary.len(), 2); // priority and wait_time

        let top = data.top_factors(2);
        assert_eq!(top[0].name, "priority");
    }

    #[test]
    fn test_generator_summary() {
        let generator = NaturalExplanationGenerator::new();

        let mut data = DecisionData::new(String::from("terminate process"), 0.9);
        data.add_factor(
            Factor::new(String::from("memory_leak"), 1.0, 0.8)
                .with_description(String::from("memory usage exceeded threshold")),
        );

        let context = ExplanationContext {
            detail: DetailLevel::Summary,
            ..Default::default()
        };

        let explanation = generator.generate(&data, &context);
        assert!(explanation.contains("terminate process"));
        assert!(explanation.contains("memory usage exceeded threshold"));
    }

    #[test]
    fn test_kernel_explainer_scheduling() {
        let explainer = KernelExplainer::new();

        let explanation = explainer.explain_scheduling(
            "my_app",
            "scheduled",
            10,
            0.75,
            50,
        );

        assert!(explanation.contains("scheduled"));
        assert!(explanation.contains("priority"));
    }

    #[test]
    fn test_contrastive_explanation() {
        let generator = NaturalExplanationGenerator::new();

        let mut data = DecisionData::new(String::from("use FIFO scheduler"), 0.8);
        data.add_factor(
            Factor::new(String::from("load"), 0.3, 0.5)
                .with_description(String::from("system load is low")),
        );

        let explanation = generator.generate_contrastive(
            &data,
            "CFS scheduler",
            &ExplanationContext::default(),
        );

        assert!(explanation.contains("FIFO"));
        assert!(explanation.contains("CFS"));
        assert!(explanation.contains("instead of"));
    }

    #[test]
    fn test_log_summary() {
        let generator = NaturalExplanationGenerator::new();

        let mut data = DecisionData::new(String::from("block_access"), 0.95);
        data.add_factor(Factor::new(String::from("risk"), 0.9, 0.8));

        let log = generator.summarize_for_log(&data);
        assert!(log.contains("[HIGH]"));
        assert!(log.contains("block_access"));
        assert!(log.contains("risk"));
    }
}
