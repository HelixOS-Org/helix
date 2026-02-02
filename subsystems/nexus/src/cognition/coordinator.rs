//! # Domain Coordination
//!
//! Coordinates the execution of cognitive domains.
//! Handles dependencies, scheduling, and synchronization.

#![allow(dead_code)]

extern crate alloc;
use alloc::format;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// COORDINATOR TYPES
// ============================================================================

/// Domain descriptor
#[derive(Debug, Clone)]
pub struct DomainDescriptor {
    /// Domain ID
    pub id: DomainId,
    /// Domain name
    pub name: String,
    /// Domain type
    pub domain_type: DomainType,
    /// Dependencies
    pub dependencies: Vec<DomainId>,
    /// Priority
    pub priority: u32,
    /// State
    pub state: DomainState,
    /// Configuration
    pub config: DomainConfig,
    /// Statistics
    pub stats: DomainStats,
}

/// Domain type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainType {
    /// SENSE - Signal collection
    Sense,
    /// UNDERSTAND - Pattern detection
    Understand,
    /// REASON - Causal analysis
    Reason,
    /// DECIDE - Decision making
    Decide,
    /// ACT - Action execution
    Act,
    /// REFLECT - Outcome evaluation
    Reflect,
    /// LEARN - Knowledge update
    Learn,
    /// LTM - Long-term memory
    LongTermMemory,
}

impl DomainType {
    /// Get execution order
    pub fn order(&self) -> u8 {
        match self {
            Self::Sense => 0,
            Self::Understand => 1,
            Self::Reason => 2,
            Self::Decide => 3,
            Self::Act => 4,
            Self::Reflect => 5,
            Self::Learn => 6,
            Self::LongTermMemory => 7,
        }
    }
}

/// Domain state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainState {
    /// Not initialized
    Uninitialized,
    /// Ready to run
    Ready,
    /// Currently running
    Running,
    /// Waiting for dependencies
    Waiting,
    /// Completed
    Completed,
    /// Error state
    Error,
    /// Disabled
    Disabled,
}

/// Domain configuration
#[derive(Debug, Clone)]
pub struct DomainConfig {
    /// Maximum execution time (ns)
    pub max_time_ns: u64,
    /// Batch size
    pub batch_size: u32,
    /// Enable parallel processing
    pub parallel: bool,
    /// Enable caching
    pub caching: bool,
    /// Log level
    pub log_level: u8,
}

impl Default for DomainConfig {
    fn default() -> Self {
        Self {
            max_time_ns: 1_000_000, // 1ms
            batch_size: 100,
            parallel: false,
            caching: true,
            log_level: 1,
        }
    }
}

/// Domain statistics
#[derive(Debug, Clone, Default)]
pub struct DomainStats {
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful: u64,
    /// Failed executions
    pub failed: u64,
    /// Total execution time (ns)
    pub total_time_ns: u64,
    /// Average execution time (ns)
    pub avg_time_ns: u64,
    /// Items processed
    pub items_processed: u64,
    /// Items produced
    pub items_produced: u64,
}

// ============================================================================
// EXECUTION REQUEST
// ============================================================================

/// Request to execute a domain
#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    /// Request ID
    pub id: u64,
    /// Domain ID
    pub domain_id: DomainId,
    /// Priority
    pub priority: u32,
    /// Deadline
    pub deadline: Option<Timestamp>,
    /// Input data IDs
    pub inputs: Vec<u64>,
    /// Callback on completion
    pub callback_id: Option<u64>,
}

/// Result of domain execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Request ID
    pub request_id: u64,
    /// Domain ID
    pub domain_id: DomainId,
    /// Success
    pub success: bool,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Output data IDs
    pub outputs: Vec<u64>,
    /// Error message if failed
    pub error: Option<String>,
}

// ============================================================================
// COORDINATOR
// ============================================================================

/// Coordinates domain execution
pub struct DomainCoordinator {
    /// Registered domains
    domains: BTreeMap<DomainId, DomainDescriptor>,
    /// Execution queue
    queue: Vec<ExecutionRequest>,
    /// Pending results
    pending_results: BTreeMap<u64, ExecutionResult>,
    /// Next request ID
    next_request_id: AtomicU64,
    /// Current cycle
    current_cycle: u64,
    /// Configuration
    config: CoordinatorConfig,
    /// Statistics
    stats: CoordinatorStats,
}

/// Coordinator configuration
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Enable parallel domain execution
    pub parallel_domains: bool,
    /// Dependency check interval
    pub dep_check_interval: u64,
    /// Result retention (cycles)
    pub result_retention: u64,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            parallel_domains: false,
            dep_check_interval: 1,
            result_retention: 100,
        }
    }
}

/// Coordinator statistics
#[derive(Debug, Clone, Default)]
pub struct CoordinatorStats {
    /// Total requests
    pub total_requests: u64,
    /// Completed requests
    pub completed: u64,
    /// Failed requests
    pub failed: u64,
    /// Dependency waits
    pub dep_waits: u64,
    /// Average queue depth
    pub avg_queue_depth: f32,
}

