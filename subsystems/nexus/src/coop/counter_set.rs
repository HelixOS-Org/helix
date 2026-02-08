// SPDX-License-Identifier: GPL-2.0
//! Coop counter_set — atomic distributed counter set for cooperative metrics.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Counter type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterType {
    Monotonic,
    Gauge,
    Histogram,
    Rate,
}

/// Counter overflow policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowPolicy {
    Wrap,
    Saturate,
    Reset,
}

/// Single counter
#[derive(Debug, Clone)]
pub struct Counter {
    pub name: String,
    pub counter_type: CounterType,
    pub value: i64,
    pub min_value: i64,
    pub max_value: i64,
    pub overflow_policy: OverflowPolicy,
    pub update_count: u64,
    pub last_update: u64,
    pub created_at: u64,
    pub per_cpu: bool,
}

impl Counter {
    pub fn new(name: String, ctype: CounterType, now: u64) -> Self {
        Self {
            name, counter_type: ctype, value: 0,
            min_value: i64::MIN, max_value: i64::MAX,
            overflow_policy: OverflowPolicy::Saturate,
            update_count: 0, last_update: now, created_at: now,
            per_cpu: false,
        }
    }

    pub fn increment(&mut self, delta: i64, now: u64) {
        let new_val = self.value.saturating_add(delta);
        self.value = match self.overflow_policy {
            OverflowPolicy::Saturate => new_val.min(self.max_value).max(self.min_value),
            OverflowPolicy::Wrap => {
                if new_val > self.max_value { self.min_value }
                else if new_val < self.min_value { self.max_value }
                else { new_val }
            }
            OverflowPolicy::Reset => {
                if new_val > self.max_value || new_val < self.min_value { 0 }
                else { new_val }
            }
        };
        self.update_count += 1;
        self.last_update = now;
    }

    pub fn set(&mut self, value: i64, now: u64) {
        self.value = value.min(self.max_value).max(self.min_value);
        self.update_count += 1;
        self.last_update = now;
    }

    pub fn reset(&mut self, now: u64) {
        self.value = 0;
        self.update_count += 1;
        self.last_update = now;
    }

    pub fn idle_time(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_update)
    }
}

/// Per-CPU counter state
#[derive(Debug, Clone)]
pub struct PerCpuCounter {
    pub name: String,
    pub cpu_values: Vec<i64>,
    pub cpu_count: u32,
}

impl PerCpuCounter {
    pub fn new(name: String, cpu_count: u32) -> Self {
        Self { name, cpu_values: alloc::vec![0i64; cpu_count as usize], cpu_count }
    }

    pub fn add(&mut self, cpu: u32, delta: i64) {
        if (cpu as usize) < self.cpu_values.len() {
            self.cpu_values[cpu as usize] = self.cpu_values[cpu as usize].saturating_add(delta);
        }
    }

    pub fn total(&self) -> i64 {
        self.cpu_values.iter().sum()
    }

    pub fn max_cpu(&self) -> (u32, i64) {
        self.cpu_values.iter().enumerate()
            .max_by_key(|(_, &v)| v)
            .map(|(i, &v)| (i as u32, v))
            .unwrap_or((0, 0))
    }
}

/// Counter set — named group of counters
#[derive(Debug)]
pub struct CounterSet {
    pub name: String,
    pub counters: BTreeMap<String, Counter>,
    pub per_cpu_counters: BTreeMap<String, PerCpuCounter>,
    pub created_at: u64,
}

impl CounterSet {
    pub fn new(name: String, now: u64) -> Self {
        Self { name, counters: BTreeMap::new(), per_cpu_counters: BTreeMap::new(), created_at: now }
    }

    pub fn add_counter(&mut self, name: String, ctype: CounterType, now: u64) {
        self.counters.insert(name.clone(), Counter::new(name, ctype, now));
    }

    pub fn add_per_cpu_counter(&mut self, name: String, cpu_count: u32) {
        self.per_cpu_counters.insert(name.clone(), PerCpuCounter::new(name, cpu_count));
    }

    pub fn get(&self, name: &str) -> Option<i64> {
        self.counters.get(name).map(|c| c.value)
    }

    pub fn increment(&mut self, name: &str, delta: i64, now: u64) -> bool {
        if let Some(c) = self.counters.get_mut(name) {
            c.increment(delta, now);
            true
        } else { false }
    }

    pub fn counter_count(&self) -> usize {
        self.counters.len() + self.per_cpu_counters.len()
    }

    pub fn snapshot(&self) -> BTreeMap<String, i64> {
        let mut snap = BTreeMap::new();
        for (k, c) in &self.counters { snap.insert(k.clone(), c.value); }
        for (k, c) in &self.per_cpu_counters { snap.insert(k.clone(), c.total()); }
        snap
    }
}

/// Counter set stats
#[derive(Debug, Clone)]
pub struct CounterSetStats {
    pub total_sets: u32,
    pub total_counters: u64,
    pub total_per_cpu: u64,
    pub total_updates: u64,
}

/// Main counter set manager
pub struct CoopCounterSet {
    sets: BTreeMap<String, CounterSet>,
    total_updates: u64,
}

impl CoopCounterSet {
    pub fn new() -> Self {
        Self { sets: BTreeMap::new(), total_updates: 0 }
    }

    pub fn create_set(&mut self, name: String, now: u64) {
        self.sets.insert(name.clone(), CounterSet::new(name, now));
    }

    pub fn remove_set(&mut self, name: &str) -> bool {
        self.sets.remove(name).is_some()
    }

    pub fn add_counter(&mut self, set_name: &str, counter_name: String, ctype: CounterType, now: u64) -> bool {
        if let Some(s) = self.sets.get_mut(set_name) {
            s.add_counter(counter_name, ctype, now);
            true
        } else { false }
    }

    pub fn increment(&mut self, set_name: &str, counter_name: &str, delta: i64, now: u64) -> bool {
        self.total_updates += 1;
        if let Some(s) = self.sets.get_mut(set_name) {
            s.increment(counter_name, delta, now)
        } else { false }
    }

    pub fn get(&self, set_name: &str, counter_name: &str) -> Option<i64> {
        self.sets.get(set_name)?.get(counter_name)
    }

    pub fn snapshot_set(&self, name: &str) -> Option<BTreeMap<String, i64>> {
        self.sets.get(name).map(|s| s.snapshot())
    }

    pub fn stats(&self) -> CounterSetStats {
        CounterSetStats {
            total_sets: self.sets.len() as u32,
            total_counters: self.sets.values().map(|s| s.counters.len() as u64).sum(),
            total_per_cpu: self.sets.values().map(|s| s.per_cpu_counters.len() as u64).sum(),
            total_updates: self.total_updates,
        }
    }
}
