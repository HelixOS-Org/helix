// SPDX-License-Identifier: GPL-2.0
//! Holistic dma_engine — DMA channel and transfer management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// DMA transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    MemToMem,
    MemToDev,
    DevToMem,
    DevToDev,
}

/// DMA transfer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaTransferType {
    Single,
    Scatter,
    Cyclic,
    Interleaved,
    Memset,
}

/// DMA channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaChannelState {
    Idle,
    Active,
    Paused,
    Error,
    Disabled,
}

/// DMA transfer priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DmaPriority {
    Realtime = 0,
    High = 1,
    Normal = 2,
    Low = 3,
}

/// Physical address range for DMA
#[derive(Debug, Clone, Copy)]
pub struct DmaRegion {
    pub phys_addr: u64,
    pub size: u64,
    pub direction: DmaDirection,
}

impl DmaRegion {
    pub fn new(addr: u64, size: u64, dir: DmaDirection) -> Self {
        Self { phys_addr: addr, size, direction: dir }
    }

    pub fn end(&self) -> u64 { self.phys_addr + self.size }

    pub fn overlaps(&self, other: &DmaRegion) -> bool {
        self.phys_addr < other.end() && other.phys_addr < self.end()
    }
}

/// Scatter-gather entry
#[derive(Debug, Clone, Copy)]
pub struct SgEntry {
    pub addr: u64,
    pub len: u32,
    pub is_last: bool,
}

/// DMA transfer descriptor
#[derive(Debug, Clone)]
pub struct DmaTransfer {
    pub id: u64,
    pub channel_id: u32,
    pub transfer_type: DmaTransferType,
    pub direction: DmaDirection,
    pub priority: DmaPriority,
    pub src: DmaRegion,
    pub dst: DmaRegion,
    pub sg_list: Vec<SgEntry>,
    pub bytes_transferred: u64,
    pub submitted_at: u64,
    pub started_at: u64,
    pub completed_at: u64,
    pub error_count: u32,
}

impl DmaTransfer {
    pub fn new(id: u64, ch: u32, dir: DmaDirection, src: DmaRegion, dst: DmaRegion, now: u64) -> Self {
        Self {
            id, channel_id: ch, transfer_type: DmaTransferType::Single,
            direction: dir, priority: DmaPriority::Normal,
            src, dst, sg_list: Vec::new(),
            bytes_transferred: 0, submitted_at: now, started_at: 0, completed_at: 0,
            error_count: 0,
        }
    }

    pub fn start(&mut self, now: u64) { self.started_at = now; }

    pub fn complete(&mut self, bytes: u64, now: u64) {
        self.bytes_transferred = bytes;
        self.completed_at = now;
    }

    pub fn latency_ns(&self) -> u64 {
        if self.completed_at > 0 { self.completed_at.saturating_sub(self.submitted_at) } else { 0 }
    }

    pub fn throughput_mbps(&self) -> f64 {
        let lat = self.latency_ns();
        if lat == 0 { return 0.0; }
        let bytes_per_sec = self.bytes_transferred as f64 * 1_000_000_000.0 / lat as f64;
        bytes_per_sec / (1024.0 * 1024.0)
    }
}

/// DMA channel
#[derive(Debug)]
pub struct DmaChannel {
    pub id: u32,
    pub state: DmaChannelState,
    pub max_burst: u32,
    pub addr_width: u32,
    pub supported_dirs: u8,
    pub active_transfer: Option<u64>,
    pub pending: Vec<DmaTransfer>,
    pub total_transfers: u64,
    pub total_bytes: u64,
    pub total_errors: u64,
}

impl DmaChannel {
    pub fn new(id: u32) -> Self {
        Self {
            id, state: DmaChannelState::Idle, max_burst: 256,
            addr_width: 64, supported_dirs: 0xFF,
            active_transfer: None, pending: Vec::new(),
            total_transfers: 0, total_bytes: 0, total_errors: 0,
        }
    }

    pub fn submit(&mut self, transfer: DmaTransfer) -> bool {
        if self.state == DmaChannelState::Disabled || self.state == DmaChannelState::Error {
            return false;
        }
        self.pending.push(transfer);
        true
    }

    pub fn start_next(&mut self, now: u64) -> Option<u64> {
        if self.active_transfer.is_some() { return None; }
        if let Some(mut t) = self.pending.pop() {
            let id = t.id;
            t.start(now);
            self.active_transfer = Some(id);
            self.state = DmaChannelState::Active;
            self.pending.push(t); // keep for tracking
            Some(id)
        } else { None }
    }

    pub fn complete_active(&mut self, bytes: u64, now: u64) {
        self.active_transfer = None;
        self.state = DmaChannelState::Idle;
        self.total_transfers += 1;
        self.total_bytes += bytes;
    }

    pub fn utilization(&self) -> f64 {
        if self.state == DmaChannelState::Active { 1.0 } else { 0.0 }
    }
}

/// DMA engine stats
#[derive(Debug, Clone)]
pub struct DmaEngineStats {
    pub total_channels: u32,
    pub active_channels: u32,
    pub total_transfers: u64,
    pub total_bytes: u64,
    pub total_errors: u64,
    pub avg_throughput_mbps: f64,
}

/// Main DMA engine
pub struct HolisticDmaEngine {
    channels: BTreeMap<u32, DmaChannel>,
    completed: Vec<DmaTransfer>,
    next_transfer_id: u64,
    max_completed: usize,
}

impl HolisticDmaEngine {
    pub fn new() -> Self {
        Self { channels: BTreeMap::new(), completed: Vec::new(), next_transfer_id: 1, max_completed: 4096 }
    }

    pub fn add_channel(&mut self, id: u32) {
        self.channels.entry(id).or_insert_with(|| DmaChannel::new(id));
    }

    pub fn submit(&mut self, ch_id: u32, dir: DmaDirection, src: DmaRegion, dst: DmaRegion, now: u64) -> Option<u64> {
        let tid = self.next_transfer_id;
        self.next_transfer_id += 1;
        let transfer = DmaTransfer::new(tid, ch_id, dir, src, dst, now);
        self.channels.get_mut(&ch_id)?.submit(transfer).then_some(tid)
    }

    pub fn stats(&self) -> DmaEngineStats {
        let active = self.channels.values().filter(|c| c.state == DmaChannelState::Active).count() as u32;
        let tot_transfers: u64 = self.channels.values().map(|c| c.total_transfers).sum();
        let tot_bytes: u64 = self.channels.values().map(|c| c.total_bytes).sum();
        let tot_errors: u64 = self.channels.values().map(|c| c.total_errors).sum();
        let avg_tp = if self.completed.is_empty() { 0.0 } else {
            self.completed.iter().map(|t| t.throughput_mbps()).sum::<f64>() / self.completed.len() as f64
        };
        DmaEngineStats {
            total_channels: self.channels.len() as u32,
            active_channels: active, total_transfers: tot_transfers,
            total_bytes: tot_bytes, total_errors: tot_errors,
            avg_throughput_mbps: avg_tp,
        }
    }
}
