//! # Advanced Kernel Advisory System
//!
//! Kernel → application advisory protocol:
//! - Memory pressure advisories
//! - CPU contention notifications
//! - I/O bandwidth advisories
//! - Thermal throttling warnings
//! - Power state changes
//! - Security alerts
//! - QoS level adjustments
//! - Advisory prioritization and coalescing

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ADVISORY TYPES
// ============================================================================

/// Advisory urgency level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdvisoryUrgency {
    /// Informational only
    Info,
    /// Suggested action
    Suggestion,
    /// Recommended action
    Recommended,
    /// Urgent action needed
    Urgent,
    /// Critical - immediate action required
    Critical,
}

/// Advisory category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdvisoryCategory {
    /// Memory-related
    Memory,
    /// CPU-related
    Cpu,
    /// I/O-related
    Io,
    /// Network-related
    Network,
    /// Thermal
    Thermal,
    /// Power/energy
    Power,
    /// Security
    Security,
    /// QoS
    Qos,
    /// Scheduling
    Scheduling,
    /// Resource limits
    ResourceLimit,
    /// System-wide
    System,
}

/// Specific advisory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisoryType {
    // Memory
    MemoryPressureLow,
    MemoryPressureMedium,
    MemoryPressureHigh,
    MemoryPressureCritical,
    MemorySwapActive,
    MemoryFragmented,
    MemoryLeakSuspected,
    MemoryNunaRebalance,

    // CPU
    CpuThrottling,
    CpuContention,
    CpuMigrationSuggested,
    CpuAffinityChange,
    CpuFrequencyChange,

    // I/O
    IoSaturation,
    IoBandwidthReduced,
    IoLatencyHigh,
    IoDeviceError,

    // Network
    NetworkCongestion,
    NetworkBandwidthReduced,
    NetworkLatencyHigh,

    // Thermal
    ThermalWarning,
    ThermalThrottling,
    ThermalCritical,

    // Power
    PowerSavingEnabled,
    PowerBatteryLow,
    PowerSourceChanged,

    // Security
    SecurityThreat,
    SecurityPolicyChange,

    // QoS
    QosLevelChanged,
    QosDegradation,

    // Scheduling
    SchedPriorityChange,
    SchedClassChange,

    // Resource limits
    ResourceLimitApproaching,
    ResourceLimitReached,

    // System
    SystemShutdown,
    SystemReboot,
    SystemMaintenance,
}

impl AdvisoryType {
    /// Get category for advisory type
    pub fn category(&self) -> AdvisoryCategory {
        match self {
            Self::MemoryPressureLow
            | Self::MemoryPressureMedium
            | Self::MemoryPressureHigh
            | Self::MemoryPressureCritical
            | Self::MemorySwapActive
            | Self::MemoryFragmented
            | Self::MemoryLeakSuspected
            | Self::MemoryNunaRebalance => AdvisoryCategory::Memory,

            Self::CpuThrottling
            | Self::CpuContention
            | Self::CpuMigrationSuggested
            | Self::CpuAffinityChange
            | Self::CpuFrequencyChange => AdvisoryCategory::Cpu,

            Self::IoSaturation
            | Self::IoBandwidthReduced
            | Self::IoLatencyHigh
            | Self::IoDeviceError => AdvisoryCategory::Io,

            Self::NetworkCongestion
            | Self::NetworkBandwidthReduced
            | Self::NetworkLatencyHigh => AdvisoryCategory::Network,

            Self::ThermalWarning
            | Self::ThermalThrottling
            | Self::ThermalCritical => AdvisoryCategory::Thermal,

            Self::PowerSavingEnabled
            | Self::PowerBatteryLow
            | Self::PowerSourceChanged => AdvisoryCategory::Power,

            Self::SecurityThreat
            | Self::SecurityPolicyChange => AdvisoryCategory::Security,

            Self::QosLevelChanged
            | Self::QosDegradation => AdvisoryCategory::Qos,

            Self::SchedPriorityChange
            | Self::SchedClassChange => AdvisoryCategory::Scheduling,

            Self::ResourceLimitApproaching
            | Self::ResourceLimitReached => AdvisoryCategory::ResourceLimit,

            Self::SystemShutdown
            | Self::SystemReboot
            | Self::SystemMaintenance => AdvisoryCategory::System,
        }
    }

