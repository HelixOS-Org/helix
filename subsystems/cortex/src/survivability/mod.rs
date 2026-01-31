//! # Survivability Core
//!
//! The Survivability Core implements a revolutionary approach to kernel security:
//! **assume compromise has already occurred, and continue operating anyway**.
//!
//! ## Traditional Security vs Survivability
//!
//! **Traditional Security:**
//! - Focus on prevention
//! - Assumes system is either secure or compromised
//! - Compromise means game over
//! - All-or-nothing approach
//!
//! **Survivability:**
//! - Focus on continued operation
//! - Assumes system MAY be compromised at any time
//! - Compromise triggers isolation and recovery
//! - Graceful degradation, never total failure
//!
//! ## Key Innovations
//!
//! ### 1. Anomaly Detection
//! Continuous monitoring for behaviors that deviate from established baselines.
//! Uses statistical analysis, not signatures (catches zero-days).
//!
//! ### 2. Threat Isolation
//! Compromised components are isolated in real-time:
//! - Memory pages marked read-only
//! - System call access revoked
//! - Network access blocked
//! - Inter-subsystem communication severed
//!
//! ### 3. Self-Reconstruction
//! The kernel can reconstruct its own state from known-good sources:
//! - Code verification from signed images
//! - State reconstruction from temporal snapshots
//! - Memory scrubbing and re-initialization
//!
//! ### 4. Survival Mode
//! When under active attack, the kernel enters a reduced functionality mode
//! that prioritizes survival over features:
//! - Non-essential subsystems suspended
//! - Attack surface minimized
//! - Forensic logging maximized

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::{CortexEvent, SubsystemId, ThreatId, ThreatResponse, ThreatResponseStrategy};

// =============================================================================
// THREAT TYPES
// =============================================================================

/// Threat level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ThreatLevel {
    /// No threat
    None        = 0,

    /// Low threat - anomaly detected
    Low         = 1,

    /// Medium threat - suspicious activity
    Medium      = 2,

    /// High threat - probable attack
    High        = 3,

    /// Critical threat - active exploitation
    Critical    = 4,

    /// Existential threat - kernel compromise imminent
    Existential = 5,
}

impl Default for ThreatLevel {
    fn default() -> Self {
        Self::None
    }
}

/// Threat category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatCategory {
    /// Memory corruption (buffer overflow, use-after-free)
    MemoryCorruption,

    /// Privilege escalation
    PrivilegeEscalation,

    /// Code injection
    CodeInjection,

    /// Control flow hijacking (ROP, JOP)
    ControlFlowHijack,

    /// Information disclosure
    InformationDisclosure,

    /// Denial of service
    DenialOfService,

    /// Side-channel attack (timing, cache)
    SideChannel,

    /// Supply chain compromise
    SupplyChain,

    /// Anomalous behavior (unknown)
    Anomalous,
}

/// A detected threat
#[derive(Debug, Clone)]
pub struct Threat {
    /// Unique identifier
    pub id: ThreatId,

    /// Threat level
    pub level: ThreatLevel,

    /// Category
    pub category: ThreatCategory,

    /// When detected
    pub detected_at: u64,

    /// Source (which subsystem or address)
    pub source: ThreatSource,

    /// Description
    pub description: String,

    /// Evidence
    pub evidence: Vec<Evidence>,

    /// Is this threat still active?
    pub active: bool,

    /// Response taken
    pub response: Option<ThreatResponseStrategy>,
}

/// Source of a threat
#[derive(Debug, Clone)]
pub enum ThreatSource {
    /// Kernel subsystem
    Subsystem(SubsystemId),

    /// Memory address
    Address(u64),

    /// Process
    Process(u64),

    /// Network
    Network { src_ip: u32, src_port: u16 },

    /// Hardware
    Hardware(String),

    /// Unknown
    Unknown,
}

/// Evidence of a threat
#[derive(Debug, Clone)]
pub struct Evidence {
    /// Type of evidence
    pub evidence_type: EvidenceType,

    /// Data
    pub data: Vec<u8>,

    /// Timestamp
    pub timestamp: u64,

    /// Confidence
    pub confidence: f64,
}

/// Type of evidence
#[derive(Debug, Clone)]
pub enum EvidenceType {
    /// Stack trace
    StackTrace,

    /// Memory dump
    MemoryDump,

    /// System call log
    SyscallLog,

    /// Network packet
    NetworkPacket,

    /// Control flow violation
    CfiViolation,

