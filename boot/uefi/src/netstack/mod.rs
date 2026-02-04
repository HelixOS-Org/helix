//! Network Protocol Stack for Helix UEFI Bootloader
//!
//! This module provides comprehensive network protocol support for
//! PXE boot, HTTP boot, and network-based OS installation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                      Network Protocol Stack                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                    Application Layer                             │  │
//! │  │  HTTP │ HTTPS │ TFTP │ PXE │ iSCSI │ NFS                        │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                              │                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                    Transport Layer                               │  │
//! │  │  TCP │ UDP │ ICMP │ ICMPv6                                       │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                              │                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                    Network Layer                                 │  │
//! │  │  IPv4 │ IPv6 │ ARP │ NDP │ DHCP │ DHCPv6                         │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                              │                                         │
//! │  ┌──────────────────────────────────────────────────────────────────┐  │
//! │  │                    Data Link Layer                               │  │
//! │  │  Ethernet │ VLAN │ MAC │ SNP (Simple Network Protocol)           │  │
//! │  └──────────────────────────────────────────────────────────────────┘  │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - IPv4 and IPv6 support
//! - DHCP and DHCPv6 client
//! - PXE boot support
//! - HTTP/HTTPS boot
//! - TFTP client
//! - DNS resolution

use core::fmt;

// =============================================================================
// MAC ADDRESS
// =============================================================================

/// MAC address (48-bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MacAddress {
    /// Address bytes
    pub octets: [u8; 6],
}

impl MacAddress {
    /// Broadcast address
    pub const BROADCAST: Self = Self { octets: [0xFF; 6] };

    /// Zero address
    pub const ZERO: Self = Self { octets: [0; 6] };

    /// Create from octets
    pub const fn new(octets: [u8; 6]) -> Self {
        Self { octets }
    }

    /// Check if broadcast
    pub const fn is_broadcast(&self) -> bool {
        self.octets[0] == 0xFF
            && self.octets[1] == 0xFF
            && self.octets[2] == 0xFF
            && self.octets[3] == 0xFF
            && self.octets[4] == 0xFF
            && self.octets[5] == 0xFF
    }

    /// Check if multicast
    pub const fn is_multicast(&self) -> bool {
        (self.octets[0] & 0x01) != 0
    }

    /// Check if locally administered
    pub const fn is_local(&self) -> bool {
        (self.octets[0] & 0x02) != 0
    }

    /// Check if zero
    pub const fn is_zero(&self) -> bool {
        self.octets[0] == 0
            && self.octets[1] == 0
            && self.octets[2] == 0
            && self.octets[3] == 0
            && self.octets[4] == 0
            && self.octets[5] == 0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.octets[0],
            self.octets[1],
            self.octets[2],
            self.octets[3],
            self.octets[4],
            self.octets[5]
        )
    }
}

// =============================================================================
// IPv4 ADDRESS
// =============================================================================

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ipv4Address {
    /// Address bytes
    pub octets: [u8; 4],
}

impl Ipv4Address {
    /// Any address (0.0.0.0)
    pub const ANY: Self = Self {
        octets: [0, 0, 0, 0],
    };

    /// Broadcast address (255.255.255.255)
    pub const BROADCAST: Self = Self {
        octets: [255, 255, 255, 255],
    };

    /// Localhost (127.0.0.1)
    pub const LOCALHOST: Self = Self {
        octets: [127, 0, 0, 1],
    };

