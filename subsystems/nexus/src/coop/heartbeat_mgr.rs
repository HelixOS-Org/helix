// SPDX-License-Identifier: GPL-2.0
//! Coop heartbeat_mgr — heartbeat monitoring and failure detection.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Heartbeat state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatState {
    Healthy,
    Warning,
    Critical,
    TimedOut,
    Recovered,
}

/// Failure detector type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorType {
    FixedTimeout,
    Adaptive,
    Phi,
    Swim,
}

/// Heartbeat record
#[derive(Debug, Clone)]
pub struct HeartbeatRecord {
    pub node_id: u64,
    pub seq: u64,
    pub sent_at: u64,
    pub received_at: u64,
    pub rtt_ns: u64,
}

/// Monitored node
#[derive(Debug)]
pub struct MonitoredNode {
    pub id: u64,
    pub state: HeartbeatState,
    pub last_heartbeat: u64,
    pub heartbeat_count: u64,
    pub missed_count: u64,
    pub timeout_count: u64,
    pub rtt_samples: Vec<u64>,
    pub avg_rtt_ns: u64,
    pub max_rtt_ns: u64,
    pub timeout_ns: u64,
    pub last_state_change: u64,
}

impl MonitoredNode {
    pub fn new(id: u64, timeout: u64, now: u64) -> Self {
        Self {
            id, state: HeartbeatState::Healthy, last_heartbeat: now,
            heartbeat_count: 0, missed_count: 0, timeout_count: 0,
            rtt_samples: Vec::new(), avg_rtt_ns: 0, max_rtt_ns: 0,
            timeout_ns: timeout, last_state_change: now,
        }
    }

    pub fn receive_heartbeat(&mut self, rtt: u64, now: u64) {
        self.heartbeat_count += 1;
        self.last_heartbeat = now;
        self.rtt_samples.push(rtt);
        if self.rtt_samples.len() > 100 { self.rtt_samples.drain(..50); }
        self.avg_rtt_ns = self.rtt_samples.iter().sum::<u64>() / self.rtt_samples.len() as u64;
        if rtt > self.max_rtt_ns { self.max_rtt_ns = rtt; }

        if self.state != HeartbeatState::Healthy {
            self.state = HeartbeatState::Recovered;
            self.last_state_change = now;
        }
    }

    pub fn check(&mut self, now: u64) -> HeartbeatState {
        let elapsed = now.saturating_sub(self.last_heartbeat);
        let old = self.state;
        self.state = if elapsed > self.timeout_ns * 3 {
            self.timeout_count += 1;
            HeartbeatState::TimedOut
        } else if elapsed > self.timeout_ns * 2 {
            HeartbeatState::Critical
        } else if elapsed > self.timeout_ns {
            self.missed_count += 1;
            HeartbeatState::Warning
        } else { HeartbeatState::Healthy };
        if self.state != old { self.last_state_change = now; }
        self.state
    }

    #[inline]
    pub fn failure_rate(&self) -> f64 {
        let total = self.heartbeat_count + self.missed_count;
        if total == 0 { return 0.0; }
        self.missed_count as f64 / total as f64
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HeartbeatMgrStats {
    pub total_nodes: u32,
    pub healthy: u32,
    pub warning: u32,
    pub critical: u32,
    pub timed_out: u32,
    pub avg_rtt_ns: u64,
    pub total_heartbeats: u64,
}

/// Main heartbeat manager
pub struct CoopHeartbeatMgr {
    nodes: BTreeMap<u64, MonitoredNode>,
    detector_type: DetectorType,
    default_timeout: u64,
}

impl CoopHeartbeatMgr {
    pub fn new(detector: DetectorType, timeout: u64) -> Self {
        Self { nodes: BTreeMap::new(), detector_type: detector, default_timeout: timeout }
    }

    #[inline(always)]
    pub fn monitor(&mut self, node_id: u64, now: u64) {
        self.nodes.insert(node_id, MonitoredNode::new(node_id, self.default_timeout, now));
    }

    #[inline(always)]
    pub fn heartbeat(&mut self, node_id: u64, rtt: u64, now: u64) {
        if let Some(n) = self.nodes.get_mut(&node_id) { n.receive_heartbeat(rtt, now); }
    }

    #[inline(always)]
    pub fn check_all(&mut self, now: u64) -> Vec<(u64, HeartbeatState)> {
        self.nodes.values_mut().map(|n| (n.id, n.check(now))).collect()
    }

    #[inline(always)]
    pub fn timed_out_nodes(&self) -> Vec<u64> {
        self.nodes.values().filter(|n| n.state == HeartbeatState::TimedOut).map(|n| n.id).collect()
    }

    pub fn stats(&self) -> HeartbeatMgrStats {
        let healthy = self.nodes.values().filter(|n| n.state == HeartbeatState::Healthy).count() as u32;
        let warning = self.nodes.values().filter(|n| n.state == HeartbeatState::Warning).count() as u32;
        let critical = self.nodes.values().filter(|n| n.state == HeartbeatState::Critical).count() as u32;
        let timed_out = self.nodes.values().filter(|n| n.state == HeartbeatState::TimedOut).count() as u32;
        let rtts: Vec<u64> = self.nodes.values().filter(|n| n.avg_rtt_ns > 0).map(|n| n.avg_rtt_ns).collect();
        let avg_rtt = if rtts.is_empty() { 0 } else { rtts.iter().sum::<u64>() / rtts.len() as u64 };
        let hbs: u64 = self.nodes.values().map(|n| n.heartbeat_count).sum();
        HeartbeatMgrStats {
            total_nodes: self.nodes.len() as u32, healthy, warning, critical,
            timed_out, avg_rtt_ns: avg_rtt, total_heartbeats: hbs,
        }
    }
}
