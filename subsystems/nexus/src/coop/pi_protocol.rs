//! # Cooperative Priority Inheritance Protocol
//!
//! Priority inheritance for cooperative resource sharing:
//! - Transitive priority propagation
//! - Priority ceiling protocol
//! - Nested lock priority tracking
//! - Unbounded priority inversion prevention
//! - Priority donation chains
//! - Statistics on boosting events

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Priority protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PiProtocol {
    /// Basic priority inheritance
    Inheritance,
    /// Priority ceiling protocol
    Ceiling,
    /// Immediate priority ceiling
    ImmediateCeiling,
}

/// Boost reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoostReasonCoop {
    DirectInheritance,
    TransitiveInheritance,
    CeilingBoost,
    DonationChain,
}

/// Priority boost record
#[derive(Debug, Clone)]
pub struct PriorityBoost {
    pub task_id: u64,
    pub original_priority: i32,
    pub boosted_priority: i32,
    pub reason: BoostReasonCoop,
    pub resource_id: u64,
    pub boosted_by: u64,
    pub timestamp: u64,
}

/// Resource with priority ceiling
#[derive(Debug, Clone)]
pub struct PiResource {
    pub resource_id: u64,
    pub protocol: PiProtocol,
    pub ceiling: i32,
    pub holder: Option<u64>,
    pub holder_original_prio: i32,
    pub waiters: Vec<(u64, i32)>, // (task_id, priority)
}

impl PiResource {
    pub fn new(resource_id: u64, protocol: PiProtocol, ceiling: i32) -> Self {
        Self {
            resource_id,
            protocol,
            ceiling,
            holder: None,
            holder_original_prio: 0,
            waiters: Vec::new(),
        }
    }

    pub fn highest_waiter_priority(&self) -> Option<i32> {
        self.waiters.iter().map(|&(_, p)| p).max()
    }

    pub fn effective_ceiling(&self) -> i32 {
        let waiter_max = self.highest_waiter_priority().unwrap_or(i32::MIN);
        if waiter_max > self.ceiling { waiter_max } else { self.ceiling }
    }
}

/// Per-task priority state
#[derive(Debug, Clone)]
pub struct TaskPriorityState {
    pub task_id: u64,
    pub base_priority: i32,
    pub effective_priority: i32,
    pub held_resources: Vec<u64>,
    pub waiting_for: Option<u64>,
    pub boost_stack: Vec<PriorityBoost>,
    pub total_boosts: u64,
    pub total_boost_ns: u64,
}

impl TaskPriorityState {
    pub fn new(task_id: u64, priority: i32) -> Self {
        Self {
            task_id,
            base_priority: priority,
            effective_priority: priority,
            held_resources: Vec::new(),
            waiting_for: None,
            boost_stack: Vec::new(),
            total_boosts: 0,
            total_boost_ns: 0,
        }
    }

    pub fn is_boosted(&self) -> bool {
        self.effective_priority > self.base_priority
    }

    pub fn boost_to(&mut self, new_prio: i32, reason: BoostReasonCoop, resource_id: u64, by: u64, ts: u64) {
        if new_prio > self.effective_priority {
            self.boost_stack.push(PriorityBoost {
                task_id: self.task_id,
                original_priority: self.effective_priority,
                boosted_priority: new_prio,
                reason,
                resource_id,
                boosted_by: by,
                timestamp: ts,
            });
            self.effective_priority = new_prio;
            self.total_boosts += 1;
        }
    }

    pub fn unboost(&mut self) {
        if let Some(_boost) = self.boost_stack.pop() {
            // Recalculate from remaining boosts
            self.effective_priority = self.boost_stack.iter()
                .map(|b| b.boosted_priority)
                .max()
                .unwrap_or(self.base_priority);
        }
    }

    pub fn reset_priority(&mut self) {
        self.boost_stack.clear();
        self.effective_priority = self.base_priority;
    }
}

/// Coop PI protocol stats
#[derive(Debug, Clone, Default)]
pub struct CoopPiProtocolStats {
    pub total_resources: usize,
    pub total_tasks: usize,
    pub currently_boosted: usize,
    pub total_boosts: u64,
    pub max_chain_depth: u32,
    pub active_inversions: usize,
}

/// Cooperative Priority Inheritance Protocol
pub struct CoopPiProtocol {
    resources: BTreeMap<u64, PiResource>,
    tasks: BTreeMap<u64, TaskPriorityState>,
    stats: CoopPiProtocolStats,
}

