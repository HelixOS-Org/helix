//! Capabilities Intelligence Module
//!
//! AI-powered Linux capabilities analysis and privilege management.
//!
//! This module provides:
//! - Capability set management
//! - Privilege escalation detection
//! - Least-privilege recommendations
//! - Intelligent security policy enforcement

mod analysis;
mod file;
mod intelligence;
mod process;
mod sets;
mod tracker;
mod types;
mod usage;

pub use analysis::{
    CapAction, CapIssueType, CapabilityAnalysis, CapabilityIssue, CapabilityRecommendation,
};
pub use file::FileCaps;
pub use intelligence::CapabilitiesIntelligence;
pub use process::ProcessCaps;
pub use sets::{CapSetType, CapabilitySet};
pub use tracker::{CapEventType, CapabilityEvent, CapabilityTracker};
pub use types::{Capability, CapabilityCategory, Pid, RiskLevel, Uid};
pub use usage::{LeastPrivilegeAnalyzer, LeastPrivilegeRec, PrivilegeUsage};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_set() {
        let mut set = CapabilitySet::new();
        assert!(set.is_empty());

        set.set(Capability::NetBindService);
        assert!(set.has(Capability::NetBindService));
        assert!(!set.has(Capability::SysAdmin));

        set.clear(Capability::NetBindService);
        assert!(!set.has(Capability::NetBindService));
    }

    #[test]
    fn test_capability_set_operations() {
        let mut set1 = CapabilitySet::new();
        set1.set(Capability::NetBindService);
        set1.set(Capability::NetRaw);

        let mut set2 = CapabilitySet::new();
        set2.set(Capability::NetRaw);
        set2.set(Capability::SysAdmin);

        let union = set1.union(&set2);
        assert!(union.has(Capability::NetBindService));
        assert!(union.has(Capability::NetRaw));
        assert!(union.has(Capability::SysAdmin));

        let inter = set1.intersection(&set2);
        assert!(!inter.has(Capability::NetBindService));
        assert!(inter.has(Capability::NetRaw));
        assert!(!inter.has(Capability::SysAdmin));
    }

    #[test]
    fn test_process_caps() {
        let mut caps = ProcessCaps::new(Pid::new(1), Uid::new(1000));

        caps.permitted.set(Capability::NetBindService);
        assert!(caps.raise_cap(Capability::NetBindService));
        assert!(caps.has_capability(Capability::NetBindService));

        assert!(!caps.raise_cap(Capability::SysAdmin));
    }

    #[test]
    fn test_risk_levels() {
        assert_eq!(Capability::SysAdmin.risk_level(), RiskLevel::Critical);
        assert_eq!(Capability::SysModule.risk_level(), RiskLevel::Critical);
        assert_eq!(Capability::Setuid.risk_level(), RiskLevel::High);
        assert_eq!(Capability::NetBindService.risk_level(), RiskLevel::Low);
    }

    #[test]
    fn test_capabilities_intelligence() {
        let mut intel = CapabilitiesIntelligence::new();

        let mut caps = ProcessCaps::new(Pid::new(1), Uid::new(1000));
        caps.effective.set(Capability::NetBindService);
        caps.permitted.set(Capability::NetBindService);

        intel.register_process(caps);

        let analysis = intel.analyze();
        assert!(analysis.security_score > 0.0);
    }
}
