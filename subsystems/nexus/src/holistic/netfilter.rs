// SPDX-License-Identifier: GPL-2.0
//! Holistic netfilter — packet filtering, NAT, and connection tracking

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Netfilter hook point
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NfHook {
    PreRouting,
    LocalIn,
    Forward,
    LocalOut,
    PostRouting,
}

/// Netfilter table type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfTableType {
    Filter,
    Nat,
    Mangle,
    Raw,
    Security,
}

/// Netfilter verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfVerdict {
    Accept,
    Drop,
    Stolen,
    Queue,
    Repeat,
    Stop,
    Jump(u32),
    Goto(u32),
    Return,
}

/// Netfilter match type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NfMatchType {
    SrcIp,
    DstIp,
    SrcPort,
    DstPort,
    Protocol,
    Interface,
    State,
    Limit,
    Mark,
    Conntrack,
    Owner,
    Multiport,
}

/// Connection tracking state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConntrackState {
    New,
    Established,
    Related,
    Invalid,
    Untracked,
    SnatReply,
    DnatReply,
}

/// NAT type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatType {
    Snat,
    Dnat,
    Masquerade,
    Redirect,
    Netmap,
    FullCone,
    RestrictedCone,
}

/// Netfilter rule match
#[derive(Debug, Clone)]
pub struct NfMatch {
    pub match_type: NfMatchType,
    pub value: u64,
    pub mask: u64,
    pub negate: bool,
}

impl NfMatch {
    pub fn check(&self, packet_val: u64) -> bool {
        let result = (packet_val & self.mask) == (self.value & self.mask);
        if self.negate { !result } else { result }
    }
}

/// Netfilter rule
#[derive(Debug, Clone)]
pub struct NfRule {
    pub rule_id: u32,
    pub chain_id: u32,
    pub priority: i32,
    pub matches: Vec<NfMatch>,
    pub verdict: NfVerdict,
    pub byte_counter: u64,
    pub packet_counter: u64,
    pub enabled: bool,
}

impl NfRule {
    pub fn new(rule_id: u32, chain_id: u32, verdict: NfVerdict) -> Self {
        Self {
            rule_id,
            chain_id,
            priority: 0,
            matches: Vec::new(),
            verdict,
            byte_counter: 0,
            packet_counter: 0,
            enabled: true,
        }
    }

    pub fn evaluate(&self, packet_fields: &[(NfMatchType, u64)]) -> Option<NfVerdict> {
        if !self.enabled {
            return None;
        }
        for m in &self.matches {
            let field_val = packet_fields.iter()
                .find(|(t, _)| *t == m.match_type)
                .map(|(_, v)| *v)
                .unwrap_or(0);
            if !m.check(field_val) {
                return None;
            }
        }
        Some(self.verdict)
    }
}

/// Netfilter chain
#[derive(Debug, Clone)]
pub struct NfChain {
    pub chain_id: u32,
    pub hook: NfHook,
    pub table_type: NfTableType,
    pub priority: i32,
    pub policy: NfVerdict,
    pub rules: Vec<NfRule>,
}

impl NfChain {
    pub fn new(chain_id: u32, hook: NfHook, table_type: NfTableType) -> Self {
        Self {
            chain_id,
            hook,
            table_type,
            priority: 0,
            policy: NfVerdict::Accept,
            rules: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: NfRule) {
        self.rules.push(rule);
        self.rules.sort_by_key(|r| r.priority);
    }

    pub fn evaluate_packet(&mut self, fields: &[(NfMatchType, u64)], pkt_bytes: u64) -> NfVerdict {
        for rule in &mut self.rules {
            if let Some(verdict) = rule.evaluate(fields) {
                rule.packet_counter += 1;
                rule.byte_counter += pkt_bytes;
                return verdict;
            }
        }
        self.policy
    }
}

/// Connection tracking entry
#[derive(Debug, Clone)]
pub struct ConntrackEntry {
    pub ct_id: u64,
    pub state: ConntrackState,
    pub src_ip: u32,
    pub dst_ip: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub packets_orig: u64,
    pub packets_reply: u64,
    pub bytes_orig: u64,
    pub bytes_reply: u64,
    pub timeout_ms: u64,
    pub nat_type: Option<NatType>,
    pub nat_addr: u32,
    pub nat_port: u16,
}

impl ConntrackEntry {
    pub fn new(ct_id: u64, src_ip: u32, dst_ip: u32, src_port: u16, dst_port: u16, proto: u8) -> Self {
        Self {
            ct_id,
            state: ConntrackState::New,
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            protocol: proto,
            packets_orig: 1,
            packets_reply: 0,
            bytes_orig: 0,
            bytes_reply: 0,
            timeout_ms: 120_000,
            nat_type: None,
            nat_addr: 0,
            nat_port: 0,
        }
    }

