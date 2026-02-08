//! # Holistic Quality of Service
//!
//! System-wide QoS management:
//! - QoS class definitions and enforcement
//! - Multi-resource QoS policies
//! - Admission control
//! - SLA-to-QoS mapping
//! - Dynamic QoS adaptation
//! - Cross-subsystem QoS coordination

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// QOS CLASSES
// ============================================================================

/// QoS class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QosClass {
    /// Best effort — no guarantees
    BestEffort,
    /// Background — lowest priority but some fairness
    Background,
    /// Standard — default for user processes
    Standard,
    /// Interactive — latency-sensitive UI
    Interactive,
    /// Realtime — strict timing guarantees
    Realtime,
    /// Critical — system services
    Critical,
    /// Emergency — fault recovery
    Emergency,
}

impl QosClass {
    /// Priority weight
    pub fn weight(&self) -> u32 {
        match self {
            QosClass::BestEffort => 1,
            QosClass::Background => 2,
            QosClass::Standard => 8,
            QosClass::Interactive => 16,
            QosClass::Realtime => 32,
            QosClass::Critical => 64,
            QosClass::Emergency => 128,
        }
    }

    /// Max latency target (us)
    pub fn latency_target_us(&self) -> u64 {
        match self {
            QosClass::BestEffort => 100_000,
            QosClass::Background => 50_000,
            QosClass::Standard => 10_000,
            QosClass::Interactive => 1_000,
            QosClass::Realtime => 100,
            QosClass::Critical => 50,
            QosClass::Emergency => 10,
        }
    }

    /// All classes in priority order
    pub fn all_desc() -> &'static [QosClass] {
        &[
            QosClass::Emergency,
            QosClass::Critical,
            QosClass::Realtime,
            QosClass::Interactive,
            QosClass::Standard,
            QosClass::Background,
            QosClass::BestEffort,
        ]
    }
}

// ============================================================================
// QOS RESOURCE SPEC
// ============================================================================

/// QoS resource dimension
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QosResource {
    /// CPU time share
    CpuShare,
    /// Memory pages
    Memory,
    /// I/O bandwidth (bytes/s)
    IoBandwidth,
    /// I/O operations per second
    Iops,
    /// Network bandwidth (bytes/s)
    NetworkBandwidth,
    /// IPC message rate
    IpcRate,
}

/// Resource guarantee
#[derive(Debug, Clone)]
pub struct ResourceGuarantee {
    /// Resource
    pub resource: QosResource,
    /// Minimum guaranteed
    pub minimum: u64,
    /// Maximum allowed (0 = unlimited)
    pub maximum: u64,
    /// Burst allowed above maximum
    pub burst: u64,
    /// Current usage
    pub current: u64,
}

impl ResourceGuarantee {
    pub fn new(resource: QosResource, minimum: u64, maximum: u64) -> Self {
        Self {
            resource,
            minimum,
            maximum,
            burst: 0,
            current: 0,
        }
    }

    pub fn with_burst(mut self, burst: u64) -> Self {
        self.burst = burst;
        self
    }

    /// Check if within limits
    pub fn is_within_limits(&self) -> bool {
        if self.maximum == 0 {
            return true;
        }
        self.current <= self.maximum + self.burst
    }

    /// Utilization against guarantee
    pub fn utilization(&self) -> f64 {
        if self.minimum == 0 {
            return 0.0;
        }
        self.current as f64 / self.minimum as f64
    }

    /// Headroom
    pub fn headroom(&self) -> u64 {
        if self.maximum == 0 {
            return u64::MAX;
        }
        (self.maximum + self.burst).saturating_sub(self.current)
    }
}

// ============================================================================
// QOS POLICY
// ============================================================================

/// QoS enforcement mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosEnforcementMode {
    /// Monitor only, no enforcement
    Monitor,
    /// Soft enforcement (warnings)
    Soft,
    /// Hard enforcement (throttle/kill)
    Hard,
    /// Adaptive (adjust based on load)
    Adaptive,
}

/// QoS policy
#[derive(Debug, Clone)]
pub struct QosPolicy {
    /// Policy ID
    pub id: u64,
    /// QoS class
    pub class: QosClass,
    /// Resource guarantees
    pub guarantees: Vec<ResourceGuarantee>,
    /// Enforcement mode
    pub enforcement: QosEnforcementMode,
    /// Preemption allowed
    pub preemptible: bool,
    /// Priority within class
    pub intra_class_priority: u32,
}

