//! # Network Subsystem
//!
//! Network stack initialization and management.
//! Late phase subsystem for networking support.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// =============================================================================
// NETWORK TYPES
// =============================================================================

/// Network interface index
pub type InterfaceIndex = u32;

/// Socket descriptor
pub type SocketFd = i32;

/// Port number
pub type Port = u16;

// =============================================================================
// ADDRESSES
// =============================================================================

/// MAC address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MacAddress(pub [u8; 6]);

impl MacAddress {
    /// Broadcast address
    pub const BROADCAST: Self = Self([0xFF; 6]);

    /// Check if broadcast
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xFF; 6]
    }

    /// Check if multicast
    pub fn is_multicast(&self) -> bool {
        (self.0[0] & 0x01) != 0
    }
}

impl core::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// IPv4 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ipv4Address(pub [u8; 4]);

impl Ipv4Address {
    /// Any address (0.0.0.0)
    pub const ANY: Self = Self([0, 0, 0, 0]);

    /// Loopback address (127.0.0.1)
    pub const LOOPBACK: Self = Self([127, 0, 0, 1]);

    /// Broadcast address (255.255.255.255)
    pub const BROADCAST: Self = Self([255, 255, 255, 255]);

    /// Create from bytes
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// Is loopback?
    pub fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }

    /// Is private?
    pub fn is_private(&self) -> bool {
        self.0[0] == 10
            || (self.0[0] == 172 && (self.0[1] >= 16 && self.0[1] <= 31))
            || (self.0[0] == 192 && self.0[1] == 168)
    }

    /// To u32
    pub fn to_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }
}

impl core::fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

/// IPv6 address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ipv6Address(pub [u8; 16]);

impl Ipv6Address {
    /// Any address (::)
    pub const ANY: Self = Self([0; 16]);

    /// Loopback address (::1)
    pub const LOOPBACK: Self = Self([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
}

/// Socket address
#[derive(Debug, Clone, Copy)]
pub enum SocketAddress {
    V4 { addr: Ipv4Address, port: Port },
    V6 { addr: Ipv6Address, port: Port },
}

impl SocketAddress {
    /// Create IPv4 socket address
    pub fn v4(addr: Ipv4Address, port: Port) -> Self {
        Self::V4 { addr, port }
    }

    /// Create IPv6 socket address
    pub fn v6(addr: Ipv6Address, port: Port) -> Self {
        Self::V6 { addr, port }
    }

    /// Get port
    pub fn port(&self) -> Port {
        match self {
            Self::V4 { port, .. } => *port,
            Self::V6 { port, .. } => *port,
        }
    }
}

// =============================================================================
// NETWORK INTERFACE
// =============================================================================

/// Interface state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    Down,
    Up,
    Running,
    Dormant,
}

impl Default for InterfaceState {
    fn default() -> Self {
        Self::Down
    }
}

/// Interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    Loopback,
    Ethernet,
    Wifi,
    Virtual,
    Bridge,
    Tunnel,
}

impl Default for InterfaceType {
    fn default() -> Self {
        Self::Ethernet
    }
}

/// Network interface
pub struct NetworkInterface {
    pub index: InterfaceIndex,
    pub name: String,
    pub if_type: InterfaceType,
    pub state: InterfaceState,
    pub mac: MacAddress,
    pub mtu: u32,

    // IPv4 config
    pub ipv4_addr: Option<Ipv4Address>,
    pub ipv4_netmask: Option<Ipv4Address>,
    pub ipv4_gateway: Option<Ipv4Address>,

    // IPv6 config
    pub ipv6_addrs: Vec<Ipv6Address>,

    // Statistics
    pub rx_packets: AtomicU64,
    pub tx_packets: AtomicU64,
    pub rx_bytes: AtomicU64,
    pub tx_bytes: AtomicU64,
    pub rx_errors: AtomicU64,
    pub tx_errors: AtomicU64,
    pub rx_dropped: AtomicU64,
    pub tx_dropped: AtomicU64,
}

impl NetworkInterface {
    /// Create new interface
    pub fn new(index: InterfaceIndex, name: String, if_type: InterfaceType) -> Self {
        Self {
            index,
            name,
            if_type,
            state: InterfaceState::Down,
            mac: MacAddress::default(),
            mtu: 1500,
            ipv4_addr: None,
            ipv4_netmask: None,
            ipv4_gateway: None,
            ipv6_addrs: Vec::new(),
            rx_packets: AtomicU64::new(0),
            tx_packets: AtomicU64::new(0),
            rx_bytes: AtomicU64::new(0),
            tx_bytes: AtomicU64::new(0),
            rx_errors: AtomicU64::new(0),
            tx_errors: AtomicU64::new(0),
            rx_dropped: AtomicU64::new(0),
            tx_dropped: AtomicU64::new(0),
        }
    }

