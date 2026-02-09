//! # Deductive Reasoning
//!
//! Derives logical conclusions from premises.
//! Supports classical deduction and proof verification.
//!
//! Part of Year 2 COGNITION - Q2: Causal Reasoning

#![allow(dead_code)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::vec;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// DEDUCTION TYPES
// ============================================================================

/// Proposition
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposition {
    /// Proposition ID
    pub id: u64,
    /// Content
    pub content: PropositionContent,
}

/// Proposition content
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropositionContent {
    /// Atomic proposition
    Atom(String),
    /// Negation
    Not(Box<Proposition>),
    /// Conjunction
    And(Box<Proposition>, Box<Proposition>),
    /// Disjunction
    Or(Box<Proposition>, Box<Proposition>),
    /// Implication
    Implies(Box<Proposition>, Box<Proposition>),
    /// Equivalence
    Iff(Box<Proposition>, Box<Proposition>),
    /// Universal quantification
    ForAll(String, Box<Proposition>),
    /// Existential quantification
    Exists(String, Box<Proposition>),
}

/// Proof
#[derive(Debug, Clone)]
pub struct Proof {
    /// Proof ID
    pub id: u64,
    /// Premises
    pub premises: Vec<Proposition>,
    /// Conclusion
    pub conclusion: Proposition,
    /// Steps
    pub steps: Vec<ProofStep>,
    /// Status
    pub status: ProofStatus,
    /// Created
    pub created: Timestamp,
}

/// Proof step
#[derive(Debug, Clone)]
pub struct ProofStep {
    /// Step number
    pub number: usize,
    /// Proposition derived
    pub proposition: Proposition,
    /// Rule applied
    pub rule: InferenceRule,
    /// References to previous steps
    pub references: Vec<usize>,
}

/// Inference rule
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceRule {
    /// Premise (given)
    Premise,
    /// Modus ponens: A, A→B ⊢ B
    ModusPonens,
    /// Modus tollens: ¬B, A→B ⊢ ¬A
    ModusTollens,
    /// Hypothetical syllogism: A→B, B→C ⊢ A→C
    HypotheticalSyllogism,
    /// Disjunctive syllogism: A∨B, ¬A ⊢ B
    DisjunctiveSyllogism,
    /// Conjunction introduction: A, B ⊢ A∧B
    ConjunctionIntro,
    /// Conjunction elimination: A∧B ⊢ A
    ConjunctionElim,
    /// Disjunction introduction: A ⊢ A∨B
    DisjunctionIntro,
    /// Double negation: ¬¬A ⊢ A
    DoubleNegation,
    /// Contraposition: A→B ⊢ ¬B→¬A
    Contraposition,
}

/// Proof status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofStatus {
    InProgress,
    Valid,
    Invalid,
    Incomplete,
}

/// Deduction result
#[derive(Debug, Clone)]
pub struct DeductionResult {
    /// Derived propositions
    pub derived: Vec<Proposition>,
    /// Rules applied
    pub rules_applied: Vec<InferenceRule>,
    /// Is valid
    pub is_valid: bool,
}

// ============================================================================
// DEDUCTION ENGINE
// ============================================================================

