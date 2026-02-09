//! # Search Strategies
//!
//! Year 3 EVOLUTION - Program search strategies
//! Intelligent exploration of the synthesis search space.

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::while_let_loop)]
#![allow(clippy::single_match)]
#![allow(clippy::derivable_impls)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::Specification;
use crate::math::F64Ext;

// ============================================================================
// SEARCH TYPES
// ============================================================================

/// Search strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStrategy {
    /// Breadth-first search
    BreadthFirst,
    /// Depth-first search
    DepthFirst,
    /// Iterative deepening
    IterativeDeepening,
    /// Best-first search
    BestFirst,
    /// A* search
    AStar,
    /// Monte Carlo tree search
    MCTS,
    /// Beam search
    Beam,
    /// Genetic algorithm
    Genetic,
    /// Simulated annealing
    SimulatedAnnealing,
}

/// Search node
#[derive(Debug, Clone)]
pub struct SearchNode {
    /// Node ID
    pub id: u64,
    /// Parent node
    pub parent: Option<u64>,
    /// Depth in search tree
    pub depth: usize,
    /// Program fragment
    pub fragment: ProgramFragment,
    /// Heuristic score
    pub h_score: f64,
    /// Cost so far
    pub g_score: f64,
    /// f = g + h (for A*)
    pub f_score: f64,
    /// Visit count (for MCTS)
    pub visits: u32,
    /// Total reward (for MCTS)
    pub total_reward: f64,
    /// Children
    pub children: Vec<u64>,
    /// Expanded
    pub expanded: bool,
}

/// Program fragment
#[derive(Debug, Clone)]
pub enum ProgramFragment {
    /// Empty (hole)
    Hole(HoleType),
    /// Variable reference
    Var(String),
    /// Constant
    Const(i128),
    /// Binary operation
    BinOp(BinOpKind, Box<ProgramFragment>, Box<ProgramFragment>),
    /// Unary operation
    UnaryOp(UnaryOpKind, Box<ProgramFragment>),
    /// Conditional
    If(
        Box<ProgramFragment>,
        Box<ProgramFragment>,
        Box<ProgramFragment>,
    ),
    /// Loop
    Loop(Box<ProgramFragment>, Box<ProgramFragment>),
    /// Function call
    Call(String, Vec<ProgramFragment>),
    /// Sequence
    Seq(Vec<ProgramFragment>),
    /// Let binding
    Let(String, Box<ProgramFragment>, Box<ProgramFragment>),
}

/// Hole type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoleType {
    Expr,
    Stmt,
    BoolExpr,
    IntExpr,
}

/// Binary operation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Unary operation kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOpKind {
    Neg,
    Not,
    Deref,
    Ref,
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Found programs
    pub programs: Vec<ProgramFragment>,
    /// Nodes explored
    pub nodes_explored: u64,
    /// Maximum depth reached
    pub max_depth: usize,
    /// Time taken (ms)
    pub time_ms: u64,
    /// Strategy used
    pub strategy: SearchStrategy,
}

/// Search frontier
pub trait Frontier {
    fn push(&mut self, node: SearchNode);
    fn pop(&mut self) -> Option<SearchNode>;
    fn is_empty(&self) -> bool;
    fn len(&self) -> usize;
}

/// FIFO frontier for BFS
#[derive(Default)]
pub struct FIFOFrontier {
    queue: VecDeque<SearchNode>,
}

impl Frontier for FIFOFrontier {
    fn push(&mut self, node: SearchNode) {
        self.queue.push_back(node);
    }

    fn pop(&mut self) -> Option<SearchNode> {
        if self.queue.is_empty() {
            None
        } else {
            self.queue.pop_front()
        }
    }

    fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    fn len(&self) -> usize {
        self.queue.len()
    }
}

/// LIFO frontier for DFS
#[derive(Default)]
pub struct LIFOFrontier {
    stack: Vec<SearchNode>,
}

impl Frontier for LIFOFrontier {
    fn push(&mut self, node: SearchNode) {
        self.stack.push(node);
    }

