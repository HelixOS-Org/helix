//! Network Intelligence Module
//!
//! This module provides AI-powered network subsystem analysis including interface
//! management, packet statistics, driver monitoring, and intelligent traffic analysis.

// Submodules
mod driver;
mod intelligence;
mod interface;
mod manager;
mod queue;
mod stats;
mod types;

// Re-export core types
// Re-export driver features
pub use driver::{DriverFeature, DriverFeatures};
// Re-export intelligence
pub use intelligence::{
    NetworkAction, NetworkAnalysis, NetworkIntelligence, NetworkIssue, NetworkIssueType,
    NetworkRecommendation,
};
// Re-export interface types
pub use interface::{InterfaceState, InterfaceType, LinkState, NetworkInterface};
// Re-export manager
pub use manager::NetworkManager;
// Re-export queue and traffic
pub use queue::{QdiscType, TrafficAnalysis, TrafficPattern};
// Re-export statistics
pub use stats::{InterfaceStats, QueueStats, RingStats};
pub use types::{Duplex, IfIndex, Ipv4Address, Ipv6Address, LinkSpeed, MacAddress};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mac_address() {
        let mac = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert!(!mac.is_broadcast());
        assert!(!mac.is_multicast());
    }

    #[test]
    fn test_ipv4_address() {
        let ip = Ipv4Address::new(192, 168, 1, 1);
        assert!(ip.is_private());

        let loopback = Ipv4Address::new(127, 0, 0, 1);
        assert!(loopback.is_loopback());
    }

    #[test]
    fn test_link_speed() {
        let speed = LinkSpeed::SPEED_1000;
        assert_eq!(speed.bytes_per_sec(), 125_000_000);
    }

    #[test]
    fn test_interface_stats() {
        let mut stats = InterfaceStats::new();
        stats.rx_packets = 1000;
        stats.tx_packets = 500;
        stats.rx_errors = 5;
        stats.tx_errors = 2;

        assert_eq!(stats.total_packets(), 1500);
        assert!(stats.error_rate() > 0.004);
    }

    #[test]
    fn test_network_interface() {
        let mut iface = NetworkInterface::new(
            IfIndex::new(1),
            String::from("eth0"),
            InterfaceType::Ethernet,
        );

        iface.state = InterfaceState::Up;
        iface.link_state = LinkState::Up;

        assert!(iface.is_running());
    }

    #[test]
    fn test_network_intelligence() {
        let mut intel = NetworkIntelligence::new();

        let mut iface = NetworkInterface::new(
            IfIndex::new(1),
            String::from("eth0"),
            InterfaceType::Ethernet,
        );
        iface.state = InterfaceState::Up;
        iface.link_state = LinkState::Down; // No link

        intel.register_interface(iface);

        let analysis = intel.analyze();
        // Should detect no link issue
        assert!(
            analysis
                .issues
                .iter()
                .any(|i| matches!(i.issue_type, NetworkIssueType::NoLink))
        );
    }
}
