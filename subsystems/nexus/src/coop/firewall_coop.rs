// SPDX-License-Identifier: GPL-2.0
//! Coop firewall — cooperative packet filtering with shared rule sets

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coop firewall action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopFwAction {
    Accept,
    Drop,
    Reject,
    Log,
    Redirect,
    Nat,
    Mark,
    RateLimit,
}

/// Coop firewall chain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopFwChain {
    Input,
    Output,
    Forward,
    PreRouting,
    PostRouting,
}

/// Coop firewall match
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopFwMatch {
    SrcAddr,
    DstAddr,
    SrcPort,
    DstPort,
    Protocol,
    Interface,
    Mark,
    State,
    RateLimit,
}

/// Firewall rule
#[derive(Debug, Clone)]
pub struct CoopFwRule {
    pub rule_id: u64,
    pub chain: CoopFwChain,
    pub match_type: CoopFwMatch,
    pub match_value: u64,
    pub action: CoopFwAction,
    pub priority: i32,
    pub hit_count: u64,
    pub byte_count: u64,
    pub shared: bool,
}

impl CoopFwRule {
    pub fn new(rule_id: u64, chain: CoopFwChain, match_type: CoopFwMatch, action: CoopFwAction) -> Self {
        Self {
            rule_id, chain, match_type, match_value: 0, action, priority: 0,
            hit_count: 0, byte_count: 0, shared: false,
        }
    }

    pub fn matches(&self, packet_value: u64) -> bool {
        self.match_value == packet_value
    }

    pub fn apply(&mut self, pkt_bytes: u64) -> CoopFwAction {
        self.hit_count += 1;
        self.byte_count += pkt_bytes;
        self.action
    }
}

/// Shared rule set
#[derive(Debug, Clone)]
pub struct SharedRuleSet {
    pub set_id: u64,
    pub rules: Vec<CoopFwRule>,
    pub subscribers: Vec<u64>,
    pub version: u64,
}

impl SharedRuleSet {
    pub fn new(set_id: u64) -> Self {
        Self { set_id, rules: Vec::new(), subscribers: Vec::new(), version: 0 }
    }

    pub fn add_rule(&mut self, mut rule: CoopFwRule) {
        rule.shared = true;
        let pos = self.rules.iter().position(|r| r.priority > rule.priority).unwrap_or(self.rules.len());
        self.rules.insert(pos, rule);
        self.version += 1;
    }

    pub fn evaluate(&mut self, pkt_value: u64, pkt_bytes: u64) -> CoopFwAction {
        for rule in &mut self.rules {
            if rule.matches(pkt_value) {
                return rule.apply(pkt_bytes);
            }
        }
        CoopFwAction::Accept
    }

    pub fn subscribe(&mut self, ns_id: u64) {
        if !self.subscribers.contains(&ns_id) { self.subscribers.push(ns_id); }
    }
}

/// Coop firewall stats
#[derive(Debug, Clone)]
pub struct CoopFwStats {
    pub total_rules: u64,
    pub shared_sets: u64,
    pub total_packets: u64,
    pub total_drops: u64,
    pub total_accepts: u64,
}

/// Main coop firewall manager
#[derive(Debug)]
pub struct CoopFirewall {
    pub rule_sets: BTreeMap<u64, SharedRuleSet>,
    pub stats: CoopFwStats,
}

impl CoopFirewall {
    pub fn new() -> Self {
        Self {
            rule_sets: BTreeMap::new(),
            stats: CoopFwStats { total_rules: 0, shared_sets: 0, total_packets: 0, total_drops: 0, total_accepts: 0 },
        }
    }

    pub fn create_set(&mut self, set_id: u64) {
        self.rule_sets.insert(set_id, SharedRuleSet::new(set_id));
        self.stats.shared_sets += 1;
    }

    pub fn add_rule(&mut self, set_id: u64, rule: CoopFwRule) -> bool {
        if let Some(set) = self.rule_sets.get_mut(&set_id) {
            set.add_rule(rule);
            self.stats.total_rules += 1;
            true
        } else { false }
    }

    pub fn drop_rate(&self) -> f64 {
        if self.stats.total_packets == 0 { 0.0 }
        else { self.stats.total_drops as f64 / self.stats.total_packets as f64 }
    }
}
