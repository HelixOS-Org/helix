//! Basic Blocks
//!
//! This module defines basic blocks for control flow in the Lumina IR.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use crate::instruction::{BlockId, Instruction, LoopControl, SelectionControl};

/// A basic block in the IR
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Block identifier
    pub id: BlockId,
    /// Block label (debug name)
    pub label: Option<String>,
    /// Instructions in the block
    pub instructions: Vec<Instruction>,
    /// Predecessor blocks
    pub predecessors: Vec<BlockId>,
    /// Successor blocks
    pub successors: Vec<BlockId>,
    /// Merge block (for structured control flow)
    pub merge_block: Option<BlockId>,
    /// Continue target (for loops)
    pub continue_target: Option<BlockId>,
    /// Loop control hints
    pub loop_control: Option<LoopControl>,
    /// Selection control hints
    pub selection_control: Option<SelectionControl>,
    /// Is this an entry block
    pub is_entry: bool,
    /// Is this an exit block
    pub is_exit: bool,
}

impl BasicBlock {
    /// Create a new basic block
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            label: None,
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
            merge_block: None,
            continue_target: None,
            loop_control: None,
            selection_control: None,
            is_entry: false,
            is_exit: false,
        }
    }

    /// Create an entry block
    pub fn entry(id: BlockId) -> Self {
        let mut block = Self::new(id);
        block.is_entry = true;
        block
    }

    /// Set the label
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add an instruction to the block
    pub fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Add a predecessor
    pub fn add_predecessor(&mut self, pred: BlockId) {
        if !self.predecessors.contains(&pred) {
            self.predecessors.push(pred);
        }
    }

    /// Add a successor
    pub fn add_successor(&mut self, succ: BlockId) {
        if !self.successors.contains(&succ) {
            self.successors.push(succ);
        }
    }

    /// Remove a predecessor
    pub fn remove_predecessor(&mut self, pred: BlockId) {
        self.predecessors.retain(|&p| p != pred);
    }

    /// Remove a successor
    pub fn remove_successor(&mut self, succ: BlockId) {
        self.successors.retain(|&s| s != succ);
    }

    /// Get the terminator instruction
    pub fn terminator(&self) -> Option<&Instruction> {
        self.instructions.last().filter(|i| i.is_terminator())
    }

    /// Get a mutable reference to the terminator
    pub fn terminator_mut(&mut self) -> Option<&mut Instruction> {
        self.instructions.last_mut().filter(|i| i.is_terminator())
    }

    /// Check if the block has a terminator
    pub fn has_terminator(&self) -> bool {
        self.terminator().is_some()
    }

    /// Check if this is an empty block (only label, no instructions)
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Get the number of instructions
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if this block ends with a branch
    pub fn ends_with_branch(&self) -> bool {
        matches!(
            self.terminator(),
            Some(Instruction::Branch { .. }) | Some(Instruction::BranchConditional { .. })
        )
    }

    /// Check if this block ends with a return
    pub fn ends_with_return(&self) -> bool {
        matches!(
            self.terminator(),
            Some(Instruction::Return) | Some(Instruction::ReturnValue { .. })
        )
    }

    /// Check if this is a loop header
    pub fn is_loop_header(&self) -> bool {
        self.continue_target.is_some()
    }

    /// Check if this is a merge point
    pub fn is_merge_point(&self) -> bool {
        self.predecessors.len() > 1
    }

    /// Get target blocks for the terminator
    pub fn branch_targets(&self) -> Vec<BlockId> {
        match self.terminator() {
            Some(Instruction::Branch { target }) => vec![*target],
            Some(Instruction::BranchConditional {
                true_target,
                false_target,
                ..
            }) => vec![*true_target, *false_target],
            Some(Instruction::Switch {
                default_target,
                cases,
                ..
            }) => {
                let mut targets = vec![*default_target];
                targets.extend(cases.iter().map(|(_, t)| *t));
                targets
            },
            _ => Vec::new(),
        }
    }

    /// Iterate over instructions
    pub fn iter(&self) -> impl Iterator<Item = &Instruction> {
        self.instructions.iter()
    }

    /// Iterate over instructions mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Instruction> {
        self.instructions.iter_mut()
    }

    /// Insert instruction at index
    pub fn insert(&mut self, index: usize, instruction: Instruction) {
        self.instructions.insert(index, instruction);
    }

    /// Remove instruction at index
    pub fn remove(&mut self, index: usize) -> Instruction {
        self.instructions.remove(index)
    }

    /// Replace instruction at index
    pub fn replace(&mut self, index: usize, instruction: Instruction) -> Instruction {
        core::mem::replace(&mut self.instructions[index], instruction)
    }

    /// Split the block at the given instruction index
    /// Returns the new block containing instructions from the split point
    pub fn split_at(&mut self, index: usize, new_id: BlockId) -> BasicBlock {
        let tail = self.instructions.split_off(index);
        let mut new_block = BasicBlock::new(new_id);
        new_block.instructions = tail;
        new_block.successors = core::mem::take(&mut self.successors);
        new_block.predecessors = vec![self.id];
        self.successors = vec![new_id];
        new_block
    }

    /// Merge another block into this one (for block fusion)
    pub fn merge(&mut self, other: BasicBlock) {
        // Remove terminator from this block
        if self.has_terminator() {
            self.instructions.pop();
        }
        // Append other block's instructions
        self.instructions.extend(other.instructions);
        // Update successors
        self.successors = other.successors;
    }

    /// Set as loop header
    pub fn set_loop_header(
        &mut self,
        merge: BlockId,
        continue_target: BlockId,
        control: LoopControl,
    ) {
        self.merge_block = Some(merge);
        self.continue_target = Some(continue_target);
        self.loop_control = Some(control);
    }

    /// Set as selection header
    pub fn set_selection_header(&mut self, merge: BlockId, control: SelectionControl) {
        self.merge_block = Some(merge);
        self.selection_control = Some(control);
    }

    /// Clear merge info
    pub fn clear_merge_info(&mut self) {
        self.merge_block = None;
        self.continue_target = None;
        self.loop_control = None;
        self.selection_control = None;
    }
}

