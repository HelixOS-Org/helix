// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Getsockopt (socket option querying)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SockoptLevel {
    Socket,
    Tcp,
    Udp,
    Ip,
    Ipv6,
    Sctp,
    Raw,
    Netlink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SockoptName {
    SoReuseAddr,
    SoReusePort,
    SoKeepAlive,
    SoLinger,
    SoRcvBuf,
    SoSndBuf,
    SoRcvTimeo,
    SoSndTimeo,
    SoError,
    SoType,
    SoBroadcast,
    SoTimestamp,
    SoBindToDevice,
    SoMark,
    SoPriority,
    TcpNoDelay,
    TcpCork,
    TcpKeepIdle,
    TcpKeepIntvl,
    TcpKeepCnt,
    TcpMaxSeg,
    TcpCongestion,
    TcpInfo,
    TcpFastOpen,
    TcpDeferAccept,
    TcpWindowClamp,
    IpTtl,
    IpTos,
    IpMulticastLoop,
    Ipv6UnicastHops,
    Ipv6MulticastHops,
    Ipv6V6Only,
}

#[derive(Debug, Clone)]
pub struct SockoptValue {
    pub int_val: Option<i32>,
    pub u64_val: Option<u64>,
    pub timeval_sec: Option<u64>,
    pub timeval_usec: Option<u64>,
    pub raw_len: u32,
}

impl SockoptValue {
    #[inline(always)]
    pub fn from_int(v: i32) -> Self {
        Self { int_val: Some(v), u64_val: None, timeval_sec: None, timeval_usec: None, raw_len: 4 }
    }

    #[inline(always)]
    pub fn from_u64(v: u64) -> Self {
        Self { int_val: None, u64_val: Some(v), timeval_sec: None, timeval_usec: None, raw_len: 8 }
    }

    #[inline(always)]
    pub fn from_timeval(sec: u64, usec: u64) -> Self {
        Self { int_val: None, u64_val: None, timeval_sec: Some(sec), timeval_usec: Some(usec), raw_len: 16 }
    }

    #[inline(always)]
    pub fn as_bool(&self) -> bool {
        self.int_val.map(|v| v != 0).unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
pub struct SockoptQuery {
    pub fd: u64,
    pub level: SockoptLevel,
    pub name: SockoptName,
    pub value: SockoptValue,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct GetsockoptAppStats {
    pub total_queries: u64,
    pub per_level_counts: BTreeMap<u8, u64>,
    pub error_queries: u64,
}

pub struct AppGetsockopt {
    cached_options: BTreeMap<u64, Vec<SockoptQuery>>,
    stats: GetsockoptAppStats,
}

impl AppGetsockopt {
    pub fn new() -> Self {
        Self {
            cached_options: BTreeMap::new(),
            stats: GetsockoptAppStats {
                total_queries: 0,
                per_level_counts: BTreeMap::new(),
                error_queries: 0,
            },
        }
    }

    #[inline]
    pub fn record_query(&mut self, query: SockoptQuery) {
        let level_key = query.level as u8;
        *self.stats.per_level_counts.entry(level_key).or_insert(0) += 1;
        self.stats.total_queries += 1;
        self.cached_options.entry(query.fd).or_insert_with(Vec::new).push(query);
    }

    #[inline]
    pub fn get_cached(&self, fd: u64, name: SockoptName) -> Option<&SockoptValue> {
        self.cached_options.get(&fd).and_then(|opts| {
            opts.iter().rev().find(|q| q.name == name).map(|q| &q.value)
        })
    }

    #[inline(always)]
    pub fn stats(&self) -> &GetsockoptAppStats { &self.stats }
}

// ============================================================================
// Merged from getsockopt_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SockOptLevel { Socket, Tcp, Udp, Ipv4, Ipv6 }

/// Socket option name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SockOptNameV2 { ReuseAddr, KeepAlive, NoDelay, RcvBuf, SndBuf, Linger, Error }

/// Getsockopt v2 request
#[derive(Debug, Clone)]
pub struct GetsockoptV2Request {
    pub fd: i32,
    pub level: SockOptLevel,
    pub name: SockOptNameV2,
}

impl GetsockoptV2Request {
    pub fn new(fd: i32, level: SockOptLevel, name: SockOptNameV2) -> Self { Self { fd, level, name } }
}

/// Getsockopt v2 app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct GetsockoptV2AppStats { pub total_queries: u64, pub socket_level: u64, pub tcp_level: u64, pub errors: u64 }

/// Main app getsockopt v2
#[derive(Debug)]
pub struct AppGetsockoptV2 { pub stats: GetsockoptV2AppStats }

impl AppGetsockoptV2 {
    pub fn new() -> Self { Self { stats: GetsockoptV2AppStats { total_queries: 0, socket_level: 0, tcp_level: 0, errors: 0 } } }
    #[inline]
    pub fn query(&mut self, req: &GetsockoptV2Request) -> u64 {
        self.stats.total_queries += 1;
        match req.level {
            SockOptLevel::Socket => self.stats.socket_level += 1,
            SockOptLevel::Tcp => self.stats.tcp_level += 1,
            _ => {}
        }
        0
    }
}
