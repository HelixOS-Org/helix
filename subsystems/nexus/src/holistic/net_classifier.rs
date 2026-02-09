// SPDX-License-Identifier: GPL-2.0
//! Holistic net_classifier — network traffic classification and QoS marking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Traffic class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrafficClass {
    BestEffort,
    Background,
    Interactive,
    Streaming,
    Realtime,
    Control,
    Scavenger,
}

impl TrafficClass {
    #[inline]
    pub fn dscp_value(&self) -> u8 {
        match self {
            Self::BestEffort => 0,
            Self::Background => 8,
            Self::Interactive => 24,
            Self::Streaming => 32,
            Self::Realtime => 46,
            Self::Control => 48,
            Self::Scavenger => 10,
        }
    }

    #[inline]
    pub fn priority(&self) -> u8 {
        match self {
            Self::Scavenger => 0,
            Self::Background => 1,
            Self::BestEffort => 2,
            Self::Streaming => 3,
            Self::Interactive => 4,
            Self::Realtime => 5,
            Self::Control => 6,
        }
    }
}

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
    Sctp,
    Unknown,
}

/// Match criteria for classification
#[derive(Debug, Clone)]
pub struct ClassifyMatch {
    pub src_port: Option<(u16, u16)>,
    pub dst_port: Option<(u16, u16)>,
    pub protocol: Option<Protocol>,
    pub cgroup_id: Option<u64>,
    pub pid: Option<u32>,
    pub mark: Option<u32>,
    pub dscp: Option<u8>,
}

impl ClassifyMatch {
    pub fn new() -> Self {
        Self {
            src_port: None, dst_port: None, protocol: None,
            cgroup_id: None, pid: None, mark: None, dscp: None,
        }
    }

    #[inline]
    pub fn matches_port(&self, src: u16, dst: u16) -> bool {
        if let Some((lo, hi)) = self.src_port {
            if src < lo || src > hi { return false; }
        }
        if let Some((lo, hi)) = self.dst_port {
            if dst < lo || dst > hi { return false; }
        }
        true
    }
}

/// Classification rule
#[derive(Debug, Clone)]
pub struct ClassifyRule {
    pub id: u32,
    pub name: String,
    pub priority: u32,
    pub match_criteria: ClassifyMatch,
    pub traffic_class: TrafficClass,
    pub rate_limit_bps: Option<u64>,
    pub burst_bytes: Option<u64>,
    pub hit_count: u64,
    pub bytes_matched: u64,
    pub enabled: bool,
}

impl ClassifyRule {
    pub fn new(id: u32, name: String, tc: TrafficClass) -> Self {
        Self {
            id, name, priority: 100,
            match_criteria: ClassifyMatch::new(),
            traffic_class: tc,
            rate_limit_bps: None,
            burst_bytes: None,
            hit_count: 0,
            bytes_matched: 0,
            enabled: true,
        }
    }

    #[inline(always)]
    pub fn avg_packet_size(&self) -> u64 {
        if self.hit_count == 0 { return 0; }
        self.bytes_matched / self.hit_count
    }
}

/// Per-class statistics
#[derive(Debug)]
#[repr(align(64))]
pub struct ClassStats {
    pub traffic_class: TrafficClass,
    pub packets: u64,
    pub bytes: u64,
    pub drops: u64,
    pub rate_limited: u64,
    pub current_rate_bps: u64,
    pub avg_latency_us: u64,
}

impl ClassStats {
    pub fn new(tc: TrafficClass) -> Self {
        Self {
            traffic_class: tc,
            packets: 0, bytes: 0, drops: 0,
            rate_limited: 0, current_rate_bps: 0,
            avg_latency_us: 0,
        }
    }

    #[inline(always)]
    pub fn drop_rate(&self) -> f64 {
        if self.packets == 0 { return 0.0; }
        self.drops as f64 / self.packets as f64
    }

    #[inline(always)]
    pub fn avg_packet_size(&self) -> u64 {
        if self.packets == 0 { return 0; }
        self.bytes / self.packets
    }
}

/// Per-flow tracking
#[derive(Debug)]
pub struct FlowEntry {
    pub flow_hash: u64,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: Protocol,
    pub traffic_class: TrafficClass,
    pub packets: u64,
    pub bytes: u64,
    pub first_seen: u64,
    pub last_seen: u64,
}

impl FlowEntry {
    #[inline(always)]
    pub fn duration(&self) -> u64 {
        self.last_seen.saturating_sub(self.first_seen)
    }

    #[inline]
    pub fn rate_bps(&self) -> u64 {
        let dur_s = self.duration() / 1_000_000;
        if dur_s == 0 { return self.bytes * 8; }
        (self.bytes * 8) / dur_s
    }

