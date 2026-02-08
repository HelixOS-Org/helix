//! # Holistic DMA Manager
//!
//! DMA buffer and transfer management with holistic awareness:
//! - DMA zone tracking (DMA, DMA32, Normal)
//! - Scatter-gather list management
//! - Bounce buffer allocation for non-DMA-able memory
//! - IOMMU mapping tracking
//! - DMA coherency management
//! - Transfer statistics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// DMA zone
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaZone {
    Dma,
    Dma32,
    Normal,
    HighMem,
    Movable,
}

/// DMA direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaDirection {
    ToDevice,
    FromDevice,
    Bidirectional,
    None,
}

/// DMA coherency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaCoherency {
    Coherent,
    NonCoherent,
    WriteBack,
    WriteThrough,
    Uncached,
}

/// Scatter-gather entry
#[derive(Debug, Clone)]
pub struct SgEntry {
    pub phys_addr: u64,
    pub length: u32,
    pub dma_addr: u64,
    pub offset: u32,
}

/// Scatter-gather list
#[derive(Debug, Clone)]
pub struct SgList {
    pub id: u64,
    pub entries: Vec<SgEntry>,
    pub total_bytes: u64,
    pub direction: DmaDirection,
    pub mapped: bool,
}

impl SgList {
    pub fn new(id: u64, dir: DmaDirection) -> Self {
        Self { id, entries: Vec::new(), total_bytes: 0, direction: dir, mapped: false }
    }

    pub fn add_entry(&mut self, phys: u64, len: u32, offset: u32) {
        self.entries.push(SgEntry { phys_addr: phys, length: len, dma_addr: 0, offset });
        self.total_bytes += len as u64;
    }

    pub fn map_dma(&mut self, base_dma: u64) {
        let mut addr = base_dma;
        for e in &mut self.entries {
            e.dma_addr = addr;
            addr += e.length as u64;
        }
        self.mapped = true;
    }

    pub fn entry_count(&self) -> usize { self.entries.len() }
}

/// DMA buffer
#[derive(Debug, Clone)]
pub struct DmaBuffer {
    pub id: u64,
    pub phys_addr: u64,
    pub virt_addr: u64,
    pub dma_addr: u64,
    pub size: u64,
    pub zone: DmaZone,
    pub coherency: DmaCoherency,
    pub direction: DmaDirection,
    pub owner: u64,
    pub alloc_ts: u64,
    pub is_bounce: bool,
}

impl DmaBuffer {
    pub fn new(id: u64, phys: u64, virt: u64, dma: u64, size: u64, zone: DmaZone, coh: DmaCoherency, owner: u64, ts: u64) -> Self {
        Self { id, phys_addr: phys, virt_addr: virt, dma_addr: dma, size, zone, coherency: coh, direction: DmaDirection::None, owner, alloc_ts: ts, is_bounce: false }
    }
}

/// IOMMU mapping
#[derive(Debug, Clone)]
pub struct IommuMapping {
    pub iova: u64,
    pub phys_addr: u64,
    pub size: u64,
    pub domain_id: u64,
    pub prot_read: bool,
    pub prot_write: bool,
}

/// DMA transfer record
#[derive(Debug, Clone)]
pub struct DmaTransfer {
    pub id: u64,
    pub buffer_id: u64,
    pub direction: DmaDirection,
    pub bytes: u64,
    pub start_ts: u64,
    pub end_ts: u64,
    pub device_id: u64,
    pub status: DmaTransferStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmaTransferStatus {
    Queued,
    InProgress,
    Complete,
    Error,
    Timeout,
}

/// Per-zone DMA state
#[derive(Debug, Clone)]
pub struct DmaZoneState {
    pub zone: DmaZone,
    pub total_pages: u64,
    pub free_pages: u64,
    pub allocated_pages: u64,
    pub bounce_pages: u64,
}

impl DmaZoneState {
    pub fn new(zone: DmaZone, total: u64) -> Self {
        Self { zone, total_pages: total, free_pages: total, allocated_pages: 0, bounce_pages: 0 }
    }

    pub fn allocate(&mut self, pages: u64) -> bool {
        if self.free_pages >= pages { self.free_pages -= pages; self.allocated_pages += pages; true } else { false }
    }

    pub fn free(&mut self, pages: u64) { self.allocated_pages = self.allocated_pages.saturating_sub(pages); self.free_pages += pages; }
    pub fn usage(&self) -> f64 { if self.total_pages == 0 { 0.0 } else { self.allocated_pages as f64 / self.total_pages as f64 } }
}

/// DMA stats
#[derive(Debug, Clone, Default)]
pub struct DmaStats {
    pub buffers: usize,
    pub sg_lists: usize,
    pub total_allocated: u64,
    pub transfers: u64,
    pub bytes_transferred: u64,
    pub bounce_buffers: u64,
    pub iommu_mappings: usize,
    pub errors: u64,
}

/// Holistic DMA manager
pub struct HolisticDmaMgr {
    buffers: BTreeMap<u64, DmaBuffer>,
    sg_lists: BTreeMap<u64, SgList>,
    iommu: Vec<IommuMapping>,
    zones: BTreeMap<u8, DmaZoneState>,
    transfers: Vec<DmaTransfer>,
    stats: DmaStats,
    next_buf_id: u64,
    next_sg_id: u64,
    next_xfer_id: u64,
}

impl HolisticDmaMgr {
    pub fn new() -> Self {
        Self {
            buffers: BTreeMap::new(), sg_lists: BTreeMap::new(),
            iommu: Vec::new(), zones: BTreeMap::new(),
            transfers: Vec::new(), stats: DmaStats::default(),
            next_buf_id: 1, next_sg_id: 1, next_xfer_id: 1,
        }
    }

