//! # Coop ID Generator
//!
//! Distributed ID generation with snowflake-style IDs:
//! - 64-bit structured IDs (timestamp + node + sequence)
//! - Clock drift detection and correction
//! - Node ID management for distributed uniqueness
//! - Batch ID pre-allocation
//! - Monotonicity guarantees
//! - Multiple ID namespaces

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// ID format specification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdFormat {
    /// Snowflake: 41 bits timestamp + 10 bits node + 12 bits sequence
    Snowflake,
    /// Monotonic: 48 bits counter + 16 bits node
    Monotonic,
    /// Time-sorted: 48 bits timestamp + 16 bits random
    TimeSorted,
    /// Compact: 32 bits timestamp + 16 bits node + 16 bits sequence
    Compact,
}

/// Snowflake-style ID layout
#[derive(Debug, Clone, Copy)]
pub struct SnowflakeId {
    pub raw: u64,
}

impl SnowflakeId {
    const TIMESTAMP_BITS: u32 = 41;
    const NODE_BITS: u32 = 10;
    const SEQUENCE_BITS: u32 = 12;
    const NODE_SHIFT: u32 = Self::SEQUENCE_BITS;
    const TIMESTAMP_SHIFT: u32 = Self::NODE_BITS + Self::SEQUENCE_BITS;
    const SEQUENCE_MASK: u64 = (1 << Self::SEQUENCE_BITS) - 1;
    const NODE_MASK: u64 = (1 << Self::NODE_BITS) - 1;

    #[inline]
    pub fn compose(timestamp_ms: u64, node_id: u16, sequence: u16) -> Self {
        let raw = ((timestamp_ms & ((1 << Self::TIMESTAMP_BITS) - 1)) << Self::TIMESTAMP_SHIFT)
            | (((node_id as u64) & Self::NODE_MASK) << Self::NODE_SHIFT)
            | ((sequence as u64) & Self::SEQUENCE_MASK);
        Self { raw }
    }

    #[inline(always)]
    pub fn timestamp_ms(&self) -> u64 {
        self.raw >> Self::TIMESTAMP_SHIFT
    }
    #[inline(always)]
    pub fn node_id(&self) -> u16 {
        ((self.raw >> Self::NODE_SHIFT) & Self::NODE_MASK) as u16
    }
    #[inline(always)]
    pub fn sequence(&self) -> u16 {
        (self.raw & Self::SEQUENCE_MASK) as u16
    }
}

/// Per-namespace generator state
#[derive(Debug, Clone)]
pub struct NamespaceGenerator {
    pub namespace: u32,
    pub format: IdFormat,
    pub node_id: u16,
    pub last_timestamp_ms: u64,
    pub sequence: u16,
    pub total_generated: u64,
    pub clock_drift_events: u64,
    pub max_drift_ms: u64,
    pub epoch_ms: u64,
}

impl NamespaceGenerator {
    pub fn new(namespace: u32, format: IdFormat, node_id: u16, epoch_ms: u64) -> Self {
        Self {
            namespace,
            format,
            node_id,
            last_timestamp_ms: 0,
            sequence: 0,
            total_generated: 0,
            clock_drift_events: 0,
            max_drift_ms: 0,
            epoch_ms,
        }
    }

    pub fn next_id(&mut self, current_ms: u64) -> Option<u64> {
        let ts = current_ms.saturating_sub(self.epoch_ms);

        match self.format {
            IdFormat::Snowflake => {
                if ts < self.last_timestamp_ms {
                    let drift = self.last_timestamp_ms - ts;
                    self.clock_drift_events += 1;
                    if drift > self.max_drift_ms {
                        self.max_drift_ms = drift;
                    }
                    if drift > 5 {
                        return None;
                    } // reject if drift > 5ms
                    // tolerate small drift, use last timestamp
                    self.sequence += 1;
                    if self.sequence >= 4096 {
                        return None;
                    }
                } else if ts == self.last_timestamp_ms {
                    self.sequence += 1;
                    if self.sequence >= 4096 {
                        return None;
                    }
                } else {
                    self.last_timestamp_ms = ts;
                    self.sequence = 0;
                }
                self.total_generated += 1;
                Some(SnowflakeId::compose(self.last_timestamp_ms, self.node_id, self.sequence).raw)
            },
            IdFormat::Monotonic => {
                self.total_generated += 1;
                let counter = self.total_generated;
                Some((counter << 16) | (self.node_id as u64))
            },
            IdFormat::TimeSorted => {
                self.sequence = self.sequence.wrapping_add(1);
                self.total_generated += 1;
                // Use XOR of timestamp and sequence for pseudo-random suffix
                let rand_part =
                    (ts ^ (self.sequence as u64).wrapping_mul(0x9e3779b97f4a7c15)) & 0xFFFF;
                Some((ts << 16) | rand_part)
            },
            IdFormat::Compact => {
                let ts32 = (ts & 0xFFFF_FFFF) as u32;
                if ts == self.last_timestamp_ms {
                    self.sequence += 1;
                    if self.sequence >= 65535 {
                        return None;
                    }
                } else {
                    self.last_timestamp_ms = ts;
                    self.sequence = 0;
                }
                self.total_generated += 1;
                Some(((ts32 as u64) << 32) | ((self.node_id as u64) << 16) | (self.sequence as u64))
            },
        }
    }

