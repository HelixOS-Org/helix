//! Connection Tracking
//!
//! Netfilter connection tracking (conntrack).

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{ConntrackId, NetworkAddr, Protocol};

/// Connection tracking state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConntrackState {
    /// New connection
    New,
    /// Established
    Established,
    /// Related
    Related,
    /// Time wait
    TimeWait,
    /// Close wait
    CloseWait,
    /// Closing
    Closing,
}

/// Connection tuple (5-tuple)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnTuple {
    /// Source address
    pub src_addr: NetworkAddr,
    /// Destination address
    pub dst_addr: NetworkAddr,
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Protocol
    pub protocol: Protocol,
}

impl ConnTuple {
    /// Create new tuple
    pub fn new(
        src_addr: NetworkAddr,
        dst_addr: NetworkAddr,
        src_port: u16,
        dst_port: u16,
        protocol: Protocol,
    ) -> Self {
        Self {
            src_addr,
            dst_addr,
            src_port,
            dst_port,
            protocol,
        }
    }

    /// Get reply tuple (swapped src/dst)
    pub fn reply(&self) -> Self {
        Self {
            src_addr: self.dst_addr,
            dst_addr: self.src_addr,
            src_port: self.dst_port,
            dst_port: self.src_port,
            protocol: self.protocol,
        }
    }
}

/// Connection tracking entry
#[derive(Debug)]
pub struct ConntrackEntry {
    /// Entry ID
    pub id: ConntrackId,
    /// Original tuple
    pub original: ConnTuple,
    /// Reply tuple
    pub reply: ConnTuple,
    /// Connection state
    pub state: ConntrackState,
    /// Packets (original direction)
    pub packets_orig: AtomicU64,
    /// Packets (reply direction)
    pub packets_reply: AtomicU64,
    /// Bytes (original direction)
    pub bytes_orig: AtomicU64,
    /// Bytes (reply direction)
    pub bytes_reply: AtomicU64,
    /// Created timestamp
    pub created_at: u64,
    /// Last seen timestamp
    pub last_seen: AtomicU64,
    /// Timeout
    pub timeout: u64,
    /// Mark
    pub mark: u32,
    /// Is confirmed
    pub confirmed: bool,
    /// Is assured
    pub assured: bool,
}

impl Clone for ConntrackEntry {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            original: self.original.clone(),
            reply: self.reply.clone(),
            state: self.state,
            packets_orig: AtomicU64::new(self.packets_orig.load(Ordering::Relaxed)),
            packets_reply: AtomicU64::new(self.packets_reply.load(Ordering::Relaxed)),
            bytes_orig: AtomicU64::new(self.bytes_orig.load(Ordering::Relaxed)),
            bytes_reply: AtomicU64::new(self.bytes_reply.load(Ordering::Relaxed)),
            created_at: self.created_at,
            last_seen: AtomicU64::new(self.last_seen.load(Ordering::Relaxed)),
            timeout: self.timeout,
            mark: self.mark,
            confirmed: self.confirmed,
            assured: self.assured,
        }
    }
}

impl ConntrackEntry {
    /// Create new entry
    pub fn new(id: ConntrackId, original: ConnTuple, timestamp: u64) -> Self {
        let reply = original.reply();
        Self {
            id,
            original,
            reply,
            state: ConntrackState::New,
            packets_orig: AtomicU64::new(1),
            packets_reply: AtomicU64::new(0),
            bytes_orig: AtomicU64::new(0),
            bytes_reply: AtomicU64::new(0),
            created_at: timestamp,
            last_seen: AtomicU64::new(timestamp),
            timeout: 120_000_000_000, // 120 seconds in ns
            mark: 0,
            confirmed: false,
            assured: false,
        }
    }

    /// Update original direction
    pub fn update_orig(&self, bytes: u64, timestamp: u64) {
        self.packets_orig.fetch_add(1, Ordering::Relaxed);
        self.bytes_orig.fetch_add(bytes, Ordering::Relaxed);
        self.last_seen.store(timestamp, Ordering::Relaxed);
    }

