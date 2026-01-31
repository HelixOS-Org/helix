//! Option Generator — Creates options from conclusions
//!
//! The option generator transforms conclusions from the REASON domain
//! into concrete options that can be evaluated and executed.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::*;
use super::conclusion::{Conclusion, ConclusionType};
use super::options::{
    Option, OptionId, ActionType, ActionTarget, ActionParameters,
    ExpectedOutcome, ActionCost, OptionSource, RiskLevel,
};

// ============================================================================
// GENERATION RULE
// ============================================================================

/// A rule for generating options
#[derive(Debug, Clone)]
pub struct GenerationRule {
    /// Rule ID
    pub id: u32,
    /// Conclusion types this applies to
    pub applies_to: Vec<ConclusionType>,
    /// Minimum severity
    pub min_severity: Severity,
    /// Actions to generate
    pub actions: Vec<ActionType>,
}

impl GenerationRule {
    /// Create new rule
    pub fn new(id: u32, min_severity: Severity) -> Self {
        Self {
            id,
            applies_to: Vec::new(),
            min_severity,
            actions: Vec::new(),
        }
    }

    /// Set conclusion types
    pub fn for_types(mut self, types: Vec<ConclusionType>) -> Self {
        self.applies_to = types;
        self
    }

    /// Set actions
    pub fn with_actions(mut self, actions: Vec<ActionType>) -> Self {
        self.actions = actions;
        self
    }

    /// Check if rule applies to conclusion
    pub fn applies(&self, conclusion: &Conclusion) -> bool {
        self.applies_to.contains(&conclusion.conclusion_type)
            && conclusion.severity >= self.min_severity
    }
}

// ============================================================================
// OPTION GENERATOR
// ============================================================================

/// Option generator - creates options from conclusions
pub struct OptionGenerator {
    /// Generation rules
    rules: Vec<GenerationRule>,
    /// Options generated
    options_generated: AtomicU64,
}

impl OptionGenerator {
    /// Create new generator
    pub fn new() -> Self {
        Self {
            rules: Self::default_rules(),
            options_generated: AtomicU64::new(0),
        }
    }

    /// Default generation rules
    fn default_rules() -> Vec<GenerationRule> {
        vec![
            // Diagnosis rules
            GenerationRule {
                id: 1,
                applies_to: vec![ConclusionType::Diagnosis],
                min_severity: Severity::Warning,
                actions: vec![ActionType::Alert, ActionType::Log],
            },
            GenerationRule {
                id: 2,
                applies_to: vec![ConclusionType::Diagnosis],
                min_severity: Severity::Error,
                actions: vec![ActionType::Restart, ActionType::Alert, ActionType::Log],
            },
            GenerationRule {
                id: 3,
                applies_to: vec![ConclusionType::Diagnosis],
                min_severity: Severity::Critical,
                actions: vec![
                    ActionType::Kill,
                    ActionType::Restart,
                    ActionType::Quarantine,
                    ActionType::Alert,
                ],
            },
            // Prediction rules
            GenerationRule {
                id: 10,
                applies_to: vec![ConclusionType::Prediction],
                min_severity: Severity::Warning,
                actions: vec![ActionType::Throttle, ActionType::Log],
            },
            GenerationRule {
                id: 11,
                applies_to: vec![ConclusionType::Prediction],
                min_severity: Severity::Error,
                actions: vec![ActionType::Migrate, ActionType::Scale, ActionType::Alert],
            },
            // Opportunity rules
            GenerationRule {
                id: 20,
                applies_to: vec![ConclusionType::Opportunity],
                min_severity: Severity::Info,
                actions: vec![ActionType::Reconfigure, ActionType::Log],
            },
        ]
    }

    /// Add custom rule
    pub fn add_rule(&mut self, rule: GenerationRule) {
        self.rules.push(rule);
    }

    /// Clear all rules
    pub fn clear_rules(&mut self) {
        self.rules.clear();
    }

    /// Generate options from conclusion
    pub fn generate(&self, conclusion: &Conclusion) -> Vec<Option> {
        let mut options = Vec::new();

        // Always include NoOp option
        options.push(self.create_noop_option(conclusion));

        // Find applicable rules
        for rule in &self.rules {
            if rule.applies(conclusion) {
                for action_type in &rule.actions {
                    if let Some(option) = self.create_option(conclusion, *action_type) {
                        options.push(option);
                    }
                }
            }
        }

        self.options_generated
            .fetch_add(options.len() as u64, Ordering::Relaxed);
        options
    }

    /// Create NoOp option
    fn create_noop_option(&self, conclusion: &Conclusion) -> Option {
        Option {
            id: OptionId::generate(),
            action_type: ActionType::NoOp,
            description: String::from("Take no action"),
            target: ActionTarget::System,
            parameters: ActionParameters::new(),
            expected_outcome: ExpectedOutcome {
                description: String::from("No change to system state"),
                success_probability: 1.0,
                time_to_effect: Duration::ZERO,
                side_effects: Vec::new(),
            },
            reversible: true,
            cost: ActionCost::default(),
            source: OptionSource::Conclusion(conclusion.id),
        }
    }

