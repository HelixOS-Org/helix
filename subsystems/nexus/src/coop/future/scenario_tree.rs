// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Scenario Tree
//!
//! Branching scenario trees for cooperation futures. Predicts how resource
//! sharing will evolve using game-theoretic tree analysis with process
//! strategies. Each node represents a cooperation decision point; branches
//! capture strategy alternatives for every participant.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic key hashing in no_std.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for lightweight stochastic perturbation.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Exponential moving average update.
fn ema_update(current: u64, new_sample: u64, alpha_num: u64, alpha_den: u64) -> u64 {
    let weighted_old = current.saturating_mul(alpha_den.saturating_sub(alpha_num));
    let weighted_new = new_sample.saturating_mul(alpha_num);
    weighted_old.saturating_add(weighted_new) / alpha_den.max(1)
}

/// A strategy option available to a process at a decision node.
#[derive(Clone, Debug)]
pub struct ProcessStrategy {
    pub strategy_id: u64,
    pub process_id: u64,
    pub description_hash: u64,
    pub cooperation_level: u64,
    pub resource_offer: u64,
    pub resource_demand: u64,
    pub trust_impact: i64,
}

/// A single node in the cooperation scenario tree.
#[derive(Clone, Debug)]
pub struct ScenarioNode {
    pub node_id: u64,
    pub depth: u32,
    pub parent_id: u64,
    pub decision_process: u64,
    pub strategies: Vec<ProcessStrategy>,
    pub payoff_self: i64,
    pub payoff_social: i64,
    pub probability: u64,
    pub cumulative_trust: u64,
    pub children: Vec<u64>,
}

/// Result of evaluating a scenario path through the tree.
#[derive(Clone, Debug)]
pub struct ScenarioEvaluation {
    pub path: Vec<u64>,
    pub total_payoff: i64,
    pub social_welfare: i64,
    pub fairness_index: u64,
    pub trust_trajectory: Vec<u64>,
    pub is_nash: bool,
    pub is_pareto: bool,
}

/// Complexity metrics for the scenario tree.
#[derive(Clone, Debug)]
pub struct TreeComplexity {
    pub total_nodes: u64,
    pub max_depth: u32,
    pub branching_factor_avg: u64,
    pub unique_strategies: u64,
    pub nash_equilibria_count: u64,
    pub pareto_optimal_count: u64,
}

/// Rolling statistics for the scenario tree engine.
#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct ScenarioTreeStats {
    pub trees_built: u64,
    pub nash_paths_found: u64,
    pub social_optima_found: u64,
    pub evaluations_run: u64,
    pub avg_tree_depth: u64,
    pub avg_branching: u64,
    pub price_of_anarchy_ema: u64,
}

impl ScenarioTreeStats {
    pub fn new() -> Self {
        Self {
            trees_built: 0,
            nash_paths_found: 0,
            social_optima_found: 0,
            evaluations_run: 0,
            avg_tree_depth: 0,
            avg_branching: 0,
            price_of_anarchy_ema: 1000,
        }
    }
}

/// Internal record for a built tree.
#[derive(Clone, Debug)]
struct TreeRecord {
    tree_id: u64,
    nodes: BTreeMap<u64, ScenarioNode>,
    root_id: u64,
    leaf_ids: Vec<u64>,
    process_ids: Vec<u64>,
    build_tick: u64,
}

/// Internal cache for evaluated paths.
#[derive(Clone, Debug)]
struct PathCache {
    path_hash: u64,
    evaluation: ScenarioEvaluation,
    last_access: u64,
}

/// Historical record for strategy outcomes.
#[derive(Clone, Debug)]
struct StrategyHistory {
    strategy_hash: u64,
    outcome_samples: Vec<i64>,
    ema_payoff: i64,
    usage_count: u64,
}

