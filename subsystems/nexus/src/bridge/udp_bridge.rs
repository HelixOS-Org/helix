// SPDX-License-Identifier: GPL-2.0
//! Bridge UDP — UDP datagram bridging

extern crate alloc;

/// UDP bridge event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpBridgeEvent { Send, Recv, Bind, Drop, Truncate }

/// UDP bridge record
#[derive(Debug, Clone)]
pub struct UdpBridgeRecord {
    pub event: UdpBridgeEvent,
    pub port: u16,
    pub bytes: u64,
    pub datagrams: u32,
}

impl UdpBridgeRecord {
    pub fn new(event: UdpBridgeEvent) -> Self { Self { event, port: 0, bytes: 0, datagrams: 0 } }
}

/// UDP bridge stats
#[derive(Debug, Clone)]
pub struct UdpBridgeStats { pub total_events: u64, pub sent: u64, pub received: u64, pub dropped: u64 }

/// Main bridge UDP
#[derive(Debug)]
pub struct BridgeUdp { pub stats: UdpBridgeStats }

impl BridgeUdp {
    pub fn new() -> Self { Self { stats: UdpBridgeStats { total_events: 0, sent: 0, received: 0, dropped: 0 } } }
    pub fn record(&mut self, rec: &UdpBridgeRecord) {
        self.stats.total_events += 1;
        match rec.event {
            UdpBridgeEvent::Send => self.stats.sent += rec.datagrams as u64,
            UdpBridgeEvent::Recv => self.stats.received += rec.datagrams as u64,
            UdpBridgeEvent::Drop | UdpBridgeEvent::Truncate => self.stats.dropped += 1,
            _ => {}
        }
    }
}
