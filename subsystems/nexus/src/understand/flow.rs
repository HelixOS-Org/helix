//! # Control Flow Analysis
//!
//! Control flow graph construction and analysis.
//! Detects loops, branches, dominators, and reachability.
//!
//! Part of Year 2 COGNITION - Q1: Code Understanding Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// BASIC BLOCK
// ============================================================================

/// Basic block
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Block ID
    pub id: u64,
    /// Block name/label
    pub label: String,
    /// Instructions
    pub instructions: Vec<Instruction>,
    /// Terminator
    pub terminator: Terminator,
    /// Predecessors
    pub predecessors: Vec<u64>,
    /// Successors
    pub successors: Vec<u64>,
    /// Loop depth
    pub loop_depth: u32,
    /// Is loop header
    pub is_loop_header: bool,
}

/// Instruction
#[derive(Debug, Clone)]
pub struct Instruction {
    /// Instruction ID
    pub id: u64,
    /// Opcode
    pub opcode: Opcode,
    /// Operands
    pub operands: Vec<Operand>,
    /// Result
    pub result: Option<String>,
    /// Source location
    pub source_line: u32,
}

/// Opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    // Bitwise
    And,
    Or,
    Xor,
    Shl,
    Shr,
    // Comparison
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    // Memory
    Load,
    Store,
    Alloca,
    // Call
    Call,
    Invoke,
    // Conversion
    Cast,
    Extend,
    Truncate,
    // Other
    Phi,
    Select,
    Nop,
}

/// Operand
#[derive(Debug, Clone)]
pub enum Operand {
    Register(String),
    Immediate(i64),
    Label(String),
    Function(String),
}

/// Terminator
#[derive(Debug, Clone)]
pub enum Terminator {
    /// Unconditional branch
    Branch(u64),
    /// Conditional branch
    CondBranch {
        condition: String,
        if_true: u64,
        if_false: u64,
    },
    /// Switch
    Switch {
        value: String,
        default: u64,
        cases: Vec<(i64, u64)>,
    },
    /// Return
    Return(Option<String>),
    /// Unreachable
    Unreachable,
    /// Invoke with exception handling
    InvokeUnwind {
        func: String,
        normal: u64,
        unwind: u64,
    },
}

// ============================================================================
// CONTROL FLOW GRAPH
// ============================================================================

/// Control flow graph
pub struct ControlFlowGraph {
    /// Function name
    name: String,
    /// Entry block
    entry: u64,
    /// Exit blocks
    exits: Vec<u64>,
    /// Blocks
    blocks: BTreeMap<u64, BasicBlock>,
    /// Next ID
    next_id: AtomicU64,
}

impl ControlFlowGraph {
    /// Create new CFG
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            entry: 0,
            exits: Vec::new(),
            blocks: BTreeMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Add block
    pub fn add_block(&mut self, label: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let block = BasicBlock {
            id,
            label: label.into(),
            instructions: Vec::new(),
            terminator: Terminator::Unreachable,
            predecessors: Vec::new(),
            successors: Vec::new(),
            loop_depth: 0,
            is_loop_header: false,
        };

        if self.blocks.is_empty() {
            self.entry = id;
        }

        self.blocks.insert(id, block);
        id
    }

    /// Set entry block
    pub fn set_entry(&mut self, id: u64) {
        self.entry = id;
    }

    /// Add exit block
    pub fn add_exit(&mut self, id: u64) {
        if !self.exits.contains(&id) {
            self.exits.push(id);
        }
    }

    /// Get block
    pub fn get_block(&self, id: u64) -> Option<&BasicBlock> {
        self.blocks.get(&id)
    }

    /// Get mutable block
    pub fn get_block_mut(&mut self, id: u64) -> Option<&mut BasicBlock> {
        self.blocks.get_mut(&id)
    }

    /// Add instruction to block
    pub fn add_instruction(
        &mut self,
        block_id: u64,
        opcode: Opcode,
        operands: Vec<Operand>,
    ) -> u64 {
        let inst_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let instruction = Instruction {
            id: inst_id,
            opcode,
            operands,
            result: None,
            source_line: 0,
        };

        if let Some(block) = self.blocks.get_mut(&block_id) {
            block.instructions.push(instruction);
        }

        inst_id
    }

