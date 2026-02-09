//! # Coop Failure Detector
//!
//! Adaptive failure detection for cooperative subsystems:
//! - Phi-accrual failure detection
//! - Heartbeat window tracking
//! - Suspicion level computation
//! - Adaptive timeout calculation
//! - Multi-path failure detection
//! - Network partition detection

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Detection method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMethod {
    FixedTimeout,
    PhiAccrual,
    Swim,
    Adaptive,
}

/// Node health assessment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthAssessment {
    Healthy,
    Suspect,
    ProbablyDown,
    Down,
    Unreachable,
}

/// Heartbeat window for phi-accrual
#[derive(Debug, Clone)]
pub struct HeartbeatWindow {
    pub intervals: VecDeque<u64>,
    pub max_size: usize,
    pub last_heartbeat_ts: u64,
    pub heartbeat_count: u64,
}

impl HeartbeatWindow {
    pub fn new(max_size: usize) -> Self {
        Self { intervals: VecDeque::new(), max_size, last_heartbeat_ts: 0, heartbeat_count: 0 }
    }

    #[inline]
    pub fn record(&mut self, ts: u64) {
        if self.last_heartbeat_ts > 0 {
            let interval = ts.saturating_sub(self.last_heartbeat_ts);
            self.intervals.push_back(interval);
            if self.intervals.len() > self.max_size { self.intervals.pop_front(); }
        }
        self.last_heartbeat_ts = ts;
        self.heartbeat_count += 1;
    }

    #[inline(always)]
    pub fn mean_ns(&self) -> f64 {
        if self.intervals.is_empty() { return 0.0; }
        self.intervals.iter().sum::<u64>() as f64 / self.intervals.len() as f64
    }

    #[inline]
    pub fn variance(&self) -> f64 {
        if self.intervals.len() < 2 { return 0.0; }
        let mean = self.mean_ns();
        let sum_sq: f64 = self.intervals.iter().map(|&i| { let d = i as f64 - mean; d * d }).sum();
        sum_sq / (self.intervals.len() - 1) as f64
    }

    #[inline(always)]
    pub fn std_dev(&self) -> f64 { libm::sqrt(self.variance()) }

    /// Compute phi value: -log10(P(now - last >= t_now - last))
    /// Using normal distribution approximation
    pub fn phi(&self, now: u64) -> f64 {
        if self.intervals.is_empty() || self.last_heartbeat_ts == 0 { return 0.0; }
        let elapsed = now.saturating_sub(self.last_heartbeat_ts) as f64;
        let mean = self.mean_ns();
        let std = self.std_dev();
        if std < 1.0 { // Avoid division by zero
            return if elapsed > mean { 16.0 } else { 0.0 };
        }
        let y = (elapsed - mean) / std;
        // P(X > elapsed) = 1 - Phi(y) where Phi is CDF of normal distribution
        // Use approximation: P ≈ exp(-y²/2) / (y * sqrt(2π)) for large y
        let p = if y <= 0.0 {
            1.0
        } else if y > 8.0 {
            1e-16
        } else {
            // Approximate using erfc: P = 0.5 * erfc(y / sqrt(2))
            let t = 1.0 / (1.0 + 0.2316419 * y);
            let poly = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
            let e = libm::exp(-y * y / 2.0) * 0.3989422804014327; // 1/sqrt(2π)
            e * poly
        };
        if p <= 0.0 { 16.0 } else { -libm::log(p) / core::f64::consts::LN_10 }
    }
}

/// Per-node failure detector state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NodeDetectorState {
    pub node_id: u64,
    pub assessment: HealthAssessment,
    pub heartbeat_window: HeartbeatWindow,
    pub phi_value: f64,
    pub phi_threshold: f64,
    pub fixed_timeout_ns: u64,
    pub ping_attempts: u64,
    pub pong_received: u64,
    pub consecutive_misses: u32,
    pub last_assessment_ts: u64,
    pub detection_method: DetectionMethod,
    pub paths_checked: u32,
    pub paths_failed: u32,
}

impl NodeDetectorState {
    pub fn new(node_id: u64, method: DetectionMethod) -> Self {
        Self {
            node_id, assessment: HealthAssessment::Healthy,
            heartbeat_window: HeartbeatWindow::new(100), phi_value: 0.0,
            phi_threshold: 8.0, fixed_timeout_ns: 5_000_000_000,
            ping_attempts: 0, pong_received: 0, consecutive_misses: 0,
            last_assessment_ts: 0, detection_method: method,
            paths_checked: 0, paths_failed: 0,
        }
    }

    #[inline]
    pub fn heartbeat(&mut self, ts: u64) {
        self.heartbeat_window.record(ts);
        self.pong_received += 1;
        self.consecutive_misses = 0;
        self.assessment = HealthAssessment::Healthy;
    }

    #[inline(always)]
    pub fn miss(&mut self) { self.consecutive_misses += 1; }