    fn pop(&mut self) -> Option<SearchNode> {
        self.stack.pop()
    }

    fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    fn len(&self) -> usize {
        self.stack.len()
    }
}

/// Priority frontier for best-first search
pub struct PriorityFrontier {
    heap: Vec<SearchNode>,
}

impl Default for PriorityFrontier {
    fn default() -> Self {
        Self { heap: Vec::new() }
    }
}

impl Frontier for PriorityFrontier {
    fn push(&mut self, node: SearchNode) {
        self.heap.push(node);
        self.heap.sort_by(|a, b| {
            b.f_score
                .partial_cmp(&a.f_score)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
    }

    fn pop(&mut self) -> Option<SearchNode> {
        self.heap.pop()
    }

    fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    fn len(&self) -> usize {
        self.heap.len()
    }
}

// ============================================================================
// SEARCH ENGINE
// ============================================================================

/// Program search engine
pub struct SearchEngine {
    /// Search tree
    nodes: BTreeMap<u64, SearchNode>,
    /// Root nodes
    roots: Vec<u64>,
    /// Next node ID
    next_id: AtomicU64,
    /// Configuration
    config: SearchConfig,
    /// Statistics
    stats: SearchStats,
}

/// Search configuration
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Maximum depth
    pub max_depth: usize,
    /// Maximum nodes
    pub max_nodes: usize,
    /// Beam width
    pub beam_width: usize,
    /// MCTS exploration constant
    pub mcts_c: f64,
    /// Timeout (ms)
    pub timeout_ms: u64,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_depth: 15,
            max_nodes: 100000,
            beam_width: 100,
            mcts_c: 1.414,
            timeout_ms: 10000,
        }
    }
}

/// Search statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SearchStats {
    pub nodes_created: u64,
    pub nodes_expanded: u64,
    pub nodes_pruned: u64,
    pub solutions_found: u64,
    pub max_depth_reached: usize,
}

impl SearchEngine {
    /// Create new engine
    pub fn new(config: SearchConfig) -> Self {
        Self {
            nodes: BTreeMap::new(),
            roots: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: SearchStats::default(),
        }
    }

    /// Search for programs
    pub fn search(&mut self, spec: &Specification, strategy: SearchStrategy) -> SearchResult {
        self.nodes.clear();
        self.roots.clear();
        self.stats = SearchStats::default();

        // Create root node
        let root = self.create_node(None, 0, ProgramFragment::Hole(HoleType::Expr));
        self.roots.push(root.id);
        self.nodes.insert(root.id, root);

        // Search using selected strategy
        let programs = match strategy {
            SearchStrategy::BreadthFirst => self.bfs(spec),
            SearchStrategy::DepthFirst => self.dfs(spec),
            SearchStrategy::IterativeDeepening => self.iddfs(spec),
            SearchStrategy::BestFirst => self.best_first(spec),
            SearchStrategy::AStar => self.astar(spec),
            SearchStrategy::MCTS => self.mcts(spec),
            SearchStrategy::Beam => self.beam_search(spec),
            SearchStrategy::Genetic => self.genetic(spec),
            SearchStrategy::SimulatedAnnealing => self.simulated_annealing(spec),
        };

        SearchResult {
            programs,
            nodes_explored: self.stats.nodes_expanded,
            max_depth: self.stats.max_depth_reached,
            time_ms: 0,
            strategy,
        }
    }

    fn create_node(
        &mut self,
        parent: Option<u64>,
        depth: usize,
        fragment: ProgramFragment,
    ) -> SearchNode {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.stats.nodes_created += 1;

        SearchNode {
            id,
            parent,
            depth,
            fragment,
            h_score: 0.0,
            g_score: depth as f64,
            f_score: depth as f64,
            visits: 0,
            total_reward: 0.0,
            children: Vec::new(),
            expanded: false,
        }
    }

