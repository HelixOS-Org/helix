// SPDX-License-Identifier: GPL-2.0
//! # Holistic Curiosity Engine — System-Wide Exploration & Discovery Drive
//!
//! The grand curiosity engine for the entire NEXUS kernel. While individual
//! subsystems explore within their domain, this engine coordinates curiosity
//! at the SYSTEM level — ensuring the kernel explores hardware configurations,
//! scheduling strategies, memory layouts, I/O patterns, and their complex
//! interactions holistically.
//!
//! ## Capabilities
//!
//! - **System curiosity scoring** across all optimisation dimensions
//! - **Grand exploration strategy** balancing breadth vs. depth
//! - **Curiosity budget allocation** to under-explored frontiers
//! - **Unexplored frontier detection** in the vast configuration space
//! - **Exploration efficiency tracking** — knowledge gained per unit effort
//! - **Curiosity satisfaction monitoring** — when is enough exploration enough?
//!
//! The engine that drives the kernel to explore the unknown.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_DIMENSIONS: usize = 128;
const MAX_FRONTIERS: usize = 256;
const MAX_ALLOCATIONS: usize = 64;
const MAX_EXPLORATION_LOG: usize = 2048;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const CURIOSITY_DECAY: f32 = 0.98;
const NOVELTY_BONUS: f32 = 1.25;
const DEPTH_THRESHOLD: f32 = 0.70;
const BREADTH_THRESHOLD: f32 = 0.30;
const SATISFACTION_TARGET: f32 = 0.85;
const EFFICIENCY_FLOOR: f32 = 0.01;
const EXPLORATION_BUDGET: f32 = 1.0;
const FRONTIER_REWARD: f32 = 1.5;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn xorshift_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 10000) as f32 / 10000.0
}

// ============================================================================
// TYPES
// ============================================================================

/// Exploration domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExplorationDomain {
    HardwareConfig,
    SchedulingStrategy,
    MemoryLayout,
    IoPattern,
    IpcProtocol,
    TrustModel,
    EnergyPolicy,
    CacheStrategy,
    InterruptRouting,
    LoadBalancing,
}

/// Exploration depth level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DepthLevel {
    Surface,
    Shallow,
    Moderate,
    Deep,
    Exhaustive,
}

/// A single exploration dimension with curiosity state
#[derive(Debug, Clone)]
pub struct CuriosityDimension {
    pub id: u64,
    pub domain: ExplorationDomain,
    pub name: String,
    pub curiosity_score: f32,
    pub times_explored: u64,
    pub depth_reached: DepthLevel,
    pub last_novelty: f32,
    pub knowledge_gained: f32,
    pub last_explored_tick: u64,
    pub hash: u64,
}

/// An unexplored frontier in the configuration space
#[derive(Debug, Clone)]
pub struct Frontier {
    pub id: u64,
    pub domain: ExplorationDomain,
    pub description: String,
    pub estimated_value: f32,
    pub exploration_cost: f32,
    pub distance_from_known: f32,
    pub priority: f32,
    pub discovered_tick: u64,
}

/// Budget allocation for a domain
#[derive(Debug, Clone)]
pub struct CuriosityAllocation {
    pub domain: ExplorationDomain,
    pub budget_fraction: f32,
    pub current_spend: f32,
    pub roi_ema: f32,
    pub priority_rank: u32,
}

/// Single exploration event log entry
#[derive(Debug, Clone)]
pub struct ExplorationEvent {
    pub id: u64,
    pub domain: ExplorationDomain,
    pub novelty_found: f32,
    pub knowledge_delta: f32,
    pub cost: f32,
    pub tick: u64,
}

/// Grand exploration strategy state
#[derive(Debug, Clone)]
pub struct GrandStrategy {
    pub breadth_weight: f32,
    pub depth_weight: f32,
    pub total_explored: u64,
    pub frontier_count: u64,
    pub exploration_phase: ExplorationPhase,
    pub phase_ticks_remaining: u64,
}

/// Current phase of grand exploration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExplorationPhase {
    BroadSurvey,
    FocusedDive,
    FrontierPush,
    Consolidation,
    Reassessment,
}

/// Curiosity engine statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CuriosityStats {
    pub total_explorations: u64,
    pub total_knowledge_gained: f32,
    pub avg_novelty_ema: f32,
    pub avg_efficiency_ema: f32,
    pub frontiers_discovered: u64,
    pub frontiers_explored: u64,
    pub curiosity_satisfaction: f32,
    pub budget_utilisation: f32,
    pub domains_exhausted: u64,
    pub grand_strategy_switches: u64,
    pub deepest_depth_reached: u64,
    pub last_tick: u64,
}

