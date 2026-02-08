//! # Holistic Isolation Engine
//!
//! System-wide resource isolation and partitioning:
//! - Isolation domains
//! - Resource partitioning
//! - Interference detection
//! - Noisy neighbor mitigation
//! - QoS guarantees

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ISOLATION TYPES
// ============================================================================

/// Isolation domain type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsolationDomainType {
    /// CPU partitioning
    Cpu,
    /// Memory isolation
    Memory,
    /// Cache partitioning (CAT)
    Cache,
    /// I/O bandwidth
    Io,
    /// Network bandwidth
    Network,
    /// Full isolation (container-like)
    Full,
}

/// Isolation strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsolationStrength {
    /// Soft limits (best effort)
    Soft,
    /// Medium (throttling)
    Medium,
    /// Hard (enforced partitioning)
    Hard,
    /// Complete (hardware isolation)
    Complete,
}

/// Interference type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterferenceType {
    /// Cache pollution
    CachePollution,
    /// Memory bandwidth contention
    MemoryBandwidth,
    /// CPU contention
    CpuContention,
    /// I/O contention
    IoContention,
    /// Network contention
    NetworkContention,
    /// NUMA remote access
    NumaRemote,
}

// ============================================================================
// ISOLATION DOMAIN
// ============================================================================

/// Resource partition within a domain
#[derive(Debug, Clone)]
pub struct ResourcePartition {
    /// Minimum guaranteed
    pub minimum: u64,
    /// Maximum allowed
    pub maximum: u64,
    /// Current allocation
    pub current: u64,
    /// Current usage
    pub usage: u64,
}

impl ResourcePartition {
    pub fn new(min: u64, max: u64) -> Self {
        Self {
            minimum: min,
            maximum: max,
            current: min,
            usage: 0,
        }
    }

    /// Utilization
    pub fn utilization(&self) -> f64 {
        if self.current == 0 {
            return 0.0;
        }
        self.usage as f64 / self.current as f64
    }

    /// Is over-provisioned?
    pub fn is_over_provisioned(&self) -> bool {
        self.utilization() < 0.3 && self.current > self.minimum
    }

    /// Is under-provisioned?
    pub fn is_under_provisioned(&self) -> bool {
        self.utilization() > 0.9
    }
}

/// An isolation domain
#[derive(Debug)]
pub struct IsolationDomain {
    /// Domain id
    pub id: u64,
    /// Domain type
    pub domain_type: IsolationDomainType,
    /// Strength
    pub strength: IsolationStrength,
    /// Members (process ids)
    pub members: Vec<u64>,
    /// Partitions per resource type
    pub partitions: BTreeMap<u8, ResourcePartition>,
    /// Created at
    pub created_at: u64,
    /// Active
    pub active: bool,
    /// Interference events detected
    pub interference_count: u64,
}

impl IsolationDomain {
    pub fn new(id: u64, domain_type: IsolationDomainType, strength: IsolationStrength, now: u64) -> Self {
        Self {
            id,
            domain_type,
            strength,
            members: Vec::new(),
            partitions: BTreeMap::new(),
            created_at: now,
            active: true,
            interference_count: 0,
        }
    }

    /// Add member
    pub fn add_member(&mut self, pid: u64) {
        if !self.members.contains(&pid) {
            self.members.push(pid);
        }
    }

    /// Remove member
    pub fn remove_member(&mut self, pid: u64) {
        self.members.retain(|&p| p != pid);
    }

    /// Set partition
    pub fn set_partition(&mut self, resource: u8, min: u64, max: u64) {
        self.partitions.insert(resource, ResourcePartition::new(min, max));
    }

    /// Update usage
    pub fn update_usage(&mut self, resource: u8, usage: u64) {
        if let Some(part) = self.partitions.get_mut(&resource) {
            part.usage = usage;
        }
    }

