//! # Coop Intent Engine
//!
//! Intent-based cooperation where processes declare intentions:
//! - Intent declaration and parsing
//! - Intent matching and conflict detection
//! - Resource reservation from intents
//! - Proactive resource staging
//! - Intent fulfillment tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// INTENT TYPES
// ============================================================================

/// Intent category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentCategory {
    /// CPU-intensive computation
    Compute,
    /// Memory-intensive operation
    MemoryIntensive,
    /// IO-heavy workload
    IoHeavy,
    /// Network transfer
    NetworkTransfer,
    /// Low-latency requirement
    LowLatency,
    /// Batch processing
    Batch,
    /// Real-time processing
    RealTime,
    /// Idle/background
    Background,
}

/// Intent priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntentPriority {
    /// Best effort
    BestEffort,
    /// Normal
    Normal,
    /// Elevated
    Elevated,
    /// Critical
    Critical,
}

/// Intent state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentState {
    /// Declared but not fulfilled
    Pending,
    /// Being staged
    Staging,
    /// Resources allocated
    Fulfilled,
    /// Active (in use)
    Active,
    /// Completed
    Completed,
    /// Failed to fulfill
    Failed,
    /// Cancelled
    Cancelled,
}

// ============================================================================
// INTENT DECLARATION
// ============================================================================

/// Resource requirement in intent
#[derive(Debug, Clone)]
pub struct IntentRequirement {
    /// Resource type hash (FNV-1a)
    pub resource_hash: u64,
    /// Minimum required
    pub minimum: f64,
    /// Desired (optimal)
    pub desired: f64,
    /// Maximum useful
    pub maximum: f64,
}

impl IntentRequirement {
    pub fn new(resource_name: &str, min: f64, desired: f64, max: f64) -> Self {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in resource_name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self {
            resource_hash: hash,
            minimum: min,
            desired,
            maximum: max,
        }
    }

    /// Satisfaction ratio (0..1)
    pub fn satisfaction(&self, allocated: f64) -> f64 {
        if allocated >= self.desired {
            1.0
        } else if allocated >= self.minimum {
            (allocated - self.minimum) / (self.desired - self.minimum).max(0.001)
        } else {
            0.0
        }
    }
}

/// Intent declaration
#[derive(Debug, Clone)]
pub struct IntentDeclaration {
    /// Intent ID
    pub intent_id: u64,
    /// Declaring PID
    pub pid: u64,
    /// Category
    pub category: IntentCategory,
    /// Priority
    pub priority: IntentPriority,
    /// State
    pub state: IntentState,
    /// Requirements
    pub requirements: Vec<IntentRequirement>,
    /// Expected duration (ns)
    pub duration_ns: u64,
    /// Start time preference (ns, 0 = ASAP)
    pub start_after_ns: u64,
    /// Deadline (ns, 0 = none)
    pub deadline_ns: u64,
    /// Created at (ns)
    pub created_ns: u64,
    /// Fulfillment ratio (0..1)
    pub fulfillment: f64,
}

impl IntentDeclaration {
    pub fn new(intent_id: u64, pid: u64, category: IntentCategory, now: u64) -> Self {
        Self {
            intent_id,
            pid,
            category,
            priority: IntentPriority::Normal,
            state: IntentState::Pending,
            requirements: Vec::new(),
            duration_ns: 0,
            start_after_ns: 0,
            deadline_ns: 0,
            created_ns: now,
            fulfillment: 0.0,
        }
    }

    /// Add requirement
    pub fn add_requirement(&mut self, req: IntentRequirement) {
        self.requirements.push(req);
    }

    /// Compute fulfillment based on allocations
    pub fn compute_fulfillment(&mut self, allocations: &BTreeMap<u64, f64>) {
        if self.requirements.is_empty() {
            self.fulfillment = 1.0;
            return;
        }
        let sum: f64 = self.requirements.iter()
            .map(|r| {
                let alloc = allocations.get(&r.resource_hash).copied().unwrap_or(0.0);
                r.satisfaction(alloc)
            })
            .sum();
        self.fulfillment = sum / self.requirements.len() as f64;
    }

    /// Is urgent (has deadline approaching)
    pub fn is_urgent(&self, now: u64) -> bool {
        self.deadline_ns > 0 && now > self.deadline_ns.saturating_sub(self.duration_ns)
    }

    /// Is ready to start
    pub fn is_ready(&self, now: u64) -> bool {
        self.state == IntentState::Pending && now >= self.start_after_ns
    }
}

// ============================================================================
// CONFLICT DETECTION
// ============================================================================

