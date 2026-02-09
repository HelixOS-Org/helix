// SPDX-License-Identifier: GPL-2.0
//! Bridge TCP — TCP connection state bridging

extern crate alloc;

/// TCP bridge state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpBridgeState { SynSent, SynRecv, Established, FinWait, CloseWait, TimeWait, Closed }

/// TCP bridge record
#[derive(Debug, Clone)]
pub struct TcpBridgeRecord {
    pub state: TcpBridgeState,
    pub src_port: u16,
    pub dst_port: u16,
    pub bytes_sent: u64,
    pub bytes_recv: u64,
    pub rtt_us: u32,
}

impl TcpBridgeRecord {
    pub fn new(state: TcpBridgeState) -> Self { Self { state, src_port: 0, dst_port: 0, bytes_sent: 0, bytes_recv: 0, rtt_us: 0 } }
}

/// TCP bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TcpBridgeStats { pub total_events: u64, pub established: u64, pub closed: u64, pub total_bytes: u64 }

/// Main bridge TCP
#[derive(Debug)]
pub struct BridgeTcp { pub stats: TcpBridgeStats }

impl BridgeTcp {
    pub fn new() -> Self { Self { stats: TcpBridgeStats { total_events: 0, established: 0, closed: 0, total_bytes: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &TcpBridgeRecord) {
        self.stats.total_events += 1;
        match rec.state {
            TcpBridgeState::Established => self.stats.established += 1,
            TcpBridgeState::Closed | TcpBridgeState::TimeWait => self.stats.closed += 1,
            _ => {}
        }
        self.stats.total_bytes += rec.bytes_sent + rec.bytes_recv;
    }
}