impl CoopPiProtocol {
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            tasks: BTreeMap::new(),
            stats: CoopPiProtocolStats::default(),
        }
    }

    pub fn register_resource(&mut self, resource_id: u64, protocol: PiProtocol, ceiling: i32) {
        self.resources.entry(resource_id)
            .or_insert_with(|| PiResource::new(resource_id, protocol, ceiling));
    }

    pub fn register_task(&mut self, task_id: u64, priority: i32) {
        self.tasks.entry(task_id)
            .or_insert_with(|| TaskPriorityState::new(task_id, priority));
    }

    /// Acquire a resource — may trigger priority inheritance
    pub fn acquire(&mut self, task_id: u64, resource_id: u64, ts: u64) -> bool {
        let task_prio = if let Some(task) = self.tasks.get(&task_id) {
            task.effective_priority
        } else { return false; };

        if let Some(resource) = self.resources.get_mut(&resource_id) {
            if resource.holder.is_some() {
                // Resource busy — add to waiters and boost holder
                resource.waiters.push((task_id, task_prio));
                if let Some(task) = self.tasks.get_mut(&task_id) {
                    task.waiting_for = Some(resource_id);
                }
                // Boost the holder
                let holder_id = resource.holder.unwrap();
                self.propagate_boost(holder_id, task_prio, resource_id, task_id, ts);
                false
            } else {
                // Resource free
                resource.holder = Some(task_id);
                resource.holder_original_prio = task_prio;
                if let Some(task) = self.tasks.get_mut(&task_id) {
                    task.held_resources.push(resource_id);
                    // For ceiling protocol, boost immediately
                    if resource.protocol == PiProtocol::ImmediateCeiling {
                        task.boost_to(resource.ceiling, BoostReasonCoop::CeilingBoost, resource_id, task_id, ts);
                    }
                }
                true
            }
        } else { false }
    }

    /// Release a resource — may de-boost and wake waiters
    pub fn release(&mut self, task_id: u64, resource_id: u64) -> Option<u64> {
        let next_task = if let Some(resource) = self.resources.get_mut(&resource_id) {
            if resource.holder != Some(task_id) { return None; }
            resource.holder = None;

            // Remove from task's held list
            if let Some(task) = self.tasks.get_mut(&task_id) {
                task.held_resources.retain(|&r| r != resource_id);
                task.unboost();
            }

            // Grant to highest-priority waiter
            if !resource.waiters.is_empty() {
                resource.waiters.sort_by(|a, b| b.1.cmp(&a.1));
                let (next_id, _next_prio) = resource.waiters.remove(0);
                resource.holder = Some(next_id);
                if let Some(next) = self.tasks.get_mut(&next_id) {
                    next.waiting_for = None;
                    next.held_resources.push(resource_id);
                }
                Some(next_id)
            } else { None }
        } else { None };

        self.recompute();
        next_task
    }

    fn propagate_boost(&mut self, holder_id: u64, waiter_prio: i32, resource_id: u64, boosted_by: u64, ts: u64) {
        let mut current = holder_id;
        let mut depth = 0u32;

        // Follow the chain: if the holder is itself waiting, propagate
        loop {
            if depth > 16 { break; } // Limit chain depth
            if let Some(task) = self.tasks.get_mut(&current) {
                if waiter_prio > task.effective_priority {
                    let reason = if depth == 0 { BoostReasonCoop::DirectInheritance }
                    else { BoostReasonCoop::TransitiveInheritance };
                    task.boost_to(waiter_prio, reason, resource_id, boosted_by, ts);
                }
                if let Some(next_res) = task.waiting_for {
                    if let Some(res) = self.resources.get(&next_res) {
                        if let Some(next_holder) = res.holder {
                            current = next_holder;
                            depth += 1;
                            continue;
                        }
                    }
                }
            }
            break;
        }
    }

    fn recompute(&mut self) {
        self.stats.total_resources = self.resources.len();
        self.stats.total_tasks = self.tasks.len();
        self.stats.currently_boosted = self.tasks.values().filter(|t| t.is_boosted()).count();
        self.stats.total_boosts = self.tasks.values().map(|t| t.total_boosts).sum();
        self.stats.max_chain_depth = self.tasks.values()
            .map(|t| t.boost_stack.len() as u32)
            .max().unwrap_or(0);
        self.stats.active_inversions = self.resources.values()
            .filter(|r| {
                if let Some(holder) = r.holder {
                    if let Some(max_waiter) = r.highest_waiter_priority() {
                        if let Some(task) = self.tasks.get(&holder) {
                            return max_waiter > task.base_priority;
                        }
                    }
                }
                false
            })
            .count();
    }

    pub fn task(&self, id: u64) -> Option<&TaskPriorityState> {
        self.tasks.get(&id)
    }

    pub fn stats(&self) -> &CoopPiProtocolStats {
        &self.stats
    }
}