/// Branching scenario tree engine for cooperation futures.
pub struct CoopScenarioTree {
    trees: BTreeMap<u64, TreeRecord>,
    path_cache: BTreeMap<u64, PathCache>,
    strategy_history: BTreeMap<u64, StrategyHistory>,
    process_strategies: BTreeMap<u64, Vec<ProcessStrategy>>,
    stats: ScenarioTreeStats,
    rng_state: u64,
    current_tick: u64,
    max_depth: u32,
    max_branching: u32,
    max_cache: usize,
}

impl CoopScenarioTree {
    /// Create a new cooperation scenario tree engine.
    pub fn new(seed: u64) -> Self {
        Self {
            trees: BTreeMap::new(),
            path_cache: BTreeMap::new(),
            strategy_history: BTreeMap::new(),
            process_strategies: BTreeMap::new(),
            stats: ScenarioTreeStats::new(),
            rng_state: seed ^ 0xABCD_COOP_5CE7_0001,
            current_tick: 0,
            max_depth: 8,
            max_branching: 4,
            max_cache: 256,
        }
    }

    /// Register a strategy available to a process.
    pub fn register_strategy(
        &mut self,
        process_id: u64,
        cooperation_level: u64,
        resource_offer: u64,
        resource_demand: u64,
        trust_impact: i64,
    ) {
        let sid = fnv1a_hash(&[
            process_id.to_le_bytes().as_slice(),
            cooperation_level.to_le_bytes().as_slice(),
            resource_offer.to_le_bytes().as_slice(),
        ].concat());

        let strategy = ProcessStrategy {
            strategy_id: sid,
            process_id,
            description_hash: fnv1a_hash(&sid.to_le_bytes()),
            cooperation_level,
            resource_offer,
            resource_demand,
            trust_impact,
        };

        self.process_strategies
            .entry(process_id)
            .or_insert_with(Vec::new)
            .push(strategy);
    }

    /// Build a cooperation scenario tree for a set of participants.
    pub fn build_cooperation_tree(&mut self, participants: &[u64]) -> u64 {
        let tree_id = fnv1a_hash(
            &participants.iter().flat_map(|p| p.to_le_bytes()).collect::<Vec<u8>>(),
        ) ^ self.current_tick;

        let mut nodes: BTreeMap<u64, ScenarioNode> = BTreeMap::new();
        let root_id = fnv1a_hash(&tree_id.to_le_bytes());

        let root = ScenarioNode {
            node_id: root_id,
            depth: 0,
            parent_id: 0,
            decision_process: participants.first().copied().unwrap_or(0),
            strategies: self.get_strategies(participants.first().copied().unwrap_or(0)),
            payoff_self: 0,
            payoff_social: 0,
            probability: 1000,
            cumulative_trust: 500,
            children: Vec::new(),
        };
        nodes.insert(root_id, root);

        let mut frontier: Vec<u64> = alloc::vec![root_id];
        let mut leaf_ids: Vec<u64> = Vec::new();
        let mut next_frontier: Vec<u64> = Vec::new();

        for depth in 1..=self.max_depth {
            next_frontier.clear();
            for &parent_nid in &frontier {
                let parent = match nodes.get(&parent_nid) {
                    Some(n) => n.clone(),
                    None => continue,
                };

                let decider_idx = (depth as usize) % participants.len().max(1);
                let decider = participants.get(decider_idx).copied().unwrap_or(0);
                let strategies = self.get_strategies(decider);

                let branch_count = (strategies.len() as u32).min(self.max_branching);
                let mut child_ids: Vec<u64> = Vec::new();

                for si in 0..branch_count as usize {
                    let strat = &strategies[si % strategies.len().max(1)];
                    let child_id = fnv1a_hash(&[
                        parent_nid.to_le_bytes().as_slice(),
                        (si as u64).to_le_bytes().as_slice(),
                        depth.to_le_bytes().as_slice(),
                    ].concat());

                    let payoff_self = strat.trust_impact
                        .saturating_add(strat.resource_offer as i64)
                        .saturating_sub(strat.resource_demand as i64);
                    let payoff_social = parent.payoff_social
                        .saturating_add(strat.cooperation_level as i64);

                    let trust_delta = if strat.trust_impact > 0 {
                        parent.cumulative_trust.saturating_add(strat.trust_impact as u64)
                    } else {
                        parent.cumulative_trust.saturating_sub(strat.trust_impact.unsigned_abs())
                    };

                    let prob = parent.probability.saturating_mul(800) / 1000;

                    let child = ScenarioNode {
                        node_id: child_id,
                        depth,
                        parent_id: parent_nid,
                        decision_process: decider,
                        strategies: alloc::vec![strat.clone()],
                        payoff_self,
                        payoff_social,
                        probability: prob.max(1),
                        cumulative_trust: trust_delta.min(1000),
                        children: Vec::new(),
                    };

                    child_ids.push(child_id);
                    nodes.insert(child_id, child);

                    if depth < self.max_depth {
                        next_frontier.push(child_id);
                    } else {
                        leaf_ids.push(child_id);
                    }
                }

                if let Some(p) = nodes.get_mut(&parent_nid) {
                    p.children = child_ids;
                }
            }
            frontier = next_frontier.clone();
            if frontier.is_empty() {
                break;
            }
        }

        let record = TreeRecord {
            tree_id,
            nodes,
            root_id,
            leaf_ids,
            process_ids: participants.to_vec(),
            build_tick: self.current_tick,
        };

        self.trees.insert(tree_id, record);
        self.stats.trees_built = self.stats.trees_built.saturating_add(1);
        self.stats.avg_tree_depth = ema_update(
            self.stats.avg_tree_depth,
            self.max_depth as u64,
            200,
            1000,
        );

        tree_id
    }

