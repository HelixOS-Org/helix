//! # Emergency Response
//!
//! System emergency detection and graceful degradation:
//! - Emergency level classification
//! - Resource triage
//! - Service degradation policies
//! - Recovery procedures
//! - Watchdog integration
//! - Panic handler coordination

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// EMERGENCY LEVELS
// ============================================================================

/// System emergency level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EmergencyLevel {
    /// Normal operation
    Normal = 0,
    /// Elevated - increased monitoring
    Elevated = 1,
    /// Warning - proactive measures
    Warning = 2,
    /// Critical - active degradation
    Critical = 3,
    /// Severe - minimal services only
    Severe = 4,
    /// Panic - last resort before shutdown
    Panic = 5,
}

impl EmergencyLevel {
    /// Maximum number of non-essential services
    pub fn allowed_services(&self) -> u32 {
        match self {
            Self::Normal => u32::MAX,
            Self::Elevated => u32::MAX,
            Self::Warning => 100,
            Self::Critical => 20,
            Self::Severe => 5,
            Self::Panic => 0,
        }
    }

    /// CPU budget for non-essential work (percent)
    pub fn non_essential_cpu_budget(&self) -> u32 {
        match self {
            Self::Normal => 100,
            Self::Elevated => 80,
            Self::Warning => 50,
            Self::Critical => 20,
            Self::Severe => 5,
            Self::Panic => 0,
        }
    }

    /// Memory reservation for essential services (percent)
    pub fn essential_memory_reserve(&self) -> u32 {
        match self {
            Self::Normal => 10,
            Self::Elevated => 20,
            Self::Warning => 30,
            Self::Critical => 50,
            Self::Severe => 80,
            Self::Panic => 95,
        }
    }
}

// ============================================================================
// EMERGENCY TRIGGERS
// ============================================================================

/// What triggered the emergency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyTrigger {
    /// Memory almost exhausted
    MemoryExhaustion,
    /// OOM killer invoked
    OomKill,
    /// CPU 100% for extended time
    CpuSaturation,
    /// Thermal throttling
    ThermalThrottle,
    /// Critical temperature
    ThermalCritical,
    /// Disk full
    DiskFull,
    /// Kernel stack overflow
    StackOverflow,
    /// Deadlock detected
    DeadlockDetected,
    /// Watchdog timeout
    WatchdogTimeout,
    /// Hardware failure
    HardwareFailure,
    /// Power loss / battery critical
    PowerCritical,
    /// Security breach
    SecurityBreach,
    /// Cascading failures
    CascadingFailure,
}

impl EmergencyTrigger {
    /// Default level for this trigger
    pub fn default_level(&self) -> EmergencyLevel {
        match self {
            Self::MemoryExhaustion => EmergencyLevel::Critical,
            Self::OomKill => EmergencyLevel::Severe,
            Self::CpuSaturation => EmergencyLevel::Warning,
            Self::ThermalThrottle => EmergencyLevel::Warning,
            Self::ThermalCritical => EmergencyLevel::Severe,
            Self::DiskFull => EmergencyLevel::Critical,
            Self::StackOverflow => EmergencyLevel::Panic,
            Self::DeadlockDetected => EmergencyLevel::Critical,
            Self::WatchdogTimeout => EmergencyLevel::Severe,
            Self::HardwareFailure => EmergencyLevel::Severe,
            Self::PowerCritical => EmergencyLevel::Severe,
            Self::SecurityBreach => EmergencyLevel::Critical,
            Self::CascadingFailure => EmergencyLevel::Severe,
        }
    }

    /// Can this trigger be auto-resolved?
    pub fn auto_resolvable(&self) -> bool {
        matches!(
            self,
            Self::MemoryExhaustion
                | Self::CpuSaturation
                | Self::ThermalThrottle
                | Self::DiskFull
        )
    }
}

/// Emergency event
#[derive(Debug, Clone)]
pub struct EmergencyEvent {
    /// Event ID
    pub id: u64,
    /// Trigger
    pub trigger: EmergencyTrigger,
    /// Level
    pub level: EmergencyLevel,
    /// Timestamp
    pub timestamp: u64,
    /// Resolved
    pub resolved: bool,
    /// Resolution time
    pub resolved_at: Option<u64>,
    /// Actions taken
    pub actions_taken: Vec<EmergencyAction>,
}

