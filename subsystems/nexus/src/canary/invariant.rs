//! Invariant definitions and checking.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::core::{ComponentId, NexusTimestamp};

/// An invariant that should always hold
pub struct Invariant {
    /// Unique ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Check function
    check: Box<dyn Fn() -> InvariantResult + Send + Sync>,
    /// Is this invariant critical?
    pub critical: bool,
    /// Component this invariant belongs to
    pub component: Option<ComponentId>,
    /// Check interval (ticks)
    pub interval: u64,
    /// Last check timestamp
    last_check: NexusTimestamp,
    /// Consecutive failures
    consecutive_failures: u32,
    /// Total checks
    pub(crate) total_checks: u64,
    /// Total violations
    total_violations: u64,
    /// Is enabled?
    enabled: AtomicBool,
}

impl Invariant {
    /// Create a new invariant
    pub fn new(
        name: impl Into<String>,
        check: impl Fn() -> InvariantResult + Send + Sync + 'static,
    ) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            name: name.into(),
            description: String::new(),
            check: Box::new(check),
            critical: false,
            component: None,
            interval: 1000, // Check every 1000 ticks by default
            last_check: NexusTimestamp::from_ticks(0),
            consecutive_failures: 0,
            total_checks: 0,
            total_violations: 0,
            enabled: AtomicBool::new(true),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Mark as critical
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }

    /// Set component
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.component = Some(component);
        self
    }

    /// Set check interval
    pub fn with_interval(mut self, ticks: u64) -> Self {
        self.interval = ticks;
        self
    }

    /// Enable the invariant
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable the invariant
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// Is enabled?
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Should check now?
    pub fn should_check(&self, now: NexusTimestamp) -> bool {
        self.is_enabled() && now.duration_since(self.last_check) >= self.interval
    }

    /// Run the check
    pub fn check(&mut self) -> InvariantCheck {
        self.total_checks += 1;
        self.last_check = NexusTimestamp::now();

        let start = NexusTimestamp::now();
        let result = (self.check)();
        let end = NexusTimestamp::now();

        let violated = !result.holds;
        if violated {
            self.consecutive_failures += 1;
            self.total_violations += 1;
        } else {
            self.consecutive_failures = 0;
        }

        InvariantCheck {
            invariant_id: self.id,
            invariant_name: self.name.clone(),
            result,
            duration: end.duration_since(start),
            timestamp: self.last_check,
            consecutive_failures: self.consecutive_failures,
            critical: self.critical,
            component: self.component,
        }
    }

    /// Get violation rate
    pub fn violation_rate(&self) -> f64 {
        if self.total_checks == 0 {
            return 0.0;
        }
        self.total_violations as f64 / self.total_checks as f64
    }

    /// Get consecutive failures
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }
}

/// Result of an invariant check
#[derive(Debug, Clone)]
pub struct InvariantResult {
    /// Does the invariant hold?
    pub holds: bool,
    /// Message (especially useful when violated)
    pub message: Option<String>,
    /// Additional context
    pub context: Vec<(String, String)>,
}

impl InvariantResult {
    /// Invariant holds
    pub fn ok() -> Self {
        Self {
            holds: true,
            message: None,
            context: Vec::new(),
        }
    }

    /// Invariant violated
    pub fn violated(message: impl Into<String>) -> Self {
        Self {
            holds: false,
            message: Some(message.into()),
            context: Vec::new(),
        }
    }

    /// Add context
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push((key.into(), value.into()));
        self
    }

    /// From a boolean
    pub fn from_bool(holds: bool, violation_message: impl Into<String>) -> Self {
        if holds {
            Self::ok()
        } else {
            Self::violated(violation_message)
        }
    }
}

/// Record of an invariant check
#[derive(Debug, Clone)]
pub struct InvariantCheck {
    /// Invariant ID
    pub invariant_id: u64,
    /// Invariant name
    pub invariant_name: String,
    /// Result
    pub result: InvariantResult,
    /// Duration of check
    pub duration: u64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Consecutive failures
    pub consecutive_failures: u32,
    /// Is critical?
    pub critical: bool,
    /// Component
    pub component: Option<ComponentId>,
}

impl InvariantCheck {
    /// Was the invariant violated?
    pub fn violated(&self) -> bool {
        !self.result.holds
    }
}
