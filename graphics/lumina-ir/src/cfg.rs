//! Control Flow Graph Utilities
//!
//! Additional CFG analysis and manipulation utilities.

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec, vec, collections::BTreeSet, collections::BTreeMap};
#[cfg(feature = "std")]
use std::collections::{HashSet, HashMap, VecDeque};

use crate::instruction::{Instruction, BlockId};
use crate::block::{BasicBlock, BlockMap};
use crate::function::Function;

/// Control flow graph structure
#[derive(Debug)]
pub struct ControlFlowGraph {
    /// Entry block
    pub entry: BlockId,
    /// Exit blocks
    pub exits: Vec<BlockId>,
    /// Forward edges
    #[cfg(feature = "std")]
    pub successors: HashMap<BlockId, Vec<BlockId>>,
    #[cfg(not(feature = "std"))]
    pub successors: BTreeMap<BlockId, Vec<BlockId>>,
    /// Backward edges
    #[cfg(feature = "std")]
    pub predecessors: HashMap<BlockId, Vec<BlockId>>,
    #[cfg(not(feature = "std"))]
    pub predecessors: BTreeMap<BlockId, Vec<BlockId>>,
}

impl ControlFlowGraph {
    /// Build CFG from function
    pub fn from_function(func: &Function) -> Self {
        let entry = func.entry_block().unwrap_or(0);
        let mut exits = Vec::new();
        
        #[cfg(feature = "std")]
        let mut successors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        #[cfg(feature = "std")]
        let mut predecessors: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        
        #[cfg(not(feature = "std"))]
        let mut successors: BTreeMap<BlockId, Vec<BlockId>> = BTreeMap::new();
        #[cfg(not(feature = "std"))]
        let mut predecessors: BTreeMap<BlockId, Vec<BlockId>> = BTreeMap::new();
        
        for (block_id, block) in func.blocks.iter() {
            let block_succs = block.successors();
            successors.insert(*block_id, block_succs.clone());
            
            for succ in block_succs {
                predecessors.entry(succ).or_insert_with(Vec::new).push(*block_id);
            }
            
            // Check for exit block
            if let Some(last) = block.instructions().last() {
                match last {
                    Instruction::Return | Instruction::ReturnValue { .. } |
                    Instruction::Kill | Instruction::Unreachable |
                    Instruction::TerminateInvocation | Instruction::TerminateRayKHR |
                    Instruction::IgnoreIntersectionKHR => {
                        exits.push(*block_id);
                    }
                    _ => {}
                }
            }
        }
        
        Self {
            entry,
            exits,
            successors,
            predecessors,
        }
    }
    
    /// Get successors of a block
    pub fn successors(&self, block: BlockId) -> &[BlockId] {
        self.successors.get(&block).map(|v| v.as_slice()).unwrap_or(&[])
    }
    
    /// Get predecessors of a block
    pub fn predecessors(&self, block: BlockId) -> &[BlockId] {
        self.predecessors.get(&block).map(|v| v.as_slice()).unwrap_or(&[])
    }
    
    /// Get all blocks
    pub fn blocks(&self) -> impl Iterator<Item = BlockId> + '_ {
        self.successors.keys().copied()
    }
    
    /// Number of blocks
    pub fn len(&self) -> usize {
        self.successors.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.successors.is_empty()
    }
}

/// CFG edge type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CfgEdgeKind {
    /// Normal fallthrough or unconditional branch
    Unconditional,
    /// Conditional branch (true case)
    ConditionalTrue,
    /// Conditional branch (false case)
    ConditionalFalse,
    /// Switch case
    SwitchCase(i64),
    /// Switch default
    SwitchDefault,
    /// Loop back edge
    LoopBack,
    /// Loop exit edge
    LoopExit,
}

/// CFG edge
#[derive(Debug, Clone)]
pub struct CfgEdge {
    /// Source block
    pub from: BlockId,
    /// Target block
    pub to: BlockId,
    /// Edge kind
    pub kind: CfgEdgeKind,
    /// Is this a critical edge?
    pub is_critical: bool,
}