    /// Set terminator
    pub fn set_terminator(&mut self, block_id: u64, terminator: Terminator) {
        // Get successors from terminator
        let successors: Vec<u64> = match &terminator {
            Terminator::Branch(target) => vec![*target],
            Terminator::CondBranch {
                if_true, if_false, ..
            } => vec![*if_true, *if_false],
            Terminator::Switch { default, cases, .. } => {
                let mut succs = vec![*default];
                succs.extend(cases.iter().map(|(_, b)| *b));
                succs
            },
            Terminator::Return(_) => vec![],
            Terminator::Unreachable => vec![],
            Terminator::InvokeUnwind { normal, unwind, .. } => vec![*normal, *unwind],
        };

        if let Some(block) = self.blocks.get_mut(&block_id) {
            block.terminator = terminator;
            block.successors = successors.clone();

            // Track exit blocks
            if matches!(block.terminator, Terminator::Return(_)) {
                self.add_exit(block_id);
            }
        }

        // Update predecessors
        for succ_id in successors {
            if let Some(succ) = self.blocks.get_mut(&succ_id) {
                if !succ.predecessors.contains(&block_id) {
                    succ.predecessors.push(block_id);
                }
            }
        }
    }

    /// Add edge
    pub fn add_edge(&mut self, from: u64, to: u64) {
        if let Some(from_block) = self.blocks.get_mut(&from) {
            if !from_block.successors.contains(&to) {
                from_block.successors.push(to);
            }
        }
        if let Some(to_block) = self.blocks.get_mut(&to) {
            if !to_block.predecessors.contains(&from) {
                to_block.predecessors.push(from);
            }
        }
    }

    /// Get entry
    pub fn entry(&self) -> u64 {
        self.entry
    }

    /// Get exits
    pub fn exits(&self) -> &[u64] {
        &self.exits
    }

    /// Block count
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Get all blocks
    pub fn blocks(&self) -> impl Iterator<Item = &BasicBlock> {
        self.blocks.values()
    }
}

// ============================================================================
// CFG ANALYSIS
// ============================================================================

/// CFG analyzer
pub struct CfgAnalyzer {
    /// Dominators (block -> immediate dominator)
    dominators: BTreeMap<u64, u64>,
    /// Post-dominators
    post_dominators: BTreeMap<u64, u64>,
    /// Dominator tree children
    dom_children: BTreeMap<u64, Vec<u64>>,
    /// Loop headers
    loop_headers: Vec<u64>,
    /// Loop bodies (header -> blocks in loop)
    loop_bodies: BTreeMap<u64, Vec<u64>>,
    /// Back edges (from -> to)
    back_edges: Vec<(u64, u64)>,
}

