//! SSA Construction and Manipulation
//!
//! Static Single Assignment form utilities.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, collections::BTreeSet, string::String, vec, vec::Vec};
#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};

use crate::block::BasicBlock;
use crate::cfg::ControlFlowGraph;
use crate::dominator::{DominanceFrontier, DominatorTree};
use crate::function::Function;
use crate::instruction::{BlockId, Instruction};
use crate::types::IrType;
use crate::value::ValueId;

/// SSA construction using Cytron's algorithm
pub struct SsaBuilder {
    /// Variable counter for versioning
    version_counter: u32,
}

impl SsaBuilder {
    pub fn new() -> Self {
        Self { version_counter: 0 }
    }

    /// Convert function to SSA form
    pub fn construct_ssa(&mut self, func: &mut Function) -> SsaInfo {
        let cfg = ControlFlowGraph::from_function(func);
        let dom_tree = DominatorTree::compute(&cfg);
        let df = DominanceFrontier::compute(&cfg, &dom_tree);

        let mut info = SsaInfo::default();

        // Step 1: Find all variable definitions
        let defs = self.find_definitions(func);

        // Step 2: Insert phi nodes at iterated dominance frontier
        for (var, def_blocks) in &defs {
            let phi_blocks = df.iterated_frontier(&def_blocks.iter().copied().collect::<Vec<_>>());

            for &block_id in &phi_blocks {
                if let Some(block) = func.blocks.get_mut(block_id) {
                    // Insert phi at start of block
                    let phi_id = self.next_version();
                    info.phi_nodes.push(PhiNode {
                        result: phi_id,
                        variable: *var,
                        block: block_id,
                        operands: Vec::new(), // Filled in during rename
                    });
                }
            }
        }

        // Step 3: Rename variables
        self.rename_variables(func, &dom_tree, &mut info);

        info
    }

    /// Find all variable definitions
    #[cfg(feature = "std")]
    fn find_definitions(&self, func: &Function) -> HashMap<ValueId, HashSet<BlockId>> {
        let mut defs: HashMap<ValueId, HashSet<BlockId>> = HashMap::new();

        for (block_id, block) in func.blocks.iter() {
            for inst in block.instructions() {
                // Variable definitions come from Store instructions
                if let Instruction::Store { pointer, .. } = inst {
                    defs.entry(*pointer).or_default().insert(*block_id);
                }
            }
        }

        defs
    }

    #[cfg(not(feature = "std"))]
    fn find_definitions(&self, func: &Function) -> BTreeMap<ValueId, BTreeSet<BlockId>> {
        let mut defs: BTreeMap<ValueId, BTreeSet<BlockId>> = BTreeMap::new();

        for (block_id, block) in func.blocks.iter() {
            for inst in block.instructions() {
                if let Instruction::Store { pointer, .. } = inst {
                    defs.entry(*pointer).or_default().insert(*block_id);
                }
            }
        }

        defs
    }

    /// Rename variables during SSA construction
    fn rename_variables(
        &mut self,
        func: &mut Function,
        dom_tree: &DominatorTree,
        info: &mut SsaInfo,
    ) {
        #[cfg(feature = "std")]
        let mut stacks: HashMap<ValueId, Vec<ValueId>> = HashMap::new();
        #[cfg(not(feature = "std"))]
        let mut stacks: BTreeMap<ValueId, Vec<ValueId>> = BTreeMap::new();

        // Initialize stacks with parameters and globals
        for param in &func.parameters {
            stacks.insert(param.id, vec![param.id]);
        }

        // Visit blocks in dominator tree preorder
        self.rename_block(func, dom_tree.root(), dom_tree, &mut stacks, info);
    }

    #[cfg(feature = "std")]
    fn rename_block(
        &mut self,
        func: &mut Function,
        block_id: BlockId,
        dom_tree: &DominatorTree,
        stacks: &mut HashMap<ValueId, Vec<ValueId>>,
        info: &mut SsaInfo,
    ) {
        // Record stack depths to restore later
        let stack_depths: HashMap<_, _> = stacks.iter().map(|(k, v)| (*k, v.len())).collect();

        if let Some(block) = func.blocks.get_mut(block_id) {
            // Process phi nodes first
            for phi in &mut info.phi_nodes {
                if phi.block == block_id {
                    let new_ver = self.next_version();
                    stacks.entry(phi.variable).or_default().push(new_ver);
                    phi.result = new_ver;
                }
            }

            // Process instructions
            for inst in block.instructions_mut() {
                // Replace uses with current version
                self.replace_uses(inst, stacks);

                // Push new definition
                if let Some(result) = inst.result() {
                    stacks.entry(result).or_default().push(result);
                }
            }
        }

        // Fill in phi operands in successors
        if let Some(block) = func.blocks.get(block_id) {
            for succ in block.successors() {
                for phi in &mut info.phi_nodes {
                    if phi.block == succ {
                        if let Some(stack) = stacks.get(&phi.variable) {
                            if let Some(&current) = stack.last() {
                                phi.operands.push((current, block_id));
                            }
                        }
                    }
                }
            }
        }

        // Recurse to dominated blocks
        for &child in dom_tree.children(block_id) {
            self.rename_block(func, child, dom_tree, stacks, info);
        }

        // Restore stacks
        for (var, depth) in stack_depths {
            if let Some(stack) = stacks.get_mut(&var) {
                stack.truncate(depth);
            }
        }
    }

