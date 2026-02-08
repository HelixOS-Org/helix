//! # Holistic Scheduling Group Manager
//!
//! Scheduling group management for hierarchical scheduling:
//! - Group creation with bandwidth control
//! - Hierarchical parent/child relationships
//! - CPU bandwidth (quota/period) enforcement
//! - Group weight-based proportional sharing
//! - Burst allowance tracking
//! - Throttle/unthrottle management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Group scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupSchedPolicy {
    /// CFS proportional sharing
    Proportional,
    /// Bandwidth-limited (quota/period)
    BandwidthLimited,
    /// Priority-based
    Priority,
    /// FIFO within group
    Fifo,
}

/// Bandwidth parameters (quota/period)
#[derive(Debug, Clone)]
pub struct BandwidthParams {
    pub quota_us: i64,   // -1 = unlimited
    pub period_us: u64,
    pub burst_us: u64,
}

impl BandwidthParams {
    pub fn unlimited() -> Self {
        Self { quota_us: -1, period_us: 100_000, burst_us: 0 }
    }

    pub fn limited(quota_us: u64, period_us: u64) -> Self {
        Self { quota_us: quota_us as i64, period_us, burst_us: 0 }
    }

    pub fn is_unlimited(&self) -> bool {
        self.quota_us < 0
    }

    /// Max utilization this bandwidth allows
    pub fn max_utilization(&self) -> f64 {
        if self.quota_us < 0 { return f64::MAX; }
        if self.period_us == 0 { return 0.0; }
        self.quota_us as f64 / self.period_us as f64
    }
}

/// Runtime state for bandwidth enforcement
#[derive(Debug, Clone)]
pub struct BandwidthRuntime {
    pub remaining_quota_us: i64,
    pub accumulated_burst_us: i64,
    pub throttled: bool,
    pub throttle_count: u64,
    pub total_runtime_us: u64,
    pub periods_elapsed: u64,
}

impl BandwidthRuntime {
    pub fn new(quota_us: i64) -> Self {
        Self {
            remaining_quota_us: quota_us,
            accumulated_burst_us: 0,
            throttled: false,
            throttle_count: 0,
            total_runtime_us: 0,
            periods_elapsed: 0,
        }
    }

    /// Consume runtime. Returns true if still within budget
    pub fn consume(&mut self, us: u64) -> bool {
        self.total_runtime_us += us;
        if self.remaining_quota_us < 0 { return true; } // unlimited

        self.remaining_quota_us -= us as i64;
        if self.remaining_quota_us <= 0 {
            // Try burst
            if self.accumulated_burst_us > 0 {
                let needed = (-self.remaining_quota_us) as i64;
                let burst_use = needed.min(self.accumulated_burst_us);
                self.remaining_quota_us += burst_use;
                self.accumulated_burst_us -= burst_use;
            }
            if self.remaining_quota_us <= 0 {
                self.throttled = true;
                self.throttle_count += 1;
                return false;
            }
        }
        true
    }

    /// Refill quota for new period
    pub fn refill(&mut self, params: &BandwidthParams) {
        self.periods_elapsed += 1;
        if params.quota_us < 0 {
            self.remaining_quota_us = -1;
            self.throttled = false;
            return;
        }

        // Accumulate unused as burst
        if self.remaining_quota_us > 0 {
            self.accumulated_burst_us += self.remaining_quota_us;
            if self.accumulated_burst_us > params.burst_us as i64 {
                self.accumulated_burst_us = params.burst_us as i64;
            }
        }

        self.remaining_quota_us = params.quota_us;
        self.throttled = false;
    }
}

/// Scheduling group
#[derive(Debug, Clone)]
pub struct SchedGroup {
    pub group_id: u64,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
    pub weight: u32,
    pub policy: GroupSchedPolicy,
    pub bandwidth: BandwidthParams,
    pub runtime: BandwidthRuntime,
    pub nr_tasks: u32,
    pub nr_running: u32,
    pub cpu_set: u64, // bitmask
    pub depth: u32,
}

impl SchedGroup {
    pub fn new(group_id: u64, weight: u32) -> Self {
        Self {
            group_id,
            parent_id: None,
            children: Vec::new(),
            weight,
            policy: GroupSchedPolicy::Proportional,
            bandwidth: BandwidthParams::unlimited(),
            runtime: BandwidthRuntime::new(-1),
            nr_tasks: 0,
            nr_running: 0,
            cpu_set: u64::MAX,
            depth: 0,
        }
    }

    pub fn with_bandwidth(mut self, bw: BandwidthParams) -> Self {
        self.runtime = BandwidthRuntime::new(bw.quota_us);
        self.bandwidth = bw;
        self.policy = GroupSchedPolicy::BandwidthLimited;
        self
    }

    pub fn is_throttled(&self) -> bool {
        self.runtime.throttled
    }

    pub fn utilization(&self) -> f64 {
        if self.runtime.periods_elapsed == 0 { return 0.0; }
        let period_total = self.runtime.periods_elapsed as f64 * self.bandwidth.period_us as f64;
        if period_total < 1.0 { return 0.0; }
        self.runtime.total_runtime_us as f64 / period_total
    }
}