impl CfgAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            dominators: BTreeMap::new(),
            post_dominators: BTreeMap::new(),
            dom_children: BTreeMap::new(),
            loop_headers: Vec::new(),
            loop_bodies: BTreeMap::new(),
            back_edges: Vec::new(),
        }
    }

    /// Analyze CFG
    pub fn analyze(&mut self, cfg: &ControlFlowGraph) {
        self.compute_dominators(cfg);
        self.build_dominator_tree();
        self.find_loops(cfg);
    }

    /// Compute dominators using iterative algorithm
    fn compute_dominators(&mut self, cfg: &ControlFlowGraph) {
        self.dominators.clear();

        // Entry dominates itself
        self.dominators.insert(cfg.entry, cfg.entry);

        // Initially, all other blocks dominated by entry
        for id in cfg.blocks.keys() {
            if *id != cfg.entry {
                self.dominators.insert(*id, cfg.entry);
            }
        }

        // Iterate until fixed point
        let mut changed = true;
        while changed {
            changed = false;

            for block in cfg.blocks.values() {
                if block.id == cfg.entry {
                    continue;
                }

                // New dominator is intersection of all predecessors' dominators
                if let Some(new_dom) = self.intersect_dominators(cfg, &block.predecessors) {
                    if Some(&new_dom) != self.dominators.get(&block.id) {
                        self.dominators.insert(block.id, new_dom);
                        changed = true;
                    }
                }
            }
        }
    }

    fn intersect_dominators(&self, cfg: &ControlFlowGraph, preds: &[u64]) -> Option<u64> {
        if preds.is_empty() {
            return Some(cfg.entry);
        }

        // Find common dominator
        let mut result = preds[0];

        for &pred in &preds[1..] {
            result = self.find_common_dominator(result, pred);
        }

        Some(result)
    }

    fn find_common_dominator(&self, mut a: u64, mut b: u64) -> u64 {
        let mut a_doms = BTreeSet::new();

        // Collect a's dominators
        let mut current = a;
        loop {
            a_doms.insert(current);
            match self.dominators.get(&current) {
                Some(&dom) if dom != current => current = dom,
                _ => break,
            }
        }

        // Find first of b's dominators in a's set
        current = b;
        loop {
            if a_doms.contains(&current) {
                return current;
            }
            match self.dominators.get(&current) {
                Some(&dom) if dom != current => current = dom,
                _ => return current,
            }
        }
    }

    fn build_dominator_tree(&mut self) {
        self.dom_children.clear();

        for (&block, &dom) in &self.dominators {
            if block != dom {
                self.dom_children
                    .entry(dom)
                    .or_insert_with(Vec::new)
                    .push(block);
            }
        }
    }

    /// Find loops using back edges
    fn find_loops(&mut self, cfg: &ControlFlowGraph) {
        self.loop_headers.clear();
        self.back_edges.clear();
        self.loop_bodies.clear();

        // Find back edges (edge from block to its dominator)
        for block in cfg.blocks.values() {
            for &succ in &block.successors {
                if self.dominates(succ, block.id) {
                    self.back_edges.push((block.id, succ));
                    if !self.loop_headers.contains(&succ) {
                        self.loop_headers.push(succ);
                    }
                }
            }
        }

        // Compute loop bodies
        for &(tail, header) in &self.back_edges {
            let body = self.compute_natural_loop(cfg, header, tail);
            self.loop_bodies.insert(header, body);
        }
    }

    fn dominates(&self, a: u64, b: u64) -> bool {
        let mut current = b;
        loop {
            if current == a {
                return true;
            }
            match self.dominators.get(&current) {
                Some(&dom) if dom != current => current = dom,
                _ => return false,
            }
        }
    }

    fn compute_natural_loop(&self, cfg: &ControlFlowGraph, header: u64, tail: u64) -> Vec<u64> {
        let mut body = vec![header];
        let mut stack = vec![tail];

        while let Some(block_id) = stack.pop() {
            if !body.contains(&block_id) {
                body.push(block_id);

                if let Some(block) = cfg.blocks.get(&block_id) {
                    for &pred in &block.predecessors {
                        if !body.contains(&pred) {
                            stack.push(pred);
                        }
                    }
                }
            }
        }

        body
    }

    /// Get immediate dominator
    pub fn get_dominator(&self, block: u64) -> Option<u64> {
        self.dominators.get(&block).copied()
    }

    /// Get loop headers
    pub fn loop_headers(&self) -> &[u64] {
        &self.loop_headers
    }

    /// Get loop body
    pub fn get_loop_body(&self, header: u64) -> Option<&Vec<u64>> {
        self.loop_bodies.get(&header)
    }

    /// Get back edges
    pub fn back_edges(&self) -> &[(u64, u64)] {
        &self.back_edges
    }

    /// Check if block is reachable
    pub fn is_reachable(&self, cfg: &ControlFlowGraph, block_id: u64) -> bool {
        let mut visited = BTreeSet::new();
        let mut stack = vec![cfg.entry];

        while let Some(current) = stack.pop() {
            if current == block_id {
                return true;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if let Some(block) = cfg.blocks.get(&current) {
                for &succ in &block.successors {
                    stack.push(succ);
                }
            }
        }

        false
    }
}

impl Default for CfgAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// CFG BUILDER
// ============================================================================

/// CFG builder
pub struct CfgBuilder {
    cfg: ControlFlowGraph,
    current_block: Option<u64>,
}

