//! # Self-Healing Engine
//!
//! The Healer detects, diagnoses, and repairs system issues automatically
//! without manual intervention or system restarts.
//!
//! ## Capabilities
//!
//! - **Bug Detection**: Identify anomalous behavior and crashes
//! - **Root Cause Analysis**: Trace issues to their source
//! - **Hot Patching**: Apply fixes without reboot
//! - **State Recovery**: Restore components to healthy state
//! - **Module Restart**: Gracefully restart failing modules
//! - **Rollback**: Revert problematic changes
//!
//! ## Architecture
//!
//! ```text
//!   System Events ────►┌─────────────────────────────────────┐
//!                      │            Healer                    │
//!   Error Reports ────►│                                     │
//!                      │  ┌─────────────────────────────┐    │
//!   Crash Data ───────►│  │    Anomaly Detector         │    │
//!                      │  │  - Pattern Recognition      │    │
//!                      │  │  - Statistical Analysis     │    │
//!                      │  │  - Crash Signature Match    │    │
//!                      │  └──────────────┬──────────────┘    │
//!                      │                 │                    │
//!                      │                 ▼                    │
//!                      │  ┌─────────────────────────────┐    │
//!                      │  │    Root Cause Analyzer      │    │
//!                      │  │  - Stack Trace Analysis     │    │
//!                      │  │  - Dependency Tracking      │    │
//!                      │  │  - State Inspection         │    │
//!                      │  └──────────────┬──────────────┘    │
//!                      │                 │                    │
//!                      │                 ▼                    │
//!                      │  ┌─────────────────────────────┐    │
//!                      │  │    Repair Strategy Selector │    │
//!                      │  │  - Hot Patch                │    │
//!                      │  │  - Module Restart           │    │
//!                      │  │  - State Reset              │    │
//!                      │  │  - Rollback                 │    │
//!                      │  └──────────────┬──────────────┘    │
//!                      │                 │                    │
//!                      │                 ▼                    │
//!                      │  ┌─────────────────────────────┐    │
//!                      │  │    Repair Executor          │───────► Healing Actions
//!                      │  └─────────────────────────────┘    │
//!                      │                                     │
//!                      └─────────────────────────────────────┘
//! ```

use crate::core::{
    AiAction, AiEvent, Confidence, DecisionContext,
};

use alloc::{
    collections::VecDeque,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::{Mutex, RwLock};

// =============================================================================
// Bug Signatures
// =============================================================================

/// Signature of a known bug or issue
#[derive(Debug, Clone)]
pub struct BugSignature {
    /// Unique identifier
    pub id: u64,
    /// Human-readable name
    pub name: String,
    /// Description of the bug
    pub description: String,
    /// Patterns that indicate this bug
    pub patterns: Vec<BugPattern>,
    /// Severity level
    pub severity: BugSeverity,
    /// Known fixes
    pub fixes: Vec<HealingAction>,
    /// Number of times this bug has been seen
    pub occurrence_count: u64,
    /// Last occurrence timestamp
    pub last_seen: u64,
}

/// Pattern that indicates a bug
#[derive(Debug, Clone)]
pub enum BugPattern {
    /// Error message substring
    ErrorMessage(String),
    /// Stack trace pattern
    StackTrace { module: String, function: String },
    /// Resource exhaustion
    ResourceExhaustion { resource: String, threshold: u64 },
    /// Repeated event pattern
    RepeatedEvent { event_type: String, min_count: u32, window_ms: u64 },
    /// Metric threshold
    MetricThreshold { metric: String, operator: ThresholdOperator, value: f64 },
    /// Crash signature
    CrashSignature { signal: u32, address_range: Option<(u64, u64)> },
}

/// Comparison operators for thresholds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdOperator {
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Equal,
}

/// Bug severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BugSeverity {
    /// Cosmetic or minor issue
    Low = 0,
    /// Noticeable but not blocking
    Medium = 1,
    /// Significant impact on functionality
    High = 2,
    /// System stability at risk
    Critical = 3,
    /// System failure imminent
    Emergency = 4,
}

// =============================================================================
// Healing Actions
// =============================================================================

