//! # Cooperative Load Shedder
//!
//! Cooperative overload protection and load shedding:
//! - Multi-level shed policies
//! - Request prioritization
//! - Graceful degradation
//! - Admission control during overload
//! - Shed history and hysteresis
//! - Cooperative backoff signaling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Shed level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShedLevel {
    None = 0,
    Light = 1,
    Moderate = 2,
    Heavy = 3,
    Critical = 4,
    Emergency = 5,
}

/// Request class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestClass {
    Critical,
    High,
    Normal,
    Low,
    Background,
    BestEffort,
}

/// Shed decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShedDecision {
    Accept,
    Delay,
    Shed,
    Redirect,
}

/// Overload signal
#[derive(Debug, Clone)]
pub struct OverloadSignal {
    pub source_id: u64,
    pub load_factor: f64,
    pub queue_depth: u32,
    pub latency_ms: f64,
    pub timestamp: u64,
}

/// Per-class shed policy
#[derive(Debug, Clone)]
pub struct ClassPolicy {
    pub class: RequestClass,
    pub min_shed_level: ShedLevel,
    pub max_queue: u32,
    pub max_latency_ms: f64,
    pub weight: f64,
}

impl ClassPolicy {
    pub fn should_shed(&self, level: ShedLevel) -> bool {
        level >= self.min_shed_level
    }
}

/// Shed history entry
#[derive(Debug, Clone)]
pub struct ShedHistoryEntry {
    pub level: ShedLevel,
    pub timestamp: u64,
    pub duration_ns: u64,
    pub requests_shed: u64,
    pub load_at_entry: f64,
}

/// Per-subsystem shed state
#[derive(Debug, Clone)]
pub struct SubsystemShedState {
    pub subsystem_id: u64,
    pub current_level: ShedLevel,
    pub load_factor: f64,
    pub queue_depth: u32,
    pub requests_accepted: u64,
    pub requests_shed: u64,
    pub requests_delayed: u64,
    pub level_entered_ts: u64,
    pub class_policies: Vec<ClassPolicy>,
    pub history: Vec<ShedHistoryEntry>,
    pub hysteresis_up: f64,
    pub hysteresis_down: f64,
}

impl SubsystemShedState {
    pub fn new(subsystem_id: u64) -> Self {
        let default_policies = alloc::vec![
            ClassPolicy { class: RequestClass::Critical, min_shed_level: ShedLevel::Emergency, max_queue: 1000, max_latency_ms: 1000.0, weight: 10.0 },
            ClassPolicy { class: RequestClass::High, min_shed_level: ShedLevel::Critical, max_queue: 500, max_latency_ms: 500.0, weight: 5.0 },
            ClassPolicy { class: RequestClass::Normal, min_shed_level: ShedLevel::Heavy, max_queue: 200, max_latency_ms: 200.0, weight: 2.0 },
            ClassPolicy { class: RequestClass::Low, min_shed_level: ShedLevel::Moderate, max_queue: 100, max_latency_ms: 100.0, weight: 1.0 },
            ClassPolicy { class: RequestClass::Background, min_shed_level: ShedLevel::Light, max_queue: 50, max_latency_ms: 50.0, weight: 0.5 },
            ClassPolicy { class: RequestClass::BestEffort, min_shed_level: ShedLevel::Light, max_queue: 10, max_latency_ms: 10.0, weight: 0.1 },
        ];
        Self {
            subsystem_id,
            current_level: ShedLevel::None,
            load_factor: 0.0,
            queue_depth: 0,
            requests_accepted: 0,
            requests_shed: 0,
            requests_delayed: 0,
            level_entered_ts: 0,
            class_policies: default_policies,
            history: Vec::new(),
            hysteresis_up: 0.8,
            hysteresis_down: 0.6,
        }
    }

    pub fn evaluate_request(&mut self, class: RequestClass) -> ShedDecision {
        if let Some(policy) = self.class_policies.iter().find(|p| p.class == class) {
            if policy.should_shed(self.current_level) {
                self.requests_shed += 1;
                ShedDecision::Shed
            } else if self.queue_depth > policy.max_queue {
                self.requests_delayed += 1;
                ShedDecision::Delay
            } else {
                self.requests_accepted += 1;
                ShedDecision::Accept
            }
        } else {
            self.requests_accepted += 1;
            ShedDecision::Accept
        }
    }