/// Block iterator modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockOrder {
    /// Reverse post-order (good for forward dataflow)
    ReversePostOrder,
    /// Post-order (good for backward dataflow)
    PostOrder,
    /// Breadth-first search
    BreadthFirst,
    /// Depth-first search
    DepthFirst,
}

/// Compute reverse post-order of blocks
pub fn compute_rpo(entry: BlockId, blocks: &[BasicBlock]) -> Vec<BlockId> {
    let mut visited = vec![false; blocks.len()];
    let mut post_order = Vec::with_capacity(blocks.len());

    fn dfs(
        block_id: BlockId,
        blocks: &[BasicBlock],
        visited: &mut [bool],
        post_order: &mut Vec<BlockId>,
    ) {
        let idx = block_id as usize;
        if idx >= visited.len() || visited[idx] {
            return;
        }
        visited[idx] = true;

        if let Some(block) = blocks.iter().find(|b| b.id == block_id) {
            for &succ in &block.successors {
                dfs(succ, blocks, visited, post_order);
            }
        }
        post_order.push(block_id);
    }

    dfs(entry, blocks, &mut visited, &mut post_order);
    post_order.reverse();
    post_order
}

/// Compute post-order of blocks
pub fn compute_po(entry: BlockId, blocks: &[BasicBlock]) -> Vec<BlockId> {
    let mut rpo = compute_rpo(entry, blocks);
    rpo.reverse();
    rpo
}

/// Block map for quick lookup
#[derive(Debug, Default)]
pub struct BlockMap {
    blocks: Vec<BasicBlock>,
    entry: Option<BlockId>,
    next_id: BlockId,
}