    /// Bring interface up
    pub fn up(&mut self) {
        self.state = InterfaceState::Up;
    }

    /// Bring interface down
    pub fn down(&mut self) {
        self.state = InterfaceState::Down;
    }

    /// Is interface running?
    pub fn is_running(&self) -> bool {
        self.state == InterfaceState::Running || self.state == InterfaceState::Up
    }

    /// Get statistics
    pub fn stats(&self) -> InterfaceStats {
        InterfaceStats {
            rx_packets: self.rx_packets.load(Ordering::Relaxed),
            tx_packets: self.tx_packets.load(Ordering::Relaxed),
            rx_bytes: self.rx_bytes.load(Ordering::Relaxed),
            tx_bytes: self.tx_bytes.load(Ordering::Relaxed),
            rx_errors: self.rx_errors.load(Ordering::Relaxed),
            tx_errors: self.tx_errors.load(Ordering::Relaxed),
            rx_dropped: self.rx_dropped.load(Ordering::Relaxed),
            tx_dropped: self.tx_dropped.load(Ordering::Relaxed),
        }
    }
}

/// Interface statistics
#[derive(Debug, Clone, Default)]
pub struct InterfaceStats {
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
}

// =============================================================================
// SOCKETS
// =============================================================================

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Stream,    // TCP
    Datagram,  // UDP
    Raw,       // Raw IP
    SeqPacket, // SCTP-like
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Closed,
    Bound,
    Listening,
    Connecting,
    Connected,
    Closing,
}

impl Default for SocketState {
    fn default() -> Self {
        Self::Closed
    }
}

/// Socket
pub struct Socket {
    pub fd: SocketFd,
    pub sock_type: SocketType,
    pub state: SocketState,
    pub local_addr: Option<SocketAddress>,
    pub remote_addr: Option<SocketAddress>,
    pub recv_buffer: Vec<u8>,
    pub send_buffer: Vec<u8>,
    pub nonblocking: AtomicBool,
}

impl Socket {
    /// Create new socket
    pub fn new(fd: SocketFd, sock_type: SocketType) -> Self {
        Self {
            fd,
            sock_type,
            state: SocketState::Closed,
            local_addr: None,
            remote_addr: None,
            recv_buffer: Vec::with_capacity(65536),
            send_buffer: Vec::with_capacity(65536),
            nonblocking: AtomicBool::new(false),
        }
    }
}

// =============================================================================
// ROUTING
// =============================================================================

/// Route entry
#[derive(Debug, Clone)]
pub struct Route {
    pub destination: Ipv4Address,
    pub netmask: Ipv4Address,
    pub gateway: Option<Ipv4Address>,
    pub interface: InterfaceIndex,
    pub metric: u32,
    pub flags: RouteFlags,
}

/// Route flags
#[derive(Debug, Clone, Copy, Default)]
pub struct RouteFlags {
    pub up: bool,
    pub gateway: bool,
    pub host: bool,
    pub reject: bool,
    pub dynamic: bool,
}

/// Routing table
pub struct RoutingTable {
    routes: Vec<Route>,
}

impl RoutingTable {
    /// Create new routing table
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add route
    pub fn add(&mut self, route: Route) {
        self.routes.push(route);
        // Sort by netmask (most specific first)
        self.routes
            .sort_by(|a, b| b.netmask.to_u32().cmp(&a.netmask.to_u32()));
    }

    /// Remove route
    pub fn remove(&mut self, dest: Ipv4Address, mask: Ipv4Address) -> bool {
        if let Some(pos) = self
            .routes
            .iter()
            .position(|r| r.destination == dest && r.netmask == mask)
        {
            self.routes.remove(pos);
            true
        } else {
            false
        }
    }

    /// Lookup route for destination
    pub fn lookup(&self, dest: Ipv4Address) -> Option<&Route> {
        let dest_u32 = dest.to_u32();

        for route in &self.routes {
            let mask_u32 = route.netmask.to_u32();
            let net_u32 = route.destination.to_u32();

            if (dest_u32 & mask_u32) == (net_u32 & mask_u32) {
                return Some(route);
            }
        }

        None
    }