    pub fn init_zone(&mut self, zone: DmaZone, total_pages: u64) {
        self.zones.insert(zone as u8, DmaZoneState::new(zone, total_pages));
    }

    pub fn alloc_buffer(&mut self, phys: u64, virt: u64, dma: u64, size: u64, zone: DmaZone, coh: DmaCoherency, owner: u64, ts: u64) -> Option<u64> {
        let pages = (size + 4095) / 4096;
        if let Some(z) = self.zones.get_mut(&(zone as u8)) { if !z.allocate(pages) { return None; } }
        let id = self.next_buf_id; self.next_buf_id += 1;
        self.buffers.insert(id, DmaBuffer::new(id, phys, virt, dma, size, zone, coh, owner, ts));
        self.stats.total_allocated += size;
        Some(id)
    }

    pub fn free_buffer(&mut self, id: u64) {
        if let Some(buf) = self.buffers.remove(&id) {
            let pages = (buf.size + 4095) / 4096;
            if let Some(z) = self.zones.get_mut(&(buf.zone as u8)) { z.free(pages); }
            self.stats.total_allocated = self.stats.total_allocated.saturating_sub(buf.size);
        }
    }

    pub fn alloc_bounce(&mut self, phys: u64, virt: u64, dma: u64, size: u64, zone: DmaZone, owner: u64, ts: u64) -> Option<u64> {
        let id = self.alloc_buffer(phys, virt, dma, size, zone, DmaCoherency::Coherent, owner, ts)?;
        if let Some(buf) = self.buffers.get_mut(&id) { buf.is_bounce = true; }
        if let Some(z) = self.zones.get_mut(&(zone as u8)) { z.bounce_pages += (size + 4095) / 4096; }
        self.stats.bounce_buffers += 1;
        Some(id)
    }

    pub fn create_sg(&mut self, dir: DmaDirection) -> u64 {
        let id = self.next_sg_id; self.next_sg_id += 1;
        self.sg_lists.insert(id, SgList::new(id, dir));
        id
    }

    pub fn add_sg_entry(&mut self, sg_id: u64, phys: u64, len: u32, offset: u32) {
        if let Some(sg) = self.sg_lists.get_mut(&sg_id) { sg.add_entry(phys, len, offset); }
    }

    pub fn map_sg(&mut self, sg_id: u64, base_dma: u64) {
        if let Some(sg) = self.sg_lists.get_mut(&sg_id) { sg.map_dma(base_dma); }
    }

    pub fn add_iommu_mapping(&mut self, iova: u64, phys: u64, size: u64, domain: u64, read: bool, write: bool) {
        self.iommu.push(IommuMapping { iova, phys_addr: phys, size, domain_id: domain, prot_read: read, prot_write: write });
    }

    pub fn start_transfer(&mut self, buf_id: u64, dir: DmaDirection, device: u64, ts: u64) -> u64 {
        let id = self.next_xfer_id; self.next_xfer_id += 1;
        let bytes = self.buffers.get(&buf_id).map(|b| b.size).unwrap_or(0);
        self.transfers.push(DmaTransfer { id, buffer_id: buf_id, direction: dir, bytes, start_ts: ts, end_ts: 0, device_id: device, status: DmaTransferStatus::InProgress });
        self.stats.transfers += 1;
        id
    }

    pub fn complete_transfer(&mut self, xfer_id: u64, ts: u64) {
        if let Some(x) = self.transfers.iter_mut().find(|x| x.id == xfer_id) {
            x.status = DmaTransferStatus::Complete;
            x.end_ts = ts;
            self.stats.bytes_transferred += x.bytes;
        }
    }

    pub fn recompute(&mut self) {
        self.stats.buffers = self.buffers.len();
        self.stats.sg_lists = self.sg_lists.len();
        self.stats.iommu_mappings = self.iommu.len();
        self.stats.errors = self.transfers.iter().filter(|x| x.status == DmaTransferStatus::Error).count() as u64;
    }

    pub fn buffer(&self, id: u64) -> Option<&DmaBuffer> { self.buffers.get(&id) }
    pub fn sg(&self, id: u64) -> Option<&SgList> { self.sg_lists.get(&id) }
    pub fn stats(&self) -> &DmaStats { &self.stats }
    pub fn zone(&self, z: DmaZone) -> Option<&DmaZoneState> { self.zones.get(&(z as u8)) }
}