// ============================================================================
// DEGRADATION POLICIES
// ============================================================================

/// Service priority for degradation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServicePriority {
    /// Kernel core (never degrade)
    Kernel = 0,
    /// Essential system services
    Essential = 1,
    /// Important services
    Important = 2,
    /// Standard services
    Standard = 3,
    /// Nice-to-have services
    Optional = 4,
    /// Background / best-effort
    Background = 5,
}

/// Degradation action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationAction {
    /// Reduce quality of service
    ReduceQos,
    /// Suspend service
    Suspend,
    /// Kill service
    Kill,
    /// Disable feature
    DisableFeature,
    /// Reduce memory allocation
    ReduceMemory,
    /// Reduce CPU allocation
    ReduceCpu,
    /// Disable I/O
    DisableIo,
    /// Checkpoint and stop
    Checkpoint,
}

/// Service record
#[derive(Debug, Clone)]
pub struct ServiceRecord {
    /// Process ID
    pub pid: u64,
    /// Service priority
    pub priority: ServicePriority,
    /// Current state
    pub state: ServiceState,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// CPU usage (percent * 100)
    pub cpu_usage: u32,
    /// Is essential at current emergency level
    pub is_essential: bool,
    /// Degradation level applied (0 = none)
    pub degradation_level: u32,
}

/// Service state during emergency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    /// Running normally
    Running,
    /// Running with reduced QoS
    Degraded,
    /// Suspended
    Suspended,
    /// Being checkpointed
    Checkpointing,
    /// Killed
    Killed,
}

// ============================================================================
// EMERGENCY ACTIONS
// ============================================================================

/// Action taken during emergency
#[derive(Debug, Clone)]
pub struct EmergencyAction {
    /// Action type
    pub action: EmergencyActionType,
    /// Target (PID or resource)
    pub target: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Success
    pub success: bool,
}

/// Emergency action types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyActionType {
    /// Kill process
    KillProcess,
    /// Suspend process
    SuspendProcess,
    /// Free cache
    FreeCache,
    /// Swap out
    SwapOut,
    /// Compress memory
    CompressMemory,
    /// Reduce frequency
    ReduceFrequency,
    /// Disable core
    DisableCore,
    /// Flush I/O
    FlushIo,
    /// Checkpoint process
    CheckpointProcess,
    /// Enable emergency swap
    EnableEmergencySwap,
    /// Alert user
    AlertUser,
    /// Initiate shutdown
    InitiateShutdown,
}

// ============================================================================
// WATCHDOG
// ============================================================================

/// Watchdog entry
#[derive(Debug, Clone)]
pub struct WatchdogEntry {
    /// Monitored process/subsystem ID
    pub id: u64,
    /// Last heartbeat
    pub last_heartbeat: u64,
    /// Expected heartbeat interval (ms)
    pub interval_ms: u64,
    /// Max missed heartbeats before timeout
    pub max_missed: u32,
    /// Current missed count
    pub missed_count: u32,
    /// Is timed out
    pub timed_out: bool,
}

impl WatchdogEntry {
    pub fn new(id: u64, interval_ms: u64, max_missed: u32) -> Self {
        Self {
            id,
            last_heartbeat: 0,
            interval_ms,
            max_missed,
            missed_count: 0,
            timed_out: false,
        }
    }

    /// Record heartbeat
    pub fn heartbeat(&mut self, timestamp: u64) {
        self.last_heartbeat = timestamp;
        self.missed_count = 0;
        self.timed_out = false;
    }

    /// Check watchdog
    pub fn check(&mut self, current_time: u64) -> bool {
        if self.last_heartbeat == 0 {
            return true; // Not started yet
        }

        let elapsed = current_time.saturating_sub(self.last_heartbeat);
        let expected = self.interval_ms;

        if elapsed > expected * 2 {
            self.missed_count += 1;
            if self.missed_count >= self.max_missed {
                self.timed_out = true;
                return false;
            }
        }

        true
    }
}

// ============================================================================
// RECOVERY
// ============================================================================

/// Recovery procedure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryProcedure {
    /// Restart failed service
    RestartService,
    /// Restore from checkpoint
    RestoreCheckpoint,
    /// Reclaim memory
    ReclaimMemory,
    /// Rebalance load
    RebalanceLoad,
    /// Reduce load
    ReduceLoad,
    /// Full system recovery
    FullRecovery,
}

