// SPDX-License-Identifier: GPL-2.0
//! Holistic network namespace — network namespace isolation with veth pairs and routing

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Network namespace state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetNsState {
    Creating,
    Active,
    Migrating,
    Destroying,
    Destroyed,
}

/// Veth pair state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VethState {
    Creating,
    Up,
    Down,
    Error,
}

/// Network namespace capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetNsCap {
    RawSocket,
    BindToDevice,
    NetAdmin,
    BpfAttach,
    IpForward,
    Multicast,
    BridgeCreate,
    VlanCreate,
}

/// Veth pair
#[derive(Debug, Clone)]
pub struct VethPair {
    pub veth_id: u32,
    pub state: VethState,
    pub host_ns_id: u32,
    pub guest_ns_id: u32,
    pub host_name: String,
    pub guest_name: String,
    pub mtu: u32,
    pub tx_packets: u64,
    pub rx_packets: u64,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub tx_drops: u64,
    pub rx_drops: u64,
}

impl VethPair {
    pub fn new(veth_id: u32, host_ns_id: u32, guest_ns_id: u32) -> Self {
        Self {
            veth_id,
            state: VethState::Creating,
            host_ns_id,
            guest_ns_id,
            host_name: String::new(),
            guest_name: String::new(),
            mtu: 1500,
            tx_packets: 0,
            rx_packets: 0,
            tx_bytes: 0,
            rx_bytes: 0,
            tx_drops: 0,
            rx_drops: 0,
        }
    }

    #[inline(always)]
    pub fn bring_up(&mut self) {
        self.state = VethState::Up;
    }

    #[inline(always)]
    pub fn transmit(&mut self, bytes: u64) {
        self.tx_packets += 1;
        self.tx_bytes += bytes;
    }

    #[inline(always)]
    pub fn receive(&mut self, bytes: u64) {
        self.rx_packets += 1;
        self.rx_bytes += bytes;
    }

    #[inline(always)]
    pub fn total_throughput(&self) -> u64 {
        self.tx_bytes + self.rx_bytes
    }

    #[inline]
    pub fn drop_rate(&self) -> f64 {
        let total = self.tx_packets + self.rx_packets + self.tx_drops + self.rx_drops;
        if total == 0 {
            return 0.0;
        }
        (self.tx_drops + self.rx_drops) as f64 / total as f64
    }
}

/// Network namespace
#[derive(Debug, Clone)]
pub struct NetNamespace {
    pub ns_id: u32,
    pub state: NetNsState,
    pub name: String,
    pub name_hash: u64,
    pub parent_ns: u32,
    pub interfaces: Vec<u32>,
    pub veth_pairs: Vec<u32>,
    pub routes: u32,
    pub iptables_rules: u32,
    pub capabilities: u64,
    pub created_ns: u64,
    pub process_count: u32,
    pub socket_count: u32,
}

impl NetNamespace {
    pub fn new(ns_id: u32, name: &str) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            ns_id,
            state: NetNsState::Creating,
            name: String::from(name),
            name_hash: h,
            parent_ns: 0,
            interfaces: Vec::new(),
            veth_pairs: Vec::new(),
            routes: 0,
            iptables_rules: 0,
            capabilities: 0,
            created_ns: 0,
            process_count: 0,
            socket_count: 0,
        }
    }

    #[inline(always)]
    pub fn activate(&mut self, ts_ns: u64) {
        self.state = NetNsState::Active;
        self.created_ns = ts_ns;
    }

    #[inline(always)]
    pub fn add_interface(&mut self, iface_id: u32) {
        self.interfaces.push(iface_id);
    }

    #[inline(always)]
    pub fn add_veth(&mut self, veth_id: u32) {
        self.veth_pairs.push(veth_id);
    }

    #[inline(always)]
    pub fn grant_cap(&mut self, cap: NetNsCap) {
        self.capabilities |= 1u64 << (cap as u64);
    }

    #[inline(always)]
    pub fn has_cap(&self, cap: NetNsCap) -> bool {
        self.capabilities & (1u64 << (cap as u64)) != 0
    }

    #[inline(always)]
    pub fn attach_process(&mut self) {
        self.process_count += 1;
    }

    #[inline(always)]
    pub fn detach_process(&mut self) {
        self.process_count = self.process_count.saturating_sub(1);
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.process_count == 0 && self.socket_count == 0
    }

    #[inline(always)]
    pub fn destroy(&mut self) {
        self.state = NetNsState::Destroying;
    }
}