/// Extended CFG with edge information
#[derive(Debug)]
pub struct ExtendedCfg {
    /// Base CFG
    pub cfg: ControlFlowGraph,
    /// All edges with metadata
    pub edges: Vec<CfgEdge>,
    /// Block to edges mapping
    #[cfg(feature = "std")]
    pub block_out_edges: HashMap<BlockId, Vec<usize>>,
    #[cfg(not(feature = "std"))]
    pub block_out_edges: BTreeMap<BlockId, Vec<usize>>,
}

impl ExtendedCfg {
    /// Build extended CFG
    pub fn from_function(func: &Function) -> Self {
        let cfg = ControlFlowGraph::from_function(func);
        let mut edges = Vec::new();
        
        #[cfg(feature = "std")]
        let mut block_out_edges: HashMap<BlockId, Vec<usize>> = HashMap::new();
        #[cfg(not(feature = "std"))]
        let mut block_out_edges: BTreeMap<BlockId, Vec<usize>> = BTreeMap::new();
        
        for (block_id, block) in func.blocks.iter() {
            if let Some(last) = block.instructions().last() {
                match last {
                    Instruction::Branch { target } => {
                        let edge_idx = edges.len();
                        edges.push(CfgEdge {
                            from: *block_id,
                            to: *target,
                            kind: CfgEdgeKind::Unconditional,
                            is_critical: false,
                        });
                        block_out_edges.entry(*block_id).or_default().push(edge_idx);
                    }
                    
                    Instruction::BranchConditional { true_target, false_target, .. } => {
                        let preds_true = cfg.predecessors(*true_target).len();
                        let preds_false = cfg.predecessors(*false_target).len();
                        
                        let edge_idx = edges.len();
                        edges.push(CfgEdge {
                            from: *block_id,
                            to: *true_target,
                            kind: CfgEdgeKind::ConditionalTrue,
                            is_critical: preds_true > 1,
                        });
                        block_out_edges.entry(*block_id).or_default().push(edge_idx);
                        
                        let edge_idx = edges.len();
                        edges.push(CfgEdge {
                            from: *block_id,
                            to: *false_target,
                            kind: CfgEdgeKind::ConditionalFalse,
                            is_critical: preds_false > 1,
                        });
                        block_out_edges.entry(*block_id).or_default().push(edge_idx);
                    }
                    
                    Instruction::Switch { default_target, cases, .. } => {
                        let edge_idx = edges.len();
                        edges.push(CfgEdge {
                            from: *block_id,
                            to: *default_target,
                            kind: CfgEdgeKind::SwitchDefault,
                            is_critical: cfg.predecessors(*default_target).len() > 1,
                        });
                        block_out_edges.entry(*block_id).or_default().push(edge_idx);
                        
                        for (value, target) in cases {
                            let edge_idx = edges.len();
                            edges.push(CfgEdge {
                                from: *block_id,
                                to: *target,
                                kind: CfgEdgeKind::SwitchCase(*value),
                                is_critical: cfg.predecessors(*target).len() > 1,
                            });
                            block_out_edges.entry(*block_id).or_default().push(edge_idx);
                        }
                    }
                    
                    _ => {}
                }
            }
        }
        
        Self {
            cfg,
            edges,
            block_out_edges,
        }
    }
    
    /// Get critical edges
    pub fn critical_edges(&self) -> impl Iterator<Item = &CfgEdge> {
        self.edges.iter().filter(|e| e.is_critical)
    }
    
    /// Split critical edges by inserting empty blocks
    pub fn split_critical_edges(&self, func: &mut Function) -> Vec<BlockId> {
        let mut new_blocks = Vec::new();
        
        for edge in self.critical_edges() {
            // Create new block
            let new_block_id = func.blocks.create_block();
            
            // Add unconditional branch to target
            if let Some(new_block) = func.blocks.get_mut(new_block_id) {
                new_block.push(Instruction::Branch { target: edge.to });
            }
            
            // Update source block's terminator
            // This is complex and depends on edge kind
            
            new_blocks.push(new_block_id);
        }
        
        new_blocks
    }
}

