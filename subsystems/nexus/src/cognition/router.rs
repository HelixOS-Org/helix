//! # Cognitive Message Router
//!
//! Routes messages between cognitive domains.
//! Implements content-based routing and load balancing.

#![allow(dead_code)]

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::boxed::Box;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// ROUTING TYPES
// ============================================================================

/// A routable message
#[derive(Debug, Clone)]
pub struct RoutableMessage {
    /// Message ID
    pub id: u64,
    /// Source domain
    pub source: DomainId,
    /// Message type
    pub msg_type: String,
    /// Message content
    pub content: MessageContent,
    /// Routing hints
    pub hints: RoutingHints,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Message content
#[derive(Debug, Clone)]
pub enum MessageContent {
    /// Empty
    Empty,
    /// Bytes
    Bytes(Vec<u8>),
    /// Text
    Text(String),
    /// Numeric
    Numeric(f64),
    /// Structured
    Structured(BTreeMap<String, MessageContent>),
    /// Array
    Array(Vec<MessageContent>),
}

/// Routing hints
#[derive(Debug, Clone, Default)]
pub struct RoutingHints {
    /// Preferred destination
    pub preferred: Option<DomainId>,
    /// Priority
    pub priority: u32,
    /// TTL (hops)
    pub ttl: u8,
    /// Require acknowledgment
    pub require_ack: bool,
    /// Broadcast
    pub broadcast: bool,
    /// Sticky routing (same destination for same key)
    pub sticky_key: Option<String>,
}

/// Routing rule
#[derive(Debug, Clone)]
pub struct RoutingRule {
    /// Rule ID
    pub id: u64,
    /// Rule name
    pub name: String,
    /// Match criteria
    pub criteria: RoutingCriteria,
    /// Destinations
    pub destinations: Vec<DomainId>,
    /// Load balancing strategy
    pub strategy: LoadBalanceStrategy,
    /// Priority
    pub priority: u32,
    /// Enabled
    pub enabled: bool,
}

/// Routing criteria
#[derive(Debug, Clone)]
pub enum RoutingCriteria {
    /// Match message type
    MessageType(String),
    /// Match source domain
    Source(DomainId),
    /// Match content field
    ContentField(String, FieldMatcher),
    /// Match all
    All,
    /// Match any of criteria
    Any(Vec<RoutingCriteria>),
    /// Match all of criteria
    AllOf(Vec<RoutingCriteria>),
    /// Negate criteria
    Not(Box<RoutingCriteria>),
}

/// Field matcher
#[derive(Debug, Clone)]
pub enum FieldMatcher {
    /// Equals value
    Equals(String),
    /// Contains substring
    Contains(String),
    /// Starts with
    StartsWith(String),
    /// Numeric comparison
    NumericGt(f64),
    NumericLt(f64),
    NumericEq(f64),
    /// Exists
    Exists,
}

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalanceStrategy {
    /// Round robin
    RoundRobin,
    /// Random
    Random,
    /// Least loaded
    LeastLoaded,
    /// Hash-based (consistent)
    HashBased,
    /// First available
    First,
    /// Broadcast to all
    Broadcast,
}

/// Routing result
#[derive(Debug, Clone)]
pub struct RoutingResult {
    /// Message ID
    pub message_id: u64,
    /// Matched rule
    pub rule_id: Option<u64>,
    /// Destinations
    pub destinations: Vec<DomainId>,
    /// Routing time (ns)
    pub routing_time_ns: u64,
}

// ============================================================================
// ROUTER
// ============================================================================

/// Routes messages between domains
pub struct MessageRouter {
    /// Routing rules
    rules: BTreeMap<u64, RoutingRule>,
    /// Next rule ID
    next_rule_id: AtomicU64,
    /// Round-robin counters per rule
    rr_counters: LinearMap<usize, 64>,
    /// Sticky routing cache
    sticky_cache: BTreeMap<String, DomainId>,
    /// Domain load
    domain_load: BTreeMap<DomainId, u32>,
    /// Default destination
    default_destination: Option<DomainId>,
    /// Statistics
    stats: RouterStats,
}

/// Router statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct RouterStats {
    /// Total messages routed
    pub total_routed: u64,
    /// Messages with no match
    pub no_match: u64,
    /// Messages broadcast
    pub broadcast: u64,
    /// Average routing time (ns)
    pub avg_routing_time_ns: f32,
    /// Rules matched per message
    pub avg_rules_matched: f32,
}

