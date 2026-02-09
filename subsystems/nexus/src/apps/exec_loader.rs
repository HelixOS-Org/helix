//! # Apps Exec Loader Bridge
//!
//! Application executable loading management:
//! - ELF header parsing and validation metadata
//! - Dynamic linker interaction tracking
//! - Shared library dependency graph
//! - Symbol resolution statistics
//! - Interposition and LD_PRELOAD tracking
//! - ASLR layout recording

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// ELF type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfType {
    Executable,
    SharedObject,
    Relocatable,
    Core,
}

/// Architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfArch {
    X86_64,
    Aarch64,
    Riscv64,
    X86,
    Arm,
}

/// ELF metadata (parsed header info)
#[derive(Debug, Clone)]
pub struct ElfMetadata {
    pub path_hash: u64,
    pub elf_type: ElfType,
    pub arch: ElfArch,
    pub entry_point: u64,
    pub phdr_count: u16,
    pub shdr_count: u16,
    pub interp_hash: u64,
    pub is_pie: bool,
    pub has_relro: bool,
    pub has_nx_stack: bool,
    pub build_id_hash: u64,
}

/// Shared library descriptor
#[derive(Debug, Clone)]
pub struct SharedLib {
    pub path_hash: u64,
    pub load_base: u64,
    pub size: u64,
    pub soname_hash: u64,
    pub ref_count: u32,
    pub symbols_exported: u32,
    pub symbols_imported: u32,
    pub init_called: bool,
    pub fini_registered: bool,
    pub tls_size: u64,
    pub load_order: u32,
}

/// Symbol resolution record
#[derive(Debug, Clone)]
pub struct SymbolResolution {
    pub name_hash: u64,
    pub resolved_addr: u64,
    pub defining_lib_hash: u64,
    pub bind_type: SymbolBind,
    pub resolution_ns: u64,
    pub interposed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolBind {
    Local,
    Global,
    Weak,
    Lazy,
    Now,
}

/// ASLR layout
#[derive(Debug, Clone)]
pub struct AslrLayout {
    pub text_base: u64,
    pub data_base: u64,
    pub heap_start: u64,
    pub mmap_base: u64,
    pub stack_top: u64,
    pub vdso_base: u64,
    pub entropy_bits: u8,
}

/// Per-process exec state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessExecState {
    pub process_id: u64,
    pub elf_meta: Option<ElfMetadata>,
    pub libraries: Vec<SharedLib>,
    pub symbol_cache: BTreeMap<u64, SymbolResolution>,
    pub aslr: Option<AslrLayout>,
    pub ld_preload_count: u32,
    pub total_relocations: u64,
    pub lazy_binds_remaining: u64,
    pub exec_count: u32,
    pub last_exec_ns: u64,
}

impl ProcessExecState {
    pub fn new(pid: u64) -> Self {
        Self {
            process_id: pid,
            elf_meta: None,
            libraries: Vec::new(),
            symbol_cache: BTreeMap::new(),
            aslr: None,
            ld_preload_count: 0,
            total_relocations: 0,
            lazy_binds_remaining: 0,
            exec_count: 0,
            last_exec_ns: 0,
        }
    }

    #[inline]
    pub fn load_elf(&mut self, meta: ElfMetadata, ts: u64) {
        self.elf_meta = Some(meta);
        self.exec_count += 1;
        self.last_exec_ns = ts;
    }

    #[inline(always)]
    pub fn add_library(&mut self, lib: SharedLib) {
        self.libraries.push(lib);
    }

    #[inline(always)]
    pub fn resolve_symbol(&mut self, res: SymbolResolution) {
        self.total_relocations += 1;
        self.symbol_cache.insert(res.name_hash, res);
    }

    #[inline(always)]
    pub fn set_aslr(&mut self, layout: AslrLayout) {
        self.aslr = Some(layout);
    }

    #[inline(always)]
    pub fn lib_count(&self) -> usize { self.libraries.len() }

