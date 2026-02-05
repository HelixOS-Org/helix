//! # Program Analysis Engine for NEXUS
//!
//! Year 2 "COGNITION" - Advanced program analysis capabilities for
//! deep code understanding, including abstract interpretation,
//! symbolic execution concepts, and program slicing.
//!
//! ## Features
//!
//! - Abstract interpretation framework
//! - Interval analysis
//! - Pointer analysis
//! - Program slicing
//! - Dependency analysis
//! - Side effect tracking

#![allow(dead_code)]
#![allow(clippy::derivable_impls)]
#![allow(clippy::only_used_in_recursion)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec;
use alloc::vec::Vec;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Variable identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarId(pub u32);

/// Instruction/statement identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StmtId(pub u32);

/// Block identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(pub u32);

/// Function identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FuncId(pub u32);

// ============================================================================
// ABSTRACT VALUES
// ============================================================================

/// Abstract value (lattice element)
#[derive(Debug, Clone, PartialEq)]
pub enum AbstractValue {
    /// Bottom (no information / unreachable)
    Bottom,
    /// Top (unknown / any value)
    Top,
    /// Constant integer value
    ConstInt(i64),
    /// Constant boolean
    ConstBool(bool),
    /// Integer interval [low, high]
    Interval(i64, i64),
    /// Set of possible values
    Set(BTreeSet<i64>),
    /// Null pointer
    NullPtr,
    /// Non-null pointer
    NonNullPtr,
    /// Points to a specific allocation
    PointsTo(AllocId),
    /// Sign abstraction
    Sign(SignValue),
}

/// Sign abstract domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignValue {
    /// Negative
    Negative,
    /// Zero
    Zero,
    /// Positive
    Positive,
    /// Non-negative (>= 0)
    NonNegative,
    /// Non-positive (<= 0)
    NonPositive,
    /// Non-zero (!= 0)
    NonZero,
}

/// Allocation identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AllocId(pub u32);

impl AbstractValue {
    /// Join two abstract values (least upper bound)
    pub fn join(&self, other: &AbstractValue) -> AbstractValue {
        match (self, other) {
            (AbstractValue::Bottom, v) | (v, AbstractValue::Bottom) => v.clone(),
            (AbstractValue::Top, _) | (_, AbstractValue::Top) => AbstractValue::Top,

            // Constant propagation
            (AbstractValue::ConstInt(a), AbstractValue::ConstInt(b)) => {
                if a == b {
                    AbstractValue::ConstInt(*a)
                } else {
                    AbstractValue::Interval(*a.min(b), *a.max(b))
                }
            },

            // Interval widening
            (AbstractValue::Interval(l1, h1), AbstractValue::Interval(l2, h2)) => {
                AbstractValue::Interval(*l1.min(l2), *h1.max(h2))
            },

            // Constant to interval
            (AbstractValue::ConstInt(c), AbstractValue::Interval(l, h))
            | (AbstractValue::Interval(l, h), AbstractValue::ConstInt(c)) => {
                AbstractValue::Interval(*l.min(c), *h.max(c))
            },

            // Pointer analysis
            (AbstractValue::NullPtr, AbstractValue::NullPtr) => AbstractValue::NullPtr,
            (AbstractValue::NonNullPtr, AbstractValue::NonNullPtr) => AbstractValue::NonNullPtr,
            (AbstractValue::NullPtr, AbstractValue::NonNullPtr)
            | (AbstractValue::NonNullPtr, AbstractValue::NullPtr) => AbstractValue::Top,

            (AbstractValue::PointsTo(a), AbstractValue::PointsTo(b)) => {
                if a == b {
                    AbstractValue::PointsTo(*a)
                } else {
                    AbstractValue::NonNullPtr
                }
            },

            // Default
            _ => AbstractValue::Top,
        }
    }