    #[cfg(not(feature = "std"))]
    fn rename_block(
        &mut self,
        func: &mut Function,
        block_id: BlockId,
        dom_tree: &DominatorTree,
        stacks: &mut BTreeMap<ValueId, Vec<ValueId>>,
        info: &mut SsaInfo,
    ) {
        let stack_depths: BTreeMap<_, _> = stacks.iter().map(|(k, v)| (*k, v.len())).collect();

        if let Some(block) = func.blocks.get_mut(block_id) {
            for phi in &mut info.phi_nodes {
                if phi.block == block_id {
                    let new_ver = self.next_version();
                    stacks.entry(phi.variable).or_default().push(new_ver);
                    phi.result = new_ver;
                }
            }

            for inst in block.instructions_mut() {
                self.replace_uses(inst, stacks);

                if let Some(result) = inst.result() {
                    stacks.entry(result).or_default().push(result);
                }
            }
        }

        if let Some(block) = func.blocks.get(block_id) {
            for succ in block.successors() {
                for phi in &mut info.phi_nodes {
                    if phi.block == succ {
                        if let Some(stack) = stacks.get(&phi.variable) {
                            if let Some(&current) = stack.last() {
                                phi.operands.push((current, block_id));
                            }
                        }
                    }
                }
            }
        }

        for &child in dom_tree.children(block_id) {
            self.rename_block(func, child, dom_tree, stacks, info);
        }

        for (var, depth) in stack_depths {
            if let Some(stack) = stacks.get_mut(&var) {
                stack.truncate(depth);
            }
        }
    }

    /// Replace uses in an instruction with current version
    #[cfg(feature = "std")]
    fn replace_uses(&self, inst: &mut Instruction, stacks: &HashMap<ValueId, Vec<ValueId>>) {
        // Would need to modify instruction operands
        // This is a placeholder
    }

    #[cfg(not(feature = "std"))]
    fn replace_uses(&self, inst: &mut Instruction, stacks: &BTreeMap<ValueId, Vec<ValueId>>) {
        // Would need to modify instruction operands
    }

    /// Get next version number
    fn next_version(&mut self) -> ValueId {
        let v = self.version_counter;
        self.version_counter += 1;
        v
    }
}

impl Default for SsaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about SSA form
#[derive(Debug, Default)]
pub struct SsaInfo {
    /// Phi nodes in the function
    pub phi_nodes: Vec<PhiNode>,
    /// Original variable to versions mapping
    #[cfg(feature = "std")]
    pub versions: HashMap<ValueId, Vec<ValueId>>,
    #[cfg(not(feature = "std"))]
    pub versions: BTreeMap<ValueId, Vec<ValueId>>,
}

/// Phi node information
#[derive(Debug, Clone)]
pub struct PhiNode {
    /// Result value
    pub result: ValueId,
    /// Original variable
    pub variable: ValueId,
    /// Block containing the phi
    pub block: BlockId,
    /// Operands (value, predecessor block)
    pub operands: Vec<(ValueId, BlockId)>,
}

impl PhiNode {
    /// Convert to IR instruction
    pub fn to_instruction(&self, ty: IrType) -> Instruction {
        Instruction::Phi {
            result: self.result,
            ty,
            operands: self.operands.clone(),
        }
    }
}

/// SSA deconstruction (for code generation)
pub struct SsaDestructor;

impl SsaDestructor {
    pub fn new() -> Self {
        Self
    }

    /// Convert out of SSA form
    pub fn destruct_ssa(&self, func: &mut Function) {
        // Find all phi nodes
        let mut phis_to_remove: Vec<(BlockId, usize)> = Vec::new();
        let mut copies_to_insert: Vec<(BlockId, Instruction)> = Vec::new();

        for (block_id, block) in func.blocks.iter() {
            for (idx, inst) in block.instructions().iter().enumerate() {
                if let Instruction::Phi {
                    result,
                    ty,
                    operands,
                    ..
                } = inst
                {
                    phis_to_remove.push((*block_id, idx));

                    // Insert parallel copies in predecessor blocks
                    for (value, pred_block) in operands {
                        // Need to insert: result = value
                        // This should be a copy instruction
                        copies_to_insert.push((*pred_block, Instruction::CopyObject {
                            result: *result,
                            ty: ty.clone(),
                            operand: *value,
                        }));
                    }
                }
            }
        }

        // Insert copies (before terminator in each block)
        for (block_id, copy_inst) in copies_to_insert {
            if let Some(block) = func.blocks.get_mut(block_id) {
                // Insert before terminator
                let len = block.instructions().len();
                if len > 0 {
                    block.insert(len - 1, copy_inst);
                }
            }
        }

        // Remove phi nodes (in reverse order to preserve indices)
        phis_to_remove.sort_by(|a, b| b.cmp(a));
        for (block_id, idx) in phis_to_remove {
            if let Some(block) = func.blocks.get_mut(block_id) {
                block.remove(idx);
            }
        }

        // Handle critical edges if needed
        // self.split_critical_edges(func);

        // Coalesce copies if possible
        // self.coalesce_copies(func);
    }
}

