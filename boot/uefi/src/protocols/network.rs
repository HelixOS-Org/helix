//! Network Protocol
//!
//! High-level network abstraction for network access.

use super::Protocol;
use crate::error::{Error, Result};
use crate::raw::types::*;

extern crate alloc;
use alloc::vec::Vec;

// =============================================================================
// NETWORK INTERFACE
// =============================================================================

/// High-level network interface abstraction
pub struct NetworkInterface {
    /// Handle
    _handle: Handle,
    /// MAC address
    mac_address: MacAddress,
    /// Interface state
    state: InterfaceState,
    /// MTU
    mtu: u32,
}

impl NetworkInterface {
    /// Create new network interface
    pub fn new(handle: Handle) -> Self {
        Self {
            _handle: handle,
            mac_address: MacAddress::ZERO,
            state: InterfaceState::Stopped,
            mtu: 1500,
        }
    }

    /// Get MAC address
    pub fn mac_address(&self) -> &MacAddress {
        &self.mac_address
    }

    /// Get MTU
    pub fn mtu(&self) -> u32 {
        self.mtu
    }

    /// Get interface state
    pub fn state(&self) -> InterfaceState {
        self.state
    }

    /// Check if interface is up
    pub fn is_up(&self) -> bool {
        self.state == InterfaceState::Started
    }

    /// Start interface
    pub fn start(&mut self) -> Result<()> {
        self.state = InterfaceState::Started;
        Ok(())
    }

    /// Stop interface
    pub fn stop(&mut self) -> Result<()> {
        self.state = InterfaceState::Stopped;
        Ok(())
    }

    /// Configure interface
    pub fn configure(&mut self, _config: &NetworkConfig) -> Result<()> {
        // TODO: Implement actual configuration
        Ok(())
    }

    /// Send raw packet
    pub fn send(&self, _destination: &MacAddress, _protocol: u16, _data: &[u8]) -> Result<()> {
        if self.state != InterfaceState::Started {
            return Err(Error::NotReady);
        }

        // TODO: Implement actual sending
        Ok(())
    }

    /// Receive raw packet
    pub fn receive(&self) -> Result<Option<(MacAddress, u16, Vec<u8>)>> {
        if self.state != InterfaceState::Started {
            return Err(Error::NotReady);
        }

        // TODO: Implement actual receiving
        Ok(None)
    }

    /// Get statistics
    pub fn statistics(&self) -> NetworkStatistics {
        NetworkStatistics::default()
    }
}

impl Protocol for NetworkInterface {
    const GUID: Guid = Guid::new(0xE185_41CD, 0xF755, 0x4F73, [
        0x92, 0x8D, 0x64, 0x3C, 0x8A, 0x79, 0xB2, 0x29,
    ]);

    fn open(handle: Handle) -> Result<Self> {
        Ok(Self::new(handle))
    }
}

// =============================================================================
// INTERFACE STATE
// =============================================================================

/// Network interface state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    /// Interface is stopped
    Stopped,
    /// Interface is started
    Started,
    /// Interface is in initialization
    Initializing,
}

// =============================================================================
// NETWORK CONFIG
// =============================================================================

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// IP address
    pub ip_address: Option<IpAddress>,
    /// Subnet mask
    pub subnet_mask: Option<IpAddress>,
    /// Gateway
    pub gateway: Option<IpAddress>,
    /// DNS servers
    pub dns_servers: Vec<IpAddress>,
    /// Use DHCP
    pub use_dhcp: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            ip_address: None,
            subnet_mask: None,
            gateway: None,
            dns_servers: Vec::new(),
            use_dhcp: true,
        }
    }
}

impl NetworkConfig {
    /// Create static configuration
    pub fn static_config(ip: IpAddress, mask: IpAddress, gateway: IpAddress) -> Self {
        Self {
            ip_address: Some(ip),
            subnet_mask: Some(mask),
            gateway: Some(gateway),
            dns_servers: Vec::new(),
            use_dhcp: false,
        }
    }

    /// Create DHCP configuration
    pub fn dhcp() -> Self {
        Self::default()
    }
}

// =============================================================================
// NETWORK STATISTICS
// =============================================================================

/// Network statistics
#[derive(Debug, Clone, Default)]
pub struct NetworkStatistics {
    /// Packets received
    pub rx_packets: u64,
    /// Packets transmitted
    pub tx_packets: u64,
    /// Bytes received
    pub rx_bytes: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
    /// Receive errors
    pub rx_errors: u64,
    /// Transmit errors
    pub tx_errors: u64,
    /// Dropped packets
    pub rx_dropped: u64,
    /// Collision count
    pub collisions: u64,
}