impl QosPolicy {
    pub fn new(id: u64, class: QosClass) -> Self {
        Self {
            id,
            class,
            guarantees: Vec::new(),
            enforcement: QosEnforcementMode::Hard,
            preemptible: true,
            intra_class_priority: 0,
        }
    }

    pub fn add_guarantee(&mut self, guarantee: ResourceGuarantee) {
        self.guarantees.push(guarantee);
    }

    pub fn set_enforcement(&mut self, mode: QosEnforcementMode) {
        self.enforcement = mode;
    }

    /// Check all guarantees
    pub fn check_compliance(&self) -> bool {
        self.guarantees.iter().all(|g| g.is_within_limits())
    }

    /// Find violated guarantees
    pub fn violations(&self) -> Vec<QosResource> {
        self.guarantees
            .iter()
            .filter(|g| !g.is_within_limits())
            .map(|g| g.resource)
            .collect()
    }
}

// ============================================================================
// ADMISSION CONTROL
// ============================================================================

/// Admission decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosAdmissionResult {
    /// Admitted
    Admitted,
    /// Admitted with degraded QoS
    Degraded,
    /// Rejected — insufficient resources
    Rejected,
    /// Queued — waiting for resources
    Queued,
}

/// Admission control state
#[derive(Debug, Clone)]
pub struct AdmissionState {
    /// Available capacity per resource
    pub available: BTreeMap<u8, u64>,
    /// Reserved capacity per resource
    pub reserved: BTreeMap<u8, u64>,
    /// Admission count
    pub admissions: u64,
    /// Rejection count
    pub rejections: u64,
}

impl AdmissionState {
    pub fn new() -> Self {
        Self {
            available: BTreeMap::new(),
            reserved: BTreeMap::new(),
            admissions: 0,
            rejections: 0,
        }
    }

    /// Set available capacity
    pub fn set_capacity(&mut self, resource: QosResource, capacity: u64) {
        self.available.insert(resource as u8, capacity);
    }

    /// Try admit
    pub fn try_admit(&mut self, policy: &QosPolicy) -> QosAdmissionResult {
        // Check if resources available
        for guarantee in &policy.guarantees {
            let key = guarantee.resource as u8;
            let available = self.available.get(&key).copied().unwrap_or(0);
            let reserved = self.reserved.get(&key).copied().unwrap_or(0);
            let free = available.saturating_sub(reserved);

            if free < guarantee.minimum {
                self.rejections += 1;
                return QosAdmissionResult::Rejected;
            }
        }

        // Reserve resources
        for guarantee in &policy.guarantees {
            let key = guarantee.resource as u8;
            *self.reserved.entry(key).or_insert(0) += guarantee.minimum;
        }

        self.admissions += 1;
        QosAdmissionResult::Admitted
    }

    /// Release reservation
    pub fn release(&mut self, policy: &QosPolicy) {
        for guarantee in &policy.guarantees {
            let key = guarantee.resource as u8;
            if let Some(reserved) = self.reserved.get_mut(&key) {
                *reserved = reserved.saturating_sub(guarantee.minimum);
            }
        }
    }

    /// Admission rate
    pub fn admission_rate(&self) -> f64 {
        let total = self.admissions + self.rejections;
        if total == 0 {
            return 1.0;
        }
        self.admissions as f64 / total as f64
    }
}

// ============================================================================
// QOS MANAGER
// ============================================================================

/// QoS manager stats
#[derive(Debug, Clone, Default)]
pub struct HolisticQosStats {
    /// Active policies
    pub active_policies: usize,
    /// Processes by class
    pub processes_per_class: BTreeMap<u8, usize>,
    /// Total violations
    pub total_violations: u64,
    /// Admission rate
    pub admission_rate: f64,
    /// Average compliance
    pub avg_compliance: f64,
}

/// System-wide QoS manager
pub struct HolisticQosManager {
    /// Process → policy mapping
    process_policies: BTreeMap<u64, QosPolicy>,
    /// Admission control
    admission: AdmissionState,
    /// Violation history (process → violation count)
    violations: BTreeMap<u64, u64>,
    /// QoS adaptation state (class → current multiplier)
    adaptation: BTreeMap<u8, f64>,
    /// Stats
    stats: HolisticQosStats,
}