impl BlockMap {
    /// Create a new block map
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            entry: None,
            next_id: 0,
        }
    }

    /// Create a new block
    pub fn create_block(&mut self) -> BlockId {
        let id = self.next_id;
        self.next_id += 1;
        self.blocks.push(BasicBlock::new(id));
        id
    }

    /// Create an entry block
    pub fn create_entry_block(&mut self) -> BlockId {
        let id = self.next_id;
        self.next_id += 1;
        self.blocks.push(BasicBlock::entry(id));
        self.entry = Some(id);
        id
    }

    /// Get entry block ID
    pub fn entry(&self) -> Option<BlockId> {
        self.entry
    }

    /// Set entry block
    pub fn set_entry(&mut self, id: BlockId) {
        self.entry = Some(id);
        if let Some(block) = self.get_mut(id) {
            block.is_entry = true;
        }
    }

    /// Get a block by ID
    pub fn get(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.iter().find(|b| b.id == id)
    }

    /// Get a mutable block by ID
    pub fn get_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
        self.blocks.iter_mut().find(|b| b.id == id)
    }

    /// Check if a block exists
    pub fn contains(&self, id: BlockId) -> bool {
        self.blocks.iter().any(|b| b.id == id)
    }

    /// Remove a block
    pub fn remove(&mut self, id: BlockId) -> Option<BasicBlock> {
        if let Some(pos) = self.blocks.iter().position(|b| b.id == id) {
            Some(self.blocks.remove(pos))
        } else {
            None
        }
    }

    /// Get the number of blocks
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Iterate over blocks
    pub fn iter(&self) -> impl Iterator<Item = &BasicBlock> {
        self.blocks.iter()
    }

    /// Iterate over blocks mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut BasicBlock> {
        self.blocks.iter_mut()
    }

    /// Get blocks in reverse post-order
    pub fn rpo(&self) -> Vec<BlockId> {
        if let Some(entry) = self.entry {
            compute_rpo(entry, &self.blocks)
        } else {
            Vec::new()
        }
    }

    /// Get blocks in post-order
    pub fn po(&self) -> Vec<BlockId> {
        if let Some(entry) = self.entry {
            compute_po(entry, &self.blocks)
        } else {
            Vec::new()
        }
    }

    /// Add an edge between blocks
    pub fn add_edge(&mut self, from: BlockId, to: BlockId) {
        if let Some(from_block) = self.get_mut(from) {
            from_block.add_successor(to);
        }
        if let Some(to_block) = self.get_mut(to) {
            to_block.add_predecessor(from);
        }
    }

    /// Remove an edge between blocks
    pub fn remove_edge(&mut self, from: BlockId, to: BlockId) {
        if let Some(from_block) = self.get_mut(from) {
            from_block.remove_successor(to);
        }
        if let Some(to_block) = self.get_mut(to) {
            to_block.remove_predecessor(from);
        }
    }

    /// Update edges based on terminators
    pub fn rebuild_edges(&mut self) {
        // Clear all edges
        for block in &mut self.blocks {
            block.successors.clear();
            block.predecessors.clear();
        }

        // Rebuild from terminators
        let edges: Vec<_> = self
            .blocks
            .iter()
            .flat_map(|b| b.branch_targets().into_iter().map(move |t| (b.id, t)))
            .collect();

        for (from, to) in edges {
            self.add_edge(from, to);
        }
    }

    /// Find exit blocks (blocks with no successors or ending in return)
    pub fn exit_blocks(&self) -> Vec<BlockId> {
        self.blocks
            .iter()
            .filter(|b| b.successors.is_empty() || b.ends_with_return())
            .map(|b| b.id)
            .collect()
    }

    /// Get all blocks as a slice
    pub fn as_slice(&self) -> &[BasicBlock] {
        &self.blocks
    }

    /// Get all blocks as a mutable slice
    pub fn as_mut_slice(&mut self) -> &mut [BasicBlock] {
        &mut self.blocks
    }
}

/// Edge in the control flow graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Edge {
    pub from: BlockId,
    pub to: BlockId,
}

impl Edge {
    pub const fn new(from: BlockId, to: BlockId) -> Self {
        Self { from, to }
    }
}

/// Edge classification for CFG analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Tree edge in DFS tree
    Tree,
    /// Back edge (loop)
    Back,
    /// Forward edge
    Forward,
    /// Cross edge
    Cross,
}

/// Classify edges in the CFG
pub fn classify_edges(entry: BlockId, blocks: &[BasicBlock]) -> Vec<(Edge, EdgeKind)> {
    let mut result = Vec::new();
    let mut visited = vec![false; blocks.len()];
    let mut finished = vec![false; blocks.len()];
    let mut pre_order = vec![0u32; blocks.len()];
    let mut counter = 0u32;

    fn dfs(
        block_id: BlockId,
        blocks: &[BasicBlock],
        visited: &mut [bool],
        finished: &mut [bool],
        pre_order: &mut [u32],
        counter: &mut u32,
        result: &mut Vec<(Edge, EdgeKind)>,
    ) {
        let idx = block_id as usize;
        if idx >= visited.len() {
            return;
        }

        visited[idx] = true;
        pre_order[idx] = *counter;
        *counter += 1;

        if let Some(block) = blocks.iter().find(|b| b.id == block_id) {
            for &succ in &block.successors {
                let succ_idx = succ as usize;
                if succ_idx >= visited.len() {
                    continue;
                }

                let edge = Edge::new(block_id, succ);

                if !visited[succ_idx] {
                    result.push((edge, EdgeKind::Tree));
                    dfs(succ, blocks, visited, finished, pre_order, counter, result);
                } else if !finished[succ_idx] {
                    result.push((edge, EdgeKind::Back));
                } else if pre_order[idx] < pre_order[succ_idx] {
                    result.push((edge, EdgeKind::Forward));
                } else {
                    result.push((edge, EdgeKind::Cross));
                }
            }
        }

        finished[idx] = true;
    }

    dfs(
        entry,
        blocks,
        &mut visited,
        &mut finished,
        &mut pre_order,
        &mut counter,
        &mut result,
    );
    result
}