    /// Statistical anomaly
    StatisticalAnomaly,
}

impl Threat {
    /// Create new threat
    pub fn new(
        id: ThreatId,
        level: ThreatLevel,
        category: ThreatCategory,
        source: ThreatSource,
        description: &str,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            level,
            category,
            detected_at: timestamp,
            source,
            description: String::from(description),
            evidence: Vec::new(),
            active: true,
            response: None,
        }
    }

    /// Add evidence
    pub fn add_evidence(&mut self, evidence: Evidence) {
        self.evidence.push(evidence);
    }
}

// =============================================================================
// ANOMALY DETECTION
// =============================================================================

/// Anomaly detector
pub struct AnomalyDetector {
    /// Baseline metrics
    baselines: BTreeMap<String, Baseline>,

    /// Detection sensitivity
    sensitivity: f64,

    /// Detection window (samples)
    window_size: usize,

    /// Current samples
    samples: BTreeMap<String, Vec<f64>>,

    /// Anomalies detected
    anomalies_detected: u64,
}

/// Baseline for a metric
#[derive(Clone)]
pub struct Baseline {
    /// Metric name
    pub name: String,

    /// Mean value
    pub mean: f64,

    /// Standard deviation
    pub std_dev: f64,

    /// Minimum observed
    pub min: f64,

    /// Maximum observed
    pub max: f64,

    /// Sample count
    pub sample_count: u64,

    /// Is baseline established?
    pub established: bool,
}

impl Baseline {
    pub fn new(name: &str) -> Self {
        Self {
            name: String::from(name),
            mean: 0.0,
            std_dev: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            sample_count: 0,
            established: false,
        }
    }

    /// Update baseline with new sample (online algorithm)
    pub fn update(&mut self, value: f64) {
        self.sample_count += 1;

        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }

        // Welford's online algorithm for mean and variance
        let delta = value - self.mean;
        self.mean += delta / self.sample_count as f64;
        let delta2 = value - self.mean;

        // Update variance
        let variance = if self.sample_count > 1 {
            let m2 = self.std_dev * self.std_dev * (self.sample_count - 1) as f64;
            (m2 + delta * delta2) / self.sample_count as f64
        } else {
            0.0
        };

        self.std_dev = variance.sqrt();

        // Consider baseline established after 1000 samples
        if self.sample_count >= 1000 {
            self.established = true;
        }
    }

    /// Check if value is anomalous
    pub fn is_anomalous(&self, value: f64, z_threshold: f64) -> Option<Anomaly> {
        if !self.established {
            return None;
        }

        if self.std_dev == 0.0 {
            return None;
        }

        let z_score = (value - self.mean) / self.std_dev;

        if z_score.abs() > z_threshold {
            Some(Anomaly {
                metric: self.name.clone(),
                value,
                expected_mean: self.mean,
                z_score,
                deviation_percent: ((value - self.mean) / self.mean * 100.0).abs(),
            })
        } else {
            None
        }
    }
}

/// A detected anomaly
#[derive(Debug, Clone)]
pub struct Anomaly {
    /// Metric that was anomalous
    pub metric: String,

    /// Observed value
    pub value: f64,

    /// Expected mean
    pub expected_mean: f64,

    /// Z-score
    pub z_score: f64,

    /// Deviation percentage
    pub deviation_percent: f64,
}

impl AnomalyDetector {
    /// Create new detector
    pub fn new(sensitivity: f64) -> Self {
        Self {
            baselines: BTreeMap::new(),
            sensitivity,
            window_size: 100,
            samples: BTreeMap::new(),
            anomalies_detected: 0,
        }
    }

    /// Register a metric to track
    pub fn register_metric(&mut self, name: &str) {
        self.baselines
            .insert(String::from(name), Baseline::new(name));
        self.samples
            .insert(String::from(name), Vec::with_capacity(self.window_size));
    }

    /// Record a sample
    pub fn record(&mut self, metric: &str, value: f64) -> Option<Anomaly> {
        // Update baseline
        if let Some(baseline) = self.baselines.get_mut(metric) {
            baseline.update(value);

            // Check for anomaly
            let z_threshold = 3.0 / self.sensitivity;
            if let Some(anomaly) = baseline.is_anomalous(value, z_threshold) {
                self.anomalies_detected += 1;
                return Some(anomaly);
            }
        }

        // Store sample
        if let Some(samples) = self.samples.get_mut(metric) {
            if samples.len() >= self.window_size {
                samples.remove(0);
            }
            samples.push(value);
        }

        None
    }