    /// Create option for action type
    fn create_option(&self, conclusion: &Conclusion, action_type: ActionType) -> Option<Option> {
        let target = self.infer_target(conclusion);
        let description = self.generate_description(action_type, &target);

        Some(Option {
            id: OptionId::generate(),
            action_type,
            description,
            target,
            parameters: ActionParameters::new(),
            expected_outcome: self.estimate_outcome(action_type),
            reversible: !action_type.is_destructive(),
            cost: self.estimate_cost(action_type),
            source: OptionSource::Conclusion(conclusion.id),
        })
    }

    /// Infer target from conclusion
    fn infer_target(&self, _conclusion: &Conclusion) -> ActionTarget {
        // Default to system-wide
        ActionTarget::System
    }

    /// Generate description
    fn generate_description(&self, action_type: ActionType, target: &ActionTarget) -> String {
        let target_str = match target {
            ActionTarget::System => "system",
            ActionTarget::Cpu(id) => return format!("{:?} CPU {}", action_type, id),
            ActionTarget::Process(pid) => return format!("{:?} process {}", action_type, pid),
            _ => "target",
        };
        format!("{:?} {}", action_type, target_str)
    }

    /// Estimate outcome
    fn estimate_outcome(&self, action_type: ActionType) -> ExpectedOutcome {
        match action_type {
            ActionType::NoOp => ExpectedOutcome {
                description: String::from("No change"),
                success_probability: 1.0,
                time_to_effect: Duration::ZERO,
                side_effects: Vec::new(),
            },
            ActionType::Log | ActionType::Alert => ExpectedOutcome {
                description: String::from("Record event"),
                success_probability: 0.99,
                time_to_effect: Duration::from_millis(1),
                side_effects: Vec::new(),
            },
            ActionType::Restart => ExpectedOutcome {
                description: String::from("Component restarted"),
                success_probability: 0.95,
                time_to_effect: Duration::from_secs(5),
                side_effects: vec![String::from("Temporary unavailability")],
            },
            ActionType::Kill => ExpectedOutcome {
                description: String::from("Component terminated"),
                success_probability: 0.99,
                time_to_effect: Duration::from_millis(100),
                side_effects: vec![String::from("Data loss possible")],
            },
            ActionType::Throttle => ExpectedOutcome {
                description: String::from("Resource usage reduced"),
                success_probability: 0.95,
                time_to_effect: Duration::from_millis(10),
                side_effects: vec![String::from("Performance degradation")],
            },
            _ => ExpectedOutcome {
                description: String::from("Action performed"),
                success_probability: 0.9,
                time_to_effect: Duration::from_secs(1),
                side_effects: Vec::new(),
            },
        }
    }

    /// Estimate cost
    fn estimate_cost(&self, action_type: ActionType) -> ActionCost {
        match action_type {
            ActionType::NoOp => ActionCost::default(),
            ActionType::Log => ActionCost {
                cpu: 1,
                memory: 1024,
                io: 5,
                time: Duration::from_millis(1),
                risk: RiskLevel::Minimal,
            },
            ActionType::Restart => ActionCost {
                cpu: 20,
                memory: 10 * 1024 * 1024,
                io: 30,
                time: Duration::from_secs(5),
                risk: RiskLevel::Medium,
            },
            ActionType::Kill => ActionCost {
                cpu: 5,
                memory: 0,
                io: 5,
                time: Duration::from_millis(100),
                risk: RiskLevel::High,
            },
            _ => ActionCost {
                cpu: 10,
                memory: 1024 * 1024,
                io: 10,
                time: Duration::from_secs(1),
                risk: RiskLevel::Low,
            },
        }
    }

    /// Get statistics
    pub fn stats(&self) -> u64 {
        self.options_generated.load(Ordering::Relaxed)
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for OptionGenerator {
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

    fn make_test_conclusion() -> Conclusion {
        Conclusion {
            id: ConclusionId::generate(),
            conclusion_type: ConclusionType::Diagnosis,
            severity: Severity::Error,
            confidence: Confidence::new(0.85),
            summary: String::from("Test conclusion"),
            explanation: String::from("This is a test"),
            evidence: Vec::new(),
            suggested_actions: Vec::new(),
            timestamp: Timestamp::now(),
            ttl: Duration::from_secs(60),
        }
    }

    #[test]
    fn test_option_generator() {
        let generator = OptionGenerator::new();
        let conclusion = make_test_conclusion();

        let options = generator.generate(&conclusion);
        assert!(!options.is_empty());
        assert!(options.iter().any(|o| o.action_type == ActionType::NoOp));
    }

    #[test]
    fn test_noop_always_included() {
        let generator = OptionGenerator::new();
        let conclusion = make_test_conclusion();

        let options = generator.generate(&conclusion);
        assert!(options.first().map(|o| o.action_type == ActionType::NoOp).unwrap_or(false));
    }

    #[test]
    fn test_generation_rule() {
        let rule = GenerationRule::new(1, Severity::Warning)
            .for_types(vec![ConclusionType::Diagnosis])
            .with_actions(vec![ActionType::Alert]);

        let conclusion = make_test_conclusion();
        assert!(rule.applies(&conclusion));
    }
}