    /// Create from octets
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self {
            octets: [a, b, c, d],
        }
    }

    /// Create from 32-bit value (network byte order)
    pub const fn from_u32(value: u32) -> Self {
        Self {
            octets: [
                ((value >> 24) & 0xFF) as u8,
                ((value >> 16) & 0xFF) as u8,
                ((value >> 8) & 0xFF) as u8,
                (value & 0xFF) as u8,
            ],
        }
    }

    /// Convert to 32-bit value (network byte order)
    pub const fn to_u32(&self) -> u32 {
        ((self.octets[0] as u32) << 24)
            | ((self.octets[1] as u32) << 16)
            | ((self.octets[2] as u32) << 8)
            | (self.octets[3] as u32)
    }

    /// Check if loopback
    pub const fn is_loopback(&self) -> bool {
        self.octets[0] == 127
    }

    /// Check if private
    pub const fn is_private(&self) -> bool {
        // 10.0.0.0/8
        self.octets[0] == 10 ||
        // 172.16.0.0/12
        (self.octets[0] == 172 && (self.octets[1] & 0xF0) == 16) ||
        // 192.168.0.0/16
        (self.octets[0] == 192 && self.octets[1] == 168)
    }

    /// Check if link-local
    pub const fn is_link_local(&self) -> bool {
        self.octets[0] == 169 && self.octets[1] == 254
    }

    /// Check if multicast
    pub const fn is_multicast(&self) -> bool {
        (self.octets[0] & 0xF0) == 224
    }

    /// Apply netmask
    pub const fn apply_mask(&self, mask: &Self) -> Self {
        Self {
            octets: [
                self.octets[0] & mask.octets[0],
                self.octets[1] & mask.octets[1],
                self.octets[2] & mask.octets[2],
                self.octets[3] & mask.octets[3],
            ],
        }
    }
}

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{}.{}.{}",
            self.octets[0], self.octets[1], self.octets[2], self.octets[3]
        )
    }
}

// =============================================================================
// IPv6 ADDRESS
// =============================================================================

/// IPv6 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ipv6Address {
    /// Address bytes
    pub octets: [u8; 16],
}

impl Ipv6Address {
    /// Any address (::)
    pub const ANY: Self = Self { octets: [0; 16] };

    /// Localhost (::1)
    pub const LOCALHOST: Self = Self {
        octets: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    };

    /// Create from segments
    pub const fn new(segments: [u16; 8]) -> Self {
        Self {
            octets: [
                (segments[0] >> 8) as u8,
                segments[0] as u8,
                (segments[1] >> 8) as u8,
                segments[1] as u8,
                (segments[2] >> 8) as u8,
                segments[2] as u8,
                (segments[3] >> 8) as u8,
                segments[3] as u8,
                (segments[4] >> 8) as u8,
                segments[4] as u8,
                (segments[5] >> 8) as u8,
                segments[5] as u8,
                (segments[6] >> 8) as u8,
                segments[6] as u8,
                (segments[7] >> 8) as u8,
                segments[7] as u8,
            ],
        }
    }

    /// Get segment at index
    pub const fn segment(&self, index: usize) -> u16 {
        let i = index * 2;
        ((self.octets[i] as u16) << 8) | (self.octets[i + 1] as u16)
    }

    /// Check if loopback
    pub const fn is_loopback(&self) -> bool {
        self.octets[0] == 0
            && self.octets[1] == 0
            && self.octets[2] == 0
            && self.octets[3] == 0
            && self.octets[4] == 0
            && self.octets[5] == 0
            && self.octets[6] == 0
            && self.octets[7] == 0
            && self.octets[8] == 0
            && self.octets[9] == 0
            && self.octets[10] == 0
            && self.octets[11] == 0
            && self.octets[12] == 0
            && self.octets[13] == 0
            && self.octets[14] == 0
            && self.octets[15] == 1
    }

    /// Check if link-local
    pub const fn is_link_local(&self) -> bool {
        self.octets[0] == 0xFE && (self.octets[1] & 0xC0) == 0x80
    }

    /// Check if site-local
    pub const fn is_site_local(&self) -> bool {
        self.octets[0] == 0xFE && (self.octets[1] & 0xC0) == 0xC0
    }

    /// Check if multicast
    pub const fn is_multicast(&self) -> bool {
        self.octets[0] == 0xFF
    }

    /// Check if global unicast
    pub const fn is_global(&self) -> bool {
        (self.octets[0] & 0xE0) == 0x20
    }