/// Action to heal a detected issue
#[derive(Debug, Clone)]
pub enum HealingAction {
    /// Apply a hot patch to code
    HotPatch {
        patch_id: u64,
        target_module: String,
        patch_data: Vec<u8>,
    },

    /// Restart a module
    RestartModule {
        module_id: u64,
        preserve_state: bool,
        delay_ms: u64,
    },

    /// Reset module state
    ResetState {
        module_id: u64,
        state_key: String,
    },

    /// Rollback to previous version
    Rollback {
        module_id: u64,
        target_version: u64,
    },

    /// Clear cache or buffer
    ClearCache {
        cache_id: u32,
        scope: CacheClearScope,
    },

    /// Isolate failing component
    Isolate {
        component_id: u64,
        isolation_level: IsolationLevel,
    },

    /// Reinitialize subsystem
    Reinitialize {
        subsystem: String,
        config: Vec<u8>,
    },

    /// Kill and restart process
    RestartProcess {
        pid: u64,
        preserve_files: bool,
    },

    /// Increase resource limits
    IncreaseResource {
        resource: String,
        amount: u64,
    },

    /// Force garbage collection
    ForceGc {
        scope: GcScope,
    },

    /// Apply workaround
    ApplyWorkaround {
        workaround_id: u64,
        description: String,
    },

    /// Composite: try multiple actions in order
    Sequence(Vec<HealingAction>),

    /// Escalate to human intervention
    Escalate {
        reason: String,
        severity: BugSeverity,
    },
}

/// Cache clear scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheClearScope {
    Full,
    Partial(u32), // Percentage
    Selective,    // Only invalid entries
}

/// Isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Light isolation - monitor only
    Monitor = 0,
    /// Restrict resources
    ResourceLimit = 1,
    /// Sandbox - limited syscalls
    Sandbox = 2,
    /// Full isolation - no external access
    Full = 3,
}

/// Garbage collection scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcScope {
    Memory,
    FileDescriptors,
    Handles,
    All,
}

// =============================================================================
// Patch Management
// =============================================================================

/// A hot patch that can be applied
#[derive(Debug, Clone)]
pub struct HotPatch {
    /// Unique identifier
    pub id: u64,
    /// Target module
    pub target_module: String,
    /// Target function (if applicable)
    pub target_function: Option<String>,
    /// Patch type
    pub patch_type: PatchType,
    /// Patch binary data
    pub data: Vec<u8>,
    /// Bug signatures this fixes
    pub fixes_bugs: Vec<u64>,
    /// Whether patch is reversible
    pub reversible: bool,
    /// Rollback data
    pub rollback_data: Option<Vec<u8>>,
}

/// Types of patches
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchType {
    /// Replace function entirely
    FunctionReplace,
    /// Trampoline/hook
    Trampoline,
    /// Data patch
    DataPatch,
    /// Configuration change
    ConfigPatch,
}

/// Patch application result
#[derive(Debug, Clone)]
pub enum PatchResult {
    Success,
    PartialSuccess { applied: usize, total: usize },
    Failed { reason: String },
    Reverted { reason: String },
}

// =============================================================================
// Health Status
// =============================================================================

/// Health status of a component
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// Component identifier
    pub component_id: u64,
    /// Component name
    pub name: String,
    /// Health score (0-100)
    pub health_score: u8,
    /// Current issues
    pub issues: Vec<DetectedIssue>,
    /// Recent crashes
    pub recent_crashes: u32,
    /// Uptime since last restart (seconds)
    pub uptime_s: u64,
    /// Applied patches
    pub applied_patches: Vec<u64>,
    /// Last health check timestamp
    pub last_check: u64,
}

/// A detected issue
#[derive(Debug, Clone)]
pub struct DetectedIssue {
    /// Issue identifier
    pub id: u64,
    /// Bug signature (if matched)
    pub bug_signature: Option<u64>,
    /// Detection timestamp
    pub detected_at: u64,
    /// Issue description
    pub description: String,
    /// Severity
    pub severity: BugSeverity,
    /// Status
    pub status: IssueStatus,
    /// Applied fixes
    pub applied_fixes: Vec<u64>,
}

