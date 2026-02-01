//! # Distributed Protocol
//!
//! Year 3 EVOLUTION - Q4 - Communication protocol for distributed evolution

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{NodeId, ClusterId, ImprovementId, Epoch, Term, SessionId, Improvement, NodeInfo};

// ============================================================================
// MESSAGE TYPES
// ============================================================================

/// Message ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(pub u64);

static MESSAGE_COUNTER: AtomicU64 = AtomicU64::new(1);

impl MessageId {
    pub fn generate() -> Self {
        Self(MESSAGE_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Protocol message
#[derive(Debug, Clone)]
pub struct Message {
    /// Message ID
    pub id: MessageId,
    /// Source node
    pub source: NodeId,
    /// Destination node (None for broadcast)
    pub destination: Option<NodeId>,
    /// Message type
    pub message_type: MessageType,
    /// Payload
    pub payload: MessagePayload,
    /// Timestamp
    pub timestamp: u64,
    /// TTL (hops)
    pub ttl: u8,
    /// Signature
    pub signature: u64,
}

/// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Heartbeat
    Heartbeat,
    /// Join request
    JoinRequest,
    /// Join response
    JoinResponse,
    /// Leave notification
    Leave,
    /// Improvement proposal
    ProposeImprovement,
    /// Vote request
    VoteRequest,
    /// Vote response
    VoteResponse,
    /// Sync request
    SyncRequest,
    /// Sync response
    SyncResponse,
    /// Gradient update
    GradientUpdate,
    /// Aggregate request
    AggregateRequest,
    /// Aggregate response
    AggregateResponse,
    /// Consensus prepare
    ConsensusPrepare,
    /// Consensus accept
    ConsensusAccept,
    /// Consensus commit
    ConsensusCommit,
    /// Discovery ping
    DiscoveryPing,
    /// Discovery pong
    DiscoveryPong,
    /// Gossip
    Gossip,
}

/// Message payload
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// Empty
    Empty,
    /// Heartbeat data
    Heartbeat(HeartbeatPayload),
    /// Join data
    Join(JoinPayload),
    /// Join response data
    JoinResponse(JoinResponsePayload),
    /// Improvement
    Improvement(Box<Improvement>),
    /// Vote
    Vote(VotePayload),
    /// Sync
    Sync(SyncPayload),
    /// Gradient
    Gradient(GradientPayload),
    /// Aggregate
    Aggregate(AggregatePayload),
    /// Consensus
    Consensus(ConsensusPayload),
    /// Discovery
    Discovery(DiscoveryPayload),
    /// Gossip
    Gossip(GossipPayload),
    /// Raw bytes
    Raw(Vec<u8>),
}

// ============================================================================
// PAYLOAD TYPES
// ============================================================================

/// Heartbeat payload
#[derive(Debug, Clone)]
pub struct HeartbeatPayload {
    /// Epoch
    pub epoch: Epoch,
    /// Term
    pub term: Term,
    /// Leader ID
    pub leader: Option<NodeId>,
    /// Load (0-100)
    pub load: u8,
    /// Improvements count
    pub improvements_count: u64,
}

/// Join payload
#[derive(Debug, Clone)]
pub struct JoinPayload {
    /// Node info
    pub node_info: NodeInfo,
    /// Requested cluster
    pub cluster_id: Option<ClusterId>,
    /// Token (if required)
    pub token: Option<String>,
}

/// Join response payload
#[derive(Debug, Clone)]
pub struct JoinResponsePayload {
    /// Accepted
    pub accepted: bool,
    /// Cluster ID
    pub cluster_id: ClusterId,
    /// Current epoch
    pub epoch: Epoch,
    /// Current term
    pub term: Term,
    /// Leader
    pub leader: Option<NodeId>,
    /// Members
    pub members: Vec<NodeId>,
    /// Reason (if rejected)
    pub reason: Option<String>,
}

/// Vote payload
#[derive(Debug, Clone)]
pub struct VotePayload {
    /// Improvement ID
    pub improvement_id: ImprovementId,
    /// Decision
    pub approve: bool,
    /// Reason
    pub reason: Option<String>,
}

/// Sync payload
#[derive(Debug, Clone)]
pub struct SyncPayload {
    /// Sync type
    pub sync_type: SyncType,
    /// From epoch
    pub from_epoch: Epoch,
    /// To epoch
    pub to_epoch: Epoch,
    /// Items
    pub items: Vec<SyncItem>,
}

/// Sync type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncType {
    /// Full sync
    Full,
    /// Incremental sync
    Incremental,
    /// Snapshot
    Snapshot,
    /// Improvements only
    Improvements,
    /// State only
    State,
}

