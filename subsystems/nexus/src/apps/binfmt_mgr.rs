// SPDX-License-Identifier: GPL-2.0
//! Apps binfmt_mgr — binary format handler for exec-family syscalls.

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Binary format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryFormat {
    Elf64,
    Elf32,
    Script,
    FlatBinary,
    JavaClass,
    PeExe,
    MachO,
    Wasm,
    Custom,
}

/// ELF machine type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfMachine {
    X86,
    X86_64,
    Arm,
    Aarch64,
    RiscV,
    Mips,
    PowerPc,
    S390x,
    Unknown(u16),
}

/// Interpreter info for scripts / dynamically linked binaries
#[derive(Debug, Clone)]
pub struct InterpreterInfo {
    pub path: String,
    pub args: Vec<String>,
    pub is_dynamic_linker: bool,
}

/// Binary header info
#[derive(Debug, Clone)]
pub struct BinaryHeader {
    pub format: BinaryFormat,
    pub machine: ElfMachine,
    pub entry_point: u64,
    pub phdr_offset: u64,
    pub phdr_count: u16,
    pub shdr_offset: u64,
    pub shdr_count: u16,
    pub flags: u32,
    pub is_pie: bool,
    pub has_interp: bool,
}

/// Program segment
#[derive(Debug, Clone, Copy)]
pub struct ProgramSegment {
    pub seg_type: SegmentType,
    pub vaddr: u64,
    pub paddr: u64,
    pub file_size: u64,
    pub mem_size: u64,
    pub alignment: u64,
    pub flags: u32,
}

/// Segment types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentType {
    Null,
    Load,
    Dynamic,
    Interp,
    Note,
    Phdr,
    Tls,
    GnuEhFrame,
    GnuStack,
    GnuRelro,
    Unknown(u32),
}

impl ProgramSegment {
    #[inline(always)]
    pub fn is_readable(&self) -> bool { self.flags & 0x4 != 0 }
    #[inline(always)]
    pub fn is_writable(&self) -> bool { self.flags & 0x2 != 0 }
    #[inline(always)]
    pub fn is_executable(&self) -> bool { self.flags & 0x1 != 0 }

    #[inline(always)]
    pub fn end_vaddr(&self) -> u64 { self.vaddr + self.mem_size }
    #[inline(always)]
    pub fn bss_size(&self) -> u64 { self.mem_size.saturating_sub(self.file_size) }
}

/// binfmt_misc registration
#[derive(Debug, Clone)]
pub struct BinfmtMiscEntry {
    pub name: String,
    pub magic: Vec<u8>,
    pub mask: Vec<u8>,
    pub interpreter: String,
    pub offset: u32,
    pub enabled: bool,
    pub hit_count: u64,
}

impl BinfmtMiscEntry {
    #[inline]
    pub fn matches(&self, header: &[u8]) -> bool {
        if !self.enabled { return false; }
        let off = self.offset as usize;
        if header.len() < off + self.magic.len() { return false; }
        for (i, &m) in self.magic.iter().enumerate() {
            let mask_byte = if i < self.mask.len() { self.mask[i] } else { 0xFF };
            if (header[off + i] & mask_byte) != (m & mask_byte) { return false; }
        }
        true
    }
}

/// Exec validation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecValidation {
    Valid,
    InvalidMagic,
    UnsupportedArch,
    CorruptHeader,
    TooLarge,
    NoExecutePermission,
    SecurityBlocked,
}

/// Binfmt manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BinfmtMgrStats {
    pub total_execs: u64,
    pub format_counts: ArrayMap<u64, 32>,
    pub validation_failures: u64,
    pub misc_registrations: u32,
    pub interpreter_lookups: u64,
}

/// Main binary format manager
pub struct AppBinfmtMgr {
    misc_entries: Vec<BinfmtMiscEntry>,
    format_counts: ArrayMap<u64, 32>,
    total_execs: u64,
    validation_failures: u64,
    interpreter_lookups: u64,
    max_binary_size: u64,
    stack_executable: bool,
}

impl AppBinfmtMgr {
    pub fn new() -> Self {
        Self {
            misc_entries: Vec::new(), format_counts: ArrayMap::new(0),
            total_execs: 0, validation_failures: 0,
            interpreter_lookups: 0, max_binary_size: 256 * 1024 * 1024,
            stack_executable: false,
        }
    }

    #[inline(always)]
    pub fn register_misc(&mut self, entry: BinfmtMiscEntry) {
        self.misc_entries.push(entry);
    }

    #[inline]
    pub fn unregister_misc(&mut self, name: &str) -> bool {
        let before = self.misc_entries.len();
        self.misc_entries.retain(|e| e.name != name);
        self.misc_entries.len() < before
    }

    pub fn detect_format(&self, header: &[u8]) -> Option<BinaryFormat> {
        if header.len() < 4 { return None; }
        if header[0..4] == [0x7f, b'E', b'L', b'F'] {
            if header.len() > 4 {
                return Some(if header[4] == 2 { BinaryFormat::Elf64 } else { BinaryFormat::Elf32 });
            }
            return Some(BinaryFormat::Elf64);
        }
        if header[0..2] == [b'#', b'!'] { return Some(BinaryFormat::Script); }
        if header[0..2] == [b'M', b'Z'] { return Some(BinaryFormat::PeExe); }
        if header[0..4] == [0x00, 0x61, 0x73, 0x6d] { return Some(BinaryFormat::Wasm); }
        for entry in &self.misc_entries {
            if entry.matches(header) { return Some(BinaryFormat::Custom); }
        }
        None
    }

    #[inline]
    pub fn validate_exec(&self, header: &BinaryHeader, file_size: u64) -> ExecValidation {
        if file_size > self.max_binary_size { return ExecValidation::TooLarge; }
        if header.entry_point == 0 && header.format != BinaryFormat::Script {
            return ExecValidation::CorruptHeader;
        }
        ExecValidation::Valid
    }

    #[inline(always)]
    pub fn record_exec(&mut self, format: BinaryFormat) {
        self.total_execs += 1;
        self.format_counts.add(format as usize, 1);
    }

    #[inline(always)]
    pub fn record_failure(&mut self) { self.validation_failures += 1; }

    #[inline]
    pub fn lookup_interpreter(&mut self, header: &BinaryHeader) -> Option<InterpreterInfo> {
        self.interpreter_lookups += 1;
        if header.has_interp {
            Some(InterpreterInfo {
                path: String::from("/lib64/ld-linux-x86-64.so.2"),
                args: Vec::new(),
                is_dynamic_linker: true,
            })
        } else { None }
    }

    #[inline]
    pub fn stats(&self) -> BinfmtMgrStats {
        BinfmtMgrStats {
            total_execs: self.total_execs,
            format_counts: self.format_counts.clone(),
            validation_failures: self.validation_failures,
            misc_registrations: self.misc_entries.len() as u32,
            interpreter_lookups: self.interpreter_lookups,
        }
    }
}