    #[inline]
    pub fn next_batch(&mut self, count: usize, current_ms: u64) -> Vec<u64> {
        let mut ids = Vec::with_capacity(count);
        for _ in 0..count {
            match self.next_id(current_ms) {
                Some(id) => ids.push(id),
                None => break,
            }
        }
        ids
    }
}

/// Node registration
#[derive(Debug, Clone)]
pub struct NodeRegistration {
    pub node_id: u16,
    pub registered_ts: u64,
    pub last_seen_ts: u64,
    pub ids_generated: u64,
}

/// ID gen stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct IdGenStats {
    pub total_namespaces: usize,
    pub total_nodes: usize,
    pub total_generated: u64,
    pub total_drift_events: u64,
    pub max_drift_ms: u64,
}

/// Cooperative ID generator
pub struct CoopIdGen {
    namespaces: BTreeMap<u32, NamespaceGenerator>,
    nodes: BTreeMap<u16, NodeRegistration>,
    default_epoch_ms: u64,
    stats: IdGenStats,
}

impl CoopIdGen {
    pub fn new(epoch_ms: u64) -> Self {
        Self {
            namespaces: BTreeMap::new(),
            nodes: BTreeMap::new(),
            default_epoch_ms: epoch_ms,
            stats: IdGenStats::default(),
        }
    }

    #[inline]
    pub fn register_node(&mut self, node_id: u16, ts: u64) {
        self.nodes.insert(node_id, NodeRegistration {
            node_id,
            registered_ts: ts,
            last_seen_ts: ts,
            ids_generated: 0,
        });
    }

    #[inline(always)]
    pub fn create_namespace(&mut self, ns: u32, format: IdFormat, node_id: u16) {
        self.namespaces.insert(
            ns,
            NamespaceGenerator::new(ns, format, node_id, self.default_epoch_ms),
        );
    }

    #[inline]
    pub fn generate(&mut self, ns: u32, current_ms: u64) -> Option<u64> {
        let generator = self.namespaces.get_mut(&ns)?;
        let node_id = generator.node_id;
        let id = generator.next_id(current_ms)?;
        if let Some(n) = self.nodes.get_mut(&node_id) {
            n.ids_generated += 1;
            n.last_seen_ts = current_ms;
        }
        Some(id)
    }

    #[inline]
    pub fn generate_batch(&mut self, ns: u32, count: usize, current_ms: u64) -> Vec<u64> {
        if let Some(generator) = self.namespaces.get_mut(&ns) {
            generator.next_batch(count, current_ms)
        } else {
            Vec::new()
        }
    }

    #[inline(always)]
    pub fn parse_snowflake(raw: u64) -> SnowflakeId {
        SnowflakeId { raw }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_namespaces = self.namespaces.len();
        self.stats.total_nodes = self.nodes.len();
        self.stats.total_generated = self.namespaces.values().map(|n| n.total_generated).sum();
        self.stats.total_drift_events =
            self.namespaces.values().map(|n| n.clock_drift_events).sum();
        self.stats.max_drift_ms = self
            .namespaces
            .values()
            .map(|n| n.max_drift_ms)
            .max()
            .unwrap_or(0);
    }

    #[inline(always)]
    pub fn stats(&self) -> &IdGenStats {
        &self.stats
    }
}
