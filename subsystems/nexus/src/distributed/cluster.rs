//! # Cluster Management
//!
//! Year 3 EVOLUTION - Q4 - Cluster lifecycle and membership management

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{ClusterId, Epoch, NodeCapabilities, NodeId};
use crate::math::F64Ext;

// ============================================================================
// MEMBERSHIP TYPES
// ============================================================================

/// Member ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemberId(pub u64);

static MEMBER_COUNTER: AtomicU64 = AtomicU64::new(1);

impl MemberId {
    pub fn generate() -> Self {
        Self(MEMBER_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Cluster member
#[derive(Debug, Clone)]
pub struct Member {
    /// Member ID
    pub id: MemberId,
    /// Node ID
    pub node_id: NodeId,
    /// Role
    pub role: MemberRole,
    /// Status
    pub status: MemberStatus,
    /// Join time
    pub join_time: u64,
    /// Last heartbeat
    pub last_heartbeat: u64,
    /// Vote weight
    pub vote_weight: u32,
    /// Capabilities
    pub capabilities: NodeCapabilities,
}

/// Member role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberRole {
    /// Voting member
    Voter,
    /// Non-voting member (learner)
    Learner,
    /// Staging (being added)
    Staging,
    /// Leaving
    Leaving,
}

/// Member status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberStatus {
    /// Active
    Active,
    /// Suspect (missed heartbeats)
    Suspect,
    /// Failed
    Failed,
    /// Removed
    Removed,
}

/// Membership change
#[derive(Debug, Clone)]
pub struct MembershipChange {
    /// Change type
    pub change_type: MembershipChangeType,
    /// Node ID
    pub node_id: NodeId,
    /// Member role
    pub role: MemberRole,
    /// Epoch
    pub epoch: Epoch,
    /// Timestamp
    pub timestamp: u64,
}

/// Membership change type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipChangeType {
    /// Add member
    Add,
    /// Remove member
    Remove,
    /// Promote learner to voter
    Promote,
    /// Demote voter to learner
    Demote,
    /// Update capabilities
    Update,
}

// ============================================================================
// CLUSTER CONFIGURATION
// ============================================================================

/// Cluster configuration
#[derive(Debug, Clone)]
pub struct ClusterConfiguration {
    /// Configuration version
    pub version: u64,
    /// Members
    pub members: Vec<Member>,
    /// Minimum voters
    pub min_voters: usize,
    /// Maximum voters
    pub max_voters: usize,
    /// Heartbeat interval (ms)
    pub heartbeat_interval: u64,
    /// Failure threshold
    pub failure_threshold: u32,
    /// Auto-remove failed
    pub auto_remove_failed: bool,
}

impl Default for ClusterConfiguration {
    fn default() -> Self {
        Self {
            version: 0,
            members: Vec::new(),
            min_voters: 3,
            max_voters: 7,
            heartbeat_interval: 100,
            failure_threshold: 3,
            auto_remove_failed: true,
        }
    }
}

// ============================================================================
// HEALTH CHECK
// ============================================================================

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Healthy
    Healthy,
    /// Degraded
    Degraded,
    /// Unhealthy
    Unhealthy,
    /// Unknown
    Unknown,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Node ID
    pub node_id: NodeId,
    /// Status
    pub status: HealthStatus,
    /// Latency (ms)
    pub latency: u32,
    /// Checks passed
    pub checks_passed: u32,
    /// Checks failed
    pub checks_failed: u32,
    /// Last check time
    pub last_check: u64,
}

/// Health checker
pub struct HealthChecker {
    /// Check history
    history: BTreeMap<NodeId, Vec<HealthCheck>>,
    /// Configuration
    config: HealthConfig,
}

/// Health configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Check interval (ms)
    pub check_interval: u64,
    /// Timeout (ms)
    pub timeout: u64,
    /// History size
    pub history_size: usize,
    /// Healthy threshold (consecutive successes)
    pub healthy_threshold: u32,
    /// Unhealthy threshold (consecutive failures)
    pub unhealthy_threshold: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: 1000,
            timeout: 5000,
            history_size: 10,
            healthy_threshold: 3,
            unhealthy_threshold: 3,
        }
    }
}

impl HealthChecker {
    /// Create new health checker
    pub fn new(config: HealthConfig) -> Self {
        Self {
            history: BTreeMap::new(),
            config,
        }
    }