    /// Check if IPv4-mapped
    pub const fn is_ipv4_mapped(&self) -> bool {
        self.octets[0] == 0
            && self.octets[1] == 0
            && self.octets[2] == 0
            && self.octets[3] == 0
            && self.octets[4] == 0
            && self.octets[5] == 0
            && self.octets[6] == 0
            && self.octets[7] == 0
            && self.octets[8] == 0
            && self.octets[9] == 0
            && self.octets[10] == 0xFF
            && self.octets[11] == 0xFF
    }
}

// =============================================================================
// ETHERNET
// =============================================================================

/// Ethernet frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum EtherType {
    /// IPv4
    Ipv4 = 0x0800,
    /// ARP
    Arp  = 0x0806,
    /// VLAN tagged
    Vlan = 0x8100,
    /// IPv6
    Ipv6 = 0x86DD,
    /// LLDP
    Lldp = 0x88CC,
    /// PTP
    Ptp  = 0x88F7,
}

/// Ethernet header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct EthernetHeader {
    /// Destination MAC
    pub dst_mac: [u8; 6],
    /// Source MAC
    pub src_mac: [u8; 6],
    /// EtherType
    pub ether_type: u16,
}

impl EthernetHeader {
    /// Header size
    pub const SIZE: usize = 14;

    /// Get EtherType (convert from network byte order)
    pub const fn get_ether_type(&self) -> u16 {
        u16::from_be(self.ether_type)
    }
}

/// VLAN tag
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VlanTag {
    /// TPID (0x8100)
    pub tpid: u16,
    /// TCI (PCP + DEI + VID)
    pub tci: u16,
}

impl VlanTag {
    /// Get VLAN ID
    pub const fn vlan_id(&self) -> u16 {
        u16::from_be(self.tci) & 0x0FFF
    }

    /// Get Priority Code Point
    pub const fn pcp(&self) -> u8 {
        ((u16::from_be(self.tci) >> 13) & 0x07) as u8
    }
}

// =============================================================================
// IPv4
// =============================================================================

/// IPv4 header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct Ipv4Header {
    /// Version and IHL
    pub version_ihl: u8,
    /// DSCP and ECN
    pub dscp_ecn: u8,
    /// Total length
    pub total_length: u16,
    /// Identification
    pub identification: u16,
    /// Flags and fragment offset
    pub flags_fragment: u16,
    /// Time to live
    pub ttl: u8,
    /// Protocol
    pub protocol: u8,
    /// Header checksum
    pub checksum: u16,
    /// Source address
    pub src_addr: [u8; 4],
    /// Destination address
    pub dst_addr: [u8; 4],
}

impl Ipv4Header {
    /// Minimum header size
    pub const MIN_SIZE: usize = 20;

    /// Get version
    pub const fn version(&self) -> u8 {
        self.version_ihl >> 4
    }

    /// Get header length in bytes
    pub const fn header_length(&self) -> usize {
        ((self.version_ihl & 0x0F) as usize) * 4
    }

    /// Get total length (network byte order)
    pub const fn get_total_length(&self) -> u16 {
        u16::from_be(self.total_length)
    }

    /// Get protocol
    pub const fn get_protocol(&self) -> IpProtocol {
        IpProtocol::from_u8(self.protocol)
    }

    /// Check if Don't Fragment flag is set
    pub const fn dont_fragment(&self) -> bool {
        (u16::from_be(self.flags_fragment) & 0x4000) != 0
    }

    /// Check if More Fragments flag is set
    pub const fn more_fragments(&self) -> bool {
        (u16::from_be(self.flags_fragment) & 0x2000) != 0
    }

    /// Get fragment offset
    pub const fn fragment_offset(&self) -> u16 {
        (u16::from_be(self.flags_fragment) & 0x1FFF) * 8
    }
}

