//! Dominator Tree Analysis
//!
//! Dominance computation for control flow analysis.

#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, collections::BTreeSet, vec, vec::Vec};
#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};

use crate::block::BlockMap;
use crate::cfg::ControlFlowGraph;
use crate::function::Function;
use crate::instruction::BlockId;

/// Dominator tree node
#[derive(Debug, Clone)]
pub struct DomTreeNode {
    /// Block ID
    pub block: BlockId,
    /// Immediate dominator
    pub idom: Option<BlockId>,
    /// Dominated children
    pub children: Vec<BlockId>,
    /// Depth in tree
    pub depth: u32,
}

/// Dominator tree
#[derive(Debug)]
pub struct DominatorTree {
    /// Entry block
    pub entry: BlockId,
    /// Nodes indexed by block ID
    #[cfg(feature = "std")]
    pub nodes: HashMap<BlockId, DomTreeNode>,
    #[cfg(not(feature = "std"))]
    pub nodes: BTreeMap<BlockId, DomTreeNode>,
}

impl DominatorTree {
    /// Compute dominator tree using lengauer-tarjan algorithm
    pub fn compute(cfg: &ControlFlowGraph) -> Self {
        let entry = cfg.entry;

        #[cfg(feature = "std")]
        let mut nodes: HashMap<BlockId, DomTreeNode> = HashMap::new();
        #[cfg(not(feature = "std"))]
        let mut nodes: BTreeMap<BlockId, DomTreeNode> = BTreeMap::new();

        // Simple quadratic algorithm for now
        // Could be replaced with lengauer-tarjan for O(n alpha(n))

        // Initialize
        for block in cfg.blocks() {
            nodes.insert(block, DomTreeNode {
                block,
                idom: None,
                children: Vec::new(),
                depth: 0,
            });
        }

        // Entry dominates itself (no idom)
        if let Some(entry_node) = nodes.get_mut(&entry) {
            entry_node.depth = 0;
        }

        // Iterative dataflow
        let mut changed = true;
        while changed {
            changed = false;

            let rpo = crate::cfg::reverse_postorder(cfg);

            for &block in &rpo {
                if block == entry {
                    continue;
                }

                // Find new idom
                let preds = cfg.predecessors(block);
                let mut new_idom: Option<BlockId> = None;

                for &pred in preds {
                    if nodes
                        .get(&pred)
                        .and_then(|n| if pred == entry { Some(pred) } else { n.idom })
                        .is_some()
                    {
                        match new_idom {
                            None => new_idom = Some(pred),
                            Some(current) => {
                                // Find common dominator
                                new_idom = Some(Self::intersect_tmp(&nodes, entry, current, pred));
                            },
                        }
                    }
                }

                if let Some(new) = new_idom {
                    if nodes.get(&block).and_then(|n| n.idom) != Some(new) {
                        if let Some(node) = nodes.get_mut(&block) {
                            node.idom = Some(new);
                        }
                        changed = true;
                    }
                }
            }
        }

        // Build children lists and compute depths
        let block_ids: Vec<_> = nodes.keys().copied().collect();
        for block in &block_ids {
            if let Some(node) = nodes.get(block) {
                if let Some(idom) = node.idom {
                    let parent_depth = nodes.get(&idom).map(|n| n.depth).unwrap_or(0);
                    if let Some(node) = nodes.get_mut(block) {
                        node.depth = parent_depth + 1;
                    }
                }
            }
        }

        for block in &block_ids {
            let idom = nodes.get(block).and_then(|n| n.idom);
            if let Some(parent) = idom {
                if let Some(parent_node) = nodes.get_mut(&parent) {
                    parent_node.children.push(*block);
                }
            }
        }

        Self { entry, nodes }
    }

    /// Intersect helper for idom computation
    #[cfg(feature = "std")]
    fn intersect_tmp(
        nodes: &HashMap<BlockId, DomTreeNode>,
        entry: BlockId,
        mut a: BlockId,
        mut b: BlockId,
    ) -> BlockId {
        while a != b {
            while Self::postorder_num(nodes, entry, a) < Self::postorder_num(nodes, entry, b) {
                a = nodes.get(&a).and_then(|n| n.idom).unwrap_or(entry);
            }
            while Self::postorder_num(nodes, entry, b) < Self::postorder_num(nodes, entry, a) {
                b = nodes.get(&b).and_then(|n| n.idom).unwrap_or(entry);
            }
        }
        a
    }

    #[cfg(not(feature = "std"))]
    fn intersect_tmp(
        nodes: &BTreeMap<BlockId, DomTreeNode>,
        entry: BlockId,
        mut a: BlockId,
        mut b: BlockId,
    ) -> BlockId {
        while a != b {
            while Self::postorder_num(nodes, entry, a) < Self::postorder_num(nodes, entry, b) {
                a = nodes.get(&a).and_then(|n| n.idom).unwrap_or(entry);
            }
            while Self::postorder_num(nodes, entry, b) < Self::postorder_num(nodes, entry, a) {
                b = nodes.get(&b).and_then(|n| n.idom).unwrap_or(entry);
            }
        }
        a
    }