    /// Default urgency for this advisory type
    pub fn default_urgency(&self) -> AdvisoryUrgency {
        match self {
            Self::MemoryPressureCritical
            | Self::ThermalCritical
            | Self::SecurityThreat
            | Self::SystemShutdown => AdvisoryUrgency::Critical,

            Self::MemoryPressureHigh
            | Self::ThermalThrottling
            | Self::IoDeviceError
            | Self::ResourceLimitReached => AdvisoryUrgency::Urgent,

            Self::MemoryPressureMedium
            | Self::CpuThrottling
            | Self::IoSaturation
            | Self::ThermalWarning
            | Self::QosDegradation
            | Self::ResourceLimitApproaching => AdvisoryUrgency::Recommended,

            Self::CpuMigrationSuggested
            | Self::MemoryNunaRebalance
            | Self::IoBandwidthReduced
            | Self::NetworkBandwidthReduced => AdvisoryUrgency::Suggestion,

            _ => AdvisoryUrgency::Info,
        }
    }
}

// ============================================================================
// ADVISORY MESSAGE
// ============================================================================

/// An advisory message from kernel to application
#[derive(Debug, Clone)]
pub struct Advisory {
    /// Unique advisory ID
    pub id: u64,
    /// Advisory type
    pub advisory_type: AdvisoryType,
    /// Urgency level
    pub urgency: AdvisoryUrgency,
    /// Target PID (0 = broadcast)
    pub target_pid: u64,
    /// Creation timestamp
    pub timestamp: u64,
    /// Expiration time (0 = no expiry)
    pub expires_at: u64,
    /// Numeric parameter (context-dependent)
    pub param1: u64,
    /// Second numeric parameter
    pub param2: u64,
    /// Whether acknowledgment is required
    pub ack_required: bool,
    /// Whether this was acknowledged
    pub acknowledged: bool,
    /// Sequence number
    pub sequence: u64,
}

impl Advisory {
    pub fn new(
        id: u64,
        advisory_type: AdvisoryType,
        target_pid: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            advisory_type,
            urgency: advisory_type.default_urgency(),
            target_pid,
            timestamp,
            expires_at: 0,
            param1: 0,
            param2: 0,
            ack_required: false,
            acknowledged: false,
            sequence: 0,
        }
    }

    /// Check if expired
    #[inline(always)]
    pub fn is_expired(&self, current_time: u64) -> bool {
        self.expires_at > 0 && current_time > self.expires_at
    }

    /// Set expiry
    #[inline(always)]
    pub fn with_expiry(mut self, duration_ms: u64) -> Self {
        self.expires_at = self.timestamp + duration_ms;
        self
    }

    /// Set parameters
    #[inline]
    pub fn with_params(mut self, p1: u64, p2: u64) -> Self {
        self.param1 = p1;
        self.param2 = p2;
        self
    }

    /// Require acknowledgment
    #[inline(always)]
    pub fn require_ack(mut self) -> Self {
        self.ack_required = true;
        self
    }
}

// ============================================================================
// PER-PROCESS ADVISORY STATE
// ============================================================================

/// Advisory subscription filter
#[derive(Debug, Clone)]
pub struct AdvisorySubscription {
    /// Categories subscribed to (empty = all)
    pub categories: Vec<AdvisoryCategory>,
    /// Minimum urgency level
    pub min_urgency: AdvisoryUrgency,
    /// Max pending advisories
    pub max_pending: usize,
}

impl Default for AdvisorySubscription {
    fn default() -> Self {
        Self {
            categories: Vec::new(),
            min_urgency: AdvisoryUrgency::Info,
            max_pending: 64,
        }
    }
}

impl AdvisorySubscription {
    /// Check if advisory matches subscription
    pub fn matches(&self, advisory: &Advisory) -> bool {
        if advisory.urgency < self.min_urgency {
            return false;
        }
        if !self.categories.is_empty() {
            let cat = advisory.advisory_type.category();
            if !self.categories.contains(&cat) {
                return false;
            }
        }
        true
    }
}

