//! Network Addresses
//!
//! IP address types for netfilter.

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv4Addr(pub [u8; 4]);

impl Ipv4Addr {
    /// Create from octets
    #[inline(always)]
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// Create from u32
    #[inline]
    pub const fn from_u32(addr: u32) -> Self {
        Self([
            ((addr >> 24) & 0xFF) as u8,
            ((addr >> 16) & 0xFF) as u8,
            ((addr >> 8) & 0xFF) as u8,
            (addr & 0xFF) as u8,
        ])
    }

    /// Convert to u32
    #[inline]
    pub const fn to_u32(&self) -> u32 {
        ((self.0[0] as u32) << 24)
            | ((self.0[1] as u32) << 16)
            | ((self.0[2] as u32) << 8)
            | (self.0[3] as u32)
    }

    /// Any address (0.0.0.0)
    pub const ANY: Self = Self([0, 0, 0, 0]);

    /// Broadcast (255.255.255.255)
    pub const BROADCAST: Self = Self([255, 255, 255, 255]);

    /// Localhost (127.0.0.1)
    pub const LOCALHOST: Self = Self([127, 0, 0, 1]);
}

/// IPv6 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ipv6Addr(pub [u8; 16]);

impl Ipv6Addr {
    /// Create from segments
    #[inline]
    pub const fn new(segments: [u16; 8]) -> Self {
        let mut octets = [0u8; 16];
        let mut i = 0;
        while i < 8 {
            octets[i * 2] = (segments[i] >> 8) as u8;
            octets[i * 2 + 1] = (segments[i] & 0xFF) as u8;
            i += 1;
        }
        Self(octets)
    }

    /// Any address (::)
    pub const ANY: Self = Self([0; 16]);

    /// Localhost (::1)
    pub const LOCALHOST: Self = Self([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
}

/// Network address with CIDR
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkAddr {
    /// IPv4 with prefix
    V4(Ipv4Addr, u8),
    /// IPv6 with prefix
    V6(Ipv6Addr, u8),
}

impl NetworkAddr {
    /// Check if address matches
    pub fn matches(&self, addr: &NetworkAddr) -> bool {
        match (self, addr) {
            (NetworkAddr::V4(net, prefix), NetworkAddr::V4(ip, _)) => {
                let mask = if *prefix >= 32 {
                    u32::MAX
                } else {
                    u32::MAX << (32 - prefix)
                };
                (net.to_u32() & mask) == (ip.to_u32() & mask)
            }
            (NetworkAddr::V6(_, _), NetworkAddr::V6(_, _)) => {
                // Simplified - would need full implementation
                true
            }
            _ => false,
        }
    }
}

/// Port range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortRange {
    /// Start port
    pub start: u16,
    /// End port
    pub end: u16,
}

impl PortRange {
    /// Create single port
    #[inline(always)]
    pub const fn single(port: u16) -> Self {
        Self { start: port, end: port }
    }

    /// Create range
    #[inline(always)]
    pub const fn range(start: u16, end: u16) -> Self {
        Self { start, end }
    }

    /// Any port
    pub const ANY: Self = Self { start: 0, end: 65535 };

    /// Check if port is in range
    #[inline(always)]
    pub fn contains(&self, port: u16) -> bool {
        port >= self.start && port <= self.end
    }
}