    fn bfs(&mut self, spec: &Specification) -> Vec<ProgramFragment> {
        let mut frontier = FIFOFrontier::default();
        let mut solutions = Vec::new();

        // Initialize with root
        if let Some(&root_id) = self.roots.first() {
            if let Some(root) = self.nodes.get(&root_id).cloned() {
                frontier.push(root);
            }
        }

        while let Some(node) = frontier.pop() {
            if self.stats.nodes_expanded >= self.config.max_nodes as u64 {
                break;
            }

            self.stats.nodes_expanded += 1;
            self.stats.max_depth_reached = self.stats.max_depth_reached.max(node.depth);

            // Check if complete program
            if self.is_complete(&node.fragment) {
                if self.is_valid(spec, &node.fragment) {
                    solutions.push(node.fragment.clone());
                    self.stats.solutions_found += 1;
                }
                continue;
            }

            // Expand node
            if node.depth < self.config.max_depth {
                let children = self.expand(&node, spec);
                for child in children {
                    frontier.push(child);
                }
            }
        }

        solutions
    }

    fn dfs(&mut self, spec: &Specification) -> Vec<ProgramFragment> {
        let mut frontier = LIFOFrontier::default();
        let mut solutions = Vec::new();

        if let Some(&root_id) = self.roots.first() {
            if let Some(root) = self.nodes.get(&root_id).cloned() {
                frontier.push(root);
            }
        }

        while let Some(node) = frontier.pop() {
            if self.stats.nodes_expanded >= self.config.max_nodes as u64 {
                break;
            }

            self.stats.nodes_expanded += 1;
            self.stats.max_depth_reached = self.stats.max_depth_reached.max(node.depth);

            if self.is_complete(&node.fragment) {
                if self.is_valid(spec, &node.fragment) {
                    solutions.push(node.fragment.clone());
                    self.stats.solutions_found += 1;
                }
                continue;
            }

            if node.depth < self.config.max_depth {
                let children = self.expand(&node, spec);
                for child in children.into_iter().rev() {
                    frontier.push(child);
                }
            }
        }

        solutions
    }

    fn iddfs(&mut self, spec: &Specification) -> Vec<ProgramFragment> {
        let mut solutions = Vec::new();

        for depth_limit in 1..=self.config.max_depth {
            self.stats = SearchStats::default();

            let mut frontier = LIFOFrontier::default();

            if let Some(&root_id) = self.roots.first() {
                if let Some(root) = self.nodes.get(&root_id).cloned() {
                    frontier.push(root);
                }
            }

            while let Some(node) = frontier.pop() {
                self.stats.nodes_expanded += 1;

                if self.is_complete(&node.fragment) {
                    if self.is_valid(spec, &node.fragment) {
                        solutions.push(node.fragment.clone());
                        self.stats.solutions_found += 1;
                    }
                    continue;
                }

                if node.depth < depth_limit {
                    let children = self.expand(&node, spec);
                    for child in children.into_iter().rev() {
                        frontier.push(child);
                    }
                }
            }

            if !solutions.is_empty() {
                break;
            }
        }

        solutions
    }

    fn best_first(&mut self, spec: &Specification) -> Vec<ProgramFragment> {
        let mut frontier = PriorityFrontier::default();
        let mut solutions = Vec::new();

        if let Some(&root_id) = self.roots.first() {
            if let Some(mut root) = self.nodes.get(&root_id).cloned() {
                root.h_score = self.heuristic(spec, &root.fragment);
                root.f_score = root.h_score;
                frontier.push(root);
            }
        }

        while let Some(node) = frontier.pop() {
            if self.stats.nodes_expanded >= self.config.max_nodes as u64 {
                break;
            }

            self.stats.nodes_expanded += 1;
            self.stats.max_depth_reached = self.stats.max_depth_reached.max(node.depth);

            if self.is_complete(&node.fragment) {
                if self.is_valid(spec, &node.fragment) {
                    solutions.push(node.fragment.clone());
                    return solutions;
                }
                continue;
            }

            if node.depth < self.config.max_depth {
                let children = self.expand_with_heuristic(&node, spec);
                for child in children {
                    frontier.push(child);
                }
            }
        }

        solutions
    }

