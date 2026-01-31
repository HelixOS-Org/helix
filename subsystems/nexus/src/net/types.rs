//! Network Core Types
//!
//! Fundamental network types: addresses and identifiers.

use alloc::string::String;

// ============================================================================
// CORE IDENTIFIERS
// ============================================================================

/// Network interface index
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IfIndex(pub u32);

impl IfIndex {
    /// Create new index
    pub const fn new(idx: u32) -> Self {
        Self(idx)
    }
}

// ============================================================================
// MAC ADDRESS
// ============================================================================

/// MAC address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    /// Create new MAC address
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    /// Zero MAC
    pub const fn zero() -> Self {
        Self([0; 6])
    }

    /// Broadcast MAC
    pub const fn broadcast() -> Self {
        Self([0xff; 6])
    }

    /// Is multicast
    pub fn is_multicast(&self) -> bool {
        self.0[0] & 0x01 != 0
    }

    /// Is broadcast
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xff; 6]
    }

    /// Is zero
    pub fn is_zero(&self) -> bool {
        self.0 == [0; 6]
    }

    /// Format as string
    pub fn to_string(&self) -> String {
        alloc::format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0],
            self.0[1],
            self.0[2],
            self.0[3],
            self.0[4],
            self.0[5]
        )
    }
}

// ============================================================================
// IP ADDRESSES
// ============================================================================

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    /// Create new IPv4 address
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// From u32
    pub fn from_u32(val: u32) -> Self {
        Self(val.to_be_bytes())
    }

    /// To u32
    pub fn to_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    /// Is loopback
    pub fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }

    /// Is private
    pub fn is_private(&self) -> bool {
        self.0[0] == 10
            || (self.0[0] == 172 && (self.0[1] >= 16 && self.0[1] <= 31))
            || (self.0[0] == 192 && self.0[1] == 168)
    }

    /// Format as string
    pub fn to_string(&self) -> String {
        alloc::format!("{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// IPv6 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv6Address(pub [u8; 16]);

impl Ipv6Address {
    /// Create new IPv6 address
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Loopback
    pub const fn loopback() -> Self {
        Self([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1])
    }

    /// Is loopback
    pub fn is_loopback(&self) -> bool {
        self.0 == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]
    }

    /// Is link local
    pub fn is_link_local(&self) -> bool {
        self.0[0] == 0xfe && (self.0[1] & 0xc0) == 0x80
    }
}

// ============================================================================
// LINK PROPERTIES
// ============================================================================

/// Duplex mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Duplex {
    /// Half duplex
    Half,
    /// Full duplex
    Full,
    /// Unknown
    Unknown,
}

impl Duplex {
    /// Get duplex name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Half => "half",
            Self::Full => "full",
            Self::Unknown => "unknown",
        }
    }
}

/// Link speed (Mbps)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LinkSpeed(pub u32);

impl LinkSpeed {
    /// Common speeds
    pub const SPEED_10: Self = Self(10);
    pub const SPEED_100: Self = Self(100);
    pub const SPEED_1000: Self = Self(1000);
    pub const SPEED_2500: Self = Self(2500);
    pub const SPEED_5000: Self = Self(5000);
    pub const SPEED_10000: Self = Self(10000);
    pub const SPEED_25000: Self = Self(25000);
    pub const SPEED_40000: Self = Self(40000);
    pub const SPEED_50000: Self = Self(50000);
    pub const SPEED_100000: Self = Self(100000);
    pub const SPEED_200000: Self = Self(200000);
    pub const SPEED_400000: Self = Self(400000);

    /// Format as string
    pub fn to_string(&self) -> String {
        if self.0 >= 1000 {
            alloc::format!("{}Gbps", self.0 / 1000)
        } else {
            alloc::format!("{}Mbps", self.0)
        }
    }

    /// Bytes per second
    pub fn bytes_per_sec(&self) -> u64 {
        (self.0 as u64) * 1_000_000 / 8
    }
}
