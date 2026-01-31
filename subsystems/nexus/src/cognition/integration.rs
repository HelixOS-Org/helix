//! # Cognitive Integration Hub
//!
//! Integrates cognitive domains and external systems.
//! Provides adapters and transformers.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// INTEGRATION TYPES
// ============================================================================

/// An integration endpoint
#[derive(Debug, Clone)]
pub struct IntegrationEndpoint {
    /// Endpoint ID
    pub id: u64,
    /// Endpoint name
    pub name: String,
    /// Endpoint type
    pub endpoint_type: EndpointType,
    /// Protocol
    pub protocol: Protocol,
    /// Owner domain
    pub owner: DomainId,
    /// Configuration
    pub config: EndpointConfig,
    /// Status
    pub status: EndpointStatus,
    /// Created time
    pub created: Timestamp,
    /// Last active
    pub last_active: Timestamp,
}

/// Endpoint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointType {
    /// Produces data
    Producer,
    /// Consumes data
    Consumer,
    /// Both produces and consumes
    BiDirectional,
    /// Request-response
    RequestResponse,
}

/// Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Internal message passing
    Internal,
    /// Shared memory
    SharedMemory,
    /// Event-based
    Event,
    /// Stream-based
    Stream,
    /// RPC-like
    Rpc,
}

/// Endpoint configuration
#[derive(Debug, Clone, Default)]
pub struct EndpointConfig {
    /// Buffer size
    pub buffer_size: usize,
    /// Timeout (ns)
    pub timeout_ns: u64,
    /// Retry count
    pub retry_count: u32,
    /// Custom properties
    pub properties: BTreeMap<String, String>,
}

/// Endpoint status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointStatus {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected and active
    Connected,
    /// Error state
    Error,
    /// Paused
    Paused,
}

/// Integration message
#[derive(Debug, Clone)]
pub struct IntegrationMessage {
    /// Message ID
    pub id: u64,
    /// Source endpoint
    pub source: u64,
    /// Target endpoint
    pub target: Option<u64>,
    /// Message type
    pub msg_type: String,
    /// Payload
    pub payload: Vec<u8>,
    /// Headers
    pub headers: BTreeMap<String, String>,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Correlation ID (for request-response)
    pub correlation_id: Option<u64>,
}

/// Data transformer
pub trait DataTransformer: Send + Sync {
    /// Transform data
    fn transform(&self, data: &[u8]) -> Result<Vec<u8>, &'static str>;

    /// Transformer name
    fn name(&self) -> &str;
}

// ============================================================================
// INTEGRATION HUB
// ============================================================================

/// Integration hub for cognitive domains
pub struct IntegrationHub {
    /// Endpoints
    endpoints: BTreeMap<u64, IntegrationEndpoint>,
    /// Connections (source -> targets)
    connections: BTreeMap<u64, Vec<u64>>,
    /// Message queue
    message_queue: Vec<IntegrationMessage>,
    /// Pending responses
    pending_responses: BTreeMap<u64, PendingResponse>,
    /// Next endpoint ID
    next_endpoint_id: AtomicU64,
    /// Next message ID
    next_message_id: AtomicU64,
    /// Configuration
    config: HubConfig,
    /// Statistics
    stats: HubStats,
}

/// Pending response
#[derive(Debug)]
struct PendingResponse {
    /// Request message ID
    request_id: u64,
    /// Source endpoint
    source: u64,
    /// Deadline
    deadline: Timestamp,
}

/// Hub configuration
#[derive(Debug, Clone)]
pub struct HubConfig {
    /// Maximum endpoints
    pub max_endpoints: usize,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Default timeout (ns)
    pub default_timeout_ns: u64,
    /// Enable message routing
    pub enable_routing: bool,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            max_endpoints: 1000,
            max_queue_size: 100000,
            default_timeout_ns: 5_000_000_000, // 5 seconds
            enable_routing: true,
        }
    }
}