    /// Find the Nash equilibrium path in a built tree.
    pub fn nash_path(&mut self, tree_id: u64) -> Option<ScenarioEvaluation> {
        let tree = self.trees.get(&tree_id)?;

        let mut best_path: Option<ScenarioEvaluation> = None;
        let mut best_nash_score: i64 = i64::MIN;

        for &leaf_id in &tree.leaf_ids {
            let path = self.trace_path(&tree.nodes, tree.root_id, leaf_id);
            let eval = self.evaluate_path_internal(&tree.nodes, &path);

            let nash_score = self.compute_nash_deviation(&tree.nodes, &path);

            if nash_score > best_nash_score {
                best_nash_score = nash_score;
                let mut scenario_eval = eval;
                scenario_eval.is_nash = nash_score >= 0;
                best_path = Some(scenario_eval);
            }
        }

        self.stats.nash_paths_found = self.stats.nash_paths_found.saturating_add(1);
        best_path
    }

    /// Find the socially optimal path in a built tree.
    pub fn socially_optimal_path(&mut self, tree_id: u64) -> Option<ScenarioEvaluation> {
        let tree = self.trees.get(&tree_id)?;

        let mut best_path: Option<ScenarioEvaluation> = None;
        let mut best_social: i64 = i64::MIN;

        for &leaf_id in &tree.leaf_ids {
            let path = self.trace_path(&tree.nodes, tree.root_id, leaf_id);
            let eval = self.evaluate_path_internal(&tree.nodes, &path);

            if eval.social_welfare > best_social {
                best_social = eval.social_welfare;
                let mut scenario_eval = eval;
                scenario_eval.is_pareto = true;
                best_path = Some(scenario_eval);
            }
        }

        self.stats.social_optima_found = self.stats.social_optima_found.saturating_add(1);
        best_path
    }

