//! # Holistic IOMMU Manager
//!
//! IOMMU (Input/Output Memory Management Unit) management:
//! - DMA remapping and translation
//! - IOVA (I/O Virtual Address) space management
//! - Device isolation via IOMMU domains
//! - Address translation caching (IOTLB)
//! - Passthrough vs translated mode
//! - Fault logging and interrupt remapping

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// IOMMU domain type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IommuDomainType {
    /// Direct identity mapping (passthrough)
    Identity,
    /// DMA address translation enabled
    Translated,
    /// Blocked — no access allowed
    Blocked,
    /// Nested translation (two-level)
    Nested,
}

/// IOVA region
#[derive(Debug, Clone)]
pub struct IovaRegion {
    pub iova_start: u64,
    pub iova_end: u64,
    pub phys_addr: u64,
    pub size_bytes: u64,
    pub readable: bool,
    pub writable: bool,
    pub cacheable: bool,
}

impl IovaRegion {
    #[inline(always)]
    pub fn contains(&self, iova: u64) -> bool {
        iova >= self.iova_start && iova < self.iova_end
    }

    #[inline]
    pub fn translate(&self, iova: u64) -> Option<u64> {
        if self.contains(iova) {
            Some(self.phys_addr + (iova - self.iova_start))
        } else {
            None
        }
    }
}

/// IOMMU domain
#[derive(Debug, Clone)]
pub struct IommuDomain {
    pub domain_id: u32,
    pub domain_type: IommuDomainType,
    pub mappings: Vec<IovaRegion>,
    pub assigned_devices: Vec<u32>,
    pub total_mapped_bytes: u64,
    pub translation_faults: u64,
    pub dma_ops: u64,
}

impl IommuDomain {
    pub fn new(id: u32, dtype: IommuDomainType) -> Self {
        Self {
            domain_id: id, domain_type: dtype, mappings: Vec::new(),
            assigned_devices: Vec::new(), total_mapped_bytes: 0,
            translation_faults: 0, dma_ops: 0,
        }
    }

    #[inline]
    pub fn map(&mut self, iova: u64, phys: u64, size: u64, read: bool, write: bool) {
        self.mappings.push(IovaRegion {
            iova_start: iova, iova_end: iova + size, phys_addr: phys,
            size_bytes: size, readable: read, writable: write, cacheable: true,
        });
        self.total_mapped_bytes += size;
    }

    #[inline]
    pub fn unmap(&mut self, iova: u64) -> Option<IovaRegion> {
        if let Some(idx) = self.mappings.iter().position(|m| m.iova_start == iova) {
            let region = self.mappings.remove(idx);
            self.total_mapped_bytes = self.total_mapped_bytes.saturating_sub(region.size_bytes);
            Some(region)
        } else {
            None
        }
    }

    pub fn translate(&mut self, iova: u64) -> Option<u64> {
        self.dma_ops += 1;
        match self.domain_type {
            IommuDomainType::Identity => Some(iova),
            IommuDomainType::Blocked => { self.translation_faults += 1; None }
            IommuDomainType::Translated | IommuDomainType::Nested => {
                for region in &self.mappings {
                    if let Some(pa) = region.translate(iova) { return Some(pa); }
                }
                self.translation_faults += 1;
                None
            }
        }
    }

    #[inline(always)]
    pub fn attach_device(&mut self, dev_id: u32) {
        if !self.assigned_devices.contains(&dev_id) { self.assigned_devices.push(dev_id); }
    }

    #[inline(always)]
    pub fn detach_device(&mut self, dev_id: u32) {
        self.assigned_devices.retain(|&d| d != dev_id);
    }

    #[inline(always)]
    pub fn fault_rate(&self) -> f64 {
        if self.dma_ops == 0 { return 0.0; }
        self.translation_faults as f64 / self.dma_ops as f64
    }
}

/// IOTLB cache entry
#[derive(Debug, Clone)]
pub struct IotlbEntry {
    pub iova: u64,
    pub phys: u64,
    pub domain_id: u32,
    pub hits: u64,
    pub last_access_ts: u64,
}

/// IOTLB cache
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct IotlbCache {
    entries: BTreeMap<u64, IotlbEntry>,
    max_entries: usize,
    hits: u64,
    misses: u64,
}

impl IotlbCache {
    pub fn new(max: usize) -> Self {
        Self { entries: BTreeMap::new(), max_entries: max, hits: 0, misses: 0 }
    }

    #[inline]
    pub fn lookup(&mut self, iova: u64, ts: u64) -> Option<u64> {
        if let Some(entry) = self.entries.get_mut(&iova) {
            entry.hits += 1;
            entry.last_access_ts = ts;
            self.hits += 1;
            Some(entry.phys)
        } else {
            self.misses += 1;
            None
        }
    }

    pub fn insert(&mut self, iova: u64, phys: u64, domain_id: u32, ts: u64) {
        if self.entries.len() >= self.max_entries {
            // Evict LRU
            let lru_key = self.entries.iter()
                .min_by_key(|(_, e)| e.last_access_ts)
                .map(|(&k, _)| k);
            if let Some(k) = lru_key { self.entries.remove(&k); }
        }
        self.entries.insert(iova, IotlbEntry {
            iova, phys, domain_id, hits: 0, last_access_ts: ts,
        });
    }

