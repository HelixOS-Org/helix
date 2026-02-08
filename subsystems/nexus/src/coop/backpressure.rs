//! # Coop Backpressure Protocol
//!
//! Cooperative backpressure signaling between kernel and applications:
//! - Multi-level pressure propagation
//! - Flow control negotiation
//! - Adaptive drain rate
//! - Pressure gradient routing
//! - Cascading backpressure chains

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// PRESSURE TYPES
// ============================================================================

/// Backpressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BackpressureLevel {
    /// No pressure
    None,
    /// Light pressure (advisory)
    Light,
    /// Moderate (should slow down)
    Moderate,
    /// Heavy (must slow down)
    Heavy,
    /// Critical (stop immediately)
    Critical,
}

/// Pressure source
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureSource {
    /// Memory subsystem
    Memory,
    /// CPU scheduler
    Cpu,
    /// IO subsystem
    Io,
    /// Network stack
    Network,
    /// IPC channels
    Ipc,
    /// Storage
    Storage,
}

/// Backpressure signal
#[derive(Debug, Clone)]
pub struct BackpressureSignalMsg {
    /// Source of pressure
    pub source: PressureSource,
    /// Current level
    pub level: BackpressureLevel,
    /// Target PID (0 = global)
    pub target_pid: u64,
    /// Recommended action
    pub action: PressureAction,
    /// Timestamp
    pub timestamp_ns: u64,
    /// Duration hint (ns)
    pub duration_hint_ns: u64,
}

/// Pressure action recommendation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureAction {
    /// Continue normally
    Continue,
    /// Reduce allocation rate
    ReduceAlloc,
    /// Release cached resources
    ReleaseCache,
    /// Throttle IO
    ThrottleIo,
    /// Pause non-critical work
    PauseNonCritical,
    /// Emergency shed load
    ShedLoad,
}

// ============================================================================
// FLOW CONTROL
// ============================================================================

/// Flow control state for a process
#[derive(Debug, Clone)]
pub struct FlowControlState {
    /// PID
    pub pid: u64,
    /// Current drain rate (ops/sec)
    pub drain_rate: f64,
    /// Target drain rate
    pub target_rate: f64,
    /// Max drain rate
    pub max_rate: f64,
    /// Queue depth
    pub queue_depth: u64,
    /// Queue capacity
    pub queue_capacity: u64,
    /// Accumulated pressure
    pub pressure_accumulator: f64,
    /// Last update (ns)
    pub last_update_ns: u64,
}

impl FlowControlState {
    pub fn new(pid: u64, max_rate: f64) -> Self {
        Self {
            pid,
            drain_rate: max_rate,
            target_rate: max_rate,
            max_rate,
            queue_depth: 0,
            queue_capacity: 1024,
            pressure_accumulator: 0.0,
            last_update_ns: 0,
        }
    }

    /// Apply pressure signal
    pub fn apply_pressure(&mut self, level: BackpressureLevel, now: u64) {
        let factor = match level {
            BackpressureLevel::None => 1.0,
            BackpressureLevel::Light => 0.8,
            BackpressureLevel::Moderate => 0.5,
            BackpressureLevel::Heavy => 0.2,
            BackpressureLevel::Critical => 0.05,
        };
        self.target_rate = self.max_rate * factor;
        // Exponential smoothing toward target
        self.drain_rate = 0.7 * self.drain_rate + 0.3 * self.target_rate;
        self.last_update_ns = now;
    }

    /// Relieve pressure
    pub fn relieve(&mut self, now: u64) {
        // AIMD recovery
        self.target_rate = (self.target_rate * 1.1).min(self.max_rate);
        self.drain_rate = 0.7 * self.drain_rate + 0.3 * self.target_rate;
        self.last_update_ns = now;
    }

    /// Queue utilization
    pub fn queue_utilization(&self) -> f64 {
        if self.queue_capacity == 0 {
            return 0.0;
        }
        self.queue_depth as f64 / self.queue_capacity as f64
    }

    /// Is throttled
    pub fn is_throttled(&self) -> bool {
        self.drain_rate < self.max_rate * 0.95
    }

    /// Enqueue
    pub fn try_enqueue(&mut self) -> bool {
        if self.queue_depth >= self.queue_capacity {
            return false;
        }
        self.queue_depth += 1;
        true
    }

    /// Dequeue
    pub fn dequeue(&mut self) {
        if self.queue_depth > 0 {
            self.queue_depth -= 1;
        }
    }
}

// ============================================================================
// PRESSURE CHAIN
// ============================================================================

/// Pressure propagation chain
#[derive(Debug, Clone)]
pub struct PressureChain {
    /// Chain ID
    pub chain_id: u64,
    /// Source PID
    pub source_pid: u64,
    /// Target PIDs in chain order
    pub targets: Vec<u64>,
    /// Attenuation per hop (0..1)
    pub attenuation: f64,
    /// Current active level
    pub active_level: BackpressureLevel,
}

