//! # Gossip Protocol
//!
//! Year 3 EVOLUTION - Q4 - Epidemic gossip for state dissemination

#![allow(dead_code)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::NodeId;

// ============================================================================
// GOSSIP TYPES
// ============================================================================

/// Gossip message ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GossipId(pub u64);

static GOSSIP_COUNTER: AtomicU64 = AtomicU64::new(1);

impl GossipId {
    pub fn generate() -> Self {
        Self(GOSSIP_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Gossip entry
#[derive(Debug, Clone)]
pub struct GossipEntry {
    /// Key
    pub key: String,
    /// Value
    pub value: Vec<u8>,
    /// Version
    pub version: u64,
    /// Origin node
    pub origin: NodeId,
    /// Timestamp
    pub timestamp: u64,
    /// TTL (remaining hops)
    pub ttl: u8,
}

/// Gossip message
#[derive(Debug, Clone)]
pub struct GossipMessage {
    /// Message ID
    pub id: GossipId,
    /// Sender
    pub sender: NodeId,
    /// Entries
    pub entries: Vec<GossipEntry>,
    /// Seen nodes (to prevent loops)
    pub seen: Vec<NodeId>,
    /// Digest (for efficient sync)
    pub digest: Option<GossipDigest>,
}

/// Gossip digest (summary for efficient comparison)
#[derive(Debug, Clone)]
pub struct GossipDigest {
    /// Key to (origin, version)
    pub entries: Vec<(String, NodeId, u64)>,
}

/// Gossip mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GossipMode {
    /// Push (send updates)
    Push,
    /// Pull (request updates)
    Pull,
    /// Push-pull (bidirectional)
    PushPull,
}

// ============================================================================
// GOSSIP STATE
// ============================================================================

/// Local gossip state
pub struct GossipState {
    /// Local entries
    entries: BTreeMap<String, GossipEntry>,
    /// Seen message IDs (to prevent duplicates)
    seen_messages: Vec<GossipId>,
    /// Max seen messages to track
    max_seen: usize,
    /// Node ID
    node_id: NodeId,
}

impl GossipState {
    /// Create new state
    pub fn new(node_id: NodeId) -> Self {
        Self {
            entries: BTreeMap::new(),
            seen_messages: Vec::new(),
            max_seen: 1000,
            node_id,
        }
    }

    /// Set local value
    pub fn set(&mut self, key: String, value: Vec<u8>) {
        let entry = self
            .entries
            .entry(key.clone())
            .or_insert_with(|| GossipEntry {
                key: key.clone(),
                value: Vec::new(),
                version: 0,
                origin: self.node_id,
                timestamp: 0,
                ttl: 10,
            });

        entry.value = value;
        entry.version += 1;
        entry.origin = self.node_id;
    }

    /// Get value
    pub fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.entries.get(key).map(|e| &e.value)
    }

    /// Merge received entries
    pub fn merge(&mut self, entries: Vec<GossipEntry>) -> Vec<String> {
        let mut updated = Vec::new();

        for entry in entries {
            match self.entries.get(&entry.key) {
                Some(existing) => {
                    // Update if newer version
                    if entry.version > existing.version {
                        updated.push(entry.key.clone());
                        self.entries.insert(entry.key.clone(), entry);
                    }
                },
                None => {
                    updated.push(entry.key.clone());
                    self.entries.insert(entry.key.clone(), entry);
                },
            }
        }

        updated
    }

    /// Check if message already seen
    pub fn is_seen(&self, id: GossipId) -> bool {
        self.seen_messages.contains(&id)
    }

    /// Mark message as seen
    pub fn mark_seen(&mut self, id: GossipId) {
        if !self.seen_messages.contains(&id) {
            self.seen_messages.push(id);
            if self.seen_messages.len() > self.max_seen {
                self.seen_messages.remove(0);
            }
        }
    }

    /// Get digest
    pub fn digest(&self) -> GossipDigest {
        GossipDigest {
            entries: self
                .entries
                .values()
                .map(|e| (e.key.clone(), e.origin, e.version))
                .collect(),
        }
    }

    /// Get entries newer than digest
    pub fn delta(&self, digest: &GossipDigest) -> Vec<GossipEntry> {
        let digest_map: BTreeMap<&str, (NodeId, u64)> = digest
            .entries
            .iter()
            .map(|(k, o, v)| (k.as_str(), (*o, *v)))
            .collect();

        self.entries
            .values()
            .filter(|e| match digest_map.get(e.key.as_str()) {
                Some((_, version)) => e.version > *version,
                None => true,
            })
            .cloned()
            .collect()
    }

    /// Get all entries
    pub fn all_entries(&self) -> Vec<GossipEntry> {
        self.entries.values().cloned().collect()
    }
}

// ============================================================================
// PEER SELECTION
// ============================================================================

/// Peer selector
pub trait PeerSelector: Send + Sync {
    /// Select peers for gossip
    fn select(&self, from: NodeId, peers: &[NodeId], count: usize) -> Vec<NodeId>;
}

/// Random peer selector
pub struct RandomSelector {
    state: AtomicU64,
}

impl RandomSelector {
    pub fn new() -> Self {
        Self {
            state: AtomicU64::new(0xfedcba9876543210),
        }
    }

