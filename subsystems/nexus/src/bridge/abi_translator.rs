//! # Bridge ABI Translator
//!
//! Application Binary Interface translation layer:
//! - Syscall ABI conversion between versions
//! - Struct layout translation (32-bit ↔ 64-bit)
//! - Endianness handling
//! - Argument register mapping
//! - Return value adaptation
//! - Compat mode detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// ABI version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbiVersion {
    V1Legacy,
    V2Compat,
    V3Current,
    V4Extended,
}

/// Argument type for ABI translation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiArgType {
    U32,
    U64,
    Pointer32,
    Pointer64,
    StructRef,
    Buffer,
    Fd,
}

/// ABI register mapping
#[derive(Debug, Clone)]
pub struct RegisterMapping {
    pub syscall_nr_reg: u8,
    pub arg_regs: [u8; 6],
    pub return_reg: u8,
    pub error_reg: Option<u8>,
}

impl RegisterMapping {
    #[inline]
    pub fn x86_64_default() -> Self {
        Self {
            syscall_nr_reg: 0, // rax
            arg_regs: [7, 6, 2, 10, 8, 9], // rdi, rsi, rdx, r10, r8, r9
            return_reg: 0, // rax
            error_reg: None,
        }
    }
}

/// Struct field descriptor
#[derive(Debug, Clone)]
pub struct FieldDescriptor {
    pub offset_v1: u32,
    pub offset_v2: u32,
    pub size_v1: u32,
    pub size_v2: u32,
    pub field_type: AbiArgType,
}

/// Struct layout translation
#[derive(Debug, Clone)]
pub struct StructTranslation {
    pub struct_hash: u64,
    pub v1_size: u32,
    pub v2_size: u32,
    pub fields: Vec<FieldDescriptor>,
}

impl StructTranslation {
    pub fn new(struct_hash: u64, v1_size: u32, v2_size: u32) -> Self {
        Self { struct_hash, v1_size, v2_size, fields: Vec::new() }
    }

    #[inline(always)]
    pub fn add_field(&mut self, field: FieldDescriptor) {
        self.fields.push(field);
    }

    /// Needs translation?
    #[inline(always)]
    pub fn needs_translation(&self) -> bool {
        self.v1_size != self.v2_size || self.fields.iter().any(|f| f.offset_v1 != f.offset_v2 || f.size_v1 != f.size_v2)
    }
}

/// Syscall translation entry
#[derive(Debug, Clone)]
pub struct SyscallTranslation {
    pub from_nr: u32,
    pub to_nr: u32,
    pub from_abi: AbiVersion,
    pub to_abi: AbiVersion,
    pub arg_translations: Vec<(usize, AbiArgType, AbiArgType)>,
    pub struct_translations: Vec<u64>, // struct hashes needing translation
    pub invocations: u64,
}

impl SyscallTranslation {
    pub fn new(from_nr: u32, to_nr: u32, from_abi: AbiVersion, to_abi: AbiVersion) -> Self {
        Self {
            from_nr,
            to_nr,
            from_abi,
            to_abi,
            arg_translations: Vec::new(),
            struct_translations: Vec::new(),
            invocations: 0,
        }
    }

    #[inline(always)]
    pub fn is_identity(&self) -> bool {
        self.from_nr == self.to_nr && self.arg_translations.is_empty()
    }
}

/// ABI translator stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeAbiTranslatorStats {
    pub registered_translations: usize,
    pub struct_layouts: usize,
    pub total_invocations: u64,
    pub identity_translations: usize,
    pub compat_processes: usize,
}

/// Bridge ABI Translator
#[repr(align(64))]
pub struct BridgeAbiTranslator {
    translations: BTreeMap<(AbiVersion, u32), SyscallTranslation>,
    struct_layouts: BTreeMap<u64, StructTranslation>,
    register_maps: BTreeMap<AbiVersion, RegisterMapping>,
    compat_pids: BTreeMap<u64, AbiVersion>,
    stats: BridgeAbiTranslatorStats,
}

impl BridgeAbiTranslator {
    pub fn new() -> Self {
        let mut register_maps = BTreeMap::new();
        register_maps.insert(AbiVersion::V3Current, RegisterMapping::x86_64_default());

        Self {
            translations: BTreeMap::new(),
            struct_layouts: BTreeMap::new(),
            register_maps,
            compat_pids: BTreeMap::new(),
            stats: BridgeAbiTranslatorStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_translation(&mut self, trans: SyscallTranslation) {
        self.translations.insert((trans.from_abi, trans.from_nr), trans);
        self.recompute();
    }

    #[inline(always)]
    pub fn register_struct_layout(&mut self, layout: StructTranslation) {
        self.struct_layouts.insert(layout.struct_hash, layout);
        self.recompute();
    }

    #[inline(always)]
    pub fn set_compat_mode(&mut self, pid: u64, abi: AbiVersion) {
        self.compat_pids.insert(pid, abi);
        self.recompute();
    }

    #[inline(always)]
    pub fn get_compat_mode(&self, pid: u64) -> AbiVersion {
        self.compat_pids.get(&pid).copied().unwrap_or(AbiVersion::V3Current)
    }

    /// Translate a syscall number from compat ABI to current
    #[inline]
    pub fn translate_syscall(&mut self, pid: u64, syscall_nr: u32) -> u32 {
        let abi = self.get_compat_mode(pid);
        if abi == AbiVersion::V3Current { return syscall_nr; }

        if let Some(trans) = self.translations.get_mut(&(abi, syscall_nr)) {
            trans.invocations += 1;
            trans.to_nr
        } else {
            syscall_nr
        }
    }

    /// Check if struct needs translation
    #[inline]
    pub fn needs_struct_translation(&self, struct_hash: u64) -> bool {
        self.struct_layouts.get(&struct_hash)
            .map(|s| s.needs_translation())
            .unwrap_or(false)
    }

    fn recompute(&mut self) {
        self.stats.registered_translations = self.translations.len();
        self.stats.struct_layouts = self.struct_layouts.len();
        self.stats.total_invocations = self.translations.values().map(|t| t.invocations).sum();
        self.stats.identity_translations = self.translations.values().filter(|t| t.is_identity()).count();
        self.stats.compat_processes = self.compat_pids.len();
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeAbiTranslatorStats {
        &self.stats
    }
}
