// SPDX-License-Identifier: GPL-2.0
//! Holistic page_table — page table management and manipulation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page table level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtLevel {
    Pml5,  // 5-level paging
    Pml4,  // PML4/PGD
    Pdpt,  // Page Directory Pointer Table
    Pd,    // Page Directory
    Pt,    // Page Table
}

/// Page flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageFlags(pub u64);

impl PageFlags {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const CACHE_DISABLE: u64 = 1 << 4;
    pub const ACCESSED: u64 = 1 << 5;
    pub const DIRTY: u64 = 1 << 6;
    pub const HUGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;
    pub const NO_EXECUTE: u64 = 1 << 63;

    pub fn new() -> Self { Self(0) }
    pub fn set(&mut self, flag: u64) { self.0 |= flag; }
    pub fn clear(&mut self, flag: u64) { self.0 &= !flag; }
    pub fn has(&self, flag: u64) -> bool { self.0 & flag != 0 }
    pub fn is_present(&self) -> bool { self.has(Self::PRESENT) }
    pub fn is_writable(&self) -> bool { self.has(Self::WRITABLE) }
    pub fn is_user(&self) -> bool { self.has(Self::USER) }
    pub fn is_huge(&self) -> bool { self.has(Self::HUGE) }
}

/// Page table entry
#[derive(Debug, Clone)]
pub struct PtEntry {
    pub vaddr: u64,
    pub paddr: u64,
    pub flags: PageFlags,
    pub level: PtLevel,
    pub access_count: u64,
}

impl PtEntry {
    pub fn new(vaddr: u64, paddr: u64, flags: PageFlags, level: PtLevel) -> Self {
        Self { vaddr, paddr, flags, level, access_count: 0 }
    }

    pub fn page_size(&self) -> u64 {
        match self.level {
            PtLevel::Pml5 | PtLevel::Pml4 => 512 * 1024 * 1024 * 1024,
            PtLevel::Pdpt => 1024 * 1024 * 1024,
            PtLevel::Pd => 2 * 1024 * 1024,
            PtLevel::Pt => 4096,
        }
    }

    pub fn frame_number(&self) -> u64 { self.paddr >> 12 }
}

/// Address space
#[derive(Debug)]
pub struct AddressSpace {
    pub id: u64,
    pub root_paddr: u64,
    pub entries: BTreeMap<u64, PtEntry>,
    pub total_mapped: u64,
    pub page_faults: u64,
    pub tlb_flushes: u64,
    pub levels: u8,
}

impl AddressSpace {
    pub fn new(id: u64, root: u64, levels: u8) -> Self {
        Self {
            id, root_paddr: root, entries: BTreeMap::new(),
            total_mapped: 0, page_faults: 0, tlb_flushes: 0, levels,
        }
    }

    pub fn map_page(&mut self, vaddr: u64, paddr: u64, flags: PageFlags, level: PtLevel) {
        let entry = PtEntry::new(vaddr, paddr, flags, level);
        let size = entry.page_size();
        self.entries.insert(vaddr, entry);
        self.total_mapped += size;
    }

    pub fn unmap_page(&mut self, vaddr: u64) -> Option<u64> {
        let entry = self.entries.remove(&vaddr)?;
        self.total_mapped -= entry.page_size();
        self.tlb_flushes += 1;
        Some(entry.paddr)
    }

    pub fn translate(&self, vaddr: u64) -> Option<u64> {
        // Walk: find containing entry
        for (base, entry) in &self.entries {
            if vaddr >= *base && vaddr < *base + entry.page_size() {
                if entry.flags.is_present() {
                    return Some(entry.paddr + (vaddr - base));
                }
            }
        }
        None
    }
}

/// TLB flush type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlbFlushType {
    Single(u64),
    Range(u64, u64),
    Full,
    Global,
    Pcid(u16),
}

/// Stats
#[derive(Debug, Clone)]
pub struct PageTableStats {
    pub total_address_spaces: u32,
    pub total_mappings: u64,
    pub total_mapped_bytes: u64,
    pub total_page_faults: u64,
    pub total_tlb_flushes: u64,
    pub huge_pages_2m: u32,
    pub huge_pages_1g: u32,
}

/// Main page table manager
pub struct HolisticPageTable {
    spaces: BTreeMap<u64, AddressSpace>,
    next_id: u64,
}

impl HolisticPageTable {
    pub fn new() -> Self { Self { spaces: BTreeMap::new(), next_id: 1 } }

    pub fn create_space(&mut self, root: u64, levels: u8) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.spaces.insert(id, AddressSpace::new(id, root, levels));
        id
    }

    pub fn map(&mut self, space: u64, vaddr: u64, paddr: u64, flags: PageFlags, level: PtLevel) {
        if let Some(s) = self.spaces.get_mut(&space) { s.map_page(vaddr, paddr, flags, level); }
    }

    pub fn unmap(&mut self, space: u64, vaddr: u64) -> Option<u64> {
        self.spaces.get_mut(&space)?.unmap_page(vaddr)
    }

    pub fn translate(&self, space: u64, vaddr: u64) -> Option<u64> {
        self.spaces.get(&space)?.translate(vaddr)
    }

    pub fn stats(&self) -> PageTableStats {
        let mappings: u64 = self.spaces.values().map(|s| s.entries.len() as u64).sum();
        let mapped: u64 = self.spaces.values().map(|s| s.total_mapped).sum();
        let faults: u64 = self.spaces.values().map(|s| s.page_faults).sum();
        let flushes: u64 = self.spaces.values().map(|s| s.tlb_flushes).sum();
        let huge_2m = self.spaces.values().flat_map(|s| s.entries.values()).filter(|e| matches!(e.level, PtLevel::Pd) && e.flags.is_huge()).count() as u32;
        let huge_1g = self.spaces.values().flat_map(|s| s.entries.values()).filter(|e| matches!(e.level, PtLevel::Pdpt) && e.flags.is_huge()).count() as u32;
        PageTableStats {
            total_address_spaces: self.spaces.len() as u32, total_mappings: mappings,
            total_mapped_bytes: mapped, total_page_faults: faults,
            total_tlb_flushes: flushes, huge_pages_2m: huge_2m, huge_pages_1g: huge_1g,
        }
    }
}
