//! IR Analysis
//!
//! Analysis passes for extracting information from IR.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, collections::BTreeSet, string::String, vec, vec::Vec};
#[cfg(feature = "std")]
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::block::{BasicBlock, BlockMap};
use crate::function::{Function, FunctionId};
use crate::instruction::{BlockId, Instruction};
use crate::module::Module;
use crate::types::{AddressSpace, IrType};
use crate::value::ValueId;

// ============================================================================
// Use-Def Analysis
// ============================================================================

/// Definition of a value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueDef {
    /// Block containing the definition
    pub block: BlockId,
    /// Instruction index within block
    pub instruction: usize,
    /// Type of the value
    pub ty: IrType,
}

/// Use of a value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueUse {
    /// Block containing the use
    pub block: BlockId,
    /// Instruction index within block
    pub instruction: usize,
    /// Operand position (0-indexed)
    pub operand: usize,
}

/// Use-def chain for a function
#[derive(Debug, Default)]
pub struct UseDefChain {
    /// Map from value to its definition
    #[cfg(feature = "std")]
    pub definitions: HashMap<ValueId, ValueDef>,
    #[cfg(not(feature = "std"))]
    pub definitions: BTreeMap<ValueId, ValueDef>,

    /// Map from value to its uses
    #[cfg(feature = "std")]
    pub uses: HashMap<ValueId, Vec<ValueUse>>,
    #[cfg(not(feature = "std"))]
    pub uses: BTreeMap<ValueId, Vec<ValueUse>>,
}

impl UseDefChain {
    /// Build use-def chains for a function
    pub fn analyze(func: &Function) -> Self {
        let mut chain = Self::default();

        for (block_id, block) in func.blocks.iter() {
            for (inst_idx, inst) in block.instructions().iter().enumerate() {
                // Record definition
                if let Some(result) = inst.result() {
                    if let Some(ty) = inst.result_type() {
                        chain.definitions.insert(result, ValueDef {
                            block: *block_id,
                            instruction: inst_idx,
                            ty,
                        });
                    }
                }

                // Record uses
                for (operand_idx, operand) in inst.operands().iter().enumerate() {
                    chain
                        .uses
                        .entry(*operand)
                        .or_insert_with(Vec::new)
                        .push(ValueUse {
                            block: *block_id,
                            instruction: inst_idx,
                            operand: operand_idx,
                        });
                }
            }
        }

        chain
    }

    /// Check if a value is used
    pub fn is_used(&self, value: ValueId) -> bool {
        self.uses
            .get(&value)
            .map(|u| !u.is_empty())
            .unwrap_or(false)
    }

    /// Get use count
    pub fn use_count(&self, value: ValueId) -> usize {
        self.uses.get(&value).map(|u| u.len()).unwrap_or(0)
    }

    /// Check if a value has a single use
    pub fn has_single_use(&self, value: ValueId) -> bool {
        self.use_count(value) == 1
    }
}

// ============================================================================
// Liveness Analysis
// ============================================================================

/// Live variable analysis results
#[derive(Debug, Default)]
pub struct LivenessAnalysis {
    /// Variables live at block entry
    #[cfg(feature = "std")]
    pub live_in: HashMap<BlockId, HashSet<ValueId>>,
    #[cfg(not(feature = "std"))]
    pub live_in: BTreeMap<BlockId, BTreeSet<ValueId>>,

    /// Variables live at block exit
    #[cfg(feature = "std")]
    pub live_out: HashMap<BlockId, HashSet<ValueId>>,
    #[cfg(not(feature = "std"))]
    pub live_out: BTreeMap<BlockId, BTreeSet<ValueId>>,
}

