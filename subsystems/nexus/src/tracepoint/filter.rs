//! Event Filter Engine
//!
//! Filter expressions and predicates for trace events.

use alloc::boxed::Box;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{EventData, TracepointId};

/// Filter operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    /// Equal
    Eq,
    /// Not equal
    Ne,
    /// Less than
    Lt,
    /// Less than or equal
    Le,
    /// Greater than
    Gt,
    /// Greater than or equal
    Ge,
    /// Bitwise AND non-zero
    BitAnd,
    /// String match
    StrMatch,
    /// String glob
    StrGlob,
}

/// Filter predicate
#[derive(Debug, Clone)]
pub struct FilterPredicate {
    /// Field name to filter on
    pub field_name: String,
    /// Field offset
    pub field_offset: usize,
    /// Field size
    pub field_size: usize,
    /// Operation
    pub op: FilterOp,
    /// Value to compare (as u64)
    pub value: u64,
    /// String value (for string operations)
    pub string_value: Option<String>,
}

impl FilterPredicate {
    /// Create new numeric predicate
    pub fn numeric(
        field_name: String,
        field_offset: usize,
        field_size: usize,
        op: FilterOp,
        value: u64,
    ) -> Self {
        Self {
            field_name,
            field_offset,
            field_size,
            op,
            value,
            string_value: None,
        }
    }

    /// Create string predicate
    pub fn string(
        field_name: String,
        field_offset: usize,
        field_size: usize,
        op: FilterOp,
        string_value: String,
    ) -> Self {
        Self {
            field_name,
            field_offset,
            field_size,
            op,
            value: 0,
            string_value: Some(string_value),
        }
    }

    /// Evaluate predicate against event data
    pub fn evaluate(&self, event: &EventData) -> bool {
        let field_value = match self.field_size {
            1 => event.read_u8(self.field_offset).map(|v| v as u64),
            2 => event.read_u16(self.field_offset).map(|v| v as u64),
            4 => event.read_u32(self.field_offset).map(|v| v as u64),
            8 => event.read_u64(self.field_offset),
            _ => None,
        };

        let field_value = match field_value {
            Some(v) => v,
            None => return false,
        };

        match self.op {
            FilterOp::Eq => field_value == self.value,
            FilterOp::Ne => field_value != self.value,
            FilterOp::Lt => field_value < self.value,
            FilterOp::Le => field_value <= self.value,
            FilterOp::Gt => field_value > self.value,
            FilterOp::Ge => field_value >= self.value,
            FilterOp::BitAnd => (field_value & self.value) != 0,
            _ => false, // String ops not implemented for numeric
        }
    }
}

/// Filter expression
#[derive(Debug, Clone)]
pub enum FilterExpr {
    /// Single predicate
    Predicate(FilterPredicate),
    /// AND of expressions
    And(Box<FilterExpr>, Box<FilterExpr>),
    /// OR of expressions
    Or(Box<FilterExpr>, Box<FilterExpr>),
    /// NOT of expression
    Not(Box<FilterExpr>),
    /// Always true
    True,
    /// Always false
    False,
}

impl FilterExpr {
    /// Evaluate filter expression
    pub fn evaluate(&self, event: &EventData) -> bool {
        match self {
            Self::Predicate(pred) => pred.evaluate(event),
            Self::And(left, right) => left.evaluate(event) && right.evaluate(event),
            Self::Or(left, right) => left.evaluate(event) || right.evaluate(event),
            Self::Not(expr) => !expr.evaluate(event),
            Self::True => true,
            Self::False => false,
        }
    }

    /// Create AND expression
    pub fn and(self, other: FilterExpr) -> Self {
        Self::And(Box::new(self), Box::new(other))
    }

    /// Create OR expression
    pub fn or(self, other: FilterExpr) -> Self {
        Self::Or(Box::new(self), Box::new(other))
    }

    /// Create NOT expression
    pub fn not(self) -> Self {
        Self::Not(Box::new(self))
    }
}

/// Event filter
#[derive(Debug)]
pub struct EventFilter {
    /// Filter ID
    pub id: u64,
    /// Target tracepoint
    pub tracepoint_id: TracepointId,
    /// Filter expression
    pub expression: FilterExpr,
    /// Events passed
    pub events_passed: AtomicU64,
    /// Events filtered
    pub events_filtered: AtomicU64,
    /// Enabled
    pub enabled: bool,
}

impl EventFilter {
    /// Create new event filter
    pub fn new(id: u64, tracepoint_id: TracepointId, expression: FilterExpr) -> Self {
        Self {
            id,
            tracepoint_id,
            expression,
            events_passed: AtomicU64::new(0),
            events_filtered: AtomicU64::new(0),
            enabled: true,
        }
    }

    /// Apply filter
    pub fn apply(&self, event: &EventData) -> bool {
        if !self.enabled {
            return true; // Disabled filter passes all
        }

        let passed = self.expression.evaluate(event);
        if passed {
            self.events_passed.fetch_add(1, Ordering::Relaxed);
        } else {
            self.events_filtered.fetch_add(1, Ordering::Relaxed);
        }
        passed
    }

    /// Get pass rate
    pub fn pass_rate(&self) -> f32 {
        let passed = self.events_passed.load(Ordering::Relaxed);
        let filtered = self.events_filtered.load(Ordering::Relaxed);
        let total = passed + filtered;
        if total == 0 {
            1.0
        } else {
            passed as f32 / total as f32
        }
    }

    /// Get events passed
    pub fn events_passed(&self) -> u64 {
        self.events_passed.load(Ordering::Relaxed)
    }

    /// Get events filtered
    pub fn events_filtered(&self) -> u64 {
        self.events_filtered.load(Ordering::Relaxed)
    }

    /// Enable filter
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable filter
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}
