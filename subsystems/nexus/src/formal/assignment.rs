//! Variable assignment tracking for SAT solving.

extern crate alloc;

use alloc::vec::Vec;

use super::clause::Clause;
use super::types::{Literal, VarId};

/// Assignment of variables to truth values
#[derive(Debug, Clone)]
pub struct Assignment {
    /// Value for each variable (None = unassigned)
    values: Vec<Option<bool>>,
    /// Assignment trail (for backtracking)
    trail: Vec<(VarId, bool, usize)>, // (var, value, decision_level)
    /// Current decision level
    pub(crate) decision_level: usize,
}

impl Assignment {
    /// Create new empty assignment
    pub fn new(num_vars: VarId) -> Self {
        Self {
            values: alloc::vec![None; num_vars as usize],
            trail: Vec::new(),
            decision_level: 0,
        }
    }

    /// Get value of variable
    #[inline(always)]
    pub fn get(&self, var: VarId) -> Option<bool> {
        self.values.get(var as usize).copied().flatten()
    }

    /// Evaluate literal under current assignment
    #[inline(always)]
    pub fn eval_lit(&self, lit: Literal) -> Option<bool> {
        self.get(lit.var())
            .map(|v| if lit.is_negated() { !v } else { v })
    }

    /// Evaluate clause under current assignment
    pub fn eval_clause(&self, clause: &Clause) -> Option<bool> {
        let mut has_unassigned = false;

        for lit in &clause.literals {
            match self.eval_lit(*lit) {
                Some(true) => return Some(true),
                Some(false) => {},
                None => has_unassigned = true,
            }
        }

        if has_unassigned { None } else { Some(false) }
    }

    /// Assign a variable
    #[inline(always)]
    pub fn assign(&mut self, var: VarId, value: bool) {
        self.values[var as usize] = Some(value);
        self.trail.push((var, value, self.decision_level));
    }

    /// Make a decision (increase level and assign)
    #[inline(always)]
    pub fn decide(&mut self, var: VarId, value: bool) {
        self.decision_level += 1;
        self.assign(var, value);
    }

    /// Backtrack to a decision level
    #[inline]
    pub fn backtrack_to(&mut self, level: usize) {
        while let Some(&(var, _, dl)) = self.trail.last() {
            if dl <= level {
                break;
            }
            self.values[var as usize] = None;
            self.trail.pop();
        }
        self.decision_level = level;
    }

    /// Is fully assigned?
    #[inline(always)]
    pub fn is_complete(&self) -> bool {
        self.values.iter().all(|v| v.is_some())
    }

    /// Get unassigned variable
    #[inline]
    pub fn pick_unassigned(&self) -> Option<VarId> {
        for (i, v) in self.values.iter().enumerate() {
            if v.is_none() {
                return Some(i as VarId);
            }
        }
        None
    }

    /// Get the trail for conflict analysis
    #[inline(always)]
    pub fn trail(&self) -> &[(VarId, bool, usize)] {
        &self.trail
    }
}