// =============================================================================
// MAC ADDRESS
// =============================================================================

/// MAC address (48-bit Ethernet address)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    /// Zero address
    pub const ZERO: Self = Self([0, 0, 0, 0, 0, 0]);

    /// Broadcast address
    pub const BROADCAST: Self = Self([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

    /// Create from bytes
    pub const fn new(
        octet1: u8,
        octet2: u8,
        octet3: u8,
        octet4: u8,
        octet5: u8,
        octet6: u8,
    ) -> Self {
        Self([octet1, octet2, octet3, octet4, octet5, octet6])
    }

    /// Create from array
    pub const fn from_bytes(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Get as bytes
    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    /// Check if broadcast
    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }

    /// Check if multicast
    pub fn is_multicast(&self) -> bool {
        (self.0[0] & 0x01) != 0
    }

    /// Check if unicast
    pub fn is_unicast(&self) -> bool {
        !self.is_multicast()
    }

    /// Check if locally administered
    pub fn is_local(&self) -> bool {
        (self.0[0] & 0x02) != 0
    }

    /// Check if universally administered
    pub fn is_universal(&self) -> bool {
        !self.is_local()
    }

    /// Parse from string (XX:XX:XX:XX:XX:XX)
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return None;
        }

        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16).ok()?;
        }

        Some(Self(bytes))
    }
}

impl core::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

// =============================================================================
// IP ADDRESS
// =============================================================================

/// IP address (v4 or v6)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpAddress {
    /// IPv4 address
    V4(Ipv4Address),
    /// IPv6 address
    V6(Ipv6Address),
}

impl IpAddress {
    /// Create IPv4 address
    pub const fn v4(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self::V4(Ipv4Address::new(a, b, c, d))
    }

    /// Create IPv6 address
    pub const fn v6(bytes: [u8; 16]) -> Self {
        Self::V6(Ipv6Address(bytes))
    }

    /// Check if IPv4
    pub fn is_v4(&self) -> bool {
        matches!(self, Self::V4(_))
    }

    /// Check if IPv6
    pub fn is_v6(&self) -> bool {
        matches!(self, Self::V6(_))
    }

    /// Check if loopback
    pub fn is_loopback(&self) -> bool {
        match self {
            Self::V4(v4) => v4.is_loopback(),
            Self::V6(v6) => v6.is_loopback(),
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        // Try IPv4 first
        if let Some(v4) = Ipv4Address::parse(s) {
            return Some(Self::V4(v4));
        }

        // Try IPv6
        if let Some(v6) = Ipv6Address::parse(s) {
            return Some(Self::V6(v6));
        }

        None
    }
}

impl core::fmt::Display for IpAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::V4(v4) => write!(f, "{}", v4),
            Self::V6(v6) => write!(f, "{}", v6),
        }
    }
}

// =============================================================================
// IPV4 ADDRESS
// =============================================================================

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    /// Any address (0.0.0.0)
    pub const ANY: Self = Self([0, 0, 0, 0]);

    /// Localhost (127.0.0.1)
    pub const LOCALHOST: Self = Self([127, 0, 0, 1]);

    /// Broadcast (255.255.255.255)
    pub const BROADCAST: Self = Self([255, 255, 255, 255]);

    /// Create from octets
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// Create from bytes
    pub const fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Create from u32 (network byte order)
    pub const fn from_u32(value: u32) -> Self {
        Self([
            ((value >> 24) & 0xFF) as u8,
            ((value >> 16) & 0xFF) as u8,
            ((value >> 8) & 0xFF) as u8,
            (value & 0xFF) as u8,
        ])
    }

    /// Convert to u32 (network byte order)
    pub const fn to_u32(&self) -> u32 {
        ((self.0[0] as u32) << 24)
            | ((self.0[1] as u32) << 16)
            | ((self.0[2] as u32) << 8)
            | (self.0[3] as u32)
    }

    /// Get octets
    pub fn octets(&self) -> [u8; 4] {
        self.0
    }

    /// Check if loopback
    pub fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }

    /// Check if private
    pub fn is_private(&self) -> bool {
        // 10.0.0.0/8
        self.0[0] == 10 ||
        // 172.16.0.0/12
        (self.0[0] == 172 && (self.0[1] >= 16 && self.0[1] <= 31)) ||
        // 192.168.0.0/16
        (self.0[0] == 192 && self.0[1] == 168)
    }

    /// Check if link-local
    pub fn is_link_local(&self) -> bool {
        self.0[0] == 169 && self.0[1] == 254
    }

    /// Check if broadcast
    pub fn is_broadcast(&self) -> bool {
        *self == Self::BROADCAST
    }

    /// Check if multicast
    pub fn is_multicast(&self) -> bool {
        self.0[0] >= 224 && self.0[0] <= 239
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return None;
        }

        let mut bytes = [0u8; 4];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = part.parse().ok()?;
        }

        Some(Self(bytes))
    }
}