/// Recovery state
#[derive(Debug, Clone)]
pub struct RecoveryState {
    /// Currently recovering
    pub in_recovery: bool,
    /// Recovery start time
    pub start_time: u64,
    /// Current procedure
    pub procedure: Option<RecoveryProcedure>,
    /// Services restored
    pub services_restored: u32,
    /// Services pending restoration
    pub services_pending: u32,
}

// ============================================================================
// EMERGENCY MANAGER
// ============================================================================

/// Emergency response manager
pub struct EmergencyManager {
    /// Current emergency level
    pub current_level: EmergencyLevel,
    /// Active events
    events: Vec<EmergencyEvent>,
    /// Registered services
    services: BTreeMap<u64, ServiceRecord>,
    /// Watchdog entries
    watchdogs: BTreeMap<u64, WatchdogEntry>,
    /// Recovery state
    pub recovery: RecoveryState,
    /// Event counter
    next_event_id: u64,
    /// Total emergencies
    pub total_emergencies: u64,
    /// Max event history
    max_events: usize,
    /// Degradation order (priority-sorted PIDs)
    degradation_order: Vec<u64>,
}

impl EmergencyManager {
    pub fn new() -> Self {
        Self {
            current_level: EmergencyLevel::Normal,
            events: Vec::new(),
            services: BTreeMap::new(),
            watchdogs: BTreeMap::new(),
            recovery: RecoveryState {
                in_recovery: false,
                start_time: 0,
                procedure: None,
                services_restored: 0,
                services_pending: 0,
            },
            next_event_id: 0,
            total_emergencies: 0,
            max_events: 500,
            degradation_order: Vec::new(),
        }
    }

    /// Register service
    pub fn register_service(&mut self, pid: u64, priority: ServicePriority) {
        self.services.insert(
            pid,
            ServiceRecord {
                pid,
                priority,
                state: ServiceState::Running,
                memory_usage: 0,
                cpu_usage: 0,
                is_essential: matches!(
                    priority,
                    ServicePriority::Kernel | ServicePriority::Essential
                ),
                degradation_level: 0,
            },
        );
        self.rebuild_degradation_order();
    }

    /// Remove service
    pub fn unregister_service(&mut self, pid: u64) {
        self.services.remove(&pid);
        self.rebuild_degradation_order();
    }

    /// Update service stats
    pub fn update_service(&mut self, pid: u64, memory: u64, cpu: u32) {
        if let Some(svc) = self.services.get_mut(&pid) {
            svc.memory_usage = memory;
            svc.cpu_usage = cpu;
        }
    }

    /// Register watchdog
    pub fn register_watchdog(&mut self, id: u64, interval_ms: u64, max_missed: u32) {
        self.watchdogs
            .insert(id, WatchdogEntry::new(id, interval_ms, max_missed));
    }

    /// Watchdog heartbeat
    pub fn watchdog_heartbeat(&mut self, id: u64, timestamp: u64) {
        if let Some(wd) = self.watchdogs.get_mut(&id) {
            wd.heartbeat(timestamp);
        }
    }

    /// Check all watchdogs
    pub fn check_watchdogs(&mut self, current_time: u64) -> Vec<u64> {
        let mut timed_out = Vec::new();
        for wd in self.watchdogs.values_mut() {
            if !wd.check(current_time) {
                timed_out.push(wd.id);
            }
        }
        timed_out
    }

    /// Trigger emergency
    pub fn trigger(&mut self, trigger: EmergencyTrigger, timestamp: u64) -> u64 {
        let level = trigger.default_level();

        // Escalate if already in emergency
        let effective_level = if level > self.current_level {
            level
        } else {
            self.current_level
        };

        self.current_level = effective_level;
        self.total_emergencies += 1;

        let event_id = self.next_event_id;
        self.next_event_id += 1;

        self.events.push(EmergencyEvent {
            id: event_id,
            trigger,
            level: effective_level,
            timestamp,
            resolved: false,
            resolved_at: None,
            actions_taken: Vec::new(),
        });

        if self.events.len() > self.max_events {
            self.events.remove(0);
        }

        event_id
    }