/// IP protocol numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpProtocol {
    /// ICMP
    Icmp,
    /// IGMP
    Igmp,
    /// TCP
    Tcp,
    /// UDP
    Udp,
    /// IPv6 encapsulation
    Ipv6,
    /// GRE
    Gre,
    /// ESP
    Esp,
    /// AH
    Ah,
    /// ICMPv6
    Icmpv6,
    /// SCTP
    Sctp,
    /// Unknown
    Unknown(u8),
}

impl IpProtocol {
    /// Create from u8
    pub const fn from_u8(value: u8) -> Self {
        match value {
            1 => IpProtocol::Icmp,
            2 => IpProtocol::Igmp,
            6 => IpProtocol::Tcp,
            17 => IpProtocol::Udp,
            41 => IpProtocol::Ipv6,
            47 => IpProtocol::Gre,
            50 => IpProtocol::Esp,
            51 => IpProtocol::Ah,
            58 => IpProtocol::Icmpv6,
            132 => IpProtocol::Sctp,
            n => IpProtocol::Unknown(n),
        }
    }

    /// Convert to u8
    pub const fn to_u8(&self) -> u8 {
        match self {
            IpProtocol::Icmp => 1,
            IpProtocol::Igmp => 2,
            IpProtocol::Tcp => 6,
            IpProtocol::Udp => 17,
            IpProtocol::Ipv6 => 41,
            IpProtocol::Gre => 47,
            IpProtocol::Esp => 50,
            IpProtocol::Ah => 51,
            IpProtocol::Icmpv6 => 58,
            IpProtocol::Sctp => 132,
            IpProtocol::Unknown(n) => *n,
        }
    }
}

// =============================================================================
// UDP
// =============================================================================

/// UDP header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct UdpHeader {
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Length
    pub length: u16,
    /// Checksum
    pub checksum: u16,
}

impl UdpHeader {
    /// Header size
    pub const SIZE: usize = 8;

    /// Get source port
    pub const fn get_src_port(&self) -> u16 {
        u16::from_be(self.src_port)
    }

    /// Get destination port
    pub const fn get_dst_port(&self) -> u16 {
        u16::from_be(self.dst_port)
    }

    /// Get length
    pub const fn get_length(&self) -> u16 {
        u16::from_be(self.length)
    }
}

// =============================================================================
// TCP
// =============================================================================

/// TCP header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TcpHeader {
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Sequence number
    pub seq_num: u32,
    /// Acknowledgment number
    pub ack_num: u32,
    /// Data offset and reserved
    pub data_offset: u8,
    /// Flags
    pub flags: u8,
    /// Window size
    pub window: u16,
    /// Checksum
    pub checksum: u16,
    /// Urgent pointer
    pub urgent_ptr: u16,
}

impl TcpHeader {
    /// Minimum header size
    pub const MIN_SIZE: usize = 20;

    /// Flag: FIN
    pub const FIN: u8 = 0x01;
    /// Flag: SYN
    pub const SYN: u8 = 0x02;
    /// Flag: RST
    pub const RST: u8 = 0x04;
    /// Flag: PSH
    pub const PSH: u8 = 0x08;
    /// Flag: ACK
    pub const ACK: u8 = 0x10;
    /// Flag: URG
    pub const URG: u8 = 0x20;
    /// Flag: ECE
    pub const ECE: u8 = 0x40;
    /// Flag: CWR
    pub const CWR: u8 = 0x80;

    /// Get source port
    pub const fn get_src_port(&self) -> u16 {
        u16::from_be(self.src_port)
    }

    /// Get destination port
    pub const fn get_dst_port(&self) -> u16 {
        u16::from_be(self.dst_port)
    }

    /// Get header length in bytes
    pub const fn header_length(&self) -> usize {
        ((self.data_offset >> 4) as usize) * 4
    }

    /// Check if SYN flag set
    pub const fn is_syn(&self) -> bool {
        (self.flags & Self::SYN) != 0
    }

    /// Check if ACK flag set
    pub const fn is_ack(&self) -> bool {
        (self.flags & Self::ACK) != 0
    }

    /// Check if FIN flag set
    pub const fn is_fin(&self) -> bool {
        (self.flags & Self::FIN) != 0
    }