    #[inline(always)]
    pub fn is_idle(&self, now: u64, timeout: u64) -> bool {
        now.saturating_sub(self.last_seen) > timeout
    }
}

/// Classifier stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NetClassifierStats {
    pub total_rules: u32,
    pub active_rules: u32,
    pub total_packets: u64,
    pub total_bytes: u64,
    pub unclassified: u64,
    pub active_flows: u64,
}

/// Main net classifier
pub struct HolisticNetClassifier {
    rules: BTreeMap<u32, ClassifyRule>,
    class_stats: BTreeMap<u8, ClassStats>,
    flows: BTreeMap<u64, FlowEntry>,
    max_flows: usize,
    next_rule_id: u32,
    stats: NetClassifierStats,
}

impl HolisticNetClassifier {
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
            class_stats: BTreeMap::new(),
            flows: BTreeMap::new(),
            max_flows: 16384,
            next_rule_id: 1,
            stats: NetClassifierStats {
                total_rules: 0, active_rules: 0,
                total_packets: 0, total_bytes: 0,
                unclassified: 0, active_flows: 0,
            },
        }
    }

    #[inline]
    pub fn add_rule(&mut self, mut rule: ClassifyRule) -> u32 {
        let id = self.next_rule_id;
        self.next_rule_id += 1;
        rule.id = id;
        self.stats.total_rules += 1;
        if rule.enabled { self.stats.active_rules += 1; }
        self.rules.insert(id, rule);
        id
    }

    #[inline]
    pub fn remove_rule(&mut self, id: u32) -> bool {
        if let Some(rule) = self.rules.remove(&id) {
            self.stats.total_rules -= 1;
            if rule.enabled { self.stats.active_rules -= 1; }
            true
        } else {
            false
        }
    }

    pub fn classify(&mut self, src_port: u16, dst_port: u16, protocol: Protocol, bytes: u64) -> TrafficClass {
        self.stats.total_packets += 1;
        self.stats.total_bytes += bytes;

        // rules sorted by priority (lower = higher priority)
        let mut matched_class = None;
        let mut matched_id = None;
        let mut best_prio = u32::MAX;

        for (&id, rule) in &self.rules {
            if !rule.enabled { continue; }
            if rule.priority >= best_prio { continue; }
            if let Some(p) = rule.match_criteria.protocol {
                if p != protocol { continue; }
            }
            if !rule.match_criteria.matches_port(src_port, dst_port) { continue; }
            matched_class = Some(rule.traffic_class);
            matched_id = Some(id);
            best_prio = rule.priority;
        }

        if let Some(id) = matched_id {
            if let Some(rule) = self.rules.get_mut(&id) {
                rule.hit_count += 1;
                rule.bytes_matched += bytes;
            }
        }

        let tc = matched_class.unwrap_or_else(|| {
            self.stats.unclassified += 1;
            TrafficClass::BestEffort
        });

        // update class stats
        let cs = self.class_stats.entry(tc.priority())
            .or_insert_with(|| ClassStats::new(tc));
        cs.packets += 1;
        cs.bytes += bytes;

        tc
    }

    pub fn track_flow(&mut self, flow_hash: u64, src_port: u16, dst_port: u16,
                       protocol: Protocol, tc: TrafficClass, bytes: u64, now: u64) {
        if let Some(flow) = self.flows.get_mut(&flow_hash) {
            flow.packets += 1;
            flow.bytes += bytes;
            flow.last_seen = now;
        } else {
            if self.flows.len() >= self.max_flows {
                // evict oldest
                if let Some(&oldest) = self.flows.iter()
                    .min_by_key(|(_, f)| f.last_seen)
                    .map(|(k, _)| k)
                {
                    self.flows.remove(&oldest);
                }
            }
            self.flows.insert(flow_hash, FlowEntry {
                flow_hash, src_port, dst_port, protocol,
                traffic_class: tc, packets: 1, bytes,
                first_seen: now, last_seen: now,
            });
            self.stats.active_flows += 1;
        }
    }

    #[inline]
    pub fn expire_flows(&mut self, now: u64, timeout: u64) -> u64 {
        let before = self.flows.len();
        self.flows.retain(|_, f| !f.is_idle(now, timeout));
        let removed = before - self.flows.len();
        self.stats.active_flows = self.flows.len() as u64;
        removed as u64
    }

    #[inline]
    pub fn top_flows(&self, n: usize) -> Vec<(u64, u64)> {
        let mut v: Vec<_> = self.flows.iter()
            .map(|(&h, f)| (h, f.bytes))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    #[inline(always)]
    pub fn stats(&self) -> &NetClassifierStats {
        &self.stats
    }
}
