//! # Security Anomaly Detection
//!
//! AI-powered security monitoring and threat detection for kernel operations.
//!
//! ## Key Features
//!
//! - **Behavioral Analysis**: Detect anomalous behavior patterns
//! - **Syscall Monitoring**: Monitor and analyze syscall patterns
//! - **Privilege Escalation Detection**: Detect unauthorized privilege changes
//! - **Memory Corruption Detection**: Detect buffer overflows and corruption
//! - **Network Anomaly Detection**: Detect suspicious network activity
//! - **Intrusion Detection**: Multi-layer intrusion detection system

mod behavioral;
mod ids;
mod memory;
mod network;
mod syscall;
mod types;

// Re-export types
// Re-export behavioral
pub use behavioral::{
    BehavioralProfile, CurrentBehavior, FileBaseline, MemoryBaseline, NetworkBaseline,
};
// Re-export IDS
pub use ids::{DetectionMode, IDSStats, IntrusionDetectionSystem};
// Re-export memory
pub use memory::{
    MemoryProtectionFlags, MemorySecurityMonitor, MemoryViolation, MemoryViolationType,
    ProtectedRegion,
};
// Re-export network
pub use network::{NetworkSecurityMonitor, NetworkThresholds};
// Re-export syscall
pub use syscall::{SyscallMonitor, SyscallMonitorStats, SyscallPattern};
pub use types::{Threat, ThreatSeverity, ThreatType};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_creation() {
        let threat = Threat::new(ThreatType::BufferOverflow, 123)
            .with_severity(ThreatSeverity::Critical)
            .with_description("Test threat");

        assert_eq!(threat.threat_type, ThreatType::BufferOverflow);
        assert_eq!(threat.severity, ThreatSeverity::Critical);
        assert_eq!(threat.source_id, 123);
    }

    #[test]
    fn test_behavioral_profile() {
        let mut profile = BehavioralProfile::new(1);

        // Train profile
        for _ in 0..100 {
            profile.update_syscall(1, 50, 100);
            profile.update_syscall(2, 30, 100);
        }

        // Normal behavior - should not be anomalous
        assert!(!profile.is_syscall_anomalous(1, 0.55));

        // Anomalous behavior
        assert!(profile.is_syscall_anomalous(1, 2.0));
    }

    #[test]
    fn test_syscall_monitor() {
        let mut monitor = SyscallMonitor::new();

        // Record some syscalls
        for i in 0..100 {
            monitor.record(1, 1, i * 1000);
        }

        assert!(monitor.get_rate(1) > 0.0);
    }

    #[test]
    fn test_memory_security() {
        let mut monitor = MemorySecurityMonitor::new();

        // Add protected region
        monitor.add_protected_region(ProtectedRegion {
            start: 0x1000,
            end: 0x2000,
            flags: MemoryProtectionFlags {
                read: true,
                write: false,
                execute: false,
                kernel: true,
            },
            description: "Kernel code".into(),
        });

        // Check write violation
        let violation = monitor.check_access(0x1500, true, false, 1, false);
        assert!(violation.is_some());
    }

    #[test]
    fn test_ids() {
        let mut ids = IntrusionDetectionSystem::new();

        // Record syscalls
        for i in 0..10 {
            ids.record_syscall(1, i);
        }

        // Should have statistics
        assert!(ids.syscall_monitor().stats().total_syscalls > 0);
    }
}