    /// Record health check
    pub fn record(&mut self, check: HealthCheck) {
        let history = self.history.entry(check.node_id).or_insert_with(Vec::new);
        history.push(check);

        // Trim history
        if history.len() > self.config.history_size {
            history.remove(0);
        }
    }

    /// Get status for node
    pub fn get_status(&self, node_id: NodeId) -> HealthStatus {
        let history = match self.history.get(&node_id) {
            Some(h) => h,
            None => return HealthStatus::Unknown,
        };

        if history.is_empty() {
            return HealthStatus::Unknown;
        }

        // Count recent successes/failures
        let recent: Vec<_> = history
            .iter()
            .rev()
            .take(self.config.healthy_threshold as usize)
            .collect();

        let healthy_count = recent
            .iter()
            .filter(|c| c.status == HealthStatus::Healthy)
            .count();

        let unhealthy_count = recent
            .iter()
            .filter(|c| c.status == HealthStatus::Unhealthy)
            .count();

        if healthy_count >= self.config.healthy_threshold as usize {
            HealthStatus::Healthy
        } else if unhealthy_count >= self.config.unhealthy_threshold as usize {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Degraded
        }
    }

    /// Get average latency
    pub fn average_latency(&self, node_id: NodeId) -> Option<u32> {
        let history = self.history.get(&node_id)?;
        if history.is_empty() {
            return None;
        }

        let sum: u32 = history.iter().map(|c| c.latency).sum();
        Some(sum / history.len() as u32)
    }
}

// ============================================================================
// FAILURE DETECTOR
// ============================================================================

/// Phi accrual failure detector
pub struct PhiAccrualDetector {
    /// Heartbeat history per node
    heartbeats: BTreeMap<NodeId, Vec<u64>>,
    /// Window size
    window_size: usize,
    /// Threshold phi
    threshold: f64,
}

impl PhiAccrualDetector {
    /// Create new detector
    pub fn new(window_size: usize, threshold: f64) -> Self {
        Self {
            heartbeats: BTreeMap::new(),
            window_size,
            threshold,
        }
    }

    /// Record heartbeat
    pub fn heartbeat(&mut self, node_id: NodeId, timestamp: u64) {
        let history = self.heartbeats.entry(node_id).or_insert_with(Vec::new);
        history.push(timestamp);

        if history.len() > self.window_size {
            history.remove(0);
        }
    }

    /// Calculate phi
    pub fn phi(&self, node_id: NodeId, now: u64) -> Option<f64> {
        let history = self.heartbeats.get(&node_id)?;
        if history.len() < 2 {
            return None;
        }

        // Calculate intervals
        let intervals: Vec<f64> = history.windows(2).map(|w| (w[1] - w[0]) as f64).collect();

        // Mean and variance
        let mean: f64 = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let variance: f64 =
            intervals.iter().map(|i| (i - mean).powi(2)).sum::<f64>() / intervals.len() as f64;
        let std_dev = variance.sqrt();

        // Time since last heartbeat
        let last = history.last()?;
        let elapsed = (now - last) as f64;

        // Phi = -log10(P(t > elapsed))
        // For normal distribution: P(t > x) = 1 - CDF(x)
        if std_dev > 0.0 {
            let z = (elapsed - mean) / std_dev;
            // Simplified: use exponential approximation
            let p = (-z * z / 2.0).exp() / 2.0;
            if p > 0.0 {
                Some(-p.log10())
            } else {
                Some(f64::INFINITY)
            }
        } else {
            if elapsed > mean {
                Some(f64::INFINITY)
            } else {
                Some(0.0)
            }
        }
    }

    /// Is node suspected failed?
    pub fn is_failed(&self, node_id: NodeId, now: u64) -> bool {
        match self.phi(node_id, now) {
            Some(phi) => phi > self.threshold,
            None => false,
        }
    }
}

impl Default for PhiAccrualDetector {
    fn default() -> Self {
        Self::new(100, 8.0)
    }
}

// ============================================================================
// CLUSTER MANAGER
// ============================================================================

/// Cluster manager
pub struct ClusterManager {
    /// Cluster ID
    cluster_id: ClusterId,
    /// Local node ID
    local_node: NodeId,
    /// Configuration
    config: ClusterConfiguration,
    /// Health checker
    health: HealthChecker,
    /// Failure detector
    failure_detector: PhiAccrualDetector,
    /// Pending changes
    pending_changes: Vec<MembershipChange>,
    /// Current epoch
    epoch: Epoch,
    /// Running
    running: AtomicBool,
    /// Statistics
    stats: ClusterStats,
}