// ============================================================================
// HOLISTIC CURIOSITY ENGINE
// ============================================================================

/// System-wide curiosity and exploration engine
pub struct HolisticCuriosityEngine {
    dimensions: BTreeMap<u64, CuriosityDimension>,
    frontiers: Vec<Frontier>,
    allocations: BTreeMap<u64, CuriosityAllocation>,
    exploration_log: VecDeque<ExplorationEvent>,
    strategy: GrandStrategy,
    rng_state: u64,
    tick: u64,
    stats: CuriosityStats,
}

impl HolisticCuriosityEngine {
    /// Create a new holistic curiosity engine
    pub fn new(seed: u64) -> Self {
        Self {
            dimensions: BTreeMap::new(),
            frontiers: Vec::new(),
            allocations: BTreeMap::new(),
            exploration_log: VecDeque::new(),
            strategy: GrandStrategy {
                breadth_weight: 0.6,
                depth_weight: 0.4,
                total_explored: 0,
                frontier_count: 0,
                exploration_phase: ExplorationPhase::BroadSurvey,
                phase_ticks_remaining: 100,
            },
            rng_state: seed | 1,
            tick: 0,
            stats: CuriosityStats {
                total_explorations: 0,
                total_knowledge_gained: 0.0,
                avg_novelty_ema: 0.0,
                avg_efficiency_ema: 0.0,
                frontiers_discovered: 0,
                frontiers_explored: 0,
                curiosity_satisfaction: 0.0,
                budget_utilisation: 0.0,
                domains_exhausted: 0,
                grand_strategy_switches: 0,
                deepest_depth_reached: 0,
                last_tick: 0,
            },
        }
    }

    /// Register an exploration dimension
    pub fn register_dimension(&mut self, domain: ExplorationDomain, name: String) {
        let hash = fnv1a_hash(name.as_bytes());
        let id = hash;
        if self.dimensions.len() >= MAX_DIMENSIONS { return; }
        let dim = CuriosityDimension {
            id, domain, name, curiosity_score: 1.0,
            times_explored: 0, depth_reached: DepthLevel::Surface,
            last_novelty: 0.0, knowledge_gained: 0.0,
            last_explored_tick: 0, hash,
        };
        self.dimensions.insert(id, dim);
    }

    /// Compute system-wide curiosity score
    pub fn system_curiosity(&mut self) -> f32 {
        if self.dimensions.is_empty() { return 0.0; }
        let mut total_curiosity = 0.0f32;
        let mut total_weight = 0.0f32;
        for dim in self.dimensions.values() {
            let age_factor = if self.tick > dim.last_explored_tick {
                ((self.tick - dim.last_explored_tick) as f32 * 0.01).min(2.0)
            } else { 0.0 };
            let depth_bonus = match dim.depth_reached {
                DepthLevel::Surface => 1.0,
                DepthLevel::Shallow => 0.8,
                DepthLevel::Moderate => 0.5,
                DepthLevel::Deep => 0.3,
                DepthLevel::Exhaustive => 0.1,
            };
            let curiosity = dim.curiosity_score * depth_bonus + age_factor;
            total_curiosity += curiosity;
            total_weight += 1.0;
        }
        let system_score = if total_weight > 0.0 {
            total_curiosity / total_weight
        } else { 0.0 };
        system_score.min(2.0)
    }