/// Hub statistics
#[derive(Debug, Clone, Default)]
pub struct HubStats {
    /// Total messages sent
    pub messages_sent: u64,
    /// Total messages received
    pub messages_received: u64,
    /// Total bytes transferred
    pub bytes_transferred: u64,
    /// Errors
    pub errors: u64,
    /// Active endpoints
    pub active_endpoints: u64,
    /// Average latency (ns)
    pub avg_latency_ns: f64,
}

impl IntegrationHub {
    /// Create a new integration hub
    pub fn new(config: HubConfig) -> Self {
        Self {
            endpoints: BTreeMap::new(),
            connections: BTreeMap::new(),
            message_queue: Vec::new(),
            pending_responses: BTreeMap::new(),
            next_endpoint_id: AtomicU64::new(1),
            next_message_id: AtomicU64::new(1),
            config,
            stats: HubStats::default(),
        }
    }

    /// Register an endpoint
    pub fn register_endpoint(
        &mut self,
        name: &str,
        endpoint_type: EndpointType,
        protocol: Protocol,
        owner: DomainId,
        config: Option<EndpointConfig>,
    ) -> u64 {
        let id = self.next_endpoint_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let endpoint = IntegrationEndpoint {
            id,
            name: name.into(),
            endpoint_type,
            protocol,
            owner,
            config: config.unwrap_or_default(),
            status: EndpointStatus::Disconnected,
            created: now,
            last_active: now,
        };

        self.endpoints.insert(id, endpoint);
        self.connections.insert(id, Vec::new());
        self.update_stats();

        id
    }

    /// Unregister an endpoint
    pub fn unregister_endpoint(&mut self, id: u64) -> bool {
        // Remove connections
        self.connections.remove(&id);
        for targets in self.connections.values_mut() {
            targets.retain(|&t| t != id);
        }

        let removed = self.endpoints.remove(&id).is_some();
        if removed {
            self.update_stats();
        }
        removed
    }

    /// Connect endpoints
    pub fn connect(&mut self, source: u64, target: u64) -> Result<(), &'static str> {
        // Validate endpoints exist
        if !self.endpoints.contains_key(&source) {
            return Err("Source endpoint not found");
        }
        if !self.endpoints.contains_key(&target) {
            return Err("Target endpoint not found");
        }

        // Check compatibility
        let source_type = self.endpoints[&source].endpoint_type;
        let target_type = self.endpoints[&target].endpoint_type;

        let compatible = matches!(
            (source_type, target_type),
            (EndpointType::Producer, EndpointType::Consumer)
                | (EndpointType::Producer, EndpointType::BiDirectional)
                | (EndpointType::BiDirectional, EndpointType::Consumer)
                | (EndpointType::BiDirectional, EndpointType::BiDirectional)
                | (EndpointType::RequestResponse, EndpointType::RequestResponse)
        );

        if !compatible {
            return Err("Incompatible endpoint types");
        }

        // Add connection
        if let Some(targets) = self.connections.get_mut(&source) {
            if !targets.contains(&target) {
                targets.push(target);
            }
        }

        // Update status
        if let Some(ep) = self.endpoints.get_mut(&source) {
            ep.status = EndpointStatus::Connected;
        }
        if let Some(ep) = self.endpoints.get_mut(&target) {
            ep.status = EndpointStatus::Connected;
        }