    #[inline(always)]
    pub fn invalidate_domain(&mut self, domain_id: u32) {
        self.entries.retain(|_, e| e.domain_id != domain_id);
    }

    #[inline(always)]
    pub fn flush(&mut self) { self.entries.clear(); }

    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { return 0.0; }
        self.hits as f64 / total as f64
    }
}

/// DMA fault record
#[derive(Debug, Clone)]
pub struct DmaFault {
    pub fault_id: u64,
    pub domain_id: u32,
    pub device_id: u32,
    pub iova: u64,
    pub is_write: bool,
    pub timestamp_ns: u64,
}

/// IOMMU stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct IommuMgrStats {
    pub total_domains: usize,
    pub total_devices: usize,
    pub total_mapped_bytes: u64,
    pub total_dma_ops: u64,
    pub total_faults: u64,
    pub iotlb_hit_rate: f64,
    pub iotlb_entries: usize,
    pub fault_log_size: usize,
}

/// Holistic IOMMU manager
pub struct HolisticIommuMgr {
    domains: BTreeMap<u32, IommuDomain>,
    device_to_domain: ArrayMap<u32, 32>,
    iotlb: IotlbCache,
    fault_log: VecDeque<DmaFault>,
    next_domain_id: u32,
    next_fault_id: u64,
    max_fault_log: usize,
    stats: IommuMgrStats,
}

impl HolisticIommuMgr {
    pub fn new(iotlb_size: usize) -> Self {
        Self {
            domains: BTreeMap::new(), device_to_domain: BTreeMap::new(),
            iotlb: IotlbCache::new(iotlb_size), fault_log: VecDeque::new(),
            next_domain_id: 1, next_fault_id: 1, max_fault_log: 1000,
            stats: IommuMgrStats::default(),
        }
    }

    #[inline]
    pub fn create_domain(&mut self, dtype: IommuDomainType) -> u32 {
        let id = self.next_domain_id; self.next_domain_id += 1;
        self.domains.insert(id, IommuDomain::new(id, dtype));
        id
    }

    #[inline]
    pub fn attach(&mut self, dev_id: u32, domain_id: u32) {
        if let Some(dom) = self.domains.get_mut(&domain_id) {
            dom.attach_device(dev_id);
            self.device_to_domain.insert(dev_id, domain_id);
        }
    }

    #[inline]
    pub fn detach(&mut self, dev_id: u32) {
        if let Some(&dom_id) = self.device_to_domain.try_get(dev_id as usize) {
            if let Some(dom) = self.domains.get_mut(&dom_id) { dom.detach_device(dev_id); }
            self.device_to_domain.remove(&dev_id);
        }
    }

    #[inline(always)]
    pub fn map_iova(&mut self, domain_id: u32, iova: u64, phys: u64, size: u64, read: bool, write: bool) {
        if let Some(dom) = self.domains.get_mut(&domain_id) { dom.map(iova, phys, size, read, write); }
    }

    #[inline]
    pub fn unmap_iova(&mut self, domain_id: u32, iova: u64) {
        if let Some(dom) = self.domains.get_mut(&domain_id) {
            dom.unmap(iova);
            self.iotlb.invalidate_domain(domain_id);
        }
    }

    pub fn translate(&mut self, dev_id: u32, iova: u64, is_write: bool, ts: u64) -> Option<u64> {
        // Try IOTLB first
        if let Some(phys) = self.iotlb.lookup(iova, ts) { return Some(phys); }
        let &dom_id = self.device_to_domain.try_get(dev_id as usize)?;
        let dom = self.domains.get_mut(&dom_id)?;
        match dom.translate(iova) {
            Some(phys) => { self.iotlb.insert(iova, phys, dom_id, ts); Some(phys) }
            None => {
                let fid = self.next_fault_id; self.next_fault_id += 1;
                self.fault_log.push_back(DmaFault {
                    fault_id: fid, domain_id: dom_id, device_id: dev_id,
                    iova, is_write, timestamp_ns: ts,
                });
                if self.fault_log.len() > self.max_fault_log { self.fault_log.pop_front(); }
                None
            }
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_domains = self.domains.len();
        self.stats.total_devices = self.device_to_domain.len();
        self.stats.total_mapped_bytes = self.domains.values().map(|d| d.total_mapped_bytes).sum();
        self.stats.total_dma_ops = self.domains.values().map(|d| d.dma_ops).sum();
        self.stats.total_faults = self.domains.values().map(|d| d.translation_faults).sum();
        self.stats.iotlb_hit_rate = self.iotlb.hit_rate();
        self.stats.iotlb_entries = self.iotlb.entries.len();
        self.stats.fault_log_size = self.fault_log.len();
    }

    #[inline(always)]
    pub fn stats(&self) -> &IommuMgrStats { &self.stats }
}