impl HolisticQosManager {
    pub fn new() -> Self {
        Self {
            process_policies: BTreeMap::new(),
            admission: AdmissionState::new(),
            violations: BTreeMap::new(),
            adaptation: BTreeMap::new(),
            stats: HolisticQosStats::default(),
        }
    }

    /// Set system capacity
    pub fn set_capacity(&mut self, resource: QosResource, capacity: u64) {
        self.admission.set_capacity(resource, capacity);
    }

    /// Register process with QoS policy
    pub fn register_process(&mut self, pid: u64, policy: QosPolicy) -> QosAdmissionResult {
        let result = self.admission.try_admit(&policy);
        match result {
            QosAdmissionResult::Admitted | QosAdmissionResult::Degraded => {
                self.process_policies.insert(pid, policy);
                self.update_stats();
            }
            _ => {}
        }
        result
    }

    /// Unregister process
    pub fn unregister_process(&mut self, pid: u64) {
        if let Some(policy) = self.process_policies.remove(&pid) {
            self.admission.release(&policy);
            self.violations.remove(&pid);
            self.update_stats();
        }
    }

    /// Update resource usage
    pub fn update_usage(&mut self, pid: u64, resource: QosResource, current: u64) {
        if let Some(policy) = self.process_policies.get_mut(&pid) {
            for g in &mut policy.guarantees {
                if g.resource == resource {
                    g.current = current;
                }
            }

            // Check violations
            let viols = policy.violations();
            if !viols.is_empty() {
                *self.violations.entry(pid).or_insert(0) += viols.len() as u64;
                self.stats.total_violations += viols.len() as u64;
            }
        }
    }

    /// Get process QoS class
    pub fn process_class(&self, pid: u64) -> Option<QosClass> {
        self.process_policies.get(&pid).map(|p| p.class)
    }

    /// Get policy
    pub fn policy(&self, pid: u64) -> Option<&QosPolicy> {
        self.process_policies.get(&pid)
    }

    /// Adapt QoS under pressure
    pub fn adapt(&mut self, class: QosClass, pressure: f64) {
        let key = class as u8;
        let multiplier = if pressure > 0.9 {
            0.5
        } else if pressure > 0.7 {
            0.75
        } else {
            1.0
        };
        self.adaptation.insert(key, multiplier);
    }

    /// Get adaptation multiplier
    pub fn adaptation_multiplier(&self, class: QosClass) -> f64 {
        self.adaptation.get(&(class as u8)).copied().unwrap_or(1.0)
    }

    fn update_stats(&mut self) {
        self.stats.active_policies = self.process_policies.len();
        self.stats.processes_per_class.clear();
        for policy in self.process_policies.values() {
            *self
                .stats
                .processes_per_class
                .entry(policy.class as u8)
                .or_insert(0) += 1;
        }
        self.stats.admission_rate = self.admission.admission_rate();

        // Average compliance
        if self.process_policies.is_empty() {
            self.stats.avg_compliance = 1.0;
        } else {
            let compliant = self
                .process_policies
                .values()
                .filter(|p| p.check_compliance())
                .count();
            self.stats.avg_compliance =
                compliant as f64 / self.process_policies.len() as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &HolisticQosStats {
        &self.stats
    }
}

// ============================================================================
// Merged from qos_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QosClassV2 {
    /// Guaranteed low-latency
    RealTime,
    /// Interactive / latency-sensitive
    Interactive,
    /// Standard workloads
    Standard,
    /// Batch processing
    Batch,
    /// Best effort / scavenger
    Scavenger,
}

/// Resource type for QoS
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QosResourceV2 {
    /// CPU time
    Cpu,
    /// Memory bandwidth
    MemoryBandwidth,
    /// IO bandwidth
    IoBandwidth,
    /// Network bandwidth
    NetworkBandwidth,
    /// Cache allocation
    CacheWays,
}

/// SLO type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosSloType {
    /// Max latency (ns)
    MaxLatency,
    /// Min throughput (ops/sec)
    MinThroughput,
    /// Min bandwidth (bytes/sec)
    MinBandwidth,
    /// Max jitter (ns)
    MaxJitter,
}

