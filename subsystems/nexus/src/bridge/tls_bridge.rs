// SPDX-License-Identifier: GPL-2.0
//! Bridge tls_bridge — kernel TLS offload bridge.

extern crate alloc;

use alloc::collections::BTreeMap;

/// TLS version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

/// TLS cipher
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsCipher {
    AesGcm128,
    AesGcm256,
    AesCcm128,
    Chacha20Poly1305,
    Sm4Gcm,
    Sm4Ccm,
}

/// TLS direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsDirection {
    Tx,
    Rx,
    Both,
}

/// TLS connection
#[derive(Debug)]
#[repr(align(64))]
pub struct TlsConnection {
    pub fd: u64,
    pub version: TlsVersion,
    pub cipher: TlsCipher,
    pub direction: TlsDirection,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_records: u64,
    pub rx_records: u64,
    pub hw_offload: bool,
    pub established_at: u64,
}

impl TlsConnection {
    pub fn new(fd: u64, ver: TlsVersion, cipher: TlsCipher, dir: TlsDirection, now: u64) -> Self {
        Self { fd, version: ver, cipher, direction: dir, tx_bytes: 0, rx_bytes: 0, tx_records: 0, rx_records: 0, hw_offload: false, established_at: now }
    }

    #[inline(always)]
    pub fn send(&mut self, bytes: u64) { self.tx_bytes += bytes; self.tx_records += 1; }
    #[inline(always)]
    pub fn recv(&mut self, bytes: u64) { self.rx_bytes += bytes; self.rx_records += 1; }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TlsBridgeStats {
    pub total_connections: u32,
    pub hw_offloaded: u32,
    pub total_tx_bytes: u64,
    pub total_rx_bytes: u64,
    pub tls13_connections: u32,
}

/// Main bridge TLS
#[repr(align(64))]
pub struct BridgeTls {
    connections: BTreeMap<u64, TlsConnection>,
}

impl BridgeTls {
    pub fn new() -> Self { Self { connections: BTreeMap::new() } }

    #[inline(always)]
    pub fn establish(&mut self, fd: u64, ver: TlsVersion, cipher: TlsCipher, dir: TlsDirection, now: u64) {
        self.connections.insert(fd, TlsConnection::new(fd, ver, cipher, dir, now));
    }

    #[inline(always)]
    pub fn close(&mut self, fd: u64) { self.connections.remove(&fd); }

    #[inline(always)]
    pub fn send(&mut self, fd: u64, bytes: u64) {
        if let Some(c) = self.connections.get_mut(&fd) { c.send(bytes); }
    }

    #[inline(always)]
    pub fn recv(&mut self, fd: u64, bytes: u64) {
        if let Some(c) = self.connections.get_mut(&fd) { c.recv(bytes); }
    }

    #[inline]
    pub fn stats(&self) -> TlsBridgeStats {
        let hw = self.connections.values().filter(|c| c.hw_offload).count() as u32;
        let tx: u64 = self.connections.values().map(|c| c.tx_bytes).sum();
        let rx: u64 = self.connections.values().map(|c| c.rx_bytes).sum();
        let tls13 = self.connections.values().filter(|c| c.version == TlsVersion::Tls13).count() as u32;
        TlsBridgeStats { total_connections: self.connections.len() as u32, hw_offloaded: hw, total_tx_bytes: tx, total_rx_bytes: rx, tls13_connections: tls13 }
    }
}