    /// Meet two abstract values (greatest lower bound)
    pub fn meet(&self, other: &AbstractValue) -> AbstractValue {
        match (self, other) {
            (AbstractValue::Top, v) | (v, AbstractValue::Top) => v.clone(),
            (AbstractValue::Bottom, _) | (_, AbstractValue::Bottom) => AbstractValue::Bottom,

            // Constant narrowing
            (AbstractValue::ConstInt(a), AbstractValue::ConstInt(b)) => {
                if a == b {
                    AbstractValue::ConstInt(*a)
                } else {
                    AbstractValue::Bottom
                }
            },

            // Interval intersection
            (AbstractValue::Interval(l1, h1), AbstractValue::Interval(l2, h2)) => {
                let new_l = *l1.max(l2);
                let new_h = *h1.min(h2);
                if new_l <= new_h {
                    AbstractValue::Interval(new_l, new_h)
                } else {
                    AbstractValue::Bottom
                }
            },

            _ => AbstractValue::Bottom,
        }
    }

    /// Check if this value is subsumed by another
    pub fn is_subsumed_by(&self, other: &AbstractValue) -> bool {
        match (self, other) {
            (_, AbstractValue::Top) => true,
            (AbstractValue::Bottom, _) => true,
            (AbstractValue::ConstInt(a), AbstractValue::ConstInt(b)) => a == b,
            (AbstractValue::ConstInt(c), AbstractValue::Interval(l, h)) => *c >= *l && *c <= *h,
            (AbstractValue::Interval(l1, h1), AbstractValue::Interval(l2, h2)) => {
                *l1 >= *l2 && *h1 <= *h2
            },
            _ => false,
        }
    }

    /// Is this value bottom?
    pub fn is_bottom(&self) -> bool {
        matches!(self, AbstractValue::Bottom)
    }

    /// Is this value top?
    pub fn is_top(&self) -> bool {
        matches!(self, AbstractValue::Top)
    }

    /// Is this a constant?
    pub fn is_constant(&self) -> bool {
        matches!(
            self,
            AbstractValue::ConstInt(_) | AbstractValue::ConstBool(_)
        )
    }

    /// Get constant value if known
    pub fn get_constant(&self) -> Option<i64> {
        match self {
            AbstractValue::ConstInt(c) => Some(*c),
            _ => None,
        }
    }
}

impl Default for AbstractValue {
    fn default() -> Self {
        AbstractValue::Top
    }
}

// ============================================================================
// ABSTRACT STATE
// ============================================================================

/// Abstract state (maps variables to abstract values)
#[derive(Debug, Clone)]
pub struct AbstractState {
    /// Variable values
    values: BTreeMap<VarId, AbstractValue>,
    /// Is this state reachable?
    reachable: bool,
}

impl AbstractState {
    /// Create a new abstract state
    pub fn new() -> Self {
        Self {
            values: BTreeMap::new(),
            reachable: true,
        }
    }

    /// Create an unreachable state
    pub fn unreachable() -> Self {
        Self {
            values: BTreeMap::new(),
            reachable: false,
        }
    }

    /// Set a variable's value
    pub fn set(&mut self, var: VarId, value: AbstractValue) {
        self.values.insert(var, value);
    }

    /// Get a variable's value
    pub fn get(&self, var: VarId) -> &AbstractValue {
        self.values.get(&var).unwrap_or(&AbstractValue::Top)
    }

    /// Join with another state
    pub fn join(&self, other: &AbstractState) -> AbstractState {
        if !self.reachable {
            return other.clone();
        }
        if !other.reachable {
            return self.clone();
        }

        let mut result = AbstractState::new();

        // Join all variables
        let all_vars: BTreeSet<VarId> = self
            .values
            .keys()
            .chain(other.values.keys())
            .copied()
            .collect();

        for var in all_vars {
            let v1 = self.get(var);
            let v2 = other.get(var);
            result.set(var, v1.join(v2));
        }

        result
    }

