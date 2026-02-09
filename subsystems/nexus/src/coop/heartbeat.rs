//! # Coop Heartbeat V2
//!
//! Advanced heartbeat with φ accrual failure detection:
//! - Adaptive heartbeat intervals
//! - Phi (φ) accrual failure detection algorithm
//! - Exponential distribution-based suspicion level
//! - Multi-window arrival time statistics
//! - Heartbeat payload exchange
//! - Failure suspicion level gradients

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Heartbeat status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatStatus {
    Alive,
    Suspected,
    Failed,
    Unknown,
}

/// Arrival time window for phi calculation
#[derive(Debug, Clone)]
pub struct ArrivalWindow {
    samples: VecDeque<u64>,
    max_samples: usize,
}

impl ArrivalWindow {
    pub fn new(max: usize) -> Self { Self { samples: VecDeque::new(), max_samples: max } }

    #[inline(always)]
    pub fn record(&mut self, interval_ns: u64) {
        self.samples.push_back(interval_ns);
        if self.samples.len() > self.max_samples { self.samples.pop_front(); }
    }

    #[inline]
    pub fn mean(&self) -> f64 {
        if self.samples.is_empty() { return 0.0; }
        let sum: u64 = self.samples.iter().sum();
        sum as f64 / self.samples.len() as f64
    }

    #[inline]
    pub fn variance(&self) -> f64 {
        if self.samples.len() < 2 { return 0.0; }
        let m = self.mean();
        let sum_sq: f64 = self.samples.iter().map(|&s| { let d = s as f64 - m; d * d }).sum();
        sum_sq / (self.samples.len() - 1) as f64
    }

    #[inline(always)]
    pub fn stddev(&self) -> f64 { libm::sqrt(self.variance()) }

    pub fn phi(&self, time_since_last_ns: u64) -> f64 {
        let m = self.mean();
        if m <= 0.0 { return 0.0; }
        let sd = self.stddev().max(1.0);
        let y = (time_since_last_ns as f64 - m) / sd;
        // Approximate phi using exponential CDF: phi = -log10(1 - F(y))
        // where F(y) = 1 - e^(-0.5 * e^(y * 0.75))
        // Simplified: phi ≈ y * 0.4342944819 for large y
        if y <= 0.0 { return 0.0; }
        let exponent = -0.5 * libm::exp(y * 0.5);
        let p = libm::exp(exponent);
        if p <= 0.0 { return 16.0; } // cap at 16
        let phi = -libm::log(p) / core::f64::consts::LN_10;
        if phi > 16.0 { 16.0 } else { phi }
    }

    #[inline(always)]
    pub fn sample_count(&self) -> usize { self.samples.len() }
}

/// Per-peer heartbeat state
#[derive(Debug, Clone)]
pub struct PeerHeartbeat {
    pub peer_id: u64,
    pub status: HeartbeatStatus,
    pub arrival_window: ArrivalWindow,
    pub last_heartbeat_ts: u64,
    pub last_phi: f64,
    pub send_interval_ns: u64,
    pub heartbeats_received: u64,
    pub heartbeats_sent: u64,
    pub consecutive_misses: u32,
    pub last_payload_version: u64,
    pub registered_ts: u64,
}

impl PeerHeartbeat {
    pub fn new(id: u64, interval_ns: u64, ts: u64) -> Self {
        Self {
            peer_id: id, status: HeartbeatStatus::Unknown,
            arrival_window: ArrivalWindow::new(100),
            last_heartbeat_ts: ts, last_phi: 0.0,
            send_interval_ns: interval_ns, heartbeats_received: 0,
            heartbeats_sent: 0, consecutive_misses: 0,
            last_payload_version: 0, registered_ts: ts,
        }
    }

    #[inline]
    pub fn receive_heartbeat(&mut self, ts: u64, payload_version: u64) {
        if self.heartbeats_received > 0 {
            let interval = ts.saturating_sub(self.last_heartbeat_ts);
            self.arrival_window.record(interval);
        }
        self.last_heartbeat_ts = ts;
        self.heartbeats_received += 1;
        self.consecutive_misses = 0;
        self.last_payload_version = payload_version;
        self.status = HeartbeatStatus::Alive;
    }

    #[inline]
    pub fn compute_phi(&mut self, now: u64) -> f64 {
        if self.heartbeats_received < 2 { return 0.0; }
        let time_since = now.saturating_sub(self.last_heartbeat_ts);
        self.last_phi = self.arrival_window.phi(time_since);
        self.last_phi
    }