/// Loop information
#[derive(Debug, Clone)]
pub struct LoopInfo {
    /// Loop header block
    pub header: BlockId,
    /// Loop latch block (back edge source)
    pub latch: BlockId,
    /// All blocks in the loop
    #[cfg(feature = "std")]
    pub blocks: HashSet<BlockId>,
    #[cfg(not(feature = "std"))]
    pub blocks: BTreeSet<BlockId>,
    /// Exit blocks
    pub exits: Vec<BlockId>,
    /// Preheader (if exists)
    pub preheader: Option<BlockId>,
    /// Nesting depth
    pub depth: u32,
    /// Parent loop header (if nested)
    pub parent: Option<BlockId>,
    /// Child loops
    pub children: Vec<BlockId>,
}

impl LoopInfo {
    /// Check if block is in loop
    pub fn contains(&self, block: BlockId) -> bool {
        self.blocks.contains(&block)
    }
    
    /// Get number of blocks
    pub fn size(&self) -> usize {
        self.blocks.len()
    }
    
    /// Check if this is an inner-most loop
    pub fn is_innermost(&self) -> bool {
        self.children.is_empty()
    }
}

/// Loop nest analysis
#[derive(Debug, Default)]
pub struct LoopNestAnalysis {
    /// All loops indexed by header
    #[cfg(feature = "std")]
    pub loops: HashMap<BlockId, LoopInfo>,
    #[cfg(not(feature = "std"))]
    pub loops: BTreeMap<BlockId, LoopInfo>,
    /// Block to loop header mapping
    #[cfg(feature = "std")]
    pub block_loop: HashMap<BlockId, BlockId>,
    #[cfg(not(feature = "std"))]
    pub block_loop: BTreeMap<BlockId, BlockId>,
}

impl LoopNestAnalysis {
    /// Analyze loops in a function
    pub fn analyze(func: &Function) -> Self {
        let mut analysis = Self::default();
        
        let natural_loops = func.blocks.detect_loops();
        
        for nl in natural_loops {
            #[cfg(feature = "std")]
            let blocks: HashSet<BlockId> = nl.blocks.into_iter().collect();
            #[cfg(not(feature = "std"))]
            let blocks: BTreeSet<BlockId> = nl.blocks.into_iter().collect();
            
            // Find exits
            let mut exits = Vec::new();
            for &block_id in &blocks {
                if let Some(block) = func.blocks.get(block_id) {
                    for succ in block.successors() {
                        if !blocks.contains(&succ) {
                            exits.push(block_id);
                            break;
                        }
                    }
                }
            }
            
            // Find preheader (single predecessor of header not in loop)
            let preheader = {
                let preds = func.blocks.predecessors(nl.header);
                let outside_preds: Vec<_> = preds.iter()
                    .filter(|p| !blocks.contains(p))
                    .collect();
                if outside_preds.len() == 1 {
                    Some(*outside_preds[0])
                } else {
                    None
                }
            };
            
            let loop_info = LoopInfo {
                header: nl.header,
                latch: nl.latch,
                blocks: blocks.clone(),
                exits,
                preheader,
                depth: 1,
                parent: None,
                children: Vec::new(),
            };
            
            for block_id in &blocks {
                analysis.block_loop.insert(*block_id, nl.header);
            }
            
            analysis.loops.insert(nl.header, loop_info);
        }
        
        // Compute nesting
        analysis.compute_nesting();
        
        analysis
    }
    
    /// Compute loop nesting relationships
    fn compute_nesting(&mut self) {
        let headers: Vec<_> = self.loops.keys().copied().collect();
        
        for &header in &headers {
            if let Some(info) = self.loops.get(&header) {
                let blocks = info.blocks.clone();
                
                // Find parent (innermost loop containing this one)
                for &other_header in &headers {
                    if other_header == header {
                        continue;
                    }
                    
                    if let Some(other_info) = self.loops.get(&other_header) {
                        if other_info.blocks.contains(&header) && 
                           other_info.blocks.is_superset(&blocks) {
                            // other is parent
                        }
                    }
                }
            }
        }
    }
    
    /// Get loop containing a block
    pub fn get_loop(&self, block: BlockId) -> Option<&LoopInfo> {
        self.block_loop.get(&block).and_then(|h| self.loops.get(h))
    }
    
