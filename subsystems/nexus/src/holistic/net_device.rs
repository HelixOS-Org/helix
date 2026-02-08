// SPDX-License-Identifier: GPL-2.0
//! Holistic network device — NIC management, queues, and offload features

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Network device state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetDevState {
    Down,
    Up,
    Running,
    Dormant,
    NotPresent,
    LowerLayerDown,
    Testing,
}

/// Network device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetDevType {
    Ethernet,
    Loopback,
    Bridge,
    Bond,
    Vlan,
    Vxlan,
    Macvlan,
    Tun,
    Tap,
    Veth,
    WireGuard,
    Dummy,
}

/// Offload feature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffloadFeature {
    TxChecksumIpv4,
    TxChecksumIpv6,
    RxChecksum,
    Tso,
    Gso,
    Gro,
    Lro,
    ScatterGather,
    TxUdpTunnelSegmentation,
    RxHashIndirection,
    XdpAttach,
    HwTimestamp,
}

/// NIC queue
#[derive(Debug, Clone)]
pub struct NetDevQueue {
    pub queue_id: u16,
    pub is_tx: bool,
    pub ring_size: u32,
    pub ring_used: u32,
    pub packets: u64,
    pub bytes: u64,
    pub drops: u64,
    pub xdp_redirect: u64,
    pub cpu_affinity: u32,
    pub napi_weight: u32,
}

impl NetDevQueue {
    pub fn new(queue_id: u16, is_tx: bool, ring_size: u32) -> Self {
        Self {
            queue_id,
            is_tx,
            ring_size,
            ring_used: 0,
            packets: 0,
            bytes: 0,
            drops: 0,
            xdp_redirect: 0,
            cpu_affinity: queue_id as u32,
            napi_weight: 64,
        }
    }

    pub fn process_packet(&mut self, pkt_bytes: u64) -> bool {
        if self.ring_used >= self.ring_size {
            self.drops += 1;
            return false;
        }
        self.ring_used += 1;
        self.packets += 1;
        self.bytes += pkt_bytes;
        true
    }

    pub fn complete(&mut self, count: u32) {
        self.ring_used = self.ring_used.saturating_sub(count);
    }

    pub fn utilization_pct(&self) -> f64 {
        if self.ring_size == 0 {
            return 0.0;
        }
        (self.ring_used as f64 / self.ring_size as f64) * 100.0
    }

    pub fn avg_pkt_size(&self) -> u64 {
        if self.packets == 0 { 0 } else { self.bytes / self.packets }
    }

    pub fn drop_rate(&self) -> f64 {
        let total = self.packets + self.drops;
        if total == 0 { 0.0 } else { self.drops as f64 / total as f64 }
    }
}

/// Network device
#[derive(Debug, Clone)]
pub struct NetDevice {
    pub dev_id: u32,
    pub name: String,
    pub dev_type: NetDevType,
    pub state: NetDevState,
    pub mtu: u32,
    pub mac_hash: u64,
    pub speed_mbps: u32,
    pub duplex_full: bool,
    pub tx_queues: Vec<NetDevQueue>,
    pub rx_queues: Vec<NetDevQueue>,
    pub features: u64,
    pub promisc_count: u32,
    pub allmulti_count: u32,
    pub carrier_changes: u64,
    pub link_up_ns: u64,
}

impl NetDevice {
    pub fn new(dev_id: u32, name: String, dev_type: NetDevType) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            dev_id,
            name,
            dev_type,
            state: NetDevState::Down,
            mtu: 1500,
            mac_hash: h,
            speed_mbps: 1000,
            duplex_full: true,
            tx_queues: Vec::new(),
            rx_queues: Vec::new(),
            features: 0,
            promisc_count: 0,
            allmulti_count: 0,
            carrier_changes: 0,
            link_up_ns: 0,
        }
    }

    pub fn bring_up(&mut self, ts_ns: u64) {
        self.state = NetDevState::Up;
        self.link_up_ns = ts_ns;
    }

    pub fn bring_down(&mut self) {
        self.state = NetDevState::Down;
    }

    pub fn add_tx_queue(&mut self, ring_size: u32) {
        let qid = self.tx_queues.len() as u16;
        self.tx_queues.push(NetDevQueue::new(qid, true, ring_size));
    }

    pub fn add_rx_queue(&mut self, ring_size: u32) {
        let qid = self.rx_queues.len() as u16;
        self.rx_queues.push(NetDevQueue::new(qid, false, ring_size));
    }

    pub fn enable_feature(&mut self, feature: OffloadFeature) {
        self.features |= 1u64 << (feature as u64);
    }

    pub fn has_feature(&self, feature: OffloadFeature) -> bool {
        self.features & (1u64 << (feature as u64)) != 0
    }

    pub fn total_tx_bytes(&self) -> u64 {
        self.tx_queues.iter().map(|q| q.bytes).sum()
    }

    pub fn total_rx_bytes(&self) -> u64 {
        self.rx_queues.iter().map(|q| q.bytes).sum()
    }

    pub fn total_drops(&self) -> u64 {
        self.tx_queues.iter().map(|q| q.drops).sum::<u64>()
            + self.rx_queues.iter().map(|q| q.drops).sum::<u64>()
    }
}

/// Net device stats
#[derive(Debug, Clone)]
pub struct NetDeviceStats {
    pub total_devices: u64,
    pub active_devices: u64,
    pub total_tx_bytes: u64,
    pub total_rx_bytes: u64,
    pub total_drops: u64,
}

/// Main holistic net device manager
#[derive(Debug)]
pub struct HolisticNetDevice {
    pub devices: BTreeMap<u32, NetDevice>,
    pub stats: NetDeviceStats,
    pub next_dev_id: u32,
}

impl HolisticNetDevice {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            stats: NetDeviceStats {
                total_devices: 0,
                active_devices: 0,
                total_tx_bytes: 0,
                total_rx_bytes: 0,
                total_drops: 0,
            },
            next_dev_id: 1,
        }
    }

    pub fn register_device(&mut self, name: String, dev_type: NetDevType, num_queues: u16) -> u32 {
        let id = self.next_dev_id;
        self.next_dev_id += 1;
        let mut dev = NetDevice::new(id, name, dev_type);
        for _ in 0..num_queues {
            dev.add_tx_queue(256);
            dev.add_rx_queue(256);
        }
        self.devices.insert(id, dev);
        self.stats.total_devices += 1;
        id
    }

    pub fn bring_up(&mut self, dev_id: u32, ts_ns: u64) -> bool {
        if let Some(dev) = self.devices.get_mut(&dev_id) {
            dev.bring_up(ts_ns);
            self.stats.active_devices += 1;
            true
        } else {
            false
        }
    }

    pub fn aggregate_throughput_bytes(&self) -> u64 {
        self.devices.values()
            .filter(|d| d.state == NetDevState::Up || d.state == NetDevState::Running)
            .map(|d| d.total_tx_bytes() + d.total_rx_bytes())
            .sum()
    }
}