        Ok(())
    }

    /// Disconnect endpoints
    pub fn disconnect(&mut self, source: u64, target: u64) {
        if let Some(targets) = self.connections.get_mut(&source) {
            targets.retain(|&t| t != target);

            if targets.is_empty() {
                if let Some(ep) = self.endpoints.get_mut(&source) {
                    ep.status = EndpointStatus::Disconnected;
                }
            }
        }
    }

    /// Send a message
    pub fn send(
        &mut self,
        source: u64,
        target: Option<u64>,
        msg_type: &str,
        payload: Vec<u8>,
        headers: BTreeMap<String, String>,
    ) -> Result<u64, &'static str> {
        // Validate source
        if !self.endpoints.contains_key(&source) {
            return Err("Source endpoint not found");
        }

        // Validate target if specified
        if let Some(t) = target {
            if !self.endpoints.contains_key(&t) {
                return Err("Target endpoint not found");
            }
        }

        // Check queue limit
        if self.message_queue.len() >= self.config.max_queue_size {
            self.stats.errors += 1;
            return Err("Message queue full");
        }

        let id = self.next_message_id.fetch_add(1, Ordering::Relaxed);

        let message = IntegrationMessage {
            id,
            source,
            target,
            msg_type: msg_type.into(),
            payload: payload.clone(),
            headers,
            timestamp: Timestamp::now(),
            correlation_id: None,
        };

        self.message_queue.push(message);

        // Update stats
        self.stats.messages_sent += 1;
        self.stats.bytes_transferred += payload.len() as u64;

        // Update endpoint activity
        if let Some(ep) = self.endpoints.get_mut(&source) {
            ep.last_active = Timestamp::now();
        }

        Ok(id)
    }

    /// Send request and wait for response
    pub fn request(
        &mut self,
        source: u64,
        target: u64,
        msg_type: &str,
        payload: Vec<u8>,
        headers: BTreeMap<String, String>,
    ) -> Result<u64, &'static str> {
        // Validate endpoints
        if !self.endpoints.contains_key(&source) {
            return Err("Source endpoint not found");
        }
        if !self.endpoints.contains_key(&target) {
            return Err("Target endpoint not found");
        }

        let id = self.next_message_id.fetch_add(1, Ordering::Relaxed);
        let timeout = self
            .endpoints
            .get(&source)
            .map(|e| e.config.timeout_ns)
            .unwrap_or(self.config.default_timeout_ns);

        let message = IntegrationMessage {
            id,
            source,
            target: Some(target),
            msg_type: msg_type.into(),
            payload: payload.clone(),
            headers,
            timestamp: Timestamp::now(),
            correlation_id: Some(id),
        };

        self.message_queue.push(message);

        // Track pending response
        let deadline = Timestamp::from_raw(Timestamp::now().raw() + timeout);
        self.pending_responses.insert(id, PendingResponse {
            request_id: id,
            source,
            deadline,
        });

        self.stats.messages_sent += 1;
        self.stats.bytes_transferred += payload.len() as u64;

        Ok(id)
    }

    /// Respond to a request
    pub fn respond(
        &mut self,
        source: u64,
        correlation_id: u64,
        payload: Vec<u8>,
        headers: BTreeMap<String, String>,
    ) -> Result<u64, &'static str> {
        // Find the original request
        let pending = self
            .pending_responses
            .remove(&correlation_id)
            .ok_or("No pending request found")?;

        let id = self.next_message_id.fetch_add(1, Ordering::Relaxed);

        let message = IntegrationMessage {
            id,
            source,
            target: Some(pending.source),
            msg_type: "response".into(),
            payload: payload.clone(),
            headers,
            timestamp: Timestamp::now(),
            correlation_id: Some(correlation_id),
        };

        self.message_queue.push(message);

        self.stats.messages_sent += 1;
        self.stats.bytes_transferred += payload.len() as u64;

        Ok(id)
    }

    /// Receive messages for an endpoint
    pub fn receive(&mut self, endpoint: u64) -> Vec<IntegrationMessage> {
        let messages: Vec<_> = self
            .message_queue
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                m.target == Some(endpoint)
                    || (m.target.is_none() && self.is_subscribed(m.source, endpoint))
            })
            .map(|(i, m)| (i, m.clone()))
            .collect();

        // Remove received messages (in reverse to maintain indices)
        let indices: Vec<_> = messages.iter().map(|(i, _)| *i).collect();
        for idx in indices.into_iter().rev() {
            self.message_queue.remove(idx);
        }

        // Update stats
        self.stats.messages_received += messages.len() as u64;

        // Update endpoint activity
        if let Some(ep) = self.endpoints.get_mut(&endpoint) {
            ep.last_active = Timestamp::now();
        }

        messages.into_iter().map(|(_, m)| m).collect()
    }

    /// Check if endpoint is subscribed to source
    fn is_subscribed(&self, source: u64, endpoint: u64) -> bool {
        self.connections
            .get(&source)
            .map(|targets| targets.contains(&endpoint))
            .unwrap_or(false)
    }

    /// Broadcast message to all connected endpoints
    pub fn broadcast(
        &mut self,
        source: u64,
        msg_type: &str,
        payload: Vec<u8>,
        headers: BTreeMap<String, String>,
    ) -> Result<Vec<u64>, &'static str> {
        let targets = self.connections.get(&source).cloned().unwrap_or_default();

        let mut ids = Vec::new();
        for target in targets {
            let id = self.send(
                source,
                Some(target),
                msg_type,
                payload.clone(),
                headers.clone(),
            )?;
            ids.push(id);
        }

        Ok(ids)
    }

    /// Get endpoint
    pub fn get_endpoint(&self, id: u64) -> Option<&IntegrationEndpoint> {
        self.endpoints.get(&id)
    }

    /// Get endpoint by name
    pub fn get_endpoint_by_name(&self, name: &str) -> Option<&IntegrationEndpoint> {
        self.endpoints.values().find(|e| e.name == name)
    }

    /// Get endpoints by owner
    pub fn get_endpoints_by_owner(&self, owner: DomainId) -> Vec<&IntegrationEndpoint> {
        self.endpoints
            .values()
            .filter(|e| e.owner == owner)
            .collect()
    }

    /// Get connected endpoints
    pub fn get_connections(&self, source: u64) -> Vec<u64> {
        self.connections.get(&source).cloned().unwrap_or_default()
    }

    /// Update endpoint status
    pub fn set_endpoint_status(&mut self, id: u64, status: EndpointStatus) {
        if let Some(ep) = self.endpoints.get_mut(&id) {
            ep.status = status;
        }
        self.update_stats();
    }

    /// Check for timed out requests
    pub fn check_timeouts(&mut self) -> Vec<u64> {
        let now = Timestamp::now();
        let timed_out: Vec<_> = self
            .pending_responses
            .iter()
            .filter(|(_, p)| now.raw() > p.deadline.raw())
            .map(|(id, _)| *id)
            .collect();

        for id in &timed_out {
            self.pending_responses.remove(id);
            self.stats.errors += 1;
        }

        timed_out
    }

    /// Update statistics
    fn update_stats(&mut self) {
        self.stats.active_endpoints = self
            .endpoints
            .values()
            .filter(|e| e.status == EndpointStatus::Connected)
            .count() as u64;
    }

    /// Get statistics
    pub fn stats(&self) -> &HubStats {
        &self.stats
    }

    /// Get endpoint count
    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    /// Get pending message count
    pub fn pending_message_count(&self) -> usize {
        self.message_queue.len()
    }
}