    fn random(&self, max: usize) -> usize {
        let mut x = self.state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state.store(x, Ordering::Relaxed);
        (x as usize) % max
    }
}

impl Default for RandomSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerSelector for RandomSelector {
    fn select(&self, from: NodeId, peers: &[NodeId], count: usize) -> Vec<NodeId> {
        let mut available: Vec<_> = peers.iter().filter(|&&p| p != from).copied().collect();

        let mut selected = Vec::new();
        while selected.len() < count && !available.is_empty() {
            let idx = self.random(available.len());
            selected.push(available.remove(idx));
        }

        selected
    }
}

/// Weighted peer selector (based on staleness)
pub struct WeightedSelector {
    /// Last gossip time per peer
    last_gossip: BTreeMap<NodeId, u64>,
    /// Random state
    state: AtomicU64,
}

impl WeightedSelector {
    pub fn new() -> Self {
        Self {
            last_gossip: BTreeMap::new(),
            state: AtomicU64::new(0x1234567890abcdef),
        }
    }

    pub fn record_gossip(&mut self, peer: NodeId, time: u64) {
        self.last_gossip.insert(peer, time);
    }
}

impl Default for WeightedSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl PeerSelector for WeightedSelector {
    fn select(&self, from: NodeId, peers: &[NodeId], count: usize) -> Vec<NodeId> {
        // Sort by staleness (oldest first)
        let mut peers_with_time: Vec<_> = peers
            .iter()
            .filter(|&&p| p != from)
            .map(|&p| (p, self.last_gossip.get(&p).copied().unwrap_or(0)))
            .collect();

        peers_with_time.sort_by_key(|(_, time)| *time);

        peers_with_time
            .iter()
            .take(count)
            .map(|(p, _)| *p)
            .collect()
    }
}

// ============================================================================
// GOSSIP ENGINE
// ============================================================================

/// Gossip engine
pub struct GossipEngine {
    /// Local node ID
    node_id: NodeId,
    /// State
    state: GossipState,
    /// Known peers
    peers: Vec<NodeId>,
    /// Peer selector
    selector: Box<dyn PeerSelector>,
    /// Configuration
    config: GossipConfig,
    /// Statistics
    stats: GossipStats,
}

/// Gossip configuration
#[derive(Debug, Clone)]
pub struct GossipConfig {
    /// Gossip interval (ms)
    pub interval: u64,
    /// Fanout (number of peers per round)
    pub fanout: usize,
    /// Default TTL
    pub default_ttl: u8,
    /// Mode
    pub mode: GossipMode,
    /// Max entries per message
    pub max_entries: usize,
}

impl Default for GossipConfig {
    fn default() -> Self {
        Self {
            interval: 1000,
            fanout: 3,
            default_ttl: 10,
            mode: GossipMode::PushPull,
            max_entries: 100,
        }
    }
}

/// Gossip statistics
#[derive(Debug, Clone, Default)]
pub struct GossipStats {
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Entries updated
    pub entries_updated: u64,
    /// Duplicates ignored
    pub duplicates: u64,
    /// Rounds completed
    pub rounds: u64,
}

impl GossipEngine {
    /// Create new gossip engine
    pub fn new(node_id: NodeId, config: GossipConfig) -> Self {
        Self {
            node_id,
            state: GossipState::new(node_id),
            peers: Vec::new(),
            selector: Box::new(RandomSelector::new()),
            config,
            stats: GossipStats::default(),
        }
    }

    /// Add peer
    pub fn add_peer(&mut self, peer: NodeId) {
        if !self.peers.contains(&peer) && peer != self.node_id {
            self.peers.push(peer);
        }
    }

    /// Remove peer
    pub fn remove_peer(&mut self, peer: NodeId) {
        self.peers.retain(|&p| p != peer);
    }