/// Cluster statistics
#[derive(Debug, Clone, Default)]
pub struct ClusterStats {
    /// Members joined
    pub members_joined: u64,
    /// Members left
    pub members_left: u64,
    /// Members failed
    pub members_failed: u64,
    /// Configuration changes
    pub config_changes: u64,
    /// Elections
    pub elections: u64,
}

impl ClusterManager {
    /// Create new cluster manager
    pub fn new(cluster_id: ClusterId, local_node: NodeId) -> Self {
        Self {
            cluster_id,
            local_node,
            config: ClusterConfiguration::default(),
            health: HealthChecker::new(HealthConfig::default()),
            failure_detector: PhiAccrualDetector::default(),
            pending_changes: Vec::new(),
            epoch: Epoch(0),
            running: AtomicBool::new(false),
            stats: ClusterStats::default(),
        }
    }

    /// Start the manager
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    /// Stop the manager
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Add member
    pub fn add_member(
        &mut self,
        node_id: NodeId,
        role: MemberRole,
    ) -> Result<MemberId, ClusterError> {
        // Check limits
        let voters = self
            .config
            .members
            .iter()
            .filter(|m| m.role == MemberRole::Voter && m.status == MemberStatus::Active)
            .count();

        if role == MemberRole::Voter && voters >= self.config.max_voters {
            return Err(ClusterError::TooManyVoters);
        }

        // Check if already member
        if self.config.members.iter().any(|m| m.node_id == node_id) {
            return Err(ClusterError::AlreadyMember);
        }

        let member_id = MemberId::generate();
        let member = Member {
            id: member_id,
            node_id,
            role,
            status: MemberStatus::Active,
            join_time: 0,
            last_heartbeat: 0,
            vote_weight: 1,
            capabilities: NodeCapabilities::default(),
        };

        self.config.members.push(member);
        self.config.version += 1;

        // Record change
        self.pending_changes.push(MembershipChange {
            change_type: MembershipChangeType::Add,
            node_id,
            role,
            epoch: self.epoch,
            timestamp: 0,
        });

        self.stats.members_joined += 1;
        self.stats.config_changes += 1;

        Ok(member_id)
    }

    /// Remove member
    pub fn remove_member(&mut self, node_id: NodeId) -> Result<(), ClusterError> {
        let idx = self
            .config
            .members
            .iter()
            .position(|m| m.node_id == node_id)
            .ok_or(ClusterError::NotMember)?;

        let member = &self.config.members[idx];

        // Check minimum voters
        if member.role == MemberRole::Voter {
            let voters = self
                .config
                .members
                .iter()
                .filter(|m| {
                    m.role == MemberRole::Voter
                        && m.status == MemberStatus::Active
                        && m.node_id != node_id
                })
                .count();

            if voters < self.config.min_voters {
                return Err(ClusterError::TooFewVoters);
            }
        }

        self.config.members.remove(idx);
        self.config.version += 1;

        self.pending_changes.push(MembershipChange {
            change_type: MembershipChangeType::Remove,
            node_id,
            role: MemberRole::Leaving,
            epoch: self.epoch,
            timestamp: 0,
        });

        self.stats.members_left += 1;
        self.stats.config_changes += 1;

        Ok(())
    }

    /// Promote learner to voter
    pub fn promote(&mut self, node_id: NodeId) -> Result<(), ClusterError> {
        // First check if member exists and has the right role (immutable borrow)
        let member_idx = self
            .config
            .members
            .iter()
            .position(|m| m.node_id == node_id)
            .ok_or(ClusterError::NotMember)?;

        if self.config.members[member_idx].role != MemberRole::Learner {
            return Err(ClusterError::InvalidRole);
        }

        let voters = self
            .config
            .members
            .iter()
            .filter(|m| m.role == MemberRole::Voter && m.status == MemberStatus::Active)
            .count();

        if voters >= self.config.max_voters {
            return Err(ClusterError::TooManyVoters);
        }

        // Now mutate after all checks are done
        self.config.members[member_idx].role = MemberRole::Voter;
        self.config.version += 1;

        self.pending_changes.push(MembershipChange {
            change_type: MembershipChangeType::Promote,
            node_id,
            role: MemberRole::Voter,
            epoch: self.epoch,
            timestamp: 0,
        });

        self.stats.config_changes += 1;

        Ok(())
    }