/// SLO violation severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SloViolation {
    None,
    Warning,
    Minor,
    Major,
    Critical,
}

// ============================================================================
// QOS ALLOCATION
// ============================================================================

/// Per-resource allocation
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// Resource type
    pub resource: QosResourceV2,
    /// Guaranteed share (0..1)
    pub guaranteed: f64,
    /// Maximum share (0..1)
    pub limit: f64,
    /// Current usage (0..1)
    pub current_usage: f64,
    /// Weight (for proportional sharing above guaranteed)
    pub weight: u32,
}

impl ResourceAllocation {
    pub fn new(resource: QosResourceV2, guaranteed: f64, limit: f64, weight: u32) -> Self {
        Self {
            resource,
            guaranteed: guaranteed.max(0.0).min(1.0),
            limit: limit.max(guaranteed).min(1.0),
            current_usage: 0.0,
            weight,
        }
    }

    /// Headroom (how much more can be used)
    pub fn headroom(&self) -> f64 {
        (self.limit - self.current_usage).max(0.0)
    }

    /// Using guaranteed?
    pub fn using_guaranteed(&self) -> bool {
        self.current_usage <= self.guaranteed
    }

    /// Over limit?
    pub fn over_limit(&self) -> bool {
        self.current_usage > self.limit
    }
}

// ============================================================================
// SLO DEFINITION
// ============================================================================

/// Service level objective
#[derive(Debug, Clone)]
pub struct QosSloV2 {
    /// SLO type
    pub slo_type: QosSloType,
    /// Target value
    pub target: f64,
    /// Current value
    pub current: f64,
    /// History (ring buffer)
    history: Vec<f64>,
    /// History position
    pos: usize,
    /// Violations count
    pub violations: u64,
    /// Total samples
    pub samples: u64,
}

impl QosSloV2 {
    pub fn new(slo_type: QosSloType, target: f64) -> Self {
        Self {
            slo_type,
            target,
            current: 0.0,
            history: alloc::vec![0.0; 64],
            pos: 0,
            violations: 0,
            samples: 0,
        }
    }

    /// Record measurement
    pub fn record(&mut self, value: f64) {
        self.current = value;
        self.history[self.pos % self.history.len()] = value;
        self.pos += 1;
        self.samples += 1;

        if self.is_violated() {
            self.violations += 1;
        }
    }

    /// Check violation
    pub fn is_violated(&self) -> bool {
        match self.slo_type {
            QosSloType::MaxLatency | QosSloType::MaxJitter => self.current > self.target,
            QosSloType::MinThroughput | QosSloType::MinBandwidth => self.current < self.target,
        }
    }

    /// Violation severity
    pub fn severity(&self) -> SloViolation {
        if !self.is_violated() {
            return SloViolation::None;
        }
        let ratio = match self.slo_type {
            QosSloType::MaxLatency | QosSloType::MaxJitter => {
                if self.target > 0.0 { self.current / self.target } else { 1.0 }
            }
            QosSloType::MinThroughput | QosSloType::MinBandwidth => {
                if self.current > 0.0 { self.target / self.current } else { 10.0 }
            }
        };
        if ratio > 5.0 {
            SloViolation::Critical
        } else if ratio > 2.0 {
            SloViolation::Major
        } else if ratio > 1.5 {
            SloViolation::Minor
        } else {
            SloViolation::Warning
        }
    }

    /// Compliance rate (0..1)
    pub fn compliance_rate(&self) -> f64 {
        if self.samples == 0 {
            return 1.0;
        }
        1.0 - (self.violations as f64 / self.samples as f64)
    }

    /// Percentile from history
    pub fn percentile(&self, p: f64) -> f64 {
        let count = self.pos.min(self.history.len());
        if count == 0 {
            return 0.0;
        }
        let mut sorted: Vec<f64> = self.history[..count].to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let idx = ((p / 100.0) * (count - 1) as f64) as usize;
        sorted[idx.min(count - 1)]
    }
}

// ============================================================================
// QOS GROUP
// ============================================================================

/// QoS group (container for a workload or tenant)
#[derive(Debug)]
pub struct QosGroupV2 {
    /// Group ID (FNV-1a hash of name)
    pub group_id: u64,
    /// QoS class
    pub class: QosClassV2,
    /// Resource allocations
    allocations: BTreeMap<u8, ResourceAllocation>,
    /// SLOs
    slos: Vec<QosSloV2>,
    /// Member PIDs
    pub members: Vec<u64>,
    /// Active
    pub active: bool,
}