impl DomainCoordinator {
    /// Create a new coordinator
    pub fn new(config: CoordinatorConfig) -> Self {
        Self {
            domains: BTreeMap::new(),
            queue: Vec::new(),
            pending_results: BTreeMap::new(),
            next_request_id: AtomicU64::new(1),
            current_cycle: 0,
            config,
            stats: CoordinatorStats::default(),
        }
    }

    /// Register a domain
    pub fn register_domain(&mut self, descriptor: DomainDescriptor) {
        self.domains.insert(descriptor.id, descriptor);
    }

    /// Unregister a domain
    pub fn unregister_domain(&mut self, domain_id: DomainId) {
        self.domains.remove(&domain_id);
    }

    /// Get domain
    pub fn get_domain(&self, domain_id: DomainId) -> Option<&DomainDescriptor> {
        self.domains.get(&domain_id)
    }

    /// Get mutable domain
    pub fn get_domain_mut(&mut self, domain_id: DomainId) -> Option<&mut DomainDescriptor> {
        self.domains.get_mut(&domain_id)
    }

    /// Submit execution request
    pub fn submit(&mut self, mut request: ExecutionRequest) -> u64 {
        request.id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let id = request.id;

        // Check queue capacity
        if self.queue.len() >= self.config.max_queue_size {
            // Evict lowest priority
            if let Some(pos) = self
                .queue
                .iter()
                .enumerate()
                .min_by_key(|(_, r)| r.priority)
                .map(|(i, _)| i)
            {
                self.queue.remove(pos);
            }
        }

        self.queue.push(request);
        self.stats.total_requests += 1;

        // Update queue stats
        self.stats.avg_queue_depth = (self.stats.avg_queue_depth
            * (self.stats.total_requests - 1) as f32
            + self.queue.len() as f32)
            / self.stats.total_requests as f32;

        id
    }

    /// Get next request to execute
    pub fn next_request(&mut self) -> Option<ExecutionRequest> {
        if self.queue.is_empty() {
            return None;
        }

        // Find highest priority request with satisfied dependencies
        let ready_idx = self
            .queue
            .iter()
            .enumerate()
            .filter(|(_, r)| self.dependencies_satisfied(r.domain_id))
            .max_by_key(|(_, r)| r.priority)
            .map(|(i, _)| i);

        ready_idx.map(|i| self.queue.remove(i))
    }

    /// Check if dependencies are satisfied
    pub fn dependencies_satisfied(&self, domain_id: DomainId) -> bool {
        let domain = match self.domains.get(&domain_id) {
            Some(d) => d,
            None => return false,
        };

        for dep_id in &domain.dependencies {
            if let Some(dep) = self.domains.get(dep_id) {
                if dep.state != DomainState::Completed {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Record execution result
    pub fn record_result(&mut self, result: ExecutionResult) {
        // Update domain stats
        if let Some(domain) = self.domains.get_mut(&result.domain_id) {
            domain.stats.total_executions += 1;
            domain.stats.total_time_ns += result.duration_ns;
            domain.stats.avg_time_ns = domain.stats.total_time_ns / domain.stats.total_executions;

            if result.success {
                domain.stats.successful += 1;
                domain.state = DomainState::Completed;
                self.stats.completed += 1;
            } else {
                domain.stats.failed += 1;
                domain.state = DomainState::Error;
                self.stats.failed += 1;
            }

            domain.stats.items_produced += result.outputs.len() as u64;
        }

        // Store result
        self.pending_results.insert(result.request_id, result);
    }

    /// Get result
    pub fn get_result(&self, request_id: u64) -> Option<&ExecutionResult> {
        self.pending_results.get(&request_id)
    }

    /// Reset all domains for new cycle
    pub fn reset_for_cycle(&mut self) {
        self.current_cycle += 1;

        for domain in self.domains.values_mut() {
            if domain.state == DomainState::Completed {
                domain.state = DomainState::Ready;
            }
        }

        // Clean old results
        let threshold = self
            .current_cycle
            .saturating_sub(self.config.result_retention);
        self.pending_results.retain(|id, _| *id >= threshold);
    }

    /// Set domain state
    pub fn set_domain_state(&mut self, domain_id: DomainId, state: DomainState) {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.state = state;
        }
    }

    /// Get execution order
    pub fn get_execution_order(&self) -> Vec<DomainId> {
        let mut domains: Vec<_> = self
            .domains
            .values()
            .filter(|d| d.state != DomainState::Disabled)
            .collect();

        // Sort by type order and priority
        domains.sort_by(|a, b| {
            let order_cmp = a.domain_type.order().cmp(&b.domain_type.order());
            if order_cmp != core::cmp::Ordering::Equal {
                order_cmp
            } else {
                b.priority.cmp(&a.priority)
            }
        });

        domains.iter().map(|d| d.id).collect()
    }

    /// Get queue depth
    pub fn queue_depth(&self) -> usize {
        self.queue.len()
    }

    /// Get statistics
    pub fn stats(&self) -> &CoordinatorStats {
        &self.stats
    }
}

// ============================================================================
// DEPENDENCY GRAPH
// ============================================================================

/// Dependency graph for domains
pub struct DependencyGraph {
    /// Nodes (domain IDs)
    nodes: Vec<DomainId>,
    /// Edges (from -> to)
    edges: BTreeMap<DomainId, Vec<DomainId>>,
    /// Reverse edges (to -> from)
    reverse_edges: BTreeMap<DomainId, Vec<DomainId>>,
}

impl DependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: BTreeMap::new(),
            reverse_edges: BTreeMap::new(),
        }
    }

