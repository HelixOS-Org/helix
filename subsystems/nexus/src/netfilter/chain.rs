//! Chain and Table Definitions
//!
//! Netfilter chains and tables.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{AddressFamily, ChainId, HookType, RuleId, TableId};

/// Chain type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainType {
    /// Filter chain
    Filter,
    /// NAT chain
    Nat,
    /// Route chain
    Route,
}

/// Chain policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainPolicy {
    /// Accept by default
    Accept,
    /// Drop by default
    Drop,
}

/// Chain definition
#[derive(Debug)]
pub struct ChainDef {
    /// Chain ID
    pub id: ChainId,
    /// Parent table
    pub table_id: TableId,
    /// Chain name
    pub name: String,
    /// Chain type
    pub chain_type: ChainType,
    /// Hook (if base chain)
    pub hook: Option<HookType>,
    /// Priority
    pub priority: i32,
    /// Policy (base chains only)
    pub policy: Option<ChainPolicy>,
    /// Rules in chain
    pub rules: Vec<RuleId>,
    /// Packets processed
    pub packets: AtomicU64,
    /// Bytes processed
    pub bytes: AtomicU64,
}

impl Clone for ChainDef {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            table_id: self.table_id,
            name: self.name.clone(),
            chain_type: self.chain_type,
            hook: self.hook,
            priority: self.priority,
            policy: self.policy,
            rules: self.rules.clone(),
            packets: AtomicU64::new(self.packets.load(Ordering::Relaxed)),
            bytes: AtomicU64::new(self.bytes.load(Ordering::Relaxed)),
        }
    }
}

impl ChainDef {
    /// Create new chain
    pub fn new(id: ChainId, table_id: TableId, name: String, chain_type: ChainType) -> Self {
        Self {
            id,
            table_id,
            name,
            chain_type,
            hook: None,
            priority: 0,
            policy: None,
            rules: Vec::new(),
            packets: AtomicU64::new(0),
            bytes: AtomicU64::new(0),
        }
    }

    /// Is base chain (attached to hook)
    #[inline(always)]
    pub fn is_base_chain(&self) -> bool {
        self.hook.is_some()
    }

    /// Record packet
    #[inline(always)]
    pub fn record_packet(&self, size: u64) {
        self.packets.fetch_add(1, Ordering::Relaxed);
        self.bytes.fetch_add(size, Ordering::Relaxed);
    }

    /// Get packet count
    #[inline(always)]
    pub fn packet_count(&self) -> u64 {
        self.packets.load(Ordering::Relaxed)
    }
}

/// Table definition
#[derive(Debug, Clone)]
pub struct TableDef {
    /// Table ID
    pub id: TableId,
    /// Table name
    pub name: String,
    /// Address family
    pub family: AddressFamily,
    /// Chains in table
    pub chains: Vec<ChainId>,
    /// Created timestamp
    pub created_at: u64,
}

impl TableDef {
    /// Create new table
    pub fn new(id: TableId, name: String, family: AddressFamily, timestamp: u64) -> Self {
        Self {
            id,
            name,
            family,
            chains: Vec::new(),
            created_at: timestamp,
        }
    }
}