    /// Get all routes
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }
}

impl Default for RoutingTable {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// NETWORK SUBSYSTEM
// =============================================================================

/// Network Subsystem
///
/// Manages network interfaces and the network stack.
pub struct NetworkSubsystem {
    info: SubsystemInfo,

    // Interfaces
    interfaces: Vec<NetworkInterface>,
    next_if_index: InterfaceIndex,

    // Sockets
    sockets: BTreeMap<SocketFd, Socket>,
    next_socket_fd: AtomicU64,

    // Routing
    routing_table: RoutingTable,

    // DNS
    dns_servers: Vec<Ipv4Address>,

    // State
    initialized: bool,
}

static NET_DEPS: [Dependency; 2] = [Dependency::required("drivers"), Dependency::required("ipc")];

impl NetworkSubsystem {
    /// Create new network subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("network", InitPhase::Late)
                .with_priority(700)
                .with_description("Network stack")
                .with_dependencies(&NET_DEPS)
                .provides(PhaseCapabilities::NETWORK),
            interfaces: Vec::new(),
            next_if_index: 1,
            sockets: BTreeMap::new(),
            next_socket_fd: AtomicU64::new(1),
            routing_table: RoutingTable::new(),
            dns_servers: Vec::new(),
            initialized: false,
        }
    }

    /// Register network interface
    pub fn register_interface(&mut self, mut iface: NetworkInterface) -> InterfaceIndex {
        iface.index = self.next_if_index;
        self.next_if_index += 1;

        let index = iface.index;
        self.interfaces.push(iface);
        index
    }

    /// Get interface by index
    pub fn get_interface(&self, index: InterfaceIndex) -> Option<&NetworkInterface> {
        self.interfaces.iter().find(|i| i.index == index)
    }

    /// Get interface by index (mutable)
    pub fn get_interface_mut(&mut self, index: InterfaceIndex) -> Option<&mut NetworkInterface> {
        self.interfaces.iter_mut().find(|i| i.index == index)
    }

    /// Get interface by name
    pub fn get_interface_by_name(&self, name: &str) -> Option<&NetworkInterface> {
        self.interfaces.iter().find(|i| i.name == name)
    }

    /// List all interfaces
    pub fn interfaces(&self) -> &[NetworkInterface] {
        &self.interfaces
    }

    /// Configure interface IP
    pub fn configure_ip(
        &mut self,
        index: InterfaceIndex,
        addr: Ipv4Address,
        netmask: Ipv4Address,
        gateway: Option<Ipv4Address>,
    ) -> InitResult<()> {
        let iface = self
            .get_interface_mut(index)
            .ok_or_else(|| InitError::new(ErrorKind::NotFound, "Interface not found"))?;

        iface.ipv4_addr = Some(addr);
        iface.ipv4_netmask = Some(netmask);
        iface.ipv4_gateway = gateway;

        // Add route for local network
        let dest = Ipv4Address([
            addr.0[0] & netmask.0[0],
            addr.0[1] & netmask.0[1],
            addr.0[2] & netmask.0[2],
            addr.0[3] & netmask.0[3],
        ]);

        self.routing_table.add(Route {
            destination: dest,
            netmask,
            gateway: None,
            interface: index,
            metric: 0,
            flags: RouteFlags {
                up: true,
                ..Default::default()
            },
        });

        // Add default route if gateway provided
        if let Some(gw) = gateway {
            self.routing_table.add(Route {
                destination: Ipv4Address::ANY,
                netmask: Ipv4Address::ANY,
                gateway: Some(gw),
                interface: index,
                metric: 100,
                flags: RouteFlags {
                    up: true,
                    gateway: true,
                    ..Default::default()
                },
            });
        }

        Ok(())
    }

    /// Create socket
    pub fn socket(&mut self, sock_type: SocketType) -> SocketFd {
        let fd = self.next_socket_fd.fetch_add(1, Ordering::SeqCst) as SocketFd;
        self.sockets.insert(fd, Socket::new(fd, sock_type));
        fd
    }

    /// Close socket
    pub fn close_socket(&mut self, fd: SocketFd) -> InitResult<()> {
        self.sockets
            .remove(&fd)
            .ok_or_else(|| InitError::new(ErrorKind::InvalidArgument, "Bad socket"))?;
        Ok(())
    }

    /// Bind socket
    pub fn bind(&mut self, fd: SocketFd, addr: SocketAddress) -> InitResult<()> {
        let socket = self
            .sockets
            .get_mut(&fd)
            .ok_or_else(|| InitError::new(ErrorKind::InvalidArgument, "Bad socket"))?;

        socket.local_addr = Some(addr);
        socket.state = SocketState::Bound;
        Ok(())
    }

    /// Connect socket
    pub fn connect(&mut self, fd: SocketFd, addr: SocketAddress) -> InitResult<()> {
        let socket = self
            .sockets
            .get_mut(&fd)
            .ok_or_else(|| InitError::new(ErrorKind::InvalidArgument, "Bad socket"))?;

        socket.remote_addr = Some(addr);
        socket.state = SocketState::Connected;
        Ok(())
    }

    /// Listen on socket
    pub fn listen(&mut self, fd: SocketFd, _backlog: u32) -> InitResult<()> {
        let socket = self
            .sockets
            .get_mut(&fd)
            .ok_or_else(|| InitError::new(ErrorKind::InvalidArgument, "Bad socket"))?;

        if socket.state != SocketState::Bound {
            return Err(InitError::new(ErrorKind::InvalidState, "Not bound"));
        }

        socket.state = SocketState::Listening;
        Ok(())
    }

    /// Add DNS server
    pub fn add_dns_server(&mut self, server: Ipv4Address) {
        if !self.dns_servers.contains(&server) {
            self.dns_servers.push(server);
        }
    }

    /// Get DNS servers
    pub fn dns_servers(&self) -> &[Ipv4Address] {
        &self.dns_servers
    }

    /// Get routing table
    pub fn routing_table(&self) -> &RoutingTable {
        &self.routing_table
    }

    /// Initialize loopback interface
    fn init_loopback(&mut self) {
        let mut lo = NetworkInterface::new(0, String::from("lo"), InterfaceType::Loopback);
        lo.mac = MacAddress([0, 0, 0, 0, 0, 0]);
        lo.mtu = 65536;
        lo.ipv4_addr = Some(Ipv4Address::LOOPBACK);
        lo.ipv4_netmask = Some(Ipv4Address::new(255, 0, 0, 0));
        lo.state = InterfaceState::Up;

        let idx = self.register_interface(lo);

        // Add loopback route
        self.routing_table.add(Route {
            destination: Ipv4Address::LOOPBACK,
            netmask: Ipv4Address::new(255, 0, 0, 0),
            gateway: None,
            interface: idx,
            metric: 0,
            flags: RouteFlags {
                up: true,
                host: true,
                ..Default::default()
            },
        });
    }
}