/// Sync item
#[derive(Debug, Clone)]
pub struct SyncItem {
    /// Item type
    pub item_type: SyncItemType,
    /// Key
    pub key: u64,
    /// Data
    pub data: Vec<u8>,
    /// Hash
    pub hash: u64,
}

/// Sync item type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncItemType {
    /// Improvement
    Improvement,
    /// Node state
    NodeState,
    /// Model weights
    ModelWeights,
    /// Configuration
    Configuration,
}

/// Gradient payload
#[derive(Debug, Clone)]
pub struct GradientPayload {
    /// Model ID
    pub model_id: u64,
    /// Epoch
    pub epoch: Epoch,
    /// Gradients
    pub gradients: Vec<f64>,
    /// Sample count
    pub sample_count: u64,
    /// Privacy budget spent
    pub privacy_budget: f64,
}

/// Aggregate payload
#[derive(Debug, Clone)]
pub struct AggregatePayload {
    /// Request type
    pub request_type: AggregateType,
    /// Model ID
    pub model_id: u64,
    /// Participants
    pub participants: Vec<NodeId>,
    /// Aggregated result
    pub result: Option<Vec<f64>>,
}

/// Aggregate type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateType {
    /// Request aggregation
    Request,
    /// Contribution
    Contribution,
    /// Result
    Result,
}

/// Consensus payload
#[derive(Debug, Clone)]
pub struct ConsensusPayload {
    /// Phase
    pub phase: ConsensusPhase,
    /// Proposal ID
    pub proposal_id: u64,
    /// Term
    pub term: Term,
    /// Value
    pub value: Vec<u8>,
    /// Accepted
    pub accepted: bool,
}

/// Consensus phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsensusPhase {
    /// Prepare (Paxos phase 1a)
    Prepare,
    /// Promise (Paxos phase 1b)
    Promise,
    /// Accept (Paxos phase 2a)
    Accept,
    /// Accepted (Paxos phase 2b)
    Accepted,
    /// Commit
    Commit,
    /// Nack
    Nack,
}

/// Discovery payload
#[derive(Debug, Clone)]
pub struct DiscoveryPayload {
    /// Discovery type
    pub discovery_type: DiscoveryType,
    /// Nodes
    pub nodes: Vec<NodeInfo>,
    /// Clusters
    pub clusters: Vec<ClusterInfo>,
}

/// Discovery type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryType {
    /// Ping
    Ping,
    /// Pong
    Pong,
    /// Announce
    Announce,
    /// Query
    Query,
}

/// Cluster info
#[derive(Debug, Clone)]
pub struct ClusterInfo {
    /// Cluster ID
    pub id: ClusterId,
    /// Name
    pub name: String,
    /// Member count
    pub member_count: usize,
    /// Epoch
    pub epoch: Epoch,
}

/// Gossip payload
#[derive(Debug, Clone)]
pub struct GossipPayload {
    /// Gossip items
    pub items: Vec<GossipItem>,
    /// Seen nodes
    pub seen: Vec<NodeId>,
}

/// Gossip item
#[derive(Debug, Clone)]
pub struct GossipItem {
    /// Key
    pub key: String,
    /// Value
    pub value: Vec<u8>,
    /// Version
    pub version: u64,
    /// Origin
    pub origin: NodeId,
}

// ============================================================================
// MESSAGE BUILDER
// ============================================================================

/// Message builder
pub struct MessageBuilder {
    source: NodeId,
    destination: Option<NodeId>,
    message_type: MessageType,
    payload: MessagePayload,
    ttl: u8,
}

impl MessageBuilder {
    /// Create new builder
    pub fn new(source: NodeId, message_type: MessageType) -> Self {
        Self {
            source,
            destination: None,
            message_type,
            payload: MessagePayload::Empty,
            ttl: 10,
        }
    }

    /// Set destination
    pub fn to(mut self, destination: NodeId) -> Self {
        self.destination = Some(destination);
        self
    }

    /// Set broadcast
    pub fn broadcast(mut self) -> Self {
        self.destination = None;
        self
    }