    /// Check if RST flag set
    pub const fn is_rst(&self) -> bool {
        (self.flags & Self::RST) != 0
    }
}

// =============================================================================
// DHCP
// =============================================================================

/// DHCP message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DhcpMessageType {
    /// Discover
    Discover = 1,
    /// Offer
    Offer    = 2,
    /// Request
    Request  = 3,
    /// Decline
    Decline  = 4,
    /// ACK
    Ack      = 5,
    /// NAK
    Nak      = 6,
    /// Release
    Release  = 7,
    /// Inform
    Inform   = 8,
}

/// DHCP option codes
pub mod dhcp_options {
    pub const SUBNET_MASK: u8 = 1;
    pub const ROUTER: u8 = 3;
    pub const DNS_SERVER: u8 = 6;
    pub const HOSTNAME: u8 = 12;
    pub const DOMAIN_NAME: u8 = 15;
    pub const BROADCAST_ADDR: u8 = 28;
    pub const NTP_SERVER: u8 = 42;
    pub const REQUESTED_IP: u8 = 50;
    pub const LEASE_TIME: u8 = 51;
    pub const MESSAGE_TYPE: u8 = 53;
    pub const SERVER_ID: u8 = 54;
    pub const PARAM_REQUEST: u8 = 55;
    pub const RENEWAL_TIME: u8 = 58;
    pub const REBINDING_TIME: u8 = 59;
    pub const CLIENT_ID: u8 = 61;
    pub const TFTP_SERVER: u8 = 66;
    pub const BOOTFILE_NAME: u8 = 67;
    pub const END: u8 = 255;
}

/// DHCP header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DhcpHeader {
    /// Operation (1 = request, 2 = reply)
    pub op: u8,
    /// Hardware type (1 = Ethernet)
    pub htype: u8,
    /// Hardware address length
    pub hlen: u8,
    /// Hops
    pub hops: u8,
    /// Transaction ID
    pub xid: u32,
    /// Seconds elapsed
    pub secs: u16,
    /// Flags
    pub flags: u16,
    /// Client IP address
    pub ciaddr: [u8; 4],
    /// Your IP address
    pub yiaddr: [u8; 4],
    /// Server IP address
    pub siaddr: [u8; 4],
    /// Gateway IP address
    pub giaddr: [u8; 4],
    /// Client hardware address
    pub chaddr: [u8; 16],
    /// Server name
    pub sname: [u8; 64],
    /// Boot filename
    pub file: [u8; 128],
}

impl DhcpHeader {
    /// DHCP magic cookie
    pub const MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];

    /// Header size (without options)
    pub const SIZE: usize = 236;

    /// Boot request
    pub const BOOTREQUEST: u8 = 1;
    /// Boot reply
    pub const BOOTREPLY: u8 = 2;
}

/// DHCP configuration
#[derive(Debug, Clone)]
pub struct DhcpConfig {
    /// Assigned IP address
    pub ip_address: Ipv4Address,
    /// Subnet mask
    pub subnet_mask: Ipv4Address,
    /// Default gateway
    pub gateway: Ipv4Address,
    /// DNS servers
    pub dns_servers: [Ipv4Address; 4],
    /// Number of DNS servers
    pub dns_count: usize,
    /// DHCP server
    pub dhcp_server: Ipv4Address,
    /// Lease time in seconds
    pub lease_time: u32,
    /// Renewal time
    pub renewal_time: u32,
    /// Domain name
    pub domain: [u8; 64],
    /// Domain length
    pub domain_len: usize,
    /// TFTP server
    pub tftp_server: [u8; 64],
    /// TFTP server length
    pub tftp_len: usize,
    /// Boot filename
    pub boot_file: [u8; 128],
    /// Boot file length
    pub boot_file_len: usize,
}