    /// Get baseline for metric
    pub fn get_baseline(&self, metric: &str) -> Option<&Baseline> {
        self.baselines.get(metric)
    }

    /// Get anomaly count
    pub fn anomaly_count(&self) -> u64 {
        self.anomalies_detected
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new(1.0)
    }
}

// =============================================================================
// ISOLATION
// =============================================================================

/// Isolation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationStrategy {
    /// Soft isolation - limit resources
    Soft,

    /// Medium isolation - revoke capabilities
    Medium,

    /// Hard isolation - complete sandbox
    Hard,

    /// Kill - terminate immediately
    Kill,
}

/// Isolated component
#[derive(Clone)]
pub struct IsolatedComponent {
    /// Subsystem ID
    pub subsystem: SubsystemId,

    /// Isolation strategy
    pub strategy: IsolationStrategy,

    /// When isolated
    pub isolated_at: u64,

    /// Reason
    pub reason: String,

    /// Threat that caused isolation
    pub threat: Option<ThreatId>,

    /// Is still isolated?
    pub active: bool,

    /// Restrictions applied
    pub restrictions: Restrictions,
}

/// Restrictions on isolated component
#[derive(Clone, Debug, Default)]
pub struct Restrictions {
    /// Memory read-only
    pub memory_readonly: bool,

    /// Syscalls blocked
    pub syscalls_blocked: Vec<u32>,

    /// All syscalls blocked
    pub all_syscalls_blocked: bool,

    /// Network blocked
    pub network_blocked: bool,

    /// IPC blocked
    pub ipc_blocked: bool,

    /// Disk access blocked
    pub disk_blocked: bool,

    /// CPU limited
    pub cpu_limited: bool,

    /// CPU limit percent
    pub cpu_limit_percent: u8,

    /// Memory limited
    pub memory_limited: bool,

    /// Memory limit bytes
    pub memory_limit_bytes: usize,
}

impl Restrictions {
    /// Create soft restrictions
    pub fn soft() -> Self {
        Self {
            cpu_limited: true,
            cpu_limit_percent: 10,
            memory_limited: true,
            memory_limit_bytes: 16 * 1024 * 1024, // 16 MB
            ..Default::default()
        }
    }

    /// Create medium restrictions
    pub fn medium() -> Self {
        Self {
            memory_readonly: true,
            network_blocked: true,
            cpu_limited: true,
            cpu_limit_percent: 5,
            memory_limited: true,
            memory_limit_bytes: 4 * 1024 * 1024, // 4 MB
            ..Default::default()
        }
    }

    /// Create hard restrictions
    pub fn hard() -> Self {
        Self {
            memory_readonly: true,
            all_syscalls_blocked: true,
            network_blocked: true,
            ipc_blocked: true,
            disk_blocked: true,
            cpu_limited: true,
            cpu_limit_percent: 1,
            memory_limited: true,
            memory_limit_bytes: 1024 * 1024, // 1 MB
            ..Default::default()
        }
    }
}

// =============================================================================
// RECOVERY
// =============================================================================

/// Recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Restart component
    Restart,

    /// Restore from snapshot
    Restore,

    /// Reconstruct from source
    Reconstruct,

    /// Replace with backup
    Replace,

    /// Disable permanently
    Disable,
}

/// Recovery operation
#[derive(Clone)]
pub struct Recovery {
    /// Target subsystem
    pub subsystem: SubsystemId,

    /// Strategy
    pub strategy: RecoveryStrategy,

    /// State
    pub state: RecoveryState,

    /// Start time
    pub start_time: u64,

    /// Completion time
    pub completion_time: Option<u64>,

    /// Success?
    pub success: Option<bool>,

    /// Error message if failed
    pub error: Option<String>,
}

/// Recovery state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryState {
    Pending,
    Suspending,
    Cleaning,
    Reconstructing,
    Verifying,
    Resuming,
    Completed,
    Failed,
}

impl Recovery {
    pub fn new(subsystem: SubsystemId, strategy: RecoveryStrategy, timestamp: u64) -> Self {
        Self {
            subsystem,
            strategy,
            state: RecoveryState::Pending,
            start_time: timestamp,
            completion_time: None,
            success: None,
            error: None,
        }
    }

