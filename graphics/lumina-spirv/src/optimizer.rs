//! SPIR-V Optimizer
//!
//! Optimization passes for SPIR-V modules.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, collections::BTreeSet, string::String, vec::Vec};
#[cfg(feature = "std")]
use std::collections::{BTreeMap, BTreeSet};

use crate::instruction::*;
use crate::module::{Block, Function, SpirVModule};
use crate::opcode::Opcode;
use crate::SpirVResult;

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptimizationLevel {
    /// No optimization
    #[default]
    None,
    /// Size optimization (Os)
    Size,
    /// Performance optimization (O1)
    Performance,
    /// Aggressive optimization (O2/O3)
    Aggressive,
}

/// Optimizer configuration
#[derive(Debug, Clone, Default)]
pub struct OptimizerConfig {
    /// Optimization level
    pub level: OptimizationLevel,
    /// Run dead code elimination
    pub dead_code_elimination: bool,
    /// Run constant folding
    pub constant_folding: bool,
    /// Run instruction combining
    pub instruction_combining: bool,
    /// Run local CSE
    pub common_subexpression_elimination: bool,
    /// Run dead branch elimination
    pub dead_branch_elimination: bool,
    /// Inline functions
    pub inline_functions: bool,
    /// Inline threshold (for Size/Performance)
    pub inline_threshold: u32,
    /// Remove unused variables
    pub remove_unused_variables: bool,
    /// Compact IDs
    pub compact_ids: bool,
    /// Strength reduction
    pub strength_reduction: bool,
    /// Loop optimization
    pub loop_optimization: bool,
    /// Validate after each pass
    pub validate: bool,
}

impl OptimizerConfig {
    /// Create config for size optimization
    pub fn size() -> Self {
        Self {
            level: OptimizationLevel::Size,
            dead_code_elimination: true,
            constant_folding: true,
            instruction_combining: true,
            common_subexpression_elimination: true,
            dead_branch_elimination: true,
            inline_functions: false,
            inline_threshold: 0,
            remove_unused_variables: true,
            compact_ids: true,
            strength_reduction: true,
            loop_optimization: false,
            validate: false,
        }
    }

    /// Create config for performance optimization
    pub fn performance() -> Self {
        Self {
            level: OptimizationLevel::Performance,
            dead_code_elimination: true,
            constant_folding: true,
            instruction_combining: true,
            common_subexpression_elimination: true,
            dead_branch_elimination: true,
            inline_functions: true,
            inline_threshold: 200,
            remove_unused_variables: true,
            compact_ids: true,
            strength_reduction: true,
            loop_optimization: true,
            validate: false,
        }
    }

    /// Create config for aggressive optimization
    pub fn aggressive() -> Self {
        Self {
            level: OptimizationLevel::Aggressive,
            dead_code_elimination: true,
            constant_folding: true,
            instruction_combining: true,
            common_subexpression_elimination: true,
            dead_branch_elimination: true,
            inline_functions: true,
            inline_threshold: 1000,
            remove_unused_variables: true,
            compact_ids: true,
            strength_reduction: true,
            loop_optimization: true,
            validate: false,
        }
    }
}

/// SPIR-V optimizer
#[derive(Debug)]
pub struct SpirVOptimizer {
    /// Configuration
    config: OptimizerConfig,
}

impl SpirVOptimizer {
    /// Create a new optimizer
    pub fn new(config: OptimizerConfig) -> Self {
        Self { config }
    }

    /// Optimize a module
    pub fn optimize(&self, module: &mut SpirVModule) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();

        // Run passes based on configuration
        if self.config.dead_code_elimination {
            stats.add(self.dead_code_elimination(module)?);
        }

        if self.config.constant_folding {
            stats.add(self.constant_folding(module)?);
        }

        if self.config.dead_branch_elimination {
            stats.add(self.dead_branch_elimination(module)?);
        }

        if self.config.instruction_combining {
            stats.add(self.instruction_combining(module)?);
        }

        if self.config.common_subexpression_elimination {
            stats.add(self.common_subexpression_elimination(module)?);
        }