impl DhcpConfig {
    /// Create new empty config
    pub const fn new() -> Self {
        Self {
            ip_address: Ipv4Address::ANY,
            subnet_mask: Ipv4Address::ANY,
            gateway: Ipv4Address::ANY,
            dns_servers: [Ipv4Address::ANY; 4],
            dns_count: 0,
            dhcp_server: Ipv4Address::ANY,
            lease_time: 0,
            renewal_time: 0,
            domain: [0u8; 64],
            domain_len: 0,
            tftp_server: [0u8; 64],
            tftp_len: 0,
            boot_file: [0u8; 128],
            boot_file_len: 0,
        }
    }
}

impl Default for DhcpConfig {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// PXE
// =============================================================================

/// PXE boot mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PxeBootMode {
    /// Legacy BIOS PXE
    Legacy,
    /// UEFI PXE (x64)
    UefiX64,
    /// UEFI PXE (IA32)
    UefiIa32,
    /// UEFI PXE (ARM64)
    UefiArm64,
    /// UEFI HTTP
    UefiHttp,
}

impl PxeBootMode {
    /// Get architecture identifier for PXE
    pub const fn arch_id(&self) -> u16 {
        match self {
            PxeBootMode::Legacy => 0,
            PxeBootMode::UefiIa32 => 6,
            PxeBootMode::UefiX64 => 7,
            PxeBootMode::UefiArm64 => 11,
            PxeBootMode::UefiHttp => 16,
        }
    }
}

/// PXE boot info
#[derive(Debug, Clone)]
pub struct PxeBootInfo {
    /// Boot mode
    pub mode: PxeBootMode,
    /// Server IP
    pub server_ip: Ipv4Address,
    /// Boot filename
    pub boot_file: [u8; 128],
    /// Boot file length
    pub boot_file_len: usize,
    /// Configuration file (pxelinux.cfg)
    pub config_file: [u8; 128],
    /// Config file length
    pub config_file_len: usize,
    /// MTFTP server (if different)
    pub mtftp_server: Ipv4Address,
    /// MTFTP port
    pub mtftp_port: u16,
}

impl PxeBootInfo {
    /// Create new PXE boot info
    pub const fn new(mode: PxeBootMode) -> Self {
        Self {
            mode,
            server_ip: Ipv4Address::ANY,
            boot_file: [0u8; 128],
            boot_file_len: 0,
            config_file: [0u8; 128],
            config_file_len: 0,
            mtftp_server: Ipv4Address::ANY,
            mtftp_port: 0,
        }
    }
}

// =============================================================================
// TFTP
// =============================================================================

/// TFTP opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TftpOpcode {
    /// Read request
    Rrq   = 1,
    /// Write request
    Wrq   = 2,
    /// Data
    Data  = 3,
    /// Acknowledgment
    Ack   = 4,
    /// Error
    Error = 5,
    /// Option acknowledgment
    Oack  = 6,
}

/// TFTP error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum TftpError {
    /// Not defined
    NotDefined        = 0,
    /// File not found
    FileNotFound      = 1,
    /// Access violation
    AccessViolation   = 2,
    /// Disk full
    DiskFull          = 3,
    /// Illegal operation
    IllegalOperation  = 4,
    /// Unknown transfer ID
    UnknownTid        = 5,
    /// File already exists
    FileExists        = 6,
    /// No such user
    NoSuchUser        = 7,
    /// Option negotiation failed
    OptionNegotiation = 8,
}

/// TFTP transfer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TftpMode {
    /// Binary (octet)
    Binary,
    /// ASCII (netascii)
    Ascii,
}

impl TftpMode {
    /// Get mode string
    pub const fn as_str(&self) -> &'static str {
        match self {
            TftpMode::Binary => "octet",
            TftpMode::Ascii => "netascii",
        }
    }
}

// =============================================================================
// HTTP
// =============================================================================

/// HTTP method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    /// GET
    Get,
    /// POST
    Post,
    /// HEAD
    Head,
    /// PUT
    Put,
    /// DELETE
    Delete,
    /// OPTIONS
    Options,
}