/// Per-process advisory queue
struct ProcessAdvisoryQueue {
    /// PID
    pid: u64,
    /// Pending advisories
    pending: Vec<Advisory>,
    /// Subscription filter
    subscription: AdvisorySubscription,
    /// Advisories delivered
    delivered: u64,
    /// Advisories dropped (queue full)
    dropped: u64,
    /// Advisories acknowledged
    acknowledged: u64,
}

impl ProcessAdvisoryQueue {
    fn new(pid: u64) -> Self {
        Self {
            pid,
            pending: Vec::new(),
            subscription: AdvisorySubscription::default(),
            delivered: 0,
            dropped: 0,
            acknowledged: 0,
        }
    }

    /// Enqueue advisory
    fn enqueue(&mut self, advisory: Advisory) -> bool {
        if !self.subscription.matches(&advisory) {
            return false;
        }

        if self.pending.len() >= self.subscription.max_pending {
            // Drop lowest urgency advisory
            if let Some(min_idx) = self
                .pending
                .iter()
                .enumerate()
                .min_by_key(|(_, a)| a.urgency)
                .map(|(i, _)| i)
            {
                if self.pending[min_idx].urgency < advisory.urgency {
                    self.pending.swap_remove(min_idx);
                    self.dropped += 1;
                } else {
                    self.dropped += 1;
                    return false;
                }
            }
        }

        self.pending.push(advisory);
        true
    }

    /// Dequeue next advisory (highest urgency first)
    fn dequeue(&mut self) -> Option<Advisory> {
        if self.pending.is_empty() {
            return None;
        }

        let max_idx = self
            .pending
            .iter()
            .enumerate()
            .max_by_key(|(_, a)| a.urgency)
            .map(|(i, _)| i)?;

        self.delivered += 1;
        Some(self.pending.swap_remove(max_idx))
    }

    /// Remove expired advisories
    fn cleanup_expired(&mut self, current_time: u64) {
        self.pending.retain(|a| !a.is_expired(current_time));
    }