    /// Member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

// ============================================================================
// INTERFERENCE DETECTION
// ============================================================================

/// Interference event
#[derive(Debug, Clone)]
pub struct InterferenceEvent {
    /// Victim domain
    pub victim: u64,
    /// Aggressor domain (if known)
    pub aggressor: Option<u64>,
    /// Type
    pub interference_type: InterferenceType,
    /// Severity (0-100)
    pub severity: u32,
    /// Timestamp
    pub timestamp: u64,
    /// Performance impact (%)
    pub impact_pct: f64,
}

/// Interference detector
#[derive(Debug)]
pub struct InterferenceDetector {
    /// Baseline performance per domain
    baselines: BTreeMap<u64, f64>,
    /// Current performance per domain
    current: BTreeMap<u64, f64>,
    /// Threshold for detection (%)
    pub threshold_pct: f64,
}

impl InterferenceDetector {
    pub fn new() -> Self {
        Self {
            baselines: BTreeMap::new(),
            current: BTreeMap::new(),
            threshold_pct: 10.0,
        }
    }

    /// Set baseline
    pub fn set_baseline(&mut self, domain: u64, perf: f64) {
        self.baselines.insert(domain, perf);
    }

    /// Update current performance
    pub fn update(&mut self, domain: u64, perf: f64) {
        self.current.insert(domain, perf);
    }

    /// Detect interference
    pub fn detect(&self, now: u64) -> Vec<InterferenceEvent> {
        let mut events = Vec::new();
        for (&domain, &current) in &self.current {
            if let Some(&baseline) = self.baselines.get(&domain) {
                if baseline > 0.0 {
                    let degradation = (baseline - current) / baseline * 100.0;
                    if degradation > self.threshold_pct {
                        events.push(InterferenceEvent {
                            victim: domain,
                            aggressor: None,
                            interference_type: InterferenceType::CpuContention,
                            severity: (degradation as u32).min(100),
                            timestamp: now,
                            impact_pct: degradation,
                        });
                    }
                }
            }
        }
        events
    }
}

// ============================================================================
// ISOLATION ENGINE
// ============================================================================

/// Isolation stats
#[derive(Debug, Clone, Default)]
pub struct HolisticIsolationStats {
    /// Active domains
    pub active_domains: usize,
    /// Total members
    pub total_members: usize,
    /// Interference events
    pub interference_events: u64,
}

/// Holistic isolation engine
pub struct HolisticIsolationEngine {
    /// Domains
    domains: BTreeMap<u64, IsolationDomain>,
    /// Process -> domain mapping
    membership: BTreeMap<u64, Vec<u64>>,
    /// Interference detector
    detector: InterferenceDetector,
    /// Next id
    next_id: u64,
    /// Stats
    stats: HolisticIsolationStats,
}

impl HolisticIsolationEngine {
    pub fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            membership: BTreeMap::new(),
            detector: InterferenceDetector::new(),
            next_id: 1,
            stats: HolisticIsolationStats::default(),
        }
    }

    /// Create domain
    pub fn create_domain(
        &mut self,
        domain_type: IsolationDomainType,
        strength: IsolationStrength,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let domain = IsolationDomain::new(id, domain_type, strength, now);
        self.domains.insert(id, domain);
        self.update_stats();
        id
    }

    /// Add process to domain
    pub fn add_to_domain(&mut self, domain_id: u64, pid: u64) -> bool {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.add_member(pid);
            self.membership
                .entry(pid)
                .or_insert_with(Vec::new)
                .push(domain_id);
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Remove from domain
    pub fn remove_from_domain(&mut self, domain_id: u64, pid: u64) {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.remove_member(pid);
        }
        if let Some(memberships) = self.membership.get_mut(&pid) {
            memberships.retain(|&d| d != domain_id);
        }
        self.update_stats();
    }

    /// Check for interference
    pub fn detect_interference(&mut self, now: u64) -> Vec<InterferenceEvent> {
        let events = self.detector.detect(now);
        self.stats.interference_events += events.len() as u64;
        events
    }

    /// Get domain
    pub fn domain(&self, id: u64) -> Option<&IsolationDomain> {
        self.domains.get(&id)
    }

    fn update_stats(&mut self) {
        self.stats.active_domains = self.domains.values().filter(|d| d.active).count();
        self.stats.total_members = self.domains.values().map(|d| d.member_count()).sum();
    }

    /// Stats
    pub fn stats(&self) -> &HolisticIsolationStats {
        &self.stats
    }
}
