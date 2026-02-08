//! # Bridge Netlink Proxy
//!
//! Bridges netlink socket operations between kernel subsystems:
//! - Netlink family registration and multicast groups
//! - Message routing and buffering
//! - Generic netlink (genl) support
//! - Notification subscription
//! - Flow control and backpressure

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Netlink protocol family
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlFamily {
    Route,
    Firewall,
    Sock,
    XfrmSa,
    Audit,
    Kobject,
    Generic,
    Connector,
    CryptoApi,
    Scsitransport,
}

/// Netlink message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NlMsgType {
    Request,
    Response,
    Notification,
    Error,
    Done,
    Multipart,
    Dump,
    Ack,
}

/// Netlink message flags
#[derive(Debug, Clone, Copy)]
pub struct NlMsgFlags {
    pub request: bool,
    pub multi: bool,
    pub ack: bool,
    pub echo: bool,
    pub dump: bool,
    pub root: bool,
    pub match_flag: bool,
    pub atomic: bool,
}

impl NlMsgFlags {
    pub fn new() -> Self {
        Self { request: false, multi: false, ack: false, echo: false, dump: false, root: false, match_flag: false, atomic: false }
    }
    pub fn request() -> Self { Self { request: true, ..Self::new() } }
    pub fn dump_request() -> Self { Self { request: true, dump: true, root: true, match_flag: true, ..Self::new() } }
}

/// Netlink message
#[derive(Debug, Clone)]
pub struct NlMessage {
    pub seq: u64,
    pub family: NlFamily,
    pub msg_type: NlMsgType,
    pub flags: NlMsgFlags,
    pub src_port: u32,
    pub dst_port: u32,
    pub payload: Vec<u8>,
    pub ts: u64,
}

/// Netlink socket
#[derive(Debug, Clone)]
pub struct NlSocket {
    pub port_id: u32,
    pub family: NlFamily,
    pub groups: Vec<u32>,
    pub recv_buf_size: u32,
    pub send_buf_size: u32,
    pub recv_queue: Vec<NlMessage>,
    pub pending_acks: u32,
    pub drop_count: u64,
    pub msg_count: u64,
}

impl NlSocket {
    pub fn new(port: u32, family: NlFamily) -> Self {
        Self {
            port_id: port, family, groups: Vec::new(),
            recv_buf_size: 65536, send_buf_size: 65536,
            recv_queue: Vec::new(), pending_acks: 0,
            drop_count: 0, msg_count: 0,
        }
    }

    pub fn join_group(&mut self, group: u32) { if !self.groups.contains(&group) { self.groups.push(group); } }
    pub fn leave_group(&mut self, group: u32) { self.groups.retain(|&g| g != group); }

    pub fn enqueue(&mut self, msg: NlMessage) -> bool {
        let total_bytes: usize = self.recv_queue.iter().map(|m| m.payload.len()).sum();
        if total_bytes + msg.payload.len() > self.recv_buf_size as usize { self.drop_count += 1; return false; }
        self.recv_queue.push(msg);
        self.msg_count += 1;
        true
    }

    pub fn dequeue(&mut self) -> Option<NlMessage> { if self.recv_queue.is_empty() { None } else { Some(self.recv_queue.remove(0)) } }
    pub fn queue_len(&self) -> usize { self.recv_queue.len() }
}

/// Multicast group
#[derive(Debug, Clone)]
pub struct NlMcastGroup {
    pub id: u32,
    pub name: String,
    pub family: NlFamily,
    pub subscribers: Vec<u32>,
}

/// Generic netlink family
#[derive(Debug, Clone)]
pub struct GenlFamily {
    pub id: u16,
    pub name: String,
    pub version: u8,
    pub max_attr: u16,
    pub ops: Vec<GenlOp>,
    pub mcast_groups: Vec<u32>,
}

/// Generic netlink operation
#[derive(Debug, Clone)]
pub struct GenlOp {
    pub cmd: u8,
    pub name: String,
    pub flags: u32,
}