    fn astar(&mut self, spec: &Specification) -> Vec<ProgramFragment> {
        let mut frontier = PriorityFrontier::default();
        let mut solutions = Vec::new();

        if let Some(&root_id) = self.roots.first() {
            if let Some(mut root) = self.nodes.get(&root_id).cloned() {
                root.g_score = 0.0;
                root.h_score = self.heuristic(spec, &root.fragment);
                root.f_score = root.g_score + root.h_score;
                frontier.push(root);
            }
        }

        while let Some(node) = frontier.pop() {
            if self.stats.nodes_expanded >= self.config.max_nodes as u64 {
                break;
            }

            self.stats.nodes_expanded += 1;

            if self.is_complete(&node.fragment) {
                if self.is_valid(spec, &node.fragment) {
                    solutions.push(node.fragment.clone());
                    return solutions;
                }
                continue;
            }

            if node.depth < self.config.max_depth {
                let children = self.expand_astar(&node, spec);
                for child in children {
                    frontier.push(child);
                }
            }
        }

        solutions
    }

    fn mcts(&mut self, spec: &Specification) -> Vec<ProgramFragment> {
        let mut solutions = Vec::new();
        let iterations = self.config.max_nodes;

        for _ in 0..iterations {
            // Selection
            let selected = self.mcts_select();

            // Expansion
            let expansion_needed = if let Some(node) = self.nodes.get(&selected) {
                !node.expanded && node.depth < self.config.max_depth
            } else {
                false
            };

            if expansion_needed {
                let children = self.expand_mcts(selected, spec);
                let child_ids: Vec<u64> = children.iter().map(|c| c.id).collect();

                for child in children {
                    self.nodes.insert(child.id, child);
                }

                if let Some(node) = self.nodes.get_mut(&selected) {
                    node.expanded = true;
                    node.children = child_ids;
                }
            }

            // Simulation
            let reward = self.mcts_simulate(selected, spec);

            // Backpropagation
            self.mcts_backpropagate(selected, reward);

            // Check for solutions
            if let Some(node) = self.nodes.get(&selected) {
                if self.is_complete(&node.fragment) && self.is_valid(spec, &node.fragment) {
                    solutions.push(node.fragment.clone());
                }
            }
        }

        solutions
    }

    fn mcts_select(&self) -> u64 {
        let mut current = *self.roots.first().unwrap_or(&0);

        loop {
            if let Some(node) = self.nodes.get(&current) {
                if !node.expanded || node.children.is_empty() {
                    return current;
                }

                // UCB1 selection
                let parent_visits = node.visits as f64;
                let mut best_child = node.children[0];
                let mut best_ucb = f64::NEG_INFINITY;

                for &child_id in &node.children {
                    if let Some(child) = self.nodes.get(&child_id) {
                        let ucb = if child.visits == 0 {
                            f64::INFINITY
                        } else {
                            (child.total_reward / child.visits as f64)
                                + self.config.mcts_c
                                    * (parent_visits.ln() / child.visits as f64).sqrt()
                        };

                        if ucb > best_ucb {
                            best_ucb = ucb;
                            best_child = child_id;
                        }
                    }
                }

                current = best_child;
            } else {
                return current;
            }
        }
    }

    fn mcts_simulate(&self, node_id: u64, spec: &Specification) -> f64 {
        if let Some(node) = self.nodes.get(&node_id) {
            if self.is_complete(&node.fragment) {
                if self.is_valid(spec, &node.fragment) {
                    return 1.0;
                } else {
                    return 0.0;
                }
            }
            // Random simulation
            0.5
        } else {
            0.0
        }
    }