    /// Set value
    pub fn set(&mut self, key: impl Into<String>, value: Vec<u8>) {
        self.state.set(key.into(), value);
    }

    /// Get value
    pub fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.state.get(key)
    }

    /// Create gossip message for a round
    pub fn create_message(&mut self) -> GossipMessage {
        let entries = match self.config.mode {
            GossipMode::Push | GossipMode::PushPull => {
                let mut entries = self.state.all_entries();
                entries.truncate(self.config.max_entries);
                entries
            },
            GossipMode::Pull => Vec::new(),
        };

        let digest =
            if self.config.mode == GossipMode::Pull || self.config.mode == GossipMode::PushPull {
                Some(self.state.digest())
            } else {
                None
            };

        GossipMessage {
            id: GossipId::generate(),
            sender: self.node_id,
            entries,
            seen: vec![self.node_id],
            digest,
        }
    }

    /// Select targets for gossip
    pub fn select_targets(&self) -> Vec<NodeId> {
        self.selector
            .select(self.node_id, &self.peers, self.config.fanout)
    }

    /// Handle received message
    pub fn receive(&mut self, message: GossipMessage) -> Option<GossipMessage> {
        // Check if already seen
        if self.state.is_seen(message.id) {
            self.stats.duplicates += 1;
            return None;
        }

        self.state.mark_seen(message.id);
        self.stats.messages_received += 1;

        // Merge entries
        if !message.entries.is_empty() {
            let updated = self.state.merge(message.entries);
            self.stats.entries_updated += updated.len() as u64;
        }

        // Create response if pull mode
        match self.config.mode {
            GossipMode::Pull | GossipMode::PushPull => {
                if let Some(digest) = &message.digest {
                    let delta = self.state.delta(digest);
                    if !delta.is_empty() {
                        return Some(GossipMessage {
                            id: GossipId::generate(),
                            sender: self.node_id,
                            entries: delta,
                            seen: vec![self.node_id, message.sender],
                            digest: None,
                        });
                    }
                }
            },
            _ => {},
        }

        None
    }

    /// Run one gossip round
    pub fn gossip_round(&mut self) -> (Vec<NodeId>, GossipMessage) {
        let targets = self.select_targets();
        let message = self.create_message();

        self.stats.messages_sent += targets.len() as u64;
        self.stats.rounds += 1;

        (targets, message)
    }

    /// Forward message (for epidemic spread)
    pub fn forward(&mut self, mut message: GossipMessage) -> Option<(Vec<NodeId>, GossipMessage)> {
        // Decrement TTL
        for entry in &mut message.entries {
            if entry.ttl == 0 {
                return None;
            }
            entry.ttl -= 1;
        }

        // Don't forward to nodes that have seen it
        let targets: Vec<NodeId> = self
            .peers
            .iter()
            .filter(|p| !message.seen.contains(p))
            .copied()
            .take(self.config.fanout)
            .collect();

        if targets.is_empty() {
            return None;
        }

        message.seen.push(self.node_id);

        Some((targets, message))
    }

    /// Set peer selector
    pub fn set_selector(&mut self, selector: Box<dyn PeerSelector>) {
        self.selector = selector;
    }

    /// Get statistics
    pub fn stats(&self) -> &GossipStats {
        &self.stats
    }

    /// Get peers
    pub fn peers(&self) -> &[NodeId] {
        &self.peers
    }
}

impl Default for GossipEngine {
    fn default() -> Self {
        Self::new(NodeId(0), GossipConfig::default())
    }
}

// ============================================================================
// CRDT-BASED GOSSIP
// ============================================================================

/// G-Counter (grow-only counter)
#[derive(Debug, Clone, Default)]
pub struct GCounter {
    /// Per-node counts
    counts: BTreeMap<NodeId, u64>,
}

impl GCounter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn increment(&mut self, node: NodeId) {
        *self.counts.entry(node).or_insert(0) += 1;
    }

    pub fn value(&self) -> u64 {
        self.counts.values().sum()
    }

    pub fn merge(&mut self, other: &GCounter) {
        for (&node, &count) in &other.counts {
            let entry = self.counts.entry(node).or_insert(0);
            *entry = (*entry).max(count);
        }
    }
}

/// LWW-Register (last-writer-wins)
#[derive(Debug, Clone)]
pub struct LWWRegister<T> {
    /// Value
    value: T,
    /// Timestamp
    timestamp: u64,
    /// Writer
    writer: NodeId,
}