    #[inline(always)]
    pub fn total_lib_size(&self) -> u64 {
        self.libraries.iter().map(|l| l.size).sum()
    }

    #[inline(always)]
    pub fn find_lib_at(&self, addr: u64) -> Option<&SharedLib> {
        self.libraries.iter().find(|l| addr >= l.load_base && addr < l.load_base + l.size)
    }
}

/// Dependency edge
#[derive(Debug, Clone)]
pub struct LibDependency {
    pub from_hash: u64,
    pub to_hash: u64,
    pub symbols_used: u32,
}

/// Apps exec loader stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppsExecLoaderStats {
    pub total_processes: usize,
    pub total_libraries: usize,
    pub total_symbols: usize,
    pub total_relocations: u64,
    pub total_execs: u64,
    pub pie_count: usize,
}

/// Apps Exec Loader Bridge
pub struct AppsExecLoader {
    states: BTreeMap<u64, ProcessExecState>,
    lib_deps: Vec<LibDependency>,
    stats: AppsExecLoaderStats,
}

impl AppsExecLoader {
    pub fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            lib_deps: Vec::new(),
            stats: AppsExecLoaderStats::default(),
        }
    }

    #[inline(always)]
    pub fn register(&mut self, pid: u64) {
        self.states.entry(pid).or_insert_with(|| ProcessExecState::new(pid));
    }

    #[inline]
    pub fn exec(&mut self, pid: u64, meta: ElfMetadata, ts: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            state.load_elf(meta, ts);
            state.libraries.clear();
            state.symbol_cache.clear();
        }
    }

    #[inline(always)]
    pub fn load_library(&mut self, pid: u64, lib: SharedLib) {
        if let Some(state) = self.states.get_mut(&pid) { state.add_library(lib); }
    }

    #[inline(always)]
    pub fn resolve_symbol(&mut self, pid: u64, res: SymbolResolution) {
        if let Some(state) = self.states.get_mut(&pid) { state.resolve_symbol(res); }
    }

    #[inline(always)]
    pub fn set_aslr(&mut self, pid: u64, layout: AslrLayout) {
        if let Some(state) = self.states.get_mut(&pid) { state.set_aslr(layout); }
    }

    #[inline(always)]
    pub fn add_dependency(&mut self, dep: LibDependency) {
        self.lib_deps.push(dep);
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) { self.states.remove(&pid); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.states.len();
        self.stats.total_libraries = self.states.values().map(|s| s.lib_count()).sum();
        self.stats.total_symbols = self.states.values().map(|s| s.symbol_cache.len()).sum();
        self.stats.total_relocations = self.states.values().map(|s| s.total_relocations).sum();
        self.stats.total_execs = self.states.values().map(|s| s.exec_count as u64).sum();
        self.stats.pie_count = self.states.values()
            .filter(|s| s.elf_meta.as_ref().map(|e| e.is_pie).unwrap_or(false))
            .count();
    }

    #[inline(always)]
    pub fn process_exec(&self, pid: u64) -> Option<&ProcessExecState> { self.states.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &AppsExecLoaderStats { &self.stats }
}

// ============================================================================
// Merged from exec_loader_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecType {
    StaticElf,
    DynamicElf,
    Script,
    FlatBinary,
}

/// Loader state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderState {
    Init,
    Parsing,
    Mapping,
    Relocating,
    Setting,
    Ready,
    Failed,
}

/// Memory mapping
#[derive(Debug, Clone)]
pub struct ExecMapping {
    pub vaddr: u64,
    pub size: u64,
    pub paddr: u64,
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
    pub file_offset: u64,
    pub file_size: u64,
}

/// Auxv entry
#[derive(Debug, Clone, Copy)]
pub struct AuxvEntry {
    pub a_type: u64,
    pub a_val: u64,
}

