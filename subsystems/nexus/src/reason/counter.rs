//! # Counterfactual Reasoning
//!
//! What-if analysis and counterfactual scenario evaluation.
//! Answers "what would have happened if..." questions.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// COUNTERFACTUAL TYPES
// ============================================================================

/// Counterfactual scenario
#[derive(Debug, Clone)]
pub struct CounterfactualScenario {
    /// Scenario ID
    pub id: u64,
    /// Scenario name
    pub name: String,
    /// Base world (factual state)
    pub base_world: World,
    /// Counterfactual world
    pub cf_world: World,
    /// Antecedent (what we're changing)
    pub antecedent: Antecedent,
    /// Consequent (what we're asking about)
    pub consequent: Consequent,
    /// Status
    pub status: ScenarioStatus,
    /// Created
    pub created: Timestamp,
}

/// World state
#[derive(Debug, Clone)]
pub struct World {
    /// World ID
    pub id: u64,
    /// Variables and their values
    pub variables: BTreeMap<String, WorldValue>,
    /// Structural equations
    pub equations: Vec<StructuralEquation>,
    /// External factors (exogenous)
    pub exogenous: BTreeMap<String, WorldValue>,
}

impl World {
    /// Create new world
    pub fn new() -> Self {
        Self {
            id: 0,
            variables: BTreeMap::new(),
            equations: Vec::new(),
            exogenous: BTreeMap::new(),
        }
    }

    /// Set variable
    pub fn set(&mut self, name: &str, value: WorldValue) {
        self.variables.insert(name.into(), value);
    }

    /// Get variable
    pub fn get(&self, name: &str) -> Option<&WorldValue> {
        self.variables.get(name)
    }

    /// Clone and modify
    pub fn with_change(&self, name: &str, value: WorldValue) -> Self {
        let mut new_world = self.clone();
        new_world.variables.insert(name.into(), value);
        new_world
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// World value
#[derive(Debug, Clone, PartialEq)]
pub enum WorldValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<WorldValue>),
    Null,
}

impl WorldValue {
    /// Convert to float
    pub fn as_float(&self) -> Option<f64> {
        match self {
            WorldValue::Float(f) => Some(*f),
            WorldValue::Int(i) => Some(*i as f64),
            WorldValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    /// Convert to bool
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            WorldValue::Bool(b) => Some(*b),
            WorldValue::Int(i) => Some(*i != 0),
            WorldValue::Float(f) => Some(*f != 0.0),
            _ => None,
        }
    }
}

/// Structural equation
#[derive(Debug, Clone)]
pub struct StructuralEquation {
    /// Target variable
    pub target: String,
    /// Parent variables
    pub parents: Vec<String>,
    /// Function type
    pub function: EquationFunction,
}

/// Equation function
#[derive(Debug, Clone)]
pub enum EquationFunction {
    /// Identity (copy from parent)
    Identity(String),
    /// Linear combination
    Linear { coefficients: Vec<f64>, constant: f64 },
    /// Boolean AND
    And,
    /// Boolean OR
    Or,
    /// Threshold
    Threshold { value: f64 },
    /// Custom (named function)
    Custom(String),
}

/// Antecedent (the counterfactual premise)
#[derive(Debug, Clone)]
pub struct Antecedent {
    /// Variable being changed
    pub variable: String,
    /// Counterfactual value
    pub value: WorldValue,
    /// Original value
    pub original: WorldValue,
    /// Description
    pub description: String,
}

/// Consequent (what we're asking)
#[derive(Debug, Clone)]
pub struct Consequent {
    /// Variable of interest
    pub variable: String,
    /// Query type
    pub query: ConsequentQuery,
}

/// Consequent query type
#[derive(Debug, Clone)]
pub enum ConsequentQuery {
    /// What would the value be?
    Value,
    /// Would it be different?
    Different,
    /// Would it be above threshold?
    AboveThreshold(f64),
    /// Would it change by more than?
    ChangeMoreThan(f64),
}

/// Scenario status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioStatus {
    Created,
    Computing,
    Computed,
    Failed,
}

// ============================================================================
// COUNTERFACTUAL RESULT
// ============================================================================

