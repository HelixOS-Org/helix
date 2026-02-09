//! # Coop Clock Sync
//!
//! Distributed clock synchronization for cooperative modules:
//! - Hybrid logical clock (HLC) with physical + logical components
//! - NTP-style offset estimation
//! - Clock skew detection and correction
//! - Bounded clock uncertainty tracking
//! - Causal timestamp generation
//! - Clock drift rate estimation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Clock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockType {
    Physical,
    Logical,
    HybridLogical,
    VectorClock,
    IntervalClock,
}

/// Hybrid logical clock timestamp
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HlcTimestamp {
    pub wall_ns: u64,
    pub logical: u32,
    pub node_id: u64,
}

impl HlcTimestamp {
    pub fn new(wall: u64, node: u64) -> Self { Self { wall_ns: wall, logical: 0, node_id: node } }

    #[inline(always)]
    pub fn zero(node: u64) -> Self { Self { wall_ns: 0, logical: 0, node_id: node } }

    #[inline]
    pub fn advance(&mut self, physical_now: u64) {
        if physical_now > self.wall_ns {
            self.wall_ns = physical_now;
            self.logical = 0;
        } else {
            self.logical += 1;
        }
    }

    pub fn merge(&mut self, other: &HlcTimestamp, physical_now: u64) {
        let max_wall = core::cmp::max(core::cmp::max(self.wall_ns, other.wall_ns), physical_now);
        if max_wall == self.wall_ns && max_wall == other.wall_ns {
            self.logical = core::cmp::max(self.logical, other.logical) + 1;
        } else if max_wall == self.wall_ns {
            self.logical += 1;
        } else if max_wall == other.wall_ns {
            self.logical = other.logical + 1;
        } else {
            self.logical = 0;
        }
        self.wall_ns = max_wall;
    }

    #[inline(always)]
    pub fn happens_before(&self, other: &HlcTimestamp) -> bool {
        self.wall_ns < other.wall_ns || (self.wall_ns == other.wall_ns && self.logical < other.logical)
    }

    #[inline(always)]
    pub fn to_u128(&self) -> u128 { ((self.wall_ns as u128) << 64) | ((self.logical as u128) << 32) | self.node_id as u128 }
}

/// Clock sample from a peer
#[derive(Debug, Clone)]
pub struct ClockSample {
    pub peer: u64,
    pub send_ts: u64,
    pub recv_ts: u64,
    pub remote_ts: u64,
    pub rtt_ns: u64,
    pub offset_ns: i64,
}

impl ClockSample {
    #[inline]
    pub fn compute(peer: u64, t1: u64, t2: u64, t3: u64, t4: u64) -> Self {
        let rtt = (t4.saturating_sub(t1)).saturating_sub(t3.saturating_sub(t2));
        let offset = ((t2 as i128 - t1 as i128) + (t3 as i128 - t4 as i128)) / 2;
        Self { peer, send_ts: t1, recv_ts: t4, remote_ts: t2, rtt_ns: rtt, offset_ns: offset as i64 }
    }
}

/// Per-peer clock state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PeerClockState {
    pub peer_id: u64,
    pub samples: VecDeque<ClockSample>,
    pub avg_offset_ns: i64,
    pub avg_rtt_ns: u64,
    pub drift_ppb: i64,
    pub uncertainty_ns: u64,
    pub last_sync_ts: u64,
    pub sample_count: u64,
    pub max_samples: usize,
}

impl PeerClockState {
    pub fn new(peer: u64, max_samples: usize) -> Self {
        Self { peer_id: peer, samples: VecDeque::new(), avg_offset_ns: 0, avg_rtt_ns: 0, drift_ppb: 0, uncertainty_ns: u64::MAX, last_sync_ts: 0, sample_count: 0, max_samples }
    }

    #[inline]
    pub fn add_sample(&mut self, sample: ClockSample) {
        self.last_sync_ts = sample.recv_ts;
        self.sample_count += 1;
        self.samples.push_back(sample);
        if self.samples.len() > self.max_samples { self.samples.pop_front(); }
        self.recompute();
    }