impl HttpMethod {
    /// Get method string
    pub const fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Head => "HEAD",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

/// HTTP status codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpStatus {
    /// 200 OK
    Ok,
    /// 201 Created
    Created,
    /// 204 No Content
    NoContent,
    /// 301 Moved Permanently
    MovedPermanently,
    /// 302 Found
    Found,
    /// 304 Not Modified
    NotModified,
    /// 400 Bad Request
    BadRequest,
    /// 401 Unauthorized
    Unauthorized,
    /// 403 Forbidden
    Forbidden,
    /// 404 Not Found
    NotFound,
    /// 408 Request Timeout
    RequestTimeout,
    /// 500 Internal Server Error
    InternalServerError,
    /// 502 Bad Gateway
    BadGateway,
    /// 503 Service Unavailable
    ServiceUnavailable,
    /// Other status
    Other(u16),
}

impl HttpStatus {
    /// Create from status code
    pub const fn from_code(code: u16) -> Self {
        match code {
            200 => HttpStatus::Ok,
            201 => HttpStatus::Created,
            204 => HttpStatus::NoContent,
            301 => HttpStatus::MovedPermanently,
            302 => HttpStatus::Found,
            304 => HttpStatus::NotModified,
            400 => HttpStatus::BadRequest,
            401 => HttpStatus::Unauthorized,
            403 => HttpStatus::Forbidden,
            404 => HttpStatus::NotFound,
            408 => HttpStatus::RequestTimeout,
            500 => HttpStatus::InternalServerError,
            502 => HttpStatus::BadGateway,
            503 => HttpStatus::ServiceUnavailable,
            n => HttpStatus::Other(n),
        }
    }

    /// Get status code
    pub const fn code(&self) -> u16 {
        match self {
            HttpStatus::Ok => 200,
            HttpStatus::Created => 201,
            HttpStatus::NoContent => 204,
            HttpStatus::MovedPermanently => 301,
            HttpStatus::Found => 302,
            HttpStatus::NotModified => 304,
            HttpStatus::BadRequest => 400,
            HttpStatus::Unauthorized => 401,
            HttpStatus::Forbidden => 403,
            HttpStatus::NotFound => 404,
            HttpStatus::RequestTimeout => 408,
            HttpStatus::InternalServerError => 500,
            HttpStatus::BadGateway => 502,
            HttpStatus::ServiceUnavailable => 503,
            HttpStatus::Other(n) => *n,
        }
    }

    /// Check if success (2xx)
    pub const fn is_success(&self) -> bool {
        let code = self.code();
        code >= 200 && code < 300
    }

    /// Check if redirect (3xx)
    pub const fn is_redirect(&self) -> bool {
        let code = self.code();
        code >= 300 && code < 400
    }

    /// Check if client error (4xx)
    pub const fn is_client_error(&self) -> bool {
        let code = self.code();
        code >= 400 && code < 500
    }

    /// Check if server error (5xx)
    pub const fn is_server_error(&self) -> bool {
        let code = self.code();
        code >= 500 && code < 600
    }
}

// =============================================================================
// DNS
// =============================================================================

/// DNS record types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum DnsRecordType {
    /// A record (IPv4)
    A     = 1,
    /// NS record
    Ns    = 2,
    /// CNAME record
    Cname = 5,
    /// SOA record
    Soa   = 6,
    /// PTR record
    Ptr   = 12,
    /// MX record
    Mx    = 15,
    /// TXT record
    Txt   = 16,
    /// AAAA record (IPv6)
    Aaaa  = 28,
    /// SRV record
    Srv   = 33,
}

/// DNS header
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DnsHeader {
    /// Transaction ID
    pub id: u16,
    /// Flags
    pub flags: u16,
    /// Number of questions
    pub qdcount: u16,
    /// Number of answers
    pub ancount: u16,
    /// Number of authority records
    pub nscount: u16,
    /// Number of additional records
    pub arcount: u16,
}

impl DnsHeader {
    /// Header size
    pub const SIZE: usize = 12;