/// Counterfactual result
#[derive(Debug, Clone)]
pub struct CounterfactualResult {
    /// Scenario ID
    pub scenario_id: u64,
    /// Factual value
    pub factual: WorldValue,
    /// Counterfactual value
    pub counterfactual: WorldValue,
    /// Difference (if numeric)
    pub difference: Option<f64>,
    /// Answer to query
    pub answer: CounterfactualAnswer,
    /// Confidence
    pub confidence: f64,
    /// Explanation
    pub explanation: Vec<String>,
    /// Computation steps
    pub steps: Vec<ComputationStep>,
}

/// Counterfactual answer
#[derive(Debug, Clone)]
pub enum CounterfactualAnswer {
    Yes,
    No,
    Value(WorldValue),
    Uncertain(f64), // probability
}

/// Computation step
#[derive(Debug, Clone)]
pub struct ComputationStep {
    /// Step description
    pub description: String,
    /// Variable affected
    pub variable: String,
    /// Old value
    pub old_value: WorldValue,
    /// New value
    pub new_value: WorldValue,
}

// ============================================================================
// COUNTERFACTUAL ENGINE
// ============================================================================

/// Counterfactual reasoning engine
pub struct CounterfactualEngine {
    /// Scenarios
    scenarios: BTreeMap<u64, CounterfactualScenario>,
    /// Results
    results: BTreeMap<u64, CounterfactualResult>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: CounterfactualConfig,
    /// Statistics
    stats: CounterfactualStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct CounterfactualConfig {
    /// Maximum propagation depth
    pub max_depth: usize,
    /// Confidence threshold
    pub min_confidence: f64,
    /// Allow loops in propagation
    pub allow_loops: bool,
}

impl Default for CounterfactualConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            min_confidence: 0.5,
            allow_loops: false,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct CounterfactualStats {
    /// Scenarios created
    pub scenarios_created: u64,
    /// Computations performed
    pub computations: u64,
    /// Average computation time (ns)
    pub avg_computation_ns: f64,
}

impl CounterfactualEngine {
    /// Create new engine
    pub fn new(config: CounterfactualConfig) -> Self {
        Self {
            scenarios: BTreeMap::new(),
            results: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: CounterfactualStats::default(),
        }
    }

    /// Create scenario
    pub fn create_scenario(
        &mut self,
        name: &str,
        base_world: World,
        antecedent: Antecedent,
        consequent: Consequent,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let cf_world = base_world.with_change(&antecedent.variable, antecedent.value.clone());

        let scenario = CounterfactualScenario {
            id,
            name: name.into(),
            base_world,
            cf_world,
            antecedent,
            consequent,
            status: ScenarioStatus::Created,
            created: Timestamp::now(),
        };

        self.scenarios.insert(id, scenario);
        self.stats.scenarios_created += 1;

        id
    }

    /// Compute counterfactual
    pub fn compute(&mut self, scenario_id: u64) -> Option<CounterfactualResult> {
        let scenario = self.scenarios.get_mut(&scenario_id)?;
        scenario.status = ScenarioStatus::Computing;

        self.stats.computations += 1;

        // Step 1: Abduction - determine exogenous values from factual world
        let exogenous = self.abduct(&scenario.base_world);

        // Step 2: Intervention - apply counterfactual change
        let mut cf_world = scenario.cf_world.clone();
        cf_world.exogenous = exogenous;

        // Step 3: Prediction - propagate to get counterfactual outcome
        let steps = self.propagate(&mut cf_world, &scenario.antecedent.variable);

        // Get values
        let factual = scenario.base_world.get(&scenario.consequent.variable)
            .cloned()
            .unwrap_or(WorldValue::Null);

        let counterfactual = cf_world.get(&scenario.consequent.variable)
            .cloned()
            .unwrap_or(WorldValue::Null);

        // Compute difference
        let difference = match (&factual, &counterfactual) {
            (WorldValue::Float(f), WorldValue::Float(c)) => Some(c - f),
            (WorldValue::Int(f), WorldValue::Int(c)) => Some((c - f) as f64),
            _ => None,
        };

        // Determine answer
        let answer = self.determine_answer(&scenario.consequent, &factual, &counterfactual, difference);

        // Generate explanation
        let explanation = self.generate_explanation(scenario, &steps);

        let result = CounterfactualResult {
            scenario_id,
            factual,
            counterfactual,
            difference,
            answer,
            confidence: 0.9, // Would be computed based on model quality
            explanation,
            steps,
        };

        scenario.status = ScenarioStatus::Computed;
        self.results.insert(scenario_id, result.clone());

        Some(result)
    }