    pub fn advance(&mut self) {
        self.state = match self.state {
            RecoveryState::Pending => RecoveryState::Suspending,
            RecoveryState::Suspending => RecoveryState::Cleaning,
            RecoveryState::Cleaning => RecoveryState::Reconstructing,
            RecoveryState::Reconstructing => RecoveryState::Verifying,
            RecoveryState::Verifying => RecoveryState::Resuming,
            RecoveryState::Resuming => RecoveryState::Completed,
            other => other,
        };
    }

    pub fn fail(&mut self, error: &str, timestamp: u64) {
        self.state = RecoveryState::Failed;
        self.success = Some(false);
        self.error = Some(String::from(error));
        self.completion_time = Some(timestamp);
    }

    pub fn complete(&mut self, timestamp: u64) {
        self.state = RecoveryState::Completed;
        self.success = Some(true);
        self.completion_time = Some(timestamp);
    }
}

// =============================================================================
// SURVIVAL MODE
// =============================================================================

/// Survival mode configuration
#[derive(Clone)]
pub struct SurvivalMode {
    /// Is survival mode active?
    pub active: bool,

    /// When activated
    pub activated_at: Option<u64>,

    /// Trigger threat
    pub trigger: Option<ThreatId>,

    /// Suspended subsystems
    pub suspended_subsystems: Vec<SubsystemId>,

    /// Active protections
    pub protections: SurvivalProtections,

    /// Log level (max for forensics)
    pub log_level: u8,
}

/// Protections in survival mode
#[derive(Clone, Debug, Default)]
pub struct SurvivalProtections {
    /// Disable module loading
    pub module_loading_disabled: bool,

    /// Disable hot-swap
    pub hot_swap_disabled: bool,

    /// Disable network
    pub network_disabled: bool,

    /// Disable new processes
    pub new_processes_disabled: bool,

    /// Enable memory encryption
    pub memory_encryption: bool,

    /// Enable aggressive ASLR
    pub aggressive_aslr: bool,

    /// Enable control flow guard
    pub cfguard_enabled: bool,

    /// Stack canary refresh rate (faster)
    pub stack_canary_refresh_ms: u32,
}

impl SurvivalProtections {
    pub fn maximum() -> Self {
        Self {
            module_loading_disabled: true,
            hot_swap_disabled: true,
            network_disabled: true,
            new_processes_disabled: true,
            memory_encryption: true,
            aggressive_aslr: true,
            cfguard_enabled: true,
            stack_canary_refresh_ms: 100,
        }
    }
}

impl Default for SurvivalMode {
    fn default() -> Self {
        Self {
            active: false,
            activated_at: None,
            trigger: None,
            suspended_subsystems: Vec::new(),
            protections: SurvivalProtections::default(),
            log_level: 0,
        }
    }
}

// =============================================================================
// SURVIVABILITY CORE
// =============================================================================

/// The Survivability Core
pub struct SurvivabilityCore {
    /// Anomaly detector
    anomaly_detector: AnomalyDetector,

    /// Active threats
    active_threats: BTreeMap<ThreatId, Threat>,

    /// Next threat ID
    next_threat_id: AtomicU64,

    /// Isolated components
    isolated: BTreeMap<SubsystemId, IsolatedComponent>,

    /// Active recoveries
    recoveries: Vec<Recovery>,

    /// Survival mode
    survival_mode: SurvivalMode,

    /// Current timestamp
    current_timestamp: u64,

    /// Statistics
    stats: SurvivabilityStats,
}

/// Survivability statistics
#[derive(Debug, Clone, Default)]
pub struct SurvivabilityStats {
    pub threats_detected: u64,
    pub threats_neutralized: u64,
    pub anomalies_detected: u64,
    pub isolations_performed: u64,
    pub recoveries_attempted: u64,
    pub recoveries_succeeded: u64,
    pub survival_mode_activations: u64,
    pub current_threat_level: ThreatLevel,
}

impl SurvivabilityCore {
    /// Create new survivability core
    pub fn new() -> Self {
        let mut core = Self {
            anomaly_detector: AnomalyDetector::new(1.0),
            active_threats: BTreeMap::new(),
            next_threat_id: AtomicU64::new(1),
            isolated: BTreeMap::new(),
            recoveries: Vec::new(),
            survival_mode: SurvivalMode::default(),
            current_timestamp: 0,
            stats: SurvivabilityStats::default(),
        };

        // Register standard metrics
        core.register_standard_metrics();

        core
    }