    /// Pending count
    fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

// ============================================================================
// ADVISORY ENGINE
// ============================================================================

/// Advisory coalescing rule
#[derive(Debug, Clone, Copy)]
pub struct CoalesceRule {
    /// Advisory type to coalesce
    pub advisory_type: AdvisoryType,
    /// Minimum interval between same advisories (ms)
    pub min_interval_ms: u64,
    /// Whether to update params of existing advisory
    pub update_existing: bool,
}

/// Global advisory engine
pub struct AdvisoryEngine {
    /// Per-process queues
    queues: BTreeMap<u64, ProcessAdvisoryQueue>,
    /// Coalescing rules
    coalesce_rules: Vec<CoalesceRule>,
    /// Last emission time per (pid, type) for coalescing
    last_emission: BTreeMap<(u64, u8), u64>,
    /// Next advisory ID
    next_id: u64,
    /// Global sequence number
    sequence: u64,
    /// Total advisories created
    pub total_created: u64,
    /// Total advisories delivered
    pub total_delivered: u64,
    /// Total advisories coalesced (suppressed)
    pub total_coalesced: u64,
    /// Total advisories dropped
    pub total_dropped: u64,
}

impl AdvisoryEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            queues: BTreeMap::new(),
            coalesce_rules: Vec::new(),
            last_emission: BTreeMap::new(),
            next_id: 1,
            sequence: 0,
            total_created: 0,
            total_delivered: 0,
            total_coalesced: 0,
            total_dropped: 0,
        };

        // Default coalescing rules
        engine.add_coalesce_rule(AdvisoryType::MemoryPressureLow, 5000, true);
        engine.add_coalesce_rule(AdvisoryType::MemoryPressureMedium, 2000, true);
        engine.add_coalesce_rule(AdvisoryType::CpuContention, 3000, true);
        engine.add_coalesce_rule(AdvisoryType::IoSaturation, 2000, true);
        engine.add_coalesce_rule(AdvisoryType::ThermalWarning, 10000, true);

        engine
    }

    /// Add a coalescing rule
    fn add_coalesce_rule(
        &mut self,
        advisory_type: AdvisoryType,
        min_interval_ms: u64,
        update_existing: bool,
    ) {
        self.coalesce_rules.push(CoalesceRule {
            advisory_type,
            min_interval_ms,
            update_existing,
        });
    }

    /// Register a process
    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.queues.entry(pid).or_insert_with(|| ProcessAdvisoryQueue::new(pid));
    }

    /// Unregister a process
    #[inline]
    pub fn unregister_process(&mut self, pid: u64) {
        self.queues.remove(&pid);
        // Clean up coalescing state
        self.last_emission.retain(|&(p, _), _| p != pid);
    }

    /// Set subscription for a process
    #[inline]
    pub fn set_subscription(&mut self, pid: u64, sub: AdvisorySubscription) {
        if let Some(queue) = self.queues.get_mut(&pid) {
            queue.subscription = sub;
        }
    }

    /// Emit an advisory to a specific process
    pub fn emit(
        &mut self,
        advisory_type: AdvisoryType,
        target_pid: u64,
        timestamp: u64,
    ) -> Option<u64> {
        // Check coalescing
        let type_key = advisory_type as u8;
        if self.should_coalesce(target_pid, type_key, timestamp) {
            self.total_coalesced += 1;
            return None;
        }

        let id = self.next_id;
        self.next_id += 1;
        self.sequence += 1;

        let mut advisory = Advisory::new(id, advisory_type, target_pid, timestamp);
        advisory.sequence = self.sequence;

        self.last_emission.insert((target_pid, type_key), timestamp);

        if let Some(queue) = self.queues.get_mut(&target_pid) {
            if queue.enqueue(advisory) {
                self.total_created += 1;
                Some(id)
            } else {
                self.total_dropped += 1;
                None
            }
        } else {
            None
        }
    }

    /// Broadcast advisory to all processes
    pub fn broadcast(
        &mut self,
        advisory_type: AdvisoryType,
        timestamp: u64,
    ) -> u32 {
        let pids: Vec<u64> = self.queues.keys().copied().collect();
        let mut count = 0u32;

        for pid in pids {
            if self.emit(advisory_type, pid, timestamp).is_some() {
                count += 1;
            }
        }

        count
    }

    /// Deliver next advisory for a process
    #[inline]
    pub fn deliver(&mut self, pid: u64) -> Option<Advisory> {
        let queue = self.queues.get_mut(&pid)?;
        let adv = queue.dequeue()?;
        self.total_delivered += 1;
        Some(adv)
    }

    /// Acknowledge advisory
    pub fn acknowledge(&mut self, pid: u64, advisory_id: u64) {
        if let Some(queue) = self.queues.get_mut(&pid) {
            queue.acknowledged += 1;
            // Mark if still pending
            for adv in &mut queue.pending {
                if adv.id == advisory_id {
                    adv.acknowledged = true;
                    break;
                }
            }
        }
    }

    /// Check if should coalesce
    fn should_coalesce(&self, pid: u64, type_key: u8, timestamp: u64) -> bool {
        if let Some(&last) = self.last_emission.get(&(pid, type_key)) {
            for rule in &self.coalesce_rules {
                if rule.advisory_type as u8 == type_key {
                    return timestamp.saturating_sub(last) < rule.min_interval_ms;
                }
            }
        }
        false
    }

    /// Cleanup expired advisories across all queues
    #[inline]
    pub fn cleanup_expired(&mut self, current_time: u64) {
        for queue in self.queues.values_mut() {
            queue.cleanup_expired(current_time);
        }
    }

    /// Pending advisory count for a process
    #[inline]
    pub fn pending_count(&self, pid: u64) -> usize {
        self.queues
            .get(&pid)
            .map_or(0, |q| q.pending_count())
    }

    /// Total pending across all processes
    #[inline(always)]
    pub fn total_pending(&self) -> usize {
        self.queues.values().map(|q| q.pending_count()).sum()
    }

    /// Registered process count
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.queues.len()
    }
}