    /// Update reply direction
    pub fn update_reply(&self, bytes: u64, timestamp: u64) {
        self.packets_reply.fetch_add(1, Ordering::Relaxed);
        self.bytes_reply.fetch_add(bytes, Ordering::Relaxed);
        self.last_seen.store(timestamp, Ordering::Relaxed);
    }

    /// Is expired
    pub fn is_expired(&self, now: u64) -> bool {
        now > self.last_seen.load(Ordering::Relaxed) + self.timeout
    }

    /// Total packets
    pub fn total_packets(&self) -> u64 {
        self.packets_orig.load(Ordering::Relaxed) + self.packets_reply.load(Ordering::Relaxed)
    }

    /// Total bytes
    pub fn total_bytes(&self) -> u64 {
        self.bytes_orig.load(Ordering::Relaxed) + self.bytes_reply.load(Ordering::Relaxed)
    }
}

/// Connection tracker
pub struct Conntrack {
    /// Entries
    entries: BTreeMap<ConntrackId, ConntrackEntry>,
    /// By original tuple hash
    by_original: BTreeMap<u64, ConntrackId>,
    /// By reply tuple hash
    by_reply: BTreeMap<u64, ConntrackId>,
    /// Next ID
    next_id: AtomicU64,
    /// Max entries
    pub max_entries: usize,
    /// Total created
    total_created: AtomicU64,
    /// Total destroyed
    total_destroyed: AtomicU64,
}

impl Conntrack {
    /// Create new connection tracker
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            by_original: BTreeMap::new(),
            by_reply: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            max_entries,
            total_created: AtomicU64::new(0),
            total_destroyed: AtomicU64::new(0),
        }
    }

    /// Create or find entry
    pub fn create_or_find(&mut self, tuple: ConnTuple, timestamp: u64) -> ConntrackId {
        let hash = self.hash_tuple(&tuple);

        // Check if exists
        if let Some(&id) = self.by_original.get(&hash) {
            return id;
        }
        if let Some(&id) = self.by_reply.get(&hash) {
            return id;
        }

        // Create new
        let id = ConntrackId::new(self.next_id.fetch_add(1, Ordering::Relaxed));
        let entry = ConntrackEntry::new(id, tuple.clone(), timestamp);

        let reply_hash = self.hash_tuple(&entry.reply);

        self.by_original.insert(hash, id);
        self.by_reply.insert(reply_hash, id);
        self.entries.insert(id, entry);
        self.total_created.fetch_add(1, Ordering::Relaxed);

        id
    }

    /// Get entry
    pub fn get(&self, id: ConntrackId) -> Option<&ConntrackEntry> {
        self.entries.get(&id)
    }

    /// Delete entry
    pub fn delete(&mut self, id: ConntrackId) -> bool {
        if let Some(entry) = self.entries.remove(&id) {
            let orig_hash = self.hash_tuple(&entry.original);
            let reply_hash = self.hash_tuple(&entry.reply);
            self.by_original.remove(&orig_hash);
            self.by_reply.remove(&reply_hash);
            self.total_destroyed.fetch_add(1, Ordering::Relaxed);
            return true;
        }
        false
    }

    /// Hash tuple (simplified)
    fn hash_tuple(&self, tuple: &ConnTuple) -> u64 {
        let mut hash = tuple.src_port as u64;
        hash = hash.wrapping_mul(31).wrapping_add(tuple.dst_port as u64);
        hash = hash.wrapping_mul(31).wrapping_add(tuple.protocol.number() as u64);
        hash
    }

    /// Entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Cleanup expired
    pub fn cleanup_expired(&mut self, now: u64) -> usize {
        let expired: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| e.is_expired(now))
            .map(|(id, _)| *id)
            .collect();

        let count = expired.len();
        for id in expired {
            self.delete(id);
        }
        count
    }
}