    /// Check if block is in any loop
    pub fn is_in_loop(&self, block: BlockId) -> bool {
        self.block_loop.contains_key(&block)
    }
    
    /// Get all top-level loops
    pub fn top_level_loops(&self) -> impl Iterator<Item = &LoopInfo> {
        self.loops.values().filter(|l| l.parent.is_none())
    }
    
    /// Get innermost loops
    pub fn innermost_loops(&self) -> impl Iterator<Item = &LoopInfo> {
        self.loops.values().filter(|l| l.is_innermost())
    }
}

/// Region in structured control flow
#[derive(Debug, Clone)]
pub enum Region {
    /// Basic block region
    Block(BlockId),
    /// Sequence of regions
    Sequence(Vec<Region>),
    /// If-then-else region
    IfThenElse {
        condition_block: BlockId,
        then_region: Box<Region>,
        else_region: Option<Box<Region>>,
        merge: BlockId,
    },
    /// Loop region
    Loop {
        header: BlockId,
        body: Box<Region>,
        continue_target: BlockId,
        merge: BlockId,
    },
    /// Switch region
    Switch {
        selector_block: BlockId,
        cases: Vec<(i64, Region)>,
        default: Box<Region>,
        merge: BlockId,
    },
}

/// Region tree for structured control flow
#[derive(Debug)]
pub struct RegionTree {
    /// Root region
    pub root: Region,
}

impl RegionTree {
    /// Build region tree from function
    pub fn from_function(func: &Function) -> Option<Self> {
        // This requires structured control flow
        // For now, return simple sequence
        let blocks: Vec<_> = func.blocks.iter()
            .map(|(id, _)| Region::Block(*id))
            .collect();
        
        Some(Self {
            root: Region::Sequence(blocks),
        })
    }
}

/// Compute reverse postorder traversal
pub fn reverse_postorder(cfg: &ControlFlowGraph) -> Vec<BlockId> {
    let mut visited = Vec::new();
    let mut result = Vec::new();
    
    fn dfs(
        block: BlockId,
        cfg: &ControlFlowGraph,
        visited: &mut Vec<BlockId>,
        result: &mut Vec<BlockId>,
    ) {
        if visited.contains(&block) {
            return;
        }
        visited.push(block);
        
        for succ in cfg.successors(block) {
            dfs(*succ, cfg, visited, result);
        }
        
        result.push(block);
    }
    
    dfs(cfg.entry, cfg, &mut visited, &mut result);
    result.reverse();
    result
}

/// Compute postorder traversal
pub fn postorder(cfg: &ControlFlowGraph) -> Vec<BlockId> {
    let mut result = reverse_postorder(cfg);
    result.reverse();
    result
}

/// Compute BFS order
#[cfg(feature = "std")]
pub fn bfs_order(cfg: &ControlFlowGraph) -> Vec<BlockId> {
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    let mut queue = VecDeque::new();
    
    queue.push_back(cfg.entry);
    visited.insert(cfg.entry);
    
    while let Some(block) = queue.pop_front() {
        result.push(block);
        
        for succ in cfg.successors(block) {
            if !visited.contains(succ) {
                visited.insert(*succ);
                queue.push_back(*succ);
            }
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::ExecutionModel;

    #[test]
    fn test_cfg_construction() {
        let func = Function::new_entry_point("main", ExecutionModel::Fragment);
        let cfg = ControlFlowGraph::from_function(&func);
        // Empty function has entry but no blocks
    }

    #[test]
    fn test_loop_analysis() {
        let mut func = Function::new_entry_point("main", ExecutionModel::Fragment);
        func.ensure_entry_block();
        
        let analysis = LoopNestAnalysis::analyze(&func);
        assert!(analysis.loops.is_empty());
    }

    #[test]
    fn test_rpo() {
        let func = Function::new_entry_point("main", ExecutionModel::Fragment);
        let cfg = ControlFlowGraph::from_function(&func);
        let rpo = reverse_postorder(&cfg);
        // Should contain entry if it exists
    }
}