    pub fn update_load(&mut self, load: f64, queue: u32, now: u64) {
        self.load_factor = load;
        self.queue_depth = queue;

        let new_level = self.compute_level(load);
        if new_level != self.current_level {
            // Hysteresis: only change level if past threshold
            let should_change = if new_level > self.current_level {
                load >= self.hysteresis_up
            } else {
                load <= self.hysteresis_down
            };

            if should_change {
                if self.current_level != ShedLevel::None {
                    self.history.push(ShedHistoryEntry {
                        level: self.current_level,
                        timestamp: self.level_entered_ts,
                        duration_ns: now.saturating_sub(self.level_entered_ts),
                        requests_shed: self.requests_shed,
                        load_at_entry: load,
                    });
                    if self.history.len() > 64 { self.history.remove(0); }
                }
                self.current_level = new_level;
                self.level_entered_ts = now;
            }
        }
    }

    fn compute_level(&self, load: f64) -> ShedLevel {
        if load < 0.5 { ShedLevel::None }
        else if load < 0.7 { ShedLevel::Light }
        else if load < 0.85 { ShedLevel::Moderate }
        else if load < 0.95 { ShedLevel::Heavy }
        else if load < 0.99 { ShedLevel::Critical }
        else { ShedLevel::Emergency }
    }

    pub fn shed_ratio(&self) -> f64 {
        let total = self.requests_accepted + self.requests_shed + self.requests_delayed;
        if total == 0 { return 0.0; }
        self.requests_shed as f64 / total as f64
    }
}

/// Coop load shedder stats
#[derive(Debug, Clone, Default)]
pub struct CoopLoadShedderStats {
    pub total_subsystems: usize,
    pub total_accepted: u64,
    pub total_shed: u64,
    pub total_delayed: u64,
    pub max_level: u8,
    pub overloaded_count: usize,
}

/// Cooperative Load Shedder
pub struct CoopLoadShedder {
    subsystems: BTreeMap<u64, SubsystemShedState>,
    stats: CoopLoadShedderStats,
}

impl CoopLoadShedder {
    pub fn new() -> Self {
        Self {
            subsystems: BTreeMap::new(),
            stats: CoopLoadShedderStats::default(),
        }
    }

    pub fn register(&mut self, subsystem_id: u64) {
        self.subsystems.entry(subsystem_id)
            .or_insert_with(|| SubsystemShedState::new(subsystem_id));
        self.recompute();
    }

    pub fn evaluate(&mut self, subsystem_id: u64, class: RequestClass) -> ShedDecision {
        if let Some(state) = self.subsystems.get_mut(&subsystem_id) {
            state.evaluate_request(class)
        } else { ShedDecision::Accept }
    }

    pub fn update_load(&mut self, subsystem_id: u64, load: f64, queue: u32, now: u64) {
        if let Some(state) = self.subsystems.get_mut(&subsystem_id) {
            state.update_load(load, queue, now);
        }
        self.recompute();
    }

    fn recompute(&mut self) {
        self.stats.total_subsystems = self.subsystems.len();
        self.stats.total_accepted = self.subsystems.values().map(|s| s.requests_accepted).sum();
        self.stats.total_shed = self.subsystems.values().map(|s| s.requests_shed).sum();
        self.stats.total_delayed = self.subsystems.values().map(|s| s.requests_delayed).sum();
        self.stats.max_level = self.subsystems.values()
            .map(|s| s.current_level as u8)
            .max().unwrap_or(0);
        self.stats.overloaded_count = self.subsystems.values()
            .filter(|s| s.current_level >= ShedLevel::Heavy).count();
    }

    pub fn subsystem(&self, id: u64) -> Option<&SubsystemShedState> {
        self.subsystems.get(&id)
    }

    pub fn stats(&self) -> &CoopLoadShedderStats {
        &self.stats
    }
}

// ============================================================================
// Merged from load_shed_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShedPolicy {
    Priority,
    Random,
    OldestFirst,
    NewestFirst,
    LeastProgress,
    CostBased,
}

/// Load level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadLevel {
    Normal,
    Elevated,
    High,
    Critical,
    Overload,
}

/// Shed decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShedDecision {
    Accept,
    Defer,
    Shed,
    Redirect,
}

/// Request entry for shedding evaluation
#[derive(Debug, Clone)]
pub struct ShedRequest {
    pub id: u64,
    pub priority: u32,
    pub cost: u64,
    pub arrived_at: u64,
    pub progress: f64,
    pub shed: bool,
}

impl ShedRequest {
    pub fn new(id: u64, priority: u32, cost: u64, now: u64) -> Self {
        Self { id, priority, cost, arrived_at: now, progress: 0.0, shed: false }
    }

    pub fn age_ns(&self, now: u64) -> u64 { now.saturating_sub(self.arrived_at) }
}

/// Shedding tier configuration
#[derive(Debug, Clone)]
pub struct ShedTier {
    pub level: LoadLevel,
    pub threshold: f64,
    pub shed_fraction: f64,
    pub min_priority: u32,
}

