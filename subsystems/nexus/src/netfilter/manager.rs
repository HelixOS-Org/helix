//! Netfilter Manager
//!
//! Core netfilter management.

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    AddressFamily, ChainDef, ChainId, ChainType, Conntrack, NatTable, RuleDef, RuleId, TableDef,
    TableId, Verdict,
};

/// Netfilter manager
pub struct NetfilterManager {
    /// Tables
    tables: BTreeMap<TableId, TableDef>,
    /// Chains
    chains: BTreeMap<ChainId, ChainDef>,
    /// Rules
    rules: BTreeMap<RuleId, RuleDef>,
    /// Connection tracker
    conntrack: Conntrack,
    /// NAT table
    nat: NatTable,
    /// Next IDs
    next_table_id: AtomicU64,
    next_chain_id: AtomicU64,
    next_rule_id: AtomicU64,
    /// Total packets processed
    total_packets: AtomicU64,
    /// Total bytes processed
    total_bytes: AtomicU64,
}

impl NetfilterManager {
    /// Create new netfilter manager
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
            chains: BTreeMap::new(),
            rules: BTreeMap::new(),
            conntrack: Conntrack::new(65536),
            nat: NatTable::new(),
            next_table_id: AtomicU64::new(1),
            next_chain_id: AtomicU64::new(1),
            next_rule_id: AtomicU64::new(1),
            total_packets: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
        }
    }

    /// Create table
    #[inline]
    pub fn create_table(
        &mut self,
        name: String,
        family: AddressFamily,
        timestamp: u64,
    ) -> TableId {
        let id = TableId::new(self.next_table_id.fetch_add(1, Ordering::Relaxed));
        let table = TableDef::new(id, name, family, timestamp);
        self.tables.insert(id, table);
        id
    }

    /// Create chain
    pub fn create_chain(
        &mut self,
        table_id: TableId,
        name: String,
        chain_type: ChainType,
    ) -> Option<ChainId> {
        if !self.tables.contains_key(&table_id) {
            return None;
        }

        let id = ChainId::new(self.next_chain_id.fetch_add(1, Ordering::Relaxed));
        let chain = ChainDef::new(id, table_id, name, chain_type);

        if let Some(table) = self.tables.get_mut(&table_id) {
            table.chains.push(id);
        }

        self.chains.insert(id, chain);
        Some(id)
    }

    /// Add rule
    #[inline]
    pub fn add_rule(
        &mut self,
        chain_id: ChainId,
        verdict: Verdict,
        timestamp: u64,
    ) -> Option<RuleId> {
        let chain = self.chains.get_mut(&chain_id)?;

        let position = chain.rules.len() as u32;
        let id = RuleId::new(self.next_rule_id.fetch_add(1, Ordering::Relaxed));
        let rule = RuleDef::new(id, chain_id, position, verdict, timestamp);

        chain.rules.push(id);
        self.rules.insert(id, rule);
        Some(id)
    }

    /// Get table
    #[inline(always)]
    pub fn get_table(&self, id: TableId) -> Option<&TableDef> {
        self.tables.get(&id)
    }

    /// Get chain
    #[inline(always)]
    pub fn get_chain(&self, id: ChainId) -> Option<&ChainDef> {
        self.chains.get(&id)
    }

    /// Get rule
    #[inline(always)]
    pub fn get_rule(&self, id: RuleId) -> Option<&RuleDef> {
        self.rules.get(&id)
    }

    /// Get rule mutably
    #[inline(always)]
    pub fn get_rule_mut(&mut self, id: RuleId) -> Option<&mut RuleDef> {
        self.rules.get_mut(&id)
    }

    /// Get conntrack
    #[inline(always)]
    pub fn conntrack(&self) -> &Conntrack {
        &self.conntrack
    }

    /// Get conntrack mutably
    #[inline(always)]
    pub fn conntrack_mut(&mut self) -> &mut Conntrack {
        &mut self.conntrack
    }

    /// Get NAT table
    #[inline(always)]
    pub fn nat(&self) -> &NatTable {
        &self.nat
    }

    /// Record packet
    #[inline(always)]
    pub fn record_packet(&self, size: u64) {
        self.total_packets.fetch_add(1, Ordering::Relaxed);
        self.total_bytes.fetch_add(size, Ordering::Relaxed);
    }

    /// Get total packets
    #[inline(always)]
    pub fn total_packets(&self) -> u64 {
        self.total_packets.load(Ordering::Relaxed)
    }

    /// Get total bytes
    #[inline(always)]
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Relaxed)
    }
}

impl Default for NetfilterManager {
    fn default() -> Self {
        Self::new()
    }
}
