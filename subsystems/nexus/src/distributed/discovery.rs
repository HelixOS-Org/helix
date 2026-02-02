//! # Node Discovery
//!
//! Year 3 EVOLUTION - Q4 - Automatic discovery of NEXUS nodes

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{ClusterId, NodeAddress, NodeCapabilities, NodeId, NodeInfo, NodeRole, NodeState};

// ============================================================================
// DISCOVERY TYPES
// ============================================================================

/// Discovery method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryMethod {
    /// Static list
    Static,
    /// Multicast/Broadcast
    Multicast,
    /// DNS-based
    DNS,
    /// mDNS/Bonjour
    MDNS,
    /// Gossip-based
    Gossip,
    /// Bootstrap nodes
    Bootstrap,
    /// DHT (Kademlia-like)
    DHT,
}

/// Discovered node
#[derive(Debug, Clone)]
pub struct DiscoveredNode {
    /// Node ID
    pub node_id: NodeId,
    /// Address
    pub address: NodeAddress,
    /// Discovered via
    pub method: DiscoveryMethod,
    /// First seen
    pub first_seen: u64,
    /// Last seen
    pub last_seen: u64,
    /// Verification state
    pub verified: bool,
    /// Capabilities
    pub capabilities: Option<NodeCapabilities>,
    /// Cluster (if known)
    pub cluster: Option<ClusterId>,
}

/// Discovery announcement
#[derive(Debug, Clone)]
pub struct DiscoveryAnnouncement {
    /// Node info
    pub node_info: NodeInfo,
    /// Sequence number
    pub sequence: u64,
    /// TTL
    pub ttl: u8,
    /// Signature
    pub signature: u64,
}

/// Discovery query
#[derive(Debug, Clone)]
pub struct DiscoveryQuery {
    /// Requester
    pub requester: NodeId,
    /// Query type
    pub query_type: QueryType,
    /// Filters
    pub filters: Vec<DiscoveryFilter>,
}

/// Query type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryType {
    /// Find all nodes
    All,
    /// Find by cluster
    ByCluster(ClusterId),
    /// Find by capability
    ByCapability,
    /// Find by role
    ByRole(NodeRole),
    /// Find random
    Random(usize),
}

/// Discovery filter
#[derive(Debug, Clone)]
pub enum DiscoveryFilter {
    /// Capability filter
    HasCapability(String),
    /// State filter
    InState(NodeState),
    /// Role filter
    HasRole(NodeRole),
    /// Cluster filter
    InCluster(ClusterId),
    /// Max latency
    MaxLatency(u32),
}

// ============================================================================
// BOOTSTRAP
// ============================================================================

/// Bootstrap node
#[derive(Debug, Clone)]
pub struct BootstrapNode {
    /// Address
    pub address: NodeAddress,
    /// Priority
    pub priority: u8,
    /// Healthy
    pub healthy: bool,
    /// Last check
    pub last_check: u64,
}

/// Bootstrap configuration
#[derive(Debug, Clone, Default)]
pub struct BootstrapConfig {
    /// Bootstrap nodes
    pub nodes: Vec<BootstrapNode>,
    /// Retry count
    pub retry_count: u32,
    /// Retry delay (ms)
    pub retry_delay: u64,
    /// Timeout (ms)
    pub timeout: u64,
}

/// Bootstrap manager
pub struct BootstrapManager {
    /// Configuration
    config: BootstrapConfig,
    /// Current index
    current: usize,
    /// Attempts
    attempts: u32,
}

impl BootstrapManager {
    /// Create new manager
    pub fn new(config: BootstrapConfig) -> Self {
        Self {
            config,
            current: 0,
            attempts: 0,
        }
    }

    /// Get next bootstrap node
    pub fn next(&mut self) -> Option<&BootstrapNode> {
        if self.config.nodes.is_empty() {
            return None;
        }

        // Sort by priority
        let mut sorted_indices: Vec<usize> = (0..self.config.nodes.len()).collect();
        sorted_indices.sort_by_key(|&i| {
            let node = &self.config.nodes[i];
            (!node.healthy as u8, node.priority)
        });

        // Try next healthy node
        for &idx in &sorted_indices {
            if self.config.nodes[idx].healthy {
                self.current = idx;
                self.attempts += 1;
                return Some(&self.config.nodes[idx]);
            }
        }

        // All unhealthy, try first one anyway
        self.current = sorted_indices[0];
        self.attempts += 1;
        Some(&self.config.nodes[self.current])
    }

    /// Mark current as unhealthy
    pub fn mark_unhealthy(&mut self) {
        if self.current < self.config.nodes.len() {
            self.config.nodes[self.current].healthy = false;
        }
    }

