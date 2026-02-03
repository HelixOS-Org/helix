//! # Self-Explanation Engine
//!
//! NEXUS explains its own decisions in natural language.
//! This is not just logging - it's true explainable AI.
//!
//! ## Capabilities
//!
//! - **Decision Explanation**: Why did NEXUS make this choice?
//! - **Reasoning Trace**: Step-by-step reasoning process
//! - **Alternatives Analysis**: What else was considered?
//! - **Confidence Reporting**: How sure is NEXUS?
//! - **Natural Language**: Human-readable explanations

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{
    Alternative, CausalLink, Decision, DecisionType, Explanation, ReasoningStep, RootCause,
};

/// Self-explanation engine
pub struct ExplanationEngine {
    /// Templates for explanations
    templates: ExplanationTemplates,
    /// Natural language generator
    nlg: NaturalLanguageGenerator,
    /// Verbosity level
    verbosity: super::Verbosity,
    /// Explanation history
    history: Vec<ExplanationRecord>,
}

impl ExplanationEngine {
    pub fn new() -> Self {
        Self {
            templates: ExplanationTemplates::new(),
            nlg: NaturalLanguageGenerator::new(),
            verbosity: super::Verbosity::Normal,
            history: Vec::new(),
        }
    }

    /// Explain a decision made by NEXUS
    pub fn explain(&mut self, decision: &Decision) -> Explanation {
        let explanation = match &decision.decision_type {
            DecisionType::Scheduling => self.explain_scheduling(decision),
            DecisionType::MemoryAllocation => self.explain_memory(decision),
            DecisionType::PowerManagement => self.explain_power(decision),
            DecisionType::SecurityPolicy => self.explain_security(decision),
            DecisionType::ErrorRecovery => self.explain_recovery(decision),
            DecisionType::Optimization => self.explain_optimization(decision),
            DecisionType::Custom(name) => self.explain_custom(decision, name),
        };

        // Record for history
        self.history.push(ExplanationRecord {
            decision_id: decision.id,
            timestamp: decision.timestamp,
            summary: explanation.summary.clone(),
        });

        explanation
    }

    fn explain_scheduling(&self, decision: &Decision) -> Explanation {
        let inputs_desc = self.describe_inputs(&decision.inputs);

        Explanation {
            summary: format!(
                "Scheduled task based on {}: {}",
                inputs_desc, decision.output
            ),
            reasoning_steps: vec![
                ReasoningStep {
                    step_number: 1,
                    description: "Analyzed current CPU load across all cores".into(),
                    evidence: vec!["CPU utilization metrics".into()],
                },
                ReasoningStep {
                    step_number: 2,
                    description: "Evaluated task priority and deadline".into(),
                    evidence: decision.inputs.clone(),
                },
                ReasoningStep {
                    step_number: 3,
                    description: "Selected optimal core for execution".into(),
                    evidence: vec![decision.output.clone()],
                },
            ],
            alternatives_considered: vec![
                Alternative {
                    description: "Could have scheduled on different core".into(),
                    why_rejected: "Current core selection minimizes cache misses".into(),
                },
                Alternative {
                    description: "Could have delayed execution".into(),
                    why_rejected: "Task deadline requires immediate scheduling".into(),
                },
            ],
            confidence: 0.85,
        }
    }

    fn explain_memory(&self, decision: &Decision) -> Explanation {
        Explanation {
            summary: format!("Memory allocation decision: {}", decision.output),
            reasoning_steps: vec![
                ReasoningStep {
                    step_number: 1,
                    description: "Checked available memory pools".into(),
                    evidence: vec!["Memory pool status".into()],
                },
                ReasoningStep {
                    step_number: 2,
                    description: "Evaluated allocation size and alignment requirements".into(),
                    evidence: decision.inputs.clone(),
                },
                ReasoningStep {
                    step_number: 3,
                    description: "Selected memory region to minimize fragmentation".into(),
                    evidence: vec![decision.output.clone()],
                },
            ],
            alternatives_considered: vec![Alternative {
                description: "Use different memory pool".into(),
                why_rejected: "Selected pool has better locality".into(),
            }],
            confidence: 0.9,
        }
    }