/// Net namespace stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NetNsStats {
    pub total_namespaces: u64,
    pub active_namespaces: u64,
    pub total_veth_pairs: u64,
    pub total_migrations: u64,
    pub total_processes: u64,
}

/// Main holistic network namespace manager
#[derive(Debug)]
pub struct HolisticNetNs {
    pub namespaces: BTreeMap<u32, NetNamespace>,
    pub veth_pairs: BTreeMap<u32, VethPair>,
    pub stats: NetNsStats,
    pub next_ns_id: u32,
    pub next_veth_id: u32,
    pub default_ns_id: u32,
}

impl HolisticNetNs {
    pub fn new() -> Self {
        let mut mgr = Self {
            namespaces: BTreeMap::new(),
            veth_pairs: BTreeMap::new(),
            stats: NetNsStats {
                total_namespaces: 1,
                active_namespaces: 1,
                total_veth_pairs: 0,
                total_migrations: 0,
                total_processes: 0,
            },
            next_ns_id: 2,
            next_veth_id: 1,
            default_ns_id: 1,
        };
        let mut default = NetNamespace::new(1, "default");
        default.state = NetNsState::Active;
        mgr.namespaces.insert(1, default);
        mgr
    }

    #[inline]
    pub fn create_namespace(&mut self, name: &str, ts_ns: u64) -> u32 {
        let id = self.next_ns_id;
        self.next_ns_id += 1;
        let mut ns = NetNamespace::new(id, name);
        ns.parent_ns = self.default_ns_id;
        ns.activate(ts_ns);
        self.namespaces.insert(id, ns);
        self.stats.total_namespaces += 1;
        self.stats.active_namespaces += 1;
        id
    }

    pub fn create_veth_pair(&mut self, host_ns: u32, guest_ns: u32) -> Option<u32> {
        if !self.namespaces.contains_key(&host_ns) || !self.namespaces.contains_key(&guest_ns) {
            return None;
        }
        let id = self.next_veth_id;
        self.next_veth_id += 1;
        let mut veth = VethPair::new(id, host_ns, guest_ns);
        veth.bring_up();
        self.veth_pairs.insert(id, veth);
        if let Some(ns) = self.namespaces.get_mut(&host_ns) {
            ns.add_veth(id);
        }
        if let Some(ns) = self.namespaces.get_mut(&guest_ns) {
            ns.add_veth(id);
        }
        self.stats.total_veth_pairs += 1;
        Some(id)
    }

    pub fn destroy_namespace(&mut self, ns_id: u32) -> bool {
        if ns_id == self.default_ns_id {
            return false;
        }
        if let Some(ns) = self.namespaces.get_mut(&ns_id) {
            ns.destroy();
            self.stats.active_namespaces = self.stats.active_namespaces.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn migrate_interface(&mut self, iface_id: u32, from_ns: u32, to_ns: u32) -> bool {
        let removed = if let Some(ns) = self.namespaces.get_mut(&from_ns) {
            if let Some(pos) = ns.interfaces.iter().position(|&i| i == iface_id) {
                ns.interfaces.remove(pos);
                true
            } else {
                false
            }
        } else {
            false
        };
        if removed {
            if let Some(ns) = self.namespaces.get_mut(&to_ns) {
                ns.add_interface(iface_id);
                self.stats.total_migrations += 1;
                return true;
            }
        }
        false
    }
}