    /// Meet with another state
    pub fn meet(&self, other: &AbstractState) -> AbstractState {
        if !self.reachable || !other.reachable {
            return AbstractState::unreachable();
        }

        let mut result = AbstractState::new();

        let all_vars: BTreeSet<VarId> = self
            .values
            .keys()
            .chain(other.values.keys())
            .copied()
            .collect();

        for var in all_vars {
            let v1 = self.get(var);
            let v2 = other.get(var);
            let met = v1.meet(v2);
            if met.is_bottom() {
                return AbstractState::unreachable();
            }
            result.set(var, met);
        }

        result
    }

    /// Check if state is unreachable
    pub fn is_unreachable(&self) -> bool {
        !self.reachable
    }

    /// Check if subsumed by another state
    pub fn is_subsumed_by(&self, other: &AbstractState) -> bool {
        if !self.reachable {
            return true;
        }
        if !other.reachable {
            return false;
        }

        for (var, value) in &self.values {
            if !value.is_subsumed_by(other.get(*var)) {
                return false;
            }
        }
        true
    }
}

impl Default for AbstractState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ABSTRACT INTERPRETER
// ============================================================================

/// An abstract statement for interpretation
#[derive(Debug, Clone)]
pub enum AbstractStmt {
    /// x = constant
    AssignConst(VarId, i64),
    /// x = y
    AssignVar(VarId, VarId),
    /// x = y op z
    BinOp(VarId, VarId, BinaryOp, VarId),
    /// x = op y
    UnaryOp(VarId, UnaryOp, VarId),
    /// x = call f(args)
    Call(VarId, FuncId, Vec<VarId>),
    /// Assume condition is true
    Assume(VarId, Comparison, i64),
    /// Skip (no-op)
    Skip,
}

/// Binary operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Xor,
    Shl,
    Shr,
}

/// Unary operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// Comparison operator for assume
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Abstract interpreter
pub struct AbstractInterpreter {
    /// Widening threshold
    widening_threshold: usize,
    /// Maximum iterations
    max_iterations: usize,
    /// Enable narrowing
    narrowing_enabled: bool,
}

impl AbstractInterpreter {
    /// Create new interpreter
    pub fn new() -> Self {
        Self {
            widening_threshold: 3,
            max_iterations: 100,
            narrowing_enabled: true,
        }
    }

    /// Interpret a statement
    pub fn interpret(&self, stmt: &AbstractStmt, state: &AbstractState) -> AbstractState {
        if state.is_unreachable() {
            return state.clone();
        }

        let mut result = state.clone();

        match stmt {
            AbstractStmt::AssignConst(var, c) => {
                result.set(*var, AbstractValue::ConstInt(*c));
            },

            AbstractStmt::AssignVar(dst, src) => {
                let value = state.get(*src).clone();
                result.set(*dst, value);
            },

            AbstractStmt::BinOp(dst, lhs, op, rhs) => {
                let lhs_val = state.get(*lhs);
                let rhs_val = state.get(*rhs);
                let result_val = self.eval_binop(op, lhs_val, rhs_val);
                result.set(*dst, result_val);
            },

            AbstractStmt::UnaryOp(dst, op, src) => {
                let src_val = state.get(*src);
                let result_val = self.eval_unaryop(op, src_val);
                result.set(*dst, result_val);
            },

            AbstractStmt::Call(dst, _func, _args) => {
                // Conservative: result is unknown
                result.set(*dst, AbstractValue::Top);
            },

            AbstractStmt::Assume(var, cmp, constant) => {
                let current = state.get(*var);
                let refined = self.refine_with_assumption(current, cmp, *constant);
                if refined.is_bottom() {
                    return AbstractState::unreachable();
                }
                result.set(*var, refined);
            },

            AbstractStmt::Skip => {},
        }

        result
    }