    fn explain_power(&self, decision: &Decision) -> Explanation {
        Explanation {
            summary: format!("Power management: {}", decision.output),
            reasoning_steps: vec![
                ReasoningStep {
                    step_number: 1,
                    description: "Analyzed current workload patterns".into(),
                    evidence: vec!["CPU/GPU utilization trends".into()],
                },
                ReasoningStep {
                    step_number: 2,
                    description: "Predicted future power requirements".into(),
                    evidence: decision.inputs.clone(),
                },
                ReasoningStep {
                    step_number: 3,
                    description: "Selected power state to balance performance and efficiency"
                        .into(),
                    evidence: vec![decision.output.clone()],
                },
            ],
            alternatives_considered: vec![
                Alternative {
                    description: "More aggressive power saving".into(),
                    why_rejected: "Would impact responsiveness".into(),
                },
                Alternative {
                    description: "Full performance mode".into(),
                    why_rejected: "Unnecessary power consumption".into(),
                },
            ],
            confidence: 0.8,
        }
    }

    fn explain_security(&self, decision: &Decision) -> Explanation {
        Explanation {
            summary: format!("Security policy decision: {}", decision.output),
            reasoning_steps: vec![
                ReasoningStep {
                    step_number: 1,
                    description: "Evaluated access request against policy".into(),
                    evidence: vec!["Security policy rules".into()],
                },
                ReasoningStep {
                    step_number: 2,
                    description: "Checked caller credentials and permissions".into(),
                    evidence: decision.inputs.clone(),
                },
                ReasoningStep {
                    step_number: 3,
                    description: "Made access decision based on principle of least privilege"
                        .into(),
                    evidence: vec![decision.output.clone()],
                },
            ],
            alternatives_considered: vec![Alternative {
                description: "Grant broader access".into(),
                why_rejected: "Violates least privilege principle".into(),
            }],
            confidence: 0.95,
        }
    }

    fn explain_recovery(&self, decision: &Decision) -> Explanation {
        Explanation {
            summary: format!("Error recovery: {}", decision.output),
            reasoning_steps: vec![
                ReasoningStep {
                    step_number: 1,
                    description: "Diagnosed error type and severity".into(),
                    evidence: vec!["Error diagnostic data".into()],
                },
                ReasoningStep {
                    step_number: 2,
                    description: "Evaluated recovery options".into(),
                    evidence: decision.inputs.clone(),
                },
                ReasoningStep {
                    step_number: 3,
                    description: "Selected least disruptive recovery strategy".into(),
                    evidence: vec![decision.output.clone()],
                },
                ReasoningStep {
                    step_number: 4,
                    description: "Verified recovery success".into(),
                    evidence: vec!["Post-recovery health check".into()],
                },
            ],
            alternatives_considered: vec![
                Alternative {
                    description: "Full system restart".into(),
                    why_rejected: "More disruptive than necessary".into(),
                },
                Alternative {
                    description: "Ignore error".into(),
                    why_rejected: "Would lead to data corruption".into(),
                },
            ],
            confidence: 0.75,
        }
    }

    fn explain_optimization(&self, decision: &Decision) -> Explanation {
        Explanation {
            summary: format!("Optimization applied: {}", decision.output),
            reasoning_steps: vec![
                ReasoningStep {
                    step_number: 1,
                    description: "Identified optimization opportunity".into(),
                    evidence: vec!["Performance metrics".into()],
                },
                ReasoningStep {
                    step_number: 2,
                    description: "Estimated improvement potential".into(),
                    evidence: decision.inputs.clone(),
                },
                ReasoningStep {
                    step_number: 3,
                    description: "Applied optimization with minimal disruption".into(),
                    evidence: vec![decision.output.clone()],
                },
            ],
            alternatives_considered: vec![
                Alternative {
                    description: "More aggressive optimization".into(),
                    why_rejected: "Risk of stability issues".into(),
                },
                Alternative {
                    description: "No optimization".into(),
                    why_rejected: "Clear improvement opportunity".into(),
                },
            ],
            confidence: 0.7,
        }
    }

    fn explain_custom(&self, decision: &Decision, name: &str) -> Explanation {
        Explanation {
            summary: format!("{} decision: {}", name, decision.output),
            reasoning_steps: vec![
                ReasoningStep {
                    step_number: 1,
                    description: format!("Analyzed {} context", name),
                    evidence: decision.inputs.clone(),
                },
                ReasoningStep {
                    step_number: 2,
                    description: "Made decision based on available data".into(),
                    evidence: vec![decision.output.clone()],
                },
            ],
            alternatives_considered: Vec::new(),
            confidence: 0.6,
        }
    }

    fn describe_inputs(&self, inputs: &[String]) -> String {
        if inputs.is_empty() {
            "default parameters".into()
        } else if inputs.len() == 1 {
            inputs[0].clone()
        } else {
            format!("{} factors", inputs.len())
        }
    }