    /// Calculate the price of anarchy for a tree.
    pub fn price_of_anarchy(&mut self, tree_id: u64) -> u64 {
        let nash = self.nash_path(tree_id);
        let social = self.socially_optimal_path(tree_id);

        let nash_welfare = nash.map(|e| e.social_welfare).unwrap_or(1);
        let social_welfare = social.map(|e| e.social_welfare).unwrap_or(1);

        let poa = if nash_welfare > 0 && social_welfare > 0 {
            (social_welfare as u64).saturating_mul(1000) / (nash_welfare as u64).max(1)
        } else if nash_welfare <= 0 && social_welfare > 0 {
            2000
        } else {
            1000
        };

        self.stats.price_of_anarchy_ema = ema_update(
            self.stats.price_of_anarchy_ema,
            poa,
            150,
            1000,
        );

        poa
    }

    /// Evaluate a specific scenario path in a tree.
    pub fn scenario_evaluate(&mut self, tree_id: u64, path_nodes: &[u64]) -> Option<ScenarioEvaluation> {
        let tree = self.trees.get(&tree_id)?;
        let eval = self.evaluate_path_internal(&tree.nodes, path_nodes);

        let cache_key = fnv1a_hash(
            &path_nodes.iter().flat_map(|n| n.to_le_bytes()).collect::<Vec<u8>>(),
        );
        self.path_cache.insert(cache_key, PathCache {
            path_hash: cache_key,
            evaluation: eval.clone(),
            last_access: self.current_tick,
        });

        self.stats.evaluations_run = self.stats.evaluations_run.saturating_add(1);
        self.prune_cache();

        Some(eval)
    }

    /// Measure complexity of a built tree.
    pub fn tree_complexity(&self, tree_id: u64) -> Option<TreeComplexity> {
        let tree = self.trees.get(&tree_id)?;

        let total_nodes = tree.nodes.len() as u64;
        let max_depth = tree.nodes.values().map(|n| n.depth).max().unwrap_or(0);

        let total_children: u64 = tree.nodes.values()
            .filter(|n| !n.children.is_empty())
            .map(|n| n.children.len() as u64)
            .sum();
        let inner_count = tree.nodes.values()
            .filter(|n| !n.children.is_empty())
            .count() as u64;
        let branching_avg = if inner_count > 0 {
            total_children.saturating_mul(1000) / inner_count
        } else {
            0
        };

        let mut strat_set: LinearMap<bool, 64> = BTreeMap::new();
        for node in tree.nodes.values() {
            for s in &node.strategies {
                strat_set.insert(s.strategy_id, true);
            }
        }

        let nash_count = tree.leaf_ids.iter().filter(|&&lid| {
            let path = self.trace_path(&tree.nodes, tree.root_id, lid);
            self.compute_nash_deviation(&tree.nodes, &path) >= 0
        }).count() as u64;

        Some(TreeComplexity {
            total_nodes,
            max_depth,
            branching_factor_avg: branching_avg,
            unique_strategies: strat_set.len() as u64,
            nash_equilibria_count: nash_count,
            pareto_optimal_count: self.count_pareto(&tree.nodes, &tree.leaf_ids, tree.root_id),
        })
    }

