//! Network Interface Types and States
//!
//! Interface type definitions and state enumerations.

use alloc::string::String;
use alloc::vec::Vec;

use super::{
    Duplex, IfIndex, InterfaceStats, Ipv4Address, Ipv6Address, LinkSpeed, MacAddress, QdiscType,
    RingStats,
};

// ============================================================================
// INTERFACE TYPES
// ============================================================================

/// Interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// Ethernet
    Ethernet,
    /// Loopback
    Loopback,
    /// WiFi
    Wifi,
    /// Bridge
    Bridge,
    /// Bond
    Bond,
    /// VLAN
    Vlan,
    /// VxLAN
    Vxlan,
    /// Tunnel
    Tunnel,
    /// Tun
    Tun,
    /// Tap
    Tap,
    /// Virtual
    Virtual,
    /// Dummy
    Dummy,
    /// Unknown
    Unknown,
}

impl InterfaceType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ethernet => "ethernet",
            Self::Loopback => "loopback",
            Self::Wifi => "wifi",
            Self::Bridge => "bridge",
            Self::Bond => "bond",
            Self::Vlan => "vlan",
            Self::Vxlan => "vxlan",
            Self::Tunnel => "tunnel",
            Self::Tun => "tun",
            Self::Tap => "tap",
            Self::Virtual => "virtual",
            Self::Dummy => "dummy",
            Self::Unknown => "unknown",
        }
    }

    /// Is physical
    pub fn is_physical(&self) -> bool {
        matches!(self, Self::Ethernet | Self::Wifi)
    }

    /// Is virtual
    pub fn is_virtual(&self) -> bool {
        matches!(
            self,
            Self::Bridge
                | Self::Bond
                | Self::Vlan
                | Self::Vxlan
                | Self::Tunnel
                | Self::Tun
                | Self::Tap
                | Self::Virtual
                | Self::Dummy
        )
    }
}

/// Interface state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    /// Up
    Up,
    /// Down
    Down,
    /// Unknown
    Unknown,
}

impl InterfaceState {
    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Unknown => "unknown",
        }
    }
}

/// Link state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkState {
    /// Link detected
    Up,
    /// No link
    Down,
    /// Testing
    Testing,
    /// Dormant
    Dormant,
    /// Unknown
    Unknown,
}

impl LinkState {
    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Testing => "testing",
            Self::Dormant => "dormant",
            Self::Unknown => "unknown",
        }
    }
}

// ============================================================================
// NETWORK INTERFACE
// ============================================================================

/// Network interface
#[derive(Debug)]
pub struct NetworkInterface {
    /// Interface index
    pub index: IfIndex,
    /// Interface name
    pub name: String,
    /// Interface type
    pub if_type: InterfaceType,
    /// MAC address
    pub mac: MacAddress,
    /// MTU
    pub mtu: u32,
    /// TX queue length
    pub txqlen: u32,
    /// Interface state (admin)
    pub state: InterfaceState,
    /// Link state (carrier)
    pub link_state: LinkState,
    /// Link speed
    pub speed: Option<LinkSpeed>,
    /// Duplex
    pub duplex: Duplex,
    /// IPv4 addresses
    pub ipv4_addrs: Vec<Ipv4Address>,
    /// IPv6 addresses
    pub ipv6_addrs: Vec<Ipv6Address>,
    /// Statistics
    pub stats: InterfaceStats,
    /// Ring stats
    pub ring_stats: RingStats,
    /// Queue discipline
    pub qdisc: QdiscType,
    /// Driver name
    pub driver: Option<String>,
    /// Firmware version
    pub firmware: Option<String>,
    /// Bus info
    pub bus_info: Option<String>,
    /// Promiscuous mode
    pub promiscuous: bool,
    /// All multicast
    pub allmulticast: bool,
    /// Number of TX queues
    pub num_tx_queues: u32,
    /// Number of RX queues
    pub num_rx_queues: u32,
    /// Master interface (for virtual)
    pub master: Option<IfIndex>,
    /// Slave interfaces
    pub slaves: Vec<IfIndex>,
}

impl NetworkInterface {
    /// Create new interface
    pub fn new(index: IfIndex, name: String, if_type: InterfaceType) -> Self {
        Self {
            index,
            name,
            if_type,
            mac: MacAddress::zero(),
            mtu: 1500,
            txqlen: 1000,
            state: InterfaceState::Down,
            link_state: LinkState::Unknown,
            speed: None,
            duplex: Duplex::Unknown,
            ipv4_addrs: Vec::new(),
            ipv6_addrs: Vec::new(),
            stats: InterfaceStats::new(),
            ring_stats: RingStats::new(),
            qdisc: QdiscType::PfifoFast,
            driver: None,
            firmware: None,
            bus_info: None,
            promiscuous: false,
            allmulticast: false,
            num_tx_queues: 1,
            num_rx_queues: 1,
            master: None,
            slaves: Vec::new(),
        }
    }

    /// Is up
    pub fn is_up(&self) -> bool {
        matches!(self.state, InterfaceState::Up)
    }

    /// Has link
    pub fn has_link(&self) -> bool {
        matches!(self.link_state, LinkState::Up)
    }

    /// Is running (up + link)
    pub fn is_running(&self) -> bool {
        self.is_up() && self.has_link()
    }

    /// Has IPv4
    pub fn has_ipv4(&self) -> bool {
        !self.ipv4_addrs.is_empty()
    }

    /// Has IPv6
    pub fn has_ipv6(&self) -> bool {
        !self.ipv6_addrs.is_empty()
    }

    /// Throughput capacity (bytes/sec)
    pub fn throughput_capacity(&self) -> u64 {
        self.speed.map(|s| s.bytes_per_sec()).unwrap_or(0)
    }

    /// Current RX utilization
    pub fn rx_utilization(&self, interval_bytes: u64, interval_secs: f64) -> f32 {
        let capacity = self.throughput_capacity();
        if capacity > 0 && interval_secs > 0.0 {
            ((interval_bytes as f64 / interval_secs) / capacity as f64) as f32
        } else {
            0.0
        }
    }
}
