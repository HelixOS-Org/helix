// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Mmap (memory mapping bridge)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Mmap protection flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMmapProt {
    None,
    Read,
    ReadWrite,
    ReadExec,
    ReadWriteExec,
}

/// Mmap flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMmapFlag {
    Private,
    Shared,
    Anonymous,
    Fixed,
    Stack,
    Growsdown,
}

/// Mmap region
#[derive(Debug, Clone)]
pub struct BridgeMmapRegion {
    pub addr: u64,
    pub length: u64,
    pub prot: BridgeMmapProt,
    pub flags: BridgeMmapFlag,
    pub fd: Option<u64>,
    pub offset: u64,
}

/// Mmap stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeMmapStats {
    pub total_maps: u64,
    pub anonymous: u64,
    pub file_backed: u64,
    pub shared: u64,
    pub total_mapped_bytes: u64,
    pub avg_region_size: u64,
}

/// Manager for mmap bridging
#[repr(align(64))]
pub struct BridgeMmapManager {
    regions: BTreeMap<u64, BridgeMmapRegion>,
    next_addr: u64,
    stats: BridgeMmapStats,
}

impl BridgeMmapManager {
    pub fn new() -> Self {
        Self {
            regions: BTreeMap::new(),
            next_addr: 0x7f0000000000,
            stats: BridgeMmapStats {
                total_maps: 0,
                anonymous: 0,
                file_backed: 0,
                shared: 0,
                total_mapped_bytes: 0,
                avg_region_size: 0,
            },
        }
    }

    pub fn mmap(&mut self, length: u64, prot: BridgeMmapProt, flags: BridgeMmapFlag, fd: Option<u64>) -> u64 {
        let addr = self.next_addr;
        self.next_addr += (length + 4095) & !4095;
        let region = BridgeMmapRegion { addr, length, prot, flags, fd, offset: 0 };
        self.regions.insert(addr, region);
        self.stats.total_maps += 1;
        self.stats.total_mapped_bytes += length;
        match flags {
            BridgeMmapFlag::Anonymous => self.stats.anonymous += 1,
            BridgeMmapFlag::Shared => self.stats.shared += 1,
            _ => if fd.is_some() { self.stats.file_backed += 1; } else { self.stats.anonymous += 1; }
        }
        addr
    }

    #[inline(always)]
    pub fn region_count(&self) -> usize { self.regions.len() }
    #[inline(always)]
    pub fn stats(&self) -> &BridgeMmapStats { &self.stats }
}
