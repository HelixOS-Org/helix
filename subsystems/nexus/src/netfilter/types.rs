//! Netfilter Core Types
//!
//! Fundamental types for packet filtering.

use alloc::string::String;

/// Rule identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleId(pub u64);

impl RuleId {
    /// Create a new rule ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Chain identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChainId(pub u64);

impl ChainId {
    /// Create a new chain ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Table identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TableId(pub u64);

impl TableId {
    /// Create a new table ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Connection tracking entry ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConntrackId(pub u64);

impl ConntrackId {
    /// Create a new conntrack ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Protocol {
    /// Any protocol
    Any,
    /// ICMP
    Icmp,
    /// TCP
    Tcp,
    /// UDP
    Udp,
    /// SCTP
    Sctp,
    /// GRE
    Gre,
    /// ESP
    Esp,
    /// AH
    Ah,
    /// ICMPv6
    Icmpv6,
    /// Raw IP number
    Raw(u8),
}

impl Protocol {
    /// Get protocol number
    pub fn number(&self) -> u8 {
        match self {
            Self::Any => 0,
            Self::Icmp => 1,
            Self::Tcp => 6,
            Self::Udp => 17,
            Self::Sctp => 132,
            Self::Gre => 47,
            Self::Esp => 50,
            Self::Ah => 51,
            Self::Icmpv6 => 58,
            Self::Raw(n) => *n,
        }
    }

    /// Get protocol name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Icmp => "icmp",
            Self::Tcp => "tcp",
            Self::Udp => "udp",
            Self::Sctp => "sctp",
            Self::Gre => "gre",
            Self::Esp => "esp",
            Self::Ah => "ah",
            Self::Icmpv6 => "icmpv6",
            Self::Raw(_) => "raw",
        }
    }
}

/// Address family
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AddressFamily {
    /// IPv4
    Inet,
    /// IPv6
    Inet6,
    /// ARP
    Arp,
    /// Bridge
    Bridge,
    /// Netdev
    Netdev,
}

impl AddressFamily {
    /// Get family name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Inet => "inet",
            Self::Inet6 => "inet6",
            Self::Arp => "arp",
            Self::Bridge => "bridge",
            Self::Netdev => "netdev",
        }
    }
}

/// Hook type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HookType {
    /// Pre-routing
    Prerouting,
    /// Input
    Input,
    /// Forward
    Forward,
    /// Output
    Output,
    /// Post-routing
    Postrouting,
    /// Ingress (netdev)
    Ingress,
    /// Egress (netdev)
    Egress,
}

impl HookType {
    /// Get hook name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Prerouting => "prerouting",
            Self::Input => "input",
            Self::Forward => "forward",
            Self::Output => "output",
            Self::Postrouting => "postrouting",
            Self::Ingress => "ingress",
            Self::Egress => "egress",
        }
    }

    /// Get hook priority range (lower = earlier)
    #[inline]
    pub fn default_priority(&self) -> i32 {
        match self {
            Self::Prerouting => -100,
            Self::Input => 0,
            Self::Forward => 0,
            Self::Output => 0,
            Self::Postrouting => 100,
            Self::Ingress => -450,
            Self::Egress => -450,
        }
    }
}

/// Rule verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Accept packet
    Accept,
    /// Drop packet
    Drop,
    /// Reject packet (with ICMP)
    Reject,
    /// Jump to chain
    Jump(ChainId),
    /// Go to chain (no return)
    Goto(ChainId),
    /// Return from chain
    Return,
    /// Continue to next rule
    Continue,
    /// Queue to userspace
    Queue(u16),
    /// Mark and continue
    Mark(u32),
    /// Masquerade (NAT)
    Masquerade,
    /// SNAT
    Snat,
    /// DNAT
    Dnat,
    /// Redirect
    Redirect,
}

impl Verdict {
    /// Get verdict name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Drop => "drop",
            Self::Reject => "reject",
            Self::Jump(_) => "jump",
            Self::Goto(_) => "goto",
            Self::Return => "return",
            Self::Continue => "continue",
            Self::Queue(_) => "queue",
            Self::Mark(_) => "mark",
            Self::Masquerade => "masquerade",
            Self::Snat => "snat",
            Self::Dnat => "dnat",
            Self::Redirect => "redirect",
        }
    }

    /// Is terminal verdict
    #[inline(always)]
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Accept | Self::Drop | Self::Reject | Self::Goto(_))
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    /// New connection
    New,
    /// Established connection
    Established,
    /// Related connection
    Related,
    /// Invalid packet
    Invalid,
    /// Untracked
    Untracked,
}

impl ConnState {
    /// Get state name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Established => "established",
            Self::Related => "related",
            Self::Invalid => "invalid",
            Self::Untracked => "untracked",
        }
    }
}