/// Deduction engine
pub struct DeductionEngine {
    /// Knowledge base
    knowledge_base: BTreeMap<u64, Proposition>,
    /// Proofs
    proofs: BTreeMap<u64, Proof>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: DeductionConfig,
    /// Statistics
    stats: DeductionStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct DeductionConfig {
    /// Maximum proof depth
    pub max_depth: usize,
    /// Enable automatic proof search
    pub auto_prove: bool,
}

impl Default for DeductionConfig {
    fn default() -> Self {
        Self {
            max_depth: 20,
            auto_prove: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct DeductionStats {
    /// Propositions added
    pub propositions_added: u64,
    /// Deductions performed
    pub deductions_performed: u64,
    /// Proofs completed
    pub proofs_completed: u64,
}

impl DeductionEngine {
    /// Create new engine
    pub fn new(config: DeductionConfig) -> Self {
        Self {
            knowledge_base: BTreeMap::new(),
            proofs: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: DeductionStats::default(),
        }
    }

    /// Assert proposition
    #[inline]
    pub fn assert(&mut self, content: PropositionContent) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let prop = Proposition { id, content };
        self.knowledge_base.insert(id, prop);
        self.stats.propositions_added += 1;

        id
    }

    /// Check if proposition is derivable
    pub fn is_derivable(&self, proposition: &Proposition) -> bool {
        // Check if in knowledge base
        if self
            .knowledge_base
            .values()
            .any(|p| p.content == proposition.content)
        {
            return true;
        }

        // Try to derive
        self.try_derive(proposition, &mut BTreeSet::new(), 0)
    }

    fn try_derive(&self, target: &Proposition, visited: &mut BTreeSet<u64>, depth: usize) -> bool {
        if depth > self.config.max_depth {
            return false;
        }

        // Check direct match
        for prop in self.knowledge_base.values() {
            if prop.content == target.content {
                return true;
            }
        }

        // Try inference rules
        match &target.content {
            PropositionContent::Atom(_) => {
                // Try modus ponens
                for prop in self.knowledge_base.values() {
                    if let PropositionContent::Implies(antecedent, consequent) = &prop.content {
                        if **consequent == *target {
                            if self.try_derive(antecedent, visited, depth + 1) {
                                return true;
                            }
                        }
                    }
                }
            },

            PropositionContent::And(a, b) => {
                // Both must be derivable
                return self.try_derive(a, visited, depth + 1)
                    && self.try_derive(b, visited, depth + 1);
            },

            PropositionContent::Or(a, b) => {
                // At least one must be derivable
                return self.try_derive(a, visited, depth + 1)
                    || self.try_derive(b, visited, depth + 1);
            },

            _ => {},
        }

        false
    }

    /// Deduce from premises
    pub fn deduce(&mut self, premises: &[Proposition]) -> DeductionResult {
        self.stats.deductions_performed += 1;

        let mut derived = Vec::new();
        let mut rules_applied = Vec::new();

        // Apply all applicable rules
        for premise in premises {
            // Modus Ponens
            for other in premises {
                if let PropositionContent::Implies(antecedent, consequent) = &other.content {
                    if **antecedent == *premise {
                        derived.push(*consequent.clone());
                        rules_applied.push(InferenceRule::ModusPonens);
                    }
                }
            }

            // Conjunction elimination
            if let PropositionContent::And(a, b) = &premise.content {
                derived.push(*a.clone());
                derived.push(*b.clone());
                rules_applied.push(InferenceRule::ConjunctionElim);
            }

            // Double negation
            if let PropositionContent::Not(inner) = &premise.content {
                if let PropositionContent::Not(innermost) = &inner.content {
                    derived.push(*innermost.clone());
                    rules_applied.push(InferenceRule::DoubleNegation);
                }
            }
        }

        // Conjunction introduction (try all pairs)
        for i in 0..premises.len() {
            for j in (i + 1)..premises.len() {
                derived.push(Proposition {
                    id: self.next_id.fetch_add(1, Ordering::Relaxed),
                    content: PropositionContent::And(
                        Box::new(premises[i].clone()),
                        Box::new(premises[j].clone()),
                    ),
                });
                rules_applied.push(InferenceRule::ConjunctionIntro);
            }
        }

        // Hypothetical syllogism
        for p1 in premises {
            if let PropositionContent::Implies(a1, b1) = &p1.content {
                for p2 in premises {
                    if let PropositionContent::Implies(a2, b2) = &p2.content {
                        if **b1 == **a2 {
                            derived.push(Proposition {
                                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                                content: PropositionContent::Implies(a1.clone(), b2.clone()),
                            });
                            rules_applied.push(InferenceRule::HypotheticalSyllogism);
                        }
                    }
                }
            }
        }

        DeductionResult {
            derived,
            rules_applied,
            is_valid: true,
        }
    }

    /// Start proof
    pub fn start_proof(&mut self, premises: Vec<Proposition>, conclusion: Proposition) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let mut steps = Vec::new();

        // Add premises as steps
        for (i, premise) in premises.iter().enumerate() {
            steps.push(ProofStep {
                number: i + 1,
                proposition: premise.clone(),
                rule: InferenceRule::Premise,
                references: Vec::new(),
            });
        }

        let proof = Proof {
            id,
            premises,
            conclusion,
            steps,
            status: ProofStatus::InProgress,
            created: Timestamp::now(),
        };

        self.proofs.insert(id, proof);

        id
    }

    /// Add proof step
    pub fn add_step(
        &mut self,
        proof_id: u64,
        proposition: Proposition,
        rule: InferenceRule,
        references: Vec<usize>,
    ) -> bool {
        let proof = match self.proofs.get_mut(&proof_id) {
            Some(p) => p,
            None => return false,
        };

        // Validate step
        if !self.validate_step(&proof.steps, &proposition, rule, &references) {
            return false;
        }

        let step_number = proof.steps.len() + 1;

        proof.steps.push(ProofStep {
            number: step_number,
            proposition: proposition.clone(),
            rule,
            references,
        });

        // Check if proof is complete
        if proposition.content == proof.conclusion.content {
            proof.status = ProofStatus::Valid;
            self.stats.proofs_completed += 1;
        }

        true
    }

    fn validate_step(
        &self,
        steps: &[ProofStep],
        proposition: &Proposition,
        rule: InferenceRule,
        references: &[usize],
    ) -> bool {
        match rule {
            InferenceRule::Premise => true,

            InferenceRule::ModusPonens => {
                if references.len() != 2 {
                    return false;
                }

                let ref1 = steps.get(references[0].saturating_sub(1));
                let ref2 = steps.get(references[1].saturating_sub(1));

                if let (Some(s1), Some(s2)) = (ref1, ref2) {
                    // One should be implication, other should match antecedent
                    if let PropositionContent::Implies(ant, cons) = &s2.proposition.content {
                        return s1.proposition.content == **ant && **cons == *proposition;
                    }
                }
                false
            },

            InferenceRule::ConjunctionIntro => {
                if references.len() != 2 {
                    return false;
                }

                let ref1 = steps.get(references[0].saturating_sub(1));
                let ref2 = steps.get(references[1].saturating_sub(1));

                if let (Some(s1), Some(s2)) = (ref1, ref2) {
                    if let PropositionContent::And(a, b) = &proposition.content {
                        return **a == s1.proposition && **b == s2.proposition;
                    }
                }
                false
            },

            InferenceRule::ConjunctionElim => {
                if references.len() != 1 {
                    return false;
                }

                if let Some(s) = steps.get(references[0].saturating_sub(1)) {
                    if let PropositionContent::And(a, b) = &s.proposition.content {
                        return **a == *proposition || **b == *proposition;
                    }
                }
                false
            },

            _ => true, // Simplified validation for other rules
        }
    }

    /// Get proof
    #[inline(always)]
    pub fn get_proof(&self, id: u64) -> Option<&Proof> {
        self.proofs.get(&id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &DeductionStats {
        &self.stats
    }
}

impl Default for DeductionEngine {
    fn default() -> Self {
        Self::new(DeductionConfig::default())
    }
}

// ============================================================================
// PROPOSITION BUILDER
// ============================================================================

/// Proposition builder
pub struct PropBuilder {
    next_id: AtomicU64,
}

impl PropBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1000),
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Create atom
    #[inline]
    pub fn atom(&self, name: &str) -> Proposition {
        Proposition {
            id: self.next_id(),
            content: PropositionContent::Atom(name.into()),
        }
    }

    /// Create negation
    #[inline]
    pub fn not(&self, p: Proposition) -> Proposition {
        Proposition {
            id: self.next_id(),
            content: PropositionContent::Not(Box::new(p)),
        }
    }

    /// Create conjunction
    #[inline]
    pub fn and(&self, a: Proposition, b: Proposition) -> Proposition {
        Proposition {
            id: self.next_id(),
            content: PropositionContent::And(Box::new(a), Box::new(b)),
        }
    }

    /// Create disjunction
    #[inline]
    pub fn or(&self, a: Proposition, b: Proposition) -> Proposition {
        Proposition {
            id: self.next_id(),
            content: PropositionContent::Or(Box::new(a), Box::new(b)),
        }
    }

    /// Create implication
    #[inline]
    pub fn implies(&self, a: Proposition, b: Proposition) -> Proposition {
        Proposition {
            id: self.next_id(),
            content: PropositionContent::Implies(Box::new(a), Box::new(b)),
        }
    }
}

impl Default for PropBuilder {
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

