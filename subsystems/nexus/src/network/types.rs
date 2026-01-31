//! Network Types
//!
//! Core types for network intelligence.

/// Network protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// TCP protocol
    Tcp,
    /// UDP protocol
    Udp,
    /// ICMP protocol
    Icmp,
    /// Raw IP
    RawIp,
    /// SCTP protocol
    Sctp,
    /// Unknown protocol
    Unknown(u8),
}

impl Protocol {
    /// From IP protocol number
    pub fn from_number(n: u8) -> Self {
        match n {
            1 => Self::Icmp,
            6 => Self::Tcp,
            17 => Self::Udp,
            132 => Self::Sctp,
            _ => Self::Unknown(n),
        }
    }

    /// To IP protocol number
    pub fn to_number(&self) -> u8 {
        match self {
            Self::Icmp => 1,
            Self::Tcp => 6,
            Self::Udp => 17,
            Self::Sctp => 132,
            Self::RawIp => 0,
            Self::Unknown(n) => *n,
        }
    }

    /// Is reliable protocol
    pub fn is_reliable(&self) -> bool {
        matches!(self, Self::Tcp | Self::Sctp)
    }

    /// Is connection-oriented
    pub fn is_connection_oriented(&self) -> bool {
        matches!(self, Self::Tcp | Self::Sctp)
    }
}

/// Network direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Inbound traffic
    Inbound,
    /// Outbound traffic
    Outbound,
    /// Both directions
    Both,
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// New connection
    New,
    /// Established connection
    Established,
    /// Closing connection
    Closing,
    /// Closed connection
    Closed,
    /// Connection timeout
    Timeout,
    /// Connection reset
    Reset,
}

/// QoS class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QosClass {
    /// Best effort (default)
    BestEffort     = 0,
    /// Background traffic
    Background     = 1,
    /// Standard priority
    Standard       = 2,
    /// Video streaming
    Video          = 3,
    /// Voice traffic
    Voice          = 4,
    /// Real-time traffic
    RealTime       = 5,
    /// Network control
    NetworkControl = 6,
}

impl QosClass {
    /// Get DSCP value
    pub fn to_dscp(&self) -> u8 {
        match self {
            Self::BestEffort => 0,
            Self::Background => 8,
            Self::Standard => 16,
            Self::Video => 34,
            Self::Voice => 46,
            Self::RealTime => 48,
            Self::NetworkControl => 56,
        }
    }
}