    /// Advance the internal tick counter.
    #[inline(always)]
    pub fn tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
    }

    /// Retrieve current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &ScenarioTreeStats {
        &self.stats
    }

    // ── Private helpers ──────────────────────────────────────────────

    fn get_strategies(&self, process_id: u64) -> Vec<ProcessStrategy> {
        self.process_strategies
            .get(&process_id)
            .cloned()
            .unwrap_or_else(|| {
                let mut defaults = Vec::new();
                for level in [200u64, 500, 800] {
                    defaults.push(ProcessStrategy {
                        strategy_id: fnv1a_hash(&[
                            process_id.to_le_bytes().as_slice(),
                            level.to_le_bytes().as_slice(),
                        ].concat()),
                        process_id,
                        description_hash: fnv1a_hash(&level.to_le_bytes()),
                        cooperation_level: level,
                        resource_offer: level / 2,
                        resource_demand: 1000u64.saturating_sub(level) / 2,
                        trust_impact: (level as i64) - 500,
                    });
                }
                defaults
            })
    }

    fn trace_path(&self, nodes: &BTreeMap<u64, ScenarioNode>, root: u64, leaf: u64) -> Vec<u64> {
        let mut path = Vec::new();
        let mut current = leaf;
        let mut visited = 0u32;
        while current != 0 && visited < 64 {
            path.push(current);
            if current == root {
                break;
            }
            current = nodes.get(&current).map(|n| n.parent_id).unwrap_or(0);
            visited += 1;
        }
        path.reverse();
        path
    }

    fn evaluate_path_internal(
        &self,
        nodes: &BTreeMap<u64, ScenarioNode>,
        path: &[u64],
    ) -> ScenarioEvaluation {
        let mut total_payoff: i64 = 0;
        let mut social_welfare: i64 = 0;
        let mut trust_trajectory: Vec<u64> = Vec::new();
        let mut min_payoff: i64 = i64::MAX;
        let mut max_payoff: i64 = i64::MIN;

        for &nid in path {
            if let Some(node) = nodes.get(&nid) {
                total_payoff = total_payoff.saturating_add(node.payoff_self);
                social_welfare = social_welfare.saturating_add(node.payoff_social);
                trust_trajectory.push(node.cumulative_trust);
                if node.payoff_self < min_payoff {
                    min_payoff = node.payoff_self;
                }
                if node.payoff_self > max_payoff {
                    max_payoff = node.payoff_self;
                }
            }
        }

        let fairness = if max_payoff > min_payoff {
            let range = (max_payoff - min_payoff) as u64;
            1000u64.saturating_sub(range.min(1000))
        } else {
            1000
        };

        ScenarioEvaluation {
            path: path.to_vec(),
            total_payoff,
            social_welfare,
            fairness_index: fairness,
            trust_trajectory,
            is_nash: false,
            is_pareto: false,
        }
    }

    fn compute_nash_deviation(&self, nodes: &BTreeMap<u64, ScenarioNode>, path: &[u64]) -> i64 {
        let mut deviation_sum: i64 = 0;

        for &nid in path {
            if let Some(node) = nodes.get(&nid) {
                let current_payoff = node.payoff_self;
                let best_alt = node.strategies.iter()
                    .map(|s| s.trust_impact
                        .saturating_add(s.resource_offer as i64)
                        .saturating_sub(s.resource_demand as i64))
                    .max()
                    .unwrap_or(current_payoff);

                deviation_sum = deviation_sum.saturating_add(current_payoff - best_alt);
            }
        }

        deviation_sum
    }

    fn count_pareto(
        &self,
        nodes: &BTreeMap<u64, ScenarioNode>,
        leaf_ids: &[u64],
        root: u64,
    ) -> u64 {
        let mut evaluations: Vec<(i64, i64)> = Vec::new();
        for &lid in leaf_ids {
            let path = self.trace_path(nodes, root, lid);
            let eval = self.evaluate_path_internal(nodes, &path);
            evaluations.push((eval.total_payoff, eval.social_welfare));
        }

        let mut pareto_count: u64 = 0;
        for i in 0..evaluations.len() {
            let mut dominated = false;
            for j in 0..evaluations.len() {
                if i == j {
                    continue;
                }
                if evaluations[j].0 >= evaluations[i].0
                    && evaluations[j].1 >= evaluations[i].1
                    && (evaluations[j].0 > evaluations[i].0 || evaluations[j].1 > evaluations[i].1)
                {
                    dominated = true;
                    break;
                }
            }
            if !dominated {
                pareto_count += 1;
            }
        }

        pareto_count
    }

    fn prune_cache(&mut self) {
        while self.path_cache.len() > self.max_cache {
            let oldest = self.path_cache.iter()
                .min_by_key(|(_, v)| v.last_access)
                .map(|(&k, _)| k);
            if let Some(key) = oldest {
                self.path_cache.remove(&key);
            } else {
                break;
            }
        }
    }
}
