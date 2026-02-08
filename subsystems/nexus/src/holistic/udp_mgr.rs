// SPDX-License-Identifier: GPL-2.0
//! Holistic UDP manager — datagram-oriented transport with multicast support

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// UDP socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpSocketState {
    Unbound,
    Bound,
    Connected,
    Closed,
}

/// UDP multicast mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpMulticastMode {
    AsmInclude,
    AsmExclude,
    Ssm,
    Disabled,
}

/// UDP checksum mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpChecksumMode {
    Full,
    NoCheck,
    HardwareOffload,
    Partial,
}

/// UDP datagram record
#[derive(Debug, Clone)]
pub struct UdpDatagram {
    pub src_port: u16,
    pub dst_port: u16,
    pub payload_len: u32,
    pub checksum: u32,
    pub timestamp_ns: u64,
    pub ttl: u8,
    pub fragmented: bool,
}

impl UdpDatagram {
    pub fn new(src_port: u16, dst_port: u16, payload_len: u32) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in src_port.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        for b in dst_port.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            src_port,
            dst_port,
            payload_len,
            checksum: h as u32,
            timestamp_ns: 0,
            ttl: 64,
            fragmented: payload_len > 1472,
        }
    }

    pub fn fragment_count(&self) -> u32 {
        if self.payload_len <= 1472 {
            1
        } else {
            (self.payload_len + 1471) / 1472
        }
    }
}

/// UDP socket
#[derive(Debug, Clone)]
pub struct UdpSocket {
    pub socket_id: u64,
    pub state: UdpSocketState,
    pub local_port: u16,
    pub remote_port: u16,
    pub checksum_mode: UdpChecksumMode,
    pub multicast_mode: UdpMulticastMode,
    pub multicast_groups: Vec<u32>,
    pub datagrams_sent: u64,
    pub datagrams_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub send_buffer_size: u32,
    pub recv_buffer_size: u32,
    pub drops: u64,
}

impl UdpSocket {
    pub fn new(socket_id: u64) -> Self {
        Self {
            socket_id,
            state: UdpSocketState::Unbound,
            local_port: 0,
            remote_port: 0,
            checksum_mode: UdpChecksumMode::Full,
            multicast_mode: UdpMulticastMode::Disabled,
            multicast_groups: Vec::new(),
            datagrams_sent: 0,
            datagrams_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            send_buffer_size: 212992,
            recv_buffer_size: 212992,
            drops: 0,
        }
    }

    pub fn bind(&mut self, port: u16) {
        self.local_port = port;
        self.state = UdpSocketState::Bound;
    }

    pub fn send(&mut self, datagram: &UdpDatagram) {
        self.datagrams_sent += 1;
        self.bytes_sent += datagram.payload_len as u64;
    }

    pub fn receive(&mut self, datagram: &UdpDatagram) {
        self.datagrams_received += 1;
        self.bytes_received += datagram.payload_len as u64;
    }

    pub fn join_multicast(&mut self, group_id: u32) {
        if !self.multicast_groups.contains(&group_id) {
            self.multicast_groups.push(group_id);
            self.multicast_mode = UdpMulticastMode::AsmInclude;
        }
    }

    pub fn drop_rate(&self) -> f64 {
        let total = self.datagrams_received + self.drops;
        if total == 0 {
            return 0.0;
        }
        self.drops as f64 / total as f64
    }

    pub fn avg_datagram_size(&self) -> u64 {
        if self.datagrams_sent == 0 {
            return 0;
        }
        self.bytes_sent / self.datagrams_sent
    }
}

/// UDP manager stats
#[derive(Debug, Clone)]
pub struct UdpMgrStats {
    pub total_sockets: u64,
    pub active_sockets: u64,
    pub total_datagrams: u64,
    pub total_bytes: u64,
    pub total_drops: u64,
}

/// Main holistic UDP manager
#[derive(Debug)]
pub struct HolisticUdpMgr {
    pub sockets: BTreeMap<u64, UdpSocket>,
    pub port_map: BTreeMap<u16, u64>,
    pub stats: UdpMgrStats,
    pub next_socket_id: u64,
}

impl HolisticUdpMgr {
    pub fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            port_map: BTreeMap::new(),
            stats: UdpMgrStats {
                total_sockets: 0,
                active_sockets: 0,
                total_datagrams: 0,
                total_bytes: 0,
                total_drops: 0,
            },
            next_socket_id: 1,
        }
    }

    pub fn create_socket(&mut self) -> u64 {
        let id = self.next_socket_id;
        self.next_socket_id += 1;
        self.sockets.insert(id, UdpSocket::new(id));
        self.stats.total_sockets += 1;
        self.stats.active_sockets += 1;
        id
    }

    pub fn bind_socket(&mut self, socket_id: u64, port: u16) -> bool {
        if self.port_map.contains_key(&port) {
            return false;
        }
        if let Some(sock) = self.sockets.get_mut(&socket_id) {
            sock.bind(port);
            self.port_map.insert(port, socket_id);
            true
        } else {
            false
        }
    }

    pub fn close_socket(&mut self, socket_id: u64) -> bool {
        if let Some(sock) = self.sockets.get_mut(&socket_id) {
            sock.state = UdpSocketState::Closed;
            if sock.local_port > 0 {
                self.port_map.remove(&sock.local_port);
            }
            self.stats.active_sockets = self.stats.active_sockets.saturating_sub(1);
            true
        } else {
            false
        }
    }
}