    /// Evaluate binary operation abstractly
    fn eval_binop(&self, op: &BinaryOp, lhs: &AbstractValue, rhs: &AbstractValue) -> AbstractValue {
        match (lhs, rhs) {
            (AbstractValue::ConstInt(a), AbstractValue::ConstInt(b)) => {
                let result = match op {
                    BinaryOp::Add => a.wrapping_add(*b),
                    BinaryOp::Sub => a.wrapping_sub(*b),
                    BinaryOp::Mul => a.wrapping_mul(*b),
                    BinaryOp::Div => {
                        if *b == 0 {
                            return AbstractValue::Top;
                        }
                        a.wrapping_div(*b)
                    },
                    BinaryOp::Mod => {
                        if *b == 0 {
                            return AbstractValue::Top;
                        }
                        a.wrapping_rem(*b)
                    },
                    BinaryOp::And => a & b,
                    BinaryOp::Or => a | b,
                    BinaryOp::Xor => a ^ b,
                    BinaryOp::Shl => a << (b & 63),
                    BinaryOp::Shr => a >> (b & 63),
                };
                AbstractValue::ConstInt(result)
            },

            (AbstractValue::Interval(l1, h1), AbstractValue::Interval(l2, h2)) => {
                match op {
                    BinaryOp::Add => {
                        AbstractValue::Interval(l1.saturating_add(*l2), h1.saturating_add(*h2))
                    },
                    BinaryOp::Sub => {
                        AbstractValue::Interval(l1.saturating_sub(*h2), h1.saturating_sub(*l2))
                    },
                    BinaryOp::Mul => {
                        // Product of intervals
                        let products = [l1 * l2, l1 * h2, h1 * l2, h1 * h2];
                        let min = *products.iter().min().unwrap();
                        let max = *products.iter().max().unwrap();
                        AbstractValue::Interval(min, max)
                    },
                    _ => AbstractValue::Top,
                }
            },

            (AbstractValue::ConstInt(c), AbstractValue::Interval(l, h))
            | (AbstractValue::Interval(l, h), AbstractValue::ConstInt(c)) => {
                // Treat constant as singleton interval
                let c_int = AbstractValue::Interval(*c, *c);
                let other_int = AbstractValue::Interval(*l, *h);
                self.eval_binop(op, &c_int, &other_int)
            },

            _ => AbstractValue::Top,
        }
    }

    /// Evaluate unary operation abstractly
    fn eval_unaryop(&self, op: &UnaryOp, src: &AbstractValue) -> AbstractValue {
        match (op, src) {
            (UnaryOp::Neg, AbstractValue::ConstInt(c)) => AbstractValue::ConstInt(-c),
            (UnaryOp::Neg, AbstractValue::Interval(l, h)) => AbstractValue::Interval(-h, -l),
            (UnaryOp::Not, AbstractValue::ConstInt(c)) => AbstractValue::ConstInt(!c),
            (UnaryOp::Not, AbstractValue::ConstBool(b)) => AbstractValue::ConstBool(!b),
            _ => AbstractValue::Top,
        }
    }