/// Group hierarchy stats
#[derive(Debug, Clone, Default)]
pub struct HolisticSchedGroupStats {
    pub total_groups: usize,
    pub root_groups: usize,
    pub max_depth: u32,
    pub total_tasks: u32,
    pub throttled_groups: usize,
    pub total_throttle_events: u64,
    pub avg_utilization: f64,
}

/// Holistic Scheduling Group Manager
pub struct HolisticSchedGroup {
    groups: BTreeMap<u64, SchedGroup>,
    root_groups: Vec<u64>,
    stats: HolisticSchedGroupStats,
}

impl HolisticSchedGroup {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            root_groups: Vec::new(),
            stats: HolisticSchedGroupStats::default(),
        }
    }

    /// Create a root group
    pub fn create_root(&mut self, group_id: u64, weight: u32) {
        let group = SchedGroup::new(group_id, weight);
        self.groups.insert(group_id, group);
        self.root_groups.push(group_id);
        self.recompute();
    }

    /// Create a child group
    pub fn create_child(&mut self, group_id: u64, parent_id: u64, weight: u32) -> bool {
        let parent_depth = self.groups.get(&parent_id).map(|g| g.depth).unwrap_or(0);

        let mut group = SchedGroup::new(group_id, weight);
        group.parent_id = Some(parent_id);
        group.depth = parent_depth + 1;
        self.groups.insert(group_id, group);

        if let Some(parent) = self.groups.get_mut(&parent_id) {
            parent.children.push(group_id);
            true
        } else { false }
    }

    /// Set bandwidth for a group
    pub fn set_bandwidth(&mut self, group_id: u64, bw: BandwidthParams) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.runtime = BandwidthRuntime::new(bw.quota_us);
            group.bandwidth = bw;
            group.policy = GroupSchedPolicy::BandwidthLimited;
        }
    }

    /// Set weight
    pub fn set_weight(&mut self, group_id: u64, weight: u32) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.weight = weight;
        }
    }

    /// Consume runtime for a group (and propagate to parents)
    pub fn consume_runtime(&mut self, group_id: u64, us: u64) -> bool {
        let mut current = Some(group_id);
        let mut ok = true;

        while let Some(gid) = current {
            if let Some(group) = self.groups.get_mut(&gid) {
                if !group.runtime.consume(us) {
                    ok = false;
                }
                current = group.parent_id;
            } else {
                break;
            }
        }

        self.recompute();
        ok
    }

    /// Period tick — refill all groups
    pub fn period_tick(&mut self) {
        let ids: Vec<u64> = self.groups.keys().copied().collect();
        for id in ids {
            if let Some(group) = self.groups.get_mut(&id) {
                let params = group.bandwidth.clone();
                group.runtime.refill(&params);
            }
        }
        self.recompute();
    }

    /// Update task count
    pub fn set_tasks(&mut self, group_id: u64, nr_tasks: u32, nr_running: u32) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.nr_tasks = nr_tasks;
            group.nr_running = nr_running;
        }
        self.recompute();
    }

    /// Get effective weight (considering parent weights)
    pub fn effective_weight(&self, group_id: u64) -> f64 {
        let mut weight = 1.0;
        let mut current = Some(group_id);

        while let Some(gid) = current {
            if let Some(group) = self.groups.get(&gid) {
                // Sibling total weight at this level
                let siblings = if let Some(pid) = group.parent_id {
                    self.groups.get(&pid)
                        .map(|p| p.children.iter()
                            .filter_map(|&cid| self.groups.get(&cid).map(|c| c.weight as f64))
                            .sum::<f64>())
                        .unwrap_or(1.0)
                } else {
                    self.root_groups.iter()
                        .filter_map(|&rid| self.groups.get(&rid).map(|r| r.weight as f64))
                        .sum::<f64>()
                };

                if siblings > 0.0 {
                    weight *= group.weight as f64 / siblings;
                }
                current = group.parent_id;
            } else {
                break;
            }
        }

        weight
    }

    /// Remove a group
    pub fn remove(&mut self, group_id: u64) -> bool {
        if let Some(group) = self.groups.remove(&group_id) {
            // Remove from parent's children
            if let Some(pid) = group.parent_id {
                if let Some(parent) = self.groups.get_mut(&pid) {
                    parent.children.retain(|&c| c != group_id);
                }
            }
            self.root_groups.retain(|&r| r != group_id);
            self.recompute();
            true
        } else { false }
    }

    fn recompute(&mut self) {
        let max_depth = self.groups.values().map(|g| g.depth).max().unwrap_or(0);
        let total_tasks: u32 = self.groups.values().map(|g| g.nr_tasks).sum();
        let throttled = self.groups.values().filter(|g| g.is_throttled()).count();
        let total_throttles: u64 = self.groups.values().map(|g| g.runtime.throttle_count).sum();

        let avg_util = if self.groups.is_empty() { 0.0 } else {
            self.groups.values().map(|g| g.utilization()).sum::<f64>() / self.groups.len() as f64
        };

        self.stats = HolisticSchedGroupStats {
            total_groups: self.groups.len(),
            root_groups: self.root_groups.len(),
            max_depth,
            total_tasks,
            throttled_groups: throttled,
            total_throttle_events: total_throttles,
            avg_utilization: avg_util,
        };
    }

    pub fn stats(&self) -> &HolisticSchedGroupStats {
        &self.stats
    }

    pub fn group(&self, group_id: u64) -> Option<&SchedGroup> {
        self.groups.get(&group_id)
    }
}
