// SPDX-License-Identifier: GPL-2.0
//! Holistic queueing discipline — traffic control with HTB, TBF, FQ, and SFQ

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Qdisc type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdiscType {
    Pfifo,
    Bfifo,
    PfifoFast,
    Sfq,
    Tbf,
    Htb,
    Fq,
    FqCodel,
    Cake,
    Mqprio,
    Red,
    Netem,
    Noqueue,
}

/// Qdisc state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QdiscState {
    Active,
    Throttled,
    Dormant,
    Disabled,
}

/// TC class state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcClassState {
    Active,
    Borrowing,
    Capped,
    Idle,
}

/// Token bucket parameters
#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub rate_bps: u64,
    pub burst_bytes: u32,
    pub tokens: u64,
    pub max_tokens: u64,
    pub last_fill_ns: u64,
}

impl TokenBucket {
    pub fn new(rate_bps: u64, burst_bytes: u32) -> Self {
        Self {
            rate_bps,
            burst_bytes,
            tokens: burst_bytes as u64,
            max_tokens: burst_bytes as u64,
            last_fill_ns: 0,
        }
    }

    pub fn refill(&mut self, now_ns: u64) {
        if self.last_fill_ns == 0 {
            self.last_fill_ns = now_ns;
            return;
        }
        let elapsed_ns = now_ns.saturating_sub(self.last_fill_ns);
        let new_tokens = (self.rate_bps * elapsed_ns) / (8 * 1_000_000_000);
        self.tokens = (self.tokens + new_tokens).min(self.max_tokens);
        self.last_fill_ns = now_ns;
    }

    pub fn consume(&mut self, bytes: u64) -> bool {
        if self.tokens >= bytes {
            self.tokens -= bytes;
            true
        } else {
            false
        }
    }

    pub fn fill_ratio(&self) -> f64 {
        if self.max_tokens == 0 {
            return 0.0;
        }
        self.tokens as f64 / self.max_tokens as f64
    }
}

/// FQ-CoDel flow state
#[derive(Debug, Clone)]
pub struct FqCodelFlow {
    pub flow_hash: u64,
    pub queue_depth: u32,
    pub total_packets: u64,
    pub total_bytes: u64,
    pub drops: u64,
    pub ce_marks: u64,
    pub sojourn_us: u64,
    pub target_us: u64,
    pub interval_us: u64,
    pub dropping: bool,
}

impl FqCodelFlow {
    pub fn new(flow_hash: u64) -> Self {
        Self {
            flow_hash,
            queue_depth: 0,
            total_packets: 0,
            total_bytes: 0,
            drops: 0,
            ce_marks: 0,
            sojourn_us: 0,
            target_us: 5000,
            interval_us: 100_000,
            dropping: false,
        }
    }

    pub fn enqueue(&mut self, pkt_bytes: u64) {
        self.queue_depth += 1;
        self.total_packets += 1;
        self.total_bytes += pkt_bytes;
    }

    pub fn dequeue(&mut self) -> bool {
        if self.queue_depth > 0 {
            self.queue_depth -= 1;
            true
        } else {
            false
        }
    }

    pub fn check_codel(&mut self, sojourn_us: u64) -> bool {
        self.sojourn_us = sojourn_us;
        if sojourn_us > self.target_us {
            if !self.dropping {
                self.dropping = true;
            }
            self.drops += 1;
            true
        } else {
            self.dropping = false;
            false
        }
    }
}

/// HTB class
#[derive(Debug, Clone)]
pub struct HtbClass {
    pub class_id: u32,
    pub parent_id: u32,
    pub state: TcClassState,
    pub rate: TokenBucket,
    pub ceil: TokenBucket,
    pub quantum: u32,
    pub prio: u8,
    pub level: u8,
    pub packets: u64,
    pub bytes: u64,
    pub drops: u64,
    pub overlimits: u64,
    pub children: Vec<u32>,
}