impl Default for SsaDestructor {
    fn default() -> Self {
        Self::new()
    }
}

/// SSA-based value numbering
#[derive(Debug)]
pub struct ValueNumbering {
    /// Value number to canonical value
    #[cfg(feature = "std")]
    pub numbers: HashMap<u32, ValueId>,
    #[cfg(not(feature = "std"))]
    pub numbers: BTreeMap<u32, ValueId>,
    /// Value to number mapping
    #[cfg(feature = "std")]
    pub value_to_number: HashMap<ValueId, u32>,
    #[cfg(not(feature = "std"))]
    pub value_to_number: BTreeMap<ValueId, u32>,
    next_number: u32,
}

impl ValueNumbering {
    pub fn new() -> Self {
        Self {
            numbers: Default::default(),
            value_to_number: Default::default(),
            next_number: 0,
        }
    }

    /// Get or create value number
    pub fn get_number(&mut self, value: ValueId) -> u32 {
        if let Some(&num) = self.value_to_number.get(&value) {
            return num;
        }

        let num = self.next_number;
        self.next_number += 1;
        self.value_to_number.insert(value, num);
        self.numbers.insert(num, value);
        num
    }

    /// Set value number (for equivalent values)
    pub fn set_number(&mut self, value: ValueId, number: u32) {
        self.value_to_number.insert(value, number);
    }

    /// Check if two values have the same number
    pub fn equivalent(&self, a: ValueId, b: ValueId) -> bool {
        match (self.value_to_number.get(&a), self.value_to_number.get(&b)) {
            (Some(na), Some(nb)) => na == nb,
            _ => a == b,
        }
    }

    /// Get canonical value for a number
    pub fn canonical(&self, number: u32) -> Option<ValueId> {
        self.numbers.get(&number).copied()
    }
}

impl Default for ValueNumbering {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if IR is in SSA form
pub fn is_ssa_form(func: &Function) -> bool {
    #[cfg(feature = "std")]
    let mut defined: HashSet<ValueId> = HashSet::new();
    #[cfg(not(feature = "std"))]
    let mut defined: BTreeSet<ValueId> = BTreeSet::new();

    // Parameters are defined
    for param in &func.parameters {
        defined.insert(param.id);
    }

    for (_, block) in func.blocks.iter() {
        for inst in block.instructions() {
            if let Some(result) = inst.result() {
                if defined.contains(&result) {
                    return false; // Multiple definitions
                }
                defined.insert(result);
            }
        }
    }

    true
}

/// Count phi nodes in a function
pub fn count_phi_nodes(func: &Function) -> usize {
    let mut count = 0;

    for (_, block) in func.blocks.iter() {
        for inst in block.instructions() {
            if matches!(inst, Instruction::Phi { .. }) {
                count += 1;
            }
        }
    }

    count
}

/// Get all phi operands for a block
pub fn get_phi_operands(block: &BasicBlock) -> Vec<&Instruction> {
    block
        .instructions()
        .iter()
        .take_while(|inst| matches!(inst, Instruction::Phi { .. }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::ExecutionModel;

    #[test]
    fn test_ssa_builder() {
        let builder = SsaBuilder::new();
        assert_eq!(builder.version_counter, 0);
    }

    #[test]
    fn test_is_ssa_form() {
        let mut func = Function::new_entry_point("main", ExecutionModel::Fragment);
        func.ensure_entry_block();

        // Empty function is trivially SSA
        assert!(is_ssa_form(&func));
    }

    #[test]
    fn test_count_phi_nodes() {
        let mut func = Function::new_entry_point("main", ExecutionModel::Fragment);
        func.ensure_entry_block();

        assert_eq!(count_phi_nodes(&func), 0);
    }

    #[test]
    fn test_value_numbering() {
        let mut vn = ValueNumbering::new();

        let n1 = vn.get_number(1);
        let n2 = vn.get_number(2);
        let n1_again = vn.get_number(1);

        assert_eq!(n1, n1_again);
        assert_ne!(n1, n2);
    }
}