/// Intent conflict
#[derive(Debug, Clone)]
pub struct IntentConflict {
    /// First intent
    pub intent_a: u64,
    /// Second intent
    pub intent_b: u64,
    /// Conflicting resource hash
    pub resource_hash: u64,
    /// Combined demand
    pub combined_demand: f64,
    /// Available supply
    pub available: f64,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Intent engine stats
#[derive(Debug, Clone, Default)]
pub struct CoopIntentStats {
    /// Total intents declared
    pub total_declared: u64,
    /// Active intents
    pub active_intents: usize,
    /// Fulfilled intents
    pub fulfilled: u64,
    /// Failed intents
    pub failed: u64,
    /// Detected conflicts
    pub conflicts: usize,
    /// Average fulfillment
    pub avg_fulfillment: f64,
}

/// Coop intent engine
pub struct CoopIntentEngine {
    /// Intents
    intents: BTreeMap<u64, IntentDeclaration>,
    /// Process -> intent IDs
    process_intents: BTreeMap<u64, Vec<u64>>,
    /// Resource availability
    available: BTreeMap<u64, f64>,
    /// Stats
    stats: CoopIntentStats,
    /// Next intent ID
    next_intent_id: u64,
}

impl CoopIntentEngine {
    pub fn new() -> Self {
        Self {
            intents: BTreeMap::new(),
            process_intents: BTreeMap::new(),
            available: BTreeMap::new(),
            stats: CoopIntentStats::default(),
            next_intent_id: 1,
        }
    }

    /// Set resource availability
    pub fn set_available(&mut self, resource_hash: u64, amount: f64) {
        self.available.insert(resource_hash, amount);
    }

    /// Declare intent
    pub fn declare(&mut self, pid: u64, category: IntentCategory, now: u64) -> u64 {
        let id = self.next_intent_id;
        self.next_intent_id += 1;
        self.intents.insert(id, IntentDeclaration::new(id, pid, category, now));
        self.process_intents.entry(pid).or_insert_with(Vec::new).push(id);
        self.stats.total_declared += 1;
        self.update_stats();
        id
    }

    /// Add requirement to intent
    pub fn add_requirement(&mut self, intent_id: u64, req: IntentRequirement) {
        if let Some(intent) = self.intents.get_mut(&intent_id) {
            intent.add_requirement(req);
        }
    }

    /// Try fulfill pending intents
    pub fn try_fulfill(&mut self) {
        let pending: Vec<u64> = self.intents.iter()
            .filter(|(_, i)| i.state == IntentState::Pending)
            .map(|(&id, _)| id)
            .collect();

        for id in pending {
            if let Some(intent) = self.intents.get_mut(&id) {
                intent.compute_fulfillment(&self.available);
                if intent.fulfillment >= 0.8 {
                    intent.state = IntentState::Fulfilled;
                    self.stats.fulfilled += 1;
                }
            }
        }
        self.update_stats();
    }

    /// Detect conflicts
    pub fn detect_conflicts(&self) -> Vec<IntentConflict> {
        let mut conflicts = Vec::new();
        let active: Vec<&IntentDeclaration> = self.intents.values()
            .filter(|i| matches!(i.state, IntentState::Pending | IntentState::Staging | IntentState::Active))
            .collect();

        // Check each resource for over-commitment
        let mut demand_per_resource: BTreeMap<u64, Vec<(u64, f64)>> = BTreeMap::new();
        for intent in &active {
            for req in &intent.requirements {
                demand_per_resource.entry(req.resource_hash)
                    .or_insert_with(Vec::new)
                    .push((intent.intent_id, req.desired));
            }
        }

        for (&resource_hash, demands) in &demand_per_resource {
            let total_demand: f64 = demands.iter().map(|(_, d)| d).sum();
            let available = self.available.get(&resource_hash).copied().unwrap_or(0.0);
            if total_demand > available && demands.len() >= 2 {
                // Pairwise conflicts
                for i in 0..demands.len().min(8) {
                    for j in (i + 1)..demands.len().min(8) {
                        conflicts.push(IntentConflict {
                            intent_a: demands[i].0,
                            intent_b: demands[j].0,
                            resource_hash,
                            combined_demand: demands[i].1 + demands[j].1,
                            available,
                        });
                    }
                }
            }
        }

        conflicts
    }

    /// Cancel intent
    pub fn cancel(&mut self, intent_id: u64) {
        if let Some(intent) = self.intents.get_mut(&intent_id) {
            intent.state = IntentState::Cancelled;
        }
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        if let Some(ids) = self.process_intents.remove(&pid) {
            for id in ids {
                self.intents.remove(&id);
            }
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.active_intents = self.intents.values()
            .filter(|i| matches!(i.state, IntentState::Pending | IntentState::Staging | IntentState::Active | IntentState::Fulfilled))
            .count();
        let active_fulfillments: Vec<f64> = self.intents.values()
            .filter(|i| i.state != IntentState::Cancelled)
            .map(|i| i.fulfillment)
            .collect();
        if !active_fulfillments.is_empty() {
            self.stats.avg_fulfillment = active_fulfillments.iter().sum::<f64>() / active_fulfillments.len() as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &CoopIntentStats {
        &self.stats
    }
}