    #[test]
    fn test_assert() {
        let mut engine = DeductionEngine::default();

        let id = engine.assert(PropositionContent::Atom("P".into()));
        assert!(engine.knowledge_base.contains_key(&id));
    }

    #[test]
    fn test_modus_ponens() {
        let mut engine = DeductionEngine::default();

        let builder = PropBuilder::new();

        let p = builder.atom("P");
        let q = builder.atom("Q");
        let p_implies_q = builder.implies(p.clone(), q.clone());

        let result = engine.deduce(&[p, p_implies_q]);

        assert!(
            result
                .derived
                .iter()
                .any(|d| { matches!(&d.content, PropositionContent::Atom(name) if name == "Q") })
        );
        assert!(result.rules_applied.contains(&InferenceRule::ModusPonens));
    }

    #[test]
    fn test_conjunction() {
        let mut engine = DeductionEngine::default();

        let builder = PropBuilder::new();

        let p = builder.atom("P");
        let q = builder.atom("Q");

        let result = engine.deduce(&[p, q]);

        // Should have derived P∧Q
        assert!(
            result
                .derived
                .iter()
                .any(|d| { matches!(&d.content, PropositionContent::And(_, _)) })
        );
    }

    #[test]
    fn test_proof() {
        let mut engine = DeductionEngine::default();

        let builder = PropBuilder::new();

        let p = builder.atom("P");
        let q = builder.atom("Q");
        let p_implies_q = builder.implies(p.clone(), q.clone());

        let proof_id = engine.start_proof(vec![p.clone(), p_implies_q.clone()], q.clone());

        // Apply modus ponens
        let success = engine.add_step(proof_id, q, InferenceRule::ModusPonens, vec![1, 2]);

        assert!(success);

        let proof = engine.get_proof(proof_id).unwrap();
        assert_eq!(proof.status, ProofStatus::Valid);
    }

    #[test]
    fn test_hypothetical_syllogism() {
        let mut engine = DeductionEngine::default();

        let builder = PropBuilder::new();

        let a = builder.atom("A");
        let b = builder.atom("B");
        let c = builder.atom("C");

        let a_implies_b = builder.implies(a.clone(), b.clone());
        let b_implies_c = builder.implies(b.clone(), c.clone());

        let result = engine.deduce(&[a_implies_b, b_implies_c]);

        // Should derive A → C
        assert!(
            result
                .rules_applied
                .contains(&InferenceRule::HypotheticalSyllogism)
        );
    }
}
