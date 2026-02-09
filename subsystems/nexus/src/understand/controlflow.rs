//! Control flow analysis for code understanding
//!
//! This module provides control flow graph construction and analysis.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

/// Basic block ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockId(pub u32);

impl BlockId {
    /// Create new block ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Basic block
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Block ID
    pub id: BlockId,
    /// Instructions
    pub instructions: Vec<u32>,
    /// Predecessors
    pub predecessors: Vec<BlockId>,
    /// Successors
    pub successors: Vec<BlockId>,
    /// Terminator
    pub terminator: Terminator,
}

impl BasicBlock {
    /// Create new block
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
            terminator: Terminator::Return,
        }
    }

    /// Add instruction
    #[inline(always)]
    pub fn add_instruction(&mut self, instr: u32) {
        self.instructions.push(instr);
    }

    /// Set terminator
    #[inline(always)]
    pub fn set_terminator(&mut self, terminator: Terminator) {
        self.terminator = terminator;
    }

    /// Check if block is empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Get instruction count
    #[inline(always)]
    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }
}

/// Block terminator
#[derive(Debug, Clone)]
pub enum Terminator {
    /// Return from function
    Return,
    /// Unconditional branch
    Goto(BlockId),
    /// Conditional branch
    Branch {
        cond: u32,
        true_block: BlockId,
        false_block: BlockId,
    },
    /// Switch
    Switch {
        cond: u32,
        cases: Vec<(i64, BlockId)>,
        default: BlockId,
    },
    /// Unreachable
    Unreachable,
    /// Panic
    Panic,
}

impl Terminator {
    /// Get successor blocks
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            Self::Return | Self::Unreachable | Self::Panic => Vec::new(),
            Self::Goto(target) => vec![*target],
            Self::Branch {
                true_block,
                false_block,
                ..
            } => vec![*true_block, *false_block],
            Self::Switch { cases, default, .. } => {
                let mut succs: Vec<BlockId> = cases.iter().map(|(_, b)| *b).collect();
                succs.push(*default);
                succs
            },
        }
    }

    /// Check if terminator is conditional
    #[inline(always)]
    pub fn is_conditional(&self) -> bool {
        matches!(self, Self::Branch { .. } | Self::Switch { .. })
    }
}

/// Control flow graph
#[derive(Debug)]
pub struct ControlFlowGraph {
    /// Entry block
    pub entry: BlockId,
    /// Exit block
    pub exit: BlockId,
    /// Blocks
    pub blocks: BTreeMap<BlockId, BasicBlock>,
    /// Block counter
    block_counter: u32,
}

impl ControlFlowGraph {
    /// Create new CFG
    pub fn new() -> Self {
        let entry = BlockId(0);
        let exit = BlockId(1);

        let mut blocks = BTreeMap::new();
        blocks.insert(entry, BasicBlock::new(entry));
        blocks.insert(exit, BasicBlock::new(exit));

        Self {
            entry,
            exit,
            blocks,
            block_counter: 2,
        }
    }

    /// Create new block
    #[inline]
    pub fn create_block(&mut self) -> BlockId {
        let id = BlockId(self.block_counter);
        self.block_counter += 1;
        self.blocks.insert(id, BasicBlock::new(id));
        id
    }

    /// Add edge
    pub fn add_edge(&mut self, from: BlockId, to: BlockId) {
        if let Some(block) = self.blocks.get_mut(&from) {
            if !block.successors.contains(&to) {
                block.successors.push(to);
            }
        }
        if let Some(block) = self.blocks.get_mut(&to) {
            if !block.predecessors.contains(&from) {
                block.predecessors.push(from);
            }
        }
    }

    /// Remove edge
    #[inline]
    pub fn remove_edge(&mut self, from: BlockId, to: BlockId) {
        if let Some(block) = self.blocks.get_mut(&from) {
            block.successors.retain(|&b| b != to);
        }
        if let Some(block) = self.blocks.get_mut(&to) {
            block.predecessors.retain(|&b| b != from);
        }
    }

    /// Get block
    #[inline(always)]
    pub fn get_block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.get(&id)
    }

    /// Get block mutably
    #[inline(always)]
    pub fn get_block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
        self.blocks.get_mut(&id)
    }

    /// Block count
    #[inline(always)]
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Get all block IDs in order
    #[inline(always)]
    pub fn block_ids(&self) -> Vec<BlockId> {
        self.blocks.keys().copied().collect()
    }

    /// Get dominators (simplified)
    #[inline]
    pub fn dominators(&self) -> BTreeMap<BlockId, Vec<BlockId>> {
        // Simplified dominator calculation
        let mut doms = BTreeMap::new();
        for &id in self.blocks.keys() {
            doms.insert(id, vec![id]);
        }
        doms
    }

    /// Get post-dominators
    #[inline]
    pub fn post_dominators(&self) -> BTreeMap<BlockId, Vec<BlockId>> {
        // Simplified post-dominator calculation
        let mut pdoms = BTreeMap::new();
        for &id in self.blocks.keys() {
            pdoms.insert(id, vec![id]);
        }
        pdoms
    }

    /// Get reverse post order
    pub fn reverse_post_order(&self) -> Vec<BlockId> {
        let mut visited = BTreeMap::new();
        let mut order = Vec::new();

        fn dfs(
            cfg: &ControlFlowGraph,
            block: BlockId,
            visited: &mut BTreeMap<BlockId, bool>,
            order: &mut Vec<BlockId>,
        ) {
            if visited.contains_key(&block) {
                return;
            }
            visited.insert(block, true);

            if let Some(b) = cfg.blocks.get(&block) {
                for &succ in &b.successors {
                    dfs(cfg, succ, visited, order);
                }
            }
            order.push(block);
        }

        dfs(self, self.entry, &mut visited, &mut order);
        order.reverse();
        order
    }

    /// Find back edges (for loop detection)
    pub fn back_edges(&self) -> Vec<(BlockId, BlockId)> {
        let dom = self.dominators();
        let mut back_edges = Vec::new();

        for (id, block) in &self.blocks {
            for &succ in &block.successors {
                // An edge is a back edge if the target dominates the source
                if dom.get(id).is_some_and(|doms| doms.contains(&succ)) {
                    back_edges.push((*id, succ));
                }
            }
        }

        back_edges
    }
}

impl Default for ControlFlowGraph {
    fn default() -> Self {
        Self::new()
    }
}