/// Netlink proxy stats
#[derive(Debug, Clone, Default)]
pub struct NetlinkProxyStats {
    pub total_sockets: usize,
    pub total_messages: u64,
    pub total_drops: u64,
    pub total_mcast_groups: usize,
    pub total_genl_families: usize,
    pub queued_messages: usize,
}

/// Bridge netlink proxy
pub struct BridgeNetlinkProxy {
    sockets: BTreeMap<u32, NlSocket>,
    mcast_groups: BTreeMap<u32, NlMcastGroup>,
    genl_families: BTreeMap<u16, GenlFamily>,
    stats: NetlinkProxyStats,
    next_group: u32,
    next_genl_id: u16,
    msg_seq: u64,
}

impl BridgeNetlinkProxy {
    pub fn new() -> Self {
        Self { sockets: BTreeMap::new(), mcast_groups: BTreeMap::new(), genl_families: BTreeMap::new(), stats: NetlinkProxyStats::default(), next_group: 1, next_genl_id: 128, msg_seq: 0 }
    }

    pub fn create_socket(&mut self, port: u32, family: NlFamily) {
        self.sockets.insert(port, NlSocket::new(port, family));
    }

    pub fn destroy_socket(&mut self, port: u32) {
        self.sockets.remove(&port);
        for g in self.mcast_groups.values_mut() { g.subscribers.retain(|&s| s != port); }
    }

    pub fn send_message(&mut self, from: u32, to: u32, payload: Vec<u8>, ts: u64) -> bool {
        self.msg_seq += 1;
        let family = self.sockets.get(&from).map(|s| s.family).unwrap_or(NlFamily::Generic);
        let msg = NlMessage { seq: self.msg_seq, family, msg_type: NlMsgType::Request, flags: NlMsgFlags::request(), src_port: from, dst_port: to, payload, ts };
        if let Some(s) = self.sockets.get_mut(&to) { s.enqueue(msg) } else { false }
    }

    pub fn multicast(&mut self, group_id: u32, payload: Vec<u8>, ts: u64) {
        let subs: Vec<u32> = self.mcast_groups.get(&group_id).map(|g| g.subscribers.clone()).unwrap_or_default();
        let family = self.mcast_groups.get(&group_id).map(|g| g.family).unwrap_or(NlFamily::Generic);
        for port in subs {
            self.msg_seq += 1;
            let msg = NlMessage { seq: self.msg_seq, family, msg_type: NlMsgType::Notification, flags: NlMsgFlags::new(), src_port: 0, dst_port: port, payload: payload.clone(), ts };
            if let Some(s) = self.sockets.get_mut(&port) { s.enqueue(msg); }
        }
    }

    pub fn register_mcast_group(&mut self, name: String, family: NlFamily) -> u32 {
        let id = self.next_group; self.next_group += 1;
        self.mcast_groups.insert(id, NlMcastGroup { id, name, family, subscribers: Vec::new() });
        id
    }

    pub fn subscribe(&mut self, port: u32, group: u32) {
        if let Some(s) = self.sockets.get_mut(&port) { s.join_group(group); }
        if let Some(g) = self.mcast_groups.get_mut(&group) { if !g.subscribers.contains(&port) { g.subscribers.push(port); } }
    }

    pub fn register_genl_family(&mut self, name: String, version: u8) -> u16 {
        let id = self.next_genl_id; self.next_genl_id += 1;
        self.genl_families.insert(id, GenlFamily { id, name, version, max_attr: 0, ops: Vec::new(), mcast_groups: Vec::new() });
        id
    }

    pub fn recompute(&mut self) {
        self.stats.total_sockets = self.sockets.len();
        self.stats.total_messages = self.sockets.values().map(|s| s.msg_count).sum();
        self.stats.total_drops = self.sockets.values().map(|s| s.drop_count).sum();
        self.stats.total_mcast_groups = self.mcast_groups.len();
        self.stats.total_genl_families = self.genl_families.len();
        self.stats.queued_messages = self.sockets.values().map(|s| s.queue_len()).sum();
    }

    pub fn socket(&self, port: u32) -> Option<&NlSocket> { self.sockets.get(&port) }
    pub fn stats(&self) -> &NetlinkProxyStats { &self.stats }
}