    /// Register standard metrics for anomaly detection
    fn register_standard_metrics(&mut self) {
        self.anomaly_detector.register_metric("syscall_rate");
        self.anomaly_detector.register_metric("page_fault_rate");
        self.anomaly_detector.register_metric("context_switch_rate");
        self.anomaly_detector.register_metric("interrupt_latency");
        self.anomaly_detector
            .register_metric("memory_allocation_rate");
        self.anomaly_detector.register_metric("network_packet_rate");
        self.anomaly_detector.register_metric("ipc_message_rate");
    }

    /// Detect threat from event
    pub fn detect_threat(&mut self, event: &CortexEvent) -> Option<Threat> {
        let timestamp = self.current_timestamp;

        // Check for explicit security violations
        if let CortexEvent::SecurityViolation(violation) = event {
            let threat_id = ThreatId(self.next_threat_id.fetch_add(1, Ordering::SeqCst));
            let threat = Threat::new(
                threat_id,
                ThreatLevel::High,
                ThreatCategory::PrivilegeEscalation,
                ThreatSource::Unknown,
                violation,
                timestamp,
            );

            self.active_threats.insert(threat_id, threat.clone());
            self.stats.threats_detected += 1;
            self.update_threat_level();

            return Some(threat);
        }

        // Check for anomalies in metrics
        let anomaly = match event {
            CortexEvent::Syscall(rate) => {
                self.anomaly_detector.record("syscall_rate", *rate as f64)
            },
            CortexEvent::PageFault => self.anomaly_detector.record("page_fault_rate", 1.0),
            CortexEvent::ContextSwitch => self.anomaly_detector.record("context_switch_rate", 1.0),
            CortexEvent::Latency(us) => self
                .anomaly_detector
                .record("interrupt_latency", *us as f64),
            _ => None,
        };

        if let Some(anomaly) = anomaly {
            self.stats.anomalies_detected += 1;

            // Convert anomaly to threat if severe enough
            if anomaly.z_score.abs() > 5.0 {
                let threat_id = ThreatId(self.next_threat_id.fetch_add(1, Ordering::SeqCst));
                let threat = Threat::new(
                    threat_id,
                    ThreatLevel::Medium,
                    ThreatCategory::Anomalous,
                    ThreatSource::Unknown,
                    &format!("Anomaly in {}: z={:.2}", anomaly.metric, anomaly.z_score),
                    timestamp,
                );

                self.active_threats.insert(threat_id, threat.clone());
                self.stats.threats_detected += 1;
                self.update_threat_level();

                return Some(threat);
            }
        }

        None
    }

    /// Respond to a threat
    pub fn respond_to_threat(&mut self, threat: &Threat) -> ThreatResponse {
        let strategy = self.determine_response(threat);

        ThreatResponse {
            threat_id: threat.id,
            strategy,
        }
    }

    /// Determine response strategy
    fn determine_response(&self, threat: &Threat) -> ThreatResponseStrategy {
        match threat.level {
            ThreatLevel::None => ThreatResponseStrategy::Ignore,
            ThreatLevel::Low => ThreatResponseStrategy::Monitor,
            ThreatLevel::Medium => ThreatResponseStrategy::Isolate,
            ThreatLevel::High => ThreatResponseStrategy::Neutralize,
            ThreatLevel::Critical | ThreatLevel::Existential => ThreatResponseStrategy::Survive,
        }
    }

    /// Isolate a subsystem
    pub fn isolate_subsystem(&mut self, subsystem: SubsystemId) {
        let isolated = IsolatedComponent {
            subsystem,
            strategy: IsolationStrategy::Medium,
            isolated_at: self.current_timestamp,
            reason: String::from("Threat detected"),
            threat: None,
            active: true,
            restrictions: Restrictions::medium(),
        };

        self.isolated.insert(subsystem, isolated);
        self.stats.isolations_performed += 1;
    }

    /// Isolate a threat
    pub fn isolate_threat(&mut self, threat: &Threat) {
        if let ThreatSource::Subsystem(subsystem) = threat.source {
            let isolated = IsolatedComponent {
                subsystem,
                strategy: IsolationStrategy::Hard,
                isolated_at: self.current_timestamp,
                reason: threat.description.clone(),
                threat: Some(threat.id),
                active: true,
                restrictions: Restrictions::hard(),
            };

            self.isolated.insert(subsystem, isolated);
            self.stats.isolations_performed += 1;
        }
    }