impl LivenessAnalysis {
    /// Perform liveness analysis on a function
    pub fn analyze(func: &Function) -> Self {
        let mut result = Self::default();

        // Initialize empty sets
        for (block_id, _) in func.blocks.iter() {
            #[cfg(feature = "std")]
            {
                result.live_in.insert(*block_id, HashSet::new());
                result.live_out.insert(*block_id, HashSet::new());
            }
            #[cfg(not(feature = "std"))]
            {
                result.live_in.insert(*block_id, BTreeSet::new());
                result.live_out.insert(*block_id, BTreeSet::new());
            }
        }

        // Compute gen and kill sets
        #[cfg(feature = "std")]
        let mut gen: HashMap<BlockId, HashSet<ValueId>> = HashMap::new();
        #[cfg(feature = "std")]
        let mut kill: HashMap<BlockId, HashSet<ValueId>> = HashMap::new();

        #[cfg(not(feature = "std"))]
        let mut gen: BTreeMap<BlockId, BTreeSet<ValueId>> = BTreeMap::new();
        #[cfg(not(feature = "std"))]
        let mut kill: BTreeMap<BlockId, BTreeSet<ValueId>> = BTreeMap::new();

        for (block_id, block) in func.blocks.iter() {
            #[cfg(feature = "std")]
            {
                gen.insert(*block_id, HashSet::new());
                kill.insert(*block_id, HashSet::new());
            }
            #[cfg(not(feature = "std"))]
            {
                gen.insert(*block_id, BTreeSet::new());
                kill.insert(*block_id, BTreeSet::new());
            }

            for inst in block.instructions() {
                // Uses before definition
                for operand in inst.operands() {
                    if !kill
                        .get(block_id)
                        .map(|k| k.contains(&operand))
                        .unwrap_or(false)
                    {
                        gen.get_mut(block_id).map(|g| g.insert(operand));
                    }
                }

                // Definition
                if let Some(result) = inst.result() {
                    kill.get_mut(block_id).map(|k| k.insert(result));
                }
            }
        }

        // Fixed-point iteration
        let mut changed = true;
        while changed {
            changed = false;

            for (block_id, block) in func.blocks.iter().rev() {
                // live_out = union of live_in of successors
                #[cfg(feature = "std")]
                let mut new_out: HashSet<ValueId> = HashSet::new();
                #[cfg(not(feature = "std"))]
                let mut new_out: BTreeSet<ValueId> = BTreeSet::new();

                for succ in block.successors() {
                    if let Some(succ_in) = result.live_in.get(&succ) {
                        for v in succ_in {
                            new_out.insert(*v);
                        }
                    }
                }

                // live_in = gen ∪ (live_out - kill)
                #[cfg(feature = "std")]
                let mut new_in: HashSet<ValueId> = HashSet::new();
                #[cfg(not(feature = "std"))]
                let mut new_in: BTreeSet<ValueId> = BTreeSet::new();

                if let Some(g) = gen.get(block_id) {
                    for v in g {
                        new_in.insert(*v);
                    }
                }

                let k = kill.get(block_id);
                for v in &new_out {
                    if !k.map(|k| k.contains(v)).unwrap_or(false) {
                        new_in.insert(*v);
                    }
                }

                // Check for changes
                if result.live_in.get(block_id) != Some(&new_in) {
                    changed = true;
                }
                if result.live_out.get(block_id) != Some(&new_out) {
                    changed = true;
                }

                result.live_in.insert(*block_id, new_in);
                result.live_out.insert(*block_id, new_out);
            }
        }

        result
    }

    /// Check if a value is live at block entry
    pub fn is_live_at_entry(&self, block: BlockId, value: ValueId) -> bool {
        self.live_in
            .get(&block)
            .map(|s| s.contains(&value))
            .unwrap_or(false)
    }

    /// Check if a value is live at block exit
    pub fn is_live_at_exit(&self, block: BlockId, value: ValueId) -> bool {
        self.live_out
            .get(&block)
            .map(|s| s.contains(&value))
            .unwrap_or(false)
    }
}

// ============================================================================
// Dependency Analysis
// ============================================================================

/// Dependency type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// True dependency (read-after-write)
    True,
    /// Anti-dependency (write-after-read)
    Anti,
    /// Output dependency (write-after-write)
    Output,
    /// Control dependency
    Control,
}

/// Dependency between two instructions
#[derive(Debug, Clone)]
pub struct Dependency {
    /// Source instruction
    pub source: (BlockId, usize),
    /// Target instruction
    pub target: (BlockId, usize),
    /// Type of dependency
    pub kind: DependencyType,
}

/// Dependency graph for a function
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Dependencies
    pub dependencies: Vec<Dependency>,
}