impl Default for NetworkSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for NetworkSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing network subsystem");

        // Initialize loopback
        self.init_loopback();
        ctx.debug("Loopback interface initialized");

        // Initialize default DNS
        self.add_dns_server(Ipv4Address::new(8, 8, 8, 8));
        self.add_dns_server(Ipv4Address::new(8, 8, 4, 4));

        self.initialized = true;

        ctx.info(alloc::format!(
            "Network: {} interfaces, {} routes",
            self.interfaces.len(),
            self.routing_table.routes().len()
        ));

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Network subsystem shutdown");

        // Close all sockets
        self.sockets.clear();

        // Bring down all interfaces
        for iface in &mut self.interfaces {
            iface.down();
        }

        self.initialized = false;

        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_subsystem() {
        let sub = NetworkSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Late);
        assert!(sub.info().provides.contains(PhaseCapabilities::NETWORK));
    }

    #[test]
    fn test_ipv4_address() {
        let addr = Ipv4Address::new(192, 168, 1, 1);
        assert!(addr.is_private());
        assert!(!addr.is_loopback());

        let lo = Ipv4Address::LOOPBACK;
        assert!(lo.is_loopback());
    }

    #[test]
    fn test_mac_address() {
        let mac = MacAddress([0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
        assert!(!mac.is_broadcast());
        assert!(!mac.is_multicast());

        assert!(MacAddress::BROADCAST.is_broadcast());
    }

    #[test]
    fn test_routing() {
        let mut table = RoutingTable::new();

        table.add(Route {
            destination: Ipv4Address::new(192, 168, 1, 0),
            netmask: Ipv4Address::new(255, 255, 255, 0),
            gateway: None,
            interface: 1,
            metric: 0,
            flags: RouteFlags::default(),
        });

        let route = table.lookup(Ipv4Address::new(192, 168, 1, 100));
        assert!(route.is_some());
        assert_eq!(route.unwrap().interface, 1);
    }
}