    /// Execute degradation based on current level
    pub fn execute_degradation(&mut self, timestamp: u64) -> Vec<EmergencyAction> {
        let mut actions = Vec::new();
        let level = self.current_level;

        if level == EmergencyLevel::Normal {
            return actions;
        }

        // Walk degradation order (lowest priority first)
        for &pid in &self.degradation_order.clone() {
            if let Some(svc) = self.services.get_mut(&pid) {
                if svc.is_essential {
                    continue;
                }

                let action = match level {
                    EmergencyLevel::Elevated => continue,
                    EmergencyLevel::Warning => {
                        if svc.priority >= ServicePriority::Optional {
                            svc.state = ServiceState::Degraded;
                            svc.degradation_level = 1;
                            EmergencyActionType::SuspendProcess
                        } else {
                            continue;
                        }
                    }
                    EmergencyLevel::Critical => {
                        if svc.priority >= ServicePriority::Standard {
                            svc.state = ServiceState::Suspended;
                            svc.degradation_level = 2;
                            EmergencyActionType::SuspendProcess
                        } else {
                            continue;
                        }
                    }
                    EmergencyLevel::Severe => {
                        if svc.priority >= ServicePriority::Important {
                            svc.state = ServiceState::Killed;
                            svc.degradation_level = 3;
                            EmergencyActionType::KillProcess
                        } else {
                            continue;
                        }
                    }
                    EmergencyLevel::Panic => {
                        if !svc.is_essential {
                            svc.state = ServiceState::Killed;
                            svc.degradation_level = 4;
                            EmergencyActionType::KillProcess
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                };

                actions.push(EmergencyAction {
                    action,
                    target: pid,
                    timestamp,
                    success: true,
                });
            }
        }

        actions
    }

    /// Resolve emergency event
    pub fn resolve(&mut self, event_id: u64, timestamp: u64) {
        if let Some(event) = self.events.iter_mut().find(|e| e.id == event_id) {
            event.resolved = true;
            event.resolved_at = Some(timestamp);
        }

        // Check if all events resolved
        let all_resolved = self.events.iter().all(|e| e.resolved);
        if all_resolved {
            self.current_level = EmergencyLevel::Normal;
        } else {
            // Set level to highest unresolved
            self.current_level = self
                .events
                .iter()
                .filter(|e| !e.resolved)
                .map(|e| e.level)
                .max()
                .unwrap_or(EmergencyLevel::Normal);
        }
    }

    /// Begin recovery
    pub fn begin_recovery(&mut self, procedure: RecoveryProcedure, timestamp: u64) {
        self.recovery.in_recovery = true;
        self.recovery.start_time = timestamp;
        self.recovery.procedure = Some(procedure);
        self.recovery.services_restored = 0;
        self.recovery.services_pending = self
            .services
            .values()
            .filter(|s| s.state != ServiceState::Running)
            .count() as u32;
    }

    /// Restore a suspended/degraded service
    pub fn restore_service(&mut self, pid: u64) -> bool {
        if let Some(svc) = self.services.get_mut(&pid) {
            if svc.state != ServiceState::Running && svc.state != ServiceState::Killed {
                svc.state = ServiceState::Running;
                svc.degradation_level = 0;
                self.recovery.services_restored += 1;
                if self.recovery.services_pending > 0 {
                    self.recovery.services_pending -= 1;
                }
                return true;
            }
        }
        false
    }

    /// End recovery
    pub fn end_recovery(&mut self) {
        self.recovery.in_recovery = false;
        self.recovery.procedure = None;
    }

    /// Rebuild degradation order (lowest priority first)
    fn rebuild_degradation_order(&mut self) {
        let mut order: Vec<(u64, ServicePriority)> = self
            .services
            .values()
            .map(|s| (s.pid, s.priority))
            .collect();
        order.sort_by(|a, b| b.1.cmp(&a.1)); // Higher priority value = lower importance
        self.degradation_order = order.into_iter().map(|(pid, _)| pid).collect();
    }

    /// Active events
    pub fn active_events(&self) -> Vec<&EmergencyEvent> {
        self.events.iter().filter(|e| !e.resolved).collect()
    }

    /// Service count by state
    pub fn service_counts(&self) -> BTreeMap<u8, u32> {
        let mut counts = BTreeMap::new();
        for svc in self.services.values() {
            *counts.entry(svc.state as u8).or_insert(0) += 1;
        }
        counts
    }

    /// Total registered services
    pub fn total_services(&self) -> usize {
        self.services.len()
    }
}
