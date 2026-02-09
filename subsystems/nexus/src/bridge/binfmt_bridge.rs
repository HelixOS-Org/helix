// SPDX-License-Identifier: GPL-2.0
//! Bridge binfmt_bridge — binary format handler bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Binary format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinfmtType {
    Elf,
    Script,
    Flat,
    Misc,
    AOut,
    Wasm,
}

/// Load state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinfmtLoadState {
    Parsing,
    Loading,
    Relocating,
    Ready,
    Failed,
}

/// ELF header info
#[derive(Debug, Clone)]
pub struct ElfInfo {
    pub e_type: u16,
    pub e_machine: u16,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_phnum: u16,
    pub e_shnum: u16,
    pub is_pie: bool,
    pub interp_hash: u64,
}

/// Program header (segment)
#[derive(Debug, Clone)]
pub struct ProgHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

/// Binary image
#[derive(Debug)]
#[repr(align(64))]
pub struct BinaryImage {
    pub id: u64,
    pub fmt_type: BinfmtType,
    pub state: BinfmtLoadState,
    pub elf_info: Option<ElfInfo>,
    pub segments: Vec<ProgHeader>,
    pub load_addr: u64,
    pub entry_point: u64,
    pub total_size: u64,
    pub load_time_ns: u64,
}

impl BinaryImage {
    pub fn new(id: u64, fmt: BinfmtType) -> Self {
        Self {
            id, fmt_type: fmt, state: BinfmtLoadState::Parsing,
            elf_info: None, segments: Vec::new(), load_addr: 0,
            entry_point: 0, total_size: 0, load_time_ns: 0,
        }
    }

    #[inline]
    pub fn set_loaded(&mut self, addr: u64, entry: u64, size: u64, duration: u64) {
        self.load_addr = addr;
        self.entry_point = entry;
        self.total_size = size;
        self.load_time_ns = duration;
        self.state = BinfmtLoadState::Ready;
    }

    #[inline(always)]
    pub fn fail(&mut self) { self.state = BinfmtLoadState::Failed; }
}

/// Binfmt misc rule
#[derive(Debug, Clone)]
pub struct BinfmtMiscRule {
    pub name_hash: u64,
    pub magic: Vec<u8>,
    pub mask: Vec<u8>,
    pub interpreter_hash: u64,
    pub offset: u32,
    pub enabled: bool,
    pub match_count: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BinfmtBridgeStats {
    pub total_loaded: u64,
    pub total_failed: u64,
    pub elf_count: u64,
    pub script_count: u64,
    pub misc_rules: u32,
    pub avg_load_time_ns: u64,
}

/// Main binfmt bridge
#[repr(align(64))]
pub struct BridgeBinfmt {
    images: BTreeMap<u64, BinaryImage>,
    misc_rules: Vec<BinfmtMiscRule>,
    next_id: u64,
}

impl BridgeBinfmt {
    pub fn new() -> Self { Self { images: BTreeMap::new(), misc_rules: Vec::new(), next_id: 1 } }

    #[inline]
    pub fn load_image(&mut self, fmt: BinfmtType) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.images.insert(id, BinaryImage::new(id, fmt));
        id
    }

    #[inline(always)]
    pub fn complete_load(&mut self, id: u64, addr: u64, entry: u64, size: u64, duration: u64) {
        if let Some(img) = self.images.get_mut(&id) { img.set_loaded(addr, entry, size, duration); }
    }

    #[inline(always)]
    pub fn add_misc_rule(&mut self, name_hash: u64, magic: Vec<u8>, interp_hash: u64) {
        self.misc_rules.push(BinfmtMiscRule { name_hash, magic: magic.clone(), mask: Vec::new(), interpreter_hash: interp_hash, offset: 0, enabled: true, match_count: 0 });
    }

    #[inline]
    pub fn stats(&self) -> BinfmtBridgeStats {
        let loaded = self.images.values().filter(|i| i.state == BinfmtLoadState::Ready).count() as u64;
        let failed = self.images.values().filter(|i| i.state == BinfmtLoadState::Failed).count() as u64;
        let elf = self.images.values().filter(|i| i.fmt_type == BinfmtType::Elf).count() as u64;
        let script = self.images.values().filter(|i| i.fmt_type == BinfmtType::Script).count() as u64;
        let times: Vec<u64> = self.images.values().filter(|i| i.load_time_ns > 0).map(|i| i.load_time_ns).collect();
        let avg = if times.is_empty() { 0 } else { times.iter().sum::<u64>() / times.len() as u64 };
        BinfmtBridgeStats { total_loaded: loaded, total_failed: failed, elf_count: elf, script_count: script, misc_rules: self.misc_rules.len() as u32, avg_load_time_ns: avg }
    }
}

// ============================================================================
// Merged from binfmt_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinfmtV2Type {
    Elf64,
    Elf32,
    Script,
    Misc,
    Flat,
    AOut,
}

/// Binary format entry v2
#[derive(Debug)]
pub struct BinfmtV2Entry {
    pub magic: u64,
    pub mask: u64,
    pub fmt_type: BinfmtV2Type,
    pub interpreter_hash: u64,
    pub offset: u32,
    pub flags: u32,
    pub enabled: bool,
    pub exec_count: u64,
}

impl BinfmtV2Entry {
    pub fn new(magic: u64, fmt: BinfmtV2Type) -> Self {
        Self { magic, mask: u64::MAX, fmt_type: fmt, interpreter_hash: 0, offset: 0, flags: 0, enabled: true, exec_count: 0 }
    }
}

/// Load info v2
#[derive(Debug)]
#[repr(align(64))]
pub struct LoadInfoV2 {
    pub interp_hash: u64,
    pub entry_point: u64,
    pub load_addr: u64,
    pub phdr_addr: u64,
    pub phnum: u16,
    pub is_pie: bool,
    pub stack_size: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BinfmtV2BridgeStats {
    pub total_formats: u32,
    pub enabled: u32,
    pub total_executions: u64,
}

/// Main bridge binfmt v2
#[repr(align(64))]
pub struct BridgeBinfmtV2 {
    formats: BTreeMap<u64, BinfmtV2Entry>,
}

impl BridgeBinfmtV2 {
    pub fn new() -> Self { Self { formats: BTreeMap::new() } }

    #[inline(always)]
    pub fn register(&mut self, magic: u64, fmt: BinfmtV2Type) { self.formats.insert(magic, BinfmtV2Entry::new(magic, fmt)); }

    #[inline]
    pub fn lookup(&mut self, magic: u64) -> Option<&BinfmtV2Entry> {
        for (_, entry) in &mut self.formats {
            if entry.enabled && (magic & entry.mask) == entry.magic {
                entry.exec_count += 1;
                return Some(entry);
            }
        }
        None
    }

    #[inline(always)]
    pub fn disable(&mut self, magic: u64) { if let Some(e) = self.formats.get_mut(&magic) { e.enabled = false; } }

    #[inline]
    pub fn stats(&self) -> BinfmtV2BridgeStats {
        let enabled = self.formats.values().filter(|e| e.enabled).count() as u32;
        let execs: u64 = self.formats.values().map(|e| e.exec_count).sum();
        BinfmtV2BridgeStats { total_formats: self.formats.len() as u32, enabled, total_executions: execs }
    }
}