impl MessageRouter {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            rules: BTreeMap::new(),
            next_rule_id: AtomicU64::new(1),
            rr_counters: LinearMap::new(),
            sticky_cache: BTreeMap::new(),
            domain_load: BTreeMap::new(),
            default_destination: None,
            stats: RouterStats::default(),
        }
    }

    /// Add a routing rule
    pub fn add_rule(&mut self, rule: RoutingRule) -> u64 {
        let id = if rule.id == 0 {
            self.next_rule_id.fetch_add(1, Ordering::Relaxed)
        } else {
            rule.id
        };

        let mut rule = rule;
        rule.id = id;
        self.rules.insert(id, rule);
        self.rr_counters.insert(id, 0);

        id
    }

    /// Remove a routing rule
    #[inline(always)]
    pub fn remove_rule(&mut self, rule_id: u64) -> bool {
        self.rr_counters.remove(rule_id);
        self.rules.remove(&rule_id).is_some()
    }

    /// Enable/disable a rule
    #[inline]
    pub fn set_rule_enabled(&mut self, rule_id: u64, enabled: bool) {
        if let Some(rule) = self.rules.get_mut(&rule_id) {
            rule.enabled = enabled;
        }
    }

    /// Set default destination
    #[inline(always)]
    pub fn set_default(&mut self, domain: DomainId) {
        self.default_destination = Some(domain);
    }

    /// Update domain load
    #[inline(always)]
    pub fn update_load(&mut self, domain: DomainId, load: u32) {
        self.domain_load.insert(domain, load);
    }

    /// Route a message
    pub fn route(&mut self, message: &RoutableMessage) -> RoutingResult {
        let start = Timestamp::now();
        self.stats.total_routed += 1;

        // Check hints first
        if message.hints.broadcast {
            let all_destinations: Vec<_> = self.domain_load.keys().copied().collect();
            self.stats.broadcast += 1;
            return RoutingResult {
                message_id: message.id,
                rule_id: None,
                destinations: all_destinations,
                routing_time_ns: Timestamp::now().elapsed_since(start),
            };
        }

        if let Some(preferred) = message.hints.preferred {
            return RoutingResult {
                message_id: message.id,
                rule_id: None,
                destinations: vec![preferred],
                routing_time_ns: Timestamp::now().elapsed_since(start),
            };
        }

        // Check sticky routing
        if let Some(ref key) = message.hints.sticky_key {
            if let Some(&dest) = self.sticky_cache.get(key) {
                return RoutingResult {
                    message_id: message.id,
                    rule_id: None,
                    destinations: vec![dest],
                    routing_time_ns: Timestamp::now().elapsed_since(start),
                };
            }
        }

        // Find matching rules
        let mut matching_rules: Vec<_> = self
            .rules
            .values()
            .filter(|r| r.enabled && self.matches_criteria(message, &r.criteria))
            .collect();

        // Sort by priority
        matching_rules.sort_by_key(|r| core::cmp::Reverse(r.priority));

        if matching_rules.is_empty() {
            self.stats.no_match += 1;

            let destinations = self
                .default_destination
                .map(|d| vec![d])
                .unwrap_or_default();

            return RoutingResult {
                message_id: message.id,
                rule_id: None,
                destinations,
                routing_time_ns: Timestamp::now().elapsed_since(start),
            };
        }

        // Use first matching rule
        let rule = matching_rules[0];
        let destinations = self.select_destinations(rule, message);

        // Update sticky cache
        if let Some(ref key) = message.hints.sticky_key {
            if let Some(&dest) = destinations.first() {
                self.sticky_cache.insert(key.clone(), dest);
            }
        }

        let routing_time = Timestamp::now().elapsed_since(start);
        self.stats.avg_routing_time_ns = (self.stats.avg_routing_time_ns
            * (self.stats.total_routed - 1) as f32
            + routing_time as f32)
            / self.stats.total_routed as f32;

        RoutingResult {
            message_id: message.id,
            rule_id: Some(rule.id),
            destinations,
            routing_time_ns: routing_time,
        }
    }

    /// Check if message matches criteria
    fn matches_criteria(&self, message: &RoutableMessage, criteria: &RoutingCriteria) -> bool {
        match criteria {
            RoutingCriteria::All => true,
            RoutingCriteria::MessageType(t) => message.msg_type == *t,
            RoutingCriteria::Source(s) => message.source == *s,
            RoutingCriteria::ContentField(field, matcher) => {
                self.match_content_field(&message.content, field, matcher)
            },
            RoutingCriteria::Any(crits) => crits.iter().any(|c| self.matches_criteria(message, c)),
            RoutingCriteria::AllOf(crits) => {
                crits.iter().all(|c| self.matches_criteria(message, c))
            },
            RoutingCriteria::Not(c) => !self.matches_criteria(message, c),
        }
    }

    /// Match content field
    fn match_content_field(
        &self,
        content: &MessageContent,
        field: &str,
        matcher: &FieldMatcher,
    ) -> bool {
        // Navigate to field
        let value = match content {
            MessageContent::Structured(map) => map.get(field),
            _ => return matches!(matcher, FieldMatcher::Exists) && false,
        };

        match (value, matcher) {
            (Some(_), FieldMatcher::Exists) => true,
            (None, _) => false,
            (Some(MessageContent::Text(s)), FieldMatcher::Equals(expected)) => s == expected,
            (Some(MessageContent::Text(s)), FieldMatcher::Contains(sub)) => {
                s.contains(sub.as_str())
            },
            (Some(MessageContent::Text(s)), FieldMatcher::StartsWith(prefix)) => {
                s.starts_with(prefix.as_str())
            },
            (Some(MessageContent::Numeric(n)), FieldMatcher::NumericGt(v)) => *n > *v,
            (Some(MessageContent::Numeric(n)), FieldMatcher::NumericLt(v)) => *n < *v,
            (Some(MessageContent::Numeric(n)), FieldMatcher::NumericEq(v)) => {
                (*n - *v).abs() < f64::EPSILON
            },
            _ => false,
        }
    }

    /// Select destinations based on strategy
    fn select_destinations(
        &mut self,
        rule: &RoutingRule,
        message: &RoutableMessage,
    ) -> Vec<DomainId> {
        if rule.destinations.is_empty() {
            return Vec::new();
        }

        match rule.strategy {
            LoadBalanceStrategy::First => {
                vec![rule.destinations[0]]
            },
            LoadBalanceStrategy::RoundRobin => {
                let counter = self.rr_counters.entry(rule.id).or_default();
                let idx = *counter % rule.destinations.len();
                *counter = (*counter + 1) % rule.destinations.len();
                vec![rule.destinations[idx]]
            },
            LoadBalanceStrategy::Random => {
                // Simple pseudo-random using timestamp
                let idx = (Timestamp::now().raw() as usize) % rule.destinations.len();
                vec![rule.destinations[idx]]
            },
            LoadBalanceStrategy::LeastLoaded => {
                let min_loaded = rule
                    .destinations
                    .iter()
                    .min_by_key(|d| self.domain_load.get(d).copied().unwrap_or(0))
                    .copied();
                min_loaded.map(|d| vec![d]).unwrap_or_default()
            },
            LoadBalanceStrategy::HashBased => {
                // Hash on sticky key or message type
                let key = message
                    .hints
                    .sticky_key
                    .as_ref()
                    .unwrap_or(&message.msg_type);
                let hash: usize = key.bytes().map(|b| b as usize).sum();
                let idx = hash % rule.destinations.len();
                vec![rule.destinations[idx]]
            },
            LoadBalanceStrategy::Broadcast => rule.destinations.clone(),
        }
    }

    /// Clear sticky cache
    #[inline(always)]
    pub fn clear_sticky_cache(&mut self) {
        self.sticky_cache.clear();
    }

    /// Get routing rules
    #[inline(always)]
    pub fn rules(&self) -> &BTreeMap<u64, RoutingRule> {
        &self.rules
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &RouterStats {
        &self.stats
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ROUTE BUILDER
// ============================================================================

/// Builder for routing rules
pub struct RouteBuilder {
    rule: RoutingRule,
}

impl RouteBuilder {
    /// Create a new builder
    pub fn new(name: &str) -> Self {
        Self {
            rule: RoutingRule {
                id: 0,
                name: name.into(),
                criteria: RoutingCriteria::All,
                destinations: Vec::new(),
                strategy: LoadBalanceStrategy::RoundRobin,
                priority: 100,
                enabled: true,
            },
        }
    }

    /// Set criteria
    #[inline(always)]
    pub fn when(mut self, criteria: RoutingCriteria) -> Self {
        self.rule.criteria = criteria;
        self
    }

    /// When message type matches
    #[inline(always)]
    pub fn when_type(self, msg_type: &str) -> Self {
        self.when(RoutingCriteria::MessageType(msg_type.into()))
    }

    /// When source matches
    #[inline(always)]
    pub fn when_source(self, source: DomainId) -> Self {
        self.when(RoutingCriteria::Source(source))
    }

    /// Add destination
    #[inline(always)]
    pub fn to(mut self, domain: DomainId) -> Self {
        self.rule.destinations.push(domain);
        self
    }

    /// Set strategy
    #[inline(always)]
    pub fn with_strategy(mut self, strategy: LoadBalanceStrategy) -> Self {
        self.rule.strategy = strategy;
        self
    }

    /// Set priority
    #[inline(always)]
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.rule.priority = priority;
        self
    }

    /// Build the rule
    #[inline(always)]
    pub fn build(self) -> RoutingRule {
        self.rule
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_routing() {
        let mut router = MessageRouter::new();

        let rule = RouteBuilder::new("signals")
            .when_type("signal")
            .to(DomainId::new(1))
            .to(DomainId::new(2))
            .with_strategy(LoadBalanceStrategy::RoundRobin)
            .build();

        router.add_rule(rule);

        let message = RoutableMessage {
            id: 1,
            source: DomainId::new(0),
            msg_type: "signal".into(),
            content: MessageContent::Empty,
            hints: RoutingHints::default(),
            timestamp: Timestamp::now(),
        };

        let result = router.route(&message);
        assert!(!result.destinations.is_empty());
    }

    #[test]
    fn test_round_robin() {
        let mut router = MessageRouter::new();

        let rule = RouteBuilder::new("rr_test")
            .when_type("test")
            .to(DomainId::new(1))
            .to(DomainId::new(2))
            .to(DomainId::new(3))
            .with_strategy(LoadBalanceStrategy::RoundRobin)
            .build();

        router.add_rule(rule);

        let mut destinations = Vec::new();
        for i in 0..6 {
            let message = RoutableMessage {
                id: i,
                source: DomainId::new(0),
                msg_type: "test".into(),
                content: MessageContent::Empty,
                hints: RoutingHints::default(),
                timestamp: Timestamp::now(),
            };

            let result = router.route(&message);
            destinations.push(result.destinations[0].as_u64());
        }

        // Should cycle through 1, 2, 3, 1, 2, 3
        assert_eq!(destinations, vec![1, 2, 3, 1, 2, 3]);
    }

    #[test]
    fn test_sticky_routing() {
        let mut router = MessageRouter::new();

        let rule = RouteBuilder::new("sticky_test")
            .when_type("session")
            .to(DomainId::new(1))
            .to(DomainId::new(2))
            .with_strategy(LoadBalanceStrategy::RoundRobin)
            .build();

        router.add_rule(rule);

        let mut message = RoutableMessage {
            id: 1,
            source: DomainId::new(0),
            msg_type: "session".into(),
            content: MessageContent::Empty,
            hints: RoutingHints {
                sticky_key: Some("user123".into()),
                ..Default::default()
            },
            timestamp: Timestamp::now(),
        };

        let result1 = router.route(&message);
        let first_dest = result1.destinations[0];

        // Same sticky key should go to same destination
        message.id = 2;
        let result2 = router.route(&message);
        assert_eq!(result2.destinations[0], first_dest);
    }

    #[test]
    fn test_broadcast() {
        let mut router = MessageRouter::new();
        router.update_load(DomainId::new(1), 10);
        router.update_load(DomainId::new(2), 20);
        router.update_load(DomainId::new(3), 30);

        let message = RoutableMessage {
            id: 1,
            source: DomainId::new(0),
            msg_type: "broadcast".into(),
            content: MessageContent::Empty,
            hints: RoutingHints {
                broadcast: true,
                ..Default::default()
            },
            timestamp: Timestamp::now(),
        };

        let result = router.route(&message);
        assert_eq!(result.destinations.len(), 3);
    }
}