    fn abduct(&self, world: &World) -> BTreeMap<String, WorldValue> {
        // In a full implementation, this would infer exogenous variables
        // that explain the observed factual values
        world.exogenous.clone()
    }

    fn propagate(&self, world: &mut World, changed: &str) -> Vec<ComputationStep> {
        let mut steps = Vec::new();
        let mut to_update: Vec<String> = Vec::new();
        let mut updated = alloc::collections::BTreeSet::new();

        // Find equations that depend on the changed variable
        for eq in &world.equations {
            if eq.parents.contains(&changed.to_string()) {
                to_update.push(eq.target.clone());
            }
        }

        // Propagate changes
        let mut depth = 0;
        while !to_update.is_empty() && depth < self.config.max_depth {
            let current = to_update.remove(0);

            if updated.contains(&current) && !self.config.allow_loops {
                continue;
            }
            updated.insert(current.clone());

            if let Some(eq) = world.equations.iter().find(|e| e.target == current) {
                let old_value = world.get(&current).cloned().unwrap_or(WorldValue::Null);
                let new_value = self.evaluate_equation(eq, world);

                if old_value != new_value {
                    steps.push(ComputationStep {
                        description: format!("Updated {} based on {}", current, eq.parents.join(", ")),
                        variable: current.clone(),
                        old_value: old_value.clone(),
                        new_value: new_value.clone(),
                    });

                    world.set(&current, new_value);

                    // Find downstream equations
                    for other_eq in &world.equations {
                        if other_eq.parents.contains(&current) && !to_update.contains(&other_eq.target) {
                            to_update.push(other_eq.target.clone());
                        }
                    }
                }
            }

            depth += 1;
        }

        steps
    }

    fn evaluate_equation(&self, eq: &StructuralEquation, world: &World) -> WorldValue {
        match &eq.function {
            EquationFunction::Identity(parent) => {
                world.get(parent).cloned().unwrap_or(WorldValue::Null)
            }
            EquationFunction::Linear { coefficients, constant } => {
                let mut sum = *constant;
                for (i, parent) in eq.parents.iter().enumerate() {
                    if let Some(value) = world.get(parent) {
                        if let Some(f) = value.as_float() {
                            sum += coefficients.get(i).unwrap_or(&1.0) * f;
                        }
                    }
                }
                WorldValue::Float(sum)
            }
            EquationFunction::And => {
                let result = eq.parents.iter()
                    .all(|p| world.get(p).and_then(|v| v.as_bool()).unwrap_or(false));
                WorldValue::Bool(result)
            }
            EquationFunction::Or => {
                let result = eq.parents.iter()
                    .any(|p| world.get(p).and_then(|v| v.as_bool()).unwrap_or(false));
                WorldValue::Bool(result)
            }
            EquationFunction::Threshold { value } => {
                let parent_val = eq.parents.first()
                    .and_then(|p| world.get(p))
                    .and_then(|v| v.as_float())
                    .unwrap_or(0.0);
                WorldValue::Bool(parent_val >= *value)
            }
            EquationFunction::Custom(_) => WorldValue::Null,
        }
    }

