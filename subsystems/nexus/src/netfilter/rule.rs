//! Rule Definition
//!
//! Netfilter rules and match conditions.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{ChainId, ConnState, NetworkAddr, PortRange, Protocol, RuleId, Verdict};

/// Match condition
#[derive(Debug, Clone)]
pub enum MatchCondition {
    /// Source address
    SrcAddr(NetworkAddr),
    /// Destination address
    DstAddr(NetworkAddr),
    /// Source port (TCP/UDP)
    SrcPort(PortRange),
    /// Destination port (TCP/UDP)
    DstPort(PortRange),
    /// Protocol
    Proto(Protocol),
    /// Input interface
    InInterface(String),
    /// Output interface
    OutInterface(String),
    /// TCP flags
    TcpFlags { mask: u8, value: u8 },
    /// ICMP type
    IcmpType(u8),
    /// Connection state
    ConnState(Vec<ConnState>),
    /// Mark value
    Mark { value: u32, mask: u32 },
    /// Packet length
    Length { min: u16, max: u16 },
    /// TTL/hop limit
    Ttl(u8),
    /// Rate limit (packets per second)
    RateLimit { rate: u32, burst: u32 },
    /// Negated condition
    Not(Box<MatchCondition>),
}

/// Rule definition
#[derive(Debug)]
pub struct RuleDef {
    /// Rule ID
    pub id: RuleId,
    /// Parent chain
    pub chain_id: ChainId,
    /// Position in chain
    pub position: u32,
    /// Match conditions
    pub matches: Vec<MatchCondition>,
    /// Target verdict
    pub verdict: Verdict,
    /// Comment
    pub comment: Option<String>,
    /// Enabled
    pub enabled: bool,
    /// Hit counter
    pub hits: AtomicU64,
    /// Byte counter
    pub bytes: AtomicU64,
    /// Created timestamp
    pub created_at: u64,
    /// Last hit timestamp
    pub last_hit: AtomicU64,
}

impl Clone for RuleDef {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            chain_id: self.chain_id,
            position: self.position,
            matches: self.matches.clone(),
            verdict: self.verdict,
            comment: self.comment.clone(),
            enabled: self.enabled,
            hits: AtomicU64::new(self.hits.load(Ordering::Relaxed)),
            bytes: AtomicU64::new(self.bytes.load(Ordering::Relaxed)),
            created_at: self.created_at,
            last_hit: AtomicU64::new(self.last_hit.load(Ordering::Relaxed)),
        }
    }
}

impl RuleDef {
    /// Create new rule
    pub fn new(id: RuleId, chain_id: ChainId, position: u32, verdict: Verdict, timestamp: u64) -> Self {
        Self {
            id,
            chain_id,
            position,
            matches: Vec::new(),
            verdict,
            comment: None,
            enabled: true,
            hits: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
            created_at: timestamp,
            last_hit: AtomicU64::new(0),
        }
    }

    /// Add match condition
    #[inline(always)]
    pub fn add_match(&mut self, condition: MatchCondition) {
        self.matches.push(condition);
    }

    /// Record hit
    #[inline]
    pub fn hit(&self, packet_size: u64, timestamp: u64) {
        self.hits.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(packet_size, Ordering::Relaxed);
        self.last_hit.store(timestamp, Ordering::Relaxed);
    }

    /// Get hit count
    #[inline(always)]
    pub fn hit_count(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Get byte count
    #[inline(always)]
    pub fn byte_count(&self) -> u64 {
        self.bytes.load(Ordering::Relaxed)
    }

    /// Get last hit
    #[inline(always)]
    pub fn get_last_hit(&self) -> u64 {
        self.last_hit.load(Ordering::Relaxed)
    }
}
