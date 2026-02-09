// SPDX-License-Identifier: GPL-2.0
//! Bridge netlink_bridge — netlink protocol family bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Netlink protocol family
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlProto {
    Route,
    Firewall,
    NfLog,
    Xfrm,
    Selinux,
    Audit,
    Connector,
    Netfilter,
    Generic,
    Kobject,
    Crypto,
}

/// Netlink message type (generic)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlMsgType {
    Noop,
    Error,
    Done,
    Overrun,
    Custom(u16),
}

/// Netlink message flags
#[derive(Debug, Clone, Copy)]
pub struct NlMsgFlags {
    pub bits: u16,
}

impl NlMsgFlags {
    pub const REQUEST: u16 = 1;
    pub const MULTI: u16 = 2;
    pub const ACK: u16 = 4;
    pub const ECHO: u16 = 8;
    pub const DUMP: u16 = 0x100 | 0x200;
    pub const ROOT: u16 = 0x100;
    pub const MATCH: u16 = 0x200;
    pub const ATOMIC: u16 = 0x400;

    pub fn new(bits: u16) -> Self { Self { bits } }
    #[inline(always)]
    pub fn has(&self, flag: u16) -> bool { self.bits & flag != 0 }
    #[inline(always)]
    pub fn is_request(&self) -> bool { self.has(Self::REQUEST) }
    #[inline(always)]
    pub fn is_dump(&self) -> bool { self.has(Self::DUMP) }
}

/// Netlink message header
#[derive(Debug, Clone)]
pub struct NlMsgHeader {
    pub len: u32,
    pub msg_type: NlMsgType,
    pub flags: NlMsgFlags,
    pub seq: u32,
    pub pid: u32,
}

/// Netlink socket
#[derive(Debug)]
pub struct NlSocket {
    pub id: u64,
    pub pid: u32,
    pub protocol: NlProto,
    pub groups: u32,
    pub bound: bool,
    pub tx_msgs: u64,
    pub rx_msgs: u64,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub drops: u64,
    pub created_at: u64,
}

impl NlSocket {
    pub fn new(id: u64, pid: u32, proto: NlProto, now: u64) -> Self {
        Self {
            id, pid, protocol: proto, groups: 0, bound: false,
            tx_msgs: 0, rx_msgs: 0, tx_bytes: 0, rx_bytes: 0,
            drops: 0, created_at: now,
        }
    }

    #[inline(always)]
    pub fn bind(&mut self, groups: u32) { self.groups = groups; self.bound = true; }

    #[inline(always)]
    pub fn send(&mut self, bytes: u64) { self.tx_msgs += 1; self.tx_bytes += bytes; }
    #[inline(always)]
    pub fn recv(&mut self, bytes: u64) { self.rx_msgs += 1; self.rx_bytes += bytes; }

    #[inline]
    pub fn drop_rate(&self) -> f64 {
        let total = self.rx_msgs + self.drops;
        if total == 0 { return 0.0; }
        self.drops as f64 / total as f64
    }
}

/// Multicast group
#[derive(Debug, Clone)]
pub struct NlMcastGroup {
    pub id: u32,
    pub name_hash: u64,
    pub protocol: NlProto,
    pub subscribers: u32,
    pub messages_sent: u64,
}

/// Generic netlink family
#[derive(Debug, Clone)]
pub struct GenlFamily {
    pub id: u16,
    pub name_hash: u64,
    pub version: u8,
    pub max_attr: u16,
    pub ops_count: u16,
    pub mcast_groups: Vec<u32>,
}

/// Bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NetlinkBridgeStats {
    pub total_sockets: u32,
    pub total_families: u32,
    pub total_tx_msgs: u64,
    pub total_rx_msgs: u64,
    pub total_drops: u64,
    pub total_bytes: u64,
}

/// Main netlink bridge
#[repr(align(64))]
pub struct BridgeNetlink {
    sockets: BTreeMap<u64, NlSocket>,
    families: BTreeMap<u16, GenlFamily>,
    groups: BTreeMap<u32, NlMcastGroup>,
    next_sock_id: u64,
}

impl BridgeNetlink {
    pub fn new() -> Self {
        Self { sockets: BTreeMap::new(), families: BTreeMap::new(), groups: BTreeMap::new(), next_sock_id: 1 }
    }

    #[inline]
    pub fn open_socket(&mut self, pid: u32, proto: NlProto, now: u64) -> u64 {
        let id = self.next_sock_id;
        self.next_sock_id += 1;
        self.sockets.insert(id, NlSocket::new(id, pid, proto, now));
        id
    }

    #[inline(always)]
    pub fn close_socket(&mut self, id: u64) -> bool { self.sockets.remove(&id).is_some() }