    /// Refine value with assumption
    fn refine_with_assumption(
        &self,
        current: &AbstractValue,
        cmp: &Comparison,
        constant: i64,
    ) -> AbstractValue {
        match current {
            AbstractValue::Top => {
                // Create interval from assumption
                match cmp {
                    Comparison::Eq => AbstractValue::ConstInt(constant),
                    Comparison::Ne => AbstractValue::Top, // Can't refine much
                    Comparison::Lt => AbstractValue::Interval(i64::MIN, constant - 1),
                    Comparison::Le => AbstractValue::Interval(i64::MIN, constant),
                    Comparison::Gt => AbstractValue::Interval(constant + 1, i64::MAX),
                    Comparison::Ge => AbstractValue::Interval(constant, i64::MAX),
                }
            },

            AbstractValue::Interval(l, h) => {
                match cmp {
                    Comparison::Eq => {
                        if constant >= *l && constant <= *h {
                            AbstractValue::ConstInt(constant)
                        } else {
                            AbstractValue::Bottom
                        }
                    },
                    Comparison::Ne => {
                        // Narrow slightly if at boundary
                        if *l == constant && *h == constant {
                            AbstractValue::Bottom
                        } else if *l == constant {
                            AbstractValue::Interval(l + 1, *h)
                        } else if *h == constant {
                            AbstractValue::Interval(*l, h - 1)
                        } else {
                            current.clone()
                        }
                    },
                    Comparison::Lt => AbstractValue::Interval(*l, (constant - 1).min(*h)),
                    Comparison::Le => AbstractValue::Interval(*l, constant.min(*h)),
                    Comparison::Gt => AbstractValue::Interval((constant + 1).max(*l), *h),
                    Comparison::Ge => AbstractValue::Interval(constant.max(*l), *h),
                }
            },

            AbstractValue::ConstInt(c) => {
                let holds = match cmp {
                    Comparison::Eq => *c == constant,
                    Comparison::Ne => *c != constant,
                    Comparison::Lt => *c < constant,
                    Comparison::Le => *c <= constant,
                    Comparison::Gt => *c > constant,
                    Comparison::Ge => *c >= constant,
                };

                if holds {
                    current.clone()
                } else {
                    AbstractValue::Bottom
                }
            },

            _ => current.clone(),
        }
    }

    /// Widen value (to ensure termination)
    pub fn widen(&self, old: &AbstractValue, new: &AbstractValue) -> AbstractValue {
        match (old, new) {
            (AbstractValue::Interval(l1, h1), AbstractValue::Interval(l2, h2)) => {
                let new_l = if *l2 < *l1 { i64::MIN } else { *l1 };
                let new_h = if *h2 > *h1 { i64::MAX } else { *h1 };
                AbstractValue::Interval(new_l, new_h)
            },

            _ => old.join(new),
        }
    }

    /// Narrow value (after widening)
    pub fn narrow(&self, old: &AbstractValue, new: &AbstractValue) -> AbstractValue {
        match (old, new) {
            (AbstractValue::Interval(l1, h1), AbstractValue::Interval(l2, h2)) => {
                let new_l = if *l1 == i64::MIN { *l2 } else { *l1 };
                let new_h = if *h1 == i64::MAX { *h2 } else { *h1 };
                AbstractValue::Interval(new_l, new_h)
            },

            _ => new.clone(),
        }
    }
}

impl Default for AbstractInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PROGRAM SLICING
// ============================================================================

/// A program slice (subset of program affecting a criterion)
#[derive(Debug, Clone)]
pub struct ProgramSlice {
    /// Slice criterion (variable at statement)
    pub criterion: (StmtId, VarId),
    /// Included statements
    pub statements: BTreeSet<StmtId>,
    /// Relevant variables
    pub variables: BTreeSet<VarId>,
    /// Is this a backward slice?
    pub backward: bool,
}

impl ProgramSlice {
    /// Create new slice
    pub fn new(stmt: StmtId, var: VarId, backward: bool) -> Self {
        let mut statements = BTreeSet::new();
        statements.insert(stmt);

        let mut variables = BTreeSet::new();
        variables.insert(var);

        Self {
            criterion: (stmt, var),
            statements,
            variables,
            backward,
        }
    }

    /// Add statement to slice
    pub fn add_statement(&mut self, stmt: StmtId) {
        self.statements.insert(stmt);
    }

    /// Add variable to slice
    pub fn add_variable(&mut self, var: VarId) {
        self.variables.insert(var);
    }

    /// Check if statement is in slice
    pub fn contains_statement(&self, stmt: StmtId) -> bool {
        self.statements.contains(&stmt)
    }

    /// Get slice size
    pub fn size(&self) -> usize {
        self.statements.len()
    }
}