impl DependencyGraph {
    /// Build dependency graph
    pub fn analyze(func: &Function) -> Self {
        let mut graph = Self::default();
        let use_def = UseDefChain::analyze(func);

        // Find true dependencies (RAW)
        for (value, uses) in &use_def.uses {
            if let Some(def) = use_def.definitions.get(value) {
                for use_ in uses {
                    graph.dependencies.push(Dependency {
                        source: (def.block, def.instruction),
                        target: (use_.block, use_.instruction),
                        kind: DependencyType::True,
                    });
                }
            }
        }

        graph
    }
}

// ============================================================================
// Memory Access Analysis
// ============================================================================

/// Memory access
#[derive(Debug, Clone)]
pub struct MemoryAccess {
    /// Block containing access
    pub block: BlockId,
    /// Instruction index
    pub instruction: usize,
    /// Is this a write?
    pub is_write: bool,
    /// Base pointer (if known)
    pub base: Option<ValueId>,
    /// Address space
    pub address_space: AddressSpace,
}

/// Memory access analysis
#[derive(Debug, Default)]
pub struct MemoryAnalysis {
    /// All memory accesses
    pub accesses: Vec<MemoryAccess>,
}

impl MemoryAnalysis {
    /// Analyze memory accesses
    pub fn analyze(func: &Function) -> Self {
        let mut analysis = Self::default();

        for (block_id, block) in func.blocks.iter() {
            for (inst_idx, inst) in block.instructions().iter().enumerate() {
                match inst {
                    Instruction::Load { pointer, .. } => {
                        analysis.accesses.push(MemoryAccess {
                            block: *block_id,
                            instruction: inst_idx,
                            is_write: false,
                            base: Some(*pointer),
                            address_space: AddressSpace::Private, // Would need type info
                        });
                    },
                    Instruction::Store { pointer, .. } => {
                        analysis.accesses.push(MemoryAccess {
                            block: *block_id,
                            instruction: inst_idx,
                            is_write: true,
                            base: Some(*pointer),
                            address_space: AddressSpace::Private,
                        });
                    },
                    Instruction::ImageWrite { .. } => {
                        analysis.accesses.push(MemoryAccess {
                            block: *block_id,
                            instruction: inst_idx,
                            is_write: true,
                            base: None,
                            address_space: AddressSpace::Image,
                        });
                    },
                    Instruction::ImageRead { .. } | Instruction::ImageFetch { .. } => {
                        analysis.accesses.push(MemoryAccess {
                            block: *block_id,
                            instruction: inst_idx,
                            is_write: false,
                            base: None,
                            address_space: AddressSpace::Image,
                        });
                    },
                    _ => {},
                }
            }
        }

        analysis
    }

    /// Check if two accesses may alias
    pub fn may_alias(&self, a: &MemoryAccess, b: &MemoryAccess) -> bool {
        // Conservative: different address spaces don't alias
        if a.address_space != b.address_space {
            return false;
        }

        // Same base pointer may alias
        match (a.base, b.base) {
            (Some(base_a), Some(base_b)) if base_a == base_b => true,
            _ => true, // Conservative
        }
    }
}

// ============================================================================
// Resource Usage Analysis
// ============================================================================

/// Resource usage for a shader
#[derive(Debug, Default, Clone)]
pub struct ResourceUsage {
    /// Number of scalar registers
    pub scalar_registers: u32,
    /// Number of vector registers
    pub vector_registers: u32,
    /// Shared memory usage (bytes)
    pub shared_memory: u32,
    /// Constant buffer slots
    pub constant_buffers: u32,
    /// Texture slots
    pub textures: u32,
    /// Image (UAV) slots
    pub images: u32,
    /// Sampler slots
    pub samplers: u32,
    /// Buffer slots
    pub buffers: u32,
    /// Instructions count
    pub instructions: u32,
    /// Control flow depth
    pub control_flow_depth: u32,
    /// Uses group operations
    pub uses_group_operations: bool,
    /// Uses atomics
    pub uses_atomics: bool,
    /// Uses barriers
    pub uses_barriers: bool,
    /// Uses derivatives
    pub uses_derivatives: bool,
}

