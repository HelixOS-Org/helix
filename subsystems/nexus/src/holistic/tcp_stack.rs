// SPDX-License-Identifier: GPL-2.0
//! Holistic TCP stack management — full TCP protocol state machine with congestion control

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// TCP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

/// Congestion control algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpCongestionAlgo {
    Reno,
    NewReno,
    Cubic,
    Bbr,
    BbrV2,
    Vegas,
    Westwood,
    Dctcp,
    Hybla,
    Illinois,
}

/// TCP timer kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpTimerKind {
    Retransmit,
    DelayedAck,
    Keepalive,
    TimeWait,
    PersistWindow,
    Linger,
}

/// TCP congestion window state
#[derive(Debug, Clone)]
pub struct TcpCwndState {
    pub cwnd: u32,
    pub ssthresh: u32,
    pub srtt_us: u64,
    pub rttvar_us: u64,
    pub rto_ms: u32,
    pub mss: u16,
    pub in_recovery: bool,
    pub loss_count: u64,
    pub fast_retransmit_count: u64,
}

impl TcpCwndState {
    pub fn new(mss: u16) -> Self {
        Self {
            cwnd: 10 * mss as u32,
            ssthresh: 65535,
            srtt_us: 0,
            rttvar_us: 0,
            rto_ms: 1000,
            mss,
            in_recovery: false,
            loss_count: 0,
            fast_retransmit_count: 0,
        }
    }

    pub fn update_rtt(&mut self, sample_us: u64) {
        if self.srtt_us == 0 {
            self.srtt_us = sample_us;
            self.rttvar_us = sample_us / 2;
        } else {
            let diff = if sample_us > self.srtt_us {
                sample_us - self.srtt_us
            } else {
                self.srtt_us - sample_us
            };
            self.rttvar_us = (3 * self.rttvar_us + diff) / 4;
            self.srtt_us = (7 * self.srtt_us + sample_us) / 8;
        }
        self.rto_ms = ((self.srtt_us + 4 * self.rttvar_us) / 1000) as u32;
        if self.rto_ms < 200 {
            self.rto_ms = 200;
        }
        if self.rto_ms > 120_000 {
            self.rto_ms = 120_000;
        }
    }

    pub fn on_ack(&mut self) {
        if self.cwnd < self.ssthresh {
            self.cwnd += self.mss as u32;
        } else {
            self.cwnd += (self.mss as u32 * self.mss as u32) / self.cwnd;
        }
    }

    pub fn on_loss(&mut self) {
        self.ssthresh = if self.cwnd / 2 > 2 * self.mss as u32 {
            self.cwnd / 2
        } else {
            2 * self.mss as u32
        };
        self.cwnd = self.ssthresh;
        self.in_recovery = true;
        self.loss_count += 1;
    }

    pub fn bandwidth_estimate_bps(&self) -> u64 {
        if self.srtt_us == 0 {
            return 0;
        }
        (self.cwnd as u64 * 8 * 1_000_000) / self.srtt_us
    }
}

/// TCP connection
#[derive(Debug, Clone)]
pub struct TcpConnection {
    pub conn_id: u64,
    pub state: TcpState,
    pub algo: TcpCongestionAlgo,
    pub cwnd_state: TcpCwndState,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub segments_sent: u64,
    pub segments_received: u64,
    pub retransmits: u64,
    pub dup_acks: u32,
    pub window_scale: u8,
    pub sack_enabled: bool,
    pub timestamps_enabled: bool,
}

impl TcpConnection {
    pub fn new(conn_id: u64, algo: TcpCongestionAlgo) -> Self {
        Self {
            conn_id,
            state: TcpState::Closed,
            algo,
            cwnd_state: TcpCwndState::new(1460),
            bytes_sent: 0,
            bytes_received: 0,
            segments_sent: 0,
            segments_received: 0,
            retransmits: 0,
            dup_acks: 0,
            window_scale: 7,
            sack_enabled: true,
            timestamps_enabled: true,
        }
    }

    pub fn connect(&mut self) {
        self.state = TcpState::SynSent;
        self.segments_sent += 1;
    }

    pub fn syn_ack_received(&mut self) {
        if self.state == TcpState::SynSent {
            self.state = TcpState::Established;
            self.segments_received += 1;
            self.segments_sent += 1;
        }
    }

    pub fn send_data(&mut self, bytes: u64) {
        if self.state == TcpState::Established {
            self.bytes_sent += bytes;
            let segs = (bytes + self.cwnd_state.mss as u64 - 1) / self.cwnd_state.mss as u64;
            self.segments_sent += segs;
        }
    }

    pub fn receive_data(&mut self, bytes: u64) {
        if self.state == TcpState::Established {
            self.bytes_received += bytes;
            self.segments_received += 1;
            self.cwnd_state.on_ack();
        }
    }

    pub fn retransmit_rate(&self) -> f64 {
        if self.segments_sent == 0 {
            return 0.0;
        }
        self.retransmits as f64 / self.segments_sent as f64
    }

    pub fn close(&mut self) {
        match self.state {
            TcpState::Established => self.state = TcpState::FinWait1,
            TcpState::CloseWait => self.state = TcpState::LastAck,
            _ => {}
        }
        self.segments_sent += 1;
    }
}

/// TCP stack stats
#[derive(Debug, Clone)]
pub struct TcpStackStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub total_retransmits: u64,
}

/// Main holistic TCP stack manager
#[derive(Debug)]
pub struct HolisticTcpStack {
    pub connections: BTreeMap<u64, TcpConnection>,
    pub stats: TcpStackStats,
    pub default_algo: TcpCongestionAlgo,
    pub max_connections: u32,
    pub listen_backlog: BTreeMap<u64, Vec<u64>>,
    pub next_conn_id: u64,
}

impl HolisticTcpStack {
    pub fn new(max_connections: u32) -> Self {
        Self {
            connections: BTreeMap::new(),
            stats: TcpStackStats {
                total_connections: 0,
                active_connections: 0,
                total_bytes_sent: 0,
                total_bytes_received: 0,
                total_retransmits: 0,
            },
            default_algo: TcpCongestionAlgo::Cubic,
            max_connections,
            listen_backlog: BTreeMap::new(),
            next_conn_id: 1,
        }
    }

    pub fn create_connection(&mut self) -> Option<u64> {
        if self.stats.active_connections >= self.max_connections as u64 {
            return None;
        }
        let id = self.next_conn_id;
        self.next_conn_id += 1;
        let conn = TcpConnection::new(id, self.default_algo);
        self.connections.insert(id, conn);
        self.stats.total_connections += 1;
        self.stats.active_connections += 1;
        Some(id)
    }

    pub fn close_connection(&mut self, conn_id: u64) -> bool {
        if let Some(conn) = self.connections.get_mut(&conn_id) {
            conn.close();
            self.stats.active_connections = self.stats.active_connections.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn avg_bandwidth_bps(&self) -> u64 {
        if self.connections.is_empty() {
            return 0;
        }
        let total: u64 = self.connections.values().map(|c| c.cwnd_state.bandwidth_estimate_bps()).sum();
        total / self.connections.len() as u64
    }
}