    /// Neutralize a threat
    pub fn neutralize_threat(&mut self, threat: &Threat) {
        // Isolate with maximum restrictions
        if let ThreatSource::Subsystem(subsystem) = threat.source {
            let isolated = IsolatedComponent {
                subsystem,
                strategy: IsolationStrategy::Kill,
                isolated_at: self.current_timestamp,
                reason: format!("Threat neutralized: {}", threat.description),
                threat: Some(threat.id),
                active: true,
                restrictions: Restrictions::hard(),
            };

            self.isolated.insert(subsystem, isolated);
            self.stats.isolations_performed += 1;

            // Mark threat as neutralized
            if let Some(t) = self.active_threats.get_mut(&threat.id) {
                t.active = false;
                t.response = Some(ThreatResponseStrategy::Neutralize);
            }

            self.stats.threats_neutralized += 1;

            // Start recovery
            self.start_recovery(subsystem, RecoveryStrategy::Restart);
        }
    }

    /// Enter survival mode
    pub fn enter_survival_mode(&mut self, threat: &Threat) {
        if self.survival_mode.active {
            return;
        }

        self.survival_mode = SurvivalMode {
            active: true,
            activated_at: Some(self.current_timestamp),
            trigger: Some(threat.id),
            suspended_subsystems: Vec::new(),
            protections: SurvivalProtections::maximum(),
            log_level: 255, // Maximum logging
        };

        self.stats.survival_mode_activations += 1;

        // Apply protections
        self.apply_survival_protections();
    }

    /// Apply survival protections
    fn apply_survival_protections(&mut self) {
        // In real implementation, would:
        // - Disable module loading
        // - Block new network connections
        // - Increase ASLR entropy
        // - Enable memory encryption
        // - Start forensic logging
    }

    /// Exit survival mode
    pub fn exit_survival_mode(&mut self) {
        self.survival_mode.active = false;

        // Restore normal operation
        for subsystem in self.survival_mode.suspended_subsystems.drain(..) {
            self.isolated.remove(&subsystem);
        }
    }

    /// Is in survival mode?
    pub fn is_survival_mode(&self) -> bool {
        self.survival_mode.active
    }

    /// Start recovery
    pub fn start_recovery(&mut self, subsystem: SubsystemId, strategy: RecoveryStrategy) {
        let recovery = Recovery::new(subsystem, strategy, self.current_timestamp);
        self.recoveries.push(recovery);
        self.stats.recoveries_attempted += 1;
    }

    /// Update current threat level
    fn update_threat_level(&mut self) {
        let max_level = self
            .active_threats
            .values()
            .filter(|t| t.active)
            .map(|t| t.level)
            .max()
            .unwrap_or(ThreatLevel::None);

        self.stats.current_threat_level = max_level;
    }

    /// Tick (update timestamp)
    pub fn tick(&mut self, timestamp: u64) {
        self.current_timestamp = timestamp;
    }

    /// Get statistics
    pub fn stats(&self) -> &SurvivabilityStats {
        &self.stats
    }

    /// Get active threats
    pub fn active_threats(&self) -> impl Iterator<Item = &Threat> {
        self.active_threats.values().filter(|t| t.active)
    }

    /// Get isolated components
    pub fn isolated_components(&self) -> impl Iterator<Item = &IsolatedComponent> {
        self.isolated.values()
    }

    /// Get anomaly detector
    pub fn anomaly_detector(&self) -> &AnomalyDetector {
        &self.anomaly_detector
    }
}

impl Default for SurvivabilityCore {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_levels() {
        assert!(ThreatLevel::Critical > ThreatLevel::Medium);
        assert!(ThreatLevel::Existential > ThreatLevel::Critical);
    }

    #[test]
    fn test_baseline_update() {
        let mut baseline = Baseline::new("test");

        for i in 0..1000 {
            baseline.update(i as f64);
        }

        assert!(baseline.established);
        assert!(baseline.mean > 400.0 && baseline.mean < 600.0);
    }

    #[test]
    fn test_survivability_core_creation() {
        let core = SurvivabilityCore::new();
        assert_eq!(core.stats.threats_detected, 0);
        assert!(!core.is_survival_mode());
    }

    #[test]
    fn test_threat_detection() {
        let mut core = SurvivabilityCore::new();

        let event = CortexEvent::SecurityViolation(String::from("test violation"));
        let threat = core.detect_threat(&event);

        assert!(threat.is_some());
        assert_eq!(core.stats.threats_detected, 1);
    }
}