impl<T: Clone + Default> LWWRegister<T> {
    pub fn new(value: T, node: NodeId, timestamp: u64) -> Self {
        Self {
            value,
            timestamp,
            writer: node,
        }
    }

    pub fn set(&mut self, value: T, node: NodeId, timestamp: u64) {
        if timestamp > self.timestamp || (timestamp == self.timestamp && node.0 > self.writer.0) {
            self.value = value;
            self.timestamp = timestamp;
            self.writer = node;
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn merge(&mut self, other: &LWWRegister<T>) {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.writer.0 > self.writer.0)
        {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.writer = other.writer;
        }
    }
}

impl<T: Default> Default for LWWRegister<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            timestamp: 0,
            writer: NodeId(0),
        }
    }
}

/// OR-Set (observed-remove set)
#[derive(Debug, Clone, Default)]
pub struct ORSet<T: Clone + Ord> {
    /// Elements with unique tags
    elements: BTreeMap<T, Vec<(NodeId, u64)>>,
    /// Removed tags
    tombstones: BTreeMap<T, Vec<(NodeId, u64)>>,
}

impl<T: Clone + Ord> ORSet<T> {
    pub fn new() -> Self {
        Self {
            elements: BTreeMap::new(),
            tombstones: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, element: T, node: NodeId, timestamp: u64) {
        self.elements
            .entry(element)
            .or_default()
            .push((node, timestamp));
    }

    pub fn remove(&mut self, element: &T) {
        if let Some(tags) = self.elements.remove(element) {
            self.tombstones
                .entry(element.clone())
                .or_default()
                .extend(tags);
        }
    }

    pub fn contains(&self, element: &T) -> bool {
        self.elements.contains_key(element)
    }

    pub fn elements(&self) -> Vec<&T> {
        self.elements.keys().collect()
    }

    pub fn merge(&mut self, other: &ORSet<T>) {
        // Merge elements
        for (elem, tags) in &other.elements {
            let entry = self.elements.entry(elem.clone()).or_default();
            for tag in tags {
                if !entry.contains(tag) {
                    entry.push(*tag);
                }
            }
        }

        // Merge tombstones
        for (elem, tags) in &other.tombstones {
            let entry = self.tombstones.entry(elem.clone()).or_default();
            for tag in tags {
                if !entry.contains(tag) {
                    entry.push(*tag);
                }
            }
        }

        // Remove tombstoned elements
        for (elem, tombstone_tags) in &self.tombstones {
            if let Some(element_tags) = self.elements.get_mut(elem) {
                element_tags.retain(|tag| !tombstone_tags.contains(tag));
                if element_tags.is_empty() {
                    self.elements.remove(elem);
                }
            }
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gossip_state() {
        let mut state = GossipState::new(NodeId(1));

        state.set(String::from("key1"), vec![1, 2, 3]);
        assert_eq!(state.get("key1"), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_gossip_merge() {
        let mut state = GossipState::new(NodeId(1));
        state.set(String::from("key1"), vec![1, 2, 3]);

        let entries = vec![GossipEntry {
            key: String::from("key2"),
            value: vec![4, 5, 6],
            version: 1,
            origin: NodeId(2),
            timestamp: 0,
            ttl: 10,
        }];

        let updated = state.merge(entries);
        assert_eq!(updated, vec!["key2"]);
        assert_eq!(state.get("key2"), Some(&vec![4, 5, 6]));
    }

    #[test]
    fn test_gossip_engine() {
        let mut engine = GossipEngine::new(NodeId(1), GossipConfig::default());

        engine.add_peer(NodeId(2));
        engine.add_peer(NodeId(3));

        engine.set("key1", vec![1, 2, 3]);

        let (targets, message) = engine.gossip_round();
        assert!(targets.len() <= engine.config.fanout);
        assert!(!message.entries.is_empty());
    }

    #[test]
    fn test_g_counter() {
        let mut c1 = GCounter::new();
        let mut c2 = GCounter::new();

        c1.increment(NodeId(1));
        c1.increment(NodeId(1));
        c2.increment(NodeId(2));

        c1.merge(&c2);
        assert_eq!(c1.value(), 3);
    }

    #[test]
    fn test_or_set() {
        let mut set = ORSet::new();

        set.add("a", NodeId(1), 1);
        set.add("b", NodeId(1), 2);

        assert!(set.contains(&"a"));
        assert!(set.contains(&"b"));

        set.remove(&"a");
        assert!(!set.contains(&"a"));
    }
}