/// Program slicer using data/control dependencies
pub struct ProgramSlicer {
    /// Data dependencies: stmt -> (stmts it depends on)
    data_deps: BTreeMap<StmtId, BTreeSet<StmtId>>,
    /// Control dependencies: stmt -> (controlling stmts)
    control_deps: BTreeMap<StmtId, BTreeSet<StmtId>>,
    /// Definitions: stmt -> (variables it defines)
    definitions: BTreeMap<StmtId, BTreeSet<VarId>>,
    /// Uses: stmt -> (variables it uses)
    uses: BTreeMap<StmtId, BTreeSet<VarId>>,
}

impl ProgramSlicer {
    /// Create new slicer
    pub fn new() -> Self {
        Self {
            data_deps: BTreeMap::new(),
            control_deps: BTreeMap::new(),
            definitions: BTreeMap::new(),
            uses: BTreeMap::new(),
        }
    }

    /// Add data dependency
    pub fn add_data_dep(&mut self, from: StmtId, to: StmtId) {
        self.data_deps.entry(from).or_default().insert(to);
    }

    /// Add control dependency
    pub fn add_control_dep(&mut self, stmt: StmtId, controller: StmtId) {
        self.control_deps
            .entry(stmt)
            .or_default()
            .insert(controller);
    }

    /// Add definition
    pub fn add_definition(&mut self, stmt: StmtId, var: VarId) {
        self.definitions.entry(stmt).or_default().insert(var);
    }

    /// Add use
    pub fn add_use(&mut self, stmt: StmtId, var: VarId) {
        self.uses.entry(stmt).or_default().insert(var);
    }

    /// Compute backward slice from criterion
    pub fn backward_slice(&self, criterion_stmt: StmtId, criterion_var: VarId) -> ProgramSlice {
        let mut slice = ProgramSlice::new(criterion_stmt, criterion_var, true);
        let mut worklist = vec![criterion_stmt];
        let mut visited = BTreeSet::new();

        while let Some(stmt) = worklist.pop() {
            if !visited.insert(stmt) {
                continue;
            }

            slice.add_statement(stmt);

            // Add all uses as relevant variables
            if let Some(used_vars) = self.uses.get(&stmt) {
                for &var in used_vars {
                    slice.add_variable(var);
                }
            }

            // Follow data dependencies
            if let Some(deps) = self.data_deps.get(&stmt) {
                for &dep in deps {
                    worklist.push(dep);
                }
            }

            // Follow control dependencies
            if let Some(ctrl_deps) = self.control_deps.get(&stmt) {
                for &ctrl in ctrl_deps {
                    worklist.push(ctrl);
                }
            }
        }

        slice
    }

    /// Compute forward slice from criterion
    pub fn forward_slice(&self, criterion_stmt: StmtId, criterion_var: VarId) -> ProgramSlice {
        let mut slice = ProgramSlice::new(criterion_stmt, criterion_var, false);

        // Build reverse dependencies
        let mut reverse_data: BTreeMap<StmtId, BTreeSet<StmtId>> = BTreeMap::new();
        for (&stmt, deps) in &self.data_deps {
            for &dep in deps {
                reverse_data.entry(dep).or_default().insert(stmt);
            }
        }

        let mut worklist = vec![criterion_stmt];
        let mut visited = BTreeSet::new();

        while let Some(stmt) = worklist.pop() {
            if !visited.insert(stmt) {
                continue;
            }

            slice.add_statement(stmt);

            // Add definitions as relevant variables
            if let Some(defs) = self.definitions.get(&stmt) {
                for &var in defs {
                    slice.add_variable(var);
                }
            }

            // Follow reverse data dependencies
            if let Some(dependents) = reverse_data.get(&stmt) {
                for &dep in dependents {
                    worklist.push(dep);
                }
            }
        }

        slice
    }
}

impl Default for ProgramSlicer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SIDE EFFECT ANALYSIS
// ============================================================================