    /// Add a node
    pub fn add_node(&mut self, domain_id: DomainId) {
        if !self.nodes.contains(&domain_id) {
            self.nodes.push(domain_id);
        }
    }

    /// Add a dependency edge
    pub fn add_dependency(&mut self, from: DomainId, to: DomainId) {
        self.edges.entry(from).or_default().push(to);
        self.reverse_edges.entry(to).or_default().push(from);
    }

    /// Get dependencies for a domain
    pub fn get_dependencies(&self, domain_id: DomainId) -> &[DomainId] {
        self.edges
            .get(&domain_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get dependents (domains that depend on this one)
    pub fn get_dependents(&self, domain_id: DomainId) -> &[DomainId] {
        self.reverse_edges
            .get(&domain_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Topological sort
    pub fn topological_sort(&self) -> Option<Vec<DomainId>> {
        let mut result = Vec::new();
        let mut in_degree: BTreeMap<DomainId, usize> = BTreeMap::new();

        // Initialize in-degree
        for &node in &self.nodes {
            in_degree.insert(node, 0);
        }

        for targets in self.edges.values() {
            for &target in targets {
                *in_degree.entry(target).or_default() += 1;
            }
        }

        // Find nodes with no incoming edges
        let mut queue: Vec<_> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        while let Some(node) = queue.pop() {
            result.push(node);

            if let Some(neighbors) = self.edges.get(&node) {
                for &neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(&neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None // Cycle detected
        }
    }

    /// Check for cycles
    pub fn has_cycle(&self) -> bool {
        self.topological_sort().is_none()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_domain(id: u8, domain_type: DomainType) -> DomainDescriptor {
        DomainDescriptor {
            id: DomainId::new(id as u64),
            name: format!("domain_{}", id),
            domain_type,
            dependencies: Vec::new(),
            priority: 100,
            state: DomainState::Ready,
            config: DomainConfig::default(),
            stats: DomainStats::default(),
        }
    }

    #[test]
    fn test_coordinator() {
        let config = CoordinatorConfig::default();
        let mut coord = DomainCoordinator::new(config);

        coord.register_domain(make_domain(1, DomainType::Sense));
        coord.register_domain(make_domain(2, DomainType::Understand));

        let request = ExecutionRequest {
            id: 0,
            domain_id: DomainId::new(1),
            priority: 100,
            deadline: None,
            inputs: Vec::new(),
            callback_id: None,
        };

        let id = coord.submit(request);
        assert!(id > 0);
    }

    #[test]
    fn test_execution_order() {
        let config = CoordinatorConfig::default();
        let mut coord = DomainCoordinator::new(config);

        coord.register_domain(make_domain(1, DomainType::Decide));
        coord.register_domain(make_domain(2, DomainType::Sense));
        coord.register_domain(make_domain(3, DomainType::Understand));

        let order = coord.get_execution_order();

        // Should be sorted by type order
        let sense_pos = order.iter().position(|id| id.as_u64() == 2).unwrap();
        let understand_pos = order.iter().position(|id| id.as_u64() == 3).unwrap();
        let decide_pos = order.iter().position(|id| id.as_u64() == 1).unwrap();

        assert!(sense_pos < understand_pos);
        assert!(understand_pos < decide_pos);
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();

        graph.add_node(DomainId::new(1));
        graph.add_node(DomainId::new(2));
        graph.add_node(DomainId::new(3));

        graph.add_dependency(DomainId::new(1), DomainId::new(2));
        graph.add_dependency(DomainId::new(2), DomainId::new(3));

        let sorted = graph.topological_sort();
        assert!(sorted.is_some());

        let order = sorted.unwrap();
        let pos1 = order.iter().position(|id| id.as_u64() == 1).unwrap();
        let pos2 = order.iter().position(|id| id.as_u64() == 2).unwrap();
        let pos3 = order.iter().position(|id| id.as_u64() == 3).unwrap();

        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        graph.add_node(DomainId::new(1));
        graph.add_node(DomainId::new(2));
        graph.add_node(DomainId::new(3));

        // Create cycle: 1 -> 2 -> 3 -> 1
        graph.add_dependency(DomainId::new(1), DomainId::new(2));
        graph.add_dependency(DomainId::new(2), DomainId::new(3));
        graph.add_dependency(DomainId::new(3), DomainId::new(1));

        assert!(graph.has_cycle());
    }
}