    /// Set payload
    pub fn payload(mut self, payload: MessagePayload) -> Self {
        self.payload = payload;
        self
    }

    /// Set TTL
    pub fn ttl(mut self, ttl: u8) -> Self {
        self.ttl = ttl;
        self
    }

    /// Build heartbeat
    pub fn heartbeat(self, epoch: Epoch, term: Term, leader: Option<NodeId>) -> Message {
        let mut msg = self;
        msg.message_type = MessageType::Heartbeat;
        msg.payload = MessagePayload::Heartbeat(HeartbeatPayload {
            epoch,
            term,
            leader,
            load: 0,
            improvements_count: 0,
        });
        msg.build()
    }

    /// Build join request
    pub fn join_request(self, node_info: NodeInfo) -> Message {
        let mut msg = self;
        msg.message_type = MessageType::JoinRequest;
        msg.payload = MessagePayload::Join(JoinPayload {
            node_info,
            cluster_id: None,
            token: None,
        });
        msg.build()
    }

    /// Build the message
    pub fn build(self) -> Message {
        Message {
            id: MessageId::generate(),
            source: self.source,
            destination: self.destination,
            message_type: self.message_type,
            payload: self.payload,
            timestamp: 0,
            ttl: self.ttl,
            signature: 0,
        }
    }
}

// ============================================================================
// PROTOCOL HANDLER
// ============================================================================

/// Protocol handler trait
pub trait ProtocolHandler: Send + Sync {
    /// Handle incoming message
    fn handle(&self, message: &Message) -> Option<Message>;

    /// Get supported message types
    fn supported_types(&self) -> &[MessageType];
}

/// Default protocol handler
pub struct DefaultProtocolHandler {
    node_id: NodeId,
    handlers: BTreeMap<MessageType, Box<dyn Fn(&Message) -> Option<Message> + Send + Sync>>,
}

impl DefaultProtocolHandler {
    /// Create new handler
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            handlers: BTreeMap::new(),
        }
    }

    /// Register handler for message type
    pub fn register<F>(&mut self, message_type: MessageType, handler: F)
    where
        F: Fn(&Message) -> Option<Message> + Send + Sync + 'static,
    {
        self.handlers.insert(message_type, Box::new(handler));
    }
}

impl ProtocolHandler for DefaultProtocolHandler {
    fn handle(&self, message: &Message) -> Option<Message> {
        self.handlers.get(&message.message_type)
            .and_then(|h| h(message))
    }

    fn supported_types(&self) -> &[MessageType] {
        &[
            MessageType::Heartbeat,
            MessageType::JoinRequest,
            MessageType::JoinResponse,
        ]
    }
}

// ============================================================================
// MESSAGE ROUTER
// ============================================================================

/// Message router
pub struct MessageRouter {
    /// Local node
    node_id: NodeId,
    /// Handlers
    handlers: Vec<Box<dyn ProtocolHandler>>,
    /// Pending responses
    pending: BTreeMap<MessageId, PendingMessage>,
    /// Statistics
    stats: RouterStats,
}

/// Pending message
#[derive(Debug)]
pub struct PendingMessage {
    /// Original message ID
    pub message_id: MessageId,
    /// Expected response type
    pub expected_type: MessageType,
    /// Timeout
    pub timeout: u64,
    /// Created at
    pub created_at: u64,
}

/// Router statistics
#[derive(Debug, Clone, Default)]
pub struct RouterStats {
    /// Messages routed
    pub messages_routed: u64,
    /// Messages dropped
    pub messages_dropped: u64,
    /// Timeouts
    pub timeouts: u64,
}