    /// Query flag
    pub const FLAG_QR: u16 = 0x8000;
    /// Authoritative answer
    pub const FLAG_AA: u16 = 0x0400;
    /// Truncated
    pub const FLAG_TC: u16 = 0x0200;
    /// Recursion desired
    pub const FLAG_RD: u16 = 0x0100;
    /// Recursion available
    pub const FLAG_RA: u16 = 0x0080;

    /// Check if response
    pub const fn is_response(&self) -> bool {
        (u16::from_be(self.flags) & Self::FLAG_QR) != 0
    }

    /// Get response code
    pub const fn rcode(&self) -> u8 {
        (u16::from_be(self.flags) & 0x000F) as u8
    }
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Network error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    /// No network interface
    NoInterface,
    /// Link down
    LinkDown,
    /// DHCP failed
    DhcpFailed,
    /// DNS resolution failed
    DnsResolutionFailed,
    /// Connection refused
    ConnectionRefused,
    /// Connection timeout
    ConnectionTimeout,
    /// Connection reset
    ConnectionReset,
    /// Host unreachable
    HostUnreachable,
    /// Network unreachable
    NetworkUnreachable,
    /// TFTP error
    TftpError,
    /// HTTP error
    HttpError,
    /// Buffer too small
    BufferTooSmall,
    /// Invalid response
    InvalidResponse,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::NoInterface => write!(f, "No network interface"),
            NetworkError::LinkDown => write!(f, "Link down"),
            NetworkError::DhcpFailed => write!(f, "DHCP failed"),
            NetworkError::DnsResolutionFailed => write!(f, "DNS resolution failed"),
            NetworkError::ConnectionRefused => write!(f, "Connection refused"),
            NetworkError::ConnectionTimeout => write!(f, "Connection timeout"),
            NetworkError::ConnectionReset => write!(f, "Connection reset"),
            NetworkError::HostUnreachable => write!(f, "Host unreachable"),
            NetworkError::NetworkUnreachable => write!(f, "Network unreachable"),
            NetworkError::TftpError => write!(f, "TFTP error"),
            NetworkError::HttpError => write!(f, "HTTP error"),
            NetworkError::BufferTooSmall => write!(f, "Buffer too small"),
            NetworkError::InvalidResponse => write!(f, "Invalid response"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_mac_address() {
        let mac = MacAddress::new([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert!(!mac.is_broadcast());
        assert!(!mac.is_multicast());
        assert!(MacAddress::BROADCAST.is_broadcast());
    }

    #[test]
    fn test_ipv4_address() {
        let addr = Ipv4Address::new(192, 168, 1, 1);
        assert!(addr.is_private());
        assert!(!addr.is_loopback());
        assert!(!addr.is_multicast());

        let loopback = Ipv4Address::LOCALHOST;
        assert!(loopback.is_loopback());
    }

    #[test]
    fn test_ipv4_mask() {
        let addr = Ipv4Address::new(192, 168, 1, 100);
        let mask = Ipv4Address::new(255, 255, 255, 0);
        let network = addr.apply_mask(&mask);
        assert_eq!(network.octets, [192, 168, 1, 0]);
    }

    #[test]
    fn test_ipv6_address() {
        let localhost = Ipv6Address::LOCALHOST;
        assert!(localhost.is_loopback());
        assert!(!localhost.is_multicast());
    }

    #[test]
    fn test_http_status() {
        assert!(HttpStatus::Ok.is_success());
        assert!(HttpStatus::NotFound.is_client_error());
        assert!(HttpStatus::InternalServerError.is_server_error());
        assert!(HttpStatus::MovedPermanently.is_redirect());
    }

    #[test]
    fn test_tcp_flags() {
        let header = TcpHeader {
            flags: TcpHeader::SYN | TcpHeader::ACK,
            ..unsafe { core::mem::zeroed() }
        };
        assert!(header.is_syn());
        assert!(header.is_ack());
        assert!(!header.is_fin());
    }
}