impl CfgBuilder {
    /// Create new builder
    pub fn new(name: &str) -> Self {
        let mut cfg = ControlFlowGraph::new(name);
        let entry = cfg.add_block("entry");
        cfg.set_entry(entry);

        Self {
            cfg,
            current_block: Some(entry),
        }
    }

    /// Create new block
    pub fn new_block(&mut self, label: &str) -> u64 {
        self.cfg.add_block(label)
    }

    /// Switch to block
    pub fn switch_to(&mut self, block: u64) {
        self.current_block = Some(block);
    }

    /// Add instruction
    pub fn emit(&mut self, opcode: Opcode, operands: Vec<Operand>) -> u64 {
        if let Some(block) = self.current_block {
            self.cfg.add_instruction(block, opcode, operands)
        } else {
            0
        }
    }

    /// Branch
    pub fn branch(&mut self, target: u64) {
        if let Some(block) = self.current_block {
            self.cfg.set_terminator(block, Terminator::Branch(target));
        }
    }

    /// Conditional branch
    pub fn cond_branch(&mut self, cond: &str, if_true: u64, if_false: u64) {
        if let Some(block) = self.current_block {
            self.cfg.set_terminator(block, Terminator::CondBranch {
                condition: cond.into(),
                if_true,
                if_false,
            });
        }
    }

    /// Return
    pub fn ret(&mut self, value: Option<&str>) {
        if let Some(block) = self.current_block {
            self.cfg
                .set_terminator(block, Terminator::Return(value.map(String::from)));
        }
    }

    /// Build
    pub fn build(self) -> ControlFlowGraph {
        self.cfg
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfg_construction() {
        let mut builder = CfgBuilder::new("test");

        let loop_header = builder.new_block("loop.header");
        let loop_body = builder.new_block("loop.body");
        let exit = builder.new_block("exit");

        builder.emit(Opcode::Nop, vec![]);
        builder.branch(loop_header);

        builder.switch_to(loop_header);
        builder.cond_branch("cond", loop_body, exit);

        builder.switch_to(loop_body);
        builder.branch(loop_header);

        builder.switch_to(exit);
        builder.ret(None);

        let cfg = builder.build();
        assert_eq!(cfg.block_count(), 4);
    }

    #[test]
    fn test_loop_detection() {
        let mut builder = CfgBuilder::new("test");

        let header = builder.new_block("header");
        let body = builder.new_block("body");
        let exit = builder.new_block("exit");

        builder.branch(header);

        builder.switch_to(header);
        builder.cond_branch("cond", body, exit);

        builder.switch_to(body);
        builder.branch(header);

        builder.switch_to(exit);
        builder.ret(None);

        let cfg = builder.build();
        let mut analyzer = CfgAnalyzer::new();
        analyzer.analyze(&cfg);

        assert!(!analyzer.loop_headers().is_empty());
        assert!(!analyzer.back_edges().is_empty());
    }

    #[test]
    fn test_dominators() {
        let mut builder = CfgBuilder::new("test");

        let a = builder.cfg.entry();
        let b = builder.new_block("B");
        let c = builder.new_block("C");
        let d = builder.new_block("D");

        builder.cond_branch("cond", b, c);

        builder.switch_to(b);
        builder.branch(d);

        builder.switch_to(c);
        builder.branch(d);

        builder.switch_to(d);
        builder.ret(None);

        let cfg = builder.build();
        let mut analyzer = CfgAnalyzer::new();
        analyzer.analyze(&cfg);

        // A dominates all blocks
        assert!(analyzer.dominates(a, b) || analyzer.get_dominator(b) == Some(a));
    }

    #[test]
    fn test_reachability() {
        let mut builder = CfgBuilder::new("test");

        let reachable = builder.new_block("reachable");
        let _unreachable = builder.new_block("unreachable");

        builder.branch(reachable);
        builder.switch_to(reachable);
        builder.ret(None);

        let cfg = builder.build();
        let analyzer = CfgAnalyzer::default();

        assert!(analyzer.is_reachable(&cfg, reachable));
    }
}