    #[inline(always)]
    pub fn send_msg(&mut self, sock_id: u64, bytes: u64) -> bool {
        if let Some(s) = self.sockets.get_mut(&sock_id) { s.send(bytes); true } else { false }
    }

    #[inline]
    pub fn register_family(&mut self, id: u16, version: u8) {
        self.families.insert(id, GenlFamily {
            id, name_hash: id as u64, version, max_attr: 0,
            ops_count: 0, mcast_groups: Vec::new(),
        });
    }

    #[inline]
    pub fn stats(&self) -> NetlinkBridgeStats {
        NetlinkBridgeStats {
            total_sockets: self.sockets.len() as u32,
            total_families: self.families.len() as u32,
            total_tx_msgs: self.sockets.values().map(|s| s.tx_msgs).sum(),
            total_rx_msgs: self.sockets.values().map(|s| s.rx_msgs).sum(),
            total_drops: self.sockets.values().map(|s| s.drops).sum(),
            total_bytes: self.sockets.values().map(|s| s.tx_bytes + s.rx_bytes).sum(),
        }
    }
}

// ============================================================================
// Merged from netlink_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlV2Protocol {
    Route, Unused, Usersock, Firewall, SockDiag, Nflog,
    Xfrm, Selinux, Iscsi, Audit, FibLookup, Connector,
    Netfilter, Ip6Fw, Dnrtmsg, KobjectUevent, Generic,
    Scsitransport, Ecryptfs, Rdma, Crypto, Smc,
}

/// Netlink v2 message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlV2MsgType {
    Noop, Error, Done, Overrun,
    NewLink, DelLink, GetLink,
    NewAddr, DelAddr, GetAddr,
    NewRoute, DelRoute, GetRoute,
    NewNeigh, DelNeigh, GetNeigh,
    Custom(u16),
}

/// Netlink v2 message
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NlV2Message {
    pub msg_type: NlV2MsgType,
    pub flags: u16,
    pub seq: u32,
    pub pid: u32,
    pub payload_len: u32,
    pub payload_hash: u64,
}

/// Netlink v2 socket
#[derive(Debug)]
pub struct NlV2Socket {
    pub id: u64,
    pub protocol: NlV2Protocol,
    pub pid: u32,
    pub groups: u64,
    pub recv_queue: VecDeque<NlV2Message>,
    pub send_count: u64,
    pub recv_count: u64,
    pub drop_count: u64,
    pub max_recv: usize,
}

impl NlV2Socket {
    pub fn new(id: u64, protocol: NlV2Protocol, pid: u32) -> Self {
        Self { id, protocol, pid, groups: 0, recv_queue: VecDeque::new(), send_count: 0, recv_count: 0, drop_count: 0, max_recv: 256 }
    }

    #[inline(always)]
    pub fn enqueue(&mut self, msg: NlV2Message) {
        if self.recv_queue.len() >= self.max_recv { self.drop_count += 1; return; }
        self.recv_queue.push_back(msg);
    }