    /// Mark current as healthy
    pub fn mark_healthy(&mut self) {
        if self.current < self.config.nodes.len() {
            self.config.nodes[self.current].healthy = true;
        }
    }

    /// Should retry
    pub fn should_retry(&self) -> bool {
        self.attempts < self.config.retry_count
    }

    /// Reset attempts
    pub fn reset(&mut self) {
        self.attempts = 0;
    }
}

// ============================================================================
// DHT (KADEMLIA-LIKE)
// ============================================================================

/// DHT node entry
#[derive(Debug, Clone)]
pub struct DHTEntry {
    /// Node ID
    pub node_id: NodeId,
    /// Address
    pub address: NodeAddress,
    /// Last seen
    pub last_seen: u64,
    /// RTT (round-trip time in ms)
    pub rtt: u32,
}

/// K-bucket (for Kademlia)
pub struct KBucket {
    /// Entries
    entries: Vec<DHTEntry>,
    /// Max size (k)
    k: usize,
}

impl KBucket {
    /// Create new bucket
    pub fn new(k: usize) -> Self {
        Self {
            entries: Vec::with_capacity(k),
            k,
        }
    }

    /// Add or update entry
    pub fn add(&mut self, entry: DHTEntry) -> bool {
        // Check if exists
        if let Some(idx) = self.entries.iter().position(|e| e.node_id == entry.node_id) {
            // Move to tail (most recently seen)
            let existing = self.entries.remove(idx);
            self.entries.push(DHTEntry {
                last_seen: entry.last_seen,
                rtt: entry.rtt,
                ..existing
            });
            return true;
        }

        // Add new
        if self.entries.len() < self.k {
            self.entries.push(entry);
            true
        } else {
            // Bucket full - in real implementation, ping oldest
            false
        }
    }

    /// Get entries
    pub fn entries(&self) -> &[DHTEntry] {
        &self.entries
    }

    /// Is full
    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.k
    }
}

/// DHT routing table
pub struct DHTRoutingTable {
    /// Local node ID
    local_id: NodeId,
    /// Buckets (one per bit)
    buckets: Vec<KBucket>,
    /// K (bucket size)
    k: usize,
}

impl DHTRoutingTable {
    /// Create new routing table
    pub fn new(local_id: NodeId, k: usize) -> Self {
        let mut buckets = Vec::with_capacity(64);
        for _ in 0..64 {
            buckets.push(KBucket::new(k));
        }

        Self {
            local_id,
            buckets,
            k,
        }
    }

    /// Get bucket index for node
    fn bucket_index(&self, node_id: NodeId) -> usize {
        let distance = self.local_id.0 ^ node_id.0;
        if distance == 0 {
            return 0;
        }
        (63 - distance.leading_zeros()) as usize
    }

    /// Add node
    pub fn add(&mut self, entry: DHTEntry) -> bool {
        let idx = self.bucket_index(entry.node_id);
        self.buckets[idx].add(entry)
    }

    /// Find closest nodes
    pub fn find_closest(&self, target: NodeId, count: usize) -> Vec<&DHTEntry> {
        let mut all_entries: Vec<_> = self.buckets.iter().flat_map(|b| b.entries()).collect();

        // Sort by XOR distance
        all_entries.sort_by_key(|e| e.node_id.0 ^ target.0);

        all_entries.into_iter().take(count).collect()
    }

    /// Get all entries
    pub fn all_entries(&self) -> Vec<&DHTEntry> {
        self.buckets.iter().flat_map(|b| b.entries()).collect()
    }
}

// ============================================================================
// DISCOVERY ENGINE
// ============================================================================

/// Discovery engine
pub struct DiscoveryEngine {
    /// Local node ID
    node_id: NodeId,
    /// Local node info
    local_info: NodeInfo,
    /// Discovered nodes
    discovered: BTreeMap<NodeId, DiscoveredNode>,
    /// Bootstrap manager
    bootstrap: BootstrapManager,
    /// DHT routing table
    dht: DHTRoutingTable,
    /// Enabled methods
    enabled_methods: Vec<DiscoveryMethod>,
    /// Configuration
    config: DiscoveryConfig,
    /// Running
    running: AtomicBool,
    /// Sequence number
    sequence: AtomicU64,
    /// Statistics
    stats: DiscoveryStats,
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Announce interval (ms)
    pub announce_interval: u64,
    /// Discovery interval (ms)
    pub discovery_interval: u64,
    /// Node TTL (ms)
    pub node_ttl: u64,
    /// Max nodes
    pub max_nodes: usize,
    /// Verification required
    pub verify_nodes: bool,
    /// DHT enabled
    pub dht_enabled: bool,
    /// K value for DHT
    pub dht_k: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            announce_interval: 5000,
            discovery_interval: 10000,
            node_ttl: 30000,
            max_nodes: 1000,
            verify_nodes: true,
            dht_enabled: true,
            dht_k: 20,
        }
    }
}