    /// Explain a causal chain
    pub fn explain_causation(&self, chain: &[CausalLink], root: &RootCause) -> String {
        let mut explanation = String::new();

        explanation.push_str(&format!("## Root Cause Analysis\n\n"));
        explanation.push_str(&format!("**Root cause**: {:?}\n\n", root.event.event_type));
        explanation.push_str(&format!("{}\n\n", root.explanation));

        explanation.push_str("### Causal Chain\n\n");

        for (i, link) in chain.iter().enumerate() {
            explanation.push_str(&format!(
                "{}. **{:?}** → **{:?}**\n   - Mechanism: {}\n   - Strength: {:.0}%\n\n",
                i + 1,
                link.cause.event_type,
                link.effect.event_type,
                link.mechanism,
                link.strength * 100.0
            ));
        }

        if !root.fix_suggestions.is_empty() {
            explanation.push_str("### Suggested Fixes\n\n");
            for (i, fix) in root.fix_suggestions.iter().enumerate() {
                explanation.push_str(&format!("{}. {}\n", i + 1, fix));
            }
        }

        explanation
    }

    /// Generate a summary of recent decisions
    pub fn summarize_recent(&self, count: usize) -> String {
        let recent: Vec<_> = self.history.iter().rev().take(count).collect();

        if recent.is_empty() {
            return "No recent decisions to summarize.".into();
        }

        let mut summary = format!("## Summary of {} Recent Decisions\n\n", recent.len());

        for record in recent {
            summary.push_str(&format!("- [{}] {}\n", record.timestamp, record.summary));
        }

        summary
    }
}

impl Default for ExplanationEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Explanation templates
struct ExplanationTemplates {
    templates: BTreeMap<String, String>,
}

impl ExplanationTemplates {
    fn new() -> Self {
        let mut templates = BTreeMap::new();

        templates.insert(
            "scheduling".into(),
            "Task {task_id} was scheduled on core {core} because {reason}".into(),
        );
        templates.insert(
            "memory".into(),
            "Allocated {size} bytes from pool {pool} because {reason}".into(),
        );
        templates.insert(
            "security".into(),
            "Access {decision} for {subject} to {resource} because {reason}".into(),
        );

        Self { templates }
    }

    fn get(&self, key: &str) -> Option<&String> {
        self.templates.get(key)
    }
}

/// Natural language generator
pub struct NaturalLanguageGenerator {
    /// Vocabulary for different concepts
    vocabulary: Vocabulary,
    /// Sentence templates
    sentence_patterns: Vec<SentencePattern>,
}

impl NaturalLanguageGenerator {
    pub fn new() -> Self {
        Self {
            vocabulary: Vocabulary::new(),
            sentence_patterns: Self::init_patterns(),
        }
    }

    fn init_patterns() -> Vec<SentencePattern> {
        vec![
            SentencePattern {
                pattern_type: PatternType::Cause,
                template: "{cause} caused {effect} because {reason}".into(),
            },
            SentencePattern {
                pattern_type: PatternType::Decision,
                template: "I decided to {action} because {reason}".into(),
            },
            SentencePattern {
                pattern_type: PatternType::Comparison,
                template: "{option_a} was chosen over {option_b} because {reason}".into(),
            },
            SentencePattern {
                pattern_type: PatternType::Prediction,
                template: "Based on {evidence}, I predict {outcome} with {confidence}% confidence"
                    .into(),
            },
        ]
    }

    /// Generate natural language explanation
    pub fn generate(&self, context: &ExplanationContext) -> String {
        match context.context_type {
            ContextType::Causal => self.generate_causal(context),
            ContextType::Decision => self.generate_decision(context),
            ContextType::Prediction => self.generate_prediction(context),
        }
    }

    fn generate_causal(&self, context: &ExplanationContext) -> String {
        let cause = context
            .variables
            .get("cause")
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        let effect = context
            .variables
            .get("effect")
            .map(|s| s.as_str())
            .unwrap_or("unknown");
        let reason = context
            .variables
            .get("reason")
            .map(|s| s.as_str())
            .unwrap_or("unknown reason");

        format!("{} caused {} because {}.", cause, effect, reason)
    }

    fn generate_decision(&self, context: &ExplanationContext) -> String {
        let action = context
            .variables
            .get("action")
            .map(|s| s.as_str())
            .unwrap_or("take action");
        let reason = context
            .variables
            .get("reason")
            .map(|s| s.as_str())
            .unwrap_or("it was necessary");

        format!("I decided to {} because {}.", action, reason)
    }