    #[inline(always)]
    pub fn dequeue(&mut self) -> Option<NlV2Message> {
        if self.recv_queue.is_empty() { None } else { self.recv_count += 1; self.recv_queue.pop_front() }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NetlinkV2BridgeStats {
    pub total_sockets: u32,
    pub total_sent: u64,
    pub total_recv: u64,
    pub total_dropped: u64,
    pub queued_messages: u32,
}

/// Main netlink v2 bridge
#[repr(align(64))]
pub struct BridgeNetlinkV2 {
    sockets: BTreeMap<u64, NlV2Socket>,
    next_id: u64,
}

impl BridgeNetlinkV2 {
    pub fn new() -> Self { Self { sockets: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create_socket(&mut self, protocol: NlV2Protocol, pid: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.sockets.insert(id, NlV2Socket::new(id, protocol, pid));
        id
    }

    #[inline(always)]
    pub fn close(&mut self, id: u64) { self.sockets.remove(&id); }

    #[inline(always)]
    pub fn send(&mut self, id: u64, msg: NlV2Message) {
        if let Some(sock) = self.sockets.get_mut(&id) { sock.send_count += 1; sock.enqueue(msg); }
    }

    #[inline]
    pub fn stats(&self) -> NetlinkV2BridgeStats {
        let sent: u64 = self.sockets.values().map(|s| s.send_count).sum();
        let recv: u64 = self.sockets.values().map(|s| s.recv_count).sum();
        let dropped: u64 = self.sockets.values().map(|s| s.drop_count).sum();
        let queued: u32 = self.sockets.values().map(|s| s.recv_queue.len() as u32).sum();
        NetlinkV2BridgeStats { total_sockets: self.sockets.len() as u32, total_sent: sent, total_recv: recv, total_dropped: dropped, queued_messages: queued }
    }
}

// ============================================================================
// Merged from netlink_v3_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetlinkV3Proto {
    Route,
    Unused,
    Usersock,
    Firewall,
    SockDiag,
    Nflog,
    Xfrm,
    Selinux,
    Iscsi,
    Audit,
    Connector,
    Netfilter,
    Ip6Fw,
    Kobject,
    Generic,
    Crypto,
}

/// Netlink v3 message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlMsgTypeV3 {
    Done,
    Error,
    Overrun,
    GetLink,
    NewLink,
    DelLink,
    GetAddr,
    NewAddr,
    DelAddr,
    GetRoute,
    NewRoute,
    DelRoute,
    Custom(u16),
}

/// Netlink v3 socket
#[derive(Debug)]
pub struct NetlinkV3Socket {
    pub pid: u32,
    pub groups: u32,
    pub protocol: NetlinkV3Proto,
    pub rx_msgs: u64,
    pub tx_msgs: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub drops: u64,
}

impl NetlinkV3Socket {
    pub fn new(pid: u32, proto: NetlinkV3Proto) -> Self {
        Self { pid, groups: 0, protocol: proto, rx_msgs: 0, tx_msgs: 0, rx_bytes: 0, tx_bytes: 0, drops: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NetlinkV3BridgeStats {
    pub total_sockets: u32,
    pub total_rx_msgs: u64,
    pub total_tx_msgs: u64,
    pub total_drops: u64,
}

/// Main bridge netlink v3
#[repr(align(64))]
pub struct BridgeNetlinkV3 {
    sockets: BTreeMap<u32, NetlinkV3Socket>,
}

impl BridgeNetlinkV3 {
    pub fn new() -> Self { Self { sockets: BTreeMap::new() } }

    #[inline]
    pub fn bind(&mut self, pid: u32, proto: NetlinkV3Proto, groups: u32) {
        let mut s = NetlinkV3Socket::new(pid, proto);
        s.groups = groups;
        self.sockets.insert(pid, s);
    }

    #[inline(always)]
    pub fn send(&mut self, pid: u32, bytes: u64) {
        if let Some(s) = self.sockets.get_mut(&pid) { s.tx_msgs += 1; s.tx_bytes += bytes; }
    }

    #[inline(always)]
    pub fn receive(&mut self, pid: u32, bytes: u64) {
        if let Some(s) = self.sockets.get_mut(&pid) { s.rx_msgs += 1; s.rx_bytes += bytes; }
    }

    #[inline(always)]
    pub fn close(&mut self, pid: u32) { self.sockets.remove(&pid); }

    #[inline]
    pub fn stats(&self) -> NetlinkV3BridgeStats {
        let rx: u64 = self.sockets.values().map(|s| s.rx_msgs).sum();
        let tx: u64 = self.sockets.values().map(|s| s.tx_msgs).sum();
        let drops: u64 = self.sockets.values().map(|s| s.drops).sum();
        NetlinkV3BridgeStats { total_sockets: self.sockets.len() as u32, total_rx_msgs: rx, total_tx_msgs: tx, total_drops: drops }
    }
}

// ============================================================================
// Merged from netlink_v4_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetlinkV4MsgType { Route, Link, Address, Neighbor, Rule, Qdisc }

/// Netlink v4 record
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NetlinkV4Record {
    pub msg_type: NetlinkV4MsgType,
    pub family: u16,
    pub flags: u32,
    pub seq: u32,
    pub payload_len: u32,
}

impl NetlinkV4Record {
    pub fn new(msg_type: NetlinkV4MsgType) -> Self { Self { msg_type, family: 0, flags: 0, seq: 0, payload_len: 0 } }
}

/// Netlink v4 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NetlinkV4BridgeStats { pub total_msgs: u64, pub route_msgs: u64, pub link_msgs: u64, pub total_bytes: u64 }

/// Main bridge netlink v4
#[derive(Debug)]
pub struct BridgeNetlinkV4 { pub stats: NetlinkV4BridgeStats }

impl BridgeNetlinkV4 {
    pub fn new() -> Self { Self { stats: NetlinkV4BridgeStats { total_msgs: 0, route_msgs: 0, link_msgs: 0, total_bytes: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &NetlinkV4Record) {
        self.stats.total_msgs += 1;
        match rec.msg_type {
            NetlinkV4MsgType::Route | NetlinkV4MsgType::Rule => self.stats.route_msgs += 1,
            NetlinkV4MsgType::Link | NetlinkV4MsgType::Address => self.stats.link_msgs += 1,
            _ => {}
        }
        self.stats.total_bytes += rec.payload_len as u64;
    }
}