impl QosGroupV2 {
    pub fn new(name: &str, class: QosClassV2) -> Self {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self {
            group_id: hash,
            class,
            allocations: BTreeMap::new(),
            slos: Vec::new(),
            members: Vec::new(),
            active: true,
        }
    }

    /// Add resource allocation
    pub fn add_allocation(&mut self, alloc: ResourceAllocation) {
        self.allocations.insert(alloc.resource as u8, alloc);
    }

    /// Add SLO
    pub fn add_slo(&mut self, slo: QosSloV2) {
        self.slos.push(slo);
    }

    /// Worst SLO violation
    pub fn worst_violation(&self) -> SloViolation {
        self.slos.iter()
            .map(|s| s.severity())
            .max_by_key(|v| *v as u8)
            .unwrap_or(SloViolation::None)
    }

    /// Overall compliance
    pub fn overall_compliance(&self) -> f64 {
        if self.slos.is_empty() {
            return 1.0;
        }
        self.slos.iter()
            .map(|s| s.compliance_rate())
            .sum::<f64>() / self.slos.len() as f64
    }

    /// Resource headroom for a specific resource
    pub fn resource_headroom(&self, resource: QosResourceV2) -> f64 {
        self.allocations.get(&(resource as u8))
            .map(|a| a.headroom())
            .unwrap_or(0.0)
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// QoS V2 stats
#[derive(Debug, Clone, Default)]
pub struct HolisticQosV2Stats {
    /// Active groups
    pub active_groups: usize,
    /// Total members
    pub total_members: usize,
    /// SLO violations
    pub slo_violations: usize,
    /// Average compliance
    pub avg_compliance: f64,
    /// Groups by class
    pub groups_by_class: [u32; 5],
}

/// System-wide QoS V2 engine
pub struct HolisticQosV2 {
    /// Groups
    groups: BTreeMap<u64, QosGroupV2>,
    /// Stats
    stats: HolisticQosV2Stats,
}

impl HolisticQosV2 {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            stats: HolisticQosV2Stats::default(),
        }
    }

    /// Create group
    pub fn create_group(&mut self, name: &str, class: QosClassV2) -> u64 {
        let group = QosGroupV2::new(name, class);
        let id = group.group_id;
        self.groups.insert(id, group);
        self.update_stats();
        id
    }

    /// Add SLO to group
    pub fn add_slo(&mut self, group_id: u64, slo_type: QosSloType, target: f64) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.add_slo(QosSloV2::new(slo_type, target));
        }
    }

    /// Record SLO measurement
    pub fn record_slo(&mut self, group_id: u64, slo_idx: usize, value: f64) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            if let Some(slo) = group.slos.get_mut(slo_idx) {
                slo.record(value);
            }
        }
        self.update_stats();
    }

    /// Get groups with violations
    pub fn violated_groups(&self) -> Vec<(u64, SloViolation)> {
        self.groups.iter()
            .filter_map(|(&id, g)| {
                let v = g.worst_violation();
                if v != SloViolation::None { Some((id, v)) } else { None }
            })
            .collect()
    }

    fn update_stats(&mut self) {
        self.stats.active_groups = self.groups.values().filter(|g| g.active).count();
        self.stats.total_members = self.groups.values().map(|g| g.members.len()).sum();
        self.stats.slo_violations = self.groups.values()
            .filter(|g| g.worst_violation() != SloViolation::None)
            .count();

        let compliances: Vec<f64> = self.groups.values()
            .filter(|g| g.active)
            .map(|g| g.overall_compliance())
            .collect();
        self.stats.avg_compliance = if !compliances.is_empty() {
            compliances.iter().sum::<f64>() / compliances.len() as f64
        } else {
            1.0
        };

        self.stats.groups_by_class = [0; 5];
        for g in self.groups.values() {
            let idx = g.class as usize;
            if idx < 5 {
                self.stats.groups_by_class[idx] += 1;
            }
        }
    }

    /// Stats
    pub fn stats(&self) -> &HolisticQosV2Stats {
        &self.stats
    }
}
