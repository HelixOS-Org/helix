//! Options — Possible actions to take
//!
//! Options represent the possible actions that can be taken in response
//! to a conclusion. They include metadata about cost, risk, and expected outcome.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::types::*;

// ============================================================================
// OPTION
// ============================================================================

/// Option ID type
define_id!(OptionId, "Option identifier");

/// An option for action
#[derive(Debug, Clone)]
pub struct Option {
    /// Option ID
    pub id: OptionId,
    /// Action type
    pub action_type: ActionType,
    /// Description
    pub description: String,
    /// Target of the action
    pub target: ActionTarget,
    /// Parameters
    pub parameters: ActionParameters,
    /// Expected outcome
    pub expected_outcome: ExpectedOutcome,
    /// Is reversible
    pub reversible: bool,
    /// Cost estimate
    pub cost: ActionCost,
    /// Source (why this option was generated)
    pub source: OptionSource,
}

impl Option {
    /// Create a new option
    pub fn new(action_type: ActionType, description: impl Into<String>) -> Self {
        Self {
            id: OptionId::generate(),
            action_type,
            description: description.into(),
            target: ActionTarget::System,
            parameters: ActionParameters::new(),
            expected_outcome: ExpectedOutcome::default(),
            reversible: !action_type.is_destructive(),
            cost: ActionCost::default(),
            source: OptionSource::Default,
        }
    }

    /// Set target
    pub fn with_target(mut self, target: ActionTarget) -> Self {
        self.target = target;
        self
    }

    /// Set parameters
    pub fn with_parameters(mut self, parameters: ActionParameters) -> Self {
        self.parameters = parameters;
        self
    }

    /// Set expected outcome
    pub fn with_outcome(mut self, outcome: ExpectedOutcome) -> Self {
        self.expected_outcome = outcome;
        self
    }

    /// Set cost
    pub fn with_cost(mut self, cost: ActionCost) -> Self {
        self.cost = cost;
        self
    }

    /// Set source
    pub fn with_source(mut self, source: OptionSource) -> Self {
        self.source = source;
        self
    }

    /// Is this a safe option?
    pub fn is_safe(&self) -> bool {
        self.action_type.is_safe() && self.cost.risk <= RiskLevel::Low
    }
}

// ============================================================================
// ACTION TYPE
// ============================================================================

/// Action type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionType {
    /// Do nothing
    NoOp,
    /// Restart a component
    Restart,
    /// Kill/terminate a component
    Kill,
    /// Migrate a workload
    Migrate,
    /// Scale resources
    Scale,
    /// Throttle resources
    Throttle,
    /// Reconfigure settings
    Reconfigure,
    /// Allocate resources
    Allocate,
    /// Deallocate resources
    Deallocate,
    /// Enable a feature
    Enable,
    /// Disable a feature
    Disable,
    /// Repair corruption
    Repair,
    /// Quarantine a component
    Quarantine,
    /// Alert operators
    Alert,
    /// Log for analysis
    Log,
    /// Custom action
    Custom(u32),
}

impl ActionType {
    /// Is this a safe action
    pub fn is_safe(&self) -> bool {
        matches!(
            self,
            Self::NoOp | Self::Log | Self::Alert | Self::Throttle | Self::Disable
        )
    }

    /// Is this a destructive action
    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::Kill | Self::Deallocate | Self::Quarantine)
    }

    /// Requires confirmation
    pub fn requires_confirmation(&self) -> bool {
        self.is_destructive()
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::NoOp => "No Operation",
            Self::Restart => "Restart",
            Self::Kill => "Kill",
            Self::Migrate => "Migrate",
            Self::Scale => "Scale",
            Self::Throttle => "Throttle",
            Self::Reconfigure => "Reconfigure",
            Self::Allocate => "Allocate",
            Self::Deallocate => "Deallocate",
            Self::Enable => "Enable",
            Self::Disable => "Disable",
            Self::Repair => "Repair",
            Self::Quarantine => "Quarantine",
            Self::Alert => "Alert",
            Self::Log => "Log",
            Self::Custom(_) => "Custom",
        }
    }

    /// Get default priority (higher = more disruptive)
    pub fn disruption_level(&self) -> u8 {
        match self {
            Self::NoOp | Self::Log => 0,
            Self::Alert => 1,
            Self::Throttle | Self::Disable => 2,
            Self::Reconfigure | Self::Enable => 3,
            Self::Scale | Self::Allocate => 4,
            Self::Migrate | Self::Repair => 5,
            Self::Restart | Self::Deallocate => 6,
            Self::Kill | Self::Quarantine => 7,
            Self::Custom(_) => 5,
        }
    }
}