    pub fn update_orig(&mut self, bytes: u64) {
        self.packets_orig += 1;
        self.bytes_orig += bytes;
        if self.state == ConntrackState::New && self.packets_reply > 0 {
            self.state = ConntrackState::Established;
        }
    }

    pub fn update_reply(&mut self, bytes: u64) {
        self.packets_reply += 1;
        self.bytes_reply += bytes;
        if self.state == ConntrackState::New {
            self.state = ConntrackState::Established;
        }
    }

    pub fn flow_hash(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in self.src_ip.to_le_bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        for b in self.dst_ip.to_le_bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        for b in self.src_port.to_le_bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        for b in self.dst_port.to_le_bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        h ^= self.protocol as u64; h = h.wrapping_mul(0x100000001b3);
        h
    }
}

/// Netfilter stats
#[derive(Debug, Clone)]
pub struct NetfilterStats {
    pub total_rules: u64,
    pub total_chains: u64,
    pub packets_accepted: u64,
    pub packets_dropped: u64,
    pub conntrack_entries: u64,
    pub nat_translations: u64,
}

/// Main holistic netfilter manager
#[derive(Debug)]
pub struct HolisticNetfilter {
    pub chains: BTreeMap<u32, NfChain>,
    pub conntrack: BTreeMap<u64, ConntrackEntry>,
    pub stats: NetfilterStats,
    pub next_chain_id: u32,
    pub next_ct_id: u64,
    pub conntrack_max: u64,
}

impl HolisticNetfilter {
    pub fn new(conntrack_max: u64) -> Self {
        Self {
            chains: BTreeMap::new(),
            conntrack: BTreeMap::new(),
            stats: NetfilterStats {
                total_rules: 0,
                total_chains: 0,
                packets_accepted: 0,
                packets_dropped: 0,
                conntrack_entries: 0,
                nat_translations: 0,
            },
            next_chain_id: 1,
            next_ct_id: 1,
            conntrack_max,
        }
    }

    pub fn create_chain(&mut self, hook: NfHook, table_type: NfTableType) -> u32 {
        let id = self.next_chain_id;
        self.next_chain_id += 1;
        self.chains.insert(id, NfChain::new(id, hook, table_type));
        self.stats.total_chains += 1;
        id
    }

    pub fn add_rule_to_chain(&mut self, chain_id: u32, rule: NfRule) -> bool {
        if let Some(chain) = self.chains.get_mut(&chain_id) {
            chain.add_rule(rule);
            self.stats.total_rules += 1;
            true
        } else {
            false
        }
    }

    pub fn process_hook(&mut self, hook: NfHook, fields: &[(NfMatchType, u64)], pkt_bytes: u64) -> NfVerdict {
        let chain_ids: Vec<u32> = self.chains.iter()
            .filter(|(_, c)| c.hook == hook)
            .map(|(&id, _)| id)
            .collect();
        for cid in chain_ids {
            if let Some(chain) = self.chains.get_mut(&cid) {
                match chain.evaluate_packet(fields, pkt_bytes) {
                    NfVerdict::Drop => {
                        self.stats.packets_dropped += 1;
                        return NfVerdict::Drop;
                    }
                    NfVerdict::Accept => {}
                    other => return other,
                }
            }
        }
        self.stats.packets_accepted += 1;
        NfVerdict::Accept
    }

    pub fn drop_rate(&self) -> f64 {
        let total = self.stats.packets_accepted + self.stats.packets_dropped;
        if total == 0 {
            return 0.0;
        }
        self.stats.packets_dropped as f64 / total as f64
    }
}