impl HtbClass {
    pub fn new(class_id: u32, parent_id: u32, rate_bps: u64, ceil_bps: u64) -> Self {
        Self {
            class_id,
            parent_id,
            state: TcClassState::Active,
            rate: TokenBucket::new(rate_bps, (rate_bps / 8000) as u32),
            ceil: TokenBucket::new(ceil_bps, (ceil_bps / 8000) as u32),
            quantum: 1500,
            prio: 0,
            level: 0,
            packets: 0,
            bytes: 0,
            drops: 0,
            overlimits: 0,
            children: Vec::new(),
        }
    }

    pub fn try_send(&mut self, pkt_bytes: u64, now_ns: u64) -> bool {
        self.rate.refill(now_ns);
        self.ceil.refill(now_ns);
        if self.rate.consume(pkt_bytes) {
            self.packets += 1;
            self.bytes += pkt_bytes;
            self.state = TcClassState::Active;
            true
        } else if self.ceil.consume(pkt_bytes) {
            self.packets += 1;
            self.bytes += pkt_bytes;
            self.state = TcClassState::Borrowing;
            true
        } else {
            self.overlimits += 1;
            self.state = TcClassState::Capped;
            false
        }
    }

    pub fn utilization_pct(&self) -> f64 {
        if self.rate.rate_bps == 0 {
            return 0.0;
        }
        let used_ratio = 1.0 - self.rate.fill_ratio();
        used_ratio * 100.0
    }
}

/// Qdisc stats
#[derive(Debug, Clone)]
pub struct QdiscStats {
    pub total_qdiscs: u64,
    pub total_classes: u64,
    pub total_packets: u64,
    pub total_bytes: u64,
    pub total_drops: u64,
    pub total_overlimits: u64,
}

/// Main holistic qdisc manager
#[derive(Debug)]
pub struct HolisticQdisc {
    pub classes: BTreeMap<u32, HtbClass>,
    pub flows: BTreeMap<u64, FqCodelFlow>,
    pub stats: QdiscStats,
    pub default_class: u32,
    pub qdisc_type: QdiscType,
    pub next_class_id: u32,
}

impl HolisticQdisc {
    pub fn new(qdisc_type: QdiscType) -> Self {
        Self {
            classes: BTreeMap::new(),
            flows: BTreeMap::new(),
            stats: QdiscStats {
                total_qdiscs: 1,
                total_classes: 0,
                total_packets: 0,
                total_bytes: 0,
                total_drops: 0,
                total_overlimits: 0,
            },
            default_class: 0,
            qdisc_type,
            next_class_id: 1,
        }
    }

    pub fn add_class(&mut self, parent_id: u32, rate_bps: u64, ceil_bps: u64) -> u32 {
        let id = self.next_class_id;
        self.next_class_id += 1;
        let class = HtbClass::new(id, parent_id, rate_bps, ceil_bps);
        self.classes.insert(id, class);
        if let Some(parent) = self.classes.get_mut(&parent_id) {
            parent.children.push(id);
        }
        self.stats.total_classes += 1;
        id
    }

    pub fn classify_and_send(&mut self, class_id: u32, flow_hash: u64, pkt_bytes: u64, now_ns: u64) -> bool {
        let cid = if self.classes.contains_key(&class_id) { class_id } else { self.default_class };
        if let Some(class) = self.classes.get_mut(&cid) {
            if class.try_send(pkt_bytes, now_ns) {
                let flow = self.flows.entry(flow_hash).or_insert_with(|| FqCodelFlow::new(flow_hash));
                flow.enqueue(pkt_bytes);
                self.stats.total_packets += 1;
                self.stats.total_bytes += pkt_bytes;
                true
            } else {
                self.stats.total_drops += 1;
                false
            }
        } else {
            self.stats.total_drops += 1;
            false
        }
    }

    pub fn total_bandwidth_bps(&self) -> u64 {
        self.classes.values().map(|c| c.rate.rate_bps).sum()
    }
}