/// Exec context
#[derive(Debug)]
#[repr(align(64))]
pub struct ExecContext {
    pub id: u64,
    pub exec_type: ExecType,
    pub state: LoaderState,
    pub entry_point: u64,
    pub interp_base: u64,
    pub phdr_addr: u64,
    pub phnum: u16,
    pub mappings: Vec<ExecMapping>,
    pub auxv: Vec<AuxvEntry>,
    pub stack_top: u64,
    pub stack_size: u64,
    pub brk_start: u64,
    pub brk_end: u64,
    pub load_time_ns: u64,
    pub total_mapped: u64,
}

impl ExecContext {
    pub fn new(id: u64, etype: ExecType) -> Self {
        Self {
            id, exec_type: etype, state: LoaderState::Init, entry_point: 0,
            interp_base: 0, phdr_addr: 0, phnum: 0, mappings: Vec::new(),
            auxv: Vec::new(), stack_top: 0, stack_size: 8 * 1024 * 1024,
            brk_start: 0, brk_end: 0, load_time_ns: 0, total_mapped: 0,
        }
    }

    #[inline(always)]
    pub fn add_mapping(&mut self, m: ExecMapping) { self.total_mapped += m.size; self.mappings.push(m); }
    #[inline(always)]
    pub fn set_ready(&mut self, entry: u64, duration: u64) { self.entry_point = entry; self.load_time_ns = duration; self.state = LoaderState::Ready; }
    #[inline(always)]
    pub fn fail(&mut self) { self.state = LoaderState::Failed; }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ExecLoaderV2Stats {
    pub total_loaded: u64,
    pub total_failed: u64,
    pub static_count: u64,
    pub dynamic_count: u64,
    pub total_mapped_bytes: u64,
    pub avg_load_ns: u64,
}

/// Main exec loader v2
pub struct AppExecLoaderV2 {
    contexts: BTreeMap<u64, ExecContext>,
    next_id: u64,
}

impl AppExecLoaderV2 {
    pub fn new() -> Self { Self { contexts: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn begin_load(&mut self, etype: ExecType) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.contexts.insert(id, ExecContext::new(id, etype));
        id
    }

    #[inline(always)]
    pub fn complete(&mut self, id: u64, entry: u64, duration: u64) {
        if let Some(ctx) = self.contexts.get_mut(&id) { ctx.set_ready(entry, duration); }
    }

    #[inline]
    pub fn stats(&self) -> ExecLoaderV2Stats {
        let loaded = self.contexts.values().filter(|c| c.state == LoaderState::Ready).count() as u64;
        let failed = self.contexts.values().filter(|c| c.state == LoaderState::Failed).count() as u64;
        let stat = self.contexts.values().filter(|c| c.exec_type == ExecType::StaticElf).count() as u64;
        let dyn_ = self.contexts.values().filter(|c| c.exec_type == ExecType::DynamicElf).count() as u64;
        let mapped: u64 = self.contexts.values().map(|c| c.total_mapped).sum();
        let times: Vec<u64> = self.contexts.values().filter(|c| c.load_time_ns > 0).map(|c| c.load_time_ns).collect();
        let avg = if times.is_empty() { 0 } else { times.iter().sum::<u64>() / times.len() as u64 };
        ExecLoaderV2Stats { total_loaded: loaded, total_failed: failed, static_count: stat, dynamic_count: dyn_, total_mapped_bytes: mapped, avg_load_ns: avg }
    }
}

// ============================================================================
// Merged from exec_loader_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AslrLevel {
    None,
    Conservative,
    Standard,
    Full,
}

/// Executable format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecV3Format {
    Elf64,
    Elf32,
    Script,
    FlatBin,
}

/// Loader phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadPhase {
    ParseHeaders,
    ValidateElf,
    MapSegments,
    ProcessRelocs,
    SetupStack,
    LoadInterp,
    SetProtections,
    Complete,
    Error,
}

/// Relocation entry
#[derive(Debug, Clone)]
pub struct RelocEntry {
    pub offset: u64,
    pub reloc_type: u32,
    pub symbol_idx: u32,
    pub addend: i64,
    pub resolved: bool,
}

/// Segment info
#[derive(Debug, Clone)]
pub struct SegmentV3 {
    pub vaddr: u64,
    pub paddr: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub mem_size: u64,
    pub flags: u32,
    pub align: u64,
}