/// Load shedder instance
#[derive(Debug)]
pub struct LoadShedder {
    pub id: u64,
    pub policy: ShedPolicy,
    pub load: f64,
    pub capacity: f64,
    pub tiers: Vec<ShedTier>,
    pub active_requests: Vec<ShedRequest>,
    pub total_accepted: u64,
    pub total_shed: u64,
    pub total_deferred: u64,
}

impl LoadShedder {
    pub fn new(id: u64, policy: ShedPolicy, capacity: f64) -> Self {
        let tiers = alloc::vec![
            ShedTier { level: LoadLevel::Elevated, threshold: 0.7, shed_fraction: 0.0, min_priority: 0 },
            ShedTier { level: LoadLevel::High, threshold: 0.85, shed_fraction: 0.2, min_priority: 3 },
            ShedTier { level: LoadLevel::Critical, threshold: 0.95, shed_fraction: 0.5, min_priority: 5 },
            ShedTier { level: LoadLevel::Overload, threshold: 1.0, shed_fraction: 0.8, min_priority: 8 },
        ];
        Self {
            id, policy, load: 0.0, capacity, tiers,
            active_requests: Vec::new(), total_accepted: 0, total_shed: 0, total_deferred: 0,
        }
    }

    pub fn current_level(&self) -> LoadLevel {
        let util = if self.capacity == 0.0 { 1.0 } else { self.load / self.capacity };
        if util >= 1.0 { LoadLevel::Overload }
        else if util >= 0.95 { LoadLevel::Critical }
        else if util >= 0.85 { LoadLevel::High }
        else if util >= 0.70 { LoadLevel::Elevated }
        else { LoadLevel::Normal }
    }

    pub fn evaluate(&mut self, req: &ShedRequest) -> ShedDecision {
        let level = self.current_level();
        match level {
            LoadLevel::Normal => ShedDecision::Accept,
            LoadLevel::Elevated => ShedDecision::Accept,
            LoadLevel::High => {
                if req.priority >= 5 { ShedDecision::Accept } else { ShedDecision::Defer }
            }
            LoadLevel::Critical => {
                if req.priority >= 8 { ShedDecision::Accept } else { ShedDecision::Shed }
            }
            LoadLevel::Overload => {
                if req.priority >= 10 { ShedDecision::Defer } else { ShedDecision::Shed }
            }
        }
    }

    pub fn accept(&mut self, req: ShedRequest) {
        self.load += req.cost as f64;
        self.total_accepted += 1;
        self.active_requests.push(req);
    }

    pub fn shed(&mut self) { self.total_shed += 1; }
    pub fn defer(&mut self) { self.total_deferred += 1; }

    pub fn complete_request(&mut self, id: u64) {
        if let Some(pos) = self.active_requests.iter().position(|r| r.id == id) {
            let req = self.active_requests.remove(pos);
            self.load -= req.cost as f64;
            if self.load < 0.0 { self.load = 0.0; }
        }
    }

    pub fn shed_rate(&self) -> f64 {
        let total = self.total_accepted + self.total_shed;
        if total == 0 { return 0.0; }
        self.total_shed as f64 / total as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct LoadShedV2Stats {
    pub total_shedders: u32,
    pub overloaded: u32,
    pub total_accepted: u64,
    pub total_shed: u64,
    pub total_deferred: u64,
    pub avg_shed_rate: f64,
}

/// Main load shed v2 manager
pub struct CoopLoadShedV2 {
    shedders: BTreeMap<u64, LoadShedder>,
    next_id: u64,
}

impl CoopLoadShedV2 {
    pub fn new() -> Self { Self { shedders: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, policy: ShedPolicy, capacity: f64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.shedders.insert(id, LoadShedder::new(id, policy, capacity));
        id
    }

    pub fn evaluate(&mut self, shedder_id: u64, req: &ShedRequest) -> ShedDecision {
        self.shedders.get_mut(&shedder_id).map(|s| s.evaluate(req)).unwrap_or(ShedDecision::Shed)
    }

    pub fn stats(&self) -> LoadShedV2Stats {
        let overloaded = self.shedders.values().filter(|s| s.current_level() == LoadLevel::Overload).count() as u32;
        let accepted: u64 = self.shedders.values().map(|s| s.total_accepted).sum();
        let shed: u64 = self.shedders.values().map(|s| s.total_shed).sum();
        let deferred: u64 = self.shedders.values().map(|s| s.total_deferred).sum();
        let rates: Vec<f64> = self.shedders.values().map(|s| s.shed_rate()).collect();
        let avg = if rates.is_empty() { 0.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 };
        LoadShedV2Stats {
            total_shedders: self.shedders.len() as u32, overloaded,
            total_accepted: accepted, total_shed: shed,
            total_deferred: deferred, avg_shed_rate: avg,
        }
    }
}