/// Issue status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueStatus {
    Detected,
    Analyzing,
    Repairing,
    Repaired,
    Monitoring,
    Escalated,
    Ignored,
}

// =============================================================================
// Healer Engine
// =============================================================================

/// The Self-Healing Engine
pub struct Healer {
    /// Whether healing is enabled
    enabled: bool,

    /// Known bug signatures
    bug_signatures: RwLock<Vec<BugSignature>>,

    /// Available patches
    patches: RwLock<Vec<HotPatch>>,

    /// Component health status
    component_health: RwLock<Vec<ComponentHealth>>,

    /// Active issues
    active_issues: Mutex<Vec<DetectedIssue>>,

    /// Issue history
    issue_history: Mutex<VecDeque<DetectedIssue>>,

    /// Event buffer for pattern detection
    event_buffer: Mutex<VecDeque<BufferedEvent>>,

    /// Statistics
    stats: HealerStats,
}

/// Buffered event for pattern analysis
#[derive(Debug, Clone)]
struct BufferedEvent {
    timestamp: u64,
    event: AiEvent,
    severity: BugSeverity,
}

struct HealerStats {
    issues_detected: AtomicU64,
    issues_repaired: AtomicU64,
    patches_applied: AtomicU64,
    modules_restarted: AtomicU64,
    escalations: AtomicU64,
    false_positives: AtomicU64,
}

impl Default for HealerStats {
    fn default() -> Self {
        Self {
            issues_detected: AtomicU64::new(0),
            issues_repaired: AtomicU64::new(0),
            patches_applied: AtomicU64::new(0),
            modules_restarted: AtomicU64::new(0),
            escalations: AtomicU64::new(0),
            false_positives: AtomicU64::new(0),
        }
    }
}

impl Healer {
    /// Maximum event buffer size
    const MAX_EVENT_BUFFER: usize = 1000;
    /// Maximum issue history size
    const MAX_ISSUE_HISTORY: usize = 1000;