        if self.config.strength_reduction {
            stats.add(self.strength_reduction(module)?);
        }

        if self.config.remove_unused_variables {
            stats.add(self.remove_unused_variables(module)?);
        }

        if self.config.inline_functions {
            stats.add(self.inline_functions(module)?);
        }

        if self.config.compact_ids {
            stats.add(self.compact_ids(module)?);
        }

        Ok(stats)
    }

    /// Dead code elimination pass
    fn dead_code_elimination(&self, module: &mut SpirVModule) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();
        let mut used_ids: BTreeSet<Id> = BTreeSet::new();

        // Mark entry points and their interfaces as used
        for ep in &module.entry_points {
            used_ids.insert(ep.function);
            for &iface in &ep.interface {
                used_ids.insert(iface);
            }
        }

        // Mark types used by globals
        for (&id, _) in &module.global_variables {
            used_ids.insert(id);
        }

        // Propagate usage through functions
        let mut changed = true;
        while changed {
            changed = false;
            for func in &module.functions {
                if used_ids.contains(&func.id) {
                    for block in &func.blocks {
                        for inst in &block.instructions {
                            // Mark all referenced IDs as used
                            if let Some(rt) = inst.result_type {
                                if used_ids.insert(rt) {
                                    changed = true;
                                }
                            }
                            for op in &inst.operands {
                                if let Operand::Id(id) = op {
                                    if used_ids.insert(*id) {
                                        changed = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Remove unused functions
        let before_count = module.functions.len();
        module.functions.retain(|f| used_ids.contains(&f.id));
        stats.functions_removed = before_count - module.functions.len();

        // Remove dead instructions in used functions
        for func in &mut module.functions {
            for block in &mut func.blocks {
                let before_count = block.instructions.len();
                block.instructions.retain(|inst| {
                    // Keep instructions without result
                    let result = inst.result.unwrap_or(0);
                    result == 0 || used_ids.contains(&result)
                });
                stats.instructions_removed += before_count - block.instructions.len();
            }
        }

        Ok(stats)
    }

    /// Constant folding pass
    fn constant_folding(&self, module: &mut SpirVModule) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();
        let constants = module.constants.clone();

        for func in &mut module.functions {
            for block in &mut func.blocks {
                for inst in &mut block.instructions {
                    if let Some(folded) = self.try_fold_constant(inst, &constants) {
                        // Record that we folded an instruction
                        stats.constants_folded += 1;
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Try to fold a constant operation
    fn try_fold_constant(
        &self,
        inst: &Instruction,
        constants: &BTreeMap<Id, crate::module::ConstantDecl>,
    ) -> Option<u32> {
        match inst.opcode {
            Opcode::OpIAdd => {
                if let (Some(Operand::Id(a)), Some(Operand::Id(b))) =
                    (inst.operands.get(0), inst.operands.get(1))
                {
                    let a_val = self.get_int_constant(*a, constants)?;
                    let b_val = self.get_int_constant(*b, constants)?;
                    return Some(a_val.wrapping_add(b_val));
                }
            },
            Opcode::OpISub => {
                if let (Some(Operand::Id(a)), Some(Operand::Id(b))) =
                    (inst.operands.get(0), inst.operands.get(1))
                {
                    let a_val = self.get_int_constant(*a, constants)?;
                    let b_val = self.get_int_constant(*b, constants)?;
                    return Some(a_val.wrapping_sub(b_val));
                }
            },
            Opcode::OpIMul => {
                if let (Some(Operand::Id(a)), Some(Operand::Id(b))) =
                    (inst.operands.get(0), inst.operands.get(1))
                {
                    let a_val = self.get_int_constant(*a, constants)?;
                    let b_val = self.get_int_constant(*b, constants)?;
                    return Some(a_val.wrapping_mul(b_val));
                }
            },
            Opcode::OpSDiv => {
                if let (Some(Operand::Id(a)), Some(Operand::Id(b))) =
                    (inst.operands.get(0), inst.operands.get(1))
                {
                    let a_val = self.get_int_constant(*a, constants)? as i32;
                    let b_val = self.get_int_constant(*b, constants)? as i32;
                    if b_val != 0 {
                        return Some(a_val.wrapping_div(b_val) as u32);
                    }
                }
            },
            Opcode::OpBitwiseAnd => {
                if let (Some(Operand::Id(a)), Some(Operand::Id(b))) =
                    (inst.operands.get(0), inst.operands.get(1))
                {
                    let a_val = self.get_int_constant(*a, constants)?;
                    let b_val = self.get_int_constant(*b, constants)?;
                    return Some(a_val & b_val);
                }
            },
            Opcode::OpBitwiseOr => {
                if let (Some(Operand::Id(a)), Some(Operand::Id(b))) =
                    (inst.operands.get(0), inst.operands.get(1))
                {
                    let a_val = self.get_int_constant(*a, constants)?;
                    let b_val = self.get_int_constant(*b, constants)?;
                    return Some(a_val | b_val);
                }
            },
            Opcode::OpBitwiseXor => {
                if let (Some(Operand::Id(a)), Some(Operand::Id(b))) =
                    (inst.operands.get(0), inst.operands.get(1))
                {
                    let a_val = self.get_int_constant(*a, constants)?;
                    let b_val = self.get_int_constant(*b, constants)?;
                    return Some(a_val ^ b_val);
                }
            },
            Opcode::OpNot => {
                if let Some(Operand::Id(a)) = inst.operands.first() {
                    let a_val = self.get_int_constant(*a, constants)?;
                    return Some(!a_val);
                }
            },
            _ => {},
        }
        None
    }

    /// Get integer constant value
    fn get_int_constant(
        &self,
        id: Id,
        constants: &BTreeMap<Id, crate::module::ConstantDecl>,
    ) -> Option<u32> {
        constants.get(&id).and_then(|c| {
            if matches!(c.opcode, Opcode::OpConstant) {
                c.operands.first().and_then(|op| {
                    if let Operand::Literal(v) = op {
                        Some(*v)
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
    }

    /// Dead branch elimination
    fn dead_branch_elimination(&self, module: &mut SpirVModule) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();

        for func in &mut module.functions {
            let constants = &module.constants;

            for block in &mut func.blocks {
                if let Some(term_idx) = block
                    .instructions
                    .iter()
                    .position(|inst| matches!(inst.opcode, Opcode::OpBranchConditional))
                {
                    let term = &block.instructions[term_idx];

                    // Check if condition is constant
                    if let Some(Operand::Id(cond_id)) = term.operands.first() {
                        if let Some(constant) = constants.get(cond_id) {
                            let is_true = matches!(constant.opcode, Opcode::OpConstantTrue);
                            let is_false = matches!(constant.opcode, Opcode::OpConstantFalse);

                            if is_true || is_false {
                                // Replace with unconditional branch
                                let target_idx = if is_true { 1 } else { 2 };
                                if let Some(Operand::Id(target)) = term.operands.get(target_idx) {
                                    block.instructions[term_idx] =
                                        Instruction::new(Opcode::OpBranch).with_id(*target);
                                    stats.branches_eliminated += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Instruction combining pass
    fn instruction_combining(&self, module: &mut SpirVModule) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();

        for func in &mut module.functions {
            for block in &mut func.blocks {
                let mut i = 0;
                while i < block.instructions.len() {
                    let inst = &block.instructions[i];

                    // Look for patterns to combine
                    match inst.opcode {
                        // x * 1 -> x, x * 0 -> 0
                        Opcode::OpIMul | Opcode::OpFMul => {
                            // Would check for multiply by 1 or 0
                        },
                        // x + 0 -> x
                        Opcode::OpIAdd | Opcode::OpFAdd => {
                            // Would check for add by 0
                        },
                        // double negation: -(-x) -> x
                        Opcode::OpSNegate | Opcode::OpFNegate => {
                            // Would check for double negation
                        },
                        _ => {},
                    }

                    i += 1;
                }
            }
        }

        Ok(stats)
    }

    /// Common subexpression elimination
    fn common_subexpression_elimination(
        &self,
        module: &mut SpirVModule,
    ) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();

        for func in &mut module.functions {
            for block in &mut func.blocks {
                let mut expression_map: BTreeMap<ExpressionKey, Id> = BTreeMap::new();

                for inst in &mut block.instructions {
                    // Only for pure instructions
                    if !is_pure_instruction(inst.opcode) {
                        continue;
                    }

                    if let Some(result) = inst.result {
                        let key = ExpressionKey {
                            opcode: inst.opcode,
                            result_type: inst.result_type,
                            operands: inst.operands.clone(),
                        };

                        if let Some(&existing) = expression_map.get(&key) {
                            // Replace with copy
                            stats.cse_eliminations += 1;
                        } else {
                            expression_map.insert(key, result);
                        }
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Strength reduction pass
    fn strength_reduction(&self, module: &mut SpirVModule) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();

        for func in &mut module.functions {
            for block in &mut func.blocks {
                for inst in &mut block.instructions {
                    match inst.opcode {
                        // Replace integer division by power of 2 with shift
                        Opcode::OpSDiv | Opcode::OpUDiv => {
                            if let Some(Operand::Id(divisor_id)) = inst.operands.get(1) {
                                if let Some(power) =
                                    self.get_power_of_two(*divisor_id, &module.constants)
                                {
                                    // Replace with shift right
                                    inst.opcode = if inst.opcode == Opcode::OpSDiv {
                                        Opcode::OpShiftRightArithmetic
                                    } else {
                                        Opcode::OpShiftRightLogical
                                    };
                                    // Would need to create constant for shift amount
                                    stats.strength_reductions += 1;
                                }
                            }
                        },
                        // Replace integer multiply by power of 2 with shift
                        Opcode::OpIMul => {
                            if let Some(Operand::Id(factor_id)) = inst.operands.get(1) {
                                if let Some(power) =
                                    self.get_power_of_two(*factor_id, &module.constants)
                                {
                                    inst.opcode = Opcode::OpShiftLeftLogical;
                                    stats.strength_reductions += 1;
                                }
                            }
                        },
                        _ => {},
                    }
                }
            }
        }

        Ok(stats)
    }

    /// Check if value is power of 2
    fn get_power_of_two(
        &self,
        id: Id,
        constants: &BTreeMap<Id, crate::module::ConstantDecl>,
    ) -> Option<u32> {
        let value = self.get_int_constant(id, constants)?;
        if value != 0 && (value & (value - 1)) == 0 {
            Some(value.trailing_zeros())
        } else {
            None
        }
    }

    /// Remove unused variables
    fn remove_unused_variables(&self, module: &mut SpirVModule) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();

        // Collect all referenced IDs
        let mut referenced: BTreeSet<Id> = BTreeSet::new();

        // Add entry point interfaces
        for ep in &module.entry_points {
            for &iface in &ep.interface {
                referenced.insert(iface);
            }
        }

        // Add references from functions
        for func in &module.functions {
            for block in &func.blocks {
                for inst in &block.instructions {
                    for op in &inst.operands {
                        if let Operand::Id(id) = op {
                            referenced.insert(*id);
                        }
                    }
                }
            }
        }

        // Remove unreferenced global variables
        let before_count = module.global_variables.len();
        module
            .global_variables
            .retain(|&id, _| referenced.contains(&id));
        stats.variables_removed = before_count - module.global_variables.len();

        Ok(stats)
    }

    /// Inline functions pass
    fn inline_functions(&self, module: &mut SpirVModule) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();

        // Would implement function inlining based on threshold
        // This is complex and requires:
        // 1. Finding call sites
        // 2. Checking function size against threshold
        // 3. Cloning function body with remapped IDs
        // 4. Inserting at call site
        // 5. Removing call instruction

        Ok(stats)
    }

    /// Compact IDs pass
    fn compact_ids(&self, module: &mut SpirVModule) -> SpirVResult<OptimizationStats> {
        let mut stats = OptimizationStats::default();

        // Collect all used IDs
        let mut used_ids: BTreeSet<Id> = BTreeSet::new();

        // Add all definition IDs
        for &id in module.types.keys() {
            used_ids.insert(id);
        }
        for &id in module.constants.keys() {
            used_ids.insert(id);
        }
        for &id in module.global_variables.keys() {
            used_ids.insert(id);
        }
        for (&id, _) in &module.ext_inst_imports {
            used_ids.insert(id);
        }
        for func in &module.functions {
            used_ids.insert(func.id);
            used_ids.insert(func.return_type);
            used_ids.insert(func.function_type);
            for param in &func.parameters {
                used_ids.insert(param.id);
                used_ids.insert(param.param_type);
            }
            for block in &func.blocks {
                used_ids.insert(block.label);
                for inst in &block.instructions {
                    if let Some(rt) = inst.result_type {
                        used_ids.insert(rt);
                    }
                    if let Some(r) = inst.result {
                        used_ids.insert(r);
                    }
                    for op in &inst.operands {
                        if let Operand::Id(id) = op {
                            used_ids.insert(*id);
                        }
                    }
                }
            }
        }

        // Create remapping
        let mut id_map: BTreeMap<Id, Id> = BTreeMap::new();
        let mut next_id: Id = 1;
        for &old_id in &used_ids {
            id_map.insert(old_id, next_id);
            next_id += 1;
        }

        // Update bound
        let old_bound = module.header.bound;
        module.header.bound = next_id;
        stats.ids_compacted = (old_bound - next_id) as usize;

        // Would need to apply remapping to all IDs in the module

        Ok(stats)
    }
}

/// Expression key for CSE
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ExpressionKey {
    opcode: Opcode,
    result_type: Option<Id>,
    operands: Vec<Operand>,
}

/// Check if instruction has no side effects
fn is_pure_instruction(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::OpFAdd
            | Opcode::OpFSub
            | Opcode::OpFMul
            | Opcode::OpFDiv
            | Opcode::OpFNegate
            | Opcode::OpIAdd
            | Opcode::OpISub
            | Opcode::OpIMul
            | Opcode::OpSDiv
            | Opcode::OpUDiv
            | Opcode::OpSMod
            | Opcode::OpUMod
            | Opcode::OpSNegate
            | Opcode::OpBitwiseAnd
            | Opcode::OpBitwiseOr
            | Opcode::OpBitwiseXor
            | Opcode::OpNot
            | Opcode::OpShiftLeftLogical
            | Opcode::OpShiftRightLogical
            | Opcode::OpShiftRightArithmetic
            | Opcode::OpFOrdEqual
            | Opcode::OpFOrdNotEqual
            | Opcode::OpFOrdLessThan
            | Opcode::OpFOrdGreaterThan
            | Opcode::OpFOrdLessThanEqual
            | Opcode::OpFOrdGreaterThanEqual
            | Opcode::OpIEqual
            | Opcode::OpINotEqual
            | Opcode::OpSLessThan
            | Opcode::OpSGreaterThan
            | Opcode::OpSLessThanEqual
            | Opcode::OpSGreaterThanEqual
            | Opcode::OpLogicalAnd
            | Opcode::OpLogicalOr
            | Opcode::OpLogicalNot
            | Opcode::OpSelect
            | Opcode::OpConvertFToS
            | Opcode::OpConvertFToU
            | Opcode::OpConvertSToF
            | Opcode::OpConvertUToF
            | Opcode::OpBitcast
            | Opcode::OpCompositeConstruct
            | Opcode::OpCompositeExtract
            | Opcode::OpCompositeInsert
            | Opcode::OpVectorShuffle
            | Opcode::OpDot
    )
}

/// Optimization statistics
#[derive(Debug, Clone, Default)]
pub struct OptimizationStats {
    /// Number of instructions removed
    pub instructions_removed: usize,
    /// Number of functions removed
    pub functions_removed: usize,
    /// Number of variables removed
    pub variables_removed: usize,
    /// Number of constants folded
    pub constants_folded: usize,
    /// Number of branches eliminated
    pub branches_eliminated: usize,
    /// Number of CSE eliminations
    pub cse_eliminations: usize,
    /// Number of strength reductions
    pub strength_reductions: usize,
    /// Number of IDs compacted
    pub ids_compacted: usize,
    /// Number of passes run
    pub passes_run: usize,
}

impl OptimizationStats {
    /// Add stats from another
    pub fn add(&mut self, other: OptimizationStats) {
        self.instructions_removed += other.instructions_removed;
        self.functions_removed += other.functions_removed;
        self.variables_removed += other.variables_removed;
        self.constants_folded += other.constants_folded;
        self.branches_eliminated += other.branches_eliminated;
        self.cse_eliminations += other.cse_eliminations;
        self.strength_reductions += other.strength_reductions;
        self.ids_compacted += other.ids_compacted;
        self.passes_run += 1;
    }

    /// Check if any optimization was performed
    pub fn has_changes(&self) -> bool {
        self.instructions_removed > 0
            || self.functions_removed > 0
            || self.variables_removed > 0
            || self.constants_folded > 0
            || self.branches_eliminated > 0
            || self.cse_eliminations > 0
            || self.strength_reductions > 0
    }

    /// Total eliminations
    pub fn total_eliminations(&self) -> usize {
        self.instructions_removed
            + self.functions_removed
            + self.variables_removed
            + self.constants_folded
            + self.cse_eliminations
    }
}

/// Optimization pass trait
pub trait OptimizationPass {
    /// Pass name
    fn name(&self) -> &'static str;

    /// Run the pass
    fn run(&self, module: &mut SpirVModule) -> SpirVResult<bool>;
}

/// Dead code elimination pass
pub struct DeadCodeEliminationPass;

impl OptimizationPass for DeadCodeEliminationPass {
    fn name(&self) -> &'static str {
        "dead-code-elimination"
    }

    fn run(&self, module: &mut SpirVModule) -> SpirVResult<bool> {
        let optimizer = SpirVOptimizer::new(OptimizerConfig::default());
        let stats = optimizer.dead_code_elimination(module)?;
        Ok(stats.instructions_removed > 0 || stats.functions_removed > 0)
    }
}

/// Constant folding pass
pub struct ConstantFoldingPass;

impl OptimizationPass for ConstantFoldingPass {
    fn name(&self) -> &'static str {
        "constant-folding"
    }

    fn run(&self, module: &mut SpirVModule) -> SpirVResult<bool> {
        let optimizer = SpirVOptimizer::new(OptimizerConfig::default());
        let stats = optimizer.constant_folding(module)?;
        Ok(stats.constants_folded > 0)
    }
}

/// Pass manager for running multiple passes
pub struct PassManager {
    passes: Vec<Box<dyn OptimizationPass>>,
    max_iterations: usize,
}

impl PassManager {
    /// Create a new pass manager
    pub fn new() -> Self {
        Self {
            passes: Vec::new(),
            max_iterations: 10,
        }
    }

    /// Add a pass
    pub fn add_pass(&mut self, pass: impl OptimizationPass + 'static) {
        self.passes.push(Box::new(pass));
    }

    /// Set maximum iterations
    pub fn max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }

    /// Run all passes
    pub fn run(&self, module: &mut SpirVModule) -> SpirVResult<bool> {
        let mut changed_overall = false;

        for iteration in 0..self.max_iterations {
            let mut changed_this_iteration = false;

            for pass in &self.passes {
                if pass.run(module)? {
                    changed_this_iteration = true;
                    changed_overall = true;
                }
            }

            if !changed_this_iteration {
                break;
            }
        }

        Ok(changed_overall)
    }
}

impl Default for PassManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_config() {
        let config = OptimizerConfig::performance();
        assert!(config.dead_code_elimination);
        assert!(config.constant_folding);
        assert!(config.inline_functions);
    }

    #[test]
    fn test_optimization_stats() {
        let mut stats = OptimizationStats::default();
        stats.instructions_removed = 5;
        stats.constants_folded = 3;

        assert!(stats.has_changes());
        assert_eq!(stats.total_eliminations(), 8);
    }
}