impl ResourceUsage {
    /// Analyze resource usage for a function
    pub fn analyze(func: &Function, module: &Module) -> Self {
        let mut usage = Self::default();

        // Count instructions and analyze usage
        for (_, block) in func.blocks.iter() {
            for inst in block.instructions() {
                usage.instructions += 1;

                match inst {
                    Instruction::AtomicExchange { .. }
                    | Instruction::AtomicCompareExchange { .. }
                    | Instruction::AtomicLoad { .. }
                    | Instruction::AtomicStore { .. }
                    | Instruction::AtomicIAdd { .. }
                    | Instruction::AtomicISub { .. }
                    | Instruction::AtomicAnd { .. }
                    | Instruction::AtomicOr { .. }
                    | Instruction::AtomicXor { .. }
                    | Instruction::AtomicSMin { .. }
                    | Instruction::AtomicSMax { .. }
                    | Instruction::AtomicUMin { .. }
                    | Instruction::AtomicUMax { .. } => {
                        usage.uses_atomics = true;
                    },

                    Instruction::ControlBarrier { .. } | Instruction::MemoryBarrier { .. } => {
                        usage.uses_barriers = true;
                    },

                    Instruction::UnaryOp { op, .. } => {
                        use crate::instruction::UnaryOp::*;
                        match op {
                            DPdx | DPdy | DPdxFine | DPdyFine | DPdxCoarse | DPdyCoarse
                            | Fwidth | FwidthFine | FwidthCoarse => {
                                usage.uses_derivatives = true;
                            },
                            _ => {},
                        }
                    },

                    Instruction::GroupAll { .. }
                    | Instruction::GroupAny { .. }
                    | Instruction::GroupBroadcast { .. }
                    | Instruction::GroupIAdd { .. }
                    | Instruction::GroupFAdd { .. }
                    | Instruction::GroupSMin { .. }
                    | Instruction::GroupUMin { .. }
                    | Instruction::GroupFMin { .. }
                    | Instruction::GroupSMax { .. }
                    | Instruction::GroupUMax { .. }
                    | Instruction::GroupFMax { .. }
                    | Instruction::GroupBitwiseAnd { .. }
                    | Instruction::GroupBitwiseOr { .. }
                    | Instruction::GroupBitwiseXor { .. }
                    | Instruction::SubgroupElect { .. }
                    | Instruction::SubgroupBallot { .. }
                    | Instruction::SubgroupShuffle { .. }
                    | Instruction::SubgroupShuffleXor { .. } => {
                        usage.uses_group_operations = true;
                    },

                    _ => {},
                }
            }
        }

        // Count resources from global variables
        for global in &func.interface_variables {
            if let Some(gv) = module.get_global(*global) {
                match gv.address_space {
                    AddressSpace::Uniform => usage.constant_buffers += 1,
                    AddressSpace::StorageBuffer => usage.buffers += 1,
                    AddressSpace::Image => {
                        if gv.ty.is_sampled_image() {
                            usage.textures += 1;
                        } else {
                            usage.images += 1;
                        }
                    },
                    AddressSpace::Workgroup => {
                        if let Some(size) = gv.ty.size_of() {
                            usage.shared_memory += size as u32;
                        }
                    },
                    _ => {},
                }
            }
        }

        usage
    }
}

// ============================================================================
// Uniformity Analysis
// ============================================================================

/// Uniformity of a value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Uniformity {
    /// Value is uniform across all invocations
    Uniform,
    /// Value may vary per invocation
    NonUniform,
    /// Value is constant
    Constant,
}

/// Uniformity analysis
#[derive(Debug, Default)]
pub struct UniformityAnalysis {
    #[cfg(feature = "std")]
    pub value_uniformity: HashMap<ValueId, Uniformity>,
    #[cfg(not(feature = "std"))]
    pub value_uniformity: BTreeMap<ValueId, Uniformity>,
}

impl UniformityAnalysis {
    /// Analyze uniformity
    pub fn analyze(func: &Function, module: &Module) -> Self {
        let mut analysis = Self::default();

        // Parameters are uniform if they come from uniform buffers
        // Built-ins like gl_GlobalInvocationID are non-uniform

        // Propagate uniformity through instructions
        let mut changed = true;
        while changed {
            changed = false;

            for (_, block) in func.blocks.iter() {
                for inst in block.instructions() {
                    if let Some(result) = inst.result() {
                        let uniformity = analysis.compute_uniformity(inst, module);

                        if analysis.value_uniformity.get(&result) != Some(&uniformity) {
                            analysis.value_uniformity.insert(result, uniformity);
                            changed = true;
                        }
                    }
                }
            }
        }

        analysis
    }

