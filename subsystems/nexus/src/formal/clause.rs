//! CNF clause representation.

extern crate alloc;

use alloc::vec::Vec;

use super::types::Literal;

/// CNF clause (disjunction of literals)
#[derive(Debug, Clone, Default)]
pub struct Clause {
    /// Literals in the clause
    pub literals: Vec<Literal>,
}

impl Clause {
    /// Create empty clause (contradiction)
    pub fn empty() -> Self {
        Self {
            literals: Vec::new(),
        }
    }

    /// Create unit clause
    pub fn unit(lit: Literal) -> Self {
        Self {
            literals: alloc::vec![lit],
        }
    }

    /// Create binary clause
    pub fn binary(a: Literal, b: Literal) -> Self {
        Self {
            literals: alloc::vec![a, b],
        }
    }

    /// Create from multiple literals
    pub fn from_lits(lits: &[Literal]) -> Self {
        Self {
            literals: lits.to_vec(),
        }
    }

    /// Is empty (conflict)?
    pub fn is_empty(&self) -> bool {
        self.literals.is_empty()
    }

    /// Is unit?
    pub fn is_unit(&self) -> bool {
        self.literals.len() == 1
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.literals.len()
    }
}