    /// Get pseudo postorder number (using block id as proxy)
    #[cfg(feature = "std")]
    fn postorder_num(nodes: &HashMap<BlockId, DomTreeNode>, entry: BlockId, block: BlockId) -> u32 {
        if block == entry {
            u32::MAX
        } else {
            block as u32
        }
    }

    #[cfg(not(feature = "std"))]
    fn postorder_num(
        nodes: &BTreeMap<BlockId, DomTreeNode>,
        entry: BlockId,
        block: BlockId,
    ) -> u32 {
        if block == entry {
            u32::MAX
        } else {
            block as u32
        }
    }

    /// Get immediate dominator
    pub fn idom(&self, block: BlockId) -> Option<BlockId> {
        self.nodes.get(&block).and_then(|n| n.idom)
    }

    /// Get children (dominated blocks)
    pub fn children(&self, block: BlockId) -> &[BlockId] {
        self.nodes
            .get(&block)
            .map(|n| n.children.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a dominates b
    pub fn dominates(&self, a: BlockId, b: BlockId) -> bool {
        if a == b {
            return true;
        }

        let mut current = b;
        while let Some(idom) = self.idom(current) {
            if idom == a {
                return true;
            }
            current = idom;
        }

        false
    }

    /// Check if a strictly dominates b
    pub fn strictly_dominates(&self, a: BlockId, b: BlockId) -> bool {
        a != b && self.dominates(a, b)
    }

    /// Get depth in dominator tree
    pub fn depth(&self, block: BlockId) -> u32 {
        self.nodes.get(&block).map(|n| n.depth).unwrap_or(0)
    }

    /// Get dominator tree root
    pub fn root(&self) -> BlockId {
        self.entry
    }

    /// Iterate over nodes in preorder
    pub fn preorder(&self) -> Vec<BlockId> {
        let mut result = Vec::new();
        self.preorder_dfs(self.entry, &mut result);
        result
    }

    fn preorder_dfs(&self, block: BlockId, result: &mut Vec<BlockId>) {
        result.push(block);
        for &child in self.children(block) {
            self.preorder_dfs(child, result);
        }
    }

    /// Iterate over nodes in postorder
    pub fn postorder(&self) -> Vec<BlockId> {
        let mut result = Vec::new();
        self.postorder_dfs(self.entry, &mut result);
        result
    }

    fn postorder_dfs(&self, block: BlockId, result: &mut Vec<BlockId>) {
        for &child in self.children(block) {
            self.postorder_dfs(child, result);
        }
        result.push(block);
    }
}

/// Post-dominator tree (dominance from exits)
#[derive(Debug)]
pub struct PostDominatorTree {
    /// Virtual exit node
    pub exit: BlockId,
    /// Nodes indexed by block ID
    #[cfg(feature = "std")]
    pub nodes: HashMap<BlockId, DomTreeNode>,
    #[cfg(not(feature = "std"))]
    pub nodes: BTreeMap<BlockId, DomTreeNode>,
}

impl PostDominatorTree {
    /// Compute post-dominator tree
    pub fn compute(cfg: &ControlFlowGraph) -> Self {
        // Create reverse CFG
        #[cfg(feature = "std")]
        let mut reverse_succs: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        #[cfg(not(feature = "std"))]
        let mut reverse_succs: BTreeMap<BlockId, Vec<BlockId>> = BTreeMap::new();

        // Virtual exit node
        let exit = BlockId::MAX;

        for block in cfg.blocks() {
            reverse_succs.insert(block, Vec::new());
        }
        reverse_succs.insert(exit, Vec::new());

        // Add reverse edges
        for block in cfg.blocks() {
            for succ in cfg.successors(block) {
                reverse_succs.entry(*succ).or_default().push(block);
            }
        }

        // Connect exit blocks to virtual exit
        for &exit_block in &cfg.exits {
            reverse_succs.entry(exit).or_default().push(exit_block);
        }

        // Build reverse CFG
        let reverse_cfg = ControlFlowGraph {
            entry: exit,
            exits: vec![cfg.entry],
            successors: reverse_succs.clone(),
            predecessors: cfg.successors.clone(), // Swap for reverse
        };

        // Compute dominators on reverse CFG
        let dom_tree = DominatorTree::compute(&reverse_cfg);

        Self {
            exit,
            nodes: dom_tree.nodes,
        }
    }

    /// Get immediate post-dominator
    pub fn ipdom(&self, block: BlockId) -> Option<BlockId> {
        self.nodes.get(&block).and_then(|n| n.idom)
    }

    /// Check if a post-dominates b
    pub fn post_dominates(&self, a: BlockId, b: BlockId) -> bool {
        if a == b {
            return true;
        }

        let mut current = b;
        while let Some(ipdom) = self.ipdom(current) {
            if ipdom == a {
                return true;
            }
            if ipdom == self.exit {
                break;
            }
            current = ipdom;
        }

        false
    }
}

/// Dominance frontier
#[derive(Debug, Default)]
pub struct DominanceFrontier {
    #[cfg(feature = "std")]
    pub frontiers: HashMap<BlockId, HashSet<BlockId>>,
    #[cfg(not(feature = "std"))]
    pub frontiers: BTreeMap<BlockId, BTreeSet<BlockId>>,
}

impl DominanceFrontier {
    /// Compute dominance frontier
    pub fn compute(cfg: &ControlFlowGraph, dom_tree: &DominatorTree) -> Self {
        #[cfg(feature = "std")]
        let mut frontiers: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
        #[cfg(not(feature = "std"))]
        let mut frontiers: BTreeMap<BlockId, BTreeSet<BlockId>> = BTreeMap::new();

        for block in cfg.blocks() {
            frontiers.insert(block, Default::default());
        }

        for block in cfg.blocks() {
            let preds = cfg.predecessors(block);
            if preds.len() >= 2 {
                // Block is a join point
                for &pred in preds {
                    let mut runner = pred;
                    while Some(runner) != dom_tree.idom(block) && runner != block {
                        frontiers.entry(runner).or_default().insert(block);
                        if let Some(idom) = dom_tree.idom(runner) {
                            runner = idom;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Self { frontiers }
    }

    /// Get dominance frontier of a block
    #[cfg(feature = "std")]
    pub fn frontier(&self, block: BlockId) -> &HashSet<BlockId> {
        static EMPTY: HashSet<BlockId> = HashSet::new();
        self.frontiers.get(&block).unwrap_or(&EMPTY)
    }

    #[cfg(not(feature = "std"))]
    pub fn frontier(&self, block: BlockId) -> &BTreeSet<BlockId> {
        static EMPTY: BTreeSet<BlockId> = BTreeSet::new();
        self.frontiers.get(&block).unwrap_or(&EMPTY)
    }

    /// Compute iterated dominance frontier
    #[cfg(feature = "std")]
    pub fn iterated_frontier(&self, blocks: &[BlockId]) -> HashSet<BlockId> {
        let mut result = HashSet::new();
        let mut worklist: Vec<_> = blocks.to_vec();

        while let Some(block) = worklist.pop() {
            for &df_block in self.frontier(block) {
                if result.insert(df_block) {
                    worklist.push(df_block);
                }
            }
        }

        result
    }

    #[cfg(not(feature = "std"))]
    pub fn iterated_frontier(&self, blocks: &[BlockId]) -> BTreeSet<BlockId> {
        let mut result = BTreeSet::new();
        let mut worklist: Vec<_> = blocks.to_vec();

        while let Some(block) = worklist.pop() {
            for &df_block in self.frontier(block) {
                if result.insert(df_block) {
                    worklist.push(df_block);
                }
            }
        }

        result
    }
}

/// Check if block a dominates block b in a function
pub fn dominates(func: &Function, a: BlockId, b: BlockId) -> bool {
    let cfg = ControlFlowGraph::from_function(func);
    let dom_tree = DominatorTree::compute(&cfg);
    dom_tree.dominates(a, b)
}

/// Get immediate dominator of a block
pub fn immediate_dominator(func: &Function, block: BlockId) -> Option<BlockId> {
    let cfg = ControlFlowGraph::from_function(func);
    let dom_tree = DominatorTree::compute(&cfg);
    dom_tree.idom(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::ExecutionModel;

    #[test]
    fn test_dominator_tree_empty() {
        let func = Function::new_entry_point("main", ExecutionModel::Fragment);
        let cfg = ControlFlowGraph::from_function(&func);
        let dom_tree = DominatorTree::compute(&cfg);
        // Entry dominates itself
    }

    #[test]
    fn test_self_dominance() {
        let mut func = Function::new_entry_point("main", ExecutionModel::Fragment);
        let entry = func.ensure_entry_block();

        let cfg = ControlFlowGraph::from_function(&func);
        let dom_tree = DominatorTree::compute(&cfg);

        assert!(dom_tree.dominates(entry, entry));
    }

    #[test]
    fn test_dominance_frontier() {
        let mut func = Function::new_entry_point("main", ExecutionModel::Fragment);
        func.ensure_entry_block();

        let cfg = ControlFlowGraph::from_function(&func);
        let dom_tree = DominatorTree::compute(&cfg);
        let df = DominanceFrontier::compute(&cfg, &dom_tree);

        // Single block has empty frontier
    }
}