/// Side effect of a function/statement
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideEffect {
    /// Modifies a global variable
    ModifiesGlobal(VarId),
    /// Reads a global variable
    ReadsGlobal(VarId),
    /// Allocates memory
    Allocates,
    /// Deallocates memory
    Deallocates,
    /// Performs I/O
    PerformsIO,
    /// Modifies a parameter (by reference)
    ModifiesParam(usize),
    /// Throws exception
    MayThrow,
    /// Diverges (non-termination)
    MayDiverge,
    /// Acquires lock
    AcquiresLock(VarId),
    /// Releases lock
    ReleasesLock(VarId),
}

/// Side effect summary for a function
#[derive(Debug, Clone, Default)]
pub struct SideEffectSummary {
    /// Function ID
    pub func: Option<FuncId>,
    /// All side effects
    pub effects: Vec<SideEffect>,
    /// Is pure (no side effects)?
    pub is_pure: bool,
    /// Is total (always terminates)?
    pub is_total: bool,
}

impl SideEffectSummary {
    /// Create new summary
    pub fn new(func: FuncId) -> Self {
        Self {
            func: Some(func),
            effects: Vec::new(),
            is_pure: true,
            is_total: true,
        }
    }

    /// Add side effect
    pub fn add_effect(&mut self, effect: SideEffect) {
        self.is_pure = false;
        if matches!(effect, SideEffect::MayDiverge) {
            self.is_total = false;
        }
        self.effects.push(effect);
    }

    /// Check if function modifies any global
    pub fn modifies_globals(&self) -> bool {
        self.effects
            .iter()
            .any(|e| matches!(e, SideEffect::ModifiesGlobal(_)))
    }

    /// Check if function allocates
    pub fn allocates(&self) -> bool {
        self.effects
            .iter()
            .any(|e| matches!(e, SideEffect::Allocates))
    }

    /// Get modified globals
    pub fn get_modified_globals(&self) -> Vec<VarId> {
        self.effects
            .iter()
            .filter_map(|e| match e {
                SideEffect::ModifiesGlobal(v) => Some(*v),
                _ => None,
            })
            .collect()
    }
}

/// Side effect analyzer
pub struct SideEffectAnalyzer {
    /// Function summaries
    summaries: BTreeMap<FuncId, SideEffectSummary>,
    /// Call graph
    call_graph: BTreeMap<FuncId, BTreeSet<FuncId>>,
}

impl SideEffectAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            summaries: BTreeMap::new(),
            call_graph: BTreeMap::new(),
        }
    }

    /// Add function summary
    pub fn add_summary(&mut self, func: FuncId, summary: SideEffectSummary) {
        self.summaries.insert(func, summary);
    }

    /// Add call edge
    pub fn add_call(&mut self, caller: FuncId, callee: FuncId) {
        self.call_graph.entry(caller).or_default().insert(callee);
    }

    /// Get transitive effects (including callees)
    pub fn transitive_effects(&self, func: FuncId) -> SideEffectSummary {
        let mut visited = BTreeSet::new();
        let mut combined = SideEffectSummary::new(func);

        self.collect_effects(func, &mut combined, &mut visited);

        combined
    }

    fn collect_effects(
        &self,
        func: FuncId,
        combined: &mut SideEffectSummary,
        visited: &mut BTreeSet<FuncId>,
    ) {
        if !visited.insert(func) {
            return;
        }

        // Add direct effects
        if let Some(summary) = self.summaries.get(&func) {
            for effect in &summary.effects {
                combined.add_effect(effect.clone());
            }
        }

        // Add effects from callees
        if let Some(callees) = self.call_graph.get(&func) {
            for &callee in callees {
                self.collect_effects(callee, combined, visited);
            }
        }
    }

    /// Check if function is pure
    pub fn is_pure(&self, func: FuncId) -> bool {
        self.transitive_effects(func).is_pure
    }

    /// Check if function is total
    pub fn is_total(&self, func: FuncId) -> bool {
        self.transitive_effects(func).is_total
    }
}