    /// Create a new Healer
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            bug_signatures: RwLock::new(Self::default_signatures()),
            patches: RwLock::new(Vec::new()),
            component_health: RwLock::new(Vec::new()),
            active_issues: Mutex::new(Vec::new()),
            issue_history: Mutex::new(VecDeque::with_capacity(Self::MAX_ISSUE_HISTORY)),
            event_buffer: Mutex::new(VecDeque::with_capacity(Self::MAX_EVENT_BUFFER)),
            stats: HealerStats::default(),
        }
    }

    /// Default bug signatures based on common issues
    fn default_signatures() -> Vec<BugSignature> {
        vec![
            BugSignature {
                id: 1,
                name: "Memory Leak".to_string(),
                description: "Gradual memory consumption increase".to_string(),
                patterns: vec![
                    BugPattern::MetricThreshold {
                        metric: "memory_growth_rate".to_string(),
                        operator: ThresholdOperator::GreaterThan,
                        value: 0.1, // 10% growth per hour
                    },
                ],
                severity: BugSeverity::High,
                fixes: vec![
                    HealingAction::ForceGc { scope: GcScope::Memory },
                    HealingAction::RestartModule {
                        module_id: 0, // Will be filled
                        preserve_state: true,
                        delay_ms: 1000,
                    },
                ],
                occurrence_count: 0,
                last_seen: 0,
            },
            BugSignature {
                id: 2,
                name: "Deadlock".to_string(),
                description: "Suspected deadlock detected".to_string(),
                patterns: vec![
                    BugPattern::MetricThreshold {
                        metric: "thread_blocked_time_ms".to_string(),
                        operator: ThresholdOperator::GreaterThan,
                        value: 30000.0, // 30 seconds
                    },
                ],
                severity: BugSeverity::Critical,
                fixes: vec![
                    HealingAction::Escalate {
                        reason: "Deadlock requires manual intervention".to_string(),
                        severity: BugSeverity::Critical,
                    },
                ],
                occurrence_count: 0,
                last_seen: 0,
            },
            BugSignature {
                id: 3,
                name: "Crash Loop".to_string(),
                description: "Module crashing repeatedly".to_string(),
                patterns: vec![
                    BugPattern::RepeatedEvent {
                        event_type: "module_crash".to_string(),
                        min_count: 3,
                        window_ms: 60000, // 3 crashes in 1 minute
                    },
                ],
                severity: BugSeverity::Critical,
                fixes: vec![
                    HealingAction::Rollback {
                        module_id: 0,
                        target_version: 0, // Previous version
                    },
                    HealingAction::Isolate {
                        component_id: 0,
                        isolation_level: IsolationLevel::Sandbox,
                    },
                ],
                occurrence_count: 0,
                last_seen: 0,
            },
            BugSignature {
                id: 4,
                name: "Resource Exhaustion".to_string(),
                description: "System running out of resources".to_string(),
                patterns: vec![
                    BugPattern::ResourceExhaustion {
                        resource: "file_descriptors".to_string(),
                        threshold: 90, // 90% used
                    },
                ],
                severity: BugSeverity::High,
                fixes: vec![
                    HealingAction::ForceGc { scope: GcScope::FileDescriptors },
                    HealingAction::IncreaseResource {
                        resource: "file_descriptors".to_string(),
                        amount: 1024,
                    },
                ],
                occurrence_count: 0,
                last_seen: 0,
            },
            BugSignature {
                id: 5,
                name: "Cache Corruption".to_string(),
                description: "Cache data integrity failure".to_string(),
                patterns: vec![
                    BugPattern::ErrorMessage("cache checksum mismatch".to_string()),
                ],
                severity: BugSeverity::Medium,
                fixes: vec![
                    HealingAction::ClearCache {
                        cache_id: 0,
                        scope: CacheClearScope::Full,
                    },
                ],
                occurrence_count: 0,
                last_seen: 0,
            },
        ]
    }

    /// Check if healer is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Analyze an event for potential issues
    pub fn analyze(
        &self,
        event: &AiEvent,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        if !self.enabled {
            return Ok(None);
        }

        // Buffer event for pattern analysis
        self.buffer_event(event);

        match event {
            AiEvent::ModuleError { module_id, error } => {
                self.handle_module_error(*module_id, error, context)
            }
            AiEvent::HardwareError { device_id, error_code } => {
                self.handle_hardware_error(*device_id, *error_code, context)
            }
            AiEvent::AnomalyDetected { source, severity } => {
                self.handle_anomaly(source, *severity, context)
            }
            AiEvent::ProcessResourceSpike { pid, resource } => {
                self.handle_resource_spike(*pid, resource, context)
            }
            _ => {
                // Check for pattern matches
                self.check_patterns(context)
            }
        }
    }

    /// Buffer an event for pattern analysis
    fn buffer_event(&self, event: &AiEvent) {
        let mut buffer = self.event_buffer.lock();

        while buffer.len() >= Self::MAX_EVENT_BUFFER {
            buffer.pop_front();
        }

        buffer.push_back(BufferedEvent {
            timestamp: 0, // Would be real timestamp
            event: event.clone(),
            severity: BugSeverity::Low,
        });
    }

    /// Handle module error event
    fn handle_module_error(
        &self,
        module_id: u64,
        error: &str,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        // Try to match against known bug signatures
        if let Some(signature) = self.match_error_signature(error) {
            self.record_issue(module_id, signature.id, error);

            // Get first recommended fix
            if let Some(fix) = signature.fixes.first() {
                let action = self.healing_to_ai_action(fix, module_id);
                return Ok(Some((
                    action,
                    Confidence::new(0.8),
                    format!("Detected '{}': {}", signature.name, signature.description),
                )));
            }
        }

        // Unknown error - restart module as fallback
        Ok(Some((
            AiAction::RestartModule {
                module_id,
                module_name: String::from("unknown"),
            },
            Confidence::new(0.6),
            format!("Module {} error: {}. Attempting restart.", module_id, error),
        )))
    }

    /// Handle hardware error
    fn handle_hardware_error(
        &self,
        device_id: u32,
        error_code: u32,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        // Hardware errors are serious - log and potentially isolate
        self.stats.issues_detected.fetch_add(1, Ordering::Relaxed);

        let action = if error_code > 1000 {
            // Critical hardware error
            AiAction::Sequence(vec![
                AiAction::ResetCache { cache_id: device_id },
                AiAction::TuneIoScheduler {
                    parameter: "device_timeout".to_string(),
                    value: 30000, // Increase timeout
                },
            ])
        } else {
            // Minor hardware error - just log
            AiAction::NoOp
        };

        if matches!(action, AiAction::NoOp) {
            Ok(None)
        } else {
            Ok(Some((
                action,
                Confidence::new(0.7),
                format!("Hardware error on device {}: code {}", device_id, error_code),
            )))
        }
    }

    /// Handle anomaly detection
    fn handle_anomaly(
        &self,
        source: &str,
        severity: u8,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        self.stats.issues_detected.fetch_add(1, Ordering::Relaxed);

        if severity < 5 {
            return Ok(None); // Low severity, just monitor
        }

        let action = if severity >= 8 {
            // High severity - investigate and potentially isolate
            AiAction::TriggerSecurityScan {
                scope: crate::core::SecurityScanScope::QuickScan,
            }
        } else {
            // Medium severity - log and monitor
            AiAction::NoOp
        };

        if matches!(action, AiAction::NoOp) {
            Ok(None)
        } else {
            Ok(Some((
                action,
                Confidence::new(0.6 + (severity as f32 / 20.0)),
                format!("Anomaly detected from {}: severity {}", source, severity),
            )))
        }
    }

    /// Handle resource spike
    fn handle_resource_spike(
        &self,
        pid: u64,
        resource: &crate::core::ResourceType,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        use crate::core::ResourceType;

        let action = match resource {
            ResourceType::Memory => {
                AiAction::IsolateProcess {
                    pid,
                    isolation_level: 1,
                }
            }
            ResourceType::Cpu => {
                AiAction::AdjustProcessPriority {
                    pid,
                    old_priority: 0,
                    new_priority: 19, // Nice to lowest priority
                }
            }
            _ => return Ok(None),
        };

        Ok(Some((
            action,
            Confidence::new(0.7),
            format!("Process {} resource spike: {:?}", pid, resource),
        )))
    }

    /// Check for pattern matches in buffered events
    fn check_patterns(
        &self,
        context: &DecisionContext,
    ) -> Result<Option<(AiAction, Confidence, String)>, ()> {
        let signatures = self.bug_signatures.read();
        let buffer = self.event_buffer.lock();

        for sig in signatures.iter() {
            for pattern in &sig.patterns {
                if let BugPattern::RepeatedEvent { event_type, min_count, window_ms } = pattern {
                    let count = self.count_events_in_window(&buffer, event_type, *window_ms);
                    if count >= *min_count {
                        if let Some(fix) = sig.fixes.first() {
                            let action = self.healing_to_ai_action(fix, 0);
                            return Ok(Some((
                                action,
                                Confidence::new(0.75),
                                format!("Pattern detected: {} ({} occurrences)", sig.name, count),
                            )));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Count events of a type in a time window
    fn count_events_in_window(
        &self,
        buffer: &VecDeque<BufferedEvent>,
        event_type: &str,
        _window_ms: u64,
    ) -> u32 {
        // Simplified: just count matching events in buffer
        buffer
            .iter()
            .filter(|e| format!("{:?}", e.event).contains(event_type))
            .count() as u32
    }

    /// Match error message against known signatures
    fn match_error_signature(&self, error: &str) -> Option<BugSignature> {
        let signatures = self.bug_signatures.read();
        let error_lower = error.to_lowercase();

        for sig in signatures.iter() {
            for pattern in &sig.patterns {
                if let BugPattern::ErrorMessage(msg) = pattern {
                    if error_lower.contains(&msg.to_lowercase()) {
                        return Some(sig.clone());
                    }
                }
            }
        }

        None
    }

    /// Convert healing action to AI action
    fn healing_to_ai_action(&self, healing: &HealingAction, default_module: u64) -> AiAction {
        match healing {
            HealingAction::HotPatch { patch_id, target_module, .. } => {
                AiAction::ApplyPatch {
                    patch_id: *patch_id,
                    target: target_module.clone(),
                }
            }
            HealingAction::RestartModule { module_id,  .. } => {
                AiAction::RestartModule {
                    module_id: if *module_id == 0 { default_module } else { *module_id },
                    module_name: String::from("unknown"),
                }
            }
            HealingAction::Rollback { module_id, target_version } => {
                AiAction::RollbackModule {
                    module_id: if *module_id == 0 { default_module } else { *module_id },
                    target_version: *target_version,
                }
            }
            HealingAction::ClearCache { cache_id, .. } => {
                AiAction::ResetCache { cache_id: *cache_id }
            }
            HealingAction::Isolate { component_id, isolation_level } => {
                AiAction::IsolateProcess {
                    pid: *component_id,
                    isolation_level: *isolation_level as u8,
                }
            }
            HealingAction::ForceGc { .. } => {
                // Translate to memory reclaim
                AiAction::TuneAllocator {
                    strategy: String::from("force_gc"),
                }
            }
            HealingAction::Sequence(actions) => {
                AiAction::Sequence(
                    actions
                        .iter()
                        .map(|a| self.healing_to_ai_action(a, default_module))
                        .collect(),
                )
            }
            HealingAction::Escalate { reason, severity } => {
                // Log escalation, return no-op (human intervention needed)
                self.stats.escalations.fetch_add(1, Ordering::Relaxed);
                log::warn!("ESCALATION ({:?}): {}", severity, reason);
                AiAction::NoOp
            }
            _ => AiAction::NoOp,
        }
    }

    /// Record a detected issue
    fn record_issue(&self, component_id: u64, bug_signature: u64, description: &str) {
        static ISSUE_COUNTER: AtomicU64 = AtomicU64::new(1);

        let issue = DetectedIssue {
            id: ISSUE_COUNTER.fetch_add(1, Ordering::Relaxed),
            bug_signature: Some(bug_signature),
            detected_at: 0, // Would be real timestamp
            description: description.to_string(),
            severity: BugSeverity::Medium,
            status: IssueStatus::Detected,
            applied_fixes: Vec::new(),
        };

        self.active_issues.lock().push(issue.clone());
        self.stats.issues_detected.fetch_add(1, Ordering::Relaxed);

        // Update bug signature occurrence
        if let Some(sig) = self.bug_signatures.write().iter_mut().find(|s| s.id == bug_signature) {
            sig.occurrence_count += 1;
            sig.last_seen = 0; // Would be real timestamp
        }
    }

    /// Register a hot patch
    pub fn register_patch(&self, patch: HotPatch) {
        self.patches.write().push(patch);
    }

    /// Register a bug signature
    pub fn register_signature(&self, signature: BugSignature) {
        self.bug_signatures.write().push(signature);
    }

    /// Get component health
    pub fn get_health(&self, component_id: u64) -> Option<ComponentHealth> {
        self.component_health
            .read()
            .iter()
            .find(|h| h.component_id == component_id)
            .cloned()
    }

    /// Update component health
    pub fn update_health(&self, health: ComponentHealth) {
        let mut healths = self.component_health.write();
        if let Some(existing) = healths.iter_mut().find(|h| h.component_id == health.component_id) {
            *existing = health;
        } else {
            healths.push(health);
        }
    }

    /// Get active issues
    pub fn active_issues(&self) -> Vec<DetectedIssue> {
        self.active_issues.lock().clone()
    }

    /// Mark issue as repaired
    pub fn mark_repaired(&self, issue_id: u64) {
        let mut active = self.active_issues.lock();
        let mut history = self.issue_history.lock();

        if let Some(pos) = active.iter().position(|i| i.id == issue_id) {
            let mut issue = active.remove(pos);
            issue.status = IssueStatus::Repaired;

            while history.len() >= Self::MAX_ISSUE_HISTORY {
                history.pop_front();
            }
            history.push_back(issue);

            self.stats.issues_repaired.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get system health score (0-100)
    pub fn system_health_score(&self) -> u8 {
        let active = self.active_issues.lock();

        if active.is_empty() {
            return 100;
        }

        // Deduct points based on severity
        let mut deductions = 0u32;
        for issue in active.iter() {
            deductions += match issue.severity {
                BugSeverity::Low => 2,
                BugSeverity::Medium => 5,
                BugSeverity::High => 15,
                BugSeverity::Critical => 30,
                BugSeverity::Emergency => 50,
            };
        }

        100u8.saturating_sub(deductions.min(100) as u8)
    }

    /// Get statistics
    pub fn statistics(&self) -> HealerStatistics {
        HealerStatistics {
            enabled: self.enabled,
            issues_detected: self.stats.issues_detected.load(Ordering::Relaxed),
            issues_repaired: self.stats.issues_repaired.load(Ordering::Relaxed),
            patches_applied: self.stats.patches_applied.load(Ordering::Relaxed),
            modules_restarted: self.stats.modules_restarted.load(Ordering::Relaxed),
            escalations: self.stats.escalations.load(Ordering::Relaxed),
            false_positives: self.stats.false_positives.load(Ordering::Relaxed),
            active_issues: self.active_issues.lock().len(),
            known_signatures: self.bug_signatures.read().len(),
            available_patches: self.patches.read().len(),
            system_health_score: self.system_health_score(),
        }
    }
}

/// Public statistics structure
#[derive(Debug, Clone)]
pub struct HealerStatistics {
    pub enabled: bool,
    pub issues_detected: u64,
    pub issues_repaired: u64,
    pub patches_applied: u64,
    pub modules_restarted: u64,
    pub escalations: u64,
    pub false_positives: u64,
    pub active_issues: usize,
    pub known_signatures: usize,
    pub available_patches: usize,
    pub system_health_score: u8,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healer_creation() {
        let healer = Healer::new(true);
        assert!(healer.is_enabled());
        assert_eq!(healer.system_health_score(), 100);
    }

    #[test]
    fn test_error_signature_matching() {
        let healer = Healer::new(true);

        // Add a custom signature
        healer.register_signature(BugSignature {
            id: 100,
            name: "Test Bug".to_string(),
            description: "A test bug".to_string(),
            patterns: vec![BugPattern::ErrorMessage("test error message".to_string())],
            severity: BugSeverity::Medium,
            fixes: vec![],
            occurrence_count: 0,
            last_seen: 0,
        });

        let matched = healer.match_error_signature("found test error message in log");
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().name, "Test Bug");
    }

    #[test]
    fn test_issue_tracking() {
        let healer = Healer::new(true);

        healer.record_issue(1, 1, "Test issue");
        assert_eq!(healer.active_issues().len(), 1);

        let issue_id = healer.active_issues()[0].id;
        healer.mark_repaired(issue_id);
        assert_eq!(healer.active_issues().len(), 0);
    }

    #[test]
    fn test_health_score_calculation() {
        let healer = Healer::new(true);

        // Perfect health
        assert_eq!(healer.system_health_score(), 100);

        // Add some issues
        healer.record_issue(1, 1, "Minor issue");
        assert!(healer.system_health_score() < 100);

        // Add critical issue
        {
            let mut active = healer.active_issues.lock();
            active.push(DetectedIssue {
                id: 999,
                bug_signature: None,
                detected_at: 0,
                description: "Critical issue".to_string(),
                severity: BugSeverity::Critical,
                status: IssueStatus::Detected,
                applied_fixes: Vec::new(),
            });
        }
        assert!(healer.system_health_score() < 80);
    }

    #[test]
    fn test_patch_registration() {
        let healer = Healer::new(true);

        healer.register_patch(HotPatch {
            id: 1,
            target_module: "test_module".to_string(),
            target_function: Some("broken_function".to_string()),
            patch_type: PatchType::FunctionReplace,
            data: vec![0x90], // NOP
            fixes_bugs: vec![1],
            reversible: true,
            rollback_data: Some(vec![0x00]),
        });

        let stats = healer.statistics();
        assert_eq!(stats.available_patches, 1);
    }
}