    pub fn evaluate(&mut self, now: u64) -> HealthAssessment {
        self.last_assessment_ts = now;
        match self.detection_method {
            DetectionMethod::PhiAccrual => {
                self.phi_value = self.heartbeat_window.phi(now);
                if self.phi_value >= self.phi_threshold * 2.0 { self.assessment = HealthAssessment::Down; }
                else if self.phi_value >= self.phi_threshold { self.assessment = HealthAssessment::ProbablyDown; }
                else if self.phi_value >= self.phi_threshold * 0.5 { self.assessment = HealthAssessment::Suspect; }
                else { self.assessment = HealthAssessment::Healthy; }
            }
            DetectionMethod::FixedTimeout => {
                let elapsed = now.saturating_sub(self.heartbeat_window.last_heartbeat_ts);
                if elapsed > self.fixed_timeout_ns * 3 { self.assessment = HealthAssessment::Down; }
                else if elapsed > self.fixed_timeout_ns * 2 { self.assessment = HealthAssessment::ProbablyDown; }
                else if elapsed > self.fixed_timeout_ns { self.assessment = HealthAssessment::Suspect; }
                else { self.assessment = HealthAssessment::Healthy; }
            }
            DetectionMethod::Adaptive => {
                let mean = self.heartbeat_window.mean_ns();
                let std = self.heartbeat_window.std_dev();
                let adaptive_timeout = mean + 4.0 * std;
                let elapsed = now.saturating_sub(self.heartbeat_window.last_heartbeat_ts) as f64;
                if elapsed > adaptive_timeout * 3.0 { self.assessment = HealthAssessment::Down; }
                else if elapsed > adaptive_timeout * 2.0 { self.assessment = HealthAssessment::ProbablyDown; }
                else if elapsed > adaptive_timeout { self.assessment = HealthAssessment::Suspect; }
                else { self.assessment = HealthAssessment::Healthy; }
            }
            _ => {}
        }
        self.assessment
    }

    #[inline(always)]
    pub fn response_rate(&self) -> f64 {
        if self.ping_attempts == 0 { return 1.0; }
        self.pong_received as f64 / self.ping_attempts as f64
    }
}

/// Partition detection state
#[derive(Debug, Clone)]
pub struct PartitionDetector {
    pub groups: Vec<Vec<u64>>,
    pub is_partitioned: bool,
    pub partition_ts: Option<u64>,
    pub merge_ts: Option<u64>,
}

impl PartitionDetector {
    pub fn new() -> Self {
        Self { groups: Vec::new(), is_partitioned: false, partition_ts: None, merge_ts: None }
    }

    pub fn detect(&mut self, reachability: &BTreeMap<u64, Vec<u64>>, now: u64) {
        // Simple connected components
        let mut visited: LinearMap<bool, 64> = BTreeMap::new();
        self.groups.clear();
        for &node in reachability.keys() { visited.insert(node, false); }

        for &start in reachability.keys() {
            if *visited.get(&start).unwrap_or(&true) { continue; }
            let mut component = Vec::new();
            let mut stack = alloc::vec![start];
            while let Some(n) = stack.pop() {
                if *visited.get(&n).unwrap_or(&true) { continue; }
                visited.insert(n, true);
                component.push(n);
                if let Some(neighbors) = reachability.get(&n) {
                    for &nb in neighbors { if !*visited.get(&nb).unwrap_or(&true) { stack.push(nb); } }
                }
            }
            self.groups.push(component);
        }
        let was_partitioned = self.is_partitioned;
        self.is_partitioned = self.groups.len() > 1;
        if self.is_partitioned && !was_partitioned { self.partition_ts = Some(now); }
        if !self.is_partitioned && was_partitioned { self.merge_ts = Some(now); }
    }
}

/// Failure detector stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FailureDetectorStats {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub suspect_nodes: usize,
    pub down_nodes: usize,
    pub avg_phi: f64,
    pub max_phi: f64,
    pub is_partitioned: bool,
    pub partition_groups: usize,
}

/// Coop failure detector
pub struct CoopFailureDetector {
    nodes: BTreeMap<u64, NodeDetectorState>,
    partition: PartitionDetector,
    stats: FailureDetectorStats,
    default_method: DetectionMethod,
}

impl CoopFailureDetector {
    pub fn new(method: DetectionMethod) -> Self {
        Self { nodes: BTreeMap::new(), partition: PartitionDetector::new(), stats: FailureDetectorStats::default(), default_method: method }
    }

    #[inline(always)]
    pub fn add_node(&mut self, node_id: u64) {
        self.nodes.entry(node_id).or_insert_with(|| NodeDetectorState::new(node_id, self.default_method));
    }

    #[inline(always)]
    pub fn heartbeat(&mut self, node_id: u64, ts: u64) {
        if let Some(n) = self.nodes.get_mut(&node_id) { n.heartbeat(ts); }
    }

    #[inline(always)]
    pub fn evaluate_all(&mut self, now: u64) {
        for n in self.nodes.values_mut() { n.evaluate(now); }
    }

    #[inline(always)]
    pub fn get_assessment(&self, node_id: u64) -> Option<HealthAssessment> {
        self.nodes.get(&node_id).map(|n| n.assessment)
    }

    #[inline(always)]
    pub fn detect_partition(&mut self, reachability: &BTreeMap<u64, Vec<u64>>, now: u64) {
        self.partition.detect(reachability, now);
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_nodes = self.nodes.len();
        self.stats.healthy_nodes = self.nodes.values().filter(|n| n.assessment == HealthAssessment::Healthy).count();
        self.stats.suspect_nodes = self.nodes.values().filter(|n| n.assessment == HealthAssessment::Suspect).count();
        self.stats.down_nodes = self.nodes.values().filter(|n| n.assessment == HealthAssessment::Down || n.assessment == HealthAssessment::ProbablyDown).count();
        let phis: Vec<f64> = self.nodes.values().map(|n| n.phi_value).collect();
        self.stats.avg_phi = if phis.is_empty() { 0.0 } else { phis.iter().sum::<f64>() / phis.len() as f64 };
        self.stats.max_phi = phis.iter().cloned().fold(0.0_f64, f64::max);
        self.stats.is_partitioned = self.partition.is_partitioned;
        self.stats.partition_groups = self.partition.groups.len();
    }

    #[inline(always)]
    pub fn node(&self, id: u64) -> Option<&NodeDetectorState> { self.nodes.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &FailureDetectorStats { &self.stats }
}