// ============================================================================
// ACTION TARGET
// ============================================================================

/// Target of an action
#[derive(Debug, Clone)]
pub enum ActionTarget {
    /// System-wide
    System,
    /// Specific CPU
    Cpu(u32),
    /// Specific memory region
    Memory { start: u64, size: u64 },
    /// Process
    Process(u32),
    /// Thread
    Thread { pid: u32, tid: u32 },
    /// Device
    Device(String),
    /// Network interface
    Network(String),
    /// Filesystem
    Filesystem(String),
    /// Module
    Module(String),
    /// Custom target
    Custom(String),
}

impl ActionTarget {
    /// Get target type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Cpu(_) => "cpu",
            Self::Memory { .. } => "memory",
            Self::Process(_) => "process",
            Self::Thread { .. } => "thread",
            Self::Device(_) => "device",
            Self::Network(_) => "network",
            Self::Filesystem(_) => "filesystem",
            Self::Module(_) => "module",
            Self::Custom(_) => "custom",
        }
    }

    /// Is system-wide target?
    pub fn is_system_wide(&self) -> bool {
        matches!(self, Self::System)
    }
}

// ============================================================================
// ACTION PARAMETERS
// ============================================================================

/// Action parameters
#[derive(Debug, Clone, Default)]
pub struct ActionParameters {
    /// Integer parameters
    pub integers: BTreeMap<String, i64>,
    /// String parameters
    pub strings: BTreeMap<String, String>,
    /// Boolean parameters
    pub booleans: BTreeMap<String, bool>,
    /// Float parameters
    pub floats: BTreeMap<String, f64>,
}

impl ActionParameters {
    /// Create empty parameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Set integer parameter
    pub fn set_int(&mut self, key: impl Into<String>, value: i64) -> &mut Self {
        self.integers.insert(key.into(), value);
        self
    }

    /// Set string parameter
    pub fn set_str(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.strings.insert(key.into(), value.into());
        self
    }

    /// Set boolean parameter
    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) -> &mut Self {
        self.booleans.insert(key.into(), value);
        self
    }

    /// Set float parameter
    pub fn set_float(&mut self, key: impl Into<String>, value: f64) -> &mut Self {
        self.floats.insert(key.into(), value);
        self
    }

    /// Get integer parameter
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.integers.get(key).copied()
    }

    /// Get string parameter
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.strings.get(key).map(|s| s.as_str())
    }

    /// Get boolean parameter
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.booleans.get(key).copied()
    }

    /// Get float parameter
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.floats.get(key).copied()
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.integers.is_empty()
            && self.strings.is_empty()
            && self.booleans.is_empty()
            && self.floats.is_empty()
    }

    /// Total parameter count
    pub fn len(&self) -> usize {
        self.integers.len() + self.strings.len() + self.booleans.len() + self.floats.len()
    }
}

// ============================================================================
// EXPECTED OUTCOME
// ============================================================================

/// Expected outcome of an action
#[derive(Debug, Clone)]
pub struct ExpectedOutcome {
    /// Description
    pub description: String,
    /// Probability of success (0.0 to 1.0)
    pub success_probability: f32,
    /// Time to effect
    pub time_to_effect: Duration,
    /// Side effects
    pub side_effects: Vec<String>,
}

impl Default for ExpectedOutcome {
    fn default() -> Self {
        Self {
            description: String::new(),
            success_probability: 0.5,
            time_to_effect: Duration::from_secs(1),
            side_effects: Vec::new(),
        }
    }
}

impl ExpectedOutcome {
    /// Create new outcome
    pub fn new(description: impl Into<String>, probability: f32) -> Self {
        Self {
            description: description.into(),
            success_probability: probability.clamp(0.0, 1.0),
            time_to_effect: Duration::from_secs(1),
            side_effects: Vec::new(),
        }
    }

    /// Set time to effect
    pub fn with_time(mut self, time: Duration) -> Self {
        self.time_to_effect = time;
        self
    }

    /// Add side effect
    pub fn with_side_effect(mut self, effect: impl Into<String>) -> Self {
        self.side_effects.push(effect.into());
        self
    }