    /// Execute the grand exploration strategy — decide what to explore next
    pub fn grand_exploration(&mut self) -> Vec<(ExplorationDomain, f32)> {
        let mut recommendations: Vec<(ExplorationDomain, f32)> = Vec::new();
        if self.strategy.phase_ticks_remaining == 0 {
            self.strategy.exploration_phase = match self.strategy.exploration_phase {
                ExplorationPhase::BroadSurvey => ExplorationPhase::FocusedDive,
                ExplorationPhase::FocusedDive => ExplorationPhase::FrontierPush,
                ExplorationPhase::FrontierPush => ExplorationPhase::Consolidation,
                ExplorationPhase::Consolidation => ExplorationPhase::Reassessment,
                ExplorationPhase::Reassessment => ExplorationPhase::BroadSurvey,
            };
            self.strategy.phase_ticks_remaining = 80 + (xorshift64(&mut self.rng_state) % 40) as u64;
            self.stats.grand_strategy_switches += 1;
        }
        match self.strategy.exploration_phase {
            ExplorationPhase::BroadSurvey => {
                for dim in self.dimensions.values() {
                    if dim.times_explored < 3 {
                        recommendations.push((dim.domain, dim.curiosity_score * NOVELTY_BONUS));
                    }
                }
            }
            ExplorationPhase::FocusedDive => {
                let mut best_domain = ExplorationDomain::HardwareConfig;
                let mut best_score = 0.0f32;
                for dim in self.dimensions.values() {
                    if dim.knowledge_gained > best_score {
                        best_score = dim.knowledge_gained;
                        best_domain = dim.domain;
                    }
                }
                recommendations.push((best_domain, best_score * 2.0));
            }
            ExplorationPhase::FrontierPush => {
                for frontier in &self.frontiers {
                    if frontier.priority > 0.5 {
                        recommendations.push((frontier.domain, frontier.estimated_value * FRONTIER_REWARD));
                    }
                }
            }
            ExplorationPhase::Consolidation => {
                for dim in self.dimensions.values() {
                    if dim.depth_reached == DepthLevel::Shallow
                        || dim.depth_reached == DepthLevel::Moderate {
                        recommendations.push((dim.domain, dim.curiosity_score * 0.8));
                    }
                }
            }
            ExplorationPhase::Reassessment => {
                let noise = xorshift_f32(&mut self.rng_state);
                for dim in self.dimensions.values() {
                    let score = dim.curiosity_score * (0.5 + noise * 0.5);
                    recommendations.push((dim.domain, score));
                }
            }
        }
        self.strategy.phase_ticks_remaining = self.strategy.phase_ticks_remaining.saturating_sub(1);
        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        recommendations.truncate(10);
        recommendations
    }

    /// Allocate curiosity budget across exploration domains
    pub fn curiosity_allocation(&mut self) -> Vec<CuriosityAllocation> {
        let domains = [
            ExplorationDomain::HardwareConfig, ExplorationDomain::SchedulingStrategy,
            ExplorationDomain::MemoryLayout, ExplorationDomain::IoPattern,
            ExplorationDomain::IpcProtocol, ExplorationDomain::TrustModel,
            ExplorationDomain::EnergyPolicy, ExplorationDomain::CacheStrategy,
        ];
        let mut scores: Vec<(ExplorationDomain, f32)> = Vec::new();
        let mut total_score = 0.0f32;
        for &domain in &domains {
            let dim_score: f32 = self.dimensions.values()
                .filter(|d| d.domain == domain)
                .map(|d| d.curiosity_score * (1.0 + 0.1 * (self.tick.saturating_sub(d.last_explored_tick)) as f32))
                .sum();
            let frontier_bonus: f32 = self.frontiers.iter()
                .filter(|f| f.domain == domain)
                .map(|f| f.estimated_value * 0.5)
                .sum();
            let combined = dim_score + frontier_bonus + 0.01;
            scores.push((domain, combined));
            total_score += combined;
        }
        let mut allocations = Vec::new();
        for (rank, (domain, score)) in scores.iter().enumerate() {
            let fraction = if total_score > 0.0 { score / total_score } else { 1.0 / domains.len() as f32 };
            let roi = self.allocations.get(&(*domain as u64))
                .map(|a| a.roi_ema).unwrap_or(0.5);
            let alloc = CuriosityAllocation {
                domain: *domain,
                budget_fraction: fraction * EXPLORATION_BUDGET,
                current_spend: 0.0,
                roi_ema: roi,
                priority_rank: rank as u32,
            };
            self.allocations.insert(*domain as u64, alloc.clone());
            allocations.push(alloc);
        }
        let used: f32 = allocations.iter().map(|a| a.budget_fraction).sum();
        self.stats.budget_utilisation = used / EXPLORATION_BUDGET;
        allocations
    }