/// Find back edges (loops) in the CFG
pub fn find_back_edges(entry: BlockId, blocks: &[BasicBlock]) -> Vec<Edge> {
    classify_edges(entry, blocks)
        .into_iter()
        .filter(|(_, kind)| *kind == EdgeKind::Back)
        .map(|(edge, _)| edge)
        .collect()
}

/// Find natural loops in the CFG
pub fn find_natural_loops(entry: BlockId, blocks: &[BasicBlock]) -> Vec<NaturalLoop> {
    let back_edges = find_back_edges(entry, blocks);
    let mut loops = Vec::new();

    for edge in back_edges {
        let header = edge.to;
        let mut body = vec![header];
        let mut worklist = vec![edge.from];

        while let Some(block_id) = worklist.pop() {
            if body.contains(&block_id) {
                continue;
            }
            body.push(block_id);

            if let Some(block) = blocks.iter().find(|b| b.id == block_id) {
                for &pred in &block.predecessors {
                    if !body.contains(&pred) {
                        worklist.push(pred);
                    }
                }
            }
        }

        loops.push(NaturalLoop {
            header,
            back_edge: edge,
            body,
        });
    }

    loops
}

/// Natural loop structure
#[derive(Debug, Clone)]
pub struct NaturalLoop {
    /// Loop header block
    pub header: BlockId,
    /// Back edge that defines the loop
    pub back_edge: Edge,
    /// All blocks in the loop body
    pub body: Vec<BlockId>,
}

impl NaturalLoop {
    /// Check if a block is in this loop
    pub fn contains(&self, block: BlockId) -> bool {
        self.body.contains(&block)
    }

    /// Get the latch block (source of back edge)
    pub fn latch(&self) -> BlockId {
        self.back_edge.from
    }

    /// Get exit blocks (blocks with successors outside the loop)
    pub fn exit_blocks(&self, blocks: &[BasicBlock]) -> Vec<BlockId> {
        self.body
            .iter()
            .filter(|&&b| {
                if let Some(block) = blocks.iter().find(|bb| bb.id == b) {
                    block.successors.iter().any(|s| !self.body.contains(s))
                } else {
                    false
                }
            })
            .copied()
            .collect()
    }

    /// Get loop depth (1 for outermost loops)
    pub fn depth(&self, all_loops: &[NaturalLoop]) -> usize {
        let mut depth = 1;
        for other in all_loops {
            if other.header != self.header && other.contains(self.header) {
                depth += 1;
            }
        }
        depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_block() {
        let mut block = BasicBlock::new(0);
        assert!(block.is_empty());

        block.push(Instruction::Nop);
        assert!(!block.is_empty());
        assert_eq!(block.len(), 1);
    }

    #[test]
    fn test_block_map() {
        let mut map = BlockMap::new();
        let entry = map.create_entry_block();
        let block1 = map.create_block();
        let block2 = map.create_block();

        map.add_edge(entry, block1);
        map.add_edge(entry, block2);
        map.add_edge(block1, block2);

        assert_eq!(map.len(), 3);
        assert_eq!(map.entry(), Some(entry));

        let entry_block = map.get(entry).unwrap();
        assert_eq!(entry_block.successors.len(), 2);
    }

    #[test]
    fn test_block_split() {
        let mut block = BasicBlock::new(0);
        block.push(Instruction::Nop);
        block.push(Instruction::Nop);
        block.push(Instruction::Nop);

        let new_block = block.split_at(1, 1);

        assert_eq!(block.len(), 1);
        assert_eq!(new_block.len(), 2);
        assert_eq!(block.successors, vec![1]);
        assert_eq!(new_block.predecessors, vec![0]);
    }

    #[test]
    fn test_rpo() {
        let mut map = BlockMap::new();
        let b0 = map.create_entry_block();
        let b1 = map.create_block();
        let b2 = map.create_block();

        map.add_edge(b0, b1);
        map.add_edge(b0, b2);
        map.add_edge(b1, b2);

        let rpo = map.rpo();
        assert_eq!(rpo.len(), 3);
        assert_eq!(rpo[0], b0); // Entry first
    }
}