/// Discovery statistics
#[derive(Debug, Clone, Default)]
pub struct DiscoveryStats {
    /// Nodes discovered
    pub nodes_discovered: u64,
    /// Nodes verified
    pub nodes_verified: u64,
    /// Nodes expired
    pub nodes_expired: u64,
    /// Announcements sent
    pub announcements_sent: u64,
    /// Queries sent
    pub queries_sent: u64,
    /// Queries answered
    pub queries_answered: u64,
}

impl DiscoveryEngine {
    /// Create new discovery engine
    pub fn new(node_id: NodeId, local_info: NodeInfo, config: DiscoveryConfig) -> Self {
        Self {
            node_id,
            local_info,
            discovered: BTreeMap::new(),
            bootstrap: BootstrapManager::new(BootstrapConfig::default()),
            dht: DHTRoutingTable::new(node_id, config.dht_k),
            enabled_methods: vec![DiscoveryMethod::Bootstrap, DiscoveryMethod::Gossip],
            config,
            running: AtomicBool::new(false),
            sequence: AtomicU64::new(1),
            stats: DiscoveryStats::default(),
        }
    }

    /// Start discovery
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    /// Stop discovery
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Create announcement
    pub fn create_announcement(&self) -> DiscoveryAnnouncement {
        DiscoveryAnnouncement {
            node_info: self.local_info.clone(),
            sequence: self.sequence.fetch_add(1, Ordering::SeqCst),
            ttl: 10,
            signature: 0,
        }
    }

    /// Handle announcement
    pub fn handle_announcement(&mut self, announcement: DiscoveryAnnouncement) -> bool {
        let node_id = announcement.node_info.id;

        // Don't discover self
        if node_id == self.node_id {
            return false;
        }

        // Check if already discovered
        if let Some(existing) = self.discovered.get_mut(&node_id) {
            existing.last_seen = 0; // Would be current time
            return false;
        }

        // Check capacity
        if self.discovered.len() >= self.config.max_nodes {
            self.prune_expired(0);
            if self.discovered.len() >= self.config.max_nodes {
                return false;
            }
        }

        // Add to discovered
        let discovered = DiscoveredNode {
            node_id,
            address: announcement.node_info.address.clone(),
            method: DiscoveryMethod::Gossip,
            first_seen: 0,
            last_seen: 0,
            verified: !self.config.verify_nodes,
            capabilities: Some(announcement.node_info.capabilities),
            cluster: announcement.node_info.cluster,
        };

        self.discovered.insert(node_id, discovered);

        // Add to DHT
        if self.config.dht_enabled {
            self.dht.add(DHTEntry {
                node_id,
                address: announcement.node_info.address,
                last_seen: 0,
                rtt: 0,
            });
        }

        self.stats.nodes_discovered += 1;
        true
    }

    /// Handle query
    pub fn handle_query(&mut self, query: &DiscoveryQuery) -> Vec<DiscoveredNode> {
        self.stats.queries_answered += 1;

        let nodes: Vec<&DiscoveredNode> = match query.query_type {
            QueryType::All => self.discovered.values().collect(),
            QueryType::ByCluster(cluster_id) => self
                .discovered
                .values()
                .filter(|n| n.cluster == Some(cluster_id))
                .collect(),
            QueryType::ByRole(_role) => {
                // Would need role info
                self.discovered.values().collect()
            },
            QueryType::Random(count) => self.discovered.values().take(count).collect(),
            _ => self.discovered.values().collect(),
        };

        // Apply filters
        nodes
            .into_iter()
            .filter(|n| self.matches_filters(n, &query.filters))
            .cloned()
            .collect()
    }