impl Default for SideEffectAnalyzer {
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
    fn test_abstract_value_join() {
        let v1 = AbstractValue::ConstInt(5);
        let v2 = AbstractValue::ConstInt(10);

        let joined = v1.join(&v2);
        assert_eq!(joined, AbstractValue::Interval(5, 10));
    }

    #[test]
    fn test_abstract_value_meet() {
        let v1 = AbstractValue::Interval(0, 10);
        let v2 = AbstractValue::Interval(5, 15);

        let met = v1.meet(&v2);
        assert_eq!(met, AbstractValue::Interval(5, 10));
    }

    #[test]
    fn test_abstract_state() {
        let mut state = AbstractState::new();

        state.set(VarId(0), AbstractValue::ConstInt(42));
        assert_eq!(state.get(VarId(0)).get_constant(), Some(42));

        // Unknown variable should be Top
        assert!(state.get(VarId(1)).is_top());
    }

    #[test]
    fn test_abstract_interpreter() {
        let interp = AbstractInterpreter::new();
        let mut state = AbstractState::new();

        // x = 5
        state = interp.interpret(&AbstractStmt::AssignConst(VarId(0), 5), &state);
        assert_eq!(state.get(VarId(0)).get_constant(), Some(5));

        // y = x
        state = interp.interpret(&AbstractStmt::AssignVar(VarId(1), VarId(0)), &state);
        assert_eq!(state.get(VarId(1)).get_constant(), Some(5));

        // z = x + y
        state = interp.interpret(
            &AbstractStmt::BinOp(VarId(2), VarId(0), BinaryOp::Add, VarId(1)),
            &state,
        );
        assert_eq!(state.get(VarId(2)).get_constant(), Some(10));
    }

    #[test]
    fn test_assume() {
        let interp = AbstractInterpreter::new();
        let mut state = AbstractState::new();

        // x is unknown (Top)
        // assume x < 10
        state = interp.interpret(&AbstractStmt::Assume(VarId(0), Comparison::Lt, 10), &state);

        match state.get(VarId(0)) {
            AbstractValue::Interval(_, h) => assert!(*h <= 9),
            _ => panic!("Expected interval"),
        }
    }

    #[test]
    fn test_program_slice() {
        let mut slicer = ProgramSlicer::new();

        // Simple program:
        // S0: x = 1
        // S1: y = 2
        // S2: z = x + y
        // S3: w = z * 2

        slicer.add_definition(StmtId(0), VarId(0)); // x
        slicer.add_definition(StmtId(1), VarId(1)); // y
        slicer.add_definition(StmtId(2), VarId(2)); // z
        slicer.add_definition(StmtId(3), VarId(3)); // w

        slicer.add_use(StmtId(2), VarId(0)); // z uses x
        slicer.add_use(StmtId(2), VarId(1)); // z uses y
        slicer.add_use(StmtId(3), VarId(2)); // w uses z

        slicer.add_data_dep(StmtId(2), StmtId(0)); // S2 depends on S0
        slicer.add_data_dep(StmtId(2), StmtId(1)); // S2 depends on S1
        slicer.add_data_dep(StmtId(3), StmtId(2)); // S3 depends on S2

        // Backward slice from S3, w
        let slice = slicer.backward_slice(StmtId(3), VarId(3));

        assert!(slice.contains_statement(StmtId(0)));
        assert!(slice.contains_statement(StmtId(1)));
        assert!(slice.contains_statement(StmtId(2)));
        assert!(slice.contains_statement(StmtId(3)));
        assert_eq!(slice.size(), 4);
    }

    #[test]
    fn test_side_effect_summary() {
        let mut summary = SideEffectSummary::new(FuncId(0));

        assert!(summary.is_pure);

        summary.add_effect(SideEffect::ModifiesGlobal(VarId(0)));
        assert!(!summary.is_pure);
        assert!(summary.modifies_globals());

        summary.add_effect(SideEffect::Allocates);
        assert!(summary.allocates());
    }
}