    fn determine_answer(
        &self,
        consequent: &Consequent,
        factual: &WorldValue,
        counterfactual: &WorldValue,
        difference: Option<f64>,
    ) -> CounterfactualAnswer {
        match &consequent.query {
            ConsequentQuery::Value => CounterfactualAnswer::Value(counterfactual.clone()),
            ConsequentQuery::Different => {
                if factual != counterfactual {
                    CounterfactualAnswer::Yes
                } else {
                    CounterfactualAnswer::No
                }
            }
            ConsequentQuery::AboveThreshold(threshold) => {
                if let Some(f) = counterfactual.as_float() {
                    if f >= *threshold {
                        CounterfactualAnswer::Yes
                    } else {
                        CounterfactualAnswer::No
                    }
                } else {
                    CounterfactualAnswer::Uncertain(0.5)
                }
            }
            ConsequentQuery::ChangeMoreThan(amount) => {
                if let Some(diff) = difference {
                    if diff.abs() > *amount {
                        CounterfactualAnswer::Yes
                    } else {
                        CounterfactualAnswer::No
                    }
                } else {
                    CounterfactualAnswer::Uncertain(0.5)
                }
            }
        }
    }

    fn generate_explanation(
        &self,
        scenario: &CounterfactualScenario,
        steps: &[ComputationStep],
    ) -> Vec<String> {
        let mut explanation = Vec::new();

        explanation.push(format!(
            "If {} had been {:?} instead of {:?}...",
            scenario.antecedent.variable,
            scenario.antecedent.value,
            scenario.antecedent.original
        ));

        for step in steps {
            explanation.push(format!(
                "Then {} would change from {:?} to {:?}",
                step.variable, step.old_value, step.new_value
            ));
        }

        explanation
    }

    /// Get scenario
    pub fn get_scenario(&self, id: u64) -> Option<&CounterfactualScenario> {
        self.scenarios.get(&id)
    }

    /// Get result
    pub fn get_result(&self, id: u64) -> Option<&CounterfactualResult> {
        self.results.get(&id)
    }

    /// Get statistics
    pub fn stats(&self) -> &CounterfactualStats {
        &self.stats
    }
}

impl Default for CounterfactualEngine {
    fn default() -> Self {
        Self::new(CounterfactualConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_creation() {
        let mut world = World::new();
        world.set("X", WorldValue::Float(5.0));
        world.set("Y", WorldValue::Float(10.0));

        assert_eq!(world.get("X"), Some(&WorldValue::Float(5.0)));
    }

    #[test]
    fn test_counterfactual_basic() {
        let mut engine = CounterfactualEngine::default();

        let mut world = World::new();
        world.set("cause", WorldValue::Float(1.0));
        world.set("effect", WorldValue::Float(2.0));

        // effect = 2 * cause
        world.equations.push(StructuralEquation {
            target: "effect".into(),
            parents: vec!["cause".into()],
            function: EquationFunction::Linear {
                coefficients: vec![2.0],
                constant: 0.0,
            },
        });

        let antecedent = Antecedent {
            variable: "cause".into(),
            value: WorldValue::Float(3.0),
            original: WorldValue::Float(1.0),
            description: "What if cause was 3?".into(),
        };

        let consequent = Consequent {
            variable: "effect".into(),
            query: ConsequentQuery::Value,
        };

        let id = engine.create_scenario("test", world, antecedent, consequent);
        let result = engine.compute(id);

        assert!(result.is_some());
        let result = result.unwrap();

        // effect should be 6.0 (3.0 * 2)
        assert_eq!(result.counterfactual, WorldValue::Float(6.0));
    }

    #[test]
    fn test_boolean_counterfactual() {
        let mut engine = CounterfactualEngine::default();

        let mut world = World::new();
        world.set("A", WorldValue::Bool(true));
        world.set("B", WorldValue::Bool(true));
        world.set("C", WorldValue::Bool(true)); // A AND B

        world.equations.push(StructuralEquation {
            target: "C".into(),
            parents: vec!["A".into(), "B".into()],
            function: EquationFunction::And,
        });

        let antecedent = Antecedent {
            variable: "A".into(),
            value: WorldValue::Bool(false),
            original: WorldValue::Bool(true),
            description: "What if A was false?".into(),
        };

        let consequent = Consequent {
            variable: "C".into(),
            query: ConsequentQuery::Different,
        };

        let id = engine.create_scenario("bool_test", world, antecedent, consequent);
        let result = engine.compute(id).unwrap();

        assert!(matches!(result.answer, CounterfactualAnswer::Yes));
    }
}