/// Exec image
#[derive(Debug)]
pub struct ExecImageV3 {
    pub id: u64,
    pub format: ExecV3Format,
    pub phase: LoadPhase,
    pub entry: u64,
    pub base_addr: u64,
    pub aslr_offset: u64,
    pub segments: Vec<SegmentV3>,
    pub relocs: Vec<RelocEntry>,
    pub interp_base: u64,
    pub stack_addr: u64,
    pub stack_size: u64,
    pub brk: u64,
    pub load_duration_ns: u64,
    pub total_mapped: u64,
    pub aslr: AslrLevel,
}

impl ExecImageV3 {
    pub fn new(id: u64, fmt: ExecV3Format, aslr: AslrLevel) -> Self {
        Self {
            id, format: fmt, phase: LoadPhase::ParseHeaders, entry: 0,
            base_addr: 0, aslr_offset: 0, segments: Vec::new(), relocs: Vec::new(),
            interp_base: 0, stack_addr: 0, stack_size: 8 * 1024 * 1024, brk: 0,
            load_duration_ns: 0, total_mapped: 0, aslr,
        }
    }

    #[inline(always)]
    pub fn add_segment(&mut self, seg: SegmentV3) { self.total_mapped += seg.mem_size; self.segments.push(seg); }
    #[inline(always)]
    pub fn advance(&mut self, phase: LoadPhase) { self.phase = phase; }
    #[inline(always)]
    pub fn complete(&mut self, entry: u64, duration: u64) { self.entry = entry; self.load_duration_ns = duration; self.phase = LoadPhase::Complete; }
    #[inline(always)]
    pub fn fail(&mut self) { self.phase = LoadPhase::Error; }
    #[inline(always)]
    pub fn effective_entry(&self) -> u64 { self.entry.wrapping_add(self.aslr_offset) }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ExecLoaderV3Stats {
    pub total_images: u32,
    pub completed: u32,
    pub failed: u32,
    pub total_mapped: u64,
    pub total_relocs: u64,
    pub avg_load_ns: u64,
}

/// Main exec loader v3
pub struct AppExecLoaderV3 {
    images: BTreeMap<u64, ExecImageV3>,
    next_id: u64,
    seed: u64,
}

impl AppExecLoaderV3 {
    pub fn new() -> Self { Self { images: BTreeMap::new(), next_id: 1, seed: 0xdeadbeef12345678 } }

    fn gen_aslr(&mut self, level: AslrLevel) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        match level {
            AslrLevel::None => 0,
            AslrLevel::Conservative => (self.seed & 0xFF) << 12,
            AslrLevel::Standard => (self.seed & 0xFFFF) << 12,
            AslrLevel::Full => (self.seed & 0xFFFFFFF) << 12,
        }
    }

    #[inline]
    pub fn load(&mut self, fmt: ExecV3Format, aslr: AslrLevel) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let mut img = ExecImageV3::new(id, fmt, aslr);
        img.aslr_offset = self.gen_aslr(aslr);
        self.images.insert(id, img);
        id
    }

    #[inline]
    pub fn stats(&self) -> ExecLoaderV3Stats {
        let comp = self.images.values().filter(|i| i.phase == LoadPhase::Complete).count() as u32;
        let fail = self.images.values().filter(|i| i.phase == LoadPhase::Error).count() as u32;
        let mapped: u64 = self.images.values().map(|i| i.total_mapped).sum();
        let relocs: u64 = self.images.values().map(|i| i.relocs.len() as u64).sum();
        let times: Vec<u64> = self.images.values().filter(|i| i.load_duration_ns > 0).map(|i| i.load_duration_ns).collect();
        let avg = if times.is_empty() { 0 } else { times.iter().sum::<u64>() / times.len() as u64 };
        ExecLoaderV3Stats { total_images: self.images.len() as u32, completed: comp, failed: fail, total_mapped: mapped, total_relocs: relocs, avg_load_ns: avg }
    }
}