    /// Discover unexplored frontiers in the configuration space
    pub fn unexplored_frontiers(&mut self) -> Vec<Frontier> {
        let mut new_frontiers = Vec::new();
        for dim in self.dimensions.values() {
            if dim.times_explored < 2 {
                let noise = xorshift_f32(&mut self.rng_state);
                let value = dim.curiosity_score * FRONTIER_REWARD * (0.8 + noise * 0.4);
                let cost = 1.0 / (dim.times_explored as f32 + 1.0);
                let distance = match dim.depth_reached {
                    DepthLevel::Surface => 1.0,
                    DepthLevel::Shallow => 0.7,
                    DepthLevel::Moderate => 0.4,
                    _ => 0.2,
                };
                let id = fnv1a_hash(&[dim.domain as u8, (self.tick & 0xFF) as u8, 0xFE]);
                let frontier = Frontier {
                    id, domain: dim.domain,
                    description: dim.name.clone(),
                    estimated_value: value,
                    exploration_cost: cost,
                    distance_from_known: distance,
                    priority: value / (cost + 0.001),
                    discovered_tick: self.tick,
                };
                new_frontiers.push(frontier);
            }
        }
        new_frontiers.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(core::cmp::Ordering::Equal));
        for f in &new_frontiers {
            if self.frontiers.len() < MAX_FRONTIERS {
                self.frontiers.push(f.clone());
                self.stats.frontiers_discovered += 1;
            }
        }
        self.strategy.frontier_count = self.frontiers.len() as u64;
        new_frontiers
    }

    /// Measure exploration efficiency — knowledge gained per unit cost
    pub fn exploration_efficiency(&self) -> f32 {
        if self.exploration_log.is_empty() { return 0.0; }
        let total_knowledge: f32 = self.exploration_log.iter()
            .map(|e| e.knowledge_delta).sum();
        let total_cost: f32 = self.exploration_log.iter()
            .map(|e| e.cost).sum();
        if total_cost > EFFICIENCY_FLOOR {
            total_knowledge / total_cost
        } else {
            0.0
        }
    }

    /// Compute curiosity satisfaction — how much of the curiosity has been addressed
    pub fn curiosity_satisfaction(&mut self) -> f32 {
        if self.dimensions.is_empty() { return 1.0; }
        let mut satisfied = 0u64;
        let mut total = 0u64;
        for dim in self.dimensions.values() {
            total += 1;
            let depth_sat = match dim.depth_reached {
                DepthLevel::Surface => 0.0,
                DepthLevel::Shallow => 0.25,
                DepthLevel::Moderate => 0.5,
                DepthLevel::Deep => 0.8,
                DepthLevel::Exhaustive => 1.0,
            };
            if depth_sat >= 0.5 && dim.curiosity_score < 0.3 {
                satisfied += 1;
            }
        }
        let sat = if total > 0 { satisfied as f32 / total as f32 } else { 0.0 };
        self.stats.curiosity_satisfaction = self.stats.curiosity_satisfaction
            * (1.0 - EMA_ALPHA) + sat * EMA_ALPHA;
        sat
    }

    /// Record an exploration event
    pub fn record_exploration(&mut self, domain: ExplorationDomain,
                              novelty: f32, knowledge: f32, cost: f32) {
        let id = self.stats.total_explorations;
        let event = ExplorationEvent {
            id, domain, novelty_found: novelty,
            knowledge_delta: knowledge, cost, tick: self.tick,
        };
        if self.exploration_log.len() >= MAX_EXPLORATION_LOG {
            self.exploration_log.pop_front();
        }
        self.exploration_log.push_back(event);
        if let Some(dim) = self.dimensions.values_mut()
            .find(|d| d.domain == domain) {
            dim.times_explored += 1;
            dim.last_novelty = novelty;
            dim.knowledge_gained += knowledge;
            dim.last_explored_tick = self.tick;
            dim.curiosity_score *= CURIOSITY_DECAY;
            if dim.times_explored > 20 { dim.depth_reached = DepthLevel::Exhaustive; }
            else if dim.times_explored > 12 { dim.depth_reached = DepthLevel::Deep; }
            else if dim.times_explored > 6 { dim.depth_reached = DepthLevel::Moderate; }
            else if dim.times_explored > 2 { dim.depth_reached = DepthLevel::Shallow; }
        }
        self.stats.total_explorations += 1;
        self.stats.total_knowledge_gained += knowledge;
        self.stats.avg_novelty_ema = self.stats.avg_novelty_ema
            * (1.0 - EMA_ALPHA) + novelty * EMA_ALPHA;
        let eff = if cost > 0.001 { knowledge / cost } else { 0.0 };
        self.stats.avg_efficiency_ema = self.stats.avg_efficiency_ema
            * (1.0 - EMA_ALPHA) + eff * EMA_ALPHA;
        self.strategy.total_explored += 1;
        self.stats.last_tick = self.tick;
    }

    /// Advance the engine tick
    #[inline(always)]
    pub fn tick(&mut self) {
        self.tick += 1;
    }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &CuriosityStats {
        &self.stats
    }

    /// Get the grand strategy state
    #[inline(always)]
    pub fn grand_strategy(&self) -> &GrandStrategy {
        &self.strategy
    }

    /// Get all registered dimensions
    #[inline(always)]
    pub fn dimensions(&self) -> &BTreeMap<u64, CuriosityDimension> {
        &self.dimensions
    }

    /// Get all discovered frontiers
    #[inline(always)]
    pub fn frontiers(&self) -> &[Frontier] {
        &self.frontiers
    }

    /// Get exploration log
    #[inline(always)]
    pub fn exploration_log(&self) -> &[ExplorationEvent] {
        &self.exploration_log
    }
}