    fn matches_filters(&self, node: &DiscoveredNode, filters: &[DiscoveryFilter]) -> bool {
        for filter in filters {
            match filter {
                DiscoveryFilter::InCluster(cluster_id) => {
                    if node.cluster != Some(*cluster_id) {
                        return false;
                    }
                },
                DiscoveryFilter::HasCapability(cap) => {
                    if let Some(caps) = &node.capabilities {
                        if !caps.features.contains(cap) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                },
                _ => {},
            }
        }
        true
    }

    /// Find closest nodes (DHT)
    pub fn find_closest(&self, target: NodeId, count: usize) -> Vec<NodeId> {
        self.dht
            .find_closest(target, count)
            .into_iter()
            .map(|e| e.node_id)
            .collect()
    }

    /// Prune expired nodes
    pub fn prune_expired(&mut self, now: u64) {
        let ttl = self.config.node_ttl;
        let expired: Vec<NodeId> = self
            .discovered
            .iter()
            .filter(|(_, n)| now - n.last_seen > ttl)
            .map(|(id, _)| *id)
            .collect();

        for id in expired {
            self.discovered.remove(&id);
            self.stats.nodes_expired += 1;
        }
    }

    /// Mark node as verified
    pub fn verify(&mut self, node_id: NodeId) {
        if let Some(node) = self.discovered.get_mut(&node_id) {
            node.verified = true;
            self.stats.nodes_verified += 1;
        }
    }

    /// Get discovered nodes
    pub fn discovered(&self) -> impl Iterator<Item = &DiscoveredNode> {
        self.discovered.values()
    }

    /// Get verified nodes
    pub fn verified_nodes(&self) -> impl Iterator<Item = &DiscoveredNode> {
        self.discovered.values().filter(|n| n.verified)
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.discovered.len()
    }

    /// Set bootstrap config
    pub fn set_bootstrap(&mut self, config: BootstrapConfig) {
        self.bootstrap = BootstrapManager::new(config);
    }

    /// Enable method
    pub fn enable_method(&mut self, method: DiscoveryMethod) {
        if !self.enabled_methods.contains(&method) {
            self.enabled_methods.push(method);
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &DiscoveryStats {
        &self.stats
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_node_info(id: u64) -> NodeInfo {
        NodeInfo {
            id: NodeId(id),
            name: String::from("test"),
            role: NodeRole::Follower,
            state: NodeState::Active,
            cluster: None,
            capabilities: NodeCapabilities::default(),
            address: NodeAddress {
                host: String::from("localhost"),
                port: 7878,
                public_key_hash: 0,
            },
            metrics: super::super::NodeMetrics::default(),
            last_seen: 0,
        }
    }

    #[test]
    fn test_discovery_engine() {
        let node_info = create_node_info(1);
        let mut engine = DiscoveryEngine::new(NodeId(1), node_info, DiscoveryConfig::default());

        // Discover another node
        let announcement = DiscoveryAnnouncement {
            node_info: create_node_info(2),
            sequence: 1,
            ttl: 10,
            signature: 0,
        };

        let added = engine.handle_announcement(announcement);
        assert!(added);
        assert_eq!(engine.node_count(), 1);
    }

    #[test]
    fn test_dht_routing() {
        let mut table = DHTRoutingTable::new(NodeId(1), 20);

        // Add nodes
        for i in 2..10 {
            table.add(DHTEntry {
                node_id: NodeId(i),
                address: NodeAddress {
                    host: String::from("localhost"),
                    port: 7878,
                    public_key_hash: 0,
                },
                last_seen: 0,
                rtt: 10,
            });
        }

        // Find closest
        let closest = table.find_closest(NodeId(5), 3);
        assert_eq!(closest.len(), 3);
    }

    #[test]
    fn test_k_bucket() {
        let mut bucket = KBucket::new(3);

        for i in 1..=3 {
            bucket.add(DHTEntry {
                node_id: NodeId(i),
                address: NodeAddress {
                    host: String::from("localhost"),
                    port: 7878,
                    public_key_hash: 0,
                },
                last_seen: 0,
                rtt: 10,
            });
        }

        assert!(bucket.is_full());

        // Adding more should fail
        let added = bucket.add(DHTEntry {
            node_id: NodeId(100),
            address: NodeAddress {
                host: String::from("localhost"),
                port: 7878,
                public_key_hash: 0,
            },
            last_seen: 0,
            rtt: 10,
        });

        assert!(!added);
    }

    #[test]
    fn test_bootstrap() {
        let config = BootstrapConfig {
            nodes: vec![
                BootstrapNode {
                    address: NodeAddress {
                        host: String::from("node1"),
                        port: 7878,
                        public_key_hash: 0,
                    },
                    priority: 1,
                    healthy: true,
                    last_check: 0,
                },
                BootstrapNode {
                    address: NodeAddress {
                        host: String::from("node2"),
                        port: 7878,
                        public_key_hash: 0,
                    },
                    priority: 2,
                    healthy: true,
                    last_check: 0,
                },
            ],
            retry_count: 3,
            retry_delay: 1000,
            timeout: 5000,
        };

        let mut bootstrap = BootstrapManager::new(config);

        let node = bootstrap.next();
        assert!(node.is_some());
        assert_eq!(node.unwrap().address.host, "node1"); // Lower priority first
    }
}
