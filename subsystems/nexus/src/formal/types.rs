//! Basic type aliases and literal types for SAT/SMT solving.

/// Variable identifier
pub type VarId = u32;

/// Literal (positive or negative variable)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Literal {
    /// Variable ID
    var: VarId,
    /// Is negated
    negated: bool,
}

impl Literal {
    /// Create positive literal
    pub fn pos(var: VarId) -> Self {
        Self {
            var,
            negated: false,
        }
    }

    /// Create negative literal
    pub fn neg(var: VarId) -> Self {
        Self { var, negated: true }
    }

    /// Get underlying variable
    pub fn var(&self) -> VarId {
        self.var
    }

    /// Is this literal negated?
    pub fn is_negated(&self) -> bool {
        self.negated
    }

    /// Negate this literal
    pub fn negate(&self) -> Self {
        Self {
            var: self.var,
            negated: !self.negated,
        }
    }

    /// To DIMACS format (positive = var+1, negative = -(var+1))
    pub fn to_dimacs(&self) -> i32 {
        if self.negated {
            -((self.var + 1) as i32)
        } else {
            (self.var + 1) as i32
        }
    }
}
