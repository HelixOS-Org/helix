//! CNF formula representation.

extern crate alloc;

use alloc::vec::Vec;

use super::clause::Clause;
use super::types::{Literal, VarId};

/// CNF formula (conjunction of clauses)
#[derive(Debug, Clone, Default)]
pub struct CnfFormula {
    /// Number of variables
    pub num_vars: VarId,
    /// Clauses
    pub clauses: Vec<Clause>,
}

impl CnfFormula {
    /// Create empty formula
    pub fn new() -> Self {
        Self {
            num_vars: 0,
            clauses: Vec::new(),
        }
    }

    /// Add a new variable
    pub fn new_var(&mut self) -> VarId {
        let id = self.num_vars;
        self.num_vars += 1;
        id
    }

    /// Add a clause
    pub fn add_clause(&mut self, clause: Clause) {
        self.clauses.push(clause);
    }

    /// Add unit clause (assert literal)
    pub fn add_unit(&mut self, lit: Literal) {
        self.add_clause(Clause::unit(lit));
    }
}