    #[inline]
    pub fn update_status(&mut self, phi_suspect: f64, phi_fail: f64) {
        if self.last_phi >= phi_fail {
            self.status = HeartbeatStatus::Failed;
        } else if self.last_phi >= phi_suspect {
            self.status = HeartbeatStatus::Suspected;
        } else if self.heartbeats_received > 0 {
            self.status = HeartbeatStatus::Alive;
        }
    }
}

/// Heartbeat V2 stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HeartbeatV2Stats {
    pub total_peers: usize,
    pub alive_peers: usize,
    pub suspected_peers: usize,
    pub failed_peers: usize,
    pub unknown_peers: usize,
    pub total_received: u64,
    pub total_sent: u64,
    pub avg_phi: f64,
    pub max_phi: f64,
}

/// Cooperative heartbeat V2 manager
pub struct CoopHeartbeatV2 {
    peers: BTreeMap<u64, PeerHeartbeat>,
    default_interval_ns: u64,
    phi_suspect_threshold: f64,
    phi_fail_threshold: f64,
    stats: HeartbeatV2Stats,
}

impl CoopHeartbeatV2 {
    pub fn new(interval_ns: u64) -> Self {
        Self {
            peers: BTreeMap::new(), default_interval_ns: interval_ns,
            phi_suspect_threshold: 5.0, phi_fail_threshold: 8.0,
            stats: HeartbeatV2Stats::default(),
        }
    }

    #[inline(always)]
    pub fn register_peer(&mut self, id: u64, ts: u64) {
        self.peers.insert(id, PeerHeartbeat::new(id, self.default_interval_ns, ts));
    }

    #[inline(always)]
    pub fn unregister_peer(&mut self, id: u64) { self.peers.remove(&id); }

    #[inline(always)]
    pub fn receive_heartbeat(&mut self, peer_id: u64, ts: u64, payload_version: u64) {
        if let Some(p) = self.peers.get_mut(&peer_id) { p.receive_heartbeat(ts, payload_version); }
    }

    #[inline(always)]
    pub fn record_send(&mut self, peer_id: u64) {
        if let Some(p) = self.peers.get_mut(&peer_id) { p.heartbeats_sent += 1; }
    }

    #[inline]
    pub fn tick(&mut self, now: u64) {
        for peer in self.peers.values_mut() {
            peer.compute_phi(now);
            peer.update_status(self.phi_suspect_threshold, self.phi_fail_threshold);
        }
    }

    #[inline(always)]
    pub fn set_thresholds(&mut self, suspect: f64, fail: f64) {
        self.phi_suspect_threshold = suspect;
        self.phi_fail_threshold = fail;
    }

    #[inline(always)]
    pub fn failed_peers(&self) -> Vec<u64> {
        self.peers.iter().filter(|(_, p)| p.status == HeartbeatStatus::Failed).map(|(&id, _)| id).collect()
    }

    #[inline(always)]
    pub fn suspected_peers(&self) -> Vec<u64> {
        self.peers.iter().filter(|(_, p)| p.status == HeartbeatStatus::Suspected).map(|(&id, _)| id).collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_peers = self.peers.len();
        self.stats.alive_peers = self.peers.values().filter(|p| p.status == HeartbeatStatus::Alive).count();
        self.stats.suspected_peers = self.peers.values().filter(|p| p.status == HeartbeatStatus::Suspected).count();
        self.stats.failed_peers = self.peers.values().filter(|p| p.status == HeartbeatStatus::Failed).count();
        self.stats.unknown_peers = self.peers.values().filter(|p| p.status == HeartbeatStatus::Unknown).count();
        self.stats.total_received = self.peers.values().map(|p| p.heartbeats_received).sum();
        self.stats.total_sent = self.peers.values().map(|p| p.heartbeats_sent).sum();
        if !self.peers.is_empty() {
            self.stats.avg_phi = self.peers.values().map(|p| p.last_phi).sum::<f64>() / self.peers.len() as f64;
            self.stats.max_phi = self.peers.values().map(|p| p.last_phi).fold(0.0f64, |a, b| if b > a { b } else { a });
        }
    }

    #[inline(always)]
    pub fn peer(&self, id: u64) -> Option<&PeerHeartbeat> { self.peers.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &HeartbeatV2Stats { &self.stats }
}