impl PressureChain {
    pub fn new(chain_id: u64, source_pid: u64, attenuation: f64) -> Self {
        Self {
            chain_id,
            source_pid,
            targets: Vec::new(),
            attenuation: if attenuation > 0.0 && attenuation < 1.0 { attenuation } else { 0.8 },
            active_level: BackpressureLevel::None,
        }
    }

    /// Add target to chain
    pub fn add_target(&mut self, pid: u64) {
        if !self.targets.contains(&pid) {
            self.targets.push(pid);
        }
    }

    /// Propagate pressure down chain
    pub fn propagate(&self, initial_level: BackpressureLevel) -> Vec<(u64, BackpressureLevel)> {
        let mut result = Vec::new();
        let mut current_pressure = initial_level as u8;

        for &pid in &self.targets {
            if current_pressure == 0 {
                break;
            }
            let attenuated = (current_pressure as f64 * self.attenuation) as u8;
            let level = match attenuated {
                0 => BackpressureLevel::None,
                1 => BackpressureLevel::Light,
                2 => BackpressureLevel::Moderate,
                3 => BackpressureLevel::Heavy,
                _ => BackpressureLevel::Critical,
            };
            result.push((pid, level));
            current_pressure = attenuated;
        }
        result
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Backpressure protocol stats
#[derive(Debug, Clone, Default)]
pub struct CoopBackpressureStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Active pressure chains
    pub active_chains: usize,
    /// Currently throttled
    pub throttled_processes: usize,
    /// Total signals sent
    pub total_signals: u64,
}

/// Coop backpressure protocol engine
pub struct CoopBackpressureProtocol {
    /// Flow control per process
    flows: BTreeMap<u64, FlowControlState>,
    /// Pressure chains
    chains: BTreeMap<u64, PressureChain>,
    /// Signal history (bounded)
    signals: Vec<BackpressureSignalMsg>,
    /// Stats
    stats: CoopBackpressureStats,
    /// Next chain ID
    next_chain_id: u64,
}

impl CoopBackpressureProtocol {
    pub fn new() -> Self {
        Self {
            flows: BTreeMap::new(),
            chains: BTreeMap::new(),
            signals: Vec::new(),
            stats: CoopBackpressureStats::default(),
            next_chain_id: 1,
        }
    }

    /// Register process
    pub fn register(&mut self, pid: u64, max_rate: f64) {
        self.flows.insert(pid, FlowControlState::new(pid, max_rate));
    }

    /// Send pressure signal
    pub fn send_signal(&mut self, source: PressureSource, level: BackpressureLevel, target_pid: u64, now: u64) {
        if let Some(flow) = self.flows.get_mut(&target_pid) {
            flow.apply_pressure(level, now);
        }

        let signal = BackpressureSignalMsg {
            source,
            level,
            target_pid,
            action: match level {
                BackpressureLevel::None => PressureAction::Continue,
                BackpressureLevel::Light => PressureAction::ReduceAlloc,
                BackpressureLevel::Moderate => PressureAction::ReleaseCache,
                BackpressureLevel::Heavy => PressureAction::ThrottleIo,
                BackpressureLevel::Critical => PressureAction::ShedLoad,
            },
            timestamp_ns: now,
            duration_hint_ns: 1_000_000_000,
        };

        if self.signals.len() >= 1024 {
            self.signals.remove(0);
        }
        self.signals.push(signal);
        self.stats.total_signals += 1;
        self.update_stats();
    }

    /// Create pressure chain
    pub fn create_chain(&mut self, source_pid: u64, attenuation: f64) -> u64 {
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        self.chains.insert(id, PressureChain::new(id, source_pid, attenuation));
        id
    }

    /// Add target to chain
    pub fn add_chain_target(&mut self, chain_id: u64, target_pid: u64) {
        if let Some(chain) = self.chains.get_mut(&chain_id) {
            chain.add_target(target_pid);
        }
    }

    /// Propagate pressure through chain
    pub fn propagate_chain(&mut self, chain_id: u64, level: BackpressureLevel, now: u64) {
        let propagated = if let Some(chain) = self.chains.get(&chain_id) {
            chain.propagate(level)
        } else {
            return;
        };

        for (pid, lvl) in propagated {
            if let Some(flow) = self.flows.get_mut(&pid) {
                flow.apply_pressure(lvl, now);
            }
        }
    }

    /// Relieve process
    pub fn relieve(&mut self, pid: u64, now: u64) {
        if let Some(flow) = self.flows.get_mut(&pid) {
            flow.relieve(now);
        }
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.flows.remove(&pid);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.flows.len();
        self.stats.active_chains = self.chains.len();
        self.stats.throttled_processes = self.flows.values()
            .filter(|f| f.is_throttled())
            .count();
    }

    /// Stats
    pub fn stats(&self) -> &CoopBackpressureStats {
        &self.stats
    }
}

// ============================================================================
// Merged from backpressure_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpStrategy {
    DropTail,
    DropHead,
    Random,
    Ecn,
    Pause,
    AdaptiveRate,
}