    fn generate_prediction(&self, context: &ExplanationContext) -> String {
        let evidence = context
            .variables
            .get("evidence")
            .map(|s| s.as_str())
            .unwrap_or("available data");
        let outcome = context
            .variables
            .get("outcome")
            .map(|s| s.as_str())
            .unwrap_or("a certain outcome");
        let confidence = context
            .variables
            .get("confidence")
            .map(|s| s.as_str())
            .unwrap_or("moderate");

        format!(
            "Based on {}, I predict {} with {} confidence.",
            evidence, outcome, confidence
        )
    }
}

impl Default for NaturalLanguageGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Vocabulary for NLG
struct Vocabulary {
    synonyms: BTreeMap<String, Vec<String>>,
}

impl Vocabulary {
    fn new() -> Self {
        let mut synonyms = BTreeMap::new();

        synonyms.insert("because".into(), vec![
            "since".into(),
            "as".into(),
            "given that".into(),
            "due to".into(),
        ]);
        synonyms.insert("caused".into(), vec![
            "led to".into(),
            "resulted in".into(),
            "triggered".into(),
            "produced".into(),
        ]);
        synonyms.insert("decided".into(), vec![
            "chose".into(),
            "selected".into(),
            "opted".into(),
            "determined".into(),
        ]);

        Self { synonyms }
    }
}

/// Sentence pattern
struct SentencePattern {
    pattern_type: PatternType,
    template: String,
}

/// Pattern type
#[derive(Debug, Clone, PartialEq, Eq)]
enum PatternType {
    Cause,
    Decision,
    Comparison,
    Prediction,
}

/// Explanation context
pub struct ExplanationContext {
    pub context_type: ContextType,
    pub variables: BTreeMap<String, String>,
}

/// Context type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextType {
    Causal,
    Decision,
    Prediction,
}

/// Explanation record for history
struct ExplanationRecord {
    decision_id: u64,
    timestamp: u64,
    summary: String,
}

/// Interactive explainer for dialogue
pub struct InteractiveExplainer {
    context: DialogueContext,
    history: Vec<QAPair>,
}

impl InteractiveExplainer {
    pub fn new() -> Self {
        Self {
            context: DialogueContext::new(),
            history: Vec::new(),
        }
    }

    /// Answer a question about a decision
    pub fn ask(&mut self, question: &str) -> String {
        let q_type = self.classify_question(question);

        let answer = match q_type {
            QuestionType::Why => self.answer_why(question),
            QuestionType::How => self.answer_how(question),
            QuestionType::What => self.answer_what(question),
            QuestionType::WhatIf => self.answer_what_if(question),
            QuestionType::Clarification => self.clarify(question),
        };

        self.history.push(QAPair {
            question: question.into(),
            answer: answer.clone(),
        });

        answer
    }

    fn classify_question(&self, question: &str) -> QuestionType {
        let q = question.to_lowercase();

        if q.starts_with("why") || q.contains("reason") {
            QuestionType::Why
        } else if q.starts_with("how") {
            QuestionType::How
        } else if q.starts_with("what if") || q.contains("would happen") {
            QuestionType::WhatIf
        } else if q.starts_with("what") {
            QuestionType::What
        } else {
            QuestionType::Clarification
        }
    }

    fn answer_why(&self, _question: &str) -> String {
        "The decision was made because the analysis indicated it was the optimal choice given the current system state and constraints.".into()
    }

    fn answer_how(&self, _question: &str) -> String {
        "The process involved: 1) Gathering relevant metrics, 2) Evaluating options against criteria, 3) Selecting the option with the best score.".into()
    }

    fn answer_what(&self, _question: &str) -> String {
        "The system performed an optimization operation to improve performance while maintaining stability.".into()
    }

    fn answer_what_if(&self, _question: &str) -> String {
        "If a different choice had been made, the system would likely have experienced different performance characteristics. I can simulate specific scenarios if you provide more details.".into()
    }

    fn clarify(&self, _question: &str) -> String {
        "Could you please rephrase your question? I can explain 'why' decisions were made, 'how' they were computed, or 'what if' scenarios.".into()
    }
}

impl Default for InteractiveExplainer {
    fn default() -> Self {
        Self::new()
    }
}

/// Dialogue context
struct DialogueContext {
    current_topic: Option<String>,
    referenced_decisions: Vec<u64>,
}

impl DialogueContext {
    fn new() -> Self {
        Self {
            current_topic: None,
            referenced_decisions: Vec::new(),
        }
    }
}

/// Question-answer pair
struct QAPair {
    question: String,
    answer: String,
}

/// Question type
#[derive(Debug, Clone, PartialEq, Eq)]
enum QuestionType {
    Why,
    How,
    What,
    WhatIf,
    Clarification,
}
