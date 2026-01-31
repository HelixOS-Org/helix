//! Code understanding intelligence
//!
//! This module provides the main code understanding engine.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::controlflow::ControlFlowGraph;
use super::dataflow::DataFlowAnalyzer;
use super::invariant::InvariantMiner;
use super::lexer::Lexer;
use super::semantic::{SemanticModel, SymbolId};
use super::token::Token;

/// Code understanding analysis result
#[derive(Debug)]
pub struct CodeUnderstandingAnalysis {
    /// Total tokens
    pub total_tokens: u64,
    /// Total symbols
    pub total_symbols: u64,
    /// Total invariants
    pub total_invariants: u64,
    /// Functions analyzed
    pub functions_analyzed: u64,
    /// Complexity score
    pub complexity_score: f32,
    /// Invariant coverage
    pub invariant_coverage: f32,
}

impl CodeUnderstandingAnalysis {
    /// Create new analysis
    pub fn new() -> Self {
        Self {
            total_tokens: 0,
            total_symbols: 0,
            total_invariants: 0,
            functions_analyzed: 0,
            complexity_score: 0.0,
            invariant_coverage: 0.0,
        }
    }
}

impl Default for CodeUnderstandingAnalysis {
    fn default() -> Self {
        Self::new()
    }
}

/// Code understanding intelligence engine
pub struct CodeUnderstandingIntelligence {
    /// Semantic model
    model: SemanticModel,
    /// Invariant miner
    miner: InvariantMiner,
    /// Data flow analyzer
    data_flow: DataFlowAnalyzer,
    /// Control flow graphs
    cfgs: BTreeMap<SymbolId, ControlFlowGraph>,
    /// Files analyzed
    files_analyzed: u64,
    /// Total tokens processed
    total_tokens: u64,
}

impl CodeUnderstandingIntelligence {
    /// Create new intelligence
    pub fn new() -> Self {
        Self {
            model: SemanticModel::new(),
            miner: InvariantMiner::new(),
            data_flow: DataFlowAnalyzer::new(),
            cfgs: BTreeMap::new(),
            files_analyzed: 0,
            total_tokens: 0,
        }
    }

    /// Parse and analyze source code
    pub fn analyze_source(&mut self, source: String, file_id: u32) -> Vec<Token> {
        let mut lexer = Lexer::new(source, file_id);
        let tokens = lexer.tokenize_all();
        self.files_analyzed += 1;
        self.total_tokens += tokens.len() as u64;
        tokens
    }

    /// Mine invariants from model
    pub fn mine_invariants(&mut self) {
        self.miner.mine_null_checks(&self.model);
        self.miner.mine_bounds_checks(&self.model);
        self.miner.mine_ownership(&self.model);
        self.miner.mine_loop_invariants(&self.model);
    }

    /// Build control flow graph for function
    pub fn build_cfg(&mut self, func_id: SymbolId) -> &ControlFlowGraph {
        if !self.cfgs.contains_key(&func_id) {
            let cfg = ControlFlowGraph::new();
            self.cfgs.insert(func_id, cfg);
        }
        self.cfgs.get(&func_id).unwrap()
    }

    /// Analyze data flow for function
    pub fn analyze_data_flow(&mut self, func_id: SymbolId) {
        self.data_flow.analyze_function(func_id, &self.model);
    }

    /// Get analysis summary
    pub fn analyze(&self) -> CodeUnderstandingAnalysis {
        CodeUnderstandingAnalysis {
            total_tokens: self.total_tokens,
            total_symbols: self.model.symbol_count() as u64,
            total_invariants: self.miner.count() as u64,
            functions_analyzed: self.cfgs.len() as u64,
            complexity_score: self.calculate_complexity(),
            invariant_coverage: if self.model.symbol_count() > 0 {
                self.miner.count() as f32 / self.model.symbol_count() as f32
            } else {
                0.0
            },
        }
    }

    /// Calculate complexity score
    fn calculate_complexity(&self) -> f32 {
        let mut total_complexity = 0.0;
        let mut count = 0;

        for cfg in self.cfgs.values() {
            // Cyclomatic complexity = E - N + 2P
            // where E = edges, N = nodes, P = connected components (1 for single function)
            let edges: usize = cfg.blocks.values().map(|b| b.successors.len()).sum();
            let nodes = cfg.block_count();
            let complexity = (edges as f32) - (nodes as f32) + 2.0;
            total_complexity += complexity;
            count += 1;
        }

        if count > 0 {
            total_complexity / count as f32
        } else {
            0.0
        }
    }

    /// Get semantic model
    pub fn model(&self) -> &SemanticModel {
        &self.model
    }

    /// Get semantic model mutably
    pub fn model_mut(&mut self) -> &mut SemanticModel {
        &mut self.model
    }

    /// Get invariant miner
    pub fn miner(&self) -> &InvariantMiner {
        &self.miner
    }

    /// Get invariant miner mutably
    pub fn miner_mut(&mut self) -> &mut InvariantMiner {
        &mut self.miner
    }

    /// Get data flow analyzer
    pub fn data_flow(&self) -> &DataFlowAnalyzer {
        &self.data_flow
    }

    /// Get data flow analyzer mutably
    pub fn data_flow_mut(&mut self) -> &mut DataFlowAnalyzer {
        &mut self.data_flow
    }

    /// Get control flow graphs
    pub fn cfgs(&self) -> &BTreeMap<SymbolId, ControlFlowGraph> {
        &self.cfgs
    }

    /// Get files analyzed count
    pub fn files_analyzed(&self) -> u64 {
        self.files_analyzed
    }

    /// Reset all analysis
    pub fn reset(&mut self) {
        self.model = SemanticModel::new();
        self.miner = InvariantMiner::new();
        self.data_flow = DataFlowAnalyzer::new();
        self.cfgs.clear();
        self.files_analyzed = 0;
        self.total_tokens = 0;
    }
}

impl Default for CodeUnderstandingIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