/// Backpressure signal level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

/// Flow control state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowState {
    Normal,
    SlowDown,
    Paused,
    Dropping,
    Recovering,
}

/// Backpressure source
#[derive(Debug, Clone)]
pub struct BpSource {
    pub id: u64,
    pub name_hash: u64,
    pub strategy: BpStrategy,
    pub state: FlowState,
    pub level: BpLevel,
    pub queue_depth: u64,
    pub queue_capacity: u64,
    pub items_accepted: u64,
    pub items_dropped: u64,
    pub items_paused: u64,
    pub rate_limit: f64,
    pub current_rate: f64,
    pub last_signal_at: u64,
}

impl BpSource {
    pub fn new(id: u64, capacity: u64, strategy: BpStrategy) -> Self {
        Self {
            id, name_hash: id, strategy, state: FlowState::Normal,
            level: BpLevel::None, queue_depth: 0, queue_capacity: capacity,
            items_accepted: 0, items_dropped: 0, items_paused: 0,
            rate_limit: f64::MAX, current_rate: 0.0, last_signal_at: 0,
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.queue_capacity == 0 { return 0.0; }
        self.queue_depth as f64 / self.queue_capacity as f64
    }

    pub fn update_level(&mut self) {
        let util = self.utilization();
        self.level = if util > 0.95 { BpLevel::Critical }
            else if util > 0.80 { BpLevel::High }
            else if util > 0.60 { BpLevel::Medium }
            else if util > 0.40 { BpLevel::Low }
            else { BpLevel::None };
    }

    pub fn try_accept(&mut self, now: u64) -> bool {
        self.update_level();
        match self.level {
            BpLevel::Critical => {
                self.items_dropped += 1;
                self.state = FlowState::Dropping;
                false
            }
            BpLevel::High => {
                if self.strategy == BpStrategy::Pause {
                    self.items_paused += 1;
                    self.state = FlowState::Paused;
                    self.last_signal_at = now;
                    false
                } else {
                    self.queue_depth += 1;
                    self.items_accepted += 1;
                    self.state = FlowState::SlowDown;
                    true
                }
            }
            _ => {
                self.queue_depth += 1;
                self.items_accepted += 1;
                self.state = FlowState::Normal;
                true
            }
        }
    }

    pub fn dequeue(&mut self, n: u64) {
        self.queue_depth = self.queue_depth.saturating_sub(n);
        self.update_level();
        if self.level == BpLevel::None { self.state = FlowState::Normal; }
    }

    pub fn drop_rate(&self) -> f64 {
        let total = self.items_accepted + self.items_dropped;
        if total == 0 { return 0.0; }
        self.items_dropped as f64 / total as f64
    }
}

/// Backpressure event
#[derive(Debug, Clone)]
pub struct BpEvent {
    pub source_id: u64,
    pub level: BpLevel,
    pub utilization: f64,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct BackpressureV2Stats {
    pub total_sources: u32,
    pub active_backpressure: u32,
    pub total_accepted: u64,
    pub total_dropped: u64,
    pub total_paused: u64,
    pub avg_utilization: f64,
}

/// Main backpressure v2 manager
pub struct CoopBackpressureV2 {
    sources: BTreeMap<u64, BpSource>,
    events: Vec<BpEvent>,
    next_id: u64,
    max_events: usize,
}

impl CoopBackpressureV2 {
    pub fn new() -> Self {
        Self { sources: BTreeMap::new(), events: Vec::new(), next_id: 1, max_events: 4096 }
    }

    pub fn register(&mut self, capacity: u64, strategy: BpStrategy) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.sources.insert(id, BpSource::new(id, capacity, strategy));
        id
    }

    pub fn try_accept(&mut self, id: u64, now: u64) -> bool {
        self.sources.get_mut(&id).map(|s| s.try_accept(now)).unwrap_or(false)
    }

    pub fn dequeue(&mut self, id: u64, n: u64) {
        if let Some(s) = self.sources.get_mut(&id) { s.dequeue(n); }
    }

    pub fn stats(&self) -> BackpressureV2Stats {
        let active = self.sources.values().filter(|s| s.level != BpLevel::None).count() as u32;
        let accepted: u64 = self.sources.values().map(|s| s.items_accepted).sum();
        let dropped: u64 = self.sources.values().map(|s| s.items_dropped).sum();
        let paused: u64 = self.sources.values().map(|s| s.items_paused).sum();
        let utils: Vec<f64> = self.sources.values().map(|s| s.utilization()).collect();
        let avg = if utils.is_empty() { 0.0 } else { utils.iter().sum::<f64>() / utils.len() as f64 };
        BackpressureV2Stats {
            total_sources: self.sources.len() as u32, active_backpressure: active,
            total_accepted: accepted, total_dropped: dropped,
            total_paused: paused, avg_utilization: avg,
        }
    }
}