    fn recompute(&mut self) {
        if self.samples.is_empty() { return; }
        let n = self.samples.len() as i64;
        let sum_off: i64 = self.samples.iter().map(|s| s.offset_ns).sum();
        self.avg_offset_ns = sum_off / n;
        let sum_rtt: u64 = self.samples.iter().map(|s| s.rtt_ns).sum();
        self.avg_rtt_ns = sum_rtt / n as u64;
        self.uncertainty_ns = self.avg_rtt_ns / 2;

        // Estimate drift if we have enough samples
        if self.samples.len() >= 2 {
            let first = &self.samples[0];
            let last = &self.samples[self.samples.len() - 1];
            let dt = last.recv_ts.saturating_sub(first.recv_ts);
            if dt > 0 {
                let doff = last.offset_ns - first.offset_ns;
                self.drift_ppb = (doff as i128 * 1_000_000_000 / dt as i128) as i64;
            }
        }
    }

    #[inline(always)]
    pub fn estimated_offset(&self, now: u64) -> i64 {
        let elapsed = now.saturating_sub(self.last_sync_ts) as i64;
        self.avg_offset_ns + (self.drift_ppb * elapsed / 1_000_000_000)
    }
}

/// Skew alert
#[derive(Debug, Clone)]
pub struct SkewAlert {
    pub peer: u64,
    pub offset_ns: i64,
    pub threshold_ns: u64,
    pub ts: u64,
}

/// Clock sync stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ClockSyncStats {
    pub peers: usize,
    pub total_samples: u64,
    pub max_offset_ns: i64,
    pub min_offset_ns: i64,
    pub max_rtt_ns: u64,
    pub alerts: usize,
}

/// Cooperative clock synchronization
pub struct CoopClockSync {
    node_id: u64,
    hlc: HlcTimestamp,
    peers: BTreeMap<u64, PeerClockState>,
    alerts: Vec<SkewAlert>,
    stats: ClockSyncStats,
    skew_threshold_ns: u64,
    max_samples: usize,
}

impl CoopClockSync {
    pub fn new(node_id: u64, skew_threshold: u64, max_samples: usize) -> Self {
        Self {
            node_id, hlc: HlcTimestamp::zero(node_id),
            peers: BTreeMap::new(), alerts: Vec::new(),
            stats: ClockSyncStats::default(), skew_threshold_ns: skew_threshold,
            max_samples,
        }
    }

    #[inline(always)]
    pub fn tick(&mut self, physical_now: u64) -> HlcTimestamp { self.hlc.advance(physical_now); self.hlc }

    #[inline(always)]
    pub fn receive(&mut self, remote: &HlcTimestamp, physical_now: u64) -> HlcTimestamp {
        self.hlc.merge(remote, physical_now);
        self.hlc
    }

    #[inline(always)]
    pub fn add_peer(&mut self, peer: u64) {
        self.peers.entry(peer).or_insert_with(|| PeerClockState::new(peer, self.max_samples));
    }

    #[inline]
    pub fn record_sample(&mut self, peer: u64, t1: u64, t2: u64, t3: u64, t4: u64) {
        let sample = ClockSample::compute(peer, t1, t2, t3, t4);
        let thresh = self.skew_threshold_ns;
        let p = self.peers.entry(peer).or_insert_with(|| PeerClockState::new(peer, self.max_samples));
        p.add_sample(sample);
        if (p.avg_offset_ns.unsigned_abs()) > thresh {
            self.alerts.push(SkewAlert { peer, offset_ns: p.avg_offset_ns, threshold_ns: thresh, ts: t4 });
        }
    }

    #[inline(always)]
    pub fn estimated_offset(&self, peer: u64, now: u64) -> Option<i64> {
        self.peers.get(&peer).map(|p| p.estimated_offset(now))
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.peers = self.peers.len();
        self.stats.total_samples = self.peers.values().map(|p| p.sample_count).sum();
        self.stats.max_offset_ns = self.peers.values().map(|p| p.avg_offset_ns).max().unwrap_or(0);
        self.stats.min_offset_ns = self.peers.values().map(|p| p.avg_offset_ns).min().unwrap_or(0);
        self.stats.max_rtt_ns = self.peers.values().map(|p| p.avg_rtt_ns).max().unwrap_or(0);
        self.stats.alerts = self.alerts.len();
    }

    #[inline(always)]
    pub fn hlc(&self) -> &HlcTimestamp { &self.hlc }
    #[inline(always)]
    pub fn peer(&self, id: u64) -> Option<&PeerClockState> { self.peers.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &ClockSyncStats { &self.stats }
    #[inline(always)]
    pub fn alerts(&self) -> &[SkewAlert] { &self.alerts }
    #[inline(always)]
    pub fn node_id(&self) -> u64 { self.node_id }
}
