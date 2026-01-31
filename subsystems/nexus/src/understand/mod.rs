//! Code Understanding Domain
//!
//! This module provides the code understanding subsystem for NEXUS,
//! including lexical analysis, semantic modeling, invariant mining,
//! and control/data flow analysis.

mod ast;
mod controlflow;
mod dataflow;
mod intelligence;
mod invariant;
mod lexer;
mod semantic;
mod token;

// Re-export token types
pub use token::{SourceLoc, Span, Token, TokenId, TokenKind};

// Re-export lexer
pub use lexer::Lexer;

// Re-export AST types
pub use ast::{
    AstNode, Attribute, BinaryOp, EnumVariant, Expr, FnParam, FnSig, GenericParam, Item,
    LiteralKind, MatchArm, Mutability, NodeId, Pattern, Stmt, StructField, StructFields, TypeRef,
    UnaryOp, UseTree, Visibility, WherePredicate,
};

// Re-export semantic model
pub use semantic::{Scope, SemanticModel, Symbol, SymbolId, SymbolKind};

// Re-export invariant types
pub use invariant::{
    Invariant, InvariantConfidence, InvariantExpr, InvariantId, InvariantKind, InvariantMiner,
    InvariantOp, InvariantSource, InvariantUnaryOp, QuantifierKind,
};

// Re-export data flow types
pub use dataflow::{DataFlowAnalyzer, DataFlowFact, DataFlowResult};

// Re-export control flow types
pub use controlflow::{BasicBlock, BlockId, ControlFlowGraph, Terminator};

// Re-export intelligence
pub use intelligence::{CodeUnderstandingAnalysis, CodeUnderstandingIntelligence};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_keywords() {
        let source = "fn let mut const struct enum impl trait pub".to_string();
        let mut lexer = Lexer::new(source, 0);
        let tokens = lexer.tokenize_all();

        let keywords: Vec<_> = tokens.iter().filter(|t| t.kind.is_keyword()).collect();
        assert_eq!(keywords.len(), 9);
    }

    #[test]
    fn test_lexer_operators() {
        let source = "+ - * / % == != < > <= >= && || -> =>".to_string();
        let mut lexer = Lexer::new(source, 0);
        let tokens = lexer.tokenize_all();

        let operators: Vec<_> = tokens.iter().filter(|t| t.kind.is_operator()).collect();
        assert!(operators.len() >= 10);
    }

    #[test]
    fn test_lexer_literals() {
        let source = r#"42 3.14 "hello" 'c' true false"#.to_string();
        let mut lexer = Lexer::new(source, 0);
        let tokens = lexer.tokenize_all();

        let literals: Vec<_> = tokens.iter().filter(|t| t.kind.is_literal()).collect();
        assert_eq!(literals.len(), 6);
    }

    #[test]
    fn test_semantic_model() {
        let mut model = SemanticModel::new();

        let func_id = model.create_symbol(
            "test_func".to_string(),
            SymbolKind::Function,
            Span::unknown(),
        );

        assert!(model.get_symbol(func_id).is_some());
        assert_eq!(model.symbol_count(), 1);
    }

    #[test]
    fn test_invariant_miner() {
        let mut miner = InvariantMiner::new();

        let inv = Invariant::new(
            InvariantId::new(0),
            InvariantKind::NullInvariant,
            InvariantExpr::Var("ptr".to_string()),
            InvariantConfidence::High,
            Span::unknown(),
        );

        miner.add_invariant(inv);
        assert_eq!(miner.count(), 1);
    }

    #[test]
    fn test_cfg() {
        let mut cfg = ControlFlowGraph::new();

        let block1 = cfg.create_block();
        let block2 = cfg.create_block();

        cfg.add_edge(cfg.entry, block1);
        cfg.add_edge(block1, block2);
        cfg.add_edge(block2, cfg.exit);

        assert_eq!(cfg.block_count(), 4); // entry, exit, block1, block2
    }

    #[test]
    fn test_code_understanding() {
        let mut intel = CodeUnderstandingIntelligence::new();

        let tokens = intel.analyze_source("fn main() {}".to_string(), 0);
        assert!(!tokens.is_empty());

        let analysis = intel.analyze();
        assert_eq!(analysis.functions_analyzed, 0); // No CFG built yet
    }
}