impl MessageRouter {
    /// Create new router
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node_id,
            handlers: Vec::new(),
            pending: BTreeMap::new(),
            stats: RouterStats::default(),
        }
    }

    /// Add handler
    pub fn add_handler(&mut self, handler: Box<dyn ProtocolHandler>) {
        self.handlers.push(handler);
    }

    /// Route message
    pub fn route(&mut self, message: Message) -> Option<Message> {
        // Check TTL
        if message.ttl == 0 {
            self.stats.messages_dropped += 1;
            return None;
        }

        // Check if for us
        if let Some(dest) = message.destination {
            if dest != self.node_id {
                // Forward
                self.stats.messages_routed += 1;
                return Some(message);
            }
        }

        // Handle locally
        for handler in &self.handlers {
            if let Some(response) = handler.handle(&message) {
                self.stats.messages_routed += 1;
                return Some(response);
            }
        }

        None
    }

    /// Send and wait for response
    pub fn send_with_response(&mut self, message: Message, timeout: u64) {
        let pending = PendingMessage {
            message_id: message.id,
            expected_type: self.expected_response_type(message.message_type),
            timeout,
            created_at: 0,
        };
        self.pending.insert(message.id, pending);
    }

    /// Check for response
    pub fn check_response(&mut self, message_id: MessageId) -> Option<PendingMessage> {
        self.pending.remove(&message_id)
    }

    fn expected_response_type(&self, request_type: MessageType) -> MessageType {
        match request_type {
            MessageType::JoinRequest => MessageType::JoinResponse,
            MessageType::VoteRequest => MessageType::VoteResponse,
            MessageType::SyncRequest => MessageType::SyncResponse,
            MessageType::AggregateRequest => MessageType::AggregateResponse,
            MessageType::DiscoveryPing => MessageType::DiscoveryPong,
            _ => request_type,
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &RouterStats {
        &self.stats
    }
}

// ============================================================================
// SERIALIZATION
// ============================================================================

/// Message serializer
pub struct MessageSerializer;

impl MessageSerializer {
    /// Serialize message
    pub fn serialize(message: &Message) -> Vec<u8> {
        // Simplified serialization
        let mut bytes = Vec::new();

        // Header
        bytes.extend_from_slice(&message.id.0.to_le_bytes());
        bytes.extend_from_slice(&message.source.0.to_le_bytes());
        bytes.push(message.destination.is_some() as u8);
        if let Some(dest) = message.destination {
            bytes.extend_from_slice(&dest.0.to_le_bytes());
        }
        bytes.push(message.message_type as u8);
        bytes.extend_from_slice(&message.timestamp.to_le_bytes());
        bytes.push(message.ttl);
        bytes.extend_from_slice(&message.signature.to_le_bytes());

        bytes
    }

    /// Deserialize message
    pub fn deserialize(bytes: &[u8]) -> Option<Message> {
        if bytes.len() < 26 {
            return None;
        }

        let id = MessageId(u64::from_le_bytes(bytes[0..8].try_into().ok()?));
        let source = NodeId(u64::from_le_bytes(bytes[8..16].try_into().ok()?));
        let has_dest = bytes[16] != 0;
        let offset = if has_dest { 25 } else { 17 };
        let destination = if has_dest {
            Some(NodeId(u64::from_le_bytes(bytes[17..25].try_into().ok()?)))
        } else {
            None
        };

        Some(Message {
            id,
            source,
            destination,
            message_type: MessageType::Heartbeat, // Simplified
            payload: MessagePayload::Empty,
            timestamp: 0,
            ttl: bytes.get(offset).copied().unwrap_or(10),
            signature: 0,
        })
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_builder() {
        let source = NodeId(1);
        let dest = NodeId(2);

        let msg = MessageBuilder::new(source, MessageType::Heartbeat)
            .to(dest)
            .ttl(5)
            .build();

        assert_eq!(msg.source, source);
        assert_eq!(msg.destination, Some(dest));
        assert_eq!(msg.ttl, 5);
    }

    #[test]
    fn test_serialization() {
        let msg = Message {
            id: MessageId(1),
            source: NodeId(2),
            destination: Some(NodeId(3)),
            message_type: MessageType::Heartbeat,
            payload: MessagePayload::Empty,
            timestamp: 0,
            ttl: 10,
            signature: 0,
        };

        let bytes = MessageSerializer::serialize(&msg);
        let decoded = MessageSerializer::deserialize(&bytes).unwrap();

        assert_eq!(decoded.id, msg.id);
        assert_eq!(decoded.source, msg.source);
    }

    #[test]
    fn test_router() {
        let node_id = NodeId(1);
        let mut router = MessageRouter::new(node_id);

        let msg = Message {
            id: MessageId(1),
            source: NodeId(2),
            destination: Some(node_id),
            message_type: MessageType::Heartbeat,
            payload: MessagePayload::Empty,
            timestamp: 0,
            ttl: 10,
            signature: 0,
        };

        router.route(msg);
        assert_eq!(router.stats().messages_routed, 0); // No handler
    }
}