    /// Is likely to succeed?
    pub fn is_likely(&self) -> bool {
        self.success_probability >= 0.7
    }
}

// ============================================================================
// ACTION COST
// ============================================================================

/// Action cost
#[derive(Debug, Clone)]
pub struct ActionCost {
    /// CPU cost (0-100 scale)
    pub cpu: u8,
    /// Memory cost (bytes)
    pub memory: u64,
    /// I/O cost (0-100 scale)
    pub io: u8,
    /// Time cost
    pub time: Duration,
    /// Risk level
    pub risk: RiskLevel,
}

impl Default for ActionCost {
    fn default() -> Self {
        Self {
            cpu: 0,
            memory: 0,
            io: 0,
            time: Duration::ZERO,
            risk: RiskLevel::Minimal,
        }
    }
}

impl ActionCost {
    /// Create new cost
    pub fn new(cpu: u8, io: u8, risk: RiskLevel) -> Self {
        Self {
            cpu,
            memory: 0,
            io,
            time: Duration::ZERO,
            risk,
        }
    }

    /// Set memory cost
    pub fn with_memory(mut self, memory: u64) -> Self {
        self.memory = memory;
        self
    }

    /// Set time cost
    pub fn with_time(mut self, time: Duration) -> Self {
        self.time = time;
        self
    }

    /// Get total resource cost (0-100)
    pub fn total_resource_cost(&self) -> u8 {
        ((self.cpu as u16 + self.io as u16) / 2).min(100) as u8
    }

    /// Is expensive?
    pub fn is_expensive(&self) -> bool {
        self.total_resource_cost() > 50 || self.risk >= RiskLevel::Medium
    }
}

// ============================================================================
// RISK LEVEL
// ============================================================================

/// Risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// No risk
    Minimal,
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Critical risk
    Critical,
}

impl RiskLevel {
    /// Get numeric value (0-4)
    pub fn value(&self) -> u8 {
        match self {
            Self::Minimal => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }

    /// From numeric value
    pub fn from_value(value: u8) -> Self {
        match value {
            0 => Self::Minimal,
            1 => Self::Low,
            2 => Self::Medium,
            3 => Self::High,
            _ => Self::Critical,
        }
    }
}

// ============================================================================
// OPTION SOURCE
// ============================================================================

/// Option source
#[derive(Debug, Clone)]
pub enum OptionSource {
    /// Generated from conclusion
    Conclusion(ConclusionId),
    /// From policy
    Policy(PolicyId),
    /// Default option
    Default,
    /// Manual request
    Manual,
}

// ============================================================================
// IMPACT
// ============================================================================

/// Expected impact of an action
#[derive(Debug, Clone, Default)]
pub struct Impact {
    /// Performance change (-100 to +100)
    pub performance: i8,
    /// Reliability change (-100 to +100)
    pub reliability: i8,
    /// Resource usage change (-100 to +100)
    pub resources: i8,
}

impl Impact {
    /// Create new impact
    pub fn new(performance: i8, reliability: i8, resources: i8) -> Self {
        Self {
            performance: performance.clamp(-100, 100),
            reliability: reliability.clamp(-100, 100),
            resources: resources.clamp(-100, 100),
        }
    }

    /// Is positive overall?
    pub fn is_positive(&self) -> bool {
        (self.performance as i16 + self.reliability as i16 - self.resources as i16) > 0
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type_properties() {
        assert!(ActionType::NoOp.is_safe());
        assert!(ActionType::Kill.is_destructive());
        assert!(ActionType::Kill.requires_confirmation());
        assert!(!ActionType::Log.is_destructive());
    }

    #[test]
    fn test_action_parameters() {
        let mut params = ActionParameters::new();
        params.set_int("count", 42);
        params.set_str("name", "test");
        params.set_bool("enabled", true);

        assert_eq!(params.get_int("count"), Some(42));
        assert_eq!(params.get_str("name"), Some("test"));
        assert_eq!(params.get_bool("enabled"), Some(true));
    }

    #[test]
    fn test_action_cost() {
        let cost = ActionCost::new(50, 30, RiskLevel::Medium)
            .with_memory(1024 * 1024);

        assert!(cost.is_expensive());
        assert_eq!(cost.total_resource_cost(), 40);
    }

    #[test]
    fn test_risk_level() {
        assert!(RiskLevel::High > RiskLevel::Low);
        assert_eq!(RiskLevel::from_value(2), RiskLevel::Medium);
    }
}