impl Default for IntegrationHub {
    fn default() -> Self {
        Self::new(HubConfig::default())
    }
}

// ============================================================================
// DATA ADAPTERS
// ============================================================================

/// Identity transformer (no-op)
pub struct IdentityTransformer;

impl DataTransformer for IdentityTransformer {
    fn transform(&self, data: &[u8]) -> Result<Vec<u8>, &'static str> {
        Ok(data.to_vec())
    }

    fn name(&self) -> &str {
        "identity"
    }
}

/// Prefix transformer
pub struct PrefixTransformer {
    prefix: Vec<u8>,
}

impl PrefixTransformer {
    pub fn new(prefix: &[u8]) -> Self {
        Self {
            prefix: prefix.to_vec(),
        }
    }
}

impl DataTransformer for PrefixTransformer {
    fn transform(&self, data: &[u8]) -> Result<Vec<u8>, &'static str> {
        let mut result = self.prefix.clone();
        result.extend_from_slice(data);
        Ok(result)
    }

    fn name(&self) -> &str {
        "prefix"
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_registration() {
        let mut hub = IntegrationHub::default();
        let domain = DomainId::new(1);

        let id = hub.register_endpoint(
            "test_producer",
            EndpointType::Producer,
            Protocol::Internal,
            domain,
            None,
        );

        let endpoint = hub.get_endpoint(id).unwrap();
        assert_eq!(endpoint.name, "test_producer");
        assert_eq!(endpoint.endpoint_type, EndpointType::Producer);
    }

    #[test]
    fn test_endpoint_connection() {
        let mut hub = IntegrationHub::default();
        let domain = DomainId::new(1);

        let producer = hub.register_endpoint(
            "producer",
            EndpointType::Producer,
            Protocol::Internal,
            domain,
            None,
        );

        let consumer = hub.register_endpoint(
            "consumer",
            EndpointType::Consumer,
            Protocol::Internal,
            domain,
            None,
        );

        assert!(hub.connect(producer, consumer).is_ok());
        assert!(hub.get_connections(producer).contains(&consumer));
    }

    #[test]
    fn test_message_send_receive() {
        let mut hub = IntegrationHub::default();
        let domain = DomainId::new(1);

        let producer = hub.register_endpoint(
            "producer",
            EndpointType::Producer,
            Protocol::Internal,
            domain,
            None,
        );

        let consumer = hub.register_endpoint(
            "consumer",
            EndpointType::Consumer,
            Protocol::Internal,
            domain,
            None,
        );

        hub.connect(producer, consumer).unwrap();

        // Send message
        let payload = b"Hello".to_vec();
        hub.send(
            producer,
            Some(consumer),
            "greeting",
            payload.clone(),
            BTreeMap::new(),
        )
        .unwrap();

        // Receive message
        let messages = hub.receive(consumer);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].payload, payload);
    }

    #[test]
    fn test_broadcast() {
        let mut hub = IntegrationHub::default();
        let domain = DomainId::new(1);

        let producer = hub.register_endpoint(
            "producer",
            EndpointType::Producer,
            Protocol::Internal,
            domain,
            None,
        );

        let consumer1 = hub.register_endpoint(
            "consumer1",
            EndpointType::Consumer,
            Protocol::Internal,
            domain,
            None,
        );

        let consumer2 = hub.register_endpoint(
            "consumer2",
            EndpointType::Consumer,
            Protocol::Internal,
            domain,
            None,
        );

        hub.connect(producer, consumer1).unwrap();
        hub.connect(producer, consumer2).unwrap();

        let ids = hub
            .broadcast(producer, "event", b"data".to_vec(), BTreeMap::new())
            .unwrap();
        assert_eq!(ids.len(), 2);

        assert_eq!(hub.receive(consumer1).len(), 1);
        assert_eq!(hub.receive(consumer2).len(), 1);
    }

    #[test]
    fn test_request_response() {
        let mut hub = IntegrationHub::default();
        let domain = DomainId::new(1);

        let client = hub.register_endpoint(
            "client",
            EndpointType::RequestResponse,
            Protocol::Rpc,
            domain,
            None,
        );

        let server = hub.register_endpoint(
            "server",
            EndpointType::RequestResponse,
            Protocol::Rpc,
            domain,
            None,
        );

        hub.connect(client, server).unwrap();

        // Send request
        let request_id = hub
            .request(
                client,
                server,
                "query",
                b"request".to_vec(),
                BTreeMap::new(),
            )
            .unwrap();

        // Server receives and responds
        let requests = hub.receive(server);
        assert_eq!(requests.len(), 1);

        hub.respond(server, request_id, b"response".to_vec(), BTreeMap::new())
            .unwrap();

        // Client receives response
        let responses = hub.receive(client);
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].correlation_id, Some(request_id));
    }
}