    /// Compute uniformity for an instruction result
    fn compute_uniformity(&self, inst: &Instruction, _module: &Module) -> Uniformity {
        match inst {
            Instruction::Constant { .. } => Uniformity::Constant,

            // Intrinsically non-uniform operations
            Instruction::UnaryOp { op, .. } => {
                use crate::instruction::UnaryOp::*;
                match op {
                    DPdx | DPdy | DPdxFine | DPdyFine | DPdxCoarse | DPdyCoarse | Fwidth
                    | FwidthFine | FwidthCoarse => Uniformity::NonUniform,
                    _ => self.merge_operand_uniformity(inst),
                }
            },

            // Most operations inherit from operands
            _ => self.merge_operand_uniformity(inst),
        }
    }

    /// Merge uniformity of operands
    fn merge_operand_uniformity(&self, inst: &Instruction) -> Uniformity {
        let mut result = Uniformity::Constant;

        for operand in inst.operands() {
            match self.value_uniformity.get(&operand) {
                Some(Uniformity::NonUniform) => return Uniformity::NonUniform,
                Some(Uniformity::Uniform) => result = Uniformity::Uniform,
                Some(Uniformity::Constant) | None => {},
            }
        }

        result
    }

    /// Check if a value is uniform
    pub fn is_uniform(&self, value: ValueId) -> bool {
        matches!(
            self.value_uniformity.get(&value),
            Some(Uniformity::Uniform) | Some(Uniformity::Constant)
        )
    }
}

// ============================================================================
// Divergence Analysis
// ============================================================================

/// Control flow divergence analysis
#[derive(Debug, Default)]
pub struct DivergenceAnalysis {
    /// Blocks with divergent control flow
    #[cfg(feature = "std")]
    pub divergent_blocks: HashSet<BlockId>,
    #[cfg(not(feature = "std"))]
    pub divergent_blocks: BTreeSet<BlockId>,

    /// Reconvergence points
    #[cfg(feature = "std")]
    pub reconvergence_points: HashMap<BlockId, BlockId>,
    #[cfg(not(feature = "std"))]
    pub reconvergence_points: BTreeMap<BlockId, BlockId>,
}

impl DivergenceAnalysis {
    /// Analyze control flow divergence
    pub fn analyze(func: &Function, uniformity: &UniformityAnalysis) -> Self {
        let mut analysis = Self::default();

        for (block_id, block) in func.blocks.iter() {
            // Check if block ends with divergent branch
            if let Some(last) = block.instructions().last() {
                match last {
                    Instruction::BranchConditional { condition, .. } => {
                        if !uniformity.is_uniform(*condition) {
                            analysis.divergent_blocks.insert(*block_id);
                        }
                    },
                    Instruction::Switch { selector, .. } => {
                        if !uniformity.is_uniform(*selector) {
                            analysis.divergent_blocks.insert(*block_id);
                        }
                    },
                    _ => {},
                }
            }
        }

        analysis
    }

    /// Check if a block has divergent control flow
    pub fn is_divergent(&self, block: BlockId) -> bool {
        self.divergent_blocks.contains(&block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::ExecutionModel;

    #[test]
    fn test_use_def_chain() {
        let mut func = Function::new_entry_point("main", ExecutionModel::Fragment);
        // Would need to add instructions to test
        let chain = UseDefChain::analyze(&func);
        assert!(chain.definitions.is_empty());
    }

    #[test]
    fn test_liveness_analysis() {
        let mut func = Function::new_entry_point("main", ExecutionModel::Fragment);
        let _ = func.ensure_entry_block();
        let liveness = LivenessAnalysis::analyze(&func);
        assert!(!liveness.live_in.is_empty());
    }

    #[test]
    fn test_resource_usage() {
        let func = Function::new_entry_point("main", ExecutionModel::Fragment);
        let module = Module::new("test");
        let usage = ResourceUsage::analyze(&func, &module);
        assert_eq!(usage.instructions, 0);
    }
}
