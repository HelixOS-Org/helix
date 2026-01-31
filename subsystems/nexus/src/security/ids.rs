//! Intrusion Detection System (IDS).

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::vec::Vec;

use super::behavioral::{BehavioralProfile, CurrentBehavior};
use super::memory::{MemorySecurityMonitor, MemoryViolationType};
use super::network::NetworkSecurityMonitor;
use super::syscall::SyscallMonitor;
use super::types::{Threat, ThreatSeverity, ThreatType};
use crate::core::NexusTimestamp;

// ============================================================================
// INTRUSION DETECTION SYSTEM
// ============================================================================

/// Multi-layer intrusion detection system
pub struct IntrusionDetectionSystem {
    /// Behavioral profiles
    profiles: BTreeMap<u64, BehavioralProfile>,
    /// Syscall monitor
    syscall_monitor: SyscallMonitor,
    /// Memory monitor
    memory_monitor: MemorySecurityMonitor,
    /// Network monitor
    network_monitor: NetworkSecurityMonitor,
    /// Active threats
    active_threats: Vec<Threat>,
    /// Threat history
    threat_history: Vec<Threat>,
    /// Max history size
    max_history: usize,
    /// Detection mode
    mode: DetectionMode,
    /// Statistics
    stats: IDSStats,
}

/// IDS detection mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMode {
    /// Passive - log only
    Passive,
    /// Active - log and alert
    Active,
    /// Aggressive - log, alert, and block
    Aggressive,
}

/// IDS statistics
#[derive(Debug, Clone, Default)]
pub struct IDSStats {
    /// Total threats detected
    pub threats_detected: u64,
    /// Threats blocked
    pub threats_blocked: u64,
    /// False positives (manually marked)
    pub false_positives: u64,
    /// Processes monitored
    pub processes_monitored: u64,
}

impl IntrusionDetectionSystem {
    /// Create new IDS
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            syscall_monitor: SyscallMonitor::new(),
            memory_monitor: MemorySecurityMonitor::new(),
            network_monitor: NetworkSecurityMonitor::new(),
            active_threats: Vec::new(),
            threat_history: Vec::new(),
            max_history: 10000,
            mode: DetectionMode::Active,
            stats: IDSStats::default(),
        }
    }

    /// Get or create behavioral profile
    pub fn get_profile(&mut self, process_id: u64) -> &mut BehavioralProfile {
        self.profiles
            .entry(process_id)
            .or_insert_with(|| BehavioralProfile::new(process_id))
    }

    /// Record syscall
    pub fn record_syscall(&mut self, process_id: u64, syscall_num: u32) -> Option<Threat> {
        let now = NexusTimestamp::now().raw();
        self.syscall_monitor.record(process_id, syscall_num, now);

        // Check for patterns
        let matches = self.syscall_monitor.check_patterns(process_id);
        if let Some(pattern) = matches.first() {
            let threat = Threat::new(pattern.threat_type, process_id)
                .with_severity(pattern.severity)
                .with_description(format!("Syscall pattern matched: {}", pattern.name));

            self.report_threat(threat.clone());
            return Some(threat);
        }

        None
    }

    /// Record memory access
    pub fn record_memory_access(
        &mut self,
        address: u64,
        is_write: bool,
        is_execute: bool,
        source_id: u64,
        is_kernel: bool,
    ) -> Option<Threat> {
        if let Some(violation) = self
            .memory_monitor
            .check_access(address, is_write, is_execute, source_id, is_kernel)
        {
            self.memory_monitor.record_violation(violation.clone());

            let threat = Threat::new(
                match violation.violation_type {
                    MemoryViolationType::ExecuteViolation => ThreatType::CodeInjection,
                    MemoryViolationType::KernelAccessViolation => {
                        ThreatType::UnauthorizedMemoryAccess
                    },
                    MemoryViolationType::StackOverflow => ThreatType::BufferOverflow,
                    MemoryViolationType::HeapCorruption => ThreatType::BufferOverflow,
                    _ => ThreatType::UnauthorizedMemoryAccess,
                },
                source_id,
            )
            .with_severity(violation.severity);

            self.report_threat(threat.clone());
            return Some(threat);
        }

        None
    }

    /// Check behavioral anomaly
    pub fn check_behavioral_anomaly(
        &mut self,
        process_id: u64,
        current: &CurrentBehavior,
    ) -> Option<Threat> {
        let profile = self.profiles.get(&process_id)?;
        let score = profile.anomaly_score(current);

        if score > 0.7 {
            let threat = Threat::new(ThreatType::Unknown, process_id)
                .with_severity(ThreatSeverity::Medium)
                .with_description(format!("Behavioral anomaly detected (score: {:.2})", score));

            self.report_threat(threat.clone());
            return Some(threat);
        }

        None
    }

    /// Report threat
    fn report_threat(&mut self, threat: Threat) {
        self.stats.threats_detected += 1;

        if self.mode == DetectionMode::Aggressive && threat.severity.should_block() {
            self.stats.threats_blocked += 1;
        }

        self.active_threats.push(threat.clone());
        self.threat_history.push(threat);

        // Evict old history
        if self.threat_history.len() > self.max_history {
            self.threat_history.remove(0);
        }
    }

    /// Resolve threat
    pub fn resolve_threat(&mut self, threat_id: u64) {
        if let Some(pos) = self.active_threats.iter().position(|t| t.id == threat_id) {
            let mut threat = self.active_threats.remove(pos);
            threat.mitigate();
        }
    }

    /// Mark false positive
    pub fn mark_false_positive(&mut self, threat_id: u64) {
        self.resolve_threat(threat_id);
        self.stats.false_positives += 1;
    }

    /// Get active threats
    pub fn active_threats(&self) -> &[Threat] {
        &self.active_threats
    }

    /// Get active critical threats
    pub fn critical_threats(&self) -> Vec<&Threat> {
        self.active_threats
            .iter()
            .filter(|t| t.severity >= ThreatSeverity::High)
            .collect()
    }

    /// Set detection mode
    pub fn set_mode(&mut self, mode: DetectionMode) {
        self.mode = mode;
    }

    /// Get statistics
    pub fn stats(&self) -> &IDSStats {
        &self.stats
    }

    /// Get syscall monitor
    pub fn syscall_monitor(&self) -> &SyscallMonitor {
        &self.syscall_monitor
    }

    /// Get memory monitor
    pub fn memory_monitor(&self) -> &MemorySecurityMonitor {
        &self.memory_monitor
    }

    /// Get mutable memory monitor
    pub fn memory_monitor_mut(&mut self) -> &mut MemorySecurityMonitor {
        &mut self.memory_monitor
    }

    /// Get network monitor
    pub fn network_monitor(&self) -> &NetworkSecurityMonitor {
        &self.network_monitor
    }

    /// Get mutable network monitor
    pub fn network_monitor_mut(&mut self) -> &mut NetworkSecurityMonitor {
        &mut self.network_monitor
    }
}

impl Default for IntrusionDetectionSystem {
    fn default() -> Self {
        Self::new()
    }
}
