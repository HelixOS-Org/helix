// SPDX-License-Identifier: GPL-2.0
//! Coop packet — cooperative packet processing with shared buffer pools

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Packet coop protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopPktProto {
    Ethernet,
    Ipv4,
    Ipv6,
    Tcp,
    Udp,
    Icmp,
    Arp,
    Vlan,
    Mpls,
}

/// Packet buffer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PktBufState {
    Free,
    Allocated,
    InUse,
    Shared,
    Cloned,
    Freed,
}

/// Shared packet buffer
#[derive(Debug, Clone)]
pub struct SharedPktBuf {
    pub buf_id: u64,
    pub state: PktBufState,
    pub data_len: u32,
    pub headroom: u32,
    pub tailroom: u32,
    pub ref_count: u32,
    pub protocol: CoopPktProto,
    pub hash: u64,
}

impl SharedPktBuf {
    pub fn new(buf_id: u64, capacity: u32) -> Self {
        Self {
            buf_id, state: PktBufState::Free, data_len: 0,
            headroom: 128, tailroom: capacity - 128, ref_count: 0,
            protocol: CoopPktProto::Ethernet, hash: 0,
        }
    }

    pub fn allocate(&mut self, data_len: u32) -> bool {
        if self.state != PktBufState::Free { return false; }
        self.state = PktBufState::Allocated;
        self.data_len = data_len;
        self.ref_count = 1;
        true
    }

    pub fn share(&mut self) { self.ref_count += 1; self.state = PktBufState::Shared; }
    pub fn release(&mut self) -> bool {
        self.ref_count = self.ref_count.saturating_sub(1);
        if self.ref_count == 0 { self.state = PktBufState::Free; true } else { false }
    }

    pub fn clone_buf(&mut self) { self.ref_count += 1; self.state = PktBufState::Cloned; }

    pub fn compute_hash(&mut self, data: &[u8]) {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in data { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        self.hash = h;
    }
}

/// Shared buffer pool
#[derive(Debug, Clone)]
pub struct SharedBufPool {
    pub pool_id: u64,
    pub buffers: Vec<SharedPktBuf>,
    pub capacity: u32,
    pub allocated: u32,
    pub shared_count: u32,
    pub total_allocs: u64,
    pub total_frees: u64,
}

impl SharedBufPool {
    pub fn new(pool_id: u64, capacity: u32) -> Self {
        let mut buffers = Vec::new();
        for i in 0..capacity {
            buffers.push(SharedPktBuf::new(i as u64, 2048));
        }
        Self { pool_id, buffers, capacity, allocated: 0, shared_count: 0, total_allocs: 0, total_frees: 0 }
    }

    pub fn alloc(&mut self, data_len: u32) -> Option<u64> {
        for buf in &mut self.buffers {
            if buf.state == PktBufState::Free {
                buf.allocate(data_len);
                self.allocated += 1;
                self.total_allocs += 1;
                return Some(buf.buf_id);
            }
        }
        None
    }

    pub fn free(&mut self, buf_id: u64) -> bool {
        if let Some(buf) = self.buffers.iter_mut().find(|b| b.buf_id == buf_id) {
            if buf.release() { self.allocated -= 1; self.total_frees += 1; true }
            else { false }
        } else { false }
    }

    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { 0.0 } else { self.allocated as f64 / self.capacity as f64 }
    }
}

/// Coop packet stats
#[derive(Debug, Clone)]
pub struct CoopPktStats {
    pub total_pools: u64,
    pub total_allocs: u64,
    pub total_shares: u64,
    pub total_bytes: u64,
}

/// Main coop packet manager
#[derive(Debug)]
pub struct CoopPacket {
    pub pools: BTreeMap<u64, SharedBufPool>,
    pub stats: CoopPktStats,
}

impl CoopPacket {
    pub fn new() -> Self {
        Self {
            pools: BTreeMap::new(),
            stats: CoopPktStats { total_pools: 0, total_allocs: 0, total_shares: 0, total_bytes: 0 },
        }
    }

    pub fn create_pool(&mut self, pool_id: u64, capacity: u32) {
        self.pools.insert(pool_id, SharedBufPool::new(pool_id, capacity));
        self.stats.total_pools += 1;
    }

    pub fn alloc_from_pool(&mut self, pool_id: u64, data_len: u32) -> Option<u64> {
        if let Some(pool) = self.pools.get_mut(&pool_id) {
            let result = pool.alloc(data_len);
            if result.is_some() {
                self.stats.total_allocs += 1;
                self.stats.total_bytes += data_len as u64;
            }
            result
        } else { None }
    }
}