impl core::fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

// =============================================================================
// IPV6 ADDRESS
// =============================================================================

/// IPv6 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ipv6Address(pub [u8; 16]);

impl Ipv6Address {
    /// Unspecified address (::)
    pub const UNSPECIFIED: Self = Self([0; 16]);

    /// Loopback address (::1)
    pub const LOOPBACK: Self = Self([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);

    /// Create from bytes
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Create from segments
    pub fn from_segments(segments: [u16; 8]) -> Self {
        let mut bytes = [0u8; 16];
        for (i, seg) in segments.iter().enumerate() {
            bytes[i * 2] = (seg >> 8) as u8;
            bytes[i * 2 + 1] = (seg & 0xFF) as u8;
        }
        Self(bytes)
    }

    /// Get segments
    pub fn segments(&self) -> [u16; 8] {
        let mut segs = [0u16; 8];
        for (i, seg) in segs.iter_mut().enumerate() {
            *seg = ((self.0[i * 2] as u16) << 8) | (self.0[i * 2 + 1] as u16);
        }
        segs
    }

    /// Check if loopback
    pub fn is_loopback(&self) -> bool {
        *self == Self::LOOPBACK
    }

    /// Check if unspecified
    pub fn is_unspecified(&self) -> bool {
        *self == Self::UNSPECIFIED
    }

    /// Check if link-local
    pub fn is_link_local(&self) -> bool {
        self.0[0] == 0xFE && (self.0[1] & 0xC0) == 0x80
    }

    /// Check if multicast
    pub fn is_multicast(&self) -> bool {
        self.0[0] == 0xFF
    }

    /// Parse from string (simplified, doesn't handle :: compression)
    pub fn parse(s: &str) -> Option<Self> {
        // Handle ::1 style
        if s == "::1" {
            return Some(Self::LOOPBACK);
        }
        if s == "::" {
            return Some(Self::UNSPECIFIED);
        }

        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 8 {
            return None;
        }

        let mut segments = [0u16; 8];
        for (i, part) in parts.iter().enumerate() {
            segments[i] = u16::from_str_radix(part, 16).ok()?;
        }

        Some(Self::from_segments(segments))
    }
}

impl core::fmt::Display for Ipv6Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let segs = self.segments();
        write!(
            f,
            "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
            segs[0], segs[1], segs[2], segs[3], segs[4], segs[5], segs[6], segs[7]
        )
    }
}

// =============================================================================
// ETHERNET TYPES
// =============================================================================

/// Common Ethernet type values
pub mod ether_types {
    /// IPv4
    pub const IPV4: u16 = 0x0800;
    /// ARP
    pub const ARP: u16 = 0x0806;
    /// RARP
    pub const RARP: u16 = 0x8035;
    /// IPv6
    pub const IPV6: u16 = 0x86DD;
    /// VLAN tagged
    pub const VLAN: u16 = 0x8100;
    /// LLDP
    pub const LLDP: u16 = 0x88CC;
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_mac_address() {
        let mac = MacAddress::new(0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF);
        assert_eq!(mac.to_string(), "AA:BB:CC:DD:EE:FF");
        assert!(mac.is_local());
        assert!(!mac.is_multicast());
    }

    #[test]
    fn test_mac_parse() {
        let mac = MacAddress::parse("AA:BB:CC:DD:EE:FF").unwrap();
        assert_eq!(mac.0, [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF]);
    }

    #[test]
    fn test_ipv4_address() {
        let ip = Ipv4Address::new(192, 168, 1, 1);
        assert!(ip.is_private());
        assert!(!ip.is_loopback());
    }

    #[test]
    fn test_ipv4_parse() {
        let ip = Ipv4Address::parse("192.168.1.1").unwrap();
        assert_eq!(ip.0, [192, 168, 1, 1]);
    }

    #[test]
    fn test_ipv6_loopback() {
        assert!(Ipv6Address::LOOPBACK.is_loopback());
        assert!(!Ipv6Address::LOOPBACK.is_unspecified());
    }
}