    /// Handle heartbeat
    pub fn heartbeat(&mut self, node_id: NodeId, timestamp: u64) {
        self.failure_detector.heartbeat(node_id, timestamp);

        if let Some(member) = self
            .config
            .members
            .iter_mut()
            .find(|m| m.node_id == node_id)
        {
            member.last_heartbeat = timestamp;
            if member.status == MemberStatus::Suspect {
                member.status = MemberStatus::Active;
            }
        }
    }

    /// Check for failures
    pub fn check_failures(&mut self, now: u64) -> Vec<NodeId> {
        let mut failed = Vec::new();

        for member in &mut self.config.members {
            if member.status == MemberStatus::Removed {
                continue;
            }

            if self.failure_detector.is_failed(member.node_id, now) {
                if member.status == MemberStatus::Active {
                    member.status = MemberStatus::Suspect;
                } else if member.status == MemberStatus::Suspect {
                    member.status = MemberStatus::Failed;
                    failed.push(member.node_id);
                    self.stats.members_failed += 1;
                }
            }
        }

        // Auto-remove failed members
        if self.config.auto_remove_failed {
            for node_id in &failed {
                let _ = self.remove_member(*node_id);
            }
        }

        failed
    }

    /// Get members
    pub fn members(&self) -> &[Member] {
        &self.config.members
    }

    /// Get active voters
    pub fn voters(&self) -> impl Iterator<Item = &Member> {
        self.config
            .members
            .iter()
            .filter(|m| m.role == MemberRole::Voter && m.status == MemberStatus::Active)
    }

    /// Get quorum size
    pub fn quorum_size(&self) -> usize {
        let voters = self.voters().count();
        (voters / 2) + 1
    }

    /// Get cluster ID
    pub fn cluster_id(&self) -> ClusterId {
        self.cluster_id
    }

    /// Get configuration
    pub fn config(&self) -> &ClusterConfiguration {
        &self.config
    }

    /// Get statistics
    pub fn stats(&self) -> &ClusterStats {
        &self.stats
    }
}

impl Default for ClusterManager {
    fn default() -> Self {
        Self::new(ClusterId(0), NodeId(0))
    }
}

/// Cluster error
#[derive(Debug)]
pub enum ClusterError {
    /// Too many voters
    TooManyVoters,
    /// Too few voters
    TooFewVoters,
    /// Already member
    AlreadyMember,
    /// Not a member
    NotMember,
    /// Invalid role
    InvalidRole,
    /// Quorum not met
    QuorumNotMet,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_remove_member() {
        let mut manager = ClusterManager::new(ClusterId(1), NodeId(0));

        let id = manager.add_member(NodeId(1), MemberRole::Voter).unwrap();
        assert!(id.0 > 0);
        assert_eq!(manager.members().len(), 1);

        manager.add_member(NodeId(2), MemberRole::Voter).unwrap();
        manager.add_member(NodeId(3), MemberRole::Voter).unwrap();

        assert_eq!(manager.quorum_size(), 2);

        manager.remove_member(NodeId(1)).unwrap();
        assert_eq!(manager.members().len(), 2);
    }

    #[test]
    fn test_phi_accrual() {
        let mut detector = PhiAccrualDetector::new(10, 8.0);

        // Simulate regular heartbeats
        for i in 0..10 {
            detector.heartbeat(NodeId(1), i * 100);
        }

        // Check phi after normal interval
        let phi = detector.phi(NodeId(1), 1000);
        assert!(phi.is_some());

        // Check phi after long delay
        let phi_late = detector.phi(NodeId(1), 2000);
        assert!(phi_late.unwrap() > phi.unwrap());
    }

    #[test]
    fn test_health_checker() {
        let mut checker = HealthChecker::new(HealthConfig {
            healthy_threshold: 2,
            ..Default::default()
        });

        checker.record(HealthCheck {
            node_id: NodeId(1),
            status: HealthStatus::Healthy,
            latency: 10,
            checks_passed: 1,
            checks_failed: 0,
            last_check: 0,
        });

        checker.record(HealthCheck {
            node_id: NodeId(1),
            status: HealthStatus::Healthy,
            latency: 15,
            checks_passed: 1,
            checks_failed: 0,
            last_check: 1,
        });

        assert_eq!(checker.get_status(NodeId(1)), HealthStatus::Healthy);
        assert_eq!(checker.average_latency(NodeId(1)), Some(12));
    }
}