    fn mcts_backpropagate(&mut self, mut node_id: u64, reward: f64) {
        loop {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.visits += 1;
                node.total_reward += reward;

                if let Some(parent) = node.parent {
                    node_id = parent;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn beam_search(&mut self, spec: &Specification) -> Vec<ProgramFragment> {
        let mut solutions = Vec::new();
        let mut beam: Vec<SearchNode> = Vec::new();

        // Initialize beam with root
        if let Some(&root_id) = self.roots.first() {
            if let Some(root) = self.nodes.get(&root_id).cloned() {
                beam.push(root);
            }
        }

        for _ in 0..self.config.max_depth {
            let mut candidates = Vec::new();

            for node in &beam {
                if self.is_complete(&node.fragment) {
                    if self.is_valid(spec, &node.fragment) {
                        solutions.push(node.fragment.clone());
                    }
                    continue;
                }

                let children = self.expand_with_heuristic(node, spec);
                candidates.extend(children);
            }

            if candidates.is_empty() {
                break;
            }

            // Sort by score and keep top beam_width
            candidates.sort_by(|a, b| {
                a.f_score
                    .partial_cmp(&b.f_score)
                    .unwrap_or(core::cmp::Ordering::Equal)
            });
            candidates.truncate(self.config.beam_width);

            beam = candidates;
        }

        solutions
    }

    fn genetic(&mut self, spec: &Specification) -> Vec<ProgramFragment> {
        // Simplified genetic algorithm
        let population_size = self.config.beam_width;
        let generations = self.config.max_depth;
        let mut solutions = Vec::new();

        // Initialize population
        let mut population: Vec<ProgramFragment> = Vec::new();
        for _ in 0..population_size {
            population.push(self.random_program(spec, 3));
        }

        for _ in 0..generations {
            // Evaluate fitness
            let mut scored: Vec<(f64, ProgramFragment)> = population
                .iter()
                .map(|p| (self.fitness(spec, p), p.clone()))
                .collect();

            // Check for solutions
            for (score, prog) in &scored {
                if *score >= 1.0 && self.is_valid(spec, prog) {
                    solutions.push(prog.clone());
                }
            }

            if !solutions.is_empty() {
                break;
            }

            // Selection
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(core::cmp::Ordering::Equal));
            let survivors: Vec<_> = scored
                .into_iter()
                .take(population_size / 2)
                .map(|(_, p)| p)
                .collect();

            // Crossover and mutation
            population.clear();
            population.extend(survivors.clone());

            while population.len() < population_size {
                let parent1 = &survivors[0 % survivors.len()];
                let parent2 = &survivors[1 % survivors.len()];
                let child = self.crossover(parent1, parent2);
                let mutated = self.mutate(&child);
                population.push(mutated);
            }
        }

        solutions
    }

    fn simulated_annealing(&mut self, spec: &Specification) -> Vec<ProgramFragment> {
        let mut solutions = Vec::new();
        let mut current = self.random_program(spec, 5);
        let mut current_score = self.fitness(spec, &current);
        let mut temperature = 1.0;
        let cooling_rate = 0.99;
        let iterations = self.config.max_nodes;

        for _ in 0..iterations {
            let neighbor = self.mutate(&current);
            let neighbor_score = self.fitness(spec, &neighbor);

            let delta = neighbor_score - current_score;

            if delta > 0.0 || ((-delta / temperature).exp() > 0.5) {
                current = neighbor;
                current_score = neighbor_score;
            }

            if current_score >= 1.0 && self.is_valid(spec, &current) {
                solutions.push(current.clone());
                break;
            }

            temperature *= cooling_rate;
        }

        solutions
    }

    fn expand(&self, node: &SearchNode, spec: &Specification) -> Vec<SearchNode> {
        let mut children = Vec::new();

        match &node.fragment {
            ProgramFragment::Hole(HoleType::Expr) => {
                // Variables
                for input in &spec.inputs {
                    children.push(SearchNode {
                        id: self.next_id.fetch_add(1, Ordering::Relaxed),
                        parent: Some(node.id),
                        depth: node.depth + 1,
                        fragment: ProgramFragment::Var(input.name.clone()),
                        h_score: 0.0,
                        g_score: (node.depth + 1) as f64,
                        f_score: (node.depth + 1) as f64,
                        visits: 0,
                        total_reward: 0.0,
                        children: Vec::new(),
                        expanded: false,
                    });
                }

                // Constants
                for c in &[0, 1, 2, -1] {
                    children.push(SearchNode {
                        id: self.next_id.fetch_add(1, Ordering::Relaxed),
                        parent: Some(node.id),
                        depth: node.depth + 1,
                        fragment: ProgramFragment::Const(*c),
                        h_score: 0.0,
                        g_score: (node.depth + 1) as f64,
                        f_score: (node.depth + 1) as f64,
                        visits: 0,
                        total_reward: 0.0,
                        children: Vec::new(),
                        expanded: false,
                    });
                }

                // Binary operations
                for op in &[BinOpKind::Add, BinOpKind::Sub, BinOpKind::Mul] {
                    children.push(SearchNode {
                        id: self.next_id.fetch_add(1, Ordering::Relaxed),
                        parent: Some(node.id),
                        depth: node.depth + 1,
                        fragment: ProgramFragment::BinOp(
                            *op,
                            Box::new(ProgramFragment::Hole(HoleType::Expr)),
                            Box::new(ProgramFragment::Hole(HoleType::Expr)),
                        ),
                        h_score: 0.0,
                        g_score: (node.depth + 1) as f64,
                        f_score: (node.depth + 1) as f64,
                        visits: 0,
                        total_reward: 0.0,
                        children: Vec::new(),
                        expanded: false,
                    });
                }
            },
            _ => {},
        }

        children
    }

    fn expand_with_heuristic(&self, node: &SearchNode, spec: &Specification) -> Vec<SearchNode> {
        let mut children = self.expand(node, spec);
        for child in &mut children {
            child.h_score = self.heuristic(spec, &child.fragment);
            child.f_score = child.h_score;
        }
        children
    }

    fn expand_astar(&self, node: &SearchNode, spec: &Specification) -> Vec<SearchNode> {
        let mut children = self.expand(node, spec);
        for child in &mut children {
            child.g_score = node.g_score + 1.0;
            child.h_score = self.heuristic(spec, &child.fragment);
            child.f_score = child.g_score + child.h_score;
        }
        children
    }

    fn expand_mcts(&self, node_id: u64, spec: &Specification) -> Vec<SearchNode> {
        if let Some(node) = self.nodes.get(&node_id) {
            self.expand(node, spec)
        } else {
            Vec::new()
        }
    }

    fn is_complete(&self, fragment: &ProgramFragment) -> bool {
        match fragment {
            ProgramFragment::Hole(_) => false,
            ProgramFragment::Var(_) | ProgramFragment::Const(_) => true,
            ProgramFragment::BinOp(_, l, r) => self.is_complete(l) && self.is_complete(r),
            ProgramFragment::UnaryOp(_, e) => self.is_complete(e),
            ProgramFragment::If(c, t, e) => {
                self.is_complete(c) && self.is_complete(t) && self.is_complete(e)
            },
            ProgramFragment::Loop(c, b) => self.is_complete(c) && self.is_complete(b),
            ProgramFragment::Call(_, args) => args.iter().all(|a| self.is_complete(a)),
            ProgramFragment::Seq(stmts) => stmts.iter().all(|s| self.is_complete(s)),
            ProgramFragment::Let(_, v, b) => self.is_complete(v) && self.is_complete(b),
        }
    }

    fn is_valid(&self, _spec: &Specification, _fragment: &ProgramFragment) -> bool {
        // Simplified validation
        true
    }

    fn heuristic(&self, _spec: &Specification, fragment: &ProgramFragment) -> f64 {
        // Count remaining holes
        self.count_holes(fragment) as f64
    }

    fn count_holes(&self, fragment: &ProgramFragment) -> usize {
        match fragment {
            ProgramFragment::Hole(_) => 1,
            ProgramFragment::Var(_) | ProgramFragment::Const(_) => 0,
            ProgramFragment::BinOp(_, l, r) => self.count_holes(l) + self.count_holes(r),
            ProgramFragment::UnaryOp(_, e) => self.count_holes(e),
            ProgramFragment::If(c, t, e) => {
                self.count_holes(c) + self.count_holes(t) + self.count_holes(e)
            },
            ProgramFragment::Loop(c, b) => self.count_holes(c) + self.count_holes(b),
            ProgramFragment::Call(_, args) => args.iter().map(|a| self.count_holes(a)).sum(),
            ProgramFragment::Seq(stmts) => stmts.iter().map(|s| self.count_holes(s)).sum(),
            ProgramFragment::Let(_, v, b) => self.count_holes(v) + self.count_holes(b),
        }
    }

    fn fitness(&self, spec: &Specification, fragment: &ProgramFragment) -> f64 {
        if !self.is_complete(fragment) {
            return 0.0;
        }

        // Simplified fitness
        if self.is_valid(spec, fragment) {
            1.0
        } else {
            0.5
        }
    }

    fn random_program(&self, spec: &Specification, max_depth: usize) -> ProgramFragment {
        if max_depth == 0 {
            // Terminal
            if !spec.inputs.is_empty() {
                ProgramFragment::Var(spec.inputs[0].name.clone())
            } else {
                ProgramFragment::Const(0)
            }
        } else {
            // Random choice
            let op = BinOpKind::Add;
            ProgramFragment::BinOp(
                op,
                Box::new(self.random_program(spec, max_depth - 1)),
                Box::new(self.random_program(spec, max_depth - 1)),
            )
        }
    }

    fn crossover(&self, p1: &ProgramFragment, p2: &ProgramFragment) -> ProgramFragment {
        // Simplified crossover
        match (p1, p2) {
            (ProgramFragment::BinOp(op, l, _), ProgramFragment::BinOp(_, _, r)) => {
                ProgramFragment::BinOp(*op, l.clone(), r.clone())
            },
            _ => p1.clone(),
        }
    }

    fn mutate(&self, program: &ProgramFragment) -> ProgramFragment {
        // Simplified mutation
        match program {
            ProgramFragment::BinOp(BinOpKind::Add, l, r) => {
                ProgramFragment::BinOp(BinOpKind::Sub, l.clone(), r.clone())
            },
            ProgramFragment::Const(n) => ProgramFragment::Const(n + 1),
            _ => program.clone(),
        }
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &SearchStats {
        &self.stats
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new(SearchConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::super::PerformanceSpec;
    use super::*;

    fn test_spec() -> Specification {
        Specification {
            id: 1,
            name: "add".into(),
            inputs: vec![
                super::super::Parameter {
                    name: "a".into(),
                    typ: super::super::TypeSpec::I64,
                    constraints: vec![],
                },
                super::super::Parameter {
                    name: "b".into(),
                    typ: super::super::TypeSpec::I64,
                    constraints: vec![],
                },
            ],
            output: super::super::TypeSpec::I64,
            preconditions: vec![],
            postconditions: vec![],
            invariants: vec![],
            performance: PerformanceSpec {
                max_cycles: None,
                max_memory: None,
                time_complexity: None,
                space_complexity: None,
                inline: false,
                no_alloc: false,
            },
        }
    }

    #[test]
    fn test_bfs() {
        let mut engine = SearchEngine::default();
        let result = engine.search(&test_spec(), SearchStrategy::BreadthFirst);
        assert!(result.nodes_explored > 0);
    }

    #[test]
    fn test_dfs() {
        let mut engine = SearchEngine::default();
        let result = engine.search(&test_spec(), SearchStrategy::DepthFirst);
        assert!(result.nodes_explored > 0);
    }

    #[test]
    fn test_beam_search() {
        let mut engine = SearchEngine::default();
        let result = engine.search(&test_spec(), SearchStrategy::Beam);
        assert!(result.nodes_explored > 0);
    }
}
